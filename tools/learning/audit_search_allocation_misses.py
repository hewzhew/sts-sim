#!/usr/bin/env python3
"""Summarize search-allocation gate misses.

This is an audit tool for model-guided search allocation, not a policy trainer.
It reads gate miss JSONL rows and reports where budget recall failed by slice,
objective, score, action-kind pair, card pair, and seed/step.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
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


def parse_decision_key(value: Any) -> dict[str, Any]:
    if not isinstance(value, str):
        return {}
    try:
        parsed = json.loads(value)
    except json.JSONDecodeError:
        return {}
    return parsed if isinstance(parsed, dict) else {}


def candidate_summary(candidate: dict[str, Any]) -> str:
    kind = candidate.get("action_kind")
    card = candidate.get("card_id")
    if kind == "play_card":
        return f"{kind}:{card}"
    return str(kind or "unknown")


def pair_kind(item: dict[str, Any]) -> str:
    left = item.get("left_candidate") or {}
    right = item.get("right_candidate") or {}
    if not left and not right:
        candidate = item.get("candidate") or {}
        return f"{candidate_summary(candidate)}->None"
    return f"{candidate_summary(left)}->{candidate_summary(right)}"


def action_kind_pair(item: dict[str, Any]) -> str:
    left = item.get("left_candidate") or {}
    right = item.get("right_candidate") or {}
    if not left and not right:
        candidate = item.get("candidate") or {}
        return f"{candidate.get('action_kind') or 'unknown'}->None"
    return f"{left.get('action_kind') or 'unknown'}->{right.get('action_kind') or 'unknown'}"


def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return default
    return number if number == number else default


def target_magnitude(item: dict[str, Any], allocation_kind: str) -> float:
    targets = item.get("targets") or {}
    if allocation_kind == "pair":
        return abs(safe_float(targets.get("hp_left_minus_right")))
    return abs(safe_float(targets.get("hp_delta")))


def assert_no_label_leak(row: dict[str, Any], *, index: int) -> None:
    if row.get("trainable_as_action_label") is not False:
        raise ValueError(f"miss row {index} is action-label-like")
    if (row.get("label_policy") or {}).get("action_label") is not False:
        raise ValueError(f"miss row {index} has action_label=true")
    serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
    for key in FORBIDDEN_LABEL_KEYS:
        if f'"{key}"' in serialized:
            raise ValueError(f"miss row {index} contains forbidden key {key}")


def passes_filters(row: dict[str, Any], args: argparse.Namespace) -> bool:
    if args.slice_name and row.get("slice_name") != args.slice_name:
        return False
    if args.allocation_kind and row.get("allocation_kind") != args.allocation_kind:
        return False
    if args.objective and row.get("objective") != args.objective:
        return False
    if args.score_name and row.get("score_name") != args.score_name:
        return False
    if args.required_only and not row.get("required_gate_check"):
        return False
    return True


def audit(rows: list[dict[str, Any]]) -> dict[str, Any]:
    row_counts: Counter[str] = Counter()
    item_counts: Counter[str] = Counter()
    pair_counts: Counter[str] = Counter()
    action_pair_counts: Counter[str] = Counter()
    action_pair_magnitude_counts: Counter[str] = Counter()
    card_counts: Counter[str] = Counter()
    seed_counts: Counter[str] = Counter()
    objective_score_counts: Counter[str] = Counter()
    magnitude_buckets: Counter[str] = Counter()
    top_items: list[dict[str, Any]] = []
    decision_keys: set[str] = set()
    required_rows = 0
    for row in rows:
        allocation_kind = str(row.get("allocation_kind") or "unknown")
        key = (
            f"{row.get('slice_name')}|{allocation_kind}|{row.get('objective')}|"
            f"{row.get('score_name')}|budget:{row.get('budget')}"
        )
        row_counts[key] += 1
        objective_score_counts[f"{row.get('objective')}|{row.get('score_name')}"] += 1
        if row.get("required_gate_check"):
            required_rows += 1
        decision = parse_decision_key(row.get("decision_key"))
        seed = decision.get("episode_seed")
        step = decision.get("episode_step")
        if seed is not None:
            seed_counts[str(seed)] += 1
        decision_keys.add(str(row.get("decision_key")))
        for item in row.get("missed_target_items") or []:
            item_counts[allocation_kind] += 1
            kind = pair_kind(item)
            pair_counts[kind] += 1
            coarse_kind = action_kind_pair(item)
            action_pair_counts[coarse_kind] += 1
            if "->" in kind:
                card_counts[kind] += 1
            magnitude = target_magnitude(item, allocation_kind)
            if magnitude >= 20:
                bucket = ">=20"
            elif magnitude >= 15:
                bucket = "15..19"
            elif magnitude >= 10:
                bucket = "10..14"
            elif magnitude >= 5:
                bucket = "5..9"
            else:
                bucket = "<5"
            magnitude_buckets[bucket] += 1
            action_pair_magnitude_counts[f"{coarse_kind}|{bucket}"] += 1
            top_items.append(
                {
                    "priority": magnitude + (5.0 if row.get("required_gate_check") else 0.0),
                    "slice_name": row.get("slice_name"),
                    "allocation_kind": allocation_kind,
                    "objective": row.get("objective"),
                    "score_name": row.get("score_name"),
                    "budget": row.get("budget"),
                    "episode_seed": seed,
                    "episode_step": step,
                    "kind": kind,
                    "targets": item.get("targets") or {},
                }
            )
    top_items.sort(
        key=lambda item: (
            -safe_float(item.get("priority")),
            int(item.get("episode_seed") or 0),
            int(item.get("episode_step") or 0),
            str(item.get("kind") or ""),
        )
    )
    return {
        "schema_version": "search_allocation_miss_audit_v0",
        "miss_row_count": len(rows),
        "required_miss_row_count": required_rows,
        "miss_decision_count": len(decision_keys),
        "missed_item_counts_by_kind": dict(item_counts),
        "miss_rows_by_slice_kind_objective_score": dict(row_counts),
        "objective_score_counts": dict(objective_score_counts),
        "miss_rows_by_seed": dict(seed_counts),
        "missed_target_pair_kind_top": dict(pair_counts.most_common(30)),
        "missed_target_action_kind_pair_counts": dict(action_pair_counts.most_common()),
        "missed_target_action_kind_pair_magnitude_counts": dict(
            action_pair_magnitude_counts.most_common()
        ),
        "missed_target_card_pair_top": dict(card_counts.most_common(30)),
        "missed_target_magnitude_buckets": dict(magnitude_buckets),
        "top_missed_items": top_items[:50],
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "audit_is_search_allocation_not_policy": True,
        },
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--misses", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--slice-name")
    parser.add_argument("--allocation-kind")
    parser.add_argument("--objective")
    parser.add_argument("--score-name")
    parser.add_argument("--required-only", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    rows: list[dict[str, Any]] = []
    for index, row in enumerate(iter_jsonl(args.misses)):
        assert_no_label_leak(row, index=index)
        if passes_filters(row, args):
            rows.append(row)
    summary = audit(rows)
    summary.update(
        {
            "misses": str(args.misses),
            "filters": {
                "slice_name": args.slice_name,
                "allocation_kind": args.allocation_kind,
                "objective": args.objective,
                "score_name": args.score_name,
                "required_only": args.required_only,
            },
        }
    )
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
