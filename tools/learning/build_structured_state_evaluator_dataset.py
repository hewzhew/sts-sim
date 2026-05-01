#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import random
from pathlib import Path
from typing import Any

import numpy as np

from build_structured_bc_teacher_dataset import (
    choose_teacher_action,
    legal_candidates,
    make_env,
    replay_prefix,
)
from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from train_structured_combat_ppo import load_start_spec_name, parse_seed_list

REGRESSION_TARGETS = [
    "discounted_return",
    "hp_delta",
    "enemy_hp_delta",
    "final_visible_unblocked",
    "final_player_hp",
    "final_enemy_hp",
]

BINARY_TARGETS = [
    "survived_horizon",
    "terminal_victory",
    "terminal_defeat",
]

CONTINUATION_POLICY_CHOICES = ["greedy_transition"]


def judge_protocol(continuation_policy: str, horizon: int) -> str:
    if continuation_policy == "greedy_transition":
        return f"greedy_transition_h{int(horizon)}"
    raise ValueError(f"unsupported continuation policy: {continuation_policy}")


def normalize_state_policy(state_policy: str) -> str:
    return "greedy_transition" if state_policy == "teacher" else state_policy


def sample_count(samples: dict[str, list[np.ndarray]]) -> int:
    return len(next(iter(samples.values()), []))


def append_sample(
    obs_samples: dict[str, list[np.ndarray]],
    target_samples: dict[str, list[float]],
    obs: dict[str, np.ndarray],
    targets: dict[str, float],
) -> None:
    for key, value in obs.items():
        obs_samples.setdefault(key, []).append(np.asarray(value))
    for key in REGRESSION_TARGETS + BINARY_TARGETS + ["steps_observed"]:
        target_samples.setdefault(key, []).append(float(targets.get(key, 0.0)))


def write_npz_dataset(
    path: Path,
    obs_samples: dict[str, list[np.ndarray]],
    target_samples: dict[str, list[float]],
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload: dict[str, np.ndarray] = {}
    for key, values in obs_samples.items():
        payload[f"obs__{key}"] = np.stack(values, axis=0)
    for key, values in target_samples.items():
        payload[f"target__{key}"] = np.asarray(values, dtype=np.float32)
    np.savez_compressed(path, **payload)


def total_living_monster_hp(raw_observation: dict[str, Any]) -> float:
    total = 0.0
    for monster in raw_observation.get("monsters") or []:
        if monster.get("alive", True):
            total += float(monster.get("current_hp") or 0.0)
    return total


def player_hp(raw_observation: dict[str, Any]) -> float:
    return float(raw_observation.get("player_hp") or 0.0)


def visible_unblocked(raw_observation: dict[str, Any]) -> float:
    pressure = raw_observation.get("pressure") or {}
    return float(pressure.get("visible_unblocked") or 0.0)


def discounted_sum(rewards: list[float], gamma: float) -> float:
    total = 0.0
    weight = 1.0
    for reward in rewards:
        total += weight * float(reward)
        weight *= float(gamma)
    return total


def teacher_rollout_label(
    *,
    spec_path: Path,
    seed_hint: int,
    prefix_actions: list[dict[str, int]],
    env_args: dict[str, Any],
    horizon: int,
    gamma: float,
    continuation_policy: str = "greedy_transition",
) -> tuple[dict[str, float] | None, dict[str, Any]]:
    if continuation_policy not in CONTINUATION_POLICY_CHOICES:
        raise ValueError(f"unsupported continuation policy: {continuation_policy}")
    probe = make_env(**env_args)
    try:
        _, info, replay_ok = replay_prefix(
            probe,
            spec_path=spec_path,
            seed_hint=seed_hint,
            prefix_actions=prefix_actions,
        )
        if not replay_ok:
            return None, {"replay_ok": False}

        raw_start = info.get("raw_observation") or {}
        start_hp = player_hp(raw_start)
        start_enemy_hp = total_living_monster_hp(raw_start)
        rollout_prefix = list(prefix_actions)
        rewards: list[float] = []
        rollout_actions: list[dict[str, Any]] = []
        done = bool(info.get("outcome") in {"victory", "defeat"})
        truncated = False

        for step_index in range(int(horizon)):
            if done or truncated:
                break
            candidates = legal_candidates(info)
            if not candidates:
                break
            action, audit = choose_teacher_action(
                spec_path=spec_path,
                seed_hint=seed_hint,
                prefix_actions=rollout_prefix,
                candidates=candidates,
                main_env=probe,
                env_args=env_args,
            )
            if action is None:
                break
            _, reward, done, truncated, info = probe.step(action)
            rewards.append(float(reward))
            rollout_actions.append(
                {
                    "step": step_index,
                    "action": action,
                    "label": info.get("chosen_action_label"),
                    "reward": float(reward),
                    "outcome": info.get("outcome"),
                    "source": f"{continuation_policy}_continuation",
                    "judge_gap": (audit or {}).get("gap"),
                    "greedy_transition_gap": (audit or {}).get("gap"),
                }
            )
            if info.get("invalid_action") or info.get("decoder_failure"):
                break
            rollout_prefix.append(action)

        raw_final = info.get("raw_observation") or {}
        outcome = str(info.get("outcome") or ("truncated" if truncated else "ongoing"))
        final_hp = player_hp(raw_final)
        final_enemy_hp = total_living_monster_hp(raw_final)
        targets = {
            "discounted_return": discounted_sum(rewards, gamma),
            "hp_delta": final_hp - start_hp,
            "enemy_hp_delta": start_enemy_hp - final_enemy_hp,
            "final_visible_unblocked": visible_unblocked(raw_final),
            "final_player_hp": final_hp,
            "final_enemy_hp": final_enemy_hp,
            "survived_horizon": 0.0 if outcome == "defeat" else 1.0,
            "terminal_victory": 1.0 if outcome == "victory" else 0.0,
            "terminal_defeat": 1.0 if outcome == "defeat" else 0.0,
            "steps_observed": float(len(rewards)),
        }
        audit = {
            "replay_ok": True,
            "horizon": int(horizon),
            "gamma": float(gamma),
            "label_mode": "fixed_seed_replay",
            "continuation_policy": continuation_policy,
            "judge_protocol": judge_protocol(continuation_policy, horizon),
            "outcome": outcome,
            "rollout_actions": rollout_actions,
            "start": {
                "player_hp": start_hp,
                "enemy_hp": start_enemy_hp,
                "visible_unblocked": visible_unblocked(raw_start),
            },
            "final": {
                "player_hp": final_hp,
                "enemy_hp": final_enemy_hp,
                "visible_unblocked": visible_unblocked(raw_final),
            },
        }
        return targets, audit
    finally:
        probe.close()


def choose_collection_action(
    *,
    state_policy: str,
    rng: random.Random,
    mixed_random_rate: float,
    spec_path: Path,
    seed_hint: int,
    prefix_actions: list[dict[str, int]],
    candidates: list[dict[str, Any]],
    main_env: Any,
    env_args: dict[str, Any],
) -> tuple[dict[str, int] | None, str]:
    normalized_policy = normalize_state_policy(state_policy)
    use_random = normalized_policy == "random" or (
        normalized_policy == "mixed" and rng.random() < float(mixed_random_rate)
    )
    if use_random:
        return main_env.sample_random_legal_action(), "random"
    action, _ = choose_teacher_action(
        spec_path=spec_path,
        seed_hint=seed_hint,
        prefix_actions=prefix_actions,
        candidates=candidates,
        main_env=main_env,
        env_args=env_args,
    )
    return action, "greedy_transition"


def main() -> None:
    parser = argparse.ArgumentParser(description="Build a structured combat state evaluator dataset.")
    parser.add_argument("--spec-source", choices=["start_spec"], default="start_spec")
    parser.add_argument("--start-spec", action="append", required=True, type=Path)
    parser.add_argument("--seeds", default="2009,2010,2011,2012")
    parser.add_argument("--samples", default=128, type=int)
    parser.add_argument("--max-episode-steps", default=96, type=int)
    parser.add_argument("--label-horizon", default=8, type=int)
    parser.add_argument("--gamma", default=0.97, type=float)
    parser.add_argument("--continuation-policy", choices=CONTINUATION_POLICY_CHOICES, default="greedy_transition")
    parser.add_argument("--state-policy", choices=["teacher", "greedy_transition", "random", "mixed"], default="mixed")
    parser.add_argument("--mixed-random-rate", default=0.25, type=float)
    parser.add_argument("--draw-order-variant", choices=["exact", "reshuffle_draw"], default="reshuffle_draw")
    parser.add_argument("--reward-mode", choices=["legacy", "minimal_rl"], default="minimal_rl")
    parser.add_argument("--victory-reward", default=1.0, type=float)
    parser.add_argument("--defeat-reward", default=-1.0, type=float)
    parser.add_argument("--hp-loss-scale", default=0.02, type=float)
    parser.add_argument("--enemy-hp-delta-scale", default=0.01, type=float)
    parser.add_argument("--kill-bonus-scale", default=0.0, type=float)
    parser.add_argument("--catastrophe-unblocked-threshold", default=18.0, type=float)
    parser.add_argument("--catastrophe-penalty", default=0.25, type=float)
    parser.add_argument("--next-enemy-window-relief-scale", default=0.0, type=float)
    parser.add_argument("--persistent-attack-script-relief-scale", default=0.0, type=float)
    parser.add_argument("--driver-binary", default=None, type=Path)
    parser.add_argument("--rng-seed", default=7, type=int)
    parser.add_argument(
        "--out",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "structured_state_evaluator_dataset.npz",
        type=Path,
    )
    parser.add_argument("--rows-out", default=None, type=Path)
    parser.add_argument("--summary-out", default=None, type=Path)
    args = parser.parse_args()
    args.state_policy = normalize_state_policy(args.state_policy)

    spec_paths = [Path(path) for path in args.start_spec]
    seeds = parse_seed_list(args.seeds)
    reward_config = {
        "victory_reward": float(args.victory_reward),
        "defeat_reward": float(args.defeat_reward),
        "hp_loss_scale": float(args.hp_loss_scale),
        "enemy_hp_delta_scale": float(args.enemy_hp_delta_scale),
        "kill_bonus_scale": float(args.kill_bonus_scale),
        "catastrophe_unblocked_threshold": float(args.catastrophe_unblocked_threshold),
        "catastrophe_penalty": float(args.catastrophe_penalty),
        "next_enemy_window_relief_scale": float(args.next_enemy_window_relief_scale),
        "persistent_attack_script_relief_scale": float(args.persistent_attack_script_relief_scale),
    }
    env_args = {
        "spec_paths": spec_paths,
        "spec_source": args.spec_source,
        "driver_binary": args.driver_binary,
        "max_episode_steps": args.max_episode_steps,
        "draw_order_variant": args.draw_order_variant,
        "reward_mode": args.reward_mode,
        "reward_config": reward_config,
        "seed": 0,
    }

    rng = random.Random(int(args.rng_seed))
    obs_samples: dict[str, list[np.ndarray]] = {}
    target_samples: dict[str, list[float]] = {}
    rows: list[dict[str, Any]] = []
    episodes_started = 0
    collection_policy_counts = {"greedy_transition": 0, "random": 0}
    main_env = make_env(**env_args)
    try:
        for spec_path in spec_paths:
            for seed_hint in seeds:
                if sample_count(obs_samples) >= args.samples:
                    break
                episodes_started += 1
                obs, info = main_env.reset(options={"spec_path": spec_path, "seed_hint": seed_hint})
                prefix_actions: list[dict[str, int]] = []
                done = False
                truncated = False
                step_index = 0
                while not done and not truncated and step_index < args.max_episode_steps:
                    if sample_count(obs_samples) >= args.samples:
                        break
                    candidates = legal_candidates(info)
                    if not candidates:
                        break
                    targets, rollout_audit = teacher_rollout_label(
                        spec_path=spec_path,
                        seed_hint=seed_hint,
                        prefix_actions=prefix_actions,
                        env_args=env_args,
                        horizon=args.label_horizon,
                        gamma=args.gamma,
                        continuation_policy=args.continuation_policy,
                    )
                    if targets is None:
                        break
                    append_sample(obs_samples, target_samples, obs, targets)
                    rows.append(
                        {
                            "sample_index": len(rows),
                            "spec_path": str(spec_path.relative_to(REPO_ROOT) if spec_path.is_relative_to(REPO_ROOT) else spec_path),
                            "spec_name": load_start_spec_name(spec_path),
                            "seed": int(seed_hint),
                            "step_index": int(step_index),
                            "prefix_len": len(prefix_actions),
                            "legal_action_count": len(candidates),
                            "state_policy": args.state_policy,
                            "targets": targets,
                            "rollout": rollout_audit,
                        }
                    )

                    action, source = choose_collection_action(
                        state_policy=args.state_policy,
                        rng=rng,
                        mixed_random_rate=args.mixed_random_rate,
                        spec_path=spec_path,
                        seed_hint=seed_hint,
                        prefix_actions=prefix_actions,
                        candidates=candidates,
                        main_env=main_env,
                        env_args=env_args,
                    )
                    if action is None:
                        break
                    collection_policy_counts[source] = collection_policy_counts.get(source, 0) + 1
                    obs, _, done, truncated, info = main_env.step(action)
                    if info.get("invalid_action") or info.get("decoder_failure"):
                        break
                    prefix_actions.append(action)
                    step_index += 1
            if sample_count(obs_samples) >= args.samples:
                break
    finally:
        main_env.close()

    count = sample_count(obs_samples)
    if count == 0:
        raise SystemExit("state evaluator dataset generation produced no samples")
    write_npz_dataset(args.out, obs_samples, target_samples)
    rows_out = args.rows_out or args.out.with_suffix(".jsonl")
    summary_out = args.summary_out or args.out.with_suffix(".summary.json")
    write_jsonl(rows_out, rows)
    summary = {
        "dataset": str(args.out),
        "rows": str(rows_out),
        "sample_count": count,
        "episodes_started": episodes_started,
        "specs": [str(path) for path in spec_paths],
        "seeds": seeds,
        "state_policy": args.state_policy,
        "collection_policy_counts": collection_policy_counts,
        "label_mode": "fixed_seed_replay",
        "continuation_policy": args.continuation_policy,
        "judge_protocol": judge_protocol(args.continuation_policy, int(args.label_horizon)),
        "label_policy": f"{args.continuation_policy}_continuation",
        "label_horizon": int(args.label_horizon),
        "gamma": float(args.gamma),
        "draw_order_variant": args.draw_order_variant,
        "reward_mode": args.reward_mode,
        "reward": reward_config,
        "regression_targets": REGRESSION_TARGETS,
        "binary_targets": BINARY_TARGETS,
        "notes": [
            "samples are structured observations labelled by short greedy-transition continuation outcomes",
            "targets are state values, not direct action labels",
            "mixed state-policy intentionally visits some off-greedy states while labels still use greedy-transition continuation",
        ],
    }
    write_json(summary_out, summary)
    print(json.dumps(summary, indent=2, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
