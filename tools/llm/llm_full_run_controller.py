#!/usr/bin/env python3
"""LLM controller harness for the full-run DecisionEnv driver.

This script is intentionally a controller adapter, not a teacher-label or
training pipeline. It reads public observations and legal candidates from
`full_run_env_driver`, asks an LLM (or a mock policy) to choose one candidate id,
validates the choice against the current candidate set, then steps the driver.

Provider mode `openai_compatible` works with APIs that expose the common
`/chat/completions` shape. Configure it with:

  LLM_API_KEY      required
  LLM_BASE_URL     default: https://api.openai.com/v1
  LLM_MODEL        default: gpt-4o-mini

For DeepSeek-style endpoints, set:

  LLM_BASE_URL=https://api.deepseek.com
  LLM_MODEL=deepseek-chat

Use `--provider dry_run` to inspect the prompt without calling any model.
Use `--provider mock` for a local smoke test of the adapter loop.
"""

from __future__ import annotations

import argparse
import json
import os
import random
import re
import subprocess
import sys
import textwrap
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]


def default_driver_path() -> Path:
    suffix = ".exe" if sys.platform.startswith("win") else ""
    release = REPO_ROOT / "target" / "release" / f"full_run_env_driver{suffix}"
    debug = REPO_ROOT / "target" / "debug" / f"full_run_env_driver{suffix}"
    return release if release.exists() else debug


class DriverClient:
    def __init__(self, driver_path: Path) -> None:
        if not driver_path.exists():
            raise FileNotFoundError(f"driver binary not found: {driver_path}")
        self.proc = subprocess.Popen(
            [str(driver_path)],
            cwd=REPO_ROOT,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
        )

    def request(self, payload: dict[str, Any]) -> dict[str, Any]:
        assert self.proc.stdin is not None
        assert self.proc.stdout is not None
        self.proc.stdin.write(json.dumps(payload, separators=(",", ":")) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            stderr = self.proc.stderr.read() if self.proc.stderr else ""
            raise RuntimeError(f"driver closed stdout; stderr={stderr}")
        response = json.loads(line)
        if not response.get("ok"):
            raise RuntimeError(response.get("error") or f"driver request failed: {payload}")
        return response

    def close(self) -> None:
        if self.proc.poll() is None:
            try:
                self.request({"cmd": "close"})
            except Exception:
                pass
        if self.proc.poll() is None:
            self.proc.terminate()


def compact_json(value: Any, *, limit: int = 900) -> str:
    text = json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    if len(text) <= limit:
        return text
    return text[: limit - 3] + "..."


def observation_summary(observation: dict[str, Any]) -> list[str]:
    payload = observation.get("payload") or observation
    lines = [
        f"decision_type={payload.get('decision_type')} engine_state={payload.get('engine_state')}",
        (
            f"act={payload.get('act')} floor={payload.get('floor')} "
            f"hp={payload.get('current_hp')}/{payload.get('max_hp')} "
            f"gold={payload.get('gold')} deck_size={payload.get('deck_size')} "
            f"boss={payload.get('act_boss')}"
        ),
    ]
    deck = payload.get("deck") or {}
    if deck:
        lines.append(
            "deck="
            + ", ".join(
                f"{key}:{deck.get(key)}"
                for key in [
                    "attack_count",
                    "skill_count",
                    "power_count",
                    "draw_card_count",
                    "scaling_card_count",
                    "starter_basic_count",
                    "average_cost_milli",
                ]
            )
        )
    combat = payload.get("combat")
    if combat:
        lines.append(
            (
                "combat="
                f"hp:{combat.get('player_hp')} block:{combat.get('player_block')} "
                f"energy:{combat.get('energy')} turn:{combat.get('turn_count')} "
                f"incoming:{combat.get('visible_incoming_damage')} "
                f"monster_hp_total:{combat.get('total_monster_hp')} "
                f"alive_monsters:{combat.get('alive_monster_count')} "
                f"draw:{combat.get('draw_count')} discard:{combat.get('discard_count')}"
            )
        )
        hand = combat.get("hand_cards") or []
        if hand:
            lines.append("hand:")
            for card in hand[:12]:
                tags = ",".join(card.get("base_semantics") or [])
                lines.append(
                    "  "
                    f"h{card.get('hand_index')} {card.get('card_id')}"
                    f"{'+' if card.get('upgraded') else ''} "
                    f"cost={card.get('cost_for_turn')} playable={card.get('playable')} "
                    f"tags={tags}"
                )
    screen = payload.get("screen") or {}
    if screen:
        active_counts = {
            key: value
            for key, value in screen.items()
            if isinstance(value, int) and value > 0
        }
        if active_counts:
            lines.append(f"screen_counts={active_counts}")
    next_nodes = payload.get("next_nodes") or []
    if next_nodes:
        lines.append("next_nodes:")
        for node in next_nodes[:8]:
            lines.append(
                "  "
                f"x={node.get('x')} y={node.get('y')} "
                f"room={node.get('room_type')} emerald={node.get('has_emerald_key')}"
            )
    rewards = screen.get("reward_items") or []
    if rewards:
        lines.append("reward_items:")
        for item in rewards[:8]:
            lines.append("  " + compact_json(item, limit=300))
    return lines


def candidate_lines(candidates: list[dict[str, Any]]) -> list[str]:
    lines = []
    for candidate in candidates:
        payload = candidate.get("payload") or candidate
        card = payload.get("card")
        card_text = ""
        if isinstance(card, dict):
            card_text = (
                f" card={card.get('card_id')} cost={card.get('cost')} "
                f"damage={card.get('base_damage')} block={card.get('base_block')} "
                f"magic={card.get('base_magic')}"
            )
        lines.append(
            f"- id={candidate.get('id')} key={candidate.get('action_key')}"
            f"{card_text} action={compact_json(payload.get('action'), limit=450)}"
        )
    return lines


def build_prompt(timestep: dict[str, Any], *, max_candidates: int) -> tuple[str, str]:
    candidates = (timestep.get("candidates") or [])[:max_candidates]
    system = (
        "You are a cautious controller for a Slay the Spire simulator. "
        "Use only the public observation and the legal candidate list. "
        "Do not invent hidden information. Return exactly one JSON object with "
        'keys: "action_id" (integer), "confidence" ("low"|"medium"|"high"), '
        'and "reason" (short string).'
    )
    user = "\n".join(
        [
            "Choose the next legal action.",
            "",
            "Public state:",
            *observation_summary(timestep.get("observation") or {}),
            "",
            "Legal candidates:",
            *candidate_lines(candidates),
            "",
            "Return strict JSON only. The action_id must be one of the listed ids.",
        ]
    )
    return system, user


def extract_json_object(text: str) -> dict[str, Any]:
    stripped = text.strip()
    if stripped.startswith("```"):
        stripped = re.sub(r"^```(?:json)?\s*", "", stripped)
        stripped = re.sub(r"\s*```$", "", stripped)
    try:
        value = json.loads(stripped)
    except json.JSONDecodeError:
        match = re.search(r"\{.*\}", stripped, flags=re.DOTALL)
        if not match:
            raise
        value = json.loads(match.group(0))
    if not isinstance(value, dict):
        raise ValueError("LLM response JSON was not an object")
    return value


def mock_choice(timestep: dict[str, Any], rng: random.Random) -> dict[str, Any]:
    candidates = timestep.get("candidates") or []
    if not candidates:
        raise RuntimeError("no candidates")
    observation = (timestep.get("observation") or {}).get("payload") or {}
    decision_type = observation.get("decision_type")
    chosen = candidates[0]
    if decision_type == "combat":
        non_end = [
            c
            for c in candidates
            if "combat/end_turn" not in str(c.get("action_key"))
        ]
        if non_end:
            chosen = non_end[0]
    elif decision_type == "map":
        chosen = candidates[rng.randrange(len(candidates))]
    return {
        "action_id": chosen["id"],
        "confidence": "low",
        "reason": "mock provider smoke-test choice; not a strategy label",
    }


def call_openai_compatible(
    *,
    base_url: str,
    api_key: str,
    model: str,
    system: str,
    user: str,
    temperature: float,
    timeout: int,
) -> tuple[str, dict[str, Any]]:
    url = base_url.rstrip("/") + "/chat/completions"
    payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "temperature": temperature,
    }
    req = urllib.request.Request(
        url,
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as response:
            raw = response.read().decode("utf-8")
    except urllib.error.HTTPError as err:
        body = err.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"LLM HTTP {err.code}: {body}") from err
    parsed = json.loads(raw)
    content = parsed["choices"][0]["message"]["content"]
    return content, parsed


def validate_action_id(action_id: Any, candidates: list[dict[str, Any]]) -> tuple[int, bool, str]:
    legal_ids = [int(candidate["id"]) for candidate in candidates]
    try:
        parsed = int(action_id)
    except (TypeError, ValueError):
        return legal_ids[0], False, f"non-integer action_id {action_id!r}; fell back to {legal_ids[0]}"
    if parsed in legal_ids:
        return parsed, True, "legal"
    return legal_ids[0], False, f"illegal action_id {parsed}; fell back to {legal_ids[0]}"


def run_controller(args: argparse.Namespace) -> dict[str, Any]:
    rng = random.Random(args.seed ^ 0xC0DEC0DE)
    client = DriverClient(args.driver)
    records: list[dict[str, Any]] = []
    try:
        client.request(
            {
                "cmd": "reset",
                "seed": args.seed,
                "ascension": args.ascension,
                "final_act": args.final_act,
                "class": args.player_class,
                "max_steps": args.max_steps,
            }
        )
        done = False
        for step_index in range(args.steps):
            timestep = client.request({"cmd": "decision_env_observation"})["payload"]
            candidates = timestep.get("candidates") or []
            if not candidates:
                raise RuntimeError("driver returned no legal candidates")
            system, user = build_prompt(timestep, max_candidates=args.max_candidates)
            if args.provider == "dry_run":
                records.append(
                    {
                        "step_index": step_index,
                        "provider": "dry_run",
                        "system": system,
                        "prompt": user,
                        "candidate_count": len(candidates),
                    }
                )
                break
            if args.provider == "mock":
                parsed = mock_choice(timestep, rng)
                raw_text = json.dumps(parsed, ensure_ascii=False)
                raw_payload = None
            else:
                raw_text, raw_payload = call_openai_compatible(
                    base_url=args.base_url,
                    api_key=args.api_key,
                    model=args.model,
                    system=system,
                    user=user,
                    temperature=args.temperature,
                    timeout=args.timeout,
                )
                parsed = extract_json_object(raw_text)
            action_id, legal, validation = validate_action_id(parsed.get("action_id"), candidates)
            step = client.request({"cmd": "decision_env_step", "action_id": action_id})
            done = bool(step.get("done"))
            records.append(
                {
                    "step_index": step_index,
                    "provider": args.provider,
                    "model": args.model if args.provider == "openai_compatible" else None,
                    "decision_type": (timestep.get("observation") or {}).get("decision_type"),
                    "candidate_count": len(candidates),
                    "llm_raw_text": raw_text,
                    "llm_raw_payload": raw_payload if args.include_raw_llm_payload else None,
                    "llm_choice": parsed,
                    "selected_action_id": action_id,
                    "selected_action_key": next(
                        (
                            c.get("action_key")
                            for c in candidates
                            if int(c.get("id")) == action_id
                        ),
                        None,
                    ),
                    "choice_was_legal": legal,
                    "validation": validation,
                    "reward": step.get("reward"),
                    "done": done,
                    "info": step.get("info"),
                }
            )
            if done:
                break
            time.sleep(args.sleep)
    finally:
        client.close()
    return {
        "schema_version": "llm_full_run_controller_demo_v1",
        "provider": args.provider,
        "seed": args.seed,
        "ascension": args.ascension,
        "class": args.player_class,
        "records": records,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver", type=Path, default=default_driver_path())
    parser.add_argument("--provider", choices=["dry_run", "mock", "openai_compatible"], default="dry_run")
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--max-steps", type=int, default=500)
    parser.add_argument("--steps", type=int, default=1)
    parser.add_argument("--max-candidates", type=int, default=24)
    parser.add_argument("--out", type=Path, default=REPO_ROOT / "tools" / "artifacts" / "llm_demo" / "run.json")
    parser.add_argument("--temperature", type=float, default=0.0)
    parser.add_argument("--timeout", type=int, default=60)
    parser.add_argument("--sleep", type=float, default=0.0)
    parser.add_argument("--include-raw-llm-payload", action="store_true")
    parser.add_argument("--base-url", default=os.environ.get("LLM_BASE_URL") or os.environ.get("OPENAI_BASE_URL") or "https://api.openai.com/v1")
    parser.add_argument("--model", default=os.environ.get("LLM_MODEL") or "gpt-4o-mini")
    parser.add_argument("--api-key", default=os.environ.get("LLM_API_KEY") or os.environ.get("OPENAI_API_KEY") or os.environ.get("DEEPSEEK_API_KEY") or "")
    args = parser.parse_args()
    if args.provider == "openai_compatible" and not args.api_key:
        parser.error("--provider openai_compatible requires LLM_API_KEY, OPENAI_API_KEY, or DEEPSEEK_API_KEY")
    return args


def main() -> int:
    args = parse_args()
    report = run_controller(args)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2))
    if args.provider == "dry_run":
        prompt = report["records"][0]["prompt"] if report["records"] else ""
        print("\n--- prompt preview ---\n")
        print(textwrap.shorten(prompt.replace("\n", " | "), width=1200, placeholder=" ..."))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
