import argparse
import json
import pickle
import random
import warnings
from collections import defaultdict
from pathlib import Path
from typing import Dict, List, Tuple


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Train a first-pass reward chooser model from reward_choice_rows.jsonl"
    )
    parser.add_argument(
        "--input",
        default=r"D:\rust\sts_simulator\data\reward_choice_rows.jsonl",
        help="Path to reward choice row jsonl exported from export_reward_samples",
    )
    parser.add_argument(
        "--class-filter",
        default="IRONCLAD",
        help="Only train on rows from this class. Default keeps phase-1 scope narrow.",
    )
    parser.add_argument(
        "--validation-frac",
        type=float,
        default=0.25,
        help="Fraction of sample_ids held out for validation.",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=1337,
        help="Deterministic shuffle seed for group split.",
    )
    parser.add_argument(
        "--model-out",
        default=r"D:\rust\sts_simulator\data\reward_ranker.pkl",
        help="Pickle artifact path.",
    )
    parser.add_argument(
        "--metrics-out",
        default=r"D:\rust\sts_simulator\data\reward_ranker_metrics.json",
        help="Metrics json path.",
    )
    parser.add_argument(
        "--feature-limit",
        type=int,
        default=40,
        help="How many top feature names to emit in metrics output.",
    )
    parser.add_argument(
        "--review-out",
        default=r"D:\rust\sts_simulator\data\reward_ranker_review.jsonl",
        help="Per-sample prediction review output.",
    )
    parser.add_argument(
        "--weight-disagreements",
        action="store_true",
        help="Multiply sample weights by disagreement_weight when present.",
    )
    return parser.parse_args()


def load_rows(path: str, class_filter: str) -> List[dict]:
    rows: List[dict] = []
    with open(path, "r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            if class_filter and row.get("class_name") != class_filter:
                continue
            rows.append(row)
    return rows


def build_feature_vocab(rows: List[dict]) -> List[str]:
    vocab = set()
    for row in rows:
        vocab.update(row.get("features", {}).keys())
    return sorted(vocab)


def build_matrix(rows: List[dict], feature_names: List[str]):
    import numpy as np

    feature_index = {name: idx for idx, name in enumerate(feature_names)}
    X = np.zeros((len(rows), len(feature_names)), dtype=np.float32)
    y = np.zeros(len(rows), dtype=np.int32)
    sample_ids: List[str] = []
    card_ids: List[str] = []
    sources: List[str] = []
    quality_weights = np.ones(len(rows), dtype=np.float32)
    disagreement_weights = np.ones(len(rows), dtype=np.float32)
    for i, row in enumerate(rows):
        y[i] = int(row.get("label", 0))
        sample_ids.append(row["sample_id"])
        card_ids.append(row["card_id"])
        sources.append(row.get("source", "unknown"))
        quality_weights[i] = float(row.get("quality_weight", 1.0))
        disagreement_weights[i] = float(row.get("disagreement_weight", 1.0))
        for key, value in row.get("features", {}).items():
            idx = feature_index.get(key)
            if idx is not None:
                X[i, idx] = float(value)
    return X, y, sample_ids, card_ids, sources, quality_weights, disagreement_weights


def split_by_sample_id(sample_ids: List[str], validation_frac: float, seed: int) -> Tuple[set, set]:
    ordered = sorted(set(sample_ids))
    rng = random.Random(seed)
    rng.shuffle(ordered)
    val_count = max(1, int(len(ordered) * validation_frac)) if ordered else 0
    val_ids = set(ordered[:val_count])
    train_ids = set(ordered[val_count:])
    if not train_ids and val_ids:
        moved = next(iter(val_ids))
        val_ids.remove(moved)
        train_ids.add(moved)
    return train_ids, val_ids


def row_mask(sample_ids: List[str], allowed: set) -> List[bool]:
    return [sample_id in allowed for sample_id in sample_ids]


def group_lengths(sample_ids: List[str], allowed: set) -> List[int]:
    lengths: List[int] = []
    current = None
    current_len = 0
    for sample_id in sample_ids:
        if sample_id not in allowed:
            continue
        if current is None:
            current = sample_id
            current_len = 1
        elif sample_id == current:
            current_len += 1
        else:
            lengths.append(current_len)
            current = sample_id
            current_len = 1
    if current_len:
        lengths.append(current_len)
    return lengths


def train_lightgbm_ranker(
    X_train, y_train, group_train, X_val, y_val, group_val, sample_weight_train, sample_weight_val
):
    import lightgbm as lgb

    model = lgb.LGBMRanker(
        objective="lambdarank",
        metric="ndcg",
        n_estimators=200,
        learning_rate=0.05,
        num_leaves=31,
        min_child_samples=5,
        random_state=1337,
    )
    fit_kwargs = {
        "X": X_train,
        "y": y_train,
        "group": group_train,
        "sample_weight": sample_weight_train,
    }
    if len(group_val) > 0:
        fit_kwargs.update(
            {
                "eval_set": [(X_val, y_val)],
                "eval_group": [group_val],
                "eval_sample_weight": [sample_weight_val],
            }
        )
    model.fit(**fit_kwargs)
    return model, "lightgbm_lambdarank"


def train_sklearn_fallback(X_train, y_train, sample_weight_train):
    try:
        from sklearn.ensemble import HistGradientBoostingClassifier
    except ImportError as exc:
        raise RuntimeError(
            "No training backend available. Install either `lightgbm` or `scikit-learn`.\n"
            "Suggested commands:\n"
            "  pip install lightgbm\n"
            "or\n"
            "  pip install scikit-learn"
        ) from exc

    model = HistGradientBoostingClassifier(
        learning_rate=0.05,
        max_depth=6,
        max_iter=200,
        random_state=1337,
    )
    model.fit(X_train, y_train, sample_weight=sample_weight_train)
    return model, "sklearn_hist_gradient_boosting"


def predict_scores(model, model_kind: str, X):
    warnings.filterwarnings(
        "ignore",
        message="X does not have valid feature names, but LGBMRanker was fitted with feature names",
    )
    if model_kind == "lightgbm_lambdarank":
        return model.predict(X)
    proba = model.predict_proba(X)
    if proba.shape[1] == 2:
        return proba[:, 1]
    return proba[:, 0]


def evaluate_top1(sample_ids: List[str], y, scores) -> float:
    grouped: Dict[str, List[Tuple[float, int]]] = defaultdict(list)
    for sample_id, label, score in zip(sample_ids, y, scores):
        grouped[sample_id].append((float(score), int(label)))

    total = 0
    correct = 0
    for _, rows in grouped.items():
        if not rows:
            continue
        total += 1
        best = max(rows, key=lambda item: item[0])
        if best[1] == 1:
            correct += 1
    return correct / total if total else 0.0


def extract_top_features(model, model_kind: str, feature_names: List[str], limit: int):
    if model_kind == "lightgbm_lambdarank":
        importances = list(model.feature_importances_)
    elif hasattr(model, "feature_importances_"):
        importances = list(model.feature_importances_)
    else:
        return []

    ranked = sorted(
        zip(feature_names, importances),
        key=lambda item: item[1],
        reverse=True,
    )
    return [
        {"feature": feature, "importance": float(importance)}
        for feature, importance in ranked[:limit]
        if float(importance) > 0
    ]


def ensure_parent(path: str) -> None:
    Path(path).parent.mkdir(parents=True, exist_ok=True)


def build_review_records(rows: List[dict], scores, split_name: str) -> List[dict]:
    grouped: Dict[str, List[Tuple[dict, float]]] = defaultdict(list)
    for row, score in zip(rows, scores):
        grouped[row["sample_id"]].append((row, float(score)))

    records = []
    for sample_id, sample_rows in grouped.items():
        ranked = sorted(sample_rows, key=lambda item: item[1], reverse=True)
        predicted_row = ranked[0][0]
        actual_row = next((row for row, _ in sample_rows if int(row.get("label", 0)) == 1), None)
        records.append(
            {
                "sample_id": sample_id,
                "split": split_name,
                "source": predicted_row.get("source", "unknown"),
                "quality_weight": float(predicted_row.get("quality_weight", 1.0)),
                "predicted_card_id": predicted_row["card_id"],
                "predicted_choice_index": predicted_row["choice_index"],
                "actual_card_id": actual_row["card_id"] if actual_row else None,
                "actual_choice_index": actual_row["choice_index"] if actual_row else None,
                "correct": actual_row is not None
                and predicted_row["card_id"] == actual_row["card_id"],
                "candidates": [
                    {
                        "card_id": row["card_id"],
                        "choice_index": row["choice_index"],
                        "label": int(row.get("label", 0)),
                        "score": score,
                    }
                    for row, score in ranked
                ],
            }
        )
    return records


def main() -> None:
    args = parse_args()
    rows = load_rows(args.input, args.class_filter)
    if not rows:
        raise SystemExit(f"no rows found in {args.input} for class {args.class_filter}")

    feature_names = build_feature_vocab(rows)
    X, y, sample_ids, card_ids, sources, quality_weights, disagreement_weights = build_matrix(
        rows, feature_names
    )
    train_ids, val_ids = split_by_sample_id(sample_ids, args.validation_frac, args.seed)

    import numpy as np

    train_mask = np.array(row_mask(sample_ids, train_ids), dtype=bool)
    val_mask = np.array(row_mask(sample_ids, val_ids), dtype=bool)

    X_train = X[train_mask]
    y_train = y[train_mask]
    train_sample_ids = [sample_ids[i] for i in range(len(sample_ids)) if train_mask[i]]
    train_rows = [rows[i] for i in range(len(rows)) if train_mask[i]]
    sample_weight_train = quality_weights[train_mask].copy()
    if args.weight_disagreements:
        sample_weight_train *= disagreement_weights[train_mask]

    X_val = X[val_mask]
    y_val = y[val_mask]
    val_sample_ids = [sample_ids[i] for i in range(len(sample_ids)) if val_mask[i]]
    val_rows = [rows[i] for i in range(len(rows)) if val_mask[i]]
    sample_weight_val = quality_weights[val_mask].copy()
    if args.weight_disagreements:
        sample_weight_val *= disagreement_weights[val_mask]

    group_train = group_lengths(sample_ids, train_ids)
    group_val = group_lengths(sample_ids, val_ids)

    try:
        model, model_kind = train_lightgbm_ranker(
            X_train,
            y_train,
            group_train,
            X_val,
            y_val,
            group_val,
            sample_weight_train,
            sample_weight_val,
        )
    except ImportError:
        model, model_kind = train_sklearn_fallback(X_train, y_train, sample_weight_train)

    train_scores = predict_scores(model, model_kind, X_train)
    val_scores = predict_scores(model, model_kind, X_val) if len(X_val) > 0 else np.array([])

    metrics = {
        "input_path": args.input,
        "class_filter": args.class_filter,
        "model_kind": model_kind,
        "num_rows": len(rows),
        "num_samples": len(set(sample_ids)),
        "num_features": len(feature_names),
        "weight_disagreements": args.weight_disagreements,
        "train_rows": int(train_mask.sum()),
        "val_rows": int(val_mask.sum()),
        "train_samples": len(train_ids),
        "val_samples": len(val_ids),
        "disagreement_rows": int(sum(1 for row in rows if row.get("bot_human_agree") is False)),
        "disagreement_samples": len(
            {row["sample_id"] for row in rows if row.get("bot_human_agree") is False}
        ),
        "train_top1_accuracy": evaluate_top1(train_sample_ids, y_train, train_scores),
        "val_top1_accuracy": evaluate_top1(val_sample_ids, y_val, val_scores)
        if len(val_scores) > 0
        else None,
        "top_features": extract_top_features(
            model, model_kind, feature_names, args.feature_limit
        ),
    }

    review_records = build_review_records(train_rows, train_scores, "train")
    if len(val_scores) > 0:
        review_records.extend(build_review_records(val_rows, val_scores, "val"))

    ensure_parent(args.model_out)
    with open(args.model_out, "wb") as handle:
        pickle.dump(
            {
                "model_kind": model_kind,
                "class_filter": args.class_filter,
                "feature_names": feature_names,
                "model": model,
            },
            handle,
        )

    ensure_parent(args.metrics_out)
    with open(args.metrics_out, "w", encoding="utf-8") as handle:
        json.dump(metrics, handle, indent=2, ensure_ascii=False)

    ensure_parent(args.review_out)
    with open(args.review_out, "w", encoding="utf-8") as handle:
        for record in review_records:
            handle.write(json.dumps(record, ensure_ascii=False) + "\n")

    print(json.dumps(metrics, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
