"""Compact rendering and diagnostics for planner tool results."""

from __future__ import annotations

import textwrap
from typing import Any

from sts_agent.runtime.action_selection import plan_by_name, plan_score, search_evidence
from sts_agent.evidence.card_eval import candidate_tool_descriptor

def compact_combat_probe_result(probe: dict[str, Any]) -> dict[str, Any]:
    plans = {}
    for name in ["Lethal", "FullBlock", "BlockEnoughThenDamage", "MaxDamage"]:
        plan = plan_by_name(probe, name)
        if plan:
            score = plan_score(plan)
            best_action_keys = plan.get("best_action_keys") or []
            plans[name] = {
                "first_action_key": (best_action_keys or [None])[0],
                "sequence_len": len(best_action_keys),
                "lethal_score": score.get("lethal_score"),
                "block_score": score.get("block_score"),
                "hp_loss_score": score.get("hp_loss_score"),
                "damage_score": score.get("damage_score"),
                "candidate_sequence_count": plan.get("candidate_sequence_count"),
            }
    return {
        "tool": "combat_turn_probe",
        "status": "ok",
        "schema_version": probe.get("schema_version"),
        "state_summary": search_evidence(
            probe=probe,
            selected_plan="not_selected",
            selected_action_key=None,
            fallback=False,
            fallback_reason=None,
        ).get("search_state_summary"),
        "plans": plans,
        "truth_warnings": probe.get("truth_warnings") or [],
        "probability_model": "not_implemented_v0",
        "worldline_model": "current_turn_sequence_search_only",
    }

def compact_afterstate_result(
    result: dict[str, Any],
    candidates: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    candidate_by_id = {}
    for candidate in candidates or []:
        try:
            candidate_by_id[int(candidate.get("id"))] = candidate
        except (TypeError, ValueError):
            continue
    summaries = []
    for item in result.get("summaries") or []:
        info = item.get("info") if isinstance(item, dict) else {}
        env_info = item.get("after_env_info") if isinstance(item, dict) else None
        if env_info is None and isinstance(info, dict):
            env_info = info.get("env_info")
        try:
            action_id = int(item.get("action_id"))
        except (TypeError, ValueError):
            action_id = item.get("action_id")
        candidate = candidate_by_id.get(action_id) if isinstance(action_id, int) else None
        summaries.append(
            {
                "action_id": action_id,
                "action_key": item.get("action_key"),
                "candidate": candidate_tool_descriptor(candidate),
                "ok": item.get("ok"),
                "reward": item.get("reward"),
                "done": item.get("done"),
                "env_info": env_info,
                "state_delta": item.get("state_delta"),
                "after_summary": item.get("after_summary"),
                "risk_flags_after": item.get("risk_flags_after") or [],
                "next_legal_action_count": item.get("next_legal_action_count"),
                "terminal": item.get("terminal"),
                "error": item.get("error"),
            }
        )
    return {
        "tool": "candidate_afterstate_summary",
        "status": "ok",
        "schema_version": result.get("schema_version"),
        "worldline_model": result.get("worldline_model"),
        "truth_warnings": result.get("truth_warnings") or [],
        "before_summary": result.get("before_summary"),
        "summaries": summaries,
    }

def compact_decision_lab_result(
    result: dict[str, Any],
    candidates: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    candidate_by_id = {}
    for candidate in candidates or []:
        try:
            candidate_by_id[int(candidate.get("id"))] = candidate
        except (TypeError, ValueError):
            continue
    branches = []
    for branch in result.get("branches") or []:
        if not isinstance(branch, dict):
            continue
        try:
            root_action_id = int(branch.get("root_action_id"))
        except (TypeError, ValueError):
            root_action_id = branch.get("root_action_id")
        candidate = candidate_by_id.get(root_action_id) if isinstance(root_action_id, int) else None
        root_afterstate = branch.get("root_afterstate") if isinstance(branch.get("root_afterstate"), dict) else {}
        rollout_steps = []
        for step in (branch.get("rollout_steps") or [])[:6]:
            if not isinstance(step, dict):
                continue
            afterstate = step.get("afterstate") if isinstance(step.get("afterstate"), dict) else {}
            rollout_steps.append(
                {
                    "rollout_index": step.get("rollout_index"),
                    "authority": step.get("authority"),
                    "reason": step.get("reason"),
                    "selected_plan": step.get("selected_plan"),
                    "action_id": step.get("action_id"),
                    "action_key": step.get("action_key"),
                    "state_delta": afterstate.get("state_delta"),
                    "risk_flags_after": afterstate.get("risk_flags_after") or [],
                    "terminal": afterstate.get("terminal"),
                    "next_legal_action_count": afterstate.get("next_legal_action_count"),
                }
            )
        branches.append(
            {
                "root_action_id": root_action_id,
                "root_action_key": branch.get("root_action_key"),
                "candidate": candidate_tool_descriptor(candidate),
                "ok": branch.get("ok"),
                "error": branch.get("error"),
                "root_state_delta": root_afterstate.get("state_delta"),
                "root_risk_flags_after": root_afterstate.get("risk_flags_after") or [],
                "rollout_step_count": branch.get("rollout_step_count"),
                "rollout_steps": rollout_steps,
                "stop_reason": branch.get("stop_reason"),
                "final_summary": branch.get("final_summary"),
                "final_risk_flags": branch.get("final_risk_flags") or [],
                "terminal": branch.get("terminal"),
            }
        )
    return {
        "tool": "decision_lab",
        "status": "ok",
        "schema_version": result.get("schema_version"),
        "worldline_model": result.get("worldline_model"),
        "probability_model": result.get("probability_model"),
        "truth_warnings": result.get("truth_warnings") or [],
        "max_rollout_steps": result.get("max_rollout_steps"),
        "branches": branches,
    }

def compact_event_delta(delta: Any) -> Any:
    if not isinstance(delta, dict):
        return delta
    preferred_keys = [
        "hp_delta",
        "player_hp_delta",
        "block_delta",
        "monster_hp_delta",
        "floor_delta",
        "gold_delta",
        "deck_size_delta",
        "relic_count_delta",
        "combat_ended",
        "death",
    ]
    compact = {key: delta.get(key) for key in preferred_keys if key in delta}
    if compact:
        return compact
    for key, value in delta.items():
        if len(compact) >= 6:
            break
        if isinstance(value, (str, int, float, bool)) or value is None:
            compact[str(key)] = value
    return compact

def compact_event_action_key(action_key: Any, *, width: int = 96) -> Any:
    if action_key is None:
        return None
    return textwrap.shorten(str(action_key), width=width, placeholder="...")

def compact_campfire_result(result: dict[str, Any]) -> dict[str, Any]:
    return {
        "tool": "campfire_rest_smith_eval",
        "status": "ok",
        "current_hp": result.get("current_hp"),
        "max_hp": result.get("max_hp"),
        "hp_ratio_milli": result.get("hp_ratio_milli"),
        "rest_legal": result.get("rest_legal"),
        "rest_candidates": result.get("rest_candidates") or [],
        "smith_count": len(result.get("smith_candidates") or []),
    }
