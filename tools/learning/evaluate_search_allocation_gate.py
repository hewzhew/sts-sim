#!/usr/bin/env python3
"""Formal search-allocation gate over branch/pair prediction artifacts.

This is not a policy evaluator. It does not choose actions, create action
labels, or convert pairwise outcome differences into preferences. The gate asks
only whether model/audit signals allocate limited deeper-search budget to
branches or pairs that later proved material under branch-outcome labels.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable


FORBIDDEN_LABEL_KEYS = {
    "winner",
    "preferred",
    "preferred_action",
    "selected_action",
    "teacher_choice",
}

BRANCH_OBJECTIVES = (
    "best_total_reward",
    "best_hp_delta",
    "worst_hp_delta",
    "hp_loss_ge_5",
    "hp_loss_ge_10",
)

PAIR_OBJECTIVES = (
    "abs_hp_diff_ge_5",
    "abs_hp_diff_ge_10",
    "abs_hp_diff_ge_15",
    "branch_model_severe_underestimate_ge_10",
    "residual_model_severe_underestimate_ge_10",
    "end_turn_play_card_abs_hp_diff_ge_10",
)

BRANCH_SCORE_NAMES = (
    "input_order_high",
    "random_hash_high",
    "pred_total_reward_high",
    "pred_hp_delta_high",
    "pred_hp_loss_ge_5_risk_high",
    "pred_hp_loss_ge_10_risk_high",
    "value_or_risk_attention_high",
    "end_turn_risk_baseline_high",
    "play_card_value_baseline_high",
    "allocation_hp_loss_ge_10_probability_high",
    "allocation_branch_priority_high",
)

PAIR_SCORE_NAMES = (
    "input_order_high",
    "random_hash_high",
    "branch_abs_hp_diff_high",
    "residual_abs_hp_diff_high",
    "residual_branch_gap_high",
    "tail_abs_ge_10_probability_high",
    "tail_directional_ge_10_probability_high",
    "tail_abs_ge_15_probability_high",
    "combined_tail_attention_high",
    "end_turn_play_card_baseline_high",
    "allocation_abs_ge_10_probability_high",
    "allocation_abs_ge_15_probability_high",
    "allocation_branch_severe_probability_high",
    "allocation_residual_severe_probability_high",
    "allocation_end_turn_play_card_abs_ge_10_probability_high",
    "allocation_pair_required_probability_high",
    "allocation_pair_watch_probability_high",
    "allocation_pair_priority_high",
)


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
    if not math.isfinite(number):
        return default
    return number


def safe_prob(value: Any) -> float:
    return min(1.0, max(0.0, safe_float(value, 0.0)))


def decision_key(row: dict[str, Any]) -> str:
    return json.dumps(
        {
            "episode_seed": row.get("episode_seed"),
            "episode_step": row.get("episode_step"),
            "decision_id": row.get("decision_id"),
        },
        sort_keys=True,
        separators=(",", ":"),
    )


def stable_hash_unit(text: str) -> float:
    digest = hashlib.blake2b(text.encode("utf-8"), digest_size=8).digest()
    return int.from_bytes(digest, byteorder="big") / float(2**64 - 1)


def assert_no_action_label_leak(row: dict[str, Any], *, row_kind: str, row_index: int) -> None:
    if row.get("trainable_as_action_label") is not False:
        raise ValueError(f"{row_kind} row {row_index} is action-label-like")
    if (row.get("label_policy") or {}).get("action_label") is not False:
        raise ValueError(f"{row_kind} row {row_index} has action_label=true")
    serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
    for key in FORBIDDEN_LABEL_KEYS:
        if f'"{key}"' in serialized:
            raise ValueError(f"{row_kind} row {row_index} contains forbidden key {key}")


def load_rows(path: Path, row_kind: str) -> list[dict[str, Any]]:
    rows = list(iter_jsonl(path))
    for index, row in enumerate(rows):
        assert_no_action_label_leak(row, row_kind=row_kind, row_index=index)
    return rows


def parse_int_list(text: str) -> list[int]:
    values: list[int] = []
    for part in text.split(","):
        part = part.strip()
        if part:
            values.append(int(part))
    return sorted(set(values))


def item_key(row: dict[str, Any], index: int) -> str:
    if row.get("branch_id") is not None:
        candidate = row.get("candidate") or {}
        return str(row.get("branch_id") or candidate.get("action_key") or index)
    left = row.get("left") or {}
    right = row.get("right") or {}
    return "|".join(
        [
            str(row.get("comparison_id") or ""),
            str((left.get("candidate") or {}).get("action_key")),
            str((right.get("candidate") or {}).get("action_key")),
            str(index),
        ]
    )


def action_summary(candidate: dict[str, Any]) -> dict[str, Any]:
    return {
        "action_kind": candidate.get("action_kind"),
        "action_type": candidate.get("action_type"),
        "action_key": candidate.get("action_key"),
        "card_id": candidate.get("card_id"),
    }


def branch_item_summary(row: dict[str, Any]) -> dict[str, Any]:
    return {
        "branch_id": row.get("branch_id"),
        "candidate": action_summary(row.get("candidate") or {}),
        "targets": row.get("targets") or {},
    }


def pair_item_summary(row: dict[str, Any]) -> dict[str, Any]:
    return {
        "comparison_id": row.get("comparison_id"),
        "left_branch_id": (row.get("left") or {}).get("branch_id"),
        "right_branch_id": (row.get("right") or {}).get("branch_id"),
        "left_candidate": action_summary(((row.get("left") or {}).get("candidate") or {})),
        "right_candidate": action_summary(((row.get("right") or {}).get("candidate") or {})),
        "targets": row.get("targets") or {},
    }


def branch_scores(row: dict[str, Any], *, index: int) -> dict[str, float]:
    signals = row.get("search_allocation_signals") or {}
    outputs = row.get("model_outputs") or {}
    risks = outputs.get("risks") or {}
    risk5 = risks.get("hp_loss_ge_5") or {}
    risk10 = risks.get("hp_loss_ge_10") or {}
    candidate = row.get("candidate") or {}
    context = row.get("decision_context") or {}
    is_end_turn = candidate.get("action_kind") == "end_turn"
    is_play_card = candidate.get("action_kind") == "play_card"
    predicted_reward = safe_float(outputs.get("total_reward"))
    predicted_hp = safe_float(outputs.get("hp_delta"))
    risk10_prob = safe_prob(risk10.get("probability"))
    end_turn_risk = 0.0
    if is_end_turn:
        end_turn_risk += 1.0 if context.get("end_turn_with_playable_cards") else 0.0
        end_turn_risk += 1.0 if context.get("end_turn_with_unspent_energy") else 0.0
        end_turn_risk += max(0.0, safe_float(context.get("incoming_minus_current_block"))) / 20.0
    return {
        "input_order_high": -float(index),
        "random_hash_high": stable_hash_unit(item_key(row, index)),
        "pred_total_reward_high": predicted_reward,
        "pred_hp_delta_high": predicted_hp,
        "pred_hp_loss_ge_5_risk_high": safe_prob(risk5.get("probability")),
        "pred_hp_loss_ge_10_risk_high": risk10_prob,
        "value_or_risk_attention_high": max(predicted_reward, risk10_prob * 2.0, end_turn_risk),
        "end_turn_risk_baseline_high": end_turn_risk,
        "play_card_value_baseline_high": (
            safe_float(candidate.get("card_base_damage"))
            + safe_float(candidate.get("card_base_block")) * 0.25
            if is_play_card
            else -0.25
        ),
        "allocation_hp_loss_ge_10_probability_high": safe_prob(
            signals.get("allocation_hp_loss_ge_10_probability")
        ),
        "allocation_branch_priority_high": safe_prob(
            signals.get("allocation_branch_priority")
        ),
    }


def pair_scores(row: dict[str, Any], *, index: int) -> dict[str, float]:
    signals = row.get("search_allocation_signals") or {}
    outputs = row.get("model_outputs") or {}
    tails = outputs.get("tail_probabilities") or {}
    branch_abs = safe_float(signals.get("branch_model_abs_hp_diff"))
    residual_abs = safe_float(signals.get("residual_corrected_abs_hp_diff"))
    tail_abs10 = safe_prob(signals.get("tail_abs_hp_diff_ge_10_probability"))
    directional10 = max(
        safe_prob(signals.get("tail_left_worse_ge_10_probability")),
        safe_prob(signals.get("tail_left_better_ge_10_probability")),
    )
    tail_abs15 = safe_prob(tails.get("abs_hp_diff_ge_15"))
    left_kind = (((row.get("left") or {}).get("candidate") or {}).get("action_kind"))
    right_kind = (((row.get("right") or {}).get("candidate") or {}).get("action_kind"))
    end_turn_play_card = {left_kind, right_kind} == {"end_turn", "play_card"}
    return {
        "input_order_high": -float(index),
        "random_hash_high": stable_hash_unit(item_key(row, index)),
        "branch_abs_hp_diff_high": branch_abs,
        "residual_abs_hp_diff_high": residual_abs,
        "residual_branch_gap_high": max(0.0, residual_abs - branch_abs),
        "tail_abs_ge_10_probability_high": tail_abs10,
        "tail_directional_ge_10_probability_high": directional10,
        "tail_abs_ge_15_probability_high": tail_abs15,
        "combined_tail_attention_high": max(
            branch_abs / 15.0,
            residual_abs / 15.0,
            tail_abs10 * 2.0,
            directional10 * 2.0,
            tail_abs15 * 3.0,
        ),
        "end_turn_play_card_baseline_high": 1.0 if end_turn_play_card else 0.0,
        "allocation_abs_ge_10_probability_high": safe_prob(
            signals.get("allocation_abs_hp_diff_ge_10_probability")
        ),
        "allocation_abs_ge_15_probability_high": safe_prob(
            signals.get("allocation_abs_hp_diff_ge_15_probability")
        ),
        "allocation_branch_severe_probability_high": safe_prob(
            signals.get("allocation_branch_model_severe_underestimate_ge_10_probability")
        ),
        "allocation_residual_severe_probability_high": safe_prob(
            signals.get("allocation_residual_model_severe_underestimate_ge_10_probability")
        ),
        "allocation_end_turn_play_card_abs_ge_10_probability_high": safe_prob(
            signals.get("allocation_end_turn_play_card_abs_hp_diff_ge_10_probability")
        ),
        "allocation_pair_required_probability_high": safe_prob(
            signals.get("allocation_pair_required_probability")
        ),
        "allocation_pair_watch_probability_high": safe_prob(
            signals.get("allocation_pair_watch_probability")
        ),
        "allocation_pair_priority_high": safe_prob(
            signals.get("allocation_pair_priority")
        ),
    }


def top_k_indices(
    rows: list[dict[str, Any]],
    scores: list[dict[str, float]],
    score_name: str,
    budget: int,
) -> set[int]:
    ordered = sorted(
        range(len(rows)),
        key=lambda index: (-scores[index].get(score_name, 0.0), item_key(rows[index], index)),
    )
    return set(ordered[: max(0, min(budget, len(ordered)))])


def random_expected_any_recall(n_items: int, n_positive: int, budget: int) -> float:
    if n_items <= 0 or n_positive <= 0:
        return 0.0
    k = min(budget, n_items)
    if k <= 0:
        return 0.0
    if k >= n_items - n_positive + 1:
        return 1.0
    return 1.0 - (math.comb(n_items - n_positive, k) / math.comb(n_items, k))


def make_bucket() -> dict[str, float]:
    return {
        "eligible_decisions": 0.0,
        "hit_decisions": 0.0,
        "eligible_items": 0.0,
        "hit_items": 0.0,
        "allocated_items": 0.0,
        "true_positive_allocated_items": 0.0,
        "random_expected_hit_decisions": 0.0,
        "random_expected_hit_items": 0.0,
        "random_expected_true_positive_allocated_items": 0.0,
        "random_expected_allocated_items": 0.0,
    }


def update_bucket(
    bucket: dict[str, float],
    *,
    group_size: int,
    positives: set[int],
    allocated: set[int],
    budget: int,
) -> None:
    bucket["eligible_decisions"] += 1.0
    bucket["eligible_items"] += float(len(positives))
    bucket["allocated_items"] += float(len(allocated))
    hit = allocated & positives
    if hit:
        bucket["hit_decisions"] += 1.0
        bucket["hit_items"] += float(len(hit))
    bucket["true_positive_allocated_items"] += float(len(hit))

    k = min(budget, group_size)
    p = len(positives)
    expected_item_recall = k / group_size if group_size else 0.0
    expected_hit_items = p * expected_item_recall
    bucket["random_expected_hit_decisions"] += random_expected_any_recall(group_size, p, budget)
    bucket["random_expected_hit_items"] += expected_hit_items
    bucket["random_expected_allocated_items"] += float(k)
    bucket["random_expected_true_positive_allocated_items"] += expected_hit_items


def finalize_bucket(bucket: dict[str, float]) -> dict[str, Any]:
    eligible_decisions = int(bucket["eligible_decisions"])
    eligible_items = int(bucket["eligible_items"])
    allocated_items = int(bucket["allocated_items"])
    random_allocated = bucket["random_expected_allocated_items"]
    decision_any_recall = (
        bucket["hit_decisions"] / eligible_decisions if eligible_decisions else None
    )
    random_decision_any_recall = (
        bucket["random_expected_hit_decisions"] / eligible_decisions if eligible_decisions else None
    )
    item_recall = bucket["hit_items"] / eligible_items if eligible_items else None
    random_item_recall = (
        bucket["random_expected_hit_items"] / eligible_items if eligible_items else None
    )
    allocation_precision = (
        bucket["true_positive_allocated_items"] / allocated_items if allocated_items else None
    )
    random_allocation_precision = (
        bucket["random_expected_true_positive_allocated_items"] / random_allocated
        if random_allocated
        else None
    )
    lift_over_random_any_recall = None
    if random_decision_any_recall and decision_any_recall is not None:
        lift_over_random_any_recall = decision_any_recall / random_decision_any_recall
    return {
        "eligible_decisions": eligible_decisions,
        "decision_any_recall": decision_any_recall,
        "random_expected_decision_any_recall": random_decision_any_recall,
        "decision_any_recall_lift_over_random": lift_over_random_any_recall,
        "eligible_items": eligible_items,
        "item_recall": item_recall,
        "random_expected_item_recall": random_item_recall,
        "allocated_items": allocated_items,
        "allocation_precision": allocation_precision,
        "random_expected_allocation_precision": random_allocation_precision,
    }


def branch_objective_sets(group: list[dict[str, Any]]) -> dict[str, set[int]]:
    targets = [row.get("targets") or {} for row in group]
    total_rewards = [safe_float(target.get("total_reward")) for target in targets]
    hp_deltas = [safe_float(target.get("hp_delta")) for target in targets]
    best_reward = max(total_rewards)
    best_hp = max(hp_deltas)
    worst_hp = min(hp_deltas)
    return {
        "best_total_reward": {i for i, value in enumerate(total_rewards) if value == best_reward},
        "best_hp_delta": {i for i, value in enumerate(hp_deltas) if value == best_hp},
        "worst_hp_delta": {i for i, value in enumerate(hp_deltas) if value == worst_hp},
        "hp_loss_ge_5": {i for i, value in enumerate(hp_deltas) if value <= -5},
        "hp_loss_ge_10": {i for i, value in enumerate(hp_deltas) if value <= -10},
    }


def is_end_turn_play_card_pair(row: dict[str, Any]) -> bool:
    left_kind = (((row.get("left") or {}).get("candidate") or {}).get("action_kind"))
    right_kind = (((row.get("right") or {}).get("candidate") or {}).get("action_kind"))
    return {left_kind, right_kind} == {"end_turn", "play_card"}


def pair_objective_sets(group: list[dict[str, Any]]) -> dict[str, set[int]]:
    true_hp = [safe_float((row.get("targets") or {}).get("hp_left_minus_right")) for row in group]
    errors = [row.get("errors") or {} for row in group]
    return {
        "abs_hp_diff_ge_5": {i for i, value in enumerate(true_hp) if abs(value) >= 5.0},
        "abs_hp_diff_ge_10": {i for i, value in enumerate(true_hp) if abs(value) >= 10.0},
        "abs_hp_diff_ge_15": {i for i, value in enumerate(true_hp) if abs(value) >= 15.0},
        "branch_model_severe_underestimate_ge_10": {
            i
            for i, err in enumerate(errors)
            if err.get("branch_model_severe_underestimate_abs_ge_10_pred_abs_lt_5")
        },
        "residual_model_severe_underestimate_ge_10": {
            i
            for i, err in enumerate(errors)
            if err.get("residual_corrected_severe_underestimate_abs_ge_10_pred_abs_lt_5")
        },
        "end_turn_play_card_abs_hp_diff_ge_10": {
            i
            for i, value in enumerate(true_hp)
            if abs(value) >= 10.0 and is_end_turn_play_card_pair(group[i])
        },
    }


def group_rows(rows: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        groups[decision_key(row)].append(row)
    return groups


def evaluate_allocation(
    *,
    rows: list[dict[str, Any]],
    kind: str,
    budgets: list[int],
) -> tuple[dict[str, Any], dict[tuple[str, str, int], list[dict[str, Any]]]]:
    if kind == "branch":
        score_names = BRANCH_SCORE_NAMES
        objective_names = BRANCH_OBJECTIVES
        score_fn = branch_scores
        objective_fn = branch_objective_sets
        summary_fn = branch_item_summary
    elif kind == "pair":
        score_names = PAIR_SCORE_NAMES
        objective_names = PAIR_OBJECTIVES
        score_fn = pair_scores
        objective_fn = pair_objective_sets
        summary_fn = pair_item_summary
    else:
        raise ValueError(f"unknown allocation kind {kind}")

    groups = group_rows(rows)
    buckets: dict[tuple[str, int, str], dict[str, float]] = {
        (score_name, budget, objective): make_bucket()
        for score_name in score_names
        for budget in budgets
        for objective in objective_names
    }
    misses: dict[tuple[str, str, int], list[dict[str, Any]]] = defaultdict(list)
    decision_group_sizes: list[int] = []

    for key, group in groups.items():
        if len(group) < 2:
            continue
        decision_group_sizes.append(len(group))
        scores = [score_fn(row, index=index) for index, row in enumerate(group)]
        objective_sets = objective_fn(group)
        for score_name in score_names:
            for budget in budgets:
                allocated = top_k_indices(group, scores, score_name, budget)
                for objective in objective_names:
                    positives = objective_sets.get(objective, set())
                    if not positives:
                        continue
                    update_bucket(
                        buckets[(score_name, budget, objective)],
                        group_size=len(group),
                        positives=positives,
                        allocated=allocated,
                        budget=budget,
                    )
                    if not (allocated & positives):
                        misses[(score_name, objective, budget)].append(
                            {
                                "decision_key": key,
                                "group_size": len(group),
                                "positive_count": len(positives),
                                "allocated_items": [
                                    summary_fn(group[index]) for index in sorted(allocated)
                                ],
                                "missed_target_items": [
                                    summary_fn(group[index]) for index in sorted(positives)
                                ],
                            }
                        )

    metrics: dict[str, dict[str, dict[str, Any]]] = {}
    for score_name in score_names:
        metrics[score_name] = {}
        for budget in budgets:
            budget_key = f"budget_{budget}"
            metrics[score_name][budget_key] = {}
            for objective in objective_names:
                metrics[score_name][budget_key][objective] = finalize_bucket(
                    buckets[(score_name, budget, objective)]
                )

    return (
        {
            "row_count": len(rows),
            "decision_count": len(groups),
            "multi_item_decision_count": len(decision_group_sizes),
            "avg_items_per_multi_item_decision": (
                sum(decision_group_sizes) / len(decision_group_sizes)
                if decision_group_sizes
                else None
            ),
            "score_metrics": metrics,
        },
        misses,
    )


def default_check_defs() -> list[dict[str, Any]]:
    return [
        {
            "slice": "baseline",
            "kind": "branch",
            "score_name": "pred_hp_loss_ge_10_risk_high",
            "objective": "hp_loss_ge_10",
            "budget": 1,
            "min_decision_any_recall": 0.95,
            "required": True,
        },
        {
            "slice": "baseline",
            "kind": "pair",
            "score_name": "tail_abs_ge_10_probability_high",
            "objective": "abs_hp_diff_ge_10",
            "budget": 2,
            "min_decision_any_recall": 0.90,
            "required": True,
        },
        {
            "slice": "baseline",
            "kind": "pair",
            "score_name": "tail_abs_ge_15_probability_high",
            "objective": "abs_hp_diff_ge_15",
            "budget": 2,
            "min_decision_any_recall": 0.90,
            "required": True,
        },
        {
            "slice": "hard",
            "kind": "branch",
            "score_name": "pred_hp_loss_ge_10_risk_high",
            "objective": "hp_loss_ge_10",
            "budget": 1,
            "min_decision_any_recall": 0.90,
            "required": True,
        },
        {
            "slice": "hard",
            "kind": "pair",
            "score_name": "tail_abs_ge_10_probability_high",
            "objective": "abs_hp_diff_ge_10",
            "budget": 3,
            "min_decision_any_recall": 0.90,
            "required": True,
        },
        {
            "slice": "hard",
            "kind": "pair",
            "score_name": "tail_abs_ge_15_probability_high",
            "objective": "abs_hp_diff_ge_15",
            "budget": 5,
            "min_decision_any_recall": 0.80,
            "required": True,
        },
        {
            "slice": "hard",
            "kind": "pair",
            "score_name": "residual_branch_gap_high",
            "objective": "branch_model_severe_underestimate_ge_10",
            "budget": 5,
            "min_decision_any_recall": 0.50,
            "required": False,
        },
    ]


def allocation_model_check_defs() -> list[dict[str, Any]]:
    return [
        {
            "slice": "baseline",
            "kind": "branch",
            "score_name": "allocation_hp_loss_ge_10_probability_high",
            "objective": "hp_loss_ge_10",
            "budget": 1,
            "min_decision_any_recall": 0.95,
            "required": True,
        },
        {
            "slice": "baseline",
            "kind": "pair",
            "score_name": "allocation_abs_ge_10_probability_high",
            "objective": "abs_hp_diff_ge_10",
            "budget": 2,
            "min_decision_any_recall": 0.90,
            "required": True,
        },
        {
            "slice": "baseline",
            "kind": "pair",
            "score_name": "allocation_abs_ge_15_probability_high",
            "objective": "abs_hp_diff_ge_15",
            "budget": 2,
            "min_decision_any_recall": 0.90,
            "required": True,
        },
        {
            "slice": "hard",
            "kind": "branch",
            "score_name": "allocation_hp_loss_ge_10_probability_high",
            "objective": "hp_loss_ge_10",
            "budget": 1,
            "min_decision_any_recall": 0.90,
            "required": True,
        },
        {
            "slice": "hard",
            "kind": "pair",
            "score_name": "allocation_abs_ge_10_probability_high",
            "objective": "abs_hp_diff_ge_10",
            "budget": 3,
            "min_decision_any_recall": 0.90,
            "required": True,
        },
        {
            "slice": "hard",
            "kind": "pair",
            "score_name": "allocation_abs_ge_15_probability_high",
            "objective": "abs_hp_diff_ge_15",
            "budget": 5,
            "min_decision_any_recall": 0.80,
            "required": True,
        },
        {
            "slice": "hard",
            "kind": "pair",
            "score_name": "allocation_branch_severe_probability_high",
            "objective": "branch_model_severe_underestimate_ge_10",
            "budget": 5,
            "min_decision_any_recall": 0.50,
            "required": False,
        },
    ]


def check_defs(profile: str) -> list[dict[str, Any]]:
    if profile == "default":
        return default_check_defs()
    if profile == "allocation_model":
        return allocation_model_check_defs()
    raise ValueError(f"unknown check profile {profile}")


def metric_lookup(summary: dict[str, Any], check: dict[str, Any]) -> dict[str, Any] | None:
    slice_summary = summary["slices"].get(check["slice"]) or {}
    kind_summary = slice_summary.get(f"{check['kind']}_allocation") or {}
    score_metrics = kind_summary.get("score_metrics") or {}
    return (
        score_metrics.get(check["score_name"], {})
        .get(f"budget_{check['budget']}", {})
        .get(check["objective"])
    )


def build_gate_checks(summary: dict[str, Any], profile: str) -> list[dict[str, Any]]:
    checks: list[dict[str, Any]] = []
    for definition in check_defs(profile):
        metric = metric_lookup(summary, definition)
        actual = None if metric is None else metric.get("decision_any_recall")
        passed = actual is not None and actual >= definition["min_decision_any_recall"]
        checks.append(
            {
                **definition,
                "decision_any_recall": actual,
                "item_recall": None if metric is None else metric.get("item_recall"),
                "allocation_precision": (
                    None if metric is None else metric.get("allocation_precision")
                ),
                "random_expected_decision_any_recall": (
                    None if metric is None else metric.get("random_expected_decision_any_recall")
                ),
                "decision_any_recall_lift_over_random": (
                    None if metric is None else metric.get("decision_any_recall_lift_over_random")
                ),
                "passed": passed,
            }
        )
    return checks


def build_miss_rows(
    *,
    all_misses: dict[tuple[str, str, str, int], list[dict[str, Any]]],
    gate_checks: list[dict[str, Any]],
    max_misses_per_check: int,
) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for check in gate_checks:
        key = (
            check["slice"],
            check["kind"],
            check["score_name"],
            check["objective"],
            check["budget"],
        )
        for miss in all_misses.get(key, [])[:max_misses_per_check]:
            out.append(
                {
                    "schema_version": "search_allocation_gate_miss_v0",
                    "slice_name": check["slice"],
                    "allocation_kind": check["kind"],
                    "score_name": check["score_name"],
                    "objective": check["objective"],
                    "budget": check["budget"],
                    "required_gate_check": check["required"],
                    "decision_key": miss["decision_key"],
                    "group_size": miss["group_size"],
                    "positive_count": miss["positive_count"],
                    "allocated_items": miss["allocated_items"],
                    "missed_target_items": miss["missed_target_items"],
                    "trainable_role": "search_allocation_recollection_target",
                    "trainable_as_action_label": False,
                    "label_policy": {
                        "action_label": False,
                        "source": "search_allocation_gate_v1",
                    },
                }
            )
    return out


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline-branch-predictions", type=Path, required=True)
    parser.add_argument("--baseline-pair-predictions", type=Path, required=True)
    parser.add_argument("--hard-branch-predictions", type=Path, required=True)
    parser.add_argument("--hard-pair-predictions", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--miss-out", type=Path, required=True)
    parser.add_argument("--budgets", default="1,2,3,5")
    parser.add_argument("--max-misses-per-check", type=int, default=50)
    parser.add_argument(
        "--check-profile",
        choices=("default", "allocation_model"),
        default="default",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    budgets = parse_int_list(args.budgets)
    slices = {
        "baseline": {
            "branch": load_rows(args.baseline_branch_predictions, "baseline_branch_prediction"),
            "pair": load_rows(args.baseline_pair_predictions, "baseline_pair_prediction"),
        },
        "hard": {
            "branch": load_rows(args.hard_branch_predictions, "hard_branch_prediction"),
            "pair": load_rows(args.hard_pair_predictions, "hard_pair_prediction"),
        },
    }

    summary: dict[str, Any] = {
        "schema_version": "search_allocation_gate_v1",
        "check_profile": args.check_profile,
        "budgets": budgets,
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "gate_is_search_allocation_not_policy": True,
        },
        "slices": {},
    }
    all_misses: dict[tuple[str, str, str, int], list[dict[str, Any]]] = {}

    for slice_name, slice_rows in slices.items():
        slice_summary: dict[str, Any] = {
            "branch_prediction_count": len(slice_rows["branch"]),
            "pair_prediction_count": len(slice_rows["pair"]),
        }
        branch_summary, branch_misses = evaluate_allocation(
            rows=slice_rows["branch"],
            kind="branch",
            budgets=budgets,
        )
        pair_summary, pair_misses = evaluate_allocation(
            rows=slice_rows["pair"],
            kind="pair",
            budgets=budgets,
        )
        slice_summary["branch_allocation"] = branch_summary
        slice_summary["pair_allocation"] = pair_summary
        summary["slices"][slice_name] = slice_summary
        for (score_name, objective, budget), misses in branch_misses.items():
            all_misses[(slice_name, "branch", score_name, objective, budget)] = misses
        for (score_name, objective, budget), misses in pair_misses.items():
            all_misses[(slice_name, "pair", score_name, objective, budget)] = misses

    gate_checks = build_gate_checks(summary, args.check_profile)
    summary["gate_checks"] = gate_checks
    required_failures = [check for check in gate_checks if check["required"] and not check["passed"]]
    watch_failures = [check for check in gate_checks if not check["required"] and not check["passed"]]
    summary["overall_status"] = "pass" if not required_failures else "fail"
    summary["required_failure_count"] = len(required_failures)
    summary["watch_failure_count"] = len(watch_failures)
    summary["watch_failures_are_recollection_targets_not_gate_failures"] = True

    miss_rows = build_miss_rows(
        all_misses=all_misses,
        gate_checks=gate_checks,
        max_misses_per_check=args.max_misses_per_check,
    )
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    write_jsonl(args.miss_out, miss_rows)
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
