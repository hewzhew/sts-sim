#!/usr/bin/env python3
"""Run a tiny harness usability suite and score the result.

This is not a policy-strength evaluation. It measures whether the current
agent harness is reliable, inspectable, low-noise, and useful enough to guide
the next engineering iteration.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from collections import Counter
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
CONTROLLER = REPO_ROOT / "tools" / "llm" / "llm_full_run_controller.py"
SUMMARIZER = REPO_ROOT / "tools" / "llm" / "summarize_tool_needs.py"


def parse_seed_list(values: list[str]) -> list[int]:
    seeds: list[int] = []
    for value in values:
        for part in value.split(","):
            part = part.strip()
            if part:
                seeds.append(int(part))
    return seeds


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    if not path.exists():
        return events
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        value = json.loads(line)
        if isinstance(value, dict):
            events.append(value)
    return events


def event_counter(events: list[dict[str, Any]]) -> Counter[str]:
    return Counter(str(event.get("event_type") or event.get("schema_name") or "unknown") for event in events)


def max_floor(events: list[dict[str, Any]]) -> int:
    floor = 0
    for event in events:
        value = event.get("floor")
        if isinstance(value, int):
            floor = max(floor, value)
    return floor


def illegal_action_count(events: list[dict[str, Any]]) -> int:
    return sum(
        1
        for event in events
        if event.get("schema_name") == "VerifierDecisionEvent" and event.get("legal") is False
    )


def tool_count(events: list[dict[str, Any]], tool_name: str) -> int:
    return sum(
        1
        for event in events
        if event.get("schema_name") == "ToolResultEvent" and event.get("tool") == tool_name
    )


def count_tool_design_questions(path: Path) -> int:
    return len(read_jsonl(path))


def score_run(
    *,
    returncode: int,
    elapsed_s: float,
    journal_path: Path,
    tool_design_path: Path,
    steps: int,
    question_limit: int,
) -> dict[str, Any]:
    parse_error = None
    try:
        events = read_jsonl(journal_path)
        question_events = read_jsonl(tool_design_path)
    except Exception as err:  # noqa: BLE001 - report parse failure in score.
        events = []
        question_events = []
        parse_error = str(err)

    journal_bytes = journal_path.stat().st_size if journal_path.exists() else 0
    question_bytes = tool_design_path.stat().st_size if tool_design_path.exists() else 0
    illegal = illegal_action_count(events)
    floor = max_floor(events)
    counters = event_counter(events)
    tool_design_count = len(question_events)
    tool_result_events = counters.get("tool_result", 0)
    search_verifier_events = sum(
        1
        for event in events
        if event.get("schema_name") == "VerifierDecisionEvent"
        and event.get("final_authority") == "search_verifier"
    )
    has_tool_design = tool_design_count > 0
    combat_lab_count = tool_count(events, "combat_multi_turn_lab")
    noise_limit = max(80_000, steps * 1_600)
    speed_limit = max(20.0, steps * 0.35)

    mandatory_failures = []
    if returncode != 0:
        mandatory_failures.append("controller_returncode_nonzero")
    if parse_error:
        mandatory_failures.append("journal_parse_error")
    if not events:
        mandatory_failures.append("empty_journal")
    if illegal:
        mandatory_failures.append("illegal_actions")
    if journal_bytes > noise_limit:
        mandatory_failures.append("journal_too_large")
    if tool_design_count > question_limit:
        mandatory_failures.append("tool_design_too_noisy")
    if combat_lab_count == 0:
        mandatory_failures.append("missing_combat_multi_turn_lab_signal")

    score = 0
    score += 35 if returncode == 0 and not parse_error and events else 0
    score += 20 if illegal == 0 and events else 0
    score += min(15, floor * 2)
    score += 8 if tool_result_events > 0 else 0
    score += 5 if search_verifier_events > 0 else 0
    score += 7 if combat_lab_count > 0 else 0
    score += 5 if has_tool_design else 0
    score += 5 if journal_bytes <= noise_limit else 0
    score += 5 if tool_design_count <= question_limit else 0
    score += 5 if elapsed_s <= speed_limit else 0
    score = min(score, 100)
    passed = not mandatory_failures and score >= 85

    return {
        "score": score,
        "passed": passed,
        "mandatory_failures": mandatory_failures,
        "returncode": returncode,
        "elapsed_s": round(elapsed_s, 3),
        "max_floor": floor,
        "illegal_actions": illegal,
        "journal_bytes": journal_bytes,
        "journal_noise_limit": noise_limit,
        "tool_design_questions": tool_design_count,
        "tool_design_question_limit": question_limit,
        "tool_design_bytes": question_bytes,
        "event_counts": dict(counters),
        "tool_counts": {
            "combat_turn_probe": tool_count(events, "combat_turn_probe"),
            "combat_multi_turn_lab": tool_count(events, "combat_multi_turn_lab"),
            "decision_lab": tool_count(events, "decision_lab"),
            "reward_card_eval": tool_count(events, "reward_card_eval"),
            "map_route_eval": tool_count(events, "map_route_eval"),
            "campfire_eval": tool_count(events, "campfire_eval"),
        },
        "journal_path": str(journal_path),
        "tool_design_path": str(tool_design_path),
    }


def run_seed(args: argparse.Namespace, seed: int, out_dir: Path) -> dict[str, Any]:
    journal_path = out_dir / f"seed_{seed}.jsonl"
    tool_design_path = out_dir / f"seed_{seed}_questions.jsonl"
    command = [
        sys.executable,
        str(CONTROLLER),
        "--provider",
        args.provider,
        "--agent-mode",
        "planner",
        "--combat-decision-owner",
        args.combat_decision_owner,
        "--tool-policy",
        args.tool_policy,
        "--tool-design-mode",
        "observe",
        "--trace-level",
        "compact",
        "--journal-format",
        "events",
        "--steps",
        str(args.steps),
        "--max-steps",
        str(args.max_steps),
        "--seed",
        str(seed),
        "--out",
        str(journal_path),
        "--tool-design-out",
        str(tool_design_path),
        "--tool-design-max-events",
        str(args.tool_design_max_events),
        "--tool-design-max-questions",
        str(args.tool_design_max_questions),
        "--timeout",
        str(args.timeout),
        "--planner-timeout",
        str(args.planner_timeout),
    ]
    if args.provider == "openai_compatible" and args.model:
        command.extend(["--model", args.model])
    if args.combat_lab_with_search:
        command.append("--combat-lab-with-search")
    start = time.perf_counter()
    proc = subprocess.run(
        command,
        cwd=REPO_ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=args.process_timeout,
    )
    elapsed = time.perf_counter() - start
    result = score_run(
        returncode=proc.returncode,
        elapsed_s=elapsed,
        journal_path=journal_path,
        tool_design_path=tool_design_path,
        steps=args.steps,
        question_limit=args.tool_design_max_events,
    )
    result["seed"] = seed
    result["stdout_tail"] = proc.stdout[-2000:]
    result["stderr_tail"] = proc.stderr[-2000:]
    return result


def summarize_tool_design(out_dir: Path, question_paths: list[Path]) -> str | None:
    existing = [path for path in question_paths if path.exists() and path.stat().st_size > 0]
    if not existing:
        return None
    summary_path = out_dir / "tool_needs_summary.json"
    command = [
        sys.executable,
        str(SUMMARIZER),
        *[str(path) for path in existing],
        "--out",
        str(summary_path),
    ]
    subprocess.run(command, cwd=REPO_ROOT, check=False)
    return str(summary_path) if summary_path.exists() else None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--provider", choices=["mock", "openai_compatible"], default="mock")
    parser.add_argument("--combat-decision-owner", choices=["llm", "search"], default="search")
    parser.add_argument("--tool-policy", choices=["always", "risk_gated", "none"], default="risk_gated")
    parser.add_argument("--seeds", nargs="+", default=["42", "43", "44"])
    parser.add_argument("--steps", type=int, default=160)
    parser.add_argument("--max-steps", type=int, default=800)
    parser.add_argument("--timeout", type=int, default=120)
    parser.add_argument("--planner-timeout", type=int, default=120)
    parser.add_argument("--process-timeout", type=int, default=240)
    parser.add_argument("--tool-design-max-events", type=int, default=12)
    parser.add_argument("--tool-design-max-questions", type=int, default=1)
    parser.add_argument("--combat-lab-with-search", action="store_true", default=True)
    parser.add_argument("--model", default=None)
    parser.add_argument("--out-dir", type=Path, default=None)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    seeds = parse_seed_list(args.seeds)
    eval_id = f"hug_v0_{args.provider}_{args.combat_decision_owner}_{int(time.time())}"
    out_dir = args.out_dir or (REPO_ROOT / "tools" / "artifacts" / "evals" / eval_id)
    out_dir.mkdir(parents=True, exist_ok=True)

    runs = [run_seed(args, seed, out_dir) for seed in seeds]
    question_paths = [out_dir / f"seed_{seed}_questions.jsonl" for seed in seeds]
    tool_needs_summary = summarize_tool_design(out_dir, question_paths)
    average_score = round(sum(run["score"] for run in runs) / max(1, len(runs)), 2)
    suite_passed = all(run["passed"] for run in runs) and average_score >= 85
    summary = {
        "schema_name": "HarnessUsabilityGate",
        "schema_version": 1,
        "metric_name": "HUG_v1",
        "provider": args.provider,
        "combat_decision_owner": args.combat_decision_owner,
        "tool_policy": args.tool_policy,
        "seeds": seeds,
        "steps": args.steps,
        "suite_passed": suite_passed,
        "average_score": average_score,
        "pass_threshold": 85,
        "runs": runs,
        "tool_needs_summary": tool_needs_summary,
        "claim_level": "harness_usability_smoke_only",
        "policy_quality_claim": False,
        "label_role": "not_a_label",
    }
    summary_path = out_dir / "summary.json"
    summary_path.write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(summary, ensure_ascii=False, indent=2))
    return 0 if suite_passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
