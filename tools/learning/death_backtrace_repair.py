#!/usr/bin/env python3
"""Build a single-run death backtrace repair report from a full-run trace.

This tool is intentionally not a policy, teacher, or action-label generator.
It reads one completed run trace and explains:

* where the run died,
* what the death combat looked like,
* which earlier non-combat choices are relevant repair candidates,
* whether the immediate failure looks like combat execution, run preparation,
  resource conversion, or a mixture.

The output is a repair record for one seed. It should be used to choose the next
counterfactual experiment, not to declare a preferred action.
"""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


INTANGIBLE_CARDS = {"Apparition"}
DEFENSE_CARDS_HINTS = {
    "Defend",
    "ShrugItOff",
    "PowerThrough",
    "FlameBarrier",
    "Impervious",
    "TrueGrit",
    "Entrench",
    "SecondWind",
    "Disarm",
    "Shockwave",
    "Clothesline",
}
DAMAGE_CARDS_HINTS = {
    "Strike",
    "PommelStrike",
    "Bash",
    "Cleave",
    "Carnage",
    "Bludgeon",
    "BloodForBlood",
    "Feed",
    "Rampage",
    "WildStrike",
    "Anger",
    "HeavyBlade",
    "Headbutt",
    "ThunderClap",
}
SETUP_CARDS_HINTS = {
    "Inflame",
    "DemonForm",
    "Barricade",
    "DarkEmbrace",
    "Evolve",
    "FireBreathing",
    "Rupture",
    "Corruption",
    "FeelNoPain",
}


def safe_int(value: Any, default: int = 0) -> int:
    try:
        if value is None:
            return default
        return int(value)
    except (TypeError, ValueError):
        return default


def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        if value is None:
            return default
        return float(value)
    except (TypeError, ValueError):
        return default


def action_key(step: dict[str, Any]) -> str:
    return str(step.get("chosen_action_key") or "")


def candidate_action_key(candidate: dict[str, Any]) -> str:
    return str(candidate.get("action_key") or "")


def chosen_candidate(step: dict[str, Any]) -> dict[str, Any] | None:
    key = action_key(step)
    for candidate in step.get("action_mask") or []:
        if candidate_action_key(candidate) == key:
            return candidate
    idx = step.get("chosen_action_index")
    if idx is not None:
        for candidate in step.get("action_mask") or []:
            if candidate.get("action_index") == idx:
                return candidate
    return None


def card_id_from_candidate(candidate: dict[str, Any] | None) -> str | None:
    if not isinstance(candidate, dict):
        return None
    card = candidate.get("card")
    if isinstance(card, dict) and card.get("card_id"):
        return str(card.get("card_id"))
    return None


def card_id_from_action_key(key: str) -> str | None:
    match = re.search(r"card:([^/]+)", key)
    return match.group(1) if match else None


def card_id_from_step(step: dict[str, Any]) -> str | None:
    return card_id_from_candidate(chosen_candidate(step)) or card_id_from_action_key(action_key(step))


def compact_card(card: dict[str, Any]) -> dict[str, Any]:
    return {
        "card_id": card.get("card_id"),
        "upgrades": safe_int(card.get("upgrades")),
        "cost": safe_int(card.get("cost")),
        "base_damage": safe_int(card.get("base_damage")),
        "base_block": safe_int(card.get("base_block")),
        "draws_cards": bool(card.get("draws_cards")),
        "exhaust": bool(card.get("exhaust")),
        "ethereal": bool(card.get("ethereal")),
        "applies_weak": bool(card.get("applies_weak")),
        "applies_vulnerable": bool(card.get("applies_vulnerable")),
        "scaling_piece": bool(card.get("scaling_piece")),
    }


def deck_cards(step: dict[str, Any]) -> list[dict[str, Any]]:
    obs = step.get("observation") or {}
    cards = []
    for item in obs.get("deck_cards") or []:
        card = item.get("card") if isinstance(item, dict) else None
        if isinstance(card, dict):
            cards.append(compact_card(card))
    return cards


def relic_ids(step: dict[str, Any]) -> list[str]:
    obs = step.get("observation") or {}
    out = []
    for relic in obs.get("relics") or []:
        if isinstance(relic, dict) and relic.get("relic_id"):
            suffix = "(used)" if relic.get("used_up") else ""
            out.append(f"{relic.get('relic_id')}{suffix}")
    return out


def potion_ids(step: dict[str, Any]) -> list[str]:
    obs = step.get("observation") or {}
    out = []
    for potion in obs.get("potions") or []:
        if isinstance(potion, dict) and potion.get("potion_id"):
            out.append(str(potion.get("potion_id")))
    return out


def combat(step: dict[str, Any]) -> dict[str, Any]:
    return ((step.get("observation") or {}).get("combat")) or {}


def combat_hand_ids(step: dict[str, Any]) -> list[str]:
    out = []
    for item in combat(step).get("hand_cards") or []:
        if isinstance(item, dict) and item.get("card_id"):
            out.append(str(item.get("card_id")))
    return out


def legal_play_card_ids(step: dict[str, Any]) -> list[str]:
    out = []
    for candidate in step.get("action_mask") or []:
        key = candidate_action_key(candidate)
        if not key.startswith("combat/play_card"):
            continue
        card_id = card_id_from_candidate(candidate) or card_id_from_action_key(key)
        if card_id:
            out.append(card_id)
    return out


def monsters_summary(step: dict[str, Any]) -> list[dict[str, Any]]:
    out = []
    for monster in combat(step).get("monsters") or []:
        if not isinstance(monster, dict):
            continue
        out.append(
            {
                "slot": monster.get("slot"),
                "monster_id": monster.get("monster_id"),
                "hp": safe_int(monster.get("hp")),
                "block": safe_int(monster.get("block")),
                "incoming": safe_int(monster.get("visible_incoming_damage")),
                "move_id": monster.get("planned_move_id"),
                "alive": bool(monster.get("alive")),
            }
        )
    return out


def action_family_from_key(key: str) -> str:
    if key.startswith("combat/end_turn"):
        return "combat_end_turn"
    if key.startswith("combat/play_card"):
        card = card_id_from_action_key(key)
        if card in INTANGIBLE_CARDS:
            return "combat_play_intangible"
        if card in DEFENSE_CARDS_HINTS:
            return "combat_play_defense_or_control"
        if card in SETUP_CARDS_HINTS:
            return "combat_play_setup"
        if card in DAMAGE_CARDS_HINTS:
            return "combat_play_damage"
        return "combat_play_card"
    if key.startswith("combat/use_potion"):
        return "combat_use_potion"
    if key.startswith("reward/select_card"):
        return "card_reward_pick"
    if key == "proceed":
        return "proceed"
    if key.startswith("reward/claim"):
        return "claim_reward"
    if key.startswith("event/choice"):
        return "event_choice"
    if key.startswith("campfire/rest"):
        return "campfire_rest"
    if key.startswith("campfire/smith"):
        return "campfire_smith"
    if key.startswith("map/select"):
        return "map_select"
    if key.startswith("selection/deck"):
        return "deck_selection"
    if key.startswith("boss_relic"):
        return "boss_relic"
    return key.split("/", 1)[0] if key else "unknown"


def classify_card_role(card_id: str | None, candidate: dict[str, Any] | None = None) -> str:
    if not card_id:
        return "none"
    if card_id in INTANGIBLE_CARDS:
        return "intangible"
    if card_id in DEFENSE_CARDS_HINTS:
        return "defense_or_control"
    if card_id in SETUP_CARDS_HINTS:
        return "setup_or_scaling"
    if card_id in DAMAGE_CARDS_HINTS:
        return "damage"
    card = candidate.get("card") if isinstance(candidate, dict) else None
    if isinstance(card, dict):
        if safe_int(card.get("base_block")) > 0 or card.get("applies_weak"):
            return "defense_or_control"
        if safe_int(card.get("base_damage")) > 0:
            return "damage"
        if card.get("draws_cards"):
            return "draw"
        if card.get("scaling_piece"):
            return "setup_or_scaling"
    return "other"


def find_death_combat_steps(steps: list[dict[str, Any]]) -> list[dict[str, Any]]:
    combat_steps = [s for s in steps if s.get("decision_type") == "combat"]
    if not combat_steps:
        return []
    death_floor = combat_steps[-1].get("floor")
    return [s for s in combat_steps if s.get("floor") == death_floor]


def summarize_death_combat(death_steps: list[dict[str, Any]]) -> dict[str, Any]:
    if not death_steps:
        return {"timeline": [], "flags": [], "turns": {}}

    timeline = []
    flags = []
    skipped_intangible = []
    lethal_end_turns = []
    attack_under_pressure = []
    turns: dict[str, dict[str, Any]] = {}

    for step in death_steps:
        c = combat(step)
        key = action_key(step)
        family = action_family_from_key(key)
        hp = safe_int(c.get("player_hp"))
        block = safe_int(c.get("player_block"))
        incoming = safe_int(c.get("visible_incoming_damage"))
        energy = safe_int(c.get("energy"))
        turn = safe_int(c.get("turn_count"))
        hand = combat_hand_ids(step)
        gap = max(0, incoming - block)
        lethal_after_block = gap >= hp and incoming > 0
        legal_cards = legal_play_card_ids(step)
        has_intangible = any(card in INTANGIBLE_CARDS for card in legal_cards)
        played_intangible = family == "combat_play_intangible"
        played_attack = family == "combat_play_damage"
        row_flags = []
        if has_intangible and gap >= 6 and not played_intangible:
            row_flags.append("intangible_available_not_played_under_incoming")
            skipped_intangible.append(step.get("step_index"))
        if lethal_after_block and family == "combat_end_turn":
            row_flags.append("ended_turn_into_lethal_incoming")
            lethal_end_turns.append(step.get("step_index"))
        if played_attack and gap >= 10 and not played_intangible:
            row_flags.append("attack_played_under_large_block_gap")
            attack_under_pressure.append(step.get("step_index"))

        timeline.append(
            {
                "step": step.get("step_index"),
                "turn": turn,
                "hp": hp,
                "block": block,
                "incoming": incoming,
                "block_gap": gap,
                "energy": energy,
                "hand": hand,
                "legal_cards": legal_cards,
                "monsters": monsters_summary(step),
                "chosen_action_key": key,
                "chosen_card": card_id_from_step(step),
                "action_family": family,
                "flags": row_flags,
            }
        )

        turn_key = str(turn)
        t = turns.setdefault(
            turn_key,
            {
                "turn": turn,
                "first_step": step.get("step_index"),
                "start_hp": hp,
                "max_incoming": 0,
                "actions": [],
                "cards_seen": set(),
                "flags": set(),
            },
        )
        t["max_incoming"] = max(t["max_incoming"], incoming)
        t["actions"].append(key)
        t["cards_seen"].update(hand)
        t["flags"].update(row_flags)

    for t in turns.values():
        t["cards_seen"] = sorted(t["cards_seen"])
        t["flags"] = sorted(t["flags"])

    if skipped_intangible:
        flags.append(
            {
                "tag": "combat_execution_intangible_not_allocated",
                "evidence_steps": skipped_intangible,
                "interpretation": "Intangible card was visible while incoming damage exceeded block, but the chosen action did not play it.",
            }
        )
    if lethal_end_turns:
        flags.append(
            {
                "tag": "combat_execution_ended_into_lethal",
                "evidence_steps": lethal_end_turns,
                "interpretation": "The final end-turn had lethal incoming after current block.",
            }
        )
    if attack_under_pressure:
        flags.append(
            {
                "tag": "combat_execution_attack_under_defense_pressure",
                "evidence_steps": attack_under_pressure,
                "interpretation": "Damage actions were taken while the current turn still had a large block gap.",
            }
        )

    return {"timeline": timeline, "turns": turns, "flags": flags}


def summarize_card_reward(step: dict[str, Any]) -> dict[str, Any]:
    candidates = []
    chosen_key = action_key(step)
    for candidate in step.get("action_mask") or []:
        key = candidate_action_key(candidate)
        card = card_id_from_candidate(candidate)
        if key.startswith("reward/select_card") or key == "proceed":
            candidates.append(
                {
                    "action_key": key,
                    "card_id": card,
                    "role": classify_card_role(card, candidate),
                    "chosen": key == chosen_key,
                    "debug_score": candidate.get("rule_policy_debug_score"),
                }
            )
    return {
        "step": step.get("step_index"),
        "floor": step.get("floor"),
        "hp": step.get("hp"),
        "max_hp": step.get("max_hp"),
        "chosen_action_key": chosen_key,
        "chosen_card": card_id_from_step(step),
        "candidates": candidates,
    }


def summarize_campfire(step: dict[str, Any]) -> dict[str, Any]:
    candidates = []
    chosen_key = action_key(step)
    deck = deck_cards(step)
    for candidate in step.get("action_mask") or []:
        key = candidate_action_key(candidate)
        if key.startswith("campfire/rest"):
            label = "Rest"
        elif key.startswith("campfire/smith"):
            idx = safe_int(key.rsplit("/", 1)[-1], -1)
            card = deck[idx]["card_id"] if 0 <= idx < len(deck) else None
            label = f"Smith {card}" if card else key
        else:
            continue
        candidates.append(
            {
                "action_key": key,
                "display": label,
                "chosen": key == chosen_key,
                "debug_score": candidate.get("rule_policy_debug_score"),
            }
        )
    return {
        "step": step.get("step_index"),
        "floor": step.get("floor"),
        "hp": step.get("hp"),
        "max_hp": step.get("max_hp"),
        "chosen_action_key": chosen_key,
        "candidates": candidates[:12],
        "candidate_count": len(candidates),
    }


def summarize_event(step: dict[str, Any]) -> dict[str, Any]:
    candidates = []
    chosen_key = action_key(step)
    for candidate in step.get("action_mask") or []:
        event = candidate.get("event_option")
        if not isinstance(event, dict):
            continue
        candidates.append(
            {
                "action_key": candidate_action_key(candidate),
                "event_id": event.get("event_id"),
                "text": event.get("text"),
                "effects": event.get("effects") or [],
                "chosen": candidate_action_key(candidate) == chosen_key,
                "debug_score": candidate.get("rule_policy_debug_score"),
            }
        )
    return {
        "step": step.get("step_index"),
        "floor": step.get("floor"),
        "hp": step.get("hp"),
        "max_hp": step.get("max_hp"),
        "gold": step.get("gold"),
        "chosen_action_key": chosen_key,
        "candidates": candidates,
    }


def summarize_deck_selection(step: dict[str, Any]) -> dict[str, Any]:
    candidates = []
    chosen_key = action_key(step)
    for candidate in step.get("action_mask") or []:
        sel = candidate.get("deck_selection")
        if not isinstance(sel, dict):
            continue
        cards = []
        for item in sel.get("selected_cards") or []:
            if isinstance(item, dict):
                cards.append(item.get("card_id"))
        candidates.append(
            {
                "action_key": candidate_action_key(candidate),
                "reason": sel.get("reason"),
                "selected_cards": cards,
                "chosen": candidate_action_key(candidate) == chosen_key,
                "debug_score": candidate.get("rule_policy_debug_score"),
            }
        )
    return {
        "step": step.get("step_index"),
        "floor": step.get("floor"),
        "hp": step.get("hp"),
        "max_hp": step.get("max_hp"),
        "gold": step.get("gold"),
        "chosen_action_key": chosen_key,
        "candidates": candidates[:20],
        "candidate_count": len(candidates),
    }


def summarize_map(step: dict[str, Any]) -> dict[str, Any]:
    chosen_key = action_key(step)
    candidates = []
    for candidate in step.get("action_mask") or []:
        route = candidate.get("map_route")
        row = {
            "action_key": candidate_action_key(candidate),
            "chosen": candidate_action_key(candidate) == chosen_key,
            "debug_score": candidate.get("rule_policy_debug_score"),
        }
        if isinstance(route, dict):
            row["map_route"] = route
        candidates.append(row)
    return {
        "step": step.get("step_index"),
        "floor": step.get("floor"),
        "hp": step.get("hp"),
        "max_hp": step.get("max_hp"),
        "gold": step.get("gold"),
        "chosen_action_key": chosen_key,
        "candidate_count": len(candidates),
        "candidates": candidates,
    }


def collect_history(steps: list[dict[str, Any]], death_floor: int) -> dict[str, Any]:
    before_death = [s for s in steps if safe_int(s.get("floor")) < death_floor]
    card_rewards = [summarize_card_reward(s) for s in before_death if s.get("decision_type") == "reward_card_choice"]
    campfires = [summarize_campfire(s) for s in before_death if s.get("decision_type") == "campfire"]
    events = [summarize_event(s) for s in before_death if s.get("decision_type") == "event"]
    deck_selections = [
        summarize_deck_selection(s) for s in before_death if s.get("decision_type") == "run_deck_selection"
    ]
    maps = [summarize_map(s) for s in before_death if s.get("decision_type") == "map"]
    shops = [s for s in before_death if s.get("decision_type") == "shop"]
    return {
        "card_rewards": card_rewards,
        "campfires": campfires,
        "events": events,
        "deck_selections": deck_selections,
        "map_choices": maps,
        "shop_decision_count": len(shops),
    }


def deck_role_counts(cards: list[dict[str, Any]]) -> dict[str, int]:
    counts: Counter[str] = Counter()
    ids: Counter[str] = Counter()
    for card in cards:
        card_id = str(card.get("card_id"))
        ids[card_id] += 1
        counts[classify_card_role(card_id)] += 1
        if card.get("draws_cards"):
            counts["draw"] += 1
        if card.get("exhaust"):
            counts["exhaust"] += 1
        if safe_int(card.get("upgrades")) > 0:
            counts["upgraded"] += 1
    counts["total"] = len(cards)
    counts["starter_strike_defend"] = sum(ids.get(x, 0) for x in ("Strike", "Defend"))
    return dict(sorted(counts.items()))


def derive_repair_targets(report: dict[str, Any]) -> list[dict[str, Any]]:
    targets = []
    death = report["death"]
    history = report["history"]
    combat_flags = {flag["tag"] for flag in report["death_combat"].get("flags", [])}
    deck_counts = report["death"]["entry_deck_role_counts"]

    if "combat_execution_intangible_not_allocated" in combat_flags:
        targets.append(
            {
                "target_id": "combat_floor32_intangible_timing_plan",
                "domain": "combat_plan",
                "priority": "high",
                "basis": "Death combat had visible Apparition under incoming damage and baseline did not play it.",
                "next_experiment": "Search full combat plan from boss entry; compare complete plans, not one-step action swaps.",
                "not_a_label": True,
            }
        )
    if "combat_execution_ended_into_lethal" in combat_flags:
        targets.append(
            {
                "target_id": "combat_floor32_lethal_end_turn_backtrace",
                "domain": "combat_plan",
                "priority": "high",
                "basis": "Final end turn entered lethal incoming after current block.",
                "next_experiment": "Backtrack same combat to the last turn where survival resource was available.",
                "not_a_label": True,
            }
        )

    if safe_int(death.get("gold")) >= 300 and history.get("shop_decision_count", 0) == 0:
        targets.append(
            {
                "target_id": "route_gold_conversion_gap",
                "domain": "route_or_shop_access",
                "priority": "medium",
                "basis": f"Entered death floor with {death.get('gold')} gold and no shop decision was observed before death.",
                "next_experiment": "Check full-act map opportunity windows for reachable shops after gold became high.",
                "not_a_label": True,
            }
        )

    rest_count = sum(1 for c in history.get("campfires", []) if str(c.get("chosen_action_key")).startswith("campfire/rest"))
    smith_count = sum(1 for c in history.get("campfires", []) if str(c.get("chosen_action_key")).startswith("campfire/smith"))
    if rest_count > smith_count:
        targets.append(
            {
                "target_id": "campfire_hp_pressure_loop",
                "domain": "campfire_or_prior_damage",
                "priority": "medium",
                "basis": f"Rest count {rest_count} exceeded smith count {smith_count}; this may be forced by earlier HP loss, not necessarily campfire policy.",
                "next_experiment": "For each rest, run branch replay from prior combat and campfire alternatives separately.",
                "not_a_label": True,
            }
        )

    if deck_counts.get("setup_or_scaling", 0) >= 3 and deck_counts.get("defense_or_control", 0) >= 8:
        targets.append(
            {
                "target_id": "deck_has_tools_execution_failed",
                "domain": "combat_execution",
                "priority": "high",
                "basis": "Deck at death had control/defense tools and Apparitions; immediate failure looks more like execution/timing than missing all defensive cards.",
                "next_experiment": "Do not first change card rewards; first prove whether a combat plan can survive the boss with the existing deck.",
                "not_a_label": True,
            }
        )

    return targets


def build_report(trace: dict[str, Any], trace_file: Path) -> dict[str, Any]:
    steps = trace.get("steps") or []
    if not steps:
        raise ValueError("trace has no steps")
    summary = trace.get("summary") or {}
    last = steps[-1]
    death_steps = find_death_combat_steps(steps)
    entry = death_steps[0] if death_steps else last
    death_floor = safe_int(entry.get("floor"))
    cards = deck_cards(entry)

    report = {
        "schema_version": "death_backtrace_repair_v0",
        "trace_file": str(trace_file),
        "run": {
            "seed": (trace.get("config") or {}).get("seed"),
            "policy": (trace.get("config") or {}).get("policy"),
            "summary": summary,
        },
        "death": {
            "act": entry.get("act"),
            "floor": death_floor,
            "entry_step": entry.get("step_index"),
            "last_step": last.get("step_index"),
            "entry_hp": entry.get("hp"),
            "entry_max_hp": entry.get("max_hp"),
            "gold": entry.get("gold"),
            "deck_size": entry.get("deck_size"),
            "entry_deck_role_counts": deck_role_counts(cards),
            "deck_cards": cards,
            "relics": relic_ids(entry),
            "potions": potion_ids(entry),
            "entry_monsters": monsters_summary(entry),
        },
        "death_combat": summarize_death_combat(death_steps),
        "history": collect_history(steps, death_floor),
    }
    report["repair_targets"] = derive_repair_targets(report)
    return report


def write_markdown(report: dict[str, Any], path: Path) -> None:
    death = report["death"]
    run = report["run"]
    lines = []
    lines.append("# Death Backtrace Repair Report")
    lines.append("")
    lines.append(f"- trace: `{report['trace_file']}`")
    lines.append(f"- seed: `{run.get('seed')}`")
    lines.append(f"- policy: `{run.get('policy')}`")
    lines.append(f"- death: Act `{death.get('act')}` floor `{death.get('floor')}`")
    lines.append(
        f"- entry: step `{death.get('entry_step')}`, HP `{death.get('entry_hp')}/{death.get('entry_max_hp')}`, gold `{death.get('gold')}`, deck `{death.get('deck_size')}`"
    )
    lines.append(f"- relics: `{', '.join(death.get('relics') or [])}`")
    lines.append(f"- potions: `{', '.join(death.get('potions') or []) or 'none'}`")
    lines.append(f"- deck role counts: `{death.get('entry_deck_role_counts')}`")
    lines.append("")

    lines.append("## Immediate Combat Diagnosis")
    flags = report["death_combat"].get("flags") or []
    if flags:
        for flag in flags:
            lines.append(f"- `{flag['tag']}` at steps `{flag.get('evidence_steps')}`: {flag.get('interpretation')}")
    else:
        lines.append("- no combat execution flags detected by v0 parser")
    lines.append("")

    lines.append("## Death Combat Timeline")
    lines.append("")
    lines.append("| step | turn | hp | block | incoming | energy | chosen | hand | flags |")
    lines.append("|---:|---:|---:|---:|---:|---:|---|---|---|")
    for row in report["death_combat"].get("timeline") or []:
        hand = ", ".join(row.get("hand") or [])
        flags_s = ", ".join(row.get("flags") or [])
        lines.append(
            f"| {row.get('step')} | {row.get('turn')} | {row.get('hp')} | {row.get('block')} | {row.get('incoming')} | {row.get('energy')} | `{row.get('chosen_action_key')}` | {hand} | {flags_s} |"
        )
    lines.append("")

    lines.append("## Repair Targets")
    targets = report.get("repair_targets") or []
    if not targets:
        lines.append("- no repair target emitted by v0 parser")
    for target in targets:
        lines.append(
            f"- `{target['target_id']}` [{target['domain']}, {target['priority']}]: {target['basis']} Next: {target['next_experiment']}"
        )
    lines.append("")

    lines.append("## Prior Card Reward Choices")
    for row in report["history"].get("card_rewards") or []:
        candidate_s = ", ".join(
            f"{'*' if c.get('chosen') else ''}{c.get('card_id') or 'skip'}:{c.get('role')}({c.get('debug_score')})"
            for c in row.get("candidates") or []
        )
        lines.append(f"- step `{row['step']}` floor `{row['floor']}` chose `{row.get('chosen_card')}` from {candidate_s}")
    lines.append("")

    lines.append("## Prior Campfires")
    for row in report["history"].get("campfires") or []:
        chosen = next((c for c in row.get("candidates") or [] if c.get("chosen")), None)
        lines.append(
            f"- step `{row['step']}` floor `{row['floor']}` HP `{row['hp']}/{row['max_hp']}` chose `{row.get('chosen_action_key')}` ({(chosen or {}).get('display')}) among `{row.get('candidate_count')}` options"
        )
    lines.append("")

    lines.append("## Not Labels")
    lines.append("- This report does not mark any action as a winner/preference/action label.")
    lines.append("- Repair targets are counterfactual experiments to run next.")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trace-file", type=Path, required=True)
    parser.add_argument("--out-json", type=Path, required=True)
    parser.add_argument("--out-md", type=Path)
    args = parser.parse_args()

    trace = json.loads(args.trace_file.read_text(encoding="utf-8"))
    report = build_report(trace, args.trace_file)
    args.out_json.parent.mkdir(parents=True, exist_ok=True)
    args.out_json.write_text(json.dumps(report, indent=2, sort_keys=True), encoding="utf-8")
    if args.out_md:
        write_markdown(report, args.out_md)
    print(
        json.dumps(
            {
                "schema_version": report["schema_version"],
                "seed": report["run"].get("seed"),
                "death": {
                    "act": report["death"].get("act"),
                    "floor": report["death"].get("floor"),
                    "entry_hp": report["death"].get("entry_hp"),
                    "entry_max_hp": report["death"].get("entry_max_hp"),
                },
                "combat_flags": [flag["tag"] for flag in report["death_combat"].get("flags") or []],
                "repair_targets": [target["target_id"] for target in report.get("repair_targets") or []],
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
