"""Prompt construction for planner and recommendation calls."""

from __future__ import annotations

from typing import Any

from sts_agent.context.action_candidates import action_candidate_policy_prompt_lines, build_action_descriptor_v1
from sts_agent.evidence.combat_search_view import combat_search_prompt_lines
from sts_agent.briefs.decision_brief import decision_brief_lines, decision_brief_v1
from sts_agent.utils.llm_utils import compact_json
from sts_agent.context.observation_context import observation_summary, public_observation_payload
from sts_agent.context.routing_policy import available_tool_specs, risk_flags
from sts_agent.journal.journal_events import tool_result_event_summary


def candidate_lines(candidates: list[dict[str, Any]]) -> list[str]:
    lines = []
    for candidate in candidates:
        payload = candidate.get("payload") or candidate
        descriptor = candidate.get("action_descriptor") or payload.get("semantic_descriptor") or {}
        descriptor_text = ""
        if isinstance(descriptor, dict) and descriptor:
            label = descriptor.get("label")
            status = descriptor.get("semantic_status")
            costs = descriptor.get("costs") or []
            effects = descriptor.get("effects") or []
            cost_text = compact_json(costs, limit=180) if costs else "[]"
            effect_text = compact_json(effects, limit=260) if effects else "[]"
            descriptor_text = (
                f" label={label!r} semantic_status={status} "
                f"costs={cost_text} effects={effect_text}"
            )
        card = payload.get("card")
        card_text = ""
        if isinstance(card, dict):
            card_text = (
                f" card={card.get('card_id')} cost={card.get('cost')} "
                f"damage={card.get('base_damage')} block={card.get('base_block')} "
                f"magic={card.get('base_magic')}"
            )
        lines.append(
            f"- id={candidate.get('id')} key={candidate.get('action_key')}"
            f"{descriptor_text}{card_text} action={compact_json(payload.get('action'), limit=450)}"
        )
    return lines

def build_prompt(timestep: dict[str, Any], *, max_candidates: int) -> tuple[str, str]:
    policy = timestep.get("action_candidate_policy") or {}
    candidates = (
        policy.get("decision_candidates")
        or timestep.get("decision_candidates")
        or timestep.get("candidates")
        or []
    )[:max_candidates]
    search_lines = combat_search_prompt_lines(timestep.get("combat_search_report"))
    policy_lines = action_candidate_policy_prompt_lines(policy)
    system = (
        "You are a cautious controller for a Slay the Spire simulator. "
        "Use only the public observation and the legal candidate list. "
        "Do not invent hidden information. Return exactly one JSON object with "
        'keys: "action_id" (integer), "confidence" ("low"|"medium"|"high"), '
        'and "reason" (short string).'
    )
    user = "\n".join(
        [
            "Choose the next legal action.",
            "",
            "Public state:",
            *observation_summary(timestep.get("observation") or {}),
            "",
            *search_lines,
            "",
            *policy_lines,
            "",
            "Decision candidates:",
            *candidate_lines(candidates),
            "",
            "Return strict JSON only. The action_id must be one of the listed ids.",
        ]
    )
    return system, user

def build_planner_prompt(
    timestep: dict[str, Any],
    *,
    max_candidates: int,
    max_tool_requests: int,
    working_memory: dict[str, Any] | None = None,
) -> tuple[str, str]:
    payload = public_observation_payload(timestep)
    decision_type = payload.get("decision_type")
    candidates = (timestep.get("candidates") or [])[:max_candidates]
    decision_brief = timestep.get("decision_brief")
    if not isinstance(decision_brief, dict):
        decision_brief = decision_brief_v1(
            payload,
            timestep.get("candidates") or [],
            working_memory,
        )
    brief_lines = decision_brief_lines(decision_brief, max_items=10)
    system = (
        "You are a planning agent for a Slay the Spire simulator harness. "
        "Use a ReAct-style loop: reason about what evidence is missing, request "
        "only allowed tools, then wait for harness observations. Do not invent "
        "tool results and do not choose an action in this phase. Return strict "
        "JSON with keys: intent, requests, reason. intent must be "
        "request_evidence or skip_tools. Use the DecisionBrief as an evidence "
        "contract when present: do not contradict known facts and do not treat "
        "unknown outcomes as known. "
        f"Use at most {max_tool_requests} requests."
    )
    user = "\n".join(
        [
            "Decide which simulator evidence to request before the final action.",
            "",
            f"Risk flags: {risk_flags(payload)}",
            "",
            "Working memory:",
            compact_json(working_memory or {}, limit=900),
            "",
            "Public state:",
            *observation_summary(timestep.get("observation") or {}),
            "",
            "DecisionBrief evidence contract:",
            *(brief_lines if brief_lines else ["none"]),
            "",
            "Legal candidates:",
            *candidate_lines(candidates),
            "",
            "Allowed tools:",
            compact_json(available_tool_specs(decision_type), limit=1200),
            "",
            "Return JSON only, for example:",
            '{"intent":"request_evidence","requests":[{"tool":"combat_turn_probe","question":"Find lethal/full-block lines."}],"reason":"Incoming damage may be dangerous."}',
        ]
    )
    return system, user

def build_recommendation_prompt(
    timestep: dict[str, Any],
    *,
    tool_results: list[dict[str, Any]],
    max_candidates: int,
    working_memory: dict[str, Any] | None = None,
) -> tuple[str, str]:
    candidates = (timestep.get("candidates") or [])[:max_candidates]
    decision_brief = timestep.get("decision_brief")
    if not isinstance(decision_brief, dict):
        decision_brief = decision_brief_v1(
            public_observation_payload(timestep),
            timestep.get("candidates") or [],
            working_memory,
        )
    brief_lines = decision_brief_lines(decision_brief, max_items=10)
    system = (
        "You are the live LLM controller for a Slay the Spire simulator harness. "
        "You own the semantic decision: tools provide evidence, not commands. "
        "Use the DecisionBrief as the primary evidence contract when present: do not "
        "contradict known facts, do not treat unknown outcomes as known, and label "
        "strategic judgments as judgments. A partial, random, or follow-up outcome is "
        "not a realized benefit; if you choose it, explain why its unresolved upside is "
        "worth more than known alternatives. If you reject a known resource-conversion "
        "line, address both its known upside and its cost. "
        "The harness validates schema and legal action_id, but in live mode it will "
        "not silently rescue bad tactical choices. In combat, treat hard_tactical_brief "
        "facts as high-priority simulator evidence: hp_delta, terminal defeat, "
        "critical HP, and forced-death branches are not optional flavor text. "
        "Use sampling_exhaustiveness_check before treating all_sampled_branches_defeat "
        "as proof that no legal survival line exists. If all sampled branches are fatal, "
        "use delaying_action_analysis to choose the least-bad legal action instead of "
        "defaulting to end_turn. Check potion_option_check in low-HP combat, "
        "rollout_depth_adequacy_check before trusting short-horizon conclusions, "
        "end_turn_commitment_check before ending the turn while non-end actions remain, "
        "and combat_evidence_conflict_resolver when tools disagree. "
        "Do not call an action safe if the brief says it causes large HP loss, "
        "critical HP, or defeat. Return strict JSON with action_id, confidence, "
        "chosen_tradeoff, used_brief_facts, why_not, uncertainty, reason, and optional plan_commitments."
    )
    user = "\n".join(
        [
            "Recommend the next legal action using the public state, legal candidates, and tool evidence.",
            "",
            "Working memory:",
            compact_json(working_memory or {}, limit=900),
            "",
            "Public state:",
            *observation_summary(timestep.get("observation") or {}),
            "",
            "DecisionBrief evidence contract:",
            *(brief_lines if brief_lines else ["none"]),
            "",
            "Legal candidates:",
            *candidate_lines(candidates),
            "",
            "Tool evidence:",
            compact_json(tool_results, limit=6500),
            "",
            "DecisionBrief comparison requirements:",
            "- If choosing a partial/random/follow-up option, explain why its unresolved upside beats the known alternatives.",
            "- If rejecting a known high-resource option, address its known upside and its cost.",
            "- Do not use Act 1 boss as the main immediate Floor-0 reason.",
            "- Do not claim unknown card/relic/shop outcomes as realized benefits.",
            "",
            "Return strict JSON only with this shape:",
            '{"action_id":0,"confidence":0.45,"chosen_tradeoff":"...","used_brief_facts":["..."],"why_not":{"1":"...","2":"...","3":"..."},"uncertainty":"...","reason":"...","plan_commitments":[]}',
            "The action_id must be one of the listed ids.",
        ]
    )
    return system, user

def build_tool_design_prompt(
    timestep: dict[str, Any],
    *,
    tool_results: list[dict[str, Any]],
    max_candidates: int,
    max_questions: int,
    working_memory: dict[str, Any] | None = None,
    final_action_key: str | None = None,
) -> tuple[str, str]:
    payload = public_observation_payload(timestep)
    decision_type = payload.get("decision_type")
    candidates = (timestep.get("candidates") or [])[:max_candidates]
    system = (
        "You are a harness tool-design observer for a Slay the Spire agent experiment. "
        "Your job is not to choose an action. Your job is to identify what information "
        "a planner would need next, whether current tools can answer it, and what new "
        "tool contract would be useful. Return strict JSON only."
    )
    user = "\n".join(
        [
            "Inspect this decision and propose high-value information needs.",
            "Do not invent tool results. Do not choose an action.",
            "",
            f"Return JSON with key questions, at most {max_questions} items.",
            "Each question item should include: question, why_action_relevant, current_tools_can_answer, missing_measurement, proposed_tool, priority_guess.",
            "current_tools_can_answer must be one of: yes, partially, no, unknown.",
            "proposed_tool should include: name, inputs, outputs.",
            "",
            f"Final action chosen by current controller: {final_action_key}",
            f"Risk flags: {risk_flags(payload)}",
            "",
            "Working memory:",
            compact_json(working_memory or {}, limit=700),
            "",
            "Public state:",
            *observation_summary(timestep.get("observation") or {}),
            "",
            "Legal candidates:",
            *candidate_lines(candidates),
            "",
            "Current allowed tools:",
            compact_json(available_tool_specs(decision_type), limit=1200),
            "",
            "Tool results already produced for this decision:",
            compact_json([tool_result_event_summary(result) for result in tool_results], limit=3000),
        ]
    )
    return system, user
