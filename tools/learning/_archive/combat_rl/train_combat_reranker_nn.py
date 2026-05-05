#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import numpy as np
from sklearn.feature_extraction import DictVectorizer
from sklearn.neural_network import MLPClassifier

from combat_reranker_common import (
    candidate_feature_dict,
    evaluate_grouped_predictions,
    iter_jsonl,
    tag_correction_summary,
    top_scoring_mistakes,
    write_json,
    write_jsonl,
)

REPO_ROOT = Path(__file__).resolve().parents[2]


def load_rows(path: Path) -> list[dict[str, Any]]:
    return [row for _, row in iter_jsonl(path)]


def training_rows(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [row for row in rows if bool(row.get("training_eligible"))]


def row_score_fn(model: MLPClassifier, vectorizer: DictVectorizer):
    def score(row: dict[str, Any]) -> float:
        features = vectorizer.transform([candidate_feature_dict(row)]).toarray()
        probabilities = model.predict_proba(features)[0]
        return float(probabilities[1])

    return score


def main() -> int:
    parser = argparse.ArgumentParser(description="Train a small neural offline combat reranker scorer.")
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument(
        "--dataset-prefix",
        default="combat_reranker",
        help="Input dataset prefix produced by prepare_combat_reranker_dataset.py.",
    )
    parser.add_argument(
        "--output-prefix",
        default="combat_reranker_nn",
        help="Prefix for metrics/predictions/review artifacts.",
    )
    parser.add_argument(
        "--metrics-out",
        default=None,
        type=Path,
    )
    parser.add_argument(
        "--predictions-out",
        default=None,
        type=Path,
    )
    parser.add_argument(
        "--review-out",
        default=None,
        type=Path,
    )
    args = parser.parse_args()

    dataset_prefix = args.dataset_prefix
    output_prefix = args.output_prefix
    metrics_out = args.metrics_out or (args.dataset_dir / f"{output_prefix}_metrics.json")
    predictions_out = args.predictions_out or (args.dataset_dir / f"{output_prefix}_predictions.jsonl")
    review_out = args.review_out or (args.dataset_dir / f"{output_prefix}_review.json")

    train = load_rows(args.dataset_dir / f"{dataset_prefix}_train.jsonl")
    val = load_rows(args.dataset_dir / f"{dataset_prefix}_val.jsonl")
    test = load_rows(args.dataset_dir / f"{dataset_prefix}_test.jsonl")
    fit_rows = training_rows(train)
    if not fit_rows:
        raise SystemExit("no training-eligible combat reranker rows found")

    vectorizer = DictVectorizer(sparse=True)
    x_train = vectorizer.fit_transform(candidate_feature_dict(row) for row in fit_rows).toarray()
    y_train = np.asarray([1 if row.get("candidate_is_positive") else 0 for row in fit_rows], dtype=np.int32)
    if len(np.unique(y_train)) < 2:
        raise SystemExit("neural reranker requires both positive and negative training rows")

    model = MLPClassifier(
        hidden_layer_sizes=(64, 32),
        activation="relu",
        max_iter=300,
        random_state=0,
        early_stopping=True,
        validation_fraction=0.2,
    )
    model.fit(x_train, y_train)

    scorer = row_score_fn(model, vectorizer)
    train_eval = evaluate_grouped_predictions(train, scorer)
    val_eval = evaluate_grouped_predictions(val, scorer)
    test_eval = evaluate_grouped_predictions(test, scorer)
    predictions = test_eval.pop("predictions")
    test_tag_corrections = tag_correction_summary(predictions)

    metrics = {
        "model": "mlp_candidate_scorer",
        "dataset_dir": str(args.dataset_dir),
        "dataset_prefix": dataset_prefix,
        "output_prefix": output_prefix,
        "feature_count": int(len(vectorizer.feature_names_)),
        "train_rows": len(train),
        "val_rows": len(val),
        "test_rows": len(test),
        "fit_rows": len(fit_rows),
        "fit_positive_rows": int(y_train.sum()),
        "fit_negative_rows": int((y_train == 0).sum()),
        "train": train_eval,
        "val": val_eval,
        "test": test_eval,
        "test_tag_corrections": test_tag_corrections,
        "nn_config": {
            "hidden_layer_sizes": [64, 32],
            "activation": "relu",
            "max_iter": 300,
            "early_stopping": True,
        },
        "notes": [
            "this is a small neural reranker entrypoint that reuses the packed baseline dataset",
            "it is intended to validate dataset compatibility, not to replace the baseline trainer in this round",
        ],
    }
    write_json(metrics_out, metrics)
    write_jsonl(predictions_out, predictions)
    write_json(
        review_out,
        {
            "metrics_path": str(metrics_out),
            "predictions_path": str(predictions_out),
            "test_tag_corrections": test_tag_corrections,
            "hard_mistakes": top_scoring_mistakes(predictions),
        },
    )

    import json
    print(json.dumps(metrics, indent=2, ensure_ascii=False))
    print(f"wrote neural reranker metrics to {metrics_out}")
    print(f"wrote neural reranker predictions to {predictions_out}")
    print(f"wrote neural reranker review to {review_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
