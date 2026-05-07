#!/usr/bin/env python3
"""Train a dependency-free pairwise linear return-Q ranker."""
from __future__ import annotations

import argparse
import json
import math
import random
from collections import defaultdict
from pathlib import Path
from typing import Any

from return_q_common import dot, read_jsonl, row_features, stable_group_split, write_json

FEATURE_SETS = [
    "action_only",
    "candidate_only",
    "state_only",
    "full_state_plus_candidate",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--model-out", type=Path, required=True)
    parser.add_argument("--report-out", type=Path)
    parser.add_argument("--epochs", type=int, default=24)
    parser.add_argument("--learning-rate", type=float, default=0.05)
    parser.add_argument("--l2", type=float, default=0.0001)
    parser.add_argument("--seed", type=int, default=17)
    parser.add_argument("--feature-set", default="full_state_plus_candidate", choices=FEATURE_SETS)
    parser.add_argument("--pair-margin", type=float, default=0.01)
    parser.add_argument("--max-pairs-per-group", type=int, default=64)
    parser.add_argument("--delta-weight-cap", type=float, default=1.0)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    rows = read_jsonl(args.input)
    if not rows:
        raise SystemExit(f"no rows found in {args.input}")
    for row in rows:
        row.setdefault("split", stable_group_split(str(row.get("group_key") or "")))
    splits = {split: [row for row in rows if row.get("split") == split] for split in ["train", "valid", "test"]}
    if not splits["train"]:
        raise SystemExit("training split is empty")

    models = {
        feature_set: train_model(
            splits["train"],
            feature_set,
            args.epochs,
            args.learning_rate,
            args.l2,
            args.seed,
            args.pair_margin,
            args.max_pairs_per_group,
            args.delta_weight_cap,
        )
        for feature_set in FEATURE_SETS
    }
    metrics = {
        feature_set: evaluate_all_splits(model, splits)
        for feature_set, model in models.items()
    }
    chosen = models[args.feature_set]
    model_payload = {
        "schema_version": "return_q_pairwise_linear_model_v0",
        "model_type": "linear",
        "feature_set": args.feature_set,
        "target_mode": "pairwise_return",
        "target_mean": 0.0,
        "target_std": 1.0,
        "bias": 0.0,
        "weights": sorted(chosen["weights"].items()),
        "config": {
            "epochs": args.epochs,
            "learning_rate": args.learning_rate,
            "l2": args.l2,
            "seed": args.seed,
            "pair_margin": args.pair_margin,
            "max_pairs_per_group": args.max_pairs_per_group,
            "delta_weight_cap": args.delta_weight_cap,
            "target_mode": "pairwise_return",
        },
    }
    write_json(args.model_out, model_payload)

    report = {
        "schema_version": "return_q_pairwise_linear_train_report_v0",
        "input": str(args.input),
        "model_out": str(args.model_out),
        "row_count": len(rows),
        "split_counts": {split: len(values) for split, values in splits.items()},
        "metrics_by_feature_set": metrics,
        "gate": gate(metrics),
    }
    report_out = args.report_out or args.model_out.with_suffix(".report.json")
    write_json(report_out, report)
    print(json.dumps(report, indent=2, sort_keys=True))


def train_model(
    rows: list[dict[str, Any]],
    feature_set: str,
    epochs: int,
    learning_rate: float,
    l2: float,
    seed: int,
    pair_margin: float,
    max_pairs_per_group: int,
    delta_weight_cap: float,
) -> dict[str, Any]:
    rng = random.Random(seed)
    weights: defaultdict[int, float] = defaultdict(float)
    pairs = build_pairs(
        rows,
        feature_set,
        rng,
        pair_margin=pair_margin,
        max_pairs_per_group=max_pairs_per_group,
        delta_weight_cap=delta_weight_cap,
    )
    for _ in range(max(epochs, 0)):
        rng.shuffle(pairs)
        for diff, pair_weight in pairs:
            score = dot(weights, diff)
            score = max(min(score, 50.0), -50.0)
            err = (sigmoid(score) - 1.0) * pair_weight
            norm = max(math.sqrt(sum(value * value for value in diff.values())), 1.0)
            step = learning_rate / norm
            for idx, value in diff.items():
                weights[idx] -= step * (err * value + l2 * weights[idx])
    compact = {idx: value for idx, value in weights.items() if abs(value) > 1e-12}
    return {
        "feature_set": feature_set,
        "weights": compact,
        "pair_count": len(pairs),
    }


def build_pairs(
    rows: list[dict[str, Any]],
    feature_set: str,
    rng: random.Random,
    *,
    pair_margin: float,
    max_pairs_per_group: int,
    delta_weight_cap: float,
) -> list[tuple[dict[int, float], float]]:
    by_group = group_rows(rows)
    pairs: list[tuple[dict[int, float], float]] = []
    for group_rows_ in by_group.values():
        prepared = [
            (row_features(row, feature_set), target(row))
            for row in group_rows_
        ]
        group_pairs = []
        for left_idx in range(len(prepared)):
            left_features, left_target = prepared[left_idx]
            for right_idx in range(left_idx + 1, len(prepared)):
                right_features, right_target = prepared[right_idx]
                delta = left_target - right_target
                if abs(delta) < pair_margin:
                    continue
                if delta > 0:
                    diff = sparse_diff(left_features, right_features)
                else:
                    diff = sparse_diff(right_features, left_features)
                if not diff:
                    continue
                cap = max(delta_weight_cap, 1e-9)
                pair_weight = min(abs(delta), cap) / cap
                group_pairs.append((diff, max(pair_weight, 1e-3)))
        if max_pairs_per_group > 0 and len(group_pairs) > max_pairs_per_group:
            group_pairs = rng.sample(group_pairs, max_pairs_per_group)
        pairs.extend(group_pairs)
    return pairs


def sparse_diff(
    better: dict[int, float],
    worse: dict[int, float],
) -> dict[int, float]:
    out: defaultdict[int, float] = defaultdict(float)
    for idx, value in better.items():
        out[idx] += value
    for idx, value in worse.items():
        out[idx] -= value
    return {idx: value for idx, value in out.items() if value}


def target(row: dict[str, Any]) -> float:
    return float(row.get("discounted_return") or 0.0)


def group_rows(rows: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    by_group: defaultdict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        by_group[str(row.get("group_key") or "")].append(row)
    return by_group


def predict(model: dict[str, Any], row: dict[str, Any]) -> float:
    return dot(model["weights"], row_features(row, str(model["feature_set"])))


def evaluate_all_splits(model: dict[str, Any], splits: dict[str, list[dict[str, Any]]]) -> dict[str, Any]:
    return {split: evaluate_rows(model, rows) for split, rows in splits.items()}


def evaluate_rows(model: dict[str, Any], rows: list[dict[str, Any]]) -> dict[str, Any]:
    if not rows:
        return {"count": 0, "top1_regret": None, "pairwise_accuracy": None, "group_count": 0}
    by_group: defaultdict[str, list[tuple[float, float]]] = defaultdict(list)
    for row in rows:
        by_group[str(row.get("group_key") or "")].append((predict(model, row), target(row)))
    return {
        "count": len(rows),
        "top1_regret": mean_top1_regret(by_group),
        "pairwise_accuracy": pairwise_accuracy(by_group),
        "group_count": len(by_group),
    }


def mean_top1_regret(by_group: dict[str, list[tuple[float, float]]]) -> float | None:
    regrets = []
    for values in by_group.values():
        if len(values) < 2:
            continue
        true_best = max(y for _, y in values)
        predicted_choice = max(values, key=lambda item: item[0])
        regrets.append(true_best - predicted_choice[1])
    return sum(regrets) / len(regrets) if regrets else None


def pairwise_accuracy(by_group: dict[str, list[tuple[float, float]]]) -> float | None:
    correct = 0
    total = 0
    for values in by_group.values():
        for left_idx in range(len(values)):
            for right_idx in range(left_idx + 1, len(values)):
                pred_left, y_left = values[left_idx]
                pred_right, y_right = values[right_idx]
                if abs(y_left - y_right) < 1e-9:
                    continue
                total += 1
                if (pred_left > pred_right) == (y_left > y_right):
                    correct += 1
    return correct / total if total else None


def gate(metrics: dict[str, Any]) -> dict[str, Any]:
    test = {name: values.get("test") or {} for name, values in metrics.items()}
    full = test.get("full_state_plus_candidate") or {}
    failures = []
    for baseline in ["action_only", "candidate_only", "state_only"]:
        base = test.get(baseline) or {}
        if not beats_pairwise(full, base, margin=0.02):
            failures.append(f"full_state_plus_candidate pairwise_accuracy does not beat {baseline} by >= 0.02")
        if not beats_top1_regret(full, base):
            failures.append(f"full_state_plus_candidate top1_regret does not beat {baseline}")
    return {
        "offline_return_q_pairwise_gate_passed": not failures,
        "failures": failures,
    }


def beats_pairwise(full: dict[str, Any], baseline: dict[str, Any], *, margin: float) -> bool:
    if not finite_metric(full.get("pairwise_accuracy")) or not finite_metric(baseline.get("pairwise_accuracy")):
        return False
    return float(full["pairwise_accuracy"]) >= float(baseline["pairwise_accuracy"]) + margin


def beats_top1_regret(full: dict[str, Any], baseline: dict[str, Any]) -> bool:
    if not finite_metric(full.get("top1_regret")) or not finite_metric(baseline.get("top1_regret")):
        return False
    return float(full["top1_regret"]) <= float(baseline["top1_regret"])


def finite_metric(value: Any) -> bool:
    return isinstance(value, (int, float)) and math.isfinite(float(value))


def sigmoid(value: float) -> float:
    if value >= 0:
        z = math.exp(-value)
        return 1.0 / (1.0 + z)
    z = math.exp(value)
    return z / (1.0 + z)


if __name__ == "__main__":
    main()
