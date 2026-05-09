#!/usr/bin/env python3
"""Run-level fact collection and audit.

This tool is deliberately not a policy or card scorer. It builds structured
run records so later experiments can ask whether route/card/shop/campfire
changes improve run-level outcomes.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
from collections import Counter, defaultdict
from pathlib import Path
from statistics import mean, median
from typing import Any

from collect_branch_traces import DriverClient, default_driver_path


REPO_ROOT = Path(__file__).resolve().parents[2]
FORBIDDEN_CANDIDATE_SNAPSHOT_KEYS = {
    "score",
    "scores",
    "logit",
    "logits",
    "q_value",
    "value",
    "winner",
    "preferred",
    "preference",
    "selected",
    "selected_action",
    "teacher_choice",
}


def safe_int(value: Any, default: int = 0) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return default
    return number if math.isfinite(number) else default


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False), encoding="utf-8")


def action_family(action_key: str | None) -> str:
    key = str(action_key or "")
    if key.startswith("combat/play_card"):
        return "play_card"
    if key.startswith("combat/end_turn"):
        return "end_turn"
    if key.startswith("combat/use_potion"):
        return "use_potion"
    if key.startswith("reward/select_card"):
        return "select_card"
    if key.startswith("reward/claim"):
        return "claim_reward"
    if key.startswith("map/"):
        return "map"
    if key.startswith("event/"):
        return "event"
    if key.startswith("shop/buy_card"):
        return "buy_card"
    if key.startswith("shop/buy_relic"):
        return "buy_relic"
    if key.startswith("shop/buy_potion"):
        return "buy_potion"
    if key.startswith("shop/purge"):
        return "purge_card"
    if key.startswith("campfire/rest"):
        return "campfire_rest"
    if key.startswith("campfire/smith"):
        return "campfire_smith"
    if key.startswith("boss_relic") or key.startswith("relic/"):
        return "boss_relic"
    return key.split("/", 1)[0] if key else "unknown"


def card_from_action(action: dict[str, Any]) -> str | None:
    display = str(action.get("display") or "")
    key = str(action.get("key") or "")
    match = re.search(r"select_card:([^ ]+)", display)
    if match:
        return match.group(1)
    match = re.search(r"card:([^/]+)", key)
    if match:
        return match.group(1)
    return None


def contains_forbidden_key(value: Any, forbidden: set[str]) -> bool:
    if isinstance(value, dict):
        for key, child in value.items():
            if str(key).lower() in forbidden:
                return True
            if contains_forbidden_key(child, forbidden):
                return True
    elif isinstance(value, list):
        return any(contains_forbidden_key(child, forbidden) for child in value)
    return False


def slim_public_candidate_payload(payload: dict[str, Any]) -> dict[str, Any]:
    """Keep public identity/features, never policy scores or labels."""
    card = payload.get("card") if isinstance(payload.get("card"), dict) else None
    action = payload.get("action") if isinstance(payload.get("action"), dict) else None
    slim: dict[str, Any] = {
        "action": action or {},
        "card": None,
    }
    if card:
        slim["card"] = {
            "card_id": card.get("card_id"),
            "card_type_id": card.get("card_type_id"),
            "rarity_id": card.get("rarity_id"),
            "cost": card.get("cost"),
            "upgrades": card.get("upgrades"),
            "base_damage": card.get("base_damage"),
            "base_block": card.get("base_block"),
            "draws_cards": card.get("draws_cards"),
            "exhaust": card.get("exhaust"),
            "applies_vulnerable": card.get("applies_vulnerable"),
            "applies_weak": card.get("applies_weak"),
            "scaling_piece": card.get("scaling_piece"),
            "starter_basic": card.get("starter_basic"),
        }
    for key in ("shop_item", "relic", "potion", "reward_item", "screen_action"):
        if key in payload:
            slim[key] = payload.get(key)
    return slim


def candidate_display(candidate: dict[str, Any], index: int) -> str:
    key = str(candidate.get("action_key") or "")
    payload = candidate.get("payload") or {}
    card = payload.get("card") if isinstance(payload.get("card"), dict) else {}
    if card.get("card_id"):
        return f"{candidate.get('action_kind')}:{card.get('card_id')} ({key})"
    return f"{candidate.get('action_kind')} ({key})"


def build_candidate_snapshot(
    *,
    decision_type: str,
    step: int,
    observation_payload: dict[str, Any] | None = None,
    candidates: list[dict[str, Any]],
) -> dict[str, Any]:
    slim_candidates: list[dict[str, Any]] = []
    forbidden_found = False
    for index, candidate in enumerate(candidates):
        payload = candidate.get("payload") if isinstance(candidate.get("payload"), dict) else {}
        row = {
            "id": candidate.get("id", candidate.get("action_index", index)),
            "action_index": candidate.get("action_index", index),
            "action_key": candidate.get("action_key"),
            "action_kind": candidate.get("action_kind"),
            "display": candidate_display(candidate, index),
            "public_payload_summary": slim_public_candidate_payload(payload),
            "trainable_as_action_label": False,
        }
        forbidden_found = forbidden_found or contains_forbidden_key(row, FORBIDDEN_CANDIDATE_SNAPSHOT_KEYS)
        slim_candidates.append(row)
    context = summarize_observation_payload(observation_payload or {})
    fingerprint_payload = {
        "decision_type": decision_type,
        "act": context.get("act"),
        "floor": context.get("floor"),
        "hp": context.get("hp"),
        "max_hp": context.get("max_hp"),
        "gold": context.get("gold"),
        "deck_size": context.get("deck_size"),
        "candidate_keys": [row.get("action_key") for row in slim_candidates],
    }
    fingerprint = hashlib.sha256(
        json.dumps(fingerprint_payload, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()[:24]
    return {
        "schema_version": "noncombat_candidate_snapshot_v1",
        "step": step,
        "decision_type": decision_type,
        "decision_fingerprint": fingerprint,
        "public_context_summary": context,
        "candidate_count": len(slim_candidates),
        "candidates": slim_candidates,
        "trainable_as_action_label": False,
        "contains_policy_scores": False,
        "contains_winner_or_preference": forbidden_found,
    }


def summarize_observation_payload(payload: dict[str, Any]) -> dict[str, Any]:
    deck = payload.get("deck") or {}
    combat = payload.get("combat") or {}
    relics = payload.get("relics") or []
    potions = payload.get("potions") or []
    deck_cards = payload.get("deck_cards") or []
    screen = payload.get("screen") or {}
    upgraded_count = 0
    for item in deck_cards:
        if not isinstance(item, dict):
            continue
        card = item.get("card") or {}
        if isinstance(card, dict) and safe_int(card.get("upgrades")) > 0:
            upgraded_count += 1
    return {
        "act": safe_int(payload.get("act")),
        "floor": safe_int(payload.get("floor")),
        "hp": safe_int(payload.get("current_hp")),
        "max_hp": safe_int(payload.get("max_hp")),
        "gold": safe_int(payload.get("gold")),
        "deck_size": safe_int(payload.get("deck_size") or len(deck_cards)),
        "relic_count": safe_int(payload.get("relic_count") or len(relics)),
        "potion_count": len([potion for potion in potions if potion]),
        "boss": payload.get("act_boss"),
        "room": payload.get("current_room"),
        "map": payload.get("map") or {},
        "next_nodes": payload.get("next_nodes") or [],
        "deck": {
            "attack_count": safe_int(deck.get("attack_count")),
            "skill_count": safe_int(deck.get("skill_count")),
            "power_count": safe_int(deck.get("power_count")),
            "damage_card_count": safe_int(deck.get("damage_card_count")),
            "block_card_count": safe_int(deck.get("block_card_count")),
            "draw_card_count": safe_int(deck.get("draw_card_count")),
            "exhaust_card_count": safe_int(deck.get("exhaust_card_count")),
            "scaling_card_count": safe_int(deck.get("scaling_card_count")),
            "starter_basic_count": safe_int(deck.get("starter_basic_count")),
            "curse_count": safe_int(deck.get("curse_count")),
            "status_count": safe_int(deck.get("status_count")),
            "upgraded_count": upgraded_count,
            "average_cost_milli": safe_int(deck.get("average_cost_milli")),
        },
        "combat": {
            "turn": safe_int(combat.get("turn_count")),
            "incoming": safe_int(combat.get("visible_incoming_damage")),
            "player_block": safe_int(combat.get("player_block")),
            "monster_count": len(combat.get("monsters") or combat.get("enemies") or []),
        }
        if isinstance(combat, dict) and combat
        else None,
        "screen": {
            "event_option_count": safe_int(screen.get("event_option_count")),
            "reward_card_choice_count": safe_int(screen.get("reward_card_choice_count")),
            "reward_claimable_item_count": safe_int(screen.get("reward_claimable_item_count")),
            "reward_item_count": safe_int(screen.get("reward_item_count")),
            "selection_target_count": safe_int(screen.get("selection_target_count")),
            "shop_card_count": safe_int(screen.get("shop_card_count")),
            "shop_relic_count": safe_int(screen.get("shop_relic_count")),
            "shop_potion_count": safe_int(screen.get("shop_potion_count")),
            "boss_relic_choice_count": safe_int(screen.get("boss_relic_choice_count")),
        },
        "deck_card_ids": [
            ((item.get("card") or {}).get("card_id"))
            for item in deck_cards
            if isinstance(item, dict)
        ],
        "relic_ids": [
            relic.get("relic_id") if isinstance(relic, dict) else relic for relic in relics
        ],
        "potion_ids": [
            potion.get("potion_id") if isinstance(potion, dict) else potion
            for potion in potions
            if potion
        ],
    }


def slim_before_from_readable(before: dict[str, Any]) -> dict[str, Any]:
    return {
        "act": safe_int(before.get("act")),
        "floor": safe_int(before.get("floor")),
        "hp": safe_int(before.get("hp")),
        "max_hp": safe_int(before.get("max_hp")),
        "gold": safe_int(before.get("gold")),
        "deck_size": safe_int(before.get("deck_size")),
        "relic_count": safe_int(before.get("relic_count")),
        "potion_count": safe_int(before.get("potion_count")),
        "boss": before.get("boss"),
        "room": before.get("room"),
        "map": before.get("map") or {},
        "next_nodes": before.get("next_nodes") or [],
        "screen": before.get("screen") or {},
        "deck": before.get("deck") or {},
        "deck_card_ids": before.get("deck_card_ids") or [],
        "relic_ids": before.get("relic_ids") or [],
        "potion_ids": before.get("potion_ids") or [],
    }


def normalize_step(row: dict[str, Any]) -> dict[str, Any]:
    before = row.get("before") or {}
    if "observation" in row:
        payload = ((row.get("observation") or {}).get("payload")) or row.get("observation") or {}
        before = summarize_observation_payload(payload)
    else:
        before = slim_before_from_readable(before)
    action = row.get("action") or {}
    after = row.get("after") or row.get("info") or {}
    return {
        "step": safe_int(row.get("step") or row.get("episode_step")),
        "decision_type": row.get("decision_type") or before.get("decision_type") or "unknown",
        "before": before,
        "combat": row.get("combat") or before.get("combat"),
        "action": action,
        "action_family": action_family(action.get("key")),
        "action_card": card_from_action(action),
        "candidate_snapshot": row.get("candidate_snapshot"),
        "after": after,
        "reward": safe_float(row.get("reward")),
        "done_after": bool(row.get("done_after")),
    }


def infer_seed(path: Path, steps: list[dict[str, Any]]) -> int:
    for row in reversed(steps):
        after = row.get("after") or {}
        if after.get("seed") is not None:
            return safe_int(after.get("seed"))
    match = re.search(r"seed(\d+)", path.name)
    return safe_int(match.group(1)) if match else 0


def first_step_matching(steps: list[dict[str, Any]], predicate) -> dict[str, Any] | None:
    for row in steps:
        if predicate(row):
            return row
    return None


def compact_state(state: dict[str, Any] | None) -> dict[str, Any]:
    state = state or {}
    return {
        "act": safe_int(state.get("act")),
        "floor": safe_int(state.get("floor")),
        "hp": safe_int(state.get("hp")),
        "max_hp": safe_int(state.get("max_hp")),
        "gold": safe_int(state.get("gold")),
        "deck_size": safe_int(state.get("deck_size")),
        "relic_count": safe_int(state.get("relic_count")),
        "potion_count": safe_int(state.get("potion_count")),
        "room": state.get("room"),
        "deck": state.get("deck") or {},
        "deck_card_ids": state.get("deck_card_ids") or [],
        "relic_ids": state.get("relic_ids") or [],
        "potion_ids": state.get("potion_ids") or [],
    }


def compact_after(after: dict[str, Any] | None) -> dict[str, Any]:
    after = after or {}
    return {
        "act": safe_int(after.get("act")),
        "floor": safe_int(after.get("floor")),
        "hp": safe_int(after.get("hp")),
        "max_hp": safe_int(after.get("max_hp")),
        "gold": safe_int(after.get("gold")),
        "deck_size": safe_int(after.get("deck_size")),
        "relic_count": safe_int(after.get("relic_count")),
        "combat_win_count": safe_int(after.get("combat_win_count")),
        "result": after.get("result"),
        "terminal_reason": after.get("terminal_reason"),
    }


def shop_offer_counts(row: dict[str, Any]) -> dict[str, int]:
    screen = ((row.get("before") or {}).get("screen")) or {}
    return {
        "cards": safe_int(screen.get("shop_card_count")),
        "relics": safe_int(screen.get("shop_relic_count")),
        "potions": safe_int(screen.get("shop_potion_count")),
    }


SHOP_ACTION_FAMILIES = {"buy_card", "buy_relic", "buy_potion", "purge_card"}
CAMPFIRE_ACTION_FAMILIES = {"campfire_rest", "campfire_smith"}


def is_shop_row(row: dict[str, Any]) -> bool:
    before = row.get("before") or {}
    return (
        row.get("decision_type") == "shop"
        or before.get("room") == "ShopRoom"
        or row.get("action_family") in SHOP_ACTION_FAMILIES
    )


def is_campfire_row(row: dict[str, Any]) -> bool:
    before = row.get("before") or {}
    return (
        row.get("decision_type") == "campfire"
        or before.get("room") == "RestRoom"
        or row.get("action_family") in CAMPFIRE_ACTION_FAMILIES
    )


def group_consecutive_rows(steps: list[dict[str, Any]], predicate) -> list[list[dict[str, Any]]]:
    groups: list[list[dict[str, Any]]] = []
    current: list[dict[str, Any]] = []
    for row in steps:
        if predicate(row):
            current.append(row)
        elif current:
            groups.append(current)
            current = []
    if current:
        groups.append(current)
    return groups


def summarize_shop_visits(steps: list[dict[str, Any]]) -> list[dict[str, Any]]:
    visits: list[dict[str, Any]] = []
    for group in group_consecutive_rows(steps, is_shop_row):
        entry_before = group[0].get("before") or {}
        exit_after = group[-1].get("after") or {}
        families = Counter(row.get("action_family") or "unknown" for row in group)
        purchase_count = sum(families[family] for family in SHOP_ACTION_FAMILIES)
        entry_gold = safe_int(entry_before.get("gold"))
        exit_gold = safe_int(exit_after.get("gold"), entry_gold)
        actions = [
            {
                "step": row.get("step"),
                "family": row.get("action_family"),
                "key": (row.get("action") or {}).get("key"),
                "display": (row.get("action") or {}).get("display"),
                "gold_before": safe_int((row.get("before") or {}).get("gold")),
                "gold_after": safe_int((row.get("after") or {}).get("gold")),
            }
            for row in group
            if row.get("action_family") in SHOP_ACTION_FAMILIES
        ]
        visits.append(
            {
                "entry_step": group[0].get("step"),
                "exit_step": group[-1].get("step"),
                "entry": compact_state(entry_before),
                "exit": compact_after(exit_after),
                "offer_counts": shop_offer_counts(group[0]),
                "action_family_counts": dict(families),
                "purchase_count": purchase_count,
                "buy_card_count": families["buy_card"],
                "buy_relic_count": families["buy_relic"],
                "buy_potion_count": families["buy_potion"],
                "purge_card_count": families["purge_card"],
                "spent_gold": max(0, entry_gold - exit_gold),
                "actions": actions,
            }
        )
    return visits


def summarize_campfire_visits(steps: list[dict[str, Any]]) -> list[dict[str, Any]]:
    visits: list[dict[str, Any]] = []
    for group in group_consecutive_rows(steps, is_campfire_row):
        entry_before = group[0].get("before") or {}
        exit_after = group[-1].get("after") or {}
        families = Counter(row.get("action_family") or "unknown" for row in group)
        actions = [
            {
                "step": row.get("step"),
                "family": row.get("action_family"),
                "key": (row.get("action") or {}).get("key"),
                "display": (row.get("action") or {}).get("display"),
                "hp_before": safe_int((row.get("before") or {}).get("hp")),
                "hp_after": safe_int((row.get("after") or {}).get("hp")),
            }
            for row in group
            if row.get("action_family") in CAMPFIRE_ACTION_FAMILIES
        ]
        visits.append(
            {
                "entry_step": group[0].get("step"),
                "exit_step": group[-1].get("step"),
                "entry": compact_state(entry_before),
                "exit": compact_after(exit_after),
                "action_family_counts": dict(families),
                "rest_count": families["campfire_rest"],
                "smith_count": families["campfire_smith"],
                "actions": actions,
            }
        )
    return visits


def visited_room_counts(steps: list[dict[str, Any]]) -> dict[str, int]:
    by_floor: dict[int, str] = {}
    for row in steps:
        before = row.get("before") or {}
        floor = safe_int(before.get("floor"), -1)
        room = before.get("room")
        if floor >= 0 and room and floor not in by_floor:
            by_floor[floor] = str(room)
    return dict(Counter(by_floor.values()))


def parse_map_action_x(action_key: str | None) -> int | None:
    match = re.search(r"map/select_x/(-?\d+)", str(action_key or ""))
    return safe_int(match.group(1)) if match else None


def node_coord(node: dict[str, Any]) -> tuple[int, int]:
    return (safe_int(node.get("x"), -999), safe_int(node.get("y"), -999))


def room_bucket(room_type: str | None) -> str:
    room = str(room_type or "unknown")
    if room == "ShopRoom":
        return "shop"
    if room == "RestRoom":
        return "campfire"
    if room == "MonsterRoomElite":
        return "elite"
    if room == "MonsterRoom":
        return "monster"
    if room == "EventRoom":
        return "event"
    if room == "TreasureRoom":
        return "treasure"
    if room == "MonsterRoomBoss":
        return "boss"
    return room


def node_estimated_floor(node: dict[str, Any]) -> int:
    y = safe_int(node.get("y"), -1)
    return y + 1 if y >= 0 else 0


def reachable_node_summary(
    start_node: dict[str, Any], node_by_coord: dict[tuple[int, int], dict[str, Any]]
) -> dict[str, Any]:
    stack = [node_coord(start_node)]
    seen: set[tuple[int, int]] = set()
    counts: Counter[str] = Counter()
    earliest: dict[str, int] = {}
    while stack:
        coord = stack.pop()
        if coord in seen:
            continue
        seen.add(coord)
        node = node_by_coord.get(coord)
        if not node:
            continue
        bucket = room_bucket(node.get("room_type"))
        counts[bucket] += 1
        floor = node_estimated_floor(node)
        if bucket not in earliest or floor < earliest[bucket]:
            earliest[bucket] = floor
        for edge in node.get("edges") or []:
            if isinstance(edge, dict):
                stack.append((safe_int(edge.get("dst_x"), -999), safe_int(edge.get("dst_y"), -999)))
    return {
        "node_count": len(seen),
        "room_counts": dict(counts),
        "earliest_shop_floor": earliest.get("shop"),
        "earliest_campfire_floor": earliest.get("campfire"),
        "earliest_elite_floor": earliest.get("elite"),
        "earliest_boss_floor": earliest.get("boss"),
        "has_shop": counts["shop"] > 0,
        "has_campfire": counts["campfire"] > 0,
        "has_elite": counts["elite"] > 0,
    }


def summarize_map_opportunities(steps: list[dict[str, Any]]) -> list[dict[str, Any]]:
    decisions: list[dict[str, Any]] = []
    for row in steps:
        if row.get("decision_type") != "map" and row.get("action_family") != "map":
            continue
        before = row.get("before") or {}
        map_payload = before.get("map") or {}
        nodes = [node for node in map_payload.get("nodes") or [] if isinstance(node, dict)]
        if not nodes:
            continue
        node_by_coord = {node_coord(node): node for node in nodes}
        next_nodes = [
            node for node in before.get("next_nodes") or [] if isinstance(node, dict)
        ]
        if not next_nodes:
            next_nodes = [node for node in nodes if node.get("reachable_now")]
        chosen_x = parse_map_action_x((row.get("action") or {}).get("key"))
        options: list[dict[str, Any]] = []
        for node in next_nodes:
            coord = node_coord(node)
            full_node = node_by_coord.get(coord, node)
            reach = reachable_node_summary(full_node, node_by_coord)
            estimated_floor = node_estimated_floor(full_node)
            option = {
                "x": coord[0],
                "y": coord[1],
                "estimated_floor": estimated_floor,
                "immediate_room": full_node.get("room_type"),
                "immediate_bucket": room_bucket(full_node.get("room_type")),
                "chosen": chosen_x is not None and coord[0] == chosen_x,
                "reachable": reach,
                "distance_to_shop": (
                    safe_int(reach.get("earliest_shop_floor")) - safe_int(before.get("floor"))
                    if reach.get("earliest_shop_floor") is not None
                    else None
                ),
                "distance_to_campfire": (
                    safe_int(reach.get("earliest_campfire_floor")) - safe_int(before.get("floor"))
                    if reach.get("earliest_campfire_floor") is not None
                    else None
                ),
            }
            options.append(option)
        chosen_options = [option for option in options if option.get("chosen")]
        chosen_option = chosen_options[0] if chosen_options else None
        decisions.append(
            {
                "step": row.get("step"),
                "act": safe_int(before.get("act")),
                "floor": safe_int(before.get("floor")),
                "hp": safe_int(before.get("hp")),
                "max_hp": safe_int(before.get("max_hp")),
                "gold": safe_int(before.get("gold")),
                "boss": before.get("boss"),
                "current_x": safe_int(map_payload.get("current_x"), -1),
                "current_y": safe_int(map_payload.get("current_y"), -1),
                "chosen_x": chosen_x,
                "option_count": len(options),
                "any_shop_reachable": any(
                    ((option.get("reachable") or {}).get("has_shop")) for option in options
                ),
                "chosen_shop_reachable": bool(
                    chosen_option and ((chosen_option.get("reachable") or {}).get("has_shop"))
                ),
                "any_campfire_reachable": any(
                    ((option.get("reachable") or {}).get("has_campfire")) for option in options
                ),
                "chosen_campfire_reachable": bool(
                    chosen_option and ((chosen_option.get("reachable") or {}).get("has_campfire"))
                ),
                "any_elite_reachable": any(
                    ((option.get("reachable") or {}).get("has_elite")) for option in options
                ),
                "chosen_elite_reachable": bool(
                    chosen_option and ((chosen_option.get("reachable") or {}).get("has_elite"))
                ),
                "options": options,
                "chosen_option": chosen_option,
            }
        )
    return decisions


NONCOMBAT_DECISION_TYPES = {
    "map",
    "shop",
    "campfire",
    "reward",
    "event",
    "boss_relic",
    "grid",
    "selection",
}


def is_noncombat_decision_type(decision_type: str | None) -> bool:
    value = str(decision_type or "unknown").lower()
    return not value.startswith("combat") and value in NONCOMBAT_DECISION_TYPES


def summarize_candidate_snapshots(steps: list[dict[str, Any]]) -> dict[str, Any]:
    snapshots: list[dict[str, Any]] = []
    missing_noncombat = 0
    candidate_counts: Counter[str] = Counter()
    forbidden_snapshot_count = 0
    for row in steps:
        decision_type = str(row.get("decision_type") or "unknown")
        if not is_noncombat_decision_type(decision_type):
            continue
        snapshot = row.get("candidate_snapshot")
        if not isinstance(snapshot, dict):
            missing_noncombat += 1
            continue
        if snapshot.get("contains_winner_or_preference") or contains_forbidden_key(
            snapshot, FORBIDDEN_CANDIDATE_SNAPSHOT_KEYS
        ):
            forbidden_snapshot_count += 1
        candidate_counts[decision_type] += safe_int(snapshot.get("candidate_count"))
        snapshots.append(
            {
                "schema_version": snapshot.get("schema_version", "noncombat_candidate_snapshot_v1"),
                "step": row.get("step"),
                "decision_type": decision_type,
                "decision_fingerprint": snapshot.get("decision_fingerprint"),
                "public_context_summary": snapshot.get("public_context_summary") or {},
                "before": compact_state(row.get("before") or {}),
                "action_taken": row.get("action") or {},
                "candidate_count": safe_int(snapshot.get("candidate_count")),
                "candidates": snapshot.get("candidates") or [],
                "trainable_as_action_label": False,
                "contains_policy_scores": bool(snapshot.get("contains_policy_scores")),
                "contains_winner_or_preference": bool(snapshot.get("contains_winner_or_preference")),
            }
        )
    return {
        "schema_version": "run_candidate_snapshot_summary_v1",
        "snapshot_count": len(snapshots),
        "missing_noncombat_snapshot_count": missing_noncombat,
        "forbidden_snapshot_count": forbidden_snapshot_count,
        "candidate_count_by_decision_type": dict(candidate_counts),
        "snapshots": snapshots,
    }


def classify_high_gold_opportunity(
    steps: list[dict[str, Any]],
    shop_visits: list[dict[str, Any]],
    map_opportunities: list[dict[str, Any]],
    *,
    final_result: str,
    final_gold: int,
    threshold: int = 250,
) -> dict[str, Any]:
    if final_result != "defeat" or final_gold < threshold:
        return {
            "threshold": threshold,
            "classification": "not_high_gold_death",
            "actionable": False,
        }
    first_high_step: int | None = None
    first_high_floor: int | None = None
    first_high_gold: int | None = None
    for row in steps:
        before_gold = safe_int((row.get("before") or {}).get("gold"))
        after_gold = safe_int((row.get("after") or {}).get("gold"))
        if max(before_gold, after_gold) >= threshold:
            first_high_step = safe_int(row.get("step"))
            first_high_floor = safe_int((row.get("before") or {}).get("floor"))
            first_high_gold = max(before_gold, after_gold)
            break
    if first_high_step is None:
        return {
            "threshold": threshold,
            "classification": "high_final_gold_without_observed_threshold_crossing",
            "actionable": False,
        }

    maps_after_high = [
        entry for entry in map_opportunities if safe_int(entry.get("step")) >= first_high_step
    ]
    shop_options_after_high = [
        entry for entry in maps_after_high if entry.get("any_shop_reachable")
    ]
    missed_shop_path = [
        entry
        for entry in shop_options_after_high
        if entry.get("any_shop_reachable") and not entry.get("chosen_shop_reachable")
    ]
    high_shop_visits = [
        visit
        for visit in shop_visits
        if safe_int(visit.get("entry_step")) >= first_high_step
        or safe_int((visit.get("entry") or {}).get("gold")) >= threshold
    ]
    last_shop = shop_visits[-1] if shop_visits else None
    last_shop_exit_gold = safe_int(((last_shop or {}).get("exit") or {}).get("gold")) if last_shop else None
    gold_after_last_shop = final_gold - last_shop_exit_gold if last_shop_exit_gold is not None else None

    classification = "unclassified_high_gold_death"
    actionable = False
    if not maps_after_high:
        classification = "late_gold_no_map_decision"
    elif not shop_options_after_high:
        if last_shop and gold_after_last_shop is not None and gold_after_last_shop >= threshold:
            classification = "spent_then_late_gold_no_later_shop_chance"
        else:
            classification = "late_gold_no_shop_chance"
    elif missed_shop_path:
        classification = "missed_reachable_shop_after_high_gold"
        actionable = True
    elif high_shop_visits:
        purchase_count = sum(safe_int(visit.get("purchase_count")) for visit in high_shop_visits)
        if purchase_count == 0:
            classification = "high_gold_shop_visit_no_purchase"
            actionable = True
        elif gold_after_last_shop is not None and gold_after_last_shop >= threshold:
            classification = "spent_then_late_gold"
        else:
            classification = "spent_but_still_high_gold"
            actionable = True
    elif shop_options_after_high:
        classification = "shop_path_chosen_but_not_reached_before_death"
    elif last_shop and gold_after_last_shop is not None and gold_after_last_shop >= threshold:
        classification = "spent_then_late_gold"

    return {
        "threshold": threshold,
        "classification": classification,
        "actionable": actionable,
        "first_high_gold_step": first_high_step,
        "first_high_gold_floor": first_high_floor,
        "first_high_gold": first_high_gold,
        "map_decisions_after_high_gold": len(maps_after_high),
        "shop_option_decisions_after_high_gold": len(shop_options_after_high),
        "missed_shop_path_count": len(missed_shop_path),
        "shop_visits_after_high_gold": len(high_shop_visits),
        "last_shop_exit_gold": last_shop_exit_gold,
        "gold_gained_after_last_shop": gold_after_last_shop,
        "evidence_steps": {
            "missed_shop_path_steps": [entry.get("step") for entry in missed_shop_path[:5]],
            "shop_option_steps": [entry.get("step") for entry in shop_options_after_high[:5]],
        },
    }


def summarize_run(path: Path, rows: list[dict[str, Any]]) -> dict[str, Any]:
    steps = [normalize_step(row) for row in rows]
    if not steps:
        return {"source_path": str(path), "empty": True}
    seed = infer_seed(path, steps)
    final = steps[-1].get("after") or {}
    first_before = steps[0].get("before") or {}
    boss = first_before.get("boss")
    for row in steps:
        boss = boss or (row.get("before") or {}).get("boss")

    action_family_counts = Counter(row["action_family"] for row in steps)
    decision_type_counts = Counter(str(row.get("decision_type") or "unknown") for row in steps)
    picked_cards = [row["action_card"] for row in steps if row["action_family"] == "select_card" and row["action_card"]]
    played_cards = [row["action_card"] for row in steps if row["action_family"] == "play_card" and row["action_card"]]
    shop_visits = summarize_shop_visits(steps)
    campfire_visits = summarize_campfire_visits(steps)
    shop_visit_count = len(shop_visits)
    shop_purchase_count = sum(safe_int(visit.get("purchase_count")) for visit in shop_visits)
    shop_gold_spent = sum(safe_int(visit.get("spent_gold")) for visit in shop_visits)
    campfire_rest_count = sum(safe_int(visit.get("rest_count")) for visit in campfire_visits)
    campfire_smith_count = sum(safe_int(visit.get("smith_count")) for visit in campfire_visits)
    map_opportunities = summarize_map_opportunities(steps)
    candidate_snapshot_summary = summarize_candidate_snapshots(steps)

    act1_boss_entry = first_step_matching(
        steps,
        lambda row: (row.get("before") or {}).get("act") == 1
        and (row.get("before") or {}).get("floor") == 16,
    )
    entry_after_win_count = None
    if act1_boss_entry:
        entry_after_win_count = safe_int((act1_boss_entry.get("after") or {}).get("combat_win_count"))
    act1_boss_exit = None
    if act1_boss_entry:
        for row in steps:
            before = row.get("before") or {}
            after = row.get("after") or {}
            if before.get("act") == 1 and before.get("floor") == 16:
                if after.get("result") == "defeat":
                    act1_boss_exit = row
                    break
                if entry_after_win_count is not None and safe_int(after.get("combat_win_count")) > entry_after_win_count:
                    act1_boss_exit = row
                    break
    act2_entry = first_step_matching(
        steps,
        lambda row: safe_int((row.get("after") or {}).get("act")) >= 2
        or (row.get("before") or {}).get("act") >= 2,
    )

    final_result = str(final.get("result") or "unknown")
    final_floor = safe_int(final.get("floor"))
    final_act = safe_int(final.get("act"))
    final_hp = safe_int(final.get("hp"))
    final_gold = safe_int(final.get("gold"))
    final_deck_size = safe_int(final.get("deck_size"))
    final_relic_count = safe_int(final.get("relic_count"))
    bottleneck = "ongoing_or_victory"
    if final_result == "defeat":
        if final_act <= 1 and final_floor < 16:
            bottleneck = "act1_pre_boss_death"
        elif final_act == 1 and final_floor == 16:
            bottleneck = "act1_boss_death"
        elif final_act == 2 and final_floor <= 20:
            bottleneck = "act2_entry_death"
        else:
            bottleneck = "later_death"
    high_gold_opportunity = classify_high_gold_opportunity(
        steps,
        shop_visits,
        map_opportunities,
        final_result=final_result,
        final_gold=final_gold,
    )

    def snap(row: dict[str, Any] | None, source: str) -> dict[str, Any] | None:
        if row is None:
            return None
        before = row.get("before") or {}
        after = row.get("after") or {}
        return {
            "source": source,
            "step": row.get("step"),
            "act": safe_int(before.get("act") or after.get("act")),
            "floor": safe_int(before.get("floor") or after.get("floor")),
            "hp": safe_int(before.get("hp") or after.get("hp")),
            "max_hp": safe_int(before.get("max_hp") or after.get("max_hp")),
            "gold": safe_int(before.get("gold") or after.get("gold")),
            "deck_size": safe_int(before.get("deck_size") or after.get("deck_size")),
            "relic_count": safe_int(before.get("relic_count") or after.get("relic_count")),
            "potion_count": safe_int(before.get("potion_count")),
            "deck": before.get("deck") or {},
            "deck_card_ids": before.get("deck_card_ids") or [],
            "relic_ids": before.get("relic_ids") or [],
            "potion_ids": before.get("potion_ids") or [],
            "result_after": after.get("result"),
            "hp_after": safe_int(after.get("hp")),
            "gold_after": safe_int(after.get("gold")),
            "deck_size_after": safe_int(after.get("deck_size")),
            "relic_count_after": safe_int(after.get("relic_count")),
        }

    audit_flags: list[str] = []
    boss_entry_snap = snap(act1_boss_entry, "act1_boss_entry")
    boss_exit_snap = snap(act1_boss_exit, "act1_boss_exit")
    act2_entry_snap = snap(act2_entry, "act2_entry")
    if boss_entry_snap and boss_entry_snap["hp"] <= 35:
        audit_flags.append("low_act1_boss_entry_hp")
    if boss_exit_snap and boss_exit_snap["hp_after"] <= 15 and boss_exit_snap["result_after"] != "defeat":
        audit_flags.append("low_act1_boss_exit_hp")
    if act2_entry_snap and act2_entry_snap["hp_after"] <= 25:
        audit_flags.append("low_act2_entry_hp")
    if final_result == "defeat" and final_gold >= 250:
        audit_flags.append("high_unused_gold_at_death")
        high_gold_classification = high_gold_opportunity.get("classification")
        if high_gold_classification:
            audit_flags.append(f"high_gold_class:{high_gold_classification}")
        if shop_visit_count == 0:
            audit_flags.append("high_gold_death_no_shop_visit_observed")
        elif shop_purchase_count == 0:
            audit_flags.append("high_gold_death_shop_visit_no_purchase")
        else:
            audit_flags.append("high_unused_gold_after_shop_spend")
    if final_result == "defeat" and final_deck_size >= 22:
        audit_flags.append("large_deck_at_death")

    return {
        "schema_version": "run_level_record_v1",
        "source_path": str(path),
        "seed": seed,
        "boss": boss,
        "result": final_result,
        "terminal_reason": final.get("terminal_reason"),
        "final": {
            "act": final_act,
            "floor": final_floor,
            "hp": final_hp,
            "max_hp": safe_int(final.get("max_hp")),
            "gold": final_gold,
            "deck_size": final_deck_size,
            "relic_count": final_relic_count,
            "combat_win_count": safe_int(final.get("combat_win_count")),
            "step_count": safe_int(final.get("step")),
        },
        "bottleneck": bottleneck,
        "act1_boss_entry": boss_entry_snap,
        "act1_boss_exit": boss_exit_snap,
        "act2_entry": act2_entry_snap,
        "decision_type_counts": dict(decision_type_counts),
        "action_family_counts": dict(action_family_counts),
        "visited_room_counts": visited_room_counts(steps),
        "map_opportunities": map_opportunities,
        "candidate_snapshots": candidate_snapshot_summary,
        "high_gold_opportunity": high_gold_opportunity,
        "shop": {
            "visit_count": shop_visit_count,
            "purchase_count": shop_purchase_count,
            "buy_card_count": sum(safe_int(visit.get("buy_card_count")) for visit in shop_visits),
            "buy_relic_count": sum(safe_int(visit.get("buy_relic_count")) for visit in shop_visits),
            "buy_potion_count": sum(safe_int(visit.get("buy_potion_count")) for visit in shop_visits),
            "purge_card_count": sum(safe_int(visit.get("purge_card_count")) for visit in shop_visits),
            "gold_spent": shop_gold_spent,
            "visits": shop_visits,
        },
        "campfire": {
            "visit_count": len(campfire_visits),
            "rest_count": campfire_rest_count,
            "smith_count": campfire_smith_count,
            "visits": campfire_visits,
        },
        "picked_cards": picked_cards,
        "played_card_counts": dict(Counter(played_cards)),
        "audit_flags": audit_flags,
        "trainable_as_action_label": False,
    }


def summarize_strict_ab(path: Path, summary: dict[str, Any]) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for result in summary.get("episode_results") or []:
        if result.get("policy_kind") != "strict_evidence_policy_v0":
            continue
        final_result = str(result.get("final_result") or "unknown")
        final_act = safe_int((result.get("final_info") or {}).get("act"))
        final_floor = safe_int(result.get("floor"))
        bottleneck = "ongoing_or_victory"
        if final_result == "defeat":
            if final_act <= 1 and final_floor < 16:
                bottleneck = "act1_pre_boss_death"
            elif final_act == 1 and final_floor == 16:
                bottleneck = "act1_boss_death"
            elif final_act == 2 and final_floor <= 20:
                bottleneck = "act2_entry_death"
            else:
                bottleneck = "later_death"
        records.append(
            {
                "schema_version": "run_level_record_v1",
                "source_path": str(path),
                "seed": safe_int(result.get("seed")),
                "boss": None,
                "result": final_result,
                "terminal_reason": result.get("terminal_reason"),
                "final": {
                    "act": final_act,
                    "floor": final_floor,
                    "hp": safe_int(result.get("hp")),
                    "max_hp": safe_int(result.get("max_hp")),
                    "gold": safe_int(result.get("gold")),
                    "deck_size": safe_int(result.get("deck_size")),
                    "relic_count": safe_int(result.get("relic_count")),
                    "combat_win_count": safe_int(result.get("combat_win_count")),
                    "step_count": safe_int(result.get("steps")),
                },
                "bottleneck": bottleneck,
                "act1_boss_entry": None,
                "act1_boss_exit": None,
                "act2_entry": None,
                "decision_type_counts": {},
                "action_family_counts": {},
                "visited_room_counts": {},
                "map_opportunities": [],
                "candidate_snapshots": {
                    "schema_version": "run_candidate_snapshot_summary_v1",
                    "snapshot_count": 0,
                    "missing_noncombat_snapshot_count": 0,
                    "forbidden_snapshot_count": 0,
                    "candidate_count_by_decision_type": {},
                    "snapshots": [],
                },
                "high_gold_opportunity": {
                    "threshold": 250,
                    "classification": "not_available_from_strict_summary",
                    "actionable": False,
                },
                "shop": {
                    "visit_count": 0,
                    "purchase_count": 0,
                    "buy_card_count": 0,
                    "buy_relic_count": 0,
                    "buy_potion_count": 0,
                    "purge_card_count": 0,
                    "gold_spent": 0,
                    "visits": [],
                },
                "campfire": {
                    "visit_count": 0,
                    "rest_count": 0,
                    "smith_count": 0,
                    "visits": [],
                },
                "picked_cards": [],
                "played_card_counts": {},
                "audit_flags": [],
                "trainable_as_action_label": False,
            }
        )
    return records


def numeric_stats(values: list[int]) -> dict[str, Any]:
    if not values:
        return {"count": 0}
    return {
        "count": len(values),
        "min": min(values),
        "median": median(values),
        "mean": mean(values),
        "max": max(values),
    }


READINESS_FIELDS = [
    "hp",
    "max_hp",
    "gold",
    "deck_size",
    "relic_count",
    "potion_count",
    "shop_visit_count",
    "shop_purchase_count",
    "shop_gold_spent",
    "campfire_visit_count",
    "campfire_rest_count",
    "campfire_smith_count",
    "monster_rooms",
    "elite_rooms",
    "event_rooms",
    "shop_rooms",
    "campfire_rooms",
    "treasure_rooms",
    "deck_attack_count",
    "deck_skill_count",
    "deck_power_count",
    "deck_damage_card_count",
    "deck_block_card_count",
    "deck_draw_card_count",
    "deck_exhaust_card_count",
    "deck_scaling_card_count",
    "deck_starter_basic_count",
    "deck_curse_count",
    "deck_status_count",
    "deck_upgraded_count",
    "deck_average_cost_milli",
]


def boss_outcome_group(record: dict[str, Any]) -> str:
    if not record.get("act1_boss_entry"):
        return "did_not_reach_act1_boss"
    if record.get("bottleneck") == "act1_boss_death":
        return "died_at_act1_boss"
    return "cleared_act1_boss"


def readiness_features(record: dict[str, Any], snapshot_key: str) -> dict[str, Any] | None:
    snap = record.get(snapshot_key) or {}
    if not snap:
        return None
    deck = snap.get("deck") or {}
    shop = record.get("shop") or {}
    campfire = record.get("campfire") or {}
    rooms = record.get("visited_room_counts") or {}
    final = record.get("final") or {}
    return {
        "seed": record.get("seed"),
        "boss": record.get("boss") or "unknown",
        "bottleneck": record.get("bottleneck") or "unknown",
        "boss_outcome": boss_outcome_group(record),
        "final_act": safe_int(final.get("act")),
        "final_floor": safe_int(final.get("floor")),
        "final_hp": safe_int(final.get("hp")),
        "hp": safe_int(snap.get("hp")),
        "max_hp": safe_int(snap.get("max_hp")),
        "gold": safe_int(snap.get("gold")),
        "deck_size": safe_int(snap.get("deck_size")),
        "relic_count": safe_int(snap.get("relic_count")),
        "potion_count": safe_int(snap.get("potion_count")),
        "shop_visit_count": safe_int(shop.get("visit_count")),
        "shop_purchase_count": safe_int(shop.get("purchase_count")),
        "shop_gold_spent": safe_int(shop.get("gold_spent")),
        "campfire_visit_count": safe_int(campfire.get("visit_count")),
        "campfire_rest_count": safe_int(campfire.get("rest_count")),
        "campfire_smith_count": safe_int(campfire.get("smith_count")),
        "monster_rooms": safe_int(rooms.get("MonsterRoom")),
        "elite_rooms": safe_int(rooms.get("MonsterRoomElite")),
        "event_rooms": safe_int(rooms.get("EventRoom")),
        "shop_rooms": safe_int(rooms.get("ShopRoom")),
        "campfire_rooms": safe_int(rooms.get("RestRoom")),
        "treasure_rooms": safe_int(rooms.get("TreasureRoom")),
        "deck_attack_count": safe_int(deck.get("attack_count")),
        "deck_skill_count": safe_int(deck.get("skill_count")),
        "deck_power_count": safe_int(deck.get("power_count")),
        "deck_damage_card_count": safe_int(deck.get("damage_card_count")),
        "deck_block_card_count": safe_int(deck.get("block_card_count")),
        "deck_draw_card_count": safe_int(deck.get("draw_card_count")),
        "deck_exhaust_card_count": safe_int(deck.get("exhaust_card_count")),
        "deck_scaling_card_count": safe_int(deck.get("scaling_card_count")),
        "deck_starter_basic_count": safe_int(deck.get("starter_basic_count")),
        "deck_curse_count": safe_int(deck.get("curse_count")),
        "deck_status_count": safe_int(deck.get("status_count")),
        "deck_upgraded_count": safe_int(deck.get("upgraded_count")),
        "deck_average_cost_milli": safe_int(deck.get("average_cost_milli")),
        "deck_card_ids": snap.get("deck_card_ids") or [],
        "relic_ids": snap.get("relic_ids") or [],
        "potion_ids": snap.get("potion_ids") or [],
    }


def readiness_group_summary(
    rows: list[dict[str, Any]],
    *,
    group_key: str,
    max_items: int = 12,
) -> dict[str, Any]:
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        groups[str(row.get(group_key) or "unknown")].append(row)
    summary: dict[str, Any] = {}
    for key, members in groups.items():
        card_counts = Counter(card for row in members for card in row.get("deck_card_ids") or [])
        relic_counts = Counter(relic for row in members for relic in row.get("relic_ids") or [])
        potion_counts = Counter(potion for row in members for potion in row.get("potion_ids") or [])
        summary[key] = {
            "count": len(members),
            "numeric": {
                field: numeric_stats([safe_int(row.get(field)) for row in members])
                for field in READINESS_FIELDS
            },
            "top_deck_cards": dict(card_counts.most_common(max_items)),
            "top_relics": dict(relic_counts.most_common(max_items)),
            "top_potions": dict(potion_counts.most_common(max_items)),
        }
    return summary


def readiness_slices(records: list[dict[str, Any]]) -> dict[str, Any]:
    boss_entry_rows = [
        row
        for record in records
        if (row := readiness_features(record, "act1_boss_entry")) is not None
    ]
    act2_entry_rows = [
        row
        for record in records
        if (row := readiness_features(record, "act2_entry")) is not None
    ]
    return {
        "act1_boss_entry": {
            "count": len(boss_entry_rows),
            "by_boss": readiness_group_summary(boss_entry_rows, group_key="boss"),
            "by_bottleneck": readiness_group_summary(boss_entry_rows, group_key="bottleneck"),
            "by_boss_outcome": readiness_group_summary(boss_entry_rows, group_key="boss_outcome"),
        },
        "act2_entry": {
            "count": len(act2_entry_rows),
            "by_boss": readiness_group_summary(act2_entry_rows, group_key="boss"),
            "by_bottleneck": readiness_group_summary(act2_entry_rows, group_key="bottleneck"),
        },
    }


FAILURE_CLASS_NAMES = [
    "low_boss_entry_hp",
    "low_shop_conversion",
    "route_gold_conversion_gap",
    "low_upgrade_conversion",
    "low_damage_readiness",
    "low_block_readiness",
    "act2_entry_hp_pressure",
    "act2_frontload_or_block_gap",
]


def median_or_none(values: list[int]) -> int | float | None:
    return median(values) if values else None


def threshold_record(
    *,
    field: str,
    snapshot: str,
    reference_group: str,
    threshold: int | float | None,
    threshold_source: str,
    direction: str,
) -> dict[str, Any]:
    return {
        "field": field,
        "snapshot": snapshot,
        "reference_group": reference_group,
        "threshold": threshold,
        "threshold_source": threshold_source,
        "direction": direction,
    }


def build_failure_thresholds(records: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    boss_rows = [
        row
        for record in records
        if (row := readiness_features(record, "act1_boss_entry")) is not None
    ]
    boss_ref = [row for row in boss_rows if row.get("boss_outcome") == "cleared_act1_boss"]
    boss_source = "act1_boss_entry.cleared_act1_boss.median" if boss_ref else "act1_boss_entry.all.median"
    boss_reference = boss_ref or boss_rows

    act2_rows = [
        row
        for record in records
        if (row := readiness_features(record, "act2_entry")) is not None
    ]
    act2_ref = [
        row
        for row in act2_rows
        if str(row.get("bottleneck")) not in {"act2_entry_death"}
    ]
    act2_source = "act2_entry.non_act2_entry_death.median" if act2_ref else "act2_entry.all.median"
    act2_reference = act2_ref or act2_rows

    def boss_threshold(field: str, direction: str = "lt") -> dict[str, Any]:
        return threshold_record(
            field=field,
            snapshot="act1_boss_entry",
            reference_group="cleared_act1_boss" if boss_ref else "all_act1_boss_entries",
            threshold=median_or_none([safe_int(row.get(field)) for row in boss_reference]),
            threshold_source=boss_source,
            direction=direction,
        )

    def act2_threshold(field: str, direction: str = "lt") -> dict[str, Any]:
        return threshold_record(
            field=field,
            snapshot="act2_entry",
            reference_group="non_act2_entry_death" if act2_ref else "all_act2_entries",
            threshold=median_or_none([safe_int(row.get(field)) for row in act2_reference]),
            threshold_source=act2_source,
            direction=direction,
        )

    return {
        "low_boss_entry_hp": boss_threshold("hp"),
        "low_shop_conversion": boss_threshold("shop_gold_spent"),
        "low_upgrade_conversion": boss_threshold("campfire_smith_count"),
        "low_damage_readiness": boss_threshold("deck_damage_card_count"),
        "low_block_readiness": boss_threshold("deck_block_card_count"),
        "act2_entry_hp_pressure": act2_threshold("hp"),
        "act2_damage_readiness": act2_threshold("deck_damage_card_count"),
        "act2_block_readiness": act2_threshold("deck_block_card_count"),
    }


def class_basis(
    *,
    current_value: Any,
    threshold: dict[str, Any] | None,
    snapshot: dict[str, Any] | None,
    related_snapshot: dict[str, Any] | None = None,
    extra: dict[str, Any] | None = None,
) -> dict[str, Any]:
    basis = {
        "current_value": current_value,
        "threshold": (threshold or {}).get("threshold"),
        "threshold_source": (threshold or {}).get("threshold_source"),
        "reference_group": (threshold or {}).get("reference_group"),
        "comparison_direction": (threshold or {}).get("direction"),
        "field": (threshold or {}).get("field"),
        "snapshot": snapshot or {},
    }
    if related_snapshot is not None:
        basis["related_snapshot"] = related_snapshot
    if extra:
        basis.update(extra)
    return basis


def value_is_below_threshold(value: Any, threshold: dict[str, Any] | None) -> bool:
    number = safe_float(value, float("nan"))
    cutoff = (threshold or {}).get("threshold")
    if cutoff is None or not math.isfinite(number):
        return False
    return number < safe_float(cutoff)


def record_failure_classes(
    record: dict[str, Any], thresholds: dict[str, dict[str, Any]]
) -> list[dict[str, Any]]:
    classes: list[dict[str, Any]] = []
    boss_features = readiness_features(record, "act1_boss_entry")
    act2_features = readiness_features(record, "act2_entry")
    high_gold = record.get("high_gold_opportunity") or {}

    def add(name: str, basis: dict[str, Any], *, target_families: list[str]) -> None:
        classes.append(
            {
                "schema_version": "run_failure_class_v1",
                "class": name,
                "causal_claim": False,
                "trainable_as_action_label": False,
                "basis": basis,
                "target_families": target_families,
            }
        )

    if boss_features:
        if value_is_below_threshold(boss_features.get("hp"), thresholds.get("low_boss_entry_hp")):
            add(
                "low_boss_entry_hp",
                class_basis(
                    current_value=boss_features.get("hp"),
                    threshold=thresholds.get("low_boss_entry_hp"),
                    snapshot=record.get("act1_boss_entry"),
                ),
                target_families=["act1_boss_exit_gauntlet", "campfire", "route"],
            )
        if value_is_below_threshold(
            boss_features.get("shop_gold_spent"), thresholds.get("low_shop_conversion")
        ):
            add(
                "low_shop_conversion",
                class_basis(
                    current_value=boss_features.get("shop_gold_spent"),
                    threshold=thresholds.get("low_shop_conversion"),
                    snapshot=record.get("act1_boss_entry"),
                    extra={
                        "shop": record.get("shop") or {},
                        "high_gold_opportunity": high_gold,
                    },
                ),
                target_families=["route_to_shop", "shop_purchase"],
            )
        if value_is_below_threshold(
            boss_features.get("campfire_smith_count"), thresholds.get("low_upgrade_conversion")
        ):
            add(
                "low_upgrade_conversion",
                class_basis(
                    current_value=boss_features.get("campfire_smith_count"),
                    threshold=thresholds.get("low_upgrade_conversion"),
                    snapshot=record.get("act1_boss_entry"),
                    extra={"campfire": record.get("campfire") or {}},
                ),
                target_families=["campfire_smith_rest_counterfactual", "campfire_upgrade"],
            )
        if value_is_below_threshold(
            boss_features.get("deck_damage_card_count"), thresholds.get("low_damage_readiness")
        ):
            add(
                "low_damage_readiness",
                class_basis(
                    current_value=boss_features.get("deck_damage_card_count"),
                    threshold=thresholds.get("low_damage_readiness"),
                    snapshot=record.get("act1_boss_entry"),
                ),
                target_families=["card_reward", "shop_card", "campfire_upgrade"],
            )
        if value_is_below_threshold(
            boss_features.get("deck_block_card_count"), thresholds.get("low_block_readiness")
        ):
            add(
                "low_block_readiness",
                class_basis(
                    current_value=boss_features.get("deck_block_card_count"),
                    threshold=thresholds.get("low_block_readiness"),
                    snapshot=record.get("act1_boss_entry"),
                ),
                target_families=["card_reward", "shop_card", "campfire_upgrade"],
            )

    if high_gold.get("actionable") or high_gold.get("classification") in {
        "missed_reachable_shop_after_high_gold",
        "shop_path_chosen_but_not_reached_before_death",
        "high_gold_shop_visit_no_purchase",
    }:
        add(
            "route_gold_conversion_gap",
            {
                "current_value": high_gold.get("classification"),
                "threshold": high_gold.get("threshold"),
                "threshold_source": "high_gold_opportunity.threshold",
                "reference_group": "same_run_map_opportunities_after_high_gold",
                "comparison_direction": "categorical_actionable",
                "field": "high_gold_opportunity.classification",
                "snapshot": high_gold,
            },
            target_families=["route_to_shop", "shop_purchase"],
        )

    if act2_features:
        if value_is_below_threshold(
            act2_features.get("hp"), thresholds.get("act2_entry_hp_pressure")
        ):
            add(
                "act2_entry_hp_pressure",
                class_basis(
                    current_value=act2_features.get("hp"),
                    threshold=thresholds.get("act2_entry_hp_pressure"),
                    snapshot=record.get("act2_entry"),
                ),
                target_families=["act1_boss_exit_gauntlet", "act2_entry_gauntlet"],
            )
        damage_low = value_is_below_threshold(
            act2_features.get("deck_damage_card_count"), thresholds.get("act2_damage_readiness")
        )
        block_low = value_is_below_threshold(
            act2_features.get("deck_block_card_count"), thresholds.get("act2_block_readiness")
        )
        if damage_low or block_low:
            add(
                "act2_frontload_or_block_gap",
                {
                    "current_value": {
                        "deck_damage_card_count": act2_features.get("deck_damage_card_count"),
                        "deck_block_card_count": act2_features.get("deck_block_card_count"),
                    },
                    "threshold": {
                        "deck_damage_card_count": (thresholds.get("act2_damage_readiness") or {}).get("threshold"),
                        "deck_block_card_count": (thresholds.get("act2_block_readiness") or {}).get("threshold"),
                    },
                    "threshold_source": {
                        "deck_damage_card_count": (thresholds.get("act2_damage_readiness") or {}).get("threshold_source"),
                        "deck_block_card_count": (thresholds.get("act2_block_readiness") or {}).get("threshold_source"),
                    },
                    "reference_group": {
                        "deck_damage_card_count": (thresholds.get("act2_damage_readiness") or {}).get("reference_group"),
                        "deck_block_card_count": (thresholds.get("act2_block_readiness") or {}).get("reference_group"),
                    },
                    "comparison_direction": "lt_any",
                    "field": "deck_damage_card_count_or_deck_block_card_count",
                    "snapshot": record.get("act2_entry") or {},
                    "damage_low": damage_low,
                    "block_low": block_low,
                },
                target_families=["card_reward", "shop_card", "act2_entry_gauntlet"],
            )

    return classes


def attach_failure_classes(records: list[dict[str, Any]]) -> dict[str, Any]:
    thresholds = build_failure_thresholds(records)
    counts: Counter[str] = Counter()
    by_bottleneck: dict[str, Counter[str]] = defaultdict(Counter)
    target_family_counts: Counter[str] = Counter()
    missing_basis_count = 0
    for record in records:
        classes = record_failure_classes(record, thresholds)
        record["failure_classes_v1"] = classes
        for item in classes:
            name = str(item.get("class") or "unknown")
            counts[name] += 1
            by_bottleneck[str(record.get("bottleneck") or "unknown")][name] += 1
            if not item.get("basis") or item.get("basis", {}).get("threshold_source") is None:
                missing_basis_count += 1
            for family in item.get("target_families") or []:
                target_family_counts[str(family)] += 1
    return {
        "schema_version": "failure_classes_v1_summary",
        "class_counts": dict(counts),
        "by_bottleneck": {key: dict(value) for key, value in by_bottleneck.items()},
        "target_family_counts": dict(target_family_counts),
        "thresholds": thresholds,
        "missing_basis_or_threshold_source_count": missing_basis_count,
        "trainable_as_action_label": False,
    }


def aggregate_records(records: list[dict[str, Any]]) -> dict[str, Any]:
    failure_class_summary = attach_failure_classes(records)
    bottlenecks = Counter(record.get("bottleneck") or "unknown" for record in records)
    results = Counter(record.get("result") or "unknown" for record in records)
    bosses = Counter(record.get("boss") or "unknown" for record in records)
    flags = Counter(flag for record in records for flag in record.get("audit_flags") or [])
    final_floors = [safe_int((record.get("final") or {}).get("floor")) for record in records]
    final_hp = [safe_int((record.get("final") or {}).get("hp")) for record in records]
    final_gold = [safe_int((record.get("final") or {}).get("gold")) for record in records]
    boss_entry_hp = [
        safe_int((record.get("act1_boss_entry") or {}).get("hp"))
        for record in records
        if record.get("act1_boss_entry")
    ]
    boss_exit_hp = [
        safe_int((record.get("act1_boss_exit") or {}).get("hp_after"))
        for record in records
        if record.get("act1_boss_exit")
    ]
    act2_entry_hp = [
        safe_int((record.get("act2_entry") or {}).get("hp_after"))
        for record in records
        if record.get("act2_entry")
    ]
    picked_cards = Counter(card for record in records for card in record.get("picked_cards") or [])
    action_families: Counter[str] = Counter()
    decision_types: Counter[str] = Counter()
    room_counts: Counter[str] = Counter()
    for record in records:
        action_families.update(record.get("action_family_counts") or {})
        decision_types.update(record.get("decision_type_counts") or {})
        room_counts.update(record.get("visited_room_counts") or {})
    shop_visit_counts = [safe_int((record.get("shop") or {}).get("visit_count")) for record in records]
    shop_purchase_counts = [
        safe_int((record.get("shop") or {}).get("purchase_count")) for record in records
    ]
    shop_gold_spent = [safe_int((record.get("shop") or {}).get("gold_spent")) for record in records]
    campfire_visit_counts = [
        safe_int((record.get("campfire") or {}).get("visit_count")) for record in records
    ]
    campfire_rest_counts = [
        safe_int((record.get("campfire") or {}).get("rest_count")) for record in records
    ]
    campfire_smith_counts = [
        safe_int((record.get("campfire") or {}).get("smith_count")) for record in records
    ]
    shop_purchase_families: Counter[str] = Counter()
    shop_by_bottleneck: dict[str, Counter[str]] = defaultdict(Counter)
    campfire_by_bottleneck: dict[str, Counter[str]] = defaultdict(Counter)
    high_gold_classifications = Counter()
    high_gold_by_bottleneck: dict[str, Counter[str]] = defaultdict(Counter)
    map_opportunity_counts: Counter[str] = Counter()
    candidate_snapshot_counts: Counter[str] = Counter()
    candidate_snapshot_by_decision: Counter[str] = Counter()
    for record in records:
        bottleneck = str(record.get("bottleneck") or "unknown")
        shop = record.get("shop") or {}
        campfire = record.get("campfire") or {}
        high_gold = record.get("high_gold_opportunity") or {}
        high_gold_class = str(high_gold.get("classification") or "unknown")
        high_gold_classifications[high_gold_class] += 1
        high_gold_by_bottleneck[bottleneck][high_gold_class] += 1
        candidate_snapshots = record.get("candidate_snapshots") or {}
        candidate_snapshot_counts["snapshot_count"] += safe_int(
            candidate_snapshots.get("snapshot_count")
        )
        candidate_snapshot_counts["missing_noncombat_snapshot_count"] += safe_int(
            candidate_snapshots.get("missing_noncombat_snapshot_count")
        )
        candidate_snapshot_counts["forbidden_snapshot_count"] += safe_int(
            candidate_snapshots.get("forbidden_snapshot_count")
        )
        candidate_snapshot_by_decision.update(
            candidate_snapshots.get("candidate_count_by_decision_type") or {}
        )
        map_entries = record.get("map_opportunities") or []
        map_opportunity_counts["map_decisions"] += len(map_entries)
        map_opportunity_counts["map_decisions_with_shop_option"] += sum(
            1 for entry in map_entries if entry.get("any_shop_reachable")
        )
        map_opportunity_counts["map_decisions_chosen_shop_path"] += sum(
            1 for entry in map_entries if entry.get("chosen_shop_reachable")
        )
        map_opportunity_counts["high_gold_map_decisions"] += sum(
            1
            for entry in map_entries
            if safe_int(entry.get("gold")) >= safe_int(high_gold.get("threshold"), 250)
        )
        map_opportunity_counts["high_gold_shop_option_decisions"] += sum(
            1
            for entry in map_entries
            if safe_int(entry.get("gold")) >= safe_int(high_gold.get("threshold"), 250)
            and entry.get("any_shop_reachable")
        )
        map_opportunity_counts["high_gold_missed_shop_path_decisions"] += sum(
            1
            for entry in map_entries
            if safe_int(entry.get("gold")) >= safe_int(high_gold.get("threshold"), 250)
            and entry.get("any_shop_reachable")
            and not entry.get("chosen_shop_reachable")
        )
        shop_purchase_families["buy_card"] += safe_int(shop.get("buy_card_count"))
        shop_purchase_families["buy_relic"] += safe_int(shop.get("buy_relic_count"))
        shop_purchase_families["buy_potion"] += safe_int(shop.get("buy_potion_count"))
        shop_purchase_families["purge_card"] += safe_int(shop.get("purge_card_count"))
        shop_by_bottleneck[bottleneck]["runs"] += 1
        shop_by_bottleneck[bottleneck]["visit_runs"] += 1 if safe_int(shop.get("visit_count")) > 0 else 0
        shop_by_bottleneck[bottleneck]["purchase_runs"] += 1 if safe_int(shop.get("purchase_count")) > 0 else 0
        shop_by_bottleneck[bottleneck]["gold_spent"] += safe_int(shop.get("gold_spent"))
        campfire_by_bottleneck[bottleneck]["runs"] += 1
        campfire_by_bottleneck[bottleneck]["visit_runs"] += 1 if safe_int(campfire.get("visit_count")) > 0 else 0
        campfire_by_bottleneck[bottleneck]["rest_count"] += safe_int(campfire.get("rest_count"))
        campfire_by_bottleneck[bottleneck]["smith_count"] += safe_int(campfire.get("smith_count"))
    return {
        "schema_version": "run_level_audit_summary_v1",
        "run_count": len(records),
        "result_counts": dict(results),
        "boss_counts": dict(bosses),
        "bottleneck_counts": dict(bottlenecks),
        "audit_flag_counts": dict(flags),
        "final_floor_stats": numeric_stats(final_floors),
        "final_hp_stats": numeric_stats(final_hp),
        "final_gold_stats": numeric_stats(final_gold),
        "act1_boss_entry_hp_stats": numeric_stats(boss_entry_hp),
        "act1_boss_exit_hp_stats": numeric_stats(boss_exit_hp),
        "act2_entry_hp_stats": numeric_stats(act2_entry_hp),
        "decision_type_counts": dict(decision_types),
        "action_family_counts": dict(action_families),
        "visited_room_counts": dict(room_counts),
        "shop_summary": {
            "visit_count_stats": numeric_stats(shop_visit_counts),
            "purchase_count_stats": numeric_stats(shop_purchase_counts),
            "gold_spent_stats": numeric_stats(shop_gold_spent),
            "runs_with_shop_visit": sum(1 for count in shop_visit_counts if count > 0),
            "runs_with_shop_purchase": sum(1 for count in shop_purchase_counts if count > 0),
            "purchase_family_counts": dict(shop_purchase_families),
            "by_bottleneck": {key: dict(value) for key, value in shop_by_bottleneck.items()},
        },
        "campfire_summary": {
            "visit_count_stats": numeric_stats(campfire_visit_counts),
            "rest_count_stats": numeric_stats(campfire_rest_counts),
            "smith_count_stats": numeric_stats(campfire_smith_counts),
            "runs_with_campfire": sum(1 for count in campfire_visit_counts if count > 0),
            "by_bottleneck": {key: dict(value) for key, value in campfire_by_bottleneck.items()},
        },
        "map_opportunity_summary": dict(map_opportunity_counts),
        "high_gold_opportunity_summary": {
            "classification_counts": dict(high_gold_classifications),
            "by_bottleneck": {key: dict(value) for key, value in high_gold_by_bottleneck.items()},
            "actionable_count": sum(
                1 for record in records if (record.get("high_gold_opportunity") or {}).get("actionable")
            ),
        },
        "candidate_snapshot_summary": {
            **dict(candidate_snapshot_counts),
            "candidate_count_by_decision_type": dict(candidate_snapshot_by_decision),
            "coverage_note": "old readable logs may have missing candidate snapshots; target builder must treat those as unavailable evidence",
        },
        "failure_classes_v1_summary": failure_class_summary,
        "readiness_slices": readiness_slices(records),
        "picked_card_counts": dict(picked_cards.most_common()),
        "records": records,
        "label_safety": {
            "trainable_as_action_label": False,
            "contains_policy_scores": False,
            "contains_card_tier_scores": False,
            "intended_use": "run_level_fact_table_for_evaluator_design",
        },
    }


def markdown_report(summary: dict[str, Any], *, max_rows: int = 30) -> str:
    lines = [
        "# Run-Level Audit",
        "",
        f"- runs: `{summary['run_count']}`",
        f"- results: `{summary['result_counts']}`",
        f"- bottlenecks: `{summary['bottleneck_counts']}`",
        f"- flags: `{summary['audit_flag_counts']}`",
        "",
        "## HP / Gold",
        "",
        f"- Act1 boss entry HP: `{summary['act1_boss_entry_hp_stats']}`",
        f"- Act1 boss exit HP: `{summary['act1_boss_exit_hp_stats']}`",
        f"- Act2 entry HP: `{summary['act2_entry_hp_stats']}`",
        f"- Final gold: `{summary['final_gold_stats']}`",
        "",
        "## Shop / Campfire",
        "",
        f"- Shop: `{summary.get('shop_summary', {})}`",
        f"- Campfire: `{summary.get('campfire_summary', {})}`",
        f"- Visited rooms: `{summary.get('visited_room_counts', {})}`",
        "",
        "## Map Opportunity",
        "",
        f"- Map opportunity: `{summary.get('map_opportunity_summary', {})}`",
        f"- High-gold classifications: `{summary.get('high_gold_opportunity_summary', {})}`",
        f"- Candidate snapshot coverage: `{summary.get('candidate_snapshot_summary', {})}`",
        "",
        "## Failure Classes V1",
        "",
        f"- Failure classes: `{summary.get('failure_classes_v1_summary', {})}`",
        "",
        "## Readiness Slices",
        "",
        f"- Readiness slices: `{summary.get('readiness_slices', {})}`",
        "",
        "## Runs",
        "",
        "| seed | result | final | boss | bottleneck | boss entry | boss exit | act2 entry | gold | shop | campfire | high gold | flags |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for record in summary.get("records", [])[:max_rows]:
        final = record.get("final") or {}
        boss_entry = record.get("act1_boss_entry") or {}
        boss_exit = record.get("act1_boss_exit") or {}
        act2_entry = record.get("act2_entry") or {}
        shop = record.get("shop") or {}
        campfire = record.get("campfire") or {}
        high_gold = record.get("high_gold_opportunity") or {}
        lines.append(
            "| {seed} | {result} | A{act}F{floor} HP{hp} | {boss} | {bottleneck} | {entry} | {exit} | {act2} | {gold} | {shop} | {campfire} | {high_gold} | {flags} |".format(
                seed=record.get("seed"),
                result=record.get("result"),
                act=final.get("act"),
                floor=final.get("floor"),
                hp=final.get("hp"),
                boss=record.get("boss") or "",
                bottleneck=record.get("bottleneck"),
                entry=boss_entry.get("hp") if boss_entry else "",
                exit=boss_exit.get("hp_after") if boss_exit else "",
                act2=act2_entry.get("hp_after") if act2_entry else "",
                gold=final.get("gold"),
                shop="v{}/p{}/g{}".format(
                    shop.get("visit_count", 0),
                    shop.get("purchase_count", 0),
                    shop.get("gold_spent", 0),
                ),
                campfire="v{}/r{}/s{}".format(
                    campfire.get("visit_count", 0),
                    campfire.get("rest_count", 0),
                    campfire.get("smith_count", 0),
                ),
                high_gold=high_gold.get("classification", ""),
                flags=",".join(record.get("audit_flags") or []),
            )
        )
    lines.append("")
    return "\n".join(lines)


def action_display(candidate: dict[str, Any], index: int) -> str:
    key = str(candidate.get("action_key") or "")
    payload = candidate.get("payload") or {}
    card = payload.get("card") if isinstance(payload.get("card"), dict) else {}
    if card.get("card_id"):
        return f"{candidate.get('action_kind')}:{card.get('card_id')} ({key})"
    return f"{candidate.get('action_kind')} ({key})"


def collect_readable_runs(args: argparse.Namespace) -> list[Path]:
    out_dir = args.collect_out_dir
    out_dir.mkdir(parents=True, exist_ok=True)
    driver = args.driver or default_driver_path()
    client = DriverClient(driver)
    paths: list[Path] = []
    try:
        seeds = args.collect_seeds or [
            args.collect_seed_start + idx * args.collect_seed_step
            for idx in range(args.collect_episodes)
        ]
        for seed in seeds:
            path = out_dir / f"readable_run_seed{seed}_{args.collect_policy}.jsonl"
            paths.append(path)
            client.request(
                {
                    "cmd": "reset",
                    "seed": seed,
                    "ascension": args.ascension,
                    "final_act": args.final_act,
                    "class": "ironclad",
                    "max_steps": args.env_max_steps,
                    "reward_shaping_profile": "baseline",
                }
            )
            done = False
            step_index = 0
            with path.open("w", encoding="utf-8") as out:
                while not done and step_index < args.max_steps:
                    policy_input = client.request({"cmd": "policy_input", "time_budget_ms": 25})[
                        "payload"
                    ]
                    observation = policy_input.get("observation") or {}
                    payload = observation.get("payload") or {}
                    candidates = policy_input.get("candidates") or []
                    if not candidates:
                        break
                    preview = client.request(
                        {
                            "cmd": "preview_policy_action",
                            "policy": args.collect_policy,
                            "include_state": False,
                            "include_next_state": False,
                            "check_live_env_unchanged": False,
                        }
                    )["payload"]
                    action_id = preview.get("chosen_action_index")
                    if not isinstance(action_id, int) or action_id < 0 or action_id >= len(candidates):
                        break
                    candidate = candidates[action_id]
                    step = client.request({"cmd": "decision_env_step", "action_id": action_id})
                    decision_type = payload.get("decision_type") or (
                        policy_input.get("decision_id") or {}
                    ).get("decision_type")
                    record = {
                        "schema_version": "readable_run_record_v1",
                        "step": step_index,
                        "decision_type": decision_type,
                        "observation": observation,
                        "action": {
                            "index": action_id,
                            "key": candidate.get("action_key"),
                            "display": action_display(candidate, action_id),
                        },
                        "candidate_snapshot": build_candidate_snapshot(
                            decision_type=str(decision_type or "unknown"),
                            step=step_index,
                            observation_payload=payload,
                            candidates=candidates,
                        )
                        if is_noncombat_decision_type(str(decision_type or "unknown"))
                        else None,
                        "after": step.get("info") or {},
                        "reward": step.get("reward"),
                        "done_after": bool(step.get("done")),
                        "trainable_as_action_label": False,
                    }
                    out.write(json.dumps(record, separators=(",", ":"), ensure_ascii=False) + "\n")
                    done = bool(step.get("done"))
                    step_index += 1
    finally:
        client.close()
    return paths


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--readable-run", type=Path, action="append", default=[])
    parser.add_argument("--readable-dir", type=Path, action="append", default=[])
    parser.add_argument("--readable-glob", default="*.jsonl")
    parser.add_argument("--strict-summary", type=Path, action="append", default=[])
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--markdown-out", type=Path)
    parser.add_argument("--print-markdown", action="store_true")
    parser.add_argument("--max-markdown-rows", type=int, default=40)
    parser.add_argument("--driver", type=Path)
    parser.add_argument("--collect", action="store_true")
    parser.add_argument("--collect-out-dir", type=Path, default=REPO_ROOT / "tools" / "artifacts" / "run_level_audit_runs")
    parser.add_argument("--collect-seeds", type=int, nargs="*")
    parser.add_argument("--collect-seed-start", type=int, default=5201)
    parser.add_argument("--collect-seed-step", type=int, default=1)
    parser.add_argument("--collect-episodes", type=int, default=0)
    parser.add_argument("--collect-policy", default="rule_baseline_v0")
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=500)
    parser.add_argument("--env-max-steps", type=int, default=600)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    readable_paths = list(args.readable_run)
    for directory in args.readable_dir:
        readable_paths.extend(sorted(directory.glob(args.readable_glob)))
    if args.collect:
        readable_paths.extend(collect_readable_runs(args))
    records: list[dict[str, Any]] = []
    for path in readable_paths:
        records.append(summarize_run(path, read_jsonl(path)))
    for path in args.strict_summary:
        records.extend(summarize_strict_ab(path, json.loads(path.read_text(encoding="utf-8"))))
    if not records:
        raise SystemExit("provide --readable-run, --strict-summary, or --collect")
    summary = aggregate_records(records)
    write_json(args.out, summary)
    markdown = markdown_report(summary, max_rows=args.max_markdown_rows)
    if args.markdown_out:
        args.markdown_out.parent.mkdir(parents=True, exist_ok=True)
        args.markdown_out.write_text(markdown, encoding="utf-8")
    if args.print_markdown:
        print(markdown)
    else:
        print(json.dumps({k: v for k, v in summary.items() if k != "records"}, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
