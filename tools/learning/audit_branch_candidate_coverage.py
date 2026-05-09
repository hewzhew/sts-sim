#!/usr/bin/env python3
"""Audit candidate sampling coverage in BranchTrace collection JSONL files."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any


def safe_int(value: Any, default: int = 0) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def action_kind(candidates: list[dict[str, Any]], index: Any) -> str:
    if not isinstance(index, int):
        return "unknown"
    if index < 0 or index >= len(candidates):
        return "invalid_index"
    return str(candidates[index].get("action_kind") or "unknown")


def action_key(candidates: list[dict[str, Any]], index: Any) -> str:
    if not isinstance(index, int) or index < 0 or index >= len(candidates):
        return "invalid"
    return str(candidates[index].get("action_key") or candidates[index].get("action_kind") or "unknown")


def bump(counter: dict[str, Any], key: str, amount: int = 1) -> None:
    counter[key] = int(counter.get(key) or 0) + amount


def load_records(paths: list[Path]):
    for path in paths:
        with path.open("r", encoding="utf-8") as handle:
            for line_no, line in enumerate(handle, start=1):
                line = line.strip()
                if not line:
                    continue
                record = json.loads(line)
                yield path, line_no, record


def audit(paths: list[Path], *, max_examples: int) -> dict[str, Any]:
    summary: dict[str, Any] = {
        "schema_version": "branch_candidate_coverage_audit_v1",
        "input_paths": [str(path) for path in paths],
        "record_count": 0,
        "decision_count": 0,
        "legal_candidate_count_total": 0,
        "requested_candidate_count_total": 0,
        "included_candidate_count_total": 0,
        "excluded_candidate_count_total": 0,
        "behavior_missing_count": 0,
        "large_candidate_decision_count": 0,
        "scope_filtered_decision_count": 0,
        "uses_forbidden_selector_signal_count": 0,
        "legal_action_kind_counts": {},
        "requested_action_kind_counts": {},
        "included_action_kind_counts": {},
        "excluded_action_kind_counts": {},
        "excluded_by_reason_counts": {},
        "sampling_spec_id_counts": {},
        "scope_counts": {},
        "examples": {
            "behavior_missing": [],
            "large_candidate_decisions": [],
            "scope_filtered": [],
            "forbidden_selector_signal": [],
        },
    }
    for path, line_no, record in load_records(paths):
        summary["record_count"] += 1
        payload = record.get("branch_trace_batch") or record.get("payload") or {}
        if not payload:
            continue
        summary["decision_count"] += 1
        traces = payload.get("traces") or []
        candidates: list[dict[str, Any]] = []
        if traces:
            candidates = traces[0].get("candidates") or []
        sampling = payload.get("candidate_sampling_spec") or {}
        requested = payload.get("requested_action_indices") or []
        included = payload.get("action_indices") or []
        requested_set = {index for index in requested if isinstance(index, int)}
        included_set = {index for index in included if isinstance(index, int)}
        excluded_set = requested_set - included_set

        legal_count = safe_int(sampling.get("legal_candidate_count"), len(candidates))
        summary["legal_candidate_count_total"] += legal_count
        summary["requested_candidate_count_total"] += len(requested_set)
        summary["included_candidate_count_total"] += len(included_set)
        summary["excluded_candidate_count_total"] += len(excluded_set)

        for candidate in candidates:
            bump(summary["legal_action_kind_counts"], str(candidate.get("action_kind") or "unknown"))
        for index in requested_set:
            bump(summary["requested_action_kind_counts"], action_kind(candidates, index))
        for index in included_set:
            bump(summary["included_action_kind_counts"], action_kind(candidates, index))
        for index in excluded_set:
            bump(summary["excluded_action_kind_counts"], action_kind(candidates, index))

        spec_id = str(sampling.get("candidate_sampling_spec_id") or "unknown")
        scope = str(sampling.get("scope") or payload.get("candidate_scope") or "unknown")
        bump(summary["sampling_spec_id_counts"], spec_id)
        bump(summary["scope_counts"], scope)
        for reason, count in (sampling.get("excluded_by_reason") or {}).items():
            bump(summary["excluded_by_reason_counts"], str(reason), safe_int(count))

        behavior_action_id = sampling.get("behavior_action_id")
        if sampling.get("include_behavior_action") is not True:
            summary["behavior_missing_count"] += 1
            if len(summary["examples"]["behavior_missing"]) < max_examples:
                summary["examples"]["behavior_missing"].append(
                    {
                        "path": str(path),
                        "line": line_no,
                        "seed": record.get("seed"),
                        "episode_step": record.get("episode_step"),
                        "decision_type": record.get("decision_type"),
                        "behavior_action_id": behavior_action_id,
                        "behavior_action_key": action_key(candidates, behavior_action_id),
                    }
                )
        if legal_count > len(included_set):
            summary["large_candidate_decision_count"] += 1
            if len(summary["examples"]["large_candidate_decisions"]) < max_examples:
                summary["examples"]["large_candidate_decisions"].append(
                    {
                        "path": str(path),
                        "line": line_no,
                        "seed": record.get("seed"),
                        "episode_step": record.get("episode_step"),
                        "decision_type": record.get("decision_type"),
                        "legal_candidate_count": legal_count,
                        "included_candidate_count": len(included_set),
                        "excluded_kinds": Counter(
                            action_kind(candidates, index) for index in excluded_set
                        ),
                    }
                )
        scope_filtered = safe_int((sampling.get("excluded_by_reason") or {}).get("scope_filter"))
        if scope_filtered > 0:
            summary["scope_filtered_decision_count"] += 1
            if len(summary["examples"]["scope_filtered"]) < max_examples:
                summary["examples"]["scope_filtered"].append(
                    {
                        "path": str(path),
                        "line": line_no,
                        "seed": record.get("seed"),
                        "episode_step": record.get("episode_step"),
                        "decision_type": record.get("decision_type"),
                        "scope_filtered": scope_filtered,
                        "excluded_kinds": Counter(
                            action_kind(candidates, index) for index in excluded_set
                        ),
                    }
                )
        forbidden_flags = [
            "uses_neutral_signal",
            "uses_legacy_best_move",
            "uses_exact_turn_best_line",
            "uses_frontier_eval_score",
        ]
        if any(sampling.get(flag) for flag in forbidden_flags):
            summary["uses_forbidden_selector_signal_count"] += 1
            if len(summary["examples"]["forbidden_selector_signal"]) < max_examples:
                summary["examples"]["forbidden_selector_signal"].append(
                    {
                        "path": str(path),
                        "line": line_no,
                        "seed": record.get("seed"),
                        "episode_step": record.get("episode_step"),
                        "decision_type": record.get("decision_type"),
                        "flags": {flag: sampling.get(flag) for flag in forbidden_flags},
                    }
                )
    legal_total = safe_int(summary.get("legal_candidate_count_total"))
    included_total = safe_int(summary.get("included_candidate_count_total"))
    requested_total = safe_int(summary.get("requested_candidate_count_total"))
    summary["included_over_legal_candidate_ratio"] = included_total / legal_total if legal_total else 0.0
    summary["included_over_requested_candidate_ratio"] = (
        included_total / requested_total if requested_total else 0.0
    )
    return summary


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("records", type=Path, nargs="+")
    parser.add_argument("--summary-out", type=Path)
    parser.add_argument("--max-examples", type=int, default=20)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    summary = audit(args.records, max_examples=args.max_examples)
    if args.summary_out:
        args.summary_out.parent.mkdir(parents=True, exist_ok=True)
        args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
