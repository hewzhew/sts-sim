#!/usr/bin/env python3
"""Run a small two-stage verified-teacher prefilter grid.

This script targets the Rust-side verified override runner.  It varies the
cheap prefilter margin and top-K retention, while the final verifier remains
the exact configured H-step teacher.
"""
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
    parser.add_argument("--episodes", type=int, default=20)
    parser.add_argument("--seed-start", type=int, default=98100)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--max-steps", type=int, default=160)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--candidate-scope", default="controlled_v1", choices=["all", "controlled_v0", "controlled_v1"])
    parser.add_argument("--horizon-decisions", type=int, default=8)
    parser.add_argument(
        "--horizon-mode",
        default="fixed_decisions",
        choices=["fixed_decisions", "adaptive_next_player_turn_v1", "adaptive_payoff_window_v1"],
    )
    parser.add_argument("--oracle-margin", type=float, default=1.0)
    parser.add_argument("--prefilter-horizon-decisions", type=int, default=8)
    parser.add_argument(
        "--prefilter-horizon-mode",
        default="adaptive_payoff_window_v1",
        choices=["fixed_decisions", "adaptive_next_player_turn_v1", "adaptive_payoff_window_v1"],
    )
    parser.add_argument("--prefilter-margins", default="1.0,2.0,3.0")
    parser.add_argument("--prefilter-top-ks", default="0,1,2")
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--continuation-policy", default="rule_baseline_v0")
    parser.add_argument("--verified-evaluation-mode", default="independent", choices=["independent", "bellman_cached_v1"])
    parser.add_argument("--verified-value-cache-scope", default="episode", choices=["request", "episode"])
    parser.add_argument("--verified-value-cache-max-entries", type=int, default=4096)
    parser.add_argument("--verified-parallelism", type=int, default=0)
    parser.add_argument("--verified-exact-root-dedup", action=argparse.BooleanOptionalAction, default=False)
    parser.add_argument("--include-single-stage-reference", action="store_true")
    parser.add_argument("--keep-episodes", action="store_true")
    parser.add_argument("--skip-existing", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)
    summary_out = args.summary_out or args.out_dir / "prefilter_grid_summary.json"
    csv_out = args.csv_out or args.out_dir / "prefilter_grid_summary.csv"
    margins = parse_float_list(args.prefilter_margins)
    top_ks = parse_int_list(args.prefilter_top_ks)
    binary = binary_path(args.binary, "full_run_env_driver")

    rows: list[dict[str, Any]] = []
    if args.include_single_stage_reference:
        rows.append(
            run_case(
                args,
                binary,
                args.out_dir / reference_filename(args),
                verifier_strategy="single_stage",
                prefilter_margin=None,
                prefilter_top_k=None,
            )
        )
        write_outputs(args, summary_out, csv_out, rows)
        print(render_row(rows[-1]), flush=True)

    for top_k in top_ks:
        for margin in margins:
            out_path = args.out_dir / case_filename(args, margin, top_k)
            row = run_case(
                args,
                binary,
                out_path,
                verifier_strategy="two_stage_prefilter_v1",
                prefilter_margin=margin,
                prefilter_top_k=top_k,
            )
            rows.append(row)
            write_outputs(args, summary_out, csv_out, rows)
            print(render_row(row), flush=True)

    summary = build_summary(args, rows)
    write_json(summary_out, summary)
    write_csv(csv_out, rows)
    print(json.dumps(summary, indent=2, sort_keys=True))


def run_case(
    args: argparse.Namespace,
    binary: Path,
    out_path: Path,
    *,
    verifier_strategy: str,
    prefilter_margin: float | None,
    prefilter_top_k: int | None,
) -> dict[str, Any]:
    started = time.perf_counter()
    if args.skip_existing and out_path.exists():
        payload = read_json(out_path)
        runtime_seconds = None
    else:
        cmd = [
            str(args.python),
            str(REPO_ROOT / "tools" / "learning" / "eval_verified_adv_override_rust_runner.py"),
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
            "--candidate-scope",
            args.candidate_scope,
            "--horizon-decisions",
            str(args.horizon_decisions),
            "--horizon-mode",
            args.horizon_mode,
            "--oracle-margin",
            str(args.oracle_margin),
            "--verifier-strategy",
            verifier_strategy,
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
        ]
        if args.verified_exact_root_dedup:
            cmd.append("--verified-exact-root-dedup")
        else:
            cmd.append("--no-verified-exact-root-dedup")
        if args.final_act:
            cmd.append("--final-act")
        if not args.keep_episodes:
            cmd.append("--summary-only")
        if verifier_strategy == "two_stage_prefilter_v1":
            cmd.extend(
                [
                    "--prefilter-horizon-decisions",
                    str(args.prefilter_horizon_decisions),
                    "--prefilter-horizon-mode",
                    args.prefilter_horizon_mode,
                    "--prefilter-margin",
                    str(prefilter_margin),
                    "--prefilter-top-k",
                    str(prefilter_top_k),
                ]
            )
        print(f"running {case_label(verifier_strategy, prefilter_margin, prefilter_top_k)} -> {out_path}", flush=True)
        result = subprocess.run(
            cmd,
            cwd=str(REPO_ROOT),
            text=True,
            capture_output=True,
            check=False,
        )
        runtime_seconds = time.perf_counter() - started
        if result.returncode != 0:
            return {
                "status": "failed",
                "verifier_strategy": verifier_strategy,
                "prefilter_margin": prefilter_margin,
                "prefilter_top_k": prefilter_top_k,
                "out": str(out_path),
                "runtime_seconds": runtime_seconds,
                "returncode": result.returncode,
                "stdout_tail": result.stdout[-4000:],
                "stderr_tail": result.stderr[-4000:],
            }
        payload = read_json(out_path)
    return summarize_case(args, payload, out_path, verifier_strategy, prefilter_margin, prefilter_top_k, runtime_seconds)


def summarize_case(
    args: argparse.Namespace,
    payload: dict[str, Any],
    out_path: Path,
    verifier_strategy: str,
    prefilter_margin: float | None,
    prefilter_top_k: int | None,
    runtime_seconds: float | None,
) -> dict[str, Any]:
    policies = payload.get("policy_summary") or {}
    policy_name, policy = next(iter(policies.items()), ("", {}))
    result_counts = policy.get("result_counts") or {}
    return {
        "status": "ok",
        "verifier_strategy": verifier_strategy,
        "prefilter_margin": prefilter_margin,
        "prefilter_top_k": prefilter_top_k,
        "policy": policy_name,
        "out": str(out_path),
        "episodes": args.episodes,
        "seed_start": args.seed_start,
        "max_steps": args.max_steps,
        "average_total_reward": nullable_float(policy.get("average_total_reward")),
        "reward_stderr": nullable_float(policy.get("reward_stderr")),
        "defeat_count": int(result_counts.get("defeat") or 0),
        "ongoing_count": int(result_counts.get("ongoing") or 0),
        "average_combat_win_count": nullable_float(policy.get("average_combat_win_count")),
        "average_final_floor": nullable_float(policy.get("average_final_floor")),
        "average_final_hp": nullable_float(policy.get("average_final_hp")),
        "crash_count": int(policy.get("crash_count") or 0),
        "verified_decision_count": int(policy.get("verified_decision_count") or 0),
        "verified_override_count": int(policy.get("verified_override_count") or 0),
        "verified_override_rate": nullable_float(policy.get("verified_override_rate")),
        "verified_adv_mean_on_overrides": nullable_float(policy.get("verified_adv_mean_on_overrides")),
        "verified_candidate_evaluation_count": int(policy.get("verified_candidate_evaluation_count") or 0),
        "verified_prefilter_candidate_evaluation_count": int(policy.get("verified_prefilter_candidate_evaluation_count") or 0),
        "verified_final_candidate_evaluation_count": int(policy.get("verified_final_candidate_evaluation_count") or 0),
        "verified_cached_policy_step_eval_count": int(policy.get("verified_cached_policy_step_eval_count") or 0),
        "verified_prefilter_policy_step_eval_count": int(policy.get("verified_prefilter_policy_step_eval_count") or 0),
        "verified_final_policy_step_eval_count": int(policy.get("verified_final_policy_step_eval_count") or 0),
        "verified_candidate_eval_wall_ms": int(policy.get("verified_candidate_eval_wall_ms") or 0),
        "verified_prefilter_kept_rate": nullable_float(policy.get("verified_prefilter_kept_rate")),
        "verified_prefilter_average_kept_candidate_count": nullable_float(
            policy.get("verified_prefilter_average_kept_candidate_count")
        ),
        "runtime_seconds": runtime_seconds,
        "python_wall_seconds": nullable_float((payload.get("runtime") or {}).get("python_wall_seconds")),
    }


def write_outputs(args: argparse.Namespace, summary_out: Path, csv_out: Path, rows: list[dict[str, Any]]) -> None:
    write_json(summary_out, build_summary(args, rows))
    write_csv(csv_out, rows)


def build_summary(args: argparse.Namespace, rows: list[dict[str, Any]]) -> dict[str, Any]:
    ok_rows = [row for row in rows if row.get("status") == "ok"]
    reference = next((row for row in ok_rows if row.get("verifier_strategy") == "single_stage"), None)
    for row in ok_rows:
        if reference and row is not reference:
            row["reward_delta_vs_reference"] = nullable_delta(
                row.get("average_total_reward"), reference.get("average_total_reward")
            )
            row["policy_step_delta_vs_reference"] = int(row.get("verified_cached_policy_step_eval_count") or 0) - int(
                reference.get("verified_cached_policy_step_eval_count") or 0
            )
            row["final_step_delta_vs_reference"] = int(row.get("verified_final_policy_step_eval_count") or 0) - int(
                reference.get("verified_final_policy_step_eval_count") or 0
            )
    best_reward = max(
        ok_rows,
        key=lambda row: float(row.get("average_total_reward") or float("-inf")),
        default=None,
    )
    cheapest_final = min(
        ok_rows,
        key=lambda row: int(row.get("verified_final_policy_step_eval_count") or 10**18),
        default=None,
    )
    return {
        "schema_version": "verified_adv_override_prefilter_grid_summary_v0",
        "config": {
            "episodes": args.episodes,
            "seed_start": args.seed_start,
            "seed_step": args.seed_step,
            "max_steps": args.max_steps,
            "ascension": args.ascension,
            "class": args.player_class,
            "final_act": args.final_act,
            "candidate_scope": args.candidate_scope,
            "horizon_decisions": args.horizon_decisions,
            "horizon_mode": args.horizon_mode,
            "oracle_margin": args.oracle_margin,
            "prefilter_horizon_decisions": args.prefilter_horizon_decisions,
            "prefilter_horizon_mode": args.prefilter_horizon_mode,
            "prefilter_margins": args.prefilter_margins,
            "prefilter_top_ks": args.prefilter_top_ks,
            "summary_only": not args.keep_episodes,
            "include_single_stage_reference": args.include_single_stage_reference,
        },
        "completed": len(rows),
        "ok": len(ok_rows),
        "failed": len(rows) - len(ok_rows),
        "reference": reference,
        "best_by_reward": best_reward,
        "cheapest_by_final_policy_steps": cheapest_final,
        "rows": rows,
    }


def write_csv(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fieldnames = [
        "status",
        "verifier_strategy",
        "prefilter_top_k",
        "prefilter_margin",
        "average_total_reward",
        "reward_stderr",
        "reward_delta_vs_reference",
        "defeat_count",
        "ongoing_count",
        "average_combat_win_count",
        "average_final_floor",
        "average_final_hp",
        "verified_override_count",
        "verified_override_rate",
        "verified_adv_mean_on_overrides",
        "verified_candidate_evaluation_count",
        "verified_prefilter_candidate_evaluation_count",
        "verified_final_candidate_evaluation_count",
        "verified_cached_policy_step_eval_count",
        "verified_prefilter_policy_step_eval_count",
        "verified_final_policy_step_eval_count",
        "policy_step_delta_vs_reference",
        "final_step_delta_vs_reference",
        "verified_candidate_eval_wall_ms",
        "verified_prefilter_kept_rate",
        "verified_prefilter_average_kept_candidate_count",
        "runtime_seconds",
        "python_wall_seconds",
        "out",
    ]
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames, extrasaction="ignore")
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def render_row(row: dict[str, Any]) -> str:
    if row.get("status") != "ok":
        return (
            f"FAILED {case_label(row.get('verifier_strategy'), row.get('prefilter_margin'), row.get('prefilter_top_k'))}: "
            f"returncode={row.get('returncode')}"
        )
    label = case_label(row.get("verifier_strategy"), row.get("prefilter_margin"), row.get("prefilter_top_k"))
    reward = row.get("average_total_reward")
    stderr = row.get("reward_stderr")
    final_steps = row.get("verified_final_policy_step_eval_count")
    total_steps = row.get("verified_cached_policy_step_eval_count")
    override_rate = row.get("verified_override_rate")
    kept = row.get("verified_prefilter_average_kept_candidate_count")
    return (
        f"{label}: reward={format_float(reward)}±{format_float(stderr)} "
        f"deaths={row.get('defeat_count')} overrides={row.get('verified_override_count')} "
        f"override_rate={format_percent(override_rate)} "
        f"final_steps={final_steps} total_steps={total_steps} kept_avg={format_float(kept)}"
    )


def case_label(strategy: Any, margin: Any, top_k: Any) -> str:
    if strategy == "single_stage":
        return "single_stage"
    return f"two_stage(topK={top_k}, margin={margin})"


def case_filename(args: argparse.Namespace, margin: float, top_k: int) -> str:
    return (
        f"twostage_h{args.horizon_decisions}_{args.horizon_mode}_"
        f"pm{float_key(margin)}_top{top_k}_{args.episodes}seed.json"
    )


def reference_filename(args: argparse.Namespace) -> str:
    return f"single_stage_h{args.horizon_decisions}_{args.horizon_mode}_{args.episodes}seed.json"


def parse_int_list(text: str) -> list[int]:
    values = [int(item.strip()) for item in text.split(",") if item.strip()]
    if not values:
        raise SystemExit("expected at least one integer")
    return values


def parse_float_list(text: str) -> list[float]:
    values = [float(item.strip()) for item in text.split(",") if item.strip()]
    if not values:
        raise SystemExit("expected at least one float")
    return values


def float_key(value: float) -> str:
    return str(value).replace(".", "p").replace("-", "neg")


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def nullable_float(value: Any) -> float | None:
    if value is None:
        return None
    return float(value)


def nullable_delta(left: Any, right: Any) -> float | None:
    if left is None or right is None:
        return None
    return float(left) - float(right)


def format_float(value: Any) -> str:
    if value is None:
        return "n/a"
    return f"{float(value):.3f}"


def format_percent(value: Any) -> str:
    if value is None:
        return "n/a"
    return f"{float(value) * 100:.2f}%"


if __name__ == "__main__":
    main()
