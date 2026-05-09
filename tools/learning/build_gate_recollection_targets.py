#!/usr/bin/env python3
"""Build targeted-recollection rows from Search Allocation Gate misses.

The output is compatible with collect_targeted_branch_traces.py. It is a data
collection target queue, not a policy target and not an action label.
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


def bump(counter: dict[str, int], key: str, amount: int = 1) -> None:
    counter[key] = int(counter.get(key) or 0) + amount


def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return default
    return number if number == number else default


def parse_decision_key(text: Any) -> dict[str, Any]:
    if not isinstance(text, str):
        return {}
    try:
        parsed = json.loads(text)
    except json.JSONDecodeError:
        return {}
    return parsed if isinstance(parsed, dict) else {}


def action_key(candidate: dict[str, Any]) -> str | None:
    value = candidate.get("action_key")
    return value if isinstance(value, str) else None


def pair_kind(left: dict[str, Any], right: dict[str, Any]) -> str:
    return f"{left.get('action_kind')}->{right.get('action_kind')}"


def pair_card(left: dict[str, Any], right: dict[str, Any]) -> str:
    return f"{left.get('card_id')}->{right.get('card_id')}"


def miss_reasons(miss: dict[str, Any]) -> list[str]:
    reasons = [
        "search_allocation_gate_miss",
        f"gate_objective:{miss.get('objective')}",
        f"gate_score:{miss.get('score_name')}",
        f"gate_budget:{miss.get('budget')}",
    ]
    if miss.get("required_gate_check"):
        reasons.append("required_gate_miss")
    else:
        reasons.append("watch_gate_miss")
    if miss.get("slice_name"):
        reasons.append(f"slice:{miss.get('slice_name')}")
    return reasons


def pair_target_from_item(
    *,
    miss: dict[str, Any],
    target_item: dict[str, Any],
    target_index: int,
) -> dict[str, Any] | None:
    decision = parse_decision_key(miss.get("decision_key"))
    seed = decision.get("episode_seed")
    step = decision.get("episode_step")
    if not isinstance(seed, int) or not isinstance(step, int):
        return None
    left_candidate = target_item.get("left_candidate") or {}
    right_candidate = target_item.get("right_candidate") or {}
    if not action_key(left_candidate) and not action_key(right_candidate):
        return None
    targets = target_item.get("targets") or {}
    true_hp = safe_float(targets.get("hp_left_minus_right"))
    priority = abs(true_hp)
    if miss.get("required_gate_check"):
        priority += 5.0
    if "severe_underestimate" in str(miss.get("objective")):
        priority += 10.0
    return {
        "schema_version": "gate_recollection_target_v1",
        "trainable_role": "search_allocation_recollection_target",
        "trainable_as_action_label": False,
        "episode_seed": seed,
        "episode_step": step,
        "decision_id": decision.get("decision_id"),
        "source_gate": {
            "schema_version": "search_allocation_gate_target_source_v1",
            "slice_name": miss.get("slice_name"),
            "allocation_kind": miss.get("allocation_kind"),
            "score_name": miss.get("score_name"),
            "objective": miss.get("objective"),
            "budget": miss.get("budget"),
            "required_gate_check": bool(miss.get("required_gate_check")),
            "gate_decision_key": miss.get("decision_key"),
        },
        "priority_score": priority,
        "reasons": miss_reasons(miss),
        "target_index_within_gate_miss": target_index,
        "pair_kind": pair_kind(left_candidate, right_candidate),
        "pair_card": pair_card(left_candidate, right_candidate),
        "left": {
            "branch_id": target_item.get("left_branch_id"),
            "candidate": left_candidate,
        },
        "right": {
            "branch_id": target_item.get("right_branch_id"),
            "candidate": right_candidate,
        },
        "targets": targets,
        "label_policy": {
            "action_label": False,
            "source": "search_allocation_gate_v1",
        },
    }


def branch_target_from_item(
    *,
    miss: dict[str, Any],
    target_item: dict[str, Any],
    target_index: int,
) -> dict[str, Any] | None:
    decision = parse_decision_key(miss.get("decision_key"))
    seed = decision.get("episode_seed")
    step = decision.get("episode_step")
    if not isinstance(seed, int) or not isinstance(step, int):
        return None
    candidate = target_item.get("candidate") or {}
    if not action_key(candidate):
        return None
    return {
        "schema_version": "gate_recollection_target_v1",
        "trainable_role": "search_allocation_recollection_target",
        "trainable_as_action_label": False,
        "episode_seed": seed,
        "episode_step": step,
        "decision_id": decision.get("decision_id"),
        "source_gate": {
            "schema_version": "search_allocation_gate_target_source_v1",
            "slice_name": miss.get("slice_name"),
            "allocation_kind": miss.get("allocation_kind"),
            "score_name": miss.get("score_name"),
            "objective": miss.get("objective"),
            "budget": miss.get("budget"),
            "required_gate_check": bool(miss.get("required_gate_check")),
            "gate_decision_key": miss.get("decision_key"),
        },
        "priority_score": 5.0 + (5.0 if miss.get("required_gate_check") else 0.0),
        "reasons": miss_reasons(miss),
        "target_index_within_gate_miss": target_index,
        "pair_kind": f"{candidate.get('action_kind')}->None",
        "pair_card": f"{candidate.get('card_id')}->None",
        "left": {
            "branch_id": target_item.get("branch_id"),
            "candidate": candidate,
        },
        "right": {
            "branch_id": None,
            "candidate": {},
        },
        "targets": target_item.get("targets") or {},
        "label_policy": {
            "action_label": False,
            "source": "search_allocation_gate_v1",
        },
    }


def miss_passes_filters(miss: dict[str, Any], args: argparse.Namespace) -> bool:
    if args.slice_name and miss.get("slice_name") != args.slice_name:
        return False
    if args.allocation_kind and miss.get("allocation_kind") != args.allocation_kind:
        return False
    if args.objective and miss.get("objective") != args.objective:
        return False
    if args.score_name and miss.get("score_name") != args.score_name:
        return False
    if args.required_only and not miss.get("required_gate_check"):
        return False
    return True


def build_targets(
    misses: list[dict[str, Any]],
    *,
    max_rows: int,
    args: argparse.Namespace,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    targets: list[dict[str, Any]] = []
    summary: dict[str, Any] = {
        "schema_version": "gate_recollection_target_build_summary_v1",
        "gate_miss_rows": len(misses),
        "target_rows_before_cap": 0,
        "target_rows": 0,
        "target_decision_count": 0,
        "required_gate_target_count": 0,
        "watch_gate_target_count": 0,
        "objective_counts": {},
        "score_counts": {},
        "pair_kind_counts": {},
        "reason_counts": {},
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "targets_are_data_collection_not_policy": True,
        },
        "filters": {
            "slice_name": args.slice_name,
            "allocation_kind": args.allocation_kind,
            "objective": args.objective,
            "score_name": args.score_name,
            "required_only": args.required_only,
        },
    }
    seen: set[tuple[str, str, str, int]] = set()
    for miss in misses:
        if not miss_passes_filters(miss, args):
            continue
        if miss.get("trainable_as_action_label") is not False:
            continue
        if (miss.get("label_policy") or {}).get("action_label") is not False:
            continue
        target_items = miss.get("missed_target_items") or []
        for index, target_item in enumerate(target_items):
            if miss.get("allocation_kind") == "branch":
                target = branch_target_from_item(
                    miss=miss,
                    target_item=target_item,
                    target_index=index,
                )
            else:
                target = pair_target_from_item(
                    miss=miss,
                    target_item=target_item,
                    target_index=index,
                )
            if target is None:
                continue
            left_key = action_key(((target.get("left") or {}).get("candidate") or {})) or ""
            right_key = action_key(((target.get("right") or {}).get("candidate") or {})) or ""
            dedupe = (
                str(target.get("episode_seed")),
                str(target.get("episode_step")),
                left_key,
                right_key,
                int(target.get("target_index_within_gate_miss") or 0),
            )
            if dedupe in seen:
                continue
            seen.add(dedupe)
            targets.append(target)
    targets.sort(
        key=lambda row: (
            -safe_float(row.get("priority_score")),
            int(row.get("episode_seed") or 0),
            int(row.get("episode_step") or 0),
            str(row.get("pair_kind") or ""),
        )
    )
    summary["target_rows_before_cap"] = len(targets)
    if max_rows > 0:
        targets = targets[:max_rows]
    decisions = {(row.get("episode_seed"), row.get("episode_step")) for row in targets}
    summary["target_rows"] = len(targets)
    summary["target_decision_count"] = len(decisions)
    for row in targets:
        source = row.get("source_gate") or {}
        if source.get("required_gate_check"):
            summary["required_gate_target_count"] += 1
        else:
            summary["watch_gate_target_count"] += 1
        bump(summary["objective_counts"], str(source.get("objective")))
        bump(summary["score_counts"], str(source.get("score_name")))
        bump(summary["pair_kind_counts"], str(row.get("pair_kind")))
        for reason in row.get("reasons") or []:
            bump(summary["reason_counts"], str(reason))
    return targets, summary


def assert_no_action_label_leak(rows: list[dict[str, Any]]) -> None:
    for index, row in enumerate(rows):
        if row.get("trainable_as_action_label") is not False:
            raise ValueError(f"target row {index} is action-label-like")
        if (row.get("label_policy") or {}).get("action_label") is not False:
            raise ValueError(f"target row {index} has action_label=true")
        serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
        for key in FORBIDDEN_LABEL_KEYS:
            if f'"{key}"' in serialized:
                raise ValueError(f"target row {index} contains forbidden key {key}")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--gate-misses", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--max-rows", type=int, default=200)
    parser.add_argument("--slice-name")
    parser.add_argument("--allocation-kind")
    parser.add_argument("--objective")
    parser.add_argument("--score-name")
    parser.add_argument("--required-only", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    misses = list(iter_jsonl(args.gate_misses))
    targets, summary = build_targets(misses, max_rows=args.max_rows, args=args)
    assert_no_action_label_leak(targets)
    write_jsonl(args.out, targets)
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
