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
from sklearn.multioutput import MultiOutputRegressor

from combat_rl_common import REPO_ROOT, iter_jsonl, transition_feature_dict, write_json, write_jsonl

NUMERIC_TARGETS = [
    "next_player_hp",
    "next_player_block",
    "next_energy",
    "next_total_monster_hp",
    "next_living_monster_count",
    "next_hand_count",
    "next_draw_count",
    "next_discard_count",
    "reward_total",
]


def load_rows(path: Path) -> list[dict[str, Any]]:
    return [row for _, row in iter_jsonl(path)]


def numeric_target_matrix(rows: list[dict[str, Any]]) -> np.ndarray:
    matrix = []
    for row in rows:
        after = row.get("state_after_features") or {}
        matrix.append(
            [
                float(after.get("player_current_hp") or 0.0),
                float(after.get("player_block") or 0.0),
                float(after.get("player_energy") or 0.0),
                float(after.get("total_monster_hp") or 0.0),
                float(after.get("living_monster_count") or 0.0),
                float(after.get("hand_count") or 0.0),
                float(after.get("draw_count") or 0.0),
                float(after.get("discard_count") or 0.0),
                float(row.get("reward_total") or 0.0),
            ]
        )
    return np.asarray(matrix, dtype=np.float32)


def terminal_targets(rows: list[dict[str, Any]]) -> tuple[np.ndarray, np.ndarray]:
    done = np.asarray([1 if row.get("done") else 0 for row in rows], dtype=np.int32)
    outcome = np.asarray(
        [2 if row.get("terminal_victory") else 1 if row.get("terminal_defeat") else 0 for row in rows],
        dtype=np.int32,
    )
    return done, outcome


def evaluate_rows(
    rows: list[dict[str, Any]],
    vectorizer: DictVectorizer,
    regressor: MultiOutputRegressor,
    done_clf: RandomForestClassifier,
    outcome_clf: RandomForestClassifier,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    if not rows:
        return ({"rows": 0}, [])
    x = vectorizer.transform([transition_feature_dict(row) for row in rows]).toarray()
    y_numeric = numeric_target_matrix(rows)
    y_done, y_outcome = terminal_targets(rows)

    pred_numeric = regressor.predict(x)
    pred_done = done_clf.predict(x)
    pred_outcome = outcome_clf.predict(x)

    numeric_mae = {
        target: float(mean_absolute_error(y_numeric[:, index], pred_numeric[:, index]))
        for index, target in enumerate(NUMERIC_TARGETS)
    }
    predictions = []
    for row, pred_vec, pred_done_value, pred_outcome_value in zip(rows, pred_numeric, pred_done, pred_outcome, strict=False):
        predictions.append(
            {
                "sample_id": row.get("sample_id"),
                "spec_name": row.get("spec_name"),
                "turn_index": row.get("turn_index"),
                "step_index": row.get("step_index"),
                "action_label": row.get("action_label"),
                "actual_next_player_hp": float((row.get("state_after_features") or {}).get("player_current_hp") or 0.0),
                "pred_next_player_hp": float(pred_vec[0]),
                "actual_next_total_monster_hp": float((row.get("state_after_features") or {}).get("total_monster_hp") or 0.0),
                "pred_next_total_monster_hp": float(pred_vec[3]),
                "actual_reward_total": float(row.get("reward_total") or 0.0),
                "pred_reward_total": float(pred_vec[8]),
                "actual_done": bool(row.get("done")),
                "pred_done": bool(pred_done_value),
                "actual_terminal_outcome": row.get("terminal_outcome"),
                "pred_terminal_outcome": "victory" if pred_outcome_value == 2 else "defeat" if pred_outcome_value == 1 else "ongoing",
            }
        )
    metrics = {
        "rows": len(rows),
        "numeric_mae": numeric_mae,
        "reward_mae": numeric_mae["reward_total"],
        "terminal_prediction_accuracy": float(accuracy_score(y_outcome, pred_outcome)),
        "done_prediction_accuracy": float(accuracy_score(y_done, pred_done)),
    }
    return metrics, predictions


def main() -> int:
    parser = argparse.ArgumentParser(description="Train baseline transition models on combat transition rows.")
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--dataset-prefix", default="combat_transition")
    parser.add_argument("--metrics-out", default=None, type=Path)
    parser.add_argument("--predictions-out", default=None, type=Path)
    args = parser.parse_args()

    metrics_out = args.metrics_out or (args.dataset_dir / "transition_metrics.json")
    predictions_out = args.predictions_out or (args.dataset_dir / "combat_transition_predictions.jsonl")

    train_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_train.jsonl")
    val_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_val.jsonl")
    test_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_test.jsonl")

    if not train_rows:
        raise SystemExit("no combat transition training rows found")

    vectorizer = DictVectorizer(sparse=True)
    x_train = vectorizer.fit_transform([transition_feature_dict(row) for row in train_rows]).toarray()
    y_train_numeric = numeric_target_matrix(train_rows)
    y_train_done, y_train_outcome = terminal_targets(train_rows)

    regressor = MultiOutputRegressor(RandomForestRegressor(n_estimators=120, random_state=0, n_jobs=-1))
    regressor.fit(x_train, y_train_numeric)

    done_clf = RandomForestClassifier(n_estimators=120, random_state=0, n_jobs=-1)
    done_clf.fit(x_train, y_train_done)

    outcome_clf = RandomForestClassifier(n_estimators=120, random_state=0, n_jobs=-1)
    outcome_clf.fit(x_train, y_train_outcome)

    train_metrics, _ = evaluate_rows(train_rows, vectorizer, regressor, done_clf, outcome_clf)
    val_metrics, _ = evaluate_rows(val_rows, vectorizer, regressor, done_clf, outcome_clf)
    test_metrics, predictions = evaluate_rows(test_rows, vectorizer, regressor, done_clf, outcome_clf)

    metrics = {
        "model": "random_forest_transition_baseline",
        "dataset_prefix": args.dataset_prefix,
        "feature_count": len(vectorizer.feature_names_),
        "train": train_metrics,
        "val": val_metrics,
        "test": test_metrics,
        "targets": NUMERIC_TARGETS,
        "notes": [
            "transition baseline predicts next-state summary fields and terminal flags from state+action features",
            "this is the first offline simulator-outcomes baseline, not a runtime model",
        ],
    }
    write_json(metrics_out, metrics)
    write_jsonl(predictions_out, predictions)

    print(json.dumps(metrics, indent=2, ensure_ascii=False))
    print(f"wrote transition metrics to {metrics_out}")
    print(f"wrote transition predictions to {predictions_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
