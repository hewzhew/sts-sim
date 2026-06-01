"""Action descriptor and candidate filtering helpers.

This module owns action-space abstraction for the controller: descriptors,
resource-action gating, and prompt summaries. It does not validate final action
legality; the driver candidate set remains authoritative.
"""

from __future__ import annotations

import json
from typing import Any

from sts_agent.utils.llm_utils import short_action_label


def build_action_descriptor_v1(
    candidate: dict[str, Any],
    public_payload: dict[str, Any],
) -> dict[str, Any]:
    payload = candidate.get("payload") or candidate
    rust_descriptor = payload.get("semantic_descriptor") or candidate.get("semantic_descriptor")
    if isinstance(rust_descriptor, dict):
        descriptor = json.loads(json.dumps(rust_descriptor))
        descriptor.setdefault("schema_name", "ActionDescriptorV1")
        descriptor.setdefault("schema_version", 1)
        descriptor.setdefault("action_id", candidate.get("id", payload.get("action_index")))
        descriptor.setdefault("action_key", candidate.get("action_key") or payload.get("action_key"))
        descriptor.setdefault("kind", candidate.get("action_kind") or (payload.get("action") or {}).get("type") or "unknown")
        descriptor.setdefault("source", "rust_runtime_semantic_descriptor")
        return descriptor
    action_key = str(candidate.get("action_key") or payload.get("action_key") or "")
    action = payload.get("action") if isinstance(payload.get("action"), dict) else {}
    card = payload.get("card") if isinstance(payload.get("card"), dict) else None
    action_id = candidate.get("id", payload.get("action_index"))
    kind = str(candidate.get("action_kind") or action.get("type") or "unknown")
    label = short_action_label(candidate)
    semantic_status = "described"
    effect_summary = None
    cost_summary = None
    risk_tags: list[str] = []
    if card:
        parts = []
        if card.get("cost") is not None:
            cost_summary = f"energy_cost={card.get('cost')}"
        if card.get("base_damage"):
            parts.append(f"damage={card.get('base_damage')}")
        if card.get("base_block"):
            parts.append(f"block={card.get('base_block')}")
        if card.get("base_magic"):
            parts.append(f"magic={card.get('base_magic')}")
        if card.get("applies_vulnerable"):
            parts.append("applies_vulnerable")
        if card.get("applies_weak"):
            parts.append("applies_weak")
        if card.get("draws_cards"):
            parts.append("draws_cards")
        if card.get("gains_energy"):
            parts.append("gains_energy")
        if card.get("exhaust"):
            parts.append("exhaust")
        effect_summary = ", ".join(parts) if parts else "card_effect_not_summarized"
    elif action_key.startswith("event/choice/"):
        index = action.get("index")
        label = f"Event choice {index}: effect_unknown"
        semantic_status = "effect_unknown"
        effect_summary = "event option text/effect is not exposed by current observation"
        risk_tags.append("under_observed_event_choice")
    elif action_key == "combat/end_turn":
        effect_summary = "end current player turn"
    elif "potion" in action_key:
        effect_summary = "potion resource action"
        risk_tags.append("resource_spend_or_discard")
    elif action_key == "reward/proceed":
        effect_summary = "leave reward screen / continue"
    elif action_key.startswith("map/"):
        effect_summary = "choose map path node"
    elif action_key.startswith("shop/"):
        effect_summary = "shop action"
    elif action_key.startswith("campfire/"):
        effect_summary = "campfire action"
    else:
        semantic_status = "partially_described"
        effect_summary = "effect inferred only from action_key"
    return {
        "schema_name": "ActionDescriptorV1",
        "schema_version": 1,
        "action_id": action_id,
        "action_key": action_key,
        "kind": kind,
        "label": label,
        "semantic_status": semantic_status,
        "effect_summary": effect_summary,
        "cost_summary": cost_summary,
        "risk_tags": risk_tags,
        "source": "python_harness_action_key_and_payload",
    }

def candidate_with_descriptor(
    candidate: dict[str, Any],
    public_payload: dict[str, Any],
) -> dict[str, Any]:
    enriched = json.loads(json.dumps(candidate))
    enriched["action_descriptor"] = build_action_descriptor_v1(enriched, public_payload)
    return enriched

def combat_has_useful_playable_card(public_payload: dict[str, Any]) -> bool:
    combat = public_payload.get("combat")
    if not isinstance(combat, dict):
        return False
    energy = combat.get("energy") or 0
    if energy <= 0:
        return False
    for card in combat.get("hand_cards") or []:
        if not isinstance(card, dict) or not card.get("playable"):
            continue
        if card.get("cost_for_turn") is not None and card.get("cost_for_turn") > energy:
            continue
        tags = set(card.get("base_semantics") or [])
        card_id = str(card.get("card_id") or "")
        if tags.intersection({"damage", "block", "attack", "skill", "power", "draw", "scaling"}):
            return True
        if card_id and card_id not in {"Wound", "Burn", "Dazed", "Slimed", "Void"}:
            return True
    return False

def action_key_is_potion_resource(action_key: str) -> bool:
    lowered = action_key.lower()
    return "potion" in lowered

def action_key_is_discard_potion(action_key: str) -> bool:
    lowered = action_key.lower()
    return "discard_potion" in lowered or "discard/potion" in lowered

def potion_unlock_reasons(public_payload: dict[str, Any]) -> list[str]:
    combat = public_payload.get("combat")
    if not isinstance(combat, dict):
        return []
    reasons = []
    room = str(public_payload.get("current_room") or "")
    hp = combat.get("player_hp") or public_payload.get("current_hp") or 0
    block = combat.get("player_block") or 0
    incoming = combat.get("visible_incoming_damage") or 0
    monster_hp = combat.get("total_monster_hp") or 999
    if max(0, incoming - block) >= hp and hp > 0:
        reasons.append("death_risk")
    if hp > 0 and max(0, incoming - block) >= max(10, hp // 2):
        reasons.append("high_incoming_damage")
    if "Elite" in room or "Boss" in room:
        reasons.append("elite_or_boss")
    if monster_hp <= 20:
        reasons.append("possible_lethal_window")
    return reasons

def build_action_candidate_policy_v1(
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
) -> dict[str, Any]:
    enriched = [candidate_with_descriptor(candidate, public_payload) for candidate in candidates]
    decision_candidates = []
    locked_actions = []
    discouraged_actions = []
    potion_reasons = potion_unlock_reasons(public_payload)
    useful_playable = combat_has_useful_playable_card(public_payload)
    combat = public_payload.get("combat") if isinstance(public_payload.get("combat"), dict) else {}
    incoming = (combat or {}).get("visible_incoming_damage") or 0
    for candidate in enriched:
        action_key = str(candidate.get("action_key") or "")
        descriptor = candidate.get("action_descriptor") or {}
        if action_key_is_discard_potion(action_key):
            locked_actions.append(
                {
                    "action": candidate,
                    "reason": "destructive_resource_action_requires_explicit_need",
                    "unlock_conditions": [
                        "potion_slot_pressure",
                        "explicit_replacement_decision",
                        "human_override",
                    ],
                }
            )
            continue
        if action_key_is_potion_resource(action_key) and not potion_reasons:
            locked_actions.append(
                {
                    "action": candidate,
                    "reason": "routine_resource_preservation",
                    "unlock_conditions": [
                        "death_risk",
                        "high_incoming_damage",
                        "elite_or_boss",
                        "possible_lethal_window",
                        "explicit_planner_resource_request",
                    ],
                }
            )
            continue
        if action_key == "combat/end_turn" and useful_playable:
            reason = "useful_playable_cards_available"
            if incoming > 0:
                reason += "_and_incoming_damage_present"
            discouraged_actions.append(
                {
                    "action": candidate,
                    "reason": reason,
                    "severity": "dangerous" if incoming > 0 else "discouraged",
                }
            )
            continue
        if descriptor.get("semantic_status") == "effect_unknown":
            candidate["descriptor_warning"] = "under_observed_action_effect"
        decision_candidates.append(candidate)
    if not decision_candidates:
        decision_candidates = [
            item["action"]
            for item in discouraged_actions
            if isinstance(item.get("action"), dict)
        ] or enriched[:1]
    return {
        "schema_name": "ActionCandidatePolicyV1",
        "schema_version": 1,
        "legal_action_count": len(enriched),
        "decision_candidate_count": len(decision_candidates),
        "locked_action_count": len(locked_actions),
        "discouraged_action_count": len(discouraged_actions),
        "decision_candidates": decision_candidates,
        "locked_actions": locked_actions,
        "dominated_or_discouraged_actions": discouraged_actions,
        "filter_reliability": "heuristic_v1",
        "truth_warnings": [
            "candidate_filter_is_action_space_abstraction_not_legality",
            "harness_validation_still_uses_all_legal_actions",
            "event_choice_effects_may_be_unknown_until_event_observation_exposes_text",
        ],
    }

def action_candidate_policy_prompt_lines(policy: dict[str, Any] | None) -> list[str]:
    if not policy:
        return []
    lines = [
        "Action candidate policy:",
        (
            f"decision={policy.get('decision_candidate_count')} "
            f"locked={policy.get('locked_action_count')} "
            f"discouraged={policy.get('discouraged_action_count')} "
            f"legal_total={policy.get('legal_action_count')}"
        ),
    ]
    locked = policy.get("locked_actions") or []
    if locked:
        lines.append("Locked legal actions summary:")
        for item in locked[:6]:
            action = item.get("action") or {}
            descriptor = action.get("action_descriptor") or {}
            lines.append(
                "  "
                + f"id={action.get('id')} {descriptor.get('label')} "
                + f"reason={item.get('reason')}"
            )
    discouraged = policy.get("dominated_or_discouraged_actions") or []
    if discouraged:
        lines.append("Discouraged legal actions summary:")
        for item in discouraged[:6]:
            action = item.get("action") or {}
            descriptor = action.get("action_descriptor") or {}
            lines.append(
                "  "
                + f"id={action.get('id')} {descriptor.get('label')} "
                + f"reason={item.get('reason')}"
            )
    return lines
