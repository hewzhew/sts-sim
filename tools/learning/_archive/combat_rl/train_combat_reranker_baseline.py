#!/usr/bin/env python3
from __future__ import annotations

import argparse
from pathlib import Path
from typing import Any

import numpy as np
from sklearn.feature_extraction import DictVectorizer
from sklearn.linear_model import LogisticRegression

from combat_reranker_common import (
    candidate_feature_dict,
    evaluate_grouped_predictions,
    group_rows,
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


def row_score_fn(model: LogisticRegression, vectorizer: DictVectorizer):
    def score(row: dict[str, Any]) -> float:
        features = vectorizer.transform([candidate_feature_dict(row)])
        probabilities = model.predict_proba(features)[0]
        return float(probabilities[1])

    return score


def baseline_move_score_fn():
    def score(row: dict[str, Any]) -> float:
        return 1.0 if row.get("candidate_move") == row.get("baseline_chosen_move") else 0.0

    return score


def current_top1_score_fn():
    def score(row: dict[str, Any]) -> float:
        rank = row.get("candidate_search_rank")
        if rank is None:
            return 0.0
        return -float(rank)

    return score


def main() -> int:
    parser = argparse.ArgumentParser(description="Train the first lightweight offline combat reranker baseline.")
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument(
        "--dataset-prefix",
        default="combat_reranker",
        help="Input dataset prefix produced by prepare_combat_reranker_dataset.py.",
    )
    parser.add_argument(
        "--output-prefix",
        default="combat_reranker_baseline",
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
    x_train = vectorizer.fit_transform(candidate_feature_dict(row) for row in fit_rows)
    y_train = np.asarray([1 if row.get("candidate_is_positive") else 0 for row in fit_rows], dtype=np.int32)
    sample_weight = np.asarray([float(row.get("sample_weight") or 0.0) for row in fit_rows], dtype=np.float32)
    model = LogisticRegression(
        max_iter=400,
        solver="liblinear",
        class_weight="balanced",
        random_state=0,
    )
    model.fit(x_train, y_train, sample_weight=sample_weight)

    model_score = row_score_fn(model, vectorizer)
    train_eval = evaluate_grouped_predictions(train, model_score)
    val_eval = evaluate_grouped_predictions(val, model_score)
    test_eval = evaluate_grouped_predictions(test, model_score)
    baseline_eval = evaluate_grouped_predictions(test, baseline_move_score_fn())
    current_top1_groups = [
        group
        for group in group_rows(test).values()
        if any(row.get("candidate_search_rank") is not None for row in group)
    ]
    current_top1_eval = evaluate_grouped_predictions(
        [row for group in current_top1_groups for row in group],
        current_top1_score_fn(),
    )

    predictions = test_eval.pop("predictions")
    baseline_predictions = baseline_eval.pop("predictions")
    current_top1_predictions = current_top1_eval.pop("predictions")
    test_tag_corrections = tag_correction_summary(predictions)
    baseline_tag_corrections = tag_correction_summary(baseline_predictions)
    current_top1_tag_corrections = tag_correction_summary(current_top1_predictions)

    metrics = {
        "model": "logistic_regression_candidate_scorer",
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
        "empty_baselines": {
            "baseline_chosen_move": {
                **baseline_eval,
                "tag_corrections": baseline_tag_corrections,
            },
            "current_search_top1": {
                **current_top1_eval,
                "tag_corrections": current_top1_tag_corrections,
            },
        },
        "notes": [
            "candidate-level reranker baseline trained only on oracle_strong / oracle_preference rows flagged as training_eligible",
            "baseline_chosen_move and current_search_top1 are evaluation-only controls",
            "features exclude oracle-only target labels to avoid offline leakage",
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
            "baseline_tag_corrections": baseline_tag_corrections,
            "current_top1_tag_corrections": current_top1_tag_corrections,
            "hard_mistakes": top_scoring_mistakes(predictions),
            "baseline_control_mistakes": top_scoring_mistakes(baseline_predictions),
            "current_top1_control_mistakes": top_scoring_mistakes(current_top1_predictions),
        },
    )

    import json
    print(json.dumps(metrics, indent=2, ensure_ascii=False))
    print(f"wrote baseline reranker metrics to {metrics_out}")
    print(f"wrote baseline reranker predictions to {predictions_out}")
    print(f"wrote baseline reranker review to {review_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
