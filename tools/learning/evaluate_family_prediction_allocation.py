#!/usr/bin/env python3
"""Evaluate family-level search allocation predictions.

This consumes family_search_allocation_model rows. The unit is a
decision-local contrast family, not an action, not a pair winner, and not a
teacher preference. The metric asks whether K family evidence requests cover
high-regret contrast families inside each decision.
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
    "family_priority",
    "family_abs_ge_10_probability",
    "family_regret_mass_abs10",
    "family_abs_ge_15_probability",
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
        raise ValueError(f"family row {index} is action-label-like")
    if (row.get("label_policy") or {}).get("action_label") is not False:
        raise ValueError(f"family row {index} has action_label=true")
    serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
    for key in FORBIDDEN_LABEL_KEYS:
        if f'"{key}"' in serialized:
            raise ValueError(f"family row {index} contains forbidden key {key}")


def load_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for index, row in enumerate(iter_jsonl(path)):
        assert_no_action_label_leak(row, index=index)
        rows.append(row)
    return rows


def score_value(row: dict[str, Any], score_name: str) -> float:
    signals = row.get("search_allocation_signals") or {}
    outputs = row.get("model_outputs") or {}
    if score_name in signals:
        return safe_float(signals.get(score_name))
    if score_name in outputs:
        return safe_float(outputs.get(score_name))
    return 0.0


def target_mass(row: dict[str, Any], *, threshold: float) -> float:
    targets = row.get("targets") or {}
    if threshold >= 15.0:
        return safe_float(targets.get("regret_mass_abs15"))
    return safe_float(targets.get("regret_mass_abs10"))


def is_target(row: dict[str, Any], *, threshold: float) -> bool:
    targets = row.get("targets") or {}
    if threshold >= 15.0:
        return bool(targets.get("high_regret_abs15"))
    return bool(targets.get("high_regret_abs10"))


def evaluate_selection(
    groups: dict[str, list[dict[str, Any]]],
    *,
    score_name: str,
    budget: int,
    threshold: float,
) -> dict[str, Any]:
    eligible_decisions = 0
    decision_any_hits = 0
    total_target_families = 0
    hit_target_families = 0
    total_regret_mass = 0.0
    hit_regret_mass = 0.0
    selected_family_counts: Counter[str] = Counter()
    missed_family_counts: Counter[str] = Counter()

    for rows in groups.values():
        targets = [row for row in rows if is_target(row, threshold=threshold)]
        if not targets:
            continue
        eligible_decisions += 1
        ranked = sorted(
            rows,
            key=lambda row: (-score_value(row, score_name), str(row.get("family"))),
        )
        selected = ranked[:budget]
        selected_families = {str(row.get("family")) for row in selected}
        target_families = {str(row.get("family")) for row in targets}
        family_hits = target_families & selected_families
        if family_hits:
            decision_any_hits += 1
        total_target_families += len(target_families)
        hit_target_families += len(family_hits)
        masses = {str(row.get("family")): target_mass(row, threshold=threshold) for row in targets}
        total_regret_mass += sum(masses.values())
        hit_regret_mass += sum(value for key, value in masses.items() if key in selected_families)
        selected_family_counts.update(selected_families)
        missed_family_counts.update(target_families - selected_families)

    return {
        "threshold": threshold,
        "budget": budget,
        "score_name": score_name,
        "eligible_decisions": eligible_decisions,
        "decision_any_family_recall": (
            decision_any_hits / eligible_decisions if eligible_decisions else None
        ),
        "target_family_recall": (
            hit_target_families / total_target_families if total_target_families else None
        ),
        "regret_mass_recall": (
            hit_regret_mass / total_regret_mass if total_regret_mass else None
        ),
        "avg_duplicate_budget_slots_per_eligible_decision": 0.0,
        "selected_family_top": dict(selected_family_counts.most_common(20)),
        "missed_family_top": dict(missed_family_counts.most_common(20)),
    }


def parse_number_list(text: str, *, kind: type) -> list[Any]:
    values: list[Any] = []
    for part in text.split(","):
        part = part.strip()
        if not part:
            continue
        values.append(kind(part))
    return values


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--family-predictions", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--budgets", default="1,2,3,5")
    parser.add_argument("--thresholds", default="10,15")
    parser.add_argument("--score-names", default=",".join(DEFAULT_SCORES))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    rows = load_rows(args.family_predictions)
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        groups[str(row.get("decision_key"))].append(row)
    checks: list[dict[str, Any]] = []
    for score_name in parse_number_list(args.score_names, kind=str):
        for threshold in parse_number_list(args.thresholds, kind=float):
            for budget in parse_number_list(args.budgets, kind=int):
                checks.append(
                    evaluate_selection(
                        groups,
                        score_name=score_name,
                        threshold=threshold,
                        budget=budget,
                    )
                )
    family_counts = Counter(str(row.get("family")) for row in rows)
    target10 = sum(1 for row in rows if is_target(row, threshold=10.0))
    target15 = sum(1 for row in rows if is_target(row, threshold=15.0))
    summary = {
        "schema_version": "family_prediction_allocation_eval_v0",
        "family_predictions": str(args.family_predictions),
        "decision_count": len(groups),
        "family_row_count": len(rows),
        "target_family_rows_abs10": target10,
        "target_family_rows_abs15": target15,
        "family_counts_top": dict(family_counts.most_common(30)),
        "checks": checks,
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "family_eval_is_search_allocation_not_policy": True,
        },
    }
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
