#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
import random
from pathlib import Path
from typing import Any

import numpy as np

from card_semantics import card_semantics, normalize_card_name
from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from gym_combat_env import GymCombatEnv

MAJOR_WINDOWS = ("Divider", "Tackle")


def make_env(start_spec: Path) -> GymCombatEnv:
    return GymCombatEnv(
        spec_paths=[start_spec],
        spec_source="start_spec",
        seed=0,
        max_episode_steps=96,
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


def action_candidates(info: dict[str, Any]) -> list[dict[str, Any]]:
    return list(info.get("action_candidates") or [])


def legal_action_index(info: dict[str, Any], keyword: str) -> int | None:
    keyword_lower = keyword.lower()
    for candidate in action_candidates(info):
        label = str(candidate.get("label") or "")
        if keyword_lower in label.lower():
            return int(candidate["index"])
    return None


def exact_action_index(info: dict[str, Any], label: str) -> int:
    for candidate in action_candidates(info):
        if str(candidate.get("label") or "") == label:
            return int(candidate["index"])
    raise RuntimeError(f"missing replay action '{label}'")


def next_major_window(info: dict[str, Any]) -> str | None:
    script = info.get("hexaghost_future_script") or {}
    for window in script.get("windows") or []:
        move_kind = str(window.get("move_kind") or "")
        if move_kind in MAJOR_WINDOWS:
            return move_kind
    return None


def current_hand_names(info: dict[str, Any]) -> list[str]:
    return [normalize_card_name(name) for name in (info.get("hand_cards") or [])]


def played_card_name(label: str | None) -> str:
    text = str(label or "")
    if not text.startswith("Play #"):
        return ""
    head = text.split(" @", 1)[0]
    parts = head.split(" ", 2)
    if len(parts) < 3:
        return ""
    return normalize_card_name(parts[2])


def choose_shared_action(info: dict[str, Any], rng: random.Random, _step_index: int) -> int:
    candidates = action_candidates(info)
    legal = [int(candidate["index"]) for candidate in candidates]
    ranked: list[tuple[int, int]] = []
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


def percentile_mean(values: list[float], fraction: float) -> float:
    if not values:
        return 0.0
    count = max(1, int(math.ceil(len(values) * fraction)))
    worst = sorted(values, reverse=True)[:count]
    return float(np.mean(worst))


def chance_outcome_key(reachable: bool, playable: bool) -> str:
    if not reachable:
        return "unavailable_before_window_1"
    if not playable:
        return "available_but_unplayable_before_window_1"
    return "available_and_playable_before_window_1"


def capture_window_metrics(
    info: dict[str, Any],
    start_hp: float,
    catastrophe_seen: bool,
    metrics: dict[str, Any],
) -> None:
    obs = {
        "player_hp": info.get("player_hp"),
        "monster_states": info.get("monster_states") or [],
    }
    current_hp = float(obs.get("player_hp") or 0.0)
    current_monster_hp = float(
        sum(int(monster.get("current_hp") or 0) for monster in (obs.get("monster_states") or []))
    )
    loss = max(start_hp - current_hp, 0.0)
    if metrics.get("hp_loss_to_window_1") is None:
        metrics["hp_loss_to_window_1"] = loss
        metrics["monster_hp_to_window_1"] = current_monster_hp
        metrics["catastrophe_to_window_1"] = 1 if catastrophe_seen else 0
    elif metrics.get("hp_loss_to_window_2") is None:
        metrics["hp_loss_to_window_2"] = loss
        metrics["monster_hp_to_window_2"] = current_monster_hp
        metrics["catastrophe_to_window_2"] = 1 if catastrophe_seen else 0


def run_until_window_2(
    env: GymCombatEnv,
    start_spec: Path,
    seed_hint: int,
    prefix_labels: list[str],
    branch_action_label: str | None,
) -> dict[str, Any]:
    _, info = env.reset(options={"spec_path": str(start_spec), "seed_hint": int(seed_hint)})
    for label in prefix_labels:
        idx = exact_action_index(info, label)
        _, _, done, truncated, info = env.step(idx)
        if done or truncated:
            raise RuntimeError("branch replay terminated before branch point")

    starting_hp = float(info.get("player_hp") or 0.0)
    metrics: dict[str, Any] = {
        "seed_hint": int(seed_hint),
        "branch_action_label": branch_action_label,
        "hp_loss_to_window_1": None,
        "hp_loss_to_window_2": None,
        "monster_hp_to_window_1": None,
        "monster_hp_to_window_2": None,
        "catastrophe_to_window_1": 0,
        "catastrophe_to_window_2": 0,
        "final_hp_loss": None,
        "final_monster_hp": None,
        "outcome": None,
    }

    previous_major = next_major_window(info)
    catastrophe_seen = False
    step_index = 0
    if branch_action_label is not None:
        idx = exact_action_index(info, branch_action_label)
        _, _, done, truncated, info = env.step(idx)
        if float(info.get("visible_unblocked") or 0.0) >= float(env.reward_config["catastrophe_unblocked_threshold"]):
            catastrophe_seen = True
        current_major = next_major_window(info)
        if previous_major == "Divider" and current_major != "Divider":
            capture_window_metrics(info, starting_hp, catastrophe_seen, metrics)
        elif previous_major == "Tackle" and current_major != "Tackle":
            capture_window_metrics(info, starting_hp, catastrophe_seen, metrics)
        previous_major = current_major
        step_index += 1
    else:
        done = False
        truncated = False

    rng = random.Random(23)
    while not done and not truncated and metrics["hp_loss_to_window_2"] is None:
        idx = choose_shared_action(info, rng, step_index)
        _, _, done, truncated, info = env.step(idx)
        if float(info.get("visible_unblocked") or 0.0) >= float(env.reward_config["catastrophe_unblocked_threshold"]):
            catastrophe_seen = True
        current_major = next_major_window(info)
        if previous_major == "Divider" and current_major != "Divider":
            capture_window_metrics(info, starting_hp, catastrophe_seen, metrics)
        elif previous_major == "Tackle" and current_major != "Tackle":
            capture_window_metrics(info, starting_hp, catastrophe_seen, metrics)
        previous_major = current_major
        step_index += 1

    final_hp = float(info.get("player_hp") or 0.0)
    final_monster_hp = float(
        sum(int(monster.get("current_hp") or 0) for monster in (info.get("monster_states") or []))
    )
    metrics["final_hp_loss"] = max(starting_hp - final_hp, 0.0)
    metrics["final_monster_hp"] = final_monster_hp
    metrics["outcome"] = info.get("outcome")
    if metrics["hp_loss_to_window_1"] is None:
        metrics["hp_loss_to_window_1"] = metrics["final_hp_loss"]
        metrics["monster_hp_to_window_1"] = final_monster_hp
        metrics["catastrophe_to_window_1"] = 1 if catastrophe_seen else 0
    if metrics["hp_loss_to_window_2"] is None:
        metrics["hp_loss_to_window_2"] = metrics["final_hp_loss"]
        metrics["monster_hp_to_window_2"] = final_monster_hp
        metrics["catastrophe_to_window_2"] = 1 if catastrophe_seen else 0
    return metrics


def discover_first_playable_point(
    env: GymCombatEnv,
    start_spec: Path,
    seed_hint: int,
    resource_keyword: str,
) -> dict[str, Any]:
    _, info = env.reset(options={"spec_path": str(start_spec), "seed_hint": int(seed_hint)})
    prefix_labels: list[str] = []
    reachable = False
    playable = False
    branch_action_label: str | None = None
    branch_point_prefix: list[str] = []
    rng = random.Random(23)
    step_index = 0

    done = False
    truncated = False
    while not done and not truncated and next_major_window(info) == "Divider":
        hand = current_hand_names(info)
        if resource_keyword.lower() in [name.lower() for name in hand]:
            reachable = True
        idx = legal_action_index(info, resource_keyword)
        if idx is not None:
            playable = True
            branch_action_label = str(action_candidates(info)[idx].get("label") or "")
            branch_point_prefix = list(prefix_labels)
            break
        idx = choose_shared_action(info, rng, step_index)
        _, _, done, truncated, info = env.step(idx)
        prefix_labels.append(str(info.get("chosen_action_label") or ""))
        step_index += 1

    return {
        "seed_hint": int(seed_hint),
        "reachable_before_window_1": reachable,
        "playable_when_reached_before_window_1": playable,
        "branch_point_prefix": branch_point_prefix,
        "resource_branch_action_label": branch_action_label,
        "opportunity_state": chance_outcome_key(reachable, playable),
    }


def summarize_branch_rows(rows: list[dict[str, Any]]) -> dict[str, Any]:
    hp1 = [float(row["hp_loss_to_window_1"]) for row in rows]
    hp2 = [float(row["hp_loss_to_window_2"]) for row in rows]
    return {
        "episodes": len(rows),
        "mean_hp_loss_to_window_1": float(np.mean(hp1)) if hp1 else 0.0,
        "mean_hp_loss_to_window_2": float(np.mean(hp2)) if hp2 else 0.0,
        "worst_20p_hp_loss_to_window_2": percentile_mean(hp2, 0.2),
        "catastrophe_rate_to_window_2": float(np.mean([float(row["catastrophe_to_window_2"]) for row in rows]))
        if rows
        else 0.0,
        "mean_monster_hp_to_window_1": float(np.mean([float(row["monster_hp_to_window_1"]) for row in rows]))
        if rows
        else 0.0,
        "mean_monster_hp_to_window_2": float(np.mean([float(row["monster_hp_to_window_2"]) for row in rows]))
        if rows
        else 0.0,
    }


def label_value_timing(resource_summary: dict[str, Any], compare_summary: dict[str, Any]) -> str:
    delta_window_1 = float(compare_summary["mean_hp_loss_to_window_1"]) - float(
        resource_summary["mean_hp_loss_to_window_1"]
    )
    delta_window_2 = float(compare_summary["mean_hp_loss_to_window_2"]) - float(
        resource_summary["mean_hp_loss_to_window_2"]
    )
    delta_tail = float(compare_summary["worst_20p_hp_loss_to_window_2"]) - float(
        resource_summary["worst_20p_hp_loss_to_window_2"]
    )
    if delta_window_1 >= 1.0:
        return "immediate_by_window_1"
    if delta_tail >= 1.0 and abs(delta_window_2) < 1.0:
        return "mostly_tail_risk_by_window_2"
    return "delayed_to_window_2"


def main() -> int:
    parser = argparse.ArgumentParser(description="Minimal Hexaghost mitigation event-bucket audit.")
    parser.add_argument("--start-spec", required=True, type=Path)
    parser.add_argument("--seeds", nargs="+", required=True, type=int)
    parser.add_argument("--resource", required=True)
    parser.add_argument("--compare-actions", default="Bash,Defend")
    parser.add_argument("--summary-out", required=True, type=Path)
    parser.add_argument("--episodes-out", required=True, type=Path)
    args = parser.parse_args()

    start_spec = (REPO_ROOT / args.start_spec).resolve() if not args.start_spec.is_absolute() else args.start_spec
    resource = str(args.resource).strip()
    compare_keywords = [part.strip() for part in str(args.compare_actions).split(",") if part.strip()]

    env = make_env(start_spec)
    discovery_rows: list[dict[str, Any]] = []
    baseline_name = "SharedPolicy"
    branch_rows_by_action: dict[str, list[dict[str, Any]]] = {resource: [], baseline_name: []}
    for action in compare_keywords:
        branch_rows_by_action[action] = []

    try:
        for seed_hint in [int(v) for v in args.seeds]:
            discovery = discover_first_playable_point(env, start_spec, seed_hint, resource)
            discovery_rows.append(discovery)
            if not discovery["playable_when_reached_before_window_1"]:
                continue

            prefix = list(discovery["branch_point_prefix"])
            resource_label = str(discovery["resource_branch_action_label"] or "")
            branch_rows_by_action[resource].append(
                run_until_window_2(env, start_spec, seed_hint, prefix, resource_label)
            )
            branch_rows_by_action[baseline_name].append(
                run_until_window_2(env, start_spec, seed_hint, prefix, None)
            )
            for action in compare_keywords:
                _, info = env.reset(options={"spec_path": str(start_spec), "seed_hint": int(seed_hint)})
                for label in prefix:
                    idx = exact_action_index(info, label)
                    _, _, done, truncated, info = env.step(idx)
                    if done or truncated:
                        raise RuntimeError("comparison replay terminated before branch point")
                compare_idx = legal_action_index(info, action)
                if compare_idx is None:
                    continue
                compare_label = str(action_candidates(info)[compare_idx].get("label") or "")
                branch_rows_by_action[action].append(
                    run_until_window_2(env, start_spec, seed_hint, prefix, compare_label)
                )
    finally:
        env.close()

    reachable_rate = float(
        np.mean([1.0 if row["reachable_before_window_1"] else 0.0 for row in discovery_rows])
    )
    playable_rate = float(
        np.mean([1.0 if row["playable_when_reached_before_window_1"] else 0.0 for row in discovery_rows])
    )
    opportunity_counts: dict[str, int] = {}
    for row in discovery_rows:
        key = str(row["opportunity_state"])
        opportunity_counts[key] = opportunity_counts.get(key, 0) + 1

    summary_by_action = {
        name: summarize_branch_rows(rows) for name, rows in branch_rows_by_action.items() if rows
    }
    label_against: dict[str, str] = {}
    if resource in summary_by_action and baseline_name in summary_by_action:
        label_against[baseline_name] = label_value_timing(
            summary_by_action[resource], summary_by_action[baseline_name]
        )
        for action in compare_keywords:
            if action in summary_by_action:
                label_against[action] = label_value_timing(summary_by_action[resource], summary_by_action[action])

    report = {
        "experiment": f"hexaghost_{normalize_card_name(resource).lower()}_event_buckets_v1",
        "start_spec": str(start_spec),
        "seed_count": len(discovery_rows),
        "resource": resource,
        "window_1": "first Divider",
        "window_2": "first Tackle after that Divider",
        "primary_metrics": [
            "P(reachable_before_window_1)",
            "P(playable_when_reached_before_window_1)",
            "mean_hp_loss_to_window_1",
            "mean_hp_loss_to_window_2",
            "worst_20p_hp_loss_to_window_2",
        ],
        "optional_metric": "catastrophe_rate_to_window_2",
        "opportunity_state_summary": {
            "P(reachable_before_window_1)": reachable_rate,
            "P(playable_when_reached_before_window_1)": playable_rate,
            "counts": opportunity_counts,
        },
        "summary_by_action": summary_by_action,
        "value_timing_labels_against_comparators": label_against,
        "notes": [
            "reachability/playability are evaluated over the fixed natural-start deck, fixed encounter, and explicit seed set",
            "playable means only physically playable before window_1; it does not include broader opportunity-cost worth-it logic",
            "value timing summaries are conditional on seeds where the resource was playable before window_1",
            "worst_20p_hp_loss_to_window_2 is the mean of the worst 20% hp-loss outcomes over that conditional subset",
        ],
    }

    episode_rows: list[dict[str, Any]] = []
    for discovery in discovery_rows:
        row = dict(discovery)
        for action_name, rows in branch_rows_by_action.items():
            match = next((branch for branch in rows if int(branch["seed_hint"]) == int(discovery["seed_hint"])), None)
            if match is not None:
                row[f"{action_name.lower()}_metrics"] = match
        episode_rows.append(row)

    write_json(args.summary_out, report)
    write_jsonl(args.episodes_out, episode_rows)
    print(json.dumps(report, indent=2, ensure_ascii=False))
    print(f"wrote mitigation bucket summary to {args.summary_out}")
    print(f"wrote mitigation bucket episodes to {args.episodes_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
