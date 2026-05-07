#!/usr/bin/env python3
"""Collect full-H verified override rows for training a high-recall proposer."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from return_q_common import FullRunDriver, legal_candidate_indices, stable_group_split, write_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--episodes", type=int, default=100)
    parser.add_argument("--seed-start", type=int, default=98100)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=160)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--candidate-scope", default="controlled_v1", choices=["all", "controlled_v0", "controlled_v1"])
    parser.add_argument("--horizon-decisions", type=int, default=8)
    parser.add_argument("--oracle-margin", type=float, default=1.0)
    parser.add_argument("--oracle-continuation-policy", default="rule_baseline_v0")
    parser.add_argument("--verified-parallelism", type=int, default=0)
    parser.add_argument(
        "--proposer-feature-horizons",
        default="",
        help="Optional cheap horizons, e.g. '1,2', to store per-candidate return features for proposer training.",
    )
    parser.add_argument(
        "--include-one-step-delta",
        action="store_true",
        help="Include engine one-step next-state deltas for resource-retention proposer features.",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    summary = {
        "schema_version": "verified_proposer_collection_summary_v0",
        "row_count": 0,
        "group_count": 0,
        "positive_count": 0,
        "negative_count": 0,
        "episode_count": args.episodes,
        "override_group_count": 0,
        "candidate_evaluation_count": 0,
        "feature_evaluation_count": 0,
        "one_step_delta_evaluation_count": 0,
    }
    driver = FullRunDriver(args.binary)
    try:
        with args.out.open("w", encoding="utf-8") as handle:
            for episode_index in range(args.episodes):
                seed = args.seed_start + episode_index * args.seed_step
                episode_summary = collect_episode(args, driver, seed, handle)
                for key, value in episode_summary.items():
                    summary[key] = int(summary.get(key, 0)) + int(value)
    finally:
        driver.close()
    summary["positive_rate"] = (
        summary["positive_count"] / summary["row_count"] if summary["row_count"] else 0.0
    )
    summary_out = args.summary_out or args.out.with_suffix(".summary.json")
    write_json(summary_out, summary)
    print(json.dumps(summary, indent=2, sort_keys=True))


def collect_episode(
    args: argparse.Namespace,
    driver: FullRunDriver,
    seed: int,
    handle: Any,
) -> dict[str, int]:
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
    steps = 0
    summary = {
        "row_count": 0,
        "group_count": 0,
        "positive_count": 0,
        "negative_count": 0,
        "override_group_count": 0,
        "candidate_evaluation_count": 0,
        "feature_evaluation_count": 0,
        "one_step_delta_evaluation_count": 0,
    }
    while not done and steps < args.max_steps:
        payload = response.get("payload") or {}
        observation = payload.get("observation") or {}
        decision_type = str(observation.get("decision_type") or "")
        if decision_type.startswith("combat"):
            chosen_index, group_summary = collect_decision_group(args, driver, seed, steps, response, handle)
            for key, value in group_summary.items():
                summary[key] += value
            response = driver.request({"cmd": "step", "action_index": chosen_index})
        else:
            response = driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
        done = bool(response.get("done"))
        steps += 1
    return summary


def collect_decision_group(
    args: argparse.Namespace,
    driver: FullRunDriver,
    seed: int,
    step: int,
    response: dict[str, Any],
    handle: Any,
) -> tuple[int, dict[str, int]]:
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    candidates = payload.get("action_candidates") or []
    scoped = legal_candidate_indices(response, args.candidate_scope)
    if not scoped:
        return 0, empty_group_summary()

    rule_index = preview_rule_index(driver)
    legal_all = set(legal_candidate_indices(response, "all"))
    if rule_index is None or rule_index not in legal_all:
        return scoped[0], empty_group_summary()
    if not any(index != rule_index for index in scoped):
        return rule_index, empty_group_summary()

    eval_indices = sorted({rule_index, *scoped})
    feature_horizons = parse_int_list(args.proposer_feature_horizons)
    cheap_by_horizon: dict[int, dict[int, dict[str, Any]]] = {}
    for horizon in feature_horizons:
        if horizon < 0 or horizon == args.horizon_decisions:
            continue
        cheap_payload = evaluate_indices(args, driver, eval_indices, horizon)
        cheap_by_horizon[horizon] = {
            int(item.get("action_index")): item
            for item in (cheap_payload.get("evaluations") or [])
            if item.get("ok")
        }
    one_step_by_index: dict[int, dict[str, Any]] = {}
    if args.include_one_step_delta:
        one_step_payload = evaluate_indices(args, driver, eval_indices, 0, include_next_state=True)
        one_step_by_index = {
            int(item.get("action_index")): item
            for item in (one_step_payload.get("evaluations") or [])
            if item.get("ok")
        }
    eval_payload = driver.request(
        {
            "cmd": "evaluate_candidates",
            "action_indices": eval_indices,
            "continuation_policy": args.oracle_continuation_policy,
            "horizon_decisions": args.horizon_decisions,
            "gamma": args.gamma,
            "evaluation_mode": "independent",
            "parallelism": args.verified_parallelism,
            "exact_root_dedup": False,
            "include_state": False,
            "include_next_state": False,
            "include_continuation_trace": False,
            "check_live_env_unchanged": False,
        }
    ).get("payload") or {}
    by_index = {
        int(item.get("action_index")): item
        for item in (eval_payload.get("evaluations") or [])
        if item.get("ok")
    }
    rule_eval = by_index.get(rule_index)
    if not rule_eval:
        return rule_index, empty_group_summary()
    rule_return = float(rule_eval.get("discounted_return") or 0.0)
    scoped_evaluated = [idx for idx in scoped if idx in by_index]
    if not scoped_evaluated:
        return rule_index, empty_group_summary()
    best_index = max(scoped_evaluated, key=lambda idx: float(by_index[idx].get("discounted_return") or 0.0))
    best_return = float(by_index[best_index].get("discounted_return") or 0.0)
    best_adv = best_return - rule_return
    selected_index = best_index if best_index != rule_index and best_adv > args.oracle_margin else rule_index
    group_key = f"verified_proposer|seed:{seed}|step:{step}|decision:{observation.get('decision_type','unknown')}"
    split = stable_group_split(group_key)

    group_summary = empty_group_summary()
    group_summary["group_count"] = 1
    group_summary["candidate_evaluation_count"] = len(by_index)
    group_summary["feature_evaluation_count"] = sum(len(rows) for rows in cheap_by_horizon.values())
    group_summary["one_step_delta_evaluation_count"] = len(one_step_by_index)
    if selected_index != rule_index:
        group_summary["override_group_count"] = 1

    rule_candidate = candidates[rule_index] if rule_index < len(candidates) else {}
    start_summary = observation_summary(observation)
    rule_delta: dict[str, float] = {}
    rule_step = one_step_by_index.get(rule_index)
    if rule_step:
        rule_delta = subtract_summaries(
            observation_summary((rule_step.get("next_state") or {}).get("observation") or {}),
            start_summary,
        )
    for idx in eval_indices:
        evaluation = by_index.get(idx)
        if not evaluation or idx >= len(candidates):
            continue
        candidate_return = float(evaluation.get("discounted_return") or 0.0)
        adv = candidate_return - rule_return
        positive = idx != rule_index and adv > args.oracle_margin
        cheap_features = cheap_features_for_index(cheap_by_horizon, idx, rule_index)
        candidate_delta: dict[str, float] = {}
        delta_vs_rule: dict[str, float] = {}
        step_eval = one_step_by_index.get(idx)
        if step_eval:
            candidate_delta = subtract_summaries(
                observation_summary((step_eval.get("next_state") or {}).get("observation") or {}),
                start_summary,
            )
            delta_vs_rule = subtract_summaries(candidate_delta, rule_delta)
        row = {
            "schema_version": "verified_proposer_candidate_v0",
            "group_key": group_key,
            "split": split,
            "seed": seed,
            "step": step,
            "decision_kind": observation.get("decision_type"),
            "observation": observation,
            "candidate_count": len(eval_indices),
            "scoped_candidate_count": len(scoped),
            "candidate_index": idx,
            "candidate": candidates[idx],
            "rule_index": rule_index,
            "rule_candidate": rule_candidate,
            "is_rule_choice": idx == rule_index,
            "selected_action_index": selected_index,
            "is_full_verified_choice": idx == selected_index,
            "oracle_margin": args.oracle_margin,
            "q_candidate_mean": candidate_return,
            "q_rule_mean": rule_return,
            "adv_vs_rule_mean": adv,
            "oracle_chosen_label": "chosen" if idx == selected_index and selected_index != rule_index else "not_chosen",
            "safe_override_label": "positive" if positive else "negative",
            "passes_margin": positive,
            "cheap_return_features": cheap_features,
            "candidate_delta_vs_start": candidate_delta,
            "rule_delta_vs_start": rule_delta,
            "delta_vs_rule_features": delta_vs_rule,
        }
        handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")
        group_summary["row_count"] += 1
        if positive:
            group_summary["positive_count"] += 1
        else:
            group_summary["negative_count"] += 1
    return selected_index, group_summary


def empty_group_summary() -> dict[str, int]:
    return {
        "row_count": 0,
        "group_count": 0,
        "positive_count": 0,
        "negative_count": 0,
        "override_group_count": 0,
        "candidate_evaluation_count": 0,
        "feature_evaluation_count": 0,
        "one_step_delta_evaluation_count": 0,
    }


def evaluate_indices(
    args: argparse.Namespace,
    driver: FullRunDriver,
    indices: list[int],
    horizon: int,
    include_next_state: bool = False,
) -> dict[str, Any]:
    return driver.request(
        {
            "cmd": "evaluate_candidates",
            "action_indices": indices,
            "continuation_policy": args.oracle_continuation_policy,
            "horizon_decisions": horizon,
            "gamma": args.gamma,
            "evaluation_mode": "independent",
            "parallelism": args.verified_parallelism,
            "exact_root_dedup": False,
            "include_state": False,
            "include_next_state": include_next_state,
            "include_continuation_trace": False,
            "check_live_env_unchanged": False,
        }
    ).get("payload") or {}


def cheap_features_for_index(
    cheap_by_horizon: dict[int, dict[int, dict[str, Any]]],
    candidate_index: int,
    rule_index: int,
) -> dict[str, float]:
    out: dict[str, float] = {}
    for horizon, by_index in sorted(cheap_by_horizon.items()):
        candidate = by_index.get(candidate_index)
        rule = by_index.get(rule_index)
        if not candidate or not rule:
            continue
        ranked = sorted(
            (
                (idx, float(row.get("discounted_return") or 0.0))
                for idx, row in by_index.items()
                if idx != rule_index
            ),
            key=lambda item: (-item[1], item[0]),
        )
        rank_by_index = {idx: rank + 1 for rank, (idx, _value) in enumerate(ranked)}
        candidate_rank = rank_by_index.get(candidate_index)
        best_non_rule = ranked[0][1] if ranked else None
        candidate_return = float(candidate.get("discounted_return") or 0.0)
        rule_return = float(rule.get("discounted_return") or 0.0)
        out[f"h{horizon}_return"] = candidate_return
        out[f"h{horizon}_rule_return"] = rule_return
        out[f"h{horizon}_adv_vs_rule"] = candidate_return - rule_return
        out[f"h{horizon}_one_step_reward"] = float(candidate.get("one_step_reward") or 0.0)
        if candidate_rank is not None and ranked:
            out[f"h{horizon}_rank"] = float(candidate_rank)
            out[f"h{horizon}_rank_percentile"] = (
                (candidate_rank - 1) / max(len(ranked) - 1, 1)
            )
            out[f"h{horizon}_best_gap"] = float(best_non_rule) - candidate_return
    return out


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


def parse_int_list(value: str) -> list[int]:
    return [int(item) for item in value.split(",") if item.strip()]


def observation_summary(observation: dict[str, Any]) -> dict[str, float]:
    combat = observation.get("combat") or {}
    out: dict[str, float] = {}

    def add(name: str, value: Any) -> None:
        try:
            out[name] = float(value)
        except (TypeError, ValueError):
            pass

    for key in [
        "current_hp",
        "max_hp",
        "hp_ratio_milli",
        "deck_size",
        "relic_count",
    ]:
        add(key, observation.get(key))
    for key in [
        "player_hp",
        "player_block",
        "energy",
        "turn_count",
        "hand_count",
        "draw_count",
        "discard_count",
        "exhaust_count",
        "alive_monster_count",
        "dying_monster_count",
        "half_dead_monster_count",
        "zero_hp_monster_count",
        "pending_rebirth_monster_count",
        "total_monster_hp",
        "visible_incoming_damage",
        "pending_action_count",
        "queued_card_count",
        "limbo_count",
    ]:
        add(f"combat_{key}", combat.get(key))

    hand_cards = combat.get("hand_cards") or []
    add("hand_playable_count", sum(1 for card in hand_cards if card.get("playable")))
    add("hand_total_cost", sum(float(card.get("cost_for_turn") or 0) for card in hand_cards))
    semantic_counts: dict[str, int] = {}
    role_sums = {
        "keeper": 0.0,
        "fuel": 0.0,
        "exhaust": 0.0,
        "retention": 0.0,
        "copy": 0.0,
    }
    for card in hand_cards:
        for semantic in card.get("base_semantics") or []:
            semantic_counts[str(semantic)] = semantic_counts.get(str(semantic), 0) + 1
        scores = card.get("estimated_role_scores") or {}
        for key in role_sums:
            try:
                role_sums[key] += float(scores.get(key) or 0.0)
            except (TypeError, ValueError):
                pass
    for semantic, count in semantic_counts.items():
        add(f"hand_semantic_{semantic}_count", count)
    for key, value in role_sums.items():
        add(f"hand_role_{key}_sum", value)
    return out


def subtract_summaries(left: dict[str, float], right: dict[str, float]) -> dict[str, float]:
    out = {}
    for key in sorted(set(left) | set(right)):
        delta = left.get(key, 0.0) - right.get(key, 0.0)
        if delta:
            out[key] = delta
    return out


if __name__ == "__main__":
    main()
