#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any

import numpy as np
import torch
from torch import nn

from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from structured_benchmark import resolve_benchmark_cases
from structured_combat_env import (
    CARD_ID_VOCAB,
    INTENT_KIND_IDS,
    MONSTER_ID_VOCAB,
    POTION_ID_VOCAB,
    POWER_ID_VOCAB,
    StructuredGymCombatEnv,
    discover_spec_paths,
)
from structured_policy import evaluate_actions, sample_actions, to_device_obs, StructuredPolicyNet

SETUP_CARD_NAMES = {"Flex", "Rage", "Inflame", "Fire Breathing", "Dark Embrace", "Battle Trance"}
SURVIVAL_CARD_NAMES = {"Defend", "Power Through", "Impervious", "Shrug It Off", "Ghostly Armor", "True Grit"}
STATUS_ENGINE_CARD_NAMES = {"Second Wind", "Fire Breathing", "Dark Embrace", "Evolve"}


def stack_obs_np(obs_list: list[dict[str, np.ndarray]]) -> dict[str, np.ndarray]:
    return {key: np.stack([obs[key] for obs in obs_list], axis=0) for key in obs_list[0].keys()}


def flatten_obs_time(obs_steps: list[dict[str, np.ndarray]]) -> dict[str, np.ndarray]:
    merged = {}
    for key in obs_steps[0].keys():
        merged[key] = np.concatenate([step[key] for step in obs_steps], axis=0)
    return merged


def index_obs(obs: dict[str, np.ndarray], indices: np.ndarray) -> dict[str, np.ndarray]:
    return {key: value[indices] for key, value in obs.items()}


def compute_gae(
    rewards: np.ndarray,
    dones: np.ndarray,
    values: np.ndarray,
    last_values: np.ndarray,
    gamma: float,
    gae_lambda: float,
) -> tuple[np.ndarray, np.ndarray]:
    advantages = np.zeros_like(rewards, dtype=np.float32)
    last_advantage = np.zeros(rewards.shape[1], dtype=np.float32)
    next_values = last_values.astype(np.float32)
    for t in reversed(range(rewards.shape[0])):
        not_done = 1.0 - dones[t]
        delta = rewards[t] + gamma * next_values * not_done - values[t]
        last_advantage = delta + gamma * gae_lambda * not_done * last_advantage
        advantages[t] = last_advantage
        next_values = values[t]
    returns = advantages + values
    return advantages, returns


def current_energy_spent(raw_observation: dict[str, Any], action: dict[str, int]) -> float:
    if int(action.get("action_type") or 0) != 1:
        return 0.0
    hand = list(raw_observation.get("hand") or [])
    slot = int(action.get("card_slot") or 0)
    if slot < 0 or slot >= len(hand):
        return 0.0
    cost = int(hand[slot].get("cost_for_turn") or 0)
    if cost < 0:
        return float(raw_observation.get("energy") or 0.0)
    return float(max(cost, 0))


def current_energy_spent_flat(env: Any, action_index: int) -> float:
    payload = ((env._last_response or {}).get("payload") or {})  # type: ignore[attr-defined]
    obs = payload.get("observation") or {}
    candidates = list(payload.get("action_candidates") or [])
    if action_index < 0 or action_index >= len(candidates):
        return 0.0
    candidate = candidates[action_index]
    if str(candidate.get("action_family") or "") != "play_card":
        return 0.0
    slot = candidate.get("slot_index")
    hand = list(obs.get("hand") or [])
    if slot is None or int(slot) < 0 or int(slot) >= len(hand):
        return 0.0
    cost = int(hand[int(slot)].get("cost_for_turn") or 0)
    if cost < 0:
        return float(obs.get("energy") or 0.0)
    return float(max(cost, 0))


def action_behavior_flags(action_label: str) -> dict[str, bool]:
    label = str(action_label or "")
    return {
        "is_end_turn": label == "EndTurn",
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


def total_living_monster_hp(raw_observation: dict[str, Any]) -> float:
    monsters = list(raw_observation.get("monsters") or [])
    return float(
        sum(float(monster.get("current_hp") or 0.0) for monster in monsters if monster.get("alive", True))
    )


def lagavulin_sleep_state(raw_observation: dict[str, Any]) -> bool | None:
    for monster in raw_observation.get("monsters") or []:
        if str(monster.get("monster_id") or "") == "Lagavulin":
            return bool((monster.get("mechanic_state") or {}).get("sleeping"))
    return None


def first_step_signals(
    before: dict[str, Any],
    after: dict[str, Any],
    action_label: str | None,
) -> dict[str, Any]:
    flags = action_behavior_flags(str(action_label or ""))
    before_pressure = before.get("pressure") or {}
    after_pressure = after.get("pressure") or {}
    before_sleep = lagavulin_sleep_state(before)
    after_sleep = lagavulin_sleep_state(after)
    lagavulin_sleep_preserved = None
    if before_sleep is True:
        lagavulin_sleep_preserved = 1.0 if after_sleep is True else 0.0
    return {
        "first_action_end_turn": flags["is_end_turn"],
        "first_action_setup": flags["is_setup"],
        "first_action_survival": flags["is_survival"],
        "first_action_attack_like": flags["is_attack_like"],
        "first_action_potion": flags["is_potion"],
        "first_action_status_engine": flags["is_status_engine"],
        "first_step_threat_reduction": float(
            float(before_pressure.get("visible_unblocked") or 0.0)
            - float(after_pressure.get("visible_unblocked") or 0.0)
        ),
        "first_step_block_gain": float(
            float(after.get("player_block") or 0.0) - float(before.get("player_block") or 0.0)
        ),
        "first_step_monster_hp_delta": float(total_living_monster_hp(before) - total_living_monster_hp(after)),
        "pending_choice_entered": 1.0 if after.get("pending_choice_kind") else 0.0,
        "lagavulin_sleep_preserved": lagavulin_sleep_preserved,
    }


def _mean_numeric(rows: list[dict[str, Any]], key: str) -> float:
    if not rows:
        return 0.0
    return float(np.mean([float(row.get(key) or 0.0) for row in rows]))


def _mean_optional_numeric(rows: list[dict[str, Any]], key: str) -> float | None:
    values = [float(row[key]) for row in rows if row.get(key) is not None]
    if not values:
        return None
    return float(np.mean(values))


def summarize_benchmark_rows(rows: list[dict[str, Any]]) -> dict[str, Any]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        grouped[str(row.get("tag") or "unknown")].append(row)

    def summarize_group(group_rows: list[dict[str, Any]]) -> dict[str, Any]:
        total_steps = max(sum(int(row.get("steps") or 0) for row in group_rows), 1)
        return {
            "episodes": len(group_rows),
            "win_rate": float(
                sum(1 for row in group_rows if row.get("outcome") == "victory") / max(len(group_rows), 1)
            ),
            "mean_reward": _mean_numeric(group_rows, "reward_total"),
            "mean_steps": _mean_numeric(group_rows, "steps"),
            "invalid_action_rate": float(
                sum(int(row.get("invalid_actions") or 0) for row in group_rows) / total_steps
            ),
            "decoder_failure_rate": float(
                sum(int(row.get("decoder_failures") or 0) for row in group_rows) / total_steps
            ),
            "mean_energy_spent": _mean_numeric(group_rows, "energy_spent"),
            "first_action_end_turn_rate": _mean_numeric(group_rows, "first_action_end_turn"),
            "first_action_setup_rate": _mean_numeric(group_rows, "first_action_setup"),
            "first_action_survival_rate": _mean_numeric(group_rows, "first_action_survival"),
            "first_action_attack_like_rate": _mean_numeric(group_rows, "first_action_attack_like"),
            "first_action_potion_rate": _mean_numeric(group_rows, "first_action_potion"),
            "first_action_status_engine_rate": _mean_numeric(group_rows, "first_action_status_engine"),
            "mean_first_step_threat_reduction": _mean_numeric(group_rows, "first_step_threat_reduction"),
            "mean_first_step_block_gain": _mean_numeric(group_rows, "first_step_block_gain"),
            "mean_first_step_monster_hp_delta": _mean_numeric(group_rows, "first_step_monster_hp_delta"),
            "pending_choice_entered_rate": _mean_numeric(group_rows, "pending_choice_entered"),
            "lagavulin_sleep_preserved_rate": _mean_optional_numeric(
                group_rows, "lagavulin_sleep_preserved"
            ),
        }

    summary = summarize_group(rows)
    summary["by_tag"] = {tag: summarize_group(group_rows) for tag, group_rows in grouped.items()}
    return summary


def evaluate_structured_policy(
    model: StructuredPolicyNet | None,
    *,
    driver_binary: Path | None,
    max_episode_steps: int,
    deterministic: bool,
    device: torch.device,
    random_policy: bool = False,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    rows: list[dict[str, Any]] = []
    for case, spec_path in resolve_benchmark_cases():
        env = StructuredGymCombatEnv([spec_path], driver_binary=driver_binary, max_episode_steps=max_episode_steps, seed=case.seed)
        try:
            obs, info = env.reset(options={"spec_path": spec_path, "seed_hint": case.seed})
            done = False
            truncated = False
            reward_total = 0.0
            steps = 0
            invalid_actions = 0
            decoder_failures = 0
            energy_spent = 0.0
            first_action_label = None
            first_step_metrics: dict[str, Any] | None = None
            while not done and not truncated:
                if random_policy:
                    action = env.sample_random_legal_action()
                else:
                    assert model is not None
                    obs_tensor = to_device_obs({key: value[None, ...] for key, value in obs.items()}, device)
                    with torch.no_grad():
                        actions, _, _, _ = sample_actions(model, obs_tensor, deterministic=deterministic)
                    action = {key: int(value[0].item()) for key, value in actions.items()}
                before = info.get("raw_observation") or {}
                energy_spent += current_energy_spent(before, action)
                obs, reward, done, truncated, info = env.step(action)
                reward_total += float(reward)
                invalid_actions += 1 if info.get("invalid_action") else 0
                decoder_failures += 1 if info.get("decoder_failure") else 0
                if first_action_label is None:
                    first_action_label = info.get("chosen_action_label")
                    first_step_metrics = first_step_signals(before, info.get("raw_observation") or {}, first_action_label)
                steps += 1
            row = {
                "spec_name": case.spec_name,
                "tag": case.tag,
                "seed": case.seed,
                "reward_total": reward_total,
                "steps": steps,
                "invalid_actions": invalid_actions,
                "decoder_failures": decoder_failures,
                "outcome": info.get("outcome"),
                "first_action_label": first_action_label,
                "energy_spent": energy_spent,
            }
            if first_step_metrics is not None:
                row.update(first_step_metrics)
            rows.append(row)
        finally:
            env.close()
    return summarize_benchmark_rows(rows), rows


def evaluate_flat_baseline(
    model_path: Path,
    *,
    driver_binary: Path | None,
    max_episode_steps: int,
    deterministic: bool,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    from sb3_contrib import MaskablePPO

    from gym_combat_env import GymCombatEnv

    model = MaskablePPO.load(str(model_path))
    rows: list[dict[str, Any]] = []
    for case, spec_path in resolve_benchmark_cases():
        env = GymCombatEnv([spec_path], driver_binary=driver_binary, max_episode_steps=max_episode_steps, seed=case.seed)
        try:
            obs, info = env.reset(options={"spec_path": spec_path, "seed_hint": case.seed})
            done = False
            truncated = False
            reward_total = 0.0
            steps = 0
            invalid_actions = 0
            energy_spent = 0.0
            first_action_label = None
            first_step_metrics: dict[str, Any] | None = None
            while not done and not truncated:
                action_masks = env.action_masks()
                action, _ = model.predict(obs, deterministic=deterministic, action_masks=action_masks)
                action_index = int(action)
                before = ((env._last_response or {}).get("payload") or {}).get("observation") or {}  # type: ignore[attr-defined]
                energy_spent += current_energy_spent_flat(env, action_index)
                obs, reward, done, truncated, info = env.step(action_index)
                reward_total += float(reward)
                invalid_actions += 1 if info.get("invalid_action") else 0
                if first_action_label is None:
                    first_action_label = info.get("chosen_action_label")
                    first_step_metrics = first_step_signals(before, ((env._last_response or {}).get("payload") or {}).get("observation") or {}, first_action_label)  # type: ignore[attr-defined]
                steps += 1
            row = {
                "spec_name": case.spec_name,
                "tag": case.tag,
                "seed": case.seed,
                "reward_total": reward_total,
                "steps": steps,
                "invalid_actions": invalid_actions,
                "decoder_failures": 0,
                "outcome": info.get("outcome"),
                "first_action_label": first_action_label,
                "energy_spent": energy_spent,
            }
            if first_step_metrics is not None:
                row.update(first_step_metrics)
            rows.append(row)
        finally:
            env.close()
    return summarize_benchmark_rows(rows), rows


def main() -> None:
    parser = argparse.ArgumentParser(description="Train a structured multi-head PPO combat policy.")
    parser.add_argument("--spec-dir", default=REPO_ROOT / "data" / "combat_lab" / "specs", type=Path)
    parser.add_argument("--driver-binary", default=None, type=Path)
    parser.add_argument("--timesteps", default=2048, type=int)
    parser.add_argument("--n-envs", default=4, type=int)
    parser.add_argument("--rollout-steps", default=64, type=int)
    parser.add_argument("--epochs", default=4, type=int)
    parser.add_argument("--minibatch-size", default=128, type=int)
    parser.add_argument("--seed", default=7, type=int)
    parser.add_argument("--max-episode-steps", default=64, type=int)
    parser.add_argument("--learning-rate", default=3e-4, type=float)
    parser.add_argument("--gamma", default=0.97, type=float)
    parser.add_argument("--gae-lambda", default=0.95, type=float)
    parser.add_argument("--clip-eps", default=0.2, type=float)
    parser.add_argument("--ent-coef", default=0.01, type=float)
    parser.add_argument("--vf-coef", default=0.5, type=float)
    parser.add_argument("--probe-coef", default=0.1, type=float)
    parser.add_argument("--output-prefix", default="", help="Optional output prefix under tools/artifacts/learning_dataset.")
    parser.add_argument("--model-out", default=None, type=Path)
    parser.add_argument("--metrics-out", default=None, type=Path)
    parser.add_argument("--episodes-out", default=None, type=Path)
    parser.add_argument("--flat-baseline-model", default=None, type=Path)
    args = parser.parse_args()

    np.random.seed(args.seed)
    torch.manual_seed(args.seed)
    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")

    spec_paths = discover_spec_paths(args.spec_dir)
    if not spec_paths:
        raise SystemExit(f"no combat specs found under {args.spec_dir}")

    dataset_dir = REPO_ROOT / "tools" / "artifacts" / "learning_dataset"
    prefix = str(args.output_prefix or "").strip()
    model_out = args.model_out or (dataset_dir / (f"{prefix}_structured_combat_ppo_model.pt" if prefix else "structured_combat_ppo_model.pt"))
    metrics_out = args.metrics_out or (dataset_dir / (f"{prefix}_structured_combat_ppo_metrics.json" if prefix else "structured_combat_ppo_metrics.json"))
    episodes_out = args.episodes_out or (dataset_dir / (f"{prefix}_structured_combat_ppo_eval_rows.jsonl" if prefix else "structured_combat_ppo_eval_rows.jsonl"))

    envs = [
        StructuredGymCombatEnv(
            spec_paths,
            driver_binary=args.driver_binary,
            max_episode_steps=args.max_episode_steps,
            seed=args.seed + env_index,
        )
        for env_index in range(args.n_envs)
    ]
    try:
        obs_list = []
        info_list = []
        for env_index, env in enumerate(envs):
            obs, info = env.reset(options={"seed_hint": args.seed + env_index})
            obs_list.append(obs)
            info_list.append(info)

        model = StructuredPolicyNet(
            card_vocab=max(len(CARD_ID_VOCAB), 1),
            potion_vocab=max(len(POTION_ID_VOCAB), 1),
            power_vocab=max(len(POWER_ID_VOCAB), 1),
            monster_vocab=max(len(MONSTER_ID_VOCAB), 1),
            intent_vocab=max(len(INTENT_KIND_IDS), 1),
        ).to(device)
        optimizer = torch.optim.AdamW(model.parameters(), lr=args.learning_rate)

        updates = max(args.timesteps // max(args.rollout_steps * args.n_envs, 1), 1)
        total_steps = 0
        for _ in range(updates):
            obs_steps: list[dict[str, np.ndarray]] = []
            action_steps: list[dict[str, np.ndarray]] = []
            logprob_steps: list[np.ndarray] = []
            value_steps: list[np.ndarray] = []
            reward_steps: list[np.ndarray] = []
            done_steps: list[np.ndarray] = []
            probe_steps: list[np.ndarray] = []

            for _ in range(args.rollout_steps):
                batch_obs_np = stack_obs_np(obs_list)
                batch_obs = to_device_obs(batch_obs_np, device)
                with torch.no_grad():
                    actions_t, logprob_t, value_t, _ = sample_actions(model, batch_obs, deterministic=False)
                step_rewards = np.zeros((args.n_envs,), dtype=np.float32)
                step_dones = np.zeros((args.n_envs,), dtype=np.float32)
                probe_targets = np.zeros((args.n_envs, 4), dtype=np.float32)
                next_obs_list: list[dict[str, np.ndarray]] = []
                next_info_list: list[dict[str, Any]] = []
                action_np = {key: value.detach().cpu().numpy().astype(np.int64) for key, value in actions_t.items()}
                for env_index, env in enumerate(envs):
                    action = {key: int(value[env_index]) for key, value in action_np.items()}
                    next_obs, reward, terminated, truncated, next_info = env.step(action)
                    step_rewards[env_index] = float(reward)
                    step_dones[env_index] = 1.0 if (terminated or truncated) else 0.0
                    raw_probe_targets = next_info.get("probe_targets")
                    if raw_probe_targets is None:
                        probe_targets[env_index] = np.zeros((4,), dtype=np.float32)
                    else:
                        probe_targets[env_index] = np.asarray(raw_probe_targets, dtype=np.float32)
                    if terminated or truncated:
                        next_obs, next_info = env.reset(options={"seed_hint": args.seed + total_steps + env_index + 1})
                    next_obs_list.append(next_obs)
                    next_info_list.append(next_info)
                obs_steps.append(batch_obs_np)
                action_steps.append(action_np)
                logprob_steps.append(logprob_t.detach().cpu().numpy().astype(np.float32))
                value_steps.append(value_t.detach().cpu().numpy().astype(np.float32))
                reward_steps.append(step_rewards)
                done_steps.append(step_dones)
                probe_steps.append(probe_targets)
                obs_list = next_obs_list
                info_list = next_info_list
                total_steps += args.n_envs
                if total_steps >= args.timesteps:
                    break

            last_obs = to_device_obs(stack_obs_np(obs_list), device)
            with torch.no_grad():
                last_state = model.encode(last_obs)
                last_values = last_state.value.detach().cpu().numpy().astype(np.float32)

            rewards = np.stack(reward_steps, axis=0)
            dones = np.stack(done_steps, axis=0)
            values = np.stack(value_steps, axis=0)
            old_logprobs = np.stack(logprob_steps, axis=0)
            probe_targets = np.stack(probe_steps, axis=0)
            advantages, returns = compute_gae(rewards, dones, values, last_values, args.gamma, args.gae_lambda)
            advantages = (advantages - advantages.mean()) / max(advantages.std(), 1e-6)

            flat_obs = flatten_obs_time(obs_steps)
            flat_actions = {key: np.concatenate([step[key] for step in action_steps], axis=0) for key in action_steps[0].keys()}
            flat_old_logprobs = old_logprobs.reshape(-1)
            flat_returns = returns.reshape(-1)
            flat_advantages = advantages.reshape(-1)
            flat_probe_targets = probe_targets.reshape(-1, 4)

            count = flat_old_logprobs.shape[0]
            indices = np.arange(count)
            for _ in range(args.epochs):
                np.random.shuffle(indices)
                for start in range(0, count, args.minibatch_size):
                    batch_index = indices[start : start + args.minibatch_size]
                    obs_mb = to_device_obs(index_obs(flat_obs, batch_index), device)
                    actions_mb = {key: torch.as_tensor(value[batch_index], device=device).long() for key, value in flat_actions.items()}
                    old_logprob_mb = torch.as_tensor(flat_old_logprobs[batch_index], device=device, dtype=torch.float32)
                    returns_mb = torch.as_tensor(flat_returns[batch_index], device=device, dtype=torch.float32)
                    advantages_mb = torch.as_tensor(flat_advantages[batch_index], device=device, dtype=torch.float32)
                    probe_targets_mb = torch.as_tensor(flat_probe_targets[batch_index], device=device, dtype=torch.float32)

                    new_logprob, entropy, value_pred, probe_logits = evaluate_actions(model, obs_mb, actions_mb)
                    ratio = (new_logprob - old_logprob_mb).exp()
                    clipped_ratio = ratio.clamp(1.0 - args.clip_eps, 1.0 + args.clip_eps)
                    actor_loss = -torch.min(ratio * advantages_mb, clipped_ratio * advantages_mb).mean()
                    value_loss = nn.functional.mse_loss(value_pred, returns_mb)
                    probe_loss = nn.functional.binary_cross_entropy_with_logits(probe_logits, probe_targets_mb)
                    entropy_bonus = entropy.mean()
                    loss = actor_loss + args.vf_coef * value_loss + args.probe_coef * probe_loss - args.ent_coef * entropy_bonus

                    optimizer.zero_grad()
                    loss.backward()
                    nn.utils.clip_grad_norm_(model.parameters(), 1.0)
                    optimizer.step()

                if total_steps >= args.timesteps:
                    break

        torch.save(
            {
                "model_state": model.state_dict(),
                "config": {
                    "card_vocab": max(len(CARD_ID_VOCAB), 1),
                    "potion_vocab": max(len(POTION_ID_VOCAB), 1),
                    "power_vocab": max(len(POWER_ID_VOCAB), 1),
                    "monster_vocab": max(len(MONSTER_ID_VOCAB), 1),
                    "intent_vocab": max(len(INTENT_KIND_IDS), 1),
                },
            },
            model_out,
        )

        eval_metrics, eval_rows = evaluate_structured_policy(
            model,
            driver_binary=args.driver_binary,
            max_episode_steps=args.max_episode_steps,
            deterministic=True,
            device=device,
            random_policy=False,
        )
        random_metrics, random_rows = evaluate_structured_policy(
            None,
            driver_binary=args.driver_binary,
            max_episode_steps=args.max_episode_steps,
            deterministic=True,
            device=device,
            random_policy=True,
        )
        flat_metrics = None
        flat_rows: list[dict[str, Any]] = []
        if args.flat_baseline_model is not None:
            flat_metrics, flat_rows = evaluate_flat_baseline(
                args.flat_baseline_model,
                driver_binary=args.driver_binary,
                max_episode_steps=args.max_episode_steps,
                deterministic=True,
            )
        metrics = {
            "model": "structured_multi_head_ppo",
            "timesteps": int(args.timesteps),
            "n_envs": int(args.n_envs),
            "rollout_steps": int(args.rollout_steps),
            "epochs": int(args.epochs),
            "seed": int(args.seed),
            "max_episode_steps": int(args.max_episode_steps),
            "spec_count": len(spec_paths),
            "driver_binary": str(args.driver_binary) if args.driver_binary else None,
            "eval": eval_metrics,
            "random_benchmark": random_metrics,
            "flat_baseline_benchmark": flat_metrics,
            "notes": [
                "structured observation/action contract over combat_env_driver",
                "custom multi-head PPO with latent tactical bottleneck and probe heads",
                "pressure/belief diagnostics excluded from primary policy tensors",
            ],
        }
        write_json(metrics_out, metrics)
        write_jsonl(
            episodes_out,
            [{"source": "structured_eval", **row} for row in eval_rows]
            + [{"source": "random_eval", **row} for row in random_rows]
            + [{"source": "flat_baseline_eval", **row} for row in flat_rows],
        )
        print(json.dumps(metrics, indent=2, ensure_ascii=False))
        print(f"wrote structured PPO metrics to {metrics_out}")
    finally:
        for env in envs:
            env.close()


if __name__ == "__main__":
    main()
