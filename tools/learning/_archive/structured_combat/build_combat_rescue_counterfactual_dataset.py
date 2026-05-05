#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import random
from pathlib import Path
from typing import Any

from build_structured_bc_teacher_dataset import legal_candidates, make_env, replay_prefix
from build_structured_candidate_value_dataset import (
    CONTINUATION_POLICY_CHOICES,
    candidate_group_diagnostics,
    judge_protocol,
    label_candidate_continuation,
)
from build_structured_state_evaluator_dataset import (
    choose_collection_action,
    normalize_state_policy,
    player_hp,
    total_living_monster_hp,
    visible_unblocked,
)
from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from train_structured_combat_ppo import load_start_spec_name, parse_seed_list


def action_key(action: dict[str, int] | None) -> tuple[int, int, int, int, int]:
    action = action or {}
    return (
        int(action.get("action_type") or 0),
        int(action.get("card_slot") or 0),
        int(action.get("target_slot") or 0),
        int(action.get("potion_slot") or 0),
        int(action.get("choice_index") or 0),
    )


def stable_hash(payload: Any, length: int = 16) -> str:
    text = json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
    return hashlib.sha256(text.encode("utf-8")).hexdigest()[:length]


def relative_path_text(path: Path) -> str:
    return str(path.relative_to(REPO_ROOT) if path.is_relative_to(REPO_ROOT) else path)


def replay_key(spec_path: Path, seed_hint: int, prefix_actions: list[dict[str, int]]) -> dict[str, Any]:
    prefix_hash = stable_hash(prefix_actions, length=16)
    return {
        "spec_path": relative_path_text(spec_path),
        "seed": int(seed_hint),
        "prefix_len": int(len(prefix_actions)),
        "prefix_action_hash": prefix_hash,
    }


def raw_observation_summary(raw: dict[str, Any]) -> dict[str, Any]:
    monsters = []
    for monster in raw.get("monsters") or []:
        monsters.append(
            {
                "id": monster.get("id"),
                "hp": monster.get("current_hp"),
                "block": monster.get("block"),
                "intent": monster.get("intent"),
            }
        )
    hand = []
    for index, card in enumerate(raw.get("hand") or []):
        hand.append(
            {
                "slot": index,
                "id": card.get("id"),
                "name": card.get("name"),
                "cost": card.get("cost_for_turn"),
                "playable": bool(card.get("playable")),
            }
        )
    return {
        "turn_count": raw.get("turn_count"),
        "phase": raw.get("phase"),
        "player_hp": player_hp(raw),
        "player_block": raw.get("player_block"),
        "energy": raw.get("energy"),
        "visible_unblocked": visible_unblocked(raw),
        "enemy_hp": total_living_monster_hp(raw),
        "monsters": monsters,
        "hand": hand,
        "draw_count": raw.get("draw_count"),
        "discard_count": raw.get("discard_count"),
        "exhaust_count": raw.get("exhaust_count"),
    }


def target_outcome_bucket(targets: dict[str, float]) -> str:
    if float(targets.get("terminal_victory") or 0.0) > 0.5:
        return "victory"
    if float(targets.get("terminal_defeat") or 0.0) > 0.5:
        return "defeat"
    if float(targets.get("survived_horizon") or 0.0) > 0.5:
        return "survives_horizon"
    return "unknown"


def rescue_reason(bad: dict[str, float], rescue: dict[str, float], min_return_delta: float) -> list[str]:
    reasons: list[str] = []
    if float(bad.get("root_terminal_defeat") or 0.0) > 0.5 and float(rescue.get("root_terminal_defeat") or 0.0) < 0.5:
        reasons.append("avoids_root_defeat")
    if float(bad.get("terminal_defeat") or 0.0) > 0.5 and float(rescue.get("terminal_defeat") or 0.0) < 0.5:
        reasons.append("avoids_horizon_defeat")
    if float(bad.get("survived_horizon") or 0.0) < 0.5 and float(rescue.get("survived_horizon") or 0.0) > 0.5:
        reasons.append("restores_horizon_survival")
    if float(rescue.get("discounted_return") or 0.0) - float(bad.get("discounted_return") or 0.0) >= float(min_return_delta):
        reasons.append("improves_return")
    return reasons


def rescue_reasons_match(reasons: list[str], rescue_mode: str) -> bool:
    reason_set = set(reasons)
    if rescue_mode == "any":
        return bool(reasons)
    if rescue_mode == "survival":
        return bool(reason_set & {"avoids_horizon_defeat", "restores_horizon_survival"})
    if rescue_mode == "root_or_survival":
        return bool(reason_set & {"avoids_root_defeat", "avoids_horizon_defeat", "restores_horizon_survival"})
    if rescue_mode == "return":
        return "improves_return" in reason_set
    raise ValueError(f"unknown rescue mode: {rescue_mode}")


def rescue_confidence(reasons: list[str]) -> str:
    reason_set = set(reasons)
    if "avoids_root_defeat" in reason_set:
        return "root_defeat_counterfactual"
    if reason_set & {"avoids_horizon_defeat", "restores_horizon_survival"}:
        return "hard_survival_counterfactual"
    if "improves_return" in reason_set:
        return "return_counterfactual"
    return "audit_only"


def pairwise_delta(base: dict[str, float], candidate: dict[str, float]) -> dict[str, float]:
    return {
        "discounted_return_delta": float(candidate.get("discounted_return") or 0.0)
        - float(base.get("discounted_return") or 0.0),
        "hp_delta_delta": float(candidate.get("hp_delta") or 0.0) - float(base.get("hp_delta") or 0.0),
        "enemy_hp_delta_delta": float(candidate.get("enemy_hp_delta") or 0.0)
        - float(base.get("enemy_hp_delta") or 0.0),
    }


def candidate_filter_reason(*, is_failed_action: bool, reasons: list[str], rescue_mode: str) -> str | None:
    if is_failed_action:
        return "failed_episode_action"
    if rescue_reasons_match(reasons, rescue_mode):
        return None
    if not reasons:
        return "no_counterfactual_improvement"
    return "counterfactual_does_not_match_rescue_mode"


def evaluate_decision_candidates(
    *,
    spec_path: Path,
    seed_hint: int,
    prefix_actions: list[dict[str, int]],
    main_env: Any,
    env_args: dict[str, Any],
    horizon: int,
    gamma: float,
    continuation_policy: str,
) -> tuple[dict[str, Any] | None, list[dict[str, Any]]]:
    probe = make_env(**env_args)
    try:
        _, info, replay_ok = replay_prefix(
            probe,
            spec_path=spec_path,
            seed_hint=seed_hint,
            prefix_actions=prefix_actions,
        )
        if not replay_ok:
            return None, []
        raw = info.get("raw_observation") or {}
        candidates = legal_candidates(info)
    finally:
        probe.close()

    rows: list[dict[str, Any]] = []
    for candidate_index, candidate in enumerate(candidates):
        targets, audit = label_candidate_continuation(
            spec_path=spec_path,
            seed_hint=seed_hint,
            prefix_actions=prefix_actions,
            candidate=candidate,
            main_env=main_env,
            env_args=env_args,
            horizon=horizon,
            gamma=gamma,
            continuation_policy=continuation_policy,
        )
        if targets is None:
            continue
        action = main_env.candidate_to_canonical(candidate)
        rows.append(
            {
                "candidate_index": candidate_index,
                "candidate_label": candidate.get("label"),
                "action": action,
                "targets": targets,
                "outcome_bucket": target_outcome_bucket(targets),
                "audit": {
                    "candidate_label": audit.get("candidate_label"),
                    "outcome": audit.get("outcome"),
                    "root_outcome": audit.get("root_outcome"),
                    "start": audit.get("start"),
                    "after_root": audit.get("after_root"),
                    "final": audit.get("final"),
                },
            }
        )
    return raw, rows


def run_episode(
    *,
    spec_path: Path,
    seed_hint: int,
    main_env: Any,
    env_args: dict[str, Any],
    state_policy: str,
    mixed_random_rate: float,
    rng: random.Random,
    max_episode_steps: int,
) -> dict[str, Any]:
    obs, info = main_env.reset(options={"spec_path": spec_path, "seed_hint": seed_hint})
    del obs
    prefix_actions: list[dict[str, int]] = []
    steps: list[dict[str, Any]] = []
    done = False
    truncated = False
    step_index = 0
    while not done and not truncated and step_index < int(max_episode_steps):
        raw_before = info.get("raw_observation") or {}
        candidates = legal_candidates(info)
        if not candidates:
            break
        action, source = choose_collection_action(
            state_policy=state_policy,
            rng=rng,
            mixed_random_rate=mixed_random_rate,
            spec_path=spec_path,
            seed_hint=seed_hint,
            prefix_actions=prefix_actions,
            candidates=candidates,
            main_env=main_env,
            env_args=env_args,
        )
        if action is None:
            break
        _, reward, done, truncated, info = main_env.step(action)
        steps.append(
            {
                "step_index": step_index,
                "prefix_len": len(prefix_actions),
                "source": source,
                "action": action,
                "label": info.get("chosen_action_label"),
                "reward": float(reward),
                "outcome_after": info.get("outcome"),
                "raw_before": raw_observation_summary(raw_before),
            }
        )
        if info.get("invalid_action") or info.get("decoder_failure"):
            break
        prefix_actions.append(action)
        step_index += 1
    return {
        "spec_path": spec_path,
        "seed": seed_hint,
        "outcome": info.get("outcome"),
        "steps": steps,
        "actions": prefix_actions,
        "truncated": bool(truncated),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="Build combat rescue decision groups from failed episodes.")
    parser.add_argument("--spec-source", choices=["start_spec"], default="start_spec")
    parser.add_argument("--start-spec", action="append", required=True, type=Path)
    parser.add_argument("--seeds", default="2009,2010,2011,2012")
    parser.add_argument("--episodes", default=8, type=int)
    parser.add_argument("--max-episode-steps", default=96, type=int)
    parser.add_argument("--max-backtrack-steps", default=8, type=int)
    parser.add_argument("--label-horizon", default=12, type=int)
    parser.add_argument("--gamma", default=0.97, type=float)
    parser.add_argument("--continuation-policy", choices=CONTINUATION_POLICY_CHOICES, default="greedy_transition")
    parser.add_argument("--rescue-mode", choices=["survival", "root_or_survival", "return", "any"], default="survival")
    parser.add_argument("--state-policy", choices=["teacher", "greedy_transition", "random", "mixed"], default="mixed")
    parser.add_argument("--mixed-random-rate", default=0.75, type=float)
    parser.add_argument("--min-return-delta", default=0.25, type=float)
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
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "combat_rescue_decision_groups.jsonl",
        type=Path,
    )
    parser.add_argument("--summary-out", default=None, type=Path)
    parser.add_argument("--macro-manifest-out", default=None, type=Path)
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
    groups: list[dict[str, Any]] = []
    macro_backtrack_rows: list[dict[str, Any]] = []
    episode_summaries: list[dict[str, Any]] = []
    episodes_started = 0
    defeated_episodes = 0
    rescued_episodes = 0
    rescue_pair_count = 0
    macro_backtrack_candidates = 0
    main_env = make_env(**env_args)
    try:
        for spec_path in spec_paths:
            for seed_hint in seeds:
                if episodes_started >= int(args.episodes):
                    break
                episodes_started += 1
                episode = run_episode(
                    spec_path=spec_path,
                    seed_hint=seed_hint,
                    main_env=main_env,
                    env_args=env_args,
                    state_policy=args.state_policy,
                    mixed_random_rate=float(args.mixed_random_rate),
                    rng=rng,
                    max_episode_steps=int(args.max_episode_steps),
                )
                failed = episode.get("outcome") == "defeat"
                if not failed:
                    episode_summaries.append(
                        {
                            "spec": str(spec_path),
                            "seed": int(seed_hint),
                            "outcome": episode.get("outcome"),
                            "steps": len(episode["steps"]),
                            "rescue_groups": 0,
                            "rescue_pairs": 0,
                        }
                    )
                    continue
                defeated_episodes += 1
                episode_rescue_groups = 0
                episode_rescue_pairs = 0
                actions = list(episode["actions"])
                steps = list(episode["steps"])
                for backtrack_offset in range(1, min(int(args.max_backtrack_steps), len(actions)) + 1):
                    decision_index = len(actions) - backtrack_offset
                    prefix_actions = actions[:decision_index]
                    bad_action = actions[decision_index]
                    bad_step = steps[decision_index]
                    raw, candidate_rows = evaluate_decision_candidates(
                        spec_path=spec_path,
                        seed_hint=seed_hint,
                        prefix_actions=prefix_actions,
                        main_env=main_env,
                        env_args=env_args,
                        horizon=int(args.label_horizon),
                        gamma=float(args.gamma),
                        continuation_policy=args.continuation_policy,
                    )
                    if raw is None or not candidate_rows:
                        continue
                    bad_candidates = [row for row in candidate_rows if action_key(row["action"]) == action_key(bad_action)]
                    if not bad_candidates:
                        continue
                    bad_candidate = bad_candidates[0]
                    group_rows = [
                        {
                            "group_index": 0,
                            "targets": row["targets"],
                        }
                        for row in candidate_rows
                    ]
                    group_diagnostics = candidate_group_diagnostics(group_rows)
                    candidate_outcomes: list[dict[str, Any]] = []
                    rescue_candidate_indices: list[int] = []
                    rescue_reasons_by_candidate: dict[str, list[str]] = {}
                    rescue_confidences: list[str] = []
                    for rescue_candidate in candidate_rows:
                        is_failed_action = action_key(rescue_candidate["action"]) == action_key(bad_action)
                        reasons: list[str] = []
                        if not is_failed_action:
                            reasons = rescue_reason(
                                bad_candidate["targets"],
                                rescue_candidate["targets"],
                                min_return_delta=float(args.min_return_delta),
                            )
                        filter_reason = candidate_filter_reason(
                            is_failed_action=is_failed_action,
                            reasons=reasons,
                            rescue_mode=args.rescue_mode,
                        )
                        is_rescue_candidate = filter_reason is None
                        if is_rescue_candidate:
                            candidate_index = int(rescue_candidate["candidate_index"])
                            rescue_candidate_indices.append(candidate_index)
                            rescue_reasons_by_candidate[str(candidate_index)] = reasons
                            rescue_confidences.append(rescue_confidence(reasons))
                        candidate_outcomes.append(
                            {
                                "candidate_index": int(rescue_candidate["candidate_index"]),
                                "label": rescue_candidate.get("candidate_label"),
                                "action": rescue_candidate["action"],
                                "targets": rescue_candidate["targets"],
                                "outcome_bucket": rescue_candidate["outcome_bucket"],
                                "is_failed_action": bool(is_failed_action),
                                "is_rescue_candidate": bool(is_rescue_candidate),
                                "counterfactual_reasons": reasons,
                                "filter_reason": filter_reason,
                                "delta_vs_failed": pairwise_delta(
                                    bad_candidate["targets"],
                                    rescue_candidate["targets"],
                                ),
                            }
                        )
                    if not rescue_candidate_indices:
                        continue
                    strongest_confidence = (
                        "root_defeat_counterfactual"
                        if "root_defeat_counterfactual" in rescue_confidences
                        else "hard_survival_counterfactual"
                        if "hard_survival_counterfactual" in rescue_confidences
                        else "return_counterfactual"
                        if "return_counterfactual" in rescue_confidences
                        else "audit_only"
                    )
                    key = replay_key(spec_path, seed_hint, prefix_actions)
                    group = {
                        "group_index": len(groups),
                        "decision_group_id": stable_hash(
                            {
                                **key,
                                "decision_step": int(decision_index),
                                "backtrack_offset": int(backtrack_offset),
                                "bad_action": bad_action,
                                "label_horizon": int(args.label_horizon),
                                "rescue_mode": args.rescue_mode,
                            },
                            length=20,
                        ),
                        "schema": "combat_rescue_decision_group/v1",
                        "spec_path": relative_path_text(spec_path),
                        "spec_name": load_start_spec_name(spec_path),
                        "seed": int(seed_hint),
                        "episode_outcome": episode.get("outcome"),
                        "episode_steps": len(actions),
                        "decision_step": int(decision_index),
                        "backtrack_offset": int(backtrack_offset),
                        "prefix_len": len(prefix_actions),
                        "replay_key": key,
                        "label_mode": "fixed_seed_replay",
                        "continuation_policy": args.continuation_policy,
                        "judge_protocol": judge_protocol(args.continuation_policy, int(args.label_horizon)),
                        "intervention_depth": "combat_kstep",
                        "source_policy": args.state_policy,
                        "public_observation": raw_observation_summary(raw),
                        "failed_action": {
                            "candidate_index": int(bad_candidate["candidate_index"]),
                            "label": bad_step.get("label") or bad_candidate.get("candidate_label"),
                            "source": bad_step.get("source"),
                            "action": bad_action,
                            "targets": bad_candidate["targets"],
                            "outcome_bucket": bad_candidate["outcome_bucket"],
                        },
                        "rescue_candidate_indices": rescue_candidate_indices,
                        "rescue_reasons_by_candidate": rescue_reasons_by_candidate,
                        "confidence": strongest_confidence,
                        "filter": {
                            "accepted": True,
                            "accept_reason": "has_rescue_candidate_matching_mode",
                            "reject_reason": None,
                            "rescue_mode": args.rescue_mode,
                            "min_return_delta": float(args.min_return_delta),
                        },
                        "candidate_group": {
                            "candidate_count": len(candidate_rows),
                            **group_diagnostics,
                        },
                        "candidate_outcomes": candidate_outcomes,
                    }
                    groups.append(group)
                    episode_rescue_groups += 1
                    episode_rescue_pairs += len(rescue_candidate_indices)
                    rescue_pair_count += len(rescue_candidate_indices)
                if episode_rescue_groups > 0:
                    rescued_episodes += 1
                else:
                    macro_backtrack_candidates += 1
                    macro_backtrack_rows.append(
                        {
                            "schema": "combat_rescue_macro_backtrack_manifest/v1",
                            "spec_path": relative_path_text(spec_path),
                            "spec_name": load_start_spec_name(spec_path),
                            "seed": int(seed_hint),
                            "episode_outcome": episode.get("outcome"),
                            "episode_steps": len(actions),
                            "max_backtrack_steps": int(args.max_backtrack_steps),
                            "intervention_depth_attempted": "combat_kstep",
                            "reject_reason": "no_combat_rescue_candidate_within_backtrack_window",
                            "source_policy": args.state_policy,
                            "label_mode": "fixed_seed_replay",
                            "continuation_policy": args.continuation_policy,
                            "judge_protocol": judge_protocol(args.continuation_policy, int(args.label_horizon)),
                            "last_failed_steps": [
                                {
                                    "decision_step": int(step.get("step_index") or 0),
                                    "source": step.get("source"),
                                    "label": step.get("label"),
                                    "action": step.get("action"),
                                    "reward": step.get("reward"),
                                    "outcome_after": step.get("outcome_after"),
                                    "state": step.get("raw_before"),
                                }
                                for step in steps[-min(int(args.max_backtrack_steps), len(steps)) :]
                            ],
                        }
                    )
                episode_summaries.append(
                    {
                        "spec": str(spec_path),
                        "seed": int(seed_hint),
                        "outcome": episode.get("outcome"),
                        "steps": len(actions),
                        "rescue_groups": int(episode_rescue_groups),
                        "rescue_pairs": int(episode_rescue_pairs),
                        "needs_macro_backtrack": bool(episode_rescue_groups == 0),
                    }
                )
            if episodes_started >= int(args.episodes):
                break
    finally:
        main_env.close()

    summary_out = args.summary_out or args.out.with_suffix(".summary.json")
    macro_manifest_out = args.macro_manifest_out or args.out.with_suffix(".macro_backtrack.jsonl")
    write_jsonl(args.out, groups)
    write_jsonl(macro_manifest_out, macro_backtrack_rows)
    summary = {
        "schema": "combat_rescue_decision_group/v1",
        "groups": str(args.out),
        "macro_backtrack_manifest": str(macro_manifest_out),
        "group_count": len(groups),
        "row_count": len(groups),
        "rescue_pair_count": int(rescue_pair_count),
        "macro_backtrack_manifest_count": len(macro_backtrack_rows),
        "episodes_started": int(episodes_started),
        "defeated_episodes": int(defeated_episodes),
        "rescued_episodes": int(rescued_episodes),
        "needs_macro_backtrack_episodes": int(macro_backtrack_candidates),
        "specs": [str(path) for path in spec_paths],
        "seeds": seeds,
        "state_policy": args.state_policy,
        "mixed_random_rate": float(args.mixed_random_rate),
        "max_backtrack_steps": int(args.max_backtrack_steps),
        "label_horizon": int(args.label_horizon),
        "gamma": float(args.gamma),
        "label_mode": "fixed_seed_replay",
        "continuation_policy": args.continuation_policy,
        "judge_protocol": judge_protocol(args.continuation_policy, int(args.label_horizon)),
        "rescue_mode": args.rescue_mode,
        "min_return_delta": float(args.min_return_delta),
        "draw_order_variant": args.draw_order_variant,
        "reward_mode": args.reward_mode,
        "reward": reward_config,
        "episodes": episode_summaries,
        "notes": [
            "one output row is one same-state decision group with all labelled root candidates",
            "candidate outcomes are labelled by root action plus short greedy-transition continuation under fixed replay",
            "episodes with no combat rescue groups are written to the macro backtrack manifest",
        ],
    }
    write_json(summary_out, summary)
    print(json.dumps(summary, indent=2, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
