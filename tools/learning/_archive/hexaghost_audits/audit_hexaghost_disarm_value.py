#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import random
from pathlib import Path
from typing import Any

import numpy as np

from card_semantics import card_semantics, normalize_card_name
from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from gym_combat_env import GymCombatEnv


def load_config(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def make_env(start_spec: Path) -> GymCombatEnv:
    return GymCombatEnv(
        spec_paths=[start_spec],
        spec_source="start_spec",
        seed=0,
        max_episode_steps=64,
        reward_mode="minimal_rl",
        reward_config={
            "victory_reward": 1.0,
            "defeat_reward": -1.0,
            "hp_loss_scale": 0.02,
            "catastrophe_unblocked_threshold": 18.0,
            "catastrophe_penalty": 0.25,
            "next_enemy_window_relief_scale": 0.0,
            "persistent_attack_script_relief_scale": 0.0,
        },
    )


def observation_from_env(env: GymCombatEnv) -> dict[str, Any]:
    payload = (env._last_response or {}).get("payload") or {}
    return payload.get("observation") or {}


def select_forced_first_action(info: dict[str, Any], keyword: str) -> int:
    candidates = info.get("action_candidates") or []
    keyword_lower = keyword.lower()
    for candidate in candidates:
        label = str(candidate.get("label") or "")
        if keyword_lower in label.lower():
            return int(candidate["index"])
    raise RuntimeError(f"missing first action matching '{keyword}'")


def choose_simple_proactive(mask: np.ndarray, info: dict[str, Any], _rng: random.Random, _step_index: int) -> int:
    legal = [idx for idx, allowed in enumerate(mask.tolist()) if allowed]
    candidates = info.get("action_candidates") or []
    for idx in legal:
        label = str(candidates[idx].get("label") or "")
        if "Disarm" in label:
            return idx
    ranked = []
    for idx in legal:
        candidate = candidates[idx]
        label = str(candidate.get("label") or "")
        if label == "EndTurn":
            continue
        card_name = normalize_card_name(candidate.get("card_name") or "")
        semantics = card_semantics(card_name)
        priority = 99
        if semantics["setup_tag"] > 0:
            priority = 1
        elif "Bash" in card_name or "Shrug" in card_name:
            priority = 2
        elif semantics["attack_tag"] > 0:
            priority = 3
        elif semantics["block_tag"] > 0:
            priority = 4
        ranked.append((priority, idx))
    if ranked:
        ranked.sort()
        return ranked[0][1]
    return legal[0]


def played_card_name(label: str | None) -> str:
    text = str(label or "")
    if not text.startswith("Play #"):
        return ""
    head = text.split(" @", 1)[0]
    parts = head.split(" ", 2)
    if len(parts) < 3:
        return ""
    return normalize_card_name(parts[2])


def action_budget_row_template(forced_first_action: str, seed_hint: int) -> dict[str, Any]:
    row: dict[str, Any] = {
        "forced_first_action": forced_first_action,
        "seed_hint": int(seed_hint),
        "script_future_raw_damage_prevented_total": 0,
        "script_future_multihit_damage_prevented_total": 0,
        "script_future_attack_windows_affected": 0,
        "script_future_inferno_damage_prevented": 0,
        "script_strength_down": 0,
        "script_future_raw_damage_prevented_by_window": [],
        "script_future_windows_after_action": [],
        "defensive_card_plays": 0,
        "defensive_energy_spend": 0.0,
        "attack_card_plays": 0,
        "attack_energy_spend": 0.0,
        "setup_card_plays": 0,
        "play_count": 0,
        "turn_of_lethal": None,
        "survived_until_turn": 0,
        "outcome": "ongoing",
    }
    for window_index in range(1, 5):
        row[f"hp_loss_after_window_{window_index}"] = None
        row[f"monster_hp_after_window_{window_index}"] = None
        row[f"catastrophe_by_window_{window_index}"] = 0
    return row


def finalize_missing_windows(row: dict[str, Any], final_hp_loss: float, final_monster_hp: float) -> None:
    for window_index in range(1, 5):
        if row[f"hp_loss_after_window_{window_index}"] is None:
            row[f"hp_loss_after_window_{window_index}"] = final_hp_loss
        if row[f"monster_hp_after_window_{window_index}"] is None:
            row[f"monster_hp_after_window_{window_index}"] = final_monster_hp


def run_forced_first_action_audit(
    start_spec: Path,
    seeds: list[int],
    forced_keyword: str,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    env = make_env(start_spec)
    rows: list[dict[str, Any]] = []
    catastrophe_threshold = float(env.reward_config["catastrophe_unblocked_threshold"])
    rng = random.Random(23)
    try:
        for seed_hint in seeds:
            _, info = env.reset(options={"spec_path": str(start_spec), "seed_hint": int(seed_hint)})
            start_obs = observation_from_env(env)
            starting_hp = float(start_obs.get("player_hp") or 0.0)
            previous_turn_count = int(start_obs.get("turn_count") or 0)
            row = action_budget_row_template(forced_keyword, seed_hint)

            first_action_index = select_forced_first_action(info, forced_keyword)
            _, _, done, truncated, info = env.step(first_action_index)
            row["first_action_label"] = info.get("chosen_action_label")
            breakdown = info.get("reward_breakdown") or {}
            script_value = breakdown.get("hexaghost_persistent_attack_script") or {}
            row["script_future_raw_damage_prevented_total"] = int(
                script_value.get("future_raw_damage_prevented_total") or 0
            )
            row["script_future_multihit_damage_prevented_total"] = int(
                script_value.get("future_multihit_damage_prevented_total") or 0
            )
            row["script_future_attack_windows_affected"] = int(
                script_value.get("future_attack_windows_affected") or 0
            )
            row["script_future_inferno_damage_prevented"] = int(
                script_value.get("future_inferno_damage_prevented") or 0
            )
            row["script_strength_down"] = int(script_value.get("strength_down") or 0)
            row["script_future_raw_damage_prevented_by_window"] = list(
                script_value.get("future_raw_damage_prevented_by_window") or []
            )
            future_script = script_value.get("future_script_after_action") or {}
            row["script_future_windows_after_action"] = list(future_script.get("windows") or [])

            catastrophe_hits = 0
            window_counter = 0
            step_index = 1
            while not done and not truncated and window_counter < 4:
                chosen_label = str(info.get("chosen_action_label") or "")
                card_name = played_card_name(chosen_label)
                if card_name:
                    semantics = card_semantics(card_name)
                    row["play_count"] += 1
                    cost = float(semantics["base_cost"])
                    if semantics["block_tag"] > 0:
                        row["defensive_card_plays"] += 1
                        row["defensive_energy_spend"] += cost
                    if semantics["attack_tag"] > 0:
                        row["attack_card_plays"] += 1
                        row["attack_energy_spend"] += cost
                    if semantics["setup_tag"] > 0:
                        row["setup_card_plays"] += 1

                mask = env.action_masks()
                action_index = choose_simple_proactive(mask, info, rng, step_index)
                _, _, done, truncated, info = env.step(int(action_index))
                current_obs = observation_from_env(env)
                current_turn_count = int(current_obs.get("turn_count") or 0)
                if float(info.get("visible_unblocked") or 0.0) >= catastrophe_threshold:
                    catastrophe_hits += 1
                if current_turn_count > previous_turn_count:
                    window_counter += 1
                    current_hp = float(current_obs.get("player_hp") or 0.0)
                    current_monster_hp = float(
                        sum(int(monster.get("current_hp") or 0) for monster in (current_obs.get("monsters") or []))
                    )
                    row[f"hp_loss_after_window_{window_counter}"] = max(starting_hp - current_hp, 0.0)
                    row[f"monster_hp_after_window_{window_counter}"] = current_monster_hp
                    row[f"catastrophe_by_window_{window_counter}"] = 1 if catastrophe_hits > 0 else 0
                    previous_turn_count = current_turn_count
                step_index += 1

            final_obs = observation_from_env(env)
            final_hp = float(final_obs.get("player_hp") or 0.0)
            final_monster_hp = float(
                sum(int(monster.get("current_hp") or 0) for monster in (final_obs.get("monsters") or []))
            )
            final_hp_loss = max(starting_hp - final_hp, 0.0)
            finalize_missing_windows(row, final_hp_loss, final_monster_hp)
            row["final_hp_loss"] = final_hp_loss
            row["final_monster_hp"] = final_monster_hp
            row["catastrophe_final"] = 1 if catastrophe_hits > 0 else 0
            row["outcome"] = info.get("outcome")
            row["survived_until_turn"] = int(final_obs.get("turn_count") or 0)
            if info.get("outcome") == "victory":
                row["turn_of_lethal"] = int(final_obs.get("turn_count") or 0)
            rows.append(row)
    finally:
        env.close()

    return summarize_rows(rows), rows


def average(rows: list[dict[str, Any]], key: str) -> float:
    values = [float(row.get(key) or 0.0) for row in rows]
    return float(np.mean(values)) if values else 0.0


def maximum(rows: list[dict[str, Any]], key: str) -> float:
    values = [float(row.get(key) or 0.0) for row in rows]
    return float(max(values)) if values else 0.0


def summarize_rows(rows: list[dict[str, Any]]) -> dict[str, Any]:
    summary: dict[str, Any] = {
        "episodes": len(rows),
        "pass_rate": average(
            [{"victory_flag": 1.0 if row.get("outcome") == "victory" else 0.0} for row in rows],
            "victory_flag",
        ),
        "avg_script_future_raw_damage_prevented_total": average(rows, "script_future_raw_damage_prevented_total"),
        "avg_script_future_multihit_damage_prevented_total": average(
            rows, "script_future_multihit_damage_prevented_total"
        ),
        "avg_script_future_attack_windows_affected": average(rows, "script_future_attack_windows_affected"),
        "avg_script_future_inferno_damage_prevented": average(rows, "script_future_inferno_damage_prevented"),
        "avg_defensive_card_plays": average(rows, "defensive_card_plays"),
        "avg_defensive_energy_spend": average(rows, "defensive_energy_spend"),
        "avg_attack_card_plays": average(rows, "attack_card_plays"),
        "avg_attack_energy_spend": average(rows, "attack_energy_spend"),
        "avg_setup_card_plays": average(rows, "setup_card_plays"),
        "worst_final_hp_loss": maximum(rows, "final_hp_loss"),
    }
    for window_index in range(1, 5):
        summary[f"avg_hp_loss_after_window_{window_index}"] = average(
            rows, f"hp_loss_after_window_{window_index}"
        )
        summary[f"worst_hp_loss_after_window_{window_index}"] = maximum(
            rows, f"hp_loss_after_window_{window_index}"
        )
        summary[f"avg_monster_hp_after_window_{window_index}"] = average(
            rows, f"monster_hp_after_window_{window_index}"
        )
        summary[f"catastrophe_rate_by_window_{window_index}"] = average(
            rows, f"catastrophe_by_window_{window_index}"
        )
    return summary


def pairwise_summary(rows_a: list[dict[str, Any]], rows_b: list[dict[str, Any]]) -> dict[str, Any]:
    return {
        "delta_avg_script_future_raw_damage_prevented_total": average(
            rows_a, "script_future_raw_damage_prevented_total"
        )
        - average(rows_b, "script_future_raw_damage_prevented_total"),
        "delta_avg_script_future_multihit_damage_prevented_total": average(
            rows_a, "script_future_multihit_damage_prevented_total"
        )
        - average(rows_b, "script_future_multihit_damage_prevented_total"),
        "delta_avg_final_hp_loss": average(rows_a, "final_hp_loss") - average(rows_b, "final_hp_loss"),
        "delta_worst_final_hp_loss": maximum(rows_a, "final_hp_loss") - maximum(rows_b, "final_hp_loss"),
        "delta_avg_defensive_card_plays": average(rows_a, "defensive_card_plays")
        - average(rows_b, "defensive_card_plays"),
        "delta_avg_attack_card_plays": average(rows_a, "attack_card_plays")
        - average(rows_b, "attack_card_plays"),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Offline Hexaghost Disarm persistent-value audit.")
    parser.add_argument("--config", required=True, type=Path)
    parser.add_argument("--actions", default="Disarm,Bash,Defend")
    parser.add_argument("--summary-out", required=True, type=Path)
    parser.add_argument("--episodes-out", required=True, type=Path)
    args = parser.parse_args()

    config = load_config(args.config)
    start_spec = REPO_ROOT / str(config["start_spec"])
    seeds = [int(value) for value in config["eval_seeds"]]
    actions = [part.strip() for part in str(args.actions).split(",") if part.strip()]

    summary_by_action: dict[str, Any] = {}
    rows_by_action: dict[str, list[dict[str, Any]]] = {}
    all_rows: list[dict[str, Any]] = []
    for action in actions:
        summary, rows = run_forced_first_action_audit(start_spec, seeds, action)
        summary_by_action[action] = summary
        rows_by_action[action] = rows
        all_rows.extend(rows)

    report = {
        "experiment": str(config["name"]),
        "start_spec": str(start_spec),
        "eval_seed_count": len(seeds),
        "forced_actions": actions,
        "primary_metrics": [
            "script_future_raw_damage_prevented_total",
            "script_future_multihit_damage_prevented_total",
            "hp_loss_after_window_1..4",
            "final_hp_loss",
            "catastrophe_by_window_1..4",
            "worst_final_hp_loss",
        ],
        "secondary_metrics": [
            "monster_hp_after_window_1..4",
            "turn_of_lethal",
            "defensive_card_plays",
            "attack_card_plays",
        ],
        "summary_by_action": summary_by_action,
        "pairwise": {
            "disarm_vs_bash": pairwise_summary(
                rows_by_action.get("Disarm", []), rows_by_action.get("Bash", [])
            ),
            "disarm_vs_defend": pairwise_summary(
                rows_by_action.get("Disarm", []), rows_by_action.get("Defend", [])
            ),
        },
        "notes": [
            "script-layer metrics come from Rust-side Hexaghost future-script export",
            "windows 1..4 use a shared simple proactive continuation policy after the forced first action",
            "primary metrics are hp-loss, catastrophe, worst-case, and future prevented damage",
            "secondary metrics are monster hp progression, turn of lethal, and realized action-budget usage",
        ],
    }
    write_json(args.summary_out, report)
    write_jsonl(args.episodes_out, all_rows)
    print(json.dumps(report, indent=2, ensure_ascii=False))
    print(f"wrote Hexaghost Disarm summary to {args.summary_out}")
    print(f"wrote Hexaghost Disarm episodes to {args.episodes_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
