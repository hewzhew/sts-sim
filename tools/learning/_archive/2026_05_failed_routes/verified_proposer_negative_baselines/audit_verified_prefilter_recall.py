#!/usr/bin/env python3
"""Audit whether a cheap prefilter keeps the full verified teacher's choices.

This is not a closed-loop score and it does not train a model.  For each combat
DecisionState it evaluates:

1. full verifier over all scoped candidates
2. cheap prefilter over the same candidates
3. whether each topK/margin prefilter setting would keep the full verifier's
   selected override candidate
"""
from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from return_q_common import FullRunDriver, legal_candidate_indices, write_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--misses-out", type=Path)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--episodes", type=int, default=20)
    parser.add_argument("--seed-start", type=int, default=98100)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=160)
    parser.add_argument("--max-groups", type=int, default=0)
    parser.add_argument("--trajectory-policy", default="full_verified", choices=["full_verified", "rule_baseline_v0"])
    parser.add_argument("--candidate-scope", default="controlled_v1", choices=["all", "controlled_v0", "controlled_v1"])
    parser.add_argument("--horizon-decisions", type=int, default=8)
    parser.add_argument(
        "--horizon-mode",
        default="fixed_decisions",
        choices=["fixed_decisions", "adaptive_next_player_turn_v1", "adaptive_payoff_window_v1"],
    )
    parser.add_argument("--oracle-margin", type=float, default=1.0)
    parser.add_argument("--prefilter-horizon-decisions", type=int, default=8)
    parser.add_argument(
        "--prefilter-horizon-mode",
        default="adaptive_payoff_window_v1",
        choices=["fixed_decisions", "adaptive_next_player_turn_v1", "adaptive_payoff_window_v1"],
    )
    parser.add_argument("--prefilter-margins", default="1.0,2.0,3.0")
    parser.add_argument("--prefilter-top-ks", default="0,1,2")
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--continuation-policy", default="rule_baseline_v0")
    parser.add_argument("--evaluation-mode", default="independent", choices=["independent", "bellman_cached_v1"])
    parser.add_argument("--value-cache-scope", default="episode", choices=["request", "episode"])
    parser.add_argument("--value-cache-max-entries", type=int, default=4096)
    parser.add_argument("--parallelism", type=int, default=0)
    parser.add_argument("--exact-root-dedup", action=argparse.BooleanOptionalAction, default=False)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    margins = parse_float_list(args.prefilter_margins)
    top_ks = parse_int_list(args.prefilter_top_ks)
    cases = {}
    for top_k in top_ks:
        for margin in margins:
            cases[case_key(top_k, margin)] = new_case_stats(top_k, margin)
    aggregate = {
        "schema_version": "verified_prefilter_recall_audit_v0",
        "config": {
            "episodes": args.episodes,
            "seed_start": args.seed_start,
            "seed_step": args.seed_step,
            "max_steps": args.max_steps,
            "max_groups": args.max_groups,
            "trajectory_policy": args.trajectory_policy,
            "candidate_scope": args.candidate_scope,
            "horizon_decisions": args.horizon_decisions,
            "horizon_mode": args.horizon_mode,
            "oracle_margin": args.oracle_margin,
            "prefilter_horizon_decisions": args.prefilter_horizon_decisions,
            "prefilter_horizon_mode": args.prefilter_horizon_mode,
            "prefilter_margins": args.prefilter_margins,
            "prefilter_top_ks": args.prefilter_top_ks,
            "continuation_policy": args.continuation_policy,
        },
        "episodes_started": 0,
        "groups": 0,
        "skipped_groups": 0,
        "skip_reasons": {},
        "full_candidate_evaluation_count": 0,
        "prefilter_candidate_evaluation_count": 0,
        "full_policy_step_eval_count": 0,
        "prefilter_policy_step_eval_count": 0,
        "decision_type_counts": {},
        "full_override_count": 0,
        "full_positive_candidate_count": 0,
        "full_override_payoff_reason_counts": {},
        "cases": cases,
    }
    counters = {
        "skip_reasons": Counter(),
        "decision_type_counts": Counter(),
        "full_override_payoff_reason_counts": Counter(),
    }

    misses_handle = None
    if args.misses_out:
        args.misses_out.parent.mkdir(parents=True, exist_ok=True)
        misses_handle = args.misses_out.open("w", encoding="utf-8")

    driver = FullRunDriver(args.binary)
    try:
        for episode_index in range(args.episodes):
            if args.max_groups and int(aggregate["groups"]) >= args.max_groups:
                break
            seed = args.seed_start + episode_index * args.seed_step
            aggregate["episodes_started"] += 1
            collect_episode(args, driver, seed, margins, top_ks, aggregate, counters, misses_handle)
    finally:
        if misses_handle:
            misses_handle.close()
        driver.close()

    for name, counter in counters.items():
        aggregate[name] = dict(sorted(counter.items()))
    for stats in aggregate["cases"].values():
        finalize_case_stats(stats)
    write_json(args.out, aggregate)
    print(json.dumps(render_summary(aggregate), indent=2, sort_keys=True))


def collect_episode(
    args: argparse.Namespace,
    driver: FullRunDriver,
    seed: int,
    margins: list[float],
    top_ks: list[int],
    aggregate: dict[str, Any],
    counters: dict[str, Counter[str]],
    misses_handle: Any,
) -> None:
    response = driver.request(
        {
            "cmd": "reset",
            "seed": seed,
            "ascension": args.ascension,
            "final_act": args.final_act,
            "class": args.player_class,
            "max_steps": args.max_steps,
            "reward_shaping_profile": "baseline",
        }
    )
    done = bool(response.get("done"))
    step = 0
    while not done and step < args.max_steps:
        if args.max_groups and int(aggregate["groups"]) >= args.max_groups:
            return
        payload = response.get("payload") or {}
        observation = payload.get("observation") or {}
        decision_type = str(observation.get("decision_type") or "")
        if decision_type.startswith("combat"):
            selected = audit_decision(
                args,
                driver,
                seed,
                step,
                response,
                margins,
                top_ks,
                aggregate,
                counters,
                misses_handle,
            )
            if args.trajectory_policy == "full_verified" and selected is not None:
                response = driver.request({"cmd": "step", "action_index": selected})
            else:
                response = driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
        else:
            response = driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
        done = bool(response.get("done"))
        step += 1


def audit_decision(
    args: argparse.Namespace,
    driver: FullRunDriver,
    seed: int,
    step: int,
    response: dict[str, Any],
    margins: list[float],
    top_ks: list[int],
    aggregate: dict[str, Any],
    counters: dict[str, Counter[str]],
    misses_handle: Any,
) -> int | None:
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    candidates = payload.get("action_candidates") or []
    decision_type = str(observation.get("decision_type") or "")
    counters["decision_type_counts"][decision_type] += 1

    scoped = legal_candidate_indices(response, args.candidate_scope)
    if not scoped:
        record_skip(aggregate, counters, "no_scoped_candidates")
        return None
    rule_index = preview_rule_index(driver)
    legal_all = set(legal_candidate_indices(response, "all"))
    if rule_index is None or rule_index not in legal_all:
        record_skip(aggregate, counters, "missing_rule_action")
        return scoped[0]
    if not any(index != rule_index for index in scoped):
        record_skip(aggregate, counters, "only_rule_candidate")
        return rule_index

    eval_indices = sorted({rule_index, *scoped})
    full_payload = evaluate_indices(
        args,
        driver,
        eval_indices,
        args.horizon_decisions,
        args.horizon_mode,
    )
    prefilter_payload = evaluate_indices(
        args,
        driver,
        eval_indices,
        args.prefilter_horizon_decisions,
        args.prefilter_horizon_mode,
    )
    full_by_index = evaluation_by_index(full_payload)
    prefilter_by_index = evaluation_by_index(prefilter_payload)
    rule_full = full_by_index.get(rule_index)
    rule_prefilter = prefilter_by_index.get(rule_index)
    if not rule_full or not rule_prefilter:
        record_skip(aggregate, counters, "missing_rule_evaluation")
        return rule_index
    scoped_evaluated = [idx for idx in scoped if idx in full_by_index]
    if not scoped_evaluated:
        record_skip(aggregate, counters, "missing_scoped_evaluations")
        return rule_index

    aggregate["groups"] += 1
    aggregate["full_candidate_evaluation_count"] += int(full_payload.get("root_candidate_count") or len(full_by_index))
    aggregate["prefilter_candidate_evaluation_count"] += int(
        prefilter_payload.get("root_candidate_count") or len(prefilter_by_index)
    )
    aggregate["full_policy_step_eval_count"] += int(full_payload.get("policy_step_eval_count") or 0)
    aggregate["prefilter_policy_step_eval_count"] += int(prefilter_payload.get("policy_step_eval_count") or 0)

    rule_return = float(rule_full.get("discounted_return") or 0.0)
    best_index = max(scoped_evaluated, key=lambda idx: float(full_by_index[idx].get("discounted_return") or 0.0))
    best_return = float(full_by_index[best_index].get("discounted_return") or 0.0)
    full_best_adv = best_return - rule_return
    full_selected = best_index if best_index != rule_index and full_best_adv > args.oracle_margin else rule_index
    full_selected_return = float(full_by_index[full_selected].get("discounted_return") or rule_return)
    positives = {
        idx
        for idx in scoped_evaluated
        if idx != rule_index and float(full_by_index[idx].get("discounted_return") or 0.0) - rule_return > args.oracle_margin
    }
    aggregate["full_positive_candidate_count"] += len(positives)
    full_override = full_selected != rule_index
    if full_override:
        aggregate["full_override_count"] += 1
        for reason in full_by_index[full_selected].get("payoff_reasons") or ["none"]:
            counters["full_override_payoff_reason_counts"][str(reason)] += 1

    prefilter_rule_return = float(rule_prefilter.get("discounted_return") or 0.0)
    ranked_prefilter = []
    for idx in scoped:
        if idx == rule_index:
            continue
        row = prefilter_by_index.get(idx)
        if not row:
            continue
        cheap_return = float(row.get("discounted_return") or 0.0)
        ranked_prefilter.append((idx, cheap_return, cheap_return - prefilter_rule_return))
    ranked_prefilter.sort(key=lambda item: (-item[1], item[0]))
    prefilter_rank = {idx: rank + 1 for rank, (idx, _ret, _adv) in enumerate(ranked_prefilter)}
    prefilter_adv = {idx: adv for idx, _ret, adv in ranked_prefilter}

    for top_k in top_ks:
        for margin in margins:
            key = case_key(top_k, margin)
            stats = aggregate["cases"][key]
            kept = kept_by_prefilter(ranked_prefilter, top_k, margin)
            final_indices = {rule_index, *kept}
            final_evaluated = [idx for idx in final_indices if idx in full_by_index]
            simulated_best = max(
                final_evaluated,
                key=lambda idx: float(full_by_index[idx].get("discounted_return") or 0.0),
            )
            simulated_best_return = float(full_by_index[simulated_best].get("discounted_return") or 0.0)
            simulated_adv = simulated_best_return - rule_return
            simulated_selected = (
                simulated_best
                if simulated_best != rule_index and simulated_adv > args.oracle_margin
                else rule_index
            )
            simulated_selected_return = float(full_by_index[simulated_selected].get("discounted_return") or rule_return)
            regret = max(0.0, full_selected_return - simulated_selected_return)
            update_case_stats(
                stats,
                scoped_non_rule_count=len([idx for idx in scoped if idx != rule_index]),
                kept_non_rule_count=len(kept),
                positive_count=len(positives),
                positive_kept_count=len(positives & kept),
                full_override=full_override,
                full_selected_kept=(full_selected == rule_index or full_selected in kept),
                selected_match=(simulated_selected == full_selected),
                simulated_override=(simulated_selected != rule_index),
                regret=regret,
            )
            if full_override and full_selected not in kept:
                record_miss(
                    misses_handle,
                    key,
                    seed,
                    step,
                    decision_type,
                    candidates,
                    full_selected,
                    rule_index,
                    full_best_adv,
                    prefilter_rank.get(full_selected),
                    prefilter_adv.get(full_selected),
                    len(scoped),
                    len(kept),
                    full_by_index[full_selected],
                    observation,
                )
    return full_selected


def evaluate_indices(
    args: argparse.Namespace,
    driver: FullRunDriver,
    indices: list[int],
    horizon: int,
    horizon_mode: str,
) -> dict[str, Any]:
    return driver.request(
        {
            "cmd": "evaluate_candidates",
            "action_indices": indices,
            "continuation_policy": args.continuation_policy,
            "horizon_decisions": horizon,
            "horizon_mode": horizon_mode,
            "gamma": args.gamma,
            "evaluation_mode": args.evaluation_mode,
            "value_cache_scope": args.value_cache_scope,
            "value_cache_max_entries": args.value_cache_max_entries,
            "parallelism": args.parallelism,
            "exact_root_dedup": args.exact_root_dedup,
            "include_state": False,
            "include_next_state": False,
            "include_continuation_trace": False,
            "check_live_env_unchanged": False,
        }
    ).get("payload") or {}


def evaluation_by_index(payload: dict[str, Any]) -> dict[int, dict[str, Any]]:
    return {
        int(item.get("action_index")): item
        for item in (payload.get("evaluations") or [])
        if item.get("ok") and item.get("action_index") is not None
    }


def kept_by_prefilter(ranked_prefilter: list[tuple[int, float, float]], top_k: int, margin: float) -> set[int]:
    kept = {idx for idx, _ret, adv in ranked_prefilter if adv > margin}
    kept.update(idx for idx, _ret, _adv in ranked_prefilter[:top_k])
    return kept


def preview_rule_index(driver: FullRunDriver) -> int | None:
    payload = driver.request(
        {
            "cmd": "preview_policy_action",
            "policy": "rule_baseline_v0",
            "include_state": False,
            "include_next_state": False,
            "check_live_env_unchanged": False,
        }
    ).get("payload") or {}
    value = payload.get("chosen_action_index")
    return int(value) if value is not None else None


def new_case_stats(top_k: int, margin: float) -> dict[str, Any]:
    return {
        "prefilter_top_k": top_k,
        "prefilter_margin": margin,
        "decision_count": 0,
        "full_override_count": 0,
        "full_override_kept_count": 0,
        "full_override_missed_count": 0,
        "full_selected_match_count": 0,
        "simulated_override_count": 0,
        "positive_candidate_count": 0,
        "positive_candidate_kept_count": 0,
        "scoped_non_rule_candidate_count": 0,
        "kept_non_rule_candidate_count": 0,
        "regret_sum": 0.0,
        "regret_max": 0.0,
    }


def update_case_stats(
    stats: dict[str, Any],
    *,
    scoped_non_rule_count: int,
    kept_non_rule_count: int,
    positive_count: int,
    positive_kept_count: int,
    full_override: bool,
    full_selected_kept: bool,
    selected_match: bool,
    simulated_override: bool,
    regret: float,
) -> None:
    stats["decision_count"] += 1
    stats["scoped_non_rule_candidate_count"] += scoped_non_rule_count
    stats["kept_non_rule_candidate_count"] += kept_non_rule_count
    stats["positive_candidate_count"] += positive_count
    stats["positive_candidate_kept_count"] += positive_kept_count
    if full_override:
        stats["full_override_count"] += 1
        if full_selected_kept:
            stats["full_override_kept_count"] += 1
        else:
            stats["full_override_missed_count"] += 1
    if selected_match:
        stats["full_selected_match_count"] += 1
    if simulated_override:
        stats["simulated_override_count"] += 1
    stats["regret_sum"] += regret
    stats["regret_max"] = max(float(stats["regret_max"]), regret)


def finalize_case_stats(stats: dict[str, Any]) -> None:
    decisions = int(stats["decision_count"])
    full_overrides = int(stats["full_override_count"])
    positives = int(stats["positive_candidate_count"])
    scoped = int(stats["scoped_non_rule_candidate_count"])
    kept = int(stats["kept_non_rule_candidate_count"])
    stats["full_override_recall"] = ratio(stats["full_override_kept_count"], full_overrides)
    stats["positive_candidate_recall"] = ratio(stats["positive_candidate_kept_count"], positives)
    stats["full_selected_match_rate"] = ratio(stats["full_selected_match_count"], decisions)
    stats["simulated_override_rate"] = ratio(stats["simulated_override_count"], decisions)
    stats["average_scoped_non_rule_candidates"] = ratio(scoped, decisions)
    stats["average_kept_non_rule_candidates"] = ratio(kept, decisions)
    stats["kept_non_rule_candidate_rate"] = ratio(kept, scoped)
    stats["average_regret"] = ratio(stats["regret_sum"], decisions)
    stats["average_regret_on_full_overrides"] = ratio(stats["regret_sum"], full_overrides)


def record_skip(aggregate: dict[str, Any], counters: dict[str, Counter[str]], reason: str) -> None:
    aggregate["skipped_groups"] += 1
    counters["skip_reasons"][reason] += 1


def record_miss(
    handle: Any,
    case: str,
    seed: int,
    step: int,
    decision_type: str,
    candidates: list[dict[str, Any]],
    full_selected: int,
    rule_index: int,
    full_adv: float,
    prefilter_rank: int | None,
    prefilter_adv: float | None,
    scoped_count: int,
    kept_count: int,
    selected_eval: dict[str, Any],
    observation: dict[str, Any],
) -> None:
    if not handle:
        return
    row = {
        "case": case,
        "seed": seed,
        "step": step,
        "decision_type": decision_type,
        "context": compact_context(observation),
        "rule_index": rule_index,
        "full_selected_index": full_selected,
        "full_selected_action_key": action_key_at(candidates, full_selected),
        "rule_action_key": action_key_at(candidates, rule_index),
        "full_adv_vs_rule": full_adv,
        "prefilter_rank": prefilter_rank,
        "prefilter_adv_vs_rule": prefilter_adv,
        "scoped_candidate_count": scoped_count,
        "kept_non_rule_candidate_count": kept_count,
        "payoff_reasons": selected_eval.get("payoff_reasons") or [],
    }
    handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")


def compact_context(observation: dict[str, Any]) -> dict[str, Any]:
    combat = observation.get("combat") or {}
    hp_ratio = observation.get("hp_ratio_milli")
    return {
        "act": observation.get("act"),
        "floor": observation.get("floor"),
        "room": observation.get("current_room"),
        "hp_ratio_milli": hp_ratio,
        "hp_band": hp_band(hp_ratio),
        "energy": combat.get("energy"),
        "turn_count": combat.get("turn_count"),
        "visible_incoming_damage": combat.get("visible_incoming_damage"),
        "player_block": combat.get("player_block"),
        "alive_monster_count": combat.get("alive_monster_count"),
        "hand_count": combat.get("hand_count"),
        "draw_count": combat.get("draw_count"),
        "discard_count": combat.get("discard_count"),
    }


def hp_band(value: Any) -> str:
    try:
        ratio_milli = float(value)
    except (TypeError, ValueError):
        return "unknown"
    if ratio_milli < 350:
        return "low"
    if ratio_milli < 700:
        return "mid"
    return "high"


def action_key_at(candidates: list[dict[str, Any]], index: int) -> str:
    if index < 0 or index >= len(candidates):
        return ""
    return str(candidates[index].get("action_key") or "")


def case_key(top_k: int, margin: float) -> str:
    return f"top{top_k}_margin{float_key(margin)}"


def float_key(value: float) -> str:
    return str(value).replace(".", "p").replace("-", "neg")


def ratio(numerator: Any, denominator: Any) -> float | None:
    denominator_f = float(denominator or 0.0)
    if denominator_f == 0.0:
        return None
    return float(numerator or 0.0) / denominator_f


def parse_int_list(text: str) -> list[int]:
    values = [int(item.strip()) for item in text.split(",") if item.strip()]
    if not values:
        raise SystemExit("expected at least one topK")
    return values


def parse_float_list(text: str) -> list[float]:
    values = [float(item.strip()) for item in text.split(",") if item.strip()]
    if not values:
        raise SystemExit("expected at least one margin")
    return values


def render_summary(payload: dict[str, Any]) -> dict[str, Any]:
    rows = []
    for key, stats in payload["cases"].items():
        rows.append(
            {
                "case": key,
                "override_recall": stats.get("full_override_recall"),
                "positive_recall": stats.get("positive_candidate_recall"),
                "match_rate": stats.get("full_selected_match_rate"),
                "avg_kept": stats.get("average_kept_non_rule_candidates"),
                "kept_rate": stats.get("kept_non_rule_candidate_rate"),
                "missed_overrides": stats.get("full_override_missed_count"),
                "avg_regret": stats.get("average_regret"),
                "max_regret": stats.get("regret_max"),
            }
        )
    rows.sort(key=lambda row: (str(row["case"])))
    return {
        "schema_version": payload["schema_version"],
        "groups": payload["groups"],
        "full_override_count": payload["full_override_count"],
        "full_positive_candidate_count": payload["full_positive_candidate_count"],
        "full_policy_step_eval_count": payload["full_policy_step_eval_count"],
        "prefilter_policy_step_eval_count": payload["prefilter_policy_step_eval_count"],
        "cases": rows,
    }


if __name__ == "__main__":
    main()
