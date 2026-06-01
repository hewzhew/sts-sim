#!/usr/bin/env python3
"""Post-run failure interrogation into a small ToolNeedCard backlog.

This script does not change controller behavior. It reads compact event JSONL,
extracts a bounded failure packet, asks a critic provider for at most five
tool-need cards, and writes JSONL for human review.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import textwrap
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(event, dict):
            event["_line"] = line_number
            events.append(event)
    return events


def compact_json(value: Any, *, limit: int = 12000) -> str:
    text = json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    if len(text) <= limit:
        return text
    return text[:limit] + "...<truncated>"


def shorten(value: Any, *, width: int = 180) -> str | None:
    if value is None:
        return None
    return textwrap.shorten(str(value), width=width, placeholder="...")


def call_openai_compatible(
    *,
    base_url: str,
    api_key: str,
    model: str,
    system: str,
    user: str,
    temperature: float,
    timeout: float,
) -> str:
    url = base_url.rstrip("/") + "/chat/completions"
    payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "temperature": temperature,
    }
    data = json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(
        url,
        data=data,
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    start = time.perf_counter()
    print("critic_call_start", file=sys.stderr)
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            body = response.read().decode("utf-8")
    except urllib.error.HTTPError as err:
        body = err.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"critic provider HTTP {err.code}: {body[:1000]}") from err
    finally:
        print(f"critic_call_end elapsed={time.perf_counter() - start:.1f}s", file=sys.stderr)
    parsed = json.loads(body)
    return str(parsed["choices"][0]["message"]["content"])


def extract_json_array(text: str) -> list[Any]:
    text = text.strip()
    try:
        parsed = json.loads(text)
        if isinstance(parsed, list):
            return parsed
        if isinstance(parsed, dict) and isinstance(parsed.get("tool_need_cards"), list):
            return parsed["tool_need_cards"]
    except json.JSONDecodeError:
        pass
    match = re.search(r"\[[\s\S]*\]", text)
    if not match:
        return []
    try:
        parsed = json.loads(match.group(0))
    except json.JSONDecodeError:
        return []
    return parsed if isinstance(parsed, list) else []


def event_step(event: dict[str, Any]) -> int | None:
    step = event.get("step")
    return step if isinstance(step, int) else None


def find_failure_step(events: list[dict[str, Any]]) -> int | None:
    verifier_events = [event for event in events if event.get("schema_name") == "VerifierDecisionEvent"]
    for event in verifier_events:
        if event.get("done") or str(event.get("post_result") or "").lower() == "defeat":
            step = event_step(event)
            if step is not None:
                return step
    largest_loss_step = None
    largest_loss = 0
    previous_hp = None
    for event in verifier_events:
        hp = event.get("post_hp")
        if isinstance(previous_hp, int) and isinstance(hp, int):
            loss = previous_hp - hp
            if loss > largest_loss:
                largest_loss = loss
                largest_loss_step = event_step(event)
        if isinstance(hp, int):
            previous_hp = hp
    return largest_loss_step


def group_by_step(events: list[dict[str, Any]]) -> dict[int, list[dict[str, Any]]]:
    grouped: dict[int, list[dict[str, Any]]] = {}
    for event in events:
        step = event_step(event)
        if step is None:
            continue
        grouped.setdefault(step, []).append(event)
    return grouped


def summarize_tool_event(event: dict[str, Any]) -> dict[str, Any]:
    summary = event.get("summary") if isinstance(event.get("summary"), dict) else {}
    if event.get("tool") == "combat_multi_turn_lab":
        return {
            "tool": event.get("tool"),
            "hard_tactical_brief": summary.get("hard_tactical_brief"),
        }
    return {
        "tool": event.get("tool"),
        "summary": summary,
    }


def build_failure_packet(events: list[dict[str, Any]], *, source_path: Path, window: int) -> dict[str, Any]:
    failure_step = find_failure_step(events)
    if failure_step is None:
        failure_step = max((event_step(event) or 0 for event in events), default=0)
    grouped = group_by_step(events)
    steps = range(max(0, failure_step - window + 1), failure_step + 1)
    decisions = []
    for step in steps:
        items = grouped.get(step) or []
        verifier = next((event for event in items if event.get("schema_name") == "VerifierDecisionEvent"), None)
        if verifier is None:
            continue
        frame = next((event for event in items if event.get("schema_name") == "DecisionFrameEvent"), None)
        recommendation = next((event for event in items if event.get("schema_name") == "RecommendationEvent"), None)
        tool_events = [event for event in items if event.get("schema_name") == "ToolResultEvent"]
        decisions.append(
            {
                "step": step,
                "floor": verifier.get("floor"),
                "hp_before": verifier.get("hp"),
                "risk_flags": verifier.get("risk_flags") or [],
                "decision_type": verifier.get("decision_type"),
                "decision_class": verifier.get("decision_class"),
                "decision_owner": verifier.get("decision_owner"),
                "executed_action_owner": verifier.get("executed_action_owner"),
                "action": verifier.get("final_action_key"),
                "post_hp": verifier.get("post_hp"),
                "post_result": verifier.get("post_result"),
                "terminal_reason": verifier.get("post_terminal_reason"),
                "done": verifier.get("done"),
                "evidence_needs": frame.get("evidence_needs") if frame else [],
                "authority_scope": frame.get("authority_scope") if frame else {},
                "llm_reason": shorten(recommendation.get("reason"), width=260) if recommendation else None,
                "tool_evidence": [summarize_tool_event(event) for event in tool_events[:3]],
            }
        )
    final = decisions[-1] if decisions else {}
    return {
        "schema_name": "FailurePacket",
        "schema_version": 1,
        "source_run": str(source_path),
        "failure_step": failure_step,
        "window": window,
        "final": final,
        "recent_decisions": decisions,
        "instructions": {
            "purpose": "Generate a small human-reviewable backlog of missing questions/tools. Do not propose automatic controller changes.",
            "max_cards": 5,
        },
    }


def mock_cards(packet: dict[str, Any], *, max_cards: int) -> list[dict[str, Any]]:
    final = packet.get("final") if isinstance(packet.get("final"), dict) else {}
    decision_class = str(final.get("decision_class") or "unknown")
    floor = final.get("floor")
    cards = [
        {
            "need_id": "forced_death_line_probe",
            "source_failure": f"{Path(str(packet.get('source_run') or '')).name}:step{packet.get('failure_step')}:floor{floor}",
            "decision_class": decision_class,
            "failure_hypothesis": "The controller selected actions without a clear pre-decision answer to whether they enter a bounded forced-death line.",
            "missing_question": "Which current legal actions preserve survival over a bounded simulator horizon, and which enter terminal or critical HP lines?",
            "proposed_tool": "forced_death_line_probe",
            "tool_input": ["decision_frame", "public_observation", "legal_actions", "horizon_steps"],
            "tool_output": ["action_key", "death_within_horizon", "terminal_hp", "critical_hp", "first_bad_transition_reason"],
            "acceptance_test": f"On {Path(str(packet.get('source_run') or '')).name} step {packet.get('failure_step')}, the card flags the executed action's death/critical line before execution.",
            "fix_category": "tool",
            "general_usefulness": "high",
            "human_status": "unreviewed",
        }
    ]
    return cards[:max_cards]


def normalize_cards(raw_cards: list[Any], packet: dict[str, Any], *, max_cards: int) -> list[dict[str, Any]]:
    cards: list[dict[str, Any]] = []
    required_keys = [
        "need_id",
        "source_failure",
        "decision_class",
        "failure_hypothesis",
        "missing_question",
        "proposed_tool",
        "tool_input",
        "tool_output",
        "acceptance_test",
        "fix_category",
    ]
    for raw in raw_cards:
        if not isinstance(raw, dict):
            continue
        card = {key: raw.get(key) for key in required_keys}
        if not all(card.get(key) for key in required_keys):
            continue
        card["schema_name"] = "ToolNeedCard"
        card["schema_version"] = 1
        card["source_run"] = packet.get("source_run")
        card["failure_step"] = packet.get("failure_step")
        card["general_usefulness"] = raw.get("general_usefulness") or "unknown"
        card["human_status"] = raw.get("human_status") or "unreviewed"
        card["policy_quality_claim"] = False
        card["label_role"] = "not_a_label"
        cards.append(card)
        if len(cards) >= max_cards:
            break
    return cards


def critic_prompt(packet: dict[str, Any], *, max_cards: int) -> tuple[str, str]:
    system = (
        "You are a post-run critic for an LLM-first Slay the Spire controller harness. "
        "Your job is not to choose actions and not to improve the controller directly. "
        "Extract a small backlog of missing questions/tools from the failure packet. "
        "Return JSON array only. Every card must be actionable and must include an acceptance_test. "
        "Do not propose replacing the LLM with search except as a clearly scoped safety override."
    )
    user = "\n".join(
        [
            f"Generate at most {max_cards} ToolNeedCard objects.",
            "Each object must include:",
            "need_id, source_failure, decision_class, failure_hypothesis, missing_question, proposed_tool, tool_input, tool_output, acceptance_test, fix_category, general_usefulness.",
            "Allowed fix_category values: tool, observation, prompt_contract, safety_override, harness_bug, simulator_bug.",
            "",
            "Failure packet:",
            compact_json(packet, limit=14000),
        ]
    )
    return system, user


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("run_jsonl", type=Path)
    parser.add_argument("--out", type=Path, default=None)
    parser.add_argument("--packet-out", type=Path, default=None)
    parser.add_argument("--provider", choices=["mock", "openai_compatible"], default="mock")
    parser.add_argument("--max-cards", type=int, default=5)
    parser.add_argument("--window", type=int, default=16)
    parser.add_argument("--base-url", default=os.environ.get("LLM_BASE_URL") or os.environ.get("OPENAI_BASE_URL") or "https://api.openai.com/v1")
    parser.add_argument("--model", default=os.environ.get("LLM_MODEL") or "gpt-4o-mini")
    parser.add_argument("--api-key", default=os.environ.get("LLM_API_KEY") or os.environ.get("OPENAI_API_KEY") or os.environ.get("DEEPSEEK_API_KEY") or "")
    parser.add_argument("--temperature", type=float, default=0.0)
    parser.add_argument("--timeout", type=float, default=120.0)
    return parser.parse_args()


def default_out_path(run_jsonl: Path) -> Path:
    return REPO_ROOT / "tools" / "artifacts" / "failure_questions" / f"{run_jsonl.stem}_tool_need_cards.jsonl"


def main() -> int:
    args = parse_args()
    events = read_jsonl(args.run_jsonl)
    packet = build_failure_packet(events, source_path=args.run_jsonl, window=args.window)
    if args.packet_out is not None:
        args.packet_out.parent.mkdir(parents=True, exist_ok=True)
        args.packet_out.write_text(json.dumps(packet, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    if args.provider == "mock":
        raw_cards = mock_cards(packet, max_cards=args.max_cards)
    else:
        if not args.api_key:
            raise SystemExit("--provider openai_compatible requires LLM_API_KEY, OPENAI_API_KEY, or DEEPSEEK_API_KEY")
        system, user = critic_prompt(packet, max_cards=args.max_cards)
        raw_text = call_openai_compatible(
            base_url=args.base_url,
            api_key=args.api_key,
            model=args.model,
            system=system,
            user=user,
            temperature=args.temperature,
            timeout=args.timeout,
        )
        raw_cards = extract_json_array(raw_text)

    cards = normalize_cards(raw_cards, packet, max_cards=args.max_cards)
    if not cards and args.provider != "mock":
        cards = normalize_cards(mock_cards(packet, max_cards=args.max_cards), packet, max_cards=args.max_cards)
    out = args.out or default_out_path(args.run_jsonl)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(
        "\n".join(json.dumps(card, ensure_ascii=False, separators=(",", ":")) for card in cards)
        + ("\n" if cards else ""),
        encoding="utf-8",
    )
    print(
        json.dumps(
            {
                "schema_name": "FailureInterrogationRun",
                "schema_version": 1,
                "source_run": str(args.run_jsonl),
                "failure_step": packet.get("failure_step"),
                "card_count": len(cards),
                "out": str(out),
                "packet_out": str(args.packet_out) if args.packet_out else None,
                "policy_quality_claim": False,
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
