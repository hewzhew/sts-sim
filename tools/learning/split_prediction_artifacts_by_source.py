#!/usr/bin/env python3
"""Split branch/pair prediction artifacts into baseline and hard subsets.

This preserves audit-only semantics. It only copies rows; it does not create
winner/preference/action labels.
"""

from __future__ import annotations

import argparse
import json
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


def assert_safe(row: dict[str, Any], *, path: Path, index: int) -> None:
    if row.get("trainable_as_action_label") is not False:
        raise ValueError(f"{path}:{index} is action-label-like")
    if (row.get("label_policy") or {}).get("action_label") is not False:
        raise ValueError(f"{path}:{index} has action_label=true")
    serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
    for key in FORBIDDEN_LABEL_KEYS:
        if f'"{key}"' in serialized:
            raise ValueError(f"{path}:{index} contains forbidden key {key}")


def branch_is_baseline(row: dict[str, Any]) -> bool:
    branch_id = row.get("branch_id")
    return isinstance(branch_id, str) and branch_id.startswith("baseline:")


def pair_is_baseline(row: dict[str, Any]) -> bool:
    left_id = ((row.get("left") or {}).get("branch_id"))
    right_id = ((row.get("right") or {}).get("branch_id"))
    return (
        isinstance(left_id, str)
        and isinstance(right_id, str)
        and left_id.startswith("baseline:")
        and right_id.startswith("baseline:")
    )


def split_file(
    src: Path,
    baseline_out: Path,
    hard_out: Path,
    *,
    is_baseline_fn,
) -> dict[str, int]:
    baseline_out.parent.mkdir(parents=True, exist_ok=True)
    hard_out.parent.mkdir(parents=True, exist_ok=True)
    counts = {"baseline": 0, "hard": 0}
    with baseline_out.open("w", encoding="utf-8") as baseline_handle, hard_out.open(
        "w", encoding="utf-8"
    ) as hard_handle:
        for index, row in enumerate(iter_jsonl(src)):
            assert_safe(row, path=src, index=index)
            if is_baseline_fn(row):
                baseline_handle.write(json.dumps(row, separators=(",", ":")) + "\n")
                counts["baseline"] += 1
            else:
                hard_handle.write(json.dumps(row, separators=(",", ":")) + "\n")
                counts["hard"] += 1
    return counts


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--branch-predictions", type=Path, required=True)
    parser.add_argument("--pair-predictions", type=Path, required=True)
    parser.add_argument("--baseline-branch-out", type=Path, required=True)
    parser.add_argument("--baseline-pair-out", type=Path, required=True)
    parser.add_argument("--hard-branch-out", type=Path, required=True)
    parser.add_argument("--hard-pair-out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    branch_counts = split_file(
        args.branch_predictions,
        args.baseline_branch_out,
        args.hard_branch_out,
        is_baseline_fn=branch_is_baseline,
    )
    pair_counts = split_file(
        args.pair_predictions,
        args.baseline_pair_out,
        args.hard_pair_out,
        is_baseline_fn=pair_is_baseline,
    )
    summary = {
        "schema_version": "prediction_artifact_source_split_summary_v0",
        "branch_predictions": str(args.branch_predictions),
        "pair_predictions": str(args.pair_predictions),
        "branch_counts": branch_counts,
        "pair_counts": pair_counts,
        "source_rule": "baseline iff branch ids start with baseline:",
        "label_safety": {
            "action_policy_trained": False,
            "winner_or_preference_label_used": False,
            "split_is_not_policy": True,
        },
    }
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
