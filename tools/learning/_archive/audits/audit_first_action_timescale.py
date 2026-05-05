#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import random
from collections import defaultdict
from pathlib import Path
from typing import Any

import numpy as np

from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from gym_combat_env import GymCombatEnv


def load_config(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


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
        card_name = str(candidate.get("card_name") or "")
        family = str(candidate.get("action_family") or "")
        priority = 99
        if family == "play_card":
            if "Bash" in card_name:
                priority = 1
            elif "Shrug" in card_name:
                priority = 2
            elif "Defend" in card_name:
                priority = 3
            else:
                priority = 4
        ranked.append((priority, idx))
    if ranked:
        ranked.sort()
        return ranked[0][1]
    return legal[0]


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


def summarize_rows(rows: list[dict[str, Any]]) -> dict[str, Any]:
    def average(key: str) -> float:
        return float(np.mean([float(row.get(key) or 0.0) for row in rows])) if rows else 0.0

    def maximum(key: str) -> float:
        return float(max((float(row.get(key) or 0.0) for row in rows), default=0.0))

    return {
        "episodes": len(rows),
        "pass_rate": average("victory_flag"),
        "avg_hp_loss_after_window_1": average("hp_loss_after_window_1"),
        "avg_hp_loss_after_window_2": average("hp_loss_after_window_2"),
        "avg_final_hp_loss": average("final_hp_loss"),
        "max_hp_loss_after_window_1": maximum("hp_loss_after_window_1"),
        "max_hp_loss_after_window_2": maximum("hp_loss_after_window_2"),
        "max_final_hp_loss": maximum("final_hp_loss"),
        "avg_monster_hp_after_window_1": average("monster_hp_after_window_1"),
        "avg_monster_hp_after_window_2": average("monster_hp_after_window_2"),
        "avg_final_monster_hp": average("final_monster_hp"),
        "catastrophe_rate_window_1": average("catastrophe_window_1"),
        "catastrophe_rate_window_2": average("catastrophe_window_2"),
        "catastrophe_rate_final": average("catastrophe_final"),
    }


def run_forced_first_action_audit(
    start_spec: Path,
    seeds: list[int],
    forced_keyword: str,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    env = GymCombatEnv(
        spec_paths=[start_spec],
        spec_source="start_spec",
        seed=0,
        max_episode_steps=64,
        reward_mode="minimal_rl",
    )
    rows: list[dict[str, Any]] = []
    catastrophe_threshold = float(env.reward_config["catastrophe_unblocked_threshold"])
    rng = random.Random(19)
    try:
        for episode_index, seed_hint in enumerate(seeds):
            _, info = env.reset(options={"spec_path": str(start_spec), "seed_hint": int(seed_hint)})
            start_obs = observation_from_env(env)
            starting_hp = float(start_obs.get("player_hp") or 0.0)
            first_action_index = select_forced_first_action(info, forced_keyword)
            _, _, done, truncated, info = env.step(first_action_index)
            first_action_label = info.get("chosen_action_label")

            window_counter = 0
            previous_turn_count = int(observation_from_env(env).get("turn_count") or 0)
            catastrophe_hits = 0
            hp_loss_after_window_1 = None
            hp_loss_after_window_2 = None
            monster_hp_after_window_1 = None
            monster_hp_after_window_2 = None
            catastrophe_window_1 = 0
            catastrophe_window_2 = 0

            while not done and not truncated and window_counter < 2:
                mask = env.action_masks()
                action_index = choose_simple_proactive(mask, info, rng, window_counter)
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
                    hp_loss = max(starting_hp - current_hp, 0.0)
                    if window_counter == 1:
                        hp_loss_after_window_1 = hp_loss
                        monster_hp_after_window_1 = current_monster_hp
                        catastrophe_window_1 = 1 if catastrophe_hits > 0 else 0
                    elif window_counter == 2:
                        hp_loss_after_window_2 = hp_loss
                        monster_hp_after_window_2 = current_monster_hp
                        catastrophe_window_2 = 1 if catastrophe_hits > 0 else 0
                    previous_turn_count = current_turn_count

            final_obs = observation_from_env(env)
            final_hp = float(final_obs.get("player_hp") or 0.0)
            final_monster_hp = float(
                sum(int(monster.get("current_hp") or 0) for monster in (final_obs.get("monsters") or []))
            )
            rows.append(
                {
                    "forced_first_action": forced_keyword,
                    "episode_index": episode_index,
                    "seed_hint": int(seed_hint),
                    "first_action_label": first_action_label,
                    "outcome": info.get("outcome"),
                    "victory_flag": 1.0 if info.get("outcome") == "victory" else 0.0,
                    "hp_loss_after_window_1": hp_loss_after_window_1
                    if hp_loss_after_window_1 is not None
                    else max(starting_hp - final_hp, 0.0),
                    "hp_loss_after_window_2": hp_loss_after_window_2
                    if hp_loss_after_window_2 is not None
                    else max(starting_hp - final_hp, 0.0),
                    "final_hp_loss": max(starting_hp - final_hp, 0.0),
                    "monster_hp_after_window_1": monster_hp_after_window_1
                    if monster_hp_after_window_1 is not None
                    else final_monster_hp,
                    "monster_hp_after_window_2": monster_hp_after_window_2
                    if monster_hp_after_window_2 is not None
                    else final_monster_hp,
                    "final_monster_hp": final_monster_hp,
                    "catastrophe_window_1": catastrophe_window_1,
                    "catastrophe_window_2": catastrophe_window_2,
                    "catastrophe_final": 1 if catastrophe_hits > 0 else 0,
                }
            )
    finally:
        env.close()
    return summarize_rows(rows), rows


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Locate the time scale at which a forced first action starts to matter."
    )
    parser.add_argument("--config", required=True, type=Path)
    parser.add_argument(
        "--actions",
        default="Disarm,Bash,Defend",
        help="Comma-separated first-action keywords to force.",
    )
    parser.add_argument("--report-out", required=True, type=Path)
    parser.add_argument("--episodes-out", required=True, type=Path)
    args = parser.parse_args()

    config = load_config(args.config)
    start_spec = REPO_ROOT / str(config["start_spec"])
    seeds = [int(value) for value in config["eval_seeds"]]
    actions = [part.strip() for part in str(args.actions).split(",") if part.strip()]

    summaries: dict[str, Any] = {}
    all_rows: list[dict[str, Any]] = []
    for action in actions:
        summary, rows = run_forced_first_action_audit(start_spec, seeds, action)
        summaries[action] = summary
        all_rows.extend(rows)

    report = {
        "experiment": str(config["name"]),
        "start_spec": str(start_spec),
        "eval_seed_count": len(seeds),
        "forced_actions": actions,
        "summary_by_action": summaries,
        "notes": [
            "each run forces the first action, then follows a shared simple proactive policy",
            "window metrics are recorded after the first and second enemy windows",
            "goal is diagnosis, not policy evaluation",
        ],
    }
    write_json(args.report_out, report)
    write_jsonl(args.episodes_out, all_rows)
    print(json.dumps(report, indent=2, ensure_ascii=False))
    print(f"wrote timescale audit report to {args.report_out}")
    print(f"wrote timescale audit episodes to {args.episodes_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
