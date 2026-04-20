#!/usr/bin/env python3
from __future__ import annotations

from typing import Any

from combat_rl_common import hand_semantics_counts, monster_intent_family_counts, snapshot_state_features
from q_local_common import aggregate_q_local_score, candidate_semantics_from_move, chance_feature_dict

STATUS_TOKENS = ("Slimed", "Wound", "Dazed", "Burn", "Void", "Curse")
TEACHER_SOURCE = "combat_lab_curriculum_dynamic_semantics"


def _clamp01(value: float) -> float:
    return float(max(0.0, min(1.0, value)))


def _normalized_search_scores(candidates: list[dict[str, Any]]) -> dict[str, float]:
    if not candidates:
        return {}
    values = [float(candidate.get("score") or 0.0) for candidate in candidates]
    low = min(values)
    high = max(values)
    if abs(high - low) <= 1e-6:
        return {str(candidate.get("move_label") or ""): 0.5 for candidate in candidates}
    return {
        str(candidate.get("move_label") or ""): float((float(candidate.get("score") or 0.0) - low) / (high - low))
        for candidate in candidates
    }


def _teacher_targets(
    *,
    move_label: str,
    snapshot: dict[str, Any],
    state: dict[str, Any],
    hand_counts: dict[str, float],
    intent_counts: dict[str, int],
    chance: dict[str, float],
    semantics: dict[str, Any],
    search_score_norm: float,
    non_end_turn_exists: bool,
    preview: dict[str, Any],
    full: dict[str, Any],
) -> dict[str, float]:
    player_hp = max(float(state.get("player_current_hp") or 0.0), 1.0)
    player_max_hp = max(float(state.get("player_max_hp") or 0.0), 1.0)
    hp_ratio = player_hp / player_max_hp
    incoming = max(float(state.get("incoming_damage") or 0.0), float(preview.get("incoming_damage") or 0.0))
    visible_unblocked = max(
        float(full.get("unblocked_incoming") or 0.0),
        max(incoming - float(state.get("player_block") or 0.0), 0.0),
    )
    pressure = _clamp01(max(visible_unblocked, incoming * 0.55) / max(player_hp * 0.35, 8.0))
    low_pressure = 1.0 - pressure
    attackers = max(float(intent_counts.get("attack", 0)), 1.0 if incoming > 0.0 else 0.0)
    lowest_monster_hp = max(float(state.get("lowest_monster_hp") or 0.0), 1.0)
    total_monster_hp = max(float(state.get("total_monster_hp") or 0.0), 1.0)
    hand_count = max(float(state.get("hand_count") or 0.0), 1.0)
    hand_status_count = float(hand_counts.get("status_cards_in_hand_count", 0.0))
    payoff_support = _clamp01(
        max(
            float(chance.get("payoff_reachability_after_setup_proxy", 0.0)),
            float(hand_counts.get("hand_payoff_count", 0.0)) / max(hand_count, 1.0),
            0.6 * float(chance.get("attack_density_next_window", 0.0)),
        )
    )
    status_support = _clamp01(
        min(hand_status_count / 2.0, 1.0) * max(float(semantics.get("consumes_status", 0.0)), 0.5 * float(semantics.get("status_tag", 0.0)))
        + 0.5 * float(chance.get("draw_chain_start_prob", 0.0))
    )
    block_need = max(visible_unblocked, incoming * 0.6)
    block_value = (
        _clamp01(float(semantics.get("base_block", 0.0)) / max(block_need, 5.0)) * pressure
        if block_need > 0.0
        else 0.0
    )
    weak_relief = _clamp01((float(semantics.get("apply_weak", 0.0)) * attackers) / 3.0) * pressure
    kill_ratio = _clamp01(float(semantics.get("base_damage", 0.0)) / lowest_monster_hp) * float(semantics.get("attack_tag", 0.0))
    damage_ratio_total = _clamp01(float(semantics.get("base_damage", 0.0)) / max(total_monster_hp * 0.35, 6.0))
    draw_value = _clamp01(float(semantics.get("draw_count", 0.0)) / 2.0)
    setup_window = _clamp01(
        float(semantics.get("setup_tag", 0.0))
        * max(
            payoff_support,
            0.5 * float(chance.get("draw_chain_start_prob", 0.0)),
            0.5 * float(chance.get("attack_density_next_window", 0.0)),
        )
    )
    consumes_status_value = _clamp01(float(semantics.get("consumes_status", 0.0)) * min(hand_status_count / 2.0, 1.0))
    sleep_pressure = 1.0 if int(intent_counts.get("sleep", 0)) > 0 else 0.0
    status_creation_risk = _clamp01(float(semantics.get("creates_status", 0.0)) / 2.0) * low_pressure * (1.0 - max(status_support, payoff_support))
    overshield_risk = (
        _clamp01((float(semantics.get("base_block", 0.0)) - max(incoming, 1.0) * 1.25) / 15.0)
        * low_pressure
        * float(semantics.get("block_tag", 0.0))
    )
    end_turn_risk = 1.0 if move_label == "EndTurn" and non_end_turn_exists else 0.0
    setup_pressure_risk = float(semantics.get("setup_tag", 0.0)) * pressure * (1.0 - max(payoff_support, hp_ratio * 0.5))
    status_play_risk = (
        1.0
        if any(token in move_label for token in STATUS_TOKENS) and non_end_turn_exists
        else 0.0
    )
    survival_score = _clamp01(
        0.70 * block_value
        + 0.25 * weak_relief
        + 0.20 * kill_ratio * pressure
        + 0.10 * draw_value * pressure
    )
    tempo_score = _clamp01(
        0.55 * damage_ratio_total
        + 0.25 * draw_value * (0.5 + float(chance.get("draw_chain_start_prob", 0.0)))
        + 0.22
        * _clamp01(
            (float(semantics.get("apply_strength", 0.0)) + float(semantics.get("apply_vulnerable", 0.0)) + float(semantics.get("apply_weak", 0.0)))
            / 3.0
        )
        * max(float(chance.get("attack_density_next_window", 0.0)), 0.35)
        + 0.12 * float(semantics.get("attack_tag", 0.0)) * low_pressure
        + 0.10 * float(semantics.get("payoff_tag", 0.0)) * low_pressure
        + 0.08 * sleep_pressure * float(semantics.get("attack_tag", 0.0))
        + 0.10 * search_score_norm
    )
    setup_payoff_score = _clamp01(
        0.75 * setup_window
        + 0.45 * consumes_status_value
        + 0.25 * float(semantics.get("status_tag", 0.0)) * min(hand_status_count / 3.0, 1.0)
        + 0.20 * float(semantics.get("card_draw_engine", 0.0)) * float(chance.get("draw_chain_start_prob", 0.0))
    )
    kill_window_score = _clamp01(
        0.72 * kill_ratio
        + 0.25 * _clamp01(float(semantics.get("apply_vulnerable", 0.0)) / 2.0) * max(float(chance.get("attack_density_next_window", 0.0)), 0.5)
        + 0.12 * search_score_norm
    )
    risk_score = _clamp01(
        0.45 * status_creation_risk
        + 0.25 * overshield_risk
        + 0.55 * end_turn_risk
        + 0.28 * setup_pressure_risk
        + 0.35 * status_play_risk
    )
    return {
        "survival_score": survival_score,
        "tempo_score": tempo_score,
        "setup_payoff_score": setup_payoff_score,
        "kill_window_score": kill_window_score,
        "risk_score": risk_score,
    }


def dynamic_teacher_for_row(row: dict[str, Any]) -> dict[str, Any]:
    candidates = list(row.get("normalized_candidates") or [])
    if len(candidates) < 2:
        return {
            "active": False,
            "teacher_source": TEACHER_SOURCE,
            "label_strength": "filtered_low_weight",
            "preferred_moves": [],
            "oracle_best_move": None,
            "oracle_margin": 0.0,
            "candidate_details": [],
        }
    snapshot = row.get("snapshot_normalized_state") or {}
    preview = row.get("state_features_preview") or {}
    full = row.get("state_features_full") or {}
    state = snapshot_state_features(snapshot)
    hand_counts = hand_semantics_counts(snapshot)
    intent_counts = monster_intent_family_counts(snapshot)
    search_score_norm = _normalized_search_scores(candidates)
    non_end_turn_exists = any(str(candidate.get("move_label") or "") != "EndTurn" for candidate in candidates)

    details: list[dict[str, Any]] = []
    for candidate in candidates:
        move_label = str(candidate.get("move_label") or "")
        semantics = candidate_semantics_from_move(move_label)
        chance = chance_feature_dict(snapshot, move_label)
        targets = _teacher_targets(
            move_label=move_label,
            snapshot=snapshot,
            state=state,
            hand_counts=hand_counts,
            intent_counts=intent_counts,
            chance=chance,
            semantics=semantics,
            search_score_norm=search_score_norm.get(move_label, 0.5),
            non_end_turn_exists=non_end_turn_exists,
            preview=preview,
            full=full,
        )
        teacher_score = aggregate_q_local_score(targets) + 0.05 * search_score_norm.get(move_label, 0.5)
        details.append(
            {
                "move_label": move_label,
                "candidate_semantics": semantics,
                "chance_features": chance,
                "teacher_targets": targets,
                "teacher_score": float(teacher_score),
                "search_score_norm": float(search_score_norm.get(move_label, 0.5)),
            }
        )

    details.sort(key=lambda item: (float(item["teacher_score"]), float(item["search_score_norm"])), reverse=True)
    chosen_move = str(row.get("chosen_move") or "")
    chosen_detail = next((detail for detail in details if detail["move_label"] == chosen_move), None)
    best_score = float(details[0]["teacher_score"])
    worst_score = float(details[-1]["teacher_score"])
    spread = max(best_score - worst_score, 0.0)
    tie_tolerance = max(0.04, min(0.12, 0.10 * spread if spread > 0.0 else 0.04))
    preferred_moves = [
        str(detail["move_label"])
        for detail in details
        if best_score - float(detail["teacher_score"]) <= tie_tolerance
    ]
    chosen_score = float(chosen_detail["teacher_score"]) if chosen_detail is not None else float("-inf")
    margin = best_score - chosen_score if chosen_detail is not None else best_score
    active = bool(preferred_moves) and chosen_move not in set(preferred_moves) and margin >= max(0.12, tie_tolerance)
    label_strength = "oracle_strong" if margin >= max(0.30, tie_tolerance * 3.0) else "oracle_preference"
    return {
        "active": active,
        "teacher_source": TEACHER_SOURCE,
        "label_strength": label_strength,
        "preferred_moves": preferred_moves,
        "oracle_best_move": preferred_moves[0] if preferred_moves else None,
        "oracle_margin": float(round(margin, 4)),
        "chosen_teacher_score": None if chosen_detail is None else float(round(chosen_score, 4)),
        "best_teacher_score": float(round(best_score, 4)),
        "tie_tolerance": float(round(tie_tolerance, 4)),
        "candidate_details": details,
    }
