#!/usr/bin/env python3
"""Train tree-model ablations on Branch Value/Risk datasets.

This script is intentionally an ablation harness, not a policy trainer. It
emits the same branch/pair prediction JSONL shape as
train_branch_value_risk_baseline.py so the existing search-allocation gate can
compare model families without changing label semantics.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path
from typing import Any, Iterable

import numpy as np
from sklearn.ensemble import ExtraTreesClassifier, ExtraTreesRegressor
from sklearn.ensemble import HistGradientBoostingClassifier, HistGradientBoostingRegressor
from sklearn.feature_extraction import DictVectorizer
from sklearn.metrics import roc_auc_score

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from train_branch_value_risk_baseline import (  # noqa: E402
    binary_metrics,
    candidate_audit_summary,
    decision_context_audit_summary,
    material_sign_metrics,
    observation_audit_summary,
    regression_metrics,
    split_for_seed,
)


FORBIDDEN_LABEL_KEYS = {
    "winner",
    "preferred",
    "preferred_action",
    "selected_action",
    "teacher_choice",
}


def iter_jsonl(path: Path) -> Iterable[dict[str, Any]]:
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                yield json.loads(line)


def load_rows(path: Path, *, row_kind: str) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for index, row in enumerate(iter_jsonl(path)):
        assert_no_action_label_leak(row, row_kind=row_kind, row_index=index)
        rows.append(row)
    return rows


def assert_no_action_label_leak(row: dict[str, Any], *, row_kind: str, row_index: int) -> None:
    if row.get("trainable_as_action_label") is not False:
        raise ValueError(f"{row_kind} row {row_index} is action-label-like")
    if (row.get("label_policy") or {}).get("action_label") is not False:
        raise ValueError(f"{row_kind} row {row_index} has action_label=true")
    serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
    for key in FORBIDDEN_LABEL_KEYS:
        if f'"{key}"' in serialized:
            raise ValueError(f"{row_kind} row {row_index} contains forbidden key {key}")


def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return default
    return number if math.isfinite(number) else default


def target_value(row: dict[str, Any], key: str) -> float:
    return safe_float((row.get("targets") or {}).get(key))


def split_rows(
    rows: list[dict[str, Any]], train_ratio: float
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    train: list[dict[str, Any]] = []
    test: list[dict[str, Any]] = []
    for row in rows:
        if split_for_seed(row.get("episode_seed"), train_ratio) == "train":
            train.append(row)
        else:
            test.append(row)
    return train, test


def add_value(features: dict[str, Any], prefix: str, key: str, value: Any) -> None:
    name = f"{prefix}{key}"
    if value is None:
        return
    if isinstance(value, bool):
        features[name] = 1.0 if value else 0.0
        features[f"{name}={str(value).lower()}"] = 1.0
        return
    if isinstance(value, (int, float)):
        number = safe_float(value, None)
        if number is not None:
            features[name] = number
        return
    if isinstance(value, str):
        features[f"{name}={value}"] = 1.0


def add_dict(features: dict[str, Any], prefix: str, values: dict[str, Any]) -> None:
    for key, value in values.items():
        if isinstance(value, dict) or isinstance(value, list):
            continue
        add_value(features, prefix, key, value)


def branch_feature_dict(row: dict[str, Any], *, prefix: str = "") -> dict[str, Any]:
    candidate = row.get("candidate") or {}
    obs = row.get("observation_features") or row.get("observation") or {}
    context = row.get("decision_context_features") or row.get("decision_context") or {}
    evidence = row.get("evidence_features") or {}
    features: dict[str, Any] = {}
    add_value(features, prefix, "decision_type", row.get("decision_type") or (row.get("decision_id") or {}).get("decision_type"))
    add_value(features, prefix, "dataset_source_name", row.get("dataset_source_name"))
    add_dict(features, f"{prefix}candidate.", candidate)
    add_dict(features, f"{prefix}obs.", obs)
    add_dict(features, f"{prefix}ctx.", context)
    add_dict(features, f"{prefix}evidence.", evidence)
    # Hand-authored interaction features here describe observation structure, not
    # action preferences. They help tree models see opportunity-cost patterns.
    incoming_gap = safe_float(context.get("incoming_minus_current_block"))
    playable = safe_float(context.get("playable_candidate_count"))
    unique_damage = safe_float(context.get("playable_unique_damage_sum"))
    unique_block = safe_float(context.get("playable_unique_block_sum"))
    is_end_turn = candidate.get("action_kind") == "end_turn"
    is_play_card = candidate.get("action_kind") == "play_card"
    features[f"{prefix}candidate.is_end_turn"] = 1.0 if is_end_turn else 0.0
    features[f"{prefix}candidate.is_play_card"] = 1.0 if is_play_card else 0.0
    features[f"{prefix}opp.incoming_x_playable"] = incoming_gap * playable
    features[f"{prefix}opp.incoming_x_unique_block"] = incoming_gap * unique_block
    features[f"{prefix}opp.incoming_x_unique_damage"] = incoming_gap * unique_damage
    if is_end_turn:
        features[f"{prefix}opp.end_turn_incoming_gap"] = incoming_gap
        features[f"{prefix}opp.end_turn_playable_count"] = playable
        features[f"{prefix}opp.end_turn_unique_damage"] = unique_damage
        features[f"{prefix}opp.end_turn_unique_block"] = unique_block
    return features


def pair_side_to_branch_row(side: dict[str, Any], pair_row: dict[str, Any]) -> dict[str, Any]:
    return {
        "trainable_as_action_label": False,
        "label_policy": {"action_label": False, "source": "pair_side"},
        "episode_seed": pair_row.get("episode_seed"),
        "episode_step": pair_row.get("episode_step"),
        "decision_id": pair_row.get("decision_id"),
        "decision_type": (pair_row.get("decision_id") or {}).get("decision_type"),
        "branch_id": side.get("branch_id"),
        "candidate": side.get("candidate") or {},
        "observation_features": {},
        "decision_context_features": side.get("decision_context_features") or {},
        "evidence_features": side.get("evidence_features") or {},
        "targets": side.get("targets") or {},
    }


def branch_target(row: dict[str, Any], key: str) -> float:
    return safe_float((row.get("targets") or {}).get(key))


def pair_true_diff(row: dict[str, Any], key: str) -> float:
    return safe_float((row.get("outcome_diff") or row.get("targets") or {}).get(key))


class TreeModelFamily:
    def __init__(self, kind: str, random_state: int) -> None:
        self.kind = kind
        self.random_state = random_state

    def regressor(self):
        if self.kind == "extra_trees":
            return ExtraTreesRegressor(
                n_estimators=300,
                max_features=0.8,
                min_samples_leaf=2,
                random_state=self.random_state,
                n_jobs=-1,
            )
        if self.kind == "hgbdt":
            return HistGradientBoostingRegressor(
                max_iter=300,
                learning_rate=0.045,
                l2_regularization=0.01,
                max_leaf_nodes=31,
                random_state=self.random_state,
            )
        raise ValueError(f"unsupported model kind {self.kind}")

    def classifier(self):
        if self.kind == "extra_trees":
            return ExtraTreesClassifier(
                n_estimators=300,
                max_features=0.8,
                min_samples_leaf=2,
                random_state=self.random_state,
                n_jobs=-1,
                class_weight="balanced",
            )
        if self.kind == "hgbdt":
            return HistGradientBoostingClassifier(
                max_iter=300,
                learning_rate=0.045,
                l2_regularization=0.01,
                max_leaf_nodes=31,
                random_state=self.random_state,
            )
        raise ValueError(f"unsupported model kind {self.kind}")


def fit_regressor(model, x_train, y_train):
    model.fit(x_train, np.asarray(y_train, dtype=float))
    return model


def fit_classifier(model, x_train, y_train):
    labels = np.asarray(y_train, dtype=int)
    if labels.sum() == 0 or labels.sum() == len(labels):
        return None
    model.fit(x_train, labels)
    return model


def predict_classifier(model, x, default_prob: float) -> list[float]:
    if model is None:
        return [default_prob for _ in range(x.shape[0])]
    probabilities = model.predict_proba(x)
    if probabilities.shape[1] == 1:
        cls = int(model.classes_[0])
        return [1.0 if cls == 1 else 0.0 for _ in range(x.shape[0])]
    class_to_index = {int(cls): i for i, cls in enumerate(model.classes_)}
    positive_index = class_to_index.get(1)
    if positive_index is None:
        return [0.0 for _ in range(x.shape[0])]
    return [float(value) for value in probabilities[:, positive_index]]


def branch_prediction_records(
    rows: list[dict[str, Any]],
    hp_pred: list[float],
    reward_pred: list[float],
    risk_scores: dict[str, list[float]],
) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for index, row in enumerate(rows):
        hp_true = branch_target(row, "hp_delta")
        reward_true = branch_target(row, "total_reward")
        risks: dict[str, Any] = {}
        for name, scores in risk_scores.items():
            if name == "hp_loss_ge_5":
                label = int(hp_true <= -5)
            elif name == "hp_loss_ge_10":
                label = int(hp_true <= -10)
            else:
                label = None
            risks[name] = {
                "label": label,
                "probability": scores[index] if index < len(scores) else None,
                "model_skipped": False,
            }
        out.append(
            {
                "schema_version": "branch_value_risk_prediction_v0",
                "trainable_role": "branch_value_risk_audit",
                "trainable_as_action_label": False,
                "episode_seed": row.get("episode_seed"),
                "episode_step": row.get("episode_step"),
                "decision_id": row.get("decision_id"),
                "branch_id": row.get("branch_id"),
                "state_hash_before": row.get("state_hash_before"),
                "scenario_seed_id": row.get("scenario_seed_id"),
                "candidate": candidate_audit_summary(row),
                "observation": observation_audit_summary(row),
                "decision_context": decision_context_audit_summary(row),
                "targets": {"hp_delta": hp_true, "total_reward": reward_true},
                "model_outputs": {
                    "hp_delta": hp_pred[index],
                    "total_reward": reward_pred[index],
                    "risks": risks,
                },
                "errors": {
                    "hp_delta_abs": abs(hp_pred[index] - hp_true),
                    "total_reward_abs": abs(reward_pred[index] - reward_true),
                },
                "label_policy": {
                    "action_label": False,
                    "source": "tree_branch_value_risk_ablation",
                },
            }
        )
    return out


def pair_tail_specs():
    return {
        "abs_hp_diff_ge_5": lambda value: abs(value) >= 5,
        "abs_hp_diff_ge_10": lambda value: abs(value) >= 10,
        "abs_hp_diff_ge_15": lambda value: abs(value) >= 15,
        "left_worse_ge_5": lambda value: value <= -5,
        "left_worse_ge_10": lambda value: value <= -10,
        "left_worse_ge_15": lambda value: value <= -15,
        "left_better_ge_5": lambda value: value >= 5,
        "left_better_ge_10": lambda value: value >= 10,
        "left_better_ge_15": lambda value: value >= 15,
    }


def pair_feature_dict(
    row: dict[str, Any],
    left: dict[str, Any],
    right: dict[str, Any],
    *,
    base_hp_diff: float,
    base_reward_diff: float,
) -> dict[str, Any]:
    features: dict[str, Any] = {}
    for key, value in branch_feature_dict(left, prefix="left.").items():
        features[key] = value
    for key, value in branch_feature_dict(right, prefix="right.").items():
        features[key] = value
    left_candidate = left.get("candidate") or {}
    right_candidate = right.get("candidate") or {}
    add_value(features, "pair.", "kind", f"{left_candidate.get('action_kind')}->{right_candidate.get('action_kind')}")
    add_value(features, "pair.", "card", f"{left_candidate.get('card_id')}->{right_candidate.get('card_id')}")
    add_value(features, "pair.", "same_action_kind", left_candidate.get("action_kind") == right_candidate.get("action_kind"))
    add_value(features, "pair.", "same_card_id", left_candidate.get("card_id") == right_candidate.get("card_id"))
    add_value(features, "pair.", "any_end_turn", left_candidate.get("action_kind") == "end_turn" or right_candidate.get("action_kind") == "end_turn")
    add_value(features, "pair.", "end_turn_vs_play_card", {left_candidate.get("action_kind"), right_candidate.get("action_kind")} == {"end_turn", "play_card"})
    add_value(features, "pair.", "rng_diverged", (row.get("pairing") or {}).get("rng_diverged"))
    features["pair.base_hp_diff"] = base_hp_diff
    features["pair.base_reward_diff"] = base_reward_diff
    features["pair.abs_base_hp_diff"] = abs(base_hp_diff)
    features["pair.left_damage_minus_right"] = safe_float(left_candidate.get("card_base_damage")) - safe_float(right_candidate.get("card_base_damage"))
    features["pair.left_block_minus_right"] = safe_float(left_candidate.get("card_base_block")) - safe_float(right_candidate.get("card_base_block"))
    return features


def pair_prediction_records(
    rows: list[dict[str, Any]],
    branch_by_id: dict[str, dict[str, Any]],
    branch_hp_pred: dict[str, float],
    branch_reward_pred: dict[str, float],
    residual_hp_pred: list[float],
    residual_reward_pred: list[float],
    tail_scores: dict[str, list[float]],
) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for index, row in enumerate(rows):
        left_side = row.get("left") or {}
        right_side = row.get("right") or {}
        left = branch_by_id.get(left_side.get("branch_id")) or pair_side_to_branch_row(left_side, row)
        right = branch_by_id.get(right_side.get("branch_id")) or pair_side_to_branch_row(right_side, row)
        left_id = left_side.get("branch_id")
        right_id = right_side.get("branch_id")
        pred_hp = branch_hp_pred.get(left_id, 0.0) - branch_hp_pred.get(right_id, 0.0)
        pred_reward = branch_reward_pred.get(left_id, 0.0) - branch_reward_pred.get(right_id, 0.0)
        residual_hp = pred_hp + residual_hp_pred[index]
        residual_reward = pred_reward + residual_reward_pred[index]
        true_hp = pair_true_diff(row, "hp_left_minus_right")
        true_reward = pair_true_diff(row, "total_reward_left_minus_right")
        tail_probabilities = {
            name: (scores[index] if index < len(scores) else None)
            for name, scores in tail_scores.items()
        }
        out.append(
            {
                "schema_version": "branch_pair_outcome_diff_prediction_v0",
                "trainable_role": "branch_pair_outcome_diff_audit",
                "trainable_as_action_label": False,
                "episode_seed": row.get("episode_seed"),
                "episode_step": row.get("episode_step"),
                "decision_id": row.get("decision_id"),
                "comparison_id": row.get("comparison_id"),
                "pairing": row.get("pairing"),
                "left": {
                    "branch_id": left_side.get("branch_id"),
                    "candidate": candidate_audit_summary(left),
                    "decision_context": decision_context_audit_summary(left),
                },
                "right": {
                    "branch_id": right_side.get("branch_id"),
                    "candidate": candidate_audit_summary(right),
                    "decision_context": decision_context_audit_summary(right),
                },
                "observation": observation_audit_summary(left),
                "targets": {
                    "hp_left_minus_right": true_hp,
                    "total_reward_left_minus_right": true_reward,
                },
                "model_outputs": {
                    "branch_model_hp_left_minus_right": pred_hp,
                    "branch_model_total_reward_left_minus_right": pred_reward,
                    "residual_corrected_hp_left_minus_right": residual_hp,
                    "residual_corrected_total_reward_left_minus_right": residual_reward,
                    "tail_probabilities": tail_probabilities,
                },
                "errors": {
                    "branch_model_hp_left_minus_right_abs": abs(pred_hp - true_hp),
                    "branch_model_total_reward_left_minus_right_abs": abs(pred_reward - true_reward),
                    "residual_corrected_hp_left_minus_right_abs": abs(residual_hp - true_hp),
                    "residual_corrected_total_reward_left_minus_right_abs": abs(residual_reward - true_reward),
                    "branch_model_hp_diff_nonzero_sign_correct": (
                        None
                        if true_hp == 0
                        else ((true_hp > 0 and pred_hp > 0) or (true_hp < 0 and pred_hp < 0))
                    ),
                    "residual_corrected_hp_diff_nonzero_sign_correct": (
                        None
                        if true_hp == 0
                        else (
                            (true_hp > 0 and residual_hp > 0)
                            or (true_hp < 0 and residual_hp < 0)
                        )
                    ),
                    "branch_model_severe_underestimate_abs_ge_10_pred_abs_lt_5": (
                        abs(true_hp) >= 10 and abs(pred_hp) < 5
                    ),
                    "residual_corrected_severe_underestimate_abs_ge_10_pred_abs_lt_5": (
                        abs(true_hp) >= 10 and abs(residual_hp) < 5
                    ),
                },
                "search_allocation_signals": {
                    "branch_model_abs_hp_diff": abs(pred_hp),
                    "residual_corrected_abs_hp_diff": abs(residual_hp),
                    "tail_abs_hp_diff_ge_10_probability": tail_probabilities.get("abs_hp_diff_ge_10"),
                    "tail_left_worse_ge_10_probability": tail_probabilities.get("left_worse_ge_10"),
                    "tail_left_better_ge_10_probability": tail_probabilities.get("left_better_ge_10"),
                },
                "label_policy": {
                    "action_label": False,
                    "source": "tree_branch_outcome_model_pair_audit",
                },
            }
        )
    return out


def write_jsonl(path: Path, rows: Iterable[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


def train_and_predict(args: argparse.Namespace) -> dict[str, Any]:
    branches = load_rows(args.branches, row_kind="branch")
    pairs = load_rows(args.pairs, row_kind="pair")
    train_branches, test_branches = split_rows(branches, args.train_ratio)
    train_pairs, test_pairs = split_rows(pairs, args.train_ratio)
    family = TreeModelFamily(args.model_kind, args.seed)

    branch_vectorizer = DictVectorizer(sparse=False)
    x_branch_train = branch_vectorizer.fit_transform([branch_feature_dict(row) for row in train_branches])
    x_branch_test = branch_vectorizer.transform([branch_feature_dict(row) for row in test_branches])
    x_branch_all = branch_vectorizer.transform([branch_feature_dict(row) for row in branches])

    y_hp_train = [branch_target(row, "hp_delta") for row in train_branches]
    y_reward_train = [branch_target(row, "total_reward") for row in train_branches]
    hp_model = fit_regressor(family.regressor(), x_branch_train, y_hp_train)
    reward_model = fit_regressor(family.regressor(), x_branch_train, y_reward_train)

    hp_pred_test = [float(v) for v in hp_model.predict(x_branch_test)]
    reward_pred_test = [float(v) for v in reward_model.predict(x_branch_test)]
    hp_pred_all = [float(v) for v in hp_model.predict(x_branch_all)]
    reward_pred_all = [float(v) for v in reward_model.predict(x_branch_all)]
    branch_hp_by_id = {
        row.get("branch_id"): hp_pred_all[index] for index, row in enumerate(branches)
    }
    branch_reward_by_id = {
        row.get("branch_id"): reward_pred_all[index] for index, row in enumerate(branches)
    }

    risk_scores_test: dict[str, list[float]] = {}
    risk_summary: dict[str, Any] = {}
    for name, threshold in (("hp_loss_ge_5", -5), ("hp_loss_ge_10", -10)):
        labels_train = [int(branch_target(row, "hp_delta") <= threshold) for row in train_branches]
        labels_test = [int(branch_target(row, "hp_delta") <= threshold) for row in test_branches]
        prior = sum(labels_train) / len(labels_train) if labels_train else 0.0
        model = fit_classifier(family.classifier(), x_branch_train, labels_train)
        scores = predict_classifier(model, x_branch_test, prior)
        risk_scores_test[name] = scores
        risk_summary[name] = binary_metrics(labels_test, scores)

    branch_predictions = branch_prediction_records(
        test_branches, hp_pred_test, reward_pred_test, risk_scores_test
    )
    if args.prediction_out:
        write_jsonl(args.prediction_out, branch_predictions)

    branch_by_id = {row.get("branch_id"): row for row in branches}

    def pair_examples(pair_rows: list[dict[str, Any]]) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
        examples: list[dict[str, Any]] = []
        feature_rows: list[dict[str, Any]] = []
        for row in pair_rows:
            left_side = row.get("left") or {}
            right_side = row.get("right") or {}
            left = branch_by_id.get(left_side.get("branch_id")) or pair_side_to_branch_row(left_side, row)
            right = branch_by_id.get(right_side.get("branch_id")) or pair_side_to_branch_row(right_side, row)
            left_id = left_side.get("branch_id")
            right_id = right_side.get("branch_id")
            base_hp = branch_hp_by_id.get(left_id, 0.0) - branch_hp_by_id.get(right_id, 0.0)
            base_reward = branch_reward_by_id.get(left_id, 0.0) - branch_reward_by_id.get(right_id, 0.0)
            features = pair_feature_dict(row, left, right, base_hp_diff=base_hp, base_reward_diff=base_reward)
            feature_rows.append(features)
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

    train_pair_examples, x_pair_train_dicts = pair_examples(train_pairs)
    test_pair_examples, x_pair_test_dicts = pair_examples(test_pairs)
    pair_vectorizer = DictVectorizer(sparse=False)
    x_pair_train = pair_vectorizer.fit_transform(x_pair_train_dicts)
    x_pair_test = pair_vectorizer.transform(x_pair_test_dicts)

    y_hp_residual_train = [item["true_hp"] - item["base_hp"] for item in train_pair_examples]
    y_reward_residual_train = [
        item["true_reward"] - item["base_reward"] for item in train_pair_examples
    ]
    hp_residual_model = fit_regressor(family.regressor(), x_pair_train, y_hp_residual_train)
    reward_residual_model = fit_regressor(
        family.regressor(), x_pair_train, y_reward_residual_train
    )
    hp_residual_pred = [float(v) for v in hp_residual_model.predict(x_pair_test)]
    reward_residual_pred = [float(v) for v in reward_residual_model.predict(x_pair_test)]

    tail_scores: dict[str, list[float]] = {}
    tail_summary: dict[str, Any] = {}
    specs = pair_tail_specs()
    for name, label_fn in specs.items():
        y_train = [int(label_fn(item["true_hp"])) for item in train_pair_examples]
        y_test = [int(label_fn(item["true_hp"])) for item in test_pair_examples]
        prior = sum(y_train) / len(y_train) if y_train else 0.0
        model = fit_classifier(family.classifier(), x_pair_train, y_train)
        scores = predict_classifier(model, x_pair_test, prior)
        tail_scores[name] = scores
        tail_summary[name] = binary_metrics(y_test, scores)

    residual_hp_corrected = [
        item["base_hp"] + hp_residual_pred[index]
        for index, item in enumerate(test_pair_examples)
    ]
    residual_reward_corrected = [
        item["base_reward"] + reward_residual_pred[index]
        for index, item in enumerate(test_pair_examples)
    ]
    pair_predictions = pair_prediction_records(
        test_pairs,
        branch_by_id,
        branch_hp_by_id,
        branch_reward_by_id,
        hp_residual_pred,
        reward_residual_pred,
        tail_scores,
    )
    if args.pair_prediction_out:
        write_jsonl(args.pair_prediction_out, pair_predictions)

    hp_true_test = [branch_target(row, "hp_delta") for row in test_branches]
    reward_true_test = [branch_target(row, "total_reward") for row in test_branches]
    pair_hp_true = [item["true_hp"] for item in test_pair_examples]
    pair_reward_true = [item["true_reward"] for item in test_pair_examples]
    pair_hp_base = [item["base_hp"] for item in test_pair_examples]
    pair_reward_base = [item["base_reward"] for item in test_pair_examples]

    summary: dict[str, Any] = {
        "schema_version": "tree_branch_value_risk_ablation_summary_v0",
        "model_kind": args.model_kind,
        "branches": str(args.branches),
        "pairs": str(args.pairs),
        "train_ratio": args.train_ratio,
        "train_branch_count": len(train_branches),
        "test_branch_count": len(test_branches),
        "train_pair_count": len(train_pairs),
        "test_pair_count": len(test_pairs),
        "branch_feature_count": len(branch_vectorizer.feature_names_),
        "pair_feature_count": len(pair_vectorizer.feature_names_),
        "models": {
            "hp_delta_regression": regression_metrics(hp_true_test, hp_pred_test),
            "total_reward_regression": regression_metrics(reward_true_test, reward_pred_test),
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
        },
    }
    return summary


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--branches", type=Path, required=True)
    parser.add_argument("--pairs", type=Path, required=True)
    parser.add_argument("--model-kind", choices=("hgbdt", "extra_trees"), required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--prediction-out", type=Path, required=True)
    parser.add_argument("--pair-prediction-out", type=Path, required=True)
    parser.add_argument("--train-ratio", type=float, default=0.8)
    parser.add_argument("--seed", type=int, default=17)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    summary = train_and_predict(args)
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
