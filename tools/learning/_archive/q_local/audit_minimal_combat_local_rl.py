#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import random
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable

import numpy as np
from sb3_contrib import MaskablePPO

from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from gym_combat_env import GymCombatEnv


def load_config(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def make_env(start_spec: Path, reward_mode: str, reward_config: dict[str, float]) -> GymCombatEnv:
    return GymCombatEnv(
        spec_paths=[start_spec],
        spec_source="start_spec",
        seed=0,
        max_episode_steps=64,
        reward_mode=reward_mode,
        reward_config=reward_config,
    )


def hand_contains_disarm(info: dict[str, Any]) -> bool:
    return any("Disarm" in str(name or "") for name in (info.get("hand_cards") or []))


def action_is_disarm(label: Any) -> bool:
    return "Disarm" in str(label or "")


def choose_random(mask: np.ndarray, _info: dict[str, Any], rng: random.Random, step_index: int) -> int:
    legal = [idx for idx, allowed in enumerate(mask.tolist()) if allowed]
    return rng.choice(legal)


def choose_random_no_first_endturn(
    mask: np.ndarray, info: dict[str, Any], rng: random.Random, step_index: int
) -> int:
    legal = [idx for idx, allowed in enumerate(mask.tolist()) if allowed]
    if step_index != 0:
        return rng.choice(legal)
    non_end = [
        idx
        for idx in legal
        if str((info.get("action_candidates") or [])[idx].get("label") or "") != "EndTurn"
    ]
    return rng.choice(non_end or legal)


def choose_simple_proactive(
    mask: np.ndarray, info: dict[str, Any], _rng: random.Random, _step_index: int
) -> int:
    legal = [idx for idx, allowed in enumerate(mask.tolist()) if allowed]
    candidates = info.get("action_candidates") or []
    for idx in legal:
        label = str(candidates[idx].get("label") or "")
        if "Disarm" in label:
            return idx
    non_end = []
    for idx in legal:
        candidate = candidates[idx]
        label = str(candidate.get("label") or "")
        if label == "EndTurn":
            continue
        family = str(candidate.get("action_family") or "")
        cost_bias = 99
        if family == "play_card":
            card_name = str(candidate.get("card_name") or "")
            if "Defend" in card_name:
                cost_bias = 3
            elif "Shrug" in card_name:
                cost_bias = 2
            elif "Bash" in card_name:
                cost_bias = 2
            else:
                cost_bias = 1
        non_end.append((cost_bias, idx))
    if non_end:
        non_end.sort()
        return non_end[0][1]
    return legal[0]


def run_policy_family(
    env: GymCombatEnv,
    eval_seeds: list[int],
    chooser: Callable[[np.ndarray, dict[str, Any], random.Random, int], int],
    policy_name: str,
    artifact_context: dict[str, Any],
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    rows: list[dict[str, Any]] = []
    rng = random.Random(17)
    catastrophe_threshold = float(env.reward_config["catastrophe_unblocked_threshold"])
    for episode_index, seed_hint in enumerate(eval_seeds):
        obs, info = env.reset(options={"spec_path": str(env.spec_paths[0]), "seed_hint": int(seed_hint)})
        done = False
        truncated = False
        episode_reward = 0.0
        catastrophe_hits = 0
        step_index = 0
        first_action_label = None
        last_info = info
        starting_hp = float(info.get("player_hp") or 0.0)
        final_hp = starting_hp
        initial_turn_count = int(info.get("turn_count") or 0)
        opening_turn_saw_disarm = hand_contains_disarm(info)
        disarm_seen_turn = initial_turn_count if opening_turn_saw_disarm else None
        disarm_played_turn = None
        played_disarm_on_opening_turn = False
        played_disarm_on_first_seen_turn = False
        while not done and not truncated:
            mask = env.action_masks()
            current_turn_count = int(last_info.get("turn_count") or info.get("turn_count") or 0)
            if disarm_seen_turn is None and hand_contains_disarm(last_info):
                disarm_seen_turn = current_turn_count
            action_index = chooser(mask, info, rng, step_index)
            obs, reward, done, truncated, info = env.step(int(action_index))
            if first_action_label is None:
                first_action_label = info.get("chosen_action_label")
            if action_is_disarm(info.get("chosen_action_label")):
                if disarm_played_turn is None:
                    disarm_played_turn = current_turn_count
                if opening_turn_saw_disarm and current_turn_count == initial_turn_count:
                    played_disarm_on_opening_turn = True
                if disarm_seen_turn is not None and current_turn_count == disarm_seen_turn:
                    played_disarm_on_first_seen_turn = True
            episode_reward += float(reward)
            if float(info.get("visible_unblocked") or 0.0) >= catastrophe_threshold:
                catastrophe_hits += 1
            final_hp = float(info.get("player_hp") or final_hp)
            last_info = info
            step_index += 1
        damage_taken = max(starting_hp - final_hp, 0.0)
        rows.append(
            {
                "artifact_context": artifact_context,
                "policy": policy_name,
                "episode_index": episode_index,
                "seed_hint": int(seed_hint),
                "outcome": last_info.get("outcome"),
                "episode_reward": round(episode_reward, 4),
                "damage_taken": round(damage_taken, 4),
                "catastrophe_hits": catastrophe_hits,
                "first_action_label": first_action_label,
                "opening_turn_saw_disarm": opening_turn_saw_disarm,
                "played_disarm_on_opening_turn": played_disarm_on_opening_turn,
                "disarm_seen_turn": disarm_seen_turn,
                "disarm_played_turn": disarm_played_turn,
                "played_disarm_on_first_seen_turn": played_disarm_on_first_seen_turn,
            }
        )
    metrics = {
        "episodes": len(rows),
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
        "first_action_counts": dict(Counter(str(row.get("first_action_label") or "None") for row in rows)),
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


def run_model_family(
    env: GymCombatEnv,
    model: MaskablePPO,
    eval_seeds: list[int],
    artifact_context: dict[str, Any],
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    rows: list[dict[str, Any]] = []
    catastrophe_threshold = float(env.reward_config["catastrophe_unblocked_threshold"])
    for episode_index, seed_hint in enumerate(eval_seeds):
        obs, info = env.reset(options={"spec_path": str(env.spec_paths[0]), "seed_hint": int(seed_hint)})
        done = False
        truncated = False
        episode_reward = 0.0
        catastrophe_hits = 0
        first_action_label = None
        last_info = info
        starting_hp = float(info.get("player_hp") or 0.0)
        final_hp = starting_hp
        initial_turn_count = int(info.get("turn_count") or 0)
        opening_turn_saw_disarm = hand_contains_disarm(info)
        disarm_seen_turn = initial_turn_count if opening_turn_saw_disarm else None
        disarm_played_turn = None
        played_disarm_on_opening_turn = False
        played_disarm_on_first_seen_turn = False
        while not done and not truncated:
            mask = env.action_masks()
            current_turn_count = int(last_info.get("turn_count") or info.get("turn_count") or 0)
            if disarm_seen_turn is None and hand_contains_disarm(last_info):
                disarm_seen_turn = current_turn_count
            action, _ = model.predict(obs, deterministic=True, action_masks=mask)
            obs, reward, done, truncated, info = env.step(int(action))
            if first_action_label is None:
                first_action_label = info.get("chosen_action_label")
            if action_is_disarm(info.get("chosen_action_label")):
                if disarm_played_turn is None:
                    disarm_played_turn = current_turn_count
                if opening_turn_saw_disarm and current_turn_count == initial_turn_count:
                    played_disarm_on_opening_turn = True
                if disarm_seen_turn is not None and current_turn_count == disarm_seen_turn:
                    played_disarm_on_first_seen_turn = True
            episode_reward += float(reward)
            if float(info.get("visible_unblocked") or 0.0) >= catastrophe_threshold:
                catastrophe_hits += 1
            final_hp = float(info.get("player_hp") or final_hp)
            last_info = info
        damage_taken = max(starting_hp - final_hp, 0.0)
        rows.append(
            {
                "artifact_context": artifact_context,
                "policy": "ppo_model",
                "episode_index": episode_index,
                "seed_hint": int(seed_hint),
                "outcome": last_info.get("outcome"),
                "episode_reward": round(episode_reward, 4),
                "damage_taken": round(damage_taken, 4),
                "catastrophe_hits": catastrophe_hits,
                "first_action_label": first_action_label,
                "opening_turn_saw_disarm": opening_turn_saw_disarm,
                "played_disarm_on_opening_turn": played_disarm_on_opening_turn,
                "disarm_seen_turn": disarm_seen_turn,
                "disarm_played_turn": disarm_played_turn,
                "played_disarm_on_first_seen_turn": played_disarm_on_first_seen_turn,
            }
        )
    metrics = {
        "episodes": len(rows),
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
        "first_action_counts": dict(Counter(str(row.get("first_action_label") or "None") for row in rows)),
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


def export_model_traces(
    env: GymCombatEnv,
    model: MaskablePPO,
    trace_seeds: list[int],
    artifact_context: dict[str, Any],
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for episode_index, seed_hint in enumerate(trace_seeds):
        obs, info = env.reset(options={"spec_path": str(env.spec_paths[0]), "seed_hint": int(seed_hint)})
        done = False
        truncated = False
        step_index = 0
        while not done and not truncated:
            mask = env.action_masks()
            action_candidates = info.get("action_candidates") or []
            candidates_before = [
                {
                    "index": int(candidate.get("index") or idx),
                    "label": candidate.get("label"),
                    "family": candidate.get("action_family"),
                    "card_name": candidate.get("card_name"),
                    "target": candidate.get("target"),
                }
                for idx, candidate in enumerate(action_candidates)
            ]
            action, _ = model.predict(obs, deterministic=True, action_masks=mask)
            obs, reward, done, truncated, after_info = env.step(int(action))
            rows.append(
                {
                    "artifact_context": artifact_context,
                    "episode_index": episode_index,
                    "seed_hint": int(seed_hint),
                    "step_index": step_index,
                    "turn_count_before": info.get("turn_count"),
                    "player_hp_before": info.get("player_hp"),
                    "player_block_before": info.get("player_block"),
                    "energy_before": info.get("energy"),
                    "visible_incoming_before": info.get("visible_incoming"),
                    "visible_unblocked_before": info.get("visible_unblocked"),
                    "hand_cards_before": info.get("hand_cards"),
                    "action_candidates_before": candidates_before,
                    "chosen_action_label": after_info.get("chosen_action_label"),
                    "reward": round(float(reward), 6),
                    "logged_reward_breakdown": after_info.get("logged_reward_breakdown")
                    or after_info.get("reward_breakdown")
                    or {},
                    "effective_reward_terms": after_info.get("effective_reward_terms") or {},
                    "diagnostic_only_breakdown_keys": after_info.get("diagnostic_only_breakdown_keys") or [],
                    "turn_count_after": after_info.get("turn_count"),
                    "player_hp_after": after_info.get("player_hp"),
                    "player_block_after": after_info.get("player_block"),
                    "energy_after": after_info.get("energy"),
                    "visible_incoming_after": after_info.get("visible_incoming"),
                    "visible_unblocked_after": after_info.get("visible_unblocked"),
                    "hand_cards_after": after_info.get("hand_cards"),
                    "hexaghost_future_script_before": info.get("hexaghost_future_script"),
                    "hexaghost_future_script_after": after_info.get("hexaghost_future_script"),
                    "outcome_after": after_info.get("outcome"),
                    "done": bool(done),
                    "truncated": bool(truncated),
                }
            )
            info = after_info
            step_index += 1
    return rows


def first_state_action_audit(
    env: GymCombatEnv, model: MaskablePPO, start_spec: Path, seed_hint: int, reward_mode: str
) -> dict[str, Any]:
    obs, info = env.reset(options={"spec_path": str(start_spec), "seed_hint": int(seed_hint)})
    mask = env.action_masks()
    obs_tensor, _ = model.policy.obs_to_tensor(obs)
    action_masks = mask.reshape(1, -1)
    dist = model.policy.get_distribution(obs_tensor, action_masks=action_masks)
    probs = dist.distribution.probs.detach().cpu().numpy()[0]
    logits = dist.distribution.logits.detach().cpu().numpy()[0]
    legal_actions = []
    mapping_checks = []
    candidates = info.get("action_candidates") or []

    for index, legal in enumerate(mask.tolist()):
        if not legal:
            continue
        candidate = candidates[index]
        step_env = make_env(start_spec, reward_mode, env.reward_config)
        try:
            _, step_info = step_env.reset(options={"spec_path": str(start_spec), "seed_hint": int(seed_hint)})
            _, reward, done, truncated, after_info = step_env.step(index)
        finally:
            step_env.close()
        breakdown = after_info.get("logged_reward_breakdown") or after_info.get("reward_breakdown") or {}
        effective_terms = after_info.get("effective_reward_terms") or {}
        legal_actions.append(
            {
                "index": index,
                "label": candidate.get("label"),
                "family": candidate.get("action_family"),
                "card_name": candidate.get("card_name"),
                "prob": round(float(probs[index]), 6),
                "logit": round(float(logits[index]), 6),
                "immediate_reward": round(float(reward), 6),
                "done": bool(done),
                "truncated": bool(truncated),
                "visible_unblocked_after": after_info.get("visible_unblocked"),
                "hp_delta": breakdown.get("player_hp_delta"),
                "enemy_hp_delta": breakdown.get("enemy_hp_delta"),
                "incoming_relief": breakdown.get("incoming_relief"),
                "next_enemy_window_hp_loss_baseline": breakdown.get(
                    "next_enemy_window_hp_loss_baseline"
                ),
                "next_enemy_window_hp_loss_after_action": breakdown.get(
                    "next_enemy_window_hp_loss_after_action"
                ),
                "next_enemy_window_relief": breakdown.get("next_enemy_window_relief"),
                "persistent_attack_script_relief": breakdown.get("persistent_attack_script_relief"),
                "persistent_multihit_attack_script_relief": breakdown.get(
                    "persistent_multihit_attack_script_relief"
                ),
                "persistent_attack_windows_affected": breakdown.get(
                    "persistent_attack_windows_affected"
                ),
                "persistent_inferno_damage_prevented": breakdown.get(
                    "persistent_inferno_damage_prevented"
                ),
                "logged_reward_breakdown": breakdown,
                "effective_reward_terms": effective_terms,
                "diagnostic_only_breakdown_keys": after_info.get("diagnostic_only_breakdown_keys") or [],
            }
        )
        mapping_checks.append(
            {
                "index": index,
                "label": candidate.get("label"),
                "step_succeeded": True,
                "after_legal_action_count": after_info.get("legal_action_count"),
            }
        )
    legal_actions.sort(key=lambda row: row["index"])
    return {
        "seed_hint": int(seed_hint),
        "spec_name": info.get("spec_name"),
        "state_summary": {
            "player_hp": info.get("player_hp"),
            "player_block": info.get("player_block"),
            "energy": info.get("energy"),
            "turn_count": info.get("turn_count"),
            "visible_incoming": info.get("visible_incoming"),
            "visible_unblocked": info.get("visible_unblocked"),
            "hand_cards": info.get("hand_cards"),
            "hexaghost_future_script": info.get("hexaghost_future_script"),
        },
        "legal_actions": legal_actions,
        "mapping_checks": mapping_checks,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Audit EndTurn collapse in the minimal combat-local RL experiment.")
    parser.add_argument("--config", required=True, type=Path)
    parser.add_argument("--model", required=True, type=Path)
    parser.add_argument("--seed", default=None, type=int, help="Fixed seed for first-state action audit; defaults to first eval seed.")
    parser.add_argument("--report-out", required=True, type=Path)
    parser.add_argument("--episodes-out", required=True, type=Path)
    parser.add_argument("--trace-out", default=None, type=Path)
    parser.add_argument("--trace-seeds", nargs="*", type=int, default=None)
    args = parser.parse_args()

    config = load_config(args.config)
    start_spec = REPO_ROOT / str(config["start_spec"])
    eval_seeds = [int(value) for value in config["eval_seeds"]]
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
    reward_mode = str(config.get("reward_mode") or "minimal_rl")
    artifact_context = {
        "config_name": str(config["name"]),
        "config_path": str(args.config.resolve()),
        "reward_mode": reward_mode,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "model_path": str(args.model.resolve()),
    }

    model = MaskablePPO.load(str(args.model))
    env = make_env(start_spec, reward_mode, reward_config)
    try:
        model_metrics, model_rows = run_model_family(env, model, eval_seeds, artifact_context)
        random_metrics, random_rows = run_policy_family(
            env, eval_seeds, choose_random, "random_legal", artifact_context
        )
        no_end_metrics, no_end_rows = run_policy_family(
            env, eval_seeds, choose_random_no_first_endturn, "random_no_first_endturn", artifact_context
        )
        proactive_metrics, proactive_rows = run_policy_family(
            env, eval_seeds, choose_simple_proactive, "simple_proactive", artifact_context
        )
        first_seed = int(args.seed if args.seed is not None else eval_seeds[0])
        first_state = first_state_action_audit(env, model, start_spec, first_seed, reward_mode)
        trace_rows: list[dict[str, Any]] = []
        if args.trace_out is not None:
            selected_trace_seeds = [int(value) for value in (args.trace_seeds or eval_seeds)]
            trace_rows = export_model_traces(env, model, selected_trace_seeds, artifact_context)
    finally:
        env.close()

    report = {
        "artifact_context": artifact_context,
        "experiment": str(config["name"]),
        "start_spec": str(start_spec),
        "eval_seed_count": len(eval_seeds),
        "reward_mode": reward_mode,
        "reward_config": reward_config,
        "baseline_metrics": {
            "ppo_model": model_metrics,
            "random_legal": random_metrics,
            "random_no_first_endturn": no_end_metrics,
            "simple_proactive": proactive_metrics,
        },
        "first_state_action_audit": first_state,
        "notes": [
            "compare PPO against small hand-built baselines before changing reward or model scale",
            "first_state_action_audit reports both logged_reward_breakdown and effective_reward_terms",
            "logged_reward_breakdown may include diagnostic-only fields that are not used by the active reward_mode",
            "mapping_checks confirm each legal first action is reachable through the action index interface",
        ],
    }
    rows = model_rows + random_rows + no_end_rows + proactive_rows
    write_json(args.report_out, report)
    write_jsonl(args.episodes_out, rows)
    if args.trace_out is not None:
        write_jsonl(args.trace_out, trace_rows)
    print(json.dumps(report, indent=2, ensure_ascii=False))
    print(f"wrote audit report to {args.report_out}")
    print(f"wrote audit episodes to {args.episodes_out}")
    if args.trace_out is not None:
        print(f"wrote trace rows to {args.trace_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
