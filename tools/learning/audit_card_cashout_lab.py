#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter
from datetime import datetime, timezone
from math import comb
from pathlib import Path
from statistics import mean
from typing import Any

from combat_rl_common import REPO_ROOT, write_json


REPORT_VERSION = "card_cashout_lab_v0_10"
SCORE_KIND = "heuristic_cashout_v0_10"
MIN_ABSOLUTE_CASHOUT_FOR_REGRET = 20.0

PLAN_FIELDS = (
    "frontload_delta",
    "block_delta",
    "draw_delta",
    "scaling_delta",
    "aoe_delta",
    "exhaust_delta",
    "kill_window_delta",
)

KILL_WINDOW_CARDS = {"Feed", "HandOfGreed", "RitualDagger"}
EXHAUST_ENGINE_CARDS = {
    "TrueGrit",
    "SecondWind",
    "FiendFire",
    "BurningPact",
    "Corruption",
    "FeelNoPain",
    "DarkEmbrace",
    "Exhume",
}
RELIABLE_EXHAUST_OUTLET_CARDS = {
    "TrueGrit",
    "SecondWind",
    "FiendFire",
    "BurningPact",
    "Corruption",
    "Exhume",
}
STATUS_BURDEN_CARDS = {
    "PowerThrough": 2,
    "WildStrike": 1,
    "RecklessCharge": 1,
}
STATUS_PAYOFF_CARDS = {"Evolve", "FireBreathing", "MedKit"}
ENERGY_CARDS = {"Offering", "SeeingRed", "Bloodletting", "Dropkick", "Sentinel"}
RESOURCE_WINDOW_CARDS = {"Offering", "SeeingRed", "Bloodletting"}
VULNERABLE_SOURCE_CARDS = {"Bash", "Uppercut", "Shockwave", "ThunderClap", "Trip"}
CORRUPTION_PAYOFF_CARDS = {"FeelNoPain", "DarkEmbrace", "Barricade", "BodySlam", "Juggernaut"}
MULTI_ENEMY_CONTROL_CARDS = {"Shockwave", "ThunderClap"}
AOE_DAMAGE_CARDS = {"Immolate", "Cleave", "Whirlwind", "Reaper"}
MULTI_HIT_NOT_AOE_CARDS = {"TwinStrike", "SwordBoomerang", "Pummel"}
HIGH_VALUE_RELIC_WARNINGS = {
    "BagOfPreparation": "draw_relic_not_exactly_modeled",
    "RingOfTheSnake": "draw_relic_not_exactly_modeled",
    "SneckoEye": "cost_randomization_not_modeled",
    "RunicPyramid": "retention_not_modeled",
    "VelvetChoker": "action_budget_context_modeled_coarsely",
    "LetterOpener": "skill_chain_context_modeled_coarsely",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Audit reward-card choices with query-conditioned relevance, hypergeometric "
            "reachability, and bucket EV. This is diagnostic attribution, not a teacher."
        )
    )
    parser.add_argument(
        "--trace-dir",
        action="append",
        default=[],
        metavar="POLICY=PATH",
        help="Saved full-run trace directory. Can be repeated.",
    )
    parser.add_argument("--min-gap", type=float, default=25.0)
    parser.add_argument("--top-cases", type=int, default=30)
    parser.add_argument("--opening-hand-size", type=int, default=5)
    parser.add_argument("--turn2-seen-cards", type=int, default=10)
    parser.add_argument(
        "--out",
        type=Path,
        default=REPO_ROOT
        / "tools"
        / "artifacts"
        / "card_cashout_lab"
        / "cashout_report.json",
    )
    parser.add_argument("--markdown-out", type=Path)
    parser.add_argument("--top-cases-out", type=Path)
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run formula smoke checks and exit.",
    )
    return parser.parse_args()


def parse_named_paths(values: list[str], label: str) -> dict[str, Path]:
    out: dict[str, Path] = {}
    for raw in values:
        if "=" not in raw:
            raise SystemExit(f"{label} must use POLICY=PATH, got {raw!r}")
        name, path = raw.split("=", 1)
        name = name.strip()
        if not name:
            raise SystemExit(f"{label} has empty policy name: {raw!r}")
        out[name] = Path(path.strip())
    return out


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def trace_files(path: Path) -> list[Path]:
    files = sorted(path.glob("episode_*.json"))
    if not files:
        files = sorted(path.rglob("episode_*.json"))
    if not files:
        raise SystemExit(f"no episode_*.json traces found in {path}")
    return files


def chosen_candidate(step: dict[str, Any]) -> dict[str, Any]:
    candidate = step.get("chosen_candidate")
    if isinstance(candidate, dict) and candidate:
        return candidate
    candidates = step.get("action_mask") or []
    index = int(step.get("chosen_action_index") or 0)
    if 0 <= index < len(candidates) and isinstance(candidates[index], dict):
        return candidates[index]
    key = str(step.get("chosen_action_key") or "")
    for candidate in candidates:
        if isinstance(candidate, dict) and str(candidate.get("action_key") or "") == key:
            return candidate
    return {}


def is_card_reward_step(step: dict[str, Any]) -> bool:
    if str(step.get("decision_type") or "") == "reward_card_choice":
        return True
    return any(
        isinstance(candidate, dict)
        and str(candidate.get("action_key") or "").startswith("reward/select_card/")
        and isinstance(candidate.get("card"), dict)
        for candidate in step.get("action_mask") or []
    )


def candidate_actions(step: dict[str, Any]) -> list[dict[str, Any]]:
    rows = []
    for candidate in step.get("action_mask") or []:
        if not isinstance(candidate, dict):
            continue
        key = str(candidate.get("action_key") or "")
        card = candidate.get("card") or {}
        if not key.startswith("reward/select_card/") or not card:
            continue
        rows.append(candidate)
    return rows


def selected_action(step: dict[str, Any], rows: list[dict[str, Any]]) -> dict[str, Any] | None:
    key = str(step.get("chosen_action_key") or "")
    if key == "proceed":
        return None
    for row in rows:
        if str(row.get("action_key") or "") == key:
            return row
    candidate = chosen_candidate(step)
    card = candidate.get("card") or {}
    if not card:
        return None
    card_id = str(card.get("card_id") or "")
    for row in rows:
        if str((row.get("card") or {}).get("card_id") or "") == card_id:
            return row
    return None


def observation(step: dict[str, Any]) -> dict[str, Any]:
    return step.get("observation") or {}


def deck_card_features(obs: dict[str, Any]) -> list[dict[str, Any]]:
    out = []
    for item in obs.get("deck_cards") or []:
        if not isinstance(item, dict):
            continue
        card = item.get("card") or {}
        if isinstance(card, dict) and card.get("card_id"):
            out.append(card)
    return out


def relic_ids(obs: dict[str, Any]) -> set[str]:
    return {
        str(relic.get("relic_id") or "")
        for relic in obs.get("relics") or []
        if isinstance(relic, dict) and relic.get("relic_id")
    }


def card_id(card: dict[str, Any]) -> str:
    return str(card.get("card_id") or "")


def card_cost(card: dict[str, Any]) -> int:
    try:
        return int(card.get("cost") if card.get("cost") is not None else 0)
    except (TypeError, ValueError):
        return 0


def card_damage(card: dict[str, Any]) -> int:
    return max(int(card.get("upgraded_damage") or card.get("base_damage") or 0), 0)


def card_block(card: dict[str, Any]) -> int:
    return max(int(card.get("upgraded_block") or card.get("base_block") or 0), 0)


def plan_delta(candidate: dict[str, Any]) -> dict[str, int]:
    delta = candidate.get("plan_delta") or {}
    return {field: int(delta.get(field) or 0) for field in PLAN_FIELDS} | {
        "deck_deficit_bonus": int(delta.get("deck_deficit_bonus") or 0),
        "bloat_penalty": int(delta.get("bloat_penalty") or 0),
        "duplicate_penalty": int(delta.get("duplicate_penalty") or 0),
        "setup_cashout_risk_delta": int(delta.get("setup_cashout_risk_delta") or 0),
        "plan_adjusted_score": int(delta.get("plan_adjusted_score") or 0),
    }


def candidate_rule_score(candidate: dict[str, Any]) -> float:
    return float((candidate.get("card") or {}).get("rule_score") or 0)


def candidate_plan_score(candidate: dict[str, Any]) -> float:
    delta = candidate.get("plan_delta") or {}
    return float(delta.get("plan_adjusted_score", candidate_rule_score(candidate)) or 0)


def is_draw(card: dict[str, Any]) -> bool:
    return bool(card.get("draws_cards"))


def is_scaling(card: dict[str, Any]) -> bool:
    return bool(card.get("scaling_piece"))


def is_aoe(card: dict[str, Any]) -> bool:
    cid = card_id(card)
    if cid in MULTI_HIT_NOT_AOE_CARDS:
        return False
    if cid in AOE_DAMAGE_CARDS or cid in MULTI_ENEMY_CONTROL_CARDS:
        return True
    return bool(card.get("aoe"))


def is_aoe_damage(card: dict[str, Any]) -> bool:
    return is_aoe(card) and card_damage(card) > 0


def is_multi_enemy_control(card: dict[str, Any]) -> bool:
    cid = card_id(card)
    if cid in MULTI_ENEMY_CONTROL_CARDS:
        return True
    return is_aoe(card) and (
        bool(card.get("applies_weak")) or bool(card.get("applies_vulnerable"))
    )


def is_energy(card: dict[str, Any]) -> bool:
    return bool(card.get("gains_energy")) or card_id(card) in ENERGY_CARDS


def is_exhaust_engine(card: dict[str, Any]) -> bool:
    return bool(card.get("exhaust")) or card_id(card) in EXHAUST_ENGINE_CARDS


def is_exhaust_outlet(card: dict[str, Any]) -> bool:
    return is_reliable_exhaust_outlet(card)


def is_reliable_exhaust_outlet(card: dict[str, Any]) -> bool:
    return card_id(card) in RELIABLE_EXHAUST_OUTLET_CARDS


def status_burden(card: dict[str, Any]) -> int:
    return STATUS_BURDEN_CARDS.get(card_id(card), 0)


def has_status_payoff(deck_cards: list[dict[str, Any]]) -> bool:
    return any(card_id(card) in STATUS_PAYOFF_CARDS for card in deck_cards)


def is_kill_window(card: dict[str, Any]) -> bool:
    return card_id(card) in KILL_WINDOW_CARDS


def is_junk(card: dict[str, Any]) -> bool:
    return int(card.get("card_type_id") or 0) in {4, 5}


def card_classes(card: dict[str, Any]) -> set[str]:
    classes: set[str] = set()
    if is_junk(card):
        classes.add("junk_status")
    if card_damage(card) > 0:
        classes.add("generic_attack")
    if card_block(card) > 0:
        classes.add("generic_block")
    if is_draw(card):
        classes.add("generic_draw")
    if is_scaling(card):
        classes.add("generic_scaling")
    if is_aoe_damage(card):
        classes.add("generic_aoe_damage")
    if is_multi_enemy_control(card):
        classes.add("generic_multi_enemy_control")
    if is_energy(card):
        classes.add("generic_energy")
    if is_exhaust_engine(card):
        classes.add("exhaust_engine")
    if is_kill_window(card):
        classes.add("kill_window")
    if not classes:
        classes.add("utility_or_low_signal")
    return classes


def primary_class(card: dict[str, Any]) -> str:
    priority = [
        "kill_window",
        "generic_aoe_damage",
        "generic_multi_enemy_control",
        "generic_scaling",
        "generic_draw",
        "generic_energy",
        "exhaust_engine",
        "generic_block",
        "generic_attack",
        "junk_status",
        "utility_or_low_signal",
    ]
    classes = card_classes(card)
    for item in priority:
        if item in classes:
            return item
    return "utility_or_low_signal"


def hypergeom_at_least_one(population: int, successes: int, draws: int) -> float:
    population = max(int(population), 0)
    successes = min(max(int(successes), 0), population)
    draws = min(max(int(draws), 0), population)
    if population <= 0 or successes <= 0 or draws <= 0:
        return 0.0
    if draws > population - successes:
        return 1.0
    return 1.0 - comb(population - successes, draws) / comb(population, draws)


def hypergeom_at_least_one_each(
    population: int, successes_a: int, successes_b: int, draws: int
) -> float:
    population = max(int(population), 0)
    successes_a = min(max(int(successes_a), 0), population)
    successes_b = min(max(int(successes_b), 0), population - successes_a)
    draws = min(max(int(draws), 0), population)
    if population <= 0 or successes_a <= 0 or successes_b <= 0 or draws < 2:
        return 0.0

    def no_success(count: int) -> float:
        misses = population - count
        if draws > misses:
            return 0.0
        return comb(misses, draws) / comb(population, draws)

    no_a = no_success(successes_a)
    no_b = no_success(successes_b)
    no_a_or_b = no_success(successes_a + successes_b)
    return clamp01(1.0 - no_a - no_b + no_a_or_b)


def clamp01(value: float) -> float:
    return max(0.0, min(1.0, float(value)))


def self_test() -> None:
    p_opening = hypergeom_at_least_one(10, 1, 5)
    if abs(p_opening - 0.5) > 1e-9:
        raise SystemExit(f"expected p_opening 0.5, got {p_opening}")
    p_opening_draw = hypergeom_at_least_one(20, 2, 5)
    p_turn2_draw = hypergeom_at_least_one(20, 2, 10)
    if p_turn2_draw <= p_opening_draw:
        raise SystemExit("expected turn-2 reachability to exceed opening reachability")
    p_combo = hypergeom_at_least_one_each(20, 1, 3, 5)
    if p_combo <= 0:
        raise SystemExit("expected same-turn combo probability > 0")
    offering = {"card_id": "Offering", "draws_cards": True, "gains_energy": True, "card_type_id": 2}
    weak_profile = {
        "frontload_supply": 45,
        "block_supply": 25,
        "draw_supply": 5,
        "scaling_supply": 0,
    }
    weak_deck = [
        {"card_id": "Strike", "base_damage": 6, "card_type_id": 1},
        {"card_id": "Defend", "base_block": 5, "card_type_id": 2},
    ]
    pressure_penalty = resource_window_pressure_gate_penalty(
        card=offering,
        obs={"act": 1, "floor": 14, "current_hp": 21, "max_hp": 80},
        profile=weak_profile,
        deck_cards=weak_deck,
    )
    if pressure_penalty < 25:
        raise SystemExit(f"expected high-pressure Offering gate, got {pressure_penalty}")
    safe_penalty = resource_window_pressure_gate_penalty(
        card=offering,
        obs={"act": 1, "floor": 4, "current_hp": 70, "max_hp": 80},
        profile={"frontload_supply": 85, "block_supply": 60, "draw_supply": 5},
        deck_cards=weak_deck
        + [
            {"card_id": "Immolate", "base_damage": 21, "aoe": True, "card_type_id": 1},
            {"card_id": "ShrugItOff", "base_block": 8, "draws_cards": True, "card_type_id": 2},
        ],
    )
    if safe_penalty >= pressure_penalty:
        raise SystemExit("expected safe Offering gate to be lower than high-pressure gate")
    pommel_gate = draw_payoff_gate_penalty(
        card={"card_id": "PommelStrike", "draws_cards": True, "base_damage": 9, "cost": 1},
        obs={"act": 1, "floor": 1, "current_hp": 70, "max_hp": 80},
        profile=weak_profile,
        deck_cards=weak_deck,
    )
    if pommel_gate < 12:
        raise SystemExit(f"expected low-payoff Pommel gate, got {pommel_gate}")
    shrug_gate = draw_payoff_gate_penalty(
        card={"card_id": "ShrugItOff", "draws_cards": True, "base_block": 8, "cost": 1},
        obs={"act": 1, "floor": 5, "current_hp": 23, "max_hp": 80},
        profile=weak_profile,
        deck_cards=weak_deck,
    )
    if shrug_gate < 20:
        raise SystemExit(f"expected low-frontload Shrug gate, got {shrug_gate}")
    stable_immolate_uncertainty = aoe_cashout_uncertainty_penalty(
        card={"card_id": "Immolate", "aoe": True, "base_damage": 21, "cost": 2},
        obs={"act": 1, "floor": 8, "current_hp": 64, "max_hp": 80},
        profile={"frontload_supply": 48, "aoe_supply": 0},
    )
    unstable_immolate_uncertainty = aoe_cashout_uncertainty_penalty(
        card={"card_id": "Immolate", "aoe": True, "base_damage": 21, "cost": 2},
        obs={"act": 1, "floor": 8, "current_hp": 64, "max_hp": 80},
        profile={"frontload_supply": 58, "aoe_supply": 0},
    )
    if unstable_immolate_uncertainty <= stable_immolate_uncertainty:
        raise SystemExit(
            "expected stronger Immolate uncertainty when deck already has more frontload"
        )
    if is_aoe_damage({"card_id": "TwinStrike", "multi_damage": True, "base_damage": 5}):
        raise SystemExit("Twin Strike is multi-hit, not AoE")
    if not is_aoe_damage({"card_id": "Cleave", "multi_damage": True, "base_damage": 8}):
        raise SystemExit("expected Cleave to be treated as AoE damage")
    starterish_deck = [
        *[{"card_id": "Strike", "base_damage": 6, "card_type_id": 1} for _ in range(5)],
        *[{"card_id": "Defend", "base_block": 5, "card_type_id": 2} for _ in range(4)],
        {"card_id": "Bash", "base_damage": 8, "applies_vulnerable": True, "card_type_id": 1},
        {"card_id": "Flex", "scaling_piece": True, "card_type_id": 2},
    ]
    corruption_gate = corruption_cashout_gate_penalty(
        card={"card_id": "Corruption", "cost": 3, "scaling_piece": True, "card_type_id": 3},
        obs={"act": 1, "floor": 6, "current_hp": 40, "max_hp": 80},
        profile={"frontload_supply": 55, "block_supply": 25, "draw_supply": 0},
        deck_cards=starterish_deck,
    )
    if corruption_gate < 30:
        raise SystemExit(f"expected Corruption prerequisite gate, got {corruption_gate}")
    dropkick_gate = dropkick_vulnerable_gate_penalty(
        card={
            "card_id": "Dropkick",
            "draws_cards": True,
            "gains_energy": True,
            "base_damage": 5,
            "card_type_id": 1,
        },
        obs={"act": 1, "floor": 6, "current_hp": 12, "max_hp": 80},
        profile={"frontload_supply": 54, "draw_supply": 0},
        deck_cards=starterish_deck,
    )
    if dropkick_gate < 30:
        raise SystemExit(f"expected Dropkick vulnerable gate, got {dropkick_gate}")
    searing_gate = buildaround_upgrade_gate_penalty(
        card={"card_id": "SearingBlow", "upgrades": 0, "base_damage": 12, "card_type_id": 1},
        obs={"act": 1, "floor": 6},
        profile={"frontload_supply": 62, "starter_basic_burden": 90},
    )
    if searing_gate < 30:
        raise SystemExit(f"expected Searing Blow upgrade gate, got {searing_gate}")
    spot_gate = spot_weakness_window_gate_penalty(
        card={"card_id": "SpotWeakness", "scaling_piece": True, "card_type_id": 2},
        obs={"act": 2, "floor": 16},
        profile={"frontload_supply": 80, "block_supply": 25, "draw_supply": 10},
        deck_cards=starterish_deck,
    )
    if spot_gate < 20:
        raise SystemExit(f"expected Spot Weakness attack-window gate, got {spot_gate}")
    exhume_gate = exhume_target_gate_penalty(
        card={"card_id": "Exhume", "card_type_id": 2},
        deck_cards=starterish_deck,
        profile={"exhaust_supply": 0},
    )
    if exhume_gate < 30:
        raise SystemExit(f"expected Exhume target gate, got {exhume_gate}")
    clash_gate = clash_playability_gate_penalty(
        card={"card_id": "Clash", "base_damage": 14, "card_type_id": 1},
        deck_cards=starterish_deck,
        profile={"draw_supply": 0, "starter_basic_burden": 90},
    )
    if clash_gate < 20:
        raise SystemExit(f"expected Clash playability gate, got {clash_gate}")
    rampage_gate = rampage_repeat_gate_penalty(
        card={"card_id": "Rampage", "base_damage": 8, "card_type_id": 1},
        profile={"frontload_supply": 52, "draw_supply": 0, "aoe_supply": 0},
    )
    if rampage_gate < 12:
        raise SystemExit(f"expected Rampage repeat gate, got {rampage_gate}")
    reaper_gate = reaper_strength_gate_penalty(
        card={"card_id": "Reaper", "base_damage": 4, "aoe": True, "card_type_id": 1},
        profile={"frontload_supply": 80, "block_supply": 31, "scaling_supply": 0, "aoe_supply": 36},
    )
    if reaper_gate < 20:
        raise SystemExit(f"expected Reaper strength gate, got {reaper_gate}")
    print(
        json.dumps(
            {
                "self_test": "ok",
                "p_opening": p_opening,
                "p_combo": p_combo,
                "pressure_penalty": pressure_penalty,
                "safe_penalty": safe_penalty,
                "pommel_gate": pommel_gate,
                "shrug_gate": shrug_gate,
                "stable_immolate_uncertainty": stable_immolate_uncertainty,
                "unstable_immolate_uncertainty": unstable_immolate_uncertainty,
            }
        )
    )


def class_counts(cards: list[dict[str, Any]]) -> dict[str, int]:
    counts: Counter[str] = Counter()
    for card in cards:
        for cls in card_classes(card):
            counts[cls] += 1
    return dict(counts)


def high_value_deck_cards(cards: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    grouped: dict[str, dict[str, Any]] = {}
    for card in cards:
        cid = card_id(card)
        if not cid:
            continue
        classes = card_classes(card)
        if classes & {
            "generic_draw",
            "generic_energy",
            "generic_scaling",
            "generic_aoe_damage",
            "generic_multi_enemy_control",
            "kill_window",
            "exhaust_engine",
        }:
            row = grouped.setdefault(
                cid,
                {"count": 0, "classes": sorted(classes), "cost": card_cost(card)},
            )
            row["count"] += 1
    return grouped


def context_flags(obs: dict[str, Any], profile: dict[str, Any]) -> tuple[list[str], list[str]]:
    flags: list[str] = []
    warnings: list[str] = []
    act = int(obs.get("act") or 0)
    floor = int(obs.get("floor") or 0)
    relics = relic_ids(obs)
    if "VelvetChoker" in relics:
        flags.append("velvet_choker_action_budget")
    if "LetterOpener" in relics:
        flags.append("letter_opener_skill_chain")
    if act == 1 and floor <= 6:
        flags.append("act1_early_nob_skill_risk_relevance")
    if act >= 2 or floor >= 7:
        flags.append("act2_or_late_act1_aoe_pressure")
    if int(profile.get("aoe_supply") or 0) < 18:
        flags.append("aoe_readiness_deficit")
    if int(profile.get("draw_supply") or 0) < 20:
        flags.append("draw_deficit")
    if int(profile.get("scaling_supply") or 0) < 20:
        flags.append("scaling_deficit")
    if int(profile.get("starter_basic_burden") or 0) >= 60:
        flags.append("starter_basic_burden_high")
    for relic in sorted(relics):
        warning = HIGH_VALUE_RELIC_WARNINGS.get(relic)
        if warning:
            warnings.append(warning)
    return sorted(set(flags)), sorted(set(warnings))


def build_relevance(
    *,
    obs: dict[str, Any],
    candidates: list[dict[str, Any]],
    deck_cards: list[dict[str, Any]],
) -> dict[str, Any]:
    profile = obs.get("plan_profile") or {}
    flags, warnings = context_flags(obs, profile)
    candidate_ids = [card_id(candidate.get("card") or {}) for candidate in candidates]
    bucket_counts = class_counts(deck_cards)
    return {
        "score_kind": SCORE_KIND,
        "tracked_candidates": candidate_ids,
        "tracked_deck_cards": high_value_deck_cards(deck_cards),
        "tracked_classes": sorted(
            {
                cls
                for card in deck_cards
                for cls in card_classes(card)
                if cls
                in {
                    "generic_draw",
                    "generic_energy",
                    "generic_scaling",
                    "generic_aoe_damage",
                    "generic_multi_enemy_control",
                    "kill_window",
                    "exhaust_engine",
                }
            }
        ),
        "bucketed_cards": bucket_counts,
        "context_flags": flags,
        "context_warnings": warnings,
    }


def reachability(
    *,
    candidate_card: dict[str, Any],
    deck_cards: list[dict[str, Any]],
    opening_hand_size: int,
    turn2_seen_cards: int,
) -> dict[str, Any]:
    deck_after = deck_cards + [candidate_card]
    population = len(deck_after)
    candidate_count = sum(1 for card in deck_after if card_id(card) == card_id(candidate_card))
    opening_draws = min(opening_hand_size, population)
    turn2_draws = min(turn2_seen_cards, population)
    bucket_counts = class_counts(deck_after)
    opening_bucket = {
        bucket: hypergeom_at_least_one(population, count, opening_draws)
        for bucket, count in sorted(bucket_counts.items())
    }
    turn2_bucket = {
        bucket: hypergeom_at_least_one(population, count, turn2_draws)
        for bucket, count in sorted(bucket_counts.items())
    }
    payoff_count = playable_payoff_count(deck_cards, candidate_card)
    return {
        "score_kind": SCORE_KIND,
        "population_after_pick": population,
        "candidate_copies_after_pick": candidate_count,
        "opening_hand_size": opening_draws,
        "turn2_seen_cards": turn2_draws,
        "p_opening_candidate": round(
            hypergeom_at_least_one(population, candidate_count, opening_draws), 6
        ),
        "p_by_turn2_candidate": round(
            hypergeom_at_least_one(population, candidate_count, turn2_draws), 6
        ),
        "p_opening_at_least_one_bucket": {
            key: round(value, 6) for key, value in opening_bucket.items()
        },
        "p_by_turn2_at_least_one_bucket": {
            key: round(value, 6) for key, value in turn2_bucket.items()
        },
        "payoff_bucket_count_excluding_candidate": payoff_count,
        "p_same_turn_candidate_plus_payoff": round(
            hypergeom_at_least_one_each(
                population, candidate_count, payoff_count, opening_draws
            ),
            6,
        ),
    }


def playable_payoff_count(deck_cards: list[dict[str, Any]], candidate_card: dict[str, Any]) -> int:
    candidate = card_id(candidate_card)
    count = 0
    for card in deck_cards:
        if card_id(card) == candidate:
            continue
        classes = card_classes(card)
        if classes & {
            "generic_attack",
            "generic_block",
            "generic_scaling",
            "generic_aoe_damage",
            "generic_multi_enemy_control",
            "generic_energy",
            "exhaust_engine",
            "kill_window",
        }:
            if not is_junk(card):
                count += 1
    return count


def deficit_factor(profile: dict[str, Any], field: str, target: float) -> float:
    current = float(profile.get(field) or 0)
    return 1.0 + max(target - current, 0.0) / max(target, 1.0)


def profile_value(profile: dict[str, Any], field: str) -> float:
    return float(profile.get(field) or 0.0)


def exhaust_outlet_count(deck_cards: list[dict[str, Any]]) -> int:
    return sum(1 for card in deck_cards if is_exhaust_outlet(card))


def deck_card_count(deck_cards: list[dict[str, Any]], ids: set[str]) -> int:
    return sum(1 for card in deck_cards if card_id(card) in ids)


def deck_type_count(deck_cards: list[dict[str, Any]], card_type_id: int) -> int:
    return sum(1 for card in deck_cards if int(card.get("card_type_id") or 0) == card_type_id)


def vulnerable_source_count(deck_cards: list[dict[str, Any]]) -> int:
    return deck_card_count(deck_cards, VULNERABLE_SOURCE_CARDS)


def card_upgrades(card: dict[str, Any]) -> int:
    try:
        return max(int(card.get("upgrades") or 0), 0)
    except (TypeError, ValueError):
        return 0


def context_penalties(
    *,
    card: dict[str, Any],
    delta: dict[str, int],
    obs: dict[str, Any],
    profile: dict[str, Any],
    deck_cards: list[dict[str, Any]],
) -> dict[str, Any]:
    relics = relic_ids(obs)
    act = int(obs.get("act") or 0)
    floor = int(obs.get("floor") or 0)
    values: dict[str, float] = {
        "choker_action_pressure": 0.0,
        "nob_skill_risk": 0.0,
        "high_curve_clog_risk": 0.0,
        "draw_over_cashout": 0.0,
        "setup_cashout_risk": 0.0,
        "act1_frontload_urgency": 0.0,
        "status_burden_risk": 0.0,
        "card_context_uncertainty": 0.0,
        "duplicate_penalty": float(abs(delta.get("duplicate_penalty") or 0)),
        "deck_bloat_penalty": float(abs(delta.get("bloat_penalty") or 0)),
    }
    if "VelvetChoker" in relics:
        if is_draw(card):
            values["choker_action_pressure"] += 16.0
        if card_cost(card) <= 1 and not is_scaling(card):
            values["choker_action_pressure"] += 5.0
    if act == 1 and floor <= 6 and int(card.get("card_type_id") or 0) == 2:
        if not (is_draw(card) and card_block(card) > 0) and card_damage(card) <= 0:
            values["nob_skill_risk"] += 18.0
        if card_damage(card) <= 0 and card_block(card) <= 0:
            values["nob_skill_risk"] += 8.0
    if card_id(card) in {"Shockwave", "Disarm"} and card_damage(card) <= 0 and card_block(card) <= 0:
        values["setup_cashout_risk"] += 18.0 if act == 1 and floor <= 6 else 10.0
    if is_multi_enemy_control(card) and card_damage(card) <= 0 and card_block(card) <= 0:
        values["setup_cashout_risk"] += 12.0 if act == 1 and floor <= 6 else 7.0
    if card_id(card) == "PommelStrike":
        if payoff_quality(deck_cards, profile) < 50:
            values["card_context_uncertainty"] += 8.0
        if profile_value(profile, "frontload_supply") >= 65:
            values["card_context_uncertainty"] += 10.0
        if profile_value(profile, "draw_supply") >= 20:
            values["card_context_uncertainty"] += 5.0
    if card_id(card) in {"ThunderClap", "Cleave", "Whirlwind"}:
        if profile_value(profile, "aoe_supply") >= 18:
            values["card_context_uncertainty"] += 12.0
        if card_id(card) == "ThunderClap" and act <= 1:
            if floor <= 5:
                values["card_context_uncertainty"] += 8.0
            elif floor >= 12:
                values["card_context_uncertainty"] += 12.0
        if act == 1 and floor <= 2 and card_id(card) != "Whirlwind":
            values["card_context_uncertainty"] += 5.0
    if card_id(card) == "Clothesline" and profile_value(profile, "frontload_supply") >= 70:
        values["card_context_uncertainty"] += 10.0
    if card_id(card) == "Clothesline":
        values["card_context_uncertainty"] += 4.0
        if act == 1 and floor <= 3:
            values["act1_frontload_urgency"] += 8.0
        if "SneckoEye" in relics:
            values["card_context_uncertainty"] += 6.0
    if card_id(card) == "ShrugItOff":
        current_hp = int(obs.get("current_hp") or obs.get("hp") or 0)
        if act <= 1 and floor <= 3 and profile_value(profile, "frontload_supply") < 60:
            values["card_context_uncertainty"] += 8.0
        elif act <= 1 and floor <= 6 and profile_value(profile, "frontload_supply") < 60:
            values["act1_frontload_urgency"] += 4.0
            if current_hp and current_hp <= 10:
                values["act1_frontload_urgency"] += 6.0
        if profile_value(profile, "draw_supply") >= 20:
            values["draw_over_cashout"] += 5.0
    draw_gate = draw_payoff_gate_penalty(
        card=card,
        obs=obs,
        profile=profile,
        deck_cards=deck_cards,
    )
    if draw_gate > 0:
        values["draw_payoff_gate"] = draw_gate
    aoe_uncertainty = aoe_cashout_uncertainty_penalty(
        card=card,
        obs=obs,
        profile=profile,
    )
    if aoe_uncertainty > 0:
        values["aoe_cashout_uncertainty"] = aoe_uncertainty
    resource_gate = resource_window_pressure_gate_penalty(
        card=card,
        obs=obs,
        profile=profile,
        deck_cards=deck_cards,
    )
    if resource_gate > 0:
        values["resource_window_pressure_gate"] = resource_gate
    corruption_gate = corruption_cashout_gate_penalty(
        card=card,
        obs=obs,
        profile=profile,
        deck_cards=deck_cards,
    )
    if corruption_gate > 0:
        values["corruption_cashout_gate"] = corruption_gate
    dropkick_gate = dropkick_vulnerable_gate_penalty(
        card=card,
        obs=obs,
        profile=profile,
        deck_cards=deck_cards,
    )
    if dropkick_gate > 0:
        values["dropkick_vulnerable_gate"] = dropkick_gate
    buildaround_gate = buildaround_upgrade_gate_penalty(
        card=card,
        obs=obs,
        profile=profile,
    )
    if buildaround_gate > 0:
        values["buildaround_upgrade_gate"] = buildaround_gate
    spot_gate = spot_weakness_window_gate_penalty(
        card=card,
        obs=obs,
        profile=profile,
        deck_cards=deck_cards,
    )
    if spot_gate > 0:
        values["spot_weakness_window_gate"] = spot_gate
    exhume_gate = exhume_target_gate_penalty(card=card, deck_cards=deck_cards, profile=profile)
    if exhume_gate > 0:
        values["exhume_target_gate"] = exhume_gate
    clash_gate = clash_playability_gate_penalty(card=card, deck_cards=deck_cards, profile=profile)
    if clash_gate > 0:
        values["clash_playability_gate"] = clash_gate
    rampage_gate = rampage_repeat_gate_penalty(card=card, profile=profile)
    if rampage_gate > 0:
        values["rampage_repeat_gate"] = rampage_gate
    reaper_gate = reaper_strength_gate_penalty(card=card, profile=profile)
    if reaper_gate > 0:
        values["reaper_strength_gate"] = reaper_gate
    generated_status = status_burden(card)
    if generated_status:
        status_penalty = generated_status * 5.0
        if not has_status_payoff(deck_cards):
            status_penalty += 5.0
        if exhaust_outlet_count(deck_cards) <= 0:
            status_penalty += 4.0
        if is_draw(card) or profile_value(profile, "draw_supply") >= 20:
            status_penalty += 3.0
        if act == 1 and floor <= 6:
            status_penalty += 3.0
        values["status_burden_risk"] += status_penalty
    if card_id(card) in {"DarkEmbrace", "FeelNoPain"}:
        outlet_count = exhaust_outlet_count(deck_cards)
        if outlet_count == 0:
            values["card_context_uncertainty"] += 20.0
        elif outlet_count == 1:
            values["card_context_uncertainty"] += 8.0
    if card_cost(card) >= 3:
        values["high_curve_clog_risk"] += 8.0 + max(card_cost(card) - 3, 0) * 4.0
        if is_scaling(card) and float(profile.get("frontload_supply") or 0) < 70:
            values["high_curve_clog_risk"] += 8.0
    if card_id(card) == "DemonForm":
        current_hp = int(obs.get("current_hp") or obs.get("hp") or 0)
        if act == 1 and floor <= 6:
            values["setup_cashout_risk"] += 14.0
            if profile_value(profile, "draw_supply") < 20:
                values["setup_cashout_risk"] += 5.0
            if profile_value(profile, "block_supply") < 30:
                values["setup_cashout_risk"] += 6.0
        if current_hp and current_hp <= 35 and act <= 1:
            values["setup_cashout_risk"] += 8.0
    if is_draw(card) and not is_energy(card):
        values["draw_over_cashout"] += max(card_cost(card), 0) * 3.0
    if is_draw(card) and "VelvetChoker" in relics:
        values["draw_over_cashout"] += 8.0
    if delta.get("setup_cashout_risk_delta", 0) > 0:
        setup_risk = float(profile.get("setup_cashout_risk") or 0)
        values["setup_cashout_risk"] += min(
            18.0, delta["setup_cashout_risk_delta"] * (1.0 + setup_risk / 30.0)
        )
    total = sum(values.values())
    active = {key: round(value, 3) for key, value in values.items() if value > 0}
    return {
        "score_kind": SCORE_KIND,
        **active,
        "total_penalty": round(total, 3),
    }


def draw_payoff_gate_penalty(
    *,
    card: dict[str, Any],
    obs: dict[str, Any],
    profile: dict[str, Any],
    deck_cards: list[dict[str, Any]],
) -> float:
    """Discount draw/block-draw when current deck cannot cash it out.

    This is a static residual repair for Pommel Strike / Shrug It Off style
    false positives. It should move marginal draw cases into rollout review,
    not decide that draw is bad.
    """
    if not is_draw(card) or is_energy(card):
        return 0.0

    cid = card_id(card)
    act = int(obs.get("act") or 0)
    floor = int(obs.get("floor") or 0)
    payoff = payoff_quality(deck_cards, profile)
    frontload = profile_value(profile, "frontload_supply")
    block = profile_value(profile, "block_supply")
    draw = profile_value(profile, "draw_supply")

    penalty = 0.0
    if payoff < 45:
        penalty += 10.0
    elif payoff < 60:
        penalty += 5.0
    if draw >= 20:
        penalty += 6.0

    if cid == "PommelStrike":
        if payoff < 50:
            penalty += 6.0
        if frontload >= 65:
            penalty += 8.0
        if act == 1 and floor <= 2 and payoff < 55:
            penalty += 4.0
    elif cid == "ShrugItOff":
        if act == 1 and floor <= 6 and frontload < 60:
            penalty += 18.0
        if block >= 50:
            penalty += 6.0
        if payoff < 50:
            penalty += 4.0
    elif cid == "Warcry":
        penalty += 8.0
    elif card_block(card) <= 0 and card_damage(card) <= 0:
        penalty += 8.0

    return round(min(max(penalty, 0.0), 42.0), 3)


def aoe_cashout_uncertainty_penalty(
    *,
    card: dict[str, Any],
    obs: dict[str, Any],
    profile: dict[str, Any],
) -> float:
    """Mark AoE cashout as uncertain unless future pressure is decisive."""
    if not (is_aoe_damage(card) or is_multi_enemy_control(card)):
        return 0.0

    cid = card_id(card)
    act = int(obs.get("act") or 0)
    floor = int(obs.get("floor") or 0)
    current_hp = int(obs.get("current_hp") or obs.get("hp") or 0)
    aoe_supply = profile_value(profile, "aoe_supply")
    frontload = profile_value(profile, "frontload_supply")

    penalty = 0.0
    if aoe_supply >= 18:
        penalty += 14.0
    elif aoe_supply >= 10:
        penalty += 8.0
    if act == 1 and floor <= 3:
        penalty += 10.0
    if cid != "Immolate":
        if act == 1 and floor <= 6:
            penalty += 8.0
        if cid == "ThunderClap":
            penalty += 8.0
    else:
        if act == 1 and floor >= 7 and frontload >= 55:
            penalty += 10.0
        if current_hp and current_hp <= 12:
            penalty *= 0.5
        if (act >= 2 or floor >= 7) and aoe_supply < 10 and frontload < 55:
            penalty *= 0.35

    return round(min(max(penalty, 0.0), 32.0), 3)


def resource_window_pressure_gate_penalty(
    *,
    card: dict[str, Any],
    obs: dict[str, Any],
    profile: dict[str, Any],
    deck_cards: list[dict[str, Any]],
) -> float:
    """Penalize resource-window cashout when the deck is under pressure.

    Rollout micro-probes showed Offering false positives where the card opened a
    draw/energy window, but high-pressure continuations did not convert it into
    combat wins, kill timing, or meaningful monster HP progress. This gate is a
    diagnostic correction, not card truth.
    """
    cid = card_id(card)
    if cid not in RESOURCE_WINDOW_CARDS:
        return 0.0

    act = int(obs.get("act") or 0)
    floor = int(obs.get("floor") or 0)
    current_hp = int(obs.get("current_hp") or obs.get("hp") or 0)
    max_hp = int(obs.get("max_hp") or 0)
    hp_ratio = current_hp / max(max_hp, 1) if current_hp and max_hp else 1.0
    frontload = profile_value(profile, "frontload_supply")
    block = profile_value(profile, "block_supply")
    payoff = payoff_quality(deck_cards, profile)

    pressure = 0.0
    if cid == "Offering":
        if current_hp and current_hp <= 12:
            pressure += 30.0
        elif current_hp and current_hp <= 24:
            pressure += 22.0
        elif current_hp and current_hp <= 36:
            pressure += 12.0
    elif current_hp and current_hp <= 18:
        pressure += 10.0
    if hp_ratio <= 0.35:
        pressure += 8.0
    if act >= 2 or floor >= 12:
        pressure += 6.0
    if frontload < 60:
        pressure += min(16.0, (60.0 - frontload) * 0.35)
    if block < 35:
        pressure += min(14.0, (35.0 - block) * 0.40)
    if payoff < 45:
        pressure += 14.0
    elif payoff < 60:
        pressure += 8.0

    if payoff >= 75 and current_hp > 35 and frontload >= 65:
        pressure *= 0.45
    elif payoff >= 65 and current_hp > 30:
        pressure *= 0.65
    if frontload >= 80 and block >= 55:
        pressure *= 0.70

    return round(min(max(pressure, 0.0), 55.0), 3)


def corruption_cashout_gate_penalty(
    *,
    card: dict[str, Any],
    obs: dict[str, Any],
    profile: dict[str, Any],
    deck_cards: list[dict[str, Any]],
) -> float:
    """Require actual skill/payoff density before Corruption gets scaling credit."""
    if card_id(card) != "Corruption":
        return 0.0

    act = int(obs.get("act") or 0)
    floor = int(obs.get("floor") or 0)
    skills = deck_type_count(deck_cards, 2)
    payoff_cards = deck_card_count(deck_cards, CORRUPTION_PAYOFF_CARDS)
    outlet_count = exhaust_outlet_count(deck_cards)
    draw = profile_value(profile, "draw_supply")
    block = profile_value(profile, "block_supply")
    frontload = profile_value(profile, "frontload_supply")

    penalty = 0.0
    if skills <= 4:
        penalty += 26.0
    elif skills <= 6:
        penalty += 18.0
    elif skills <= 8:
        penalty += 8.0
    if payoff_cards <= 0:
        penalty += 10.0
    elif payoff_cards == 1 and skills <= 6:
        penalty += 4.0
    if draw < 18:
        penalty += 8.0
    if block < 35 and skills <= 6:
        penalty += 6.0
    if act == 1 and floor <= 7:
        penalty += 8.0
    if frontload < 65:
        penalty += 5.0
    if outlet_count >= 2 and payoff_cards >= 1 and skills >= 8:
        penalty *= 0.55
    elif skills >= 9 and draw >= 25:
        penalty *= 0.70

    return round(min(max(penalty, 0.0), 55.0), 3)


def dropkick_vulnerable_gate_penalty(
    *,
    card: dict[str, Any],
    obs: dict[str, Any],
    profile: dict[str, Any],
    deck_cards: list[dict[str, Any]],
) -> float:
    """Discount Dropkick draw/energy unless vulnerable is realistically available."""
    if card_id(card) != "Dropkick":
        return 0.0

    act = int(obs.get("act") or 0)
    floor = int(obs.get("floor") or 0)
    current_hp = int(obs.get("current_hp") or obs.get("hp") or 0)
    vuln_sources = vulnerable_source_count(deck_cards)
    draw = profile_value(profile, "draw_supply")
    frontload = profile_value(profile, "frontload_supply")

    penalty = 0.0
    if vuln_sources <= 0:
        penalty += 30.0
    elif vuln_sources == 1:
        penalty += 20.0
    elif vuln_sources == 2:
        penalty += 8.0
    if draw < 15 and vuln_sources <= 1:
        penalty += 6.0
    if frontload < 60 and vuln_sources <= 1:
        penalty += 5.0
    if act == 1 and floor <= 6 and vuln_sources <= 1:
        penalty += 5.0
    if current_hp and current_hp <= 15:
        penalty += 6.0
    if vuln_sources >= 3 and draw >= 20:
        penalty *= 0.50

    return round(min(max(penalty, 0.0), 45.0), 3)


def buildaround_upgrade_gate_penalty(
    *,
    card: dict[str, Any],
    obs: dict[str, Any],
    profile: dict[str, Any],
) -> float:
    """Prevent upgrade-dependent build-arounds from looking like plain frontload."""
    cid = card_id(card)
    if cid != "SearingBlow":
        return 0.0

    upgrades = card_upgrades(card)
    floor = int(obs.get("floor") or 0)
    starter_burden = profile_value(profile, "starter_basic_burden")
    frontload = profile_value(profile, "frontload_supply")

    penalty = 0.0
    if upgrades <= 0:
        penalty += 24.0
    elif upgrades == 1:
        penalty += 14.0
    elif upgrades == 2:
        penalty += 6.0
    if floor >= 6 and upgrades <= 1:
        penalty += 8.0
    if starter_burden >= 80 and upgrades <= 1:
        penalty += 4.0
    if frontload >= 60 and upgrades <= 1:
        penalty += 5.0
    if upgrades >= 3:
        penalty *= 0.25

    return round(min(max(penalty, 0.0), 42.0), 3)


def spot_weakness_window_gate_penalty(
    *,
    card: dict[str, Any],
    obs: dict[str, Any],
    profile: dict[str, Any],
    deck_cards: list[dict[str, Any]],
) -> float:
    """Treat Spot Weakness as attack-window scaling, not generic scaling."""
    if card_id(card) != "SpotWeakness":
        return 0.0

    attack_count = deck_type_count(deck_cards, 1)
    draw = profile_value(profile, "draw_supply")
    frontload = profile_value(profile, "frontload_supply")
    block = profile_value(profile, "block_supply")
    act = int(obs.get("act") or 0)
    floor = int(obs.get("floor") or 0)

    penalty = 0.0
    if attack_count <= 5:
        penalty += 16.0
    elif attack_count <= 7:
        penalty += 8.0
    if draw < 15:
        penalty += 6.0
    if block < 35 and (act >= 2 or floor >= 8):
        penalty += 8.0
    if frontload >= 75 and draw >= 20:
        penalty += 8.0
    if attack_count >= 9 and draw >= 25:
        penalty *= 0.55

    return round(min(max(penalty, 0.0), 36.0), 3)


def exhume_target_gate_penalty(
    *,
    card: dict[str, Any],
    deck_cards: list[dict[str, Any]],
    profile: dict[str, Any],
) -> float:
    """Require enough meaningful exhaust events before Exhume gets engine value."""
    if card_id(card) != "Exhume":
        return 0.0

    exhaust_cards = sum(
        1
        for deck_card in deck_cards
        if is_exhaust_engine(deck_card) or bool(deck_card.get("exhaust"))
    )
    outlets = exhaust_outlet_count(deck_cards)
    exhaust_supply = profile_value(profile, "exhaust_supply")

    penalty = 0.0
    if exhaust_cards <= 0:
        penalty += 24.0
    elif exhaust_cards == 1:
        penalty += 14.0
    if outlets <= 0 and exhaust_supply < 10:
        penalty += 10.0
    if exhaust_supply <= 0:
        penalty += 6.0
    if exhaust_cards >= 3 and exhaust_supply >= 16:
        penalty *= 0.55

    return round(min(max(penalty, 0.0), 44.0), 3)


def clash_playability_gate_penalty(
    *,
    card: dict[str, Any],
    deck_cards: list[dict[str, Any]],
    profile: dict[str, Any],
) -> float:
    """Clash frontload is conditional on drawing an all-attack hand."""
    if card_id(card) != "Clash":
        return 0.0

    non_attack_count = sum(
        1
        for deck_card in deck_cards
        if int(deck_card.get("card_type_id") or 0) != 1 and not is_junk(deck_card)
    )
    junk_count = sum(1 for deck_card in deck_cards if is_junk(deck_card))
    draw = profile_value(profile, "draw_supply")
    starter_burden = profile_value(profile, "starter_basic_burden")

    penalty = 0.0
    if non_attack_count >= 5:
        penalty += 22.0
    elif non_attack_count >= 3:
        penalty += 16.0
    elif non_attack_count >= 1:
        penalty += 7.0
    if junk_count > 0:
        penalty += min(12.0, junk_count * 5.0)
    if draw >= 15:
        penalty += 4.0
    if starter_burden >= 80 and non_attack_count >= 3:
        penalty += 4.0

    return round(min(max(penalty, 0.0), 42.0), 3)


def rampage_repeat_gate_penalty(*, card: dict[str, Any], profile: dict[str, Any]) -> float:
    """Rampage needs repeat access; static frontload alone overstates it."""
    if card_id(card) != "Rampage":
        return 0.0

    draw = profile_value(profile, "draw_supply")
    frontload = profile_value(profile, "frontload_supply")
    aoe = profile_value(profile, "aoe_supply")

    penalty = 0.0
    if draw < 15:
        penalty += 10.0
    elif draw < 30:
        penalty += 5.0
    if frontload >= 50:
        penalty += 4.0
    if aoe <= 0:
        penalty += 3.0
    if draw >= 35:
        penalty *= 0.45

    return round(min(max(penalty, 0.0), 28.0), 3)


def reaper_strength_gate_penalty(*, card: dict[str, Any], profile: dict[str, Any]) -> float:
    """Reaper payoff needs strength or a real healing/AoE shortfall."""
    if card_id(card) != "Reaper":
        return 0.0

    scaling = profile_value(profile, "scaling_supply")
    aoe = profile_value(profile, "aoe_supply")
    frontload = profile_value(profile, "frontload_supply")
    block = profile_value(profile, "block_supply")

    penalty = 0.0
    if scaling < 15:
        penalty += 16.0
    elif scaling < 30:
        penalty += 8.0
    if aoe >= 20:
        penalty += 6.0
    if frontload >= 70:
        penalty += 5.0
    if block >= 45:
        penalty += 4.0
    if scaling >= 35:
        penalty *= 0.45

    return round(min(max(penalty, 0.0), 35.0), 3)


def payoff_quality(deck_cards: list[dict[str, Any]], profile: dict[str, Any]) -> float:
    counts = class_counts(deck_cards)
    frontload_need = deficit_factor(profile, "frontload_supply", 70.0)
    block_need = deficit_factor(profile, "block_supply", 50.0)
    scaling_need = deficit_factor(profile, "scaling_supply", 35.0)
    aoe_need = deficit_factor(profile, "aoe_supply", 18.0)
    energy = counts.get("generic_energy", 0) * 10.0
    attacks = counts.get("generic_attack", 0) * 2.5 * frontload_need
    block = counts.get("generic_block", 0) * 2.0 * block_need
    scaling = counts.get("generic_scaling", 0) * 4.0 * scaling_need
    aoe = counts.get("generic_aoe_damage", 0) * 4.0 * aoe_need
    multi_control = counts.get("generic_multi_enemy_control", 0) * 3.0 * aoe_need
    exhaust = counts.get("exhaust_engine", 0) * 2.0
    junk = counts.get("junk_status", 0) * 1.8
    return max(0.0, energy + attacks + block + scaling + aoe + multi_control + exhaust - junk)


def base_prior_value(candidate: dict[str, Any], card: dict[str, Any], profile: dict[str, Any]) -> float:
    """Keep static rule/plan priors as weak tie-breakers, not cashout proof.

    The first cashout pass let rule_score dominate slow draw/control/scaling
    cards. Rollout labels showed that many of those cards are policy/horizon
    dependent, so V0.2 discounts priors unless the card has direct current-deck
    cashout such as damage/block/AoE.
    """
    rule = candidate_rule_score(candidate)
    plan = candidate_plan_score(candidate)
    direct_output = card_damage(card) > 0 or card_block(card) > 0 or is_aoe_damage(card)
    slow_plan = (
        is_draw(card)
        or is_scaling(card)
        or is_multi_enemy_control(card)
        or card_id(card) in {"Shockwave", "Disarm"}
    )
    cid = card_id(card)
    if cid == "ThunderClap":
        return 0.04 * rule + 0.02 * plan
    if cid in {"Cleave", "Whirlwind"}:
        return 0.11 * rule + 0.055 * plan
    if cid == "PommelStrike":
        prior = 0.08 * rule + 0.04 * plan
        if profile_value(profile, "frontload_supply") >= 65:
            prior *= 0.55
        return prior
    if is_aoe_damage(card):
        return 0.17 * rule + 0.09 * plan
    if direct_output and not slow_plan:
        return 0.14 * rule + 0.07 * plan
    if direct_output and slow_plan:
        return 0.10 * rule + 0.05 * plan
    return 0.07 * rule + 0.035 * plan


def draw_context_multiplier(card: dict[str, Any], deck_cards: list[dict[str, Any]], profile: dict[str, Any]) -> float:
    cid = card_id(card)
    payoff = payoff_quality(deck_cards, profile)
    if cid in RESOURCE_WINDOW_CARDS:
        multiplier = 0.85
        if payoff < 45:
            multiplier *= 0.60
        elif payoff < 60:
            multiplier *= 0.78
        elif payoff >= 75:
            multiplier *= 1.08
        return max(0.35, min(multiplier, 0.95))
    if cid == "PommelStrike":
        multiplier = 0.55
        if payoff < 50:
            multiplier *= 0.65
        if profile_value(profile, "frontload_supply") >= 65:
            multiplier *= 0.55
        if profile_value(profile, "draw_supply") >= 20:
            multiplier *= 0.75
        if payoff >= 55:
            multiplier *= 1.20
        return multiplier
    if cid == "ShrugItOff":
        multiplier = 0.75
        if profile_value(profile, "frontload_supply") < 60:
            multiplier *= 0.70
        if profile_value(profile, "draw_supply") >= 20:
            multiplier *= 0.75
        if payoff < 50:
            multiplier *= 0.75
        return max(0.25, min(multiplier, 0.85))
    if card_damage(card) > 0 and card_block(card) <= 0:
        return 0.55
    if card_block(card) > 0:
        return 0.80
    return 1.0


def frontload_context_multiplier(card: dict[str, Any], profile: dict[str, Any]) -> float:
    cid = card_id(card)
    supply = profile_value(profile, "frontload_supply")
    if cid == "PommelStrike":
        return 0.55 if supply >= 65 else 0.85
    if cid == "Clothesline":
        return 0.55 if supply >= 70 else 0.90
    return 1.0


def aoe_context_multiplier(card: dict[str, Any], obs: dict[str, Any], profile: dict[str, Any]) -> float:
    cid = card_id(card)
    act = int(obs.get("act") or 0)
    floor = int(obs.get("floor") or 0)
    aoe_supply = profile_value(profile, "aoe_supply")
    deficit = 1.0 if aoe_supply < 12 else (0.75 if aoe_supply < 18 else 0.45)
    timing = 1.20 if act >= 2 or floor >= 7 else (0.95 if floor >= 4 else 0.75)
    if cid == "Immolate":
        return max(0.9, deficit * timing * 1.25)
    if cid == "Whirlwind":
        return deficit * timing * 0.95
    if cid == "Cleave":
        return deficit * timing * 0.78
    if cid == "ThunderClap":
        if act <= 1:
            return deficit * timing * 0.35
        return deficit * timing * 0.50
    return deficit * timing


def bucket_ev(
    *,
    candidate: dict[str, Any],
    deck_cards: list[dict[str, Any]],
    reach: dict[str, Any],
    penalties: dict[str, Any],
    obs: dict[str, Any],
) -> dict[str, Any]:
    card = candidate.get("card") or {}
    delta = plan_delta(candidate)
    profile = obs.get("plan_profile") or {}
    p_open = float(reach["p_opening_candidate"])
    p_turn2 = float(reach["p_by_turn2_candidate"])
    p_combo = float(reach["p_same_turn_candidate_plus_payoff"])
    frontload = (
        p_turn2
        * max(delta["frontload_delta"], card_damage(card) * 2 // 3)
        * deficit_factor(profile, "frontload_supply", 70.0)
        * frontload_context_multiplier(card, profile)
    )
    block = (
        p_turn2
        * max(delta["block_delta"], card_block(card))
        * deficit_factor(profile, "block_supply", 50.0)
    )
    draw_cashout = 0.0
    if is_draw(card):
        payoff = payoff_quality(deck_cards, profile)
        draw_base = 5 if card_cost(card) > 0 else 8
        draw_cashout = (
            p_turn2 * max(delta["draw_delta"], draw_base)
            + p_combo * min(payoff, 55.0) * 0.22
            + p_open * min(payoff, 35.0) * 0.06
        )
        draw_cashout *= draw_context_multiplier(card, deck_cards, profile)
    scaling = 0.0
    if is_scaling(card):
        act = int(obs.get("act") or 0)
        floor = int(obs.get("floor") or 0)
        time_factor = 1.25 if act >= 2 or floor >= 7 else 1.0
        synergy_factor = scaling_synergy_factor(card, deck_cards)
        scaling = (
            p_turn2
            * max(delta["scaling_delta"], 10)
            * deficit_factor(profile, "scaling_supply", 35.0)
            * time_factor
            * synergy_factor
        )
    aoe_damage = 0.0
    multi_enemy_control = 0.0
    if is_aoe_damage(card):
        act = int(obs.get("act") or 0)
        floor = int(obs.get("floor") or 0)
        act_factor = 1.35 if act >= 2 or floor >= 7 else 1.0
        aoe_damage = (
            p_turn2
            * max(int(delta["aoe_delta"] * 0.65), card_damage(card) // 2)
            * deficit_factor(profile, "aoe_supply", 18.0)
            * act_factor
            * aoe_context_multiplier(card, obs, profile)
        )
    if is_multi_enemy_control(card) or (delta["aoe_delta"] > 0 and not is_aoe_damage(card)):
        act = int(obs.get("act") or 0)
        floor = int(obs.get("floor") or 0)
        act_factor = 1.25 if act >= 2 or floor >= 7 else 1.0
        control_base = max(
            int(delta["aoe_delta"] * 0.55),
            int(delta["block_delta"] * 0.20),
            int(delta["frontload_delta"] * 0.10),
            6 if is_multi_enemy_control(card) else 0,
        )
        multi_enemy_control = (
            p_turn2
            * control_base
            * deficit_factor(profile, "aoe_supply", 18.0)
            * act_factor
            * aoe_context_multiplier(card, obs, profile)
        )
    aoe_total = aoe_damage + multi_enemy_control
    exhaust = 0.0
    if delta["exhaust_delta"] > 0 or is_exhaust_engine(card):
        exhaust = p_turn2 * max(delta["exhaust_delta"], 5) * deficit_factor(
            profile, "exhaust_supply", 12.0
        )
    kill_window = 0.0
    if delta["kill_window_delta"] > 0 or is_kill_window(card):
        kill_window = p_turn2 * max(delta["kill_window_delta"], 12)
    letter_opener_bonus = 0.0
    if "LetterOpener" in relic_ids(obs) and int(card.get("card_type_id") or 0) == 2:
        letter_opener_bonus = p_turn2 * 5.0

    total_penalty = float(penalties.get("total_penalty") or 0.0)
    base_prior = base_prior_value(candidate, card, profile)
    subtotal = (
        frontload
        + block
        + draw_cashout
        + scaling
        + aoe_total
        + exhaust
        + kill_window
        + letter_opener_bonus
    )
    cashout_score = subtotal + base_prior - total_penalty
    return {
        "score_kind": SCORE_KIND,
        "frontload": round(frontload, 3),
        "block": round(block, 3),
        "draw_cashout": round(draw_cashout, 3),
        "aoe_damage": round(aoe_damage, 3),
        "multi_enemy_control": round(multi_enemy_control, 3),
        "aoe": round(aoe_total, 3),
        "scaling_cashout": round(scaling, 3),
        "exhaust": round(exhaust, 3),
        "kill_window": round(kill_window, 3),
        "letter_opener_skill_bonus": round(letter_opener_bonus, 3),
        "base_prior": round(base_prior, 3),
        "context_penalty": round(total_penalty, 3),
        "cashout_score": round(cashout_score, 3),
        "dominant_cashout": dominant_cashout(
            {
                "frontload": frontload,
                "block": block,
                "draw_cashout": draw_cashout,
                "aoe_damage": aoe_damage,
                "multi_enemy_control": multi_enemy_control,
                "scaling_cashout": scaling,
                "exhaust": exhaust,
                "kill_window": kill_window,
            }
        ),
    }


def scaling_synergy_factor(card: dict[str, Any], deck_cards: list[dict[str, Any]]) -> float:
    cid = card_id(card)
    classes = class_counts(deck_cards)
    outlet_count = exhaust_outlet_count(deck_cards)
    if cid == "FeelNoPain":
        if outlet_count <= 0:
            return 0.45
        if outlet_count == 1:
            return 0.75
        return 1.15
    if cid == "DarkEmbrace":
        if outlet_count <= 0:
            return 0.35
        if outlet_count == 1:
            return 0.65
        return 1.10
    if cid in {"Inflame", "DemonForm", "LimitBreak"}:
        attack_count = classes.get("generic_attack", 0)
        if attack_count <= 4:
            return 0.75
        return 1.0
    return 0.85


def dominant_cashout(values: dict[str, float]) -> str:
    if not values:
        return "none"
    key, value = max(values.items(), key=lambda item: item[1])
    return key if value >= 4.0 else "low_signal"


def cashout_grade(score: float) -> str:
    if score >= 95:
        return "high"
    if score >= 60:
        return "medium"
    if score >= 30:
        return "low"
    return "speculative"


def candidate_cashout(
    *,
    candidate: dict[str, Any],
    deck_cards: list[dict[str, Any]],
    obs: dict[str, Any],
    opening_hand_size: int,
    turn2_seen_cards: int,
) -> dict[str, Any]:
    card = candidate.get("card") or {}
    profile = obs.get("plan_profile") or {}
    reach = reachability(
        candidate_card=card,
        deck_cards=deck_cards,
        opening_hand_size=opening_hand_size,
        turn2_seen_cards=turn2_seen_cards,
    )
    delta = plan_delta(candidate)
    penalties = context_penalties(
        card=card,
        delta=delta,
        obs=obs,
        profile=profile,
        deck_cards=deck_cards,
    )
    ev = bucket_ev(candidate=candidate, deck_cards=deck_cards, reach=reach, penalties=penalties, obs=obs)
    flags, warnings = context_flags(obs, profile)
    notes = candidate_notes(card, ev, penalties, warnings)
    return {
        "score_kind": SCORE_KIND,
        "card_id": card_id(card),
        "action_key": str(candidate.get("action_key") or ""),
        "rule_score": candidate_rule_score(candidate),
        "plan_adjusted_score": candidate_plan_score(candidate),
        "card_classes": sorted(card_classes(card)),
        "primary_class": primary_class(card),
        "relevance": {
            "tracked_candidate": card_id(card),
            "candidate_classes": sorted(card_classes(card)),
            "context_flags": flags,
            "context_warnings": warnings,
        },
        "reachability": reach,
        "bucket_ev": ev,
        "context_penalties": penalties,
        "cashout_score": ev["cashout_score"],
        "cashout_grade": cashout_grade(ev["cashout_score"]),
        "notes": notes,
    }


def candidate_notes(
    card: dict[str, Any],
    ev: dict[str, Any],
    penalties: dict[str, Any],
    warnings: list[str],
) -> list[str]:
    notes: list[str] = []
    if is_draw(card):
        notes.append(f"draw_cashout={ev['draw_cashout']:.1f} ({cashout_grade(ev['draw_cashout'])})")
    if is_scaling(card):
        notes.append(
            f"scaling_cashout={ev['scaling_cashout']:.1f} ({cashout_grade(ev['scaling_cashout'])})"
        )
    if is_aoe_damage(card):
        notes.append(
            f"aoe_damage_cashout={ev['aoe_damage']:.1f} ({cashout_grade(ev['aoe_damage'])})"
        )
    if is_multi_enemy_control(card):
        notes.append(
            "multi_enemy_control_cashout="
            f"{ev['multi_enemy_control']:.1f} ({cashout_grade(ev['multi_enemy_control'])})"
        )
    if penalties.get("total_penalty", 0) > 0:
        notes.append(f"context_penalty={penalties['total_penalty']:.1f}")
    if penalties.get("draw_payoff_gate", 0) > 0:
        notes.append(f"draw_payoff_gate={penalties['draw_payoff_gate']:.1f}")
    if penalties.get("aoe_cashout_uncertainty", 0) > 0:
        notes.append(f"aoe_cashout_uncertainty={penalties['aoe_cashout_uncertainty']:.1f}")
    if penalties.get("resource_window_pressure_gate", 0) > 0:
        notes.append(
            f"resource_window_pressure_gate={penalties['resource_window_pressure_gate']:.1f}"
        )
    if penalties.get("corruption_cashout_gate", 0) > 0:
        notes.append(f"corruption_cashout_gate={penalties['corruption_cashout_gate']:.1f}")
    if penalties.get("dropkick_vulnerable_gate", 0) > 0:
        notes.append(f"dropkick_vulnerable_gate={penalties['dropkick_vulnerable_gate']:.1f}")
    if penalties.get("buildaround_upgrade_gate", 0) > 0:
        notes.append(f"buildaround_upgrade_gate={penalties['buildaround_upgrade_gate']:.1f}")
    if penalties.get("spot_weakness_window_gate", 0) > 0:
        notes.append(f"spot_weakness_window_gate={penalties['spot_weakness_window_gate']:.1f}")
    if penalties.get("exhume_target_gate", 0) > 0:
        notes.append(f"exhume_target_gate={penalties['exhume_target_gate']:.1f}")
    if penalties.get("clash_playability_gate", 0) > 0:
        notes.append(f"clash_playability_gate={penalties['clash_playability_gate']:.1f}")
    if penalties.get("rampage_repeat_gate", 0) > 0:
        notes.append(f"rampage_repeat_gate={penalties['rampage_repeat_gate']:.1f}")
    if penalties.get("reaper_strength_gate", 0) > 0:
        notes.append(f"reaper_strength_gate={penalties['reaper_strength_gate']:.1f}")
    if warnings:
        notes.extend(warnings[:3])
    return notes


def compact_candidate(row: dict[str, Any]) -> dict[str, Any]:
    return {
        "card_id": row["card_id"],
        "action_key": row["action_key"],
        "rule_score": row["rule_score"],
        "plan_adjusted_score": row["plan_adjusted_score"],
        "cashout_score": row["cashout_score"],
        "cashout_grade": row["cashout_grade"],
        "primary_class": row["primary_class"],
        "dominant_cashout": row["bucket_ev"]["dominant_cashout"],
        "reachability": {
            "p_opening_candidate": row["reachability"]["p_opening_candidate"],
            "p_by_turn2_candidate": row["reachability"]["p_by_turn2_candidate"],
            "p_same_turn_candidate_plus_payoff": row["reachability"][
                "p_same_turn_candidate_plus_payoff"
            ],
        },
        "bucket_ev": row["bucket_ev"],
        "context_penalties": row["context_penalties"],
        "notes": row["notes"],
    }


def skip_cashout() -> dict[str, Any]:
    return {
        "card_id": "Skip",
        "action_key": "proceed",
        "rule_score": 5.0,
        "plan_adjusted_score": 5.0,
        "cashout_score": 5.0,
        "cashout_grade": "speculative",
        "primary_class": "skip",
        "dominant_cashout": "skip",
        "reachability": {
            "p_opening_candidate": 0.0,
            "p_by_turn2_candidate": 0.0,
            "p_same_turn_candidate_plus_payoff": 0.0,
        },
        "bucket_ev": {
            "score_kind": SCORE_KIND,
            "cashout_score": 5.0,
            "dominant_cashout": "skip",
        },
        "context_penalties": {"score_kind": SCORE_KIND, "total_penalty": 0.0},
        "notes": ["skip has no card cashout in this diagnostic"],
    }


def classify_case(
    *,
    selected: dict[str, Any] | None,
    best: dict[str, Any],
    cashout_gap: float,
    min_gap: float,
) -> tuple[list[str], bool, str, list[str]]:
    if cashout_gap < min_gap:
        return ["small_cashout_gap_ignore"], False, "low", ["gap below audit threshold"]
    if float(best.get("cashout_score") or 0.0) < MIN_ABSOLUTE_CASHOUT_FOR_REGRET:
        return (
            ["low_absolute_cashout_ignore"],
            False,
            "low",
            [
                "best static cashout is below absolute regret floor; "
                "this may be a bad chosen action, not a missed high-cashout card"
            ],
        )
    kinds: list[str] = []
    notes: list[str] = []
    if selected is None:
        kinds.append("skip_high_cashout_offer")
    selected_ev = selected["bucket_ev"] if selected else {"dominant_cashout": "skip"}
    best_ev = best["bucket_ev"]
    dominant = str(best_ev.get("dominant_cashout") or "low_signal")
    if dominant == "draw_cashout":
        kinds.append("missed_draw_cashout")
    elif dominant == "scaling_cashout":
        kinds.append("missed_scaling_cashout")
    elif dominant == "aoe_damage":
        kinds.append("missed_aoe_damage_cashout")
    elif dominant == "multi_enemy_control":
        kinds.append("missed_multi_enemy_control_cashout")
    elif dominant == "frontload":
        kinds.append("missed_frontload_cashout")
    elif dominant == "block":
        kinds.append("missed_block_cashout")
    elif dominant == "exhaust":
        kinds.append("missed_exhaust_cashout")
    elif dominant == "kill_window":
        kinds.append("missed_kill_window_cashout")
    if selected and selected["cashout_score"] < 30 <= best["cashout_score"]:
        kinds.append("picked_low_cashout")
    if selected and selected["bucket_ev"].get("context_penalty", 0) > best_ev.get("context_penalty", 0) + 8:
        kinds.append("picked_higher_context_penalty")
    if not kinds:
        kinds.append("cashout_gap_unclassified")
    needs_rollout = needs_deeper_model(best) or (selected is not None and needs_deeper_model(selected))
    if selected and selected_ev.get("dominant_cashout") != dominant:
        notes.append(
            f"selected dominant cashout {selected_ev.get('dominant_cashout')} differs from best {dominant}"
        )
    confidence = "high" if cashout_gap >= min_gap * 2 and not needs_rollout else "medium"
    if needs_rollout:
        confidence = "low" if cashout_gap < min_gap * 2 else "medium"
        notes.append("cashout attribution has context warnings; rollout model should verify")
    return kinds, needs_rollout, confidence, notes


def needs_deeper_model(row: dict[str, Any]) -> bool:
    warnings = (row.get("relevance") or {}).get("context_warnings") or []
    penalties = row.get("context_penalties") or {}
    if warnings:
        return True
    if penalties.get("choker_action_pressure", 0) > 0:
        return True
    if penalties.get("nob_skill_risk", 0) > 0:
        return True
    if penalties.get("card_context_uncertainty", 0) >= 8:
        return True
    if penalties.get("draw_payoff_gate", 0) >= 8:
        return True
    if penalties.get("aoe_cashout_uncertainty", 0) >= 8:
        return True
    if penalties.get("resource_window_pressure_gate", 0) >= 12:
        return True
    if penalties.get("corruption_cashout_gate", 0) >= 12:
        return True
    if penalties.get("dropkick_vulnerable_gate", 0) >= 12:
        return True
    if penalties.get("buildaround_upgrade_gate", 0) >= 12:
        return True
    if penalties.get("spot_weakness_window_gate", 0) >= 8:
        return True
    if penalties.get("exhume_target_gate", 0) >= 10:
        return True
    if penalties.get("clash_playability_gate", 0) >= 10:
        return True
    if penalties.get("rampage_repeat_gate", 0) >= 8:
        return True
    if penalties.get("reaper_strength_gate", 0) >= 10:
        return True
    if penalties.get("status_burden_risk", 0) >= 8:
        return True
    if penalties.get("act1_frontload_urgency", 0) >= 8:
        return True
    if row["bucket_ev"].get("dominant_cashout") in {"draw_cashout", "scaling_cashout"}:
        return row["cashout_grade"] in {"low", "speculative"}
    return False


def compact_case(row: dict[str, Any]) -> dict[str, Any]:
    return {
        "seed": row["seed"],
        "step_index": row["step_index"],
        "act": row["act"],
        "floor": row["floor"],
        "hp": row["hp"],
        "chosen": row["chosen"],
        "best_by_cashout": row["best_by_cashout"],
        "cashout_gap": row["cashout_gap"],
        "cashout_kinds": row["cashout_kinds"],
        "needs_rollout": row["needs_rollout"],
        "confidence": row["confidence"],
        "calibration_status": row.get("calibration_status", "uncalibrated"),
        "training_candidate": bool(row.get("training_candidate", False)),
        "calibration_notes": row.get("calibration_notes", []),
        "notes": row["notes"],
        "candidates": row["candidates"],
        "relevance": row["relevance"],
        "deck_plan_profile": row["deck_plan_profile"],
        "trace_file": row["trace_file"],
    }


def is_actionable_case(row: dict[str, Any], min_gap: float) -> bool:
    return (
        float(row.get("cashout_gap") or 0.0) >= min_gap
        and "small_cashout_gap_ignore" not in (row.get("cashout_kinds") or [])
        and "low_absolute_cashout_ignore" not in (row.get("cashout_kinds") or [])
    )


def is_high_confidence_training_candidate(row: dict[str, Any], min_gap: float) -> bool:
    best = row.get("best_by_cashout") or {}
    return (
        is_actionable_case(row, min_gap)
        and best.get("cashout_grade") == "high"
        and row.get("confidence") == "high"
        and not row.get("needs_rollout")
        and float(row.get("cashout_gap") or 0.0) >= min_gap * 2
    )


def calibrate_policies(
    policies: list[dict[str, Any]],
    *,
    min_gap: float,
    top_cases: int,
) -> dict[str, Any]:
    baseline = next(
        (policy for policy in policies if policy["policy"] == "rule_baseline_v0"),
        None,
    ) or next(
        (policy for policy in policies if "rule_baseline" in policy["policy"]),
        None,
    )
    baseline_cases = []
    if baseline:
        baseline_cases = [
            row
            for row in baseline.get("comparisons", [])
            if is_actionable_case(row, min_gap)
        ]
    baseline_best_cards = {
        (row.get("best_by_cashout") or {}).get("card_id")
        for row in baseline_cases
        if (row.get("best_by_cashout") or {}).get("card_id")
    }
    baseline_dominants = Counter(
        (row.get("best_by_cashout") or {}).get("dominant_cashout")
        for row in baseline_cases
        if (row.get("best_by_cashout") or {}).get("dominant_cashout")
    )
    baseline_kinds = Counter(
        kind
        for row in baseline_cases
        for kind in row.get("cashout_kinds", [])
        if kind != "small_cashout_gap_ignore"
    )

    for policy in policies:
        calibration_counts: Counter[str] = Counter()
        for row in policy.get("comparisons", []):
            notes = list(row.get("calibration_notes", []))
            if not is_actionable_case(row, min_gap):
                status = "ignored_small_gap"
                training_candidate = False
            elif policy is baseline:
                status = "cashout_disagreement_with_rule_baseline"
                training_candidate = False
                notes.append("rule_baseline_v0 was also flagged; treat as cashout calibration issue")
            elif (row.get("best_by_cashout") or {}).get("card_id") in baseline_best_cards:
                status = "cashout_disagreement_with_rule_baseline"
                training_candidate = False
                notes.append(
                    "same best-by-cashout card appears in rule_baseline_v0 flagged cases"
                )
            elif row.get("needs_rollout"):
                status = "needs_rollout"
                training_candidate = False
            elif is_high_confidence_training_candidate(row, min_gap):
                status = "high_confidence_candidate"
                training_candidate = True
            else:
                status = "diagnostic_only"
                training_candidate = False
            row["calibration_status"] = status
            row["training_candidate"] = training_candidate
            row["calibration_notes"] = notes
            calibration_counts[status] += 1

        actionable = [
            row
            for row in policy.get("comparisons", [])
            if is_actionable_case(row, min_gap)
        ]
        policy["calibration_counts"] = dict(calibration_counts)
        policy["high_confidence_candidate_count"] = sum(
            1
            for row in actionable
            if row.get("calibration_status") == "high_confidence_candidate"
        )
        policy["needs_rollout_calibrated_count"] = sum(
            1 for row in actionable if row.get("calibration_status") == "needs_rollout"
        )
        policy["rule_baseline_disagreement_count"] = sum(
            1
            for row in actionable
            if row.get("calibration_status") == "cashout_disagreement_with_rule_baseline"
        )
        policy["top_cases"] = [
            compact_case(row)
            for row in sorted(actionable, key=lambda item: item["cashout_gap"], reverse=True)[
                :top_cases
            ]
        ]
        policy["high_confidence_cases"] = [
            compact_case(row)
            for row in sorted(
                [
                    row
                    for row in actionable
                    if row.get("calibration_status") == "high_confidence_candidate"
                ],
                key=lambda item: item["cashout_gap"],
                reverse=True,
            )[:top_cases]
        ]

    return {
        "baseline_policy": baseline["policy"] if baseline else None,
        "baseline_actionable_regret_count": len(baseline_cases),
        "baseline_suspect_best_cards": sorted(card for card in baseline_best_cards if card),
        "baseline_suspect_dominants": dict(baseline_dominants),
        "baseline_suspect_kinds": dict(baseline_kinds),
        "policy_calibration_counts": {
            policy["policy"]: policy.get("calibration_counts", {}) for policy in policies
        },
    }


def summarize_policy(
    *,
    policy: str,
    path: Path,
    min_gap: float,
    top_cases: int,
    opening_hand_size: int,
    turn2_seen_cards: int,
) -> dict[str, Any]:
    comparisons: list[dict[str, Any]] = []
    regret_counts: Counter[str] = Counter()
    chosen_cashout_counts: Counter[str] = Counter()
    best_cashout_counts: Counter[str] = Counter()

    for trace_path in trace_files(path):
        trace = read_json(trace_path)
        seed = int((trace.get("summary") or {}).get("seed") or 0)
        for step in trace.get("steps") or []:
            if not is_card_reward_step(step):
                continue
            candidates = candidate_actions(step)
            if not candidates:
                continue
            obs = observation(step)
            deck_cards = deck_card_features(obs)
            relevance = build_relevance(
                obs=obs, candidates=candidates, deck_cards=deck_cards
            )
            cashout_rows = [
                candidate_cashout(
                    candidate=candidate,
                    deck_cards=deck_cards,
                    obs=obs,
                    opening_hand_size=opening_hand_size,
                    turn2_seen_cards=turn2_seen_cards,
                )
                for candidate in candidates
            ]
            selected_candidate = selected_action(step, candidates)
            selected = None
            if selected_candidate is not None:
                selected_key = str(selected_candidate.get("action_key") or "")
                selected = next(
                    (row for row in cashout_rows if row["action_key"] == selected_key),
                    None,
                )
            best = max(cashout_rows, key=lambda row: row["cashout_score"])
            chosen = selected or skip_cashout()
            cashout_gap = max(best["cashout_score"] - chosen["cashout_score"], 0.0)
            kinds, needs_rollout, confidence, notes = classify_case(
                selected=selected, best=best, cashout_gap=cashout_gap, min_gap=min_gap
            )
            for kind in kinds:
                if kind != "small_cashout_gap_ignore":
                    regret_counts[kind] += 1
            chosen_cashout_counts[str(chosen.get("dominant_cashout") or (chosen.get("bucket_ev") or {}).get("dominant_cashout") or "skip")] += 1
            best_cashout_counts[str(best["bucket_ev"]["dominant_cashout"])] += 1
            comparisons.append(
                {
                    "trace_file": str(trace_path),
                    "seed": seed,
                    "step_index": int(step.get("step_index") or 0),
                    "act": int(step.get("act") or obs.get("act") or 0),
                    "floor": int(step.get("floor") or obs.get("floor") or 0),
                    "hp": int(step.get("hp") or obs.get("current_hp") or 0),
                    "chosen": compact_candidate(chosen),
                    "best_by_cashout": compact_candidate(best),
                    "cashout_gap": cashout_gap,
                    "cashout_kinds": kinds,
                    "needs_rollout": needs_rollout,
                    "confidence": confidence,
                    "notes": notes,
                    "candidates": [compact_candidate(row) for row in cashout_rows],
                    "relevance": relevance,
                    "deck_plan_profile": obs.get("plan_profile") or {},
                }
            )

    actionable = [
        row
        for row in comparisons
        if row["cashout_gap"] >= min_gap
        and "small_cashout_gap_ignore" not in row["cashout_kinds"]
    ]
    return {
        "policy": policy,
        "trace_dir": str(path),
        "decision_count": len(comparisons),
        "actionable_regret_count": len(actionable),
        "cashout_kind_counts": dict(regret_counts),
        "needs_rollout_count": sum(1 for row in actionable if row["needs_rollout"]),
        "average_cashout_gap": average([row["cashout_gap"] for row in comparisons]),
        "average_actionable_cashout_gap": average(
            [row["cashout_gap"] for row in actionable]
        ),
        "chosen_cashout_counts": dict(chosen_cashout_counts),
        "best_cashout_counts": dict(best_cashout_counts),
        "top_cases": [
            compact_case(row)
            for row in sorted(actionable, key=lambda row: row["cashout_gap"], reverse=True)[
                :top_cases
            ]
        ],
        "comparisons": comparisons,
    }


def average(values: list[float]) -> float:
    return float(mean(values)) if values else 0.0


def pct(value: float) -> str:
    return f"{value:.1%}"


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        f"# Card Cashout Lab {REPORT_VERSION}",
        "",
        f"Generated: `{report['generated_at_utc']}`",
        "",
        "This report is a heuristic cashout diagnostic. It is not a teacher label, policy, or trainer.",
        "",
        "## Calibration",
        "",
    ]
    calibration = report.get("calibration") or {}
    lines.extend(
        [
            f"- baseline policy: `{calibration.get('baseline_policy') or 'none'}`",
            f"- baseline actionable cashout disagreements: `{calibration.get('baseline_actionable_regret_count', 0)}`",
            f"- baseline suspect best cards: `{', '.join(calibration.get('baseline_suspect_best_cards') or []) or '-'}`",
            "",
        ]
    )
    lines.extend(
        [
        "## Summary",
        "",
        "| policy | decisions | actionable | high confidence | needs rollout | baseline disagreement | avg gap | top cashout regrets |",
        "|---|---:|---:|---:|---:|---:|---:|---|",
        ]
    )
    for policy in report["policies"]:
        top_kinds = ", ".join(
            f"{kind}:{count}"
            for kind, count in Counter(policy["cashout_kind_counts"]).most_common(5)
        )
        lines.append(
            "| {policy} | {decisions} | {actionable} | {high_conf} | {rollout} ({rollout_share}) | {baseline_disagree} | {gap:.1f} | {kinds} |".format(
                policy=policy["policy"],
                decisions=policy["decision_count"],
                actionable=policy["actionable_regret_count"],
                high_conf=policy.get("high_confidence_candidate_count", 0),
                rollout=policy.get("needs_rollout_calibrated_count", policy["needs_rollout_count"]),
                rollout_share=pct(
                    policy.get("needs_rollout_calibrated_count", policy["needs_rollout_count"])
                    / max(policy["actionable_regret_count"], 1)
                ),
                baseline_disagree=policy.get("rule_baseline_disagreement_count", 0),
                gap=policy["average_cashout_gap"],
                kinds=top_kinds or "-",
            )
        )
    lines.extend(["", "## High Confidence Candidate Cases", ""])
    for policy in report["policies"]:
        cases = policy.get("high_confidence_cases") or []
        lines.extend([f"### {policy['policy']}", ""])
        if not cases:
            lines.append("- none")
            lines.append("")
            continue
        for case in cases[:8]:
            best = case["best_by_cashout"]
            chosen = case["chosen"]
            lines.append(
                "- seed `{seed}` step `{step}` floor `{floor}`: `{chosen}` -> `{best}`, gap `{gap:.0f}`, best `{dominant}` `{score:.0f}`".format(
                    seed=case["seed"],
                    step=case["step_index"],
                    floor=case["floor"],
                    chosen=chosen["card_id"],
                    best=best["card_id"],
                    gap=case["cashout_gap"],
                    dominant=best["dominant_cashout"],
                    score=best["cashout_score"],
                )
            )
        lines.append("")
    lines.extend(["", "## Top Cashout Regret Cases", ""])
    for policy in report["policies"]:
        lines.extend([f"### {policy['policy']}", ""])
        for case in policy["top_cases"][:12]:
            best = case["best_by_cashout"]
            chosen = case["chosen"]
            cards = ", ".join(
                "{card}:{score:.0f}:{grade}:{cashout}".format(
                    card=candidate["card_id"],
                    score=candidate["cashout_score"],
                    grade=candidate["cashout_grade"],
                    cashout=candidate["dominant_cashout"],
                )
                for candidate in case["candidates"]
            )
            lines.append(
                "- seed `{seed}` step `{step}` floor `{floor}` hp `{hp}`: chose `{chosen}` ({chosen_score:.0f}, {chosen_cashout}) vs cashout-best `{best}` ({best_score:.0f}, {best_cashout}); gap `{gap:.0f}`; calibration `{calibration}`; kinds `{kinds}`; rollout `{rollout}`; [{cards}]".format(
                    seed=case["seed"],
                    step=case["step_index"],
                    floor=case["floor"],
                    hp=case["hp"],
                    chosen=chosen["card_id"],
                    chosen_score=chosen["cashout_score"],
                    chosen_cashout=chosen["dominant_cashout"],
                    best=best["card_id"],
                    best_score=best["cashout_score"],
                    best_cashout=best["dominant_cashout"],
                    gap=case["cashout_gap"],
                    calibration=case.get("calibration_status", "uncalibrated"),
                    kinds=", ".join(case["cashout_kinds"]),
                    rollout="yes" if case["needs_rollout"] else "no",
                    cards=cards,
                )
            )
            if best["notes"]:
                lines.append(f"  - best notes: {'; '.join(best['notes'])}")
            if case["notes"]:
                lines.append(f"  - case notes: {'; '.join(case['notes'])}")
            if case.get("calibration_notes"):
                lines.append(f"  - calibration: {'; '.join(case['calibration_notes'])}")
        lines.append("")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_top_cases_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# Top Card Cashout Regret Cases",
        "",
        f"Generated: `{report['generated_at_utc']}`",
        "",
        "Each case compares the chosen card against the highest heuristic cashout candidate.",
        "",
    ]
    for policy in report["policies"]:
        lines.extend([f"## {policy['policy']}", ""])
        for case in policy["top_cases"]:
            chosen = case["chosen"]
            best = case["best_by_cashout"]
            lines.extend(
                [
                    "### seed `{seed}` step `{step}` floor `{floor}`".format(
                        seed=case["seed"],
                        step=case["step_index"],
                        floor=case["floor"],
                    ),
                    "",
                    "- chosen: `{}` cashout `{:.1f}` grade `{}` dominant `{}`".format(
                        chosen["card_id"],
                        chosen["cashout_score"],
                        chosen["cashout_grade"],
                        chosen["dominant_cashout"],
                    ),
                    "- cashout-best: `{}` cashout `{:.1f}` grade `{}` dominant `{}`".format(
                        best["card_id"],
                        best["cashout_score"],
                        best["cashout_grade"],
                        best["dominant_cashout"],
                    ),
                    "- regret: `{}`; gap `{:.1f}`, confidence `{}`, rollout `{}`".format(
                        ", ".join(case["cashout_kinds"]),
                        case["cashout_gap"],
                        case["confidence"],
                        "yes" if case["needs_rollout"] else "no",
                    ),
                    "- calibration: `{}`; training candidate `{}`".format(
                        case.get("calibration_status", "uncalibrated"),
                        "yes" if case.get("training_candidate") else "no",
                    ),
                ]
            )
            if case["notes"]:
                lines.append(f"- notes: {'; '.join(case['notes'])}")
            if case.get("calibration_notes"):
                lines.append(f"- calibration notes: {'; '.join(case['calibration_notes'])}")
            flags = case["relevance"].get("context_flags") or []
            warnings = case["relevance"].get("context_warnings") or []
            if flags or warnings:
                lines.append(f"- context: {', '.join(flags + warnings)}")
            lines.append("")
            lines.append(
                "| candidate | score | grade | dominant | p open | p turn2 | combo | front | block | draw | scaling | aoe dmg | multi ctrl | penalty |"
            )
            lines.append("|---|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
            for candidate in case["candidates"]:
                reach = candidate["reachability"]
                ev = candidate["bucket_ev"]
                lines.append(
                    "| {card} | {score:.1f} | {grade} | {dominant} | {open:.2f} | {turn2:.2f} | {combo:.2f} | {front:.1f} | {block:.1f} | {draw:.1f} | {scaling:.1f} | {aoe_damage:.1f} | {multi_control:.1f} | {penalty:.1f} |".format(
                        card=candidate["card_id"],
                        score=candidate["cashout_score"],
                        grade=candidate["cashout_grade"],
                        dominant=candidate["dominant_cashout"],
                        open=reach["p_opening_candidate"],
                        turn2=reach["p_by_turn2_candidate"],
                        combo=reach["p_same_turn_candidate_plus_payoff"],
                        front=ev.get("frontload", 0.0),
                        block=ev.get("block", 0.0),
                        draw=ev.get("draw_cashout", 0.0),
                        scaling=ev.get("scaling_cashout", 0.0),
                        aoe_damage=ev.get("aoe_damage", 0.0),
                        multi_control=ev.get("multi_enemy_control", 0.0),
                        penalty=ev.get("context_penalty", 0.0),
                    )
                )
            lines.append("")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    if args.self_test:
        self_test()
        return 0
    trace_dirs = parse_named_paths(args.trace_dir, "--trace-dir")
    if not trace_dirs:
        raise SystemExit("at least one --trace-dir POLICY=PATH is required")
    policies = [
        summarize_policy(
            policy=policy,
            path=path,
            min_gap=args.min_gap,
            top_cases=args.top_cases,
            opening_hand_size=args.opening_hand_size,
            turn2_seen_cards=args.turn2_seen_cards,
        )
        for policy, path in sorted(trace_dirs.items())
    ]
    calibration = calibrate_policies(
        policies,
        min_gap=args.min_gap,
        top_cases=args.top_cases,
    )
    report = {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {
            "score_kind": SCORE_KIND,
            "min_gap": args.min_gap,
            "top_cases": args.top_cases,
            "opening_hand_size": args.opening_hand_size,
            "turn2_seen_cards": args.turn2_seen_cards,
            "notes": [
                "cashout_score is diagnostic, not a teacher",
                "draw order, reshuffle details, Snecko/Pyramid, and exact relic semantics are not solved in V0",
                "high_confidence_candidate is the only calibration status intended for later training experiments",
            ],
        },
        "calibration": calibration,
        "policies": policies,
    }
    write_json(args.out, report)
    markdown_out = args.markdown_out or args.out.with_suffix(".md")
    write_markdown(markdown_out, report)
    top_cases_out = args.top_cases_out or args.out.parent / "top_cashout_regret_cases.md"
    write_top_cases_markdown(top_cases_out, report)
    print(
        json.dumps(
            {
                "out": str(args.out),
                "markdown_out": str(markdown_out),
                "top_cases_out": str(top_cases_out),
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
