"""Computed metrics for reward-card candidates.

These helpers expose narrowly scoped, source-backed calculations. They are not
policy labels and do not choose reward cards.
"""

from __future__ import annotations

from typing import Any


def n_choose_k(n: int, k: int) -> int:
    if k < 0 or k > n:
        return 0
    if k == 0 or k == n:
        return 1
    k = min(k, n - k)
    numerator = 1
    denominator = 1
    for i in range(1, k + 1):
        numerator *= n - k + i
        denominator *= i
    return numerator // denominator

def deck_card_feature(entry: dict[str, Any]) -> dict[str, Any]:
    if not isinstance(entry, dict):
        return {}
    card = entry.get("card")
    return card if isinstance(card, dict) else entry

def is_attack_feature(feature: dict[str, Any]) -> bool:
    return int(feature.get("card_type_id") or 0) == 1

def is_skill_feature(feature: dict[str, Any]) -> bool:
    return int(feature.get("card_type_id") or 0) == 2

def is_status_or_curse_feature(feature: dict[str, Any]) -> bool:
    return int(feature.get("card_type_id") or 0) in {4, 5}

def is_unplayable_blocker_feature(feature: dict[str, Any]) -> bool:
    cost = feature.get("cost")
    try:
        cost_value = int(cost)
    except Exception:
        cost_value = 0
    return is_status_or_curse_feature(feature) or cost_value < 0

def clash_activation_cost_v1(
    public_payload: dict[str, Any],
    reward_option: dict[str, Any],
) -> dict[str, Any] | None:
    card_id = str(reward_option.get("card_id") or "")
    card_name = str(reward_option.get("card_name") or "")
    if card_id != "Clash" and card_name.lower() != "clash":
        return None
    deck_entries = [
        deck_card_feature(entry)
        for entry in (public_payload.get("deck_cards") or [])
        if isinstance(entry, dict)
    ]
    deck_entries = [entry for entry in deck_entries if entry]
    companion_count = 4
    total_companion_hands = n_choose_k(len(deck_entries), companion_count)
    attack_cards = sum(1 for feature in deck_entries if is_attack_feature(feature))
    non_attack_cards = len(deck_entries) - attack_cards
    status_or_curse_cards = sum(1 for feature in deck_entries if is_status_or_curse_feature(feature))
    unplayable_blockers = sum(1 for feature in deck_entries if is_unplayable_blocker_feature(feature))
    if total_companion_hands <= 0:
        return {
            "schema_name": "ClashActivationCost",
            "schema_version": 1,
            "decision_authority": "evidence_only",
            "not_final_action": True,
            "card": "Clash",
            "status": "insufficient_deck_size",
            "truth_warnings": [
                "not_policy_label",
                "requires at least four companion cards to compute exact 5-card hand model",
            ],
        }

    clean_hands = 0
    clearable_hands = 0
    blocker_hands = 0
    over_budget_hands = 0
    clearable_energy_cost_sum = 0
    clearable_skill_count_sum = 0
    energy_budget = 3

    # Exact enumeration over companion hands conditioned on the offered Clash being drawn.
    # This is intentionally local: it does not simulate draw manipulation or tactical value.
    import itertools

    for combo in itertools.combinations(deck_entries, companion_count):
        non_attacks = [feature for feature in combo if not is_attack_feature(feature)]
        if not non_attacks:
            clean_hands += 1
            clearable_hands += 1
            continue
        has_blocker = any(is_unplayable_blocker_feature(feature) for feature in non_attacks)
        if has_blocker:
            blocker_hands += 1
            continue
        energy_cost = 0
        skill_count = 0
        for feature in non_attacks:
            try:
                energy_cost += max(0, int(feature.get("cost") or 0))
            except Exception:
                energy_cost += 0
            if is_skill_feature(feature):
                skill_count += 1
        if energy_cost <= energy_budget:
            clearable_hands += 1
            clearable_energy_cost_sum += energy_cost
            clearable_skill_count_sum += skill_count
        else:
            over_budget_hands += 1

    clearable_denominator = max(clearable_hands, 1)
    expected_energy = clearable_energy_cost_sum / clearable_denominator
    expected_skills = clearable_skill_count_sum / clearable_denominator
    nob_penalty = "high_if_gremlin_nob" if expected_skills >= 1.0 else "medium_if_gremlin_nob" if expected_skills > 0 else "none"
    return {
        "schema_name": "ClashActivationCost",
        "schema_version": 1,
        "decision_authority": "evidence_only",
        "not_final_action": True,
        "card": "Clash",
        "status": "computed",
        "computed_state": {
            "deck_size_before_pick": len(deck_entries),
            "deck_size_after_pick": len(deck_entries) + 1,
            "attack_cards_excluding_offered_clash": attack_cards,
            "non_attack_cards": non_attack_cards,
            "status_or_curse_cards": status_or_curse_cards,
            "unplayable_blocker_cards": unplayable_blockers,
            "conditioned_companion_hand_size": companion_count,
            "energy_budget_for_clearing_before_clash": energy_budget,
        },
        "metrics": {
            "clean_hand_probability": clean_hands / total_companion_hands,
            "clearable_hand_probability_under_simple_energy_model": clearable_hands / total_companion_hands,
            "unclearable_blocker_probability": blocker_hands / total_companion_hands,
            "over_energy_budget_probability": over_budget_hands / total_companion_hands,
            "expected_clear_energy_cost_given_clearable": expected_energy,
            "expected_skill_clear_count_given_clearable": expected_skills,
            "total_companion_hands_enumerated": total_companion_hands,
        },
        "method": {
            "name": "exact_companion_hand_enumeration_conditioned_on_clash_drawn",
            "formula_note": "clean_hand_probability equals C(attack_cards,4)/C(deck_size_before_pick,4)",
            "clearable_model": "non-attacks are assumed clearable if playable and total non-attack energy cost <= 3 before playing Clash",
        },
        "context_penalties": {
            "gremlin_nob_skill_punish": nob_penalty,
            "energy_tax": "high" if expected_energy >= 2.0 else "medium" if expected_energy > 0.5 else "low",
            "consistency": "conditional",
        },
        "truth_warnings": [
            "clean_hand_probability is not total Clash playability",
            "clearable model assumes non-attack cards can be played before Clash and ignores tactical opportunity cost",
            "does not simulate draw manipulation, energy changes, enemy intent, or Nob strength gain",
            "not_policy_label",
        ],
    }

def reward_candidate_metrics_v1(
    public_payload: dict[str, Any],
    reward_option: dict[str, Any],
) -> dict[str, Any]:
    computed_metrics: dict[str, Any] = {}
    clash = clash_activation_cost_v1(public_payload, reward_option)
    if clash:
        computed_metrics["clash_activation_cost"] = clash
    return {
        "schema_name": "RewardCandidateMetrics",
        "schema_version": 1,
        "decision_authority": "evidence_only",
        "not_final_action": True,
        "candidate": reward_option.get("card_name") or reward_option.get("card_id"),
        "computed_metrics": computed_metrics,
        "heuristic_fields_are_not_policy_evidence": True,
    }
