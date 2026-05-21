"""Planner request/recommendation runtime and evidence tool execution."""

from __future__ import annotations

import argparse
import json
import random
from typing import Any

from sts_agent.context.tool_request_defaults import (
    default_combat_lab_action_ids,
    default_tool_action_ids,
    mock_planner_requests,
)
from sts_agent.evidence.card_eval import (
    build_campfire_eval,
    build_deck_need_eval,
    build_map_route_eval,
    build_reward_card_eval,
    build_shop_purchase_eval,
)
from sts_agent.runtime.driver_requests import (
    request_candidate_afterstate_summary,
    request_campfire_rest_smith_eval,
    request_combat_multi_turn_lab,
    request_combat_plan_probe,
    request_decision_lab_probe,
)
from sts_agent.runtime.env_driver import DriverClient
from sts_agent.runtime.llm_provider import call_openai_compatible, extract_json_object, mock_choice
from sts_agent.context.observation_context import public_observation_payload
from sts_agent.runtime.prompt_builders import build_planner_prompt, build_recommendation_prompt
from sts_agent.evidence.combat_lab_views import (
    combat_evidence_conflict_resolver,
    compact_combat_multi_turn_lab_result,
)
from sts_agent.evidence.tool_result_views import (
    compact_afterstate_result,
    compact_campfire_result,
    compact_combat_probe_result,
    compact_decision_lab_result,
)


def normalize_tool_requests(value: Any, *, max_tool_requests: int) -> tuple[list[dict[str, Any]], str]:
    if not isinstance(value, dict):
        return [], "planner response was not an object"
    requests = value.get("requests") or []
    if not isinstance(requests, list):
        return [], "planner requests was not a list"
    normalized = []
    for request in requests[:max_tool_requests]:
        if isinstance(request, dict):
            normalized.append(request)
    return normalized, str(value.get("reason") or "")

def execute_planner_requests(
    client: DriverClient,
    args: argparse.Namespace,
    timestep: dict[str, Any],
    requests: list[dict[str, Any]],
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    payload = public_observation_payload(timestep)
    decision_type = payload.get("decision_type")
    candidates = timestep.get("candidates") or []
    legal_ids = {int(candidate["id"]) for candidate in candidates}
    tool_results: list[dict[str, Any]] = []
    raw_tools: dict[str, Any] = {}
    for request in requests[: args.max_tool_requests]:
        tool = request.get("tool")
        try:
            if tool == "deck_need_eval":
                result = build_deck_need_eval(payload)
                raw_tools["deck_need_eval"] = result
                tool_results.append(result)
            elif tool == "reward_card_eval":
                result = build_reward_card_eval(payload, candidates)
                raw_tools["reward_card_eval"] = result
                tool_results.append(result)
            elif tool == "map_route_eval":
                if decision_type != "map":
                    raise ValueError("map_route_eval is only available for map decisions")
                result = build_map_route_eval(payload, candidates)
                raw_tools["map_route_eval"] = result
                tool_results.append(result)
            elif tool == "shop_purchase_eval":
                if decision_type != "shop":
                    raise ValueError("shop_purchase_eval is only available for shop decisions")
                result = build_shop_purchase_eval(payload, candidates)
                raw_tools["shop_purchase_eval"] = result
                tool_results.append(result)
            elif tool == "campfire_eval":
                if decision_type != "campfire":
                    raise ValueError("campfire_eval is only available at campfire")
                raw = request_campfire_rest_smith_eval(client)
                result = build_campfire_eval(payload, candidates, raw)
                raw_tools["campfire_eval"] = result
                tool_results.append(result)
            elif tool == "combat_turn_probe":
                if decision_type != "combat":
                    raise ValueError("combat_turn_probe is only available for combat decisions")
                probe = request_combat_plan_probe(client, args)
                raw_tools["combat_turn_probe"] = probe
                tool_results.append(compact_combat_probe_result(probe))
            elif tool == "combat_multi_turn_lab":
                if decision_type != "combat":
                    raise ValueError("combat_multi_turn_lab is only available for combat decisions")
                requested_ids = request.get("action_ids")
                if not isinstance(requested_ids, list) or not requested_ids:
                    requested_ids = default_combat_lab_action_ids(
                        candidates,
                        limit=args.combat_lab_max_root_actions,
                    )
                action_ids = []
                for value in requested_ids[: args.combat_lab_max_root_actions]:
                    try:
                        parsed = int(value)
                    except (TypeError, ValueError):
                        continue
                    if parsed in legal_ids:
                        action_ids.append(parsed)
                result = request_combat_multi_turn_lab(client, args, action_ids)
                raw_tools["combat_multi_turn_lab"] = result
                tool_results.append(compact_combat_multi_turn_lab_result(result, candidates))
            elif tool == "campfire_rest_smith_eval":
                if decision_type != "campfire":
                    raise ValueError("campfire_rest_smith_eval is only available at campfire")
                result = request_campfire_rest_smith_eval(client)
                raw_tools["campfire_rest_smith_eval"] = result
                tool_results.append(compact_campfire_result(result))
            elif tool == "candidate_afterstate_summary":
                requested_ids = request.get("action_ids")
                if not isinstance(requested_ids, list) or not requested_ids:
                    requested_ids = default_tool_action_ids(candidates)
                action_ids = []
                for value in requested_ids[:5]:
                    try:
                        parsed = int(value)
                    except (TypeError, ValueError):
                        continue
                    if parsed in legal_ids:
                        action_ids.append(parsed)
                result = request_candidate_afterstate_summary(client, action_ids)
                raw_tools["candidate_afterstate_summary"] = result
                tool_results.append(compact_afterstate_result(result, candidates))
            elif tool == "decision_lab":
                requested_ids = request.get("action_ids")
                if not isinstance(requested_ids, list) or not requested_ids:
                    requested_ids = default_tool_action_ids(candidates)
                action_ids = []
                for value in requested_ids[:5]:
                    try:
                        parsed = int(value)
                    except (TypeError, ValueError):
                        continue
                    if parsed in legal_ids:
                        action_ids.append(parsed)
                result = request_decision_lab_probe(client, args, action_ids)
                raw_tools["decision_lab"] = result
                tool_results.append(compact_decision_lab_result(result, candidates))
            else:
                tool_results.append(
                    {
                        "tool": tool,
                        "status": "refused",
                        "reason": "unsupported tool request",
                    }
                )
        except Exception as err:
            tool_results.append(
                {
                    "tool": tool,
                    "status": "error",
                    "reason": str(err),
                }
            )
    if decision_type == "combat":
        probe_result = next(
            (
                result
                for result in tool_results
                if isinstance(result, dict) and result.get("tool") == "combat_turn_probe"
            ),
            None,
        )
        lab_result = next(
            (
                result
                for result in tool_results
                if isinstance(result, dict) and result.get("tool") == "combat_multi_turn_lab"
            ),
            None,
        )
        if probe_result and lab_result:
            tool_results.append(combat_evidence_conflict_resolver(probe_result, lab_result))
    return tool_results, raw_tools

def planner_request_phase(
    client: DriverClient,
    args: argparse.Namespace,
    timestep: dict[str, Any],
    working_memory: dict[str, Any] | None = None,
) -> tuple[dict[str, Any], list[dict[str, Any]], dict[str, Any]]:
    if args.provider == "mock":
        planner_payload = mock_planner_requests(timestep, args)
        requests, _ = normalize_tool_requests(
            planner_payload,
            max_tool_requests=args.max_tool_requests,
        )
        tool_results, raw_tools = execute_planner_requests(client, args, timestep, requests)
        return planner_payload, tool_results, raw_tools
    system, user = build_planner_prompt(
        timestep,
        max_candidates=args.max_candidates,
        max_tool_requests=args.max_tool_requests,
        working_memory=working_memory,
    )
    raw_text, _ = call_openai_compatible(
        base_url=args.base_url,
        api_key=args.api_key,
        model=args.model,
        system=system,
        user=user,
        temperature=args.temperature,
        timeout=args.planner_timeout,
        phase="planner_request",
    )
    try:
        planner_payload = extract_json_object(raw_text)
    except Exception as err:
        planner_payload = {
            "intent": "request_evidence",
            "requests": [],
            "reason": f"planner parse failed: {err}",
            "raw_text": raw_text,
        }
    requests, reason = normalize_tool_requests(
        planner_payload,
        max_tool_requests=args.max_tool_requests,
    )
    if not requests and reason:
        planner_payload["normalization_reason"] = reason
    tool_results, raw_tools = execute_planner_requests(client, args, timestep, requests)
    return planner_payload, tool_results, raw_tools

def planner_recommendation(
    args: argparse.Namespace,
    timestep: dict[str, Any],
    tool_results: list[dict[str, Any]],
    rng: random.Random,
    working_memory: dict[str, Any] | None = None,
) -> tuple[dict[str, Any], str, Any]:
    if args.provider == "mock":
        parsed = mock_choice(timestep, rng)
        return parsed, json.dumps(parsed, ensure_ascii=False), None
    system, user = build_recommendation_prompt(
        timestep,
        tool_results=tool_results,
        max_candidates=args.max_candidates,
        working_memory=working_memory,
    )
    raw_text, raw_payload = call_openai_compatible(
        base_url=args.base_url,
        api_key=args.api_key,
        model=args.model,
        system=system,
        user=user,
        temperature=args.temperature,
        timeout=args.timeout,
        phase="planner_recommendation",
    )
    return extract_json_object(raw_text), raw_text, raw_payload

