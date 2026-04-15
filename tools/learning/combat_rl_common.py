#!/usr/bin/env python3
from __future__ import annotations

import glob
import json
import re
import subprocess
import tempfile
from collections import defaultdict
from pathlib import Path
from typing import Any

from card_semantics import aggregate_semantics_from_cards, card_semantics, normalize_card_name
from combat_reranker_common import (
    curriculum_tag_from_spec_name,
    iter_jsonl,
    parse_move_label,
    preference_label_strength,
    preference_state_to_snapshot,
    sample_tags_from_oracle_row,
    sample_tags_from_preference_sample,
    stable_split,
    write_json,
    write_jsonl,
)

REPO_ROOT = Path(__file__).resolve().parents[2]
INTENT_DAMAGE_RE = re.compile(r"damage:\s*(?P<damage>\d+),\s*hits:\s*(?P<hits>\d+)")


def find_release_binary(explicit: Path | None, name: str) -> Path:
    if explicit and explicit.exists():
        return explicit
    exe = REPO_ROOT / "target" / "release" / f"{name}.exe"
    if exe.exists():
        return exe
    bin_path = REPO_ROOT / "target" / "release" / name
    if bin_path.exists():
        return bin_path
    raise SystemExit(f"missing release binary '{name}'; run cargo build --release --bin {name} first")


def run_combat_lab_spec(
    binary: Path,
    spec_path: Path,
    episodes: int,
    depth: int,
    base_seed: int,
    out_dir: Path,
) -> None:
    cmd = [
        str(binary),
        "--author-spec",
        str(spec_path),
        "--episodes",
        str(episodes),
        "--policy",
        "bot",
        "--depth",
        str(depth),
        "--variant-mode",
        "reshuffle_draw",
        "--base-seed",
        str(base_seed),
        "--out-dir",
        str(out_dir),
    ]
    subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def load_jsonl_rows(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [row for _, row in iter_jsonl(path)]


def intent_damage(intent_text: str | None) -> int:
    text = str(intent_text or "")
    match = INTENT_DAMAGE_RE.search(text)
    if not match:
        return 0
    return int(match.group("damage")) * int(match.group("hits"))


def monster_total_hp(monsters: list[dict[str, Any]]) -> int:
    return sum(int(monster.get("current_hp") or monster.get("hp") or 0) for monster in monsters)


def living_monster_count(monsters: list[dict[str, Any]]) -> int:
    return sum(1 for monster in monsters if int(monster.get("current_hp") or monster.get("hp") or 0) > 0)


def incoming_damage_from_monsters(monsters: list[dict[str, Any]]) -> int:
    return sum(intent_damage(monster.get("intent")) for monster in monsters if int(monster.get("current_hp") or monster.get("hp") or 0) > 0)


def snapshot_from_trace_step(step: dict[str, Any], when: str) -> dict[str, Any]:
    assert when in {"before", "after"}
    suffix = "before" if when == "before" else "after"
    monsters = []
    for monster in step.get(f"monsters_{suffix}") or []:
        monsters.append(
            {
                "id": monster.get("id") or f"monster_{monster.get('slot', 0)}",
                "name": monster.get("id") or f"monster_{monster.get('slot', 0)}",
                "current_hp": int(monster.get("hp") or 0),
                "max_hp": int(monster.get("max_hp") or monster.get("hp") or 0),
                "block": int(monster.get("block") or 0),
                "intent": monster.get("intent"),
                "powers": [],
            }
        )
    hand = []
    for index, card in enumerate(step.get(f"hand_{suffix}") or []):
        hand.append(
            {
                "id": f"{card}:{index}",
                "name": str(card).split("(", 1)[0],
                "cost": 0,
                "upgrades": 0,
            }
        )
    return {
        "act": 0,
        "floor": 0,
        "gold": 0,
        "player": {
            "current_hp": int(step.get(f"player_hp_{suffix}") or 0),
            "max_hp": int(max(step.get("player_hp_before") or 0, step.get("player_hp_after") or 0)),
            "block": int(step.get(f"player_block_{suffix}") or 0),
            "energy": int(step.get(f"energy_{suffix}") or 0),
            "powers": [],
        },
        "monsters": monsters,
        "zones": {
            "hand_count": int(step.get(f"hand_size_{suffix}") or len(hand)),
            "draw_count": int(step.get(f"draw_size_{suffix}") or 0),
            "discard_count": int(step.get(f"discard_size_{suffix}") or 0),
            "exhaust_count": 0,
            "hand": hand,
            "draw": [],
            "discard": [],
            "exhaust": [],
        },
    }


def snapshot_state_features(snapshot: dict[str, Any] | None) -> dict[str, Any]:
    state = snapshot or {}
    player = state.get("player") or {}
    zones = state.get("zones") or {}
    monsters = state.get("monsters") or []
    living = [monster for monster in monsters if int(monster.get("current_hp") or 0) > 0]
    return {
        "act": int(state.get("act") or 0),
        "floor": int(state.get("floor") or 0),
        "gold": int(state.get("gold") or 0),
        "player_current_hp": int(player.get("current_hp") or 0),
        "player_max_hp": int(player.get("max_hp") or 0),
        "player_block": int(player.get("block") or 0),
        "player_energy": int(player.get("energy") or 0),
        "hand_count": int(zones.get("hand_count") or 0),
        "draw_count": int(zones.get("draw_count") or 0),
        "discard_count": int(zones.get("discard_count") or 0),
        "exhaust_count": int(zones.get("exhaust_count") or 0),
        "monster_count": len(monsters),
        "living_monster_count": len(living),
        "total_monster_hp": sum(int(monster.get("current_hp") or 0) for monster in living),
        "lowest_monster_hp": min((int(monster.get("current_hp") or 0) for monster in living), default=0),
        "incoming_damage": incoming_damage_from_monsters(living),
        "intent_attackers": sum(1 for monster in living if intent_damage(monster.get("intent")) > 0),
    }


def card_name_counts(snapshot: dict[str, Any] | None) -> dict[str, int]:
    state = snapshot or {}
    zones = state.get("zones") or {}
    counts: dict[str, int] = defaultdict(int)
    for card in zones.get("hand") or []:
        name = str(card.get("name") or card.get("card_id") or "").strip()
        if name:
            counts[name] += 1
    return dict(counts)


def hand_semantics_counts(snapshot: dict[str, Any] | None) -> dict[str, float]:
    state = snapshot or {}
    zones = state.get("zones") or {}
    return aggregate_semantics_from_cards(list(zones.get("hand") or []))


def monster_name_counts(snapshot: dict[str, Any] | None) -> dict[str, int]:
    state = snapshot or {}
    counts: dict[str, int] = defaultdict(int)
    for monster in state.get("monsters") or []:
        name = str(monster.get("name") or monster.get("id") or "").strip()
        if name and int(monster.get("current_hp") or 0) > 0:
            counts[name] += 1
    return dict(counts)


def intent_family(intent_text: str | None) -> str:
    text = str(intent_text or "").lower()
    if "attack" in text:
        return "attack"
    if "buff" in text:
        return "buff"
    if "debuff" in text:
        return "debuff"
    if "defend" in text or "block" in text:
        return "defend"
    if "sleep" in text:
        return "sleep"
    return "other"


def monster_intent_family_counts(snapshot: dict[str, Any] | None) -> dict[str, int]:
    state = snapshot or {}
    counts: dict[str, int] = defaultdict(int)
    for monster in state.get("monsters") or []:
        if int(monster.get("current_hp") or 0) <= 0:
            continue
        counts[intent_family(monster.get("intent"))] += 1
    return dict(counts)


def chosen_move_from_trace_step(step: dict[str, Any]) -> str:
    action_debug = str(step.get("chosen_action") or "")
    hand_before = list(step.get("hand_before") or [])
    match = re.match(r"^PlayCard \{ card_index: (?P<slot>\d+), target: (?P<target>Some\((?P<target_idx>\d+)\)|None) \}$", action_debug)
    if match:
        zero_slot = int(match.group("slot"))
        slot = zero_slot + 1
        card_name = str(hand_before[zero_slot]).split("(", 1)[0] if zero_slot < len(hand_before) else None
        target_idx = match.group("target_idx")
        if target_idx is None:
            return f"Play #{slot} {card_name}" if card_name else f"Play #{slot}"
        return f"Play #{slot} {card_name} @{int(target_idx)}" if card_name else f"Play #{slot} @{int(target_idx)}"
    match = re.match(r"^UsePotion \{ potion_index: (?P<slot>\d+), target: (?P<target>Some\((?P<target_idx>\d+)\)|None) \}$", action_debug)
    if match:
        slot = int(match.group("slot"))
        target_idx = match.group("target_idx")
        return f"UsePotion#{slot}" if target_idx is None else f"UsePotion#{slot} @{int(target_idx)}"
    if action_debug == "EndTurn":
        return "EndTurn"
    return action_debug


def normalized_candidates_from_trace_step(step: dict[str, Any]) -> list[dict[str, Any]]:
    hand_before = list(step.get("hand_before") or [])
    raw_candidates = (step.get("policy_decision") or {}).get("candidate_scores") or []
    normalized = []
    for search_rank, candidate in enumerate(raw_candidates):
        action = candidate.get("action") or {}
        action_debug = str(action.get("debug") or "")
        move_label = chosen_move_from_trace_step({"chosen_action": action_debug, "hand_before": hand_before})
        parsed = parse_move_label(move_label)
        normalized.append(
            {
                "move_label": move_label,
                "move_family": parsed["move_family"],
                "card_name": parsed["card_name"],
                "slot_index": parsed["slot_index"],
                "has_target": parsed["has_target"],
                "target_index": parsed["target_index"],
                "score": float(candidate.get("score") or 0.0),
                "source": candidate.get("source"),
                "visits": candidate.get("visits"),
                "avg_score": float(candidate.get("avg_score") or candidate.get("score") or 0.0),
                "search_rank": search_rank,
            }
        )
    return normalized


def reward_breakdown_from_trace_step(step: dict[str, Any], done: bool) -> dict[str, float]:
    before_snapshot = snapshot_from_trace_step(step, "before")
    after_snapshot = snapshot_from_trace_step(step, "after")
    before_features = snapshot_state_features(before_snapshot)
    after_features = snapshot_state_features(after_snapshot)
    enemy_hp_delta = float(max(before_features["total_monster_hp"] - after_features["total_monster_hp"], 0))
    player_hp_delta = float(min(after_features["player_current_hp"] - before_features["player_current_hp"], 0))
    before_unblocked = max(before_features["incoming_damage"] - before_features["player_block"], 0)
    after_unblocked = max(after_features["incoming_damage"] - after_features["player_block"], 0)
    incoming_relief = float(max(before_unblocked - after_unblocked, 0)) * 0.35
    kill_bonus = float(max(before_features["living_monster_count"] - after_features["living_monster_count"], 0) * 2)
    stabilize_bonus = 1.5 if before_unblocked > 0 and after_unblocked == 0 else 0.0
    chosen_action = chosen_move_from_trace_step(step)
    idle_penalty = (
        -0.75
        if not done
        and enemy_hp_delta <= 0.0
        and incoming_relief <= 0.0
        and after_features["player_block"] <= before_features["player_block"]
        and chosen_action != "EndTurn"
        else 0.0
    )
    total = enemy_hp_delta + player_hp_delta * 1.5 + incoming_relief + kill_bonus + stabilize_bonus + idle_penalty
    return {
        "enemy_hp_delta": enemy_hp_delta,
        "player_hp_delta": player_hp_delta,
        "incoming_relief": incoming_relief,
        "kill_bonus": kill_bonus,
        "stabilize_bonus": stabilize_bonus,
        "idle_penalty": idle_penalty,
        "total": total,
    }


def discounted_returns(rewards: list[float], gamma: float) -> list[float]:
    returns = [0.0] * len(rewards)
    running = 0.0
    for index in range(len(rewards) - 1, -1, -1):
        running = rewards[index] + gamma * running
        returns[index] = running
    return returns


def horizon_return(rewards: list[float], start: int, gamma: float, horizon: int) -> float:
    total = 0.0
    weight = 1.0
    for reward in rewards[start : start + horizon]:
        total += reward * weight
        weight *= gamma
    return total


def transition_feature_dict(row: dict[str, Any]) -> dict[str, Any]:
    features: dict[str, Any] = {}
    snapshot = row.get("state_before")
    before_features = snapshot_state_features(snapshot)
    action = parse_move_label(row.get("action_label"))
    for key, value in before_features.items():
        features[f"state::{key}"] = value
    for card_name, count in card_name_counts(snapshot).items():
        features[f"hand_card::{card_name}"] = int(count)
        semantics = card_semantics(card_name)
        features[f"hand_card_type::{card_name}"] = float(semantics["type_id"])
    for monster_name, count in monster_name_counts(snapshot).items():
        features[f"monster_name::{monster_name}"] = int(count)
    for family, count in monster_intent_family_counts(snapshot).items():
        features[f"monster_intent::{family}"] = int(count)
    for key, value in hand_semantics_counts(snapshot).items():
        features[f"semantics::{key}"] = float(value)
    features["action::family"] = action["move_family"]
    if action["card_name"]:
        normalized_action_card = normalize_card_name(action["card_name"])
        features["action::card_name"] = normalized_action_card
        features[f"action_card_present::{normalized_action_card}"] = 1
        features["action::card_in_hand_count"] = int(card_name_counts(snapshot).get(normalized_action_card, 0))
        action_semantics = card_semantics(normalized_action_card)
        features["action::type_id"] = float(action_semantics["type_id"])
        features["action::base_cost"] = float(action_semantics["base_cost"])
        features["action::base_damage"] = float(action_semantics["base_damage"])
        features["action::base_block"] = float(action_semantics["base_block"])
        features["action::draw_count"] = float(action_semantics["draw_count"])
        features["action::setup_tag"] = float(action_semantics["setup_tag"])
        features["action::payoff_tag"] = float(action_semantics["payoff_tag"])
        features["action::status_tag"] = float(action_semantics["status_tag"])
        features["action::creates_status"] = float(action_semantics["creates_status"])
        features["action::consumes_status"] = float(action_semantics["consumes_status"])
        features["action::deals_damage"] = float(action_semantics["deals_damage"])
        features["action::grants_block"] = float(action_semantics["grants_block"])
        features["interaction::payoff_cards_remaining_after_setup_proxy"] = float(
            max(hand_semantics_counts(snapshot).get("hand_payoff_count", 0.0) - action_semantics["payoff_tag"], 0.0)
        )
    if action["slot_index"] is not None:
        features["action::slot_index"] = int(action["slot_index"])
    features["action::has_target"] = int(bool(action["has_target"]))
    if action["target_index"] is not None:
        features["action::target_index"] = int(action["target_index"])
    features["action::targets_attacker"] = int(bool(action["has_target"]))
    features["interaction::action_is_setup"] = float(features.get("action::setup_tag", 0.0))
    features["interaction::action_is_payoff"] = float(features.get("action::payoff_tag", 0.0))
    features["reward::incoming_before"] = float(row.get("incoming_before") or 0.0)
    features["meta::done"] = int(bool(row.get("done")))
    if row.get("sample_origin"):
        features["meta::sample_origin"] = str(row["sample_origin"])
    if row.get("teacher_source"):
        features["meta::teacher_source"] = str(row["teacher_source"])
    if row.get("curriculum_tag"):
        features["meta::curriculum_tag"] = str(row["curriculum_tag"])
    return features


def value_feature_dict(row: dict[str, Any]) -> dict[str, Any]:
    features = transition_feature_dict(row)
    features["reward::enemy_hp_delta"] = float((row.get("reward_breakdown") or {}).get("enemy_hp_delta") or 0.0)
    features["reward::incoming_relief"] = float((row.get("reward_breakdown") or {}).get("incoming_relief") or 0.0)
    features["reward::kill_bonus"] = float((row.get("reward_breakdown") or {}).get("kill_bonus") or 0.0)
    features["reward::stabilize_bonus"] = float((row.get("reward_breakdown") or {}).get("stabilize_bonus") or 0.0)
    features["reward::player_hp_delta"] = float((row.get("reward_breakdown") or {}).get("player_hp_delta") or 0.0)
    features["reward::idle_penalty"] = float((row.get("reward_breakdown") or {}).get("idle_penalty") or 0.0)
    return features


def policy_candidate_feature_dict(row: dict[str, Any]) -> dict[str, Any]:
    features: dict[str, Any] = {}
    snapshot = row.get("snapshot_normalized_state") or row.get("state_before") or {}
    state = snapshot_state_features(snapshot)
    move = parse_move_label(row.get("candidate_move"))
    for key, value in state.items():
        features[f"state::{key}"] = value
    for key, value in hand_semantics_counts(snapshot).items():
        features[f"semantics::{key}"] = float(value)
    features["candidate::family"] = move["move_family"]
    if move["card_name"]:
        normalized_card = normalize_card_name(move["card_name"])
        semantics = card_semantics(normalized_card)
        features["candidate::card_name"] = normalized_card
        features["candidate::type_id"] = float(semantics["type_id"])
        features["candidate::base_cost"] = float(semantics["base_cost"])
        features["candidate::base_damage"] = float(semantics["base_damage"])
        features["candidate::base_block"] = float(semantics["base_block"])
        features["candidate::draw_count"] = float(semantics["draw_count"])
        features["candidate::setup_tag"] = float(semantics["setup_tag"])
        features["candidate::payoff_tag"] = float(semantics["payoff_tag"])
        features["candidate::creates_status"] = float(semantics["creates_status"])
        features["candidate::consumes_status"] = float(semantics["consumes_status"])
        features["candidate::deals_damage"] = float(semantics["deals_damage"])
        features["candidate::grants_block"] = float(semantics["grants_block"])
    if move["slot_index"] is not None:
        features["candidate::slot_index"] = int(move["slot_index"])
    features["candidate::has_target"] = int(bool(move["has_target"]))
    if move["target_index"] is not None:
        features["candidate::target_index"] = int(move["target_index"])
    features["candidate::equals_baseline"] = int(row.get("candidate_move") == row.get("baseline_action"))
    features["candidate::search_rank"] = int(row.get("candidate_rank") or 0)
    features["candidate::score_hint"] = float(row.get("candidate_score_hint") or 0.0)
    features["candidate::teacher_weight"] = float(row.get("training_weight") or row.get("sample_weight") or 0.0)
    features["interaction::candidate_is_setup"] = float(features.get("candidate::setup_tag", 0.0))
    features["interaction::candidate_is_payoff"] = float(features.get("candidate::payoff_tag", 0.0))
    if row.get("sample_origin"):
        features["meta::sample_origin"] = str(row["sample_origin"])
    if row.get("teacher_source"):
        features["meta::teacher_source"] = str(row["teacher_source"])
    if row.get("curriculum_tag"):
        features["meta::curriculum_tag"] = str(row["curriculum_tag"])
    for tag in sorted(str(tag) for tag in (row.get("sample_tags") or [])):
        features[f"tag::{tag}"] = 1
    return features


def grouped_prediction_metrics(rows: list[dict[str, Any]], scorer) -> dict[str, Any]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        grouped[str(row.get("group_id"))].append(row)

    predictions: list[dict[str, Any]] = []
    correct = 0
    eligible = 0
    for group_id, group_rows in grouped.items():
        scored = []
        for row in group_rows:
            score = float(scorer(row))
            scored.append((score, row))
        scored.sort(key=lambda item: item[0], reverse=True)
        top_row = scored[0][1]
        positives = [row for row in group_rows if bool(row.get("candidate_is_positive"))]
        if positives:
            eligible += 1
            if bool(top_row.get("candidate_is_positive")):
                correct += 1
        predictions.append(
            {
                "group_id": group_id,
                "predicted_move": top_row.get("candidate_move"),
                "baseline_action": top_row.get("baseline_action"),
                "sample_origin": top_row.get("sample_origin"),
                "sample_tags": top_row.get("sample_tags") or [],
                "top_score": scored[0][0],
                "candidate_rows": [
                    {
                        "move": row.get("candidate_move"),
                        "score": score,
                        "positive": bool(row.get("candidate_is_positive")),
                    }
                    for score, row in scored
                ],
            }
        )
    return {
        "group_count": len(grouped),
        "top1_match_rate": float(correct / eligible) if eligible else 0.0,
        "predictions": predictions,
    }


def tag_correction_summary(predictions: list[dict[str, Any]]) -> dict[str, dict[str, float | int]]:
    counts: dict[str, dict[str, float | int]] = defaultdict(lambda: {"samples": 0, "corrected": 0})
    for prediction in predictions:
        tags = [str(tag) for tag in (prediction.get("sample_tags") or [])]
        candidate_rows = prediction.get("candidate_rows") or []
        predicted_positive = bool(candidate_rows and candidate_rows[0].get("positive"))
        for tag in tags:
            counts[tag]["samples"] += 1
            if predicted_positive:
                counts[tag]["corrected"] += 1
    for tag, summary in counts.items():
        samples = int(summary["samples"])
        corrected = int(summary["corrected"])
        summary["correction_rate"] = float(corrected / samples) if samples else 0.0
    return dict(counts)


def load_policy_seed_rows(patterns: str) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for pattern in patterns.split(","):
        for path_str in glob.glob(pattern.strip()):
            path = Path(path_str)
            for _, sample in iter_jsonl(path):
                snapshot = preference_state_to_snapshot(sample.get("state") or {})
                preferred = str(sample.get("preferred_action") or "")
                chosen = str(sample.get("chosen_action") or "")
                label_strength = preference_label_strength(sample)
                group_id = str(sample.get("sample_id") or f"policy_seed::{path.stem}")
                sample_tags = sample_tags_from_preference_sample(sample)
                training_weight = 1.0 if label_strength == "oracle_strong" else 0.6
                candidates = [preferred, chosen]
                deduped = []
                for move in candidates:
                    if move and move not in deduped:
                        deduped.append(move)
                for index, move in enumerate(deduped):
                    rows.append(
                        {
                            "dataset_kind": "combat_policy",
                            "split": stable_split(group_id),
                            "group_id": group_id,
                            "sample_origin": "policy_seed_set",
                            "teacher_source": str(sample.get("preferred_source") or "offline_audit_search"),
                            "curriculum_tag": str(sample.get("preference_kind") or "policy_seed"),
                            "state_source": str(sample.get("state_source") or "reconstructed_live_replay_state"),
                            "label_source": str(sample.get("preferred_source") or "offline_audit_search"),
                            "label_strength": label_strength,
                            "sample_tags": sample_tags,
                            "training_weight": training_weight,
                            "snapshot_normalized_state": snapshot,
                            "state_before": snapshot,
                            "candidate_move": move,
                            "candidate_rank": index,
                            "candidate_score_hint": float(sample.get("score_gap") or 0.0) if move == preferred else 0.0,
                            "candidate_is_positive": move == preferred,
                            "baseline_action": chosen,
                            "preferred_action": preferred,
                            "sample_id": group_id,
                        }
                    )
    return rows


def load_oracle_policy_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if not path.exists():
        return rows
    for _, sample in iter_jsonl(path):
        best_moves = [str(move) for move in (sample.get("oracle_equivalent_best_moves") or []) if move]
        top_candidates = list(sample.get("oracle_top_candidates") or [])
        baseline_action = str(sample.get("baseline_chosen_move") or "")
        if not best_moves or not top_candidates:
            continue
        if not bool(sample.get("oracle_disagrees_with_baseline")):
            continue
        label_strength = str(sample.get("label_strength") or "oracle_preference")
        if label_strength not in {"oracle_strong", "oracle_preference"}:
            continue
        baseline_row = sample.get("baseline_row") or {}
        snapshot = baseline_row.get("snapshot_normalized_state") or {}
        group_id = f"oracle::{sample.get('run_id')}::{sample.get('frame')}"
        for index, candidate in enumerate(top_candidates):
            move = str(candidate.get("move_label") or "")
            if not move:
                continue
            rows.append(
                {
                    "dataset_kind": "combat_policy",
                    "split": stable_split(group_id),
                    "group_id": group_id,
                    "sample_origin": "archived_clean_run",
                    "teacher_source": str(sample.get("label_source") or "offline_oracle"),
                    "curriculum_tag": "archived_oracle_policy",
                    "state_source": str(sample.get("state_source") or "validated_livecomm_audit"),
                    "label_source": str(sample.get("label_source") or "offline_oracle"),
                    "label_strength": label_strength,
                    "sample_tags": sample_tags_from_oracle_row(sample),
                    "training_weight": 1.0 if label_strength == "oracle_strong" else 0.55,
                    "snapshot_normalized_state": snapshot,
                    "state_before": snapshot,
                    "candidate_move": move,
                    "candidate_rank": index,
                    "candidate_score_hint": float(candidate.get("score") or candidate.get("avg_score") or 0.0),
                    "candidate_is_positive": move in set(best_moves),
                    "baseline_action": baseline_action,
                    "preferred_action": best_moves[0],
                    "sample_id": group_id,
                }
            )
    return rows
