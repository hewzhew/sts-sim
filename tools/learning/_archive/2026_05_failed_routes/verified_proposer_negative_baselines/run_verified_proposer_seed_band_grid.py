#!/usr/bin/env python3
"""Run full-H8 and model-proposer H8 across seed bands."""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

from return_q_common import write_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("target/release/full_run_env_driver.exe"))
    parser.add_argument("--runner", type=Path, default=Path("tools/learning/eval_verified_adv_override_rust_runner.py"))
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--out-dir", type=Path, required=True)
    parser.add_argument("--episodes", type=int, default=100)
    parser.add_argument("--seed-starts", default="98100,98200,98300")
    parser.add_argument("--thresholds", default="0.3,0.5,0.7")
    parser.add_argument("--top-k", type=int, default=1)
    parser.add_argument("--model-path", type=Path, required=True)
    parser.add_argument("--max-steps", type=int, default=160)
    parser.add_argument("--parallelism", type=int, default=0)
    parser.add_argument("--horizon-decisions", type=int, default=8)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)
    seed_starts = parse_int_list(args.seed_starts)
    thresholds = parse_float_list(args.thresholds)
    rows = []
    for seed_start in seed_starts:
        rows.append(run_case(args, seed_start, "full", None))
        for threshold in thresholds:
            rows.append(run_case(args, seed_start, f"proposer_top{args.top_k}_thr{threshold:g}", threshold))
    summary = {
        "schema_version": "verified_proposer_seed_band_grid_v0",
        "config": {
            "episodes": args.episodes,
            "seed_starts": seed_starts,
            "thresholds": thresholds,
            "top_k": args.top_k,
            "model_path": str(args.model_path),
            "max_steps": args.max_steps,
            "parallelism": args.parallelism,
            "horizon_decisions": args.horizon_decisions,
        },
        "rows": rows,
    }
    write_json(args.out, summary)
    print(json.dumps(render_compact(summary), indent=2, sort_keys=True))


def run_case(args: argparse.Namespace, seed_start: int, case_name: str, threshold: float | None) -> dict[str, Any]:
    out = args.out_dir / f"verified_{case_name}_seed{seed_start}_n{args.episodes}.json"
    cmd = [
        sys.executable,
        str(args.runner),
        "--binary",
        str(args.binary),
        "--out",
        str(out),
        "--episodes",
        str(args.episodes),
        "--seed-start",
        str(seed_start),
        "--max-steps",
        str(args.max_steps),
        "--candidate-scope",
        "controlled_v1",
        "--horizon-decisions",
        str(args.horizon_decisions),
        "--horizon-mode",
        "fixed_decisions",
        "--oracle-margin",
        "1.0",
        "--oracle-continuation-policy",
        "rule_baseline_v0",
        "--verified-evaluation-mode",
        "independent",
        "--verified-parallelism",
        str(args.parallelism),
        "--no-verified-exact-root-dedup",
        "--summary-only",
    ]
    if threshold is None:
        cmd.extend(["--verifier-strategy", "single_stage"])
    else:
        cmd.extend(
            [
                "--verifier-strategy",
                "model_proposer_v1",
                "--proposer-model-path",
                str(args.model_path),
                "--proposer-top-k",
                str(args.top_k),
                "--proposer-threshold",
                str(threshold),
            ]
        )
    started = time.perf_counter()
    completed = subprocess.run(
        cmd,
        cwd=Path.cwd(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    wall = time.perf_counter() - started
    stdout_path = out.with_suffix(".stdout.txt")
    stdout_path.write_text(completed.stdout, encoding="utf-8")
    stderr_path = out.with_suffix(".stderr.txt")
    stderr_path.write_text(completed.stderr, encoding="utf-8")
    if completed.returncode != 0:
        raise SystemExit(
            f"case failed seed_start={seed_start} case={case_name} code={completed.returncode}\n{completed.stderr}"
        )
    payload = json.loads(out.read_text(encoding="utf-8"))
    policy = next(iter((payload.get("policy_summary") or {}).values()))
    return {
        "seed_start": seed_start,
        "case": case_name,
        "threshold": threshold,
        "out": str(out),
        "wall_seconds": wall,
        "average_total_reward": policy.get("average_total_reward"),
        "reward_stderr": policy.get("reward_stderr"),
        "result_counts": policy.get("result_counts"),
        "average_combat_win_count": policy.get("average_combat_win_count"),
        "average_final_floor": policy.get("average_final_floor"),
        "verified_override_count": policy.get("verified_override_count"),
        "verified_candidate_evaluation_count": policy.get("verified_candidate_evaluation_count"),
        "verified_cached_policy_step_eval_count": policy.get("verified_cached_policy_step_eval_count"),
        "verified_candidate_eval_wall_ms": policy.get("verified_candidate_eval_wall_ms"),
        "verified_proposer_keep_rate": policy.get("verified_proposer_keep_rate"),
        "verified_proposer_kept_candidate_count": policy.get("verified_proposer_kept_candidate_count"),
        "verified_proposer_non_rule_candidate_count": policy.get("verified_proposer_non_rule_candidate_count"),
    }


def render_compact(summary: dict[str, Any]) -> dict[str, Any]:
    return {
        "rows": [
            {
                "seed_start": row["seed_start"],
                "case": row["case"],
                "reward": row["average_total_reward"],
                "stderr": row["reward_stderr"],
                "deaths": (row.get("result_counts") or {}).get("defeat", 0),
                "wins": row["average_combat_win_count"],
                "candidate_evals": row["verified_candidate_evaluation_count"],
                "policy_steps": row["verified_cached_policy_step_eval_count"],
                "keep_rate": row["verified_proposer_keep_rate"],
                "wall_seconds": row["wall_seconds"],
            }
            for row in summary["rows"]
        ]
    }


def parse_int_list(value: str) -> list[int]:
    return [int(item) for item in value.split(",") if item.strip()]


def parse_float_list(value: str) -> list[float]:
    return [float(item) for item in value.split(",") if item.strip()]


if __name__ == "__main__":
    main()
