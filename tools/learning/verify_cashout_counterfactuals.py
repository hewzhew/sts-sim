#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json


REPORT_VERSION = "cashout_counterfactual_verifier_v0"
DEFAULT_STATUSES = "high_confidence_candidate,needs_rollout"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Verify card cashout diagnostics by replaying reward-card decisions, "
            "branching legal candidates, and continuing with a fixed policy. This is "
            "a conservative simulator check, not a teacher label."
        )
    )
    parser.add_argument("--cashout-report", type=Path)
    parser.add_argument(
        "--policies",
        default="all",
        help="Comma-separated policy names from the cashout report, or 'all'.",
    )
    parser.add_argument(
        "--statuses",
        default=DEFAULT_STATUSES,
        help="Comma-separated calibration statuses to verify.",
    )
    parser.add_argument("--max-cases", type=int, default=8)
    parser.add_argument("--per-policy-limit", type=int, default=4)
    parser.add_argument(
        "--continuation-policy",
        default="rule_baseline_v0",
        choices=["rule_baseline_v0", "plan_query_v0", "random_masked"],
    )
    parser.add_argument("--continuation-steps", type=int, default=100)
    parser.add_argument("--max-branches", type=int, default=8)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=5000)
    parser.add_argument("--driver-binary", type=Path)
    parser.add_argument("--allow-replay-mismatch", action="store_true")
    parser.add_argument("--min-hp-margin", type=int, default=5)
    parser.add_argument("--min-reward-margin", type=float, default=1.0)
    parser.add_argument(
        "--out",
        type=Path,
        default=REPO_ROOT
        / "tools"
        / "artifacts"
        / "card_cashout_lab"
        / "counterfactual_verification"
        / "cashout_counterfactual_verification.json",
    )
    parser.add_argument("--markdown-out", type=Path)
    parser.add_argument("--case-report-dir", type=Path)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def parse_csv(text: str) -> set[str]:
    return {part.strip() for part in text.split(",") if part.strip()}


def selected_policies(report: dict[str, Any], text: str) -> set[str]:
    names = {str(policy.get("policy") or "") for policy in report.get("policies") or []}
    if text.strip().lower() == "all":
        return names
    wanted = parse_csv(text)
    missing = sorted(wanted - names)
    if missing:
        raise SystemExit(f"cashout report has no policy entries: {', '.join(missing)}")
    return wanted


def iter_selected_cases(args: argparse.Namespace, report: dict[str, Any]) -> list[dict[str, Any]]:
    wanted_policies = selected_policies(report, args.policies)
    wanted_statuses = parse_csv(args.statuses)
    if not wanted_statuses:
        raise SystemExit("--statuses must not be empty")
    cases: list[dict[str, Any]] = []
    per_policy_counts: Counter[str] = Counter()
    for policy in report.get("policies") or []:
        policy_name = str(policy.get("policy") or "")
        if policy_name not in wanted_policies:
            continue
        source_cases = candidate_source_cases(policy)
        seen: set[tuple[str, int, str, str]] = set()
        for case in source_cases:
            status = str(case.get("calibration_status") or "uncalibrated")
            if status not in wanted_statuses:
                continue
            key = (
                str(case.get("trace_file") or ""),
                int(case.get("step_index") or 0),
                str((case.get("chosen") or {}).get("action_key") or ""),
                str((case.get("best_by_cashout") or {}).get("action_key") or ""),
            )
            if key in seen:
                continue
            seen.add(key)
            if per_policy_counts[policy_name] >= args.per_policy_limit:
                continue
            cases.append(
                {
                    "policy": policy_name,
                    "case_id": case_id(policy_name, case),
                    "source_case": case,
                }
            )
            per_policy_counts[policy_name] += 1
            if len(cases) >= args.max_cases:
                return cases
    return cases


def candidate_source_cases(policy: dict[str, Any]) -> list[dict[str, Any]]:
    cases = list(policy.get("comparisons") or [])
    if not cases:
        cases = list(policy.get("high_confidence_cases") or []) + list(policy.get("top_cases") or [])
    return sorted(cases, key=case_priority)


def case_priority(case: dict[str, Any]) -> tuple[int, float, int, int]:
    status = str(case.get("calibration_status") or "uncalibrated")
    status_rank = {
        "high_confidence_candidate": 0,
        "needs_rollout": 1,
        "diagnostic_only": 2,
        "cashout_disagreement_with_rule_baseline": 3,
    }.get(status, 4)
    return (
        status_rank,
        -float(case.get("cashout_gap") or 0.0),
        int(case.get("seed") or 0),
        int(case.get("step_index") or 0),
    )


def case_id(policy: str, case: dict[str, Any]) -> str:
    best = case.get("best_by_cashout") or {}
    best_card = str(best.get("card_id") or best.get("action_key") or "best")
    return sanitize(
        "{policy}_seed_{seed}_step_{step}_{best}".format(
            policy=policy,
            seed=int(case.get("seed") or 0),
            step=int(case.get("step_index") or 0),
            best=best_card,
        )
    )


def sanitize(text: str) -> str:
    return "".join(ch if ch.isalnum() or ch in {"_", "-"} else "_" for ch in text)


def trace_step(trace: dict[str, Any], step_index: int) -> dict[str, Any]:
    for step in trace.get("steps") or []:
        if int(step.get("step_index") or 0) == step_index:
            return step
    raise ValueError(f"trace has no step_index={step_index}")


def candidate_indices_for_case(trace_path: Path, case: dict[str, Any]) -> list[int]:
    trace = read_json(trace_path)
    step = trace_step(trace, int(case.get("step_index") or 0))
    candidates = step.get("action_mask") or []
    wanted = {
        str((case.get("chosen") or {}).get("action_key") or ""),
        str((case.get("best_by_cashout") or {}).get("action_key") or ""),
    }
    rule_best = max(
        (candidate for candidate in case.get("candidates") or [] if candidate.get("action_key")),
        key=lambda candidate: float(candidate.get("rule_score") or 0.0),
        default=None,
    )
    if rule_best:
        wanted.add(str(rule_best.get("action_key") or ""))
    wanted.discard("")
    indices = [
        index
        for index, candidate in enumerate(candidates)
        if str(candidate.get("action_key") or "") in wanted
    ]
    if not indices:
        return list(range(min(len(candidates), 8)))
    return sorted(set(indices))


def run_counterfactual_case(
    *,
    args: argparse.Namespace,
    case: dict[str, Any],
    report_path: Path,
) -> dict[str, Any]:
    source = case["source_case"]
    trace_file = Path(str(source.get("trace_file") or ""))
    if not trace_file.is_absolute():
        trace_file = REPO_ROOT / trace_file
    if not trace_file.exists():
        return {
            "case_id": case["case_id"],
            "policy": case["policy"],
            "status": "failed",
            "error": f"missing trace file: {trace_file}",
        }
    try:
        branch_indices = candidate_indices_for_case(trace_file, source)
    except Exception as exc:
        return {
            "case_id": case["case_id"],
            "policy": case["policy"],
            "status": "failed",
            "error": f"failed to map branch indices: {exc}",
        }
    cmd = [
        sys.executable,
        str(REPO_ROOT / "tools" / "learning" / "full_run_counterfactual_lab.py"),
        "--trace-file",
        str(trace_file),
        "--step-index",
        str(int(source.get("step_index") or 0)),
        "--continuation-policy",
        args.continuation_policy,
        "--continuation-steps",
        str(args.continuation_steps),
        "--max-branches",
        str(args.max_branches),
        "--branch-indices",
        ",".join(str(index) for index in branch_indices),
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
            "case_id": case["case_id"],
            "policy": case["policy"],
            "status": "failed",
            "error": proc.stderr.strip() or proc.stdout.strip(),
            "command": cmd,
            "case_report_path": str(report_path),
            "branch_indices": branch_indices,
            "source_case": compact_source_case(source),
        }
    report = read_json(report_path)
    source_case = compact_source_case(source)
    verification = classify_case(args, source, report)
    return {
        "case_id": case["case_id"],
        "policy": case["policy"],
        "status": "ok",
        "case_report_path": str(report_path),
        "branch_indices": branch_indices,
        "source_case": source_case,
        "verification": verification,
        "calibrated_use": calibrated_use(source_case, verification),
        "counterfactual_summary": report.get("summary") or {},
    }


def compact_source_case(case: dict[str, Any]) -> dict[str, Any]:
    return {
        "seed": int(case.get("seed") or 0),
        "step_index": int(case.get("step_index") or 0),
        "act": int(case.get("act") or 0),
        "floor": int(case.get("floor") or 0),
        "hp": int(case.get("hp") or 0),
        "trace_file": str(case.get("trace_file") or ""),
        "chosen": case.get("chosen") or {},
        "best_by_cashout": case.get("best_by_cashout") or {},
        "cashout_gap": float(case.get("cashout_gap") or 0.0),
        "cashout_kinds": list(case.get("cashout_kinds") or []),
        "calibration_status": str(case.get("calibration_status") or "uncalibrated"),
        "needs_rollout": bool(case.get("needs_rollout")),
        "confidence": str(case.get("confidence") or ""),
        "notes": list(case.get("notes") or []),
        "calibration_notes": list(case.get("calibration_notes") or []),
    }


def outcome_by_key(report: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {str(row.get("candidate_key") or ""): row for row in report.get("outcomes") or []}


def classify_case(args: argparse.Namespace, source: dict[str, Any], report: dict[str, Any]) -> dict[str, Any]:
    by_key = outcome_by_key(report)
    chosen_key = str((source.get("chosen") or {}).get("action_key") or "")
    best_key = str((source.get("best_by_cashout") or {}).get("action_key") or "")
    chosen = by_key.get(chosen_key)
    cashout_best = by_key.get(best_key)
    ranked = sorted(report.get("outcomes") or [], key=outcome_sort_key, reverse=True)
    rank_by_key = {str(row.get("candidate_key") or ""): rank for rank, row in enumerate(ranked, start=1)}
    if chosen is None or cashout_best is None:
        return {
            "verdict": "inconclusive",
            "reason": "missing_chosen_or_cashout_best_branch",
            "chosen_key": chosen_key,
            "cashout_best_key": best_key,
            "available_keys": sorted(by_key),
            "rank_by_key": rank_by_key,
        }
    comparison = compare_outcomes(
        cashout_best,
        chosen,
        min_hp_margin=int(args.min_hp_margin),
        min_reward_margin=float(args.min_reward_margin),
    )
    if comparison["winner"] == "left":
        verdict = "cashout_confirmed"
    elif comparison["winner"] == "right":
        verdict = "cashout_refuted"
    elif comparison["winner"] == "equivalent":
        verdict = "equivalent"
    else:
        verdict = "inconclusive"
    return {
        "verdict": verdict,
        "comparison_reason": comparison["reason"],
        "chosen_key": chosen_key,
        "cashout_best_key": best_key,
        "chosen_rank": rank_by_key.get(chosen_key),
        "cashout_best_rank": rank_by_key.get(best_key),
        "ranked_keys": [str(row.get("candidate_key") or "") for row in ranked],
        "chosen_outcome": compact_outcome(chosen),
        "cashout_best_outcome": compact_outcome(cashout_best),
        "outcome_diff_cashout_minus_chosen": outcome_diff(cashout_best, chosen),
    }


def calibrated_use(source: dict[str, Any], verification: dict[str, Any]) -> dict[str, Any]:
    """Convert verifier verdicts into conservative downstream usage guidance.

    The key contract is intentionally strict: only a static high-confidence case
    that the simulator also confirms becomes a strong training signal. Everything
    from needs_rollout remains downweighted even if the short continuation agrees.
    """
    source_status = str(source.get("calibration_status") or "uncalibrated")
    verdict = str(verification.get("verdict") or "unknown")
    if source_status == "high_confidence_candidate" and verdict == "cashout_confirmed":
        return {
            "use": "verified_training_signal",
            "strong_training_signal": True,
            "suggested_weight": 1.0,
            "notes": [
                "static cashout was high confidence and short counterfactual continuation confirmed it"
            ],
        }
    if verdict == "cashout_confirmed":
        return {
            "use": "verified_but_downweighted",
            "strong_training_signal": False,
            "suggested_weight": 0.25,
            "notes": [
                f"source calibration was {source_status}; keep as diagnostic or low-weight auxiliary signal"
            ],
        }
    if verdict == "cashout_refuted":
        return {
            "use": "verified_refuted",
            "strong_training_signal": False,
            "suggested_weight": 0.0,
            "notes": [
                "short counterfactual continuation preferred the chosen action over cashout-best"
            ],
        }
    if verdict == "equivalent":
        return {
            "use": "verified_equivalent",
            "strong_training_signal": False,
            "suggested_weight": 0.0,
            "notes": [
                "cashout-best and chosen were below verifier margin; do not train as a preference"
            ],
        }
    return {
        "use": "needs_more_evidence",
        "strong_training_signal": False,
        "suggested_weight": 0.0,
        "notes": ["verifier result was inconclusive or unavailable"],
    }


def outcome_sort_key(row: dict[str, Any]) -> tuple[int, int, int, int, float]:
    end = row.get("end") or {}
    delta = row.get("outcome_delta") or {}
    return (
        terminal_class(end),
        int(delta.get("floor_delta") or 0),
        int(delta.get("combat_win_delta") or 0),
        int(end.get("current_hp") or 0),
        float(row.get("reward_total") or 0.0),
    )


def terminal_class(end: dict[str, Any]) -> int:
    result = str(end.get("result") or "")
    terminal_reason = str(end.get("terminal_reason") or "")
    if result == "victory" or terminal_reason == "victory":
        return 3
    if result == "ongoing" or terminal_reason in {"running", ""}:
        return 2
    if end.get("crash"):
        return 0
    return 1


def compare_outcomes(
    left: dict[str, Any],
    right: dict[str, Any],
    *,
    min_hp_margin: int,
    min_reward_margin: float,
) -> dict[str, str]:
    left_end = left.get("end") or {}
    right_end = right.get("end") or {}
    left_delta = left.get("outcome_delta") or {}
    right_delta = right.get("outcome_delta") or {}
    left_class = terminal_class(left_end)
    right_class = terminal_class(right_end)
    if left_class != right_class:
        return winner("left" if left_class > right_class else "right", "terminal_class")
    for field, reason in [
        ("floor_delta", "floor_delta"),
        ("combat_win_delta", "combat_win_delta"),
    ]:
        left_value = int(left_delta.get(field) or 0)
        right_value = int(right_delta.get(field) or 0)
        if left_value != right_value:
            return winner("left" if left_value > right_value else "right", reason)
    hp_diff = int(left_end.get("current_hp") or 0) - int(right_end.get("current_hp") or 0)
    if abs(hp_diff) >= min_hp_margin:
        return winner("left" if hp_diff > 0 else "right", "hp_margin")
    reward_diff = float(left.get("reward_total") or 0.0) - float(right.get("reward_total") or 0.0)
    if abs(reward_diff) >= min_reward_margin:
        return winner("left" if reward_diff > 0 else "right", "reward_margin")
    return winner("equivalent", "below_margin")


def winner(value: str, reason: str) -> dict[str, str]:
    return {"winner": value, "reason": reason}


def compact_outcome(row: dict[str, Any]) -> dict[str, Any]:
    card = row.get("candidate_card") or {}
    out = {
        "candidate_index": int(row.get("candidate_index") or 0),
        "candidate_key": str(row.get("candidate_key") or ""),
        "card_id": card.get("card_id") if isinstance(card, dict) else None,
        "floor_delta": int((row.get("outcome_delta") or {}).get("floor_delta") or 0),
        "combat_win_delta": int((row.get("outcome_delta") or {}).get("combat_win_delta") or 0),
        "hp_delta": int((row.get("outcome_delta") or {}).get("hp_delta") or 0),
        "end_floor": int((row.get("end") or {}).get("floor") or 0),
        "end_hp": int((row.get("end") or {}).get("current_hp") or 0),
        "end_result": (row.get("end") or {}).get("result"),
        "terminal_reason": (row.get("end") or {}).get("terminal_reason"),
        "reward_total": float(row.get("reward_total") or 0.0),
        "steps_taken": int(row.get("steps_taken") or 0),
    }
    if row.get("attribution"):
        out["attribution"] = row.get("attribution")
    return out


def outcome_diff(left: dict[str, Any], right: dict[str, Any]) -> dict[str, Any]:
    left_out = compact_outcome(left)
    right_out = compact_outcome(right)
    out = {
        "floor_delta": left_out["floor_delta"] - right_out["floor_delta"],
        "combat_win_delta": left_out["combat_win_delta"] - right_out["combat_win_delta"],
        "end_hp": left_out["end_hp"] - right_out["end_hp"],
        "reward_total": round(left_out["reward_total"] - right_out["reward_total"], 3),
    }
    left_attr = left_out.get("attribution") or {}
    right_attr = right_out.get("attribution") or {}
    if left_attr or right_attr:
        out["attribution"] = attribution_diff(left_attr, right_attr)
    return out


def attribution_diff(left: dict[str, Any], right: dict[str, Any]) -> dict[str, Any]:
    fields = [
        "hp_loss_observed",
        "monster_hp_reduction_observed",
        "alive_monster_reduction_observed",
        "combat_turns_observed",
        "combat_play_card_count",
        "combat_end_turn_count",
        "energy_unused_on_end_turn_total",
        "draw_pile_decrease_observed",
        "exhaust_count_increase_observed",
        "discard_count_increase_observed",
        "max_visible_incoming_damage",
        "max_visible_unblocked_damage",
    ]
    diff = {
        field: round(float(left.get(field) or 0.0) - float(right.get(field) or 0.0), 3)
        for field in fields
    }
    diff["scaling_played_delta"] = int(bool(left.get("scaling_played"))) - int(
        bool(right.get("scaling_played"))
    )
    diff["draw_played_delta"] = int(bool(left.get("draw_played"))) - int(bool(right.get("draw_played")))
    diff["exhaust_played_delta"] = int(bool(left.get("exhaust_played"))) - int(
        bool(right.get("exhaust_played"))
    )
    return diff


def summarize_results(results: list[dict[str, Any]]) -> dict[str, Any]:
    by_policy: dict[str, Counter[str]] = defaultdict(Counter)
    by_status: Counter[str] = Counter()
    by_source_status: dict[str, Counter[str]] = defaultdict(Counter)
    calibrated_by_policy: dict[str, Counter[str]] = defaultdict(Counter)
    strong_signals_by_policy: Counter[str] = Counter()
    for result in results:
        status = result.get("status")
        policy = str(result.get("policy") or "unknown")
        if status != "ok":
            verdict = "failed"
            source_status = "failed"
            calibrated = "verification_failed"
        else:
            verdict = str((result.get("verification") or {}).get("verdict") or "unknown")
            source_status = str((result.get("source_case") or {}).get("calibration_status") or "uncalibrated")
            calibrated = str((result.get("calibrated_use") or {}).get("use") or "unknown")
            if bool((result.get("calibrated_use") or {}).get("strong_training_signal")):
                strong_signals_by_policy[policy] += 1
        by_policy[policy][verdict] += 1
        by_status[status] += 1
        by_source_status[source_status][verdict] += 1
        calibrated_by_policy[policy][calibrated] += 1
    return {
        "case_count": len(results),
        "status_counts": dict(sorted(by_status.items())),
        "verdict_counts_by_policy": {
            policy: dict(sorted(counter.items())) for policy, counter in sorted(by_policy.items())
        },
        "verdict_counts_by_source_calibration": {
            status: dict(sorted(counter.items()))
            for status, counter in sorted(by_source_status.items())
        },
        "calibrated_use_counts_by_policy": {
            policy: dict(sorted(counter.items()))
            for policy, counter in sorted(calibrated_by_policy.items())
        },
        "strong_training_signal_counts_by_policy": dict(sorted(strong_signals_by_policy.items())),
        "calibration_rule": (
            "only high_confidence_candidate + cashout_confirmed is a strong training signal; "
            "needs_rollout remains downweighted even when confirmed"
        ),
    }


def verified_training_signal_cases(results: list[dict[str, Any]]) -> list[dict[str, Any]]:
    out = []
    for result in results:
        calibrated = result.get("calibrated_use") or {}
        if not calibrated.get("strong_training_signal"):
            continue
        source = result.get("source_case") or {}
        verification = result.get("verification") or {}
        out.append(
            {
                "policy": result.get("policy"),
                "case_id": result.get("case_id"),
                "seed": source.get("seed"),
                "step_index": source.get("step_index"),
                "floor": source.get("floor"),
                "chosen_card": (source.get("chosen") or {}).get("card_id"),
                "cashout_best_card": (source.get("best_by_cashout") or {}).get("card_id"),
                "cashout_gap": source.get("cashout_gap"),
                "cashout_kinds": source.get("cashout_kinds"),
                "comparison_reason": verification.get("comparison_reason"),
                "outcome_diff_cashout_minus_chosen": verification.get("outcome_diff_cashout_minus_chosen"),
                "suggested_weight": calibrated.get("suggested_weight"),
                "case_report_path": result.get("case_report_path"),
            }
        )
    return out


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# Cashout Counterfactual Verification V0",
        "",
        f"Generated: `{report['generated_at_utc']}`",
        "",
        "This report replays selected cashout cases and branches reward-card actions. It is a conservative verifier, not a teacher label.",
        "",
        "## Summary",
        "",
        f"- cases: `{report['summary']['case_count']}`",
        f"- statuses: `{report['summary']['status_counts']}`",
        f"- calibration rule: `{report['summary'].get('calibration_rule')}`",
        "",
        "| policy | confirmed | refuted | equivalent | inconclusive | failed |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    for policy, counts in sorted((report["summary"].get("verdict_counts_by_policy") or {}).items()):
        lines.append(
            "| {policy} | {confirmed} | {refuted} | {equivalent} | {inconclusive} | {failed} |".format(
                policy=policy,
                confirmed=counts.get("cashout_confirmed", 0),
                refuted=counts.get("cashout_refuted", 0),
                equivalent=counts.get("equivalent", 0),
                inconclusive=counts.get("inconclusive", 0) + counts.get("unknown", 0),
                failed=counts.get("failed", 0),
            )
        )
    lines.extend(["", "## Calibration Use", ""])
    lines.extend(
        [
            "| policy | verified training | downweighted confirmed | refuted | equivalent | more evidence |",
            "|---|---:|---:|---:|---:|---:|",
        ]
    )
    for policy, counts in sorted((report["summary"].get("calibrated_use_counts_by_policy") or {}).items()):
        lines.append(
            "| {policy} | {training} | {downweighted} | {refuted} | {equivalent} | {more} |".format(
                policy=policy,
                training=counts.get("verified_training_signal", 0),
                downweighted=counts.get("verified_but_downweighted", 0),
                refuted=counts.get("verified_refuted", 0),
                equivalent=counts.get("verified_equivalent", 0),
                more=counts.get("needs_more_evidence", 0) + counts.get("verification_failed", 0),
            )
        )
    lines.extend(["", "## Source Calibration Check", ""])
    lines.extend(
        [
            "| source status | confirmed | refuted | equivalent | inconclusive | failed |",
            "|---|---:|---:|---:|---:|---:|",
        ]
    )
    for status, counts in sorted((report["summary"].get("verdict_counts_by_source_calibration") or {}).items()):
        lines.append(
            "| {status} | {confirmed} | {refuted} | {equivalent} | {inconclusive} | {failed} |".format(
                status=status,
                confirmed=counts.get("cashout_confirmed", 0),
                refuted=counts.get("cashout_refuted", 0),
                equivalent=counts.get("equivalent", 0),
                inconclusive=counts.get("inconclusive", 0) + counts.get("unknown", 0),
                failed=counts.get("failed", 0),
            )
        )
    lines.extend(["", "## Verified Training Signal Cases", ""])
    signal_cases = report.get("verified_training_signal_cases") or []
    if not signal_cases:
        lines.append("- none")
    for case in signal_cases:
        diff = case.get("outcome_diff_cashout_minus_chosen") or {}
        lines.append(
            "- `{policy}` seed `{seed}` step `{step}` floor `{floor}`: `{chosen}` -> `{best}`, gap `{gap:.1f}`, by `{reason}`, diff floor `{floor_diff}` hp `{hp_diff}`".format(
                policy=case.get("policy"),
                seed=case.get("seed"),
                step=case.get("step_index"),
                floor=case.get("floor"),
                chosen=case.get("chosen_card"),
                best=case.get("cashout_best_card"),
                gap=float(case.get("cashout_gap") or 0.0),
                reason=case.get("comparison_reason"),
                floor_diff=diff.get("floor_delta"),
                hp_diff=diff.get("end_hp"),
            )
        )
    lines.extend(["", "## Cases", ""])
    for result in report.get("cases") or []:
        source = result.get("source_case") or {}
        chosen = source.get("chosen") or {}
        best = source.get("best_by_cashout") or {}
        if result.get("status") != "ok":
            lines.extend(
                [
                    "### `{}`".format(result.get("case_id")),
                    "",
                    f"- status: `{result.get('status')}`",
                    f"- error: `{result.get('error')}`",
                    "",
                ]
            )
            continue
        verification = result.get("verification") or {}
        calibrated = result.get("calibrated_use") or {}
        diff = verification.get("outcome_diff_cashout_minus_chosen") or {}
        lines.extend(
            [
                "### `{}`".format(result.get("case_id")),
                "",
                "- policy `{policy}` seed `{seed}` step `{step}` floor `{floor}` hp `{hp}`".format(
                    policy=result.get("policy"),
                    seed=source.get("seed"),
                    step=source.get("step_index"),
                    floor=source.get("floor"),
                    hp=source.get("hp"),
                ),
                "- cashout: chose `{chosen}` -> best `{best}`, gap `{gap:.1f}`, status `{status}`".format(
                    chosen=chosen.get("card_id") or chosen.get("action_key"),
                    best=best.get("card_id") or best.get("action_key"),
                    gap=float(source.get("cashout_gap") or 0.0),
                    status=source.get("calibration_status"),
                ),
                "- verifier: `{verdict}` by `{reason}`; chosen rank `{chosen_rank}`, cashout rank `{cashout_rank}`".format(
                    verdict=verification.get("verdict"),
                    reason=verification.get("comparison_reason"),
                    chosen_rank=verification.get("chosen_rank"),
                    cashout_rank=verification.get("cashout_best_rank"),
                ),
                "- calibrated use: `{use}`, strong `{strong}`, suggested weight `{weight}`".format(
                    use=calibrated.get("use"),
                    strong=calibrated.get("strong_training_signal"),
                    weight=calibrated.get("suggested_weight"),
                ),
                "- cashout minus chosen: floor `{floor}`, combats `{combats}`, end HP `{hp}`, reward `{reward}`".format(
                    floor=diff.get("floor_delta"),
                    combats=diff.get("combat_win_delta"),
                    hp=diff.get("end_hp"),
                    reward=diff.get("reward_total"),
                ),
                f"- case report: `{result.get('case_report_path')}`",
                "",
            ]
        )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def self_test() -> None:
    left = {
        "end": {"result": "ongoing", "terminal_reason": "running", "current_hp": 30},
        "outcome_delta": {"floor_delta": 1, "combat_win_delta": 1},
        "reward_total": 1.0,
    }
    right = {
        "end": {"result": "ongoing", "terminal_reason": "running", "current_hp": 20},
        "outcome_delta": {"floor_delta": 1, "combat_win_delta": 1},
        "reward_total": 1.0,
    }
    assert compare_outcomes(left, right, min_hp_margin=5, min_reward_margin=1.0)["winner"] == "left"
    assert compare_outcomes(right, left, min_hp_margin=5, min_reward_margin=1.0)["winner"] == "right"
    close = {
        "end": {"result": "ongoing", "terminal_reason": "running", "current_hp": 27},
        "outcome_delta": {"floor_delta": 1, "combat_win_delta": 1},
        "reward_total": 1.2,
    }
    assert compare_outcomes(left, close, min_hp_margin=5, min_reward_margin=1.0)["winner"] == "equivalent"
    assert calibrated_use(
        {"calibration_status": "high_confidence_candidate"},
        {"verdict": "cashout_confirmed"},
    )["strong_training_signal"]
    assert not calibrated_use(
        {"calibration_status": "needs_rollout"},
        {"verdict": "cashout_confirmed"},
    )["strong_training_signal"]
    print("self-test ok")


def main() -> int:
    args = parse_args()
    if args.self_test:
        self_test()
        return 0
    if args.cashout_report is None:
        raise SystemExit("--cashout-report is required unless --self-test is used")
    cashout_report = read_json(args.cashout_report)
    cases = iter_selected_cases(args, cashout_report)
    if not cases:
        raise SystemExit("no cashout cases selected for verification")
    case_report_dir = args.case_report_dir or args.out.parent / "case_reports" / datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    case_report_dir.mkdir(parents=True, exist_ok=True)
    results = []
    for case in cases:
        report_path = case_report_dir / f"{case['case_id']}.json"
        results.append(run_counterfactual_case(args=args, case=case, report_path=report_path))
    report = {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {
            "cashout_report": str(args.cashout_report),
            "policies": args.policies,
            "statuses": args.statuses,
            "max_cases": args.max_cases,
            "per_policy_limit": args.per_policy_limit,
            "continuation_policy": args.continuation_policy,
            "continuation_steps": args.continuation_steps,
            "max_branches": args.max_branches,
            "min_hp_margin": args.min_hp_margin,
            "min_reward_margin": args.min_reward_margin,
            "notes": [
                "cashout_confirmed/refuted requires a clear short-continuation margin",
                "equivalent/inconclusive cases are not training labels",
                "continuation policy is fixed and may bias outcomes",
            ],
        },
        "summary": summarize_results(results),
        "verified_training_signal_cases": verified_training_signal_cases(results),
        "cases": results,
    }
    write_json(args.out, report)
    markdown_out = args.markdown_out or args.out.with_suffix(".md")
    write_markdown(markdown_out, report)
    print(
        json.dumps(
            {
                "out": str(args.out),
                "markdown_out": str(markdown_out),
                "summary": report["summary"],
            },
            indent=2,
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
