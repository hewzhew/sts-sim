"""Decision classification, authority scope, and tool routing policy."""

from __future__ import annotations

import argparse
from typing import Any

from sts_agent.context.observation_context import public_observation_payload
from sts_agent.context.tool_request_defaults import mock_planner_requests


def risk_flags(payload: dict[str, Any]) -> list[str]:
    flags: list[str] = []
    decision_type = payload.get("decision_type")
    current_hp = payload.get("current_hp")
    max_hp = payload.get("max_hp")
    if isinstance(current_hp, int) and isinstance(max_hp, int) and max_hp > 0:
        if current_hp * 100 <= max_hp * 35:
            flags.append("low_hp")
        if decision_type == "campfire" and current_hp * 100 <= max_hp * 50:
            flags.append("campfire_low_hp")
    combat = payload.get("combat")
    if isinstance(combat, dict):
        hp = combat.get("player_hp")
        block = combat.get("player_block") or 0
        incoming = combat.get("visible_incoming_damage") or 0
        monster_hp = combat.get("total_monster_hp") or 0
        if incoming > 0:
            flags.append("incoming_damage")
        if isinstance(hp, int) and max(0, incoming - block) >= hp > 0:
            flags.append("lethal_incoming")
        if monster_hp <= 15:
            flags.append("possible_lethal_window")
    screen = payload.get("screen")
    if isinstance(screen, dict) and screen.get("reward_claimable_item_count", 0) > 0:
        flags.append("unclaimed_rewards")
    return flags

def should_request_tools(
    timestep: dict[str, Any],
    args: argparse.Namespace,
    risk: list[str],
) -> tuple[bool, str]:
    if args.tool_policy == "none":
        return False, "tool_policy_none"
    if args.tool_policy == "always":
        return True, "tool_policy_always"

    payload = public_observation_payload(timestep)
    decision_type = payload.get("decision_type")
    candidates = timestep.get("candidates") or []
    candidate_count = len(candidates)
    risk_set = set(risk)

    if "lethal_incoming" in risk_set:
        return True, "risk_gate_lethal_incoming"
    if "campfire_low_hp" in risk_set:
        return True, "risk_gate_campfire_low_hp"
    if "low_hp" in risk_set and decision_type in {"combat", "event", "campfire"}:
        return True, "risk_gate_low_hp"
    if "possible_lethal_window" in risk_set and decision_type == "combat":
        return True, "risk_gate_possible_lethal_window"

    if decision_type == "combat":
        combat = payload.get("combat") if isinstance(payload.get("combat"), dict) else {}
        incoming = combat.get("visible_incoming_damage") or 0
        block = combat.get("player_block") or 0
        hp = combat.get("player_hp")
        unblocked = max(0, incoming - block)
        if isinstance(hp, int) and hp > 0 and unblocked * 100 >= hp * 35:
            return True, "risk_gate_large_unblocked_damage"
        return False, "risk_gate_routine_combat"

    if decision_type == "campfire":
        return True, f"risk_gate_{decision_type}"
    if decision_type in {"reward_card_choice", "card_reward", "boss_reward"} and candidate_count >= 2:
        return True, f"risk_gate_{decision_type}"
    if candidate_count >= 4 and decision_type in {"map", "event", "shop", "chest"}:
        return True, "risk_gate_multi_candidate_noncombat"
    return False, "risk_gate_low_value_decision"

def non_potion_action_keys(candidates: list[dict[str, Any]]) -> list[str]:
    keys = []
    for candidate in candidates:
        key = str(candidate.get("action_key") or "")
        if key and not key.startswith(("potion/", "discard_potion/")):
            keys.append(key)
    return keys

def has_action_prefix(candidates: list[dict[str, Any]], *prefixes: str) -> bool:
    return any(
        key.startswith(prefix)
        for key in non_potion_action_keys(candidates)
        for prefix in prefixes
    )

def classify_decision(
    timestep: dict[str, Any],
    risk: list[str],
    *,
    use_search: bool,
) -> str:
    payload = public_observation_payload(timestep)
    decision_type = str(payload.get("decision_type") or "unknown")
    candidates = timestep.get("candidates") or []
    action_keys = non_potion_action_keys(candidates)
    risk_set = set(risk)

    if decision_type == "combat":
        return "combat_tactical"
    if decision_type == "campfire":
        return "campfire_strategy"
    if decision_type in {"reward_card_choice", "card_reward", "boss_reward"}:
        return "reward_strategy"
    if decision_type == "shop":
        if has_action_prefix(candidates, "shop/buy_", "shop/remove", "shop/purge"):
            return "shop_strategy"
        return "mechanical_progression"
    if decision_type == "map":
        if sum(1 for key in action_keys if key.startswith("map/select")) >= 2:
            return "map_route_strategy"
        return "mechanical_progression"
    if decision_type == "event":
        if len(action_keys) >= 2 or risk_set:
            return "event_uncertain"
        return "mechanical_progression"
    if decision_type in {"treasure", "run_deck_selection", "proceed", "none"}:
        return "mechanical_progression"
    if risk_set:
        return "unknown_high_risk"
    return "mechanical_progression"

def evidence_needs_for_class(decision_class: str) -> list[str]:
    return {
        "mechanical_progression": ["legal_action_id"],
        "combat_tactical": [
            "current_hp_block_energy",
            "visible_incoming_damage",
            "legal_combat_actions",
            "current_turn_tactical_lines",
        ],
        "reward_strategy": [
            "current_deck_needs",
            "card_offer_tradeoffs",
            "boss_or_elite_pressure",
        ],
        "shop_strategy": [
            "current_deck_needs",
            "gold_constraints",
            "remove_value",
            "shop_offer_tradeoffs",
            "next_route_risk",
        ],
        "campfire_strategy": [
            "current_hp",
            "rest_value",
            "smith_value",
            "next_route_risk",
            "boss_or_elite_pressure",
        ],
        "map_route_strategy": [
            "future_route_risk",
            "route_reward_opportunities",
            "current_hp",
            "deck_readiness",
        ],
        "event_uncertain": [
            "event_option_afterstates",
            "hp_gold_card_tradeoffs",
            "long_term_risk_notes",
        ],
        "unknown_high_risk": [
            "candidate_afterstates",
            "risk_flags",
        ],
    }.get(decision_class, ["legal_action_id", "candidate_afterstates"])

def combat_search_execution_enabled(args: argparse.Namespace, decision_type: Any) -> bool:
    return decision_type == "combat" and args.run_mode == "search_final_baseline"

def route_decision(
    timestep: dict[str, Any],
    args: argparse.Namespace,
    risk: list[str],
    *,
    use_search: bool,
) -> dict[str, Any]:
    should_tools, cost_hint = should_request_tools(timestep, args, risk)
    payload = public_observation_payload(timestep)
    decision_type = payload.get("decision_type")
    decision_class = classify_decision(timestep, risk, use_search=use_search)
    risk_set = set(risk)

    routes: dict[str, dict[str, Any]] = {
        "mechanical_progression": {
            "strategic_value": "low",
            "risk_level": "low",
            "planner_allowed": False,
            "required_tools": [],
            "optional_tools": [],
            "fallback_policy": "first_structural_action",
            "skip_allowed": True,
            "budget_pool": "mechanical",
            "llm_role": "none",
        },
        "combat_tactical": {
            "strategic_value": "tactical_high",
            "risk_level": "high" if risk_set else "medium",
            "planner_allowed": not use_search,
            "required_tools": ["combat_turn_probe"],
            "optional_tools": ["combat_multi_turn_lab"],
            "fallback_policy": "search_verifier_or_first_legal",
            "skip_allowed": bool(use_search),
            "budget_pool": "combat_tactical",
            "llm_role": "none_when_search_owns_final_action" if use_search else "interpret_combat_evidence",
        },
        "reward_strategy": {
            "strategic_value": "strategic_high",
            "risk_level": "medium",
            "planner_allowed": True,
            "required_tools": ["reward_card_eval"],
            "optional_tools": ["candidate_afterstate_summary", "deck_need_eval"],
            "fallback_policy": "skip_or_best_fit_if_uncertain",
            "skip_allowed": False,
            "budget_pool": "reward_strategy",
            "llm_role": "compare_deck_needs_and_card_tradeoffs",
        },
        "shop_strategy": {
            "strategic_value": "strategic_high",
            "risk_level": "high" if "low_hp" in risk_set else "medium",
            "planner_allowed": True,
            "required_tools": ["shop_purchase_eval"],
            "optional_tools": ["candidate_afterstate_summary", "deck_need_eval", "map_route_eval"],
            "fallback_policy": "do_not_buy_if_uncertain",
            "skip_allowed": False,
            "budget_pool": "shop_strategy",
            "llm_role": "compare_gold_deck_needs_and_route_risk",
        },
        "campfire_strategy": {
            "strategic_value": "strategic_high",
            "risk_level": "high" if {"low_hp", "campfire_low_hp"} & risk_set else "medium",
            "planner_allowed": True,
            "required_tools": ["campfire_eval"],
            "optional_tools": ["candidate_afterstate_summary", "deck_need_eval"],
            "fallback_policy": "prefer_rest_when_survival_risk_is_high",
            "skip_allowed": False,
            "budget_pool": "campfire_strategy",
            "llm_role": "compare_rest_smith_survival_tradeoff",
        },
        "map_route_strategy": {
            "strategic_value": "strategic_high",
            "risk_level": "medium",
            "planner_allowed": True,
            "required_tools": ["map_route_eval"],
            "optional_tools": ["deck_need_eval"],
            "fallback_policy": "prefer_lower_route_risk_if_uncertain",
            "skip_allowed": False,
            "budget_pool": "map_route_strategy",
            "llm_role": "compare_route_risk_and_reward",
        },
        "event_uncertain": {
            "strategic_value": "situational_high",
            "risk_level": "high" if risk_set else "medium",
            "planner_allowed": True,
            "required_tools": ["candidate_afterstate_summary"],
            "optional_tools": ["decision_lab"],
            "fallback_policy": "avoid_hp_loss_if_uncertain",
            "skip_allowed": False,
            "budget_pool": "event_uncertain",
            "llm_role": "compare_event_outcomes_and_hidden_risk",
        },
        "unknown_high_risk": {
            "strategic_value": "unknown_high",
            "risk_level": "high",
            "planner_allowed": True,
            "required_tools": ["candidate_afterstate_summary"],
            "optional_tools": [],
            "fallback_policy": "conservative_first_structural_action",
            "skip_allowed": False,
            "budget_pool": "unknown_high_risk",
            "llm_role": "explain_uncertain_high_risk_decision",
        },
    }
    route = dict(routes[decision_class])
    route.update(
        {
            "schema_name": "DecisionRoute",
            "schema_version": 1,
            "decision_type": decision_type,
            "decision_class": decision_class,
            "risk_flags": risk,
            "required_evidence": evidence_needs_for_class(decision_class),
            "tool_plan": {
                "required_tools": route.get("required_tools") or [],
                "optional_tools": route.get("optional_tools") or [],
            },
            "cost_hint_should_request_tools": should_tools,
            "cost_hint_reason": cost_hint,
            "information_boundary": "engine_search" if use_search else "public_observation_only",
            "label_role": "not_a_label",
            "trainable_as_action_label": False,
            "policy_quality_claim": False,
        }
    )
    return route

def authority_scope_for_route(
    route: dict[str, Any],
    args: argparse.Namespace,
    *,
    use_search: bool,
) -> dict[str, Any]:
    decision_class = str(route.get("decision_class") or "unknown")
    evidence_tools = list((route.get("tool_plan") or {}).get("required_tools") or [])
    evidence_tools.extend((route.get("tool_plan") or {}).get("optional_tools") or [])
    if args.run_mode == "search_final_baseline":
        decision_owner = "search_controller" if use_search else "routine_policy"
        override_policy = "not_applicable_baseline_owner"
        audit_policy = "llm_comparison_optional_not_controller_claim"
    elif args.run_mode == "llm_shadow_audit":
        decision_owner = "baseline_controller"
        override_policy = "not_applicable_shadow_mode"
        audit_policy = "llm_recommendation_compared_not_executed"
    else:
        decision_owner = "deterministic_harness" if decision_class == "mechanical_progression" else "llm_controller"
        override_policy = (
            "combat_tactical_fatality_only"
            if args.run_mode == "llm_live_with_tactical_safety"
            else "no_override"
        )
        audit_policy = "baseline_search_comparison_if_available"
    return {
        "schema_name": "AuthorityScope",
        "schema_version": 1,
        "run_mode": args.run_mode,
        "decision_owner": decision_owner,
        "evidence_providers": evidence_tools,
        "guardrails": ["schema_valid_json", "legal_action_id", "simulator_can_step"],
        "override_policy": override_policy,
        "override_allowed_domains": ["combat_tactical"]
        if override_policy == "combat_tactical_fatality_only"
        else [],
        "override_allowed_reasons": [
            "immediate_death_avoidance",
            "confirmed_immediate_lethal",
        ]
        if override_policy == "combat_tactical_fatality_only"
        else [],
        "audit_policy": audit_policy,
        "baseline_role": "auditor" if args.run_mode.startswith("llm_live") else "execution_owner_or_comparison",
        "tool_role_default": "evidence_provider",
        "silent_override_allowed": False,
    }

def decision_frame_for_route(
    route: dict[str, Any],
    authority_scope: dict[str, Any],
) -> dict[str, Any]:
    return {
        "schema_name": "DecisionFrame",
        "schema_version": 1,
        "decision_class": route.get("decision_class"),
        "semantics": {
            "strategic_value": route.get("strategic_value"),
            "risk_level": route.get("risk_level"),
            "skip_allowed": route.get("skip_allowed"),
            "fallback_policy": route.get("fallback_policy"),
        },
        "evidence_needs": route.get("required_evidence") or [],
        "tool_plan": route.get("tool_plan") or {},
        "authority_scope": authority_scope,
        "budget_policy": {
            "budget_pool": route.get("budget_pool"),
            "skip_allowed": route.get("skip_allowed"),
            "on_budget_exhausted": "mark_degraded_not_silent_skip"
            if not route.get("skip_allowed")
            else "allow_skip_with_note",
        },
        "cost_noise_hint": {
            "old_gate_should_request_tools": route.get("cost_hint_should_request_tools"),
            "old_gate_reason": route.get("cost_hint_reason"),
        },
        "label_role": "not_a_label",
        "trainable_as_action_label": False,
        "policy_quality_claim": False,
    }

def route_tool_requests(
    timestep: dict[str, Any],
    args: argparse.Namespace,
    route: dict[str, Any],
) -> list[dict[str, Any]]:
    if not route.get("planner_allowed"):
        return []
    required = [str(tool) for tool in route.get("required_tools") or []]
    optional = [str(tool) for tool in route.get("optional_tools") or []]
    planned = mock_planner_requests(timestep, args).get("requests") or []
    by_tool = {
        str(request.get("tool")): request
        for request in planned
        if isinstance(request, dict) and request.get("tool")
    }
    requests: list[dict[str, Any]] = []
    for tool in required + optional:
        if tool in by_tool and len(requests) < args.max_tool_requests:
            request = dict(by_tool[tool])
            request["source"] = "decision_route"
            request["question"] = request.get("question") or f"Provide required evidence for {route.get('decision_class')}."
            requests.append(request)
    return requests

def route_budget_cap(route: dict[str, Any], args: argparse.Namespace) -> int:
    pool = str(route.get("budget_pool") or "unknown")
    caps = {
        "mechanical": 0,
        "combat_tactical": 0,
        "reward_strategy": 12,
        "shop_strategy": 6,
        "campfire_strategy": 6,
        "map_route_strategy": 8,
        "event_uncertain": 8,
        "unknown_high_risk": 4,
    }
    return caps.get(pool, max(1, args.planner_max_requests // 4))

def route_budget_allows(
    route: dict[str, Any],
    counts: dict[str, int],
    args: argparse.Namespace,
) -> tuple[bool, str | None]:
    if not route.get("planner_allowed"):
        return False, "route_planner_not_allowed"
    pool = str(route.get("budget_pool") or "unknown")
    used = counts.get(pool, 0)
    cap = route_budget_cap(route, args)
    if used < cap:
        return True, None
    if not route.get("skip_allowed"):
        return True, f"budget_soft_cap_overridden:{pool}:{used}/{cap}"
    return False, f"budget_exhausted:{pool}:{used}/{cap}"

def available_tool_specs(decision_type: str | None) -> list[dict[str, Any]]:
    specs = []
    if decision_type in {"reward_card_choice", "shop", "campfire", "map", "boss_relic"}:
        specs.append(
            {
                "tool": "deck_need_eval",
                "description": "Summarize current deck role gaps using public card/relic observations.",
            }
        )
    if decision_type == "combat":
        specs.append(
            {
                "tool": "combat_turn_probe",
                "description": "Search current-turn combat sequences for lethal, block, and damage plans.",
            }
        )
        specs.append(
            {
                "tool": "combat_multi_turn_lab",
                "description": "Branch-test selected combat root actions, then let search/verifier auto-advance a bounded number of combat decisions. Experimental evidence only; not stochastic worldline enumeration.",
            }
        )
    if decision_type == "campfire":
        specs.append(
            {
                "tool": "campfire_eval",
                "description": "Compare rest priority and smith target fit against deck needs.",
            }
        )
        specs.append(
            {
                "tool": "campfire_rest_smith_eval",
                "description": "Raw rest/smith availability for debugging the semantic campfire eval.",
            }
        )
    if decision_type == "reward_card_choice":
        specs.append(
            {
                "tool": "reward_card_eval",
                "description": "Match visible reward card candidates against deck needs and card role tags.",
            }
        )
    if decision_type == "map":
        specs.append(
            {
                "tool": "map_route_eval",
                "description": "Summarize visible map route risk tags and low-HP route risk.",
            }
        )
    if decision_type == "shop":
        specs.append(
            {
                "tool": "shop_purchase_eval",
                "description": "Match visible shop/purge candidates against deck needs.",
            }
        )
    specs.append(
        {
            "tool": "candidate_afterstate_summary",
            "description": "Summarize one-step simulator outcomes for selected current legal action ids.",
        }
    )
    specs.append(
        {
            "tool": "decision_lab",
            "description": "Run bounded branch experiments: execute selected legal action ids, then auto-advance a few low-risk/search-verifiable steps. Evidence only; not a true future oracle.",
        }
    )
    return specs


