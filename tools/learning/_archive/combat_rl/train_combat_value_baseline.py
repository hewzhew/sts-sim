#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import numpy as np
from sklearn.ensemble import RandomForestClassifier, RandomForestRegressor
from sklearn.feature_extraction import DictVectorizer
from sklearn.metrics import accuracy_score, mean_absolute_error

from combat_rl_common import REPO_ROOT, iter_jsonl, value_feature_dict, write_json, write_jsonl


def load_rows(path: Path) -> list[dict[str, Any]]:
    return [row for _, row in iter_jsonl(path)]


def safe_corr(y_true: np.ndarray, y_pred: np.ndarray) -> float:
    if y_true.size == 0 or np.std(y_true) == 0 or np.std(y_pred) == 0:
        return 0.0
    return float(np.corrcoef(y_true, y_pred)[0, 1])


def brier_score(y_true: np.ndarray, y_prob: np.ndarray) -> float:
    if y_true.size == 0:
        return 0.0
    return float(np.mean((y_prob - y_true) ** 2))


def evaluate_rows(
    rows: list[dict[str, Any]],
    vectorizer: DictVectorizer,
    return_reg: RandomForestRegressor,
    short_reg: RandomForestRegressor,
    survival_clf: RandomForestClassifier,
    kill_clf: RandomForestClassifier,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    if not rows:
        return ({"rows": 0}, [])
    x = vectorizer.transform([value_feature_dict(row) for row in rows]).toarray()
    y_return = np.asarray([float(row.get("discounted_return") or 0.0) for row in rows], dtype=np.float32)
    y_short = np.asarray([float(row.get("short_horizon_return") or 0.0) for row in rows], dtype=np.float32)
    y_survival = np.asarray([1 if row.get("survives_episode") else 0 for row in rows], dtype=np.int32)
    y_kill = np.asarray([1 if row.get("kill_within_horizon") else 0 for row in rows], dtype=np.int32)

    pred_return = return_reg.predict(x)
    pred_short = short_reg.predict(x)
    pred_survival = survival_clf.predict(x)
    survival_prob = survival_clf.predict_proba(x)[:, 1]
    pred_kill = kill_clf.predict(x)
    kill_prob = kill_clf.predict_proba(x)[:, 1]

    predictions = []
    for row, pred_r, pred_s, pred_surv, prob_surv, pred_k, prob_k in zip(
        rows, pred_return, pred_short, pred_survival, survival_prob, pred_kill, kill_prob, strict=False
    ):
        predictions.append(
            {
                "sample_id": row.get("sample_id"),
                "action_label": row.get("action_label"),
                "spec_name": row.get("spec_name"),
                "discounted_return": float(row.get("discounted_return") or 0.0),
                "pred_discounted_return": float(pred_r),
                "short_horizon_return": float(row.get("short_horizon_return") or 0.0),
                "pred_short_horizon_return": float(pred_s),
                "survives_episode": bool(row.get("survives_episode")),
                "pred_survives_episode": bool(pred_surv),
                "survival_probability": float(prob_surv),
                "kill_within_horizon": bool(row.get("kill_within_horizon")),
                "pred_kill_within_horizon": bool(pred_k),
                "kill_probability": float(prob_k),
            }
        )
    metrics = {
        "rows": len(rows),
        "discounted_return_mae": float(mean_absolute_error(y_return, pred_return)),
        "discounted_return_corr": safe_corr(y_return, pred_return),
        "short_horizon_return_mae": float(mean_absolute_error(y_short, pred_short)),
        "short_horizon_return_corr": safe_corr(y_short, pred_short),
        "survival_accuracy": float(accuracy_score(y_survival, pred_survival)),
        "survival_brier": brier_score(y_survival.astype(np.float32), survival_prob.astype(np.float32)),
        "kill_accuracy": float(accuracy_score(y_kill, pred_kill)),
        "kill_brier": brier_score(y_kill.astype(np.float32), kill_prob.astype(np.float32)),
    }
    return metrics, predictions


def main() -> int:
    parser = argparse.ArgumentParser(description="Train a baseline combat value model from simulator rollouts.")
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--dataset-prefix", default="combat_value")
    parser.add_argument("--metrics-out", default=None, type=Path)
    parser.add_argument("--predictions-out", default=None, type=Path)
    args = parser.parse_args()

    metrics_out = args.metrics_out or (args.dataset_dir / "value_metrics.json")
    predictions_out = args.predictions_out or (args.dataset_dir / "combat_value_predictions.jsonl")

    train_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_train.jsonl")
    val_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_val.jsonl")
    test_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_test.jsonl")
    if not train_rows:
        raise SystemExit("no combat value training rows found")

    vectorizer = DictVectorizer(sparse=True)
    x_train = vectorizer.fit_transform([value_feature_dict(row) for row in train_rows]).toarray()
    y_return = np.asarray([float(row.get("discounted_return") or 0.0) for row in train_rows], dtype=np.float32)
    y_short = np.asarray([float(row.get("short_horizon_return") or 0.0) for row in train_rows], dtype=np.float32)
    y_survival = np.asarray([1 if row.get("survives_episode") else 0 for row in train_rows], dtype=np.int32)
    y_kill = np.asarray([1 if row.get("kill_within_horizon") else 0 for row in train_rows], dtype=np.int32)

    return_reg = RandomForestRegressor(n_estimators=160, random_state=0, n_jobs=-1)
    return_reg.fit(x_train, y_return)
    short_reg = RandomForestRegressor(n_estimators=160, random_state=1, n_jobs=-1)
    short_reg.fit(x_train, y_short)
    survival_clf = RandomForestClassifier(n_estimators=160, random_state=2, n_jobs=-1)
    survival_clf.fit(x_train, y_survival)
    kill_clf = RandomForestClassifier(n_estimators=160, random_state=3, n_jobs=-1)
    kill_clf.fit(x_train, y_kill)

    train_metrics, _ = evaluate_rows(train_rows, vectorizer, return_reg, short_reg, survival_clf, kill_clf)
    val_metrics, _ = evaluate_rows(val_rows, vectorizer, return_reg, short_reg, survival_clf, kill_clf)
    test_metrics, predictions = evaluate_rows(test_rows, vectorizer, return_reg, short_reg, survival_clf, kill_clf)

    metrics = {
        "model": "random_forest_value_baseline",
        "dataset_prefix": args.dataset_prefix,
        "feature_count": len(vectorizer.feature_names_),
        "train": train_metrics,
        "val": val_metrics,
        "test": test_metrics,
        "notes": [
            "value baseline predicts short-horizon and discounted return plus survival/kill probabilities",
            "this is intended to replace reranker-only training as the main local combat signal",
        ],
    }
    write_json(metrics_out, metrics)
    write_jsonl(predictions_out, predictions)

    print(json.dumps(metrics, indent=2, ensure_ascii=False))
    print(f"wrote value metrics to {metrics_out}")
    print(f"wrote value predictions to {predictions_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
