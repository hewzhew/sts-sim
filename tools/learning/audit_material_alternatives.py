#!/usr/bin/env python3
"""Audit material counterfactual alternatives from evidence interpretations.

This is an offline audit tool. It consumes abstain-first evidence
interpretations plus the original evidence bundles and summarizes what kinds of
counterfactuals were surfaced. It does not choose actions, emit preferences, or
create action labels.
"""

from __future__ import annotations

import argparse
import json
import math
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable


FORBIDDEN_OUTPUT_KEYS = {
    "winner",
    "preferred",
    "preferred_action",
    "selected_action",
    "teacher_choice",
}

CARD_RE = re.compile(r"card:([^/]+)")


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


def safe_int(value: Any, default: int = 0) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def hp_gain_bucket(value: int) -> str:
    if value >= 20:
        return ">=20"
    if value >= 10:
        return "10..19"
    if value >= 5:
        return "5..9"
    if value >= 1:
        return "1..4"
    if value == 0:
        return "0"
    if value >= -4:
        return "-4..-1"
    if value >= -9:
        return "-9..-5"
    if value >= -19:
        return "-19..-10"
    return "<=-20"


def value_bucket(value: int | None, cuts: list[tuple[int, str]], default: str) -> str:
    if value is None:
        return "unknown"
    for limit, label in cuts:
        if value <= limit:
            return label
    return default


def parse_card_from_key(action_key: Any) -> str | None:
    if not isinstance(action_key, str):
        return None
    match = CARD_RE.search(action_key)
    return match.group(1) if match else None


def action_kind_from_key(action_key: Any) -> str | None:
    if not isinstance(action_key, str):
        return None
    if "combat/end_turn" in action_key:
        return "end_turn"
    if "combat/play_card" in action_key:
        return "play_card"
    return None


def candidate_catalog(bundle: dict[str, Any]) -> tuple[dict[int, dict[str, Any]], dict[str, dict[str, Any]]]:
    batch = bundle.get("branch_trace_batch") or {}
    traces = batch.get("traces") or []
    candidates = []
    for trace in traces:
        candidates = trace.get("candidates") or []
        if candidates:
            break
    by_id: dict[int, dict[str, Any]] = {}
    by_key: dict[str, dict[str, Any]] = {}
    for candidate in candidates:
        action_id = candidate.get("id")
        action_key = candidate.get("action_key")
        if isinstance(action_id, int):
            by_id[action_id] = candidate
        if isinstance(action_key, str):
            by_key[action_key] = candidate
    return by_id, by_key


def summarize_action(action: dict[str, Any], bundle: dict[str, Any]) -> dict[str, Any]:
    by_id, by_key = candidate_catalog(bundle)
    action_id = action.get("action_id")
    action_key = action.get("action_key")
    candidate: dict[str, Any] = {}
    if isinstance(action_id, int):
        candidate = by_id.get(action_id) or {}
    if not candidate and isinstance(action_key, str):
        candidate = by_key.get(action_key) or {}

    payload = candidate.get("payload") or {}
    card = payload.get("card") if isinstance(payload.get("card"), dict) else {}
    kind = candidate.get("action_kind") or action.get("action_kind") or action_kind_from_key(action_key)
    card_id = card.get("card_id") or action.get("card_id") or parse_card_from_key(action_key)
    if kind == "end_turn" and not card_id:
        card_id = "EndTurn"
    summary = {
        "action_id": action_id,
        "action_kind": kind or "unknown",
        "action_key": action_key,
        "card_id": card_id,
        "card": card,
    }
    summary["tags"] = card_tags(summary)
    summary["primary_tag"] = primary_tag(summary)
    return summary


def card_tags(action: dict[str, Any]) -> list[str]:
    kind = action.get("action_kind")
    if kind == "end_turn":
        return ["end_turn"]
    card = action.get("card") if isinstance(action.get("card"), dict) else {}
    if not card:
        return [kind or "unknown"]

    tags: list[str] = []
    if safe_int(card.get("base_damage")) > 0 or safe_int(card.get("upgraded_damage")) > 0:
        tags.append("damage")
    if safe_int(card.get("base_block")) > 0 or safe_int(card.get("upgraded_block")) > 0:
        tags.append("block")
    if card.get("applies_vulnerable"):
        tags.append("vulnerable")
    if card.get("applies_weak"):
        tags.append("weak")
    if card.get("draws_cards"):
        tags.append("draw")
    if card.get("exhaust"):
        tags.append("exhaust")
    if card.get("gains_energy"):
        tags.append("energy")
    if card.get("scaling_piece") or safe_int(card.get("card_type_id")) == 3:
        tags.append("setup")
    if not tags:
        tags.append("play_card_other")
    return tags


def primary_tag(action: dict[str, Any]) -> str:
    tags = action.get("tags") or []
    if "end_turn" in tags:
        return "end_turn"
    if "block" in tags:
        return "block"
    if "damage" in tags:
        return "damage"
    if "vulnerable" in tags or "weak" in tags:
        return "debuff"
    if "draw" in tags:
        return "draw"
    if "setup" in tags:
        return "setup"
    if "energy" in tags or "exhaust" in tags:
        return "resource_or_exhaust"
    return tags[0] if tags else "unknown"


def first_context(bundle: dict[str, Any]) -> dict[str, Any]:
    batch = bundle.get("branch_trace_batch") or {}
    traces = batch.get("traces") or []
    observation = {}
    if traces:
        observation = (traces[0].get("observation") or {}).get("payload") or {}
    combat = observation.get("combat") or {}
    return {
        "floor": observation.get("floor"),
        "act": observation.get("act"),
        "player_hp": combat.get("player_hp"),
        "player_block": combat.get("player_block"),
        "energy": combat.get("energy"),
        "incoming_damage": combat.get("visible_incoming_damage"),
        "alive_monster_count": combat.get("alive_monster_count"),
        "total_monster_hp": combat.get("total_monster_hp"),
        "hand_count": combat.get("hand_count"),
        "draw_count": combat.get("draw_count"),
        "discard_count": combat.get("discard_count"),
        "exhaust_count": combat.get("exhaust_count"),
    }


def top_dict(counter: Counter[Any], limit: int = 30) -> dict[str, int]:
    return {str(key): count for key, count in counter.most_common(limit)}


def assert_output_safe(row: dict[str, Any]) -> None:
    serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
    for key in FORBIDDEN_OUTPUT_KEYS:
        if f'"{key}"' in serialized:
            raise ValueError(f"output contains forbidden key {key}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--interpretations", type=Path, required=True)
    parser.add_argument("--evidence-bundles", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--examples-out", type=Path, required=True)
    parser.add_argument("--top-examples", type=int, default=50)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    bundles = {
        (bundle.get("episode_seed"), bundle.get("episode_step")): bundle
        for bundle in iter_jsonl(args.evidence_bundles)
    }

    material_rows: list[dict[str, Any]] = []
    missing_bundle_count = 0
    for row in iter_jsonl(args.interpretations):
        if row.get("interpretation_status") != "evidence_material_alternative_found":
            continue
        key = (row.get("episode_seed"), row.get("episode_step"))
        bundle = bundles.get(key)
        if bundle is None:
            missing_bundle_count += 1
            continue
        best = row.get("best_counterfactual_for_audit") or {}
        behavior = summarize_action(row.get("behavior_action") or {}, bundle)
        audit = summarize_action(best.get("audit_action") or {}, bundle)
        context = first_context(bundle)
        behavior_outcome = row.get("behavior_outcome") or {}
        audit_outcome = best.get("outcome") or {}
        hp_gain = safe_int(best.get("hp_gain_vs_behavior"))
        reward_gain = safe_float(best.get("reward_gain_vs_behavior"))
        combat_gain = safe_int(best.get("combat_win_count_gain_vs_behavior"))
        material_rows.append(
            {
                "schema_version": "material_alternative_audit_row_v0",
                "trainable_role": "offline_material_counterfactual_audit",
                "trainable_as_action_label": False,
                "episode_seed": row.get("episode_seed"),
                "episode_step": row.get("episode_step"),
                "context": context,
                "behavior_action": {
                    "action_kind": behavior.get("action_kind"),
                    "action_key": behavior.get("action_key"),
                    "card_id": behavior.get("card_id"),
                    "primary_tag": behavior.get("primary_tag"),
                    "tags": behavior.get("tags"),
                },
                "material_counterfactual": {
                    "action_kind": audit.get("action_kind"),
                    "action_key": audit.get("action_key"),
                    "card_id": audit.get("card_id"),
                    "primary_tag": audit.get("primary_tag"),
                    "tags": audit.get("tags"),
                    "material_reason": best.get("material_reason"),
                    "hp_gain_vs_behavior": hp_gain,
                    "reward_gain_vs_behavior": reward_gain,
                    "combat_win_count_gain_vs_behavior": combat_gain,
                },
                "behavior_outcome": behavior_outcome,
                "counterfactual_outcome": audit_outcome,
                "requested_families": [
                    (family.get("family_allocator") or family).get("family")
                    for family in bundle.get("requested_families") or []
                ],
                "strict_candidate_count": row.get("strict_candidate_count"),
                "label_policy": {
                    "action_label": False,
                    "source": "material_alternative_audit_v0",
                },
            }
        )

    material_count = len(material_rows)
    behavior_cards = Counter(row["behavior_action"].get("card_id") for row in material_rows)
    audit_cards = Counter(row["material_counterfactual"].get("card_id") for row in material_rows)
    behavior_tags = Counter(row["behavior_action"].get("primary_tag") for row in material_rows)
    audit_tags = Counter(row["material_counterfactual"].get("primary_tag") for row in material_rows)
    tag_pairs = Counter(
        (
            row["behavior_action"].get("primary_tag"),
            row["material_counterfactual"].get("primary_tag"),
        )
        for row in material_rows
    )
    card_pairs = Counter(
        (
            row["behavior_action"].get("card_id"),
            row["material_counterfactual"].get("card_id"),
        )
        for row in material_rows
    )
    hp_buckets = Counter(
        hp_gain_bucket(safe_int(row["material_counterfactual"].get("hp_gain_vs_behavior")))
        for row in material_rows
    )
    requested_families = Counter(
        family
        for row in material_rows
        for family in row.get("requested_families") or []
    )
    incoming_buckets = Counter(
        value_bucket(
            row["context"].get("incoming_damage"),
            [(0, "0"), (6, "1..6"), (12, "7..12"), (20, "13..20")],
            ">20",
        )
        for row in material_rows
    )
    hp_start_buckets = Counter(
        value_bucket(
            row["context"].get("player_hp"),
            [(20, "<=20"), (40, "21..40"), (60, "41..60")],
            ">60",
        )
        for row in material_rows
    )
    monster_hp_buckets = Counter(
        value_bucket(
            row["context"].get("total_monster_hp"),
            [(10, "<=10"), (25, "11..25"), (50, "26..50")],
            ">50",
        )
        for row in material_rows
    )
    alive_counts = Counter(row["context"].get("alive_monster_count") for row in material_rows)
    floor_counts = Counter(row["context"].get("floor") for row in material_rows)
    seed_counts = Counter(row["episode_seed"] for row in material_rows)
    combat_gain_counts = Counter(
        row["material_counterfactual"].get("combat_win_count_gain_vs_behavior")
        for row in material_rows
    )

    clusters: dict[tuple[Any, ...], list[dict[str, Any]]] = defaultdict(list)
    for row in material_rows:
        context = row["context"]
        key = (
            row["episode_seed"],
            context.get("floor"),
            context.get("player_hp"),
            context.get("incoming_damage"),
            row["behavior_outcome"].get("hp"),
            row["counterfactual_outcome"].get("hp"),
            row["behavior_action"].get("primary_tag"),
            row["material_counterfactual"].get("primary_tag"),
            row["material_counterfactual"].get("card_id"),
        )
        clusters[key].append(row)

    cluster_size_counts = Counter(len(rows) for rows in clusters.values())
    repeated_clusters = sorted(
        (
            {
                "size": len(rows),
                "episode_seed": key[0],
                "floor": key[1],
                "player_hp": key[2],
                "incoming_damage": key[3],
                "behavior_outcome_hp": key[4],
                "counterfactual_outcome_hp": key[5],
                "behavior_primary_tag": key[6],
                "counterfactual_primary_tag": key[7],
                "counterfactual_card_id": key[8],
                "episode_steps": [row["episode_step"] for row in rows],
            }
            for key, rows in clusters.items()
            if len(rows) > 1
        ),
        key=lambda item: (-item["size"], item["episode_seed"], item["floor"], item["episode_steps"]),
    )

    damage_to_block = [
        row
        for row in material_rows
        if row["behavior_action"].get("primary_tag") == "damage"
        and row["material_counterfactual"].get("primary_tag") == "block"
    ]
    def row_hp(row: dict[str, Any]) -> int:
        return safe_int(row["material_counterfactual"].get("hp_gain_vs_behavior"))

    top_examples = sorted(
        material_rows,
        key=lambda row: (
            -row_hp(row),
            int(row.get("episode_seed") or 0),
            int(row.get("episode_step") or 0),
        ),
    )[: args.top_examples]
    end_turn_examples = [
        row
        for row in material_rows
        if row["material_counterfactual"].get("primary_tag") == "end_turn"
    ]

    summary = {
        "schema_version": "material_alternative_audit_summary_v0",
        "interpretations": str(args.interpretations),
        "evidence_bundles": str(args.evidence_bundles),
        "material_row_count": material_count,
        "missing_bundle_count": missing_bundle_count,
        "coarse_cluster_count": len(clusters),
        "cluster_size_distribution": top_dict(cluster_size_counts, 20),
        "repeated_cluster_count": len(repeated_clusters),
        "behavior_primary_tag_counts": top_dict(behavior_tags),
        "counterfactual_primary_tag_counts": top_dict(audit_tags),
        "behavior_to_counterfactual_primary_tag_counts": top_dict(tag_pairs),
        "behavior_card_top": top_dict(behavior_cards),
        "counterfactual_card_top": top_dict(audit_cards),
        "behavior_to_counterfactual_card_top": top_dict(card_pairs, 40),
        "hp_gain_buckets": dict(hp_buckets),
        "combat_win_count_gain_counts": top_dict(combat_gain_counts),
        "requested_family_counts_top": top_dict(requested_families, 30),
        "context": {
            "incoming_damage_buckets": dict(incoming_buckets),
            "player_hp_buckets": dict(hp_start_buckets),
            "total_monster_hp_buckets": dict(monster_hp_buckets),
            "alive_monster_count": top_dict(alive_counts),
            "floor_counts_top": top_dict(floor_counts, 30),
            "seed_counts": top_dict(seed_counts, 30),
        },
        "damage_to_block": {
            "count": len(damage_to_block),
            "hp_gain_buckets": dict(Counter(hp_gain_bucket(row_hp(row)) for row in damage_to_block)),
            "incoming_damage_buckets": dict(
                Counter(
                    value_bucket(
                        row["context"].get("incoming_damage"),
                        [(0, "0"), (6, "1..6"), (12, "7..12"), (20, "13..20")],
                        ">20",
                    )
                    for row in damage_to_block
                )
            ),
            "mean_hp_gain": (
                sum(row_hp(row) for row in damage_to_block) / len(damage_to_block)
                if damage_to_block
                else 0.0
            ),
            "max_hp_gain": max((row_hp(row) for row in damage_to_block), default=0),
        },
        "large_hp_gain_count_ge_10": sum(1 for row in material_rows if row_hp(row) >= 10),
        "end_turn_counterfactual_count": len(end_turn_examples),
        "repeated_clusters_top": repeated_clusters[:30],
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "audit_is_offline_counterfactual_analysis_not_policy": True,
        },
    }
    assert_output_safe(summary)

    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")

    args.examples_out.parent.mkdir(parents=True, exist_ok=True)
    with args.examples_out.open("w", encoding="utf-8") as handle:
        for row in top_examples:
            assert_output_safe(row)
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")

    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
