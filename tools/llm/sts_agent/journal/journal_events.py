"""Journal event construction and working-memory updates."""

from __future__ import annotations

import sys
import textwrap
from typing import Any

from sts_agent.evidence.tool_result_views import compact_event_action_key, compact_event_delta


def build_combat_search_event(
    record: dict[str, Any],
    report: dict[str, Any] | None,
    error: str | None,
) -> dict[str, Any]:
    public_state = record.get("public_state_before") or {}
    return {
        "schema_name": "CombatSearchEvent",
        "schema_version": 1,
        "event_type": "combat_search",
        "step": record.get("step_index"),
        "floor": public_state.get("floor"),
        "decision_type": record.get("decision_type"),
        "controller_role": "evidence_provider",
        "decision_authority": "evidence_only",
        "information_boundary": "engine_search",
        "label_role": "not_a_label",
        "trainable_as_action_label": False,
        "policy_quality_claim": False,
        "not_final_action": True,
        "status": "error" if error else "ok",
        "error": error,
        "search_report": report,
    }

def outcome_delta(public_state_before: dict[str, Any], info_after: Any) -> dict[str, Any]:
    if not isinstance(info_after, dict):
        return {}
    delta: dict[str, Any] = {
        "result": info_after.get("result"),
        "floor_after": info_after.get("floor"),
        "hp_after": info_after.get("hp"),
        "max_hp_after": info_after.get("max_hp"),
    }
    before_hp = public_state_before.get("current_hp")
    after_hp = info_after.get("hp")
    if isinstance(before_hp, int) and isinstance(after_hp, int):
        delta["hp_delta"] = after_hp - before_hp
    before_floor = public_state_before.get("floor")
    after_floor = info_after.get("floor")
    if isinstance(before_floor, int) and isinstance(after_floor, int):
        delta["floor_delta"] = after_floor - before_floor
    return delta

def build_journal_event(
    *,
    record: dict[str, Any],
    planner_payload: dict[str, Any] | None,
    tool_results: list[dict[str, Any]],
    guardrail: dict[str, Any] | None,
    risk: list[str],
) -> dict[str, Any]:
    public_state = record.get("public_state_before") or {}
    decision_route = record.get("decision_route") or {}
    decision_frame = record.get("decision_frame") or {}
    return {
        "schema_name": "DecisionJournalEvent",
        "schema_version": 1,
        "step": record.get("step_index"),
        "floor": public_state.get("floor"),
        "act": public_state.get("act"),
        "hp": public_state.get("current_hp"),
        "max_hp": public_state.get("max_hp"),
        "decision_type": record.get("decision_type"),
        "decision_class": decision_route.get("decision_class"),
        "strategic_value": decision_route.get("strategic_value"),
        "decision_frame": decision_frame,
        "authority_scope": record.get("authority_scope") or {},
        "risk_flags": risk,
        "planner_intent": (planner_payload or {}).get("intent"),
        "planner_reason": (planner_payload or {}).get("reason"),
        "planner_requests": (planner_payload or {}).get("requests") or [],
        "tool_results": tool_results,
        "executed_action_owner": record.get("executed_action_owner"),
        "guardrail": guardrail,
        "override": record.get("override") or {},
        "final_action_id": record.get("selected_action_id"),
        "final_action_key": record.get("selected_action_key"),
        "choice_was_legal": record.get("choice_was_legal"),
        "validation": record.get("validation"),
        "reward": record.get("reward"),
        "done": record.get("done"),
        "outcome_delta": outcome_delta(public_state, record.get("info")),
        "controller_role": record.get("controller_role"),
        "information_boundary": record.get("information_boundary"),
        "label_role": "not_a_label",
        "trainable_as_action_label": False,
        "policy_quality_claim": False,
    }

def journal_base_event(
    *,
    schema_name: str,
    event_type: str,
    record: dict[str, Any],
) -> dict[str, Any]:
    public_state = record.get("public_state_before") or {}
    return {
        "schema_name": schema_name,
        "schema_version": 1,
        "event_type": event_type,
        "step": record.get("step_index"),
        "floor": public_state.get("floor"),
        "label_role": "not_a_label",
        "trainable_as_action_label": False,
        "policy_quality_claim": False,
    }

def tool_result_event_summary(result: dict[str, Any]) -> dict[str, Any]:
    tool = result.get("tool")
    if tool == "combat_multi_turn_lab":
        return {
            "worldline_model": result.get("worldline_model"),
            "probability_model": result.get("probability_model"),
            "warnings_count": len(result.get("truth_warnings") or []),
            "max_rollout_steps": result.get("max_rollout_steps"),
            "hard_tactical_brief": result.get("hard_tactical_brief"),
            "sampling_exhaustiveness_check": result.get("sampling_exhaustiveness_check"),
            "delaying_action_analysis": result.get("delaying_action_analysis"),
            "potion_option_check": result.get("potion_option_check"),
            "rollout_depth_adequacy_check": result.get("rollout_depth_adequacy_check"),
            "end_turn_commitment_check": result.get("end_turn_commitment_check"),
            "branch_count": len(result.get("branches") or []),
        }
    if tool == "combat_evidence_conflict_resolver":
        return {
            "status": result.get("status"),
            "conflict_count": result.get("conflict_count"),
            "conflicts": result.get("conflicts") or [],
            "lab_reliability": result.get("lab_reliability"),
            "lab_exhaustiveness_level": result.get("lab_exhaustiveness_level"),
            "warnings": result.get("warnings") or [],
            "llm_handling_rule": result.get("llm_handling_rule"),
        }
    if tool == "deck_need_eval":
        return {
            "needs": result.get("needs") or [],
            "deck_size_estimate": result.get("deck_size_estimate"),
            "role_counts": result.get("role_counts") or {},
            "knowledge_source": result.get("knowledge_source"),
        }
    if tool == "reward_card_eval":
        return {
            "deck_needs": result.get("deck_needs") or [],
            "candidates": [
                {
                    "action_id": ((item.get("candidate") or {}).get("action_id")),
                    "action_key": compact_event_action_key(((item.get("candidate") or {}).get("action_key"))),
                    "card": ((item.get("card_fit") or {}).get("card")),
                    "fit": ((item.get("card_fit") or {}).get("fit")),
                    "score": ((item.get("card_fit") or {}).get("score")),
                }
                for item in (result.get("candidates") or [])[:4]
                if isinstance(item, dict)
            ],
        }
    if tool == "map_route_eval":
        return {
            "routes": [
                {
                    "action_id": ((item.get("candidate") or {}).get("action_id")),
                    "action_key": compact_event_action_key(((item.get("candidate") or {}).get("action_key"))),
                    "room_tags": item.get("room_tags") or [],
                    "risk_band": item.get("risk_band"),
                    "risk_score": item.get("risk_score"),
                }
                for item in (result.get("routes") or [])[:4]
                if isinstance(item, dict)
            ]
        }
    if tool == "shop_purchase_eval":
        return {
            "deck_needs": result.get("deck_needs") or [],
            "purchases": [
                {
                    "action_id": ((item.get("candidate") or {}).get("action_id")),
                    "action_key": compact_event_action_key(((item.get("candidate") or {}).get("action_key"))),
                    "card": ((item.get("card_fit") or {}).get("card")),
                    "fit": ((item.get("card_fit") or {}).get("fit")),
                    "score": ((item.get("card_fit") or {}).get("score")),
                }
                for item in (result.get("purchases") or [])[:4]
                if isinstance(item, dict)
            ],
        }
    if tool == "campfire_eval":
        return {
            "rest_priority": result.get("rest_priority"),
            "recommendation": result.get("recommendation"),
            "deck_needs": result.get("deck_needs") or [],
            "best_smith_targets": [
                {
                    "action_id": ((item.get("candidate") or {}).get("action_id")),
                    "card": ((item.get("card_fit") or {}).get("card")),
                    "fit": ((item.get("card_fit") or {}).get("fit")),
                    "score": ((item.get("card_fit") or {}).get("score")),
                }
                for item in (result.get("best_smith_targets") or [])[:5]
                if isinstance(item, dict)
            ],
        }
    if tool == "combat_turn_probe":
        plans = result.get("plans") or {}
        return {
            "plan_names": list(plans.keys()),
            "warnings_count": len(result.get("truth_warnings") or []),
            "worldline_model": result.get("worldline_model"),
        }
    if tool == "candidate_afterstate_summary":
        return {
            "worldline_model": result.get("worldline_model"),
            "warnings_count": len(result.get("truth_warnings") or []),
            "summaries": [
                {
                    "action_id": item.get("action_id"),
                    "action_key": compact_event_action_key(item.get("action_key")),
                    "kind": ((item.get("candidate") or {}).get("kind")),
                    "ok": item.get("ok"),
                    "reward": item.get("reward"),
                    "done": item.get("done"),
                    "state_delta": compact_event_delta(item.get("state_delta")),
                    "risk_flags_after": item.get("risk_flags_after") or [],
                    "next_legal_action_count": item.get("next_legal_action_count"),
                    "terminal": item.get("terminal"),
                    "error": item.get("error"),
                }
                for item in (result.get("summaries") or [])[:3]
                if isinstance(item, dict)
            ]
        }
    if tool == "decision_lab":
        return {
            "worldline_model": result.get("worldline_model"),
            "probability_model": result.get("probability_model"),
            "warnings_count": len(result.get("truth_warnings") or []),
            "max_rollout_steps": result.get("max_rollout_steps"),
            "branches": [
                {
                    "root_action_id": item.get("root_action_id"),
                    "root_action_key": compact_event_action_key(item.get("root_action_key")),
                    "kind": ((item.get("candidate") or {}).get("kind")),
                    "ok": item.get("ok"),
                    "root_state_delta": compact_event_delta(item.get("root_state_delta")),
                    "rollout_step_count": item.get("rollout_step_count"),
                    "stop_reason": item.get("stop_reason"),
                    "final_risk_flags": item.get("final_risk_flags") or [],
                    "terminal": item.get("terminal"),
                }
                for item in (result.get("branches") or [])[:3]
                if isinstance(item, dict)
            ],
        }
    if tool == "combat_multi_turn_lab":
        return {
            "worldline_model": result.get("worldline_model"),
            "probability_model": result.get("probability_model"),
            "warnings_count": len(result.get("truth_warnings") or []),
            "max_rollout_steps": result.get("max_rollout_steps"),
            "hard_tactical_brief": result.get("hard_tactical_brief"),
            "branches": [
                {
                    "root_action_id": item.get("root_action_id"),
                    "root_action_key": compact_event_action_key(item.get("root_action_key")),
                    "ok": item.get("ok"),
                    "root_state_delta": compact_event_delta(item.get("root_state_delta")),
                    "rollout_step_count": item.get("rollout_step_count"),
                    "stop_reason": item.get("stop_reason"),
                    "final_risk_flags": item.get("final_risk_flags") or [],
                    "terminal": item.get("terminal"),
                }
                for item in (result.get("branches") or [])[:3]
                if isinstance(item, dict)
            ],
        }
    if tool == "campfire_rest_smith_eval":
        return {
            "current_hp": result.get("current_hp"),
            "max_hp": result.get("max_hp"),
            "rest_legal": result.get("rest_legal"),
            "smith_count": result.get("smith_count"),
        }
    return {"reason": result.get("reason")}

def build_journal_events(
    *,
    record: dict[str, Any],
    planner_payload: dict[str, Any] | None,
    tool_results: list[dict[str, Any]],
    guardrail: dict[str, Any] | None,
    risk: list[str],
    planner_gate_reason: str | None,
    working_memory: dict[str, Any],
    recommendation_called: bool,
) -> list[dict[str, Any]]:
    public_state = record.get("public_state_before") or {}
    decision_route = record.get("decision_route") or {}
    decision_frame = record.get("decision_frame") or {}
    authority_scope = record.get("authority_scope") or {}
    events: list[dict[str, Any]] = []
    frame_event = journal_base_event(
        schema_name="DecisionFrameEvent",
        event_type="decision_frame",
        record=record,
    )
    frame_event.update(
        {
            "run_mode": record.get("run_mode"),
            "decision_type": record.get("decision_type"),
            "decision_class": decision_frame.get("decision_class"),
            "semantics": decision_frame.get("semantics") or {},
            "evidence_needs": decision_frame.get("evidence_needs") or [],
            "tool_plan": decision_frame.get("tool_plan") or {},
            "authority_scope": authority_scope,
            "budget_policy": decision_frame.get("budget_policy") or {},
            "cost_noise_hint": decision_frame.get("cost_noise_hint") or {},
        }
    )
    events.append(frame_event)
    if (
        record.get("step_index") == 0
        or record.get("decision_type") == "combat"
        or record.get("decision_type") in {"campfire", "boss_reward"}
        or {"lethal_incoming", "low_hp", "campfire_low_hp"} & set(risk)
    ):
        event = journal_base_event(
            schema_name="ObservationEvent",
            event_type="observation",
            record=record,
        )
        event.update(
            {
                "act": public_state.get("act"),
                "hp": public_state.get("current_hp"),
                "max_hp": public_state.get("max_hp"),
                "current_room": public_state.get("current_room"),
                "decision_type": record.get("decision_type"),
                "decision_class": decision_route.get("decision_class"),
                "strategic_value": decision_route.get("strategic_value"),
                "evidence_needs": decision_frame.get("evidence_needs") or [],
                "candidate_count": record.get("candidate_count"),
                "risk_flags": risk,
                "combat": public_state.get("combat") or {},
                "planner_gate_reason": planner_gate_reason,
                "memory": {
                    "current_goal": working_memory.get("current_goal"),
                    "known_risks": working_memory.get("known_risks") or [],
                },
            }
        )
        events.append(event)

    requests = (planner_payload or {}).get("requests") or []
    if requests:
        event = journal_base_event(
            schema_name="PlannerRequestEvent",
            event_type="planner_request",
            record=record,
        )
        event.update(
            {
                "intent": (planner_payload or {}).get("intent"),
                "reason": (planner_payload or {}).get("reason"),
                "requested_tools": [
                    request.get("tool")
                    for request in requests
                    if isinstance(request, dict)
                ],
                "decision_class": decision_route.get("decision_class"),
                "evidence_needs": decision_frame.get("evidence_needs") or [],
                "authority_scope": authority_scope,
                "skip_allowed": decision_route.get("skip_allowed"),
                "tool_policy": record.get("tool_policy"),
            }
        )
        events.append(event)

    for result in tool_results:
        search_fallback = bool((record.get("search_evidence") or {}).get("search_fallback"))
        if (
            result.get("source") == "search_verifier"
            and "lethal_incoming" not in set(risk)
            and not search_fallback
            and not record.get("done")
        ):
            continue
        event = journal_base_event(
            schema_name="ToolResultEvent",
            event_type="tool_result",
            record=record,
        )
        event.update(
            {
                "tool": result.get("tool"),
                "status": result.get("status"),
                "source": result.get("source") or "planner",
                "summary": tool_result_event_summary(result),
            }
        )
        events.append(event)

    if recommendation_called and record.get("provider") != "mock":
        event = journal_base_event(
            schema_name="RecommendationEvent",
            event_type="recommendation",
            record=record,
        )
        event.update(
            {
                "provider": record.get("provider"),
                "controller_role": "llm_planner_behavior",
                "recommended_action_id": (record.get("llm_choice") or {}).get("action_id"),
                "recommended_action_key": record.get("llm_recommended_action_key"),
                "reason": textwrap.shorten(
                    str((record.get("llm_choice") or {}).get("reason") or ""),
                    width=120,
                    placeholder="...",
                ),
                "decision_brief": record.get("decision_brief"),
            }
        )
        events.append(event)

    verifier = journal_base_event(
        schema_name="VerifierDecisionEvent",
        event_type="verifier_decision",
        record=record,
    )
    search_evidence_value = record.get("search_evidence") or {}
    post_info = record.get("info") if isinstance(record.get("info"), dict) else {}
    verifier.update(
        {
            "hp": public_state.get("current_hp"),
            "max_hp": public_state.get("max_hp"),
            "decision_type": record.get("decision_type"),
            "decision_class": decision_route.get("decision_class"),
            "strategic_value": decision_route.get("strategic_value"),
            "skip_allowed": decision_route.get("skip_allowed"),
            "fallback_policy": decision_route.get("fallback_policy"),
            "risk_flags": risk,
            "decision_owner": authority_scope.get("decision_owner"),
            "executed_action_owner": record.get("executed_action_owner"),
            "controller_role": record.get("controller_role"),
            "information_boundary": record.get("information_boundary"),
            "routine_reason": record.get("routine_reason"),
            "final_action_id": record.get("selected_action_id"),
            "final_action_key": record.get("selected_action_key"),
            "legal": record.get("choice_was_legal"),
            "guardrail_override": bool(guardrail),
            "guardrail": guardrail,
            "override": record.get("override") or {},
            "search_selected_plan": search_evidence_value.get("search_selected_plan"),
            "search_fallback": search_evidence_value.get("search_fallback"),
            "done": record.get("done"),
            "reward": record.get("reward"),
            "post_result": post_info.get("result"),
            "post_terminal_reason": post_info.get("terminal_reason"),
            "post_floor": post_info.get("floor"),
            "post_hp": post_info.get("hp"),
        }
    )
    events.append(verifier)

    delta = outcome_delta(public_state, record.get("info"))
    if (
        record.get("done")
        or delta.get("floor_delta") not in (None, 0)
    ):
        event = journal_base_event(
            schema_name="OutcomeEvent",
            event_type="outcome",
            record=record,
        )
        event.update(
            {
                "done": record.get("done"),
                "reward": record.get("reward"),
                "outcome_delta": delta,
            }
        )
        events.append(event)
    return events

def update_working_memory(
    working_memory: dict[str, Any],
    *,
    risk: list[str],
    tool_results: list[dict[str, Any]],
    record: dict[str, Any],
) -> None:
    known_risks = list(working_memory.get("known_risks") or [])
    for flag in risk:
        if flag not in known_risks:
            known_risks.append(flag)
    working_memory["known_risks"] = known_risks[-8:]
    if tool_results:
        findings = []
        for result in tool_results[-4:]:
            findings.append(
                {
                    "tool": result.get("tool"),
                    "status": result.get("status"),
                    "source": result.get("source") or "planner",
                }
            )
        working_memory["last_tool_findings"] = findings
    info = record.get("info")
    if isinstance(info, dict):
        result = info.get("result")
        if result:
            working_memory["last_result"] = result
            if "death" in str(result).lower():
                working_memory["last_failure_reason"] = result
        if info.get("floor") is not None:
            working_memory["last_floor"] = info.get("floor")
    if record.get("decision_type") == "map":
        working_memory["current_route_intent"] = "progress_from_map_choice"
        working_memory["plan_commitments"] = []
    commitments = [
        item for item in record.get("plan_commitments") or []
        if isinstance(item, dict)
    ]
    if commitments:
        prior = [
            item for item in working_memory.get("plan_commitments") or []
            if isinstance(item, dict)
        ]
        working_memory["plan_commitments"] = (prior + commitments)[-4:]

def should_log_decision(record: dict[str, Any], risk: list[str]) -> bool:
    if not record.get("choice_was_legal"):
        return True
    if record.get("done"):
        return True
    if record.get("guardrail"):
        return True
    if {"lethal_incoming", "low_hp", "campfire_low_hp", "unclaimed_rewards"} & set(risk):
        return True
    evidence = record.get("search_evidence") or {}
    if evidence.get("search_fallback"):
        return True
    delta = outcome_delta(record.get("public_state_before") or {}, record.get("info"))
    if delta.get("floor_delta") or delta.get("hp_delta"):
        return True
    if record.get("decision_type") in {"campfire", "reward", "card_reward", "boss_reward"}:
        return True
    return False

def log_decision(record: dict[str, Any], risk: list[str]) -> None:
    state = record.get("public_state_before") or {}
    combat = state.get("combat") or {}
    evidence = record.get("search_evidence") or {}
    plan = evidence.get("search_selected_plan") or ""
    line = (
        f"step={record.get('step_index')} "
        f"decision={record.get('decision_type')} "
        f"floor={state.get('floor')} "
        f"hp={state.get('current_hp')}/{state.get('max_hp')} "
        f"incoming={combat.get('visible_incoming_damage')} "
        f"monster_hp={combat.get('total_monster_hp')} "
        f"risk={','.join(risk) if risk else '-'} "
        f"plan={plan} "
        f"owner={record.get('executed_action_owner')} "
        f"action={record.get('selected_action_key')} "
        f"legal={record.get('choice_was_legal')} "
        f"guardrail={bool(record.get('guardrail'))} "
        f"done={record.get('done')}"
    )
    print(line, file=sys.stderr)
