#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import time
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from statistics import mean
from typing import Any

import numpy as np
from sb3_contrib import MaskablePPO
from stable_baselines3.common.vec_env import DummyVecEnv, VecMonitor

from combat_rl_common import REPO_ROOT, find_release_binary, write_json
from full_run_env import FullRunGymEnv


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Benchmark random/rule/PPO sanity matrix for the full-run Gym bridge."
    )
    parser.add_argument("--timesteps", type=int, default=2048)
    parser.add_argument("--n-envs", type=int, default=2)
    parser.add_argument("--train-seeds", default="30000,31000,32000")
    parser.add_argument("--eval-seeds", default="40000,50000,60000")
    parser.add_argument("--eval-episodes", type=int, default=50)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=5000)
    parser.add_argument("--driver-binary", type=Path)
    parser.add_argument("--sts-dev-tool-binary", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--artifact-dir", type=Path)
    parser.add_argument("--n-steps", type=int, default=128)
    parser.add_argument("--batch-size", type=int, default=128)
    parser.add_argument("--learning-rate", type=float, default=3e-4)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--ent-coef", type=float, default=0.01)
    parser.add_argument("--skip-rule-baseline", action="store_true")
    return parser.parse_args()


def parse_int_list(text: str) -> list[int]:
    values = [int(part.strip()) for part in str(text or "").split(",") if part.strip()]
    if not values:
        raise SystemExit("expected at least one integer seed")
    return values


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


def train_ppo(args: argparse.Namespace, train_seed: int, model_out: Path) -> tuple[MaskablePPO, dict[str, Any]]:
    vec_env = VecMonitor(
        DummyVecEnv([make_env(args, train_seed + idx) for idx in range(max(int(args.n_envs), 1))])
    )
    start = time.perf_counter()
    model = MaskablePPO(
        "MlpPolicy",
        vec_env,
        verbose=0,
        seed=train_seed,
        n_steps=args.n_steps,
        batch_size=args.batch_size,
        learning_rate=args.learning_rate,
        gamma=args.gamma,
        ent_coef=args.ent_coef,
        policy_kwargs={"net_arch": [64, 64]},
    )
    model.learn(total_timesteps=args.timesteps, progress_bar=False)
    seconds = time.perf_counter() - start
    model.save(str(model_out))
    vec_env.close()
    return model, {"seconds": seconds, "model_out": str(model_out)}


def evaluate_python_policy(
    args: argparse.Namespace,
    *,
    policy_name: str,
    base_seed: int,
    episodes: int,
    model: MaskablePPO | None = None,
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
    decision_type_counts: Counter[str] = Counter()
    action_type_counts: Counter[str] = Counter()
    action_key_prefix_counts: Counter[str] = Counter()
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
                if model is None:
                    action = env.sample_random_legal_action()
                else:
                    action_masks = env.action_masks()
                    action, _ = model.predict(obs, deterministic=True, action_masks=action_masks)
                    action = int(action)
                candidate = current_candidate(info, int(action))
                decision_type_counts[str(info.get("decision_type") or "unknown")] += 1
                action_type_counts[action_type(candidate)] += 1
                action_key_prefix_counts[action_key_prefix(candidate)] += 1
                obs, reward, done, truncated, info = env.step(int(action))
                reward_total += float(reward)
                steps += 1
                invalid_actions += 1 if info.get("invalid_action") else 0
            rows.append(episode_row(episode, base_seed + episode, info, steps, reward_total, invalid_actions))
    finally:
        env.close()
    summary = summarize_rows(rows, time.perf_counter() - start)
    summary.update(
        {
            "policy": policy_name,
            "eval_seed": base_seed,
            "decision_type_counts": dict(decision_type_counts),
            "action_type_counts": dict(action_type_counts),
            "action_key_prefix_counts": dict(action_key_prefix_counts),
            "anomaly_flags": anomaly_flags(summary, action_type_counts, decision_type_counts, args.max_steps),
        }
    )
    return summary


def current_candidate(info: dict[str, Any], action_index: int) -> dict[str, Any]:
    candidates = list(info.get("action_candidates") or [])
    if 0 <= action_index < len(candidates):
        candidate = candidates[action_index]
        if isinstance(candidate, dict):
            return candidate
    return {}


def action_type(candidate: dict[str, Any]) -> str:
    action = candidate.get("action") or {}
    if isinstance(action, dict):
        return str(action.get("type") or "unknown")
    return "unknown"


def action_key_prefix(candidate: dict[str, Any]) -> str:
    key = str(candidate.get("action_key") or "unknown")
    parts = key.split("/")
    if len(parts) >= 2:
        return "/".join(parts[:2])
    return key


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
        "failure_examples": [
            row
            for row in rows
            if row.get("crash")
            or int(row.get("invalid_actions") or 0) > 0
            or row.get("terminal_reason") == "no_progress_loop"
        ][:5],
    }


def anomaly_flags(
    summary: dict[str, Any],
    action_type_counts: Counter[str],
    decision_type_counts: Counter[str],
    max_steps: int,
) -> list[str]:
    flags: list[str] = []
    if int(summary.get("crash_count") or 0) > 0:
        flags.append("crash")
    if int(summary.get("illegal_action_count") or 0) > 0:
        flags.append("illegal_action")
    if int(summary.get("no_progress_count") or 0) > 0:
        flags.append("no_progress")
    if int((summary.get("terminal_reason_counts") or {}).get("step_cap", 0)) > 0:
        flags.append("step_cap")
    if float(summary.get("average_steps") or 0.0) > 0.9 * float(max_steps):
        flags.append("near_step_cap_average_steps")
    action_total = sum(action_type_counts.values())
    if action_total > 0:
        top_action_share = max(action_type_counts.values()) / action_total
        if top_action_share >= 0.85:
            flags.append(f"action_type_collapse:{action_type_counts.most_common(1)[0][0]}")
    decision_total = sum(decision_type_counts.values())
    if decision_total > 0:
        top_decision_share = max(decision_type_counts.values()) / decision_total
        if top_decision_share >= 0.95:
            flags.append(f"decision_type_collapse:{decision_type_counts.most_common(1)[0][0]}")
    return flags


def run_rust_policy(
    args: argparse.Namespace,
    *,
    policy: str,
    base_seed: int,
    episodes: int,
    out_dir: Path,
) -> dict[str, Any]:
    binary = find_release_binary(args.sts_dev_tool_binary, "sts_dev_tool")
    summary_path = out_dir / f"{policy}_seed_{base_seed}_episodes_{episodes}.json"
    cmd = [
        str(binary),
        "run-batch",
        "--episodes",
        str(episodes),
        "--seed",
        str(base_seed),
        "--policy",
        policy,
        "--ascension",
        str(args.ascension),
        "--class",
        args.player_class,
        "--max-steps",
        str(args.max_steps),
        "--determinism-check",
        "--summary-out",
        str(summary_path),
    ]
    if args.final_act:
        cmd.append("--final-act")
    start = time.perf_counter()
    proc = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    elapsed = time.perf_counter() - start
    if proc.returncode != 0:
        raise RuntimeError(f"{policy} run-batch failed with code {proc.returncode}: {proc.stderr}")
    data = json.loads(summary_path.read_text(encoding="utf-8"))
    summary = {
        "policy": policy,
        "eval_seed": base_seed,
        "summary_path": str(summary_path),
        "episodes": int(data.get("episodes_requested") or episodes),
        "episodes_completed": int(data.get("episodes_completed") or 0),
        "crash_count": int(data.get("crash_count") or 0),
        "illegal_action_count": int(data.get("illegal_action_count") or 0),
        "no_progress_count": int(data.get("no_progress_loop_count") or 0),
        "deterministic_replay_pass_count": int(data.get("deterministic_replay_pass_count") or 0),
        "contract_failure_count": int(data.get("contract_failure_count") or 0),
        "average_floor": float(data.get("average_floor") or 0.0),
        "median_floor": float(data.get("median_floor") or 0.0),
        "average_steps": float(data.get("average_steps") or 0.0),
        "average_reward": float(data.get("average_total_reward") or 0.0),
        "average_combat_wins": float(data.get("average_combat_wins") or 0.0),
        "steps_per_second": float(data.get("steps_per_second") or 0.0),
        "wall_seconds": elapsed,
        "result_counts": data.get("result_counts") or {},
        "terminal_reason_counts": {},
        "decision_type_counts": data.get("decision_type_counts") or {},
        "failure_examples": data.get("contract_failures") or [],
        "anomaly_flags": rust_anomaly_flags(data, args.max_steps),
    }
    return summary


def rust_anomaly_flags(data: dict[str, Any], max_steps: int) -> list[str]:
    flags: list[str] = []
    if int(data.get("crash_count") or 0) > 0:
        flags.append("crash")
    if int(data.get("illegal_action_count") or 0) > 0:
        flags.append("illegal_action")
    if int(data.get("no_progress_loop_count") or 0) > 0:
        flags.append("no_progress")
    if int(data.get("contract_failure_count") or 0) > 0:
        flags.append("contract_failure")
    requested = int(data.get("episodes_requested") or 0)
    replay = int(data.get("deterministic_replay_pass_count") or 0)
    if requested > 0 and replay < requested:
        flags.append("determinism_miss")
    if float(data.get("average_steps") or 0.0) > 0.9 * float(max_steps):
        flags.append("near_step_cap_average_steps")
    return flags


def aggregate_group(name: str, summaries: list[dict[str, Any]]) -> dict[str, Any]:
    episodes = sum(int(item.get("episodes") or item.get("episodes_completed") or 0) for item in summaries)
    if episodes <= 0:
        return {"policy": name, "episodes": 0}

    def weighted_average(key: str) -> float:
        return sum(
            float(item.get(key) or 0.0) * int(item.get("episodes") or item.get("episodes_completed") or 0)
            for item in summaries
        ) / episodes

    flags = sorted({flag for item in summaries for flag in item.get("anomaly_flags", [])})
    return {
        "policy": name,
        "episodes": episodes,
        "crash_count": sum(int(item.get("crash_count") or 0) for item in summaries),
        "illegal_action_count": sum(int(item.get("illegal_action_count") or 0) for item in summaries),
        "no_progress_count": sum(int(item.get("no_progress_count") or 0) for item in summaries),
        "contract_failure_count": sum(int(item.get("contract_failure_count") or 0) for item in summaries),
        "average_floor": weighted_average("average_floor"),
        "median_floor_mean": mean([float(item.get("median_floor") or 0.0) for item in summaries]),
        "average_steps": weighted_average("average_steps"),
        "average_reward": weighted_average("average_reward"),
        "anomaly_flags": flags,
    }


def main() -> int:
    args = parse_args()
    train_seeds = parse_int_list(args.train_seeds)
    eval_seeds = parse_int_list(args.eval_seeds)
    artifact_dir = args.artifact_dir or (REPO_ROOT / "tools" / "artifacts" / "full_run_rl_matrix")
    artifact_dir.mkdir(parents=True, exist_ok=True)
    out_path = args.out or artifact_dir / "full_run_ppo_sanity_matrix.json"

    baselines: dict[str, list[dict[str, Any]]] = {"random_masked": []}
    if not args.skip_rule_baseline:
        baselines["rule_baseline_v0"] = []

    for eval_seed in eval_seeds:
        baselines["random_masked"].append(
            run_rust_policy(
                args,
                policy="random_masked",
                base_seed=eval_seed,
                episodes=args.eval_episodes,
                out_dir=artifact_dir,
            )
        )
        if not args.skip_rule_baseline:
            baselines["rule_baseline_v0"].append(
                run_rust_policy(
                    args,
                    policy="rule_baseline_v0",
                    base_seed=eval_seed,
                    episodes=args.eval_episodes,
                    out_dir=artifact_dir,
                )
            )

    ppo_runs: list[dict[str, Any]] = []
    for train_seed in train_seeds:
        model_out = artifact_dir / f"full_run_maskable_ppo_seed_{train_seed}.zip"
        model, train_summary = train_ppo(args, train_seed, model_out)
        evals = [
            evaluate_python_policy(
                args,
                policy_name=f"ppo_seed_{train_seed}",
                base_seed=eval_seed,
                episodes=args.eval_episodes,
                model=model,
            )
            for eval_seed in eval_seeds
        ]
        ppo_runs.append(
            {
                "train_seed": train_seed,
                "train": train_summary,
                "evals": evals,
                "aggregate": aggregate_group(f"ppo_seed_{train_seed}", evals),
            }
        )

    baseline_aggregates = {
        name: aggregate_group(name, summaries) for name, summaries in baselines.items()
    }
    all_ppo_evals = [item for run in ppo_runs for item in run["evals"]]
    ppo_aggregate = aggregate_group("ppo_all_seeds", all_ppo_evals)
    random_floor = baseline_aggregates["random_masked"]["average_floor"]
    rule_floor = baseline_aggregates.get("rule_baseline_v0", {}).get("average_floor")
    comparison = {
        "ppo_minus_random_average_floor": float(ppo_aggregate["average_floor"] - random_floor),
        "ppo_minus_rule_average_floor": None
        if rule_floor is None
        else float(ppo_aggregate["average_floor"] - float(rule_floor)),
    }
    issue_flags = sorted(
        {
            flag
            for group in [*baseline_aggregates.values(), ppo_aggregate]
            for flag in group.get("anomaly_flags", [])
        }
    )

    report = {
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "purpose": "full-run PPO sanity matrix; not a policy-strength claim",
        "config": {
            "timesteps": args.timesteps,
            "n_envs": args.n_envs,
            "train_seeds": train_seeds,
            "eval_seeds": eval_seeds,
            "eval_episodes": args.eval_episodes,
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
        "baseline_aggregates": baseline_aggregates,
        "ppo_aggregate": ppo_aggregate,
        "comparison": comparison,
        "issue_flags": issue_flags,
        "baselines": baselines,
        "ppo_runs": ppo_runs,
    }
    write_json(out_path, report)
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0 if not issue_flags else 1


if __name__ == "__main__":
    raise SystemExit(main())
