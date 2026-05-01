#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
from typing import Any

import numpy as np

from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from structured_combat_env import StructuredGymCombatEnv
from train_structured_combat_ppo import load_start_spec_name, parse_seed_list, stack_obs_np


def legal_candidates(info: dict[str, Any]) -> list[dict[str, Any]]:
    return [candidate for candidate in info.get("action_candidates") or [] if bool(candidate.get("legal"))]


def action_family(action: dict[str, int]) -> str:
    action_type = int(action.get("action_type") or 0)
    return {
        0: "end_turn",
        1: "play_card",
        2: "use_potion",
        3: "choice",
        4: "proceed",
        5: "cancel",
    }.get(action_type, "unknown")


def margin(value: float | None) -> float:
    if value is None or not math.isfinite(float(value)):
        return -1e6
    return float(value)


def transition_teacher_score(
    *,
    action: dict[str, int],
    reward: float,
    done: bool,
    info: dict[str, Any],
) -> tuple[float, dict[str, Any]]:
    raw_after = info.get("raw_observation") or {}
    pressure_after = raw_after.get("pressure") or {}
    breakdown = info.get("reward_breakdown") or {}
    outcome = str(info.get("outcome") or "")
    family = action_family(action)

    after_hp = float(raw_after.get("player_hp") or 0.0)
    after_visible_unblocked = float(pressure_after.get("visible_unblocked") or 0.0)
    next_window_loss = float(breakdown.get("next_enemy_window_hp_loss_after_action") or 0.0)
    enemy_hp_delta = float(breakdown.get("enemy_hp_delta") or 0.0)
    player_hp_delta = float(breakdown.get("player_hp_delta") or 0.0)
    kill_bonus = float(breakdown.get("kill_bonus") or 0.0)
    incoming_relief = float(breakdown.get("incoming_relief") or 0.0)
    next_window_relief = float(breakdown.get("next_enemy_window_relief") or 0.0)
    persistent_relief = float(breakdown.get("persistent_attack_script_relief") or 0.0)

    if outcome == "victory":
        survival_rank = 4
        survival_margin = 100.0
    elif done and outcome == "defeat":
        survival_rank = 0
        survival_margin = -100.0
    elif family == "end_turn":
        survival_rank = 2
        survival_margin = after_hp
    else:
        window_margin = after_hp - next_window_loss
        visible_margin = after_hp - after_visible_unblocked
        survival_margin = min(window_margin, visible_margin)
        survival_rank = 3 if survival_margin > 0.0 else 1

    score = (
        survival_rank * 10000.0
        + margin(survival_margin) * 40.0
        + float(reward) * 25.0
        + enemy_hp_delta * 4.0
        + kill_bonus * 75.0
        + incoming_relief * 8.0
        + next_window_relief * 3.0
        + persistent_relief * 0.25
        + player_hp_delta * 8.0
    )
    return score, {
        "score": score,
        "survival_rank": survival_rank,
        "survival_margin": survival_margin,
        "outcome": outcome,
        "family": family,
        "reward": float(reward),
        "enemy_hp_delta": enemy_hp_delta,
        "player_hp_delta": player_hp_delta,
        "kill_bonus": kill_bonus,
        "incoming_relief": incoming_relief,
        "next_window_relief": next_window_relief,
        "persistent_relief": persistent_relief,
    }


def make_env(
    *,
    spec_paths: list[Path],
    spec_source: str,
    driver_binary: Path | None,
    max_episode_steps: int,
    draw_order_variant: str,
    reward_mode: str,
    reward_config: dict[str, float],
    seed: int,
) -> StructuredGymCombatEnv:
    return StructuredGymCombatEnv(
        spec_paths,
        spec_source=spec_source,
        driver_binary=driver_binary,
        max_episode_steps=max_episode_steps,
        seed=seed,
        draw_order_variant=draw_order_variant,
        reward_mode=reward_mode,
        reward_config=reward_config,
    )


def replay_prefix(
    env: StructuredGymCombatEnv,
    *,
    spec_path: Path,
    seed_hint: int,
    prefix_actions: list[dict[str, int]],
) -> tuple[dict[str, np.ndarray], dict[str, Any], bool]:
    obs, info = env.reset(options={"spec_path": spec_path, "seed_hint": seed_hint})
    done = False
    truncated = False
    for action in prefix_actions:
        obs, _, done, truncated, info = env.step(action)
        if info.get("invalid_action") or done or truncated:
            return obs, info, False
    return obs, info, True


def choose_teacher_action(
    *,
    spec_path: Path,
    seed_hint: int,
    prefix_actions: list[dict[str, int]],
    candidates: list[dict[str, Any]],
    main_env: StructuredGymCombatEnv,
    env_args: dict[str, Any],
) -> tuple[dict[str, int] | None, dict[str, Any]]:
    scored: list[dict[str, Any]] = []
    for candidate in candidates:
        action = main_env.candidate_to_canonical(candidate)
        probe = make_env(**env_args)
        try:
            _, _, replay_ok = replay_prefix(
                probe,
                spec_path=spec_path,
                seed_hint=seed_hint,
                prefix_actions=prefix_actions,
            )
            if not replay_ok:
                scored.append(
                    {
                        "candidate_label": candidate.get("label"),
                        "score": -1e9,
                        "replay_ok": False,
                    }
                )
                continue
            _, reward, done, truncated, info = probe.step(action)
            if info.get("invalid_action") or info.get("decoder_failure"):
                scored.append(
                    {
                        "candidate_label": candidate.get("label"),
                        "score": -1e9,
                        "replay_ok": True,
                        "invalid": True,
                    }
                )
                continue
            score, details = transition_teacher_score(
                action=action,
                reward=float(reward),
                done=bool(done or truncated),
                info=info,
            )
            scored.append(
                {
                    "candidate_label": candidate.get("label"),
                    "action": action,
                    "replay_ok": True,
                    **details,
                }
            )
        finally:
            probe.close()

    valid = [row for row in scored if row.get("action") is not None]
    if not valid:
        return None, {"candidates": scored}
    valid.sort(key=lambda row: float(row.get("score") or -1e9), reverse=True)
    best = valid[0]
    gap = float(best.get("score") or 0.0) - float(valid[1].get("score") or 0.0) if len(valid) > 1 else 1e9
    return dict(best["action"]), {
        "best": best,
        "gap": gap,
        "candidates": scored,
    }


def append_sample(
    obs_samples: dict[str, list[np.ndarray]],
    action_samples: dict[str, list[int]],
    obs: dict[str, np.ndarray],
    action: dict[str, int],
) -> None:
    for key, value in obs.items():
        obs_samples.setdefault(key, []).append(np.asarray(value))
    for key in ("action_type", "card_slot", "target_slot", "potion_slot", "choice_index"):
        action_samples.setdefault(key, []).append(int(action.get(key) or 0))


def write_npz_dataset(
    path: Path,
    obs_samples: dict[str, list[np.ndarray]],
    action_samples: dict[str, list[int]],
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload: dict[str, np.ndarray] = {}
    for key, values in obs_samples.items():
        payload[f"obs__{key}"] = np.stack(values, axis=0)
    for key, values in action_samples.items():
        payload[f"action__{key}"] = np.asarray(values, dtype=np.int64)
    np.savez_compressed(path, **payload)


def main() -> None:
    parser = argparse.ArgumentParser(description="Build a structured combat BC teacher dataset from one-step branch labels.")
    parser.add_argument("--spec-source", choices=["start_spec"], default="start_spec")
    parser.add_argument("--start-spec", action="append", required=True, type=Path)
    parser.add_argument("--seeds", default="2009,2010,2011,2012")
    parser.add_argument("--samples", default=128, type=int)
    parser.add_argument("--max-episode-steps", default=96, type=int)
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
    parser.add_argument("--out", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "structured_bc_teacher_dataset.npz", type=Path)
    parser.add_argument("--rows-out", default=None, type=Path)
    parser.add_argument("--summary-out", default=None, type=Path)
    args = parser.parse_args()

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

    obs_samples: dict[str, list[np.ndarray]] = {}
    action_samples: dict[str, list[int]] = {}
    rows: list[dict[str, Any]] = []
    episodes_started = 0
    candidate_evals = 0
    main_env = make_env(**env_args)
    try:
        for spec_path in spec_paths:
            for seed_hint in seeds:
                if len(next(iter(obs_samples.values()), [])) >= args.samples:
                    break
                episodes_started += 1
                obs, info = main_env.reset(options={"spec_path": spec_path, "seed_hint": seed_hint})
                prefix_actions: list[dict[str, int]] = []
                done = False
                truncated = False
                step_index = 0
                while not done and not truncated and step_index < args.max_episode_steps:
                    if len(next(iter(obs_samples.values()), [])) >= args.samples:
                        break
                    candidates = legal_candidates(info)
                    if not candidates:
                        break
                    action, audit = choose_teacher_action(
                        spec_path=spec_path,
                        seed_hint=seed_hint,
                        prefix_actions=prefix_actions,
                        candidates=candidates,
                        main_env=main_env,
                        env_args=env_args,
                    )
                    candidate_evals += len(candidates)
                    if action is None:
                        break
                    append_sample(obs_samples, action_samples, obs, action)
                    best = audit.get("best") or {}
                    rows.append(
                        {
                            "sample_index": len(rows),
                            "spec_path": str(spec_path.relative_to(REPO_ROOT) if spec_path.is_relative_to(REPO_ROOT) else spec_path),
                            "spec_name": load_start_spec_name(spec_path),
                            "seed": seed_hint,
                            "step_index": step_index,
                            "prefix_len": len(prefix_actions),
                            "teacher_label": best.get("candidate_label"),
                            "teacher_score": best.get("score"),
                            "teacher_survival_rank": best.get("survival_rank"),
                            "teacher_gap": audit.get("gap"),
                            "legal_action_count": len(candidates),
                            "action": action,
                        }
                    )
                    obs, _, done, truncated, info = main_env.step(action)
                    if info.get("invalid_action") or info.get("decoder_failure"):
                        break
                    prefix_actions.append(action)
                    step_index += 1
            if len(next(iter(obs_samples.values()), [])) >= args.samples:
                break
    finally:
        main_env.close()

    sample_count = len(next(iter(obs_samples.values()), []))
    if sample_count == 0:
        raise SystemExit("teacher dataset generation produced no samples")
    write_npz_dataset(args.out, obs_samples, action_samples)
    rows_out = args.rows_out or args.out.with_suffix(".jsonl")
    summary_out = args.summary_out or args.out.with_suffix(".summary.json")
    write_jsonl(rows_out, rows)
    summary = {
        "dataset": str(args.out),
        "rows": str(rows_out),
        "sample_count": sample_count,
        "episodes_started": episodes_started,
        "candidate_evals": candidate_evals,
        "specs": [str(path) for path in spec_paths],
        "seeds": seeds,
        "draw_order_variant": args.draw_order_variant,
        "reward_mode": args.reward_mode,
        "reward": reward_config,
        "teacher": {
            "kind": "one_step_branch_score",
            "notes": [
                "branch candidates are evaluated by resetting the same start spec and replaying the prefix",
                "survival rank dominates shaped reward and damage terms",
                "labels are warmup priors, not engine truth",
            ],
        },
    }
    write_json(summary_out, summary)
    print(json.dumps(summary, indent=2, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
