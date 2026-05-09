#!/usr/bin/env python3
"""Train and score contrast-family search allocation models.

The unit is a decision-local contrast family, not an action and not a pair
winner. Labels are outcome buckets aggregated from ordered pair outcome diffs.
The model output is a search-allocation priority for family evidence requests.
"""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable

import numpy as np
from sklearn.ensemble import ExtraTreesClassifier, ExtraTreesRegressor
from sklearn.feature_extraction import DictVectorizer
from sklearn.metrics import mean_absolute_error, mean_squared_error, roc_auc_score


FORBIDDEN_LABEL_KEYS = {
    "winner",
    "preferred",
    "preferred_action",
    "selected_action",
    "teacher_choice",
}

BASE_SCORE_NAMES = (
    "branch_model_abs_hp_diff",
    "residual_corrected_abs_hp_diff",
    "tail_abs_hp_diff_ge_10_probability",
    "tail_left_worse_ge_10_probability",
    "tail_left_better_ge_10_probability",
)


def iter_jsonl(path: Path) -> Iterable[dict[str, Any]]:
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                yield json.loads(line)


def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return default
    return number if math.isfinite(number) else default


def assert_no_action_label_leak(row: dict[str, Any], *, index: int) -> None:
    if row.get("trainable_as_action_label") is not False:
        raise ValueError(f"pair row {index} is action-label-like")
    if (row.get("label_policy") or {}).get("action_label") is not False:
        raise ValueError(f"pair row {index} has action_label=true")
    serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
    for key in FORBIDDEN_LABEL_KEYS:
        if f'"{key}"' in serialized:
            raise ValueError(f"pair row {index} contains forbidden key {key}")


def load_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for index, row in enumerate(iter_jsonl(path)):
        assert_no_action_label_leak(row, index=index)
        rows.append(row)
    return rows


def decision_key(row: dict[str, Any]) -> str:
    return json.dumps(
        {
            "episode_seed": row.get("episode_seed"),
            "episode_step": row.get("episode_step"),
            "decision_id": row.get("decision_id"),
        },
        sort_keys=True,
        separators=(",", ":"),
    )


def candidate_tags(candidate: dict[str, Any]) -> list[str]:
    kind = candidate.get("action_kind")
    if kind == "end_turn":
        return ["end_turn"]
    if kind != "play_card":
        return [str(kind or "unknown")]
    tags: list[str] = []
    if safe_float(candidate.get("card_base_damage")) > 0:
        tags.append("damage")
    if safe_float(candidate.get("card_base_block")) > 0:
        tags.append("block")
    if candidate.get("card_applies_vulnerable"):
        tags.append("vulnerable")
    if candidate.get("card_applies_weak"):
        tags.append("weak")
    if candidate.get("card_draws_cards"):
        tags.append("draw")
    if candidate.get("card_exhaust"):
        tags.append("exhaust")
    if candidate.get("card_scaling_piece") or candidate.get("card_type_id") == 3:
        tags.append("setup")
    if not tags:
        tags.append("play_card_other")
    return sorted(set(tags))


def primary_tag(candidate: dict[str, Any]) -> str:
    tags = candidate_tags(candidate)
    for tag in (
        "end_turn",
        "damage",
        "block",
        "vulnerable",
        "weak",
        "setup",
        "draw",
        "exhaust",
        "play_card_other",
    ):
        if tag in tags:
            return tag
    return tags[0] if tags else "unknown"


def contrast_family(row: dict[str, Any], *, mode: str) -> str:
    left = (row.get("left") or {}).get("candidate") or {}
    right = (row.get("right") or {}).get("candidate") or {}
    if mode == "action_kind":
        return f"{left.get('action_kind') or 'unknown'}_vs_{right.get('action_kind') or 'unknown'}"
    left_primary = primary_tag(left)
    right_primary = primary_tag(right)
    if mode == "primary_tag":
        return f"{left_primary}_vs_{right_primary}"
    if mode == "end_turn_split":
        if left_primary == "end_turn" and right_primary != "end_turn":
            return f"end_turn_vs_{right_primary}"
        if right_primary == "end_turn" and left_primary != "end_turn":
            return f"{left_primary}_vs_end_turn"
        return f"{left_primary}_vs_{right_primary}"
    raise ValueError(f"unknown family mode {mode}")


def score_value(row: dict[str, Any], score_name: str) -> float:
    signals = row.get("search_allocation_signals") or {}
    outputs = row.get("model_outputs") or {}
    tails = outputs.get("tail_probabilities") or {}
    if score_name == "residual_corrected_abs_hp_diff":
        return abs(safe_float(outputs.get("residual_corrected_hp_left_minus_right")))
    if score_name == "branch_model_abs_hp_diff":
        return abs(safe_float(outputs.get("branch_model_hp_left_minus_right")))
    if score_name in signals:
        return safe_float(signals.get(score_name))
    if score_name in tails:
        return safe_float(tails.get(score_name))
    return 0.0


def pair_target(row: dict[str, Any]) -> dict[str, float]:
    targets = row.get("targets") or {}
    hp_diff = safe_float(targets.get("hp_left_minus_right"))
    return {
        "hp_left_minus_right": hp_diff,
        "abs_hp_diff": abs(hp_diff),
        "total_reward_left_minus_right": safe_float(targets.get("total_reward_left_minus_right")),
    }


def add_value(features: dict[str, Any], name: str, value: Any) -> None:
    if value is None:
        return
    if isinstance(value, bool):
        features[name] = 1.0 if value else 0.0
        features[f"{name}={str(value).lower()}"] = 1.0
    elif isinstance(value, (int, float)):
        features[name] = safe_float(value)
    elif isinstance(value, str):
        features[f"{name}={value}"] = 1.0


def add_context(features: dict[str, Any], prefix: str, context: dict[str, Any]) -> None:
    for key, value in context.items():
        if isinstance(value, (dict, list)):
            continue
        add_value(features, f"{prefix}.{key}", value)


def row_feature_scores(row: dict[str, Any]) -> dict[str, float]:
    return {name: score_value(row, name) for name in BASE_SCORE_NAMES}


def family_row(
    *,
    key: str,
    family: str,
    rows: list[dict[str, Any]],
    family_mode: str,
) -> dict[str, Any]:
    representative = max(
        rows,
        key=lambda row: (
            score_value(row, "tail_abs_hp_diff_ge_10_probability"),
            score_value(row, "residual_corrected_abs_hp_diff"),
        ),
    )
    targets = [pair_target(row) for row in rows]
    abs_values = [target["abs_hp_diff"] for target in targets]
    abs10_values = [value for value in abs_values if value >= 10.0]
    abs15_values = [value for value in abs_values if value >= 15.0]
    features: dict[str, Any] = {
        "bias": 1.0,
        f"family={family}": 1.0,
        f"family_mode={family_mode}": 1.0,
        "pair_count": len(rows),
    }
    left = representative.get("left") or {}
    right = representative.get("right") or {}
    left_candidate = left.get("candidate") or {}
    right_candidate = right.get("candidate") or {}
    add_value(features, "left_primary_tag", primary_tag(left_candidate))
    add_value(features, "right_primary_tag", primary_tag(right_candidate))
    add_value(features, "pair_kind", f"{left_candidate.get('action_kind')}->{right_candidate.get('action_kind')}")
    add_context(features, "left.ctx", left.get("decision_context") or {})
    add_context(features, "right.ctx", right.get("decision_context") or {})
    score_lists: dict[str, list[float]] = {name: [] for name in BASE_SCORE_NAMES}
    for row in rows:
        for name, value in row_feature_scores(row).items():
            score_lists[name].append(value)
    for name, values in score_lists.items():
        if not values:
            continue
        features[f"{name}.max"] = max(values)
        features[f"{name}.mean"] = sum(values) / len(values)
    family_targets = {
        "high_regret_abs10": int(bool(abs10_values)),
        "high_regret_abs15": int(bool(abs15_values)),
        "max_abs_hp_diff": max(abs_values) if abs_values else 0.0,
        "regret_mass_abs10": sum(abs10_values),
        "regret_mass_abs15": sum(abs15_values),
        "high_regret_pair_count_abs10": len(abs10_values),
    }
    return {
        "schema_version": "family_search_allocation_example_v0",
        "trainable_role": "family_search_allocation",
        "trainable_as_action_label": False,
        "decision_key": key,
        "episode_seed": representative.get("episode_seed"),
        "episode_step": representative.get("episode_step"),
        "decision_id": representative.get("decision_id"),
        "family_mode": family_mode,
        "family": family,
        "representative_pair": {
            "left": {
                "branch_id": left.get("branch_id"),
                "candidate": left_candidate,
            },
            "right": {
                "branch_id": right.get("branch_id"),
                "candidate": right_candidate,
            },
        },
        "features": features,
        "targets": family_targets,
        "label_policy": {
            "action_label": False,
            "source": "family_search_allocation_model_v0",
        },
    }


def build_family_rows(rows: list[dict[str, Any]], *, family_mode: str) -> list[dict[str, Any]]:
    grouped: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        grouped[(decision_key(row), contrast_family(row, mode=family_mode))].append(row)
    out = [
        family_row(key=key, family=family, rows=items, family_mode=family_mode)
        for (key, family), items in grouped.items()
    ]
    return sorted(out, key=lambda row: (str(row.get("decision_key")), str(row.get("family"))))


class ConstantClassifier:
    def __init__(self, probability: float) -> None:
        self.probability = probability

    def predict_proba(self, x: Any) -> np.ndarray:
        return np.array([[1.0 - self.probability, self.probability]] * x.shape[0])


def fit_classifier(x_train: Any, y_train: list[int], seed: int) -> Any:
    positives = sum(y_train)
    if positives == 0 or positives == len(y_train):
        return ConstantClassifier(positives / len(y_train) if y_train else 0.0)
    model = ExtraTreesClassifier(
        n_estimators=500,
        max_features=0.85,
        min_samples_leaf=2,
        class_weight="balanced",
        random_state=seed,
        n_jobs=-1,
    )
    model.fit(x_train, np.asarray(y_train, dtype=int))
    return model


def predict_positive(model: Any, x: Any) -> list[float]:
    probabilities = model.predict_proba(x)
    if probabilities.shape[1] == 1:
        classes = list(getattr(model, "classes_", [0]))
        value = 1.0 if classes and int(classes[0]) == 1 else 0.0
        return [value for _ in range(x.shape[0])]
    classes = {int(cls): index for index, cls in enumerate(getattr(model, "classes_", [0, 1]))}
    positive_index = classes.get(1, 1)
    return [float(value) for value in probabilities[:, positive_index]]


def regression_metrics(y_true: list[float], y_pred: list[float]) -> dict[str, Any]:
    if not y_true:
        return {"count": 0}
    baseline = [sum(y_true) / len(y_true)] * len(y_true)
    baseline_mse = mean_squared_error(y_true, baseline)
    mse = mean_squared_error(y_true, y_pred)
    return {
        "count": len(y_true),
        "mae": mean_absolute_error(y_true, y_pred),
        "rmse": math.sqrt(mse),
        "baseline_rmse": math.sqrt(baseline_mse),
        "r2_vs_mean": 1.0 - mse / baseline_mse if baseline_mse > 0 else None,
    }


def binary_metrics(y_true: list[int], y_score: list[float]) -> dict[str, Any]:
    positives = sum(y_true)
    out: dict[str, Any] = {
        "count": len(y_true),
        "positive_count": positives,
        "positive_rate": positives / len(y_true) if y_true else None,
    }
    if len(set(y_true)) > 1:
        out["auc"] = roc_auc_score(y_true, y_score)
    else:
        out["auc"] = None
    return out


def train_and_score(
    train_rows: list[dict[str, Any]],
    score_rows: list[dict[str, Any]],
    *,
    seed: int,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], dict[str, Any]]:
    vectorizer = DictVectorizer(sparse=False)
    x_train = vectorizer.fit_transform([row["features"] for row in train_rows])
    x_score = vectorizer.transform([row["features"] for row in score_rows])
    y10 = [int((row.get("targets") or {}).get("high_regret_abs10") or 0) for row in train_rows]
    y15 = [int((row.get("targets") or {}).get("high_regret_abs15") or 0) for row in train_rows]
    mass = [safe_float((row.get("targets") or {}).get("regret_mass_abs10")) for row in train_rows]
    model10 = fit_classifier(x_train, y10, seed)
    model15 = fit_classifier(x_train, y15, seed + 1)
    mass_model = ExtraTreesRegressor(
        n_estimators=400,
        max_features=0.85,
        min_samples_leaf=2,
        random_state=seed + 2,
        n_jobs=-1,
    )
    mass_model.fit(x_train, np.asarray(mass, dtype=float))

    train_scores = {
        "abs10": predict_positive(model10, x_train),
        "abs15": predict_positive(model15, x_train),
        "mass10": [float(value) for value in mass_model.predict(x_train)],
    }
    score_scores = {
        "abs10": predict_positive(model10, x_score),
        "abs15": predict_positive(model15, x_score),
        "mass10": [float(value) for value in mass_model.predict(x_score)],
    }

    def augment(rows: list[dict[str, Any]], scores: dict[str, list[float]]) -> list[dict[str, Any]]:
        out: list[dict[str, Any]] = []
        for index, row in enumerate(rows):
            next_row = dict(row)
            abs10 = scores["abs10"][index]
            abs15 = scores["abs15"][index]
            mass10 = max(0.0, scores["mass10"][index])
            priority = max(abs10, abs15, min(1.0, mass10 / 30.0))
            next_row["model_outputs"] = {
                "schema_version": "family_search_allocation_model_v0",
                "high_regret_abs10_probability": abs10,
                "high_regret_abs15_probability": abs15,
                "regret_mass_abs10": mass10,
                "family_priority": priority,
            }
            next_row["search_allocation_signals"] = {
                "family_abs_ge_10_probability": abs10,
                "family_abs_ge_15_probability": abs15,
                "family_regret_mass_abs10": mass10,
                "family_priority": priority,
            }
            out.append(next_row)
        return out

    train_aug = augment(train_rows, train_scores)
    score_aug = augment(score_rows, score_scores)
    score_y10 = [int((row.get("targets") or {}).get("high_regret_abs10") or 0) for row in score_rows]
    score_y15 = [int((row.get("targets") or {}).get("high_regret_abs15") or 0) for row in score_rows]
    score_mass = [safe_float((row.get("targets") or {}).get("regret_mass_abs10")) for row in score_rows]
    metrics = {
        "feature_count": len(vectorizer.feature_names_),
        "train_rows": len(train_rows),
        "score_rows": len(score_rows),
        "score_abs10": binary_metrics(score_y10, score_scores["abs10"]),
        "score_abs15": binary_metrics(score_y15, score_scores["abs15"]),
        "score_mass10": regression_metrics(score_mass, score_scores["mass10"]),
    }
    return train_aug, score_aug, metrics


def assert_family_safety(rows: list[dict[str, Any]]) -> None:
    for index, row in enumerate(rows):
        if row.get("trainable_as_action_label") is not False:
            raise ValueError(f"family row {index} is action-label-like")
        if (row.get("label_policy") or {}).get("action_label") is not False:
            raise ValueError(f"family row {index} has action_label=true")
        serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
        for key in FORBIDDEN_LABEL_KEYS:
            if f'"{key}"' in serialized:
                raise ValueError(f"family row {index} contains forbidden key {key}")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--train-pairs", type=Path, required=True)
    parser.add_argument("--score-pairs", type=Path, required=True)
    parser.add_argument("--train-family-out", type=Path, required=True)
    parser.add_argument("--score-family-out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--family-mode", default="primary_tag")
    parser.add_argument("--seed", type=int, default=771)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    train_pair_rows = load_rows(args.train_pairs)
    score_pair_rows = load_rows(args.score_pairs)
    train_family = build_family_rows(train_pair_rows, family_mode=args.family_mode)
    score_family = build_family_rows(score_pair_rows, family_mode=args.family_mode)
    train_aug, score_aug, metrics = train_and_score(train_family, score_family, seed=args.seed)
    assert_family_safety(train_aug)
    assert_family_safety(score_aug)
    write_jsonl(args.train_family_out, train_aug)
    write_jsonl(args.score_family_out, score_aug)
    family_counts = Counter(row.get("family") for row in score_aug)
    summary = {
        "schema_version": "family_search_allocation_model_v0_summary",
        "family_mode": args.family_mode,
        "train_pairs": str(args.train_pairs),
        "score_pairs": str(args.score_pairs),
        "row_counts": {
            "train_pair_rows": len(train_pair_rows),
            "score_pair_rows": len(score_pair_rows),
            "train_family_rows": len(train_family),
            "score_family_rows": len(score_family),
        },
        "metrics": metrics,
        "score_family_counts_top": dict(family_counts.most_common(30)),
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "family_model_is_search_allocation_not_policy": True,
        },
    }
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
