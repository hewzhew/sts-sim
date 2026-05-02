#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import time
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from statistics import mean
from typing import Any

import numpy as np
from sb3_contrib import MaskablePPO
from stable_baselines3.common.vec_env import DummyVecEnv, VecMonitor

from combat_rl_common import REPO_ROOT, write_json
from full_run_env import FullRunGymEnv


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Tiny MaskablePPO sanity check for the offline full-run Gym bridge."
    )
    parser.add_argument("--timesteps", type=int, default=2048)
    parser.add_argument("--n-envs", type=int, default=2)
    parser.add_argument("--eval-episodes", type=int, default=50)
    parser.add_argument("--seed", type=int, default=30000)
    parser.add_argument("--eval-seed", type=int, default=40000)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=5000)
    parser.add_argument("--driver-binary", type=Path)
    parser.add_argument("--model-out", type=Path)
    parser.add_argument("--metrics-out", type=Path)
    parser.add_argument("--n-steps", type=int, default=128)
    parser.add_argument("--batch-size", type=int, default=128)
    parser.add_argument("--learning-rate", type=float, default=3e-4)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--ent-coef", type=float, default=0.01)
    return parser.parse_args()


def make_env(args: argparse.Namespace, env_seed: int):
    def _factory() -> FullRunGymEnv:
        return FullRunGymEnv(
            driver_binary=args.driver_binary,
            seed=env_seed,
            ascension=args.ascension,
            final_act=args.final_act,
            player_class=args.player_class,
            max_episode_steps=args.max_steps,
        )

    return _factory


def evaluate_random(args: argparse.Namespace, base_seed: int, episodes: int) -> dict[str, Any]:
    env = FullRunGymEnv(
        driver_binary=args.driver_binary,
        seed=base_seed,
        ascension=args.ascension,
        final_act=args.final_act,
        player_class=args.player_class,
        max_episode_steps=args.max_steps,
    )
    rows: list[dict[str, Any]] = []
    start = time.perf_counter()
    try:
        for episode in range(episodes):
            _, info = env.reset(options={"run_seed": base_seed + episode, "max_steps": args.max_steps})
            done = False
            truncated = False
            reward_total = 0.0
            steps = 0
            invalid_actions = 0
            while not done and not truncated:
                action = env.sample_random_legal_action()
                _, reward, done, truncated, info = env.step(action)
                reward_total += float(reward)
                steps += 1
                invalid_actions += 1 if info.get("invalid_action") else 0
            rows.append(episode_row(episode, base_seed + episode, info, steps, reward_total, invalid_actions))
    finally:
        env.close()
    return summarize_rows(rows, time.perf_counter() - start)


def evaluate_model(
    model: MaskablePPO,
    args: argparse.Namespace,
    base_seed: int,
    episodes: int,
    deterministic: bool,
) -> dict[str, Any]:
    env = FullRunGymEnv(
        driver_binary=args.driver_binary,
        seed=base_seed,
        ascension=args.ascension,
        final_act=args.final_act,
        player_class=args.player_class,
        max_episode_steps=args.max_steps,
    )
    rows: list[dict[str, Any]] = []
    start = time.perf_counter()
    try:
        for episode in range(episodes):
            obs, info = env.reset(options={"run_seed": base_seed + episode, "max_steps": args.max_steps})
            done = False
            truncated = False
            reward_total = 0.0
            steps = 0
            invalid_actions = 0
            while not done and not truncated:
                action_masks = env.action_masks()
                action, _ = model.predict(obs, deterministic=deterministic, action_masks=action_masks)
                obs, reward, done, truncated, info = env.step(int(action))
                reward_total += float(reward)
                steps += 1
                invalid_actions += 1 if info.get("invalid_action") else 0
            rows.append(episode_row(episode, base_seed + episode, info, steps, reward_total, invalid_actions))
    finally:
        env.close()
    return summarize_rows(rows, time.perf_counter() - start)


def episode_row(
    episode: int,
    seed: int,
    info: dict[str, Any],
    steps: int,
    reward_total: float,
    invalid_actions: int,
) -> dict[str, Any]:
    return {
        "episode": int(episode),
        "seed": int(seed),
        "result": info.get("result"),
        "terminal_reason": info.get("terminal_reason"),
        "floor": int(info.get("floor") or 0),
        "act": int(info.get("act") or 0),
        "steps": int(steps),
        "reward_total": float(reward_total),
        "combat_win_count": int(info.get("combat_win_count") or 0),
        "invalid_actions": int(invalid_actions),
        "crash": info.get("crash"),
    }


def summarize_rows(rows: list[dict[str, Any]], elapsed: float) -> dict[str, Any]:
    elapsed = max(float(elapsed), 1e-6)
    floors = [int(row.get("floor") or 0) for row in rows]
    steps = [int(row.get("steps") or 0) for row in rows]
    rewards = [float(row.get("reward_total") or 0.0) for row in rows]
    crashes = sum(1 for row in rows if row.get("crash"))
    invalid_actions = sum(int(row.get("invalid_actions") or 0) for row in rows)
    no_progress = sum(1 for row in rows if row.get("terminal_reason") == "no_progress_loop")
    return {
        "episodes": len(rows),
        "crash_count": crashes,
        "illegal_action_count": invalid_actions,
        "no_progress_count": no_progress,
        "average_floor": mean(floors) if floors else 0.0,
        "median_floor": float(np.median(np.asarray(floors, dtype=np.float32))) if floors else 0.0,
        "average_steps": mean(steps) if steps else 0.0,
        "average_reward": mean(rewards) if rewards else 0.0,
        "steps_per_second": sum(steps) / elapsed,
        "result_counts": dict(Counter(str(row.get("result") or "unknown") for row in rows)),
        "terminal_reason_counts": dict(
            Counter(str(row.get("terminal_reason") or "unknown") for row in rows)
        ),
    }


def main() -> int:
    args = parse_args()
    artifact_dir = REPO_ROOT / "tools" / "artifacts" / "full_run_rl"
    artifact_dir.mkdir(parents=True, exist_ok=True)
    model_out = args.model_out or artifact_dir / "full_run_maskable_ppo_sanity.zip"
    metrics_out = args.metrics_out or artifact_dir / "full_run_maskable_ppo_sanity.json"

    random_eval = evaluate_random(args, args.eval_seed, args.eval_episodes)

    vec_env = VecMonitor(
        DummyVecEnv([make_env(args, args.seed + idx) for idx in range(max(int(args.n_envs), 1))])
    )
    train_start = time.perf_counter()
    model = MaskablePPO(
        "MlpPolicy",
        vec_env,
        verbose=0,
        seed=args.seed,
        n_steps=args.n_steps,
        batch_size=args.batch_size,
        learning_rate=args.learning_rate,
        gamma=args.gamma,
        ent_coef=args.ent_coef,
        policy_kwargs={"net_arch": [64, 64]},
    )
    model.learn(total_timesteps=args.timesteps, progress_bar=False)
    train_seconds = time.perf_counter() - train_start
    model.save(str(model_out))
    vec_env.close()

    trained_eval = evaluate_model(
        model,
        args,
        base_seed=args.eval_seed,
        episodes=args.eval_episodes,
        deterministic=True,
    )

    report = {
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "purpose": "tiny full-run MaskablePPO sanity check",
        "contract": {
            "driver": "full_run_env_driver",
            "python_env": "FullRunGymEnv",
            "uses_action_masks": True,
            "not_a_strength_claim": True,
        },
        "config": {
            "timesteps": args.timesteps,
            "n_envs": args.n_envs,
            "eval_episodes": args.eval_episodes,
            "seed": args.seed,
            "eval_seed": args.eval_seed,
            "ascension": args.ascension,
            "player_class": args.player_class,
            "final_act": args.final_act,
            "max_steps": args.max_steps,
            "n_steps": args.n_steps,
            "batch_size": args.batch_size,
            "learning_rate": args.learning_rate,
            "gamma": args.gamma,
            "ent_coef": args.ent_coef,
        },
        "train": {
            "seconds": train_seconds,
            "model_out": str(model_out),
        },
        "random_eval": random_eval,
        "trained_eval": trained_eval,
        "delta": {
            "average_floor": float(trained_eval["average_floor"] - random_eval["average_floor"]),
            "average_reward": float(trained_eval["average_reward"] - random_eval["average_reward"]),
        },
    }
    write_json(metrics_out, report)
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0 if trained_eval["crash_count"] == 0 and trained_eval["illegal_action_count"] == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
