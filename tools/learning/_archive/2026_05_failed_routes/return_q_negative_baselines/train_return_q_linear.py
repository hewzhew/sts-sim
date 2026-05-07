#!/usr/bin/env python3
"""Train dependency-free hashed linear Q regressors from return-Q JSONL rows."""
from __future__ import annotations

import argparse
import json
import math
import random
from collections import Counter, defaultdict
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
    parser.add_argument("--epochs", type=int, default=12)
    parser.add_argument("--learning-rate", type=float, default=0.02)
    parser.add_argument("--l2", type=float, default=0.0001)
    parser.add_argument("--seed", type=int, default=13)
    parser.add_argument("--feature-set", default="full_state_plus_candidate", choices=FEATURE_SETS)
    parser.add_argument(
        "--target-mode",
        default="group_centered_return",
        choices=["return", "group_centered_return"],
        help="Regression target. group_centered_return removes per-state mean return so state-only features cannot win by predicting state progress.",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    rows = read_jsonl(args.input)
    if not rows:
        raise SystemExit(f"no rows found in {args.input}")
    for row in rows:
        row.setdefault("split", stable_group_split(str(row.get("group_key") or "")))
    apply_target_mode(rows, args.target_mode)
    splits = {split: [row for row in rows if row.get("split") == split] for split in ["train", "valid", "test"]}
    if not splits["train"]:
        raise SystemExit("training split is empty")

    target_mean = sum(target(row) for row in splits["train"]) / len(splits["train"])
    variance = sum((target(row) - target_mean) ** 2 for row in splits["train"]) / max(len(splits["train"]), 1)
    target_std = math.sqrt(variance) or 1.0

    models: dict[str, dict[str, Any]] = {}
    for feature_set in FEATURE_SETS:
        models[feature_set] = train_model(
            splits["train"],
            feature_set,
            target_mean,
            target_std,
            args.epochs,
            args.learning_rate,
            args.l2,
            args.seed,
        )

    metrics = {
        feature_set: evaluate_all_splits(model, splits)
        for feature_set, model in models.items()
    }
    chosen = models[args.feature_set]
    model_payload = {
        "schema_version": "return_q_linear_model_v0",
        "feature_set": args.feature_set,
        "target_mode": args.target_mode,
        "target_mean": target_mean,
        "target_std": target_std,
        "bias": chosen["bias"],
        "weights": sorted(chosen["weights"].items()),
        "config": {
            "epochs": args.epochs,
            "learning_rate": args.learning_rate,
            "l2": args.l2,
            "seed": args.seed,
            "target_mode": args.target_mode,
        },
    }
    write_json(args.model_out, model_payload)

    report = {
        "schema_version": "return_q_linear_train_report_v0",
        "input": str(args.input),
        "model_out": str(args.model_out),
        "row_count": len(rows),
        "split_counts": {split: len(values) for split, values in splits.items()},
        "target_mean": target_mean,
        "target_std": target_std,
        "target_mode": args.target_mode,
        "metrics_by_feature_set": metrics,
        "gate": gate(metrics),
    }
    report_out = args.report_out or args.model_out.with_suffix(".report.json")
    write_json(report_out, report)
    print(json.dumps(report, indent=2, sort_keys=True))


def target(row: dict[str, Any]) -> float:
    if "_training_target" in row:
        return float(row["_training_target"])
    return float(row.get("discounted_return") or 0.0)


def apply_target_mode(rows: list[dict[str, Any]], target_mode: str) -> None:
    if target_mode == "return":
        for row in rows:
            row["_training_target"] = float(row.get("discounted_return") or 0.0)
        return
    if target_mode != "group_centered_return":
        raise ValueError(f"unknown target mode {target_mode}")
    by_group: defaultdict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        by_group[str(row.get("group_key") or "")].append(row)
    for group_rows in by_group.values():
        mean_return = sum(float(row.get("discounted_return") or 0.0) for row in group_rows) / max(
            len(group_rows), 1
        )
        for row in group_rows:
            row["_training_target"] = float(row.get("discounted_return") or 0.0) - mean_return


def train_model(
    rows: list[dict[str, Any]],
    feature_set: str,
    target_mean: float,
    target_std: float,
    epochs: int,
    learning_rate: float,
    l2: float,
    seed: int,
) -> dict[str, Any]:
    rng = random.Random(seed)
    weights: defaultdict[int, float] = defaultdict(float)
    bias = 0.0
    prepared = [
        (row_features(row, feature_set), (target(row) - target_mean) / target_std)
        for row in rows
    ]
    for _ in range(max(epochs, 0)):
        rng.shuffle(prepared)
        for sparse, y in prepared:
            pred = bias + dot(weights, sparse)
            if not math.isfinite(pred):
                pred = 0.0
            err = max(min(pred - y, 10.0), -10.0)
            norm = max(math.sqrt(sum(value * value for value in sparse.values())), 1.0)
            step = learning_rate / norm
            bias -= step * err
            for idx, value in sparse.items():
                weights[idx] -= step * (err * value + l2 * weights[idx])
    compact = {idx: value for idx, value in weights.items() if abs(value) > 1e-12}
    return {
        "feature_set": feature_set,
        "target_mean": target_mean,
        "target_std": target_std,
        "bias": bias,
        "weights": compact,
    }


def predict(model: dict[str, Any], row: dict[str, Any]) -> float:
    raw = float(model["bias"]) + dot(model["weights"], row_features(row, model["feature_set"]))
    return raw * float(model["target_std"]) + float(model["target_mean"])


def evaluate_all_splits(model: dict[str, Any], splits: dict[str, list[dict[str, Any]]]) -> dict[str, Any]:
    return {split: evaluate_rows(model, rows) for split, rows in splits.items()}


def evaluate_rows(model: dict[str, Any], rows: list[dict[str, Any]]) -> dict[str, Any]:
    if not rows:
        return {"count": 0, "mse": None, "mae": None, "top1_regret": None, "pairwise_accuracy": None}
    squared = 0.0
    absolute = 0.0
    by_group: defaultdict[str, list[tuple[float, float]]] = defaultdict(list)
    for row in rows:
        y = target(row)
        pred = predict(model, row)
        if not math.isfinite(pred):
            pred = 1.0e30
        squared += (pred - y) ** 2
        absolute += abs(pred - y)
        by_group[str(row.get("group_key") or "")].append((pred, y))
    return {
        "count": len(rows),
        "mse": squared / len(rows),
        "mae": absolute / len(rows),
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
    warnings = []
    for baseline in ["action_only", "candidate_only"]:
        base = test.get(baseline) or {}
        if not finite_metric(full.get("mae")) or not finite_metric(base.get("mae")) or full["mae"] >= base["mae"]:
            failures.append(f"full_state_plus_candidate MAE does not beat {baseline}")
        if not finite_metric(full.get("mse")) or not finite_metric(base.get("mse")) or full["mse"] >= base["mse"]:
            failures.append(f"full_state_plus_candidate MSE does not beat {baseline}")
        if not beats_pairwise(full, base, margin=0.02):
            failures.append(f"full_state_plus_candidate pairwise_accuracy does not beat {baseline} by >= 0.02")
        if not beats_top1_regret(full, base):
            failures.append(f"full_state_plus_candidate top1_regret does not beat {baseline}")
    state = test.get("state_only") or {}
    if not beats_pairwise(full, state, margin=0.02):
        failures.append("full_state_plus_candidate pairwise_accuracy does not beat state_only by >= 0.02")
    if not beats_top1_regret(full, state):
        failures.append("full_state_plus_candidate top1_regret does not beat state_only")
    if finite_metric(full.get("mae")) and finite_metric(state.get("mae")) and full["mae"] >= state["mae"]:
        warnings.append("state_only MAE is as good as or better than full_state_plus_candidate")
    if finite_metric(full.get("mse")) and finite_metric(state.get("mse")) and full["mse"] >= state["mse"]:
        warnings.append("state_only MSE is as good as or better than full_state_plus_candidate")
    return {
        "offline_return_q_gate_passed": not failures,
        "failures": failures,
        "warnings": warnings,
    }


def finite_metric(value: Any) -> bool:
    return isinstance(value, (int, float)) and math.isfinite(float(value))


def beats_pairwise(full: dict[str, Any], baseline: dict[str, Any], *, margin: float) -> bool:
    if not finite_metric(full.get("pairwise_accuracy")) or not finite_metric(baseline.get("pairwise_accuracy")):
        return False
    return float(full["pairwise_accuracy"]) >= float(baseline["pairwise_accuracy"]) + margin


def beats_top1_regret(full: dict[str, Any], baseline: dict[str, Any]) -> bool:
    if not finite_metric(full.get("top1_regret")) or not finite_metric(baseline.get("top1_regret")):
        return False
    return float(full["top1_regret"]) <= float(baseline["top1_regret"])


if __name__ == "__main__":
    main()
