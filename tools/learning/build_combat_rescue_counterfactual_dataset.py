#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import random
from pathlib import Path
from typing import Any

from build_structured_bc_teacher_dataset import legal_candidates, make_env, replay_prefix
from build_structured_candidate_value_dataset import candidate_group_diagnostics, label_candidate_continuation
from build_structured_state_evaluator_dataset import choose_collection_action, player_hp, total_living_monster_hp, visible_unblocked
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


def evaluate_decision_candidates(
    *,
    spec_path: Path,
    seed_hint: int,
    prefix_actions: list[dict[str, int]],
    main_env: Any,
    env_args: dict[str, Any],
    horizon: int,
    gamma: float,
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
    parser = argparse.ArgumentParser(description="Build combat rescue counterfactual rows from failed episodes.")
    parser.add_argument("--spec-source", choices=["start_spec"], default="start_spec")
    parser.add_argument("--start-spec", action="append", required=True, type=Path)
    parser.add_argument("--seeds", default="2009,2010,2011,2012")
    parser.add_argument("--episodes", default=8, type=int)
    parser.add_argument("--max-episode-steps", default=96, type=int)
    parser.add_argument("--max-backtrack-steps", default=8, type=int)
    parser.add_argument("--label-horizon", default=12, type=int)
    parser.add_argument("--gamma", default=0.97, type=float)
    parser.add_argument("--rescue-mode", choices=["survival", "root_or_survival", "return", "any"], default="survival")
    parser.add_argument("--state-policy", choices=["teacher", "random", "mixed"], default="mixed")
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
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "combat_rescue_counterfactual_rows.jsonl",
        type=Path,
    )
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

    rng = random.Random(int(args.rng_seed))
    rows: list[dict[str, Any]] = []
    episode_summaries: list[dict[str, Any]] = []
    episodes_started = 0
    defeated_episodes = 0
    rescued_episodes = 0
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
                            "rescue_rows": 0,
                        }
                    )
                    continue
                defeated_episodes += 1
                episode_rescue_rows = 0
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
                    for rescue_candidate in candidate_rows:
                        if action_key(rescue_candidate["action"]) == action_key(bad_action):
                            continue
                        reasons = rescue_reason(
                            bad_candidate["targets"],
                            rescue_candidate["targets"],
                            min_return_delta=float(args.min_return_delta),
                        )
                        if not rescue_reasons_match(reasons, args.rescue_mode):
                            continue
                        row = {
                            "sample_index": len(rows),
                            "spec_path": str(spec_path.relative_to(REPO_ROOT) if spec_path.is_relative_to(REPO_ROOT) else spec_path),
                            "spec_name": load_start_spec_name(spec_path),
                            "seed": int(seed_hint),
                            "episode_outcome": episode.get("outcome"),
                            "episode_steps": len(actions),
                            "decision_step": int(decision_index),
                            "backtrack_offset": int(backtrack_offset),
                            "prefix_len": len(prefix_actions),
                            "state": raw_observation_summary(raw),
                            "bad": {
                                "label": bad_step.get("label") or bad_candidate.get("candidate_label"),
                                "source": bad_step.get("source"),
                                "action": bad_action,
                                "targets": bad_candidate["targets"],
                                "outcome_bucket": bad_candidate["outcome_bucket"],
                            },
                            "rescue": {
                                "label": rescue_candidate.get("candidate_label"),
                                "action": rescue_candidate["action"],
                                "targets": rescue_candidate["targets"],
                                "outcome_bucket": rescue_candidate["outcome_bucket"],
                            },
                            "improvement": {
                                "discounted_return_delta": float(rescue_candidate["targets"].get("discounted_return") or 0.0)
                                - float(bad_candidate["targets"].get("discounted_return") or 0.0),
                                "hp_delta_delta": float(rescue_candidate["targets"].get("hp_delta") or 0.0)
                                - float(bad_candidate["targets"].get("hp_delta") or 0.0),
                                "enemy_hp_delta_delta": float(rescue_candidate["targets"].get("enemy_hp_delta") or 0.0)
                                - float(bad_candidate["targets"].get("enemy_hp_delta") or 0.0),
                            },
                            "reasons": reasons,
                            "candidate_group": {
                                "candidate_count": len(candidate_rows),
                                **group_diagnostics,
                            },
                            "candidates": [
                                {
                                    "candidate_index": candidate.get("candidate_index"),
                                    "label": candidate.get("candidate_label"),
                                    "action": candidate.get("action"),
                                    "targets": candidate.get("targets"),
                                    "outcome_bucket": candidate.get("outcome_bucket"),
                                    "is_bad_action": action_key(candidate.get("action")) == action_key(bad_action),
                                    "is_rescue_action": action_key(candidate.get("action")) == action_key(rescue_candidate.get("action")),
                                }
                                for candidate in candidate_rows
                            ],
                        }
                        rows.append(row)
                        episode_rescue_rows += 1
                if episode_rescue_rows > 0:
                    rescued_episodes += 1
                else:
                    macro_backtrack_candidates += 1
                episode_summaries.append(
                    {
                        "spec": str(spec_path),
                        "seed": int(seed_hint),
                        "outcome": episode.get("outcome"),
                        "steps": len(actions),
                        "rescue_rows": int(episode_rescue_rows),
                        "needs_macro_backtrack": bool(episode_rescue_rows == 0),
                    }
                )
            if episodes_started >= int(args.episodes):
                break
    finally:
        main_env.close()

    summary_out = args.summary_out or args.out.with_suffix(".summary.json")
    write_jsonl(args.out, rows)
    summary = {
        "rows": str(args.out),
        "row_count": len(rows),
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
        "rescue_mode": args.rescue_mode,
        "min_return_delta": float(args.min_return_delta),
        "draw_order_variant": args.draw_order_variant,
        "reward_mode": args.reward_mode,
        "reward": reward_config,
        "episodes": episode_summaries,
        "notes": [
            "rows compare a failed episode action against a same-state rescue candidate",
            "candidate outcomes are labelled by root action plus short teacher continuation",
            "episodes with no combat rescue rows are candidates for macro backtracking",
        ],
    }
    write_json(summary_out, summary)
    print(json.dumps(summary, indent=2, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
