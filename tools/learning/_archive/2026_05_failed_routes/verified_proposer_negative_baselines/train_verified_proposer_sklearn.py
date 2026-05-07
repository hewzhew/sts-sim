#!/usr/bin/env python3
"""Train nonlinear sklearn verified-H proposer baselines.

This is an offline audit tool.  The trained model is not used for direct
decisions; it is evaluated only as a high-recall filter for the exact H-step
verifier.
"""
from __future__ import annotations

import argparse
import json
import pickle
from collections import defaultdict
from pathlib import Path
from typing import Any

import numpy as np
from scipy.sparse import csr_matrix
from sklearn.ensemble import ExtraTreesClassifier
from sklearn.metrics import average_precision_score, roc_auc_score
from sklearn.neural_network import MLPClassifier

from return_q_common import ADV_OVERRIDE_FEATURE_SETS, read_jsonl, write_json
from train_verified_proposer_linear import (
    best_for_target_recall,
    hybrid_metrics,
    label_counts,
    parse_float_list,
    parse_int_list,
    prepare_examples,
    split_counts,
    threshold_metrics,
    topk_metrics,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--model-out", type=Path, required=True)
    parser.add_argument("--report-out", type=Path)
    parser.add_argument("--feature-set", default="candidate_plus_cheap", choices=ADV_OVERRIDE_FEATURE_SETS)
    parser.add_argument("--target-mode", default="oracle_chosen", choices=["oracle_chosen", "margin_positive"])
    parser.add_argument("--model-kind", default="mlp", choices=["mlp", "extra_trees"])
    parser.add_argument("--feature-dim", type=int, default=32768)
    parser.add_argument("--seed", type=int, default=29)
    parser.add_argument("--thresholds", default="0.01,0.02,0.05,0.1,0.2,0.3,0.5,0.7,0.9")
    parser.add_argument("--top-k", default="1,2,3,4,6,8")
    parser.add_argument("--target-recalls", default="0.8,0.9,0.95")
    parser.add_argument("--hidden-layer-sizes", default="64")
    parser.add_argument("--max-iter", type=int, default=40)
    parser.add_argument("--alpha", type=float, default=0.0001)
    parser.add_argument("--learning-rate-init", type=float, default=0.001)
    parser.add_argument("--n-estimators", type=int, default=200)
    parser.add_argument("--max-depth", type=int, default=14)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    thresholds = parse_float_list(args.thresholds)
    top_ks = parse_int_list(args.top_k)
    target_recalls = parse_float_list(args.target_recalls)

    rows = read_jsonl(args.input)
    examples = prepare_examples(rows, args.feature_set, args.target_mode)
    if not examples:
        raise SystemExit("no trainable examples")
    x = sparse_matrix(examples, args.feature_dim)
    y = np.asarray([1 if ex["target"] > 0.5 else 0 for ex in examples], dtype=np.int64)
    train_indices = np.asarray([idx for idx, ex in enumerate(examples) if ex["split"] == "train"], dtype=np.int64)
    if train_indices.size == 0:
        raise SystemExit("no train split examples")
    train_y = y[train_indices]
    pos = int(train_y.sum())
    neg = int(train_y.size - pos)
    pos_weight = float(neg / max(pos, 1))
    sample_weight = np.where(train_y > 0, pos_weight, 1.0).astype(np.float32)

    model = build_model(args)
    fit_model(model, x[train_indices], train_y, sample_weight)
    probs = predict_positive_probability(model, x)
    metrics = {
        split: evaluate_predictions(
            [ex for ex in examples if ex["split"] == split],
            [float(probs[idx]) for idx, ex in enumerate(examples) if ex["split"] == split],
            thresholds,
            top_ks,
            target_recalls,
        )
        for split in ["train", "valid", "test"]
    }
    args.model_out.parent.mkdir(parents=True, exist_ok=True)
    with args.model_out.open("wb") as handle:
        pickle.dump(
            {
                "schema_version": "verified_proposer_sklearn_pickle_v0",
                "model_type": "verified_proposer_sklearn_pickle_v0",
                "model_kind": args.model_kind,
                "feature_set": args.feature_set,
                "target_mode": args.target_mode,
                "feature_dim": args.feature_dim,
                "model": model,
            },
            handle,
        )
    report = {
        "schema_version": "verified_proposer_sklearn_report_v0",
        "input": str(args.input),
        "model_out": str(args.model_out),
        "model_kind": args.model_kind,
        "feature_set": args.feature_set,
        "target_mode": args.target_mode,
        "row_count": len(rows),
        "example_count": len(examples),
        "label_counts": label_counts(examples),
        "split_counts": split_counts(examples),
        "config": {
            "feature_dim": args.feature_dim,
            "seed": args.seed,
            "pos_weight": pos_weight,
            "hidden_layer_sizes": parse_hidden_layers(args.hidden_layer_sizes),
            "max_iter": args.max_iter,
            "alpha": args.alpha,
            "learning_rate_init": args.learning_rate_init,
            "n_estimators": args.n_estimators,
            "max_depth": args.max_depth,
        },
        "metrics": metrics,
    }
    write_json(args.report_out or args.model_out.with_suffix(".report.json"), report)
    print(json.dumps(report, indent=2, sort_keys=True))


def build_model(args: argparse.Namespace) -> Any:
    if args.model_kind == "mlp":
        return MLPClassifier(
            hidden_layer_sizes=parse_hidden_layers(args.hidden_layer_sizes),
            activation="relu",
            solver="adam",
            alpha=args.alpha,
            batch_size=512,
            learning_rate_init=args.learning_rate_init,
            max_iter=args.max_iter,
            early_stopping=True,
            validation_fraction=0.15,
            n_iter_no_change=6,
            random_state=args.seed,
            verbose=False,
        )
    return ExtraTreesClassifier(
        n_estimators=args.n_estimators,
        max_depth=args.max_depth if args.max_depth > 0 else None,
        min_samples_leaf=2,
        class_weight="balanced",
        random_state=args.seed,
        n_jobs=-1,
    )


def fit_model(model: Any, x_train: csr_matrix, y_train: np.ndarray, sample_weight: np.ndarray) -> None:
    try:
        model.fit(x_train, y_train, sample_weight=sample_weight)
    except TypeError:
        model.fit(x_train, y_train)


def predict_positive_probability(model: Any, x: csr_matrix) -> np.ndarray:
    if hasattr(model, "predict_proba"):
        probs = model.predict_proba(x)
        classes = list(getattr(model, "classes_", [0, 1]))
        pos_idx = classes.index(1) if 1 in classes else len(classes) - 1
        return np.asarray(probs[:, pos_idx], dtype=np.float64)
    if hasattr(model, "decision_function"):
        scores = np.asarray(model.decision_function(x), dtype=np.float64)
        return 1.0 / (1.0 + np.exp(-scores))
    return np.asarray(model.predict(x), dtype=np.float64)


def sparse_matrix(examples: list[dict[str, Any]], feature_dim: int) -> csr_matrix:
    data: list[float] = []
    indices: list[int] = []
    indptr = [0]
    for ex in examples:
        sparse = ex["features"]
        for idx, value in sparse.items():
            idx_i = int(idx)
            if 0 <= idx_i < feature_dim and value:
                indices.append(idx_i)
                data.append(float(value))
        indptr.append(len(indices))
    return csr_matrix(
        (
            np.asarray(data, dtype=np.float32),
            np.asarray(indices, dtype=np.int32),
            np.asarray(indptr, dtype=np.int32),
        ),
        shape=(len(examples), feature_dim),
    )


def evaluate_predictions(
    examples: list[dict[str, Any]],
    probs: list[float],
    thresholds: list[float],
    top_ks: list[int],
    target_recalls: list[float],
) -> dict[str, Any]:
    if not examples:
        return {"count": 0}
    scored = []
    by_group: defaultdict[str, list[dict[str, Any]]] = defaultdict(list)
    for ex, prob in zip(examples, probs):
        row = {
            "group_key": ex["group_key"],
            "prob": float(prob),
            "target": float(ex["target"]),
            "adv": float(ex["adv"]),
            "is_rule": bool(ex["is_rule"]),
        }
        scored.append(row)
        by_group[row["group_key"]].append(row)
    threshold_rows = {str(threshold): threshold_metrics(scored, by_group, threshold) for threshold in thresholds}
    topk_rows = {str(k): topk_metrics(by_group, k) for k in top_ks}
    hybrid_rows = {
        f"top{k}_thr{threshold}": hybrid_metrics(by_group, k, threshold)
        for k in top_ks
        for threshold in thresholds
    }
    candidates = []
    for name, row in threshold_rows.items():
        candidates.append({"selector": "threshold", "setting": name, **row})
    for name, row in topk_rows.items():
        candidates.append({"selector": "top_k", "setting": name, **row})
    for name, row in hybrid_rows.items():
        candidates.append({"selector": "hybrid", "setting": name, **row})
    y_true = np.asarray([row["target"] for row in scored], dtype=np.int64)
    y_prob = np.asarray([row["prob"] for row in scored], dtype=np.float64)
    ap = float(average_precision_score(y_true, y_prob)) if y_true.sum() else None
    try:
        roc_auc = float(roc_auc_score(y_true, y_prob)) if 0 < y_true.sum() < len(y_true) else None
    except ValueError:
        roc_auc = None
    return {
        "count": len(scored),
        "positive_count": int(y_true.sum()),
        "positive_rate": float(y_true.mean()) if len(y_true) else 0.0,
        "average_precision": ap,
        "roc_auc": roc_auc,
        "thresholds": threshold_rows,
        "top_k": topk_rows,
        "hybrid": hybrid_rows,
        "best_for_target_recall": {
            str(target): best_for_target_recall(candidates, target)
            for target in target_recalls
        },
    }


def parse_hidden_layers(text: str) -> tuple[int, ...]:
    values = tuple(int(part.strip()) for part in text.split(",") if part.strip())
    if not values:
        raise SystemExit("expected at least one hidden layer size")
    return values


if __name__ == "__main__":
    main()
