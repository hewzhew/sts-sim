#!/usr/bin/env python3
"""Run a horizon/margin grid for verified advantage override."""
from __future__ import annotations

import argparse
import csv
import json
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

from return_q_common import REPO_ROOT, binary_path, write_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out-dir", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path)
    parser.add_argument("--csv-out", type=Path)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--python", type=Path, default=Path(sys.executable))
    parser.add_argument("--episodes", type=int, default=100)
    parser.add_argument("--seed-start", type=int, default=98100)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--max-steps", type=int, default=160)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--candidate-scope", default="controlled_v1", choices=["all", "controlled_v0", "controlled_v1"])
    parser.add_argument("--horizons", default="2,4,8")
    parser.add_argument("--margins", default="0.25,0.5,1.0")
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--continuation-policy", default="rule_baseline_v0")
    parser.add_argument("--verified-evaluation-mode", default="independent", choices=["independent", "bellman_cached_v1"])
    parser.add_argument("--verified-value-cache-scope", default="episode", choices=["request", "episode"])
    parser.add_argument("--verified-value-cache-max-entries", type=int, default=4096)
    parser.add_argument("--verified-parallelism", type=int, default=0)
    parser.add_argument("--verified-exact-root-dedup", action=argparse.BooleanOptionalAction, default=False)
    parser.add_argument("--verified-prefilter-horizon-decisions", type=int, default=-1)
    parser.add_argument("--verified-prefilter-margin", type=float, default=0.0)
    parser.add_argument("--verified-prefilter-top-k", type=int, default=0)
    parser.add_argument("--skip-existing", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)
    summary_out = args.summary_out or args.out_dir / "grid_summary.json"
    csv_out = args.csv_out or args.out_dir / "grid_summary.csv"
    horizons = parse_int_list(args.horizons)
    margins = parse_float_list(args.margins)
    binary = binary_path(args.binary, "full_run_env_driver")

    rows: list[dict[str, Any]] = []
    for horizon in horizons:
        for margin in margins:
            out_path = args.out_dir / f"eval_h{horizon}_m{margin_key(margin)}.json"
            started = time.perf_counter()
            if args.skip_existing and out_path.exists():
                payload = read_json(out_path)
                runtime_seconds = None
            else:
                cmd = [
                    str(args.python),
                    str(REPO_ROOT / "tools" / "learning" / "eval_return_q_closed_loop.py"),
                    "--binary",
                    str(binary),
                    "--out",
                    str(out_path),
                    "--episodes",
                    str(args.episodes),
                    "--seed-start",
                    str(args.seed_start),
                    "--seed-step",
                    str(args.seed_step),
                    "--max-steps",
                    str(args.max_steps),
                    "--ascension",
                    str(args.ascension),
                    "--class",
                    args.player_class,
                    "--policies",
                    f"rule_baseline_v0,verified_adv_override_agent_v0_H{horizon}",
                    "--candidate-scope",
                    args.candidate_scope,
                    "--oracle-margin",
                    str(margin),
                    "--oracle-continuation-policy",
                    args.continuation_policy,
                    "--gamma",
                    str(args.gamma),
                    "--verified-evaluation-mode",
                    args.verified_evaluation_mode,
                    "--verified-value-cache-scope",
                    args.verified_value_cache_scope,
                    "--verified-value-cache-max-entries",
                    str(args.verified_value_cache_max_entries),
                    "--verified-parallelism",
                    str(args.verified_parallelism),
                    "--verified-prefilter-horizon-decisions",
                    str(args.verified_prefilter_horizon_decisions),
                    "--verified-prefilter-margin",
                    str(args.verified_prefilter_margin),
                    "--verified-prefilter-top-k",
                    str(args.verified_prefilter_top_k),
                ]
                if args.verified_exact_root_dedup:
                    cmd.append("--verified-exact-root-dedup")
                else:
                    cmd.append("--no-verified-exact-root-dedup")
                if args.final_act:
                    cmd.append("--final-act")
                print(
                    f"running H={horizon} margin={margin} -> {out_path}",
                    flush=True,
                )
                result = subprocess.run(
                    cmd,
                    cwd=str(REPO_ROOT),
                    text=True,
                    capture_output=True,
                    check=False,
                )
                runtime_seconds = time.perf_counter() - started
                if result.returncode != 0:
                    failure = {
                        "horizon": horizon,
                        "margin": margin,
                        "out": str(out_path),
                        "runtime_seconds": runtime_seconds,
                        "returncode": result.returncode,
                        "stdout_tail": result.stdout[-4000:],
                        "stderr_tail": result.stderr[-4000:],
                    }
                    rows.append({"status": "failed", **failure})
                    write_json(summary_out, build_summary(args, rows))
                    write_csv(csv_out, rows)
                    print(json.dumps(failure, indent=2, sort_keys=True), flush=True)
                    continue
                payload = read_json(out_path)
            row = summarize_run(args, payload, horizon, margin, out_path, runtime_seconds)
            rows.append(row)
            write_json(summary_out, build_summary(args, rows))
            write_csv(csv_out, rows)
            print(render_row(row), flush=True)

    summary = build_summary(args, rows)
    write_json(summary_out, summary)
    write_csv(csv_out, rows)
    print(json.dumps(summary, indent=2, sort_keys=True))


def summarize_run(
    args: argparse.Namespace,
    payload: dict[str, Any],
    horizon: int,
    margin: float,
    out_path: Path,
    runtime_seconds: float | None,
) -> dict[str, Any]:
    policies = payload.get("policy_summary") or {}
    rule = policies.get("rule_baseline_v0") or {}
    agent_name = f"verified_adv_override_agent_v0_H{horizon}"
    agent = policies.get(agent_name) or {}
    return {
        "status": "ok",
        "horizon": horizon,
        "margin": margin,
        "agent_policy": agent_name,
        "out": str(out_path),
        "runtime_seconds": runtime_seconds,
        "episodes": args.episodes,
        "seed_start": args.seed_start,
        "max_steps": args.max_steps,
        "candidate_scope": args.candidate_scope,
        "rule_reward": as_float(rule.get("average_total_reward")),
        "agent_reward": as_float(agent.get("average_total_reward")),
        "reward_delta": as_float(agent.get("average_total_reward")) - as_float(rule.get("average_total_reward")),
        "rule_reward_stderr": nullable_float(rule.get("reward_stderr")),
        "agent_reward_stderr": nullable_float(agent.get("reward_stderr")),
        "rule_wins": as_float(rule.get("average_combat_win_count")),
        "agent_wins": as_float(agent.get("average_combat_win_count")),
        "wins_delta": as_float(agent.get("average_combat_win_count")) - as_float(rule.get("average_combat_win_count")),
        "rule_crashes": int(rule.get("crash_count") or 0),
        "agent_crashes": int(agent.get("crash_count") or 0),
        "verified_decision_count": int(agent.get("verified_decision_count") or 0),
        "verified_override_count": int(agent.get("verified_override_count") or 0),
        "verified_override_rate": as_float(agent.get("verified_override_rate")),
        "verified_candidate_evaluation_count": int(agent.get("verified_candidate_evaluation_count") or 0),
        "verified_adv_mean_on_overrides": nullable_float(agent.get("verified_adv_mean_on_overrides")),
        "verified_harmful_override_count": int(agent.get("verified_harmful_override_count") or 0),
        "verified_harmful_override_rate": nullable_float(agent.get("verified_harmful_override_rate")),
        "verified_prefilter_evaluation_count": int(agent.get("verified_prefilter_evaluation_count") or 0),
        "verified_prefilter_pass_count": int(agent.get("verified_prefilter_pass_count") or 0),
        "verified_prefilter_reject_count": int(agent.get("verified_prefilter_reject_count") or 0),
        "verified_prefilter_pass_rate": nullable_float(agent.get("verified_prefilter_pass_rate")),
        "verified_prefilter_average_final_candidate_count": nullable_float(
            agent.get("verified_prefilter_average_final_candidate_count")
        ),
        "verified_cached_root_candidate_count": int(agent.get("verified_cached_root_candidate_count") or 0),
        "verified_cached_root_exact_dedup_count": int(agent.get("verified_cached_root_exact_dedup_count") or 0),
        "verified_cached_root_exact_dedup_rate": nullable_float(agent.get("verified_cached_root_exact_dedup_rate")),
        "verified_root_rule_equivalent_prune_count": int(agent.get("verified_root_rule_equivalent_prune_count") or 0),
        "verified_root_rule_equivalent_prune_rate": nullable_float(
            agent.get("verified_root_rule_equivalent_prune_rate")
        ),
        "verified_cached_value_hit_count": int(agent.get("verified_cached_value_hit_count") or 0),
        "verified_cached_value_miss_count": int(agent.get("verified_cached_value_miss_count") or 0),
        "verified_cached_value_hit_rate": nullable_float(agent.get("verified_cached_value_hit_rate")),
        "verified_cached_policy_step_eval_count": int(agent.get("verified_cached_policy_step_eval_count") or 0),
        "verified_cached_cache_entry_count_max": int(agent.get("verified_cached_cache_entry_count_max") or 0),
        "verified_parallelism_used_max": int(agent.get("verified_parallelism_used_max") or 0),
        "verified_candidate_eval_wall_ms": int(agent.get("verified_candidate_eval_wall_ms") or 0),
        "verified_decision_type_counts": agent.get("verified_decision_type_counts") or {},
        "verified_override_decision_type_counts": agent.get("verified_override_decision_type_counts") or {},
    }


def build_summary(args: argparse.Namespace, rows: list[dict[str, Any]]) -> dict[str, Any]:
    ok_rows = [row for row in rows if row.get("status") == "ok"]
    best_reward = max(ok_rows, key=lambda row: row.get("agent_reward", float("-inf")), default=None)
    best_delta = max(ok_rows, key=lambda row: row.get("reward_delta", float("-inf")), default=None)
    return {
        "schema_version": "verified_adv_override_grid_summary_v0",
        "config": {
            "episodes": args.episodes,
            "seed_start": args.seed_start,
            "seed_step": args.seed_step,
            "max_steps": args.max_steps,
            "ascension": args.ascension,
            "class": args.player_class,
            "final_act": args.final_act,
            "candidate_scope": args.candidate_scope,
            "horizons": args.horizons,
            "margins": args.margins,
            "gamma": args.gamma,
            "continuation_policy": args.continuation_policy,
            "verified_evaluation_mode": args.verified_evaluation_mode,
            "verified_value_cache_scope": args.verified_value_cache_scope,
            "verified_value_cache_max_entries": args.verified_value_cache_max_entries,
            "verified_parallelism": args.verified_parallelism,
            "verified_exact_root_dedup": args.verified_exact_root_dedup,
            "verified_prefilter_horizon_decisions": args.verified_prefilter_horizon_decisions,
            "verified_prefilter_margin": args.verified_prefilter_margin,
            "verified_prefilter_top_k": args.verified_prefilter_top_k,
        },
        "completed": len(rows),
        "ok": len(ok_rows),
        "failed": len(rows) - len(ok_rows),
        "best_by_agent_reward": best_reward,
        "best_by_reward_delta": best_delta,
        "rows": rows,
    }


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fieldnames = [
        "status",
        "horizon",
        "margin",
        "agent_policy",
        "reward_delta",
        "rule_reward",
        "agent_reward",
        "rule_reward_stderr",
        "agent_reward_stderr",
        "wins_delta",
        "rule_wins",
        "agent_wins",
        "agent_crashes",
        "verified_override_count",
        "verified_override_rate",
        "verified_adv_mean_on_overrides",
        "verified_harmful_override_count",
        "verified_harmful_override_rate",
        "verified_prefilter_evaluation_count",
        "verified_prefilter_pass_count",
        "verified_prefilter_reject_count",
        "verified_prefilter_pass_rate",
        "verified_prefilter_average_final_candidate_count",
        "verified_cached_root_candidate_count",
        "verified_cached_root_exact_dedup_count",
        "verified_cached_root_exact_dedup_rate",
        "verified_root_rule_equivalent_prune_count",
        "verified_root_rule_equivalent_prune_rate",
        "verified_cached_value_hit_count",
        "verified_cached_value_miss_count",
        "verified_cached_value_hit_rate",
        "verified_cached_policy_step_eval_count",
        "verified_cached_cache_entry_count_max",
        "verified_parallelism_used_max",
        "verified_candidate_eval_wall_ms",
        "verified_candidate_evaluation_count",
        "runtime_seconds",
        "out",
    ]
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames, extrasaction="ignore")
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def render_row(row: dict[str, Any]) -> str:
    if row.get("status") != "ok":
        return f"FAILED H={row.get('horizon')} margin={row.get('margin')}"
    return (
        f"H={row['horizon']} margin={row['margin']}: "
        f"reward {row['agent_reward']:.3f} vs {row['rule_reward']:.3f} "
        f"(delta {row['reward_delta']:+.3f}), "
        f"wins {row['agent_wins']:.2f} vs {row['rule_wins']:.2f}, "
        f"override {row['verified_override_rate']:.2%}, "
        f"adv {row['verified_adv_mean_on_overrides']:.3f}, "
        f"harmful {row['verified_harmful_override_count']}"
    )


def parse_int_list(text: str) -> list[int]:
    values = [int(item.strip()) for item in text.split(",") if item.strip()]
    if not values:
        raise SystemExit("at least one horizon is required")
    return values


def parse_float_list(text: str) -> list[float]:
    values = [float(item.strip()) for item in text.split(",") if item.strip()]
    if not values:
        raise SystemExit("at least one margin is required")
    return values


def margin_key(value: float) -> str:
    return str(value).replace(".", "p").replace("-", "neg")


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def as_float(value: Any) -> float:
    return float(value or 0.0)


def nullable_float(value: Any) -> float | None:
    if value is None:
        return None
    return float(value)


if __name__ == "__main__":
    main()
