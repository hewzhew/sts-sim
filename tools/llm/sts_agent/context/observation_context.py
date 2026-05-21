"""Public observation shaping and context-ablation helpers."""

from __future__ import annotations

import json
from typing import Any

from sts_agent.utils.llm_utils import compact_json, map_room_label, map_route_context_lines
from sts_agent.evidence.reward_metrics import reward_candidate_metrics_v1


def text_contains_any(text: str, needles: list[str]) -> bool:
    lowered = text.lower()
    return any(needle.lower() in lowered for needle in needles)

def observation_summary(observation: dict[str, Any]) -> list[str]:
    payload = observation.get("payload") or observation
    lines = [
        f"decision_type={payload.get('decision_type')} engine_state={payload.get('engine_state')}",
        (
            f"act={payload.get('act')} floor={payload.get('floor')} "
            f"hp={payload.get('current_hp')}/{payload.get('max_hp')} "
            f"gold={payload.get('gold')} deck_size={payload.get('deck_size')} "
            f"boss={payload.get('act_boss')}"
        ),
    ]
    deck = payload.get("deck") or {}
    if deck:
        lines.append(
            "deck="
            + ", ".join(
                f"{key}:{deck.get(key)}"
                for key in [
                    "attack_count",
                    "skill_count",
                    "power_count",
                    "draw_card_count",
                    "scaling_card_count",
                    "starter_basic_count",
                    "average_cost_milli",
                ]
            )
        )
    combat = payload.get("combat")
    if combat:
        combat_context = combat.get("combat_context") or build_combat_context_v1(
            payload,
            "mechanics_plus_strategy_hints",
        )
        if combat_context:
            lines.append("combat_context=" + compact_json(combat_context, limit=1800))
        monsters = (
            (combat_context or {}).get("monsters")
            if isinstance(combat_context, dict)
            else combat.get("monsters")
        ) or []
        monster_parts = []
        for monster in monsters[:4]:
            powers = monster.get("powers") or []
            power_text = ",".join(
                f"{power.get('power_id')}:{power.get('amount')}"
                for power in powers[:4]
                if isinstance(power, dict)
            )
            monster_parts.append(
                f"{monster.get('name') or monster.get('monster_id')}"
                f" hp:{monster.get('hp', monster.get('current_hp'))}/{monster.get('max_hp')}"
                f" block:{monster.get('block')}"
                f" move:{monster.get('planned_move_id')}"
                f" intent:{monster.get('visible_intent')}"
                + (f" powers:{power_text}" if power_text else "")
            )
        lines.append(
            (
                "combat="
                f"hp:{combat.get('player_hp')} block:{combat.get('player_block')} "
                f"energy:{combat.get('energy')} turn:{combat.get('turn_count')} "
                f"incoming:{combat.get('visible_incoming_damage')} "
                f"monster_hp_total:{combat.get('total_monster_hp')} "
                f"alive_monsters:{combat.get('alive_monster_count')} "
                f"draw:{combat.get('draw_count')} discard:{combat.get('discard_count')}"
            )
        )
        player_powers = combat.get("player_powers") or []
        if player_powers:
            lines.append(
                "player_powers="
                + ", ".join(
                    f"{power.get('power_id')}:{power.get('amount')}"
                    for power in player_powers[:6]
                    if isinstance(power, dict)
                )
            )
        if monster_parts:
            lines.append("monsters=" + " | ".join(monster_parts))
        encounter_hints = (
            (combat_context or {}).get("encounter_hints")
            if isinstance(combat_context, dict)
            else combat.get("encounter_hints")
        ) or []
        if encounter_hints:
            lines.append("encounter_hints=" + " ; ".join(str(hint) for hint in encounter_hints[:6]))
        hand = combat.get("hand_cards") or []
        if hand:
            lines.append("hand:")
            for card in hand[:12]:
                tags = ",".join(card.get("base_semantics") or [])
                lines.append(
                    "  "
                    f"h{card.get('hand_index')} {card.get('card_id')}"
                    f"{'+' if card.get('upgraded') else ''} "
                    f"cost={card.get('cost_for_turn')} playable={card.get('playable')} "
                    f"tags={tags}"
                )
    screen = payload.get("screen") or {}
    if screen:
        active_counts = {
            key: value
            for key, value in screen.items()
            if isinstance(value, int) and value > 0
        }
        if active_counts:
            lines.append(f"screen_counts={active_counts}")
        event_options = screen.get("event_options") or []
        if event_options:
            lines.append("event_options:")
            for option in event_options[:8]:
                if not isinstance(option, dict):
                    continue
                descriptor = option.get("semantic_descriptor") or {}
                label = descriptor.get("label") or option.get("label")
                status = descriptor.get("semantic_status")
                costs = descriptor.get("costs") or []
                effects = descriptor.get("effects") or []
                suffix = f" status={status}" if status else ""
                lines.append(
                    "  "
                    + f"{option.get('option_index')} {label}{suffix} "
                    + f"costs={compact_json(costs, limit=160)} "
                    + f"effects={compact_json(effects, limit=220)}"
                )
    rewards = screen.get("reward_items") or []
    if rewards:
        lines.append("reward_items:")
        for item in rewards[:8]:
            lines.append("  " + compact_json(item, limit=300))
    reward_card_choices = screen.get("reward_card_choices") or []
    if reward_card_choices:
        lines.append("reward_card_choices:")
        for option in reward_card_choices[:8]:
            if not isinstance(option, dict):
                continue
            descriptor = option.get("semantic_descriptor") or {}
            label = descriptor.get("label") or option.get("card_name") or option.get("card_id")
            status = descriptor.get("semantic_status")
            hints = ",".join(str(tag) for tag in (option.get("base_semantics") or [])[:8])
            metrics = reward_candidate_metrics_v1(payload, option)
            clash_metrics = (metrics.get("computed_metrics") or {}).get("clash_activation_cost")
            suffix = f" status={status}" if status else ""
            metrics_text = ""
            if clash_metrics:
                metric_values = clash_metrics.get("metrics") or {}
                metrics_text = (
                    " clash_activation="
                    + compact_json(
                        {
                            "clean_p": round(float(metric_values.get("clean_hand_probability") or 0), 4),
                            "clearable_p": round(float(metric_values.get("clearable_hand_probability_under_simple_energy_model") or 0), 4),
                            "expected_energy": round(float(metric_values.get("expected_clear_energy_cost_given_clearable") or 0), 2),
                            "expected_skills": round(float(metric_values.get("expected_skill_clear_count_given_clearable") or 0), 2),
                            "nob_penalty": (clash_metrics.get("context_penalties") or {}).get("gremlin_nob_skill_punish"),
                        },
                        limit=240,
                    )
                )
            lines.append(
                "  "
                + f"{option.get('option_index')} {label}{suffix} "
                + f"type={option.get('card_type')} rarity={option.get('rarity')} "
                + f"cost={option.get('cost')} copies={option.get('deck_copies')} "
                + f"hints={hints}{metrics_text}"
            )
        lines.append("  note: hints/plan_delta are handwritten heuristic overlay, not policy evidence")
    route_lines = map_route_context_lines(payload.get("map_route_context"))
    if route_lines:
        lines.extend(route_lines)
    elif payload.get("next_nodes"):
        lines.append("map_next_choices:")
        for node in (payload.get("next_nodes") or [])[:8]:
            label = map_room_label(node.get("room_type"), bool(node.get("has_emerald_key")))
            lines.append(
                "  "
                + f"x={node.get('x')} y={node.get('y')} {label}"
            )
    return lines

def public_observation_payload(timestep: dict[str, Any]) -> dict[str, Any]:
    observation = timestep.get("observation") or {}
    if not isinstance(observation, dict):
        return {}
    payload = observation.get("payload")
    if isinstance(payload, dict):
        return payload
    return observation

def is_strategy_hint(hint: Any) -> bool:
    text = str(hint or "").lower()
    return (
        text.startswith("strategy:")
        or "_plan" in text
        or "prioritize" in text
        or "deck needs" in text
        or "only block when" in text
    )

def hints_for_context_level(hints: list[Any], context_level: str) -> list[str]:
    if context_level in {"none", "identity_only"}:
        return []
    cleaned = [str(hint) for hint in hints if hint]
    if context_level == "mechanics":
        return [hint for hint in cleaned if not is_strategy_hint(hint)]
    return cleaned

def build_combat_context_v1(payload: dict[str, Any], context_level: str) -> dict[str, Any] | None:
    combat = payload.get("combat")
    if not isinstance(combat, dict):
        return None
    raw_monsters = combat.get("monsters") or []
    room_type = payload.get("current_room")
    visible_monsters = []
    include_identity = context_level in {
        "identity_only",
        "mechanics",
        "mechanics_plus_strategy_hints",
    }
    for monster in raw_monsters:
        if not isinstance(monster, dict):
            continue
        if not include_identity:
            continue
        monster_hints = hints_for_context_level(
            monster.get("mechanic_hints") or [],
            context_level,
        )
        visible_monsters.append(
            {
                "monster_id": monster.get("monster_id"),
                "name": monster.get("name"),
                "hp": monster.get("current_hp"),
                "max_hp": monster.get("max_hp"),
                "block": monster.get("block"),
                "powers": monster.get("powers") or [],
                "planned_move_id": monster.get("planned_move_id"),
                "visible_intent": monster.get("visible_intent"),
                "visible_intent_kind": monster.get("visible_intent_kind"),
                "visible_intent_damage_per_hit": monster.get(
                    "visible_intent_damage_per_hit"
                ),
                "visible_intent_hits": monster.get("visible_intent_hits"),
                "visible_intent_total_damage": monster.get("visible_intent_total_damage"),
                "mechanic_hints": monster_hints,
            }
        )
    encounter_id = None
    if include_identity and visible_monsters:
        encounter_id = "+".join(
            str(monster.get("monster_id") or monster.get("name") or "unknown")
            for monster in visible_monsters
        )
    encounter_hints = hints_for_context_level(
        combat.get("encounter_hints") or [],
        context_level,
    )
    hint_source = "none"
    if encounter_hints or any(monster.get("mechanic_hints") for monster in visible_monsters):
        hint_source = (
            "simulator_curated_mechanics_and_strategy"
            if context_level == "mechanics_plus_strategy_hints"
            else "simulator_curated_mechanics"
        )
    return {
        "schema_name": "CombatContextV1",
        "schema_version": 1,
        "context_level": context_level,
        "encounter_id": encounter_id,
        "room_type": room_type,
        "act": payload.get("act"),
        "floor": payload.get("floor"),
        "monsters": visible_monsters,
        "encounter_hints": encounter_hints,
        "hint_source": hint_source,
    }

def apply_context_ablation_to_payload(payload: dict[str, Any], context_level: str) -> dict[str, Any]:
    if context_level == "mechanics_plus_strategy_hints":
        # Still rebuild context so the formal CombatContextV1 contract is present.
        copied = json.loads(json.dumps(payload))
    else:
        copied = json.loads(json.dumps(payload))
    combat = copied.get("combat")
    if not isinstance(combat, dict):
        return copied
    combat_context = build_combat_context_v1(copied, context_level)
    if context_level == "none":
        combat.pop("monsters", None)
        combat.pop("encounter_hints", None)
        combat.pop("player_powers", None)
    elif context_level == "identity_only":
        for monster in combat.get("monsters") or []:
            if isinstance(monster, dict):
                monster["mechanic_hints"] = []
        combat["encounter_hints"] = []
    elif context_level == "mechanics":
        for monster in combat.get("monsters") or []:
            if isinstance(monster, dict):
                monster["mechanic_hints"] = hints_for_context_level(
                    monster.get("mechanic_hints") or [],
                    context_level,
                )
        combat["encounter_hints"] = hints_for_context_level(
            combat.get("encounter_hints") or [],
            context_level,
        )
    combat["combat_context"] = combat_context
    return copied

def apply_context_ablation_to_timestep(timestep: dict[str, Any], context_level: str) -> dict[str, Any]:
    copied = json.loads(json.dumps(timestep))
    observation = copied.get("observation")
    if not isinstance(observation, dict):
        return copied
    payload = observation.get("payload")
    if isinstance(payload, dict):
        observation["payload"] = apply_context_ablation_to_payload(payload, context_level)
    else:
        copied["observation"] = apply_context_ablation_to_payload(observation, context_level)
    return copied

def public_state_snapshot(payload: dict[str, Any]) -> dict[str, Any]:
    snapshot = {
        "decision_type": payload.get("decision_type"),
        "engine_state": payload.get("engine_state"),
        "act": payload.get("act"),
        "floor": payload.get("floor"),
        "current_room": payload.get("current_room"),
        "current_hp": payload.get("current_hp"),
        "max_hp": payload.get("max_hp"),
        "gold": payload.get("gold"),
        "deck_size": payload.get("deck_size"),
        "act_boss": payload.get("act_boss"),
    }
    combat = payload.get("combat")
    if isinstance(combat, dict):
        snapshot["combat"] = {
            "player_hp": combat.get("player_hp"),
            "player_block": combat.get("player_block"),
            "energy": combat.get("energy"),
            "turn_count": combat.get("turn_count"),
            "player_powers": combat.get("player_powers") or [],
            "visible_incoming_damage": combat.get("visible_incoming_damage"),
            "total_monster_hp": combat.get("total_monster_hp"),
            "alive_monster_count": combat.get("alive_monster_count"),
            "monsters": combat.get("monsters") or [],
            "encounter_hints": combat.get("encounter_hints") or [],
            "combat_context": combat.get("combat_context"),
            "draw_count": combat.get("draw_count"),
            "discard_count": combat.get("discard_count"),
        }
    screen = payload.get("screen")
    if isinstance(screen, dict):
        screen_snapshot: dict[str, Any] = {}
        event_options = screen.get("event_options") or []
        if event_options:
            screen_snapshot["event_option_count"] = screen.get("event_option_count")
            screen_snapshot["event_options"] = [
                {
                    "option_index": option.get("option_index"),
                    "label": (
                        (option.get("semantic_descriptor") or {}).get("label")
                        or option.get("label")
                    ),
                    "semantic_status": (
                        option.get("semantic_descriptor") or {}
                    ).get("semantic_status"),
                    "coverage_level": (
                        option.get("semantic_descriptor") or {}
                    ).get("coverage_level"),
                    "unknown_fields": (
                        option.get("semantic_descriptor") or {}
                    ).get("unknown_fields")
                    or [],
                }
                for option in event_options[:12]
                if isinstance(option, dict)
            ]
        reward_card_choices = screen.get("reward_card_choices") or []
        if reward_card_choices:
            screen_snapshot["reward_phase"] = screen.get("reward_phase")
            screen_snapshot["reward_card_choice_count"] = screen.get("reward_card_choice_count")
            card_choice_snapshots = []
            for option in reward_card_choices[:12]:
                if not isinstance(option, dict):
                    continue
                metrics = reward_candidate_metrics_v1(payload, option)
                card_choice_snapshots.append(
                    {
                        "option_index": option.get("option_index"),
                        "card_name": option.get("card_name"),
                        "card_id": option.get("card_id"),
                        "card_type": option.get("card_type"),
                        "rarity": option.get("rarity"),
                        "cost": option.get("cost"),
                        "low_level_card_hints": option.get("base_semantics") or [],
                        "deck_copies": option.get("deck_copies"),
                        "heuristic_plan_delta": option.get("plan_delta"),
                        "heuristic_plan_delta_source": "handwritten_heuristic_overlay",
                        "heuristic_plan_delta_is_fact": False,
                        "reward_candidate_metrics": metrics,
                        "label": (
                            (option.get("semantic_descriptor") or {}).get("label")
                            or option.get("card_name")
                        ),
                        "semantic_status": (
                            option.get("semantic_descriptor") or {}
                        ).get("semantic_status"),
                    }
                )
            screen_snapshot["reward_card_choices"] = card_choice_snapshots
        if any(value not in (None, [], {}) for value in screen_snapshot.values()):
            snapshot["screen"] = screen_snapshot
    map_route_context = payload.get("map_route_context")
    if isinstance(map_route_context, dict):
        snapshot["map_route_context"] = {
            "schema_name": map_route_context.get("schema_name"),
            "schema_version": map_route_context.get("schema_version"),
            "decision_authority": map_route_context.get("decision_authority"),
            "not_final_action": map_route_context.get("not_final_action"),
            "map_scope": map_route_context.get("map_scope"),
            "context_level": map_route_context.get("context_level"),
            "act_boss": map_route_context.get("act_boss"),
            "truth_warnings": map_route_context.get("truth_warnings") or [],
            "route_choices": [
                {
                    "action_key": choice.get("action_key"),
                    "next_x": choice.get("next_x"),
                    "next_y": choice.get("next_y"),
                    "room_label": choice.get("room_label"),
                    "reachable_paths_to_boss": choice.get("reachable_paths_to_boss"),
                    "min_elites": choice.get("min_elites"),
                    "max_elites": choice.get("max_elites"),
                    "min_fires": choice.get("min_fires"),
                    "max_fires": choice.get("max_fires"),
                    "min_shops": choice.get("min_shops"),
                    "max_shops": choice.get("max_shops"),
                    "forced_fights_next_3": choice.get("forced_fights_next_3"),
                    "earliest_shop_floor": choice.get("earliest_shop_floor"),
                    "earliest_fire_floor": choice.get("earliest_fire_floor"),
                    "local_flex": choice.get("local_flex"),
                    "global_path_flex": choice.get("global_path_flex"),
                    "path_flexibility": choice.get("path_flexibility"),
                    "risk_label": choice.get("risk_label"),
                    "risk_vector": choice.get("risk_vector"),
                    "notes": choice.get("notes") or [],
                }
                for choice in (map_route_context.get("route_choices") or [])[:12]
                if isinstance(choice, dict)
            ],
        }
    return {key: value for key, value in snapshot.items() if value is not None}
