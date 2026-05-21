"""Rendering helpers for CombatSearchReport evidence.

This module formats search reports for prompts and watch output only. It does
not run search and does not choose actions.
"""

from __future__ import annotations

from typing import Any

from sts_agent.utils.llm_utils import compact_json


def compact_combat_search_report(
    report: dict[str, Any],
    *,
    mode: str,
    candidates: list[dict[str, Any]],
) -> dict[str, Any]:
    candidate_keys = {str(candidate.get("action_key")) for candidate in candidates}
    root_outcomes = []
    for root in report.get("root_action_outcomes") or []:
        if candidate_keys and str(root.get("root_action_key")) not in candidate_keys:
            continue
        vector = root.get("outcome_vector") or {}
        survival = vector.get("survival_component") or {}
        tempo = vector.get("tempo_component") or {}
        resource = vector.get("resource_component") or {}
        mechanic = vector.get("mechanic_component") or {}
        distribution = vector.get("distribution_component") or {}
        root_outcomes.append(
            {
                "root_action_id": root.get("root_action_id"),
                "root_action_key": root.get("root_action_key"),
                "search_status": root.get("search_status"),
                "survival": {
                    "fatality_rate": survival.get("fatality_rate"),
                    "survival_rate": survival.get("survival_rate"),
                    "hp_p10": survival.get("hp_p10"),
                    "hp_min": survival.get("hp_min"),
                    "block_shortfall_worst": survival.get("block_shortfall_worst"),
                },
                "tempo": {
                    "damage_done_mean": tempo.get("damage_done_mean"),
                    "enemy_hp_remaining_mean": tempo.get("enemy_hp_remaining_mean"),
                    "lethal_now": tempo.get("lethal_now"),
                },
                "resource": {
                    "potion_cost": resource.get("potion_cost"),
                    "hp_cost": resource.get("hp_cost"),
                    "energy_left_mean": resource.get("energy_left_mean"),
                },
                "mechanic_risk": mechanic,
                "distribution": {
                    "branch_count": distribution.get("branch_count"),
                    "tail_risk": distribution.get("tail_risk"),
                },
                "major_tradeoffs": (root.get("major_tradeoffs") or [])[:3],
                "risk_note_kinds": (root.get("risk_note_kinds") or [])[:4],
                "representative_branch": (root.get("representative_branch") or [])[:6],
            }
        )
    compact = {
        "schema_name": report.get("schema_name"),
        "schema_version": report.get("schema_version"),
        "information_boundary": report.get("information_boundary"),
        "decision_authority": report.get("decision_authority"),
        "not_final_action": report.get("not_final_action"),
        "source_probe_schema_version": report.get("source_probe_schema_version"),
        "search_config": report.get("search_config") or {},
        "state_summary": report.get("state_summary") or {},
        "pareto_frontier": (report.get("pareto_frontier") or [])[:8],
        "dominated_actions": (report.get("dominated_actions") or [])[:8],
        "root_action_outcomes": root_outcomes[:12],
        "failure_mode_clusters": (report.get("failure_mode_clusters") or [])[:8],
        "search_geometry": compact_search_geometry(report.get("search_geometry") or {}),
        "search_reliability": report.get("search_reliability") or {},
        "truth_warnings": (report.get("truth_warnings") or [])[:8],
    }
    if mode == "full":
        compact["full_report_available_in_record"] = True
    return compact

def compact_search_geometry(geometry: dict[str, Any]) -> dict[str, Any]:
    order = geometry.get("order_sensitivity") or {}
    clusters = geometry.get("abstract_state_clusters") or {}
    unresolved = geometry.get("unresolved_frontier") or {}
    queue = geometry.get("frontier_queue") or {}
    return {
        "schema_name": geometry.get("schema_name"),
        "budget_model": geometry.get("budget_model") or {},
        "order_sensitivity": {
            "groups_total": order.get("groups_total"),
            "sensitive_or_potentially_sensitive_groups": order.get(
                "sensitive_or_potentially_sensitive_groups"
            ),
            "items": (order.get("items") or [])[:4],
        },
        "abstract_state_clusters": {
            "clusters_total": clusters.get("clusters_total"),
            "clusters_needing_refinement": clusters.get("clusters_needing_refinement"),
            "items": (clusters.get("items") or [])[:4],
        },
        "frontier_queue": {
            "items": (queue.get("items") or [])[:6],
        },
        "unresolved_frontier": {
            "unresolved_count": unresolved.get("unresolved_count"),
            "conclusion": unresolved.get("conclusion"),
            "items": (unresolved.get("items") or [])[:6],
        },
        "belief_particle_status": geometry.get("belief_particle_status") or {},
    }

def combat_search_shadow_opinion(
    report: dict[str, Any] | None,
    candidates: list[dict[str, Any]],
) -> dict[str, Any] | None:
    if not isinstance(report, dict) or report.get("schema_name") == "CombatSearchUnavailable":
        return None
    candidate_by_key = {
        str(candidate.get("action_key")): candidate
        for candidate in candidates
        if candidate.get("action_key") is not None
    }
    frontier_keys = [
        str(item.get("root_action_key"))
        for item in (report.get("pareto_frontier") or [])
        if item.get("root_action_key") is not None
    ]
    frontier_ids = [
        candidate_by_key[key].get("id")
        for key in frontier_keys
        if key in candidate_by_key
    ]
    primary_key = frontier_keys[0] if frontier_keys else None
    primary_candidate = candidate_by_key.get(primary_key) if primary_key else None
    root_outcomes = []
    for item in (report.get("root_action_outcomes") or [])[:8]:
        vector = item.get("outcome_vector") or {}
        survival = vector.get("survival_component") or item.get("survival") or {}
        tempo = vector.get("tempo_component") or item.get("tempo") or {}
        distribution = vector.get("distribution_component") or item.get("distribution") or {}
        root_outcomes.append(
            {
                "root_action_id": item.get("root_action_id"),
                "root_action_key": item.get("root_action_key"),
                "search_status": item.get("search_status"),
                "survival_rate": survival.get("survival_rate"),
                "fatality_rate": survival.get("fatality_rate"),
                "hp_p10": survival.get("hp_p10"),
                "damage_done_mean": tempo.get("damage_done_mean"),
                "enemy_hp_remaining_mean": tempo.get("enemy_hp_remaining_mean"),
                "tail_risk": distribution.get("tail_risk"),
                "representative_branch": (item.get("representative_branch") or [])[:6],
            }
        )
    return {
        "schema_name": "CombatShadowOpinion",
        "schema_version": 1,
        "role": "shadow_opinion_not_controller",
        "decision_authority": "evidence_only",
        "not_final_action": True,
        "agreement_policy": "human_action_in_frontier_set_is_frontier_agreement; primary_frontier_mismatch_is_weak_disagreement",
        "primary_action_id": primary_candidate.get("id") if primary_candidate else None,
        "primary_action_key": primary_key,
        "frontier_action_ids": frontier_ids,
        "frontier_action_keys": frontier_keys,
        "root_outcomes": root_outcomes,
        "reliability": report.get("search_reliability") or {},
        "unresolved_frontier": ((report.get("search_geometry") or {}).get("unresolved_frontier") or {}),
    }

def combat_search_prompt_lines(report: dict[str, Any] | None) -> list[str]:
    if not report:
        return []
    if report.get("schema_name") == "CombatSearchUnavailable":
        return [
            "Combat search evidence unavailable:",
            compact_json(report, limit=1000),
        ]
    return [
        "Combat search evidence (engine_search, evidence_only, not final action):",
        compact_json(report, limit=7600),
    ]

def watch_shadow_opinion_lines(opinion: dict[str, Any] | None) -> list[str]:
    if not opinion:
        return []
    lines = [
        "search shadow: evidence_only / not controller",
        (
            "  primary_frontier="
            + f"id={opinion.get('primary_action_id')} key={opinion.get('primary_action_key')}"
        ),
    ]
    frontier_ids = opinion.get("frontier_action_ids") or []
    frontier_keys = opinion.get("frontier_action_keys") or []
    lines.append(
        "  frontier_set="
        + ", ".join(
            f"{action_id}:{str(action_key)[:80]}"
            for action_id, action_key in zip(frontier_ids, frontier_keys)
        )
    )
    unresolved = opinion.get("unresolved_frontier") or {}
    if unresolved and unresolved.get("unresolved_count") is not None:
        lines.append(
            "  unresolved="
            + f"{unresolved.get('unresolved_count')} {unresolved.get('conclusion')}"
        )
    lines.append("  recording: per-step shadow only; final comparison uses full human combat trajectory")
    return lines

def watch_search_summary_lines(report: dict[str, Any] | None) -> list[str]:
    if not report:
        return ["search: unavailable"]
    lines = [
        "search: evidence_only / not_final_action",
        "frontier:",
    ]
    for item in (report.get("pareto_frontier") or [])[:6]:
        lines.append(
            "  "
            + f"key={item.get('root_action_key')} axes={item.get('frontier_axes')} "
            + f"tradeoff={item.get('tradeoff_label')}"
        )
        if item.get("reason"):
            lines.append("    " + str(item.get("reason"))[:240])
    dominated = report.get("dominated_actions") or []
    if dominated:
        lines.append("dominated:")
        for item in dominated[:6]:
            lines.append(
                "  "
                + f"key={item.get('root_action_key')} by={item.get('dominated_by')} "
                + f"axes={item.get('dominated_axes')}"
            )
    clusters = report.get("failure_mode_clusters") or []
    if clusters:
        lines.append("failure clusters:")
        for item in clusters[:6]:
            lines.append(
                "  "
                + f"{item.get('label')} weight={item.get('probability_weight')} "
                + f"branches={item.get('branch_count')}"
            )
    geometry = report.get("search_geometry") or {}
    unresolved = geometry.get("unresolved_frontier") or {}
    if unresolved:
        if unresolved.get("unresolved_count") is not None:
            lines.append(
                "search geometry: "
                + "budget=anytime_frontier "
                + "depth=budget_guard_not_horizon "
                + f"unresolved={unresolved.get('unresolved_count')}"
            )
        for item in (unresolved.get("items") or [])[:4]:
            lines.append(
                "  unresolved "
                + f"{item.get('kind')}: {str(item.get('reason') or '')[:180]}"
            )
    order = geometry.get("order_sensitivity") or {}
    sensitive = order.get("sensitive_or_potentially_sensitive_groups")
    if sensitive:
        lines.append(f"order sensitivity: groups={sensitive}")
        for item in (order.get("items") or [])[:3]:
            lines.append(
                "  "
                + f"{item.get('status')} {str(item.get('action_multiset') or '')[:160]}"
            )
    abstract_clusters = geometry.get("abstract_state_clusters") or {}
    refine = abstract_clusters.get("clusters_needing_refinement")
    if refine:
        lines.append(f"abstract clusters needing refinement: {refine}")
    reliability = report.get("search_reliability") or {}
    if reliability:
        lines.append(
            "reliability: "
            + f"confidence={reliability.get('confidence_level')} "
            + f"budget_exhausted={reliability.get('budget_exhausted')} "
            + f"particles_evaluated={reliability.get('particles_evaluated')}"
        )
    return lines
