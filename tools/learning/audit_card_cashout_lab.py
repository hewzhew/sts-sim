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


REPORT_VERSION = "card_cashout_lab_v0_3"
SCORE_KIND = "heuristic_cashout_v0_3"

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
ENERGY_CARDS = {"Offering", "SeeingRed", "Bloodletting", "Dropkick", "Sentinel"}
MULTI_ENEMY_CONTROL_CARDS = {"Shockwave", "ThunderClap"}
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
    return bool(card.get("aoe") or card.get("multi_damage"))


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
    print(json.dumps({"self_test": "ok", "p_opening": p_opening, "p_combo": p_combo}))


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


def context_penalties(
    *,
    card: dict[str, Any],
    delta: dict[str, int],
    obs: dict[str, Any],
    profile: dict[str, Any],
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
    if card_cost(card) >= 3:
        values["high_curve_clog_risk"] += 8.0 + max(card_cost(card) - 3, 0) * 4.0
        if is_scaling(card) and float(profile.get("frontload_supply") or 0) < 70:
            values["high_curve_clog_risk"] += 8.0
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


def base_prior_value(candidate: dict[str, Any], card: dict[str, Any]) -> float:
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
    if is_aoe_damage(card):
        return 0.17 * rule + 0.09 * plan
    if direct_output and not slow_plan:
        return 0.14 * rule + 0.07 * plan
    if direct_output and slow_plan:
        return 0.10 * rule + 0.05 * plan
    return 0.07 * rule + 0.035 * plan


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
        if card_damage(card) > 0 and card_block(card) <= 0:
            draw_cashout *= 0.55
        elif card_block(card) > 0:
            draw_cashout *= 0.80
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
    base_prior = base_prior_value(candidate, card)
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
    if cid == "FeelNoPain":
        exhaust_count = classes.get("exhaust_engine", 0)
        if exhaust_count <= 0:
            return 0.45
        if exhaust_count == 1:
            return 0.75
        return 1.15
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
    penalties = context_penalties(card=card, delta=delta, obs=obs, profile=profile)
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
