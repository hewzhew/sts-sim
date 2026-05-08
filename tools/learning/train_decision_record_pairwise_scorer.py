#!/usr/bin/env python3
"""Train a lightweight pairwise candidate scorer from DecisionRecord labels.

This is a dependency-free baseline for the candidate-scorer contract. It trains
on pairwise preferences stored in `teacher_label`, not on legacy frontier fields.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import random
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable


def iter_jsonl(paths: list[Path]) -> Iterable[dict[str, Any]]:
    for path in paths:
        with path.open("r", encoding="utf-8") as handle:
            for line in handle:
                line = line.strip()
                if line:
                    yield json.loads(line)


def action_id_value(value: Any) -> int | None:
    if isinstance(value, int):
        return value
    if isinstance(value, dict) and "0" in value and isinstance(value["0"], int):
        return value["0"]
    if isinstance(value, list) and len(value) == 1 and isinstance(value[0], int):
        return value[0]
    return None


def stable_hash(text: str) -> int:
    return int.from_bytes(hashlib.blake2b(text.encode("utf-8"), digest_size=8).digest(), "little")


def split_for_group(group_key: str, train_ratio: float) -> str:
    bucket = stable_hash(group_key) % 10_000
    return "train" if bucket < int(train_ratio * 10_000) else "test"


def hash_feature(token: str, dim: int) -> int:
    return stable_hash(token) % dim


def add_token(features: dict[int, float], token: str, dim: int, value: float = 1.0) -> None:
    features[hash_feature(token, dim)] = features.get(hash_feature(token, dim), 0.0) + value


def bucket(value: Any, width: int) -> str:
    try:
        number = int(value)
    except (TypeError, ValueError):
        return "unknown"
    return str((number // width) * width)


def candidate_by_id(record: dict[str, Any]) -> dict[int, dict[str, Any]]:
    out: dict[int, dict[str, Any]] = {}
    for candidate in record.get("candidates") or []:
        action_id = action_id_value(candidate.get("id"))
        if action_id is not None:
            out[action_id] = candidate
    return out


def label_returns(record: dict[str, Any]) -> dict[int, float]:
    label = record.get("teacher_label")
    if not isinstance(label, dict):
        return {}
    out: dict[int, float] = {}
    for item in label.get("labels") or []:
        action_id = action_id_value(item.get("action_id"))
        try:
            value = float(item.get("mean_return"))
        except (TypeError, ValueError):
            continue
        if action_id is not None and math.isfinite(value):
            out[action_id] = value
    return out


def is_trainable_record(record: dict[str, Any], allow_ineligible: bool) -> bool:
    if allow_ineligible:
        return isinstance(record.get("teacher_label"), dict)
    payload = ((record.get("teacher_label") or {}).get("payload") or {})
    gate = payload.get("training_eligibility")
    return isinstance(gate, dict) and bool(gate.get("eligible_for_training"))


def features_for(record: dict[str, Any], candidate: dict[str, Any], dim: int) -> dict[int, float]:
    features: dict[int, float] = {}
    decision = record.get("decision_id") or {}
    obs = ((record.get("observation") or {}).get("payload") or {})
    cand_payload = candidate.get("payload") or {}
    card = cand_payload.get("card") or {}
    combat = obs.get("combat") or {}
    deck = obs.get("deck") or {}
    screen = obs.get("screen") or {}

    add_token(features, f"decision_type={decision.get('decision_type')}", dim)
    add_token(features, f"engine_state={obs.get('engine_state')}", dim)
    add_token(features, f"act={obs.get('act')}", dim)
    add_token(features, f"floor_bucket={bucket(obs.get('floor'), 5)}", dim)
    add_token(features, f"hp_bucket={bucket(obs.get('current_hp'), 10)}", dim)
    add_token(features, f"incoming_bucket={bucket(combat.get('visible_incoming_damage'), 5)}", dim)
    add_token(features, f"energy={combat.get('energy')}", dim)
    add_token(features, f"alive_monsters={combat.get('alive_monster_count')}", dim)
    add_token(features, f"reward_phase={screen.get('reward_phase')}", dim)
    for key in (
        "attack_count",
        "skill_count",
        "power_count",
        "damage_card_count",
        "block_card_count",
        "draw_card_count",
        "scaling_card_count",
        "exhaust_card_count",
    ):
        add_token(features, f"deck:{key}:bucket={bucket(deck.get(key), 2)}", dim)

    add_token(features, f"action_kind={candidate.get('action_kind')}", dim)
    add_token(features, f"action_key={candidate.get('action_key')}", dim)
    action = cand_payload.get("action") or {}
    if isinstance(action, dict):
        add_token(features, f"action_type={action.get('type')}", dim)
        if action.get("target") is not None:
            add_token(features, "has_target", dim)
    if card:
        add_token(features, f"card_id={card.get('card_id')}", dim)
        add_token(features, f"card_type={card.get('card_type_id')}", dim)
        add_token(features, f"card_cost={card.get('cost')}", dim)
        for key in (
            "exhaust",
            "ethereal",
            "aoe",
            "multi_damage",
            "starter_basic",
            "draws_cards",
            "gains_energy",
            "applies_weak",
            "applies_vulnerable",
            "scaling_piece",
        ):
            if card.get(key):
                add_token(features, f"card:{key}", dim)
        for key, scale in (("base_damage", 20.0), ("base_block", 20.0), ("base_magic", 10.0)):
            try:
                value = float(card.get(key) or 0.0)
            except (TypeError, ValueError):
                value = 0.0
            if value:
                add_token(features, f"num:{key}", dim, value / scale)
    return features


def subtract_features(left: dict[int, float], right: dict[int, float]) -> dict[int, float]:
    out = dict(left)
    for index, value in right.items():
        out[index] = out.get(index, 0.0) - value
        if abs(out[index]) < 1e-12:
            del out[index]
    return out


def dot(weights: list[float], features: dict[int, float]) -> float:
    return sum(weights[index] * value for index, value in features.items())


def sigmoid(value: float) -> float:
    if value >= 0:
        z = math.exp(-value)
        return 1.0 / (1.0 + z)
    z = math.exp(value)
    return z / (1.0 + z)


def load_groups(paths: list[Path], dim: int, allow_ineligible: bool) -> list[dict[str, Any]]:
    groups: list[dict[str, Any]] = []
    for record in iter_jsonl(paths):
        if not is_trainable_record(record, allow_ineligible):
            continue
        label = record.get("teacher_label") or {}
        candidates = candidate_by_id(record)
        features = {
            action_id: features_for(record, candidate, dim)
            for action_id, candidate in candidates.items()
        }
        returns = label_returns(record)
        pairs: list[tuple[int, int, float]] = []
        for pair in label.get("pairwise_preferences") or []:
            preferred = action_id_value(pair.get("preferred"))
            other = action_id_value(pair.get("other"))
            margin = pair.get("margin")
            try:
                margin_value = float(margin) if margin is not None else 1.0
            except (TypeError, ValueError):
                margin_value = 1.0
            if preferred in features and other in features:
                pairs.append((preferred, other, max(0.0, margin_value)))
        if pairs:
            decision = record.get("decision_id") or {}
            groups.append(
                {
                    "group_key": json.dumps(decision, sort_keys=True),
                    "features": features,
                    "pairs": pairs,
                    "returns": returns,
                }
            )
    return groups


def train(groups: list[dict[str, Any]], dim: int, epochs: int, lr: float, l2: float, seed: int) -> list[float]:
    rng = random.Random(seed)
    weights = [0.0] * dim
    examples: list[tuple[dict[int, float], float]] = []
    for group in groups:
        features = group["features"]
        for preferred, other, margin in group["pairs"]:
            examples.append(
                (
                    subtract_features(features[preferred], features[other]),
                    max(1.0, abs(margin)),
                )
            )
    for _ in range(epochs):
        rng.shuffle(examples)
        for diff, weight in examples:
            score = dot(weights, diff)
            grad_scale = (1.0 - sigmoid(score)) * weight
            for index, value in diff.items():
                weights[index] += lr * (grad_scale * value - l2 * weights[index])
    return weights


def evaluate(groups: list[dict[str, Any]], weights: list[float]) -> dict[str, Any]:
    pair_total = 0
    pair_correct = 0
    top1_regret_sum = 0.0
    top1_regret_count = 0
    exact_best_count = 0
    for group in groups:
        features = group["features"]
        scores = {action_id: dot(weights, feat) for action_id, feat in features.items()}
        for preferred, other, _margin in group["pairs"]:
            pair_total += 1
            pair_correct += int(scores.get(preferred, 0.0) > scores.get(other, 0.0))
        returns = group["returns"]
        if returns and scores:
            chosen = max(scores.items(), key=lambda item: item[1])[0]
            if chosen in returns:
                best_return = max(returns.values())
                regret = max(0.0, best_return - returns[chosen])
                top1_regret_sum += regret
                top1_regret_count += 1
                exact_best_count += int(regret <= 1e-6)
    return {
        "group_count": len(groups),
        "pairwise_total": pair_total,
        "pairwise_accuracy": pair_correct / pair_total if pair_total else None,
        "top1_regret_mean": top1_regret_sum / top1_regret_count if top1_regret_count else None,
        "top1_exact_best_rate": exact_best_count / top1_regret_count if top1_regret_count else None,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inputs", nargs="+", type=Path, required=True)
    parser.add_argument("--model-out", type=Path, required=True)
    parser.add_argument("--report-out", type=Path)
    parser.add_argument("--dim", type=int, default=4096)
    parser.add_argument("--epochs", type=int, default=8)
    parser.add_argument("--lr", type=float, default=0.05)
    parser.add_argument("--l2", type=float, default=1e-5)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--train-ratio", type=float, default=0.8)
    parser.add_argument("--allow-ineligible-labels", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    groups = load_groups(args.inputs, args.dim, args.allow_ineligible_labels)
    train_groups = [g for g in groups if split_for_group(g["group_key"], args.train_ratio) == "train"]
    test_groups = [g for g in groups if split_for_group(g["group_key"], args.train_ratio) == "test"]
    if not train_groups:
        raise SystemExit("no train groups with pairwise teacher labels")
    weights = train(train_groups, args.dim, args.epochs, args.lr, args.l2, args.seed)
    nonzero = {str(i): w for i, w in enumerate(weights) if abs(w) > 1e-12}
    model = {
        "schema_version": "decision_record_pairwise_scorer_v0",
        "feature_dim": args.dim,
        "weights": nonzero,
        "config": {
            "epochs": args.epochs,
            "lr": args.lr,
            "l2": args.l2,
            "seed": args.seed,
            "allow_ineligible_labels": args.allow_ineligible_labels,
        },
    }
    report = {
        "schema_version": "decision_record_pairwise_scorer_report_v0",
        "inputs": [str(path) for path in args.inputs],
        "total_groups": len(groups),
        "train": evaluate(train_groups, weights),
        "test": evaluate(test_groups, weights),
    }
    args.model_out.parent.mkdir(parents=True, exist_ok=True)
    args.model_out.write_text(json.dumps(model, indent=2), encoding="utf-8")
    report_out = args.report_out or args.model_out.with_suffix(".report.json")
    report_out.parent.mkdir(parents=True, exist_ok=True)
    report_out.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
