#!/usr/bin/env python3
"""Train explicit search-allocation classifiers over prediction artifacts.

This is not a policy trainer. It learns audit-only priority scores for branch
and pair rows that should receive deeper search budget. Targets are branch/pair
outcome buckets, not action labels, winners, preferences, or teacher choices.
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path
from typing import Any, Callable, Iterable

import numpy as np
from sklearn.ensemble import ExtraTreesClassifier, HistGradientBoostingClassifier
from sklearn.feature_extraction import DictVectorizer

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from train_branch_value_risk_baseline import binary_metrics, split_for_seed  # noqa: E402


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


def assert_no_action_label_leak(row: dict[str, Any], *, row_kind: str, row_index: int) -> None:
    if row.get("trainable_as_action_label") is not False:
        raise ValueError(f"{row_kind} row {row_index} is action-label-like")
    if (row.get("label_policy") or {}).get("action_label") is not False:
        raise ValueError(f"{row_kind} row {row_index} has action_label=true")
    serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
    for key in FORBIDDEN_LABEL_KEYS:
        if f'"{key}"' in serialized:
            raise ValueError(f"{row_kind} row {row_index} contains forbidden key {key}")


def load_rows(path: Path, row_kind: str) -> list[dict[str, Any]]:
    rows = list(iter_jsonl(path))
    for index, row in enumerate(rows):
        assert_no_action_label_leak(row, row_kind=row_kind, row_index=index)
    return rows


def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return default
    return number if math.isfinite(number) else default


def safe_prob(value: Any) -> float:
    return max(0.0, min(1.0, safe_float(value, 0.0)))


def add_scalar(features: dict[str, float], name: str, value: Any) -> None:
    if value is None:
        features[f"{name}=<none>"] = 1.0
    elif isinstance(value, bool):
        features[name] = 1.0 if value else 0.0
        features[f"{name}={str(value).lower()}"] = 1.0
    elif isinstance(value, (int, float)):
        number = safe_float(value)
        features[name] = number
        if abs(number) <= 1000.0:
            features[f"{name}_bucket={int(number // 5) * 5}"] = 1.0
    elif isinstance(value, str):
        features[f"{name}={value}"] = 1.0


def add_public_dict(features: dict[str, float], prefix: str, values: Any) -> None:
    if not isinstance(values, dict):
        return
    for key, value in values.items():
        if isinstance(value, dict):
            for inner_key, inner_value in value.items():
                if isinstance(inner_value, (dict, list)):
                    continue
                add_scalar(features, f"{prefix}.{key}.{inner_key}", inner_value)
        elif not isinstance(value, list):
            add_scalar(features, f"{prefix}.{key}", value)


def branch_features(row: dict[str, Any]) -> dict[str, float]:
    outputs = row.get("model_outputs") or {}
    risks = outputs.get("risks") or {}
    signals = row.get("search_allocation_signals") or {}
    candidate = row.get("candidate") or {}
    context = row.get("decision_context") or {}
    features: dict[str, float] = {"bias": 1.0}
    add_public_dict(features, "candidate", candidate)
    add_public_dict(features, "observation", row.get("observation") or {})
    add_public_dict(features, "decision_context", context)
    add_scalar(features, "model.hp_delta", outputs.get("hp_delta"))
    add_scalar(features, "model.total_reward", outputs.get("total_reward"))
    for risk_name, risk_payload in risks.items():
        add_scalar(features, f"risk.{risk_name}.probability", risk_payload.get("probability"))
        add_scalar(features, f"risk.{risk_name}.model_skipped", risk_payload.get("model_skipped"))
    add_public_dict(features, "existing_signal", signals)
    if candidate.get("action_kind") == "end_turn":
        add_scalar(features, "opp.end_turn_with_playable_cards", context.get("end_turn_with_playable_cards"))
        add_scalar(features, "opp.end_turn_with_unspent_energy", context.get("end_turn_with_unspent_energy"))
        add_scalar(features, "opp.incoming_minus_current_block", context.get("incoming_minus_current_block"))
        add_scalar(features, "opp.playable_unique_damage_sum", context.get("playable_unique_damage_sum"))
        add_scalar(features, "opp.playable_unique_block_sum", context.get("playable_unique_block_sum"))
    return features


def pair_features(row: dict[str, Any]) -> dict[str, float]:
    outputs = row.get("model_outputs") or {}
    tails = outputs.get("tail_probabilities") or {}
    signals = row.get("search_allocation_signals") or {}
    left = row.get("left") or {}
    right = row.get("right") or {}
    left_candidate = left.get("candidate") or {}
    right_candidate = right.get("candidate") or {}
    features: dict[str, float] = {"bias": 1.0}
    add_public_dict(features, "left.candidate", left_candidate)
    add_public_dict(features, "left.decision_context", left.get("decision_context") or {})
    add_public_dict(features, "right.candidate", right_candidate)
    add_public_dict(features, "right.decision_context", right.get("decision_context") or {})
    add_public_dict(features, "observation", row.get("observation") or {})
    add_public_dict(features, "pairing", row.get("pairing") or {})
    add_public_dict(features, "existing_signal", signals)
    add_scalar(features, "model.branch_hp_diff", outputs.get("branch_model_hp_left_minus_right"))
    add_scalar(
        features,
        "model.residual_hp_diff",
        outputs.get("residual_corrected_hp_left_minus_right"),
    )
    add_scalar(
        features,
        "model.branch_reward_diff",
        outputs.get("branch_model_total_reward_left_minus_right"),
    )
    add_scalar(
        features,
        "model.residual_reward_diff",
        outputs.get("residual_corrected_total_reward_left_minus_right"),
    )
    for tail_name, probability in tails.items():
        add_scalar(features, f"tail.{tail_name}", probability)
    left_kind = left_candidate.get("action_kind")
    right_kind = right_candidate.get("action_kind")
    add_scalar(features, "pair.kind", f"{left_kind}->{right_kind}")
    add_scalar(features, "pair.card", f"{left_candidate.get('card_id')}->{right_candidate.get('card_id')}")
    add_scalar(features, "pair.same_action_kind", left_kind == right_kind)
    add_scalar(features, "pair.end_turn_vs_play_card", {left_kind, right_kind} == {"end_turn", "play_card"})
    add_scalar(
        features,
        "pair.abs_branch_minus_residual",
        abs(safe_float(outputs.get("residual_corrected_hp_left_minus_right")) - safe_float(outputs.get("branch_model_hp_left_minus_right"))),
    )
    return features


class ConstantClassifier:
    def __init__(self, probability: float) -> None:
        self.probability = float(probability)

    def predict_proba(self, x: Any) -> np.ndarray:
        return np.array([[1.0 - self.probability, self.probability]] * x.shape[0])


def make_classifier(model_kind: str, seed: int):
    if model_kind == "extra_trees":
        return ExtraTreesClassifier(
            n_estimators=400,
            max_features=0.85,
            min_samples_leaf=2,
            random_state=seed,
            n_jobs=-1,
            class_weight="balanced",
        )
    if model_kind == "hgbdt":
        return HistGradientBoostingClassifier(
            max_iter=260,
            learning_rate=0.045,
            l2_regularization=0.02,
            max_leaf_nodes=31,
            random_state=seed,
        )
    raise ValueError(f"unsupported model kind {model_kind}")


def fit_classifier(model_kind: str, x_train: np.ndarray, y_train: list[int], seed: int):
    positives = sum(y_train)
    if positives == 0 or positives == len(y_train):
        return ConstantClassifier(positives / len(y_train) if y_train else 0.0)
    model = make_classifier(model_kind, seed)
    model.fit(x_train, np.asarray(y_train, dtype=int))
    return model


def predict_proba(model: Any, x: np.ndarray) -> list[float]:
    probabilities = model.predict_proba(x)
    if probabilities.shape[1] == 1:
        cls = int(getattr(model, "classes_", [0])[0])
        return [1.0 if cls == 1 else 0.0 for _ in range(x.shape[0])]
    classes = {int(cls): index for index, cls in enumerate(getattr(model, "classes_", [0, 1]))}
    positive_index = classes.get(1, 1)
    return [float(value) for value in probabilities[:, positive_index]]


def split_internal(rows: list[dict[str, Any]], train_ratio: float) -> tuple[list[int], list[int]]:
    train: list[int] = []
    test: list[int] = []
    for index, row in enumerate(rows):
        if split_for_seed(row.get("episode_seed"), train_ratio) == "train":
            train.append(index)
        else:
            test.append(index)
    return train, test


def branch_label(row: dict[str, Any], name: str) -> int:
    hp_delta = safe_float((row.get("targets") or {}).get("hp_delta"))
    if name == "hp_loss_ge_5":
        return int(hp_delta <= -5)
    if name == "hp_loss_ge_10":
        return int(hp_delta <= -10)
    if name == "branch_priority":
        return int(hp_delta <= -10)
    raise KeyError(name)


def pair_hp_diff(row: dict[str, Any]) -> float:
    return safe_float((row.get("targets") or {}).get("hp_left_minus_right"))


def pair_is_end_turn_play_card(row: dict[str, Any]) -> bool:
    left_kind = (((row.get("left") or {}).get("candidate") or {}).get("action_kind"))
    right_kind = (((row.get("right") or {}).get("candidate") or {}).get("action_kind"))
    return {left_kind, right_kind} == {"end_turn", "play_card"}


def pair_label(row: dict[str, Any], name: str) -> int:
    hp_diff = pair_hp_diff(row)
    errors = row.get("errors") or {}
    if name == "abs_hp_diff_ge_10":
        return int(abs(hp_diff) >= 10)
    if name == "abs_hp_diff_ge_15":
        return int(abs(hp_diff) >= 15)
    if name == "branch_model_severe_underestimate_ge_10":
        return int(bool(errors.get("branch_model_severe_underestimate_abs_ge_10_pred_abs_lt_5")))
    if name == "residual_model_severe_underestimate_ge_10":
        return int(bool(errors.get("residual_corrected_severe_underestimate_abs_ge_10_pred_abs_lt_5")))
    if name == "end_turn_play_card_abs_hp_diff_ge_10":
        return int(abs(hp_diff) >= 10 and pair_is_end_turn_play_card(row))
    if name == "pair_required":
        return int(abs(hp_diff) >= 10 or (abs(hp_diff) >= 10 and pair_is_end_turn_play_card(row)))
    if name == "pair_watch":
        return int(bool(errors.get("branch_model_severe_underestimate_abs_ge_10_pred_abs_lt_5")))
    if name == "pair_priority":
        return int(
            abs(hp_diff) >= 10
            or bool(errors.get("branch_model_severe_underestimate_abs_ge_10_pred_abs_lt_5"))
        )
    raise KeyError(name)


def train_allocation_models(
    *,
    train_rows: list[dict[str, Any]],
    score_rows: dict[str, list[dict[str, Any]]],
    feature_fn: Callable[[dict[str, Any]], dict[str, float]],
    label_fn: Callable[[dict[str, Any], str], int],
    target_names: list[str],
    model_kind: str,
    seed: int,
) -> tuple[dict[str, dict[str, list[float]]], dict[str, Any]]:
    train_features = [feature_fn(row) for row in train_rows]
    vectorizer = DictVectorizer(sparse=False)
    x_train = vectorizer.fit_transform(train_features).astype(np.float32)

    out_scores: dict[str, dict[str, list[float]]] = {
        name: {} for name in score_rows
    }
    metrics: dict[str, Any] = {
        "feature_count": len(vectorizer.feature_names_),
        "target_fit_diagnostics": {},
        "train_count": len(train_rows),
        "fit_diagnostics_are_in_sample": True,
        "fit_diagnostics_note": (
            "Generalization is evaluated by the downstream hard-slice gate; "
            "these metrics only check whether the allocation target is learnable "
            "on the baseline training source."
        ),
    }
    for target_index, target_name in enumerate(target_names):
        labels = [label_fn(row, target_name) for row in train_rows]
        model = fit_classifier(model_kind, x_train, labels, seed + target_index)
        fit_scores = predict_proba(model, x_train)
        metrics["target_fit_diagnostics"][target_name] = binary_metrics(labels, fit_scores)
        for slice_name, rows in score_rows.items():
            x_score = vectorizer.transform([feature_fn(row) for row in rows]).astype(np.float32)
            out_scores[slice_name][target_name] = predict_proba(model, x_score)
    return out_scores, metrics


def with_branch_scores(rows: list[dict[str, Any]], scores: dict[str, list[float]]) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for index, row in enumerate(rows):
        next_row = dict(row)
        signals = dict(next_row.get("search_allocation_signals") or {})
        hp_loss5 = scores["hp_loss_ge_5"][index]
        hp_loss10 = scores["hp_loss_ge_10"][index]
        priority = max(hp_loss10, scores["branch_priority"][index])
        signals.update(
            {
                "allocation_hp_loss_ge_5_probability": hp_loss5,
                "allocation_hp_loss_ge_10_probability": hp_loss10,
                "allocation_branch_priority": priority,
            }
        )
        next_row["search_allocation_signals"] = signals
        next_row["allocation_model_outputs"] = {
            "schema_version": "search_allocation_branch_model_v0",
            "hp_loss_ge_5_probability": hp_loss5,
            "hp_loss_ge_10_probability": hp_loss10,
            "branch_priority": priority,
        }
        next_row["label_policy"] = {
            **(next_row.get("label_policy") or {}),
            "action_label": False,
            "allocation_model_source": "search_allocation_model_v0",
        }
        out.append(next_row)
    return out


def with_pair_scores(rows: list[dict[str, Any]], scores: dict[str, list[float]]) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for index, row in enumerate(rows):
        next_row = dict(row)
        signals = dict(next_row.get("search_allocation_signals") or {})
        abs10 = scores["abs_hp_diff_ge_10"][index]
        abs15 = scores["abs_hp_diff_ge_15"][index]
        branch_severe = scores["branch_model_severe_underestimate_ge_10"][index]
        residual_severe = scores["residual_model_severe_underestimate_ge_10"][index]
        end_turn_pair = scores["end_turn_play_card_abs_hp_diff_ge_10"][index]
        required = max(abs10, abs15, end_turn_pair, scores["pair_required"][index])
        watch = max(branch_severe, residual_severe, scores["pair_watch"][index])
        priority = max(required, watch, scores["pair_priority"][index])
        signals.update(
            {
                "allocation_abs_hp_diff_ge_10_probability": abs10,
                "allocation_abs_hp_diff_ge_15_probability": abs15,
                "allocation_branch_model_severe_underestimate_ge_10_probability": branch_severe,
                "allocation_residual_model_severe_underestimate_ge_10_probability": residual_severe,
                "allocation_end_turn_play_card_abs_hp_diff_ge_10_probability": end_turn_pair,
                "allocation_pair_required_probability": required,
                "allocation_pair_watch_probability": watch,
                "allocation_pair_priority": priority,
            }
        )
        next_row["search_allocation_signals"] = signals
        next_row["allocation_model_outputs"] = {
            "schema_version": "search_allocation_pair_model_v0",
            "abs_hp_diff_ge_10_probability": abs10,
            "abs_hp_diff_ge_15_probability": abs15,
            "branch_model_severe_underestimate_ge_10_probability": branch_severe,
            "residual_model_severe_underestimate_ge_10_probability": residual_severe,
            "end_turn_play_card_abs_hp_diff_ge_10_probability": end_turn_pair,
            "pair_required_probability": required,
            "pair_watch_probability": watch,
            "pair_priority": priority,
        }
        next_row["label_policy"] = {
            **(next_row.get("label_policy") or {}),
            "action_label": False,
            "allocation_model_source": "search_allocation_model_v0",
        }
        out.append(next_row)
    return out


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline-branch-predictions", type=Path, required=True)
    parser.add_argument("--baseline-pair-predictions", type=Path, required=True)
    parser.add_argument("--hard-branch-predictions", type=Path, required=True)
    parser.add_argument("--hard-pair-predictions", type=Path, required=True)
    parser.add_argument("--model-kind", choices=("hgbdt", "extra_trees"), default="extra_trees")
    parser.add_argument("--baseline-branch-out", type=Path, required=True)
    parser.add_argument("--baseline-pair-out", type=Path, required=True)
    parser.add_argument("--hard-branch-out", type=Path, required=True)
    parser.add_argument("--hard-pair-out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--seed", type=int, default=123)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    baseline_branch = load_rows(args.baseline_branch_predictions, "baseline_branch_prediction")
    baseline_pair = load_rows(args.baseline_pair_predictions, "baseline_pair_prediction")
    hard_branch = load_rows(args.hard_branch_predictions, "hard_branch_prediction")
    hard_pair = load_rows(args.hard_pair_predictions, "hard_pair_prediction")

    branch_targets = ["hp_loss_ge_5", "hp_loss_ge_10", "branch_priority"]
    branch_scores, branch_metrics = train_allocation_models(
        train_rows=baseline_branch,
        score_rows={"baseline": baseline_branch, "hard": hard_branch},
        feature_fn=branch_features,
        label_fn=branch_label,
        target_names=branch_targets,
        model_kind=args.model_kind,
        seed=args.seed,
    )
    pair_targets = [
        "abs_hp_diff_ge_10",
        "abs_hp_diff_ge_15",
        "branch_model_severe_underestimate_ge_10",
        "residual_model_severe_underestimate_ge_10",
        "end_turn_play_card_abs_hp_diff_ge_10",
        "pair_required",
        "pair_watch",
        "pair_priority",
    ]
    pair_scores, pair_metrics = train_allocation_models(
        train_rows=baseline_pair,
        score_rows={"baseline": baseline_pair, "hard": hard_pair},
        feature_fn=pair_features,
        label_fn=pair_label,
        target_names=pair_targets,
        model_kind=args.model_kind,
        seed=args.seed + 1000,
    )

    baseline_branch_aug = with_branch_scores(baseline_branch, branch_scores["baseline"])
    hard_branch_aug = with_branch_scores(hard_branch, branch_scores["hard"])
    baseline_pair_aug = with_pair_scores(baseline_pair, pair_scores["baseline"])
    hard_pair_aug = with_pair_scores(hard_pair, pair_scores["hard"])

    write_jsonl(args.baseline_branch_out, baseline_branch_aug)
    write_jsonl(args.hard_branch_out, hard_branch_aug)
    write_jsonl(args.baseline_pair_out, baseline_pair_aug)
    write_jsonl(args.hard_pair_out, hard_pair_aug)

    summary = {
        "schema_version": "search_allocation_model_v0_summary",
        "model_kind": args.model_kind,
        "train_source": {
            "baseline_branch_predictions": str(args.baseline_branch_predictions),
            "baseline_pair_predictions": str(args.baseline_pair_predictions),
            "hard_branch_predictions": str(args.hard_branch_predictions),
            "hard_pair_predictions": str(args.hard_pair_predictions),
        },
        "row_counts": {
            "baseline_branch": len(baseline_branch),
            "baseline_pair": len(baseline_pair),
            "hard_branch": len(hard_branch),
            "hard_pair": len(hard_pair),
        },
        "branch_allocation_model": branch_metrics,
        "pair_allocation_model": pair_metrics,
        "label_safety": {
            "action_policy_trained": False,
            "winner_or_preference_label_used": False,
            "allocation_targets_are_search_budget_buckets": True,
        },
    }
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
