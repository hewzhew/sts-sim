#!/usr/bin/env python3
"""Interpret offline model-guided search evidence bundles.

This is an abstain-first counterfactual evidence interpreter. It consumes
model_guided_search_evidence_bundle_v0 rows and asks:

    Did the collected branch evidence reveal a material alternative to the
    behavior branch under strict pairing / completeness rules?

It does not emit action labels, comparison winners, policy choices, or live
takeover decisions.
"""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter
from pathlib import Path
from typing import Any, Iterable


FORBIDDEN_OUTPUT_KEYS = {
    "winner",
    "preferred",
    "preferred_action",
    "selected_action",
    "teacher_choice",
}


def iter_jsonl(path: Path) -> Iterable[dict[str, Any]]:
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                yield json.loads(line)


def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return default
    return number if math.isfinite(number) else default


def safe_int(value: Any, default: int = 0) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def forced_action_id(trace: dict[str, Any]) -> int | None:
    forced = trace.get("forced_prefix") or []
    if not forced:
        return None
    value = forced[0]
    return value if isinstance(value, int) else None


def forced_action_summary(trace: dict[str, Any]) -> dict[str, Any]:
    action_id = forced_action_id(trace)
    candidates = trace.get("candidates") or []
    candidate = candidates[action_id] if isinstance(action_id, int) and 0 <= action_id < len(candidates) else {}
    payload = candidate.get("payload") or {}
    card = payload.get("card") if isinstance(payload.get("card"), dict) else {}
    return {
        "action_id": action_id,
        "action_kind": candidate.get("action_kind"),
        "action_key": candidate.get("action_key"),
        "card_id": card.get("card_id"),
    }


def complete_trace(trace: dict[str, Any]) -> bool:
    outcome = trace.get("outcome") or {}
    if outcome.get("outcome_censored"):
        return False
    if outcome.get("truncated") or trace.get("truncated"):
        return False
    if outcome.get("result") == "defeat":
        return True
    if outcome.get("boundary_requested") == "combat_end":
        return bool(outcome.get("boundary_reached"))
    return True


def trace_outcome_summary(trace: dict[str, Any]) -> dict[str, Any]:
    outcome = trace.get("outcome") or {}
    return {
        "result": outcome.get("result"),
        "hp": safe_int(outcome.get("hp")),
        "hp_delta": safe_int(outcome.get("hp_delta")),
        "combat_win_count": safe_int(outcome.get("combat_win_count")),
        "combat_win_delta": safe_int(outcome.get("combat_win_delta")),
        "total_reward": safe_float(outcome.get("total_reward")),
        "outcome_censored": bool(outcome.get("outcome_censored")),
        "truncated": bool(outcome.get("truncated")),
        "boundary_reached": bool(outcome.get("boundary_reached")),
        "stop_reason": outcome.get("stop_reason"),
    }


def strict_comparison_pairs(bundle: dict[str, Any]) -> set[frozenset[str]]:
    pairs: set[frozenset[str]] = set()
    batch = bundle.get("branch_trace_batch") or {}
    traces = {trace.get("branch_id"): trace for trace in batch.get("traces") or []}
    for comparison in batch.get("comparisons") or []:
        if comparison.get("pairing_valid") is not True:
            continue
        if comparison.get("rng_diverged") is not False:
            continue
        left_id = comparison.get("left_branch_id")
        right_id = comparison.get("right_branch_id")
        left = traces.get(left_id)
        right = traces.get(right_id)
        if not isinstance(left_id, str) or not isinstance(right_id, str):
            continue
        if left is None or right is None:
            continue
        if not complete_trace(left) or not complete_trace(right):
            continue
        pairs.add(frozenset((left_id, right_id)))
    return pairs


def material_reason(
    *,
    candidate: dict[str, Any],
    behavior: dict[str, Any],
    min_hp_margin: int,
    min_reward_margin: float,
) -> str | None:
    cand = candidate.get("outcome") or {}
    beh = behavior.get("outcome") or {}
    cand_dead = cand.get("result") == "defeat"
    beh_dead = beh.get("result") == "defeat"
    if not cand_dead and beh_dead:
        return "survival_flip"
    if cand_dead and not beh_dead:
        return None
    cand_combat = safe_int(cand.get("combat_win_count"))
    beh_combat = safe_int(beh.get("combat_win_count"))
    if cand_combat > beh_combat:
        return "combat_progress_flip"
    if cand_combat < beh_combat:
        return None
    hp_gain = safe_int(cand.get("hp")) - safe_int(beh.get("hp"))
    reward_gain = safe_float(cand.get("total_reward")) - safe_float(beh.get("total_reward"))
    if hp_gain >= min_hp_margin and reward_gain >= -abs(min_reward_margin):
        return "hp_margin"
    if reward_gain >= min_reward_margin and hp_gain >= 0:
        return "reward_margin"
    return None


def hp_bucket(value: int) -> str:
    if value <= -20:
        return "<=-20"
    if value <= -10:
        return "-19..-10"
    if value <= -5:
        return "-9..-5"
    if value <= -1:
        return "-4..-1"
    if value == 0:
        return "0"
    if value <= 4:
        return "1..4"
    if value <= 9:
        return "5..9"
    if value <= 19:
        return "10..19"
    return ">=20"


def interpret_bundle(
    bundle: dict[str, Any],
    *,
    min_hp_margin: int,
    min_reward_margin: float,
) -> dict[str, Any]:
    batch = bundle.get("branch_trace_batch") or {}
    traces = batch.get("traces") or []
    trace_by_id = {trace.get("branch_id"): trace for trace in traces if isinstance(trace.get("branch_id"), str)}
    behavior_action_id = bundle.get("behavior_action_id")
    behavior_trace = next(
        (
            trace
            for trace in traces
            if forced_action_id(trace) == behavior_action_id
        ),
        None,
    )
    base = {
        "schema_version": "model_guided_search_evidence_interpretation_v0",
        "trainable_role": "model_guided_search_evidence_interpretation",
        "trainable_as_action_label": False,
        "episode_seed": bundle.get("episode_seed"),
        "episode_step": bundle.get("episode_step"),
        "decision_type": bundle.get("decision_type"),
        "behavior_action": {
            "action_id": behavior_action_id,
            "action_key": bundle.get("behavior_action_key"),
        },
        "evidence_status": bundle.get("evidence_status"),
        "controller_decision": {
            "mode": "abstain",
            "reason": "offline_interpreter_only",
            "action_id": None,
            "trainable_as_action_label": False,
        },
        "label_policy": {
            "action_label": False,
            "source": "model_guided_search_evidence_interpreter_v0",
        },
    }
    if bundle.get("evidence_status") != "evidence_ready":
        return {
            **base,
            "interpretation_status": "abstain_partial_evidence",
            "abstain_reason": bundle.get("abstain_reason") or "input_evidence_not_ready",
        }
    if behavior_trace is None:
        return {
            **base,
            "interpretation_status": "abstain_missing_behavior_branch",
            "abstain_reason": "behavior_branch_not_collected",
        }
    if not complete_trace(behavior_trace):
        return {
            **base,
            "interpretation_status": "abstain_incomplete_behavior_branch",
            "abstain_reason": "behavior_branch_incomplete",
            "behavior_outcome": trace_outcome_summary(behavior_trace),
        }
    strict_pairs = strict_comparison_pairs(bundle)
    behavior_id = behavior_trace.get("branch_id")
    alternatives: list[dict[str, Any]] = []
    strict_candidate_count = 0
    for trace in traces:
        branch_id = trace.get("branch_id")
        if branch_id == behavior_id:
            continue
        if not isinstance(branch_id, str) or not isinstance(behavior_id, str):
            continue
        if frozenset((branch_id, behavior_id)) not in strict_pairs:
            continue
        strict_candidate_count += 1
        reason = material_reason(
            candidate=trace,
            behavior=behavior_trace,
            min_hp_margin=min_hp_margin,
            min_reward_margin=min_reward_margin,
        )
        cand_outcome = trace.get("outcome") or {}
        beh_outcome = behavior_trace.get("outcome") or {}
        hp_gain = safe_int(cand_outcome.get("hp")) - safe_int(beh_outcome.get("hp"))
        reward_gain = safe_float(cand_outcome.get("total_reward")) - safe_float(
            beh_outcome.get("total_reward")
        )
        combat_gain = safe_int(cand_outcome.get("combat_win_count")) - safe_int(
            beh_outcome.get("combat_win_count")
        )
        if reason is not None:
            alternatives.append(
                {
                    "branch_id": branch_id,
                    "audit_action": forced_action_summary(trace),
                    "material_reason": reason,
                    "hp_gain_vs_behavior": hp_gain,
                    "reward_gain_vs_behavior": reward_gain,
                    "combat_win_count_gain_vs_behavior": combat_gain,
                    "outcome": trace_outcome_summary(trace),
                }
            )

    if strict_candidate_count == 0:
        return {
            **base,
            "interpretation_status": "abstain_no_strict_behavior_comparison",
            "abstain_reason": "no_complete_rng_aligned_comparison_against_behavior",
            "behavior_outcome": trace_outcome_summary(behavior_trace),
        }
    alternatives.sort(
        key=lambda item: (
            -safe_int(item.get("combat_win_count_gain_vs_behavior")),
            -safe_int(item.get("hp_gain_vs_behavior")),
            -safe_float(item.get("reward_gain_vs_behavior")),
            str((item.get("audit_action") or {}).get("action_key")),
        )
    )
    if not alternatives:
        return {
            **base,
            "interpretation_status": "evidence_no_material_alternative",
            "abstain_reason": "no_material_alternative_under_strict_rules",
            "strict_candidate_count": strict_candidate_count,
            "behavior_outcome": trace_outcome_summary(behavior_trace),
        }
    best = alternatives[0]
    return {
        **base,
        "interpretation_status": "evidence_material_alternative_found",
        "abstain_reason": "material_alternative_requires_human_or_stronger_controller",
        "strict_candidate_count": strict_candidate_count,
        "material_alternative_count": len(alternatives),
        "best_counterfactual_for_audit": best,
        "behavior_outcome": trace_outcome_summary(behavior_trace),
    }


def assert_output_safe(row: dict[str, Any], *, index: int) -> None:
    if row.get("trainable_as_action_label") is not False:
        raise ValueError(f"output row {index} is action-label-like")
    if (row.get("label_policy") or {}).get("action_label") is not False:
        raise ValueError(f"output row {index} has action_label=true")
    serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
    for key in FORBIDDEN_OUTPUT_KEYS:
        if f'"{key}"' in serialized:
            raise ValueError(f"output row {index} contains forbidden key {key}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence-bundles", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--min-hp-margin", type=int, default=5)
    parser.add_argument("--min-reward-margin", type=float, default=0.25)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    status_counts: Counter[str] = Counter()
    reason_counts: Counter[str] = Counter()
    material_reason_counts: Counter[str] = Counter()
    audit_action_kind_counts: Counter[str] = Counter()
    hp_gain_buckets: Counter[str] = Counter()
    total_strict_candidates = 0
    rows_written = 0
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8") as handle:
        for index, bundle in enumerate(iter_jsonl(args.evidence_bundles)):
            interpretation = interpret_bundle(
                bundle,
                min_hp_margin=args.min_hp_margin,
                min_reward_margin=args.min_reward_margin,
            )
            assert_output_safe(interpretation, index=index)
            status = str(interpretation.get("interpretation_status") or "unknown")
            status_counts[status] += 1
            reason = interpretation.get("abstain_reason")
            if isinstance(reason, str):
                reason_counts[reason] += 1
            total_strict_candidates += safe_int(interpretation.get("strict_candidate_count"))
            best = interpretation.get("best_counterfactual_for_audit") or {}
            if isinstance(best, dict) and best:
                material_reason_counts[str(best.get("material_reason") or "unknown")] += 1
                action = best.get("audit_action") or {}
                audit_action_kind_counts[str(action.get("action_kind") or "unknown")] += 1
                hp_gain_buckets[hp_bucket(safe_int(best.get("hp_gain_vs_behavior")))] += 1
            handle.write(json.dumps(interpretation, separators=(",", ":")) + "\n")
            rows_written += 1

    summary = {
        "schema_version": "model_guided_search_evidence_interpreter_summary_v0",
        "evidence_bundles": str(args.evidence_bundles),
        "out": str(args.out),
        "bundle_count": rows_written,
        "min_hp_margin": args.min_hp_margin,
        "min_reward_margin": args.min_reward_margin,
        "status_counts": dict(status_counts),
        "abstain_reason_counts": dict(reason_counts),
        "material_reason_counts": dict(material_reason_counts),
        "audit_action_kind_counts": dict(audit_action_kind_counts),
        "best_counterfactual_hp_gain_histogram": dict(hp_gain_buckets),
        "total_strict_candidates_compared_to_behavior": total_strict_candidates,
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "interpreter_is_offline_counterfactual_audit_not_policy": True,
            "controller_decision_is_abstain_only": True,
        },
    }
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
