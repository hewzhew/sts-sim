"""Tool-design observation runtime and event helpers."""

from __future__ import annotations

import argparse
import json
import textwrap
from pathlib import Path
from typing import Any

from sts_agent.runtime.llm_provider import call_openai_compatible, extract_json_object
from sts_agent.utils.llm_utils import compact_json
from sts_agent.context.observation_context import public_observation_payload
from sts_agent.runtime.prompt_builders import build_tool_design_prompt


REPO_ROOT = Path(__file__).resolve().parents[4]


def mock_tool_design_payload(
    timestep: dict[str, Any],
    *,
    max_questions: int,
) -> dict[str, Any]:
    payload = public_observation_payload(timestep)
    decision_type = str(payload.get("decision_type") or "unknown")
    templates: dict[str, list[dict[str, Any]]] = {
        "map": [
            {
                "question": "Which visible route has the best risk/reward profile before the next forced threat?",
                "why_action_relevant": "Map choice changes future campfire, shop, elite, treasure, and combat exposure.",
                "current_tools_can_answer": "partially",
                "missing_measurement": "multi-floor route horizon with rest/shop/elite exposure and current deck risk",
                "proposed_tool": {
                    "name": "map_route_horizon_eval",
                    "inputs": ["visible_map", "current_hp", "deck_needs", "act_boss"],
                    "outputs": ["route_risk_score", "campfire_access", "shop_access", "elite_count", "forced_combat_count"],
                },
                "priority_guess": "high",
            }
        ],
        "reward": [
            {
                "question": "Which reward card improves the deck's next-act survival bottleneck rather than just matching static tags?",
                "why_action_relevant": "Reward choice can change future combat survival, boss preparation, and deck consistency.",
                "current_tools_can_answer": "partially",
                "missing_measurement": "deck archetype, boss risk, and marginal card impact over likely future fights",
                "proposed_tool": {
                    "name": "reward_card_marginal_eval",
                    "inputs": ["deck_cards", "visible_reward_cards", "act_boss", "route_context"],
                    "outputs": ["marginal_survival_value", "damage_value", "block_value", "scaling_value", "skip_value"],
                },
                "priority_guess": "high",
            }
        ],
        "reward_card_choice": [
            {
                "question": "Which reward card improves the deck's next-act survival bottleneck rather than just matching static tags?",
                "why_action_relevant": "Reward choice can change future combat survival, boss preparation, and deck consistency.",
                "current_tools_can_answer": "partially",
                "missing_measurement": "deck archetype, boss risk, and marginal card impact over likely future fights",
                "proposed_tool": {
                    "name": "reward_card_marginal_eval",
                    "inputs": ["deck_cards", "visible_reward_cards", "act_boss", "route_context"],
                    "outputs": ["marginal_survival_value", "damage_value", "block_value", "scaling_value", "skip_value"],
                },
                "priority_guess": "high",
            }
        ],
        "campfire": [
            {
                "question": "Is rest or smith better given expected damage before the boss and the best upgrade target?",
                "why_action_relevant": "Campfire choice trades immediate survival against future deck strength.",
                "current_tools_can_answer": "partially",
                "missing_measurement": "future route damage risk plus upgrade marginal value",
                "proposed_tool": {
                    "name": "campfire_survival_upgrade_eval",
                    "inputs": ["current_hp", "deck_cards", "visible_route_context", "act_boss"],
                    "outputs": ["rest_survival_gain", "best_smith_target", "smith_survival_risk", "recommendation_band"],
                },
                "priority_guess": "high",
            }
        ],
        "shop": [
            {
                "question": "Which purchase or removal gives the best marginal improvement under current gold and deck needs?",
                "why_action_relevant": "Shop choices trade scarce gold between cards, relics, potions, and removals.",
                "current_tools_can_answer": "partially",
                "missing_measurement": "price-aware marginal value with removals/relics/potions compared together",
                "proposed_tool": {
                    "name": "shop_purchase_marginal_eval",
                    "inputs": ["gold", "deck_cards", "shop_inventory", "route_context"],
                    "outputs": ["purchase_rankings", "removal_value", "potion_value", "skip_value"],
                },
                "priority_guess": "medium",
            }
        ],
        "combat": [
            {
                "question": "What are the likely next-turn risks after the current-turn search line?",
                "why_action_relevant": "Current-turn optimal damage/block may still create bad draw or survival risk next turn.",
                "current_tools_can_answer": "partially",
                "missing_measurement": "bounded multi-turn combat horizon with draw hypotheses",
                "proposed_tool": {
                    "name": "combat_multi_turn_lab",
                    "inputs": ["combat_state", "candidate_action_lines", "draw_pile_summary"],
                    "outputs": ["next_turn_risk", "expected_hp_loss", "draw_sensitive_failures", "branch_examples"],
                },
                "priority_guess": "high",
            }
        ],
        "event": [
            {
                "question": "What are the concrete outcomes and downstream risks of each event option?",
                "why_action_relevant": "Event options can trade HP, gold, cards, relics, or future route pressure.",
                "current_tools_can_answer": "partially",
                "missing_measurement": "event option outcome semantics and follow-up state risks",
                "proposed_tool": {
                    "name": "event_option_eval",
                    "inputs": ["event_id", "visible_options", "current_hp", "deck_cards"],
                    "outputs": ["hp_delta", "gold_delta", "card_delta", "relic_delta", "risk_notes"],
                },
                "priority_guess": "medium",
            }
        ],
    }
    questions = templates.get(decision_type, templates["event"])
    return {
        "questions": questions[:max_questions],
        "observer": "mock_tool_design_observer",
    }

def normalize_tool_design_questions(value: Any, *, max_questions: int) -> list[dict[str, Any]]:
    if not isinstance(value, dict):
        return []
    raw_questions = value.get("questions")
    if isinstance(raw_questions, dict):
        raw_questions = [raw_questions]
    if not isinstance(raw_questions, list):
        raw_questions = [value]
    questions = []
    for raw in raw_questions[:max_questions]:
        if not isinstance(raw, dict):
            continue
        proposed_tool = raw.get("proposed_tool")
        if not isinstance(proposed_tool, dict):
            proposed_tool = {}
        questions.append(
            {
                "question": textwrap.shorten(str(raw.get("question") or ""), width=260, placeholder="..."),
                "why_action_relevant": textwrap.shorten(str(raw.get("why_action_relevant") or ""), width=320, placeholder="..."),
                "current_tools_can_answer": str(raw.get("current_tools_can_answer") or "unknown"),
                "missing_measurement": textwrap.shorten(str(raw.get("missing_measurement") or ""), width=260, placeholder="..."),
                "proposed_tool": {
                    "name": textwrap.shorten(str(proposed_tool.get("name") or "unnamed_tool_need"), width=80, placeholder="..."),
                    "inputs": [str(item) for item in (proposed_tool.get("inputs") or [])[:12]],
                    "outputs": [str(item) for item in (proposed_tool.get("outputs") or [])[:12]],
                },
                "priority_guess": str(raw.get("priority_guess") or "unknown"),
            }
        )
    return questions

def should_observe_tool_design(
    timestep: dict[str, Any],
    *,
    args: argparse.Namespace,
    risk: list[str],
    tool_results: list[dict[str, Any]],
) -> bool:
    if args.tool_design_mode != "observe":
        return False
    payload = public_observation_payload(timestep)
    decision_type = payload.get("decision_type")
    candidates = timestep.get("candidates") or []
    candidate_count = len(candidates)
    action_keys = [str(candidate.get("action_key") or "") for candidate in candidates]
    structural_action_keys = [
        key for key in action_keys if not key.startswith(("potion/", "discard_potion/"))
    ]
    if any(result.get("status") in {"error", "refused"} for result in tool_results):
        return True
    if {"lethal_incoming", "low_hp", "campfire_low_hp"} & set(risk):
        return True
    if decision_type == "combat" and "possible_lethal_window" in set(risk):
        return True
    if decision_type == "map":
        return sum(1 for key in structural_action_keys if key.startswith("map/select")) >= 2
    if decision_type in {"event", "shop", "campfire"} and len(structural_action_keys) >= 2:
        return True
    if decision_type in {"reward", "reward_card_choice", "card_reward", "boss_reward"} and any(
        key.startswith("reward/select_card/") or key.startswith("card_reward/")
        for key in structural_action_keys
    ):
        return True
    for result in tool_results:
        if result.get("tool") == "decision_lab":
            for branch in result.get("branches") or []:
                if isinstance(branch, dict) and str(branch.get("stop_reason") or "").startswith("stopped_at_ambiguous"):
                    return True
    return False

def should_run_combat_lab_observer(
    timestep: dict[str, Any],
    *,
    risk: list[str],
) -> bool:
    payload = public_observation_payload(timestep)
    if payload.get("decision_type") != "combat":
        return False
    risk_set = set(risk)
    if {"lethal_incoming", "possible_lethal_window"} & risk_set:
        return True
    combat = payload.get("combat") if isinstance(payload.get("combat"), dict) else {}
    incoming = combat.get("visible_incoming_damage") or 0
    block = combat.get("player_block") or 0
    hp = combat.get("player_hp")
    unblocked = max(0, incoming - block)
    return isinstance(hp, int) and hp > 0 and unblocked * 100 >= hp * 35

def tool_design_type_budget(decision_type: Any) -> int:
    return {
        "combat": 2,
        "reward_card_choice": 3,
        "card_reward": 3,
        "boss_reward": 2,
        "reward": 2,
        "map": 2,
        "event": 2,
        "shop": 2,
        "campfire": 2,
    }.get(str(decision_type or "unknown"), 2)

def tool_design_phase(
    args: argparse.Namespace,
    timestep: dict[str, Any],
    *,
    tool_results: list[dict[str, Any]],
    working_memory: dict[str, Any],
    final_action_key: str | None,
    max_questions: int | None = None,
) -> tuple[list[dict[str, Any]], str | None, Any]:
    max_questions = max(1, max_questions or args.tool_design_max_questions)
    if args.provider == "mock":
        payload = mock_tool_design_payload(
            timestep,
            max_questions=max_questions,
        )
        return (
            normalize_tool_design_questions(
                payload,
                max_questions=max_questions,
            ),
            json.dumps(payload, ensure_ascii=False),
            None,
        )
    system, user = build_tool_design_prompt(
        timestep,
        tool_results=tool_results,
        max_candidates=args.max_candidates,
        max_questions=max_questions,
        working_memory=working_memory,
        final_action_key=final_action_key,
    )
    raw_text, raw_payload = call_openai_compatible(
        base_url=args.base_url,
        api_key=args.api_key,
        model=args.model,
        system=system,
        user=user,
        temperature=args.temperature,
        timeout=args.planner_timeout,
        phase="tool_design_observer",
    )
    parsed = extract_json_object(raw_text)
    return (
        normalize_tool_design_questions(
            parsed,
            max_questions=max_questions,
        ),
        raw_text,
        raw_payload,
    )

def build_tool_design_events(
    *,
    record: dict[str, Any],
    risk: list[str],
    questions: list[dict[str, Any]],
    raw_text: str | None,
    error: str | None = None,
) -> list[dict[str, Any]]:
    public_state = record.get("public_state_before") or {}
    base = {
        "schema_name": "PlannerQuestionEvent",
        "schema_version": 1,
        "event_type": "planner_question",
        "stream": "tool_design",
        "step": record.get("step_index"),
        "floor": public_state.get("floor"),
        "decision_type": record.get("decision_type"),
        "risk_flags": risk,
        "final_action_key": record.get("selected_action_key"),
        "executed_action_owner": record.get("executed_action_owner"),
        "information_boundary": "public_observation_plus_tool_summaries",
        "label_role": "not_a_label",
        "trainable_as_action_label": False,
        "policy_quality_claim": False,
        "human_review_status": "unreviewed",
    }
    if error is not None:
        event = dict(base)
        event.update(
            {
                "status": "error",
                "error": error,
            }
        )
        return [event]
    events = []
    for index, question in enumerate(questions):
        event = dict(base)
        event.update(
            {
                "status": "ok",
                "question_index": index,
                "question": question.get("question"),
                "why_action_relevant": question.get("why_action_relevant"),
                "current_tools_can_answer": question.get("current_tools_can_answer"),
                "missing_measurement": question.get("missing_measurement"),
                "proposed_tool": question.get("proposed_tool"),
                "priority_guess": question.get("priority_guess"),
            }
        )
        events.append(event)
    if not events and raw_text:
        event = dict(base)
        event.update(
            {
                "status": "empty",
                "error": "tool design observer returned no usable questions",
            }
        )
        events.append(event)
    return events

def default_tool_design_out_path(args: argparse.Namespace) -> Path:
    if args.tool_design_out is not None:
        return args.tool_design_out
    stem = args.out.stem or "run"
    return REPO_ROOT / "tools" / "artifacts" / "tool_design" / f"{stem}_questions.jsonl"
