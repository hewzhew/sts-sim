#!/usr/bin/env python3
"""Evaluate contrast-family coverage for search allocation.

This is not a policy evaluator and does not produce action labels. It asks a
more allocation-aligned question than pair top-K recall:

    With K evidence requests, how much high-regret contrast family mass is
    covered inside each decision?

The script compares raw pair top-K against family-deduplicated top-K using
existing pair prediction artifacts.
"""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable


FORBIDDEN_LABEL_KEYS = {
    "winner",
    "preferred",
    "preferred_action",
    "selected_action",
    "teacher_choice",
}

DEFAULT_SCORES = (
    "allocation_abs_hp_diff_ge_10_probability",
    "allocation_pair_priority",
    "tail_abs_hp_diff_ge_10_probability",
    "residual_corrected_abs_hp_diff",
    "branch_model_abs_hp_diff",
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
    return number if math.isfinite(number) else default


def assert_no_action_label_leak(row: dict[str, Any], *, index: int) -> None:
    if row.get("trainable_as_action_label") is not False:
        raise ValueError(f"pair row {index} is action-label-like")
    if (row.get("label_policy") or {}).get("action_label") is not False:
        raise ValueError(f"pair row {index} has action_label=true")
    serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
    for key in FORBIDDEN_LABEL_KEYS:
        if f'"{key}"' in serialized:
            raise ValueError(f"pair row {index} contains forbidden key {key}")


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


def candidate_tags(candidate: dict[str, Any]) -> list[str]:
    kind = candidate.get("action_kind")
    if kind == "end_turn":
        return ["end_turn"]
    if kind != "play_card":
        return [str(kind or "unknown")]
    tags: list[str] = []
    if safe_float(candidate.get("card_base_damage")) > 0:
        tags.append("damage")
    if safe_float(candidate.get("card_base_block")) > 0:
        tags.append("block")
    if candidate.get("card_applies_vulnerable"):
        tags.append("vulnerable")
    if candidate.get("card_applies_weak"):
        tags.append("weak")
    if candidate.get("card_draws_cards"):
        tags.append("draw")
    if candidate.get("card_exhaust"):
        tags.append("exhaust")
    if candidate.get("card_scaling_piece") or candidate.get("card_type_id") == 3:
        tags.append("setup")
    if not tags:
        tags.append("play_card_other")
    return sorted(set(tags))


def primary_tag(candidate: dict[str, Any]) -> str:
    tags = candidate_tags(candidate)
    priority = (
        "end_turn",
        "damage",
        "block",
        "vulnerable",
        "weak",
        "setup",
        "draw",
        "exhaust",
        "play_card_other",
    )
    for tag in priority:
        if tag in tags:
            return tag
    return tags[0] if tags else "unknown"


def contrast_family(row: dict[str, Any], *, mode: str) -> str:
    left = (row.get("left") or {}).get("candidate") or {}
    right = (row.get("right") or {}).get("candidate") or {}
    left_kind = left.get("action_kind") or "unknown"
    right_kind = right.get("action_kind") or "unknown"
    if mode == "action_kind":
        return f"{left_kind}_vs_{right_kind}"
    left_primary = primary_tag(left)
    right_primary = primary_tag(right)
    if mode == "primary_tag":
        return f"{left_primary}_vs_{right_primary}"
    if mode == "end_turn_split":
        if left_primary == "end_turn" and right_primary != "end_turn":
            return f"end_turn_vs_{right_primary}"
        if right_primary == "end_turn" and left_primary != "end_turn":
            return f"{left_primary}_vs_end_turn"
        return f"{left_primary}_vs_{right_primary}"
    raise ValueError(f"unknown family mode {mode}")


def pair_label(row: dict[str, Any]) -> dict[str, float]:
    targets = row.get("targets") or {}
    hp_diff = safe_float(targets.get("hp_left_minus_right"))
    return {
        "hp_diff": hp_diff,
        "abs_hp_diff": abs(hp_diff),
        "left_worse": max(0.0, -hp_diff),
        "left_better": max(0.0, hp_diff),
    }


def score_value(row: dict[str, Any], score_name: str) -> float:
    signals = row.get("search_allocation_signals") or {}
    outputs = row.get("model_outputs") or {}
    tails = outputs.get("tail_probabilities") or {}
    if score_name == "residual_corrected_abs_hp_diff":
        return abs(safe_float(outputs.get("residual_corrected_hp_left_minus_right")))
    if score_name == "branch_model_abs_hp_diff":
        return abs(safe_float(outputs.get("branch_model_hp_left_minus_right")))
    if score_name in signals:
        return safe_float(signals.get(score_name))
    if score_name in tails:
        return safe_float(tails.get(score_name))
    allocation = row.get("allocation_model_outputs") or {}
    if score_name in allocation:
        return safe_float(allocation.get(score_name))
    return 0.0


def load_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for index, row in enumerate(iter_jsonl(path)):
        assert_no_action_label_leak(row, index=index)
        rows.append(row)
    return rows


def selected_pair_ids_pair_topk(
    rows: list[dict[str, Any]],
    *,
    score_name: str,
    budget: int,
) -> list[int]:
    ranked = sorted(
        range(len(rows)),
        key=lambda index: (-score_value(rows[index], score_name), index),
    )
    return ranked[:budget]


def selected_pair_ids_family_topk(
    rows: list[dict[str, Any]],
    *,
    score_name: str,
    family_mode: str,
    budget: int,
) -> list[int]:
    best_by_family: dict[str, tuple[float, int]] = {}
    for index, row in enumerate(rows):
        family = contrast_family(row, mode=family_mode)
        score = score_value(row, score_name)
        previous = best_by_family.get(family)
        if previous is None or score > previous[0] or (score == previous[0] and index < previous[1]):
            best_by_family[family] = (score, index)
    ranked = sorted(best_by_family.items(), key=lambda item: (-item[1][0], item[1][1], item[0]))
    return [index for _family, (_score, index) in ranked[:budget]]


def target_ids(rows: list[dict[str, Any]], *, threshold: float) -> set[int]:
    return {
        index
        for index, row in enumerate(rows)
        if pair_label(row)["abs_hp_diff"] >= threshold
    }


def family_mass(rows: list[dict[str, Any]], target: set[int], *, family_mode: str) -> dict[str, float]:
    mass: dict[str, float] = defaultdict(float)
    for index in target:
        row = rows[index]
        mass[contrast_family(row, mode=family_mode)] += pair_label(row)["abs_hp_diff"]
    return dict(mass)


def evaluate_selection(
    groups: dict[str, list[dict[str, Any]]],
    *,
    score_name: str,
    family_mode: str,
    budget: int,
    threshold: float,
    selection_mode: str,
) -> dict[str, Any]:
    eligible_decisions = 0
    decision_any_hits = 0
    total_targets = 0
    hit_targets = 0
    total_families = 0
    hit_families = 0
    total_regret_mass = 0.0
    hit_regret_mass = 0.0
    duplicate_budget_slots = 0
    selected_family_counts: Counter[str] = Counter()
    missed_family_counts: Counter[str] = Counter()
    for rows in groups.values():
        target = target_ids(rows, threshold=threshold)
        if not target:
            continue
        eligible_decisions += 1
        if selection_mode == "pair_topk":
            selected = selected_pair_ids_pair_topk(rows, score_name=score_name, budget=budget)
        elif selection_mode == "family_topk":
            selected = selected_pair_ids_family_topk(
                rows,
                score_name=score_name,
                family_mode=family_mode,
                budget=budget,
            )
        else:
            raise ValueError(f"unknown selection mode {selection_mode}")
        selected_set = set(selected)
        selected_families = {
            contrast_family(rows[index], mode=family_mode) for index in selected
        }
        duplicate_budget_slots += len(selected) - len(selected_families)
        target_families = {
            contrast_family(rows[index], mode=family_mode) for index in target
        }
        hits = target & selected_set
        family_hits = target_families & selected_families
        if hits or family_hits:
            decision_any_hits += 1
        total_targets += len(target)
        hit_targets += len(hits)
        total_families += len(target_families)
        hit_families += len(family_hits)
        masses = family_mass(rows, target, family_mode=family_mode)
        total_regret_mass += sum(masses.values())
        hit_regret_mass += sum(value for key, value in masses.items() if key in selected_families)
        selected_family_counts.update(selected_families)
        missed_family_counts.update(target_families - selected_families)
    return {
        "eligible_decisions": eligible_decisions,
        "decision_any_family_recall": (
            decision_any_hits / eligible_decisions if eligible_decisions else None
        ),
        "target_pair_recall": hit_targets / total_targets if total_targets else None,
        "target_family_recall": hit_families / total_families if total_families else None,
        "regret_mass_recall": hit_regret_mass / total_regret_mass if total_regret_mass else None,
        "duplicate_budget_slots": duplicate_budget_slots,
        "avg_duplicate_budget_slots_per_eligible_decision": (
            duplicate_budget_slots / eligible_decisions if eligible_decisions else None
        ),
        "selected_family_top": dict(selected_family_counts.most_common(20)),
        "missed_family_top": dict(missed_family_counts.most_common(20)),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pair-predictions", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--budgets", default="1,2,3,5")
    parser.add_argument("--thresholds", default="10,15")
    parser.add_argument("--score-names", default=",".join(DEFAULT_SCORES))
    parser.add_argument(
        "--family-modes",
        default="action_kind,primary_tag,end_turn_split",
    )
    return parser.parse_args()


def parse_number_list(text: str, *, kind: type) -> list[Any]:
    values: list[Any] = []
    for part in text.split(","):
        part = part.strip()
        if part:
            values.append(kind(part))
    return sorted(set(values))


def parse_string_list(text: str) -> list[str]:
    return [part.strip() for part in text.split(",") if part.strip()]


def main() -> int:
    args = parse_args()
    rows = load_rows(args.pair_predictions)
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        groups[decision_key(row)].append(row)
    budgets = parse_number_list(args.budgets, kind=int)
    thresholds = parse_number_list(args.thresholds, kind=float)
    score_names = parse_string_list(args.score_names)
    family_modes = parse_string_list(args.family_modes)
    checks: list[dict[str, Any]] = []
    for threshold in thresholds:
        for family_mode in family_modes:
            for score_name in score_names:
                for budget in budgets:
                    for selection_mode in ("pair_topk", "family_topk"):
                        metrics = evaluate_selection(
                            groups,
                            score_name=score_name,
                            family_mode=family_mode,
                            budget=budget,
                            threshold=threshold,
                            selection_mode=selection_mode,
                        )
                        checks.append(
                            {
                                "threshold": threshold,
                                "family_mode": family_mode,
                                "score_name": score_name,
                                "budget": budget,
                                "selection_mode": selection_mode,
                                **metrics,
                            }
                        )
    summary = {
        "schema_version": "family_coverage_allocation_audit_v0",
        "pair_prediction_count": len(rows),
        "decision_count": len(groups),
        "budgets": budgets,
        "thresholds": thresholds,
        "score_names": score_names,
        "family_modes": family_modes,
        "checks": checks,
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "family_coverage_is_search_allocation_not_policy": True,
        },
        "pair_predictions": str(args.pair_predictions),
    }
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
