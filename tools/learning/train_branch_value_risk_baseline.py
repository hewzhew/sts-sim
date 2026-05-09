#!/usr/bin/env python3
"""Train dependency-free baselines on Branch Value/Risk datasets.

This is a learnability smoke test for branch outcomes. It is not an action
policy trainer and does not consume or produce winner/preference labels.
"""

from __future__ import annotations

import argparse
import heapq
import hashlib
import json
import math
import random
from pathlib import Path
from typing import Any, Iterable


def iter_jsonl(path: Path) -> Iterable[dict[str, Any]]:
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                yield json.loads(line)


def stable_hash(text: str) -> int:
    return int.from_bytes(hashlib.blake2b(text.encode("utf-8"), digest_size=8).digest(), "little")


def split_for_seed(seed: Any, train_ratio: float) -> str:
    bucket = stable_hash(f"seed:{seed}") % 10_000
    return "train" if bucket < int(train_ratio * 10_000) else "test"


def hash_feature(token: str, dim: int) -> int:
    return stable_hash(token) % dim


def add_feature(features: dict[int, float], token: str, dim: int, value: float = 1.0) -> None:
    index = hash_feature(token, dim)
    features[index] = features.get(index, 0.0) + value


def bucket(value: Any, width: int) -> str:
    try:
        number = int(value)
    except (TypeError, ValueError):
        return "unknown"
    return str((number // width) * width)


def bool_token(value: Any) -> str:
    if value is True:
        return "true"
    if value is False:
        return "false"
    return "unknown"


def add_numeric(
    features: dict[int, float],
    name: str,
    value: Any,
    dim: int,
    scale: float,
) -> None:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return
    if not math.isfinite(number):
        return
    add_feature(features, f"num:{name}", dim, number / scale)


def branch_features(row: dict[str, Any], dim: int) -> dict[int, float]:
    features: dict[int, float] = {}
    obs = row.get("observation_features") or {}
    candidate = row.get("candidate") or {}
    context = row.get("decision_context_features") or {}
    evidence = row.get("evidence_features") or {}

    add_feature(features, "bias", dim)
    add_feature(features, f"decision_type={row.get('decision_type')}", dim)
    add_feature(features, f"action_kind={candidate.get('action_kind')}", dim)
    add_feature(features, f"action_type={candidate.get('action_type')}", dim)
    add_feature(features, f"card_id={candidate.get('card_id')}", dim)
    add_feature(features, f"card_type={candidate.get('card_type_id')}", dim)
    add_feature(features, f"card_cost={candidate.get('card_cost')}", dim)
    add_feature(features, f"target_present={candidate.get('target') is not None}", dim)

    for key in (
        "card_draws_cards",
        "card_exhaust",
        "card_applies_vulnerable",
        "card_applies_weak",
        "card_scaling_piece",
        "card_starter_basic",
    ):
        add_feature(features, f"{key}={bool_token(candidate.get(key))}", dim)

    for key, width in (
        ("floor", 3),
        ("current_hp", 10),
        ("gold", 50),
        ("deck_size", 5),
        ("combat_turn_count", 2),
        ("combat_energy", 1),
        ("visible_incoming_damage", 5),
        ("alive_monster_count", 1),
        ("total_monster_hp", 10),
        ("hand_count", 2),
        ("draw_count", 5),
        ("discard_count", 5),
        ("exhaust_count", 2),
    ):
        add_feature(features, f"{key}_bucket={bucket(obs.get(key), width)}", dim)

    for key, scale in (
        ("current_hp", 100.0),
        ("max_hp", 100.0),
        ("gold", 300.0),
        ("deck_size", 40.0),
        ("combat_turn_count", 10.0),
        ("combat_energy", 5.0),
        ("visible_incoming_damage", 50.0),
        ("player_block", 50.0),
        ("alive_monster_count", 5.0),
        ("total_monster_hp", 200.0),
        ("hand_count", 10.0),
        ("draw_count", 40.0),
        ("discard_count", 40.0),
        ("exhaust_count", 20.0),
        ("deck_attack_count", 30.0),
        ("deck_skill_count", 30.0),
        ("deck_power_count", 20.0),
        ("deck_damage_card_count", 30.0),
        ("deck_block_card_count", 30.0),
        ("deck_draw_card_count", 20.0),
        ("deck_exhaust_card_count", 20.0),
        ("deck_scaling_card_count", 20.0),
    ):
        add_numeric(features, key, obs.get(key), dim, scale)

    for key, scale in (
        ("card_cost", 5.0),
        ("card_base_damage", 50.0),
        ("card_base_block", 50.0),
    ):
        add_numeric(features, key, candidate.get(key), dim, scale)

    for key, width in (
        ("legal_candidate_count", 2),
        ("playable_candidate_count", 2),
        ("playable_unique_card_count", 2),
        ("playable_attack_candidate_count", 2),
        ("playable_block_candidate_count", 2),
        ("playable_draw_candidate_count", 1),
        ("playable_debuff_candidate_count", 1),
        ("playable_exhaust_candidate_count", 1),
        ("playable_setup_candidate_count", 1),
        ("playable_candidate_damage_sum", 10),
        ("playable_candidate_block_sum", 10),
        ("playable_unique_damage_sum", 10),
        ("playable_unique_block_sum", 10),
        ("incoming_minus_current_block", 5),
    ):
        add_feature(features, f"{key}_bucket={bucket(context.get(key), width)}", dim)

    for key, scale in (
        ("legal_candidate_count", 10.0),
        ("playable_candidate_count", 10.0),
        ("playable_unique_card_count", 10.0),
        ("playable_attack_candidate_count", 10.0),
        ("playable_block_candidate_count", 10.0),
        ("playable_draw_candidate_count", 5.0),
        ("playable_debuff_candidate_count", 5.0),
        ("playable_exhaust_candidate_count", 5.0),
        ("playable_setup_candidate_count", 5.0),
        ("playable_zero_cost_candidate_count", 5.0),
        ("playable_one_cost_candidate_count", 10.0),
        ("playable_two_plus_cost_candidate_count", 5.0),
        ("playable_candidate_damage_sum", 60.0),
        ("playable_candidate_block_sum", 60.0),
        ("playable_candidate_max_damage", 30.0),
        ("playable_candidate_max_block", 30.0),
        ("playable_unique_damage_sum", 60.0),
        ("playable_unique_block_sum", 60.0),
        ("incoming_minus_current_block", 50.0),
    ):
        add_numeric(features, key, context.get(key), dim, scale)

    for key in (
        "end_turn_legal",
        "end_turn_with_playable_cards",
        "end_turn_with_unspent_energy",
    ):
        add_feature(features, f"{key}={bool_token(context.get(key))}", dim)

    if candidate.get("action_kind") == "end_turn":
        add_feature(
            features,
            f"candidate_end_turn_with_playable_cards={bool_token(context.get('end_turn_with_playable_cards'))}",
            dim,
        )
        add_feature(
            features,
            f"candidate_end_turn_with_unspent_energy={bool_token(context.get('end_turn_with_unspent_energy'))}",
            dim,
        )
        add_numeric(
            features,
            "candidate_end_turn_unique_damage_opportunity",
            context.get("playable_unique_damage_sum"),
            dim,
            60.0,
        )
        add_numeric(
            features,
            "candidate_end_turn_unique_block_opportunity",
            context.get("playable_unique_block_sum"),
            dim,
            60.0,
        )

    add_feature(features, f"evidence_scope={evidence.get('evidence_scope')}", dim)
    add_feature(
        features,
        f"evidence_horizon_lt_label_horizon={bool_token(evidence.get('evidence_horizon_lt_label_horizon'))}",
        dim,
    )
    add_feature(
        features,
        f"one_step_available={bool_token(evidence.get('one_step_available'))}",
        dim,
    )
    add_feature(
        features,
        f"one_step_action_kind={evidence.get('one_step_action_kind')}",
        dim,
    )
    add_feature(
        features,
        f"one_step_decision_type_after={evidence.get('one_step_decision_type_after')}",
        dim,
    )
    add_feature(features, f"one_step_result={evidence.get('one_step_result')}", dim)
    add_feature(
        features,
        f"one_step_terminal_reason={evidence.get('one_step_terminal_reason')}",
        dim,
    )
    add_feature(
        features,
        f"one_step_terminated={bool_token(evidence.get('one_step_terminated'))}",
        dim,
    )
    add_feature(
        features,
        f"one_step_truncated={bool_token(evidence.get('one_step_truncated'))}",
        dim,
    )
    for key, width in (
        ("one_step_hp_delta_from_observation", 5),
        ("one_step_legal_action_count", 2),
        ("one_step_forced_engine_ticks", 5),
        ("one_step_combat_win_count", 1),
    ):
        add_feature(features, f"{key}_bucket={bucket(evidence.get(key), width)}", dim)
    for key, scale in (
        ("one_step_reward", 3.0),
        ("one_step_hp_delta_from_observation", 30.0),
        ("one_step_legal_action_count", 10.0),
        ("one_step_forced_engine_ticks", 50.0),
        ("one_step_combat_win_count", 5.0),
    ):
        add_numeric(features, key, evidence.get(key), dim, scale)

    # A small amount of target context helps pair-diff without becoming an action label.
    add_feature(features, f"source={row.get('label_policy', {}).get('source')}", dim)
    return features


def dot(weights: list[float], features: dict[int, float]) -> float:
    return sum(weights[index] * value for index, value in features.items())


def subtract_features(left: dict[int, float], right: dict[int, float]) -> dict[int, float]:
    out = dict(left)
    for index, value in right.items():
        out[index] = out.get(index, 0.0) - value
        if abs(out[index]) < 1e-12:
            del out[index]
    return out


def sigmoid(value: float) -> float:
    if value >= 0:
        z = math.exp(-value)
        return 1.0 / (1.0 + z)
    z = math.exp(value)
    return z / (1.0 + z)


def target_value(row: dict[str, Any], target: str) -> float:
    targets = row.get("targets") or {}
    value = targets.get(target)
    if isinstance(value, bool):
        return 1.0 if value else 0.0
    return float(value)


def safe_rows(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    safe: list[dict[str, Any]] = []
    for row in rows:
        if row.get("trainable_as_action_label") is not False:
            continue
        if (row.get("label_policy") or {}).get("action_label") is not False:
            continue
        safe.append(row)
    return safe


class BucketStats:
    def __init__(self) -> None:
        self.count = 0
        self.abs_error_sum = 0.0
        self.sq_error_sum = 0.0
        self.target_sum = 0.0
        self.pred_sum = 0.0

    def add(self, target: float, pred: float) -> None:
        error = pred - target
        self.count += 1
        self.abs_error_sum += abs(error)
        self.sq_error_sum += error * error
        self.target_sum += target
        self.pred_sum += pred

    def to_dict(self) -> dict[str, float | int]:
        return {
            "count": self.count,
            "mae": self.abs_error_sum / self.count if self.count else 0.0,
            "rmse": math.sqrt(self.sq_error_sum / self.count) if self.count else 0.0,
            "target_mean": self.target_sum / self.count if self.count else 0.0,
            "pred_mean": self.pred_sum / self.count if self.count else 0.0,
        }


def top_bucket_stats(
    rows: list[dict[str, Any]],
    y_true: list[float],
    y_pred: list[float],
    bucket_fn,
    *,
    min_count: int,
    top_k: int,
) -> list[dict[str, Any]]:
    buckets: dict[str, BucketStats] = {}
    for row, target, pred in zip(rows, y_true, y_pred):
        key = bucket_fn(row)
        buckets.setdefault(key, BucketStats()).add(target, pred)
    ranked = []
    for key, stats in buckets.items():
        if stats.count < min_count:
            continue
        item = stats.to_dict()
        item["bucket"] = key
        ranked.append(item)
    ranked.sort(key=lambda item: (-float(item["mae"]), -int(item["count"]), str(item["bucket"])))
    return ranked[:top_k]


def branch_bucket_functions() -> dict[str, Any]:
    return {
        "action_kind": lambda row: str((row.get("candidate") or {}).get("action_kind")),
        "card_id": lambda row: str((row.get("candidate") or {}).get("card_id")),
        "card_type": lambda row: str((row.get("candidate") or {}).get("card_type_id")),
        "card_cost": lambda row: str((row.get("candidate") or {}).get("card_cost")),
        "card_damage_bucket": lambda row: bucket(
            (row.get("candidate") or {}).get("card_base_damage"), 5
        ),
        "card_block_bucket": lambda row: bucket(
            (row.get("candidate") or {}).get("card_base_block"), 5
        ),
        "incoming_bucket": lambda row: bucket(
            (row.get("observation_features") or {}).get("visible_incoming_damage"), 5
        ),
        "monster_hp_bucket": lambda row: bucket(
            (row.get("observation_features") or {}).get("total_monster_hp"), 10
        ),
        "current_hp_bucket": lambda row: bucket(
            (row.get("observation_features") or {}).get("current_hp"), 10
        ),
        "turn_bucket": lambda row: bucket(
            (row.get("observation_features") or {}).get("combat_turn_count"), 2
        ),
        "alive_monsters": lambda row: str(
            (row.get("observation_features") or {}).get("alive_monster_count")
        ),
        "playable_cards_bucket": lambda row: bucket(
            (row.get("decision_context_features") or {}).get("playable_candidate_count"), 2
        ),
        "playable_damage_bucket": lambda row: bucket(
            (row.get("decision_context_features") or {}).get("playable_unique_damage_sum"), 10
        ),
        "playable_block_bucket": lambda row: bucket(
            (row.get("decision_context_features") or {}).get("playable_unique_block_sum"), 10
        ),
        "end_turn_with_playable_cards": lambda row: str(
            (row.get("decision_context_features") or {}).get("end_turn_with_playable_cards")
        ),
        "incoming_block_gap_bucket": lambda row: bucket(
            (row.get("decision_context_features") or {}).get("incoming_minus_current_block"), 5
        ),
    }


def worst_branch_examples(
    rows: list[dict[str, Any]],
    y_true: list[float],
    y_pred: list[float],
    *,
    top_k: int,
) -> list[dict[str, Any]]:
    heap: list[tuple[float, int]] = []
    for index, (target, pred) in enumerate(zip(y_true, y_pred)):
        error = abs(pred - target)
        if len(heap) < top_k:
            heapq.heappush(heap, (error, index))
        elif error > heap[0][0]:
            heapq.heapreplace(heap, (error, index))
    out = []
    for error, index in sorted(heap, reverse=True):
        row = rows[index]
        candidate = row.get("candidate") or {}
        obs = row.get("observation_features") or {}
        out.append(
            {
                "abs_error": error,
                "target": y_true[index],
                "prediction": y_pred[index],
                "episode_seed": row.get("episode_seed"),
                "episode_step": row.get("episode_step"),
                "branch_id": row.get("branch_id"),
                "action_kind": candidate.get("action_kind"),
                "action_key": candidate.get("action_key"),
                "card_id": candidate.get("card_id"),
                "card_cost": candidate.get("card_cost"),
                "card_base_damage": candidate.get("card_base_damage"),
                "card_base_block": candidate.get("card_base_block"),
                "current_hp": obs.get("current_hp"),
                "visible_incoming_damage": obs.get("visible_incoming_damage"),
                "total_monster_hp": obs.get("total_monster_hp"),
                "alive_monster_count": obs.get("alive_monster_count"),
                "combat_turn_count": obs.get("combat_turn_count"),
                "targets": row.get("targets"),
            }
        )
    return out


class BinaryBucketStats:
    def __init__(self) -> None:
        self.count = 0
        self.positive_count = 0
        self.score_sum = 0.0
        self.brier_sum = 0.0
        self.false_positive_count = 0
        self.false_negative_count = 0

    def add(self, label: int, score: float) -> None:
        pred_label = 1 if score >= 0.5 else 0
        self.count += 1
        self.positive_count += label
        self.score_sum += score
        self.brier_sum += (score - label) ** 2
        if pred_label == 1 and label == 0:
            self.false_positive_count += 1
        if pred_label == 0 and label == 1:
            self.false_negative_count += 1

    def to_dict(self) -> dict[str, float | int]:
        return {
            "count": self.count,
            "positive_count": self.positive_count,
            "positive_rate": self.positive_count / self.count if self.count else 0.0,
            "mean_score": self.score_sum / self.count if self.count else 0.0,
            "brier": self.brier_sum / self.count if self.count else 0.0,
            "false_positive_count": self.false_positive_count,
            "false_negative_count": self.false_negative_count,
        }


def top_binary_bucket_stats(
    rows: list[dict[str, Any]],
    labels: list[int],
    scores: list[float],
    bucket_fn,
    *,
    min_count: int,
    top_k: int,
) -> list[dict[str, Any]]:
    buckets: dict[str, BinaryBucketStats] = {}
    for row, label, score in zip(rows, labels, scores):
        key = bucket_fn(row)
        buckets.setdefault(key, BinaryBucketStats()).add(label, score)
    ranked = []
    for key, stats in buckets.items():
        if stats.count < min_count:
            continue
        item = stats.to_dict()
        item["bucket"] = key
        ranked.append(item)
    ranked.sort(key=lambda item: (-float(item["brier"]), -int(item["count"]), str(item["bucket"])))
    return ranked[:top_k]


def candidate_audit_summary(row: dict[str, Any]) -> dict[str, Any]:
    candidate = row.get("candidate") or {}
    return {
        "action_kind": candidate.get("action_kind"),
        "action_type": candidate.get("action_type"),
        "action_key": candidate.get("action_key"),
        "card_id": candidate.get("card_id"),
        "card_type_id": candidate.get("card_type_id"),
        "card_cost": candidate.get("card_cost"),
        "card_base_damage": candidate.get("card_base_damage"),
        "card_base_block": candidate.get("card_base_block"),
        "card_draws_cards": candidate.get("card_draws_cards"),
        "card_exhaust": candidate.get("card_exhaust"),
        "card_applies_vulnerable": candidate.get("card_applies_vulnerable"),
        "card_applies_weak": candidate.get("card_applies_weak"),
        "card_scaling_piece": candidate.get("card_scaling_piece"),
    }


def observation_audit_summary(row: dict[str, Any]) -> dict[str, Any]:
    obs = row.get("observation_features") or {}
    return {
        "floor": obs.get("floor"),
        "current_hp": obs.get("current_hp"),
        "combat_turn_count": obs.get("combat_turn_count"),
        "combat_energy": obs.get("combat_energy"),
        "visible_incoming_damage": obs.get("visible_incoming_damage"),
        "player_block": obs.get("player_block"),
        "alive_monster_count": obs.get("alive_monster_count"),
        "total_monster_hp": obs.get("total_monster_hp"),
        "hand_count": obs.get("hand_count"),
        "draw_count": obs.get("draw_count"),
        "discard_count": obs.get("discard_count"),
        "exhaust_count": obs.get("exhaust_count"),
        "deck_attack_count": obs.get("deck_attack_count"),
        "deck_skill_count": obs.get("deck_skill_count"),
        "deck_power_count": obs.get("deck_power_count"),
    }


def decision_context_audit_summary(row: dict[str, Any]) -> dict[str, Any]:
    ctx = row.get("decision_context_features") or {}
    return {
        "legal_candidate_count": ctx.get("legal_candidate_count"),
        "playable_candidate_count": ctx.get("playable_candidate_count"),
        "playable_unique_card_count": ctx.get("playable_unique_card_count"),
        "playable_attack_candidate_count": ctx.get("playable_attack_candidate_count"),
        "playable_block_candidate_count": ctx.get("playable_block_candidate_count"),
        "playable_draw_candidate_count": ctx.get("playable_draw_candidate_count"),
        "playable_debuff_candidate_count": ctx.get("playable_debuff_candidate_count"),
        "playable_exhaust_candidate_count": ctx.get("playable_exhaust_candidate_count"),
        "playable_setup_candidate_count": ctx.get("playable_setup_candidate_count"),
        "playable_unique_damage_sum": ctx.get("playable_unique_damage_sum"),
        "playable_unique_block_sum": ctx.get("playable_unique_block_sum"),
        "end_turn_with_playable_cards": ctx.get("end_turn_with_playable_cards"),
        "end_turn_with_unspent_energy": ctx.get("end_turn_with_unspent_energy"),
        "incoming_minus_current_block": ctx.get("incoming_minus_current_block"),
    }


def branch_prediction_records(
    rows: list[dict[str, Any]],
    hp_true: list[float],
    hp_pred: list[float],
    reward_true: list[float],
    reward_pred: list[float],
    risk_predictions: dict[str, dict[str, Any]],
) -> Iterable[dict[str, Any]]:
    for index, row in enumerate(rows):
        risks: dict[str, Any] = {}
        for name, payload in risk_predictions.items():
            labels = payload.get("labels")
            scores = payload.get("scores")
            risks[name] = {
                "label": labels[index] if isinstance(labels, list) and index < len(labels) else None,
                "probability": scores[index] if isinstance(scores, list) and index < len(scores) else None,
                "model_skipped": scores is None,
            }
        yield {
            "schema_version": "branch_value_risk_prediction_v0",
            "trainable_role": "branch_value_risk_audit",
            "trainable_as_action_label": False,
            "episode_seed": row.get("episode_seed"),
            "episode_step": row.get("episode_step"),
            "decision_id": row.get("decision_id"),
            "branch_id": row.get("branch_id"),
            "state_hash_before": row.get("state_hash_before"),
            "scenario_seed_id": row.get("scenario_seed_id"),
            "candidate": candidate_audit_summary(row),
            "observation": observation_audit_summary(row),
            "decision_context": decision_context_audit_summary(row),
            "targets": {
                "hp_delta": hp_true[index],
                "total_reward": reward_true[index],
            },
            "model_outputs": {
                "hp_delta": hp_pred[index],
                "total_reward": reward_pred[index],
                "risks": risks,
            },
            "errors": {
                "hp_delta_abs": abs(hp_pred[index] - hp_true[index]),
                "total_reward_abs": abs(reward_pred[index] - reward_true[index]),
            },
            "label_policy": {
                "action_label": False,
                "source": "branch_value_risk_feature_audit_v1",
            },
        }


def branch_feature_audit(
    rows: list[dict[str, Any]],
    hp_true: list[float],
    hp_pred: list[float],
    reward_true: list[float],
    reward_pred: list[float],
    risk_predictions: dict[str, dict[str, Any]],
    *,
    min_count: int,
    top_k: int,
) -> dict[str, Any]:
    buckets = branch_bucket_functions()
    hp_buckets = {
        name: top_bucket_stats(rows, hp_true, hp_pred, fn, min_count=min_count, top_k=top_k)
        for name, fn in buckets.items()
    }
    reward_buckets = {
        name: top_bucket_stats(rows, reward_true, reward_pred, fn, min_count=min_count, top_k=top_k)
        for name, fn in buckets.items()
    }
    risk_buckets: dict[str, Any] = {}
    for name, payload in risk_predictions.items():
        labels = payload.get("labels")
        scores = payload.get("scores")
        if not isinstance(labels, list) or not isinstance(scores, list):
            risk_buckets[name] = {"skipped": "missing_or_single_class_scores"}
            continue
        risk_buckets[name] = {
            bucket_name: top_binary_bucket_stats(
                rows, labels, scores, fn, min_count=min_count, top_k=top_k
            )
            for bucket_name, fn in buckets.items()
        }
    return {
        "schema_version": "branch_value_risk_feature_audit_v1",
        "min_bucket_count": min_count,
        "top_k": top_k,
        "hp_delta_error_buckets": hp_buckets,
        "total_reward_error_buckets": reward_buckets,
        "risk_error_buckets": risk_buckets,
        "worst_hp_delta_examples": worst_branch_examples(
            rows, hp_true, hp_pred, top_k=min(top_k, 20)
        ),
        "worst_total_reward_examples": worst_branch_examples(
            rows, reward_true, reward_pred, top_k=min(top_k, 20)
        ),
    }


def train_regression(
    rows: list[dict[str, Any]],
    target: str,
    dim: int,
    epochs: int,
    lr: float,
    l2: float,
    seed: int,
) -> dict[str, Any]:
    y_values = [target_value(row, target) for row in rows]
    mean = sum(y_values) / len(y_values)
    variance = sum((value - mean) ** 2 for value in y_values) / max(1, len(y_values))
    std = math.sqrt(variance) or 1.0
    weights = [0.0] * dim
    rng = random.Random(seed)
    examples = [(branch_features(row, dim), (target_value(row, target) - mean) / std) for row in rows]
    for _ in range(epochs):
        rng.shuffle(examples)
        for features, y_norm in examples:
            pred = dot(weights, features)
            error = max(-10.0, min(10.0, pred - y_norm))
            for index, value in features.items():
                weights[index] -= lr * (error * value + l2 * weights[index])
    return {"weights": weights, "mean": mean, "std": std, "target": target}


def train_regression_examples(
    examples: list[tuple[dict[int, float], float]],
    target: str,
    dim: int,
    epochs: int,
    lr: float,
    l2: float,
    seed: int,
) -> dict[str, Any]:
    y_values = [value for _, value in examples]
    mean = sum(y_values) / len(y_values)
    variance = sum((value - mean) ** 2 for value in y_values) / max(1, len(y_values))
    std = math.sqrt(variance) or 1.0
    weights = [0.0] * dim
    rng = random.Random(seed)
    normalized = [(features, (value - mean) / std) for features, value in examples]
    for _ in range(epochs):
        rng.shuffle(normalized)
        for features, y_norm in normalized:
            pred = dot(weights, features)
            error = max(-10.0, min(10.0, pred - y_norm))
            for index, feature_value in features.items():
                weights[index] -= lr * (error * feature_value + l2 * weights[index])
    return {"weights": weights, "mean": mean, "std": std, "target": target}


def predict_regression(model: dict[str, Any], row: dict[str, Any], dim: int) -> float:
    return model["mean"] + model["std"] * dot(model["weights"], branch_features(row, dim))


def predict_regression_features(model: dict[str, Any], features: dict[int, float]) -> float:
    return model["mean"] + model["std"] * dot(model["weights"], features)


def regression_metrics(y_true: list[float], y_pred: list[float]) -> dict[str, float]:
    if not y_true:
        return {"count": 0}
    mean = sum(y_true) / len(y_true)
    mae = sum(abs(a - b) for a, b in zip(y_true, y_pred)) / len(y_true)
    mse = sum((a - b) ** 2 for a, b in zip(y_true, y_pred)) / len(y_true)
    baseline_mae = sum(abs(value - mean) for value in y_true) / len(y_true)
    baseline_mse = sum((value - mean) ** 2 for value in y_true) / len(y_true)
    r2 = 1.0 - (mse / baseline_mse) if baseline_mse > 1e-12 else 0.0
    return {
        "count": len(y_true),
        "mae": mae,
        "rmse": math.sqrt(mse),
        "baseline_mae": baseline_mae,
        "baseline_rmse": math.sqrt(baseline_mse),
        "r2_vs_mean": r2,
    }


def train_binary(
    rows: list[dict[str, Any]],
    label_fn,
    dim: int,
    epochs: int,
    lr: float,
    l2: float,
    seed: int,
) -> dict[str, Any] | None:
    labels = [int(label_fn(row)) for row in rows]
    positives = sum(labels)
    if positives == 0 or positives == len(labels):
        return None
    weights = [0.0] * dim
    bias_prior = math.log(positives / (len(labels) - positives))
    rng = random.Random(seed)
    examples = [(branch_features(row, dim), int(label_fn(row))) for row in rows]
    add_feature_bias_index = hash_feature("bias_prior", dim)
    for _ in range(epochs):
        rng.shuffle(examples)
        for features, label in examples:
            pred = sigmoid(dot(weights, features) + bias_prior)
            error = pred - label
            for index, value in features.items():
                weights[index] -= lr * (error * value + l2 * weights[index])
            weights[add_feature_bias_index] -= lr * error
    return {"weights": weights, "bias_prior": bias_prior}


def train_binary_examples(
    examples: list[tuple[dict[int, float], int]],
    dim: int,
    epochs: int,
    lr: float,
    l2: float,
    seed: int,
) -> dict[str, Any] | None:
    labels = [label for _, label in examples]
    positives = sum(labels)
    if positives == 0 or positives == len(labels):
        return None
    weights = [0.0] * dim
    bias_prior = math.log(positives / (len(labels) - positives))
    rng = random.Random(seed)
    shuffled = list(examples)
    add_feature_bias_index = hash_feature("bias_prior", dim)
    for _ in range(epochs):
        rng.shuffle(shuffled)
        for features, label in shuffled:
            pred = sigmoid(dot(weights, features) + bias_prior)
            error = pred - label
            for index, value in features.items():
                weights[index] -= lr * (error * value + l2 * weights[index])
            weights[add_feature_bias_index] -= lr * error
    return {"weights": weights, "bias_prior": bias_prior}


def predict_binary(model: dict[str, Any], row: dict[str, Any], dim: int) -> float:
    return sigmoid(model["bias_prior"] + dot(model["weights"], branch_features(row, dim)))


def predict_binary_features(model: dict[str, Any], features: dict[int, float]) -> float:
    return sigmoid(model["bias_prior"] + dot(model["weights"], features))


def auc_score(labels: list[int], scores: list[float]) -> float | None:
    positives = sum(labels)
    negatives = len(labels) - positives
    if positives == 0 or negatives == 0:
        return None
    ranked = sorted(zip(scores, labels), key=lambda item: item[0])
    rank_sum = 0.0
    index = 0
    while index < len(ranked):
        end = index + 1
        while end < len(ranked) and ranked[end][0] == ranked[index][0]:
            end += 1
        avg_rank = (index + 1 + end) / 2.0
        for _, label in ranked[index:end]:
            if label == 1:
                rank_sum += avg_rank
        index = end
    return (rank_sum - positives * (positives + 1) / 2.0) / (positives * negatives)


def binary_metrics(labels: list[int], scores: list[float]) -> dict[str, Any]:
    if not labels:
        return {"count": 0}
    positives = sum(labels)
    pred_labels = [1 if score >= 0.5 else 0 for score in scores]
    accuracy = sum(int(a == b) for a, b in zip(labels, pred_labels)) / len(labels)
    brier = sum((score - label) ** 2 for score, label in zip(scores, labels)) / len(labels)
    prior = positives / len(labels)
    baseline_brier = sum((prior - label) ** 2 for label in labels) / len(labels)
    return {
        "count": len(labels),
        "positive_count": positives,
        "positive_rate": prior,
        "accuracy_at_0_5": accuracy,
        "brier": brier,
        "baseline_brier": baseline_brier,
        "auc": auc_score(labels, scores),
    }


def split_rows(rows: list[dict[str, Any]], train_ratio: float) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    train: list[dict[str, Any]] = []
    test: list[dict[str, Any]] = []
    for row in rows:
        if split_for_seed(row.get("episode_seed"), train_ratio) == "train":
            train.append(row)
        else:
            test.append(row)
    return train, test


def decision_key(row: dict[str, Any]) -> str:
    decision = json.dumps(row.get("decision_id"), sort_keys=True, separators=(",", ":"))
    return f"{row.get('episode_seed')}:{row.get('episode_step')}:{row.get('state_hash_before')}:{decision}"


def candidate_action_id(row: dict[str, Any]) -> Any:
    candidate = row.get("candidate") or {}
    action_id = candidate.get("action_id")
    if action_id is not None:
        return action_id
    forced = row.get("forced_prefix") or []
    return forced[0] if forced else None


def group_by_decision(rows: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    groups: dict[str, list[dict[str, Any]]] = {}
    for row in rows:
        groups.setdefault(decision_key(row), []).append(row)
    return groups


def decision_centered_labels(rows: list[dict[str, Any]]) -> dict[str, dict[str, float]]:
    labels: dict[str, dict[str, float]] = {}
    for group_rows in group_by_decision(rows).values():
        if not group_rows:
            continue
        mean_hp = sum(target_value(row, "hp_delta") for row in group_rows) / len(group_rows)
        mean_reward = sum(target_value(row, "total_reward") for row in group_rows) / len(group_rows)
        behavior_hp: float | None = None
        behavior_reward: float | None = None
        behavior_action_id = group_rows[0].get("behavior_action_id")
        for row in group_rows:
            if behavior_action_id is not None and candidate_action_id(row) == behavior_action_id:
                behavior_hp = target_value(row, "hp_delta")
                behavior_reward = target_value(row, "total_reward")
                break
        for row in group_rows:
            branch_id = row.get("branch_id")
            if not isinstance(branch_id, str):
                continue
            hp = target_value(row, "hp_delta")
            reward = target_value(row, "total_reward")
            item = {
                "hp_delta_minus_decision_mean": hp - mean_hp,
                "total_reward_minus_decision_mean": reward - mean_reward,
            }
            if behavior_hp is not None and behavior_reward is not None:
                item["hp_delta_minus_behavior"] = hp - behavior_hp
                item["total_reward_minus_behavior"] = reward - behavior_reward
            labels[branch_id] = item
    return labels


def train_advantage_regression(
    train_rows: list[dict[str, Any]],
    test_rows: list[dict[str, Any]],
    labels: dict[str, dict[str, float]],
    label_name: str,
    dim: int,
    epochs: int,
    lr: float,
    l2: float,
    seed: int,
) -> dict[str, Any]:
    train_examples = [
        (branch_features(row, dim), labels[row["branch_id"]][label_name])
        for row in train_rows
        if isinstance(row.get("branch_id"), str)
        and row["branch_id"] in labels
        and label_name in labels[row["branch_id"]]
    ]
    test_pairs = [
        (row, labels[row["branch_id"]][label_name])
        for row in test_rows
        if isinstance(row.get("branch_id"), str)
        and row["branch_id"] in labels
        and label_name in labels[row["branch_id"]]
    ]
    if not train_examples or not test_pairs:
        return {"skipped": "missing_train_or_test_examples", "label": label_name}
    model = train_regression_examples(train_examples, label_name, dim, epochs, lr, l2, seed)
    y_true = [target for _, target in test_pairs]
    y_pred = [predict_regression_features(model, branch_features(row, dim)) for row, _ in test_pairs]
    return {
        "label": label_name,
        "train_count": len(train_examples),
        "test_count": len(test_pairs),
        "metrics": regression_metrics(y_true, y_pred),
        "error_buckets": {
            name: top_bucket_stats(
                [row for row, _ in test_pairs],
                y_true,
                y_pred,
                fn,
                min_count=10,
                top_k=8,
            )
            for name, fn in branch_bucket_functions().items()
        },
    }


def load_pair_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for row in iter_jsonl(path):
        if row.get("trainable_as_action_label") is not False:
            continue
        if (row.get("label_policy") or {}).get("action_label") is not False:
            continue
        rows.append(row)
    return rows


def pair_branch_row(pair_side: dict[str, Any], pair_row: dict[str, Any]) -> dict[str, Any]:
    return {
        "trainable_as_action_label": False,
        "label_policy": {"action_label": False, "source": "pair_side"},
        "episode_seed": pair_row.get("episode_seed"),
        "decision_type": (pair_row.get("decision_id") or {}).get("decision_type"),
        "candidate": pair_side.get("candidate") or {},
        "observation_features": {},
        "decision_context_features": pair_side.get("decision_context_features") or {},
        "targets": pair_side.get("targets") or {},
    }


def evaluate_pair_hp_diff(
    pair_rows: list[dict[str, Any]],
    branch_by_id: dict[str, dict[str, Any]],
    model: dict[str, Any],
    dim: int,
    train_ratio: float,
) -> dict[str, Any]:
    test_pairs = [
        row for row in pair_rows if split_for_seed(row.get("episode_seed"), train_ratio) != "train"
    ]
    y_true: list[float] = []
    y_pred: list[float] = []
    sign_correct = 0
    nonzero_count = 0
    for row in test_pairs:
        diff = row.get("outcome_diff") or {}
        true_hp = float(diff.get("hp_left_minus_right") or 0.0)
        left_side = row.get("left") or {}
        right_side = row.get("right") or {}
        left_row = branch_by_id.get(left_side.get("branch_id")) or pair_branch_row(left_side, row)
        right_row = branch_by_id.get(right_side.get("branch_id")) or pair_branch_row(right_side, row)
        left_pred = predict_regression(model, left_row, dim)
        right_pred = predict_regression(model, right_row, dim)
        pred_hp = left_pred - right_pred
        y_true.append(true_hp)
        y_pred.append(pred_hp)
        if true_hp != 0:
            nonzero_count += 1
            if (true_hp > 0 and pred_hp > 0) or (true_hp < 0 and pred_hp < 0):
                sign_correct += 1
    metrics = regression_metrics(y_true, y_pred)
    metrics["nonzero_count"] = nonzero_count
    metrics["sign_accuracy_nonzero_hp_diff"] = (
        sign_correct / nonzero_count if nonzero_count else None
    )
    return metrics


def pair_bucket_functions() -> dict[str, Any]:
    def side_candidate(row: dict[str, Any], side: str) -> dict[str, Any]:
        return ((row.get(side) or {}).get("candidate") or {})

    def true_hp_diff(row: dict[str, Any]) -> float:
        diff = row.get("outcome_diff") or row.get("targets") or {}
        return float(diff.get("hp_left_minus_right") or 0.0)

    return {
        "kind_pair": lambda row: (
            f"{side_candidate(row, 'left').get('action_kind')}->"
            f"{side_candidate(row, 'right').get('action_kind')}"
        ),
        "card_pair": lambda row: (
            f"{side_candidate(row, 'left').get('card_id')}->"
            f"{side_candidate(row, 'right').get('card_id')}"
        ),
        "left_kind": lambda row: str(side_candidate(row, "left").get("action_kind")),
        "right_kind": lambda row: str(side_candidate(row, "right").get("action_kind")),
        "left_card": lambda row: str(side_candidate(row, "left").get("card_id")),
        "right_card": lambda row: str(side_candidate(row, "right").get("card_id")),
        "same_action_kind": lambda row: str(
            side_candidate(row, "left").get("action_kind")
            == side_candidate(row, "right").get("action_kind")
        ),
        "true_hp_diff_abs_bucket": lambda row: bucket(abs(true_hp_diff(row)), 5),
    }


def add_pair_tokens(
    features: dict[int, float],
    row: dict[str, Any],
    left: dict[str, Any],
    right: dict[str, Any],
    dim: int,
) -> None:
    left_candidate = left.get("candidate") or {}
    right_candidate = right.get("candidate") or {}
    add_feature(
        features,
        f"pair_kind={left_candidate.get('action_kind')}->{right_candidate.get('action_kind')}",
        dim,
    )
    add_feature(
        features,
        f"pair_card={left_candidate.get('card_id')}->{right_candidate.get('card_id')}",
        dim,
    )
    add_feature(
        features,
        f"same_action_kind={left_candidate.get('action_kind') == right_candidate.get('action_kind')}",
        dim,
    )
    add_feature(
        features,
        f"left_end_turn={left_candidate.get('action_kind') == 'end_turn'}",
        dim,
    )
    add_feature(
        features,
        f"right_end_turn={right_candidate.get('action_kind') == 'end_turn'}",
        dim,
    )
    add_feature(
        features,
        f"any_end_turn={(left_candidate.get('action_kind') == 'end_turn') or (right_candidate.get('action_kind') == 'end_turn')}",
        dim,
    )
    add_feature(features, f"pairing_rng_diverged={(row.get('pairing') or {}).get('rng_diverged')}", dim)


def pair_diff_features(
    row: dict[str, Any],
    left: dict[str, Any],
    right: dict[str, Any],
    dim: int,
    *,
    base_hp_diff: float | None = None,
    base_reward_diff: float | None = None,
) -> dict[int, float]:
    features = subtract_features(branch_features(left, dim), branch_features(right, dim))
    add_pair_tokens(features, row, left, right, dim)
    if base_hp_diff is not None:
        add_numeric(features, "branch_model_base_hp_diff", base_hp_diff, dim, 20.0)
        add_feature(features, f"branch_model_base_hp_diff_bucket={bucket(base_hp_diff, 5)}", dim)
    if base_reward_diff is not None:
        add_numeric(features, "branch_model_base_reward_diff", base_reward_diff, dim, 1.0)
    return features


def pair_model_examples(
    pair_rows: list[dict[str, Any]],
    branch_by_id: dict[str, dict[str, Any]],
    hp_model: dict[str, Any],
    reward_model: dict[str, Any],
    dim: int,
    train_ratio: float,
    *,
    swapped_train: bool,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    train: list[dict[str, Any]] = []
    test: list[dict[str, Any]] = []
    for row in pair_rows:
        left_side = row.get("left") or {}
        right_side = row.get("right") or {}
        left = branch_by_id.get(left_side.get("branch_id"))
        right = branch_by_id.get(right_side.get("branch_id"))
        if left is None or right is None:
            continue
        diff = row.get("outcome_diff") or {}
        hp_diff = float(diff.get("hp_left_minus_right") or 0.0)
        reward_diff = float(diff.get("total_reward_left_minus_right") or 0.0)
        base_hp = predict_regression(hp_model, left, dim) - predict_regression(hp_model, right, dim)
        base_reward = predict_regression(reward_model, left, dim) - predict_regression(
            reward_model, right, dim
        )
        item = {
            "row": row,
            "left": left,
            "right": right,
            "features": pair_diff_features(
                row, left, right, dim, base_hp_diff=base_hp, base_reward_diff=base_reward
            ),
            "hp_diff": hp_diff,
            "reward_diff": reward_diff,
            "base_hp_diff": base_hp,
            "base_reward_diff": base_reward,
        }
        target = train if split_for_seed(row.get("episode_seed"), train_ratio) == "train" else test
        target.append(item)
        if swapped_train and target is train:
            swapped = {
                "row": row,
                "left": right,
                "right": left,
                "features": pair_diff_features(
                    row, right, left, dim, base_hp_diff=-base_hp, base_reward_diff=-base_reward
                ),
                "hp_diff": -hp_diff,
                "reward_diff": -reward_diff,
                "base_hp_diff": -base_hp,
                "base_reward_diff": -base_reward,
            }
            train.append(swapped)
    return train, test


def material_sign_metrics(y_true: list[float], y_pred: list[float]) -> dict[str, Any]:
    out: dict[str, Any] = {}
    for threshold in (5, 10, 15):
        total = 0
        correct = 0
        severe_under = 0
        for true_value, pred_value in zip(y_true, y_pred):
            if abs(true_value) < threshold:
                continue
            total += 1
            if (true_value > 0 and pred_value > 0) or (true_value < 0 and pred_value < 0):
                correct += 1
            if abs(true_value) >= threshold and abs(pred_value) < 5:
                severe_under += 1
        out[f"abs_diff_ge_{threshold}"] = {
            "count": total,
            "sign_accuracy": correct / total if total else None,
            "severe_underestimate_pred_abs_lt_5": severe_under,
            "severe_underestimate_rate": severe_under / total if total else None,
        }
    return out


def fit_pair_residual_models(
    train_examples: list[dict[str, Any]],
    dim: int,
    epochs: int,
    lr: float,
    l2: float,
    seed: int,
) -> dict[str, Any]:
    hp_residual_model = train_regression_examples(
        [(item["features"], item["hp_diff"] - item["base_hp_diff"]) for item in train_examples],
        "hp_pair_residual",
        dim,
        epochs,
        lr,
        l2,
        seed,
    )
    reward_residual_model = train_regression_examples(
        [
            (item["features"], item["reward_diff"] - item["base_reward_diff"])
            for item in train_examples
        ],
        "reward_pair_residual",
        dim,
        epochs,
        lr,
        l2,
        seed + 1,
    )
    return {
        "hp_residual_model": hp_residual_model,
        "reward_residual_model": reward_residual_model,
    }


def evaluate_pair_residual_model(
    models: dict[str, Any],
    test_examples: list[dict[str, Any]],
) -> dict[str, Any]:
    hp_residual_model = models["hp_residual_model"]
    reward_residual_model = models["reward_residual_model"]
    hp_true = [item["hp_diff"] for item in test_examples]
    hp_base = [item["base_hp_diff"] for item in test_examples]
    hp_pred = [
        item["base_hp_diff"] + predict_regression_features(hp_residual_model, item["features"])
        for item in test_examples
    ]
    reward_true = [item["reward_diff"] for item in test_examples]
    reward_base = [item["base_reward_diff"] for item in test_examples]
    reward_pred = [
        item["base_reward_diff"]
        + predict_regression_features(reward_residual_model, item["features"])
        for item in test_examples
    ]
    return {
        "test_pair_count": len(test_examples),
        "hp_diff_base": regression_metrics(hp_true, hp_base),
        "hp_diff_residual_corrected": regression_metrics(hp_true, hp_pred),
        "reward_diff_base": regression_metrics(reward_true, reward_base),
        "reward_diff_residual_corrected": regression_metrics(reward_true, reward_pred),
        "material_sign_metrics_base": material_sign_metrics(hp_true, hp_base),
        "material_sign_metrics_residual_corrected": material_sign_metrics(hp_true, hp_pred),
    }


def pair_tail_specs():
    return {
        "abs_hp_diff_ge_5": lambda item: abs(item["hp_diff"]) >= 5,
        "abs_hp_diff_ge_10": lambda item: abs(item["hp_diff"]) >= 10,
        "abs_hp_diff_ge_15": lambda item: abs(item["hp_diff"]) >= 15,
        "left_worse_ge_5": lambda item: item["hp_diff"] <= -5,
        "left_worse_ge_10": lambda item: item["hp_diff"] <= -10,
        "left_worse_ge_15": lambda item: item["hp_diff"] <= -15,
        "left_better_ge_5": lambda item: item["hp_diff"] >= 5,
        "left_better_ge_10": lambda item: item["hp_diff"] >= 10,
        "left_better_ge_15": lambda item: item["hp_diff"] >= 15,
    }


def fit_pair_tail_classifiers(
    train_examples: list[dict[str, Any]],
    dim: int,
    epochs: int,
    lr: float,
    l2: float,
    seed: int,
) -> dict[str, Any]:
    specs = pair_tail_specs()
    models: dict[str, Any] = {}
    for name, label_fn in specs.items():
        train_binary_examples_payload = [
            (item["features"], int(label_fn(item))) for item in train_examples
        ]
        model = train_binary_examples(
            train_binary_examples_payload,
            dim,
            epochs,
            lr,
            l2,
            seed + stable_hash(name) % 10_000,
        )
        models[name] = model
    return models


def evaluate_pair_tail_classifiers(
    models: dict[str, Any],
    test_examples: list[dict[str, Any]],
) -> dict[str, Any]:
    specs = pair_tail_specs()
    out: dict[str, Any] = {}
    for name, label_fn in specs.items():
        labels = [int(label_fn(item)) for item in test_examples]
        model = models.get(name)
        if model is None:
            out[name] = {
                "count": len(labels),
                "positive_count": sum(labels),
                "positive_rate": sum(labels) / len(labels) if labels else 0.0,
                "skipped": "single_class_train_split",
            }
            continue
        scores = [predict_binary_features(model, item["features"]) for item in test_examples]
        out[name] = binary_metrics(labels, scores)
    return out


def pair_prediction_payloads_from_examples(
    examples: list[dict[str, Any]],
    residual_models: dict[str, Any],
    tail_models: dict[str, Any],
) -> tuple[list[dict[str, Any]], list[float], list[float], list[float], list[float]]:
    out: list[dict[str, Any]] = []
    hp_true: list[float] = []
    hp_pred: list[float] = []
    reward_true: list[float] = []
    reward_pred: list[float] = []
    hp_residual_model = residual_models["hp_residual_model"]
    reward_residual_model = residual_models["reward_residual_model"]
    for item in examples:
        row = item["row"]
        left_row = item["left"]
        right_row = item["right"]
        left_side = row.get("left") or {}
        right_side = row.get("right") or {}
        true_hp = float(item["hp_diff"])
        true_reward = float(item["reward_diff"])
        pred_hp = float(item["base_hp_diff"])
        pred_reward = float(item["base_reward_diff"])
        residual_hp = pred_hp + predict_regression_features(hp_residual_model, item["features"])
        residual_reward = pred_reward + predict_regression_features(
            reward_residual_model, item["features"]
        )
        tail_probabilities = {
            name: (
                predict_binary_features(model, item["features"])
                if model is not None
                else None
            )
            for name, model in tail_models.items()
        }
        hp_true.append(true_hp)
        hp_pred.append(pred_hp)
        reward_true.append(true_reward)
        reward_pred.append(pred_reward)
        out.append(
            {
                "schema_version": "branch_pair_outcome_diff_prediction_v0",
                "trainable_role": "branch_pair_outcome_diff_audit",
                "trainable_as_action_label": False,
                "episode_seed": row.get("episode_seed"),
                "episode_step": row.get("episode_step"),
                "decision_id": row.get("decision_id"),
                "comparison_id": row.get("comparison_id"),
                "pairing": row.get("pairing"),
                "left": {
                    "branch_id": left_side.get("branch_id"),
                    "candidate": candidate_audit_summary(left_row),
                    "decision_context": decision_context_audit_summary(left_row),
                },
                "right": {
                    "branch_id": right_side.get("branch_id"),
                    "candidate": candidate_audit_summary(right_row),
                    "decision_context": decision_context_audit_summary(right_row),
                },
                "observation": observation_audit_summary(left_row),
                "targets": {
                    "hp_left_minus_right": true_hp,
                    "total_reward_left_minus_right": true_reward,
                },
                "model_outputs": {
                    "branch_model_hp_left_minus_right": pred_hp,
                    "branch_model_total_reward_left_minus_right": pred_reward,
                    "residual_corrected_hp_left_minus_right": residual_hp,
                    "residual_corrected_total_reward_left_minus_right": residual_reward,
                    "tail_probabilities": tail_probabilities,
                },
                "errors": {
                    "branch_model_hp_left_minus_right_abs": abs(pred_hp - true_hp),
                    "branch_model_total_reward_left_minus_right_abs": abs(
                        pred_reward - true_reward
                    ),
                    "residual_corrected_hp_left_minus_right_abs": abs(residual_hp - true_hp),
                    "residual_corrected_total_reward_left_minus_right_abs": abs(
                        residual_reward - true_reward
                    ),
                    "branch_model_hp_diff_nonzero_sign_correct": (
                        None
                        if true_hp == 0
                        else ((true_hp > 0 and pred_hp > 0) or (true_hp < 0 and pred_hp < 0))
                    ),
                    "residual_corrected_hp_diff_nonzero_sign_correct": (
                        None
                        if true_hp == 0
                        else (
                            (true_hp > 0 and residual_hp > 0)
                            or (true_hp < 0 and residual_hp < 0)
                        )
                    ),
                    "branch_model_severe_underestimate_abs_ge_10_pred_abs_lt_5": (
                        abs(true_hp) >= 10 and abs(pred_hp) < 5
                    ),
                    "residual_corrected_severe_underestimate_abs_ge_10_pred_abs_lt_5": (
                        abs(true_hp) >= 10 and abs(residual_hp) < 5
                    ),
                },
                "search_allocation_signals": {
                    "branch_model_abs_hp_diff": abs(pred_hp),
                    "residual_corrected_abs_hp_diff": abs(residual_hp),
                    "tail_abs_hp_diff_ge_10_probability": tail_probabilities.get(
                        "abs_hp_diff_ge_10"
                    ),
                    "tail_left_worse_ge_10_probability": tail_probabilities.get(
                        "left_worse_ge_10"
                    ),
                    "tail_left_better_ge_10_probability": tail_probabilities.get(
                        "left_better_ge_10"
                    ),
                },
                "label_policy": {
                    "action_label": False,
                    "source": "branch_outcome_model_v1_1_pair_audit",
                },
            }
        )
    return out, hp_true, hp_pred, reward_true, reward_pred


def pair_feature_audit(
    rows: list[dict[str, Any]],
    hp_true: list[float],
    hp_pred: list[float],
    reward_true: list[float],
    reward_pred: list[float],
    *,
    min_count: int,
    top_k: int,
) -> dict[str, Any]:
    buckets = pair_bucket_functions()
    nonzero_count = 0
    sign_correct = 0
    for true_value, pred_value in zip(hp_true, hp_pred):
        if true_value == 0:
            continue
        nonzero_count += 1
        if (true_value > 0 and pred_value > 0) or (true_value < 0 and pred_value < 0):
            sign_correct += 1
    return {
        "schema_version": "branch_pair_value_risk_feature_audit_v1",
        "min_bucket_count": min_count,
        "top_k": top_k,
        "hp_diff_metrics": regression_metrics(hp_true, hp_pred),
        "reward_diff_metrics": regression_metrics(reward_true, reward_pred),
        "hp_diff_nonzero_count": nonzero_count,
        "hp_diff_sign_accuracy_nonzero": sign_correct / nonzero_count if nonzero_count else None,
        "hp_diff_error_buckets": {
            name: top_bucket_stats(rows, hp_true, hp_pred, fn, min_count=min_count, top_k=top_k)
            for name, fn in buckets.items()
        },
        "reward_diff_error_buckets": {
            name: top_bucket_stats(
                rows, reward_true, reward_pred, fn, min_count=min_count, top_k=top_k
            )
            for name, fn in buckets.items()
        },
    }


def pair_examples(
    pair_rows: list[dict[str, Any]],
    branch_by_id: dict[str, dict[str, Any]],
    dim: int,
    train_ratio: float,
) -> tuple[list[tuple[dict[int, float], float, float]], list[tuple[dict[int, float], float, float]]]:
    train: list[tuple[dict[int, float], float, float]] = []
    test: list[tuple[dict[int, float], float, float]] = []
    for row in pair_rows:
        left_side = row.get("left") or {}
        right_side = row.get("right") or {}
        left = branch_by_id.get(left_side.get("branch_id"))
        right = branch_by_id.get(right_side.get("branch_id"))
        if left is None or right is None:
            continue
        features = subtract_features(branch_features(left, dim), branch_features(right, dim))
        diff = row.get("outcome_diff") or {}
        hp_diff = float(diff.get("hp_left_minus_right") or 0.0)
        reward_diff = float(diff.get("total_reward_left_minus_right") or 0.0)
        target = train if split_for_seed(row.get("episode_seed"), train_ratio) == "train" else test
        target.append((features, hp_diff, reward_diff))
    return train, test


def evaluate_direct_pair_regression(
    train_examples: list[tuple[dict[int, float], float, float]],
    test_examples: list[tuple[dict[int, float], float, float]],
    dim: int,
    epochs: int,
    lr: float,
    l2: float,
    seed: int,
) -> dict[str, Any]:
    hp_model = train_regression_examples(
        [(features, hp) for features, hp, _ in train_examples],
        "hp_left_minus_right",
        dim,
        epochs,
        lr,
        l2,
        seed,
    )
    reward_model = train_regression_examples(
        [(features, reward) for features, _, reward in train_examples],
        "total_reward_left_minus_right",
        dim,
        epochs,
        lr,
        l2,
        seed + 1,
    )
    hp_true = [hp for _, hp, _ in test_examples]
    hp_pred = [predict_regression_features(hp_model, features) for features, _, _ in test_examples]
    reward_true = [reward for _, _, reward in test_examples]
    reward_pred = [
        predict_regression_features(reward_model, features) for features, _, _ in test_examples
    ]
    nonzero = 0
    sign_correct = 0
    for true_value, pred_value in zip(hp_true, hp_pred):
        if true_value == 0:
            continue
        nonzero += 1
        if (true_value > 0 and pred_value > 0) or (true_value < 0 and pred_value < 0):
            sign_correct += 1
    nonzero_examples = [
        (features, true_value, pred_value)
        for (features, true_value), pred_value in zip(
            [(features, hp) for features, hp, _ in test_examples], hp_pred
        )
        if true_value != 0
    ]
    return {
        "train_pair_count": len(train_examples),
        "test_pair_count": len(test_examples),
        "hp_diff_regression": regression_metrics(hp_true, hp_pred),
        "reward_diff_regression": regression_metrics(reward_true, reward_pred),
        "hp_diff_sign_accuracy_nonzero": sign_correct / nonzero if nonzero else None,
        "hp_diff_nonzero_count": nonzero,
        "hp_diff_nonzero_mean_abs_prediction": (
            sum(abs(pred) for _, _, pred in nonzero_examples) / len(nonzero_examples)
            if nonzero_examples
            else None
        ),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--branches", type=Path, required=True)
    parser.add_argument("--pairs", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--prediction-out", type=Path)
    parser.add_argument("--pair-prediction-out", type=Path)
    parser.add_argument("--dim", type=int, default=4096)
    parser.add_argument("--epochs", type=int, default=20)
    parser.add_argument("--lr", type=float, default=0.02)
    parser.add_argument("--l2", type=float, default=1e-5)
    parser.add_argument("--train-ratio", type=float, default=0.8)
    parser.add_argument("--seed", type=int, default=123)
    parser.add_argument("--audit-min-count", type=int, default=10)
    parser.add_argument("--audit-top-k", type=int, default=12)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    rows = safe_rows(list(iter_jsonl(args.branches)))
    branch_by_id = {
        row.get("branch_id"): row for row in rows if isinstance(row.get("branch_id"), str)
    }
    train_rows, test_rows = split_rows(rows, args.train_ratio)
    if not train_rows or not test_rows:
        raise SystemExit("train/test split produced an empty side")

    hp_model = train_regression(
        train_rows, "hp_delta", args.dim, args.epochs, args.lr, args.l2, args.seed
    )
    reward_model = train_regression(
        train_rows,
        "total_reward",
        args.dim,
        args.epochs,
        args.lr,
        args.l2,
        args.seed + 1,
    )

    hp_true = [target_value(row, "hp_delta") for row in test_rows]
    hp_pred = [predict_regression(hp_model, row, args.dim) for row in test_rows]
    reward_true = [target_value(row, "total_reward") for row in test_rows]
    reward_pred = [predict_regression(reward_model, row, args.dim) for row in test_rows]
    centered_labels = decision_centered_labels(rows)
    decision_centered_metrics = {
        label_name: train_advantage_regression(
            train_rows,
            test_rows,
            centered_labels,
            label_name,
            args.dim,
            args.epochs,
            args.lr,
            args.l2,
            args.seed + stable_hash(label_name) % 10_000,
        )
        for label_name in (
            "hp_delta_minus_decision_mean",
            "total_reward_minus_decision_mean",
            "hp_delta_minus_behavior",
            "total_reward_minus_behavior",
        )
    }

    risk_specs = {
        "hp_loss_ge_5": lambda row: target_value(row, "hp_delta") <= -5,
        "hp_loss_ge_10": lambda row: target_value(row, "hp_delta") <= -10,
        "combat_win": lambda row: target_value(row, "combat_win_delta") > 0,
        "death": lambda row: bool((row.get("targets") or {}).get("death")),
    }
    risk_metrics: dict[str, Any] = {}
    risk_predictions: dict[str, dict[str, Any]] = {}
    for name, label_fn in risk_specs.items():
        model = train_binary(
            train_rows,
            label_fn,
            args.dim,
            args.epochs,
            args.lr,
            args.l2,
            args.seed + stable_hash(name) % 10_000,
        )
        labels = [int(label_fn(row)) for row in test_rows]
        if model is None:
            risk_predictions[name] = {"labels": labels, "scores": None}
            risk_metrics[name] = {
                "count": len(labels),
                "positive_count": sum(labels),
                "positive_rate": (sum(labels) / len(labels)) if labels else 0.0,
                "skipped": "single_class_train_split",
            }
            continue
        scores = [predict_binary(model, row, args.dim) for row in test_rows]
        risk_predictions[name] = {"labels": labels, "scores": scores}
        risk_metrics[name] = binary_metrics(labels, scores)

    pair_rows = load_pair_rows(args.pairs)
    pair_hp_metrics = evaluate_pair_hp_diff(
        pair_rows, branch_by_id, hp_model, args.dim, args.train_ratio
    )
    pair_train, pair_test = pair_examples(pair_rows, branch_by_id, args.dim, args.train_ratio)
    direct_pair_metrics = evaluate_direct_pair_regression(
        pair_train,
        pair_test,
        args.dim,
        args.epochs,
        args.lr,
        args.l2,
        args.seed + 2,
    )
    pair_model_train, pair_model_test = pair_model_examples(
        pair_rows,
        branch_by_id,
        hp_model,
        reward_model,
        args.dim,
        args.train_ratio,
        swapped_train=True,
    )
    pair_residual_models = fit_pair_residual_models(
        pair_model_train,
        args.dim,
        args.epochs,
        args.lr,
        args.l2,
        args.seed + 3,
    )
    pair_residual_metrics = evaluate_pair_residual_model(
        pair_residual_models,
        pair_model_test,
    )
    pair_residual_metrics["train_pair_count"] = len(pair_model_train)
    pair_tail_models = fit_pair_tail_classifiers(
        pair_model_train,
        args.dim,
        args.epochs,
        args.lr,
        args.l2,
        args.seed + 4,
    )
    pair_tail_metrics = evaluate_pair_tail_classifiers(
        pair_tail_models,
        pair_model_test,
    )
    branch_audit = branch_feature_audit(
        test_rows,
        hp_true,
        hp_pred,
        reward_true,
        reward_pred,
        risk_predictions,
        min_count=args.audit_min_count,
        top_k=args.audit_top_k,
    )
    (
        pair_prediction_rows,
        pair_hp_true,
        pair_hp_pred,
        pair_reward_true,
        pair_reward_pred,
    ) = pair_prediction_payloads_from_examples(
        pair_model_test,
        pair_residual_models,
        pair_tail_models,
    )
    pair_audit = pair_feature_audit(
        pair_prediction_rows,
        pair_hp_true,
        pair_hp_pred,
        pair_reward_true,
        pair_reward_pred,
        min_count=args.audit_min_count,
        top_k=args.audit_top_k,
    )

    summary = {
        "schema_version": "branch_outcome_model_v1_1_summary",
        "branches": str(args.branches),
        "pairs": str(args.pairs),
        "dim": args.dim,
        "epochs": args.epochs,
        "lr": args.lr,
        "l2": args.l2,
        "train_ratio": args.train_ratio,
        "train_branch_count": len(train_rows),
        "test_branch_count": len(test_rows),
        "pair_count": len(pair_rows),
        "models": {
            "hp_delta_regression": regression_metrics(hp_true, hp_pred),
            "total_reward_regression": regression_metrics(reward_true, reward_pred),
            "risk_classification": risk_metrics,
            "decision_centered_advantage_regression": decision_centered_metrics,
            "pair_hp_diff_from_branch_hp_model": pair_hp_metrics,
            "direct_pair_outcome_diff_regression": direct_pair_metrics,
            "pair_residual_regression": pair_residual_metrics,
            "pair_tail_classification": pair_tail_metrics,
        },
        "feature_audit": {
            "branch": branch_audit,
            "pair_from_branch_models": pair_audit,
        },
        "label_safety": {
            "action_policy_trained": False,
            "winner_or_preference_label_used": False,
            "pair_rows_are_ordered_outcome_diffs": True,
        },
    }
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    if args.prediction_out:
        args.prediction_out.parent.mkdir(parents=True, exist_ok=True)
        with args.prediction_out.open("w", encoding="utf-8") as handle:
            for record in branch_prediction_records(
                test_rows,
                hp_true,
                hp_pred,
                reward_true,
                reward_pred,
                risk_predictions,
            ):
                handle.write(json.dumps(record, separators=(",", ":")) + "\n")
    if args.pair_prediction_out:
        args.pair_prediction_out.parent.mkdir(parents=True, exist_ok=True)
        with args.pair_prediction_out.open("w", encoding="utf-8") as handle:
            for record in pair_prediction_rows:
                handle.write(json.dumps(record, separators=(",", ":")) + "\n")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
