#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any

import numpy as np

from build_structured_candidate_value_dataset import (
    candidate_group_diagnostics,
    hard_group_checks,
    summarize_group_diagnostics,
)
from combat_rl_common import REPO_ROOT, iter_jsonl, write_json, write_jsonl


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    return [row for _, row in iter_jsonl(path)]


def should_keep_group(group_diagnostics: dict[str, Any], args: argparse.Namespace) -> bool:
    checks = hard_group_checks(group_diagnostics, args)
    if not checks:
        return True
    if args.hard_group_match == "all":
        return all(checks.values())
    return any(checks.values())


def load_npz_payload(path: Path) -> dict[str, np.ndarray]:
    with np.load(path, allow_pickle=False) as payload:
        return {key: np.asarray(payload[key]) for key in payload.files}


def write_filtered_npz(
    path: Path,
    source_payload: dict[str, np.ndarray],
    selected_indices: list[int],
    new_group_ids: list[int],
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    index_array = np.asarray(selected_indices, dtype=np.int64)
    payload: dict[str, np.ndarray] = {}
    for key, values in source_payload.items():
        payload[key] = np.asarray(values)[index_array]
    payload["group_index"] = np.asarray(new_group_ids, dtype=np.float32)
    np.savez_compressed(path, **payload)


def main() -> None:
    parser = argparse.ArgumentParser(description="Filter a structured candidate value dataset by root group hardness.")
    parser.add_argument("--dataset", required=True, type=Path)
    parser.add_argument("--rows", default=None, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--rows-out", default=None, type=Path)
    parser.add_argument("--summary-out", default=None, type=Path)
    parser.add_argument("--max-groups", default=0, type=int)
    parser.add_argument("--min-top2-gap", default=0.0, type=float)
    parser.add_argument("--min-return-range", default=0.0, type=float)
    parser.add_argument("--require-survival-disagreement", action="store_true")
    parser.add_argument("--require-terminal-disagreement", action="store_true")
    parser.add_argument("--require-root-terminal-disagreement", action="store_true")
    parser.add_argument("--hard-group-match", choices=["any", "all"], default="any")
    args = parser.parse_args()

    source_rows_path = args.rows or args.dataset.with_suffix(".jsonl")
    source_rows = load_jsonl(source_rows_path)
    source_payload = load_npz_payload(args.dataset)
    sample_count = next(iter(source_payload.values())).shape[0] if source_payload else 0
    if len(source_rows) != sample_count:
        raise SystemExit(f"row count {len(source_rows)} does not match dataset samples {sample_count}")

    groups: dict[int, list[dict[str, Any]]] = defaultdict(list)
    for row in source_rows:
        groups[int(row["group_index"])].append(row)

    selected_indices: list[int] = []
    new_group_ids: list[int] = []
    output_rows: list[dict[str, Any]] = []
    accepted_diagnostics: list[dict[str, Any]] = []
    rejected = 0
    selected_group_count = 0

    for original_group_id in sorted(groups):
        group_rows = sorted(groups[original_group_id], key=lambda row: int(row["sample_index"]))
        group_diagnostics = candidate_group_diagnostics(group_rows)
        if not should_keep_group(group_diagnostics, args):
            rejected += 1
            continue
        if int(args.max_groups) > 0 and selected_group_count >= int(args.max_groups):
            break
        new_group_id = selected_group_count
        selected_group_count += 1
        accepted_diagnostics.append(group_diagnostics)
        for row in group_rows:
            source_index = int(row["sample_index"])
            selected_indices.append(source_index)
            new_group_ids.append(new_group_id)
            out_row = dict(row)
            out_row["source_sample_index"] = source_index
            out_row["source_group_index"] = int(row["group_index"])
            out_row["sample_index"] = len(output_rows)
            out_row["group_index"] = new_group_id
            out_row["group_diagnostics"] = group_diagnostics
            output_rows.append(out_row)

    if not selected_indices:
        raise SystemExit("candidate value filtering selected no samples")

    rows_out = args.rows_out or args.out.with_suffix(".jsonl")
    summary_out = args.summary_out or args.out.with_suffix(".summary.json")
    write_filtered_npz(args.out, source_payload, selected_indices, new_group_ids)
    write_jsonl(rows_out, output_rows)
    summary = {
        "dataset": str(args.out),
        "rows": str(rows_out),
        "source_dataset": str(args.dataset),
        "source_rows": str(source_rows_path),
        "source_groups": int(len(groups)),
        "source_samples": int(sample_count),
        "selected_groups": int(selected_group_count),
        "selected_samples": int(len(output_rows)),
        "rejected_groups": int(rejected),
        "filters": {
            "min_top2_gap": float(args.min_top2_gap),
            "min_return_range": float(args.min_return_range),
            "require_survival_disagreement": bool(args.require_survival_disagreement),
            "require_terminal_disagreement": bool(args.require_terminal_disagreement),
            "require_root_terminal_disagreement": bool(args.require_root_terminal_disagreement),
            "match": args.hard_group_match,
            "max_groups": int(args.max_groups),
        },
        "accepted_group_diagnostics": summarize_group_diagnostics(accepted_diagnostics),
        "notes": [
            "sample_index and group_index are renumbered for the filtered dataset",
            "source_sample_index and source_group_index preserve provenance in rows JSONL",
        ],
    }
    write_json(summary_out, summary)
    print(json.dumps(summary, indent=2, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
