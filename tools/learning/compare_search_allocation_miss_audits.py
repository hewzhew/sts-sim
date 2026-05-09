#!/usr/bin/env python3
"""Compare two search-allocation miss audit summaries.

This is an audit helper for search allocation failures. It compares families of
misses across separate seed bands without turning outcome diffs into action
labels, winners, or preferences.
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
        raise ValueError(f"{path} is missing trainable_as_action_label=false")
    if label_safety.get("winner_or_preference_label_used") is not False:
        raise ValueError(f"{path} is action-preference-like")
    return data


def as_counts(value: Any) -> dict[str, int]:
    if not isinstance(value, dict):
        return {}
    counts: dict[str, int] = {}
    for key, raw in value.items():
        try:
            counts[str(key)] = int(raw)
        except (TypeError, ValueError):
            continue
    return counts


def count_total(counts: dict[str, int]) -> int:
    return sum(int(value) for value in counts.values())


def compare_counts(left: dict[str, int], right: dict[str, int], *, limit: int) -> dict[str, Any]:
    keys = set(left) | set(right)
    left_total = max(1, count_total(left))
    right_total = max(1, count_total(right))
    rows = []
    for key in keys:
        left_count = int(left.get(key) or 0)
        right_count = int(right.get(key) or 0)
        rows.append(
            {
                "key": key,
                "left_count": left_count,
                "right_count": right_count,
                "delta_count": right_count - left_count,
                "left_share": left_count / left_total,
                "right_share": right_count / right_total,
                "delta_share": (right_count / right_total) - (left_count / left_total),
                "present_in_both": left_count > 0 and right_count > 0,
            }
        )
    persistent = [row for row in rows if row["present_in_both"]]
    persistent.sort(
        key=lambda row: (
            -min(int(row["left_count"]), int(row["right_count"])),
            -int(row["right_count"]),
            str(row["key"]),
        )
    )
    increases = sorted(rows, key=lambda row: (-int(row["delta_count"]), str(row["key"])))
    decreases = sorted(rows, key=lambda row: (int(row["delta_count"]), str(row["key"])))
    current_top = sorted(rows, key=lambda row: (-int(row["right_count"]), str(row["key"])))
    return {
        "left_total": left_total if left else 0,
        "right_total": right_total if right else 0,
        "persistent_top": persistent[:limit],
        "current_top": current_top[:limit],
        "largest_increases": increases[:limit],
        "largest_decreases": decreases[:limit],
    }


def compact_summary(data: dict[str, Any]) -> dict[str, Any]:
    missed_items = as_counts(data.get("missed_item_counts_by_kind"))
    pair_items = int(missed_items.get("pair") or 0)
    branch_items = int(missed_items.get("branch") or 0)
    return {
        "miss_row_count": data.get("miss_row_count"),
        "required_miss_row_count": data.get("required_miss_row_count"),
        "miss_decision_count": data.get("miss_decision_count"),
        "missed_pair_item_count": pair_items,
        "missed_branch_item_count": branch_items,
        "magnitude_buckets": data.get("missed_target_magnitude_buckets") or {},
        "action_kind_pair_counts": data.get("missed_target_action_kind_pair_counts") or {},
        "filters": data.get("filters") or {},
        "misses": data.get("misses"),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--left-summary", type=Path, required=True)
    parser.add_argument("--right-summary", type=Path, required=True)
    parser.add_argument("--left-label", default="left")
    parser.add_argument("--right-label", default="right")
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--top-limit", type=int, default=20)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    left = read_json(args.left_summary)
    right = read_json(args.right_summary)
    comparison = {
        "schema_version": "search_allocation_miss_audit_comparison_v0",
        "left_label": args.left_label,
        "right_label": args.right_label,
        "left": compact_summary(left),
        "right": compact_summary(right),
        "action_kind_pair_comparison": compare_counts(
            as_counts(left.get("missed_target_action_kind_pair_counts")),
            as_counts(right.get("missed_target_action_kind_pair_counts")),
            limit=args.top_limit,
        ),
        "card_pair_comparison": compare_counts(
            as_counts(left.get("missed_target_pair_kind_top")),
            as_counts(right.get("missed_target_pair_kind_top")),
            limit=args.top_limit,
        ),
        "magnitude_bucket_comparison": compare_counts(
            as_counts(left.get("missed_target_magnitude_buckets")),
            as_counts(right.get("missed_target_magnitude_buckets")),
            limit=args.top_limit,
        ),
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "comparison_is_failure_family_audit_not_policy": True,
        },
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(comparison, indent=2), encoding="utf-8")
    print(json.dumps(comparison, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
