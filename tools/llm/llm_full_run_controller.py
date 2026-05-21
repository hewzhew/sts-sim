#!/usr/bin/env python3
"""LLM controller harness for the full-run DecisionEnv driver.

This script is intentionally a controller adapter, not a teacher-label or
training pipeline. It reads public observations and legal candidates from
`full_run_env_driver`, asks an LLM (or a mock policy) to choose one candidate id,
validates the choice against the current candidate set, then steps the driver.

Provider mode `openai_compatible` works with APIs that expose the common
`/chat/completions` shape. Configure it with:

  LLM_API_KEY      required
  LLM_BASE_URL     default: https://api.openai.com/v1
  LLM_MODEL        default: gpt-4o-mini

For DeepSeek-style endpoints, set:

  LLM_BASE_URL=https://api.deepseek.com
  LLM_MODEL=deepseek-chat

Use `--provider dry_run` to inspect the prompt without calling any model.
Use `--provider mock` for a local smoke test of the adapter loop.
"""

from __future__ import annotations

import argparse
import json
import os
import random
import re
import sys
import textwrap
import time
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
CASES_DIR = REPO_ROOT / "tools" / "artifacts" / "cases"
from sts_agent.evidence.combat_lab_views import compact_combat_multi_turn_lab_result
from sts_agent.evidence.tool_result_views import compact_combat_probe_result
from sts_agent.runtime.action_selection import (
    choose_search_action,
    first_structural_action_id,
    guardrail_action_id,
    is_routine_mechanical_single_action,
    routine_action_id,
    validate_action_id,
)
from sts_agent.runtime.driver_requests import (
    request_candidate_afterstate_summary,
    request_campfire_rest_smith_eval,
    request_combat_multi_turn_lab,
    request_combat_plan_probe,
    request_combat_search_engine,
    request_decision_lab_probe,
)
from sts_agent.runtime.env_driver import DriverClient, default_driver_path
from sts_agent.ui.watch_ui import (
    save_decision_case_v1,
    watch_before_llm_interactively,
    watch_decision_interactively,
)
from sts_agent.context.observation_context import (
    apply_context_ablation_to_payload,
    apply_context_ablation_to_timestep,
    build_combat_context_v1,
    observation_summary,
    public_observation_payload,
    public_state_snapshot,
)
from sts_agent.runtime.tool_design_runtime import (
    build_tool_design_events,
    default_tool_design_out_path,
    should_observe_tool_design,
    should_run_combat_lab_observer,
    tool_design_phase,
    tool_design_type_budget,
)
from sts_agent.runtime.planner_runtime import (
    default_combat_lab_action_ids,
    default_tool_action_ids,
    execute_planner_requests,
    planner_recommendation,
    planner_request_phase,
)
from sts_agent.runtime.prompt_builders import (
    build_planner_prompt,
    build_prompt,
    build_recommendation_prompt,
    build_tool_design_prompt,
)
from sts_agent.context.routing_policy import (
    authority_scope_for_route,
    available_tool_specs,
    combat_search_execution_enabled,
    decision_frame_for_route,
    route_budget_allows,
    route_budget_cap,
    route_decision,
    route_tool_requests,
    risk_flags,
    should_request_tools,
)
from sts_agent.context.semantic_coverage import merge_semantic_coverage_step, new_semantic_coverage_stats
from sts_agent.evidence.reward_metrics import reward_candidate_metrics_v1
from sts_agent.journal.act1_eval import (
    new_act1_eval_summary,
    update_act1_eval_from_observation,
    update_act1_eval_from_record,
)
from sts_agent.context.action_candidates import (
    action_candidate_policy_prompt_lines,
    build_action_candidate_policy_v1,
    build_action_descriptor_v1,
    candidate_with_descriptor,
)
from sts_agent.evidence.card_eval import (
    candidate_card_names,
    candidate_payload,
    collect_room_tags,
    build_campfire_eval,
    build_deck_need_eval,
    build_map_route_eval,
    build_reward_card_eval,
    build_shop_purchase_eval,
)
from sts_agent.evidence.combat_search_view import (
    combat_search_shadow_opinion,
    combat_search_prompt_lines,
    compact_combat_search_report,
    watch_search_summary_lines,
)
from sts_agent.briefs.decision_brief import (
    decision_brief_lines,
    decision_brief_v1,
    decision_lens_lines,
    infer_plan_commitments,
    llm_decision_summary_lines,
    print_decision_brief_prompt_section,
    print_full_decision_brief,
    print_full_llm_json,
)
from sts_agent.journal.journal_events import (
    build_combat_search_event,
    build_journal_event,
    build_journal_events,
    log_decision,
    should_log_decision,
    update_working_memory,
)
from sts_agent.runtime.llm_provider import call_openai_compatible, extract_json_object, mock_choice
from sts_agent.utils.llm_utils import (
    compact_json,
    find_candidate,
    json_safe,
    map_room_label,
    map_route_context_lines,
    short_action_label,
)



def start_combat_human_trajectory(
    *,
    args: argparse.Namespace,
    step_index: int,
    public_payload: dict[str, Any],
    executed_action_trace: list[dict[str, Any]],
) -> dict[str, Any]:
    return {
        "schema_name": "CombatHumanTrajectory",
        "schema_version": 1,
        "role": "human_demonstration_for_later_search_replay",
        "label_role": "human_baseline_candidate_not_teacher_label",
        "trainable_as_action_label": False,
        "policy_quality_claim": False,
        "seed": args.seed,
        "ascension": args.ascension,
        "class": args.player_class,
        "combat_start_step": step_index,
        "combat_start_floor": public_payload.get("floor"),
        "combat_start_state": public_state_snapshot(public_payload),
        "replay_prefix_before_combat": executed_action_trace.copy(),
        "actions": [],
        "search_shadow_steps": [],
        "evaluation_contract": {
            "goal": "later_search_must_find_full_trajectory_with_outcome_not_weaker_than_human_demonstration",
            "comparison_unit": "whole_combat_trajectory_not_single_step_agreement",
            "do_not_treat_stepwise_frontier_mismatch_as_failure": True,
        },
    }


def compact_combat_trajectory_step(
    *,
    step_index: int,
    public_payload: dict[str, Any],
    selected_candidate: dict[str, Any] | None,
    action_id: int,
    owner: str,
    shadow_opinion: dict[str, Any] | None,
    step_result: dict[str, Any],
) -> dict[str, Any]:
    return {
        "step_index": step_index,
        "floor": public_payload.get("floor"),
        "hp_before": public_payload.get("current_hp"),
        "max_hp_before": public_payload.get("max_hp"),
        "combat": {
            "energy": ((public_payload.get("combat") or {}).get("energy")),
            "player_block": ((public_payload.get("combat") or {}).get("player_block")),
            "visible_incoming_damage": ((public_payload.get("combat") or {}).get("visible_incoming_damage")),
            "total_monster_hp": ((public_payload.get("combat") or {}).get("total_monster_hp")),
        },
        "executed_action": {
            "action_id": action_id,
            "action_key": (selected_candidate or {}).get("action_key"),
            "owner": owner,
        },
        "search_shadow": {
            "primary_action_id": (shadow_opinion or {}).get("primary_action_id"),
            "primary_action_key": (shadow_opinion or {}).get("primary_action_key"),
            "frontier_action_ids": (shadow_opinion or {}).get("frontier_action_ids") or [],
            "frontier_action_keys": (shadow_opinion or {}).get("frontier_action_keys") or [],
            "root_outcomes": ((shadow_opinion or {}).get("root_outcomes") or [])[:6],
            "unresolved_frontier": (shadow_opinion or {}).get("unresolved_frontier") or {},
            "reliability": (shadow_opinion or {}).get("reliability") or {},
        },
        "post_step_info": step_result.get("info"),
    }


def write_combat_human_trajectory(
    args: argparse.Namespace,
    trajectory: dict[str, Any] | None,
    *,
    end_reason: str,
) -> None:
    if not trajectory or not trajectory.get("actions"):
        return
    trajectory["combat_end_reason"] = end_reason
    trajectory["combat_action_count"] = len(trajectory.get("actions") or [])
    last_action = (trajectory.get("actions") or [])[-1]
    trajectory["final_observed_info"] = last_action.get("post_step_info")
    path = args.out.parent / "combat_human_trajectories.jsonl"
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(trajectory, ensure_ascii=False, separators=(",", ":")) + "\n")


def run_controller(args: argparse.Namespace) -> dict[str, Any]:
    rng = random.Random(args.seed ^ 0xC0DEC0DE)
    client = DriverClient(args.driver)
    records: list[dict[str, Any]] = []
    journal: list[dict[str, Any]] = []
    working_memory: dict[str, Any] = {
        "schema_name": "AgentWorkingMemory",
        "schema_version": 1,
        "current_goal": "survive_and_progress",
        "known_risks": [],
        "last_tool_findings": [],
        "current_route_intent": None,
        "plan_commitments": [],
        "last_failure_reason": None,
    }
    planner_request_calls = 0
    recommendation_calls = 0
    tool_result_count = 0
    planner_tool_result_count = 0
    verifier_tool_result_count = 0
    tool_design_question_count = 0
    tool_design_type_counts: dict[str, int] = {}
    route_budget_counts: dict[str, int] = {}
    semantic_coverage = new_semantic_coverage_stats()
    manual_watch_stop = False
    executed_action_trace: list[dict[str, Any]] = []
    active_combat_trajectory: dict[str, Any] | None = None
    act1_eval_summary = new_act1_eval_summary(args) if args.act1_boss_eval else None
    run_started_at = time.time()
    tool_design_out = default_tool_design_out_path(args) if args.tool_design_mode == "observe" else None
    journal_sink = None
    tool_design_sink = None
    try:
        if args.trace_level == "compact":
            args.out.parent.mkdir(parents=True, exist_ok=True)
            journal_sink = args.out.open("w", encoding="utf-8")
        if tool_design_out is not None:
            tool_design_out.parent.mkdir(parents=True, exist_ok=True)
            tool_design_sink = tool_design_out.open("w", encoding="utf-8")
        client.request(
            {
                "cmd": "reset",
                "seed": args.seed,
                "ascension": args.ascension,
                "final_act": args.final_act,
                "class": args.player_class,
                "max_steps": args.max_steps,
            }
        )
        done = False
        for step_index in range(args.steps):
            observation_response = client.request({"cmd": "decision_env_observation"})
            raw_timestep = observation_response["payload"]
            raw_public_payload = public_observation_payload(raw_timestep)
            if act1_eval_summary is not None:
                update_act1_eval_from_observation(act1_eval_summary, raw_public_payload)
                if act1_eval_summary.get("stop_reason") == "act1_boss_killed":
                    break
                elapsed = time.time() - run_started_at
                if elapsed >= args.act1_eval_timeout_seconds:
                    act1_eval_summary["stop_reason"] = "timeout"
                    act1_eval_summary["timed_out"] = True
                    break
            timestep = apply_context_ablation_to_timestep(
                raw_timestep,
                args.context_ablation,
            )
            candidates = timestep.get("candidates") or []
            if not candidates:
                raise RuntimeError("driver returned no legal candidates")
            public_payload = public_observation_payload(timestep)
            action_candidate_policy = build_action_candidate_policy_v1(
                public_payload,
                candidates,
            )
            decision_brief = decision_brief_v1(public_payload, candidates, working_memory)
            merge_semantic_coverage_step(
                semantic_coverage,
                public_payload,
                action_candidate_policy,
            )
            timestep["action_candidate_policy"] = action_candidate_policy
            timestep["decision_brief"] = decision_brief
            timestep["decision_candidates"] = action_candidate_policy.get(
                "decision_candidates",
                candidates,
            )
            decision_type = public_payload.get("decision_type")
            if (
                args.combat_shadow_compare
                and active_combat_trajectory is not None
                and decision_type != "combat"
            ):
                write_combat_human_trajectory(
                    args,
                    active_combat_trajectory,
                    end_reason="left_combat",
                )
                active_combat_trajectory = None
            combat_search_report = None
            combat_search_prompt_report = None
            combat_search_error = None
            combat_shadow_opinion = None
            if args.combat_search_engine != "off" and decision_type == "combat":
                try:
                    combat_search_report = request_combat_search_engine(client, args)
                    combat_search_prompt_report = compact_combat_search_report(
                        combat_search_report,
                        mode=args.combat_search_engine,
                        candidates=candidates,
                    )
                    timestep["combat_search_report"] = combat_search_prompt_report
                except Exception as err:
                    combat_search_error = str(err)
                    combat_search_prompt_report = {
                        "schema_name": "CombatSearchUnavailable",
                        "schema_version": 1,
                        "information_boundary": "engine_search",
                        "decision_authority": "evidence_only",
                        "not_final_action": True,
                        "error": combat_search_error,
                    }
                    timestep["combat_search_report"] = combat_search_prompt_report
            if args.combat_shadow_compare and decision_type == "combat":
                combat_shadow_opinion = combat_search_shadow_opinion(
                    combat_search_prompt_report,
                    candidates,
                )
            risk = risk_flags(public_payload)
            use_search = combat_search_execution_enabled(args, decision_type)
            decision_route = route_decision(
                timestep,
                args,
                risk,
                use_search=use_search,
            )
            authority_scope = authority_scope_for_route(
                decision_route,
                args,
                use_search=use_search,
            )
            decision_frame = decision_frame_for_route(decision_route, authority_scope)
            if args.provider == "dry_run":
                system, user = build_prompt(timestep, max_candidates=args.max_candidates)
                records.append(
                    {
                        "step_index": step_index,
                        "provider": "dry_run",
                        "system": system,
                        "prompt": user,
                        "candidate_count": len(candidates),
                    }
                )
                break
            planner_payload = None
            tool_results: list[dict[str, Any]] = []
            raw_tools: dict[str, Any] = {}
            search_probe = None
            search_evidence_value = None
            guardrail = None
            executed_action_owner = authority_scope.get("decision_owner") or "llm_controller"
            planner_gate_reason = None
            recommendation_called = False
            routine_reason = None
            parsed: dict[str, Any] = {}
            raw_text = ""
            raw_payload = None
            pre_llm_manual_action = False
            pre_watch: dict[str, Any] | None = None
            override_record: dict[str, Any] = {
                "applied": False,
                "policy": authority_scope.get("override_policy"),
                "silent_override": False,
            }
            single_candidate = candidates[0] if len(candidates) == 1 else None
            watch_before_llm_enabled = (
                args.watch_decisions
                and (args.human_recording_mode or args.provider == "openai_compatible")
                and (args.watch_before_llm or args.provider == "openai_compatible")
                and not use_search
                and not (
                    len(candidates) <= 1
                    and not args.watch_pause_single_legal
                    and is_routine_mechanical_single_action(single_candidate, public_payload)
                )
            )
            if watch_before_llm_enabled:
                pre_watch = watch_before_llm_interactively(
                    step_index=step_index,
                    decision_type=decision_type,
                    public_payload=public_payload,
                    candidates=candidates,
                    combat_search_report=combat_search_prompt_report,
                    search_shadow_opinion=combat_shadow_opinion,
                    action_candidate_policy=action_candidate_policy,
                    max_candidates=args.watch_max_candidates,
                    decision_brief=decision_brief,
                    recording_mode=args.human_recording_mode,
                    case_metadata={
                        "provider": args.provider,
                        "model": args.model,
                        "run_mode": args.run_mode,
                        "agent_mode": args.agent_mode,
                        "combat_decision_owner": args.combat_decision_owner,
                        "tool_policy": args.tool_policy,
                        "context_ablation": args.context_ablation,
                        "combat_search_engine": args.combat_search_engine,
                        "seed": args.seed,
                        "trace_level": args.trace_level,
                        "journal_format": args.journal_format,
                        "out": args.out,
                    },
                )
                if pre_watch.get("stop"):
                    manual_watch_stop = True
                    break
                if not pre_watch.get("ask_llm", True):
                    pre_llm_manual_action = True
                    action_id = int(pre_watch.get("action_id"))
                    action_id, legal, validation = validate_action_id(action_id, candidates)
                    parsed = {
                        "action_id": action_id,
                        "confidence": "human_pre_llm",
                        "reason": "manual action selected from pre-LLM watch window",
                    }
                    raw_text = json.dumps(parsed, ensure_ascii=False)
                    raw_payload = None
                    executed_action_owner = "human_pre_llm_watch_override"
            if pre_llm_manual_action:
                pass
            elif args.agent_mode == "planner":
                routine_id = None
                if (
                    not use_search
                    and (
                        not decision_route.get("planner_allowed")
                        or args.run_mode == "search_final_baseline"
                    )
                ):
                    routine_id, routine_reason = routine_action_id(
                        candidates=candidates,
                        decision_type=decision_type,
                        public_payload=public_payload,
                    )
                should_plan_tools, budget_reason = route_budget_allows(
                    decision_route,
                    route_budget_counts,
                    args,
                )
                planner_gate_reason = (
                    f"decision_route:{decision_route.get('decision_class')}:"
                    f"{budget_reason or decision_route.get('llm_role')}"
                )
                if use_search or routine_id is not None:
                    should_plan_tools = False
                    planner_gate_reason = (
                        "decision_route:combat_search_verifier_owns_tool_use"
                        if use_search
                        else f"decision_route:{decision_route.get('decision_class')}:{routine_reason}"
                    )
                planner_payload = {
                    "intent": "skip_tools",
                    "requests": [],
                    "reason": planner_gate_reason,
                    "decision_frame": decision_frame,
                }
                if should_plan_tools:
                    requests = route_tool_requests(
                        timestep,
                        args,
                        decision_route,
                    )
                    planner_payload = {
                        "intent": "request_evidence" if requests else "recommend_without_tool_request",
                        "requests": requests,
                        "reason": planner_gate_reason,
                        "decision_frame": decision_frame,
                    }
                    tool_results, raw_tools = execute_planner_requests(
                        client,
                        args,
                        timestep,
                        requests,
                    )
                    pool = str(decision_route.get("budget_pool") or "unknown")
                    route_budget_counts[pool] = route_budget_counts.get(pool, 0) + 1
                if not should_plan_tools and not use_search and routine_id is None:
                    routine_id = first_structural_action_id(candidates)
                    routine_reason = f"route_fallback_first_structural:{planner_gate_reason}"
                should_recommend = (
                    bool(decision_route.get("planner_allowed"))
                    and not use_search
                    and routine_id is None
                )
                if should_recommend:
                    recommendation_calls += 1
                    parsed, raw_text, raw_payload = planner_recommendation(
                        args,
                        timestep,
                        tool_results,
                        rng,
                        working_memory=working_memory,
                    )
                    recommendation_called = True
                elif routine_id is not None:
                    parsed = {
                        "action_id": routine_id,
                        "confidence": "deterministic",
                        "reason": routine_reason,
                    }
                    raw_text = json.dumps(parsed, ensure_ascii=False)
                    raw_payload = None
                else:
                    parsed = {
                        "action_id": None,
                        "confidence": "not_used",
                        "reason": "LLM recommendation skipped; search verifier has final combat authority.",
                    }
                    raw_text = json.dumps(parsed, ensure_ascii=False)
                    raw_payload = None
                executed_action_owner = authority_scope.get("decision_owner") or "llm_controller"
                if use_search:
                    search_probe = raw_tools.get("combat_turn_probe")
                    if search_probe is None:
                        search_probe = request_combat_plan_probe(client, args)
                        verifier_tool_result = compact_combat_probe_result(search_probe)
                        verifier_tool_result["source"] = "search_verifier"
                        tool_results.append(verifier_tool_result)
                    if args.combat_lab_with_search and should_run_combat_lab_observer(
                        timestep,
                        risk=risk,
                    ):
                        lab_action_ids = default_combat_lab_action_ids(
                            candidates,
                            limit=args.combat_lab_max_root_actions,
                        )
                        combat_lab = request_combat_multi_turn_lab(
                            client,
                            args,
                            lab_action_ids,
                        )
                        lab_result = compact_combat_multi_turn_lab_result(
                            combat_lab,
                            candidates,
                        )
                        lab_result["source"] = "harness_observer"
                        tool_results.append(lab_result)
                    action_id, search_evidence_value = choose_search_action(
                        probe=search_probe,
                        candidates=candidates,
                    )
                    executed_action_owner = "search_controller"
                else:
                    requested_action_id, guardrail = guardrail_action_id(
                        parsed=parsed,
                        candidates=candidates,
                        public_payload=public_payload,
                    )
                    action_id, legal, validation = validate_action_id(
                        requested_action_id,
                        candidates,
                    )
                    if guardrail:
                        executed_action_owner = "harness_guardrail"
                    elif routine_id is not None:
                        executed_action_owner = "deterministic_harness"
            elif use_search:
                search_probe = request_combat_plan_probe(client, args)
                verifier_tool_result = compact_combat_probe_result(search_probe)
                verifier_tool_result["source"] = "search_verifier"
                tool_results.append(verifier_tool_result)
                if args.combat_lab_with_search and should_run_combat_lab_observer(
                    timestep,
                    risk=risk,
                ):
                    lab_action_ids = default_combat_lab_action_ids(
                        candidates,
                        limit=args.combat_lab_max_root_actions,
                    )
                    combat_lab = request_combat_multi_turn_lab(
                        client,
                        args,
                        lab_action_ids,
                    )
                    lab_result = compact_combat_multi_turn_lab_result(combat_lab, candidates)
                    lab_result["source"] = "harness_observer"
                    tool_results.append(lab_result)
                action_id, search_evidence_value = choose_search_action(
                    probe=search_probe,
                    candidates=candidates,
                )
                parsed = {
                    "action_id": action_id,
                    "confidence": "high",
                    "reason": (
                        "search controller selected "
                        f"{search_evidence_value.get('search_selected_plan')}"
                    ),
                }
                raw_text = json.dumps(parsed, ensure_ascii=False)
                raw_payload = None
                executed_action_owner = "search_controller"
            elif args.provider == "mock":
                parsed = mock_choice(timestep, rng)
                raw_text = json.dumps(parsed, ensure_ascii=False)
                raw_payload = None
            elif args.provider == "routine":
                routine_id, routine_reason = routine_action_id(
                    candidates=candidates,
                    decision_type=decision_type,
                    public_payload=public_payload,
                )
                if routine_id is None:
                    routine_id = first_structural_action_id(candidates)
                    routine_reason = "routine_fallback_first_structural"
                parsed = {
                    "action_id": routine_id,
                    "confidence": "deterministic",
                    "reason": routine_reason,
                }
                raw_text = json.dumps(parsed, ensure_ascii=False)
                raw_payload = None
                executed_action_owner = "routine_policy"
            else:
                system, user = build_prompt(timestep, max_candidates=args.max_candidates)
                recommendation_calls += 1
                raw_text, raw_payload = call_openai_compatible(
                    base_url=args.base_url,
                    api_key=args.api_key,
                    model=args.model,
                    system=system,
                    user=user,
                    temperature=args.temperature,
                    timeout=args.timeout,
                    phase="action_id_choice",
                )
                parsed = extract_json_object(raw_text)
                recommendation_called = True
            if not (args.agent_mode == "planner" and not use_search):
                action_id, legal, validation = validate_action_id(
                    action_id if use_search else parsed.get("action_id"),
                    candidates,
                )
            if args.run_mode in {"search_final_baseline", "llm_shadow_audit"} and not use_search:
                llm_shadow_choice = {
                    "action_id": action_id,
                    "action_key": (find_candidate(candidates, action_id) or {}).get("action_key"),
                    "raw_choice": parsed,
                }
                baseline_id, baseline_reason = routine_action_id(
                    candidates=candidates,
                    decision_type=decision_type,
                    public_payload=public_payload,
                )
                if baseline_id is None:
                    baseline_id = first_structural_action_id(candidates)
                    baseline_reason = "baseline_fallback_first_structural"
                action_id, legal, validation = validate_action_id(baseline_id, candidates)
                routine_reason = baseline_reason
                executed_action_owner = "routine_policy"
                override_record.update(
                    {
                        "applied": args.run_mode == "llm_shadow_audit",
                        "reason": "shadow_mode_executes_baseline_not_llm"
                        if args.run_mode == "llm_shadow_audit"
                        else "search_final_baseline_noncombat_routine",
                        "llm_shadow_choice": llm_shadow_choice,
                    }
                )
            elif args.run_mode == "llm_live_with_tactical_safety" and decision_type == "combat":
                override_record.update(
                    {
                        "evaluated": True,
                        "applied": False,
                        "reason": "no_certified_tactical_fatality_override",
                        "llm_action_id": action_id,
                        "llm_action_key": (find_candidate(candidates, action_id) or {}).get("action_key"),
                    }
                )
            tool_result_count += len(tool_results)
            planner_tool_result_count += sum(
                1 for result in tool_results if result.get("source") != "search_verifier"
            )
            verifier_tool_result_count += sum(
                1 for result in tool_results if result.get("source") == "search_verifier"
            )
            selected_candidate = find_candidate(candidates, action_id)
            inferred_commitments = infer_plan_commitments(
                public_payload=public_payload,
                candidates=candidates,
                parsed=parsed if isinstance(parsed, dict) else None,
                action_id=action_id,
            )
            if inferred_commitments and isinstance(parsed, dict):
                existing_commitments = [
                    item for item in parsed.get("plan_commitments") or []
                    if isinstance(item, dict)
                ]
                parsed["plan_commitments"] = existing_commitments + inferred_commitments
            watch_override = None
            if pre_llm_manual_action:
                watch_override = (pre_watch or {}).get("override")
            elif (
                args.watch_decisions
                and len(candidates) <= 1
                and not args.watch_pause_single_legal
                and is_routine_mechanical_single_action(selected_candidate, public_payload)
            ):
                auto_key = selected_candidate.get("action_key") if selected_candidate else None
                print(
                    f"WATCH auto step={step_index} decision={decision_type} "
                    f"single_legal id={action_id} key={auto_key}",
                    flush=True,
                )
            elif args.watch_decisions:
                watch_result = watch_decision_interactively(
                    step_index=step_index,
                    decision_type=decision_type,
                    public_payload=public_payload,
                    candidates=candidates,
                    parsed=parsed if isinstance(parsed, dict) else {},
                    action_id=action_id,
                    validation=validation,
                    combat_search_report=combat_search_prompt_report,
                    search_shadow_opinion=combat_shadow_opinion,
                    action_candidate_policy=action_candidate_policy,
                    max_candidates=args.watch_max_candidates,
                    decision_brief=decision_brief,
                    recording_mode=args.human_recording_mode,
                    case_metadata={
                        "provider": args.provider,
                        "model": args.model,
                        "run_mode": args.run_mode,
                        "agent_mode": args.agent_mode,
                        "combat_decision_owner": args.combat_decision_owner,
                        "tool_policy": args.tool_policy,
                        "context_ablation": args.context_ablation,
                        "combat_search_engine": args.combat_search_engine,
                        "seed": args.seed,
                        "trace_level": args.trace_level,
                        "journal_format": args.journal_format,
                        "out": args.out,
                    },
                    llm_raw_text=raw_text,
                )
                if watch_result.get("stop"):
                    manual_watch_stop = True
                    break
                next_action_id = watch_result.get("action_id", action_id)
                watch_override = watch_result.get("override")
                if next_action_id != action_id:
                    action_id, legal, validation = validate_action_id(next_action_id, candidates)
                    selected_candidate = find_candidate(candidates, action_id)
                    executed_action_owner = "human_watch_override"
            step = client.request({"cmd": "decision_env_step", "action_id": action_id})
            done = bool(step.get("done"))
            if args.combat_shadow_compare and decision_type == "combat":
                if active_combat_trajectory is None:
                    active_combat_trajectory = start_combat_human_trajectory(
                        args=args,
                        step_index=step_index,
                        public_payload=public_payload,
                        executed_action_trace=executed_action_trace,
                    )
                trajectory_step = compact_combat_trajectory_step(
                    step_index=step_index,
                    public_payload=public_payload,
                    selected_candidate=selected_candidate,
                    action_id=action_id,
                    owner=executed_action_owner,
                    shadow_opinion=combat_shadow_opinion,
                    step_result=step,
                )
                active_combat_trajectory["actions"].append(trajectory_step)
                active_combat_trajectory["search_shadow_steps"].append(
                    trajectory_step.get("search_shadow") or {}
                )
            executed_action_trace.append(
                {
                    "step_index": step_index,
                    "decision_type": decision_type,
                    "action_id": action_id,
                    "action_key": selected_candidate.get("action_key")
                    if selected_candidate
                    else None,
                    "owner": executed_action_owner,
                }
            )
            public_payload = public_observation_payload(timestep)
            record = {
                "step_index": step_index,
                "provider": args.provider,
                "run_mode": args.run_mode,
                "agent_mode": args.agent_mode,
                "combat_decision_owner": args.combat_decision_owner,
                "tool_policy": args.tool_policy,
                "journal_format": args.journal_format,
                "model": args.model if args.provider == "openai_compatible" else None,
                "controller_role": (
                    "llm_live_controller_behavior"
                    if args.run_mode.startswith("llm_live")
                    else (
                        "llm_shadow_audit_behavior"
                        if args.run_mode == "llm_shadow_audit"
                        else "search_final_baseline_behavior"
                    )
                ),
                "information_boundary": "engine_search"
                if use_search or any(result.get("tool") for result in tool_results)
                else "public_observation_only",
                "decision_type": decision_type,
                "public_state_before": public_state_snapshot(public_payload),
                "pre_info": observation_response.get("info"),
                "candidate_count": len(candidates),
                "candidates": candidates,
                "action_candidate_policy": action_candidate_policy,
                "decision_route": decision_route,
                "decision_frame": decision_frame,
                "authority_scope": authority_scope,
                "decision_brief": decision_brief,
                "planner_payload": planner_payload,
                "planner_gate_reason": planner_gate_reason,
                "routine_reason": routine_reason,
                "tool_results": tool_results,
                "combat_search_evidence": combat_search_prompt_report,
                "combat_shadow_opinion": combat_shadow_opinion,
                "combat_search_error": combat_search_error,
                "combat_search_report": combat_search_report if args.combat_search_engine == "full" else None,
                "llm_raw_text": raw_text,
                "llm_raw_payload": raw_payload if args.include_raw_llm_payload else None,
                "llm_choice": parsed,
                "llm_recommendation_called": recommendation_called,
                "llm_recommended_action_key": (
                    find_candidate(candidates, parsed.get("action_id")).get("action_key")
                    if isinstance(parsed, dict)
                    and find_candidate(candidates, parsed.get("action_id"))
                    else None
                ),
                "executed_action_owner": executed_action_owner,
                "guardrail": guardrail,
                "override": override_record,
                "watch_override": watch_override,
                "selected_action_id": action_id,
                "selected_action_key": selected_candidate.get("action_key")
                if selected_candidate
                else None,
                "selected_candidate": selected_candidate,
                "choice_was_legal": legal,
                "validation": validation,
                "search_evidence": search_evidence_value,
                "search_probe": search_probe
                if args.include_raw_search_probe
                else None,
                "reward": step.get("reward"),
                "done": done,
                "info": step.get("info"),
            }
            if inferred_commitments:
                if isinstance(parsed, dict):
                    record["llm_choice"] = parsed
                record["plan_commitments"] = inferred_commitments
            if args.trace_level == "full":
                records.append(record)
            tool_design_events: list[dict[str, Any]] = []
            design_type_key = str(decision_type or "unknown")
            under_type_budget = (
                tool_design_type_counts.get(design_type_key, 0)
                < tool_design_type_budget(decision_type)
            )
            if under_type_budget and should_observe_tool_design(
                timestep,
                args=args,
                risk=risk,
                tool_results=tool_results,
            ) and tool_design_question_count < args.tool_design_max_events:
                try:
                    remaining_tool_design_questions = max(
                        1,
                        args.tool_design_max_events - tool_design_question_count,
                    )
                    questions, tool_design_raw_text, _tool_design_raw_payload = tool_design_phase(
                        args,
                        timestep,
                        tool_results=tool_results,
                        working_memory=working_memory,
                        final_action_key=record.get("selected_action_key"),
                        max_questions=min(
                            args.tool_design_max_questions,
                            remaining_tool_design_questions,
                        ),
                    )
                    tool_design_events = build_tool_design_events(
                        record=record,
                        risk=risk,
                        questions=questions,
                        raw_text=tool_design_raw_text,
                    )
                except Exception as err:
                    tool_design_events = build_tool_design_events(
                        record=record,
                        risk=risk,
                        questions=[],
                        raw_text=None,
                        error=str(err),
                    )
                tool_design_question_count += sum(
                    1 for event in tool_design_events if event.get("status") == "ok"
                )
                if tool_design_events:
                    tool_design_type_counts[design_type_key] = (
                        tool_design_type_counts.get(design_type_key, 0) + 1
                    )
                if tool_design_sink is not None:
                    for event in tool_design_events:
                        tool_design_sink.write(
                            json.dumps(event, ensure_ascii=False, separators=(",", ":")) + "\n"
                        )
                    tool_design_sink.flush()
                if args.trace_level == "full":
                    record["tool_design_events"] = tool_design_events
            update_working_memory(
                working_memory,
                risk=risk,
                tool_results=tool_results,
                record=record,
            )
            if args.journal_format == "decision":
                new_events = [
                    build_journal_event(
                    record=record,
                    planner_payload=planner_payload,
                    tool_results=tool_results,
                    guardrail=guardrail,
                    risk=risk,
                )
                ]
            else:
                new_events = build_journal_events(
                        record=record,
                        planner_payload=planner_payload,
                        tool_results=tool_results,
                        guardrail=guardrail,
                        risk=risk,
                        planner_gate_reason=planner_gate_reason,
                        working_memory=working_memory,
                        recommendation_called=recommendation_called,
                    )
            if combat_search_prompt_report is not None or combat_search_error is not None:
                new_events.append(
                    build_combat_search_event(
                        record,
                        combat_search_prompt_report,
                        combat_search_error,
                    )
                )
            journal.extend(new_events)
            if journal_sink is not None:
                for event in new_events:
                    journal_sink.write(json.dumps(event, ensure_ascii=False, separators=(",", ":")) + "\n")
                journal_sink.flush()
            if args.log_decisions and should_log_decision(record, risk):
                log_decision(record, risk)
            if act1_eval_summary is not None:
                update_act1_eval_from_record(act1_eval_summary, record)
                act1_eval_summary["llm_calls"] = recommendation_calls
                if act1_eval_summary.get("stop_reason") == "player_death":
                    done = True
            if done:
                break
            time.sleep(args.sleep)
    finally:
        if active_combat_trajectory is not None:
            write_combat_human_trajectory(
                args,
                active_combat_trajectory,
                end_reason="run_stopped_or_interrupted",
            )
        if journal_sink is not None:
            journal_sink.close()
        if tool_design_sink is not None:
            tool_design_sink.close()
        client.close()
    if act1_eval_summary is not None:
        act1_eval_summary["llm_calls"] = recommendation_calls
        if act1_eval_summary.get("stop_reason") is None:
            act1_eval_summary["stop_reason"] = "manual_watch_stop" if manual_watch_stop else "step_limit"
    return {
        "schema_version": "llm_full_run_controller_demo_v1",
        "provider": args.provider,
        "run_mode": args.run_mode,
        "agent_mode": args.agent_mode,
        "combat_decision_owner": args.combat_decision_owner,
        "tool_policy": args.tool_policy,
        "seed": args.seed,
        "ascension": args.ascension,
        "class": args.player_class,
        "trace_level": args.trace_level,
        "journal_format": args.journal_format,
        "planner_request_calls": planner_request_calls,
        "recommendation_calls": recommendation_calls,
        "tool_result_count": tool_result_count,
        "planner_tool_result_count": planner_tool_result_count,
        "verifier_tool_result_count": verifier_tool_result_count,
        "route_budget_counts": route_budget_counts,
        "tool_design_mode": args.tool_design_mode,
        "tool_design_out": str(tool_design_out) if tool_design_out is not None else None,
        "tool_design_question_count": tool_design_question_count,
        "tool_design_type_counts": tool_design_type_counts,
        "working_memory": working_memory,
        "context_ablation": args.context_ablation,
        "combat_search_engine": args.combat_search_engine,
        "combat_search_horizon_turns": args.combat_search_horizon_turns,
        "combat_search_particles": args.combat_search_particles,
        "combat_search_max_nodes": args.combat_search_max_nodes,
        "combat_shadow_compare": args.combat_shadow_compare,
        "combat_human_trajectory_out": str(args.out.parent / "combat_human_trajectories.jsonl")
        if args.combat_shadow_compare
        else None,
        "watch_decisions": args.watch_decisions,
        "watch_before_llm": args.watch_before_llm,
        "human_recording_mode": args.human_recording_mode,
        "manual_watch_stop": manual_watch_stop,
        "semantic_coverage": semantic_coverage,
        "act1_eval_summary": act1_eval_summary,
        "journal": journal,
        "records": records,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver", type=Path, default=default_driver_path())
    parser.add_argument("--provider", choices=["dry_run", "mock", "routine", "openai_compatible"], default="dry_run")
    parser.add_argument(
        "--run-mode",
        choices=[
            "llm_live_controller",
            "llm_live_with_tactical_safety",
            "search_final_baseline",
            "llm_shadow_audit",
        ],
        default="llm_live_controller",
    )
    parser.add_argument("--agent-mode", choices=["off", "planner"], default="off")
    parser.add_argument("--trace-level", choices=["compact", "full", "none"], default="compact")
    parser.add_argument("--journal-format", choices=["decision", "events"], default="events")
    parser.add_argument("--tool-policy", choices=["always", "risk_gated", "none"], default="risk_gated")
    parser.add_argument(
        "--context-ablation",
        choices=[
            "none",
            "identity_only",
            "mechanics",
            "mechanics_plus_strategy_hints",
        ],
        default="mechanics_plus_strategy_hints",
    )
    parser.add_argument("--act1-boss-eval", action="store_true")
    parser.add_argument("--act1-eval-out", type=Path, default=None)
    parser.add_argument("--act1-eval-timeout-seconds", type=int, default=900)
    parser.add_argument("--tool-design-mode", choices=["off", "observe"], default="off")
    parser.add_argument("--tool-design-out", type=Path, default=None)
    parser.add_argument("--tool-design-max-questions", type=int, default=1)
    parser.add_argument("--tool-design-max-events", type=int, default=8)
    parser.add_argument("--max-tool-requests", type=int, default=2)
    parser.add_argument("--planner-max-requests", type=int, default=16)
    parser.add_argument("--planner-timeout", type=int, default=120)
    parser.add_argument("--combat-decision-owner", choices=["llm", "search"], default="llm")
    parser.add_argument("--combat-search-engine", choices=["off", "compact", "full"], default=None)
    parser.add_argument("--combat-search-horizon-turns", type=int, default=2)
    parser.add_argument("--combat-search-particles", type=int, default=32)
    parser.add_argument("--combat-search-max-nodes", type=int, default=4000)
    parser.add_argument("--combat-search-beam-width", type=int, default=48)
    parser.add_argument(
        "--combat-shadow-compare",
        action="store_true",
        help="In combat watch mode, record the complete executed combat trajectory with search shadow opinions for later whole-trajectory search comparison.",
    )
    parser.add_argument("--watch-decisions", action="store_true")
    parser.add_argument("--watch-max-candidates", type=int, default=24)
    parser.add_argument("--watch-pause-single-legal", action="store_true")
    parser.add_argument(
        "--human-recording-mode",
        action="store_true",
        help="Use a compact manual-action watch UI for recording human demonstrations; do not offer LLM calls at decision prompts.",
    )
    parser.add_argument(
        "--watch-before-llm",
        action="store_true",
        help="In watch mode, pause before external LLM calls so the user can inspect/save/override first.",
    )
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--max-steps", type=int, default=500)
    parser.add_argument("--steps", type=int, default=1)
    parser.add_argument("--max-candidates", type=int, default=24)
    parser.add_argument("--out", type=Path, default=REPO_ROOT / "tools" / "artifacts" / "llm_demo" / "run.json")
    parser.add_argument("--temperature", type=float, default=0.0)
    parser.add_argument("--timeout", type=int, default=60)
    parser.add_argument("--sleep", type=float, default=0.0)
    parser.add_argument("--include-raw-llm-payload", action="store_true")
    parser.add_argument("--include-raw-search-probe", action="store_true")
    parser.add_argument("--log-decisions", action="store_true")
    parser.add_argument("--search-max-depth", type=int, default=6)
    parser.add_argument("--search-max-nodes", type=int, default=2000)
    parser.add_argument("--search-beam-width", type=int, default=32)
    parser.add_argument("--search-max-engine-steps-per-action", type=int, default=200)
    parser.add_argument("--lab-max-rollout-steps", type=int, default=4)
    parser.add_argument("--combat-lab-max-root-actions", type=int, default=4)
    parser.add_argument("--combat-lab-max-rollout-steps", type=int, default=6)
    parser.add_argument("--combat-lab-with-search", action="store_true")
    parser.add_argument("--base-url", default=os.environ.get("LLM_BASE_URL") or os.environ.get("OPENAI_BASE_URL") or "https://api.openai.com/v1")
    parser.add_argument("--model", default=os.environ.get("LLM_MODEL") or "gpt-4o-mini")
    parser.add_argument("--api-key", default=os.environ.get("LLM_API_KEY") or os.environ.get("OPENAI_API_KEY") or os.environ.get("DEEPSEEK_API_KEY") or "")
    args = parser.parse_args()
    if args.combat_search_engine is None:
        args.combat_search_engine = "full" if args.run_mode == "search_final_baseline" else "compact"
    if args.act1_boss_eval and args.steps == 1:
        args.steps = 500
    if args.act1_boss_eval and args.act1_eval_out is None:
        args.act1_eval_out = (
            REPO_ROOT
            / "tools"
            / "artifacts"
            / "evals"
            / f"act1_eval_seed{args.seed}_{args.context_ablation}.json"
        )
    if args.provider == "openai_compatible" and not args.api_key:
        parser.error("--provider openai_compatible requires LLM_API_KEY, OPENAI_API_KEY, or DEEPSEEK_API_KEY")
    return args


def main() -> int:
    args = parse_args()
    report = run_controller(args)
    if args.trace_level != "none":
        args.out.parent.mkdir(parents=True, exist_ok=True)
    if args.trace_level == "full":
        args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    elif args.trace_level == "compact":
        args.out.write_text(
            "\n".join(
                json.dumps(event, ensure_ascii=False, separators=(",", ":"))
                for event in report.get("journal", [])
            )
            + ("\n" if report.get("journal") else ""),
            encoding="utf-8",
        )
    if args.act1_boss_eval and args.act1_eval_out is not None:
        args.act1_eval_out.parent.mkdir(parents=True, exist_ok=True)
        args.act1_eval_out.write_text(
            json.dumps(report.get("act1_eval_summary") or {}, ensure_ascii=False, indent=2),
            encoding="utf-8",
        )
    summary = {
        "schema_version": report.get("schema_version"),
        "provider": report.get("provider"),
        "agent_mode": report.get("agent_mode"),
        "combat_decision_owner": report.get("combat_decision_owner"),
        "tool_policy": report.get("tool_policy"),
        "context_ablation": report.get("context_ablation"),
        "combat_search_engine": report.get("combat_search_engine"),
        "combat_search_horizon_turns": report.get("combat_search_horizon_turns"),
        "combat_search_particles": report.get("combat_search_particles"),
        "combat_search_max_nodes": report.get("combat_search_max_nodes"),
        "combat_shadow_compare": report.get("combat_shadow_compare"),
        "combat_human_trajectory_out": report.get("combat_human_trajectory_out"),
        "watch_decisions": report.get("watch_decisions"),
        "manual_watch_stop": report.get("manual_watch_stop"),
        "tool_design_mode": report.get("tool_design_mode"),
        "trace_level": report.get("trace_level"),
        "journal_format": report.get("journal_format"),
        "seed": report.get("seed"),
        "records": len(report.get("records") or []),
        "journal_events": len(report.get("journal") or []),
        "planner_request_calls": report.get("planner_request_calls"),
        "recommendation_calls": report.get("recommendation_calls"),
        "tool_result_count": report.get("tool_result_count"),
        "planner_tool_result_count": report.get("planner_tool_result_count"),
        "verifier_tool_result_count": report.get("verifier_tool_result_count"),
        "tool_design_question_count": report.get("tool_design_question_count"),
        "semantic_coverage": report.get("semantic_coverage"),
        "out": str(args.out) if args.trace_level != "none" else None,
        "tool_design_out": report.get("tool_design_out"),
        "act1_eval_out": str(args.act1_eval_out) if args.act1_eval_out else None,
        "act1_eval_summary": report.get("act1_eval_summary"),
    }
    if args.human_recording_mode:
        print("Recording stopped.", flush=True)
        print(f"Run dir: {args.out.parent}", flush=True)
        print(f"Trajectory: {report.get('combat_human_trajectory_out')}", flush=True)
        print(f"Events: {str(args.out) if args.trace_level != 'none' else None}", flush=True)
        print(
            "LLM calls: "
            + f"recommendations={report.get('recommendation_calls')} "
            + f"planner_requests={report.get('planner_request_calls')}",
            flush=True,
        )
    else:
        print(json.dumps(summary, ensure_ascii=False, indent=2))
    if args.provider == "dry_run":
        prompt = report["records"][0]["prompt"] if report.get("records") else ""
        print("\n--- prompt preview ---\n")
        print(textwrap.shorten(prompt.replace("\n", " | "), width=1200, placeholder=" ..."))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
