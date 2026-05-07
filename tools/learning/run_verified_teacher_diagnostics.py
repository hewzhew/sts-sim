#!/usr/bin/env python3
"""Run and summarize verified-teacher diagnostics.

This script is intentionally about the teacher, not a learned proposer.  It
compares rule/plan baselines with verified override variants and surfaces the
parts that decide whether the teacher is worth distilling later: reward,
deaths, override rate, candidate/H-step cost, horizon stop reasons, and the
contexts/payoff categories where the teacher overrides rule.
"""
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
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--report-out", type=Path)
    parser.add_argument("--episodes", type=int, default=20)
    parser.add_argument("--seed-start", type=int, default=98100)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=160)
    parser.add_argument("--driver-binary", type=Path, default=Path("target/release/full_run_env_driver.exe"))
    parser.add_argument("--dev-tool-binary", type=Path, default=Path("target/release/sts_dev_tool.exe"))
    parser.add_argument("--runner", type=Path, default=Path("tools/learning/eval_verified_adv_override_rust_runner.py"))
    parser.add_argument("--python", type=Path, default=Path(sys.executable))
    parser.add_argument("--work-dir", type=Path, default=Path("target/verified_teacher_diagnostics"))
    parser.add_argument("--parallelism", type=int, default=0)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--oracle-margin", type=float, default=1.0)
    parser.add_argument(
        "--evidence-gate",
        default="horizon_cap_no_payoff_v1",
        choices=["none", "horizon_cap_no_payoff_v1", "horizon_cap_any_v1"],
    )
    parser.add_argument("--low-evidence-margin", type=float)
    parser.add_argument("--confirm-low-evidence-horizon-decisions", type=int)
    parser.add_argument("--confirm-low-evidence-margin", type=float)
    parser.add_argument(
        "--cases",
        default="rule,plan,h4_fixed,h8_fixed,h8_cached,h8_adaptive_payoff",
        help=(
            "Comma-separated cases. Supported: rule, plan, h4_fixed, h8_fixed, "
            "h8_cached, h8_adaptive_next_turn, h8_adaptive_payoff."
        ),
    )
    parser.add_argument(
        "--keep-episodes",
        action="store_true",
        help="Keep verified per-episode rows and emit seed-level deltas against rule.",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    args.work_dir.mkdir(parents=True, exist_ok=True)
    rows = []
    for case in parse_cases(args.cases):
        started = time.perf_counter()
        if case in {"rule", "plan"}:
            raw_path = args.work_dir / f"{case}_seed{args.seed_start}_n{args.episodes}.json"
            raw = run_policy_baseline(args, case, raw_path)
            row = summarize_baseline(case, raw, raw_path)
        else:
            raw_path = args.work_dir / f"{case}_seed{args.seed_start}_n{args.episodes}.json"
            raw = run_verified_case(args, case, raw_path)
            row = summarize_verified(case, raw, raw_path)
        row["wall_seconds"] = time.perf_counter() - started
        rows.append(row)

    baseline = next((row for row in rows if row["case"] == "rule"), None)
    if baseline:
        for row in rows:
            if row is baseline:
                row["reward_delta_vs_rule"] = 0.0
                row["death_delta_vs_rule"] = 0
            else:
                row["reward_delta_vs_rule"] = none_sub(row.get("average_total_reward"), baseline.get("average_total_reward"))
                row["death_delta_vs_rule"] = int(row.get("defeat_count") or 0) - int(baseline.get("defeat_count") or 0)

    summary = {
        "schema_version": "verified_teacher_diagnostics_v0",
        "config": {
            "episodes": args.episodes,
            "seed_start": args.seed_start,
            "seed_step": args.seed_step,
            "ascension": args.ascension,
            "class": args.player_class,
            "final_act": args.final_act,
            "max_steps": args.max_steps,
            "cases": parse_cases(args.cases),
            "parallelism": args.parallelism,
            "oracle_margin": args.oracle_margin,
            "evidence_gate": args.evidence_gate,
            "low_evidence_margin": args.low_evidence_margin,
            "confirm_low_evidence_horizon_decisions": args.confirm_low_evidence_horizon_decisions,
            "confirm_low_evidence_margin": args.confirm_low_evidence_margin,
            "gamma": args.gamma,
        },
        "rows": rows,
        "seed_deltas": seed_delta_reports(rows),
        "interpretation": interpret(rows),
    }
    write_json(args.out, summary)
    report_out = args.report_out or args.out.with_suffix(".md")
    report_out.parent.mkdir(parents=True, exist_ok=True)
    report_out.write_text(render_markdown(summary), encoding="utf-8")
    print(json.dumps(render_compact(summary), indent=2, sort_keys=True))


def run_policy_baseline(args: argparse.Namespace, case: str, out: Path) -> dict[str, Any]:
    policy = "rule_baseline_v0" if case == "rule" else "plan_query_v0"
    cmd = [
        str(args.dev_tool_binary),
        "run-batch",
        "--episodes",
        str(args.episodes),
        "--seed",
        str(args.seed_start),
        "--ascension",
        str(args.ascension),
        "--class",
        args.player_class,
        "--max-steps",
        str(args.max_steps),
        "--policy",
        policy,
        "--reward-shaping-profile",
        "baseline",
        "--summary-out",
        str(out),
    ]
    if args.final_act:
        cmd.append("--final-act")
    run_command(cmd)
    return read_json(out)


def run_verified_case(args: argparse.Namespace, case: str, out: Path) -> dict[str, Any]:
    horizon, horizon_mode, evaluation_mode, exact_dedup = verified_case_config(case)
    cmd = [
        str(args.python),
        str(args.runner),
        "--binary",
        str(args.driver_binary),
        "--out",
        str(out),
        "--episodes",
        str(args.episodes),
        "--seed-start",
        str(args.seed_start),
        "--seed-step",
        str(args.seed_step),
        "--ascension",
        str(args.ascension),
        "--class",
        args.player_class,
        "--max-steps",
        str(args.max_steps),
        "--candidate-scope",
        "controlled_v1",
        "--horizon-decisions",
        str(horizon),
        "--horizon-mode",
        horizon_mode,
        "--oracle-margin",
        str(args.oracle_margin),
        "--gamma",
        str(args.gamma),
        "--verifier-strategy",
        "single_stage",
        "--verified-evaluation-mode",
        evaluation_mode,
        "--verified-value-cache-scope",
        "episode",
        "--verified-value-cache-max-entries",
        "4096",
        "--verified-parallelism",
        str(args.parallelism),
    ]
    if args.low_evidence_margin is not None:
        cmd.extend(
            [
                "--evidence-gate",
                args.evidence_gate,
                "--low-evidence-margin",
                str(args.low_evidence_margin),
            ]
        )
    if args.confirm_low_evidence_horizon_decisions is not None:
        cmd.extend(
            [
                "--confirm-low-evidence-horizon-decisions",
                str(args.confirm_low_evidence_horizon_decisions),
            ]
        )
    if args.confirm_low_evidence_margin is not None:
        cmd.extend(["--confirm-low-evidence-margin", str(args.confirm_low_evidence_margin)])
    if not args.keep_episodes:
        cmd.append("--summary-only")
    cmd.append("--verified-exact-root-dedup" if exact_dedup else "--no-verified-exact-root-dedup")
    if args.final_act:
        cmd.append("--final-act")
    run_command(cmd)
    return read_json(out)


def summarize_baseline(case: str, raw: dict[str, Any], path: Path) -> dict[str, Any]:
    return {
        "case": case,
        "kind": "baseline",
        "raw_path": str(path),
        "policy": raw.get("policy"),
        "episodes": raw.get("episodes_requested"),
        "average_total_reward": raw.get("average_total_reward"),
        "reward_stderr": None,
        "average_combat_win_count": raw.get("average_combat_wins"),
        "average_final_floor": raw.get("average_floor"),
        "average_steps": raw.get("average_steps"),
        "defeat_count": int((raw.get("result_counts") or {}).get("defeat") or 0),
        "result_counts": raw.get("result_counts") or {},
        "death_floor_counts": raw.get("death_floor_counts") or {},
        "steps_per_second": raw.get("steps_per_second"),
        "episodes_per_hour": raw.get("episodes_per_hour"),
        "episodes_detail": [
            normalize_episode("baseline", episode)
            for episode in raw.get("episodes", [])
        ],
    }


def summarize_verified(case: str, raw: dict[str, Any], path: Path) -> dict[str, Any]:
    policy_summary = raw.get("policy_summary") or {}
    policy_name, policy = next(iter(policy_summary.items()))
    evals = int(policy.get("verified_candidate_evaluation_count") or 0)
    policy_steps = int(policy.get("verified_cached_policy_step_eval_count") or 0)
    wall = float((raw.get("runtime") or {}).get("python_wall_seconds") or 0.0)
    return {
        "case": case,
        "kind": "verified_teacher",
        "raw_path": str(path),
        "policy": policy_name,
        "episodes": policy.get("episodes"),
        "average_total_reward": policy.get("average_total_reward"),
        "reward_stderr": policy.get("reward_stderr"),
        "average_combat_win_count": policy.get("average_combat_win_count"),
        "average_final_floor": policy.get("average_final_floor"),
        "average_final_hp": policy.get("average_final_hp"),
        "average_steps": policy.get("average_steps"),
        "defeat_count": int((policy.get("result_counts") or {}).get("defeat") or 0),
        "result_counts": policy.get("result_counts") or {},
        "death_floor_counts": policy.get("death_floor_counts") or {},
        "verified_decision_count": policy.get("verified_decision_count"),
        "verified_override_count": policy.get("verified_override_count"),
        "verified_override_rate": policy.get("verified_override_rate"),
        "verified_adv_mean_on_overrides": policy.get("verified_adv_mean_on_overrides"),
        "verified_low_evidence_reject_count": policy.get("verified_low_evidence_reject_count"),
        "verified_confirm_decision_count": policy.get("verified_confirm_decision_count"),
        "verified_confirm_accept_count": policy.get("verified_confirm_accept_count"),
        "verified_confirm_reject_count": policy.get("verified_confirm_reject_count"),
        "verified_confirm_candidate_evaluation_count": policy.get("verified_confirm_candidate_evaluation_count"),
        "verified_confirm_policy_step_eval_count": policy.get("verified_confirm_policy_step_eval_count"),
        "verified_candidate_evaluation_count": evals,
        "verified_cached_policy_step_eval_count": policy_steps,
        "verified_policy_steps_per_candidate": policy_steps / evals if evals else None,
        "verified_candidate_eval_wall_ms": policy.get("verified_candidate_eval_wall_ms"),
        "verified_candidate_evals_per_second": evals / wall if wall > 0 else None,
        "verified_parallelism_used_max": policy.get("verified_parallelism_used_max"),
        "verified_cached_root_exact_dedup_rate": policy.get("verified_cached_root_exact_dedup_rate"),
        "verified_cached_value_hit_rate": policy.get("verified_cached_value_hit_rate"),
        "verified_horizon_stop_reason_counts": policy.get("verified_horizon_stop_reason_counts") or {},
        "verified_best_adv_bucket_counts": policy.get("verified_best_adv_bucket_counts") or {},
        "verified_override_context_top": top_counts(policy.get("verified_override_context_counts") or {}, 10),
        "verified_override_payoff_top": top_counts(policy.get("verified_override_payoff_reason_counts") or {}, 10),
        "runtime_python_wall_seconds": wall,
        "episodes_detail": [
            normalize_episode("verified", episode)
            for episode in raw.get("episodes", [])
        ],
    }


def normalize_episode(kind: str, episode: dict[str, Any]) -> dict[str, Any]:
    if kind == "baseline":
        return {
            "seed": episode.get("seed"),
            "result": episode.get("result"),
            "floor": episode.get("floor"),
            "act": episode.get("act"),
            "hp": episode.get("hp"),
            "steps": episode.get("steps"),
            "total_reward": episode.get("total_reward"),
            "combat_win_count": episode.get("combat_win_count"),
        }
    return {
        "seed": episode.get("seed"),
        "result": episode.get("result"),
        "floor": episode.get("final_floor"),
        "act": episode.get("final_act"),
        "hp": episode.get("final_hp"),
        "steps": episode.get("steps"),
        "total_reward": episode.get("total_reward"),
        "combat_win_count": episode.get("combat_win_count"),
        "verified_override_count": episode.get("verified_override_count"),
        "verified_override_rate": episode.get("verified_override_rate"),
        "verified_adv_mean_on_overrides": episode.get("verified_adv_mean_on_overrides"),
        "verified_candidate_evaluation_count": episode.get("verified_candidate_evaluation_count"),
        "verified_cached_policy_step_eval_count": episode.get("verified_cached_policy_step_eval_count"),
        "verified_override_events": episode.get("verified_override_events") or [],
    }


def seed_delta_reports(rows: list[dict[str, Any]]) -> dict[str, Any]:
    rule = next((row for row in rows if row["case"] == "rule"), None)
    if not rule or not rule.get("episodes_detail"):
        return {}
    rule_by_seed = {int(ep["seed"]): ep for ep in rule["episodes_detail"] if ep.get("seed") is not None}
    reports: dict[str, Any] = {}
    for row in rows:
        if row.get("kind") != "verified_teacher" or not row.get("episodes_detail"):
            continue
        deltas = []
        for episode in row["episodes_detail"]:
            seed = episode.get("seed")
            if seed is None or int(seed) not in rule_by_seed:
                continue
            ref = rule_by_seed[int(seed)]
            reward_delta = none_sub(episode.get("total_reward"), ref.get("total_reward"))
            win_delta = none_sub(episode.get("combat_win_count"), ref.get("combat_win_count"))
            floor_delta = none_sub(episode.get("floor"), ref.get("floor"))
            deltas.append(
                {
                    "seed": int(seed),
                    "reward_delta": reward_delta,
                    "combat_win_delta": win_delta,
                    "floor_delta": floor_delta,
                    "rule_result": ref.get("result"),
                    "teacher_result": episode.get("result"),
                    "rule_floor": ref.get("floor"),
                    "teacher_floor": episode.get("floor"),
                    "rule_reward": ref.get("total_reward"),
                    "teacher_reward": episode.get("total_reward"),
                    "teacher_overrides": episode.get("verified_override_count"),
                    "teacher_policy_steps": episode.get("verified_cached_policy_step_eval_count"),
                    "teacher_override_events": episode.get("verified_override_events") or [],
                }
            )
        reports[row["case"]] = summarize_seed_deltas(deltas)
    return reports


def summarize_seed_deltas(deltas: list[dict[str, Any]]) -> dict[str, Any]:
    improved = [row for row in deltas if float(row.get("reward_delta") or 0.0) > 0.0]
    worsened = [row for row in deltas if float(row.get("reward_delta") or 0.0) < 0.0]
    rescued = [
        row
        for row in deltas
        if row.get("rule_result") == "defeat" and row.get("teacher_result") != "defeat"
    ]
    harmed = [
        row
        for row in deltas
        if row.get("rule_result") != "defeat" and row.get("teacher_result") == "defeat"
    ]
    return {
        "seed_count": len(deltas),
        "mean_reward_delta": mean([float(row.get("reward_delta") or 0.0) for row in deltas]),
        "improved_seed_count": len(improved),
        "worsened_seed_count": len(worsened),
        "rescued_from_rule_defeat_count": len(rescued),
        "harmed_into_teacher_defeat_count": len(harmed),
        "top_improved": sorted(deltas, key=lambda row: float(row.get("reward_delta") or 0.0), reverse=True)[:8],
        "top_worsened": sorted(deltas, key=lambda row: float(row.get("reward_delta") or 0.0))[:8],
        "rescued_examples": rescued[:8],
        "harmed_examples": harmed[:8],
    }


def interpret(rows: list[dict[str, Any]]) -> list[str]:
    out = []
    rule = next((row for row in rows if row["case"] == "rule"), None)
    verified = [row for row in rows if row["kind"] == "verified_teacher"]
    if rule and verified:
        best = max(verified, key=lambda row: float(row.get("average_total_reward") or -1e9))
        out.append(
            f"Best verified case is {best['case']} with reward delta {none_sub(best.get('average_total_reward'), rule.get('average_total_reward')):.3f} vs rule."
        )
        if int(best.get("defeat_count") or 0) > int(rule.get("defeat_count") or 0):
            out.append("Best reward case still has more deaths than rule; reward and survival are not fully aligned.")
    for row in verified:
        stops = row.get("verified_horizon_stop_reason_counts") or {}
        cap = int(stops.get("horizon_decision_cap") or 0)
        total = sum(int(value) for value in stops.values())
        if total and cap / total > 0.75:
            out.append(f"{row['case']} often stops at the horizon cap ({cap / total:.1%}); longer/adaptive leaf value may matter.")
    return out


def render_markdown(summary: dict[str, Any]) -> str:
    rows = summary["rows"]
    lines = [
        "# Verified Teacher Diagnostics",
        "",
        "## Summary",
        "",
        "| case | reward | delta vs rule | defeats | combat wins | overrides | evals | policy steps | wall s |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for row in rows:
        lines.append(
            "| {case} | {reward} | {delta} | {defeats} | {wins} | {overrides} | {evals} | {steps} | {wall} |".format(
                case=row["case"],
                reward=fmt(row.get("average_total_reward")),
                delta=fmt(row.get("reward_delta_vs_rule")),
                defeats=row.get("defeat_count"),
                wins=fmt(row.get("average_combat_win_count")),
                overrides=fmt(row.get("verified_override_rate"), pct=True) if row.get("kind") == "verified_teacher" else "",
                evals=row.get("verified_candidate_evaluation_count", ""),
                steps=row.get("verified_cached_policy_step_eval_count", ""),
                wall=fmt(row.get("wall_seconds")),
            )
        )
    lines.extend(["", "## Interpretation", ""])
    for item in summary.get("interpretation") or []:
        lines.append(f"- {item}")
    if summary.get("seed_deltas"):
        lines.extend(["", "## Seed Deltas", ""])
        for case, report in summary["seed_deltas"].items():
            lines.extend(
                [
                    f"### {case}",
                    "",
                    f"- mean reward delta: `{fmt(report.get('mean_reward_delta'))}`",
                    f"- improved/worsened seeds: `{report.get('improved_seed_count')}` / `{report.get('worsened_seed_count')}`",
                    f"- rescued from rule defeat: `{report.get('rescued_from_rule_defeat_count')}`",
                    f"- harmed into teacher defeat: `{report.get('harmed_into_teacher_defeat_count')}`",
                    f"- top improved: `{report.get('top_improved')}`",
                    f"- top worsened: `{report.get('top_worsened')}`",
                    "",
                ]
            )
    lines.extend(["", "## Verified Details", ""])
    for row in rows:
        if row.get("kind") != "verified_teacher":
            continue
        lines.extend(
            [
                f"### {row['case']}",
                "",
                f"- horizon stops: `{row.get('verified_horizon_stop_reason_counts')}`",
                f"- best-adv buckets: `{row.get('verified_best_adv_bucket_counts')}`",
                f"- override contexts: `{row.get('verified_override_context_top')}`",
                f"- override payoff reasons: `{row.get('verified_override_payoff_top')}`",
                f"- low-evidence confirm: decisions `{row.get('verified_confirm_decision_count')}`, accepts `{row.get('verified_confirm_accept_count')}`, rejects `{row.get('verified_confirm_reject_count')}`, evals `{row.get('verified_confirm_candidate_evaluation_count')}`, policy steps `{row.get('verified_confirm_policy_step_eval_count')}`",
                f"- cache hit rate: `{row.get('verified_cached_value_hit_rate')}`; exact root dedup rate: `{row.get('verified_cached_root_exact_dedup_rate')}`",
                "",
            ]
        )
    return "\n".join(lines) + "\n"


def render_compact(summary: dict[str, Any]) -> dict[str, Any]:
    return {
        "config": summary["config"],
        "rows": [
            {
                "case": row["case"],
                "reward": row.get("average_total_reward"),
                "delta_vs_rule": row.get("reward_delta_vs_rule"),
                "defeats": row.get("defeat_count"),
                "combat_wins": row.get("average_combat_win_count"),
                "override_rate": row.get("verified_override_rate"),
                "low_evidence_rejects": row.get("verified_low_evidence_reject_count"),
                "confirm_decisions": row.get("verified_confirm_decision_count"),
                "confirm_accepts": row.get("verified_confirm_accept_count"),
                "confirm_rejects": row.get("verified_confirm_reject_count"),
                "evals": row.get("verified_candidate_evaluation_count"),
                "policy_steps": row.get("verified_cached_policy_step_eval_count"),
                "wall_seconds": row.get("wall_seconds"),
            }
            for row in summary["rows"]
        ],
        "seed_deltas": {
            case: {
                "mean_reward_delta": report.get("mean_reward_delta"),
                "improved": report.get("improved_seed_count"),
                "worsened": report.get("worsened_seed_count"),
                "rescued": report.get("rescued_from_rule_defeat_count"),
                "harmed": report.get("harmed_into_teacher_defeat_count"),
            }
            for case, report in (summary.get("seed_deltas") or {}).items()
        },
        "interpretation": summary["interpretation"],
    }


def verified_case_config(case: str) -> tuple[int, str, str, bool]:
    if case == "h4_fixed":
        return 4, "fixed_decisions", "independent", False
    if case == "h8_fixed":
        return 8, "fixed_decisions", "independent", False
    if case == "h8_cached":
        return 8, "fixed_decisions", "bellman_cached_v1", True
    if case == "h8_adaptive_next_turn":
        return 8, "adaptive_next_player_turn_v1", "independent", False
    if case == "h8_adaptive_payoff":
        return 8, "adaptive_payoff_window_v1", "independent", False
    raise ValueError(f"unsupported verified case {case}")


def parse_cases(value: str) -> list[str]:
    cases = [item.strip() for item in value.split(",") if item.strip()]
    supported = {
        "rule",
        "plan",
        "h4_fixed",
        "h8_fixed",
        "h8_cached",
        "h8_adaptive_next_turn",
        "h8_adaptive_payoff",
    }
    unknown = [case for case in cases if case not in supported]
    if unknown:
        raise SystemExit(f"unsupported cases: {', '.join(unknown)}")
    return cases


def top_counts(counts: dict[str, Any], limit: int) -> list[dict[str, Any]]:
    return [
        {"key": key, "count": int(value)}
        for key, value in sorted(counts.items(), key=lambda item: int(item[1]), reverse=True)[:limit]
    ]


def run_command(cmd: list[str]) -> None:
    completed = subprocess.run(cmd, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if completed.returncode != 0:
        raise SystemExit(
            "command failed with exit code {code}\nCMD: {cmd}\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}".format(
                code=completed.returncode,
                cmd=" ".join(cmd),
                stdout=completed.stdout,
                stderr=completed.stderr,
            )
        )


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def fmt(value: Any, pct: bool = False) -> str:
    if value is None:
        return ""
    try:
        numeric = float(value)
    except (TypeError, ValueError):
        return str(value)
    if pct:
        return f"{numeric * 100:.2f}%"
    return f"{numeric:.3f}"


def none_sub(left: Any, right: Any) -> float | None:
    if left is None or right is None:
        return None
    return float(left) - float(right)


def mean(values: list[float]) -> float:
    return sum(values) / len(values) if values else 0.0


if __name__ == "__main__":
    main()
