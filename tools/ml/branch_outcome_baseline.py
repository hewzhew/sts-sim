#!/usr/bin/env python3
"""
Small dependency-free branch outcome baseline.

This is an ML pilot for campaign-derived branch outcome records. It does not
train an action policy and does not treat campaign choices as teacher labels.
It only asks whether endpoint branch/state features contain enough signal to
predict a coarse bad-outcome target on held-out records.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import random
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


def stable_hash(text: str) -> int:
    return int(hashlib.sha256(text.encode("utf-8")).hexdigest()[:16], 16)


def load_jsonl(paths: list[Path]) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for path in paths:
        with path.open("r", encoding="utf-8") as handle:
            for line_no, line in enumerate(handle, start=1):
                stripped = line.strip()
                if not stripped:
                    continue
                try:
                    record = json.loads(stripped)
                except json.JSONDecodeError as exc:
                    raise SystemExit(f"{path}:{line_no}: invalid JSONL: {exc}") from exc
                if record.get("schema_name") != "BranchOutcomeRecordV1":
                    raise SystemExit(
                        f"{path}:{line_no}: expected BranchOutcomeRecordV1, got "
                        f"{record.get('schema_name')!r}"
                    )
                record["_ml_source_path"] = str(path)
                records.append(record)
    return records


def endpoint_bad_target(record: dict[str, Any]) -> int:
    outcome = record.get("outcome_class")
    if outcome in {"terminal_defeat", "abandoned", "stuck"}:
        return 1
    if outcome == "terminal_victory":
        return 0
    return -1


def risk_proxy_target(record: dict[str, Any]) -> int:
    endpoint = endpoint_bad_target(record)
    if endpoint >= 0:
        return endpoint
    features = record.get("state_features") or {}
    hp = features.get("hp")
    max_hp = features.get("max_hp")
    if isinstance(hp, int) and isinstance(max_hp, int) and max_hp > 0:
        if hp * 100 <= max_hp * 40:
            return 1
    last = features.get("last_combat") or {}
    hp_loss = last.get("hp_loss")
    if isinstance(hp_loss, int) and hp_loss >= 15:
        return 1
    if record.get("outcome_class") in {"ongoing_active", "ongoing_frozen"}:
        return 0
    return -1


def label_for_target(record: dict[str, Any], target: str) -> int:
    if target == "endpoint_bad":
        return endpoint_bad_target(record)
    if target == "risk_proxy":
        return risk_proxy_target(record)
    raise ValueError(f"unknown target: {target}")


def add_token(features: dict[str, float], token: str, value: float = 1.0) -> None:
    if token:
        features[token] += value


def add_number(features: dict[str, float], name: str, value: Any, scale: float) -> None:
    if isinstance(value, bool):
        value = int(value)
    if isinstance(value, (int, float)):
        features[f"num:{name}"] += float(value) / scale
        bucket = int(math.floor(float(value) / scale * 10.0))
        add_token(features, f"bin:{name}:{bucket}")


def card_token(card: dict[str, Any]) -> str:
    card_id = str(card.get("id") or card.get("name") or "unknown_card")
    upgrades = card.get("upgrades", 0)
    return f"card:{card_id}+{upgrades}"


def extract_features(record: dict[str, Any]) -> dict[str, float]:
    features: dict[str, float] = defaultdict(float)
    run_domain = record.get("run_domain") or {}
    state = record.get("state_features") or {}
    deck = state.get("deck") or {}
    startup = state.get("startup") or {}
    formation = state.get("formation") or {}
    strategic = record.get("strategic_summary") or {}
    last = state.get("last_combat") or {}

    add_token(features, f"branch_group:{record.get('branch_group')}")
    add_token(features, f"frontier:{record.get('frontier_title')}")
    add_token(features, f"stop_reason:{record.get('stop_reason')}")
    add_token(features, f"class:{state.get('player_class') or run_domain.get('player_class')}")
    add_token(features, f"boss:{state.get('boss') or (record.get('report_summary') or {}).get('boss')}")
    add_token(features, f"formation_stage:{formation.get('stage')}")

    add_number(features, "ascension", state.get("ascension_level") or run_domain.get("ascension_level"), 20.0)
    add_number(features, "act", state.get("act"), 4.0)
    add_number(features, "floor", state.get("floor"), 50.0)
    add_number(features, "gold", state.get("gold"), 500.0)
    add_number(features, "rank_key", record.get("rank_key"), 50_000.0)
    add_number(features, "commands_len", len(record.get("commands") or []), 80.0)

    hp = state.get("hp")
    max_hp = state.get("max_hp")
    if isinstance(hp, int) and isinstance(max_hp, int) and max_hp > 0:
        hp_pct = hp / max_hp
        add_number(features, "hp_pct", hp_pct, 1.0)
        add_token(features, f"hp_band:{int(hp_pct * 10)}")
    add_number(features, "max_hp", max_hp, 120.0)

    for key in (
        "deck_count",
        "attacks",
        "skills",
        "powers",
        "curses",
        "starter_strikes",
        "starter_defends",
        "upgraded",
    ):
        add_number(features, f"deck_{key}", deck.get(key), 40.0)
    if isinstance(deck.get("deck_count"), int) and deck["deck_count"] > 0:
        add_number(features, "upgrade_ratio", deck.get("upgraded", 0) / deck["deck_count"], 1.0)

    for card in deck.get("grouped_cards") or []:
        count = card.get("count", 1)
        if not isinstance(count, int):
            count = 1
        token = card_token(card)
        add_token(features, token, min(count, 4))
        if count > 1:
            add_token(features, f"duplicate:{token}", min(count - 1, 3))
        add_token(features, f"card_type:{card.get('card_type')}", min(count, 4))

    for relic in state.get("relics") or []:
        add_token(features, f"relic:{relic}")
    for potion in state.get("potions") or []:
        add_token(features, f"potion:{potion}")
    for need in formation.get("needs") or []:
        add_token(features, f"need:{need}")
    for strength in formation.get("strengths") or []:
        add_token(features, f"strength:{strength}")
    for liability in startup.get("liabilities") or []:
        add_token(features, f"liability:{liability}")
    for pressure in state.get("boss_pressure") or []:
        add_token(features, f"boss_pressure:{pressure}")

    for key, value in startup.items():
        if key == "liabilities":
            continue
        add_number(features, f"startup_{key}", value, 10.0)
    for key, value in strategic.items():
        if key == "present":
            continue
        add_number(features, f"strategic_{key}", value, 1000.0)

    for key in ("hp_loss", "turns", "potions_used", "cards_played"):
        add_number(features, f"last_combat_{key}", last.get(key), 20.0)

    return dict(features)


def hashed_features(features: dict[str, float], dim: int) -> dict[int, float]:
    out: dict[int, float] = defaultdict(float)
    for key, value in features.items():
        index = stable_hash(key) % dim
        sign = -1.0 if stable_hash("sign:" + key) % 2 else 1.0
        out[index] += sign * value
    return dict(out)


def split_key(record: dict[str, Any]) -> str:
    return f"{record.get('seed')}|{record.get('branch_id')}"


def train_test_split(
    examples: list[tuple[dict[str, Any], int, dict[str, float]]],
    test_ratio: float,
    holdout_sources: set[str],
) -> tuple[list[tuple[dict[str, Any], int, dict[str, float]]], list[tuple[dict[str, Any], int, dict[str, float]]]]:
    train = []
    test = []
    for example in examples:
        record = example[0]
        if holdout_sources:
            if str(record.get("_ml_source_path")) in holdout_sources:
                test.append(example)
            else:
                train.append(example)
            continue
        bucket = stable_hash(split_key(record)) % 10_000
        if bucket < int(test_ratio * 10_000):
            test.append(example)
        else:
            train.append(example)
    return train, test


def sigmoid(value: float) -> float:
    if value >= 0:
        z = math.exp(-value)
        return 1.0 / (1.0 + z)
    z = math.exp(value)
    return z / (1.0 + z)


def dot(weights: dict[int, float], features: dict[int, float], bias: float) -> float:
    return bias + sum(weights.get(index, 0.0) * value for index, value in features.items())


def train_logistic(
    train: list[tuple[dict[str, Any], int, dict[str, float]]],
    dim: int,
    epochs: int,
    learning_rate: float,
    l2: float,
    seed: int,
) -> tuple[dict[int, float], float]:
    rng = random.Random(seed)
    weights: dict[int, float] = defaultdict(float)
    bias = 0.0
    hashed_train = [(hashed_features(features, dim), label) for _, label, features in train]
    for _ in range(epochs):
        rng.shuffle(hashed_train)
        for features, label in hashed_train:
            pred = sigmoid(dot(weights, features, bias))
            error = pred - label
            bias -= learning_rate * error
            for index, value in features.items():
                weights[index] -= learning_rate * (error * value + l2 * weights[index])
    return dict(weights), bias


def predict_scores(
    examples: list[tuple[dict[str, Any], int, dict[str, float]]],
    weights: dict[int, float],
    bias: float,
    dim: int,
) -> list[float]:
    return [sigmoid(dot(weights, hashed_features(features, dim), bias)) for _, _, features in examples]


def metrics(labels: list[int], scores: list[float], threshold: float = 0.5) -> dict[str, float]:
    tp = fp = tn = fn = 0
    for label, score in zip(labels, scores):
        pred = 1 if score >= threshold else 0
        if label == 1 and pred == 1:
            tp += 1
        elif label == 0 and pred == 1:
            fp += 1
        elif label == 0 and pred == 0:
            tn += 1
        elif label == 1 and pred == 0:
            fn += 1
    total = max(1, len(labels))
    precision = tp / max(1, tp + fp)
    recall = tp / max(1, tp + fn)
    f1 = 2 * precision * recall / max(1e-9, precision + recall)
    specificity = tn / max(1, tn + fp)
    return {
        "accuracy": (tp + tn) / total,
        "precision": precision,
        "recall": recall,
        "f1": f1,
        "balanced_accuracy": (recall + specificity) / 2.0,
        "auc": auc(labels, scores),
        "tp": float(tp),
        "fp": float(fp),
        "tn": float(tn),
        "fn": float(fn),
    }


def auc(labels: list[int], scores: list[float]) -> float:
    pairs = sorted(zip(scores, labels), key=lambda item: item[0])
    pos = sum(labels)
    neg = len(labels) - pos
    if pos == 0 or neg == 0:
        return float("nan")
    rank_sum = 0.0
    for rank, (_, label) in enumerate(pairs, start=1):
        if label == 1:
            rank_sum += rank
    return (rank_sum - pos * (pos + 1) / 2.0) / (pos * neg)


def majority_scores(train_labels: list[int], test_count: int) -> list[float]:
    rate = sum(train_labels) / max(1, len(train_labels))
    return [rate] * test_count


def simple_rule_scores(examples: list[tuple[dict[str, Any], int, dict[str, float]]]) -> list[float]:
    scores = []
    for _, _, features in examples:
        score = 0.15
        score += max(0.0, 0.45 - features.get("num:hp_pct", 0.45))
        score += 0.15 * features.get("num:last_combat_hp_loss", 0.0)
        score += 0.10 * features.get("num:deck_starter_strikes", 0.0)
        score += 0.10 * features.get("num:deck_starter_defends", 0.0)
        score += 0.10 * features.get("num:startup_setup_debt", 0.0)
        scores.append(max(0.0, min(1.0, score)))
    return scores


def top_weight_features(
    weights: dict[int, float],
    train: list[tuple[dict[str, Any], int, dict[str, float]]],
    dim: int,
    limit: int,
) -> tuple[list[tuple[str, float]], list[tuple[str, float]]]:
    feature_index_names: dict[int, Counter[str]] = defaultdict(Counter)
    for _, _, features in train:
        for key in features:
            index = stable_hash(key) % dim
            feature_index_names[index][key] += 1

    named = []
    for index, weight in weights.items():
        if not weight:
            continue
        name = feature_index_names[index].most_common(1)
        if not name:
            continue
        named.append((name[0][0], weight))
    named.sort(key=lambda item: item[1])
    return named[-limit:][::-1], named[:limit]


def format_metric(value: float) -> str:
    if math.isnan(value):
        return "nan"
    return f"{value:.3f}"


def print_metrics(name: str, metric_values: dict[str, float]) -> None:
    ordered = [
        "accuracy",
        "balanced_accuracy",
        "precision",
        "recall",
        "f1",
        "auc",
    ]
    summary = " ".join(f"{key}={format_metric(metric_values[key])}" for key in ordered)
    counts = " ".join(f"{key}={int(metric_values[key])}" for key in ("tp", "fp", "tn", "fn"))
    print(f"{name}: {summary} | {counts}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="+", type=Path, help="BranchOutcomeRecordV1 JSONL inputs")
    parser.add_argument("--target", choices=["risk_proxy", "endpoint_bad"], default="risk_proxy")
    parser.add_argument("--dim", type=int, default=4096, help="Feature hashing dimension")
    parser.add_argument("--epochs", type=int, default=12)
    parser.add_argument("--learning-rate", type=float, default=0.04)
    parser.add_argument("--l2", type=float, default=1e-5)
    parser.add_argument("--test-ratio", type=float, default=0.25)
    parser.add_argument(
        "--holdout-source",
        action="append",
        type=Path,
        default=[],
        help="Use records from this input path as test data. May be repeated.",
    )
    parser.add_argument("--seed", type=int, default=17)
    parser.add_argument("--top", type=int, default=12, help="Top feature weights to print")
    args = parser.parse_args()

    records = load_jsonl(args.inputs)
    examples = []
    skipped = 0
    for record in records:
        label = label_for_target(record, args.target)
        if label < 0:
            skipped += 1
            continue
        examples.append((record, label, extract_features(record)))

    if len(examples) < 8:
        raise SystemExit(
            f"not enough labeled examples for target={args.target}: usable={len(examples)} skipped={skipped}"
        )

    holdout_sources = {str(path) for path in args.holdout_source}
    train, test = train_test_split(examples, args.test_ratio, holdout_sources)
    if not train or not test:
        raise SystemExit(f"split produced train={len(train)} test={len(test)}; provide more records")

    train_labels = [label for _, label, _ in train]
    test_labels = [label for _, label, _ in test]
    label_counts = Counter(label for _, label, _ in examples)
    train_counts = Counter(train_labels)
    test_counts = Counter(test_labels)
    source_counts: dict[str, Counter[int]] = defaultdict(Counter)
    for record, label, _ in examples:
        source_counts[str(record.get("_ml_source_path"))][label] += 1
    print(
        f"BranchOutcomeBaseline target={args.target} records={len(records)} usable={len(examples)} "
        f"skipped={skipped} train={len(train)} test={len(test)} positives={label_counts[1]} negatives={label_counts[0]}"
    )
    print(
        f"label_counts: train_pos={train_counts[1]} train_neg={train_counts[0]} "
        f"test_pos={test_counts[1]} test_neg={test_counts[0]}"
    )
    if len(source_counts) > 1:
        print("source_label_counts:")
        for source, counts in sorted(source_counts.items()):
            print(f"  {source}: pos={counts[1]} neg={counts[0]}")
    if len(set(train_labels)) < 2 or len(set(test_labels)) < 2:
        print("readiness=not_trainable reason=train_or_test_split_lacks_both_classes")
        print("warning: metrics below are smoke-test output only")
    else:
        print("readiness=ok reason=train_and_test_have_both_classes")

    model_weights, bias = train_logistic(
        train,
        dim=args.dim,
        epochs=args.epochs,
        learning_rate=args.learning_rate,
        l2=args.l2,
        seed=args.seed,
    )

    majority = majority_scores(train_labels, len(test))
    rule = simple_rule_scores(test)
    model = predict_scores(test, model_weights, bias, args.dim)

    print_metrics("majority_baseline", metrics(test_labels, majority))
    print_metrics("simple_rule", metrics(test_labels, rule))
    print_metrics("hashed_logistic", metrics(test_labels, model))

    top_positive, top_negative = top_weight_features(model_weights, train, args.dim, args.top)
    print("top_positive_features:")
    for name, weight in top_positive:
        print(f"  {weight:+.3f} {name}")
    print("top_negative_features:")
    for name, weight in top_negative:
        print(f"  {weight:+.3f} {name}")


if __name__ == "__main__":
    main()
