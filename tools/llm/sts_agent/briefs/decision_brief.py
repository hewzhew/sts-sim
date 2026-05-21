"""Decision brief, decision lens, and lightweight plan-carryover helpers.

This module is intentionally evidence-only. It renders structured runtime
descriptors into prompt/watch context; it must not recommend actions or score
policies.
"""

from __future__ import annotations

import json
import re
import textwrap
from typing import Any


def compact_json(value: Any, *, limit: int = 900) -> str:
    text = json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    if len(text) <= limit:
        return text
    return text[: limit - 3] + "..."


def find_candidate(candidates: list[dict[str, Any]], action_id: int | None) -> dict[str, Any] | None:
    if action_id is None:
        return None
    for candidate in candidates:
        try:
            candidate_id = int(candidate.get("id"))
        except (TypeError, ValueError):
            continue
        if candidate_id == int(action_id):
            return candidate
    return None


def neow_option_by_effect(candidates: list[dict[str, Any]], effect_type: str) -> dict[str, Any] | None:
    for candidate in candidates:
        descriptor = candidate.get("action_descriptor") or (candidate.get("payload") or {}).get("semantic_descriptor") or {}
        effects = list(descriptor.get("effects") or []) + list(descriptor.get("costs") or [])
        for effect in effects:
            if isinstance(effect, dict) and effect.get("effect_type") == effect_type:
                return candidate
    return None


def effect_amount(effect: dict[str, Any]) -> Any:
    return effect.get("amount") if isinstance(effect, dict) else None


def render_effect_fact(
    effect: dict[str, Any],
    *,
    option_index: Any,
    role: str,
) -> str:
    effect_type = str(effect.get("effect_type") or "unknown_effect")
    amount = effect_amount(effect)
    kind = effect.get("kind")
    count = effect.get("count")
    prefix = f"Option {option_index} current {role}: "
    if effect_type == "gain_gold":
        return prefix + f"gain {amount} gold."
    if effect_type == "lose_max_hp":
        return prefix + f"lose {amount} max HP."
    if effect_type in {"lose_hp", "take_damage", "damage_player"}:
        return prefix + f"take {amount} immediate damage."
    if effect_type in {"gain_curse", "obtain_curse"}:
        return prefix + f"gain a curse{f' ({kind})' if kind else ''}."
    if effect_type in {"lose_gold", "lose_all_gold"}:
        if amount is None:
            return prefix + "lose all current gold."
        return prefix + f"lose {amount} gold."
    if effect_type == "lose_starter_relic":
        return prefix + "lose starter relic."
    if effect_type == "obtain_relic":
        detail = f"{kind}" if kind else "relic"
        if count and count != 1:
            detail = f"{count} {detail}"
        return prefix + f"obtain {detail}."
    if effect_type == "obtain_card":
        detail = f"{kind}" if kind else "card"
        if count and count != 1:
            detail = f"{count} {detail}"
        return prefix + f"obtain {detail}."
    if effect_type in {"remove_card", "transform_card", "upgrade_card"}:
        action = effect_type.replace("_", " ")
        if count and count != 1:
            return prefix + f"{action} x{count}."
        return prefix + f"{action}."
    return prefix + f"unrecognized effect_type={effect_type}; do not infer exact value beyond the descriptor."


def render_effect_implication(
    effect: dict[str, Any],
    *,
    option_index: Any,
    role: str,
    current_hp: Any,
    max_hp: Any,
    gold: Any,
) -> dict[str, Any] | None:
    effect_type = str(effect.get("effect_type") or "")
    amount = effect_amount(effect)
    if effect_type == "lose_max_hp":
        if isinstance(current_hp, int) and isinstance(max_hp, int) and isinstance(amount, int):
            new_max = max(0, max_hp - amount)
            if current_hp > new_max:
                return {
                    "text": (
                        f"Option {option_index} current cost loses {amount} max HP; "
                        f"at {current_hp}/{max_hp}, current HP may clamp to {new_max}/{new_max}."
                    ),
                    "authority": "game_rule_from_current_effect",
                }
            return {
                "text": (
                    f"Option {option_index} current cost loses {amount} max HP; "
                    "current HP is already below the new cap, so immediate HP may not change."
                ),
                "authority": "game_rule_from_current_effect",
            }
        return {
            "text": f"Option {option_index} current cost loses max HP; exact current-HP clamp depends on current HP/max HP.",
            "authority": "game_rule_from_current_effect",
        }
    if effect_type in {"lose_hp", "take_damage", "damage_player"}:
        return {
            "text": f"Option {option_index} current cost is immediate HP damage, not max HP loss.",
            "authority": "game_rule_from_current_effect",
        }
    if effect_type in {"gain_curse", "obtain_curse"}:
        return {
            "text": f"Option {option_index} current cost is deck-quality damage from a curse, not immediate HP loss.",
            "authority": "game_rule_from_current_effect",
        }
    if effect_type in {"lose_gold", "lose_all_gold"}:
        if isinstance(gold, int):
            return {
                "text": f"Option {option_index} current cost affects current gold; current gold before this option is {gold}.",
                "authority": "derived_from_simulator_exact",
            }
        return {
            "text": f"Option {option_index} current cost affects current gold.",
            "authority": "game_rule_from_current_effect",
        }
    if effect_type == "gain_gold" and isinstance(amount, int) and isinstance(gold, int):
        return {
            "text": f"Option {option_index} current effect would make total gold {gold + amount}.",
            "authority": "derived_from_simulator_exact",
        }
    if effect_type == "lose_starter_relic":
        return {
            "text": f"Option {option_index} current cost loses the starter relic; for Ironclad this can mean losing Burning Blood sustain if present.",
            "authority": "game_rule_from_current_effect",
        }
    return None


def decision_brief_v1(
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    working_memory: dict[str, Any] | None = None,
) -> dict[str, Any] | None:
    if public_payload.get("decision_type") == "map":
        return map_decision_brief_v1(public_payload, candidates, working_memory)
    screen = public_payload.get("screen") if isinstance(public_payload.get("screen"), dict) else {}
    event_options = [
        option for option in screen.get("event_options") or []
        if isinstance(option, dict)
    ]
    is_neow = any(
        ((option.get("semantic_descriptor") or {}).get("event_name") == "Neow")
        or ((option.get("semantic_descriptor") or {}).get("event_id") == "Neow")
        for option in event_options
    )
    if not is_neow:
        return None

    current_hp = public_payload.get("current_hp")
    max_hp = public_payload.get("max_hp")
    gold = public_payload.get("gold")
    boss = public_payload.get("act_boss")
    known_facts: list[dict[str, Any]] = [
        {"text": "This is the Neow start bonus at Act 1 Floor 0.", "authority": "simulator_exact"},
        {"text": f"Current HP is {current_hp}/{max_hp}.", "authority": "simulator_exact"},
        {"text": f"Current gold is {gold}.", "authority": "simulator_exact"},
        {"text": f"Act 1 boss is {boss}.", "authority": "simulator_exact"},
    ]
    unknowns: list[dict[str, Any]] = []
    relevant_implications: list[dict[str, Any]] = []
    forbidden_assumptions: list[str] = [
        "Do not assume a follow-up card choice gives a good card unless the offered cards are shown.",
        "Do not assume a random relic or boss relic outcome unless the simulator exposes it.",
        "Do not treat the Act 1 boss as immediate Floor-0 damage pressure.",
        "Do not say max HP loss is harmless when current HP is at max HP.",
        "Do not invent exact shop contents or cross-character card rewards.",
    ]

    for option in event_options:
        descriptor = option.get("semantic_descriptor") or {}
        label = descriptor.get("label") or option.get("label") or f"option {option.get('option_index')}"
        option_index = option.get("option_index")
        status = descriptor.get("semantic_status")
        known_facts.append(
            {
                "text": f"Option {option_index}: {label} (status={status}).",
                "authority": "simulator_exact",
            }
        )
        costs = descriptor.get("costs") or []
        effects = descriptor.get("effects") or []
        for cost in costs:
            if not isinstance(cost, dict):
                continue
            known_facts.append(
                {
                    "text": render_effect_fact(cost, option_index=option_index, role="cost"),
                    "authority": "simulator_exact",
                }
            )
            implication = render_effect_implication(
                cost,
                option_index=option_index,
                role="cost",
                current_hp=current_hp,
                max_hp=max_hp,
                gold=gold,
            )
            if implication:
                relevant_implications.append(implication)
        for effect in effects:
            if not isinstance(effect, dict):
                continue
            known_facts.append(
                {
                    "text": render_effect_fact(effect, option_index=option_index, role="effect"),
                    "authority": "simulator_exact",
                }
            )
            implication = render_effect_implication(
                effect,
                option_index=option_index,
                role="effect",
                current_hp=current_hp,
                max_hp=max_hp,
                gold=gold,
            )
            if implication:
                relevant_implications.append(implication)
        if status == "partial":
            unknown_fields = descriptor.get("unknown_fields") or []
            label_lower = str(label).lower()
            if "choose" in label_lower:
                unknowns.append(
                    {
                        "text": f"Option {option_index} opens a follow-up card choice; exact offered cards are unknown now.",
                        "authority": "simulator_exact_unknown",
                    }
                )
            elif any("relic" in str(field) for field in unknown_fields) or "relic" in label_lower:
                unknowns.append(
                    {
                        "text": f"Option {option_index} has an exact relic outcome that is unknown now.",
                        "authority": "simulator_exact_unknown",
                    }
                )
            elif unknown_fields:
                unknowns.append(
                    {
                        "text": f"Option {option_index} has unresolved fields: {', '.join(str(field) for field in unknown_fields[:4])}.",
                        "authority": "simulator_exact_unknown",
                    }
                )

    route_context = public_payload.get("map_route_context") if isinstance(public_payload.get("map_route_context"), dict) else {}
    route_choices = [
        choice for choice in (route_context or {}).get("route_choices") or []
        if isinstance(choice, dict)
    ]
    earliest_shop_choices = [
        choice for choice in route_choices
        if choice.get("earliest_shop_floor") is not None
    ]
    if earliest_shop_choices:
        best_shop = min(earliest_shop_choices, key=lambda choice: int(choice.get("earliest_shop_floor") or 999))
        relevant_implications.append(
            {
                "text": f"Route {best_shop.get('action_key')} reaches the earliest shop on floor {best_shop.get('earliest_shop_floor')}, so gold can become early purchasable power if that route is taken.",
                "authority": "derived_from_map",
            }
        )
    if route_choices:
        optional_elites = all(int(choice.get("min_elites") or 0) == 0 for choice in route_choices)
        guaranteed_recovery = all(
            ((choice.get("risk_vector") or {}).get("recovery_access") == "guaranteed")
            for choice in route_choices
            if isinstance(choice.get("risk_vector"), dict)
        )
        if optional_elites:
            relevant_implications.append(
                {
                    "text": "Shown route starts have optional elites; early elite risk is route-controllable rather than forced.",
                    "authority": "derived_from_map",
                }
            )
        if guaranteed_recovery:
            relevant_implications.append(
                {
                    "text": "Rest before the boss is reachable on all shown starts.",
                    "authority": "derived_from_map",
                }
            )
    if boss:
        relevant_implications.append(
            {
                "text": f"{boss} is a long-term Act 1 build constraint, not immediate Floor-0 damage pressure.",
                "authority": "game_rule",
            }
        )

    return {
        "schema_name": "DecisionBrief",
        "schema_version": 1,
        "brief_kind": "neow_start_bonus",
        "decision_authority": "evidence_only",
        "not_final_action": True,
        "label_role": "not_a_label",
        "policy_quality_claim": False,
        "known_facts": known_facts,
        "unknowns": unknowns,
        "relevant_implications": relevant_implications,
        "forbidden_assumptions": forbidden_assumptions,
        "prompt_notes": [
            "Use the DecisionBrief as evidence, not as a recommendation.",
            "You may make strategic judgments, but label uncertainty and do not contradict known facts.",
        ],
        "truth_warnings": [
            "brief_does_not_recommend_an_action",
            "derived implications are evidence organization, not policy labels",
        ],
    }


def map_decision_brief_v1(
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    working_memory: dict[str, Any] | None = None,
) -> dict[str, Any] | None:
    route_context = public_payload.get("map_route_context") if isinstance(public_payload.get("map_route_context"), dict) else {}
    route_choices = [
        choice for choice in (route_context or {}).get("route_choices") or []
        if isinstance(choice, dict)
    ]
    if not route_choices:
        return None
    known_facts: list[dict[str, Any]] = [
        {"text": "This is a map route choice.", "authority": "simulator_exact"},
        {"text": f"Current HP is {public_payload.get('current_hp')}/{public_payload.get('max_hp')}.", "authority": "simulator_exact"},
        {"text": f"Current gold is {public_payload.get('gold')}.", "authority": "simulator_exact"},
    ]
    unknowns: list[dict[str, Any]] = []
    relevant_implications: list[dict[str, Any]] = []
    forbidden_assumptions: list[str] = [
        "Do not treat the safest immediate route as automatically best if it abandons the previous decision's stated payoff.",
        "Do not treat a previous plan commitment as mandatory; explain if you choose to deviate.",
        "Do not interpret unweighted path counts as route probabilities.",
    ]
    for choice in route_choices:
        risk_vector = choice.get("risk_vector") if isinstance(choice.get("risk_vector"), dict) else {}
        known_facts.append(
            {
                "text": (
                    f"Route {choice.get('action_key')}: first_shop={choice.get('earliest_shop_floor')}, "
                    f"forced_fights_next_3={choice.get('forced_fights_next_3')}, "
                    f"early_pressure={risk_vector.get('early_pressure')}, "
                    f"recovery={risk_vector.get('recovery_access')}."
                ),
                "authority": "derived_from_map",
            }
        )
    carryovers = [
        item for item in ((working_memory or {}).get("plan_commitments") or [])
        if isinstance(item, dict) and item.get("kind") == "early_shop_conversion"
    ]
    if carryovers:
        carry = carryovers[-1]
        target = carry.get("target_hint")
        relevant_implications.append(
            {
                "text": (
                    "Previous decision established an early-shop conversion hook: "
                    + str(carry.get("reason") or "convert newly gained gold into power")
                    + (f" Target hint: {target}." if target else "")
                ),
                "authority": "llm_prior_decision_carryover",
            }
        )
        target_choice = next((choice for choice in route_choices if choice.get("action_key") == target), None)
        if target_choice:
            relevant_implications.append(
                {
                    "text": (
                        f"{target} is the carryover route and has first_shop={target_choice.get('earliest_shop_floor')} "
                        f"with forced_fights_next_3={target_choice.get('forced_fights_next_3')}."
                    ),
                    "authority": "derived_from_map",
                }
            )
        safer_choices = [
            choice for choice in route_choices
            if ((choice.get("risk_vector") or {}).get("early_pressure") == "low")
        ]
        if safer_choices:
            safe = safer_choices[0]
            relevant_implications.append(
                {
                    "text": (
                        f"Safer immediate route {safe.get('action_key')} has early_pressure=low "
                        f"and first_shop={safe.get('earliest_shop_floor')}; choosing it may delay gold conversion."
                    ),
                    "authority": "derived_from_map",
                }
            )
        forbidden_assumptions.append(
            "If rejecting the carryover route, explicitly explain why delayed gold conversion is acceptable."
        )
    return {
        "schema_name": "DecisionBrief",
        "schema_version": 1,
        "brief_kind": "map_route_choice",
        "decision_authority": "evidence_only",
        "not_final_action": True,
        "label_role": "not_a_label",
        "policy_quality_claim": False,
        "known_facts": known_facts,
        "unknowns": unknowns,
        "relevant_implications": relevant_implications,
        "forbidden_assumptions": forbidden_assumptions,
        "prompt_notes": [
            "Use carryover only as context, not as a forced route.",
            "If deviating from a previous hook, make the tradeoff explicit.",
        ],
        "truth_warnings": [
            "brief_does_not_recommend_an_action",
            "carryover_is_not_a_forced_action",
            "route_path_counts_are_unweighted_graph_paths",
        ],
    }


def decision_brief_lines(brief: dict[str, Any] | None, *, max_items: int = 8) -> list[str]:
    if not isinstance(brief, dict):
        return []
    lines = [
        f"Decision brief: {brief.get('brief_kind')} authority={brief.get('decision_authority')} not_final_action={brief.get('not_final_action')}",
    ]
    for title, key in [
        ("known", "known_facts"),
        ("unknown", "unknowns"),
        ("implication", "relevant_implications"),
    ]:
        items = [item for item in brief.get(key) or [] if isinstance(item, dict)]
        for item in items[:max_items]:
            lines.append(f"  {title}: {item.get('text')} [{item.get('authority')}]")
    forbidden = [str(item) for item in brief.get("forbidden_assumptions") or []]
    for item in forbidden[:max_items]:
        lines.append(f"  forbidden: {item}")
    return lines


def _brief_texts(brief: dict[str, Any] | None, key: str) -> list[str]:
    if not isinstance(brief, dict):
        return []
    return [
        str(item.get("text") or "")
        for item in brief.get(key) or []
        if isinstance(item, dict) and item.get("text")
    ]


def decision_lens_v1(brief: dict[str, Any] | None) -> dict[str, Any] | None:
    """Human watch lens mechanically derived from DecisionBrief.

    This is UI only: it must not add recommendations, scores, or facts that are
    absent from the underlying brief.
    """
    if not isinstance(brief, dict):
        return None
    known = _brief_texts(brief, "known_facts")
    unknown = _brief_texts(brief, "unknowns")
    implications = _brief_texts(brief, "relevant_implications")
    tensions: list[dict[str, str]] = []
    caveats: list[dict[str, str]] = []
    if brief.get("brief_kind") == "neow_start_bonus":
        total_gold = next(
            (text for text in (known + implications) if "total gold" in text),
            None,
        )
        early_shop = next((text for text in implications if "earliest shop" in text), None)
        unknown_options = []
        for text in unknown:
            match = re.search(r"Option\s+(\d+)", text)
            if match:
                unknown_options.append(match.group(1))
        if total_gold or early_shop:
            known_parts = []
            if total_gold:
                known_parts.append(total_gold.rstrip("."))
            if early_shop:
                known_parts.append(early_shop.rstrip("."))
            unresolved = (
                " vs unresolved option"
                + "/".join(dict.fromkeys(unknown_options))
                + " follow-up/random outcomes"
                if unknown_options
                else ""
            )
            tensions.append(
                {
                    "label": "known_resource_vs_unknown_outcomes",
                    "text": "; ".join(known_parts) + unresolved + ".",
                }
            )
        hp_caveat = next((text for text in implications if "max HP" in text and "clamp" in text), None)
        boss_caveat = next((text for text in implications if "immediate Floor-0 damage" in text), None)
        if hp_caveat:
            caveats.append({"label": "hp_cost", "text": hp_caveat.rstrip(".") + "."})
        if boss_caveat:
            caveats.append({"label": "boss_timing", "text": boss_caveat.rstrip(".") + "."})
        return {
            "schema_name": "DecisionLens",
            "schema_version": 1,
            "kind": brief.get("brief_kind"),
            "decision_authority": "display_only",
            "not_final_action": True,
            "tensions": tensions[:3],
            "caveats": caveats[:3],
        }
    if brief.get("brief_kind") == "map_route_choice":
        carryover = next(
            (text for text in implications if "early-shop conversion hook" in text),
            None,
        )
        target = next(
            (text for text in implications if "carryover route" in text),
            None,
        )
        safer = next(
            (text for text in implications if "Safer immediate route" in text),
            None,
        )
        if carryover:
            tensions.append({"label": "plan_carryover", "text": carryover})
        if target and safer:
            tensions.append(
                {
                    "label": "conversion_vs_safety",
                    "text": target.rstrip(".") + " " + safer.rstrip(".") + ".",
                }
            )
        elif target:
            tensions.append({"label": "carryover_route", "text": target})
        if not tensions:
            for text in known[:3]:
                tensions.append({"label": "route_option", "text": text})
        return {
            "schema_name": "DecisionLens",
            "schema_version": 1,
            "kind": brief.get("brief_kind"),
            "decision_authority": "display_only",
            "not_final_action": True,
            "tensions": tensions[:3],
            "caveats": caveats[:3],
        }
    for text in implications[:2]:
        tensions.append({"label": "brief_implication", "text": text})
    if unknown:
        caveats.append(
            {
                "label": "unknowns",
                "text": "Unknowns: " + "; ".join(item.rstrip(".") for item in unknown[:2]) + ".",
            }
        )
    return {
        "schema_name": "DecisionLens",
        "schema_version": 1,
        "kind": brief.get("brief_kind"),
        "decision_authority": "display_only",
        "not_final_action": True,
        "tensions": tensions[:3],
        "caveats": caveats[:3],
    }


def decision_lens_lines(brief: dict[str, Any] | None) -> list[str]:
    lens = decision_lens_v1(brief)
    if not isinstance(lens, dict):
        return []
    lines: list[str] = []
    for item in lens.get("tensions") or []:
        if isinstance(item, dict) and item.get("text"):
            lines.append(f"Tension[{item.get('label')}]: {item.get('text')}")
    for item in lens.get("caveats") or []:
        if isinstance(item, dict) and item.get("text"):
            lines.append(f"Caveat[{item.get('label')}]: {item.get('text')}")
    return lines[:4]


def print_full_decision_brief(brief: dict[str, Any] | None) -> None:
    lines = decision_brief_lines(brief, max_items=100)
    if not lines:
        print("No DecisionBrief for this decision.", flush=True)
        return
    print("\n" + "-" * 104, flush=True)
    for line in lines:
        print(line, flush=True)
    print("-" * 104, flush=True)


def print_decision_brief_prompt_section(brief: dict[str, Any] | None) -> None:
    lines = decision_brief_lines(brief, max_items=10)
    if not lines:
        print("No DecisionBrief prompt section for this decision.", flush=True)
        return
    print("\nDecisionBrief evidence contract:", flush=True)
    for line in lines:
        print(line, flush=True)
    print("", flush=True)


def llm_decision_summary_lines(parsed: dict[str, Any] | None, action_id: int | None) -> list[str]:
    if not isinstance(parsed, dict):
        return ["LLM decision: unavailable"]
    lines = [
        "LLM decision: "
        + f"action={parsed.get('action_id', action_id)} "
        + f"confidence={parsed.get('confidence')}"
    ]
    tradeoff = parsed.get("chosen_tradeoff")
    if tradeoff:
        lines.append(
            "  tradeoff: "
            + textwrap.shorten(str(tradeoff), width=180, placeholder="...")
        )
    used = parsed.get("used_brief_facts")
    if isinstance(used, list) and used:
        lines.append(
            "  used: "
            + "; ".join(textwrap.shorten(str(item), width=60, placeholder="...") for item in used[:5])
        )
    uncertainty = parsed.get("uncertainty")
    if uncertainty:
        lines.append(
            "  uncertainty: "
            + textwrap.shorten(str(uncertainty), width=180, placeholder="...")
        )
    commitments = parsed.get("plan_commitments")
    if isinstance(commitments, list) and commitments:
        hooks = []
        for item in commitments[:2]:
            if not isinstance(item, dict):
                continue
            hook = str(item.get("kind") or "commitment")
            if item.get("target_hint"):
                hook += f" -> {item.get('target_hint')}"
            if item.get("strength"):
                hook += f" ({item.get('strength')})"
            hooks.append(hook)
        if hooks:
            lines.append("  next hook: " + "; ".join(hooks))
    reason = parsed.get("reason")
    if reason:
        lines.append(
            "  reason: "
            + textwrap.shorten(str(reason), width=220, placeholder="...")
        )
    return lines


def print_full_llm_json(parsed: dict[str, Any] | None) -> None:
    print("\n" + "-" * 104, flush=True)
    print("LLM raw JSON:", flush=True)
    print(compact_json(parsed or {}, limit=10000), flush=True)
    print("-" * 104, flush=True)


def route_choice_by_action_key(public_payload: dict[str, Any], action_key: str | None) -> dict[str, Any] | None:
    if not action_key:
        return None
    context = public_payload.get("map_route_context") if isinstance(public_payload.get("map_route_context"), dict) else {}
    for choice in context.get("route_choices") or []:
        if isinstance(choice, dict) and choice.get("action_key") == action_key:
            return choice
    return None


def infer_plan_commitments(
    *,
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    parsed: dict[str, Any] | None,
    action_id: int | None,
) -> list[dict[str, Any]]:
    if not isinstance(parsed, dict):
        return []
    text = " ".join(
        str(parsed.get(key) or "")
        for key in ["chosen_tradeoff", "uncertainty", "reason"]
    ).lower()
    if "shop" not in text or "gold" not in text:
        return []
    selected = find_candidate(candidates, action_id)
    descriptor = (selected or {}).get("action_descriptor") or ((selected or {}).get("payload") or {}).get("semantic_descriptor") or {}
    effects = descriptor.get("effects") or []
    if not any(isinstance(effect, dict) and effect.get("effect_type") == "gain_gold" for effect in effects):
        return []
    route_context = public_payload.get("map_route_context") if isinstance(public_payload.get("map_route_context"), dict) else {}
    route_choices = [
        choice for choice in route_context.get("route_choices") or []
        if isinstance(choice, dict) and choice.get("earliest_shop_floor") is not None
    ]
    if not route_choices:
        return []
    best_shop = min(route_choices, key=lambda choice: int(choice.get("earliest_shop_floor") or 999))
    return [
        {
            "kind": "early_shop_conversion",
            "source": "llm_tradeoff_from_selected_gain_gold_action",
            "source_action_id": action_id,
            "source_action_key": (selected or {}).get("action_key"),
            "target_hint": best_shop.get("action_key"),
            "target_first_shop_floor": best_shop.get("earliest_shop_floor"),
            "target_forced_fights_next_3": best_shop.get("forced_fights_next_3"),
            "strength": "medium",
            "reason": "selected gain-gold Neow option was justified by early shop conversion",
            "not_forced_action": True,
        }
    ]


