#!/usr/bin/env python3
from __future__ import annotations

from typing import Any

import numpy as np

from card_semantics import card_semantic_vector, card_semantics

MAX_RANKER_CANDIDATES = 32

ACTION_CLASS_IDS = {
    "other": 0,
    "end_turn": 1,
    "damage": 2,
    "mitigation": 3,
    "setup": 4,
    "draw": 5,
    "potion": 6,
    "choice": 7,
}
ACTION_CLASS_COUNT = len(ACTION_CLASS_IDS)

FAMILY_IDS = {
    "unknown": 0,
    "end_turn": 1,
    "play_card": 2,
    "use_potion": 3,
    "discovery_select": 4,
    "card_reward_select": 5,
    "stance_choice": 6,
    "scry_select": 7,
    "card_select": 8,
    "hand_select": 9,
    "grid_select": 10,
    "proceed": 11,
    "cancel": 12,
}
FAMILY_COUNT = len(FAMILY_IDS)

CANDIDATE_FEATURE_DIM = FAMILY_COUNT + ACTION_CLASS_COUNT + 27 + 10


def candidate_action_class(candidate: dict[str, Any]) -> str:
    family = str(candidate.get("action_family") or "")
    if family == "end_turn":
        return "end_turn"
    if family == "use_potion":
        return "potion"
    if family != "play_card":
        if family in {
            "discovery_select",
            "card_reward_select",
            "stance_choice",
            "scry_select",
            "card_select",
            "hand_select",
            "grid_select",
            "proceed",
            "cancel",
        }:
            return "choice"
        return "other"

    semantics = card_semantics(
        candidate.get("card_name") or candidate.get("card_id"),
        cost_for_turn=candidate.get("cost_for_turn"),
        playable=True,
    )
    if (
        float(semantics.get("apply_strength") or 0.0) < 0.0
        or float(semantics.get("apply_weak") or 0.0) > 0.0
        or float(semantics.get("apply_frail") or 0.0) > 0.0
    ):
        return "mitigation"
    if float(semantics.get("block_tag") or 0.0) > 0.0 and float(semantics.get("base_damage") or 0.0) <= 0.0:
        return "mitigation"
    if float(semantics.get("setup_tag") or 0.0) > 0.0:
        return "setup"
    if float(semantics.get("draw_count") or 0.0) > 0.0 and float(semantics.get("base_damage") or 0.0) <= 0.0:
        return "draw"
    if float(semantics.get("attack_tag") or 0.0) > 0.0 or float(semantics.get("deals_damage") or 0.0) > 0.0:
        return "damage"
    if float(semantics.get("draw_count") or 0.0) > 0.0:
        return "draw"
    return "other"


def _one_hot(index: int, count: int) -> list[float]:
    values = [0.0] * count
    if 0 <= index < count:
        values[index] = 1.0
    return values


def _target_monster(raw_observation: dict[str, Any], candidate: dict[str, Any]) -> dict[str, Any] | None:
    target_slot = candidate.get("target_slot")
    if target_slot is None:
        return None
    monsters = list(raw_observation.get("monsters") or [])
    slot = int(target_slot)
    if slot < 0 or slot >= len(monsters):
        return None
    return monsters[slot]


def candidate_feature_vector(raw_observation: dict[str, Any], candidate: dict[str, Any]) -> list[float]:
    family = str(candidate.get("action_family") or "unknown")
    family_id = FAMILY_IDS.get(family, 0)
    action_class = candidate_action_class(candidate)
    class_id = ACTION_CLASS_IDS.get(action_class, 0)
    semantics = card_semantic_vector(
        candidate.get("card_name") or candidate.get("card_id"),
        cost_for_turn=candidate.get("cost_for_turn"),
        playable=True,
    )
    target = _target_monster(raw_observation, candidate)
    intent = (target or {}).get("intent_payload") or {}
    pressure = raw_observation.get("pressure") or {}
    scalars = [
        float(candidate.get("slot_index") if candidate.get("slot_index") is not None else -1.0) / 10.0,
        float(candidate.get("target_slot") if candidate.get("target_slot") is not None else -1.0) / 5.0,
        float(raw_observation.get("energy") or 0.0) / 10.0,
        float(pressure.get("visible_unblocked") or 0.0) / 50.0,
        float((target or {}).get("current_hp") or 0.0) / 200.0,
        float((target or {}).get("current_hp") or 0.0) / max(float((target or {}).get("max_hp") or 1.0), 1.0),
        float((target or {}).get("block") or 0.0) / 100.0,
        float(intent.get("total_damage") or 0.0) / 50.0,
        1.0 if target is not None else 0.0,
        1.0 if candidate.get("target_slot") is None else 0.0,
    ]
    features = _one_hot(family_id, FAMILY_COUNT) + _one_hot(class_id, ACTION_CLASS_COUNT) + semantics + scalars
    if len(features) != CANDIDATE_FEATURE_DIM:
        raise RuntimeError(f"candidate feature dim mismatch: {len(features)} != {CANDIDATE_FEATURE_DIM}")
    return features


def candidate_action_array(action: dict[str, int]) -> list[int]:
    return [
        int(action.get("action_type") or 0),
        int(action.get("card_slot") or 0),
        int(action.get("target_slot") or 0),
        int(action.get("potion_slot") or 0),
        int(action.get("choice_index") or 0),
    ]


def empty_candidate_arrays() -> dict[str, np.ndarray]:
    return {
        "candidate_features": np.zeros((MAX_RANKER_CANDIDATES, CANDIDATE_FEATURE_DIM), dtype=np.float32),
        "candidate_mask": np.zeros((MAX_RANKER_CANDIDATES,), dtype=np.float32),
        "candidate_scores": np.full((MAX_RANKER_CANDIDATES,), -1e9, dtype=np.float32),
        "candidate_survival_rank": np.zeros((MAX_RANKER_CANDIDATES,), dtype=np.float32),
        "candidate_class": np.zeros((MAX_RANKER_CANDIDATES,), dtype=np.int64),
        "candidate_actions": np.zeros((MAX_RANKER_CANDIDATES, 5), dtype=np.int64),
        "best_mask": np.zeros((MAX_RANKER_CANDIDATES,), dtype=np.float32),
        "best_index": np.asarray(-1, dtype=np.int64),
        "best_class": np.asarray(0, dtype=np.int64),
        "best_class_mask": np.zeros((ACTION_CLASS_COUNT,), dtype=np.float32),
    }
