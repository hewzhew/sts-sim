"""Semantic export coverage accounting for run summaries."""

from __future__ import annotations

from typing import Any

from sts_agent.evidence.reward_metrics import reward_candidate_metrics_v1


def new_semantic_coverage_stats() -> dict[str, int]:
    return {
        "action_descriptor_total": 0,
        "action_descriptor_from_rust": 0,
        "action_descriptor_python_fallback": 0,
        "descriptors_known": 0,
        "descriptors_partial": 0,
        "descriptors_unknown": 0,
        "descriptors_effect_unknown": 0,
        "descriptors_missing": 0,
        "event_options_total": 0,
        "event_options_known": 0,
        "event_options_partial": 0,
        "event_options_unknown": 0,
        "under_observed_event_choice_count": 0,
        "map_route_context_present_steps": 0,
        "route_choices_total": 0,
        "route_choices_with_boss_reachable": 0,
        "reward_card_choice_context_present_steps": 0,
        "reward_card_choices_total": 0,
        "reward_card_choices_with_rust_descriptor": 0,
        "reward_card_choices_unknown": 0,
        "reward_card_choices_with_computed_metrics": 0,
        "clash_activation_cost_evaluations": 0,
    }

def merge_semantic_coverage_step(
    stats: dict[str, int],
    public_payload: dict[str, Any],
    action_candidate_policy: dict[str, Any],
) -> None:
    seen: set[Any] = set()

    def update_descriptor(descriptor: dict[str, Any] | None) -> None:
        stats["action_descriptor_total"] += 1
        if not isinstance(descriptor, dict) or not descriptor:
            stats["descriptors_missing"] += 1
            return
        source_chain = descriptor.get("source_chain") or []
        source = descriptor.get("source")
        if source_chain or source == "rust_runtime_semantic_descriptor":
            stats["action_descriptor_from_rust"] += 1
        else:
            stats["action_descriptor_python_fallback"] += 1
        status = str(descriptor.get("semantic_status") or "unknown")
        if status == "known":
            stats["descriptors_known"] += 1
        elif status == "partial":
            stats["descriptors_partial"] += 1
        elif status == "effect_unknown":
            stats["descriptors_effect_unknown"] += 1
            stats["under_observed_event_choice_count"] += 1
        else:
            stats["descriptors_unknown"] += 1
        risk_tags = set(descriptor.get("risk_tags") or [])
        if "under_observed_event_choice" in risk_tags:
            stats["under_observed_event_choice_count"] += 1

    def visit_candidate(candidate: dict[str, Any]) -> None:
        if not isinstance(candidate, dict):
            return
        key = candidate.get("id", candidate.get("action_key"))
        if key in seen:
            return
        seen.add(key)
        update_descriptor(candidate.get("action_descriptor"))

    for candidate in action_candidate_policy.get("decision_candidates") or []:
        visit_candidate(candidate)
    for item in action_candidate_policy.get("locked_actions") or []:
        visit_candidate((item or {}).get("action") or {})
    for item in action_candidate_policy.get("dominated_or_discouraged_actions") or []:
        visit_candidate((item or {}).get("action") or {})

    screen = public_payload.get("screen") if isinstance(public_payload.get("screen"), dict) else {}
    for option in (screen or {}).get("event_options") or []:
        if not isinstance(option, dict):
            continue
        stats["event_options_total"] += 1
        descriptor = option.get("semantic_descriptor") or {}
        status = str(descriptor.get("semantic_status") or "unknown")
        if status == "known":
            stats["event_options_known"] += 1
        elif status == "partial":
            stats["event_options_partial"] += 1
        else:
            stats["event_options_unknown"] += 1
    reward_card_choices = [
        option
        for option in (screen or {}).get("reward_card_choices") or []
        if isinstance(option, dict)
    ]
    if reward_card_choices:
        stats["reward_card_choice_context_present_steps"] += 1
    for option in reward_card_choices:
        stats["reward_card_choices_total"] += 1
        descriptor = option.get("semantic_descriptor") or {}
        source_chain = descriptor.get("source_chain") or []
        if source_chain:
            stats["reward_card_choices_with_rust_descriptor"] += 1
        status = str(descriptor.get("semantic_status") or "unknown")
        if status not in {"known", "partial"}:
            stats["reward_card_choices_unknown"] += 1
        metrics = reward_candidate_metrics_v1(public_payload, option)
        computed_metrics = metrics.get("computed_metrics") or {}
        if computed_metrics:
            stats["reward_card_choices_with_computed_metrics"] += 1
        if computed_metrics.get("clash_activation_cost"):
            stats["clash_activation_cost_evaluations"] += 1
    map_route_context = public_payload.get("map_route_context")
    if isinstance(map_route_context, dict):
        route_choices = [
            choice
            for choice in map_route_context.get("route_choices") or []
            if isinstance(choice, dict)
        ]
        if route_choices:
            stats["map_route_context_present_steps"] += 1
        stats["route_choices_total"] += len(route_choices)
        stats["route_choices_with_boss_reachable"] += sum(
            1
            for choice in route_choices
            if int(choice.get("reachable_paths_to_boss") or 0) > 0
        )
