#!/usr/bin/env python3
from __future__ import annotations

from typing import Any

from card_semantics import card_semantics, normalize_card_name
from combat_rl_common import (
    hand_semantics_counts,
    monster_intent_family_counts,
    monster_name_counts,
    snapshot_state_features,
)
from combat_reranker_common import parse_move_label


def observation_to_snapshot(observation: dict[str, Any], *, act: int = 0, floor: int = 0, gold: int = 0) -> dict[str, Any]:
    hand = []
    for card in observation.get("hand") or []:
        hand.append(
            {
                "id": str(card.get("uuid") or card.get("card_id") or card.get("name") or ""),
                "name": str(card.get("name") or card.get("card_id") or ""),
                "card_id": str(card.get("card_id") or card.get("name") or ""),
                "cost": int(card.get("cost_for_turn") or 0),
                "cost_for_turn": int(card.get("cost_for_turn") or 0),
                "upgrades": 1 if card.get("upgraded") else 0,
                "upgraded": bool(card.get("upgraded")),
                "playable": bool(card.get("playable")),
            }
        )
    monsters = []
    for monster in observation.get("monsters") or []:
        monsters.append(
            {
                "id": monster.get("entity_id"),
                "name": str(monster.get("name") or monster.get("entity_id") or ""),
                "current_hp": int(monster.get("current_hp") or 0),
                "max_hp": int(monster.get("max_hp") or monster.get("current_hp") or 0),
                "block": int(monster.get("block") or 0),
                "intent": str(monster.get("visible_intent") or ""),
                "powers": [],
            }
        )
    return {
        "act": int(act),
        "floor": int(floor),
        "gold": int(gold),
        "player": {
            "current_hp": int(observation.get("player_hp") or 0),
            "max_hp": int(observation.get("player_max_hp") or observation.get("player_hp") or 0),
            "block": int(observation.get("player_block") or 0),
            "energy": int(observation.get("energy") or 0),
            "powers": [],
        },
        "monsters": monsters,
        "zones": {
            "hand_count": len(hand),
            "draw_count": int(observation.get("draw_count") or 0),
            "discard_count": int(observation.get("discard_count") or 0),
            "exhaust_count": int(observation.get("exhaust_count") or 0),
            "hand": hand,
            "draw": [],
            "discard": [],
            "exhaust": [],
        },
    }


def candidate_semantics_from_move(move_label: str | None) -> dict[str, Any]:
    parsed = parse_move_label(move_label)
    semantics = card_semantics(parsed.get("card_name"))
    return {
        "move_label": str(move_label or ""),
        "move_family": parsed.get("move_family"),
        "card_name": normalize_card_name(parsed.get("card_name")),
        "slot_index": parsed.get("slot_index"),
        "target_index": parsed.get("target_index"),
        "has_target": bool(parsed.get("has_target")),
        "card_type": semantics["card_type"],
        "type_id": float(semantics["type_id"]),
        "base_cost": float(semantics["base_cost"]),
        "base_damage": float(semantics["base_damage"]),
        "base_block": float(semantics["base_block"]),
        "draw_count": float(semantics["draw_count"]),
        "apply_strength": float(semantics["apply_strength"]),
        "apply_weak": float(semantics["apply_weak"]),
        "apply_vulnerable": float(semantics["apply_vulnerable"]),
        "apply_frail": float(semantics["apply_frail"]),
        "grants_block": float(semantics["grants_block"]),
        "deals_damage": float(semantics["deals_damage"]),
        "multi_hit": float(semantics["multi_hit"]),
        "exhausts": float(semantics["exhausts"]),
        "ethereal": float(semantics["ethereal"]),
        "retain": float(semantics["retain"]),
        "creates_status": float(semantics["creates_status"]),
        "consumes_status": float(semantics["consumes_status"]),
        "card_draw_engine": float(semantics["card_draw_engine"]),
        "setup_tag": float(semantics["setup_tag"]),
        "payoff_tag": float(semantics["payoff_tag"]),
        "status_tag": float(semantics["status_tag"]),
        "attack_tag": float(semantics["attack_tag"]),
        "block_tag": float(semantics["block_tag"]),
    }


def _approx_hit_prob(successes: float, population: float, draws: float) -> float:
    population = max(float(population), 1.0)
    draws = max(float(draws), 0.0)
    successes = min(max(float(successes), 0.0), population)
    if draws <= 0.0 or successes <= 0.0:
        return 0.0
    miss_prob = (1.0 - (successes / population)) ** min(draws, population)
    return float(max(0.0, min(1.0, 1.0 - miss_prob)))


def chance_feature_dict(snapshot: dict[str, Any] | None, candidate_move: str | None = None, *, draw_window: int = 3) -> dict[str, float]:
    state = snapshot or {}
    snapshot_features = snapshot_state_features(state)
    hand_counts = hand_semantics_counts(state)
    candidate = candidate_semantics_from_move(candidate_move)
    hand_count = max(float(snapshot_features.get("hand_count") or 0.0), 1.0)
    draw_count = float(snapshot_features.get("draw_count") or 0.0)
    discard_count = float(snapshot_features.get("discard_count") or 0.0)
    exhaust_count = float(snapshot_features.get("exhaust_count") or 0.0)
    total_known = max(hand_count + draw_count + discard_count + exhaust_count, hand_count)
    setup_density = float(hand_counts.get("hand_setup_count", 0.0)) / hand_count
    payoff_density = float(hand_counts.get("hand_payoff_count", 0.0)) / hand_count
    draw_density = float(hand_counts.get("hand_total_draw", 0.0)) / hand_count
    attack_density = float(hand_counts.get("attack_cards_in_hand_count", 0.0)) / hand_count
    status_density = float(hand_counts.get("status_cards_in_hand_count", 0.0)) / hand_count
    playable_density = max(0.0, 1.0 - status_density)
    draw_window = int(max(draw_window, 1))
    k_draw_hit_payoff_prob = _approx_hit_prob(payoff_density * draw_count, max(draw_count, 1.0), min(draw_window, draw_count))
    k_draw_hit_setup_prob = _approx_hit_prob(setup_density * draw_count, max(draw_count, 1.0), min(draw_window, draw_count))
    draw_chain_start_prob = _approx_hit_prob(draw_density * draw_count, max(draw_count, 1.0), min(draw_window, draw_count))
    expected_playable_cards_next_turn = min(draw_count, float(draw_window + 2)) * playable_density
    attack_density_next_window = max(0.0, min(1.0, ((attack_density * hand_count) + (attack_density * draw_count)) / max(hand_count + draw_count, 1.0)))
    status_pollution_prob = max(0.0, min(1.0, status_density + (candidate.get("creates_status", 0.0) / max(total_known, 1.0))))
    payoff_reachability_after_setup_proxy = max(
        k_draw_hit_payoff_prob,
        1.0 if hand_counts.get("hand_payoff_count", 0.0) > candidate.get("payoff_tag", 0.0) else 0.0,
    )
    return {
        "k_draw_hit_payoff_prob": float(k_draw_hit_payoff_prob),
        "k_draw_hit_setup_prob": float(k_draw_hit_setup_prob),
        "draw_chain_start_prob": float(draw_chain_start_prob),
        "attack_density_next_window": float(attack_density_next_window),
        "status_pollution_prob": float(status_pollution_prob),
        "expected_playable_cards_next_turn": float(expected_playable_cards_next_turn),
        "payoff_reachability_after_setup_proxy": float(payoff_reachability_after_setup_proxy if candidate.get("setup_tag", 0.0) > 0.0 else 0.0),
    }


def eval_bucket_from_row(row: dict[str, Any]) -> str:
    tag = str(row.get("curriculum_tag") or "")
    if tag == "setup_before_payoff":
        return "setup_vs_payoff"
    if tag == "status_exhaust_draw":
        return "status_exhaust_management"
    tags = set(str(tag) for tag in (row.get("sample_tags") or []))
    if "kill_now_missed" in tags or "oracle_save" in tags or tag == "attack_over_defend":
        return "trade_hp_for_kill"
    return "draw_chain_start"


def q_local_feature_dict(row: dict[str, Any]) -> dict[str, Any]:
    features: dict[str, Any] = {}
    snapshot = row.get("state_before") or row.get("snapshot_normalized_state") or {}
    state = snapshot_state_features(snapshot)
    candidate = row.get("candidate_semantics") or candidate_semantics_from_move(row.get("candidate_move"))
    chance = row.get("chance_features") or chance_feature_dict(snapshot, row.get("candidate_move"))
    dynamic_targets = row.get("dynamic_teacher_targets") or row.get("curriculum_teacher_targets") or {}
    for key, value in state.items():
        features[f"state::{key}"] = value
    for key, value in hand_semantics_counts(snapshot).items():
        features[f"semantics::{key}"] = float(value)
    for family, count in monster_intent_family_counts(snapshot).items():
        features[f"monster_intent::{family}"] = int(count)
    for monster_name, count in monster_name_counts(snapshot).items():
        features[f"monster_name::{monster_name}"] = int(count)
    for key, value in candidate.items():
        if key in {"move_label", "move_family", "card_name", "card_type"}:
            if value not in (None, ""):
                features[f"candidate::{key}"] = value
        elif value is not None:
            features[f"candidate::{key}"] = float(value)
    for key, value in chance.items():
        features[f"chance::{key}"] = float(value)
    for key, value in dynamic_targets.items():
        features[f"dynamic_teacher::{key}"] = float(value)
    player_hp = float(state.get("player_current_hp", 0.0))
    player_max_hp = max(float(state.get("player_max_hp", 1.0)), 1.0)
    total_monster_hp = max(float(state.get("total_monster_hp", 0.0)), 1.0)
    lowest_monster_hp = max(float(state.get("lowest_monster_hp", 0.0)), 0.0)
    incoming = float(state.get("incoming_damage", 0.0))
    block = float(state.get("player_block", 0.0))
    candidate_damage = float(candidate.get("base_damage", 0.0))
    candidate_block = float(candidate.get("base_block", 0.0))
    features["proxy::player_hp_ratio"] = player_hp / player_max_hp
    features["proxy::incoming_over_block"] = max(incoming - block, 0.0)
    features["proxy::candidate_damage_to_lowest_ratio"] = candidate_damage / max(lowest_monster_hp, 1.0) if lowest_monster_hp > 0 else 0.0
    features["proxy::candidate_damage_to_total_ratio"] = candidate_damage / total_monster_hp
    features["proxy::candidate_block_to_incoming_ratio"] = candidate_block / max(incoming, 1.0) if incoming > 0 else float(candidate.get("grants_block", 0.0))
    features["proxy::low_hp_pressure"] = 1.0 if (player_hp / player_max_hp) <= 0.35 and incoming > block else 0.0
    features["proxy::lethalish_window"] = 1.0 if lowest_monster_hp > 0 and candidate_damage >= lowest_monster_hp else 0.0
    features["proxy::setup_reachability"] = float(chance.get("payoff_reachability_after_setup_proxy", 0.0)) * float(candidate.get("setup_tag", 0.0))
    features["interaction::action_is_setup"] = float(candidate.get("setup_tag", 0.0))
    features["interaction::action_is_payoff"] = float(candidate.get("payoff_tag", 0.0))
    features["interaction::setup_payoff_overlap"] = float(candidate.get("setup_tag", 0.0)) * float(
        chance.get("payoff_reachability_after_setup_proxy", 0.0)
    )
    features["interaction::semantic_survival_alignment"] = float(
        dynamic_targets.get("survival_score", 0.0)
    ) * float(max(candidate.get("block_tag", 0.0), candidate.get("apply_weak", 0.0)))
    features["interaction::semantic_tempo_alignment"] = float(
        dynamic_targets.get("tempo_score", 0.0)
    ) * float(max(candidate.get("attack_tag", 0.0), 1.0 if float(candidate.get("draw_count", 0.0)) > 0.0 else 0.0))
    features["interaction::semantic_setup_alignment"] = float(
        dynamic_targets.get("setup_payoff_score", 0.0)
    ) * float(max(candidate.get("setup_tag", 0.0), candidate.get("consumes_status", 0.0)))
    features["interaction::semantic_kill_alignment"] = float(
        dynamic_targets.get("kill_window_score", 0.0)
    ) * float(candidate.get("attack_tag", 0.0))
    features["interaction::semantic_risk_alignment"] = float(
        dynamic_targets.get("risk_score", 0.0)
    ) * float(max(candidate.get("creates_status", 0.0), candidate.get("block_tag", 0.0)))
    features["interaction::attack_under_low_pressure"] = float(candidate.get("attack_tag", 0.0)) * float(
        1.0 if float(state.get("incoming_damage", 0.0)) <= 6.0 else 0.0
    )
    if row.get("curriculum_tag"):
        features["meta::curriculum_tag"] = str(row["curriculum_tag"])
    if row.get("sample_origin"):
        features["meta::sample_origin"] = str(row["sample_origin"])
    if row.get("teacher_source"):
        features["meta::teacher_source"] = str(row["teacher_source"])
    if row.get("eval_bucket"):
        features["meta::eval_bucket"] = str(row["eval_bucket"])
    return features


def aggregate_q_local_score(targets: dict[str, float]) -> float:
    return float(
        0.30 * float(targets.get("survival_score", 0.0))
        + 0.20 * float(targets.get("tempo_score", 0.0))
        + 0.20 * float(targets.get("setup_payoff_score", 0.0))
        + 0.20 * float(targets.get("kill_window_score", 0.0))
        - 0.15 * float(targets.get("risk_score", 0.0))
        + 0.25 * float(targets.get("mean_return", 0.0))
    )
