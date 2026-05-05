#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
import random
from collections import Counter, defaultdict
from pathlib import Path
from statistics import mean, pvariance
from typing import Any

from card_semantics import card_semantics
from combat_rl_common import REPO_ROOT, iter_jsonl, write_json, write_jsonl
from combat_reranker_common import (
    curriculum_tag_from_spec_name,
    normalize_outcome,
    parse_move_label,
    sample_tags_from_preference_sample,
    stable_split,
)
from curriculum_dynamic_teacher import dynamic_teacher_for_row
from gym_combat_env import CombatEnvDriver
from q_local_common import (
    aggregate_q_local_score,
    candidate_semantics_from_move,
    chance_feature_dict,
    eval_bucket_from_row,
    observation_to_snapshot,
)


def load_rows(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [row for _, row in iter_jsonl(path)]


def load_transition_rows(dataset_dir: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for split in ("train", "val", "test"):
        rows.extend(load_rows(dataset_dir / f"combat_transition_{split}.jsonl"))
    return [row for row in rows if str(row.get("sample_origin") or "") == "combat_lab_spec"]


def load_policy_index(dataset_dir: Path) -> dict[tuple[str, int, int], dict[str, Any]]:
    groups: dict[tuple[str, int, int], dict[str, Any]] = {}
    for split in ("train", "val", "test"):
        for row in load_rows(dataset_dir / f"combat_policy_{split}.jsonl"):
            if str(row.get("sample_origin") or "") != "combat_lab_spec":
                continue
            spec_name = str(row.get("spec_name") or "")
            episode_id = int(row.get("episode_id") or 0)
            step_index = int(row.get("step_index") or 0)
            key = (spec_name, episode_id, step_index)
            group = groups.setdefault(
                key,
                {
                    "baseline_action": str(row.get("baseline_action") or ""),
                    "candidate_by_move": {},
                },
            )
            group["candidate_by_move"][str(row.get("candidate_move") or "")] = row
    return groups


def candidate_map_from_trace_step(step: dict[str, Any]) -> dict[str, dict[str, Any]]:
    candidate_by_move: dict[str, dict[str, Any]] = {}
    legal_candidates = (
        ((step.get("head_summary") or {}).get("legal_candidate_topk") or [])
        if isinstance(step, dict)
        else []
    )
    for rank, candidate in enumerate(legal_candidates):
        move_label = str(candidate.get("label") or "")
        if not move_label:
            continue
        candidate_by_move[move_label] = {
            "candidate_rank": rank,
            "candidate_score_hint": float(candidate.get("derived_probability") or 0.0),
            "candidate_search_order_score": float(
                candidate.get("derived_probability") or 0.0
            ),
            "action_family": candidate.get("action_family"),
            "canonical": candidate.get("canonical"),
        }
    return candidate_by_move


def choose_action_index(
    candidates: list[dict[str, Any]],
    action_mask: list[bool],
    action_label: str,
) -> int:
    for index, candidate in enumerate(candidates):
        if not action_mask[index]:
            continue
        if str(candidate.get("label") or "") == action_label:
            return index
    available = [
        str(candidate.get("label") or "")
        for index, candidate in enumerate(candidates)
        if index < len(action_mask) and action_mask[index]
    ]
    raise KeyError(
        f"failed to match action '{action_label}' against legal actions: {available}"
    )


def infer_replay_curriculum_tag(
    baseline_action: str,
    preferred_action: str | None,
    sample_tags: list[str],
) -> str:
    tags = set(str(tag) for tag in sample_tags)
    preferred_label = str(preferred_action or "")
    preferred_semantics = card_semantics(parse_move_label(preferred_label).get("card_name"))
    baseline_semantics = card_semantics(parse_move_label(baseline_action).get("card_name"))
    if "setup_flex_missed" in tags or (
        float(preferred_semantics["setup_tag"]) > 0.0
        and float(baseline_semantics["setup_tag"]) <= 0.0
    ):
        return "setup_before_payoff"
    if "status_exhaust_management" in tags or float(preferred_semantics["consumes_status"]) > 0.0:
        return "status_exhaust_draw"
    if "oracle_save" in tags:
        return "survival_override"
    if "overdefend_light_pressure" in tags or "kill_now_missed" in tags:
        return "attack_over_defend"
    if float(preferred_semantics["attack_tag"]) > 0.0 and "Defend" in baseline_action:
        return "attack_over_defend"
    return "attack_over_defend"


def dynamic_teacher_from_live_state(
    observation: dict[str, Any],
    legal_moves: list[str],
    *,
    baseline_action: str,
    candidate_by_move: dict[str, Any],
    sample_tags: list[str],
) -> dict[str, Any]:
    pressure = (observation.get("pressure") or {}) if isinstance(observation, dict) else {}
    snapshot = observation_to_snapshot(observation)
    normalized_candidates = []
    for index, move_label in enumerate(legal_moves):
        policy_row = dict(candidate_by_move.get(move_label) or {})
        normalized_candidates.append(
            {
                "move_label": move_label,
                "score": float(
                    policy_row.get("candidate_score_hint")
                    or policy_row.get("candidate_search_order_score")
                    or 0.0
                ),
                "search_rank": int(policy_row.get("candidate_rank") or index),
            }
        )
    row = {
        "chosen_move": baseline_action,
        "sample_tags": list(sample_tags or []),
        "snapshot_normalized_state": snapshot,
        "state_features_preview": {
            "incoming_damage": int(pressure.get("visible_incoming") or pressure.get("value_incoming") or 0.0),
            "player_hp": int(observation.get("player_hp") or 0),
            "player_block": int(observation.get("player_block") or 0),
            "remaining_monster_hp": sum(int(monster.get("current_hp") or 0) for monster in (observation.get("monsters") or [])),
        },
        "state_features_full": {
            "unblocked_incoming": float(
                pressure.get("visible_unblocked")
                or pressure.get("value_unblocked")
                or 0.0
            ),
        },
        "normalized_candidates": normalized_candidates,
    }
    return dynamic_teacher_for_row(row)


def rollout_action_score(
    candidate: dict[str, Any],
    observation: dict[str, Any],
    curriculum_tag: str,
    *,
    setup_active: bool,
    rng: random.Random,
) -> float:
    label = str(candidate.get("label") or "")
    semantics = card_semantics(candidate.get("card_name") or label)
    monsters = list(observation.get("monsters") or [])
    player_hp = float(observation.get("player_hp") or 0.0)
    incoming = 0.0
    for monster in monsters:
        intent = str(monster.get("visible_intent") or "").lower()
        if "attack" in intent:
            digits = [
                int(part)
                for part in "".join(ch if ch.isdigit() else " " for ch in intent).split()
                if part.isdigit()
            ]
            if digits:
                incoming += float(math.prod(digits) if len(digits) > 1 else digits[0])
    lowest_hp = min(
        (
            float(monster.get("current_hp") or 0.0)
            for monster in monsters
            if float(monster.get("current_hp") or 0.0) > 0.0
        ),
        default=999.0,
    )
    score = 0.0
    if label == "EndTurn":
        score -= 4.0
    score += 0.45 * float(semantics["base_damage"])
    score += 0.35 * float(semantics["base_block"]) * min(1.0, incoming / 12.0)
    score += 0.40 * float(semantics["draw_count"])
    score += 0.65 * float(semantics["apply_strength"])
    score += 0.35 * float(semantics["apply_vulnerable"])
    score -= 0.35 * float(semantics["creates_status"])
    score -= 0.15 * float(semantics["ethereal"])
    if (
        float(semantics["attack_tag"]) > 0.0
        and lowest_hp <= float(semantics["base_damage"]) + 1.0
    ):
        score += 2.5
    if curriculum_tag == "setup_before_payoff":
        if float(semantics["setup_tag"]) > 0.0 and incoming <= 8.0:
            score += 2.0
        if setup_active and float(semantics["payoff_tag"]) > 0.0:
            score += 2.2
        if not setup_active and ("Defend" in label or label == "EndTurn") and incoming <= 5.0:
            score -= 1.5
    if (
        curriculum_tag == "attack_over_defend"
        and float(semantics["attack_tag"]) > 0.0
        and incoming <= max(player_hp * 0.2, 6.0)
    ):
        score += 1.2
    if curriculum_tag == "status_exhaust_draw" and (
        float(semantics["consumes_status"]) > 0.0
        or float(semantics["card_draw_engine"]) > 0.0
    ):
        score += 1.1
    if curriculum_tag == "survival_override" and float(semantics["block_tag"]) > 0.0:
        score += 1.0
    return score + rng.uniform(-0.2, 0.2)


def followup_policy_action_index(
    response: dict[str, Any],
    curriculum_tag: str,
    *,
    setup_active: bool,
    rng: random.Random,
) -> tuple[int, bool, str]:
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    candidates = list(payload.get("action_candidates") or [])
    action_mask = list(payload.get("action_mask") or [])
    legal_pairs = [
        (index, candidate)
        for index, candidate in enumerate(candidates)
        if index < len(action_mask) and action_mask[index]
    ]
    if not legal_pairs:
        raise RuntimeError("no legal actions available during rollout")
    scored = []
    for index, candidate in legal_pairs:
        score = rollout_action_score(
            candidate,
            observation,
            curriculum_tag,
            setup_active=setup_active,
            rng=rng,
        )
        scored.append((score, index, candidate))
    scored.sort(key=lambda item: item[0], reverse=True)
    top_k = scored[: min(3, len(scored))]
    temperature = 0.6
    weights = [math.exp(item[0] / max(temperature, 1e-3)) for item in top_k]
    pick = rng.choices(top_k, weights=weights, k=1)[0]
    label = str(pick[2].get("label") or "")
    semantics = card_semantics(pick[2].get("card_name") or label)
    next_setup_active = setup_active or float(semantics["setup_tag"]) > 0.0
    return pick[1], next_setup_active, label


def rollout_candidate(
    driver: CombatEnvDriver,
    reset_request: dict[str, Any],
    prefix_actions: list[str],
    candidate_action: str,
    curriculum_tag: str,
    *,
    horizon_turns: int,
    trial_seed: int,
    gamma: float,
) -> dict[str, Any]:
    response = driver.request(reset_request)
    start_turn = int(
        (response.get("payload") or {}).get("observation", {}).get("turn_count") or 0
    )
    for action_label in prefix_actions:
        payload = response.get("payload") or {}
        action_index = choose_action_index(
            list(payload.get("action_candidates") or []),
            list(payload.get("action_mask") or []),
            action_label,
        )
        response = driver.request({"cmd": "step", "action_index": int(action_index)})
    target_payload = response.get("payload") or {}
    target_observation = target_payload.get("observation") or {}
    action_index = choose_action_index(
        list(target_payload.get("action_candidates") or []),
        list(target_payload.get("action_mask") or []),
        candidate_action,
    )
    rng = random.Random(trial_seed)
    chosen_semantics = card_semantics(parse_move_label(candidate_action).get("card_name"))
    setup_active = float(chosen_semantics["setup_tag"]) > 0.0
    immediate = driver.request({"cmd": "step", "action_index": int(action_index)})
    rewards = [float(immediate.get("reward") or 0.0)]
    breakdowns = [immediate.get("reward_breakdown") or {}]
    actions_taken = [str(immediate.get("chosen_action_label") or candidate_action)]
    payoff_after_setup = False
    status_engine_plays = 0
    if setup_active and float(chosen_semantics["payoff_tag"]) > 0.0:
        payoff_after_setup = True
    current = immediate
    while not bool(current.get("done")):
        payload = current.get("payload") or {}
        observation = payload.get("observation") or {}
        current_turn = int(observation.get("turn_count") or 0)
        if current_turn >= start_turn + horizon_turns:
            break
        next_index, setup_active, next_label = followup_policy_action_index(
            current,
            curriculum_tag,
            setup_active=setup_active,
            rng=rng,
        )
        semantics = card_semantics(parse_move_label(next_label).get("card_name"))
        if (
            setup_active
            and float(semantics["payoff_tag"]) > 0.0
            and next_label != candidate_action
        ):
            payoff_after_setup = True
        if (
            float(semantics["consumes_status"]) > 0.0
            or float(semantics["card_draw_engine"]) > 0.0
        ):
            status_engine_plays += 1
        current = driver.request({"cmd": "step", "action_index": int(next_index)})
        rewards.append(float(current.get("reward") or 0.0))
        breakdowns.append(current.get("reward_breakdown") or {})
        actions_taken.append(str(current.get("chosen_action_label") or ""))
    final_payload = current.get("payload") or {}
    final_obs = final_payload.get("observation") or {}
    start_snapshot = observation_to_snapshot(target_observation)
    final_snapshot = observation_to_snapshot(final_obs)
    start_hp = float((start_snapshot.get("player") or {}).get("current_hp") or 0.0)
    end_hp = float((final_snapshot.get("player") or {}).get("current_hp") or 0.0)
    max_hp = max(
        float((start_snapshot.get("player") or {}).get("max_hp") or start_hp or 1.0), 1.0
    )
    hp_loss = max(start_hp - end_hp, 0.0)
    total_monster_hp_start = sum(
        float(monster.get("current_hp") or 0.0)
        for monster in (start_snapshot.get("monsters") or [])
    )
    total_monster_hp_end = sum(
        float(monster.get("current_hp") or 0.0)
        for monster in (final_snapshot.get("monsters") or [])
    )
    monster_hp_delta = max(total_monster_hp_start - total_monster_hp_end, 0.0)
    living_end = sum(
        1
        for monster in (final_snapshot.get("monsters") or [])
        if float(monster.get("current_hp") or 0.0) > 0.0
    )
    discounted = 0.0
    weight = 1.0
    for reward in rewards:
        discounted += reward * weight
        weight *= gamma
    done = bool(current.get("done"))
    outcome = normalize_outcome(current.get("outcome"))
    death = 1.0 if outcome in {"dies", "defeat"} else 0.0
    kill = (
        1.0
        if living_end == 0
        or outcome in {"survives", "lethal_win", "victory"} and total_monster_hp_end <= 0
        else 0.0
    )
    final_pressure = (
        (final_obs.get("pressure") or {}) if isinstance(final_obs, dict) else {}
    ) or {}
    final_unblocked = float(
        final_pressure.get("value_unblocked")
        or final_pressure.get("visible_unblocked")
        or 0.0
    )
    final_incoming = float(
        final_pressure.get("value_incoming")
        or final_pressure.get("visible_incoming")
        or 0.0
    )
    end_hp_ratio = max(0.0, min(1.0, end_hp / max_hp))
    return {
        "mean_return": float(discounted),
        "hp_loss": float(hp_loss),
        "monster_hp_delta": float(monster_hp_delta),
        "survived": (
            1.0 - death if not done else (1.0 if outcome not in {"dies", "defeat"} else 0.0)
        ),
        "death": death,
        "kill": kill,
        "payoff_after_setup": 1.0 if payoff_after_setup else 0.0,
        "status_engine_plays": float(status_engine_plays),
        "actions_taken": actions_taken,
        "done": done,
        "outcome": outcome,
        "end_hp_ratio": float(end_hp_ratio),
        "final_unblocked": float(final_unblocked),
        "final_incoming": float(final_incoming),
        "final_enemy_total": float(total_monster_hp_end),
        "reward_breakdowns": breakdowns,
    }


def summarize_candidate_trials(
    trial_results: list[dict[str, Any]],
    candidate_move: str,
    state_before: dict[str, Any],
) -> dict[str, Any]:
    semantics = candidate_semantics_from_move(candidate_move)
    mean_return = float(mean(result["mean_return"] for result in trial_results))
    return_variance = (
        float(pvariance(result["mean_return"] for result in trial_results))
        if len(trial_results) > 1
        else 0.0
    )
    death_probability = float(mean(result["death"] for result in trial_results))
    kill_probability = float(mean(result["kill"] for result in trial_results))
    avg_end_hp_ratio = float(mean(result["end_hp_ratio"] for result in trial_results))
    avg_final_unblocked = float(mean(result["final_unblocked"] for result in trial_results))
    start_hp = max(float((state_before.get("player") or {}).get("current_hp") or 1.0), 1.0)
    survival_score = float(
        max(
            0.0,
            min(
                1.0,
                (1.0 - death_probability) * 0.7
                + avg_end_hp_ratio * 0.3
                - min(avg_final_unblocked / start_hp, 0.5),
            ),
        )
    )
    tempo_score = float(
        mean(
            (
                0.12 * result["monster_hp_delta"]
                - 0.10 * result["hp_loss"]
                + 0.35 * result["kill"]
                + 0.10 * max(result["status_engine_plays"], 0.0)
            )
            for result in trial_results
        )
    )
    setup_payoff_score = float(
        mean(
            (
                1.0 if semantics["setup_tag"] > 0.0 else 0.25 * semantics["payoff_tag"]
            )
            * result["payoff_after_setup"]
            for result in trial_results
        )
    )
    kill_window_score = float(
        mean(
            0.6 * result["kill"]
            + 0.4
            * min(
                result["monster_hp_delta"]
                / max(
                    sum(
                        float(m.get("current_hp") or 0.0)
                        for m in (state_before.get("monsters") or [])
                    ),
                    1.0,
                ),
                1.0,
            )
            for result in trial_results
        )
    )
    risk_score = float(
        min(
            1.0,
            mean(
                min(
                    result["hp_loss"]
                    / max(
                        float((state_before.get("player") or {}).get("current_hp") or 1.0),
                        1.0,
                    ),
                    1.0,
                )
                * 0.35
                + min(
                    result["final_unblocked"]
                    / max(
                        float((state_before.get("player") or {}).get("current_hp") or 1.0),
                        1.0,
                    ),
                    1.0,
                )
                * 0.35
                + 0.55 * result["death"]
                for result in trial_results
            )
            + min(return_variance / 6.0, 0.35),
        )
    )
    return {
        "survival_score": survival_score,
        "tempo_score": tempo_score,
        "setup_payoff_score": setup_payoff_score,
        "kill_window_score": kill_window_score,
        "risk_score": risk_score,
        "mean_return": mean_return,
        "return_variance": return_variance,
        "death_probability": death_probability,
        "kill_probability": kill_probability,
        "candidate_semantics": semantics,
    }


def select_target_groups(targets: list[dict[str, Any]], limit_groups: int) -> list[dict[str, Any]]:
    source_priority = {
        "archived_clean_run": 0,
        "policy_seed_set": 1,
        "structured_trace": 2,
        "combat_lab_spec": 3,
    }
    replay_targets = [
        target for target in targets if str(target.get("key_kind") or "") == "replay_frame"
    ]
    spec_targets = [
        target
        for target in targets
        if str(target.get("key_kind") or "") != "replay_frame"
    ]
    replay_targets.sort(
        key=lambda item: (
            source_priority.get(str(item.get("sample_origin") or "unknown"), 9),
            str(item.get("curriculum_tag") or ""),
            str(item.get("source_path") or item.get("spec_name") or ""),
            int(item.get("before_frame_id") or item.get("step_index") or 0),
        )
    )
    spec_targets.sort(
        key=lambda item: (
            source_priority.get(str(item.get("sample_origin") or "unknown"), 9),
            str(item.get("curriculum_tag") or ""),
            str(item.get("spec_name") or item.get("source_path") or ""),
            int(item.get("episode_id") or 0),
            int(item.get("step_index") or item.get("before_frame_id") or 0),
        )
    )

    if limit_groups <= 0:
        desired_total = len(replay_targets) * 2 if replay_targets else len(targets)
    else:
        desired_total = min(limit_groups, len(targets))
    if not replay_targets:
        return spec_targets[:desired_total]

    required_replay = min(len(replay_targets), math.ceil(desired_total / 2))
    max_total_with_replay_majority = required_replay + min(len(spec_targets), required_replay)
    final_total = min(desired_total, max_total_with_replay_majority)
    replay_quota = min(len(replay_targets), max(required_replay, final_total // 2 + final_total % 2))
    spec_quota = min(len(spec_targets), max(final_total - replay_quota, 0))

    selected: list[dict[str, Any]] = []
    selected.extend(replay_targets[:replay_quota])
    selected.extend(spec_targets[:spec_quota])
    selected.sort(
        key=lambda item: (
            source_priority.get(str(item.get("sample_origin") or "unknown"), 9),
            str(item.get("curriculum_tag") or ""),
            str(item.get("spec_name") or item.get("source_path") or ""),
            int(item.get("episode_id") or 0),
            int(item.get("step_index") or item.get("before_frame_id") or 0),
        )
    )
    return selected


def assign_group_splits(
    selected_targets: list[dict[str, Any]],
    *,
    min_replay_test_groups: int,
) -> dict[str, str]:
    split_by_group: dict[str, str] = {}
    replay_targets = [
        target for target in selected_targets if str(target.get("key_kind") or "") == "replay_frame"
    ]
    spec_targets = [
        target for target in selected_targets if str(target.get("key_kind") or "") != "replay_frame"
    ]
    replay_targets.sort(
        key=lambda item: (
            str(item.get("source_path") or ""),
            int(item.get("before_frame_id") or 0),
            str(item.get("group_id") or ""),
        )
    )
    spec_targets.sort(
        key=lambda item: (
            str(item.get("spec_name") or ""),
            int(item.get("episode_id") or 0),
            int(item.get("step_index") or 0),
            str(item.get("group_id") or ""),
        )
    )

    replay_test_count = min(
        len(replay_targets),
        max(int(min_replay_test_groups), min(len(replay_targets), int(min_replay_test_groups * 2))),
    )
    replay_val_count = min(
        max(len(replay_targets) // 6, 4 if len(replay_targets) >= 12 else 0),
        max(len(replay_targets) - replay_test_count, 0),
    )
    for index, target in enumerate(replay_targets):
        group_id = str(target.get("group_id") or "")
        if index < replay_test_count:
            split_by_group[group_id] = "test"
        elif index < replay_test_count + replay_val_count:
            split_by_group[group_id] = "val"
        else:
            split_by_group[group_id] = "train"
    for target in spec_targets:
        group_id = str(target.get("group_id") or "")
        split_by_group[group_id] = stable_split(group_id)
    return split_by_group


def load_learning_baseline_raw_map(path: Path) -> dict[str, str]:
    if not path.exists():
        return {}
    payload = json.loads(path.read_text(encoding="utf-8"))
    mapping: dict[str, str] = {}
    for run in payload.get("selected_runs") or []:
        run_id = str(run.get("run_id") or "")
        raw_path = str(run.get("raw_path") or "")
        if run_id and raw_path:
            mapping[run_id] = raw_path
    return mapping


def collect_spec_targets(
    dataset_dir: Path,
    spec_dir: Path,
    requested_tags: set[str],
) -> list[dict[str, Any]]:
    transition_rows = load_transition_rows(dataset_dir)
    policy_index = load_policy_index(dataset_dir)
    transition_rows.sort(
        key=lambda row: (
            str(row.get("spec_name") or ""),
            int(row.get("episode_id") or 0),
            int(row.get("step_index") or 0),
        )
    )
    episodes: dict[tuple[str, int], list[dict[str, Any]]] = defaultdict(list)
    for row in transition_rows:
        episodes[(str(row.get("spec_name") or ""), int(row.get("episode_id") or 0))].append(row)
    spec_path_map = {path.stem: path for path in sorted(spec_dir.glob("*.json"))}
    targets: list[dict[str, Any]] = []
    for (spec_name, episode_id), episode_rows in episodes.items():
        spec_path = spec_path_map.get(spec_name)
        if spec_path is None:
            continue
        curriculum_tag = curriculum_tag_from_spec_name(spec_name)
        if requested_tags and curriculum_tag not in requested_tags:
            continue
        prefix_actions: list[str] = []
        for row in sorted(episode_rows, key=lambda item: int(item.get("step_index") or 0)):
            step_index = int(row.get("step_index") or 0)
            key = (spec_name, episode_id, step_index)
            policy_group = policy_index.get(key)
            if not policy_group:
                prefix_actions.append(str(row.get("action_label") or ""))
                continue
            seed = int(row.get("seed") or 0)
            group_id = f"{spec_name}::episode::{episode_id}::step::{step_index}"
            targets.append(
                {
                    "group_id": group_id,
                    "sample_origin": "combat_lab_spec",
                    "teacher_source": "local_rollout_oracle",
                    "curriculum_tag": curriculum_tag,
                    "state_source": "combat_lab_trace",
                    "label_source": "local_rollout_oracle",
                    "key_kind": "spec_episode_step",
                    "spec_name": spec_name,
                    "episode_id": episode_id,
                    "step_index": step_index,
                    "source_path": None,
                    "before_frame_id": None,
                    "before_response_id": None,
                    "sample_tags": [],
                    "baseline_action": str(
                        policy_group.get("baseline_action") or row.get("action_label") or ""
                    ),
                    "preferred_action": None,
                    "candidate_by_move": dict(policy_group.get("candidate_by_move") or {}),
                    "prefix_actions": list(prefix_actions),
                    "seed_hint": seed,
                    "reset_request": {
                        "cmd": "reset",
                        "author_spec": str(spec_path),
                        "seed_hint": seed,
                    },
                }
            )
            prefix_actions.append(str(row.get("action_label") or ""))
    return targets


def collect_policy_seed_targets(
    policy_seed_paths: list[Path],
    requested_tags: set[str],
) -> list[dict[str, Any]]:
    grouped: dict[tuple[str, int], dict[str, Any]] = {}
    for path in policy_seed_paths:
        for row in load_rows(path):
            source_path = str(row.get("source_path") or "")
            before_frame_id = row.get("before_frame_id")
            if not source_path or before_frame_id is None:
                continue
            key = (source_path, int(before_frame_id))
            sample_tags = sample_tags_from_preference_sample(row)
            preferred_action = str(row.get("preferred_action") or "")
            chosen_action = str(row.get("chosen_action") or "")
            curriculum_tag = infer_replay_curriculum_tag(
                chosen_action,
                preferred_action,
                sample_tags,
            )
            if requested_tags and curriculum_tag not in requested_tags:
                continue
            representative = grouped.get(key)
            if representative is None or int(row.get("score_gap") or 0) > int(
                representative.get("score_gap") or 0
            ):
                grouped[key] = {
                    "group_id": f"policy_seed_set::{source_path}::frame::{int(before_frame_id)}",
                    "sample_origin": "policy_seed_set",
                    "teacher_source": "policy_seed_set_local_oracle",
                    "curriculum_tag": curriculum_tag,
                    "state_source": str(
                        row.get("state_source") or "reconstructed_live_replay_state"
                    ),
                    "label_source": "local_rollout_oracle",
                    "key_kind": "replay_frame",
                    "spec_name": None,
                    "episode_id": None,
                    "step_index": None,
                    "source_path": source_path,
                    "before_frame_id": int(before_frame_id),
                    "before_response_id": (
                        int(row.get("before_response_id"))
                        if row.get("before_response_id") is not None
                        else None
                    ),
                    "sample_tags": sample_tags,
                    "baseline_action": chosen_action,
                    "preferred_action": preferred_action,
                    "candidate_by_move": {},
                    "prefix_actions": [],
                    "seed_hint": int(before_frame_id),
                    "reset_request": {
                        "cmd": "reset",
                        "replay_raw": source_path,
                        "replay_frame": int(before_frame_id),
                        "seed_hint": int(before_frame_id),
                    },
                    "score_gap": int(row.get("score_gap") or 0),
                }
            else:
                existing_tags = set(representative.get("sample_tags") or [])
                existing_tags.update(sample_tags)
                representative["sample_tags"] = sorted(existing_tags)
    return list(grouped.values())


def collect_structured_trace_targets(
    dataset_dir: Path,
    requested_tags: set[str],
) -> list[dict[str, Any]]:
    targets: list[dict[str, Any]] = []
    for path in sorted(dataset_dir.glob("*_structured_policy_trace.json")):
        try:
            trace = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        spec_name = str(trace.get("spec_name") or path.stem.replace("_structured_policy_trace", ""))
        curriculum_tag = curriculum_tag_from_spec_name(spec_name)
        if requested_tags and curriculum_tag not in requested_tags:
            continue
        spec_path = str(trace.get("spec_path") or "")
        if not spec_path:
            continue
        seed = int(trace.get("seed") or 0)
        prefix_actions: list[str] = []
        for step_index, step in enumerate(trace.get("steps_data") or []):
            baseline_action = str(
                step.get("decoded_candidate_label")
                or step.get("chosen_action_label")
                or step.get("canonical_action_label")
                or ""
            )
            if not baseline_action:
                continue
            candidate_by_move = candidate_map_from_trace_step(step)
            targets.append(
                {
                    "group_id": (
                        f"{spec_name}::structured_trace::{path.stem}::seed::{seed}"
                        f"::step::{step_index}"
                    ),
                    "sample_origin": "structured_trace",
                    "teacher_source": "structured_trace_local_oracle",
                    "curriculum_tag": curriculum_tag,
                    "state_source": "structured_policy_trace",
                    "label_source": "local_rollout_oracle",
                    "key_kind": "spec_trace_step",
                    "spec_name": spec_name,
                    "episode_id": seed,
                    "step_index": step_index,
                    "source_path": str(path),
                    "before_frame_id": None,
                    "before_response_id": None,
                    "sample_tags": [],
                    "baseline_action": baseline_action,
                    "preferred_action": None,
                    "candidate_by_move": candidate_by_move,
                    "prefix_actions": list(prefix_actions),
                    "seed_hint": seed,
                    "reset_request": {
                        "cmd": "reset",
                        "author_spec": spec_path,
                        "seed_hint": seed,
                    },
                }
            )
            prefix_actions.append(baseline_action)
    return targets


def collect_archived_targets(
    dataset_dir: Path,
    learning_baseline_path: Path,
    requested_tags: set[str],
) -> list[dict[str, Any]]:
    raw_path_by_run = load_learning_baseline_raw_map(learning_baseline_path)
    grouped: dict[tuple[str, int], dict[str, Any]] = {}
    for row in load_rows(dataset_dir / "combat_reranker_samples.jsonl"):
        run_id = str(row.get("run_id") or "")
        raw_path = raw_path_by_run.get(run_id)
        frame = row.get("frame_count")
        if not raw_path or frame is None:
            continue
        frame = int(frame)
        oracle = row.get("oracle") or {}
        baseline_action = str(row.get("chosen_move") or "")
        preferred_action = str(oracle.get("oracle_best_move") or baseline_action)
        sample_tags = sorted(set(str(tag) for tag in (row.get("sample_tags") or [])))
        curriculum_tag = infer_replay_curriculum_tag(
            baseline_action,
            preferred_action,
            sample_tags,
        )
        if requested_tags and curriculum_tag not in requested_tags:
            continue
        key = (raw_path, frame)
        representative = grouped.get(key)
        severity = len(sample_tags) + (1 if "hard_disagreement" in sample_tags else 0)
        if representative is None or severity > int(representative.get("_severity") or -1):
            grouped[key] = {
                "group_id": f"archived_clean_run::{run_id}::frame::{frame}",
                "sample_origin": "archived_clean_run",
                "teacher_source": "archived_replay_local_oracle",
                "curriculum_tag": curriculum_tag,
                "state_source": "reconstructed_live_replay_state",
                "label_source": "local_rollout_oracle",
                "key_kind": "replay_frame",
                "spec_name": None,
                "episode_id": None,
                "step_index": None,
                "source_path": raw_path,
                "before_frame_id": frame,
                "before_response_id": (
                    int(oracle.get("response_id"))
                    if oracle.get("response_id") is not None
                    else None
                ),
                "sample_tags": sample_tags,
                "baseline_action": baseline_action,
                "preferred_action": preferred_action,
                "candidate_by_move": {},
                "prefix_actions": [],
                "seed_hint": frame,
                "reset_request": {
                    "cmd": "reset",
                    "replay_raw": raw_path,
                    "replay_frame": frame,
                    "seed_hint": frame,
                },
                "_severity": severity,
            }
    return list(grouped.values())


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Build local rollout-oracle Q_local datasets from combat spec traces and replay states."
    )
    parser.add_argument(
        "--dataset-dir",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset",
        type=Path,
    )
    parser.add_argument(
        "--spec-dir", default=REPO_ROOT / "data" / "combat_lab" / "specs", type=Path
    )
    parser.add_argument("--driver-binary", default=None, type=Path)
    parser.add_argument("--horizon-turns", default=1, type=int)
    parser.add_argument("--rollout-seeds", default=6, type=int)
    parser.add_argument("--gamma", default=0.97, type=float)
    parser.add_argument("--limit-groups", default=0, type=int)
    parser.add_argument("--curriculum-tags", default="")
    parser.add_argument("--min-gap", default=0.2, type=float)
    parser.add_argument("--min-replay-test-groups", default=16, type=int)
    parser.add_argument("--out-prefix", default="combat_q_local")
    parser.add_argument(
        "--policy-seed-glob",
        default=str(REPO_ROOT / "data" / "combat_lab" / "policy_seed_set_*.jsonl"),
    )
    parser.add_argument(
        "--learning-baseline",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_baseline.json",
        type=Path,
    )
    args = parser.parse_args()

    requested_tags = {
        tag.strip() for tag in str(args.curriculum_tags or "").split(",") if tag.strip()
    }
    policy_seed_glob = str(args.policy_seed_glob or "").strip()
    if policy_seed_glob:
        policy_seed_paths = sorted(
            Path(policy_seed_glob).parent.glob(Path(policy_seed_glob).name)
        )
    else:
        policy_seed_paths = []

    targets = []
    targets.extend(collect_spec_targets(args.dataset_dir, args.spec_dir, requested_tags))
    targets.extend(collect_structured_trace_targets(args.dataset_dir, requested_tags))
    targets.extend(collect_policy_seed_targets(policy_seed_paths, requested_tags))
    targets.extend(
        collect_archived_targets(args.dataset_dir, args.learning_baseline, requested_tags)
    )
    targets.sort(
        key=lambda item: (
            str(item.get("sample_origin") or ""),
            str(item.get("curriculum_tag") or ""),
            str(item.get("spec_name") or item.get("source_path") or ""),
            int(item.get("episode_id") or 0),
            int(item.get("step_index") or item.get("before_frame_id") or 0),
        )
    )
    selected_targets = select_target_groups(targets, args.limit_groups)
    split_by_group = assign_group_splits(
        selected_targets,
        min_replay_test_groups=args.min_replay_test_groups,
    )

    rows: list[dict[str, Any]] = []
    group_summaries: list[dict[str, Any]] = []
    skipped_groups: Counter[str] = Counter()
    driver = CombatEnvDriver(args.driver_binary)
    try:
        for target in selected_targets:
            reset_request = dict(target["reset_request"])
            try:
                response = driver.request(reset_request)
            except Exception:
                skipped_groups["reset_failed"] += 1
                continue
            replay_ok = True
            for action_label in target.get("prefix_actions") or []:
                try:
                    payload = response.get("payload") or {}
                    action_index = choose_action_index(
                        list(payload.get("action_candidates") or []),
                        list(payload.get("action_mask") or []),
                        action_label,
                    )
                    response = driver.request({"cmd": "step", "action_index": int(action_index)})
                except Exception:
                    replay_ok = False
                    break
            if not replay_ok:
                skipped_groups["prefix_replay_failed"] += 1
                continue
            payload = response.get("payload") or {}
            observation = payload.get("observation") or {}
            action_candidates = list(payload.get("action_candidates") or [])
            action_mask = list(payload.get("action_mask") or [])
            state_before = observation_to_snapshot(observation)
            legal_moves = [
                str(candidate.get("label") or "")
                for idx, candidate in enumerate(action_candidates)
                if idx < len(action_mask) and action_mask[idx]
            ]
            if len(legal_moves) < 2:
                skipped_groups["too_few_legal_moves"] += 1
                continue
            dynamic_teacher = dynamic_teacher_from_live_state(
                observation,
                legal_moves,
                baseline_action=str(target.get("baseline_action") or ""),
                candidate_by_move=dict(target.get("candidate_by_move") or {}),
                sample_tags=list(target.get("sample_tags") or []),
            )
            dynamic_preferred_set = {
                str(move) for move in (dynamic_teacher.get("preferred_moves") or []) if str(move)
            }
            dynamic_detail_by_move = {
                str(detail.get("move_label") or ""): detail
                for detail in (dynamic_teacher.get("candidate_details") or [])
            }
            dynamic_active = bool(dynamic_teacher.get("active"))
            dynamic_margin = float(dynamic_teacher.get("oracle_margin") or 0.0)
            base_seed = int(target.get("seed_hint") or 0)
            group_rows: list[dict[str, Any]] = []
            for candidate_move in legal_moves:
                trial_results = []
                for trial_index in range(args.rollout_seeds):
                    trial_seed = (
                        base_seed * 1009
                        + (trial_index * 7919)
                        + int(
                            target.get("step_index")
                            or target.get("before_frame_id")
                            or 0
                        )
                    )
                    trial_results.append(
                        rollout_candidate(
                            driver,
                            reset_request,
                            list(target.get("prefix_actions") or []),
                            candidate_move,
                            str(target.get("curriculum_tag") or "attack_over_defend"),
                            horizon_turns=args.horizon_turns,
                            trial_seed=trial_seed,
                            gamma=args.gamma,
                        )
                    )
                summary = summarize_candidate_trials(trial_results, candidate_move, state_before)
                policy_row = (target.get("candidate_by_move") or {}).get(candidate_move) or {}
                dynamic_detail = dynamic_detail_by_move.get(candidate_move) or {}
                candidate_row = {
                    "dataset_kind": "combat_q_local",
                    "sample_origin": str(target.get("sample_origin") or "unknown"),
                    "teacher_source": str(target.get("teacher_source") or "local_rollout_oracle"),
                    "curriculum_tag": str(
                        target.get("curriculum_tag") or "attack_over_defend"
                    ),
                    "state_source": str(
                        target.get("state_source") or "reconstructed_live_replay_state"
                    ),
                    "label_source": str(target.get("label_source") or "local_rollout_oracle"),
                    "label_strength": "oracle_strong",
                    "oracle_seed_set_id": str(target.get("group_id") or ""),
                    "horizon": f"{args.horizon_turns}_enemy_turn",
                    "split": split_by_group[str(target.get("group_id") or "")],
                    "group_id": str(target.get("group_id") or ""),
                    "sample_id": f"{target.get('group_id')}::{candidate_move}",
                    "key_kind": str(target.get("key_kind") or "unknown"),
                    "spec_name": target.get("spec_name"),
                    "episode_id": target.get("episode_id"),
                    "step_index": target.get("step_index"),
                    "source_path": target.get("source_path"),
                    "before_frame_id": target.get("before_frame_id"),
                    "before_response_id": target.get("before_response_id"),
                    "state_frame_id": target.get("before_frame_id"),
                    "seed": base_seed,
                    "baseline_action": str(target.get("baseline_action") or ""),
                    "preferred_action": target.get("preferred_action"),
                    "candidate_move": candidate_move,
                    "candidate_semantics": summary["candidate_semantics"],
                    "chance_features": chance_feature_dict(state_before, candidate_move),
                    "dynamic_teacher_targets": dynamic_detail.get("teacher_targets") or {},
                    "dynamic_teacher_score": float(dynamic_detail.get("teacher_score") or 0.0),
                    "dynamic_teacher_active": dynamic_active,
                    "dynamic_teacher_margin": dynamic_margin,
                    "dynamic_teacher_is_top": candidate_move in dynamic_preferred_set,
                    "state_before": state_before,
                    "sample_tags": list(target.get("sample_tags") or []),
                    "candidate_rank": policy_row.get("candidate_rank"),
                    "candidate_score_hint": float(policy_row.get("candidate_score_hint") or 0.0),
                    "immediate_reward_total": float(
                        (trial_results[0].get("reward_breakdowns") or [{}])[0].get("total")
                        or 0.0
                    ),
                    "immediate_reward_breakdown": (
                        trial_results[0].get("reward_breakdowns") or [{}]
                    )[0],
                    "trial_actions": [result["actions_taken"] for result in trial_results],
                    **summary,
                }
                candidate_row["eval_bucket"] = eval_bucket_from_row(candidate_row)
                candidate_row["q_local_teacher_score"] = aggregate_q_local_score(candidate_row)
                group_rows.append(candidate_row)
            group_rows.sort(key=lambda item: float(item["mean_return"]), reverse=True)
            best = float(group_rows[0]["mean_return"])
            second = float(group_rows[1]["mean_return"]) if len(group_rows) > 1 else best
            uncertain = abs(best - second) < float(args.min_gap)
            best_teacher_score = max(
                float(candidate_row["q_local_teacher_score"]) for candidate_row in group_rows
            )
            sample_origin = str(target.get("sample_origin") or "unknown")
            source_weight = {
                "archived_clean_run": 2.0,
                "policy_seed_set": 1.75,
                "structured_trace": 1.5,
                "combat_lab_spec": 1.0,
            }.get(sample_origin, 1.0)
            for candidate_row in group_rows:
                candidate_row["uncertain"] = uncertain
                base_weight = (
                    0.5 if uncertain and sample_origin != "combat_lab_spec" else 0.25 if uncertain else 1.0
                )
                if dynamic_active:
                    base_weight *= 1.15
                if uncertain and dynamic_active:
                    base_weight *= 1.10
                candidate_row["training_weight"] = base_weight * source_weight
                candidate_row["candidate_is_best"] = abs(
                    float(candidate_row["mean_return"]) - best
                ) < 1e-6
                candidate_row["candidate_is_teacher_top"] = abs(
                    float(candidate_row["q_local_teacher_score"]) - best_teacher_score
                ) < 1e-6
                rows.append(candidate_row)
            group_summaries.append(
                {
                    "group_id": str(target.get("group_id") or ""),
                    "split": split_by_group[str(target.get("group_id") or "")],
                    "sample_origin": str(target.get("sample_origin") or "unknown"),
                    "key_kind": str(target.get("key_kind") or "unknown"),
                    "curriculum_tag": str(
                        target.get("curriculum_tag") or "attack_over_defend"
                    ),
                    "spec_name": target.get("spec_name"),
                    "episode_id": target.get("episode_id"),
                    "step_index": target.get("step_index"),
                    "source_path": target.get("source_path"),
                    "before_frame_id": target.get("before_frame_id"),
                    "legal_candidate_count": len(group_rows),
                    "uncertain": uncertain,
                    "dynamic_teacher_active": dynamic_active,
                    "dynamic_teacher_margin": dynamic_margin,
                    "best_move": group_rows[0]["candidate_move"],
                    "best_mean_return": best,
                    "second_mean_return": second,
                    "baseline_action": str(target.get("baseline_action") or ""),
                }
            )
    finally:
        driver.close()

    split_counts: dict[str, int] = {}
    for split in ("train", "val", "test"):
        split_rows = [row for row in rows if row.get("split") == split]
        write_jsonl(args.dataset_dir / f"{args.out_prefix}_{split}.jsonl", split_rows)
        split_counts[split] = len(split_rows)

    summary = {
        "dataset_kind": "combat_q_local",
        "rows": len(rows),
        "group_count": len(group_summaries),
        "split_counts": split_counts,
        "sample_origin_counts": dict(
            Counter(str(row.get("sample_origin") or "unknown") for row in rows)
        ),
        "curriculum_tag_counts": dict(
            Counter(str(row.get("curriculum_tag") or "unknown") for row in rows)
        ),
        "eval_bucket_counts": dict(
            Counter(str(row.get("eval_bucket") or "unknown") for row in rows)
        ),
        "key_kind_counts": dict(
            Counter(str(row.get("key_kind") or "unknown") for row in rows)
        ),
        "selected_target_count_by_source": dict(
            Counter(str(target.get("sample_origin") or "unknown") for target in selected_targets)
        ),
        "split_group_counts": dict(
            Counter(str(group.get("split") or "unknown") for group in group_summaries)
        ),
        "replay_test_group_count": sum(
            1
            for group in group_summaries
            if str(group.get("key_kind") or "") == "replay_frame"
            and str(group.get("split") or "") == "test"
        ),
        "dynamic_teacher_active_group_count": sum(
            1 for group in group_summaries if bool(group.get("dynamic_teacher_active"))
        ),
        "dynamic_teacher_active_rate": float(
            sum(1 for group in group_summaries if bool(group.get("dynamic_teacher_active")))
            / max(len(group_summaries), 1)
        ),
        "uncertain_group_count": sum(
            1 for group in group_summaries if group.get("uncertain")
        ),
        "uncertain_rate_by_bucket": {
            bucket: float(
                sum(
                    1
                    for group in group_summaries
                    if str(group.get("curriculum_tag") or "") == bucket and group.get("uncertain")
                )
                / max(
                    sum(
                        1
                        for group in group_summaries
                        if str(group.get("curriculum_tag") or "") == bucket
                    ),
                    1,
                )
            )
            for bucket in sorted(
                {str(group.get("curriculum_tag") or "") for group in group_summaries}
            )
        },
        "uncertain_rate_by_source": {
            source: float(
                sum(
                    1
                    for group in group_summaries
                    if str(group.get("sample_origin") or "") == source and group.get("uncertain")
                )
                / max(
                    sum(
                        1
                        for group in group_summaries
                        if str(group.get("sample_origin") or "") == source
                    ),
                    1,
                )
            )
            for source in sorted(
                {str(group.get("sample_origin") or "") for group in group_summaries}
            )
        },
        "selected_target_count": len(selected_targets),
        "skipped_groups": dict(skipped_groups),
        "driver_binary": str(driver.binary),
        "spec_dir": str(args.spec_dir),
        "policy_seed_glob": args.policy_seed_glob,
        "learning_baseline": str(args.learning_baseline),
        "horizon_turns": args.horizon_turns,
        "rollout_seeds": args.rollout_seeds,
        "gamma": args.gamma,
        "min_gap": args.min_gap,
        "min_replay_test_groups": args.min_replay_test_groups,
        "limit_groups": args.limit_groups,
        "notes": [
            "local oracle uses shared stochastic rollout policy seeds over deterministic CombatEnv replays",
            "teacher rows are generated from combat_lab_spec, structured_trace, policy_seed_set, and archived_clean_run sources",
            "structured policy trace artifacts can be replayed directly as spec-trace roots when baseline transition datasets are absent",
            "replay-backed rows emit replay_frame keys when source_path + before_frame_id are available",
            "dynamic semantic teacher proxies are attached as auxiliary features and sample-weight hints, not as replacement rollout labels",
            "uncertain groups are retained for shadow coverage and downweighted during training",
            "replay groups are prioritized during target selection and reserved into the test split",
        ],
    }
    if int(summary["replay_test_group_count"]) < int(args.min_replay_test_groups):
        raise SystemExit(
            f"replay test split too small: {summary['replay_test_group_count']} < {args.min_replay_test_groups}"
        )
    write_json(args.dataset_dir / f"{args.out_prefix}_summary.json", summary)
    write_jsonl(args.dataset_dir / f"{args.out_prefix}_groups.jsonl", group_summaries)
    print(json.dumps(summary, indent=2, ensure_ascii=False))
    print(f"wrote combat local oracle rows to {args.dataset_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
