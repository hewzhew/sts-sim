#!/usr/bin/env python3
"""Collect decision-state advantage-over-rule rows from the full-run driver."""
from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any

from return_q_common import (
    FullRunDriver,
    legal_candidate_indices,
    stable_group_split,
    write_json,
)

SCHEMA_VERSION = "return_advantage_vs_rule_v1"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--episodes", type=int, default=8)
    parser.add_argument("--seed-start", type=int, default=1)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=500)
    parser.add_argument("--max-decisions-per-episode", type=int, default=300)
    parser.add_argument("--horizon-decisions", type=int, default=4)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--continuation-policy", default="rule_baseline_v0")
    parser.add_argument("--trajectory-policy", default="rule_baseline_v0")
    parser.add_argument("--candidate-scope", default="controlled_v1", choices=["all", "controlled_v0", "controlled_v1"])
    parser.add_argument("--margin", type=float, default=0.5)
    parser.add_argument("--z", type=float, default=1.0)
    parser.add_argument("--reward-shaping-profile", default="baseline")
    parser.add_argument("--include-full-state", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    summary_path = args.summary_out or args.out.with_suffix(".summary.json")
    row_count = 0
    group_count = 0
    episode_summaries = []
    label_counts: Counter[str] = Counter()
    decision_type_counts: Counter[str] = Counter()
    driver = FullRunDriver(args.binary)
    try:
        with args.out.open("w", encoding="utf-8") as handle:
            for episode_index in range(args.episodes):
                seed = args.seed_start + episode_index * args.seed_step
                episode = collect_episode(args, driver, seed, handle)
                row_count += int(episode["row_count"])
                group_count += int(episode["group_count"])
                label_counts.update(episode["label_counts"])
                decision_type_counts.update(episode["decision_type_counts"])
                episode_summaries.append(episode)
    finally:
        driver.close()

    summary = {
        "schema_version": "return_advantage_vs_rule_collection_summary_v1",
        "out": str(args.out),
        "row_count": row_count,
        "group_count": group_count,
        "label_counts": dict(sorted(label_counts.items())),
        "decision_type_counts": dict(sorted(decision_type_counts.items())),
        "episodes": episode_summaries,
        "config": serializable_config(args, summary_path),
    }
    write_json(summary_path, summary)
    print(json.dumps(summary, indent=2, sort_keys=True))


def serializable_config(args: argparse.Namespace, summary_path: Path) -> dict[str, Any]:
    config = vars(args).copy()
    for key in ["out", "summary_out", "binary"]:
        value = config.get(key)
        config[key] = str(value) if value else None
    config["summary_out"] = str(summary_path)
    return config


def collect_episode(
    args: argparse.Namespace,
    driver: FullRunDriver,
    seed: int,
    handle: Any,
) -> dict[str, Any]:
    response = driver.request(
        {
            "cmd": "reset",
            "seed": seed,
            "ascension": args.ascension,
            "final_act": args.final_act,
            "class": args.player_class,
            "max_steps": args.max_steps,
            "reward_shaping_profile": args.reward_shaping_profile,
        }
    )
    row_count = 0
    group_count = 0
    decisions = 0
    total_reward = float(response.get("reward") or 0.0)
    done = bool(response.get("done"))
    last_info = response.get("info") or {}
    label_counts: Counter[str] = Counter()
    decision_type_counts: Counter[str] = Counter()

    while not done and decisions < args.max_decisions_per_episode:
        payload = response.get("payload") or {}
        observation = payload.get("observation") or {}
        decision_type = str(observation.get("decision_type") or "")
        if decision_type.startswith("combat"):
            wrote, labels = collect_decision_group(args, driver, seed, last_info, response, handle)
            if wrote:
                group_count += 1
                row_count += wrote
                label_counts.update(labels)
                decision_type_counts[decision_type] += 1

        response = step_trajectory(args, driver)
        total_reward += float(response.get("reward") or 0.0)
        done = bool(response.get("done"))
        last_info = response.get("info") or last_info
        decisions += 1

    return {
        "seed": seed,
        "row_count": row_count,
        "group_count": group_count,
        "decisions": decisions,
        "done": done,
        "result": last_info.get("result"),
        "terminal_reason": last_info.get("terminal_reason"),
        "combat_win_count": last_info.get("combat_win_count"),
        "total_reward": total_reward,
        "label_counts": dict(sorted(label_counts.items())),
        "decision_type_counts": dict(sorted(decision_type_counts.items())),
    }


def collect_decision_group(
    args: argparse.Namespace,
    driver: FullRunDriver,
    seed: int,
    info: dict[str, Any],
    response: dict[str, Any],
    handle: Any,
) -> tuple[int, Counter[str]]:
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    candidates = list(payload.get("action_candidates") or [])
    scoped = legal_candidate_indices(response, args.candidate_scope)
    if not scoped:
        return 0, Counter()

    rule_index = preview_rule_index(driver)
    if rule_index is None:
        return 0, Counter()
    all_legal = set(legal_candidate_indices(response, "all"))
    if rule_index not in all_legal:
        return 0, Counter()
    if rule_index not in scoped:
        return 0, Counter({"skipped_rule_out_of_scope": 1})

    eval_indices = sorted(set(scoped + [rule_index]))
    eval_by_index = evaluate_action_indices(args, driver, eval_indices)
    rule_eval = eval_by_index.get(rule_index)
    if not rule_eval:
        return 0, Counter({"skipped_missing_rule_eval": 1})
    q_rule = float(rule_eval.get("discounted_return") or 0.0)
    rule_next_state = rule_eval.get("next_state") or {}
    start_summary = observation_summary(observation)
    rule_next_observation = (rule_next_state.get("observation") or {})
    rule_delta = subtract_summaries(observation_summary(rule_next_observation), start_summary)

    group_key = (
        f"seed:{seed}:step:{info.get('step', 0)}:"
        f"decision:{observation.get('decision_type', 'unknown')}"
    )
    labels: Counter[str] = Counter()
    rows_written = 0
    for action_index in scoped:
        evaluation = eval_by_index.get(action_index)
        if not evaluation or not evaluation.get("ok"):
            continue
        candidate = evaluation.get("candidate") or {}
        candidate_next_state = evaluation.get("next_state") or {}
        candidate_next_observation = candidate_next_state.get("observation") or {}
        candidate_delta = subtract_summaries(
            observation_summary(candidate_next_observation),
            start_summary,
        )
        delta_vs_rule = subtract_summaries(candidate_delta, rule_delta)
        q_candidate = float(evaluation.get("discounted_return") or 0.0)
        adv_mean = q_candidate - q_rule
        adv_stderr = 0.0
        label = safe_override_label(
            action_index=action_index,
            rule_index=rule_index,
            adv_mean=adv_mean,
            adv_stderr=adv_stderr,
            margin=args.margin,
            z=args.z,
        )
        labels[label] += 1
        row = {
            "schema_version": SCHEMA_VERSION,
            "group_key": group_key,
            "split": stable_group_split(group_key),
            "seed": seed,
            "step": info.get("step", 0),
            "decision_kind": observation.get("decision_type", "unknown"),
            "engine_state": observation.get("engine_state", "unknown"),
            "observation": observation,
            "candidate_index": action_index,
            "candidate": candidate,
            "rule_index": rule_index,
            "rule_candidate": candidates[rule_index] if rule_index < len(candidates) else None,
            "candidate_count": len(candidates),
            "candidate_delta_vs_start": candidate_delta,
            "rule_delta_vs_start": rule_delta,
            "delta_vs_rule_features": delta_vs_rule,
            "q_candidate_mean": q_candidate,
            "q_candidate_stderr": 0.0,
            "q_rule_mean": q_rule,
            "q_rule_stderr": 0.0,
            "adv_vs_rule_mean": adv_mean,
            "adv_vs_rule_stderr": adv_stderr,
            "safe_override_margin": args.margin,
            "safe_override_z": args.z,
            "safe_override_label": label,
            "is_rule_choice": action_index == rule_index,
            "continuation_policy": args.continuation_policy,
            "trajectory_policy": args.trajectory_policy,
            "horizon_decisions": args.horizon_decisions,
            "gamma": args.gamma,
        }
        if args.include_full_state:
            row["candidate_set"] = candidates
            row["candidate_next_state"] = candidate_next_state
            row["rule_next_state"] = rule_next_state
        handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")
        rows_written += 1
    return rows_written, labels


def safe_override_label(
    *,
    action_index: int,
    rule_index: int,
    adv_mean: float,
    adv_stderr: float,
    margin: float,
    z: float,
) -> str:
    if action_index == rule_index:
        return "negative"
    adv_lcb = adv_mean - z * adv_stderr
    adv_ucb = adv_mean + z * adv_stderr
    if adv_lcb > margin:
        return "positive"
    if adv_ucb < 0.0:
        return "negative"
    return "gray"


def preview_rule_index(driver: FullRunDriver) -> int | None:
    try:
        payload = driver.request(
            {
                "cmd": "preview_policy_action",
                "policy": "rule_baseline_v0",
                "include_state": False,
                "include_next_state": False,
                "check_live_env_unchanged": False,
            }
        ).get("payload") or {}
    except Exception:
        return None
    value = payload.get("chosen_action_index")
    return int(value) if value is not None else None


def evaluate_action_indices(
    args: argparse.Namespace,
    driver: FullRunDriver,
    action_indices: list[int],
) -> dict[int, dict[str, Any]]:
    payload = driver.request(
        {
            "cmd": "evaluate_candidates",
            "action_indices": action_indices,
            "continuation_policy": args.continuation_policy,
            "horizon_decisions": args.horizon_decisions,
            "gamma": args.gamma,
            "include_state": False,
            "include_next_state": True,
            "include_continuation_trace": False,
            "check_live_env_unchanged": False,
        }
    ).get("payload") or {}
    return {
        int(item.get("action_index")): item
        for item in (payload.get("evaluations") or [])
        if item.get("ok")
    }


def step_trajectory(args: argparse.Namespace, driver: FullRunDriver) -> dict[str, Any]:
    if args.trajectory_policy not in {"rule_baseline_v0", "plan_query_v0"}:
        raise RuntimeError(
            f"trajectory policy {args.trajectory_policy} is not supported by this collector"
        )
    return driver.request({"cmd": "step_policy", "policy": args.trajectory_policy})


def observation_summary(observation: dict[str, Any]) -> dict[str, float]:
    combat = observation.get("combat") or {}
    screen = observation.get("screen") or {}
    out: dict[str, float] = {}

    def add(name: str, value: Any) -> None:
        try:
            out[name] = float(value)
        except (TypeError, ValueError):
            pass

    for key in [
        "act",
        "floor",
        "current_hp",
        "max_hp",
        "hp_ratio_milli",
        "gold",
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
        "total_monster_hp",
        "visible_incoming_damage",
        "pending_action_count",
        "queued_card_count",
        "limbo_count",
    ]:
        add(f"combat_{key}", combat.get(key))
    add("screen_selection_target_count", screen.get("selection_target_count"))
    return out


def subtract_summaries(left: dict[str, float], right: dict[str, float]) -> dict[str, float]:
    keys = set(left) | set(right)
    return {
        key: left.get(key, 0.0) - right.get(key, 0.0)
        for key in sorted(keys)
        if left.get(key, 0.0) - right.get(key, 0.0) != 0.0
    }


if __name__ == "__main__":
    main()
