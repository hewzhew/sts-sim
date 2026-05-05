#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import numpy as np
from sklearn.feature_extraction import DictVectorizer
from sklearn.linear_model import LogisticRegression

from combat_rl_common import (
    REPO_ROOT,
    grouped_prediction_metrics,
    iter_jsonl,
    policy_candidate_feature_dict,
    tag_correction_summary,
    write_json,
    write_jsonl,
)


def load_rows(path: Path) -> list[dict[str, Any]]:
    return [row for _, row in iter_jsonl(path)]


def scorer_from_model(model: LogisticRegression, vectorizer: DictVectorizer):
    def score(row: dict[str, Any]) -> float:
        features = vectorizer.transform([policy_candidate_feature_dict(row)]).toarray()
        probabilities = model.predict_proba(features)[0]
        return float(probabilities[1])

    return score


def main() -> int:
    parser = argparse.ArgumentParser(description="Train a PPO-ready policy baseline from combat policy rows.")
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--dataset-prefix", default="combat_policy")
    parser.add_argument("--metrics-out", default=None, type=Path)
    parser.add_argument("--predictions-out", default=None, type=Path)
    args = parser.parse_args()

    metrics_out = args.metrics_out or (args.dataset_dir / "ppo_eval_metrics.json")
    predictions_out = args.predictions_out or (args.dataset_dir / "combat_policy_predictions.jsonl")

    train_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_train.jsonl")
    val_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_val.jsonl")
    test_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_test.jsonl")
    if not train_rows:
        raise SystemExit("no combat policy rows found")

    vectorizer = DictVectorizer(sparse=True)
    x_train = vectorizer.fit_transform([policy_candidate_feature_dict(row) for row in train_rows]).toarray()
    y_train = np.asarray([1 if row.get("candidate_is_positive") else 0 for row in train_rows], dtype=np.int32)
    sample_weight = np.asarray([float(row.get("training_weight") or 0.1) for row in train_rows], dtype=np.float32)
    if len(np.unique(y_train)) < 2:
        raise SystemExit("combat policy baseline requires both positive and negative examples")

    model = LogisticRegression(max_iter=1000, class_weight="balanced", solver="liblinear")
    model.fit(x_train, y_train, sample_weight=sample_weight)
    scorer = scorer_from_model(model, vectorizer)

    train_metrics = grouped_prediction_metrics(train_rows, scorer)
    val_metrics = grouped_prediction_metrics(val_rows, scorer)
    test_metrics = grouped_prediction_metrics(test_rows, scorer)
    train_metrics.pop("predictions", None)
    val_metrics.pop("predictions", None)
    predictions = test_metrics.pop("predictions")
    tag_metrics = tag_correction_summary(predictions)

    metrics = {
        "model": "logistic_policy_baseline",
        "dataset_prefix": args.dataset_prefix,
        "feature_count": len(vectorizer.feature_names_),
        "train": train_metrics,
        "val": val_metrics,
        "test": test_metrics,
        "test_tag_corrections": tag_metrics,
        "notes": [
            "this is the PPO-ready control policy baseline for the new hybrid RL pipeline",
            "it is still offline and candidate-scoring based, but no longer treated as the main end state",
            "true PPO runtime control remains a later step once a neural backend is available",
        ],
    }
    write_json(metrics_out, metrics)
    write_jsonl(predictions_out, predictions)

    print(json.dumps(metrics, indent=2, ensure_ascii=False))
    print(f"wrote policy baseline metrics to {metrics_out}")
    print(f"wrote policy baseline predictions to {predictions_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
