#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import numpy as np
from sb3_contrib import MaskablePPO
from stable_baselines3.common.vec_env import DummyVecEnv, VecMonitor

from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from gym_combat_env import GymCombatEnv


def load_config(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def make_env(
    start_spec: Path,
    driver_binary: Path | None,
    env_seed: int,
    max_episode_steps: int,
    train_seeds: list[int],
    reward_mode: str,
    reward_config: dict[str, float],
    draw_order_variant: str,
):
    def _factory():
        return GymCombatEnv(
            spec_paths=[start_spec],
            spec_source="start_spec",
            driver_binary=driver_binary,
            seed=env_seed,
            seed_pool=train_seeds,
            max_episode_steps=max_episode_steps,
            reward_mode=reward_mode,
            reward_config=reward_config,
            draw_order_variant=draw_order_variant,
        )

    return _factory


def hand_contains_disarm(info: dict[str, Any]) -> bool:
    return any("Disarm" in str(name or "") for name in (info.get("hand_cards") or []))


def action_is_disarm(label: Any) -> bool:
    return "Disarm" in str(label or "")


def evaluate_policy(
    model: MaskablePPO,
    start_spec: Path,
    driver_binary: Path | None,
    eval_seeds: list[int],
    max_episode_steps: int,
    reward_mode: str,
    reward_config: dict[str, float],
    draw_order_variant: str,
    artifact_context: dict[str, Any],
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    env = GymCombatEnv(
        spec_paths=[start_spec],
        spec_source="start_spec",
        driver_binary=driver_binary,
        seed=0,
        max_episode_steps=max_episode_steps,
        reward_mode=reward_mode,
        reward_config=reward_config,
        draw_order_variant=draw_order_variant,
    )
    rows: list[dict[str, Any]] = []
    try:
        for episode_index, seed_hint in enumerate(eval_seeds):
            obs, info = env.reset(options={"spec_path": str(start_spec), "seed_hint": int(seed_hint)})
            done = False
            truncated = False
            episode_reward = 0.0
            catastrophe_hits = 0
            last_info = info
            starting_hp = float(info.get("player_hp") or 0.0)
            final_hp = starting_hp
            first_action_label = None
            initial_turn_count = int(info.get("turn_count") or 0)
            opening_turn_saw_disarm = hand_contains_disarm(info)
            disarm_seen_turn = initial_turn_count if opening_turn_saw_disarm else None
            disarm_played_turn = None
            played_disarm_on_opening_turn = False
            played_disarm_on_first_seen_turn = False
            while not done and not truncated:
                action_masks = env.action_masks()
                current_turn_count = int(last_info.get("turn_count") or info.get("turn_count") or 0)
                if disarm_seen_turn is None and hand_contains_disarm(last_info):
                    disarm_seen_turn = current_turn_count
                action, _ = model.predict(obs, deterministic=True, action_masks=action_masks)
                obs, reward, done, truncated, step_info = env.step(int(action))
                episode_reward += float(reward)
                if first_action_label is None:
                    first_action_label = step_info.get("chosen_action_label")
                if action_is_disarm(step_info.get("chosen_action_label")):
                    if disarm_played_turn is None:
                        disarm_played_turn = current_turn_count
                    if opening_turn_saw_disarm and current_turn_count == initial_turn_count:
                        played_disarm_on_opening_turn = True
                    if disarm_seen_turn is not None and current_turn_count == disarm_seen_turn:
                        played_disarm_on_first_seen_turn = True
                if float(step_info.get("visible_unblocked") or 0.0) >= float(
                    reward_config["catastrophe_unblocked_threshold"]
                ):
                    catastrophe_hits += 1
                final_hp = float(step_info.get("player_hp") or final_hp)
                last_info = step_info
            damage_taken = max(starting_hp - final_hp, 0.0)
            rows.append(
                {
                    "artifact_context": artifact_context,
                    "episode_index": episode_index,
                    "seed_hint": int(seed_hint),
                    "spec_name": last_info.get("spec_name"),
                    "episode_reward": round(episode_reward, 4),
                    "outcome": last_info.get("outcome"),
                    "damage_taken": round(damage_taken, 4),
                    "catastrophe_hits": catastrophe_hits,
                    "final_hp": round(final_hp, 4),
                    "first_visible_incoming": info.get("visible_incoming"),
                    "first_action_label": first_action_label,
                    "opening_turn_saw_disarm": opening_turn_saw_disarm,
                    "played_disarm_on_opening_turn": played_disarm_on_opening_turn,
                    "disarm_seen_turn": disarm_seen_turn,
                    "disarm_played_turn": disarm_played_turn,
                    "played_disarm_on_first_seen_turn": played_disarm_on_first_seen_turn,
                }
            )
    finally:
        env.close()

    episodes = len(rows)
    metrics = {
        "episodes": episodes,
        "pass_rate": float(np.mean([1.0 if row.get("outcome") == "victory" else 0.0 for row in rows]))
        if rows
        else 0.0,
        "avg_reward": float(np.mean([float(row.get("episode_reward") or 0.0) for row in rows]))
        if rows
        else 0.0,
        "avg_damage_taken": float(np.mean([float(row.get("damage_taken") or 0.0) for row in rows]))
        if rows
        else 0.0,
        "catastrophe_rate": float(np.mean([1.0 if row.get("catastrophe_hits", 0) > 0 else 0.0 for row in rows]))
        if rows
        else 0.0,
        "worst_damage_taken": float(max((float(row.get("damage_taken") or 0.0) for row in rows), default=0.0)),
        "disarm_seen_episode_rate": float(
            np.mean([1.0 if row.get("disarm_seen_turn") is not None else 0.0 for row in rows])
        )
        if rows
        else 0.0,
        "disarm_played_on_first_seen_turn_rate": float(
            np.mean([1.0 if row.get("played_disarm_on_first_seen_turn") else 0.0 for row in rows])
        )
        if rows
        else 0.0,
        "disarm_played_on_opening_turn_rate": float(
            np.mean([1.0 if row.get("played_disarm_on_opening_turn") else 0.0 for row in rows])
        )
        if rows
        else 0.0,
    }
    return metrics, rows


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run a minimal fixed boss/deck combat-local RL experiment over a natural start spec."
    )
    parser.add_argument("--config", required=True, type=Path)
    parser.add_argument("--driver-binary", default=None, type=Path)
    parser.add_argument("--model-out", default=None, type=Path)
    parser.add_argument("--metrics-out", default=None, type=Path)
    parser.add_argument("--episodes-out", default=None, type=Path)
    args = parser.parse_args()

    config = load_config(args.config)
    name = str(config["name"])
    start_spec = REPO_ROOT / str(config["start_spec"])
    train_seeds = [int(value) for value in config["train_seeds"]]
    eval_seeds = [int(value) for value in config["eval_seeds"]]
    timesteps = int(config.get("timesteps", 4096))
    n_envs = int(config.get("n_envs", 4))
    max_episode_steps = int(config.get("max_episode_steps", 64))
    reward_mode = str(config.get("reward_mode") or "minimal_rl")
    draw_order_variant = str(config.get("draw_order_variant") or "exact")
    reward_config = {
        "victory_reward": 1.0,
        "defeat_reward": -1.0,
        "hp_loss_scale": 0.02,
        "catastrophe_unblocked_threshold": 18.0,
        "catastrophe_penalty": 0.25,
        "next_enemy_window_relief_scale": 0.0,
        "persistent_attack_script_relief_scale": 0.0,
    }
    reward_config.update({key: float(value) for key, value in (config.get("reward") or {}).items()})
    ppo_config = {
        "n_steps": 128,
        "batch_size": 128,
        "learning_rate": 3e-4,
        "gamma": 0.99,
        "ent_coef": 0.01,
        "seed": 7,
    }
    ppo_config.update(config.get("ppo") or {})

    artifact_dir = REPO_ROOT / "tools" / "artifacts" / "learning_dataset"
    model_out = args.model_out or artifact_dir / f"{name}_ppo_model.zip"
    metrics_out = args.metrics_out or artifact_dir / f"{name}_rl_metrics.json"
    episodes_out = args.episodes_out or artifact_dir / f"{name}_rl_eval_episodes.jsonl"

    env_fns = [
        make_env(
            start_spec=start_spec,
            driver_binary=args.driver_binary,
            env_seed=int(ppo_config["seed"]) + idx,
            max_episode_steps=max_episode_steps,
            train_seeds=train_seeds,
            reward_mode=reward_mode,
            reward_config=reward_config,
            draw_order_variant=draw_order_variant,
        )
        for idx in range(n_envs)
    ]
    vec_env = VecMonitor(DummyVecEnv(env_fns))
    model = MaskablePPO(
        "MlpPolicy",
        vec_env,
        verbose=0,
        seed=int(ppo_config["seed"]),
        n_steps=int(ppo_config["n_steps"]),
        batch_size=int(ppo_config["batch_size"]),
        learning_rate=float(ppo_config["learning_rate"]),
        gamma=float(ppo_config["gamma"]),
        ent_coef=float(ppo_config["ent_coef"]),
        tensorboard_log=None,
    )
    model.learn(total_timesteps=timesteps, progress_bar=False)
    model.save(str(model_out))

    artifact_context = {
        "config_name": name,
        "config_path": str(args.config.resolve()),
        "reward_mode": reward_mode,
        "draw_order_variant": draw_order_variant,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
    }

    eval_metrics, eval_rows = evaluate_policy(
        model=model,
        start_spec=start_spec,
        driver_binary=args.driver_binary,
        eval_seeds=eval_seeds,
        max_episode_steps=max_episode_steps,
        reward_mode=reward_mode,
        reward_config=reward_config,
        draw_order_variant=draw_order_variant,
        artifact_context=artifact_context,
    )
    report = {
        "artifact_context": artifact_context,
        "experiment": name,
        "start_spec": str(start_spec),
        "train_seed_count": len(train_seeds),
        "eval_seed_count": len(eval_seeds),
        "timesteps": timesteps,
        "n_envs": n_envs,
        "max_episode_steps": max_episode_steps,
        "reward": reward_config,
        "reward_mode": reward_mode,
        "draw_order_variant": draw_order_variant,
        "ppo": ppo_config,
        "eval": eval_metrics,
        "notes": [
            "minimal combat-local RL scaffold over natural-start start_spec",
            "train and eval seeds are explicitly disjoint",
            "reward is terminal-first with small hp-loss and catastrophe shaping",
            "auxiliary heads are a later step, not part of this first harness",
        ],
    }
    write_json(metrics_out, report)
    write_jsonl(episodes_out, eval_rows)

    print(json.dumps(report, indent=2, ensure_ascii=False))
    print(f"saved model to {model_out}")
    print(f"wrote metrics to {metrics_out}")
    print(f"wrote eval episodes to {episodes_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
