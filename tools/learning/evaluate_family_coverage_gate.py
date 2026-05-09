#!/usr/bin/env python3
"""Gate family-coverage search allocation summaries.

This consumes evaluate_family_coverage_allocation.py summaries and applies
explicit pass/fail checks. The checks are about search-allocation coverage, not
action labels, preferences, winners, or live takeover.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


FORBIDDEN_LABEL_KEYS = {
    "winner",
    "preferred",
    "preferred_action",
    "selected_action",
    "teacher_choice",
}


DEFAULT_CHECKS = [
    {
        "name": "abs10_k3_primary_family_decision_recall",
        "threshold": 10.0,
        "budget": 3,
        "family_mode": "primary_tag",
        "selection_mode": "family_topk",
        "score_name": "allocation_abs_hp_diff_ge_10_probability",
        "metric": "decision_any_family_recall",
        "min_value": 0.93,
        "required": True,
    },
    {
        "name": "abs10_k3_primary_family_regret_mass",
        "threshold": 10.0,
        "budget": 3,
        "family_mode": "primary_tag",
        "selection_mode": "family_topk",
        "score_name": "allocation_abs_hp_diff_ge_10_probability",
        "metric": "regret_mass_recall",
        "min_value": 0.80,
        "required": True,
    },
    {
        "name": "abs10_k3_primary_family_target_family_recall",
        "threshold": 10.0,
        "budget": 3,
        "family_mode": "primary_tag",
        "selection_mode": "family_topk",
        "score_name": "allocation_abs_hp_diff_ge_10_probability",
        "metric": "target_family_recall",
        "min_value": 0.75,
        "required": True,
    },
    {
        "name": "abs10_k3_primary_no_duplicate_budget",
        "threshold": 10.0,
        "budget": 3,
        "family_mode": "primary_tag",
        "selection_mode": "family_topk",
        "score_name": "allocation_abs_hp_diff_ge_10_probability",
        "metric": "avg_duplicate_budget_slots_per_eligible_decision",
        "max_value": 0.0,
        "required": True,
    },
    {
        "name": "abs15_k5_primary_family_decision_recall",
        "threshold": 15.0,
        "budget": 5,
        "family_mode": "primary_tag",
        "selection_mode": "family_topk",
        "score_name": "allocation_abs_hp_diff_ge_10_probability",
        "metric": "decision_any_family_recall",
        "min_value": 0.90,
        "required": False,
    },
    {
        "name": "abs10_k3_pair_topk_duplicate_watch",
        "threshold": 10.0,
        "budget": 3,
        "family_mode": "primary_tag",
        "selection_mode": "pair_topk",
        "score_name": "allocation_abs_hp_diff_ge_10_probability",
        "metric": "avg_duplicate_budget_slots_per_eligible_decision",
        "max_value": 0.50,
        "required": False,
        "expected_to_fail_reason": "pair_topk_duplicate_budget_baseline",
    },
]


def read_json(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"{path} did not contain a JSON object")
    serialized = json.dumps(data, sort_keys=True, separators=(",", ":"))
    for key in FORBIDDEN_LABEL_KEYS:
        if f'"{key}"' in serialized:
            raise ValueError(f"{path} contains forbidden key {key}")
    label_safety = data.get("label_safety") or {}
    if label_safety.get("trainable_as_action_label") is not False:
        raise ValueError(f"{path} is action-label-like")
    if label_safety.get("winner_or_preference_label_used") is not False:
        raise ValueError(f"{path} uses winner/preference labels")
    return data


def find_check(summary: dict[str, Any], definition: dict[str, Any]) -> dict[str, Any] | None:
    for check in summary.get("checks") or []:
        if (
            float(check.get("threshold")) == float(definition["threshold"])
            and int(check.get("budget")) == int(definition["budget"])
            and check.get("family_mode") == definition["family_mode"]
            and check.get("selection_mode") == definition["selection_mode"]
            and check.get("score_name") == definition["score_name"]
        ):
            return check
    return None


def evaluate_definition(summary: dict[str, Any], definition: dict[str, Any]) -> dict[str, Any]:
    check = find_check(summary, definition)
    actual = None if check is None else check.get(definition["metric"])
    passed = False
    if isinstance(actual, (int, float)):
        if "min_value" in definition:
            passed = float(actual) >= float(definition["min_value"])
        elif "max_value" in definition:
            passed = float(actual) <= float(definition["max_value"])
    return {
        **definition,
        "actual": actual,
        "passed": passed,
        "eligible_decisions": None if check is None else check.get("eligible_decisions"),
        "target_family_recall": None if check is None else check.get("target_family_recall"),
        "regret_mass_recall": None if check is None else check.get("regret_mass_recall"),
        "decision_any_family_recall": (
            None if check is None else check.get("decision_any_family_recall")
        ),
        "avg_duplicate_budget_slots_per_eligible_decision": (
            None if check is None else check.get("avg_duplicate_budget_slots_per_eligible_decision")
        ),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--summary", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    source = read_json(args.summary)
    checks = [evaluate_definition(source, definition) for definition in DEFAULT_CHECKS]
    required_failures = [check for check in checks if check["required"] and not check["passed"]]
    watch_failures = [check for check in checks if not check["required"] and not check["passed"]]
    out = {
        "schema_version": "family_coverage_gate_v0",
        "source_summary": str(args.summary),
        "overall_status": "pass" if not required_failures else "fail",
        "required_failure_count": len(required_failures),
        "watch_failure_count": len(watch_failures),
        "watch_failures_are_diagnostics_not_gate_failures": True,
        "checks": checks,
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "family_gate_is_search_allocation_not_policy": True,
        },
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(out, indent=2), encoding="utf-8")
    print(json.dumps(out, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
