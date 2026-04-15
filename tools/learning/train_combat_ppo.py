#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any

import numpy as np
from sb3_contrib import MaskablePPO
from sb3_contrib.common.maskable.callbacks import MaskableEvalCallback
from stable_baselines3.common.vec_env import DummyVecEnv, VecMonitor

from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from gym_combat_env import GymCombatEnv, discover_spec_paths
from combat_reranker_common import curriculum_tag_from_spec_name

SETUP_CARD_NAMES = {"Flex", "Rage", "Inflame", "Fire Breathing", "Dark Embrace", "Battle Trance"}
SURVIVAL_CARD_NAMES = {"Defend", "Power Through", "Impervious", "Shrug It Off", "Ghostly Armor", "True Grit"}
STATUS_ENGINE_CARD_NAMES = {"Second Wind", "Fire Breathing", "Dark Embrace", "Evolve"}


def action_behavior_flags(action_label: str) -> dict[str, bool]:
    label = str(action_label or "")
    return {
        "is_end_turn": label == "EndTurn",
        "is_defend": "Defend" in label or "Impervious" in label or "Power Through" in label,
        "is_potion": label.startswith("UsePotion"),
        "is_setup": any(name in label for name in SETUP_CARD_NAMES),
        "is_survival": any(name in label for name in SURVIVAL_CARD_NAMES),
        "is_status_engine": any(name in label for name in STATUS_ENGINE_CARD_NAMES),
        "is_attack_like": (
            label.startswith("Play #")
            and not any(name in label for name in SETUP_CARD_NAMES | SURVIVAL_CARD_NAMES | STATUS_ENGINE_CARD_NAMES)
            and "Defend" not in label
        ),
    }


def _setup_before_payoff_spec(spec_path: Path) -> bool:
    name = spec_path.stem.lower()
    if any(token in name for token in ("power_through", "corruption")):
        return False
    return any(token in name for token in ("flex", "rage", "spot_weakness", "inflame"))


def make_env(spec_paths: list[Path], driver_binary: Path | None, seed: int, max_episode_steps: int):
    def _factory():
        return GymCombatEnv(spec_paths=spec_paths, driver_binary=driver_binary, seed=seed, max_episode_steps=max_episode_steps)

    return _factory


def curriculum_metrics(episodes: list[dict[str, Any]]) -> dict[str, Any]:
    by_tag: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for episode in episodes:
        by_tag[str(episode.get("curriculum_tag") or "unknown")].append(episode)
    summary = {}
    for tag, rows in by_tag.items():
        win_rate = sum(1 for row in rows if row.get("outcome") == "victory") / len(rows)
        avg_reward = float(np.mean([float(row.get("episode_reward") or 0.0) for row in rows]))
        avg_damage_taken = float(np.mean([float(row.get("damage_taken") or 0.0) for row in rows]))
        avg_empty_defend = float(np.mean([float(row.get("empty_defend_count") or 0.0) for row in rows]))
        avg_setup_first = float(np.mean([1.0 if row.get("first_action_setup") else 0.0 for row in rows]))
        avg_attack_first = float(np.mean([1.0 if row.get("first_action_attack_like") else 0.0 for row in rows]))
        avg_survival_first = float(np.mean([1.0 if row.get("first_action_survival") else 0.0 for row in rows]))
        avg_potion_use = float(np.mean([1.0 if row.get("potion_use_count", 0) > 0 else 0.0 for row in rows]))
        summary[tag] = {
            "episodes": len(rows),
            "win_rate": win_rate,
            "avg_reward": avg_reward,
            "avg_damage_taken": avg_damage_taken,
            "avg_empty_defend_count": avg_empty_defend,
            "first_action_setup_rate": avg_setup_first,
            "first_action_attack_like_rate": avg_attack_first,
            "first_action_survival_rate": avg_survival_first,
            "potion_use_rate": avg_potion_use,
        }
    return summary


def tactical_bucket_metrics(episodes: list[dict[str, Any]]) -> dict[str, Any]:
    buckets = {
        "attack_over_defend": lambda row: row.get("curriculum_tag") == "attack_over_defend",
        "setup_before_payoff": lambda row: row.get("curriculum_tag") == "setup_before_payoff",
        "potion_bridge": lambda row: row.get("curriculum_tag") == "potion_bridge",
        "survival_override": lambda row: row.get("curriculum_tag") == "survival_override",
        "status_exhaust_draw": lambda row: row.get("curriculum_tag") == "status_exhaust_draw",
    }
    summary: dict[str, Any] = {}
    for name, predicate in buckets.items():
        rows = [row for row in episodes if predicate(row)]
        if not rows:
            continue
        summary[name] = {
            "episodes": len(rows),
            "win_rate": float(np.mean([1.0 if row.get("outcome") == "victory" else 0.0 for row in rows])),
            "avg_damage_taken": float(np.mean([float(row.get("damage_taken") or 0.0) for row in rows])),
            "empty_defend_rate": float(np.mean([1.0 if row.get("empty_defend_count", 0) > 0 else 0.0 for row in rows])),
            "first_action_attack_like_rate": float(np.mean([1.0 if row.get("first_action_attack_like") else 0.0 for row in rows])),
            "first_action_setup_rate": float(np.mean([1.0 if row.get("first_action_setup") else 0.0 for row in rows])),
            "first_action_survival_rate": float(np.mean([1.0 if row.get("first_action_survival") else 0.0 for row in rows])),
            "potion_use_rate": float(np.mean([1.0 if row.get("potion_use_count", 0) > 0 else 0.0 for row in rows])),
            "status_engine_play_rate": float(np.mean([1.0 if row.get("status_engine_play_count", 0) > 0 else 0.0 for row in rows])),
        }
    return summary


def spec_metrics(episodes: list[dict[str, Any]]) -> dict[str, Any]:
    by_spec: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for episode in episodes:
        by_spec[str(episode.get("spec_name") or "unknown")].append(episode)
    summary = {}
    for spec_name, rows in by_spec.items():
        summary[spec_name] = {
            "episodes": len(rows),
            "win_rate": float(np.mean([1.0 if row.get("outcome") == "victory" else 0.0 for row in rows])),
            "avg_reward": float(np.mean([float(row.get("episode_reward") or 0.0) for row in rows])),
            "avg_damage_taken": float(np.mean([float(row.get("damage_taken") or 0.0) for row in rows])),
            "empty_defend_rate": float(np.mean([1.0 if row.get("empty_defend_count", 0) > 0 else 0.0 for row in rows])),
        }
    return summary


def evaluate_policy(model: MaskablePPO, spec_paths: list[Path], driver_binary: Path | None, eval_episodes: int, max_episode_steps: int, seed: int) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    env = GymCombatEnv(spec_paths=spec_paths, driver_binary=driver_binary, seed=seed, max_episode_steps=max_episode_steps)
    episodes: list[dict[str, Any]] = []
    try:
        for episode_index in range(eval_episodes):
            obs, info = env.reset(seed=seed + episode_index)
            done = False
            truncated = False
            episode_reward = 0.0
            empty_defend_count = 0
            invalid_action_count = 0
            potion_use_count = 0
            status_engine_play_count = 0
            first_action_setup = False
            first_action_attack_like = False
            first_action_survival = False
            first_action_recorded = False
            starting_hp = float((env._last_response.get("payload") or {}).get("observation", {}).get("player_hp") or 0)  # type: ignore[attr-defined]
            final_hp = starting_hp
            last_step_info = info
            while not done and not truncated:
                action_masks = env.action_masks()
                action, _ = model.predict(obs, deterministic=True, action_masks=action_masks)
                obs, reward, done, truncated, step_info = env.step(int(action))
                episode_reward += float(reward)
                final_hp = float((env._last_response.get("payload") or {}).get("observation", {}).get("player_hp") or final_hp)  # type: ignore[attr-defined]
                action_label = str(step_info.get("chosen_action_label") or "")
                flags = action_behavior_flags(action_label)
                if not first_action_recorded:
                    first_action_recorded = True
                    first_action_setup = flags["is_setup"]
                    first_action_attack_like = flags["is_attack_like"]
                    first_action_survival = flags["is_survival"]
                if "Defend" in action_label and int(step_info.get("visible_incoming") or 0) == 0:
                    empty_defend_count += 1
                if step_info.get("invalid_action"):
                    invalid_action_count += 1
                if flags["is_potion"]:
                    potion_use_count += 1
                if flags["is_status_engine"]:
                    status_engine_play_count += 1
                last_step_info = step_info
            damage_taken = max(starting_hp - final_hp, 0.0)
            episodes.append(
                {
                    "episode_index": episode_index,
                    "spec_name": info.get("spec_name"),
                    "curriculum_tag": info.get("curriculum_tag"),
                    "episode_reward": round(episode_reward, 4),
                    "outcome": last_step_info.get("outcome"),
                    "damage_taken": round(damage_taken, 4),
                    "empty_defend_count": empty_defend_count,
                    "invalid_action_count": invalid_action_count,
                    "potion_use_count": potion_use_count,
                    "status_engine_play_count": status_engine_play_count,
                    "first_action_setup": first_action_setup,
                    "first_action_attack_like": first_action_attack_like,
                    "first_action_survival": first_action_survival,
                }
            )
    finally:
        env.close()
    win_rate = sum(1 for row in episodes if row.get("outcome") == "victory") / len(episodes) if episodes else 0.0
    metrics = {
        "episodes": len(episodes),
        "win_rate": win_rate,
        "avg_reward": float(np.mean([float(row.get("episode_reward") or 0.0) for row in episodes])) if episodes else 0.0,
        "avg_damage_taken": float(np.mean([float(row.get("damage_taken") or 0.0) for row in episodes])) if episodes else 0.0,
        "empty_defend_rate": float(np.mean([1.0 if row.get("empty_defend_count", 0) > 0 else 0.0 for row in episodes])) if episodes else 0.0,
        "invalid_action_rate": float(np.mean([1.0 if row.get("invalid_action_count", 0) > 0 else 0.0 for row in episodes])) if episodes else 0.0,
        "curriculum_tag_metrics": curriculum_metrics(episodes),
        "tactical_bucket_metrics": tactical_bucket_metrics(episodes),
        "spec_metrics": spec_metrics(episodes),
    }
    return metrics, episodes


def main() -> int:
    parser = argparse.ArgumentParser(description="Train a real MaskablePPO combat policy over the CombatEnv driver.")
    parser.add_argument("--spec-dir", default=REPO_ROOT / "data" / "combat_lab" / "specs", type=Path)
    parser.add_argument("--driver-binary", default=None, type=Path)
    parser.add_argument("--timesteps", default=4096, type=int)
    parser.add_argument("--n-envs", default=4, type=int)
    parser.add_argument("--seed", default=7, type=int)
    parser.add_argument("--max-episode-steps", default=64, type=int)
    parser.add_argument("--eval-episodes", default=16, type=int)
    parser.add_argument("--curriculum-tags", default="", help="Optional comma-separated curriculum tag filter for training/eval specs.")
    parser.add_argument("--output-prefix", default="", help="Optional output prefix under tools/artifacts/learning_dataset.")
    parser.add_argument("--model-out", default=None, type=Path)
    parser.add_argument("--metrics-out", default=None, type=Path)
    parser.add_argument("--episodes-out", default=None, type=Path)
    args = parser.parse_args()

    dataset_dir = REPO_ROOT / "tools" / "artifacts" / "learning_dataset"
    prefix = str(args.output_prefix or "").strip()
    model_out = args.model_out or (dataset_dir / (f"{prefix}_combat_maskable_ppo_model.zip" if prefix else "combat_maskable_ppo_model.zip"))
    metrics_out = args.metrics_out or (dataset_dir / (f"{prefix}_ppo_eval_metrics.json" if prefix else "ppo_eval_metrics.json"))
    episodes_out = args.episodes_out or (dataset_dir / (f"{prefix}_ppo_eval_episodes.jsonl" if prefix else "ppo_eval_episodes.jsonl"))

    spec_paths = discover_spec_paths(args.spec_dir)
    requested_tags = [tag.strip() for tag in str(args.curriculum_tags or "").split(",") if tag.strip()]
    if requested_tags:
        spec_paths = [path for path in spec_paths if curriculum_tag_from_spec_name(path.stem) in set(requested_tags)]
        if requested_tags == ["setup_before_payoff"]:
            spec_paths = [path for path in spec_paths if _setup_before_payoff_spec(path)]
    if not spec_paths:
        raise SystemExit(f"no combat specs found under {args.spec_dir}")

    env_fns = [make_env(spec_paths, args.driver_binary, args.seed + idx, args.max_episode_steps) for idx in range(args.n_envs)]
    vec_env = VecMonitor(DummyVecEnv(env_fns))
    eval_env = VecMonitor(DummyVecEnv([make_env(spec_paths, args.driver_binary, args.seed + 10_000, args.max_episode_steps)]))

    eval_callback = MaskableEvalCallback(
        eval_env,
        n_eval_episodes=min(8, args.eval_episodes),
        eval_freq=max(args.timesteps // 4, 256),
        deterministic=True,
        render=False,
        warn=False,
    )
    model = MaskablePPO(
        "MlpPolicy",
        vec_env,
        verbose=0,
        seed=args.seed,
        n_steps=128,
        batch_size=128,
        ent_coef=0.01,
        learning_rate=3e-4,
        gamma=0.97,
        tensorboard_log=None,
    )
    model.learn(total_timesteps=args.timesteps, callback=eval_callback, progress_bar=False)
    model.save(str(model_out))

    eval_metrics, eval_rows = evaluate_policy(
        model,
        spec_paths=spec_paths,
        driver_binary=args.driver_binary,
        eval_episodes=args.eval_episodes,
        max_episode_steps=args.max_episode_steps,
        seed=args.seed + 20_000,
    )
    metrics = {
        "model": "maskable_ppo",
        "timesteps": args.timesteps,
        "n_envs": args.n_envs,
        "seed": args.seed,
        "max_episode_steps": args.max_episode_steps,
        "spec_count": len(spec_paths),
        "spec_dir": str(args.spec_dir),
        "driver_binary": str(args.driver_binary) if args.driver_binary else None,
        "curriculum_tags": requested_tags,
        "output_prefix": prefix,
        "eval": eval_metrics,
        "notes": [
            "true PPO policy trained against the combat_env_driver bridge",
            "uses ActionMask through sb3-contrib MaskablePPO",
            "offline-first: fixed combat specs only, no live runtime control",
        ],
    }
    write_json(metrics_out, metrics)
    write_jsonl(episodes_out, eval_rows)

    print(json.dumps(metrics, indent=2, ensure_ascii=False))
    print(f"wrote PPO metrics to {metrics_out}")
    print(f"wrote PPO eval episodes to {episodes_out}")
    print(f"saved PPO model to {model_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
