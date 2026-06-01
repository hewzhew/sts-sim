"""Default evidence tool request plans shared by routing and mock planner paths."""

from __future__ import annotations

import argparse
from typing import Any

from sts_agent.context.observation_context import public_observation_payload


def default_tool_action_ids(candidates: list[dict[str, Any]], *, limit: int = 3) -> list[int]:
    structural = [
        candidate
        for candidate in candidates
        if not str(candidate.get("action_key") or "").startswith(
            ("potion/", "discard_potion/")
        )
    ]
    source = structural if structural else candidates
    action_ids: list[int] = []
    for candidate in source[:limit]:
        try:
            action_ids.append(int(candidate["id"]))
        except (KeyError, TypeError, ValueError):
            continue
    return action_ids

def default_combat_lab_action_ids(candidates: list[dict[str, Any]], *, limit: int = 4) -> list[int]:
    play_actions = [
        candidate
        for candidate in candidates
        if str(candidate.get("action_key") or "").startswith("combat/play_card/")
    ]
    end_turn = [
        candidate
        for candidate in candidates
        if str(candidate.get("action_key") or "") == "combat/end_turn"
    ]
    potion_actions = [
        candidate
        for candidate in candidates
        if str(candidate.get("action_key") or "").startswith(
            ("combat/use_potion/", "potion/use/")
        )
    ]
    # Always include end_turn in the bounded combat lab sample when it is legal.
    # Also sample potion roots before extra card roots: potion lines are sparse,
    # high-leverage survival options, and otherwise get crowded out by card plays.
    ordered = end_turn + potion_actions + play_actions
    if not ordered:
        ordered = candidates
    action_ids: list[int] = []
    seen: set[int] = set()
    for candidate in ordered:
        if len(action_ids) >= limit:
            break
        try:
            action_id = int(candidate["id"])
        except (KeyError, TypeError, ValueError):
            continue
        if action_id in seen:
            continue
        seen.add(action_id)
        action_ids.append(action_id)
    return action_ids

def mock_planner_requests(timestep: dict[str, Any], args: argparse.Namespace) -> dict[str, Any]:
    payload = public_observation_payload(timestep)
    decision_type = payload.get("decision_type")
    requests: list[dict[str, Any]] = []
    if decision_type == "combat":
        requests.append(
            {
                "tool": "combat_turn_probe",
                "question": "Find lethal, full-block, and damage lines.",
            }
        )
        requests.append(
            {
                "tool": "combat_multi_turn_lab",
                "action_ids": default_combat_lab_action_ids(
                    timestep.get("candidates") or [],
                    limit=args.combat_lab_max_root_actions,
                ),
                "question": "Compare bounded multi-turn risks after plausible root combat actions.",
            }
        )
    elif decision_type == "campfire":
        requests.append(
            {
                "tool": "campfire_eval",
                "question": "Check rest priority and best smith target fit.",
            }
        )
    elif decision_type == "reward_card_choice":
        requests.extend(
            [
                {"tool": "deck_need_eval", "question": "Summarize deck gaps."},
                {"tool": "reward_card_eval", "question": "Match reward cards to deck gaps."},
            ]
        )
    elif decision_type == "map":
        requests.append(
            {
                "tool": "map_route_eval",
                "question": "Compare visible route risk.",
            }
        )
        if len(timestep.get("candidates") or []) > 1:
            requests.append(
                {
                    "tool": "decision_lab",
                    "action_ids": default_tool_action_ids(timestep.get("candidates") or []),
                    "question": "Branch-test visible routes for a few bounded routine/search steps.",
                }
            )
    elif decision_type == "shop":
        requests.extend(
            [
                {"tool": "deck_need_eval", "question": "Summarize deck gaps."},
                {"tool": "shop_purchase_eval", "question": "Compare shop purchases to deck gaps."},
            ]
        )
    elif len(timestep.get("candidates") or []) > 1:
        action_ids = default_tool_action_ids(timestep.get("candidates") or [])
        requests.append(
            {
                "tool": "decision_lab",
                "action_ids": action_ids,
                "question": "Compare bounded branch outcomes for top candidates.",
            }
        )
    return {
        "intent": "request_evidence",
        "requests": requests[: args.max_tool_requests],
        "reason": "mock planner requests bounded simulator evidence",
    }
