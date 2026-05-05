#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import random
from pathlib import Path
from typing import Any

import numpy as np

from build_structured_bc_teacher_dataset import choose_teacher_action, legal_candidates, make_env, replay_prefix
from build_structured_state_evaluator_dataset import (
    BINARY_TARGETS,
    REGRESSION_TARGETS,
    choose_collection_action,
    discounted_sum,
    normalize_state_policy,
    player_hp,
    total_living_monster_hp,
    visible_unblocked,
)
from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from structured_candidate_ranker_common import (
    ACTION_CLASS_IDS,
    MAX_RANKER_CANDIDATES,
    candidate_action_array,
    candidate_action_class,
    candidate_feature_vector,
)
from train_structured_combat_ppo import load_start_spec_name, parse_seed_list

CANDIDATE_REGRESSION_TARGETS = REGRESSION_TARGETS + [
    "immediate_reward",
    "root_hp_delta",
    "root_enemy_hp_delta",
    "root_visible_unblocked",
]

CANDIDATE_BINARY_TARGETS = BINARY_TARGETS + [
    "root_terminal_victory",
    "root_terminal_defeat",
]

CONTINUATION_POLICY_CHOICES = ["greedy_transition"]


def judge_protocol(continuation_policy: str, horizon: int) -> str:
    if continuation_policy == "greedy_transition":
        return f"root_candidate_plus_greedy_transition_h{int(horizon)}"
    raise ValueError(f"unsupported continuation policy: {continuation_policy}")


def group_count(group_rows: list[dict[str, Any]]) -> int:
    return len({int(row["group_index"]) for row in group_rows})


def candidate_group_diagnostics(group_rows: list[dict[str, Any]]) -> dict[str, Any]:
    returns = sorted((float(row["targets"]["discounted_return"]) for row in group_rows), reverse=True)
    top2_gap = float(returns[0] - returns[1]) if len(returns) > 1 else 0.0
    target_names = [
        "survived_horizon",
        "terminal_defeat",
        "terminal_victory",
        "root_terminal_defeat",
        "root_terminal_victory",
    ]
    variation = {
        name: len({float(row["targets"].get(name, 0.0)) for row in group_rows}) > 1
        for name in target_names
    }
    best_score = returns[0] if returns else 0.0
    worst_score = returns[-1] if returns else 0.0
    return {
        "candidate_count": int(len(group_rows)),
        "best_discounted_return": float(best_score),
        "worst_discounted_return": float(worst_score),
        "return_range": float(best_score - worst_score),
        "top2_gap": top2_gap,
        "survival_disagreement": bool(variation["survived_horizon"]),
        "terminal_defeat_disagreement": bool(variation["terminal_defeat"]),
        "terminal_victory_disagreement": bool(variation["terminal_victory"]),
        "root_terminal_defeat_disagreement": bool(variation["root_terminal_defeat"]),
        "root_terminal_victory_disagreement": bool(variation["root_terminal_victory"]),
    }


def hard_group_checks(group_diagnostics: dict[str, Any], args: argparse.Namespace) -> dict[str, bool]:
    checks: dict[str, bool] = {}
    if float(args.min_top2_gap) > 0.0:
        checks["min_top2_gap"] = float(group_diagnostics["top2_gap"]) >= float(args.min_top2_gap)
    if float(args.min_return_range) > 0.0:
        checks["min_return_range"] = float(group_diagnostics["return_range"]) >= float(args.min_return_range)
    if bool(args.require_survival_disagreement):
        checks["survival_disagreement"] = bool(group_diagnostics["survival_disagreement"])
    if bool(args.require_terminal_disagreement):
        checks["terminal_disagreement"] = bool(
            group_diagnostics["terminal_defeat_disagreement"]
            or group_diagnostics["terminal_victory_disagreement"]
        )
    if bool(args.require_root_terminal_disagreement):
        checks["root_terminal_disagreement"] = bool(
            group_diagnostics["root_terminal_defeat_disagreement"]
            or group_diagnostics["root_terminal_victory_disagreement"]
        )
    return checks


def hard_group_reject_reason(group_diagnostics: dict[str, Any], args: argparse.Namespace) -> str | None:
    checks = hard_group_checks(group_diagnostics, args)
    if not checks:
        return None
    if args.hard_group_match == "all":
        return None if all(checks.values()) else "hard_group_all_criteria_unmet"
    return None if any(checks.values()) else "hard_group_any_criteria_unmet"


def summarize_group_diagnostics(items: list[dict[str, Any]]) -> dict[str, Any]:
    if not items:
        return {"groups": 0}
    numeric_keys = ["candidate_count", "return_range", "top2_gap", "best_discounted_return", "worst_discounted_return"]
    bool_keys = [
        "survival_disagreement",
        "terminal_defeat_disagreement",
        "terminal_victory_disagreement",
        "root_terminal_defeat_disagreement",
        "root_terminal_victory_disagreement",
    ]
    summary: dict[str, Any] = {"groups": int(len(items))}
    for key in numeric_keys:
        values = [float(item.get(key, 0.0)) for item in items]
        summary[f"{key}_mean"] = float(np.mean(values))
        summary[f"{key}_median"] = float(np.median(values))
    for key in bool_keys:
        summary[f"{key}_rate"] = float(np.mean([1.0 if item.get(key) else 0.0 for item in items]))
    summary["large_gap_0_10_rate"] = float(np.mean([1.0 if float(item.get("top2_gap", 0.0)) >= 0.10 else 0.0 for item in items]))
    return summary


def state_filter_reason(
    raw_observation: dict[str, Any],
    *,
    step_index: int,
    min_visible_unblocked: float,
    max_player_hp: float,
    min_step_index: int,
) -> str | None:
    if int(step_index) < int(min_step_index):
        return "before_min_step_index"
    if visible_unblocked(raw_observation) < float(min_visible_unblocked):
        return "below_min_visible_unblocked"
    if float(max_player_hp) > 0.0 and player_hp(raw_observation) > float(max_player_hp):
        return "above_max_player_hp"
    return None


def append_candidate_row(
    obs_samples: dict[str, list[np.ndarray]],
    value_samples: dict[str, list[np.ndarray | float]],
    obs: dict[str, np.ndarray],
    row: dict[str, Any],
) -> None:
    for key, value in obs.items():
        obs_samples.setdefault(key, []).append(np.asarray(value))
    value_samples.setdefault("candidate_features", []).append(np.asarray(row["candidate_features"], dtype=np.float32))
    value_samples.setdefault("candidate_actions", []).append(np.asarray(row["candidate_actions"], dtype=np.int64))
    value_samples.setdefault("candidate_class", []).append(float(row["candidate_class"]))
    value_samples.setdefault("group_index", []).append(float(row["group_index"]))
    value_samples.setdefault("candidate_index", []).append(float(row["candidate_index"]))
    value_samples.setdefault("candidate_is_best", []).append(float(row["candidate_is_best"]))
    for key in CANDIDATE_REGRESSION_TARGETS + CANDIDATE_BINARY_TARGETS + ["steps_observed"]:
        value_samples.setdefault(f"target__{key}", []).append(float((row.get("targets") or {}).get(key, 0.0)))


def write_npz_dataset(
    path: Path,
    obs_samples: dict[str, list[np.ndarray]],
    value_samples: dict[str, list[np.ndarray | float]],
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload: dict[str, np.ndarray] = {}
    for key, values in obs_samples.items():
        payload[f"obs__{key}"] = np.stack(values, axis=0)
    for key, values in value_samples.items():
        if key == "candidate_actions":
            payload[key] = np.stack(values, axis=0).astype(np.int64)
        elif key == "candidate_features":
            payload[key] = np.stack(values, axis=0).astype(np.float32)
        elif key in {"candidate_class", "group_index", "candidate_index", "candidate_is_best"}:
            payload[key] = np.asarray(values, dtype=np.float32)
        else:
            payload[key] = np.asarray(values, dtype=np.float32)
    np.savez_compressed(path, **payload)


def label_candidate_continuation(
    *,
    spec_path: Path,
    seed_hint: int,
    prefix_actions: list[dict[str, int]],
    candidate: dict[str, Any],
    main_env: Any,
    env_args: dict[str, Any],
    horizon: int,
    gamma: float,
    continuation_policy: str = "greedy_transition",
) -> tuple[dict[str, float] | None, dict[str, Any]]:
    if continuation_policy not in CONTINUATION_POLICY_CHOICES:
        raise ValueError(f"unsupported continuation policy: {continuation_policy}")
    action = main_env.candidate_to_canonical(candidate)
    probe = make_env(**env_args)
    try:
        _, info, replay_ok = replay_prefix(
            probe,
            spec_path=spec_path,
            seed_hint=seed_hint,
            prefix_actions=prefix_actions,
        )
        if not replay_ok:
            return None, {"replay_ok": False, "candidate_label": candidate.get("label")}
        raw_start = info.get("raw_observation") or {}
        start_hp = player_hp(raw_start)
        start_enemy_hp = total_living_monster_hp(raw_start)

        _, reward, done, truncated, info = probe.step(action)
        if info.get("invalid_action") or info.get("decoder_failure"):
            return None, {
                "replay_ok": True,
                "invalid": bool(info.get("invalid_action")),
                "decoder_failure": bool(info.get("decoder_failure")),
                "candidate_label": candidate.get("label"),
            }
        raw_after_root = info.get("raw_observation") or {}
        root_outcome = str(info.get("outcome") or "")
        rewards = [float(reward)]
        rollout_actions = [
            {
                "step": 0,
                "source": "root_candidate",
                "action": action,
                "label": info.get("chosen_action_label"),
                "reward": float(reward),
                "outcome": info.get("outcome"),
            }
        ]
        rollout_prefix = list(prefix_actions) + [action]

        while len(rewards) < int(horizon) and not done and not truncated:
            continuation_candidates = legal_candidates(info)
            if not continuation_candidates:
                break
            teacher_action, audit = choose_teacher_action(
                spec_path=spec_path,
                seed_hint=seed_hint,
                prefix_actions=rollout_prefix,
                candidates=continuation_candidates,
                main_env=probe,
                env_args=env_args,
            )
            if teacher_action is None:
                break
            _, reward, done, truncated, info = probe.step(teacher_action)
            rewards.append(float(reward))
            rollout_actions.append(
                {
                    "step": len(rewards) - 1,
                    "source": f"{continuation_policy}_continuation",
                    "action": teacher_action,
                    "label": info.get("chosen_action_label"),
                    "reward": float(reward),
                    "outcome": info.get("outcome"),
                    "judge_gap": (audit or {}).get("gap"),
                    "greedy_transition_gap": (audit or {}).get("gap"),
                }
            )
            if info.get("invalid_action") or info.get("decoder_failure"):
                break
            rollout_prefix.append(teacher_action)

        raw_final = info.get("raw_observation") or {}
        outcome = str(info.get("outcome") or ("truncated" if truncated else "ongoing"))
        final_hp = player_hp(raw_final)
        final_enemy_hp = total_living_monster_hp(raw_final)
        root_hp = player_hp(raw_after_root)
        root_enemy_hp = total_living_monster_hp(raw_after_root)
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
            "immediate_reward": float(rewards[0]) if rewards else 0.0,
            "root_hp_delta": root_hp - start_hp,
            "root_enemy_hp_delta": start_enemy_hp - root_enemy_hp,
            "root_visible_unblocked": visible_unblocked(raw_after_root),
            "root_terminal_victory": 1.0 if root_outcome == "victory" else 0.0,
            "root_terminal_defeat": 1.0 if root_outcome == "defeat" else 0.0,
            "steps_observed": float(len(rewards)),
        }
        audit = {
            "candidate_label": candidate.get("label"),
            "candidate_action": action,
            "replay_ok": True,
            "horizon": int(horizon),
            "gamma": float(gamma),
            "label_mode": "fixed_seed_replay",
            "continuation_policy": continuation_policy,
            "judge_protocol": judge_protocol(continuation_policy, horizon),
            "outcome": outcome,
            "root_outcome": root_outcome,
            "rollout_actions": rollout_actions,
            "start": {
                "player_hp": start_hp,
                "enemy_hp": start_enemy_hp,
                "visible_unblocked": visible_unblocked(raw_start),
            },
            "after_root": {
                "player_hp": root_hp,
                "enemy_hp": root_enemy_hp,
                "visible_unblocked": visible_unblocked(raw_after_root),
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


def main() -> None:
    parser = argparse.ArgumentParser(description="Build a structured candidate value dataset.")
    parser.add_argument("--spec-source", choices=["start_spec"], default="start_spec")
    parser.add_argument("--start-spec", action="append", required=True, type=Path)
    parser.add_argument("--seeds", default="2009,2010,2011,2012")
    parser.add_argument("--states", default=32, type=int)
    parser.add_argument("--max-candidates-per-state", default=8, type=int)
    parser.add_argument("--max-episode-steps", default=96, type=int)
    parser.add_argument("--label-horizon", default=8, type=int)
    parser.add_argument("--gamma", default=0.97, type=float)
    parser.add_argument("--continuation-policy", choices=CONTINUATION_POLICY_CHOICES, default="greedy_transition")
    parser.add_argument("--state-policy", choices=["teacher", "greedy_transition", "random", "mixed"], default="mixed")
    parser.add_argument("--mixed-random-rate", default=0.35, type=float)
    parser.add_argument("--min-visible-unblocked", default=0.0, type=float)
    parser.add_argument("--max-player-hp", default=0.0, type=float)
    parser.add_argument("--min-step-index", default=0, type=int)
    parser.add_argument("--min-top2-gap", default=0.0, type=float)
    parser.add_argument("--min-return-range", default=0.0, type=float)
    parser.add_argument("--require-survival-disagreement", action="store_true")
    parser.add_argument("--require-terminal-disagreement", action="store_true")
    parser.add_argument("--require-root-terminal-disagreement", action="store_true")
    parser.add_argument("--hard-group-match", choices=["any", "all"], default="any")
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
    parser.add_argument("--best-tie-epsilon", default=1e-6, type=float)
    parser.add_argument("--driver-binary", default=None, type=Path)
    parser.add_argument("--rng-seed", default=7, type=int)
    parser.add_argument(
        "--out",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "structured_candidate_value_dataset.npz",
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
    value_samples: dict[str, list[np.ndarray | float]] = {}
    rows: list[dict[str, Any]] = []
    episodes_started = 0
    collection_policy_counts = {"greedy_transition": 0, "random": 0}
    skipped_state_counts: dict[str, int] = {}
    group_filter_reject_counts: dict[str, int] = {}
    candidate_groups_considered = 0
    accepted_group_diagnostics: list[dict[str, Any]] = []
    candidate_evals = 0
    main_env = make_env(**env_args)
    try:
        for spec_path in spec_paths:
            for seed_hint in seeds:
                if group_count(rows) >= args.states:
                    break
                episodes_started += 1
                obs, info = main_env.reset(options={"spec_path": spec_path, "seed_hint": seed_hint})
                prefix_actions: list[dict[str, int]] = []
                done = False
                truncated = False
                step_index = 0
                while not done and not truncated and step_index < args.max_episode_steps:
                    if group_count(rows) >= args.states:
                        break
                    candidates = legal_candidates(info)[: min(int(args.max_candidates_per_state), MAX_RANKER_CANDIDATES)]
                    if not candidates:
                        break
                    raw = info.get("raw_observation") or {}
                    skip_reason = state_filter_reason(
                        raw,
                        step_index=step_index,
                        min_visible_unblocked=args.min_visible_unblocked,
                        max_player_hp=args.max_player_hp,
                        min_step_index=args.min_step_index,
                    )
                    if skip_reason is not None:
                        skipped_state_counts[skip_reason] = skipped_state_counts.get(skip_reason, 0) + 1
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
                        continue
                    group_index = group_count(rows)
                    group_rows: list[dict[str, Any]] = []
                    for candidate_index, candidate in enumerate(candidates):
                        targets, audit = label_candidate_continuation(
                            spec_path=spec_path,
                            seed_hint=seed_hint,
                            prefix_actions=prefix_actions,
                            candidate=candidate,
                            main_env=main_env,
                            env_args=env_args,
                            horizon=args.label_horizon,
                            gamma=args.gamma,
                            continuation_policy=args.continuation_policy,
                        )
                        candidate_evals += 1
                        if targets is None:
                            continue
                        action = main_env.candidate_to_canonical(candidate)
                        action_class = candidate_action_class(candidate)
                        group_rows.append(
                            {
                                "group_index": group_index,
                                "candidate_index": candidate_index,
                                "candidate_label": candidate.get("label"),
                                "candidate_class_name": action_class,
                                "candidate_class": int(ACTION_CLASS_IDS.get(action_class, 0)),
                                "candidate_actions": candidate_action_array(action),
                                "candidate_features": candidate_feature_vector(raw, candidate),
                                "targets": targets,
                                "audit": audit,
                            }
                        )
                    if not group_rows:
                        break
                    candidate_groups_considered += 1
                    group_diagnostics = candidate_group_diagnostics(group_rows)
                    reject_reason = hard_group_reject_reason(group_diagnostics, args)
                    if reject_reason is not None:
                        group_filter_reject_counts[reject_reason] = group_filter_reject_counts.get(reject_reason, 0) + 1
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
                        continue

                    accepted_group_diagnostics.append(group_diagnostics)
                    best_score = max(float(row["targets"]["discounted_return"]) for row in group_rows)
                    for row in group_rows:
                        row["candidate_is_best"] = (
                            float(row["targets"]["discounted_return"]) >= best_score - float(args.best_tie_epsilon)
                        )
                        append_candidate_row(obs_samples, value_samples, obs, row)
                        rows.append(
                            {
                                "sample_index": len(rows),
                                "group_index": int(row["group_index"]),
                                "candidate_index": int(row["candidate_index"]),
                                "spec_path": str(spec_path.relative_to(REPO_ROOT) if spec_path.is_relative_to(REPO_ROOT) else spec_path),
                                "spec_name": load_start_spec_name(spec_path),
                                "seed": int(seed_hint),
                                "step_index": int(step_index),
                                "prefix_len": len(prefix_actions),
                                "legal_action_count": len(candidates),
                                "candidate_label": row["candidate_label"],
                                "candidate_class": row["candidate_class_name"],
                                "candidate_is_best": bool(row["candidate_is_best"]),
                                "group_diagnostics": group_diagnostics,
                                "targets": row["targets"],
                                "audit": row["audit"],
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
            if group_count(rows) >= args.states:
                break
    finally:
        main_env.close()

    if not rows:
        raise SystemExit("candidate value dataset generation produced no samples")
    write_npz_dataset(args.out, obs_samples, value_samples)
    rows_out = args.rows_out or args.out.with_suffix(".jsonl")
    summary_out = args.summary_out or args.out.with_suffix(".summary.json")
    write_jsonl(rows_out, rows)
    summary = {
        "dataset": str(args.out),
        "rows": str(rows_out),
        "row_count": len(rows),
        "state_groups": group_count(rows),
        "candidate_groups_considered": int(candidate_groups_considered),
        "candidate_groups_accepted": int(len(accepted_group_diagnostics)),
        "episodes_started": episodes_started,
        "candidate_evals": candidate_evals,
        "specs": [str(path) for path in spec_paths],
        "seeds": seeds,
        "state_policy": args.state_policy,
        "collection_policy_counts": collection_policy_counts,
        "skipped_state_counts": skipped_state_counts,
        "group_filter_reject_counts": group_filter_reject_counts,
        "state_filters": {
            "min_visible_unblocked": float(args.min_visible_unblocked),
            "max_player_hp": float(args.max_player_hp),
            "min_step_index": int(args.min_step_index),
        },
        "hard_group_filters": {
            "min_top2_gap": float(args.min_top2_gap),
            "min_return_range": float(args.min_return_range),
            "require_survival_disagreement": bool(args.require_survival_disagreement),
            "require_terminal_disagreement": bool(args.require_terminal_disagreement),
            "require_root_terminal_disagreement": bool(args.require_root_terminal_disagreement),
            "match": args.hard_group_match,
        },
        "accepted_group_diagnostics": summarize_group_diagnostics(accepted_group_diagnostics),
        "label_mode": "fixed_seed_replay",
        "continuation_policy": args.continuation_policy,
        "judge_protocol": judge_protocol(args.continuation_policy, int(args.label_horizon)),
        "label_policy": f"candidate_then_{args.continuation_policy}_continuation",
        "label_horizon": int(args.label_horizon),
        "gamma": float(args.gamma),
        "max_candidates_per_state": int(args.max_candidates_per_state),
        "draw_order_variant": args.draw_order_variant,
        "reward_mode": args.reward_mode,
        "reward": reward_config,
        "regression_targets": CANDIDATE_REGRESSION_TARGETS,
        "binary_targets": CANDIDATE_BINARY_TARGETS,
        "notes": [
            "each row is a root candidate labelled by candidate execution plus short greedy-transition continuation",
            "groups preserve candidate alternatives from the same root state",
            "ranking labels use discounted_return within each candidate group",
        ],
    }
    write_json(summary_out, summary)
    print(json.dumps(summary, indent=2, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
