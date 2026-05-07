#!/usr/bin/env python3
"""Audit where a return-Q selector disagrees with rule_baseline_v0."""
from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from return_q_common import FullRunDriver, predict_model, write_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--episodes", type=int, default=20)
    parser.add_argument("--seed-start", type=int, default=95000)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=120)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--horizon-decisions", type=int, default=4)
    parser.add_argument("--max-records", type=int, default=400)
    parser.add_argument("--epsilon", type=float, default=0.05)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    model = json.loads(args.model.read_text(encoding="utf-8"))
    driver = FullRunDriver(args.binary)
    episodes = []
    records = []
    try:
        for episode_index in range(args.episodes):
            seed = args.seed_start + episode_index * args.seed_step
            episode, episode_records = audit_episode(args, driver, seed, model)
            episodes.append(episode)
            records.extend(episode_records)
    finally:
        driver.close()

    report = {
        "schema_version": "return_q_disagreement_audit_v0",
        "model": str(args.model),
        "config": {
            "episodes": args.episodes,
            "seed_start": args.seed_start,
            "seed_step": args.seed_step,
            "max_steps": args.max_steps,
            "horizon_decisions": args.horizon_decisions,
            "gamma": args.gamma,
            "epsilon": args.epsilon,
        },
        "summary": summarize(records, episodes, args.epsilon),
        "episodes": episodes,
        "records": records[: args.max_records],
    }
    write_json(args.out, report)
    print(json.dumps(report["summary"], indent=2, sort_keys=True))


def audit_episode(
    args: argparse.Namespace,
    driver: FullRunDriver,
    seed: int,
    model: dict[str, Any],
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    try:
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
    except Exception as err:
        return episode_crash(seed, 0, err), []

    records = []
    done = bool(response.get("done"))
    total_reward = float(response.get("reward") or 0.0)
    last_info = response.get("info") or {}
    steps = 0
    combat_decisions = 0
    disagreements = 0
    crash = None

    while not done and steps < args.max_steps:
        payload = response.get("payload") or {}
        observation = payload.get("observation") or {}
        if is_combat_payload(payload):
            combat_decisions += 1
            try:
                record = audit_current_state(args, driver, response, model)
            except Exception as err:
                crash = str(err)
                break
            if record is not None:
                records.append(record)
                disagreements += 1
            action_index = record["learned_action_index"] if record else choose_learned(response, model)[0]
            try:
                response = driver.request({"cmd": "step", "action_index": action_index})
            except Exception as err:
                crash = str(err)
                break
        else:
            try:
                response = driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
            except Exception as err:
                crash = str(err)
                break

        total_reward += float(response.get("reward") or 0.0)
        last_info = response.get("info") or last_info
        done = bool(response.get("done"))
        steps += 1

    episode = {
        "seed": seed,
        "steps": steps,
        "done": done,
        "crash": crash,
        "result": "crash" if crash else last_info.get("result"),
        "terminal_reason": "script_error" if crash else last_info.get("terminal_reason"),
        "combat_win_count": int(last_info.get("combat_win_count") or 0),
        "total_reward": total_reward,
        "combat_decisions": combat_decisions,
        "disagreements": disagreements,
    }
    return episode, records


def episode_crash(seed: int, steps: int, err: Exception) -> dict[str, Any]:
    return {
        "seed": seed,
        "steps": steps,
        "done": True,
        "crash": str(err),
        "result": "crash",
        "terminal_reason": "reset_error",
        "combat_win_count": 0,
        "total_reward": 0.0,
        "combat_decisions": 0,
        "disagreements": 0,
    }


def audit_current_state(
    args: argparse.Namespace,
    driver: FullRunDriver,
    response: dict[str, Any],
    model: dict[str, Any],
) -> dict[str, Any] | None:
    payload = response.get("payload") or {}
    candidates = payload.get("action_candidates") or []
    observation = payload.get("observation") or {}
    learned_index, learned_score = choose_learned(response, model)
    preview = driver.request(
        {
            "cmd": "preview_policy_action",
            "policy": "rule_baseline_v0",
            "include_state": False,
            "include_next_state": False,
            "check_live_env_unchanged": False,
        }
    ).get("payload") or {}
    rule_index = preview.get("chosen_action_index")
    if rule_index is None or int(rule_index) == int(learned_index):
        return None
    rule_index = int(rule_index)

    rule_score = predict_model(model, observation, candidates[rule_index])
    eval_payload = driver.request(
        {
            "cmd": "evaluate_candidates",
            "action_indices": sorted({rule_index, learned_index}),
            "continuation_policy": "rule_baseline_v0",
            "horizon_decisions": args.horizon_decisions,
            "gamma": args.gamma,
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
    learned_return = float(by_index.get(learned_index, {}).get("discounted_return") or 0.0)
    rule_return = float(by_index.get(rule_index, {}).get("discounted_return") or 0.0)
    delta = learned_return - rule_return
    return {
        "seed": (response.get("info") or {}).get("seed"),
        "step": (response.get("info") or {}).get("step"),
        "floor": observation.get("floor"),
        "current_hp": observation.get("current_hp"),
        "decision_type": observation.get("decision_type"),
        "incoming": (observation.get("combat") or {}).get("visible_incoming_damage"),
        "monster_hp": (observation.get("combat") or {}).get("total_monster_hp"),
        "energy": (observation.get("combat") or {}).get("energy"),
        "learned_action_index": learned_index,
        "rule_action_index": rule_index,
        "learned_action_key": action_key(candidates[learned_index]),
        "rule_action_key": action_key(candidates[rule_index]),
        "learned_action_family": action_family(candidates[learned_index]),
        "rule_action_family": action_family(candidates[rule_index]),
        "learned_q_score": learned_score,
        "rule_q_score": rule_score,
        "learned_short_return": learned_return,
        "rule_short_return": rule_return,
        "short_return_delta": delta,
        "short_return_bucket": bucket_delta(delta, args.epsilon),
    }


def choose_learned(response: dict[str, Any], model: dict[str, Any]) -> tuple[int, float]:
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    candidates = payload.get("action_candidates") or []
    legal = [
        idx
        for idx, legal in enumerate(payload.get("action_mask") or [])
        if legal and idx < len(candidates)
    ]
    if not legal:
        return 0, 0.0
    scored = [
        (idx, predict_model(model, observation, candidates[idx]))
        for idx in legal
    ]
    return max(scored, key=lambda item: item[1])


def is_combat_payload(payload: dict[str, Any]) -> bool:
    observation = payload.get("observation") or {}
    return str(observation.get("decision_type") or "").startswith("combat")


def action_key(candidate: dict[str, Any]) -> str:
    return str(candidate.get("action_key") or "")


def action_family(candidate: dict[str, Any]) -> str:
    key = action_key(candidate)
    if key.startswith("combat/end_turn"):
        return "end_turn"
    if key.startswith("combat/play_card"):
        card = segment(key, "card")
        return f"play:{card or 'unknown'}"
    if key.startswith("combat/use_potion"):
        return "use_potion"
    return key.split("/", 1)[0] if key else "unknown"


def segment(key: str, name: str) -> str:
    marker = f"{name}:"
    for part in key.split("/"):
        if part.startswith(marker):
            return part[len(marker) :]
    return ""


def bucket_delta(delta: float, epsilon: float) -> str:
    if delta < -epsilon:
        return "learned_worse"
    if delta > epsilon:
        return "learned_better"
    return "similar"


def summarize(
    records: list[dict[str, Any]],
    episodes: list[dict[str, Any]],
    epsilon: float,
) -> dict[str, Any]:
    buckets = Counter(record["short_return_bucket"] for record in records)
    learned_worse = [record for record in records if record["short_return_delta"] < -epsilon]
    learned_better = [record for record in records if record["short_return_delta"] > epsilon]
    by_pair = Counter(
        (record["learned_action_family"], record["rule_action_family"])
        for record in learned_worse
    )
    by_learned = Counter(record["learned_action_family"] for record in learned_worse)
    by_rule = Counter(record["rule_action_family"] for record in learned_worse)
    return {
        "episodes": len(episodes),
        "episode_crashes": sum(1 for episode in episodes if episode.get("crash")),
        "average_total_reward": mean(float(episode.get("total_reward") or 0.0) for episode in episodes),
        "average_combat_win_count": mean(float(episode.get("combat_win_count") or 0.0) for episode in episodes),
        "combat_decisions": sum(int(episode.get("combat_decisions") or 0) for episode in episodes),
        "disagreement_count": len(records),
        "short_return_buckets": dict(buckets),
        "average_short_return_delta": mean(float(record["short_return_delta"]) for record in records),
        "learned_worse_count": len(learned_worse),
        "learned_better_count": len(learned_better),
        "top_worse_action_pairs": [
            {"learned": pair[0], "rule": pair[1], "count": count}
            for pair, count in by_pair.most_common(12)
        ],
        "top_worse_learned_actions": [
            {"action": action, "count": count}
            for action, count in by_learned.most_common(12)
        ],
        "top_worse_rule_actions": [
            {"action": action, "count": count}
            for action, count in by_rule.most_common(12)
        ],
        "worst_examples": sorted(
            records,
            key=lambda record: record["short_return_delta"],
        )[:12],
    }


def mean(values: Any) -> float:
    values = list(values)
    return sum(values) / len(values) if values else 0.0


if __name__ == "__main__":
    main()
