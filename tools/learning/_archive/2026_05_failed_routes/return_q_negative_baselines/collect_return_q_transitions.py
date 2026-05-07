#!/usr/bin/env python3
"""Collect return-based Q transition rows from the full-run engine driver."""
from __future__ import annotations

import argparse
import json
import random
from pathlib import Path
from typing import Any

from return_q_common import FullRunDriver, legal_candidate_indices, model_scores_are_rank_only, predict_model, stable_group_split, write_json

SCHEMA_VERSION = "return_q_transition_v0"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--episodes", type=int, default=4)
    parser.add_argument("--seed-start", type=int, default=1)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=500)
    parser.add_argument("--max-decisions-per-episode", type=int, default=200)
    parser.add_argument("--horizon-decisions", type=int, default=8)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--continuation-policy", default="rule_baseline_v0")
    parser.add_argument("--behavior-policy", default="rule_baseline_v0")
    parser.add_argument("--model", type=Path)
    parser.add_argument("--candidate-scope", default="all", choices=["all", "controlled_v0", "controlled_v1"])
    parser.add_argument("--top-k", type=int, default=4)
    parser.add_argument("--fallback-count", type=int, default=1)
    parser.add_argument("--selective-horizon-decisions", type=int, default=4)
    parser.add_argument("--reward-shaping-profile", default="baseline")
    parser.add_argument("--max-candidates", type=int, default=0)
    parser.add_argument("--epsilon-random", type=float, default=0.0)
    parser.add_argument("--include-noncombat", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    args.out.parent.mkdir(parents=True, exist_ok=True)
    summary_path = args.summary_out or args.out.with_suffix(".summary.json")
    rng = random.Random(args.seed_start)
    model = load_model(args.model) if args.model else None
    if args.behavior_policy.startswith("learned_q") and model is None:
        raise SystemExit(f"--behavior-policy {args.behavior_policy} requires --model")
    row_count = 0
    group_count = 0
    episode_summaries = []

    driver = FullRunDriver(args.binary)
    try:
        with args.out.open("w", encoding="utf-8") as handle:
            for episode_index in range(args.episodes):
                seed = args.seed_start + episode_index * args.seed_step
                episode = collect_episode(args, driver, rng, seed, handle, model)
                row_count += int(episode["row_count"])
                group_count += int(episode["group_count"])
                episode_summaries.append(episode)
    finally:
        driver.close()

    summary = {
        "schema_version": "return_q_transition_collection_summary_v0",
        "out": str(args.out),
        "row_count": row_count,
        "group_count": group_count,
        "episodes": episode_summaries,
        "config": serializable_config(args, summary_path),
    }
    write_json(summary_path, summary)
    print(json.dumps(summary, indent=2, sort_keys=True))


def load_model(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def serializable_config(args: argparse.Namespace, summary_path: Path) -> dict[str, Any]:
    config = vars(args).copy()
    for key in ["out", "summary_out", "binary", "model"]:
        value = config.get(key)
        config[key] = str(value) if value else None
    config["summary_out"] = str(summary_path)
    return config


def collect_episode(
    args: argparse.Namespace,
    driver: FullRunDriver,
    rng: random.Random,
    seed: int,
    handle: Any,
    model: dict[str, Any] | None,
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
    total_reward = float(response.get("reward") or 0.0)
    row_count = 0
    group_count = 0
    combat_groups = 0
    decisions = 0
    done = bool(response.get("done"))
    last_info = response.get("info") or {}

    while not done and decisions < args.max_decisions_per_episode:
        payload = response.get("payload") or {}
        observation = payload.get("observation") or {}
        candidates = list(payload.get("action_candidates") or [])
        mask = list(payload.get("action_mask") or [])
        decision_type = str(observation.get("decision_type") or "")
        is_combat = decision_type.startswith("combat")

        if (is_combat or args.include_noncombat) and candidates:
            legal_indices = legal_candidate_indices(response, args.candidate_scope)
            if args.max_candidates > 0:
                legal_indices = legal_indices[: args.max_candidates]
            if legal_indices:
                group_key = f"seed:{seed}:step:{last_info.get('step', decisions)}"
                eval_payload = driver.request(
                    {
                        "cmd": "evaluate_candidates",
                        "action_indices": legal_indices,
                        "continuation_policy": args.continuation_policy,
                        "horizon_decisions": args.horizon_decisions,
                        "gamma": args.gamma,
                        "include_state": False,
                        "include_next_state": True,
                        "include_continuation_trace": False,
                        "check_live_env_unchanged": False,
                    }
                ).get("payload") or {}
                for evaluation in eval_payload.get("evaluations") or []:
                    if not evaluation.get("ok"):
                        continue
                    candidate = evaluation.get("candidate")
                    if not candidate:
                        continue
                    row = {
                        "schema_version": SCHEMA_VERSION,
                        "group_key": group_key,
                        "split": stable_group_split(group_key),
                        "seed": seed,
                        "step": last_info.get("step", decisions),
                        "decision_type": decision_type,
                        "observation": observation,
                        "candidate_set": candidates,
                        "candidate_index": evaluation.get("action_index"),
                        "candidate": candidate,
                        "one_step_reward": float(evaluation.get("one_step_reward") or 0.0),
                        "discounted_return": float(evaluation.get("discounted_return") or 0.0),
                        "next_state": evaluation.get("next_state"),
                        "done": bool(evaluation.get("done")),
                        "terminal_reason": evaluation.get("terminal_reason"),
                        "rollout_done": bool(evaluation.get("rollout_done")),
                        "rollout_terminal_reason": evaluation.get("rollout_terminal_reason"),
                        "behavior_policy": args.behavior_policy,
                        "continuation_policy": args.continuation_policy,
                        "horizon_decisions": args.horizon_decisions,
                        "gamma": args.gamma,
                    }
                    handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")
                    row_count += 1
                group_count += 1
                if is_combat:
                    combat_groups += 1

        if args.epsilon_random > 0.0 and rng.random() < args.epsilon_random:
            legal = legal_candidate_indices(response, args.candidate_scope)
            if legal:
                response = driver.request({"cmd": "step", "action_index": rng.choice(legal)})
            else:
                response = step_behavior(args, driver, rng, response, model)
        else:
            response = step_behavior(args, driver, rng, response, model)

        total_reward += float(response.get("reward") or 0.0)
        done = bool(response.get("done"))
        last_info = response.get("info") or last_info
        decisions += 1

    return {
        "seed": seed,
        "row_count": row_count,
        "group_count": group_count,
        "combat_group_count": combat_groups,
        "decisions": decisions,
        "done": done,
        "result": last_info.get("result"),
        "terminal_reason": last_info.get("terminal_reason"),
        "combat_win_count": last_info.get("combat_win_count"),
        "total_reward": total_reward,
    }


def step_behavior(
    args: argparse.Namespace,
    driver: FullRunDriver,
    rng: random.Random,
    response: dict[str, Any],
    model: dict[str, Any] | None,
) -> dict[str, Any]:
    if args.behavior_policy == "random":
        legal = scoped_legal_indices(response, args)
        if legal:
            return driver.request({"cmd": "step", "action_index": rng.choice(legal)})
        return driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
    if args.behavior_policy in {"rule_baseline_v0", "plan_query_v0"}:
        return driver.request({"cmd": "step_policy", "policy": args.behavior_policy})
    if args.behavior_policy == "learned_q_direct":
        if model is None:
            raise RuntimeError("learned_q_direct requires --model")
        if not is_combat_response(response):
            return driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
        return driver.request({"cmd": "step", "action_index": choose_direct(response, model, args)})
    if args.behavior_policy == "learned_q_selective_1ply":
        if model is None:
            raise RuntimeError("learned_q_selective_1ply requires --model")
        if not is_combat_response(response):
            return driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
        return driver.request({"cmd": "step", "action_index": choose_selective(args, driver, response, model)})
    return driver.request({"cmd": "step_policy", "policy": args.behavior_policy})


def is_combat_response(response: dict[str, Any]) -> bool:
    observation = ((response.get("payload") or {}).get("observation") or {})
    return str(observation.get("decision_type") or "").startswith("combat")


def scoped_legal_indices(response: dict[str, Any], args: argparse.Namespace) -> list[int]:
    return legal_candidate_indices(response, args.candidate_scope)


def choose_direct(response: dict[str, Any], model: dict[str, Any], args: argparse.Namespace) -> int:
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    candidates = payload.get("action_candidates") or []
    legal = scoped_legal_indices(response, args)
    if not legal:
        return 0
    return max(
        legal,
        key=lambda idx: predict_model(model, observation, candidates[idx]),
    )


def choose_selective(
    args: argparse.Namespace,
    driver: FullRunDriver,
    response: dict[str, Any],
    model: dict[str, Any],
) -> int:
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    candidates = payload.get("action_candidates") or []
    legal = scoped_legal_indices(response, args)
    if not legal:
        return 0
    ranked = sorted(
        legal,
        key=lambda idx: predict_model(model, observation, candidates[idx]),
        reverse=True,
    )
    selected = ranked[: max(args.top_k, 1)]
    if args.fallback_count > 0:
        selected.extend(ranked[-args.fallback_count :])
    selected = sorted(set(selected))
    eval_payload = driver.request(
        {
            "cmd": "evaluate_candidates",
            "action_indices": selected,
            "continuation_policy": args.continuation_policy,
            "horizon_decisions": args.selective_horizon_decisions,
            "gamma": args.gamma,
            "include_state": False,
            "include_next_state": True,
            "include_continuation_trace": False,
            "check_live_env_unchanged": False,
        }
    ).get("payload") or {}
    rank_only_model = model_scores_are_rank_only(model)
    best_index = selected[0]
    best_score = float("-inf")
    for evaluation in eval_payload.get("evaluations") or []:
        if not evaluation.get("ok"):
            continue
        action_index = int(evaluation.get("action_index"))
        if rank_only_model:
            score = float(evaluation.get("discounted_return") or 0.0)
        else:
            score = float(evaluation.get("one_step_reward") or 0.0)
            if not evaluation.get("done"):
                next_state = evaluation.get("next_state") or {}
                next_observation = next_state.get("observation") or {}
                next_candidates = next_state.get("action_candidates") or []
                next_legal = legal_candidate_indices({"payload": next_state}, args.candidate_scope)
                next_scores = [
                    predict_model(model, next_observation, next_candidates[idx])
                    for idx in next_legal
                ]
                if next_scores:
                    score += args.gamma * max(next_scores)
        if score > best_score:
            best_score = score
            best_index = action_index
    return best_index


if __name__ == "__main__":
    main()
