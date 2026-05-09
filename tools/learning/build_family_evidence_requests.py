#!/usr/bin/env python3
"""Build contrast-family evidence requests from pair prediction artifacts.

This script is the offline shape of the next allocator:

    decision -> contrast families -> representative pairs -> evidence requests

It does not choose actions, create action labels, or encode winners/preferences.
By default, output request rows do not include realized outcome targets; targets
are used only for the summary audit.
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


def load_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for index, row in enumerate(iter_jsonl(path)):
        assert_no_action_label_leak(row, index=index)
        rows.append(row)
    return rows


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
    for tag in (
        "end_turn",
        "damage",
        "block",
        "vulnerable",
        "weak",
        "setup",
        "draw",
        "exhaust",
        "play_card_other",
    ):
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


def score_value(row: dict[str, Any], score_name: str) -> float:
    signals = row.get("search_allocation_signals") or {}
    outputs = row.get("model_outputs") or {}
    tails = outputs.get("tail_probabilities") or {}
    allocation = row.get("allocation_model_outputs") or {}
    if score_name == "residual_corrected_abs_hp_diff":
        return abs(safe_float(outputs.get("residual_corrected_hp_left_minus_right")))
    if score_name == "branch_model_abs_hp_diff":
        return abs(safe_float(outputs.get("branch_model_hp_left_minus_right")))
    if score_name in signals:
        return safe_float(signals.get(score_name))
    if score_name in allocation:
        return safe_float(allocation.get(score_name))
    if score_name in tails:
        return safe_float(tails.get(score_name))
    return 0.0


def pair_target(row: dict[str, Any]) -> dict[str, float]:
    targets = row.get("targets") or {}
    hp_diff = safe_float(targets.get("hp_left_minus_right"))
    reward_diff = safe_float(targets.get("total_reward_left_minus_right"))
    return {
        "hp_left_minus_right": hp_diff,
        "total_reward_left_minus_right": reward_diff,
        "abs_hp_diff": abs(hp_diff),
    }


def action_summary(candidate: dict[str, Any]) -> dict[str, Any]:
    return {
        "action_kind": candidate.get("action_kind"),
        "action_type": candidate.get("action_type"),
        "action_key": candidate.get("action_key"),
        "card_id": candidate.get("card_id"),
        "card_tags": candidate_tags(candidate),
        "primary_tag": primary_tag(candidate),
    }


def request_from_pair(
    row: dict[str, Any],
    *,
    family: str,
    family_rank: int,
    score_name: str,
    score: float,
    family_mode: str,
    include_audit_targets: bool,
) -> dict[str, Any]:
    left = row.get("left") or {}
    right = row.get("right") or {}
    left_candidate = left.get("candidate") or {}
    right_candidate = right.get("candidate") or {}
    out = {
        "schema_version": "family_evidence_request_v0",
        "trainable_role": "contrast_family_evidence_request",
        "trainable_as_action_label": False,
        "episode_seed": row.get("episode_seed"),
        "episode_step": row.get("episode_step"),
        "decision_id": row.get("decision_id"),
        "family_allocator": {
            "schema_version": "contrast_family_allocator_v0",
            "family_mode": family_mode,
            "family": family,
            "family_rank": family_rank,
            "score_name": score_name,
            "score": score,
            "budget_item_role": "family_representative_pair",
        },
        "priority_score": score,
        "reasons": [
            "contrast_family_evidence_request",
            f"family_mode:{family_mode}",
            f"family:{family}",
            f"score:{score_name}",
        ],
        "pair_kind": f"{left_candidate.get('action_kind')}->{right_candidate.get('action_kind')}",
        "pair_card": f"{left_candidate.get('card_id')}->{right_candidate.get('card_id')}",
        "left": {
            "branch_id": left.get("branch_id"),
            "candidate": action_summary(left_candidate),
        },
        "right": {
            "branch_id": right.get("branch_id"),
            "candidate": action_summary(right_candidate),
        },
        "label_policy": {
            "action_label": False,
            "source": "contrast_family_allocator_v0",
        },
    }
    if include_audit_targets:
        out["audit_targets"] = pair_target(row)
    return out


def select_family_representatives(
    rows: list[dict[str, Any]],
    *,
    family_mode: str,
    score_name: str,
    budget: int,
    include_audit_targets: bool,
) -> list[dict[str, Any]]:
    best_by_family: dict[str, tuple[float, int]] = {}
    for index, row in enumerate(rows):
        family = contrast_family(row, mode=family_mode)
        score = score_value(row, score_name)
        previous = best_by_family.get(family)
        if previous is None or score > previous[0] or (score == previous[0] and index < previous[1]):
            best_by_family[family] = (score, index)
    ranked = sorted(best_by_family.items(), key=lambda item: (-item[1][0], item[1][1], item[0]))
    requests: list[dict[str, Any]] = []
    for family_rank, (family, (score, index)) in enumerate(ranked[:budget], start=1):
        requests.append(
            request_from_pair(
                rows[index],
                family=family,
                family_rank=family_rank,
                score_name=score_name,
                score=score,
                family_mode=family_mode,
                include_audit_targets=include_audit_targets,
            )
        )
    return requests


def assert_request_safety(rows: list[dict[str, Any]]) -> None:
    for index, row in enumerate(rows):
        if row.get("trainable_as_action_label") is not False:
            raise ValueError(f"request row {index} is action-label-like")
        if (row.get("label_policy") or {}).get("action_label") is not False:
            raise ValueError(f"request row {index} has action_label=true")
        serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
        for key in FORBIDDEN_LABEL_KEYS:
            if f'"{key}"' in serialized:
                raise ValueError(f"request row {index} contains forbidden key {key}")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


def build_requests(
    pair_rows: list[dict[str, Any]],
    *,
    family_mode: str,
    score_name: str,
    budget: int,
    include_audit_targets: bool,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in pair_rows:
        groups[decision_key(row)].append(row)
    requests: list[dict[str, Any]] = []
    family_counts: Counter[str] = Counter()
    pair_kind_counts: Counter[str] = Counter()
    request_counts_by_decision: Counter[int] = Counter()
    high_regret_family_hits = 0
    high_regret_family_total = 0
    high_regret_mass_hit = 0.0
    high_regret_mass_total = 0.0
    for group in groups.values():
        decision_requests = select_family_representatives(
            group,
            family_mode=family_mode,
            score_name=score_name,
            budget=budget,
            include_audit_targets=include_audit_targets,
        )
        requests.extend(decision_requests)
        request_counts_by_decision[len(decision_requests)] += 1
        requested_families = {
            ((row.get("family_allocator") or {}).get("family")) for row in decision_requests
        }
        target_families: dict[str, float] = defaultdict(float)
        for row in group:
            target = pair_target(row)
            if target["abs_hp_diff"] >= 10.0:
                target_families[contrast_family(row, mode=family_mode)] += target["abs_hp_diff"]
        high_regret_family_total += len(target_families)
        for family, mass in target_families.items():
            high_regret_mass_total += mass
            if family in requested_families:
                high_regret_family_hits += 1
                high_regret_mass_hit += mass
    for request in requests:
        allocator = request.get("family_allocator") or {}
        family_counts[str(allocator.get("family"))] += 1
        pair_kind_counts[str(request.get("pair_kind"))] += 1
    summary = {
        "schema_version": "family_evidence_request_build_summary_v0",
        "decision_count": len(groups),
        "request_count": len(requests),
        "budget": budget,
        "family_mode": family_mode,
        "score_name": score_name,
        "request_counts_by_decision": dict(request_counts_by_decision),
        "family_counts": dict(family_counts.most_common()),
        "pair_kind_counts": dict(pair_kind_counts.most_common()),
        "audit_high_regret_family_recall_abs10": (
            high_regret_family_hits / high_regret_family_total
            if high_regret_family_total
            else None
        ),
        "audit_high_regret_mass_recall_abs10": (
            high_regret_mass_hit / high_regret_mass_total
            if high_regret_mass_total
            else None
        ),
        "include_audit_targets": include_audit_targets,
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "requests_are_search_allocation_not_policy": True,
        },
    }
    return requests, summary


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pair-predictions", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--budget", type=int, default=3)
    parser.add_argument("--family-mode", default="primary_tag")
    parser.add_argument("--score-name", default="allocation_abs_hp_diff_ge_10_probability")
    parser.add_argument("--include-audit-targets", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    pair_rows = load_rows(args.pair_predictions)
    requests, summary = build_requests(
        pair_rows,
        family_mode=args.family_mode,
        score_name=args.score_name,
        budget=args.budget,
        include_audit_targets=args.include_audit_targets,
    )
    assert_request_safety(requests)
    write_jsonl(args.out, requests)
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
