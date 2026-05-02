#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from statistics import mean
from typing import Any

from combat_rl_common import REPO_ROOT, write_json, write_jsonl


SURVIVAL_ORDER = {
    "forced_loss": 0,
    "stochastic_loss_risk": 1,
    "severe_risk": 2,
    "risky": 3,
    "stable": 4,
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run combat abstraction lab over a batch of full-run trace combat roots and summarize "
            "chosen ranks, near-equivalence groups, and obvious local regrets."
        )
    )
    parser.add_argument("--trace-dir", type=Path, required=True)
    parser.add_argument("--max-cases", type=int, default=12)
    parser.add_argument("--per-trace-limit", type=int, default=6)
    parser.add_argument("--min-candidates", type=int, default=2)
    parser.add_argument("--case-strategy", default="danger", choices=["trace_order", "danger"])
    parser.add_argument("--min-step-gap", type=int, default=3)
    parser.add_argument("--continuation-policy", default="rule_baseline_v0", choices=["rule_baseline_v0", "random_masked"])
    parser.add_argument("--horizon", type=int, default=10)
    parser.add_argument("--samples", type=int, default=1)
    parser.add_argument("--max-branches", type=int, default=12)
    parser.add_argument("--max-interesting-cases", type=int, default=20)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=5000)
    parser.add_argument("--driver-binary", type=Path)
    parser.add_argument("--allow-replay-mismatch", action="store_true")
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "combat_abstraction_lab" / "batch",
    )
    return parser.parse_args()


def load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def trace_files(trace_dir: Path) -> list[Path]:
    files = sorted(trace_dir.glob("episode_*.json"))
    if not files:
        files = sorted(trace_dir.rglob("episode_*.json"))
    if not files:
        raise SystemExit(f"no episode_*.json files found in {trace_dir}")
    return files


def combat_obs(step: dict[str, Any]) -> dict[str, Any]:
    obs = step.get("observation") or {}
    return obs.get("combat") or {}


def unblocked_damage(step: dict[str, Any]) -> int:
    combat = combat_obs(step)
    incoming = int(combat.get("visible_incoming_damage") or 0)
    block = int(combat.get("player_block") or 0)
    return max(incoming - block, 0)


def hp(step: dict[str, Any]) -> int:
    obs = step.get("observation") or {}
    combat = combat_obs(step)
    return int(obs.get("current_hp") or combat.get("player_hp") or step.get("hp") or 0)


def legal_count(step: dict[str, Any]) -> int:
    mask = step.get("action_mask") or []
    return int(step.get("legal_action_count") or len(mask))


def case_priority(step: dict[str, Any]) -> tuple[float, int, int, int]:
    current_hp = max(hp(step), 1)
    unblocked = unblocked_damage(step)
    danger = unblocked / current_hp
    combat = combat_obs(step)
    return (
        danger,
        unblocked,
        legal_count(step),
        int(combat.get("total_monster_hp") or 0),
    )


def public_case(case: dict[str, Any]) -> dict[str, Any]:
    return {
        key: str(value) if isinstance(value, Path) else value
        for key, value in case.items()
        if key != "trace_file"
    } | {"trace_file": str(case["trace_file"])}


def step_to_case(path: Path, trace: dict[str, Any], step: dict[str, Any]) -> dict[str, Any]:
    combat = combat_obs(step)
    step_index = int(step.get("step_index") or 0)
    return {
        "case_id": f"{path.stem}_step_{step_index:04}",
        "trace_file": path,
        "episode_id": int((trace.get("summary") or {}).get("episode_id") or 0),
        "seed": int((trace.get("summary") or {}).get("seed") or 0),
        "step_index": step_index,
        "decision_type": str(step.get("decision_type") or ""),
        "floor": int(step.get("floor") or 0),
        "act": int(step.get("act") or 0),
        "hp": hp(step),
        "incoming": int(combat.get("visible_incoming_damage") or 0),
        "unblocked": unblocked_damage(step),
        "turn_count": int(combat.get("turn_count") or 0),
        "monster_hp": int(combat.get("total_monster_hp") or 0),
        "candidate_count": legal_count(step),
        "chosen_action_index": int(step.get("chosen_action_index") or 0),
        "chosen_action_key": str(step.get("chosen_action_key") or ""),
    }


def select_cases(args: argparse.Namespace) -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []
    for path in trace_files(args.trace_dir):
        trace = load_json(path)
        candidates = []
        selected_steps: list[int] = []
        for step in trace.get("steps") or []:
            if str(step.get("decision_type") or "") != "combat":
                continue
            if legal_count(step) < args.min_candidates:
                continue
            candidates.append((case_priority(step), step))
        if args.case_strategy == "danger":
            candidates.sort(key=lambda item: item[0], reverse=True)
        per_trace = 0
        for _priority, step in candidates:
            step_index = int(step.get("step_index") or 0)
            if any(abs(step_index - selected) < args.min_step_gap for selected in selected_steps):
                continue
            cases.append(step_to_case(path, trace, step))
            selected_steps.append(step_index)
            per_trace += 1
            if per_trace >= args.per_trace_limit or len(cases) >= args.max_cases:
                break
        if len(cases) >= args.max_cases:
            break
    return cases


def run_case(args: argparse.Namespace, case: dict[str, Any], reports_dir: Path) -> dict[str, Any]:
    report_path = reports_dir / f"{case['case_id']}.json"
    rows_path = reports_dir / f"{case['case_id']}.rows.jsonl"
    cmd = [
        sys.executable,
        str(REPO_ROOT / "tools" / "learning" / "combat_abstraction_lab.py"),
        "--trace-file",
        str(case["trace_file"]),
        "--step-index",
        str(case["step_index"]),
        "--continuation-policy",
        args.continuation_policy,
        "--horizon",
        str(args.horizon),
        "--samples",
        str(args.samples),
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
        "--rows-out",
        str(rows_path),
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
            "rows_path": str(rows_path),
        }
    return {
        "case": case,
        "status": "ok",
        "report_path": str(report_path),
        "rows_path": str(rows_path),
        "report": load_json(report_path),
    }


def find_ranking_row(report: dict[str, Any], candidate_index: int) -> dict[str, Any] | None:
    for row in report.get("ranking") or []:
        if int(row.get("candidate_index") or 0) == candidate_index:
            return row
    return None


def group_ranks(report: dict[str, Any]) -> dict[str, int]:
    best_by_signature: dict[str, int] = {}
    for row in report.get("ranking") or []:
        signature = str(row.get("equivalence_signature") or "")
        rank = int(row.get("rank") or 0)
        if signature and (signature not in best_by_signature or rank < best_by_signature[signature]):
            best_by_signature[signature] = rank
    return best_by_signature


def survival_rank(row: dict[str, Any] | None) -> int:
    if row is None:
        return -1
    abstraction = row.get("abstraction") or {}
    return SURVIVAL_ORDER.get(str(abstraction.get("survival_class") or ""), -1)


def estimate(row: dict[str, Any] | None, key: str) -> float:
    if row is None:
        return 0.0
    return float((row.get("estimates") or {}).get(key) or 0.0)


def action_family(candidate_key: str | None) -> str:
    key = str(candidate_key or "")
    if key.startswith("combat/play_card"):
        return "play_card"
    if key.startswith("combat/use_potion"):
        return "use_potion"
    if key == "combat/end_turn":
        return "end_turn"
    return "other"


def estimate_range(ranking: list[dict[str, Any]], key: str) -> float:
    values = [estimate(row, key) for row in ranking]
    if not values:
        return 0.0
    return max(values) - min(values)


def abstraction_counts(ranking: list[dict[str, Any]], key: str) -> dict[str, int]:
    counts = Counter(str((row.get("abstraction") or {}).get(key) or "unknown") for row in ranking)
    return dict(sorted(counts.items()))


def abstraction_value(row: dict[str, Any] | None, key: str) -> Any:
    if row is None:
        return None
    return (row.get("abstraction") or {}).get(key)


def diagnostic_flags(report: dict[str, Any], chosen: dict[str, Any] | None, best: dict[str, Any] | None, chosen_group_rank: int | None) -> list[str]:
    flags: list[str] = []
    if chosen is None or best is None:
        return ["missing_chosen_or_best"]
    if int(chosen.get("rank") or 0) != 1:
        flags.append("chosen_not_top_rank")
    if chosen_group_rank != 1:
        flags.append("chosen_not_top_equivalence_group")
    if survival_rank(best) > survival_rank(chosen):
        flags.append("survival_dominated")
    if estimate(best, "combat_win_prob") > estimate(chosen, "combat_win_prob") + 0.001:
        flags.append("missed_combat_win")
    hp_regret = estimate(best, "expected_end_hp") - estimate(chosen, "expected_end_hp")
    if hp_regret >= 5.0:
        flags.append("hp_regret_ge_5")
    if hp_regret >= 10.0:
        flags.append("hp_regret_ge_10")
    if "combat/end_turn" in str(chosen.get("candidate_key") or "") and "combat/end_turn" not in str(best.get("candidate_key") or ""):
        flags.append("end_turn_not_best")
    if action_family(best.get("candidate_key")) == "use_potion" and action_family(chosen.get("candidate_key")) != "use_potion":
        flags.append("potion_opportunity")
    if action_family(chosen.get("candidate_key")) == "use_potion" and action_family(best.get("candidate_key")) != "use_potion":
        flags.append("potion_overuse")
    chosen_fit = str(abstraction_value(chosen, "root_plan_fit") or "")
    best_fit = str(abstraction_value(best, "root_plan_fit") or "")
    chosen_role = str(abstraction_value(chosen, "root_plan_role") or "")
    best_role = str(abstraction_value(best, "root_plan_role") or "")
    pressure = str(abstraction_value(chosen, "pressure_class") or abstraction_value(best, "pressure_class") or "")
    if chosen_fit == "ignores_attack" and best_fit in {"covers_attack", "reduces_attack", "decisive"}:
        flags.append("ignored_attack_pressure")
    if chosen_role == "partial_defense" and best_role == "full_defense":
        flags.append("missed_full_defense")
    if pressure == "no_attack" and chosen_fit == "wastes_window" and best_fit in {"uses_window", "decisive"}:
        flags.append("wasted_no_attack_window")
    return flags


def triage_kind(chosen: dict[str, Any] | None, best: dict[str, Any] | None, chosen_group_rank: int | None) -> str:
    if chosen is None or best is None:
        return "invalid"
    if estimate(best, "survive_prob") <= 0.0:
        return "already_lost"
    if survival_rank(best) > survival_rank(chosen):
        return "survival_rescue"
    if estimate(best, "combat_win_prob") > estimate(chosen, "combat_win_prob") + 0.001:
        return "tactical_win"
    hp_regret = estimate(best, "expected_end_hp") - estimate(chosen, "expected_end_hp")
    if hp_regret >= 10.0:
        return "major_hp_efficiency"
    if hp_regret >= 5.0:
        return "hp_efficiency"
    if chosen_group_rank == 1:
        return "same_equivalence_group"
    return "weak_rank_gap"


def is_actionable(kind: str) -> bool:
    return kind in {"survival_rescue", "tactical_win", "major_hp_efficiency", "hp_efficiency"}


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
    ranking = list(report.get("ranking") or [])
    chosen_index = int(case["chosen_action_index"])
    chosen = find_ranking_row(report, chosen_index)
    best = ranking[0] if ranking else None
    ranks_by_group = group_ranks(report)
    chosen_signature = str((chosen or {}).get("equivalence_signature") or "")
    chosen_group_rank = ranks_by_group.get(chosen_signature)
    flags = diagnostic_flags(report, chosen, best, chosen_group_rank)
    kind = triage_kind(chosen, best, chosen_group_rank)
    group_count = int((report.get("summary") or {}).get("equivalence_group_count") or 0)
    candidate_count = int((report.get("summary") or {}).get("candidate_count") or 0)
    case_summary = {
        **public_case(case),
        "status": "ok",
        "report_path": result.get("report_path"),
        "rows_path": result.get("rows_path"),
        "candidate_count_evaluated": candidate_count,
        "equivalence_group_count": group_count,
        "compression_ratio": (group_count / candidate_count) if candidate_count else 0.0,
        "chosen_rank": None if chosen is None else int(chosen.get("rank") or 0),
        "chosen_group_rank": chosen_group_rank,
        "chosen_was_top_rank": None if chosen is None else int(chosen.get("rank") or 0) == 1,
        "chosen_was_top_group": chosen_group_rank == 1 if chosen_group_rank is not None else None,
        "chosen_equivalence_signature": chosen_signature or None,
        "chosen_action_family": None if chosen is None else action_family(chosen.get("candidate_key")),
        "chosen_survival_class": None if chosen is None else (chosen.get("abstraction") or {}).get("survival_class"),
        "chosen_kill_clock": None if chosen is None else (chosen.get("abstraction") or {}).get("kill_clock"),
        "chosen_risk_bucket": None if chosen is None else (chosen.get("abstraction") or {}).get("risk_bucket"),
        "chosen_role": None if chosen is None else (chosen.get("abstraction") or {}).get("role"),
        "chosen_pressure_class": abstraction_value(chosen, "pressure_class"),
        "chosen_root_plan_role": abstraction_value(chosen, "root_plan_role"),
        "chosen_root_plan_fit": abstraction_value(chosen, "root_plan_fit"),
        "chosen_root_block_need": abstraction_value(chosen, "root_block_need"),
        "chosen_root_unblocked_reduction": abstraction_value(chosen, "root_unblocked_reduction"),
        "chosen_root_damage_delta": abstraction_value(chosen, "root_damage_delta"),
        "best_candidate_index": None if best is None else int(best.get("candidate_index") or 0),
        "best_candidate_key": None if best is None else str(best.get("candidate_key") or ""),
        "best_action_family": None if best is None else action_family(best.get("candidate_key")),
        "best_equivalence_signature": None if best is None else str(best.get("equivalence_signature") or ""),
        "best_survival_class": None if best is None else (best.get("abstraction") or {}).get("survival_class"),
        "best_kill_clock": None if best is None else (best.get("abstraction") or {}).get("kill_clock"),
        "best_risk_bucket": None if best is None else (best.get("abstraction") or {}).get("risk_bucket"),
        "best_role": None if best is None else (best.get("abstraction") or {}).get("role"),
        "best_pressure_class": abstraction_value(best, "pressure_class"),
        "best_root_plan_role": abstraction_value(best, "root_plan_role"),
        "best_root_plan_fit": abstraction_value(best, "root_plan_fit"),
        "best_root_block_need": abstraction_value(best, "root_block_need"),
        "best_root_unblocked_reduction": abstraction_value(best, "root_unblocked_reduction"),
        "best_root_damage_delta": abstraction_value(best, "root_damage_delta"),
        "hp_regret": estimate(best, "expected_end_hp") - estimate(chosen, "expected_end_hp"),
        "combat_win_regret": estimate(best, "combat_win_prob") - estimate(chosen, "combat_win_prob"),
        "survive_prob_range": estimate_range(ranking, "survive_prob"),
        "combat_win_prob_range": estimate_range(ranking, "combat_win_prob"),
        "expected_end_hp_range": estimate_range(ranking, "expected_end_hp"),
        "survival_class_counts": abstraction_counts(ranking, "survival_class"),
        "kill_clock_counts": abstraction_counts(ranking, "kill_clock"),
        "risk_bucket_counts": abstraction_counts(ranking, "risk_bucket"),
        "role_counts": abstraction_counts(ranking, "role"),
        "pressure_class_counts": abstraction_counts(ranking, "pressure_class"),
        "root_plan_role_counts": abstraction_counts(ranking, "root_plan_role"),
        "root_plan_fit_counts": abstraction_counts(ranking, "root_plan_fit"),
        "triage_kind": kind,
        "actionable": is_actionable(kind),
        "diagnostic_flags": flags,
    }
    rows = []
    for row in ranking:
        abstraction = row.get("abstraction") or {}
        estimates = row.get("estimates") or {}
        rows.append(
            {
                **public_case(case),
                "candidate_index": int(row.get("candidate_index") or 0),
                "candidate_key": row.get("candidate_key"),
                "action_family": action_family(row.get("candidate_key")),
                "candidate_card_id": row.get("candidate_card_id"),
                "rank": int(row.get("rank") or 0),
                "equivalence_signature": row.get("equivalence_signature"),
                "is_chosen": int(row.get("candidate_index") or 0) == chosen_index,
                "is_best": int(row.get("rank") or 0) == 1,
                **abstraction,
                **estimates,
            }
        )
    return rows, case_summary


def rate(items: list[dict[str, Any]], key: str) -> float:
    if not items:
        return 0.0
    return sum(1 for item in items if item.get(key)) / len(items)


def main() -> None:
    args = parse_args()
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    out_dir = args.out_dir / stamp
    reports_dir = out_dir / "case_reports"
    reports_dir.mkdir(parents=True, exist_ok=True)
    cases = select_cases(args)
    if not cases:
        raise SystemExit("no combat cases selected")

    started = time.perf_counter()
    results = [run_case(args, case, reports_dir) for case in cases]
    elapsed = time.perf_counter() - started

    candidate_rows: list[dict[str, Any]] = []
    case_summaries: list[dict[str, Any]] = []
    for result in results:
        rows, case_summary = flatten_case(result)
        candidate_rows.extend(rows)
        case_summaries.append(case_summary)

    ok_cases = [case for case in case_summaries if case.get("status") == "ok"]
    flag_counts: Counter[str] = Counter()
    for case in ok_cases:
        flag_counts.update(case.get("diagnostic_flags") or [])
    summary = {
        "schema_version": "combat_abstraction_batch_v0",
        "created_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {
            "trace_dir": str(args.trace_dir),
            "max_cases": args.max_cases,
            "per_trace_limit": args.per_trace_limit,
            "min_candidates": args.min_candidates,
            "case_strategy": args.case_strategy,
            "min_step_gap": args.min_step_gap,
            "continuation_policy": args.continuation_policy,
            "horizon": args.horizon,
            "samples": 1 if args.continuation_policy == "rule_baseline_v0" else args.samples,
            "max_branches": args.max_branches,
            "max_interesting_cases": args.max_interesting_cases,
        },
        "counts": {
            "selected_cases": len(cases),
            "ok_cases": len(ok_cases),
            "failed_cases": sum(1 for result in results if result["status"] != "ok"),
            "candidate_rows": len(candidate_rows),
        },
        "chosen_rank_counts": dict(sorted(Counter(str(case.get("chosen_rank")) for case in ok_cases).items())),
        "chosen_group_rank_counts": dict(sorted(Counter(str(case.get("chosen_group_rank")) for case in ok_cases).items())),
        "chosen_top_rank_rate": rate(ok_cases, "chosen_was_top_rank"),
        "chosen_top_group_rate": rate(ok_cases, "chosen_was_top_group"),
        "diagnostic_flag_counts": dict(sorted(flag_counts.items())),
        "triage_kind_counts": dict(sorted(Counter(str(case.get("triage_kind")) for case in ok_cases).items())),
        "actionable_case_count": sum(1 for case in ok_cases if case.get("actionable")),
        "average_candidate_count": mean([case["candidate_count_evaluated"] for case in ok_cases]) if ok_cases else 0.0,
        "average_equivalence_group_count": mean([case["equivalence_group_count"] for case in ok_cases]) if ok_cases else 0.0,
        "average_compression_ratio": mean([case["compression_ratio"] for case in ok_cases]) if ok_cases else 0.0,
        "chosen_role_counts": dict(sorted(Counter(str(case.get("chosen_role")) for case in ok_cases).items())),
        "best_role_counts": dict(sorted(Counter(str(case.get("best_role")) for case in ok_cases).items())),
        "chosen_action_family_counts": dict(sorted(Counter(str(case.get("chosen_action_family")) for case in ok_cases).items())),
        "best_action_family_counts": dict(sorted(Counter(str(case.get("best_action_family")) for case in ok_cases).items())),
        "pressure_class_counts": dict(sorted(Counter(str(case.get("chosen_pressure_class")) for case in ok_cases).items())),
        "chosen_root_plan_fit_counts": dict(sorted(Counter(str(case.get("chosen_root_plan_fit")) for case in ok_cases).items())),
        "best_root_plan_fit_counts": dict(sorted(Counter(str(case.get("best_root_plan_fit")) for case in ok_cases).items())),
        "chosen_root_plan_role_counts": dict(sorted(Counter(str(case.get("chosen_root_plan_role")) for case in ok_cases).items())),
        "best_root_plan_role_counts": dict(sorted(Counter(str(case.get("best_root_plan_role")) for case in ok_cases).items())),
        "elapsed_seconds": elapsed,
        "interesting_cases": [
            case
            for case in ok_cases
            if case.get("actionable")
        ][: args.max_interesting_cases],
        "case_summaries": case_summaries,
    }
    write_json(out_dir / "summary.json", summary)
    write_jsonl(out_dir / "candidate_rows.jsonl", candidate_rows)
    write_jsonl(out_dir / "case_summaries.jsonl", case_summaries)
    print(
        json.dumps(
            summary["counts"]
            | {
                "chosen_top_rank_rate": summary["chosen_top_rank_rate"],
                "chosen_top_group_rate": summary["chosen_top_group_rate"],
                "triage_kind_counts": summary["triage_kind_counts"],
                "actionable_case_count": summary["actionable_case_count"],
                "diagnostic_flag_counts": summary["diagnostic_flag_counts"],
            },
            indent=2,
            ensure_ascii=False,
        )
    )
    print(f"wrote {out_dir}")


if __name__ == "__main__":
    main()
