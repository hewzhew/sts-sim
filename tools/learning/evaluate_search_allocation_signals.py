#!/usr/bin/env python3
"""Evaluate branch/pair model signals as search-allocation signals.

This script does not train an action policy and does not create winner,
preference, or action-label targets. It asks a narrower question:

Given a per-decision budget K, do current branch/pair audit signals cover
branches or pairs that later turned out to have material outcome differences?
"""

from __future__ import annotations

import argparse
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
    number = safe_float(value, 0.0)
    if number < 0.0:
        return 0.0
    if number > 1.0:
        return 1.0
    return number


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


def assert_no_action_label_leak(row: dict[str, Any], *, row_kind: str, row_index: int) -> None:
    if row.get("trainable_as_action_label") is not False:
        raise ValueError(f"{row_kind} row {row_index} is action-label-like")
    if (row.get("label_policy") or {}).get("action_label") is not False:
        raise ValueError(f"{row_kind} row {row_index} has action_label=true")
    serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
    for key in FORBIDDEN_LABEL_KEYS:
        if f'"{key}"' in serialized:
            raise ValueError(f"{row_kind} row {row_index} contains forbidden key {key}")


def branch_scores(row: dict[str, Any]) -> dict[str, float]:
    outputs = row.get("model_outputs") or {}
    risks = outputs.get("risks") or {}
    risk5 = risks.get("hp_loss_ge_5") or {}
    risk10 = risks.get("hp_loss_ge_10") or {}
    candidate = row.get("candidate") or {}
    context = row.get("decision_context") or {}
    is_end_turn = candidate.get("action_kind") == "end_turn"
    return {
        "pred_total_reward_high": safe_float(outputs.get("total_reward")),
        "pred_hp_delta_high": safe_float(outputs.get("hp_delta")),
        "pred_hp_loss_ge_5_risk_high": safe_prob(risk5.get("probability")),
        "pred_hp_loss_ge_10_risk_high": safe_prob(risk10.get("probability")),
        # A cheap attention score for recollection/search, not for action choice.
        "value_or_risk_attention_high": max(
            safe_float(outputs.get("total_reward")),
            safe_prob(risk10.get("probability")) * 2.0,
            (1.0 if is_end_turn and context.get("end_turn_with_playable_cards") else 0.0),
        ),
    }


def pair_scores(row: dict[str, Any]) -> dict[str, float]:
    signals = row.get("search_allocation_signals") or {}
    outputs = row.get("model_outputs") or {}
    tails = outputs.get("tail_probabilities") or {}
    residual_abs = safe_float(signals.get("residual_corrected_abs_hp_diff"))
    branch_abs = safe_float(signals.get("branch_model_abs_hp_diff"))
    tail_abs10 = safe_prob(signals.get("tail_abs_hp_diff_ge_10_probability"))
    directional10 = max(
        safe_prob(signals.get("tail_left_worse_ge_10_probability")),
        safe_prob(signals.get("tail_left_better_ge_10_probability")),
    )
    return {
        "branch_abs_hp_diff_high": branch_abs,
        "residual_abs_hp_diff_high": residual_abs,
        "tail_abs_ge_10_probability_high": tail_abs10,
        "tail_directional_ge_10_probability_high": directional10,
        "tail_abs_ge_15_probability_high": safe_prob(tails.get("abs_hp_diff_ge_15")),
        # This is a search-allocation attention score. It deliberately avoids
        # saying which action is better.
        "combined_tail_attention_high": max(
            residual_abs / 15.0,
            branch_abs / 15.0,
            tail_abs10 * 2.0,
            directional10 * 2.0,
            safe_prob(tails.get("abs_hp_diff_ge_15")) * 3.0,
        ),
    }


def top_k_indices(rows: list[dict[str, Any]], score_name: str, scores: list[dict[str, float]], k: int) -> set[int]:
    ordered = sorted(
        range(len(rows)),
        key=lambda index: (
            -scores[index].get(score_name, 0.0),
            str(rows[index].get("branch_id") or rows[index].get("comparison_id") or index),
        ),
    )
    return set(ordered[: max(0, min(k, len(ordered)))])


def make_metric_bucket() -> dict[str, float]:
    return {
        "eligible_decisions": 0,
        "hit_decisions": 0,
        "eligible_items": 0,
        "hit_items": 0,
        "selected_items": 0,
        "true_positive_selected_items": 0,
    }


def finalize_metric_bucket(bucket: dict[str, float]) -> dict[str, Any]:
    eligible_decisions = int(bucket["eligible_decisions"])
    eligible_items = int(bucket["eligible_items"])
    selected_items = int(bucket["selected_items"])
    return {
        "eligible_decisions": eligible_decisions,
        "decision_any_recall": (
            bucket["hit_decisions"] / eligible_decisions if eligible_decisions else None
        ),
        "eligible_items": eligible_items,
        "item_recall": bucket["hit_items"] / eligible_items if eligible_items else None,
        "selected_items": selected_items,
        "selected_precision": (
            bucket["true_positive_selected_items"] / selected_items if selected_items else None
        ),
    }


def evaluate_branch_allocation(
    rows: list[dict[str, Any]],
    budgets: list[int],
) -> dict[str, Any]:
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        groups[decision_key(row)].append(row)

    score_names = sorted(branch_scores(rows[0]).keys()) if rows else []
    metrics: dict[str, dict[str, dict[str, dict[str, float]]]] = {}
    audit_misses: list[dict[str, Any]] = []

    for score_name in score_names:
        metrics[score_name] = {}
        for budget in budgets:
            metrics[score_name][f"budget_{budget}"] = {
                "best_total_reward": make_metric_bucket(),
                "best_hp_delta": make_metric_bucket(),
                "worst_hp_delta": make_metric_bucket(),
                "hp_loss_ge_5": make_metric_bucket(),
                "hp_loss_ge_10": make_metric_bucket(),
            }

    for key, group in groups.items():
        if len(group) < 2:
            continue
        scores = [branch_scores(row) for row in group]
        targets = [row.get("targets") or {} for row in group]
        total_rewards = [safe_float(target.get("total_reward")) for target in targets]
        hp_deltas = [safe_float(target.get("hp_delta")) for target in targets]
        best_reward = max(total_rewards)
        best_hp = max(hp_deltas)
        worst_hp = min(hp_deltas)
        objective_sets = {
            "best_total_reward": {i for i, value in enumerate(total_rewards) if value == best_reward},
            "best_hp_delta": {i for i, value in enumerate(hp_deltas) if value == best_hp},
            "worst_hp_delta": {i for i, value in enumerate(hp_deltas) if value == worst_hp},
            "hp_loss_ge_5": {i for i, value in enumerate(hp_deltas) if value <= -5},
            "hp_loss_ge_10": {i for i, value in enumerate(hp_deltas) if value <= -10},
        }
        for score_name in score_names:
            for budget in budgets:
                selected = top_k_indices(group, score_name, scores, budget)
                for objective, positives in objective_sets.items():
                    if not positives:
                        continue
                    bucket = metrics[score_name][f"budget_{budget}"][objective]
                    bucket["eligible_decisions"] += 1
                    bucket["eligible_items"] += len(positives)
                    bucket["selected_items"] += len(selected)
                    hit = selected & positives
                    if hit:
                        bucket["hit_decisions"] += 1
                        bucket["hit_items"] += len(hit)
                    bucket["true_positive_selected_items"] += len(hit)
                if (
                    score_name == "pred_total_reward_high"
                    and budget == budgets[0]
                    and not (selected & objective_sets["best_total_reward"])
                ):
                    audit_misses.append(
                        {
                            "decision_key": key,
                            "score_name": score_name,
                            "budget": budget,
                            "true_best_total_reward": best_reward,
                            "allocated_branch_action_keys": [
                                (group[index].get("candidate") or {}).get("action_key")
                                for index in sorted(selected)
                            ],
                            "missed_target_branch_action_keys": [
                                (group[index].get("candidate") or {}).get("action_key")
                                for index in sorted(objective_sets["best_total_reward"])
                            ],
                        }
                    )

    finalized: dict[str, Any] = {}
    for score_name, by_budget in metrics.items():
        finalized[score_name] = {}
        for budget_name, by_objective in by_budget.items():
            finalized[score_name][budget_name] = {
                objective: finalize_metric_bucket(bucket)
                for objective, bucket in by_objective.items()
            }
    return {
        "decision_count": len(groups),
        "score_metrics": finalized,
        "audit_misses": audit_misses[:50],
    }


def evaluate_pair_allocation(
    rows: list[dict[str, Any]],
    budgets: list[int],
    thresholds: list[int],
) -> dict[str, Any]:
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        groups[decision_key(row)].append(row)
    score_names = sorted(pair_scores(rows[0]).keys()) if rows else []
    metrics: dict[str, dict[str, dict[str, dict[str, float]]]] = {}
    audit_misses: list[dict[str, Any]] = []
    for score_name in score_names:
        metrics[score_name] = {}
        for budget in budgets:
            metrics[score_name][f"budget_{budget}"] = {
                f"abs_hp_diff_ge_{threshold}": make_metric_bucket()
                for threshold in thresholds
            }
            metrics[score_name][f"budget_{budget}"].update(
                {
                    "branch_model_severe_underestimate_ge_10": make_metric_bucket(),
                    "residual_model_severe_underestimate_ge_10": make_metric_bucket(),
                }
            )

    for key, group in groups.items():
        scores = [pair_scores(row) for row in group]
        true_hp = [safe_float((row.get("targets") or {}).get("hp_left_minus_right")) for row in group]
        errors = [row.get("errors") or {} for row in group]
        objective_sets: dict[str, set[int]] = {
            f"abs_hp_diff_ge_{threshold}": {
                i for i, value in enumerate(true_hp) if abs(value) >= threshold
            }
            for threshold in thresholds
        }
        objective_sets["branch_model_severe_underestimate_ge_10"] = {
            i
            for i, err in enumerate(errors)
            if err.get("branch_model_severe_underestimate_abs_ge_10_pred_abs_lt_5")
        }
        objective_sets["residual_model_severe_underestimate_ge_10"] = {
            i
            for i, err in enumerate(errors)
            if err.get("residual_corrected_severe_underestimate_abs_ge_10_pred_abs_lt_5")
        }
        for score_name in score_names:
            for budget in budgets:
                selected = top_k_indices(group, score_name, scores, budget)
                for objective, positives in objective_sets.items():
                    if not positives:
                        continue
                    bucket = metrics[score_name][f"budget_{budget}"][objective]
                    bucket["eligible_decisions"] += 1
                    bucket["eligible_items"] += len(positives)
                    bucket["selected_items"] += len(selected)
                    hit = selected & positives
                    if hit:
                        bucket["hit_decisions"] += 1
                        bucket["hit_items"] += len(hit)
                    bucket["true_positive_selected_items"] += len(hit)
                if (
                    score_name == "combined_tail_attention_high"
                    and budget == budgets[0]
                    and objective_sets.get("abs_hp_diff_ge_10")
                    and not (selected & objective_sets["abs_hp_diff_ge_10"])
                ):
                    audit_misses.append(
                        {
                            "decision_key": key,
                            "score_name": score_name,
                            "budget": budget,
                            "max_abs_true_hp_diff": max(abs(value) for value in true_hp),
                            "allocated_pair_cards": [
                                pair_card(group[index]) for index in sorted(selected)
                            ],
                            "missed_material_pair_cards": [
                                pair_card(group[index])
                                for index in sorted(objective_sets["abs_hp_diff_ge_10"])
                            ][:10],
                        }
                    )

    finalized: dict[str, Any] = {}
    for score_name, by_budget in metrics.items():
        finalized[score_name] = {}
        for budget_name, by_objective in by_budget.items():
            finalized[score_name][budget_name] = {
                objective: finalize_metric_bucket(bucket)
                for objective, bucket in by_objective.items()
            }
    return {
        "decision_count": len(groups),
        "score_metrics": finalized,
        "audit_misses": audit_misses[:50],
    }


def pair_card(row: dict[str, Any]) -> str:
    left = ((row.get("left") or {}).get("candidate") or {}).get("card_id")
    right = ((row.get("right") or {}).get("candidate") or {}).get("card_id")
    return f"{left}->{right}"


def parse_int_list(text: str) -> list[int]:
    out: list[int] = []
    for part in text.split(","):
        part = part.strip()
        if not part:
            continue
        out.append(int(part))
    return sorted(set(out))


def load_rows(path: Path | None, row_kind: str) -> list[dict[str, Any]]:
    if path is None:
        return []
    rows = list(iter_jsonl(path))
    for index, row in enumerate(rows):
        assert_no_action_label_leak(row, row_kind=row_kind, row_index=index)
    return rows


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--branch-predictions", type=Path)
    parser.add_argument("--pair-predictions", type=Path)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--audit-out", type=Path)
    parser.add_argument("--budgets", default="1,2,3,5")
    parser.add_argument("--thresholds", default="5,10,15")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    budgets = parse_int_list(args.budgets)
    thresholds = parse_int_list(args.thresholds)
    branch_rows = load_rows(args.branch_predictions, "branch_prediction")
    pair_rows = load_rows(args.pair_predictions, "pair_prediction")
    if not branch_rows and not pair_rows:
        raise SystemExit("provide --branch-predictions and/or --pair-predictions")

    summary: dict[str, Any] = {
        "schema_version": "search_allocation_signal_audit_v0",
        "budgets": budgets,
        "thresholds": thresholds,
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "audit_is_search_allocation_not_policy": True,
        },
        "branch_prediction_count": len(branch_rows),
        "pair_prediction_count": len(pair_rows),
    }
    audit_rows: list[dict[str, Any]] = []
    if branch_rows:
        branch_result = evaluate_branch_allocation(branch_rows, budgets)
        summary["branch_allocation"] = {
            key: value for key, value in branch_result.items() if key != "audit_misses"
        }
        for row in branch_result["audit_misses"]:
            row["schema_version"] = "search_allocation_audit_miss_v0"
            row["audit_kind"] = "branch_topk_miss"
            row["trainable_as_action_label"] = False
            audit_rows.append(row)
    if pair_rows:
        pair_result = evaluate_pair_allocation(pair_rows, budgets, thresholds)
        summary["pair_allocation"] = {
            key: value for key, value in pair_result.items() if key != "audit_misses"
        }
        for row in pair_result["audit_misses"]:
            row["schema_version"] = "search_allocation_audit_miss_v0"
            row["audit_kind"] = "pair_topk_miss"
            row["trainable_as_action_label"] = False
            audit_rows.append(row)

    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    if args.audit_out is not None:
        write_jsonl(args.audit_out, audit_rows)
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
