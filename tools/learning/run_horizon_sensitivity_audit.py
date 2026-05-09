#!/usr/bin/env python3
"""Run a small horizon sensitivity audit for BranchTrace datasets.

This script is intentionally an audit wrapper, not a trainer. It runs
collect_branch_traces.py for several horizon caps, records wall time, and
summarizes how much complete combat-end data each cap buys.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
COLLECTOR = REPO_ROOT / "tools" / "learning" / "collect_branch_traces.py"


def default_driver_path() -> Path:
    suffix = ".exe" if sys.platform.startswith("win") else ""
    release = REPO_ROOT / "target" / "release" / f"full_run_env_driver{suffix}"
    debug = REPO_ROOT / "target" / "debug" / f"full_run_env_driver{suffix}"
    return release if release.exists() else debug


def parse_caps(value: str) -> list[int]:
    caps = []
    for item in value.split(","):
        item = item.strip()
        if not item:
            continue
        cap = int(item)
        if cap < 0:
            raise argparse.ArgumentTypeError("caps must be non-negative")
        caps.append(cap)
    if not caps:
        raise argparse.ArgumentTypeError("at least one cap is required")
    return caps


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def ratio(numerator: int, denominator: int) -> float:
    return numerator / denominator if denominator else 0.0


def compact_summary(summary: dict[str, Any], elapsed_sec: float, cap: int) -> dict[str, Any]:
    comparison_roles = summary.get("comparison_data_role_counts") or {}
    trace_roles = summary.get("trace_data_role_counts") or {}
    complete_pairs = int(
        comparison_roles.get("combat_end_complete_pair_rng_aligned", 0)
    ) + int(comparison_roles.get("combat_end_complete_pair_rng_diverged", 0))
    censored_pairs = int(comparison_roles.get("censored_partial_pair", 0))
    complete_branches = int(trace_roles.get("combat_end_complete_branch", 0))
    censored_branches = int(trace_roles.get("censored_partial_combat_branch", 0))
    comparison_count = int(summary.get("comparison_count") or 0)
    trace_count = int(summary.get("trace_count") or 0)
    return {
        "cap": cap,
        "elapsed_sec": elapsed_sec,
        "decision_count": int(summary.get("decision_count") or 0),
        "trace_count": trace_count,
        "comparison_count": comparison_count,
        "complete_combat_end_branches": complete_branches,
        "censored_partial_branches": censored_branches,
        "complete_combat_end_pairs": complete_pairs,
        "censored_partial_pairs": censored_pairs,
        "complete_combat_end_pair_ratio": ratio(complete_pairs, comparison_count),
        "censored_trace_ratio": ratio(censored_branches, trace_count),
        "rng_diverged_pair_ratio": float(summary.get("rng_diverged_pair_ratio") or 0.0),
        "nonzero_hp_diff_pairs": int(summary.get("comparison_nonzero_hp_diff_count") or 0),
        "nonzero_combat_win_diff_pairs": int(
            summary.get("comparison_nonzero_combat_win_diff_count") or 0
        ),
        "quality_gate": {
            "live_env_changed_count": int(summary.get("live_env_changed_count") or 0),
            "determinism_mismatch_count": int(
                summary.get("determinism_mismatch_count") or 0
            ),
            "validation_issue_count": int(summary.get("validation_issue_count") or 0),
            "redaction_violation_count": int(summary.get("redaction_violation_count") or 0),
            "trainable_action_label_count": int(
                summary.get("trainable_action_label_count") or 0
            ),
            "action_like_comparison_role_count": int(
                summary.get("action_like_comparison_role_count") or 0
            ),
            "winner_or_preference_field_count": int(
                summary.get("winner_or_preference_field_count") or 0
            ),
            "pairing_invalid_count": int(summary.get("pairing_invalid_count") or 0),
        },
        "comparison_data_role_counts": comparison_roles,
        "trace_data_role_counts": trace_roles,
        "comparison_hp_diff_histogram_by_role": summary.get(
            "comparison_hp_diff_histogram_by_role"
        )
        or {},
        "comparison_combat_win_diff_counts": summary.get(
            "comparison_combat_win_diff_counts"
        )
        or {},
    }


def run_cap(args: argparse.Namespace, cap: int) -> dict[str, Any]:
    out = args.output_dir / f"branch_trace_horizon_cap_{cap}.jsonl"
    summary_out = args.output_dir / f"branch_trace_horizon_cap_{cap}.summary.json"
    cmd = [
        sys.executable,
        str(COLLECTOR),
        "--driver",
        str(args.driver),
        "--out",
        str(out),
        "--summary-out",
        str(summary_out),
        "--episodes",
        str(args.episodes),
        "--seed-start",
        str(args.seed_start),
        "--seed-step",
        str(args.seed_step),
        "--ascension",
        str(args.ascension),
        "--max-steps",
        str(args.max_steps),
        "--env-max-steps",
        str(args.env_max_steps),
        "--max-candidates",
        str(args.max_candidates),
        "--horizon-decisions",
        str(cap),
        "--horizon-mode",
        args.horizon_mode,
        "--candidate-scope",
        args.candidate_scope,
        "--decision-type-prefixes",
        args.decision_type_prefixes,
        "--behavior-policy",
        args.behavior_policy,
        "--continuation-policy",
        args.continuation_policy,
        "--determinism-check-limit",
        str(args.determinism_check_limit),
    ]
    start = time.perf_counter()
    completed = subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        text=True,
        encoding="utf-8",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=args.timeout_per_cap_sec,
    )
    elapsed = time.perf_counter() - start
    if completed.returncode != 0:
        return {
            "cap": cap,
            "elapsed_sec": elapsed,
            "failed": True,
            "returncode": completed.returncode,
            "stderr_tail": completed.stderr[-4000:],
            "stdout_tail": completed.stdout[-4000:],
        }
    summary = load_json(summary_out)
    compact = compact_summary(summary, elapsed, cap)
    compact["failed"] = False
    compact["out"] = str(out)
    compact["summary_out"] = str(summary_out)
    return compact


def add_deltas(results: list[dict[str, Any]]) -> list[dict[str, Any]]:
    deltas = []
    previous: dict[str, Any] | None = None
    for current in results:
        if current.get("failed") or previous is None or previous.get("failed"):
            previous = current
            continue
        deltas.append(
            {
                "from_cap": previous["cap"],
                "to_cap": current["cap"],
                "elapsed_sec_delta": current["elapsed_sec"] - previous["elapsed_sec"],
                "complete_pair_delta": current["complete_combat_end_pairs"]
                - previous["complete_combat_end_pairs"],
                "complete_pair_ratio_delta": current["complete_combat_end_pair_ratio"]
                - previous["complete_combat_end_pair_ratio"],
                "censored_trace_ratio_delta": current["censored_trace_ratio"]
                - previous["censored_trace_ratio"],
                "nonzero_hp_diff_pair_delta": current["nonzero_hp_diff_pairs"]
                - previous["nonzero_hp_diff_pairs"],
                "nonzero_combat_win_diff_pair_delta": current[
                    "nonzero_combat_win_diff_pairs"
                ]
                - previous["nonzero_combat_win_diff_pairs"],
            }
        )
        previous = current
    return deltas


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver", type=Path, default=default_driver_path())
    parser.add_argument(
        "--caps",
        type=parse_caps,
        default=parse_caps("4,8,12"),
        help="Comma-separated horizon caps.",
    )
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path)
    parser.add_argument("--episodes", type=int, default=20)
    parser.add_argument("--seed-start", type=int, default=1)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--max-steps", type=int, default=30)
    parser.add_argument("--env-max-steps", type=int, default=200)
    parser.add_argument("--behavior-policy", default="rule_baseline_v0")
    parser.add_argument("--continuation-policy", default="rule_baseline_v0")
    parser.add_argument("--horizon-mode", default="combat_end_v1")
    parser.add_argument("--candidate-scope", default="controlled_v1")
    parser.add_argument("--max-candidates", type=int, default=3)
    parser.add_argument("--decision-type-prefixes", default="combat")
    parser.add_argument("--determinism-check-limit", type=int, default=20)
    parser.add_argument("--timeout-per-cap-sec", type=int, default=180)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.driver.exists():
        raise SystemExit(f"driver binary not found: {args.driver}")
    args.output_dir.mkdir(parents=True, exist_ok=True)
    summary_out = args.summary_out or args.output_dir / "horizon_sensitivity_summary.json"
    results = []
    for cap in args.caps:
        result = run_cap(args, cap)
        results.append(result)
        if result.get("failed"):
            print(json.dumps(result, indent=2), file=sys.stderr)
            break
    summary = {
        "schema_version": "horizon_sensitivity_audit_v0",
        "driver": str(args.driver),
        "caps": args.caps,
        "episodes": args.episodes,
        "seed_start": args.seed_start,
        "seed_step": args.seed_step,
        "max_steps": args.max_steps,
        "env_max_steps": args.env_max_steps,
        "horizon_mode": args.horizon_mode,
        "candidate_scope": args.candidate_scope,
        "max_candidates": args.max_candidates,
        "results": results,
        "deltas": add_deltas(results),
    }
    summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 1 if any(result.get("failed") for result in results) else 0


if __name__ == "__main__":
    raise SystemExit(main())
