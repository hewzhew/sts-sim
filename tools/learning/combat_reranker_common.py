#!/usr/bin/env python3
from __future__ import annotations

import json
import re
from collections import defaultdict
from pathlib import Path
from typing import Any, Callable


def iter_jsonl(path: Path):
    with path.open("r", encoding="utf-8") as handle:
        for line_no, line in enumerate(handle, start=1):
            text = line.strip()
            if text:
                yield line_no, json.loads(text)


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, ensure_ascii=False)
        handle.write("\n")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False) + "\n")


def move_family(move_label: str | None) -> str:
    label = str(move_label or "")
    if label.startswith("Play #"):
        return "play_card"
    if label.startswith("UsePotion#"):
        return "use_potion"
    if label.startswith("UsePotion"):
        return "use_potion"
    if label.startswith("EndTurn"):
        return "end_turn"
    if label.startswith("Proceed"):
        return "proceed"
    if label.startswith("Choose"):
        return "choose"
    return (label.split(" ", 1)[0] or "unknown").lower()


def normalize_outcome(value: Any) -> str:
    text = str(value or "").strip().lower()
    if text in {"survives", "survive"}:
        return "survives"
    if text in {"dies", "die", "defeat"}:
        return "dies"
    if text in {"lethal_win", "lethalwin", "victory"}:
        return "lethal_win"
    return text or "unknown"


def parse_move_label(move_label: str | None) -> dict[str, Any]:
    label = str(move_label or "")
    result: dict[str, Any] = {
        "move_label": label,
        "move_family": move_family(label),
        "card_name": None,
        "slot_index": None,
        "target_index": None,
        "has_target": False,
    }
    if label.startswith("Play #"):
        match = re.match(r"^Play #(?P<slot>\d+)\s+(?P<card>.+?)(?: @(?P<target>\d+))?$", label)
        if match:
            result["slot_index"] = int(match.group("slot"))
            result["card_name"] = match.group("card")
            if match.group("target") is not None:
                result["target_index"] = int(match.group("target"))
                result["has_target"] = True
        return result
    if label.startswith("UsePotion#"):
        match = re.match(r"^UsePotion#(?P<slot>\d+)(?: @(?P<target>\d+))?$", label)
        if match:
            result["slot_index"] = int(match.group("slot"))
            if match.group("target") is not None:
                result["target_index"] = int(match.group("target"))
                result["has_target"] = True
        return result
    if " @" in label:
        head, tail = label.rsplit(" @", 1)
        if tail.isdigit():
            result["target_index"] = int(tail)
            result["has_target"] = True
            result["card_name"] = head
            return result
    if " " in label:
        result["card_name"] = label.split(" ", 1)[1]
    return result


def action_debug_to_move_label(action_debug: str | None) -> str:
    text = str(action_debug or "")
    match = re.match(r"^PlayCard \{ card_index: (?P<slot>\d+), target: (?P<target>Some\((?P<target_idx>\d+)\)|None) \}$", text)
    if match:
        slot = int(match.group("slot")) + 1
        target_idx = match.group("target_idx")
        if target_idx is None:
            return f"Play #{slot}"
        return f"Play #{slot} @{int(target_idx)}"
    match = re.match(r"^UsePotion \{ potion_index: (?P<slot>\d+), target: (?P<target>Some\((?P<target_idx>\d+)\)|None) \}$", text)
    if match:
        slot = int(match.group("slot"))
        target_idx = match.group("target_idx")
        if target_idx is None:
            return f"UsePotion#{slot}"
        return f"UsePotion#{slot} @{int(target_idx)}"
    if text == "EndTurn":
        return "EndTurn"
    return text


def stable_split(group_id: str) -> str:
    bucket = sum(ord(ch) for ch in group_id) % 20
    if bucket < 14:
        return "train"
    if bucket < 17:
        return "val"
    return "test"


def positive_move_set(row: dict[str, Any]) -> set[str]:
    return {
        str(move)
        for move in (row.get("oracle_equivalent_best_moves") or [])
        if isinstance(move, str) and move
    }


def baseline_in_best_bucket(row: dict[str, Any]) -> bool:
    baseline = str(row.get("baseline_chosen_move") or "")
    return bool(baseline and baseline in positive_move_set(row))


def baseline_outcome(row: dict[str, Any]) -> str | None:
    baseline = row.get("baseline_chosen_move")
    if not baseline:
        return None
    for candidate in row.get("oracle_top_candidates") or []:
        if candidate.get("move_label") == baseline:
            return str(candidate.get("outcome") or "")
    return None


def oracle_label_strength(row: dict[str, Any]) -> str:
    return str(row.get("label_strength") or "baseline_weak")


def is_oracle_disagreement(row: dict[str, Any]) -> bool:
    return not baseline_in_best_bucket(row)


def is_strong_oracle_row(row: dict[str, Any]) -> bool:
    return oracle_label_strength(row) == "oracle_strong"


def curriculum_tag_from_spec_name(spec_name: str | None) -> str:
    name = str(spec_name or "").lower()
    if any(token in name for token in ("potion",)):
        return "potion_bridge"
    if any(token in name for token in ("survival_override", "guardrail")):
        return "survival_override"
    if "power_through" in name:
        return "survival_override"
    if any(token in name for token in ("fire_breathing", "second_wind", "dark_embrace", "status", "exhaust")):
        return "status_exhaust_draw"
    if any(token in name for token in ("rage", "spot_weakness", "setup", "corruption", "flex", "inflame")):
        return "setup_before_payoff"
    return "attack_over_defend"


def preference_label_strength(sample: dict[str, Any]) -> str:
    chosen_outcome = normalize_outcome(sample.get("chosen_outcome"))
    preferred_outcome = normalize_outcome(sample.get("preferred_outcome"))
    order = {"dies": 0, "unknown": 1, "survives": 2, "lethal_win": 3}
    if order.get(preferred_outcome, 1) > order.get(chosen_outcome, 1):
        return "oracle_strong"
    gap = int(sample.get("score_gap") or 0)
    if gap >= 150:
        return "oracle_strong"
    return "oracle_preference"


def preference_state_to_snapshot(state: dict[str, Any]) -> dict[str, Any]:
    monsters = []
    for index, monster in enumerate(state.get("monsters") or []):
        monsters.append(
            {
                "id": monster.get("name") or f"monster_{index}",
                "name": monster.get("name") or f"monster_{index}",
                "current_hp": int(monster.get("hp") or 0),
                "max_hp": int(monster.get("hp") or 0),
                "block": int(monster.get("block") or 0),
                "intent": monster.get("intent"),
                "powers": [],
            }
        )
    hand = [{"name": str(card), "id": str(card), "cost": 0, "upgrades": 0} for card in (state.get("hand") or [])]
    return {
        "act": 0,
        "floor": 0,
        "gold": 0,
        "player": {
            "current_hp": int(state.get("player_hp") or 0),
            "max_hp": int(state.get("player_max_hp") or state.get("player_hp") or 0),
            "block": int(state.get("player_block") or 0),
            "energy": int(state.get("energy") or 0),
            "powers": [],
        },
        "monsters": monsters,
        "zones": {
            "hand_count": len(hand),
            "draw_count": int(state.get("draw_count") or 0),
            "discard_count": int(state.get("discard_count") or 0),
            "exhaust_count": int(state.get("exhaust_count") or 0),
            "hand": hand,
            "draw": [],
            "discard": [],
            "exhaust": [],
        },
    }


def sample_tags_from_oracle_row(row: dict[str, Any]) -> list[str]:
    tags: list[str] = []
    baseline_move = str(row.get("baseline_chosen_move") or "")
    best_moves = positive_move_set(row)
    baseline_best = bool(baseline_move and baseline_move in best_moves)
    if baseline_outcome(row) == "dies" and normalize_outcome(row.get("oracle_outcome_bucket")) in {"survives", "lethal_win"}:
        tags.append("oracle_save")
    if row.get("oracle_disagrees_with_baseline"):
        tags.append("hard_disagreement")
    if int(row.get("oracle_best_bucket_size") or 0) > 1:
        tags.append("equivalent_best_tie")
    baseline_row = row.get("baseline_row") or {}
    reasons = set(str(reason) for reason in (baseline_row.get("reasons") or []))
    if "sequencing_conflict" in reasons and "Flex" in " ".join(best_moves):
        tags.append("setup_flex_missed")
    pressure = int(baseline_row.get("value_incoming") or 0)
    if "Defend" in baseline_move and pressure <= 5 and not baseline_best:
        tags.append("overdefend_light_pressure")
    snapshot = baseline_row.get("snapshot_normalized_state") or {}
    living_hps = [
        int(monster.get("current_hp") or 0)
        for monster in (snapshot.get("monsters") or [])
        if int(monster.get("current_hp") or 0) > 0
    ]
    lowest_hp = min(living_hps, default=999)
    if lowest_hp <= 12 and not baseline_best and (
        not baseline_move.startswith("Play #") or "Defend" in baseline_move
    ):
        tags.append("kill_now_missed")
    return sorted(set(tags))


def sample_tags_from_preference_sample(sample: dict[str, Any]) -> list[str]:
    tags: list[str] = []
    chosen_action = str(sample.get("chosen_action") or "")
    preferred_action = str(sample.get("preferred_action") or "")
    chosen_outcome = normalize_outcome(sample.get("chosen_outcome"))
    preferred_outcome = normalize_outcome(sample.get("preferred_outcome"))
    state = sample.get("state") or {}
    incoming = int(state.get("incoming") or 0)
    living_hps = [
        int(monster.get("hp") or 0)
        for monster in (state.get("monsters") or [])
        if int(monster.get("hp") or 0) > 0
    ]
    lowest_hp = min(living_hps, default=999)
    if chosen_outcome == "dies" and preferred_outcome in {"survives", "lethal_win"}:
        tags.append("oracle_save")
    if "Defend" in chosen_action and incoming <= 5 and "Defend" not in preferred_action:
        tags.append("overdefend_light_pressure")
    if lowest_hp <= 12 and "Defend" in chosen_action and "Defend" not in preferred_action:
        tags.append("kill_now_missed")
    if "Flex" in preferred_action and "Flex" not in chosen_action:
        tags.append("setup_flex_missed")
    if sample.get("preference_kind"):
        tags.append(str(sample["preference_kind"]))
    return sorted(set(tags))


def training_weight(row: dict[str, Any]) -> float:
    label_strength = oracle_label_strength(row)
    if label_strength == "oracle_strong":
        weight = 1.0
    elif label_strength == "oracle_preference":
        weight = 0.4
    else:
        return 0.0
    bucket_size = int(row.get("oracle_best_bucket_size") or 0)
    if bucket_size > 1:
        weight *= 0.5
    margin = row.get("oracle_margin")
    if margin is not None and int(margin) < 150:
        weight *= 0.75
    return round(weight, 4)


def snapshot_state_features(snapshot: dict[str, Any] | None) -> dict[str, Any]:
    state = snapshot or {}
    player = state.get("player") or {}
    zones = state.get("zones") or {}
    monsters = state.get("monsters") or []
    living = [monster for monster in monsters if int(monster.get("current_hp") or 0) > 0]
    features = {
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
        "intent_attackers": sum(
            1 for monster in living if "ATTACK" in str(monster.get("intent") or "")
        ),
    }
    return features


def candidate_feature_dict(row: dict[str, Any]) -> dict[str, Any]:
    features: dict[str, Any] = {}
    move = parse_move_label(row.get("candidate_move"))
    snapshot = row.get("snapshot_normalized_state") or {}
    state_features = snapshot_state_features(snapshot)
    reasons = set(str(reason) for reason in (row.get("reasons") or []))
    for key, value in state_features.items():
        features[f"state::{key}"] = value
    features["move::family"] = move["move_family"]
    if move["card_name"]:
        features["move::card_name"] = move["card_name"]
    if move["slot_index"] is not None:
        features["move::slot_index"] = int(move["slot_index"])
    if move["target_index"] is not None:
        features["move::target_index"] = int(move["target_index"])
    features["move::has_target"] = int(bool(move["has_target"]))
    features["move::equals_baseline"] = int(row.get("candidate_move") == row.get("baseline_chosen_move"))
    features["move::equals_heuristic"] = int(row.get("candidate_move") == row.get("heuristic_move"))
    features["move::equals_search"] = int(row.get("candidate_move") == row.get("search_move"))
    features["pressure::visible_incoming"] = int(row.get("visible_incoming") or 0)
    features["pressure::visible_unblocked"] = int(row.get("visible_unblocked") or 0)
    features["pressure::belief_expected_incoming"] = float(row.get("belief_expected_incoming") or 0.0)
    features["pressure::belief_max_incoming"] = int(row.get("belief_max_incoming") or 0)
    features["pressure::value_incoming"] = int(row.get("value_incoming") or 0)
    features["pressure::value_unblocked"] = int(row.get("value_unblocked") or 0)
    features["pressure::survival_guard_incoming"] = int(row.get("survival_guard_incoming") or 0)
    features["pressure::survival_guard_unblocked"] = int(row.get("survival_guard_unblocked") or 0)
    features["pressure::attack_probability"] = float(row.get("belief_attack_probability") or 0.0)
    features["pressure::urgent_probability"] = float(row.get("belief_urgent_probability") or 0.0)
    features["pressure::lethal_probability"] = float(row.get("belief_lethal_probability") or 0.0)
    features["search::top_gap"] = float(row.get("top_gap") or 0.0)
    features["search::sequence_bonus"] = float(row.get("sequence_bonus") or 0.0)
    features["search::sequence_frontload_bonus"] = float(row.get("sequence_frontload_bonus") or 0.0)
    features["search::sequence_defer_bonus"] = float(row.get("sequence_defer_bonus") or 0.0)
    features["search::sequence_branch_bonus"] = float(row.get("sequence_branch_bonus") or 0.0)
    features["search::sequence_downside_penalty"] = float(row.get("sequence_downside_penalty") or 0.0)
    features["search::heuristic_search_gap"] = int(bool(row.get("heuristic_search_gap")))
    features["search::tight_root_gap"] = int(bool(row.get("tight_root_gap")))
    features["search::large_sequence_bonus"] = int(bool(row.get("large_sequence_bonus")))
    if row.get("branch_family"):
        features["search::branch_family"] = str(row["branch_family"])
    if row.get("sequencing_rationale_key"):
        features["search::sequencing_rationale"] = str(row["sequencing_rationale_key"])
    if row.get("branch_rationale_key"):
        features["search::branch_rationale"] = str(row["branch_rationale_key"])
    if row.get("downside_rationale_key"):
        features["search::downside_rationale"] = str(row["downside_rationale_key"])
    for reason in sorted(reasons):
        features[f"reason::{reason}"] = 1
    if row.get("snapshot_trigger_kind"):
        features["snapshot::trigger_kind"] = str(row["snapshot_trigger_kind"])
    features["snapshot::has_state"] = int(row.get("snapshot_normalized_state") is not None)
    features["training::candidate_count"] = int(row.get("candidate_count") or 0)
    features["training::candidate_index"] = int(row.get("candidate_index") or 0)
    if row.get("candidate_source"):
        features["training::candidate_source"] = str(row["candidate_source"])
    if row.get("candidate_search_rank") is not None:
        features["training::candidate_search_rank"] = int(row["candidate_search_rank"])
    for field in (
        "candidate_search_avg_score",
        "candidate_search_order_score",
        "candidate_search_leaf_score",
        "candidate_search_sequence_bonus",
        "candidate_search_sequence_frontload_bonus",
        "candidate_search_sequence_defer_bonus",
        "candidate_search_sequence_branch_bonus",
        "candidate_search_sequence_downside_penalty",
        "candidate_projected_unblocked",
        "candidate_projected_enemy_total",
    ):
        features[f"candidate::{field}"] = float(row.get(field) or 0.0)
    features["candidate::survives"] = int(bool(row.get("candidate_survives")))
    if row.get("candidate_branch_family"):
        features["candidate::branch_family"] = str(row["candidate_branch_family"])
    if row.get("sample_origin"):
        features["meta::sample_origin"] = str(row["sample_origin"])
    if row.get("teacher_source"):
        features["meta::teacher_source"] = str(row["teacher_source"])
    if row.get("curriculum_tag"):
        features["meta::curriculum_tag"] = str(row["curriculum_tag"])
    for tag in sorted(str(tag) for tag in (row.get("sample_tags") or [])):
        features[f"tag::{tag}"] = 1
    return features


def group_rows(rows: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        grouped[str(row["group_id"])].append(row)
    for group in grouped.values():
        group.sort(key=lambda item: int(item.get("candidate_index") or 0))
    return grouped


def evaluate_grouped_predictions(
    rows: list[dict[str, Any]],
    predict_score: Callable[[dict[str, Any]], float],
) -> dict[str, Any]:
    grouped = group_rows(rows)
    frame_count = 0
    top1_match = 0
    pairwise_total = 0
    pairwise_correct = 0
    strong_frames = 0
    strong_corrected = 0
    death_escape_frames = 0
    death_escape_corrected = 0
    predictions: list[dict[str, Any]] = []
    for group_id, group in grouped.items():
        frame_count += 1
        scored = []
        for row in group:
            score = float(predict_score(row))
            scored.append((score, row))
        scored.sort(key=lambda item: item[0], reverse=True)
        predicted = scored[0][1]
        predicted_move = str(predicted.get("candidate_move") or "")
        positive_moves = {
            str(row.get("candidate_move") or "")
            for row in group
            if bool(row.get("candidate_is_positive"))
        }
        top1_match += int(predicted_move in positive_moves)
        positives = [item for item in scored if item[1].get("candidate_is_positive")]
        negatives = [item for item in scored if not item[1].get("candidate_is_positive")]
        for pos_score, _ in positives:
            for neg_score, _ in negatives:
                pairwise_total += 1
                pairwise_correct += int(pos_score > neg_score)
        template = group[0]
        is_strong = str(template.get("label_strength") or "") == "oracle_strong"
        baseline_in_best = bool(template.get("baseline_in_best_bucket"))
        if is_strong and not baseline_in_best:
            strong_frames += 1
            strong_corrected += int(predicted_move in positive_moves)
        baseline_outcome = str(template.get("baseline_outcome") or "")
        oracle_outcome = str(template.get("oracle_outcome_bucket") or "")
        if baseline_outcome == "dies" and oracle_outcome in {"survives", "lethal_win"}:
            death_escape_frames += 1
            death_escape_corrected += int(predicted_move in positive_moves)
        candidate_scores = [
            {
                "candidate_move": str(row.get("candidate_move") or ""),
                "score": round(float(score), 6),
                "is_positive": bool(row.get("candidate_is_positive")),
            }
            for score, row in scored
        ]
        predictions.append(
            {
                "group_id": group_id,
                "run_id": template.get("run_id"),
                "frame_count": template.get("frame_count"),
                "predicted_move": predicted_move,
                "baseline_chosen_move": template.get("baseline_chosen_move"),
                "oracle_equivalent_best_moves": template.get("oracle_equivalent_best_moves") or [],
                "label_strength": template.get("label_strength"),
                "baseline_in_best_bucket": baseline_in_best,
                "baseline_outcome": baseline_outcome or None,
                "oracle_outcome_bucket": oracle_outcome or None,
                "sample_origin": template.get("sample_origin"),
                "teacher_source": template.get("teacher_source"),
                "curriculum_tag": template.get("curriculum_tag"),
                "sample_tags": template.get("sample_tags") or [],
                "scores": candidate_scores,
            }
        )
    return {
        "frame_count": frame_count,
        "top1_match": top1_match,
        "top1_match_rate": round(top1_match / float(max(frame_count, 1)), 6),
        "pairwise_total": pairwise_total,
        "pairwise_correct": pairwise_correct,
        "pairwise_agreement": round(pairwise_correct / float(max(pairwise_total, 1)), 6),
        "strong_disagreement_frames": strong_frames,
        "strong_disagreement_corrected": strong_corrected,
        "strong_disagreement_correction_rate": round(
            strong_corrected / float(max(strong_frames, 1)),
            6,
        ),
        "baseline_dies_oracle_lives_frames": death_escape_frames,
        "baseline_dies_oracle_lives_corrected": death_escape_corrected,
        "baseline_dies_oracle_lives_correction_rate": round(
            death_escape_corrected / float(max(death_escape_frames, 1)),
            6,
        ),
        "predictions": predictions,
    }


def tag_correction_summary(predictions: list[dict[str, Any]]) -> dict[str, dict[str, float | int]]:
    stats: dict[str, dict[str, float | int]] = {}
    for prediction in predictions:
        best = set(prediction.get("oracle_equivalent_best_moves") or [])
        corrected = int(prediction.get("predicted_move") in best)
        for tag in prediction.get("sample_tags") or []:
            current = stats.setdefault(str(tag), {"frames": 0, "corrected": 0})
            current["frames"] = int(current["frames"]) + 1
            current["corrected"] = int(current["corrected"]) + corrected
    for value in stats.values():
        frames = int(value["frames"]) or 1
        value["correction_rate"] = round(int(value["corrected"]) / float(frames), 6)
    return dict(sorted(stats.items()))


def top_scoring_mistakes(predictions: list[dict[str, Any]], limit: int = 20) -> list[dict[str, Any]]:
    mistakes = [row for row in predictions if row.get("predicted_move") not in set(row.get("oracle_equivalent_best_moves") or [])]
    def severity(item: dict[str, Any]) -> tuple[float, int]:
        scores = item.get("scores") or []
        if len(scores) < 2:
            return (0.0, 0)
        top_score = float(scores[0].get("score") or 0.0)
        second_score = float(scores[1].get("score") or 0.0)
        return (abs(top_score - second_score), len(item.get("oracle_equivalent_best_moves") or []))
    mistakes.sort(key=severity, reverse=True)
    return mistakes[:limit]
