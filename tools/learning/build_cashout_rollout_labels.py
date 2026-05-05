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

from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from verify_cashout_counterfactuals import (
    candidate_source_cases,
    case_id,
    compact_outcome,
    compact_source_case,
    compare_outcomes,
    outcome_diff,
    outcome_sort_key,
    read_json,
    selected_policies,
)


REPORT_VERSION = "cashout_rollout_labeler_v1_2"
LABEL_MODE = "policy_horizon_paired_fixed_trace_replay"
GAME_RNG_MODE = "fixed_trace_replay"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Build policy-conditional rollout labels for card cashout cases. "
            "This turns needs_rollout into paired counterfactual evidence; it is "
            "not a policy-independent card-value teacher."
        )
    )
    parser.add_argument("--cashout-report", type=Path, required=True)
    parser.add_argument(
        "--policies",
        default="all",
        help="Comma-separated policy names from the cashout report, or 'all'.",
    )
    parser.add_argument(
        "--statuses",
        default="needs_rollout,high_confidence_candidate",
        help="Comma-separated source calibration statuses to label.",
    )
    parser.add_argument(
        "--continuation-policies",
        default="rule_baseline_v0,plan_query_v0",
        help="Comma-separated continuation policies.",
    )
    parser.add_argument(
        "--horizons",
        default="80,160",
        help="Comma-separated continuation step budgets.",
    )
    parser.add_argument("--rollouts-per-candidate", type=int, default=8)
    parser.add_argument("--max-cases", type=int, default=30)
    parser.add_argument("--per-policy-limit", type=int, default=30)
    parser.add_argument(
        "--case-ids",
        default="",
        help="Optional comma-separated case ids to keep after normal policy/status selection.",
    )
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
        "--out-dir",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "card_cashout_rollout_labels" / "v1",
    )
    return parser.parse_args()


def parse_csv(text: str) -> list[str]:
    return [part.strip() for part in text.split(",") if part.strip()]


def parse_int_csv(text: str) -> list[int]:
    out = []
    for part in parse_csv(text):
        value = int(part)
        if value <= 0:
            raise SystemExit("--horizons must contain positive integers")
        out.append(value)
    return out


def iter_selected_cases(args: argparse.Namespace, report: dict[str, Any]) -> list[dict[str, Any]]:
    wanted_policies = selected_policies(report, args.policies)
    wanted_statuses = set(parse_csv(args.statuses))
    wanted_case_ids = set(parse_csv(args.case_ids))
    if not wanted_statuses:
        raise SystemExit("--statuses must not be empty")
    cases: list[dict[str, Any]] = []
    per_policy_counts: Counter[str] = Counter()
    for policy in report.get("policies") or []:
        policy_name = str(policy.get("policy") or "")
        if policy_name not in wanted_policies:
            continue
        seen: set[tuple[str, int, str, str]] = set()
        for case in candidate_source_cases(policy):
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
            if wanted_case_ids and cases[-1]["case_id"] not in wanted_case_ids:
                cases.pop()
                continue
            per_policy_counts[policy_name] += 1
            if len(cases) >= args.max_cases:
                return cases
    return cases


def run_counterfactual(
    *,
    args: argparse.Namespace,
    case: dict[str, Any],
    continuation_policy: str,
    horizon: int,
    out_path: Path,
) -> dict[str, Any]:
    source = case["source_case"]
    trace_file = Path(str(source.get("trace_file") or ""))
    if not trace_file.is_absolute():
        trace_file = REPO_ROOT / trace_file
    if not trace_file.exists():
        return {
            "status": "failed",
            "error": f"missing trace file: {trace_file}",
            "case_id": case["case_id"],
            "continuation_policy": continuation_policy,
            "horizon": horizon,
        }

    cmd = [
        sys.executable,
        str(REPO_ROOT / "tools" / "learning" / "full_run_counterfactual_lab.py"),
        "--trace-file",
        str(trace_file),
        "--step-index",
        str(int(source.get("step_index") or 0)),
        "--continuation-policy",
        continuation_policy,
        "--continuation-steps",
        str(horizon),
        "--branch-indices",
        "all",
        "--max-branches",
        str(args.max_branches),
        "--ascension",
        str(args.ascension),
        "--class",
        args.player_class,
        "--max-steps",
        str(args.max_steps),
        "--out",
        str(out_path),
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
            "status": "failed",
            "error": proc.stderr.strip() or proc.stdout.strip(),
            "command": cmd,
            "case_id": case["case_id"],
            "continuation_policy": continuation_policy,
            "horizon": horizon,
            "case_report_path": str(out_path),
        }
    report = read_json(out_path)
    return {
        "status": "ok",
        "case_id": case["case_id"],
        "continuation_policy": continuation_policy,
        "horizon": horizon,
        "case_report_path": str(out_path),
        "report": report,
    }


def classify_policy_horizon(
    *,
    args: argparse.Namespace,
    source: dict[str, Any],
    report: dict[str, Any],
) -> dict[str, Any]:
    by_key = {str(row.get("candidate_key") or ""): row for row in report.get("outcomes") or []}
    chosen_key = str((source.get("chosen") or {}).get("action_key") or "")
    best_key = str((source.get("best_by_cashout") or {}).get("action_key") or "")
    ranked = sorted(report.get("outcomes") or [], key=outcome_sort_key, reverse=True)
    rank_by_key = {str(row.get("candidate_key") or ""): rank for rank, row in enumerate(ranked, start=1)}
    chosen = by_key.get(chosen_key)
    cashout_best = by_key.get(best_key)
    if chosen is None or cashout_best is None:
        return {
            "verdict": "inconclusive",
            "reason": "missing_chosen_or_cashout_best_branch",
            "chosen_key": chosen_key,
            "cashout_best_key": best_key,
            "available_keys": sorted(by_key),
            "ranked_keys": [str(row.get("candidate_key") or "") for row in ranked],
            "rank_by_key": rank_by_key,
        }
    comparison = compare_outcomes(
        cashout_best,
        chosen,
        min_hp_margin=int(args.min_hp_margin),
        min_reward_margin=float(args.min_reward_margin),
    )
    if comparison["winner"] == "left":
        verdict = "rollout_confirmed"
    elif comparison["winner"] == "right":
        verdict = "rollout_refuted"
    elif comparison["winner"] == "equivalent":
        verdict = "rollout_equivalent"
    else:
        verdict = "inconclusive"
    return {
        "verdict": verdict,
        "reason": comparison["reason"],
        "chosen_key": chosen_key,
        "cashout_best_key": best_key,
        "chosen_rank": rank_by_key.get(chosen_key),
        "cashout_best_rank": rank_by_key.get(best_key),
        "ranked_keys": [str(row.get("candidate_key") or "") for row in ranked],
        "rank_by_key": rank_by_key,
        "chosen_outcome": compact_outcome(chosen),
        "cashout_best_outcome": compact_outcome(cashout_best),
        "outcome_diff_cashout_minus_chosen": outcome_diff(cashout_best, chosen),
    }


def candidate_outcome_rows(
    *,
    source_policy: str,
    case_id_value: str,
    source: dict[str, Any],
    continuation_policy: str,
    horizon: int,
    report: dict[str, Any],
) -> list[dict[str, Any]]:
    rows = []
    for row in report.get("outcomes") or []:
        compact = compact_outcome(row)
        rows.append(
            {
                "label_mode": LABEL_MODE,
                "game_rng_mode": GAME_RNG_MODE,
                "source_policy": source_policy,
                "case_id": case_id_value,
                "seed": int(source.get("seed") or 0),
                "step_index": int(source.get("step_index") or 0),
                "source_calibration_status": str(source.get("calibration_status") or "uncalibrated"),
                "continuation_policy": continuation_policy,
                "horizon": horizon,
                "candidate": compact,
                "source_chosen_key": str((source.get("chosen") or {}).get("action_key") or ""),
                "source_cashout_best_key": str((source.get("best_by_cashout") or {}).get("action_key") or ""),
            }
        )
    return rows


def pairwise_edges(
    *,
    args: argparse.Namespace,
    source_policy: str,
    case_id_value: str,
    source: dict[str, Any],
    continuation_policy: str,
    horizon: int,
    report: dict[str, Any],
) -> list[dict[str, Any]]:
    outcomes = list(report.get("outcomes") or [])
    edges = []
    for left_index, left in enumerate(outcomes):
        for right in outcomes[left_index + 1 :]:
            comparison = compare_outcomes(
                left,
                right,
                min_hp_margin=int(args.min_hp_margin),
                min_reward_margin=float(args.min_reward_margin),
            )
            if comparison["winner"] not in {"left", "right"}:
                continue
            preferred = left if comparison["winner"] == "left" else right
            rejected = right if comparison["winner"] == "left" else left
            edges.append(
                {
                    "label_mode": LABEL_MODE,
                    "game_rng_mode": GAME_RNG_MODE,
                    "source_policy": source_policy,
                    "case_id": case_id_value,
                    "seed": int(source.get("seed") or 0),
                    "step_index": int(source.get("step_index") or 0),
                    "source_calibration_status": str(source.get("calibration_status") or "uncalibrated"),
                    "continuation_policy": continuation_policy,
                    "horizon": horizon,
                    "preferred_key": str(preferred.get("candidate_key") or ""),
                    "rejected_key": str(rejected.get("candidate_key") or ""),
                    "reason": comparison["reason"],
                    "preferred_outcome": compact_outcome(preferred),
                    "rejected_outcome": compact_outcome(rejected),
                    "outcome_diff_preferred_minus_rejected": outcome_diff(preferred, rejected),
                }
            )
    return edges


def aggregate_case_label(case: dict[str, Any], observations: list[dict[str, Any]]) -> dict[str, Any]:
    successful = [row for row in observations if row.get("status") == "ok"]
    verdicts = [str((row.get("classification") or {}).get("verdict") or "inconclusive") for row in successful]
    confirmed = [row for row in successful if (row.get("classification") or {}).get("verdict") == "rollout_confirmed"]
    refuted = [row for row in successful if (row.get("classification") or {}).get("verdict") == "rollout_refuted"]
    equivalent = [row for row in successful if (row.get("classification") or {}).get("verdict") == "rollout_equivalent"]
    failed = [row for row in observations if row.get("status") != "ok"]

    if confirmed and refuted:
        label = "rollout_unstable"
    elif confirmed:
        policies = {str(row.get("continuation_policy") or "") for row in confirmed}
        horizons = {int(row.get("horizon") or 0) for row in confirmed}
        rule_refuted = any(
            row.get("continuation_policy") == "rule_baseline_v0"
            and (row.get("classification") or {}).get("verdict") == "rollout_refuted"
            for row in successful
        )
        if {"rule_baseline_v0", "plan_query_v0"}.issubset(policies) and len(horizons) >= 2:
            label = "robust_confirmed"
        elif "plan_query_v0" in policies and "rule_baseline_v0" not in policies and not rule_refuted:
            label = "requires_cashout_policy"
        else:
            label = "rollout_confirmed"
    elif refuted:
        label = "rollout_refuted"
    elif equivalent and len(equivalent) == len(successful) and successful:
        label = "rollout_equivalent"
    elif failed and not successful:
        label = "rollout_failed"
    else:
        label = "inconclusive"

    diagnostics = label_diagnostics(label=label, successful=successful, source=case["source_case"])
    strong = label == "robust_confirmed"
    source = compact_source_case(case["source_case"])
    return {
        "case_id": case["case_id"],
        "source_policy": case["policy"],
        "label_mode": LABEL_MODE,
        "game_rng_mode": GAME_RNG_MODE,
        "label_status": label,
        "label_substatus": diagnostics["label_substatus"],
        "diagnostic_tags": diagnostics["diagnostic_tags"],
        "strong_training_signal": strong,
        "suggested_weight": 1.0 if strong else (0.35 if label == "requires_cashout_policy" else 0.0),
        "source_case": source,
        "observations": observations,
        "verdict_counts": dict(sorted(Counter(verdicts).items())),
        "policy_verdict_counts": diagnostics["policy_verdict_counts"],
        "horizon_verdict_counts": diagnostics["horizon_verdict_counts"],
        "notes": label_notes(label, diagnostics["label_substatus"]),
    }


def verdict(row: dict[str, Any]) -> str:
    return str((row.get("classification") or {}).get("verdict") or "inconclusive")


def label_diagnostics(
    *,
    label: str,
    successful: list[dict[str, Any]],
    source: dict[str, Any],
) -> dict[str, Any]:
    by_policy: dict[str, Counter[str]] = defaultdict(Counter)
    by_horizon: dict[str, Counter[str]] = defaultdict(Counter)
    for row in successful:
        by_policy[str(row.get("continuation_policy") or "unknown")][verdict(row)] += 1
        by_horizon[str(row.get("horizon") or "unknown")][verdict(row)] += 1

    policy_verdict_sets = {
        policy: {item for item, count in counts.items() if count > 0 and item != "inconclusive"}
        for policy, counts in by_policy.items()
    }
    horizon_verdict_sets = {
        horizon: {item for item, count in counts.items() if count > 0 and item != "inconclusive"}
        for horizon, counts in by_horizon.items()
    }
    tags: list[str] = []

    best = source.get("best_by_cashout") or {}
    best_score = float(best.get("cashout_score") or 0.0)
    best_grade = str(best.get("cashout_grade") or "")
    verdict_counts = Counter(verdict(row) for row in successful)
    confirmed = verdict_counts.get("rollout_confirmed", 0)
    refuted = verdict_counts.get("rollout_refuted", 0)
    equivalent = verdict_counts.get("rollout_equivalent", 0)

    if best_score >= 60 or best_grade in {"medium", "high"}:
        tags.append("static_high_cashout")
    elif best_score < 30:
        tags.append("static_low_or_speculative_cashout")
    if equivalent >= max(confirmed, refuted, 1):
        tags.append("many_equivalent_verdicts")

    policies_with_confirm = {
        policy for policy, counts in by_policy.items() if counts.get("rollout_confirmed", 0) > 0
    }
    policies_with_refute = {
        policy for policy, counts in by_policy.items() if counts.get("rollout_refuted", 0) > 0
    }
    horizons_with_confirm = {
        horizon for horizon, counts in by_horizon.items() if counts.get("rollout_confirmed", 0) > 0
    }
    horizons_with_refute = {
        horizon for horizon, counts in by_horizon.items() if counts.get("rollout_refuted", 0) > 0
    }

    policy_conflict = bool(policies_with_confirm and policies_with_refute)
    if len(policies_with_confirm | policies_with_refute) >= 2 and policy_conflict:
        tags.append("continuation_policy_conflict")
    if any(
        {"rollout_confirmed", "rollout_refuted"}.issubset(items)
        for items in policy_verdict_sets.values()
    ) or (len(horizons_with_confirm | horizons_with_refute) >= 2 and horizons_with_confirm and horizons_with_refute):
        tags.append("horizon_sensitive")
    if label in {"rollout_unstable", "rollout_refuted"} and best_score >= 60 and refuted > 0:
        tags.append("static_high_but_policy_unrealized")

    if label == "rollout_unstable":
        if best_score < 30 and equivalent >= 1:
            substatus = "weak_margin_equivalent"
        elif "continuation_policy_conflict" in tags:
            substatus = "continuation_policy_conflict"
        elif "horizon_sensitive" in tags:
            substatus = "horizon_sensitive"
        elif "static_high_but_policy_unrealized" in tags:
            substatus = "static_high_but_policy_unrealized"
        else:
            substatus = "mixed_unstable"
    elif label == "rollout_equivalent":
        substatus = "weak_margin_equivalent"
    elif label == "rollout_refuted" and best_score >= 60:
        substatus = "static_high_but_policy_unrealized"
        if substatus not in tags:
            tags.append(substatus)
    else:
        substatus = label

    if substatus not in tags and substatus not in {
        "robust_confirmed",
        "rollout_confirmed",
        "requires_cashout_policy",
        "rollout_failed",
        "inconclusive",
    }:
        tags.append(substatus)

    return {
        "label_substatus": substatus,
        "diagnostic_tags": sorted(set(tags)),
        "policy_verdict_counts": {
            key: dict(sorted(counter.items())) for key, counter in sorted(by_policy.items())
        },
        "horizon_verdict_counts": {
            key: dict(sorted(counter.items())) for key, counter in sorted(by_horizon.items())
        },
    }


def label_notes(label: str, substatus: str = "") -> list[str]:
    if label == "robust_confirmed":
        return [
            "cashout-best beat chosen across multiple continuation policy/horizon settings",
            "eligible as a strong preference label if downstream filters also accept the case",
        ]
    if label == "requires_cashout_policy":
        return [
            "cashout-best only paid off under plan_query_v0 or was not confirmed by rule_baseline_v0",
            "use as a diagnostic for better continuation, not as a policy-independent card label",
        ]
    if label == "rollout_unstable":
        if substatus == "continuation_policy_conflict":
            return ["continuation policies disagreed; diagnose cashout utilization before training"]
        if substatus == "horizon_sensitive":
            return ["short and long horizons disagreed; treat as horizon-sensitive, not a hard label"]
        if substatus == "weak_margin_equivalent":
            return ["effect margins are weak or mostly equivalent; use for calibration, not preference"]
        if substatus == "static_high_but_policy_unrealized":
            return ["static cashout is high but at least one continuation cannot realize it"]
        return ["continuation policies or horizons disagreed; do not train as a hard preference"]
    if label == "rollout_refuted":
        return ["paired continuation preferred the source chosen action over static cashout-best"]
    if label == "rollout_equivalent":
        return ["cashout-best and chosen were below configured effect margins"]
    return ["insufficient rollout evidence"]


def summarize(labels: list[dict[str, Any]], candidate_rows: list[dict[str, Any]], edges: list[dict[str, Any]]) -> dict[str, Any]:
    label_counts = Counter(str(label.get("label_status") or "unknown") for label in labels)
    substatus_counts = Counter(str(label.get("label_substatus") or "unknown") for label in labels)
    source_counts: dict[str, Counter[str]] = defaultdict(Counter)
    policy_horizon_counts: dict[str, Counter[str]] = defaultdict(Counter)
    for label in labels:
        source_status = str((label.get("source_case") or {}).get("calibration_status") or "uncalibrated")
        source_counts[source_status][str(label.get("label_status") or "unknown")] += 1
        for obs in label.get("observations") or []:
            key = f"{obs.get('continuation_policy')}@{obs.get('horizon')}"
            verdict = str((obs.get("classification") or {}).get("verdict") or obs.get("status") or "unknown")
            policy_horizon_counts[key][verdict] += 1
    attr_rows = [
        (row.get("candidate") or {}).get("attribution") or {}
        for row in candidate_rows
        if (row.get("candidate") or {}).get("attribution")
    ]
    return {
        "case_count": len(labels),
        "label_status_counts": dict(sorted(label_counts.items())),
        "label_substatus_counts": dict(sorted(substatus_counts.items())),
        "source_status_label_counts": {
            key: dict(sorted(counter.items())) for key, counter in sorted(source_counts.items())
        },
        "policy_horizon_verdict_counts": {
            key: dict(sorted(counter.items())) for key, counter in sorted(policy_horizon_counts.items())
        },
        "candidate_outcome_row_count": len(candidate_rows),
        "pairwise_label_count": len(edges),
        "strong_training_signal_count": sum(1 for label in labels if label.get("strong_training_signal")),
        "requires_cashout_policy_count": label_counts.get("requires_cashout_policy", 0),
        "candidate_attribution_summary": summarize_candidate_attributions(attr_rows),
        "contract": (
            "needs_rollout is not a label; only rollout-produced robust_confirmed labels "
            "are strong training signals"
        ),
    }


def summarize_candidate_attributions(rows: list[dict[str, Any]]) -> dict[str, Any]:
    if not rows:
        return {"row_count": 0}
    fields = [
        "hp_loss_observed",
        "monster_hp_reduction_observed",
        "alive_monster_reduction_observed",
        "combat_turns_observed",
        "combat_play_card_count",
        "energy_unused_on_end_turn_total",
        "draw_pile_decrease_observed",
        "exhaust_count_increase_observed",
    ]
    return {
        "row_count": len(rows),
        "averages": {
            field: round(sum(float(row.get(field) or 0.0) for row in rows) / len(rows), 3)
            for field in fields
        },
        "scaling_played_count": sum(1 for row in rows if row.get("scaling_played")),
        "draw_played_count": sum(1 for row in rows if row.get("draw_played")),
        "exhaust_played_count": sum(1 for row in rows if row.get("exhaust_played")),
        "observability": "derived from before/after full-run observations; not an engine event log",
    }


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# Cashout Rollout Labels V1",
        "",
        f"Generated: `{report['generated_at_utc']}`",
        "",
        "This report converts cashout cases into policy-conditional paired rollout evidence. It is not a policy-independent card-value oracle.",
        "",
        "## Summary",
        "",
        f"- cases: `{report['summary']['case_count']}`",
        f"- label counts: `{report['summary']['label_status_counts']}`",
        f"- label substatus counts: `{report['summary'].get('label_substatus_counts', {})}`",
        f"- strong labels: `{report['summary']['strong_training_signal_count']}`",
        f"- requires cashout policy: `{report['summary']['requires_cashout_policy_count']}`",
        f"- attribution rows: `{(report['summary'].get('candidate_attribution_summary') or {}).get('row_count', 0)}`",
        f"- RNG mode: `{report['config']['game_rng_mode']}`",
        "",
        "## Limitations",
        "",
    ]
    for limitation in report.get("limitations") or []:
        lines.append(f"- {limitation}")
    lines.extend(["", "## Labels", ""])
    for label in report.get("labels") or []:
        source = label.get("source_case") or {}
        chosen = source.get("chosen") or {}
        best = source.get("best_by_cashout") or {}
        lines.extend(
            [
                f"### `{label.get('case_id')}`",
                "",
                "- `{status}` / `{substatus}` source `{source_policy}` seed `{seed}` step `{step}` floor `{floor}`: `{chosen}` -> cashout `{best}` gap `{gap:.1f}`".format(
                    status=label.get("label_status"),
                    substatus=label.get("label_substatus"),
                    source_policy=label.get("source_policy"),
                    seed=source.get("seed"),
                    step=source.get("step_index"),
                    floor=source.get("floor"),
                    chosen=chosen.get("card_id") or chosen.get("action_key"),
                    best=best.get("card_id") or best.get("action_key"),
                    gap=float(source.get("cashout_gap") or 0.0),
                ),
                f"- verdict counts: `{label.get('verdict_counts')}`",
                f"- diagnostic tags: `{label.get('diagnostic_tags', [])}`",
                f"- suggested weight: `{label.get('suggested_weight')}`",
                "",
            ]
        )
        for obs in label.get("observations") or []:
            classification = obs.get("classification") or {}
            diff = classification.get("outcome_diff_cashout_minus_chosen") or {}
            attr = diff.get("attribution") or {}
            lines.append(
                "  - `{policy}` h`{horizon}`: `{verdict}` by `{reason}`, rank chosen `{chosen_rank}` cashout `{cashout_rank}`, diff floor `{floor}` hp `{hp}`, attr hp_loss `{attr_hp_loss}` monster_hp `{monster_hp}` kills `{kills}`".format(
                    policy=obs.get("continuation_policy"),
                    horizon=obs.get("horizon"),
                    verdict=classification.get("verdict") or obs.get("status"),
                    reason=classification.get("reason") or obs.get("error"),
                    chosen_rank=classification.get("chosen_rank"),
                    cashout_rank=classification.get("cashout_best_rank"),
                    floor=diff.get("floor_delta"),
                    hp=diff.get("end_hp"),
                    attr_hp_loss=attr.get("hp_loss_observed"),
                    monster_hp=attr.get("monster_hp_reduction_observed"),
                    kills=attr.get("alive_monster_reduction_observed"),
                )
            )
        lines.append("")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    args = parse_args()
    continuation_policies = parse_csv(args.continuation_policies)
    horizons = parse_int_csv(args.horizons)
    if not continuation_policies:
        raise SystemExit("--continuation-policies must not be empty")
    cashout_report = read_json(args.cashout_report)
    cases = iter_selected_cases(args, cashout_report)

    case_report_dir = args.out_dir / "case_reports"
    case_report_dir.mkdir(parents=True, exist_ok=True)
    labels: list[dict[str, Any]] = []
    candidate_rows: list[dict[str, Any]] = []
    pairwise_rows: list[dict[str, Any]] = []

    for case in cases:
        observations: list[dict[str, Any]] = []
        source = case["source_case"]
        for continuation_policy in continuation_policies:
            for horizon in horizons:
                report_path = (
                    case_report_dir
                    / f"{case['case_id']}__{continuation_policy}__h{horizon}.json"
                )
                result = run_counterfactual(
                    args=args,
                    case=case,
                    continuation_policy=continuation_policy,
                    horizon=horizon,
                    out_path=report_path,
                )
                if result["status"] != "ok":
                    observations.append(result)
                    continue
                report = result["report"]
                classification = classify_policy_horizon(args=args, source=source, report=report)
                observations.append(
                    {
                        "status": "ok",
                        "continuation_policy": continuation_policy,
                        "horizon": horizon,
                        "case_report_path": str(report_path),
                        "classification": classification,
                        "summary": report.get("summary") or {},
                    }
                )
                candidate_rows.extend(
                    candidate_outcome_rows(
                        source_policy=case["policy"],
                        case_id_value=case["case_id"],
                        source=source,
                        continuation_policy=continuation_policy,
                        horizon=horizon,
                        report=report,
                    )
                )
                pairwise_rows.extend(
                    pairwise_edges(
                        args=args,
                        source_policy=case["policy"],
                        case_id_value=case["case_id"],
                        source=source,
                        continuation_policy=continuation_policy,
                        horizon=horizon,
                        report=report,
                    )
                )
        labels.append(aggregate_case_label(case, observations))

    report = {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {
            "cashout_report": str(args.cashout_report),
            "policies": args.policies,
            "statuses": parse_csv(args.statuses),
            "continuation_policies": continuation_policies,
            "horizons": horizons,
            "rollouts_per_candidate_requested": int(args.rollouts_per_candidate),
            "effective_rollouts_per_candidate": 1,
            "game_rng_mode": GAME_RNG_MODE,
            "label_mode": LABEL_MODE,
            "min_hp_margin": int(args.min_hp_margin),
            "min_reward_margin": float(args.min_reward_margin),
            "max_cases": int(args.max_cases),
            "max_branches": int(args.max_branches),
        },
        "summary": summarize(labels, candidate_rows, pairwise_rows),
        "limitations": [
            "future game RNG perturbation is not exposed yet; deterministic continuations are paired under fixed trace replay",
            "all labels are conditional on continuation policy and horizon",
            "plan_query_v0 is current-turn-only combat continuation with noncombat fallback to rule_baseline_v0",
            "needs_rollout is a queue status, not a training label",
            "only robust_confirmed is marked as a strong training signal",
            "rollout attribution is derived from before/after observation deltas and is not exact engine event attribution",
        ],
        "labels": labels,
    }

    args.out_dir.mkdir(parents=True, exist_ok=True)
    write_json(args.out_dir / "cashout_rollout_label_report.json", report)
    write_jsonl(args.out_dir / "candidate_outcomes.jsonl", candidate_rows)
    write_jsonl(args.out_dir / "pairwise_labels.jsonl", pairwise_rows)
    write_markdown(args.out_dir / "cashout_rollout_label_report.md", report)


if __name__ == "__main__":
    main()
