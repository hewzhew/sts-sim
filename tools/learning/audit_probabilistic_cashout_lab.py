#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from statistics import mean
from typing import Any

from combat_rl_common import REPO_ROOT, write_json


REPORT_VERSION = "probabilistic_cashout_lab_v0"
DEFAULT_REPORT = (
    REPO_ROOT
    / "tools"
    / "artifacts"
    / "card_cashout_lab"
    / "multi_policy_4096_same_seed_v0_5"
    / "cashout_report.json"
)

COMPONENTS = (
    "frontload",
    "block",
    "draw_cashout",
    "aoe_damage",
    "multi_enemy_control",
    "scaling_cashout",
    "exhaust",
    "kill_window",
    "letter_opener_skill_bonus",
)


@dataclass(frozen=True)
class ScenarioBucket:
    name: str
    weight: float
    demands: dict[str, float]
    reach_key: str
    prior_weight: float = 0.06
    penalty_weight: float = 1.0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Stress-test cashout attribution against a small probabilistic future-world "
            "model. This is a diagnostic lab, not a teacher or rollout replacement."
        )
    )
    parser.add_argument("--cashout-report", type=Path, default=DEFAULT_REPORT)
    parser.add_argument(
        "--statuses",
        default="needs_rollout,high_confidence_candidate,cashout_disagreement_with_rule_baseline",
        help="Comma-separated calibration statuses to include. Use 'all' for every comparison.",
    )
    parser.add_argument("--max-cases", type=int, default=160)
    parser.add_argument("--top-n", type=int, default=30)
    parser.add_argument(
        "--future-room-window",
        type=int,
        default=8,
        help="How many future floors from the source trace to use for bucket weight adjustment.",
    )
    parser.add_argument(
        "--no-trace-room-priors",
        action="store_true",
        help="Disable trace-derived future-room bucket weighting.",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=REPO_ROOT
        / "tools"
        / "artifacts"
        / "probabilistic_cashout_lab"
        / "cashout_distribution_report.json",
    )
    parser.add_argument("--markdown-out", type=Path)
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def trace_path_from_case(case: dict[str, Any]) -> Path | None:
    raw = case.get("trace_file")
    if not raw:
        return None
    path = Path(str(raw))
    if path.is_absolute():
        return path
    return REPO_ROOT / path


def room_from_step(step: dict[str, Any]) -> str:
    obs = step.get("observation") or {}
    room = obs.get("current_room") or step.get("current_room")
    if room:
        return str(room)
    engine_state = str(step.get("engine_state") or obs.get("engine_state") or "")
    decision_type = str(step.get("decision_type") or obs.get("decision_type") or "")
    if "combat" in engine_state or decision_type == "combat":
        return "MonsterRoom"
    if "event" in engine_state or decision_type == "event":
        return "EventRoom"
    if decision_type == "campfire":
        return "RestRoom"
    if decision_type == "shop":
        return "ShopRoom"
    return ""


def load_trace_rooms(path: Path, cache: dict[str, list[dict[str, Any]]]) -> list[dict[str, Any]]:
    key = str(path)
    if key in cache:
        return cache[key]
    if not path.exists():
        cache[key] = []
        return []
    trace = read_json(path)
    by_floor: dict[tuple[int, int], dict[str, Any]] = {}
    for step in trace.get("steps") or []:
        room = room_from_step(step)
        if not room:
            continue
        act = int(step.get("act") or (step.get("observation") or {}).get("act") or 0)
        floor = int(step.get("floor") or (step.get("observation") or {}).get("floor") or 0)
        if floor <= 0:
            continue
        key_floor = (act, floor)
        by_floor.setdefault(
            key_floor,
            {
                "act": act,
                "floor": floor,
                "room_type": room,
                "decision_types": set(),
            },
        )
        by_floor[key_floor]["decision_types"].add(str(step.get("decision_type") or ""))
        if "Elite" in room or "Boss" in room:
            by_floor[key_floor]["room_type"] = room
    rows = []
    for row in by_floor.values():
        rows.append(
            {
                "act": row["act"],
                "floor": row["floor"],
                "room_type": row["room_type"],
                "decision_types": sorted(row["decision_types"]),
            }
        )
    rows.sort(key=lambda row: (row["act"], row["floor"]))
    cache[key] = rows
    return rows


def future_room_summary(
    case: dict[str, Any],
    *,
    trace_cache: dict[str, list[dict[str, Any]]],
    window: int,
) -> dict[str, Any]:
    path = trace_path_from_case(case)
    rows = load_trace_rooms(path, trace_cache) if path else []
    act = int(case.get("act") or 0)
    floor = int(case.get("floor") or 0)
    future = [
        row
        for row in rows
        if int(row.get("act") or 0) == act
        and floor < int(row.get("floor") or 0) <= floor + max(window, 1)
    ]
    counts = Counter(str(row.get("room_type") or "") for row in future)
    monster_like = sum(count for room, count in counts.items() if "MonsterRoom" in room)
    elite = sum(count for room, count in counts.items() if "Elite" in room)
    boss = sum(count for room, count in counts.items() if "Boss" in room)
    rest = counts.get("RestRoom", 0)
    shop = counts.get("ShopRoom", 0)
    event = counts.get("EventRoom", 0)
    pressure = monster_like + 1.2 * elite + 1.5 * boss
    recovery = rest + 0.5 * shop + 0.25 * event
    return {
        "trace_file": str(path) if path else "",
        "window": window,
        "future_floor_count": len(future),
        "room_type_counts": dict(sorted(counts.items())),
        "monster_like_count": monster_like,
        "elite_count": elite,
        "boss_count": boss,
        "rest_count": rest,
        "shop_count": shop,
        "event_count": event,
        "pressure_score": round(pressure, 3),
        "recovery_score": round(recovery, 3),
    }


def num(value: Any) -> float:
    try:
        return float(value or 0.0)
    except (TypeError, ValueError):
        return 0.0


def profile_value(case: dict[str, Any], field: str) -> float:
    return num((case.get("deck_plan_profile") or {}).get(field))


def context_flags(case: dict[str, Any]) -> set[str]:
    return set(str(flag) for flag in ((case.get("relevance") or {}).get("context_flags") or []))


def scenario_buckets(case: dict[str, Any], future_summary: dict[str, Any] | None = None) -> list[ScenarioBucket]:
    act = int(case.get("act") or 0)
    floor = int(case.get("floor") or 0)
    flags = context_flags(case)
    aoe_deficit = "aoe_readiness_deficit" in flags or profile_value(case, "aoe_supply") < 12
    scaling_deficit = "scaling_deficit" in flags or profile_value(case, "scaling_supply") < 12
    draw_deficit = "draw_deficit" in flags or profile_value(case, "draw_supply") < 10

    if act <= 1 and floor <= 5:
        buckets = [
            ScenarioBucket("act1_immediate_frontload", 0.34, {"frontload": 1.15, "block": 0.35}, "p_opening_candidate"),
            ScenarioBucket("act1_nob_or_single_elite", 0.20, {"frontload": 1.25, "block": 0.20, "draw_cashout": 0.35}, "p_opening_candidate", penalty_weight=1.25),
            ScenarioBucket("act1_multi_enemy", 0.18, {"frontload": 0.45, "aoe_damage": 1.10, "multi_enemy_control": 0.75}, "p_by_turn2_candidate"),
            ScenarioBucket("act1_survival_block", 0.16, {"block": 1.10, "frontload": 0.35, "draw_cashout": 0.35}, "p_opening_candidate"),
            ScenarioBucket("act1_boss_setup_future", 0.12, {"scaling_cashout": 0.90, "draw_cashout": 0.45, "exhaust": 0.30}, "p_by_turn2_candidate"),
        ]
    elif act <= 1 and floor <= 11:
        buckets = [
            ScenarioBucket("act1_mid_frontload", 0.26, {"frontload": 1.00, "block": 0.35}, "p_opening_candidate"),
            ScenarioBucket("act1_mid_multi_enemy", 0.24, {"aoe_damage": 1.15, "multi_enemy_control": 0.80, "frontload": 0.35}, "p_by_turn2_candidate"),
            ScenarioBucket("act1_mid_elite_pressure", 0.20, {"frontload": 1.10, "block": 0.55, "draw_cashout": 0.35}, "p_opening_candidate", penalty_weight=1.15),
            ScenarioBucket("act1_mid_block_control", 0.14, {"block": 0.95, "multi_enemy_control": 0.45, "draw_cashout": 0.35}, "p_by_turn2_candidate"),
            ScenarioBucket("act1_boss_scaling", 0.16, {"scaling_cashout": 1.05, "draw_cashout": 0.50, "exhaust": 0.35}, "p_by_turn2_candidate"),
        ]
    elif act <= 1:
        buckets = [
            ScenarioBucket("act1_boss_frontload_window", 0.32, {"frontload": 0.95, "block": 0.70}, "p_opening_candidate"),
            ScenarioBucket("act1_boss_scaling_window", 0.28, {"scaling_cashout": 1.15, "draw_cashout": 0.45, "exhaust": 0.35}, "p_by_turn2_candidate"),
            ScenarioBucket("act1_boss_survival", 0.24, {"block": 1.05, "frontload": 0.40, "multi_enemy_control": 0.25}, "p_opening_candidate"),
            ScenarioBucket("act1_late_hallway_cleanup", 0.16, {"frontload": 0.55, "aoe_damage": 0.65, "draw_cashout": 0.35}, "p_by_turn2_candidate"),
        ]
    elif act == 2:
        buckets = [
            ScenarioBucket("act2_multi_enemy_pressure", 0.34, {"aoe_damage": 1.25, "multi_enemy_control": 1.05, "block": 0.35}, "p_by_turn2_candidate"),
            ScenarioBucket("act2_burst_frontload", 0.22, {"frontload": 0.95, "block": 0.55}, "p_opening_candidate"),
            ScenarioBucket("act2_block_control", 0.22, {"block": 1.05, "multi_enemy_control": 0.65, "draw_cashout": 0.35}, "p_opening_candidate"),
            ScenarioBucket("act2_scaling_setup", 0.22, {"scaling_cashout": 1.10, "draw_cashout": 0.55, "exhaust": 0.45}, "p_by_turn2_candidate"),
        ]
    else:
        buckets = [
            ScenarioBucket("late_scaling_check", 0.30, {"scaling_cashout": 1.25, "draw_cashout": 0.55, "exhaust": 0.50}, "p_by_turn2_candidate"),
            ScenarioBucket("late_block_control", 0.25, {"block": 1.10, "multi_enemy_control": 0.70}, "p_opening_candidate"),
            ScenarioBucket("late_frontload_window", 0.22, {"frontload": 0.90, "kill_window": 0.35}, "p_opening_candidate"),
            ScenarioBucket("late_multi_enemy", 0.23, {"aoe_damage": 0.90, "multi_enemy_control": 0.85}, "p_by_turn2_candidate"),
        ]

    future_summary = future_summary or {}
    future_pressure = num(future_summary.get("pressure_score"))
    future_recovery = num(future_summary.get("recovery_score"))
    future_elites = int(future_summary.get("elite_count") or 0)
    future_bosses = int(future_summary.get("boss_count") or 0)
    future_monsters = int(future_summary.get("monster_like_count") or 0)

    adjusted: list[ScenarioBucket] = []
    for bucket in buckets:
        demands = dict(bucket.demands)
        weight = bucket.weight
        if aoe_deficit and ("aoe_damage" in demands or "multi_enemy_control" in demands):
            weight *= 1.15
        if scaling_deficit and "scaling_cashout" in demands:
            weight *= 1.10
        if draw_deficit and "draw_cashout" in demands:
            demands["draw_cashout"] = demands.get("draw_cashout", 0.0) * 1.10
        if future_summary.get("future_floor_count"):
            if "multi_enemy" in bucket.name or "aoe" in bucket.name:
                weight *= 1.0 + min(future_monsters, 4) * 0.08
                if act >= 2:
                    weight *= 1.0 + min(future_pressure, 6.0) * 0.03
            if "elite" in bucket.name or "nob" in bucket.name:
                weight *= 1.0 + min(future_elites, 3) * 0.22
            if "boss" in bucket.name or "scaling" in bucket.name:
                weight *= 1.0 + min(future_bosses, 1) * 0.25
                if floor >= 10 and act <= 1:
                    weight *= 1.0 + min(future_pressure, 5.0) * 0.04
            if "survival" in bucket.name or "block" in bucket.name or "control" in bucket.name:
                weight *= 1.0 + min(max(future_pressure - future_recovery, 0.0), 5.0) * 0.05
        adjusted.append(
            ScenarioBucket(
                bucket.name,
                weight,
                demands,
                bucket.reach_key,
                bucket.prior_weight,
                bucket.penalty_weight,
            )
        )
    total = sum(bucket.weight for bucket in adjusted) or 1.0
    return [
        ScenarioBucket(
            bucket.name,
            bucket.weight / total,
            bucket.demands,
            bucket.reach_key,
            bucket.prior_weight,
            bucket.penalty_weight,
        )
        for bucket in adjusted
    ]


def reach_factor(candidate: dict[str, Any], bucket: ScenarioBucket) -> float:
    reach = candidate.get("reachability") or {}
    p = num(reach.get(bucket.reach_key))
    return 0.58 + 0.42 * max(0.0, min(p, 1.0))


def bucket_score(candidate: dict[str, Any], bucket: ScenarioBucket) -> float:
    ev = candidate.get("bucket_ev") or {}
    raw = 0.0
    for field, weight in bucket.demands.items():
        raw += num(ev.get(field)) * weight
    raw *= reach_factor(candidate, bucket)
    raw += num(ev.get("base_prior")) * bucket.prior_weight
    raw -= num(ev.get("context_penalty")) * bucket.penalty_weight
    return round(raw, 3)


def weighted_cvar_low(scores: list[tuple[float, float]], tail: float = 0.30) -> float:
    remaining = max(min(tail, 1.0), 0.01)
    total = 0.0
    used = 0.0
    for score, weight in sorted(scores, key=lambda item: item[0]):
        take = min(weight, remaining)
        total += score * take
        used += take
        remaining -= take
        if remaining <= 1e-9:
            break
    return total / used if used else 0.0


def candidate_lab_eval(candidate: dict[str, Any], buckets: list[ScenarioBucket]) -> dict[str, Any]:
    scored = [
        {
            "bucket": bucket.name,
            "weight": round(bucket.weight, 4),
            "score": bucket_score(candidate, bucket),
            "reach_key": bucket.reach_key,
            "demands": bucket.demands,
        }
        for bucket in buckets
    ]
    expected = sum(row["score"] * row["weight"] for row in scored)
    weighted_scores = [(row["score"], row["weight"]) for row in scored]
    cvar30 = weighted_cvar_low(weighted_scores)
    worst = min(scored, key=lambda row: row["score"])
    best = max(scored, key=lambda row: row["score"])
    variance = sum(row["weight"] * (row["score"] - expected) ** 2 for row in scored)
    static_score = num(candidate.get("cashout_score"))
    flags: list[str] = []
    stddev = variance ** 0.5
    if expected >= 10 and cvar30 >= 3 and stddev <= 12:
        flags.append("stable_cashout")
    if expected >= 24 and cvar30 >= 8:
        flags.append("stable_high_cashout")
    if best["score"] >= 25 and (cvar30 < 4 or stddev >= 18):
        flags.append("narrow_cashout")
    if static_score >= 35 and expected < static_score * 0.45 and cvar30 < 4:
        flags.append("static_overstated")
    if expected - static_score >= 20:
        flags.append("static_understated")
    if stddev >= 18:
        flags.append("high_bucket_variance")
    if num((candidate.get("context_penalties") or {}).get("card_context_uncertainty")) >= 8:
        flags.append("context_uncertain")
    return {
        "card_id": candidate.get("card_id"),
        "action_key": candidate.get("action_key"),
        "primary_class": candidate.get("primary_class"),
        "dominant_cashout": candidate.get("dominant_cashout"),
        "static_cashout_score": round(static_score, 3),
        "lab_expected_score": round(expected, 3),
        "lab_cvar30": round(cvar30, 3),
        "lab_stddev": round(stddev, 3),
        "worst_bucket": {"name": worst["bucket"], "score": worst["score"]},
        "best_bucket": {"name": best["bucket"], "score": best["score"]},
        "bucket_scores": scored,
        "flags": flags,
    }


def status_filter(raw: str) -> set[str] | None:
    items = {item.strip() for item in raw.split(",") if item.strip()}
    if "all" in items:
        return None
    return items


def iter_cases(report: dict[str, Any], statuses: set[str] | None, max_cases: int) -> list[tuple[str, dict[str, Any]]]:
    selected: list[tuple[str, dict[str, Any]]] = []
    for policy in report.get("policies") or []:
        policy_name = str(policy.get("policy") or "unknown")
        rows = list(policy.get("comparisons") or [])
        rows.sort(key=lambda row: num(row.get("cashout_gap")), reverse=True)
        for row in rows:
            if statuses is not None and str(row.get("calibration_status")) not in statuses:
                continue
            selected.append((policy_name, row))
            if len(selected) >= max_cases:
                return selected
    return selected


def evaluate_case(
    policy: str,
    case: dict[str, Any],
    *,
    trace_cache: dict[str, list[dict[str, Any]]],
    future_room_window: int,
    use_trace_room_priors: bool,
) -> dict[str, Any]:
    future_summary = (
        future_room_summary(case, trace_cache=trace_cache, window=future_room_window)
        if use_trace_room_priors
        else {}
    )
    buckets = scenario_buckets(case, future_summary)
    candidates = [candidate_lab_eval(candidate, buckets) for candidate in case.get("candidates") or []]
    candidates.sort(key=lambda item: item["lab_expected_score"], reverse=True)
    lab_best = candidates[0] if candidates else {}
    static_best = case.get("best_by_cashout") or {}
    chosen = case.get("chosen") or {}
    flags: list[str] = []
    if lab_best and lab_best.get("action_key") != static_best.get("action_key"):
        margin = num(lab_best.get("lab_expected_score")) - next(
            (
                num(candidate.get("lab_expected_score"))
                for candidate in candidates
                if candidate.get("action_key") == static_best.get("action_key")
            ),
            0.0,
        )
        if margin >= 8:
            flags.append("lab_overturns_static_best")
    static_eval = next(
        (candidate for candidate in candidates if candidate.get("action_key") == static_best.get("action_key")),
        {},
    )
    if "narrow_cashout" in (static_eval.get("flags") or []):
        flags.append("static_best_is_narrow")
    if "static_overstated" in (static_eval.get("flags") or []):
        flags.append("static_best_overstated")
    if "stable_cashout" in (static_eval.get("flags") or []):
        flags.append("static_best_stable")
    return {
        "case_id": f"{policy}_seed_{case.get('seed')}_step_{case.get('step_index')}_{static_best.get('card_id')}",
        "policy": policy,
        "seed": case.get("seed"),
        "step_index": case.get("step_index"),
        "act": case.get("act"),
        "floor": case.get("floor"),
        "hp": case.get("hp"),
        "calibration_status": case.get("calibration_status"),
        "cashout_gap": case.get("cashout_gap"),
        "cashout_kinds": case.get("cashout_kinds") or [],
        "chosen": {
            "card_id": chosen.get("card_id"),
            "action_key": chosen.get("action_key"),
            "cashout_score": chosen.get("cashout_score"),
        },
        "static_best": {
            "card_id": static_best.get("card_id"),
            "action_key": static_best.get("action_key"),
            "cashout_score": static_best.get("cashout_score"),
            "dominant_cashout": static_best.get("dominant_cashout"),
        },
        "lab_best": {
            "card_id": lab_best.get("card_id"),
            "action_key": lab_best.get("action_key"),
            "lab_expected_score": lab_best.get("lab_expected_score"),
            "lab_cvar30": lab_best.get("lab_cvar30"),
            "flags": lab_best.get("flags") or [],
        },
        "scenario_buckets": [
            {"name": bucket.name, "weight": round(bucket.weight, 4), "demands": bucket.demands}
            for bucket in buckets
        ],
        "future_room_summary": future_summary,
        "candidate_evals": candidates,
        "case_flags": flags,
    }


def summarize(cases: list[dict[str, Any]]) -> dict[str, Any]:
    flag_counts: dict[str, int] = {}
    card_counts: dict[str, int] = {}
    stable_cards: dict[str, int] = {}
    overstated_cards: dict[str, int] = {}
    future_room_counts: Counter[str] = Counter()
    future_pressure_scores: list[float] = []
    future_covered = 0
    for case in cases:
        for flag in case.get("case_flags") or []:
            flag_counts[flag] = flag_counts.get(flag, 0) + 1
        static_card = str((case.get("static_best") or {}).get("card_id") or "unknown")
        card_counts[static_card] = card_counts.get(static_card, 0) + 1
        static_eval = next(
            (
                candidate
                for candidate in case.get("candidate_evals") or []
                if candidate.get("action_key") == (case.get("static_best") or {}).get("action_key")
            ),
            {},
        )
        if "stable_cashout" in (static_eval.get("flags") or []):
            stable_cards[static_card] = stable_cards.get(static_card, 0) + 1
        if "static_overstated" in (static_eval.get("flags") or []):
            overstated_cards[static_card] = overstated_cards.get(static_card, 0) + 1
        future = case.get("future_room_summary") or {}
        if future.get("future_floor_count"):
            future_covered += 1
            future_pressure_scores.append(num(future.get("pressure_score")))
            future_room_counts.update(future.get("room_type_counts") or {})
    expectations = [
        num(((case.get("lab_best") or {}).get("lab_expected_score")))
        for case in cases
        if case.get("lab_best")
    ]
    return {
        "case_count": len(cases),
        "average_lab_best_expected": round(mean(expectations), 3) if expectations else 0.0,
        "case_flag_counts": dict(sorted(flag_counts.items())),
        "static_best_card_counts": sorted(card_counts.items(), key=lambda item: (-item[1], item[0]))[:20],
        "stable_static_cards": sorted(stable_cards.items(), key=lambda item: (-item[1], item[0]))[:20],
        "overstated_static_cards": sorted(overstated_cards.items(), key=lambda item: (-item[1], item[0]))[:20],
        "future_room_prior_coverage": {
            "covered_cases": future_covered,
            "coverage_rate": round(future_covered / len(cases), 4) if cases else 0.0,
            "room_type_counts": dict(sorted(future_room_counts.items())),
            "average_pressure_score": round(mean(future_pressure_scores), 3) if future_pressure_scores else 0.0,
        },
    }


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    summary = report["summary"]
    lines = [
        f"# Probabilistic Cashout Lab {report['report_version']}",
        "",
        "This report is a diagnostic distribution stress test, not a teacher label.",
        "",
        "## Summary",
        "",
        f"- cases: `{summary['case_count']}`",
        f"- average lab-best expected score: `{summary['average_lab_best_expected']}`",
        f"- case flags: `{summary['case_flag_counts']}`",
        f"- future room prior coverage: `{summary['future_room_prior_coverage']}`",
        "",
        "## Static Best Cards",
        "",
        "| card | cases | stable | overstated |",
        "|---|---:|---:|---:|",
    ]
    stable = dict(summary["stable_static_cards"])
    overstated = dict(summary["overstated_static_cards"])
    for card, count in summary["static_best_card_counts"]:
        lines.append(f"| {card} | {count} | {stable.get(card, 0)} | {overstated.get(card, 0)} |")
    lines.extend(["", "## Top Cases", ""])
    for case in report["cases"][: report["config"]["top_n"]]:
        static_best = case["static_best"]
        lab_best = case["lab_best"]
        lines.extend(
            [
                f"### {case['case_id']}",
                "",
                f"- floor: `{case['floor']}` act `{case['act']}` hp `{case['hp']}` status `{case['calibration_status']}`",
                f"- future rooms: `{(case.get('future_room_summary') or {}).get('room_type_counts', {})}`",
                f"- chosen: `{case['chosen']['card_id']}`",
                f"- static best: `{static_best['card_id']}` score `{static_best['cashout_score']}` / `{static_best['dominant_cashout']}`",
                f"- lab best: `{lab_best.get('card_id')}` expected `{lab_best.get('lab_expected_score')}` cvar30 `{lab_best.get('lab_cvar30')}` flags `{lab_best.get('flags')}`",
                f"- case flags: `{case['case_flags']}`",
                "",
                "| candidate | static | expected | cvar30 | worst | best | flags |",
                "|---|---:|---:|---:|---|---|---|",
            ]
        )
        for candidate in case["candidate_evals"][:6]:
            lines.append(
                "| {card} | {static} | {expected} | {cvar} | {worst} {worst_score} | {best} {best_score} | {flags} |".format(
                    card=candidate["card_id"],
                    static=candidate["static_cashout_score"],
                    expected=candidate["lab_expected_score"],
                    cvar=candidate["lab_cvar30"],
                    worst=candidate["worst_bucket"]["name"],
                    worst_score=candidate["worst_bucket"]["score"],
                    best=candidate["best_bucket"]["name"],
                    best_score=candidate["best_bucket"]["score"],
                    flags=", ".join(candidate["flags"]),
                )
            )
        lines.append("")
    lines.extend(
        [
            "## Limitations",
            "",
            "- Scenario buckets are hand-written abstractions, not exact encounter probabilities.",
            "- Candidate bucket EV comes from the cashout report; this script does not run the engine.",
            "- The purpose is to identify stable/narrow/context-dependent cashout, not to create hard labels.",
        ]
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def self_test() -> None:
    case = {"act": 2, "floor": 20, "relevance": {"context_flags": ["aoe_readiness_deficit"]}, "deck_plan_profile": {}}
    buckets = scenario_buckets(case)
    assert abs(sum(bucket.weight for bucket in buckets) - 1.0) < 1e-6
    aoe_candidate = {
        "card_id": "Cleave",
        "action_key": "a",
        "cashout_score": 30,
        "bucket_ev": {"aoe_damage": 45, "frontload": 8, "base_prior": 3},
        "reachability": {"p_opening_candidate": 0.4, "p_by_turn2_candidate": 0.8},
        "context_penalties": {},
    }
    attack_candidate = {
        "card_id": "TwinStrike",
        "action_key": "b",
        "cashout_score": 30,
        "bucket_ev": {"frontload": 35, "base_prior": 3},
        "reachability": {"p_opening_candidate": 0.4, "p_by_turn2_candidate": 0.8},
        "context_penalties": {},
    }
    aoe_eval = candidate_lab_eval(aoe_candidate, buckets)
    attack_eval = candidate_lab_eval(attack_candidate, buckets)
    assert aoe_eval["lab_expected_score"] > attack_eval["lab_expected_score"]
    assert aoe_eval["lab_cvar30"] <= aoe_eval["lab_expected_score"]
    print(json.dumps({"self_test": "ok", "aoe_expected": aoe_eval["lab_expected_score"]}))


def main() -> int:
    args = parse_args()
    if args.self_test:
        self_test()
        return 0
    cashout_report = read_json(args.cashout_report)
    statuses = status_filter(args.statuses)
    selected = iter_cases(cashout_report, statuses, args.max_cases)
    trace_cache: dict[str, list[dict[str, Any]]] = {}
    cases = [
        evaluate_case(
            policy,
            case,
            trace_cache=trace_cache,
            future_room_window=args.future_room_window,
            use_trace_room_priors=not args.no_trace_room_priors,
        )
        for policy, case in selected
    ]
    cases.sort(
        key=lambda case: (
            "static_best_overstated" not in case["case_flags"],
            -num((case.get("static_best") or {}).get("cashout_score")),
        )
    )
    report = {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {
            "cashout_report": str(args.cashout_report),
            "cashout_report_version": cashout_report.get("report_version"),
            "statuses": args.statuses,
            "max_cases": args.max_cases,
            "top_n": args.top_n,
            "future_room_window": args.future_room_window,
            "trace_room_priors": not args.no_trace_room_priors,
        },
        "summary": summarize(cases),
        "cases": cases,
        "limitations": [
            "bucket probabilities are abstract and manually specified",
            "candidate EV is inherited from cashout_lab and not recomputed from engine simulation",
            "stable_cashout is a diagnostic signal, not a training label by itself",
        ],
    }
    write_json(args.out, report)
    markdown_out = args.markdown_out or args.out.with_suffix(".md")
    write_markdown(markdown_out, report)
    print(json.dumps({"out": str(args.out), "markdown_out": str(markdown_out)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
