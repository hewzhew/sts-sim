#!/usr/bin/env python3
"""Merge branch value/risk datasets while namespacing branch ids.

This is for combining baseline and hard recollection supplements without branch
id collisions. It preserves outcome labels as branch/pair outcomes only; it does
not create action labels, selected actions, winners, or preferences.
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


def namespaced(source: str, value: Any) -> Any:
    if not isinstance(value, str):
        return value
    return f"{source}:{value}"


def rewrite_branch(row: dict[str, Any], *, source: str) -> dict[str, Any]:
    out = dict(row)
    old_branch_id = out.get("branch_id")
    out["dataset_source_name"] = source
    out["original_branch_id"] = old_branch_id
    out["branch_id"] = namespaced(source, old_branch_id)
    return out


def rewrite_pair_side(side: Any, *, source: str) -> Any:
    if not isinstance(side, dict):
        return side
    out = dict(side)
    old_branch_id = out.get("branch_id")
    out["original_branch_id"] = old_branch_id
    out["branch_id"] = namespaced(source, old_branch_id)
    return out


def rewrite_pair(row: dict[str, Any], *, source: str) -> dict[str, Any]:
    out = dict(row)
    out["dataset_source_name"] = source
    out["left"] = rewrite_pair_side(out.get("left"), source=source)
    out["right"] = rewrite_pair_side(out.get("right"), source=source)
    if isinstance(out.get("comparison_id"), str):
        out["original_comparison_id"] = out["comparison_id"]
        out["comparison_id"] = namespaced(source, out["comparison_id"])
    return out


def write_jsonl(path: Path, rows: Iterable[dict[str, Any]]) -> int:
    path.parent.mkdir(parents=True, exist_ok=True)
    count = 0
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")
            count += 1
    return count


def parse_source(text: str) -> tuple[str, Path, Path]:
    parts = text.split("=", 1)
    if len(parts) != 2 or not parts[0]:
        raise argparse.ArgumentTypeError("source must be NAME=BRANCHES,PAIRS")
    files = parts[1].split(",", 1)
    if len(files) != 2:
        raise argparse.ArgumentTypeError("source must be NAME=BRANCHES,PAIRS")
    return parts[0], Path(files[0]), Path(files[1])


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--source",
        action="append",
        required=True,
        type=parse_source,
        help="NAME=branches.jsonl,pairs.jsonl; may be repeated",
    )
    parser.add_argument("--branch-out", type=Path, required=True)
    parser.add_argument("--pair-out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    source_summaries: list[dict[str, Any]] = []

    def branch_rows() -> Iterable[dict[str, Any]]:
        for source, branch_path, _ in args.source:
            count = 0
            for index, row in enumerate(iter_jsonl(branch_path)):
                assert_safe(row, path=branch_path, index=index)
                count += 1
                yield rewrite_branch(row, source=source)
            source_summaries.append(
                {"source": source, "branch_path": str(branch_path), "branch_count": count}
            )

    branch_count = write_jsonl(args.branch_out, branch_rows())
    pair_source_counts: dict[str, int] = {}

    def pair_rows() -> Iterable[dict[str, Any]]:
        for source, _, pair_path in args.source:
            count = 0
            for index, row in enumerate(iter_jsonl(pair_path)):
                assert_safe(row, path=pair_path, index=index)
                count += 1
                yield rewrite_pair(row, source=source)
            pair_source_counts[source] = count

    pair_count = write_jsonl(args.pair_out, pair_rows())
    for source_summary in source_summaries:
        source_summary["pair_count"] = pair_source_counts.get(source_summary["source"], 0)
    summary = {
        "schema_version": "merged_branch_value_risk_dataset_summary_v0",
        "branch_out": str(args.branch_out),
        "pair_out": str(args.pair_out),
        "branch_count": branch_count,
        "pair_count": pair_count,
        "sources": source_summaries,
        "branch_ids_namespaced": True,
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "merged_dataset_is_not_policy": True,
        },
    }
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
