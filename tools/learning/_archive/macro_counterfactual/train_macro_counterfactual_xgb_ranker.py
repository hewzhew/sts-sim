#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import pickle
from collections import defaultdict
from pathlib import Path
from typing import Any

import numpy as np
from sklearn.feature_extraction import DictVectorizer
from xgboost import XGBRanker

from combat_reranker_common import stable_split
from combat_rl_common import REPO_ROOT
from macro_counterfactual_common import (
    group_option_rows,
    iter_jsonl_rows,
    macro_option_feature_dict,
    top_scoring_macro_mistakes,
    write_json,
    write_jsonl,
)


def load_rows(path: Path) -> list[dict[str, Any]]:
    return iter_jsonl_rows(path) if path.exists() else []


def split_decision_ids(option_rows: list[dict[str, Any]]) -> dict[str, str]:
    decision_ids = sorted({str(row.get("decision_id") or "") for row in option_rows})
    return {decision_id: stable_split(decision_id) for decision_id in decision_ids}


def build_pairwise_targets(
    option_rows: list[dict[str, Any]],
    edge_rows: list[dict[str, Any]],
) -> tuple[dict[str, dict[str, float]], dict[str, set[str]]]:
    grouped_options = group_option_rows(option_rows)
    grouped_edges: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in edge_rows:
        grouped_edges[str(row.get("decision_id") or "")].append(row)

    target_scores: dict[str, dict[str, float]] = {}
    positive_sets: dict[str, set[str]] = {}
    for decision_id, rows in grouped_options.items():
        score_map = {str(row.get("option_id") or ""): 0.0 for row in rows}
        for edge in grouped_edges.get(decision_id, []):
            preferred = str(edge.get("preferred_option_id") or "")
            rejected = str(edge.get("rejected_option_id") or "")
            margin = float(edge.get("vote_margin") or edge.get("strength") or 0.0)
            score_map[preferred] = score_map.get(preferred, 0.0) + margin
            score_map[rejected] = score_map.get(rejected, 0.0) - margin
        if not score_map:
            continue
        top_score = max(score_map.values(), default=0.0)
        positive_sets[decision_id] = {
            option_id
            for option_id, score in score_map.items()
            if abs(score - top_score) <= 1e-6
        }
        target_scores[decision_id] = score_map
    return target_scores, positive_sets


def make_option_lookup(option_rows: list[dict[str, Any]]) -> dict[tuple[str, str], dict[str, Any]]:
    return {
        (str(row.get("decision_id") or ""), str(row.get("option_id") or "")): row
        for row in option_rows
    }


def baseline_score(row: dict[str, Any]) -> float:
    return 1.0 if bool(row.get("baseline_matches_option")) else 0.0


def encode_rank_groups(
    rows: list[dict[str, Any]],
    target_scores: dict[str, dict[str, float]],
    *,
    vectorizer: DictVectorizer,
    fit: bool,
) -> tuple[Any, np.ndarray, list[int], list[dict[str, Any]]]:
    grouped = group_option_rows(rows)
    ordered_rows: list[dict[str, Any]] = []
    labels: list[int] = []
    groups: list[int] = []
    for decision_id in sorted(grouped):
        decision_rows = grouped[decision_id]
        score_map = target_scores.get(decision_id) or {}
        unique_scores = sorted(set(float(score_map.get(str(row.get("option_id") or ""), 0.0)) for row in decision_rows))
        label_map = {score: index for index, score in enumerate(unique_scores)}
        groups.append(len(decision_rows))
        for row in decision_rows:
            option_id = str(row.get("option_id") or "")
            raw_score = float(score_map.get(option_id, 0.0))
            ordered_rows.append(row)
            labels.append(int(label_map[raw_score]))
    feature_dicts = [macro_option_feature_dict(row) for row in ordered_rows]
    matrix = vectorizer.fit_transform(feature_dicts) if fit else vectorizer.transform(feature_dicts)
    return matrix, np.asarray(labels, dtype=np.float32), groups, ordered_rows


def xgb_score_fn(model: XGBRanker, vectorizer: DictVectorizer):
    def score(row: dict[str, Any]) -> float:
        features = vectorizer.transform([macro_option_feature_dict(row)])
        return float(model.predict(features)[0])

    return score


def evaluate_pairwise(
    edge_rows: list[dict[str, Any]],
    option_lookup: dict[tuple[str, str], dict[str, Any]],
    score_fn,
) -> dict[str, Any]:
    total = 0
    correct = 0
    weighted_total = 0.0
    weighted_correct = 0.0
    for edge in edge_rows:
        decision_id = str(edge.get("decision_id") or "")
        preferred_id = str(edge.get("preferred_option_id") or "")
        rejected_id = str(edge.get("rejected_option_id") or "")
        preferred = option_lookup.get((decision_id, preferred_id))
        rejected = option_lookup.get((decision_id, rejected_id))
        if preferred is None or rejected is None:
            continue
        pref_score = float(score_fn(preferred))
        rej_score = float(score_fn(rejected))
        weight = float(edge.get("strength") or 1.0)
        total += 1
        weighted_total += weight
        if pref_score > rej_score:
            correct += 1
            weighted_correct += weight
    return {
        "pairwise_total": total,
        "pairwise_correct": correct,
        "pairwise_agreement": round(correct / float(max(total, 1)), 6),
        "weighted_pairwise_agreement": round(weighted_correct / float(max(weighted_total, 1.0)), 6),
    }


def evaluate_decisions(
    *,
    option_rows: list[dict[str, Any]],
    target_scores: dict[str, dict[str, float]],
    positive_sets: dict[str, set[str]],
    score_fn,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    grouped = group_option_rows(option_rows)
    predictions: list[dict[str, Any]] = []
    decision_count = 0
    top1_match = 0
    baseline_top1_match = 0
    source_correct: dict[str, int] = defaultdict(int)
    source_total: dict[str, int] = defaultdict(int)
    source_baseline_correct: dict[str, int] = defaultdict(int)

    for decision_id, rows in grouped.items():
        decision_count += 1
        source_kind = str(rows[0].get("source_kind") or "unknown")
        source_total[source_kind] += 1
        scored_rows = sorted(
            (
                {
                    "option_id": str(row.get("option_id") or ""),
                    "label": str(row.get("label") or ""),
                    "option_kind": str(row.get("option_kind") or ""),
                    "score": round(float(score_fn(row)), 6),
                    "target_score": round(float((target_scores.get(decision_id) or {}).get(str(row.get("option_id") or ""), 0.0)), 6),
                    "is_positive": str(row.get("option_id") or "") in positive_sets.get(decision_id, set()),
                    "is_baseline": bool(row.get("baseline_matches_option")),
                }
                for row in rows
            ),
            key=lambda item: item["score"],
            reverse=True,
        )
        predicted = scored_rows[0]
        top1_hit = bool(predicted["is_positive"])
        top1_match += int(top1_hit)
        source_correct[source_kind] += int(top1_hit)

        baseline = next((item for item in scored_rows if item["is_baseline"]), None)
        baseline_hit = bool(baseline and baseline["is_positive"])
        baseline_top1_match += int(baseline_hit)
        source_baseline_correct[source_kind] += int(baseline_hit)

        predictions.append(
            {
                "decision_id": decision_id,
                "run_id": rows[0].get("run_id"),
                "source_kind": source_kind,
                "screen_type": rows[0].get("screen_type"),
                "baseline_choice_kind": rows[0].get("baseline_choice_kind"),
                "predicted_option_id": predicted["option_id"],
                "predicted_label": predicted["label"],
                "predicted_kind": predicted["option_kind"],
                "baseline_option_id": baseline["option_id"] if baseline else None,
                "baseline_label": baseline["label"] if baseline else None,
                "positive_option_ids": sorted(positive_sets.get(decision_id, set())),
                "top1_match": top1_hit,
                "baseline_top1_match": baseline_hit,
                "scores": scored_rows,
            }
        )

    source_breakdown = {}
    for source_kind in sorted(source_total):
        total = source_total[source_kind]
        model_correct = source_correct[source_kind]
        baseline_correct = source_baseline_correct[source_kind]
        source_breakdown[source_kind] = {
            "decision_count": total,
            "top1_match_rate": round(model_correct / float(max(total, 1)), 6),
            "baseline_top1_match_rate": round(baseline_correct / float(max(total, 1)), 6),
            "top1_improvement": round((model_correct - baseline_correct) / float(max(total, 1)), 6),
        }

    metrics = {
        "decision_count": decision_count,
        "top1_match": top1_match,
        "top1_match_rate": round(top1_match / float(max(decision_count, 1)), 6),
        "baseline_top1_match": baseline_top1_match,
        "baseline_top1_match_rate": round(baseline_top1_match / float(max(decision_count, 1)), 6),
        "top1_improvement": round((top1_match - baseline_top1_match) / float(max(decision_count, 1)), 6),
        "source_breakdown": source_breakdown,
    }
    return metrics, predictions


def evaluate_split(
    *,
    name: str,
    option_rows: list[dict[str, Any]],
    edge_rows: list[dict[str, Any]],
    option_lookup: dict[tuple[str, str], dict[str, Any]],
    target_scores: dict[str, dict[str, float]],
    positive_sets: dict[str, set[str]],
    score_fn,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    pairwise = evaluate_pairwise(edge_rows, option_lookup, score_fn)
    decision_metrics, predictions = evaluate_decisions(
        option_rows=option_rows,
        target_scores=target_scores,
        positive_sets=positive_sets,
        score_fn=score_fn,
    )
    return {
        "split": name,
        "option_rows": len(option_rows),
        "edge_rows": len(edge_rows),
        **pairwise,
        **decision_metrics,
    }, predictions


def main() -> int:
    parser = argparse.ArgumentParser(description="Train an XGBoost macro counterfactual ranker.")
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--dataset-prefix", default="macro_counterfactual_pilot")
    parser.add_argument("--output-prefix", default="macro_counterfactual_xgb_ranker")
    parser.add_argument("--metrics-out", default=None, type=Path)
    parser.add_argument("--predictions-out", default=None, type=Path)
    parser.add_argument("--review-out", default=None, type=Path)
    parser.add_argument("--model-out", default=None, type=Path)
    args = parser.parse_args()

    options_path = args.dataset_dir / f"{args.dataset_prefix}_options.jsonl"
    pairwise_path = args.dataset_dir / f"{args.dataset_prefix}_pairwise.jsonl"
    option_rows = load_rows(options_path)
    edge_rows = load_rows(pairwise_path)
    if not option_rows or not edge_rows:
        raise SystemExit(f"missing macro counterfactual dataset rows for prefix '{args.dataset_prefix}'")

    split_map = split_decision_ids(option_rows)
    option_lookup = make_option_lookup(option_rows)
    target_scores, positive_sets = build_pairwise_targets(option_rows, edge_rows)

    train_option_rows = [row for row in option_rows if split_map[str(row.get("decision_id") or "")] == "train"]
    val_option_rows = [row for row in option_rows if split_map[str(row.get("decision_id") or "")] == "val"]
    test_option_rows = [row for row in option_rows if split_map[str(row.get("decision_id") or "")] == "test"]
    train_edge_rows = [row for row in edge_rows if split_map[str(row.get("decision_id") or "")] == "train"]
    val_edge_rows = [row for row in edge_rows if split_map[str(row.get("decision_id") or "")] == "val"]
    test_edge_rows = [row for row in edge_rows if split_map[str(row.get("decision_id") or "")] == "test"]

    vectorizer = DictVectorizer(sparse=True)
    x_train, y_train, train_groups, _ = encode_rank_groups(
        train_option_rows,
        target_scores,
        vectorizer=vectorizer,
        fit=True,
    )
    x_val, y_val, val_groups, _ = encode_rank_groups(
        val_option_rows,
        target_scores,
        vectorizer=vectorizer,
        fit=False,
    )

    ranker = XGBRanker(
        objective="rank:ndcg",
        learning_rate=0.05,
        max_depth=6,
        min_child_weight=2.0,
        n_estimators=300,
        subsample=0.85,
        colsample_bytree=0.85,
        reg_lambda=1.0,
        tree_method="hist",
        random_state=0,
        eval_metric=["ndcg@1", "ndcg@3"],
    )
    ranker.fit(
        x_train,
        y_train,
        group=train_groups,
        eval_set=[(x_val, y_val)],
        eval_group=[val_groups],
        verbose=False,
    )
    score_fn = xgb_score_fn(ranker, vectorizer)
    baseline_fn = baseline_score

    train_metrics, _ = evaluate_split(
        name="train",
        option_rows=train_option_rows,
        edge_rows=train_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        score_fn=score_fn,
    )
    val_metrics, _ = evaluate_split(
        name="val",
        option_rows=val_option_rows,
        edge_rows=val_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        score_fn=score_fn,
    )
    test_metrics, test_predictions = evaluate_split(
        name="test",
        option_rows=test_option_rows,
        edge_rows=test_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        score_fn=score_fn,
    )
    train_baseline_metrics, _ = evaluate_split(
        name="train",
        option_rows=train_option_rows,
        edge_rows=train_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        score_fn=baseline_fn,
    )
    val_baseline_metrics, _ = evaluate_split(
        name="val",
        option_rows=val_option_rows,
        edge_rows=val_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        score_fn=baseline_fn,
    )
    test_baseline_metrics, baseline_predictions = evaluate_split(
        name="test",
        option_rows=test_option_rows,
        edge_rows=test_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        score_fn=baseline_fn,
    )

    metrics_out = args.metrics_out or (args.dataset_dir / f"{args.output_prefix}_metrics.json")
    predictions_out = args.predictions_out or (args.dataset_dir / f"{args.output_prefix}_predictions.jsonl")
    review_out = args.review_out or (args.dataset_dir / f"{args.output_prefix}_review.json")
    model_out = args.model_out or (args.dataset_dir / f"{args.output_prefix}_model.json")
    vectorizer_out = args.dataset_dir / f"{args.output_prefix}_vectorizer.pkl"

    ranker.save_model(model_out)
    with vectorizer_out.open("wb") as handle:
        pickle.dump(vectorizer, handle)

    metrics = {
        "model": "xgboost_rank_ndcg",
        "dataset_prefix": args.dataset_prefix,
        "feature_count": int(len(vectorizer.feature_names_)),
        "decision_splits": {
            "train": len({row["decision_id"] for row in train_option_rows}),
            "val": len({row["decision_id"] for row in val_option_rows}),
            "test": len({row["decision_id"] for row in test_option_rows}),
        },
        "row_splits": {
            "train_option_rows": len(train_option_rows),
            "val_option_rows": len(val_option_rows),
            "test_option_rows": len(test_option_rows),
            "train_edge_rows": len(train_edge_rows),
            "val_edge_rows": len(val_edge_rows),
            "test_edge_rows": len(test_edge_rows),
        },
        "ranker_config": {
            "objective": "rank:ndcg",
            "learning_rate": 0.05,
            "max_depth": 6,
            "min_child_weight": 2.0,
            "n_estimators": 300,
            "subsample": 0.85,
            "colsample_bytree": 0.85,
            "tree_method": "hist",
        },
        "train": train_metrics,
        "val": val_metrics,
        "test": test_metrics,
        "baseline_controls": {
            "train": train_baseline_metrics,
            "val": val_baseline_metrics,
            "test": test_baseline_metrics,
        },
        "notes": [
            "tree ranker baseline over grouped macro counterfactual options",
            "relevance labels are derived from aggregate pairwise vote margins inside each decision graph",
            "features are current macro state and option semantics only; combat probe outcomes remain supervision, not input",
        ],
    }

    write_json(metrics_out, metrics)
    write_jsonl(predictions_out, test_predictions)
    write_json(
        review_out,
        {
            "metrics_path": str(metrics_out),
            "predictions_path": str(predictions_out),
            "model_path": str(model_out),
            "vectorizer_path": str(vectorizer_out),
            "hard_mistakes": top_scoring_macro_mistakes(test_predictions),
            "baseline_control_mistakes": top_scoring_macro_mistakes(baseline_predictions),
        },
    )

    print(json.dumps(metrics, indent=2, ensure_ascii=False))
    print(f"wrote macro xgboost ranker metrics to {metrics_out}")
    print(f"wrote macro xgboost ranker predictions to {predictions_out}")
    print(f"wrote macro xgboost ranker model to {model_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
