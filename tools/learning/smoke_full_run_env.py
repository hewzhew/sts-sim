#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import time
from pathlib import Path
from statistics import mean
from typing import Any

from full_run_env import FullRunGymEnv


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Smoke-test the full-run Gym bridge with random legal actions.")
    parser.add_argument("--episodes", type=int, default=10)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=5000)
    parser.add_argument("--driver-binary", type=Path)
    parser.add_argument("--reward-shaping-profile", choices=["baseline", "plan_deficit_v0"], default="baseline")
    parser.add_argument("--feature-profile", choices=["baseline", "plan_v0"], default="baseline")
    parser.add_argument("--details", action="store_true", help="Include per-episode detail rows in stdout JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    env = FullRunGymEnv(
        driver_binary=args.driver_binary,
        seed=args.seed,
        ascension=args.ascension,
        final_act=args.final_act,
        player_class=args.player_class,
        max_episode_steps=args.max_steps,
        reward_shaping_profile=args.reward_shaping_profile,
        feature_profile=args.feature_profile,
    )
    start = time.perf_counter()
    episode_summaries: list[dict[str, Any]] = []
    crash_count = 0
    illegal_action_count = 0
    no_progress_count = 0
    total_steps = 0

    try:
        for episode in range(args.episodes):
            run_seed = args.seed + episode
            _, info = env.reset(options={"run_seed": run_seed, "max_steps": args.max_steps})
            terminated = False
            truncated = False
            reward_total = 0.0
            steps = 0
            try:
                while not terminated and not truncated:
                    action = env.sample_random_legal_action()
                    _, reward, terminated, truncated, info = env.step(action)
                    reward_total += float(reward)
                    steps += 1
                    total_steps += 1
            except Exception as err:
                crash_count += 1
                info = {"crash": str(err), "result": "bridge_exception", "floor": info.get("floor", 0)}

            if info.get("invalid_action"):
                illegal_action_count += 1
            if info.get("terminal_reason") == "no_progress_loop":
                no_progress_count += 1
            episode_summaries.append(
                {
                    "episode": episode,
                    "seed": run_seed,
                    "result": info.get("result"),
                    "terminal_reason": info.get("terminal_reason"),
                    "floor": info.get("floor"),
                    "act": info.get("act"),
                    "steps": steps,
                    "reward_total": reward_total,
                    "combat_win_count": info.get("combat_win_count"),
                    "crash": info.get("crash"),
                }
            )
    finally:
        env.close()

    elapsed = max(time.perf_counter() - start, 1e-6)
    floors = [int(row.get("floor") or 0) for row in episode_summaries]
    result_counts: dict[str, int] = {}
    terminal_reason_counts: dict[str, int] = {}
    for row in episode_summaries:
        result_counts[str(row.get("result") or "unknown")] = result_counts.get(str(row.get("result") or "unknown"), 0) + 1
        terminal_reason = str(row.get("terminal_reason") or "unknown")
        terminal_reason_counts[terminal_reason] = terminal_reason_counts.get(terminal_reason, 0) + 1

    summary = {
        "episodes": args.episodes,
        "seed": args.seed,
        "max_steps": args.max_steps,
        "crash_count": crash_count,
        "illegal_action_count": illegal_action_count,
        "no_progress_count": no_progress_count,
        "average_floor": mean(floors) if floors else 0.0,
        "average_steps": mean([row["steps"] for row in episode_summaries]) if episode_summaries else 0.0,
        "steps_per_second": total_steps / elapsed,
        "episodes_per_hour": len(episode_summaries) / elapsed * 3600.0,
        "result_counts": result_counts,
        "terminal_reason_counts": terminal_reason_counts,
    }
    if args.details:
        summary["episodes_detail"] = episode_summaries
    print(json.dumps(summary, ensure_ascii=False, indent=2))
    return 0 if crash_count == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
