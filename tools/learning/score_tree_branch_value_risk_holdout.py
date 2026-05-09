#!/usr/bin/env python3
"""Train tree branch outcome models on one source and score a fresh holdout.

This is a no-leak generalization audit helper. Unlike
train_tree_branch_value_risk_ablation.py, it does not split one dataset into
train/test rows. It fits on explicit training branch/pair datasets and scores
explicit holdout branch/pair datasets.

The outputs keep the existing prediction JSONL shape so downstream allocation
gate tools can consume them unchanged. They are still value/risk/search audit
artifacts, not action labels or policy preferences.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import numpy as np
from sklearn.feature_extraction import DictVectorizer

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from train_tree_branch_value_risk_ablation import (  # noqa: E402
    TreeModelFamily,
    binary_metrics,
    branch_feature_dict,
    branch_prediction_records,
    branch_target,
    candidate_audit_summary,
    decision_context_audit_summary,
    fit_classifier,
    fit_regressor,
    load_rows,
    material_sign_metrics,
    pair_feature_dict,
    pair_prediction_records,
    pair_side_to_branch_row,
    pair_tail_specs,
    pair_true_diff,
    predict_classifier,
    regression_metrics,
    safe_float,
    write_jsonl,
)


def branch_id(row: dict[str, Any]) -> str | None:
    value = row.get("branch_id")
    return value if isinstance(value, str) else None


def episode_seed_set(rows: list[dict[str, Any]]) -> set[Any]:
    return {row.get("episode_seed") for row in rows if row.get("episode_seed") is not None}


def annotate_rows(
    rows: list[dict[str, Any]],
    *,
    source: str,
    train_branches: Path,
    train_pairs: Path,
    score_branches: Path,
    score_pairs: Path,
) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for row in rows:
        next_row = dict(row)
        label_policy = dict(next_row.get("label_policy") or {})
        label_policy.update(
            {
                "action_label": False,
                "source": source,
                "holdout_score_rows_used_for_fit": False,
            }
        )
        next_row["label_policy"] = label_policy
        next_row["holdout_scoring"] = {
            "schema_version": "tree_branch_value_risk_holdout_score_v0",
            "no_train_leak": True,
            "train_branches": str(train_branches),
            "train_pairs": str(train_pairs),
            "score_branches": str(score_branches),
            "score_pairs": str(score_pairs),
        }
        out.append(next_row)
    return out


def pair_examples(
    pair_rows: list[dict[str, Any]],
    *,
    branch_by_id: dict[str, dict[str, Any]],
    branch_hp_pred: dict[str, float],
    branch_reward_pred: dict[str, float],
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    examples: list[dict[str, Any]] = []
    feature_rows: list[dict[str, Any]] = []
    for row in pair_rows:
        left_side = row.get("left") or {}
        right_side = row.get("right") or {}
        left_id = left_side.get("branch_id")
        right_id = right_side.get("branch_id")
        left = branch_by_id.get(left_id) or pair_side_to_branch_row(left_side, row)
        right = branch_by_id.get(right_id) or pair_side_to_branch_row(right_side, row)
        base_hp = branch_hp_pred.get(left_id, 0.0) - branch_hp_pred.get(right_id, 0.0)
        base_reward = branch_reward_pred.get(left_id, 0.0) - branch_reward_pred.get(
            right_id, 0.0
        )
        feature_rows.append(
            pair_feature_dict(
                row,
                left,
                right,
                base_hp_diff=base_hp,
                base_reward_diff=base_reward,
            )
        )
        examples.append(
            {
                "row": row,
                "base_hp": base_hp,
                "base_reward": base_reward,
                "true_hp": pair_true_diff(row, "hp_left_minus_right"),
                "true_reward": pair_true_diff(row, "total_reward_left_minus_right"),
            }
        )
    return examples, feature_rows


def train_and_score(args: argparse.Namespace) -> dict[str, Any]:
    train_branches = load_rows(args.train_branches, row_kind="train_branch")
    train_pairs = load_rows(args.train_pairs, row_kind="train_pair")
    score_branches = load_rows(args.score_branches, row_kind="score_branch")
    score_pairs = load_rows(args.score_pairs, row_kind="score_pair")

    family = TreeModelFamily(args.model_kind, args.seed)

    branch_vectorizer = DictVectorizer(sparse=False)
    x_branch_train = branch_vectorizer.fit_transform(
        [branch_feature_dict(row) for row in train_branches]
    )
    x_branch_score = branch_vectorizer.transform(
        [branch_feature_dict(row) for row in score_branches]
    )

    hp_model = fit_regressor(
        family.regressor(), x_branch_train, [branch_target(row, "hp_delta") for row in train_branches]
    )
    reward_model = fit_regressor(
        family.regressor(),
        x_branch_train,
        [branch_target(row, "total_reward") for row in train_branches],
    )
    hp_pred_train = [float(v) for v in hp_model.predict(x_branch_train)]
    reward_pred_train = [float(v) for v in reward_model.predict(x_branch_train)]
    hp_pred_score = [float(v) for v in hp_model.predict(x_branch_score)]
    reward_pred_score = [float(v) for v in reward_model.predict(x_branch_score)]

    risk_scores_score: dict[str, list[float]] = {}
    risk_summary: dict[str, Any] = {}
    for name, threshold in (("hp_loss_ge_5", -5), ("hp_loss_ge_10", -10)):
        labels_train = [int(branch_target(row, "hp_delta") <= threshold) for row in train_branches]
        labels_score = [int(branch_target(row, "hp_delta") <= threshold) for row in score_branches]
        prior = sum(labels_train) / len(labels_train) if labels_train else 0.0
        model = fit_classifier(family.classifier(), x_branch_train, labels_train)
        scores = predict_classifier(model, x_branch_score, prior)
        risk_scores_score[name] = scores
        risk_summary[name] = binary_metrics(labels_score, scores)

    branch_predictions = annotate_rows(
        branch_prediction_records(
            score_branches, hp_pred_score, reward_pred_score, risk_scores_score
        ),
        source="tree_branch_value_risk_holdout_score",
        train_branches=args.train_branches,
        train_pairs=args.train_pairs,
        score_branches=args.score_branches,
        score_pairs=args.score_pairs,
    )
    write_jsonl(args.prediction_out, branch_predictions)

    train_branch_by_id = {
        bid: row for row in train_branches if (bid := branch_id(row)) is not None
    }
    score_branch_by_id = {
        bid: row for row in score_branches if (bid := branch_id(row)) is not None
    }
    train_hp_by_id = {
        bid: hp_pred_train[index]
        for index, row in enumerate(train_branches)
        if (bid := branch_id(row)) is not None
    }
    train_reward_by_id = {
        bid: reward_pred_train[index]
        for index, row in enumerate(train_branches)
        if (bid := branch_id(row)) is not None
    }
    score_hp_by_id = {
        bid: hp_pred_score[index]
        for index, row in enumerate(score_branches)
        if (bid := branch_id(row)) is not None
    }
    score_reward_by_id = {
        bid: reward_pred_score[index]
        for index, row in enumerate(score_branches)
        if (bid := branch_id(row)) is not None
    }

    train_pair_examples, x_pair_train_dicts = pair_examples(
        train_pairs,
        branch_by_id=train_branch_by_id,
        branch_hp_pred=train_hp_by_id,
        branch_reward_pred=train_reward_by_id,
    )
    score_pair_examples, x_pair_score_dicts = pair_examples(
        score_pairs,
        branch_by_id=score_branch_by_id,
        branch_hp_pred=score_hp_by_id,
        branch_reward_pred=score_reward_by_id,
    )
    pair_vectorizer = DictVectorizer(sparse=False)
    x_pair_train = pair_vectorizer.fit_transform(x_pair_train_dicts)
    x_pair_score = pair_vectorizer.transform(x_pair_score_dicts)

    hp_residual_model = fit_regressor(
        family.regressor(),
        x_pair_train,
        [item["true_hp"] - item["base_hp"] for item in train_pair_examples],
    )
    reward_residual_model = fit_regressor(
        family.regressor(),
        x_pair_train,
        [item["true_reward"] - item["base_reward"] for item in train_pair_examples],
    )
    hp_residual_pred = [float(v) for v in hp_residual_model.predict(x_pair_score)]
    reward_residual_pred = [float(v) for v in reward_residual_model.predict(x_pair_score)]

    tail_scores: dict[str, list[float]] = {}
    tail_summary: dict[str, Any] = {}
    for offset, (name, label_fn) in enumerate(pair_tail_specs().items()):
        y_train = [int(label_fn(item["true_hp"])) for item in train_pair_examples]
        y_score = [int(label_fn(item["true_hp"])) for item in score_pair_examples]
        prior = sum(y_train) / len(y_train) if y_train else 0.0
        model = fit_classifier(family.classifier(), x_pair_train, y_train)
        scores = predict_classifier(model, x_pair_score, prior)
        tail_scores[name] = scores
        tail_summary[name] = binary_metrics(y_score, scores)

    residual_hp_corrected = [
        item["base_hp"] + hp_residual_pred[index]
        for index, item in enumerate(score_pair_examples)
    ]
    residual_reward_corrected = [
        item["base_reward"] + reward_residual_pred[index]
        for index, item in enumerate(score_pair_examples)
    ]
    pair_predictions = annotate_rows(
        pair_prediction_records(
            score_pairs,
            score_branch_by_id,
            score_hp_by_id,
            score_reward_by_id,
            hp_residual_pred,
            reward_residual_pred,
            tail_scores,
        ),
        source="tree_branch_value_risk_holdout_score_pair_audit",
        train_branches=args.train_branches,
        train_pairs=args.train_pairs,
        score_branches=args.score_branches,
        score_pairs=args.score_pairs,
    )
    write_jsonl(args.pair_prediction_out, pair_predictions)

    hp_true_score = [branch_target(row, "hp_delta") for row in score_branches]
    reward_true_score = [branch_target(row, "total_reward") for row in score_branches]
    pair_hp_true = [item["true_hp"] for item in score_pair_examples]
    pair_reward_true = [item["true_reward"] for item in score_pair_examples]
    pair_hp_base = [item["base_hp"] for item in score_pair_examples]
    pair_reward_base = [item["base_reward"] for item in score_pair_examples]

    train_branch_seeds = episode_seed_set(train_branches)
    score_branch_seeds = episode_seed_set(score_branches)
    train_pair_seeds = episode_seed_set(train_pairs)
    score_pair_seeds = episode_seed_set(score_pairs)
    summary: dict[str, Any] = {
        "schema_version": "tree_branch_value_risk_holdout_score_summary_v0",
        "model_kind": args.model_kind,
        "train_sources": {
            "branches": str(args.train_branches),
            "pairs": str(args.train_pairs),
        },
        "score_sources": {
            "branches": str(args.score_branches),
            "pairs": str(args.score_pairs),
        },
        "no_train_leak": {
            "score_rows_used_for_fit": False,
            "explicit_train_score_split": True,
            "branch_seed_overlap_count": len(train_branch_seeds & score_branch_seeds),
            "pair_seed_overlap_count": len(train_pair_seeds & score_pair_seeds),
        },
        "row_counts": {
            "train_branch_count": len(train_branches),
            "score_branch_count": len(score_branches),
            "train_pair_count": len(train_pairs),
            "score_pair_count": len(score_pairs),
        },
        "feature_counts": {
            "branch_feature_count": len(branch_vectorizer.feature_names_),
            "pair_feature_count": len(pair_vectorizer.feature_names_),
        },
        "models": {
            "hp_delta_regression": regression_metrics(hp_true_score, hp_pred_score),
            "total_reward_regression": regression_metrics(
                reward_true_score, reward_pred_score
            ),
            "risk_classification": risk_summary,
            "pair_hp_diff_from_branch_hp_model": {
                **regression_metrics(pair_hp_true, pair_hp_base),
                "material_sign_metrics": material_sign_metrics(pair_hp_true, pair_hp_base),
            },
            "pair_residual_regression": {
                "hp_diff_base": regression_metrics(pair_hp_true, pair_hp_base),
                "hp_diff_residual_corrected": regression_metrics(
                    pair_hp_true, residual_hp_corrected
                ),
                "reward_diff_base": regression_metrics(pair_reward_true, pair_reward_base),
                "reward_diff_residual_corrected": regression_metrics(
                    pair_reward_true, residual_reward_corrected
                ),
                "material_sign_metrics_base": material_sign_metrics(pair_hp_true, pair_hp_base),
                "material_sign_metrics_residual_corrected": material_sign_metrics(
                    pair_hp_true, residual_hp_corrected
                ),
            },
            "pair_tail_classification": tail_summary,
        },
        "label_safety": {
            "action_policy_trained": False,
            "winner_or_preference_label_used": False,
            "pair_rows_are_ordered_outcome_diffs": True,
            "holdout_score_rows_are_not_action_labels": True,
        },
    }
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return summary


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--train-branches", type=Path, required=True)
    parser.add_argument("--train-pairs", type=Path, required=True)
    parser.add_argument("--score-branches", type=Path, required=True)
    parser.add_argument("--score-pairs", type=Path, required=True)
    parser.add_argument("--model-kind", choices=("hgbdt", "extra_trees"), required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--prediction-out", type=Path, required=True)
    parser.add_argument("--pair-prediction-out", type=Path, required=True)
    parser.add_argument("--seed", type=int, default=17)
    return parser.parse_args()


def main() -> int:
    train_and_score(parse_args())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
