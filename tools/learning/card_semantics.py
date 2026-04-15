#!/usr/bin/env python3
from __future__ import annotations

from typing import Any

CARD_TYPE_IDS = {
    "unknown": 0.0,
    "attack": 1.0,
    "skill": 2.0,
    "power": 3.0,
    "status": 4.0,
    "curse": 5.0,
}


def _base_entry(**overrides: Any) -> dict[str, Any]:
    base = {
        "card_type": "unknown",
        "base_cost": 0.0,
        "is_x_cost": 0.0,
        "base_damage": 0.0,
        "base_block": 0.0,
        "draw_count": 0.0,
        "apply_strength": 0.0,
        "apply_weak": 0.0,
        "apply_vulnerable": 0.0,
        "apply_frail": 0.0,
        "grants_block": 0.0,
        "deals_damage": 0.0,
        "multi_hit": 0.0,
        "exhausts": 0.0,
        "ethereal": 0.0,
        "retain": 0.0,
        "creates_status": 0.0,
        "consumes_status": 0.0,
        "card_draw_engine": 0.0,
        "setup_tag": 0.0,
        "payoff_tag": 0.0,
        "status_tag": 0.0,
        "attack_tag": 0.0,
        "block_tag": 0.0,
    }
    base.update(overrides)
    base["attack_tag"] = 1.0 if base["card_type"] == "attack" else float(base["attack_tag"])
    base["block_tag"] = 1.0 if base["base_block"] > 0 or base["grants_block"] else float(base["block_tag"])
    return base


CARD_SEMANTICS: dict[str, dict[str, Any]] = {
    "Strike": _base_entry(card_type="attack", base_cost=1.0, base_damage=6.0, deals_damage=1.0, payoff_tag=1.0),
    "Cleave": _base_entry(card_type="attack", base_cost=1.0, base_damage=8.0, deals_damage=1.0, payoff_tag=1.0),
    "Twin Strike": _base_entry(card_type="attack", base_cost=1.0, base_damage=10.0, deals_damage=1.0, multi_hit=1.0, payoff_tag=1.0),
    "Anger": _base_entry(card_type="attack", base_cost=0.0, base_damage=6.0, deals_damage=1.0, payoff_tag=1.0),
    "Bash": _base_entry(card_type="attack", base_cost=2.0, base_damage=8.0, deals_damage=1.0, apply_vulnerable=2.0, payoff_tag=1.0),
    "Hemokinesis": _base_entry(card_type="attack", base_cost=1.0, base_damage=15.0, deals_damage=1.0, payoff_tag=1.0),
    "Pommel Strike": _base_entry(card_type="attack", base_cost=1.0, base_damage=9.0, deals_damage=1.0, draw_count=1.0, payoff_tag=1.0),
    "Intimidate": _base_entry(card_type="skill", base_cost=0.0, apply_weak=1.0),
    "Defend": _base_entry(card_type="skill", base_cost=1.0, base_block=5.0, grants_block=1.0),
    "Shrug It Off": _base_entry(card_type="skill", base_cost=1.0, base_block=8.0, draw_count=1.0, grants_block=1.0, card_draw_engine=1.0),
    "Ghostly Armor": _base_entry(card_type="skill", base_cost=1.0, base_block=10.0, grants_block=1.0, ethereal=1.0),
    "Impervious": _base_entry(card_type="skill", base_cost=2.0, base_block=30.0, grants_block=1.0, exhausts=1.0),
    "Power Through": _base_entry(card_type="skill", base_cost=1.0, base_block=15.0, grants_block=1.0, creates_status=2.0),
    "True Grit": _base_entry(card_type="skill", base_cost=1.0, base_block=7.0, grants_block=1.0, consumes_status=1.0),
    "Battle Trance": _base_entry(card_type="skill", base_cost=0.0, draw_count=3.0, card_draw_engine=1.0),
    "Second Wind": _base_entry(card_type="skill", base_cost=1.0, base_block=5.0, grants_block=1.0, consumes_status=1.0, status_tag=1.0, payoff_tag=1.0),
    "Flex": _base_entry(card_type="skill", base_cost=0.0, apply_strength=2.0, setup_tag=1.0),
    "Rage": _base_entry(card_type="power", base_cost=0.0, grants_block=1.0, setup_tag=1.0),
    "Spot Weakness": _base_entry(card_type="skill", base_cost=1.0, apply_strength=3.0, setup_tag=1.0),
    "Inflame": _base_entry(card_type="power", base_cost=1.0, apply_strength=2.0, setup_tag=1.0),
    "Fire Breathing": _base_entry(card_type="power", base_cost=1.0, setup_tag=1.0, status_tag=1.0, payoff_tag=1.0),
    "Dark Embrace": _base_entry(card_type="power", base_cost=2.0, draw_count=1.0, card_draw_engine=1.0, setup_tag=1.0, status_tag=1.0),
    "Evolve": _base_entry(card_type="power", base_cost=1.0, draw_count=1.0, card_draw_engine=1.0, setup_tag=1.0, status_tag=1.0),
    "Corruption": _base_entry(card_type="power", base_cost=3.0, setup_tag=1.0),
    "Slimed": _base_entry(card_type="status", base_cost=1.0, exhausts=1.0, status_tag=1.0),
    "Burn": _base_entry(card_type="status", base_cost=0.0, status_tag=1.0),
    "Wound": _base_entry(card_type="status", base_cost=0.0, status_tag=1.0),
    "Dazed": _base_entry(card_type="status", base_cost=0.0, ethereal=1.0, status_tag=1.0),
}


def normalize_card_name(name: str | None) -> str:
    raw = str(name or "").strip()
    if not raw:
        return ""
    normalized = raw.split("(", 1)[0].strip()
    normalized = normalized.replace("+", "").strip()
    suffix_map = {
        "Strike_R": "Strike",
        "Defend_R": "Defend",
    }
    return suffix_map.get(normalized, normalized)


def card_semantics(
    name: str | None,
    *,
    cost_for_turn: Any | None = None,
    playable: Any | None = None,
    upgraded: Any | None = None,
) -> dict[str, Any]:
    normalized = normalize_card_name(name)
    semantics = dict(CARD_SEMANTICS.get(normalized, _base_entry()))
    semantics["normalized_name"] = normalized or str(name or "")
    semantics["type_id"] = CARD_TYPE_IDS.get(str(semantics["card_type"]), 0.0)
    semantics["cost_for_turn"] = float(cost_for_turn if cost_for_turn is not None else semantics["base_cost"])
    semantics["playable"] = 1.0 if playable else 0.0
    semantics["upgraded"] = 1.0 if upgraded else 0.0
    return semantics


def card_semantic_vector(
    name: str | None,
    *,
    cost_for_turn: Any | None = None,
    playable: Any | None = None,
    upgraded: Any | None = None,
) -> list[float]:
    semantics = card_semantics(name, cost_for_turn=cost_for_turn, playable=playable, upgraded=upgraded)
    return [
        float(semantics["type_id"]),
        float(semantics["base_cost"]),
        float(semantics["cost_for_turn"]),
        float(semantics["playable"]),
        float(semantics["upgraded"]),
        float(semantics["is_x_cost"]),
        float(semantics["base_damage"]),
        float(semantics["base_block"]),
        float(semantics["draw_count"]),
        float(semantics["apply_strength"]),
        float(semantics["apply_weak"]),
        float(semantics["apply_vulnerable"]),
        float(semantics["apply_frail"]),
        float(semantics["grants_block"]),
        float(semantics["deals_damage"]),
        float(semantics["multi_hit"]),
        float(semantics["exhausts"]),
        float(semantics["ethereal"]),
        float(semantics["retain"]),
        float(semantics["creates_status"]),
        float(semantics["consumes_status"]),
        float(semantics["card_draw_engine"]),
        float(semantics["setup_tag"]),
        float(semantics["payoff_tag"]),
        float(semantics["status_tag"]),
        float(semantics["attack_tag"]),
        float(semantics["block_tag"]),
    ]


def aggregate_semantics_from_cards(cards: list[dict[str, Any]] | None) -> dict[str, float]:
    totals = {
        "hand_setup_count": 0.0,
        "hand_payoff_count": 0.0,
        "setup_cards_in_hand_count": 0.0,
        "attack_cards_in_hand_count": 0.0,
        "block_cards_in_hand_count": 0.0,
        "status_cards_in_hand_count": 0.0,
        "hand_total_base_damage": 0.0,
        "hand_total_base_block": 0.0,
        "hand_total_draw": 0.0,
    }
    for card in cards or []:
        semantics = card_semantics(
            card.get("name") or card.get("card_id"),
            cost_for_turn=card.get("cost_for_turn", card.get("cost")),
            playable=card.get("playable"),
            upgraded=card.get("upgraded"),
        )
        totals["hand_setup_count"] += float(semantics["setup_tag"])
        totals["hand_payoff_count"] += float(semantics["payoff_tag"])
        totals["setup_cards_in_hand_count"] += float(semantics["setup_tag"])
        totals["attack_cards_in_hand_count"] += float(semantics["attack_tag"])
        totals["block_cards_in_hand_count"] += float(semantics["block_tag"])
        totals["status_cards_in_hand_count"] += 1.0 if semantics["card_type"] == "status" else 0.0
        totals["hand_total_base_damage"] += float(semantics["base_damage"])
        totals["hand_total_base_block"] += float(semantics["base_block"])
        totals["hand_total_draw"] += float(semantics["draw_count"])
    return totals
