#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json, write_jsonl


DEFAULT_DECISION_TYPES = "reward_card_choice,map,campfire,boss_relic"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run full-run counterfactual lab over a small batch of trace decision points."
    )
    parser.add_argument("--trace-dir", type=Path, required=True)
    parser.add_argument("--decision-types", default=DEFAULT_DECISION_TYPES)
    parser.add_argument("--max-cases", type=int, default=10)
    parser.add_argument("--per-trace-limit", type=int, default=5)
    parser.add_argument("--min-candidates", type=int, default=2)
    parser.add_argument("--continuation-policy", default="rule_baseline_v0", choices=["rule_baseline_v0", "random_masked"])
    parser.add_argument("--continuation-steps", type=int, default=40)
    parser.add_argument("--max-branches", type=int, default=8)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=5000)
    parser.add_argument("--driver-binary", type=Path)
    parser.add_argument("--allow-replay-mismatch", action="store_true")
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "full_run_counterfactual_lab" / "batch",
    )
    return parser.parse_args()


def load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def trace_files(trace_dir: Path) -> list[Path]:
    files = sorted(trace_dir.glob("episode_*.json"))
    if not files:
        raise SystemExit(f"no episode_*.json files found in {trace_dir}")
    return files


def parse_decision_types(text: str) -> set[str]:
    values = {part.strip() for part in text.split(",") if part.strip()}
    if not values:
        raise SystemExit("expected at least one decision type")
    return values


def select_cases(args: argparse.Namespace) -> list[dict[str, Any]]:
    allowed = parse_decision_types(args.decision_types)
    cases: list[dict[str, Any]] = []
    for path in trace_files(args.trace_dir):
        trace = load_json(path)
        per_trace = 0
        for step in trace.get("steps") or []:
            decision_type = str(step.get("decision_type") or "")
            candidates = step.get("action_mask") or []
            if decision_type not in allowed or len(candidates) < args.min_candidates:
                continue
            cases.append(
                {
                    "case_id": f"{path.stem}_step_{int(step.get('step_index') or 0):04}",
                    "trace_file": path,
                    "episode_id": int((trace.get("summary") or {}).get("episode_id") or 0),
                    "seed": int((trace.get("summary") or {}).get("seed") or 0),
                    "step_index": int(step.get("step_index") or 0),
                    "decision_type": decision_type,
                    "floor": int(step.get("floor") or 0),
                    "act": int(step.get("act") or 0),
                    "candidate_count": len(candidates),
                    "chosen_action_index": int(step.get("chosen_action_index") or 0),
                    "chosen_action_key": str(step.get("chosen_action_key") or ""),
                }
            )
            per_trace += 1
            if per_trace >= args.per_trace_limit or len(cases) >= args.max_cases:
                break
        if len(cases) >= args.max_cases:
            break
    return cases


def run_case(args: argparse.Namespace, case: dict[str, Any], case_dir: Path) -> dict[str, Any]:
    report_path = case_dir / f"{case['case_id']}.json"
    cmd = [
        sys.executable,
        str(REPO_ROOT / "tools" / "learning" / "full_run_counterfactual_lab.py"),
        "--trace-file",
        str(case["trace_file"]),
        "--step-index",
        str(case["step_index"]),
        "--continuation-policy",
        args.continuation_policy,
        "--continuation-steps",
        str(args.continuation_steps),
        "--max-branches",
        str(args.max_branches),
        "--ascension",
        str(args.ascension),
        "--class",
        args.player_class,
        "--max-steps",
        str(args.max_steps),
        "--out",
        str(report_path),
    ]
    if args.final_act:
        cmd.append("--final-act")
    if args.driver_binary:
        cmd.extend(["--driver-binary", str(args.driver_binary)])
    if args.allow_replay_mismatch:
        cmd.append("--allow-replay-mismatch")
    proc = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    if proc.returncode != 0:
        return {
            "case": case,
            "status": "failed",
            "error": proc.stderr.strip() or proc.stdout.strip(),
            "report_path": str(report_path),
        }
    report = load_json(report_path)
    return {
        "case": case,
        "status": "ok",
        "report_path": str(report_path),
        "report": report,
    }


def outcome_sort_key(row: dict[str, Any]) -> tuple[int, int, int, int, float]:
    end = row.get("end") or {}
    delta = row.get("outcome_delta") or {}
    result = str(end.get("result") or "")
    terminal_reason = str(end.get("terminal_reason") or "")
    alive = 1 if result == "ongoing" or terminal_reason == "running" else 0
    return (
        alive,
        int(delta.get("floor_delta") or 0),
        int(delta.get("combat_win_delta") or 0),
        int(end.get("current_hp") or 0),
        float(row.get("reward_total") or 0.0),
    )


def equivalence_signature(row: dict[str, Any]) -> str:
    end = row.get("end") or {}
    delta = row.get("outcome_delta") or {}
    hp_bucket = int(end.get("current_hp") or 0) // 5
    return "|".join(
        [
            str(end.get("result") or "unknown"),
            str(end.get("terminal_reason") or "unknown"),
            f"floor:{int(delta.get('floor_delta') or 0)}",
            f"combat:{int(delta.get('combat_win_delta') or 0)}",
            f"hp5:{hp_bucket}",
        ]
    )


def flatten_case(result: dict[str, Any]) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    case = result["case"]
    if result["status"] != "ok":
        return [], {
            **public_case(case),
            "status": result["status"],
            "error": result.get("error"),
            "report_path": result.get("report_path"),
        }
    report = result["report"]
    outcomes = list(report.get("outcomes") or [])
    ranked = sorted(outcomes, key=outcome_sort_key, reverse=True)
    rank_by_index = {int(row["candidate_index"]): rank for rank, row in enumerate(ranked, start=1)}
    groups: dict[str, list[int]] = defaultdict(list)
    for row in outcomes:
        groups[equivalence_signature(row)].append(int(row["candidate_index"]))
    chosen_index = int(case["chosen_action_index"])
    chosen_rank = rank_by_index.get(chosen_index)
    best = ranked[0] if ranked else None
    case_summary = {
        **public_case(case),
        "status": "ok",
        "report_path": result.get("report_path"),
        "outcome_count": len(outcomes),
        "chosen_rank": chosen_rank,
        "chosen_was_best": chosen_rank == 1 if chosen_rank is not None else False,
        "best_candidate_index": None if best is None else int(best["candidate_index"]),
        "best_candidate_key": None if best is None else str(best.get("candidate_key") or ""),
        "equivalence_groups": [
            {"signature": signature, "candidate_indices": indices}
            for signature, indices in sorted(groups.items())
        ],
    }
    rows = []
    for row in outcomes:
        card = row.get("candidate_card") or {}
        rows.append(
            {
                **public_case(case),
                "candidate_index": int(row.get("candidate_index") or 0),
                "candidate_key": row.get("candidate_key"),
                "candidate_card_id": card.get("card_id") if isinstance(card, dict) else None,
                "rank": rank_by_index.get(int(row.get("candidate_index") or 0)),
                "equivalence_signature": equivalence_signature(row),
                "is_chosen": int(row.get("candidate_index") or 0) == chosen_index,
                "is_best": rank_by_index.get(int(row.get("candidate_index") or 0)) == 1,
                "floor_delta": (row.get("outcome_delta") or {}).get("floor_delta"),
                "hp_delta": (row.get("outcome_delta") or {}).get("hp_delta"),
                "combat_win_delta": (row.get("outcome_delta") or {}).get("combat_win_delta"),
                "end_floor": (row.get("end") or {}).get("floor"),
                "end_hp": (row.get("end") or {}).get("current_hp"),
                "end_result": (row.get("end") or {}).get("result"),
                "terminal_reason": (row.get("end") or {}).get("terminal_reason"),
                "reward_total": row.get("reward_total"),
                "steps_taken": row.get("steps_taken"),
            }
        )
    return rows, case_summary


def public_case(case: dict[str, Any]) -> dict[str, Any]:
    return {
        key: str(value) if isinstance(value, Path) else value
        for key, value in case.items()
        if key != "trace_file"
    } | {"trace_file": str(case["trace_file"])}


def main() -> None:
    args = parse_args()
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out_dir = args.out_dir / stamp
    reports_dir = out_dir / "case_reports"
    reports_dir.mkdir(parents=True, exist_ok=True)
    cases = select_cases(args)
    if not cases:
        raise SystemExit("no candidate decision cases selected")

    started = time.perf_counter()
    results = [run_case(args, case, reports_dir) for case in cases]
    elapsed = time.perf_counter() - started

    all_rows: list[dict[str, Any]] = []
    case_summaries: list[dict[str, Any]] = []
    for result in results:
        rows, case_summary = flatten_case(result)
        all_rows.extend(rows)
        case_summaries.append(case_summary)

    summary = {
        "schema_version": "full_run_counterfactual_batch_v0",
        "created_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {
            "trace_dir": str(args.trace_dir),
            "decision_types": sorted(parse_decision_types(args.decision_types)),
            "max_cases": args.max_cases,
            "per_trace_limit": args.per_trace_limit,
            "min_candidates": args.min_candidates,
            "continuation_policy": args.continuation_policy,
            "continuation_steps": args.continuation_steps,
            "max_branches": args.max_branches,
        },
        "counts": {
            "selected_cases": len(cases),
            "ok_cases": sum(1 for result in results if result["status"] == "ok"),
            "failed_cases": sum(1 for result in results if result["status"] != "ok"),
            "outcome_rows": len(all_rows),
        },
        "decision_type_counts": dict(sorted(Counter(case["decision_type"] for case in cases).items())),
        "chosen_rank_counts": dict(
            sorted(Counter(str(case.get("chosen_rank")) for case in case_summaries if case.get("status") == "ok").items())
        ),
        "chosen_best_rate": chosen_best_rate(case_summaries),
        "elapsed_seconds": elapsed,
        "case_summaries": case_summaries,
    }
    write_json(out_dir / "summary.json", summary)
    write_jsonl(out_dir / "outcome_rows.jsonl", all_rows)
    write_jsonl(out_dir / "case_summaries.jsonl", case_summaries)
    print(json.dumps(summary["counts"] | {"chosen_best_rate": summary["chosen_best_rate"]}, indent=2, ensure_ascii=False))
    print(f"wrote {out_dir}")


def chosen_best_rate(case_summaries: list[dict[str, Any]]) -> float:
    ok = [case for case in case_summaries if case.get("status") == "ok" and case.get("chosen_rank") is not None]
    if not ok:
        return 0.0
    return sum(1 for case in ok if case.get("chosen_was_best")) / len(ok)


if __name__ == "__main__":
    main()
