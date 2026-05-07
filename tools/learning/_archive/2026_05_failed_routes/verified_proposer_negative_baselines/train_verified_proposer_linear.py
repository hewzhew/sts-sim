#!/usr/bin/env python3
"""Train a lightweight high-recall proposer for verified-H candidate filtering."""
from __future__ import annotations

import argparse
import json
import math
import random
from collections import defaultdict
from pathlib import Path
from typing import Any

from return_q_common import ADV_OVERRIDE_FEATURE_SETS, adv_override_features, read_jsonl, write_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--model-out", type=Path, required=True)
    parser.add_argument("--report-out", type=Path)
    parser.add_argument("--feature-set", default="candidate_only", choices=ADV_OVERRIDE_FEATURE_SETS)
    parser.add_argument("--target-mode", default="oracle_chosen", choices=["oracle_chosen", "margin_positive"])
    parser.add_argument("--epochs", type=int, default=8)
    parser.add_argument("--learning-rate", type=float, default=0.08)
    parser.add_argument("--l2", type=float, default=0.00001)
    parser.add_argument("--seed", type=int, default=17)
    parser.add_argument("--thresholds", default="0.02,0.05,0.1,0.2,0.3,0.5")
    parser.add_argument("--top-k", default="1,2,3,4")
    parser.add_argument("--target-recalls", default="0.9,0.95,0.98")
    parser.add_argument("--cheap-horizons", default="")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    rng = random.Random(args.seed)
    thresholds = parse_float_list(args.thresholds)
    top_ks = parse_int_list(args.top_k)
    target_recalls = parse_float_list(args.target_recalls)
    rows = read_jsonl(args.input)
    examples = prepare_examples(rows, args.feature_set, args.target_mode)
    train = [ex for ex in examples if ex["split"] == "train"]
    weights: defaultdict[int, float] = defaultdict(float)
    bias = 0.0
    positives = sum(1 for ex in train if ex["target"] > 0.5)
    negatives = max(len(train) - positives, 1)
    pos_weight = negatives / max(positives, 1)

    for _epoch in range(max(args.epochs, 0)):
        rng.shuffle(train)
        for ex in train:
            sparse = ex["features"]
            y = float(ex["target"])
            score = bias + dot(weights, sparse)
            p = sigmoid(score)
            sample_weight = pos_weight if y > 0.5 else 1.0
            grad = (p - y) * sample_weight
            bias -= args.learning_rate * grad
            for idx, value in sparse.items():
                weights[idx] -= args.learning_rate * (grad * value + args.l2 * weights[idx])

    metrics = {
        split: evaluate(
            [ex for ex in examples if ex["split"] == split],
            weights,
            bias,
            thresholds,
            top_ks,
            target_recalls,
        )
        for split in ["train", "valid", "test"]
    }
    model = {
        "schema_version": "verified_proposer_linear_v0",
        "model_type": "verified_proposer_linear",
        "feature_set": args.feature_set,
        "target_mode": f"verified_h_{args.target_mode}_candidate_proposer",
        "bias": bias,
        "weights": [[int(idx), float(value)] for idx, value in weights.items() if abs(value) > 1e-12],
        "config": {
            "epochs": args.epochs,
            "learning_rate": args.learning_rate,
            "l2": args.l2,
            "seed": args.seed,
            "pos_weight": pos_weight,
            "target_mode": args.target_mode,
            "cheap_horizons": parse_int_list(args.cheap_horizons),
        },
    }
    write_json(args.model_out, model)
    report = {
        "schema_version": "verified_proposer_linear_report_v0",
        "input": str(args.input),
        "model_out": str(args.model_out),
        "row_count": len(rows),
        "example_count": len(examples),
        "label_counts": label_counts(examples),
        "split_counts": split_counts(examples),
        "metrics": metrics,
    }
    write_json(args.report_out or args.model_out.with_suffix(".report.json"), report)
    print(json.dumps(report, indent=2, sort_keys=True))


def prepare_examples(rows: list[dict[str, Any]], feature_set: str, target_mode: str) -> list[dict[str, Any]]:
    examples = []
    for row in rows:
        if target_mode == "oracle_chosen":
            target = 1.0 if bool(row.get("is_full_verified_choice")) and not bool(row.get("is_rule_choice")) else 0.0
        else:
            label = str(row.get("safe_override_label") or "")
            if label not in {"positive", "negative"}:
                continue
            target = 1.0 if label == "positive" else 0.0
        examples.append(
            {
                "group_key": str(row.get("group_key") or ""),
                "split": str(row.get("split") or "train"),
                "target": target,
                "adv": float(row.get("adv_vs_rule_mean") or 0.0),
                "is_rule": bool(row.get("is_rule_choice")),
                "features": adv_override_features(row, feature_set),
            }
        )
    return examples


def evaluate(
    examples: list[dict[str, Any]],
    weights: dict[int, float],
    bias: float,
    thresholds: list[float],
    top_ks: list[int],
    target_recalls: list[float],
) -> dict[str, Any]:
    if not examples:
        return {"count": 0}
    scored = []
    by_group: defaultdict[str, list[dict[str, Any]]] = defaultdict(list)
    for ex in examples:
        score = bias + dot(weights, ex["features"])
        prob = sigmoid(score)
        row = {
            "group_key": ex["group_key"],
            "prob": prob,
            "target": ex["target"],
            "adv": ex["adv"],
            "is_rule": ex["is_rule"],
        }
        scored.append(row)
        by_group[ex["group_key"]].append(row)
    threshold_rows = {
        str(threshold): threshold_metrics(scored, by_group, threshold)
        for threshold in thresholds
    }
    topk_rows = {str(k): topk_metrics(by_group, k) for k in top_ks}
    hybrid_rows = {
        f"top{k}_thr{threshold}": hybrid_metrics(by_group, k, threshold)
        for k in top_ks
        for threshold in thresholds
    }
    candidates = []
    for name, row in threshold_rows.items():
        candidates.append({"selector": "threshold", "setting": name, **row})
    for name, row in topk_rows.items():
        candidates.append({"selector": "top_k", "setting": name, **row})
    for name, row in hybrid_rows.items():
        candidates.append({"selector": "hybrid", "setting": name, **row})
    return {
        "count": len(scored),
        "positive_count": sum(1 for row in scored if row["target"] > 0.5),
        "positive_rate": mean(row["target"] for row in scored),
        "average_precision": average_precision(scored),
        "thresholds": threshold_rows,
        "top_k": topk_rows,
        "hybrid": hybrid_rows,
        "best_for_target_recall": {
            str(target): best_for_target_recall(candidates, target)
            for target in target_recalls
        },
    }


def threshold_metrics(
    rows: list[dict[str, Any]],
    by_group: dict[str, list[dict[str, Any]]],
    threshold: float,
) -> dict[str, Any]:
    selected = [row for row in rows if row["prob"] >= threshold and not row["is_rule"]]
    positives = [row for row in rows if row["target"] > 0.5]
    true_selected = [row for row in selected if row["target"] > 0.5]
    group_recall_num = 0
    group_recall_den = 0
    selected_non_rule = 0
    total_non_rule = 0
    for group_rows in by_group.values():
        group_positives = [row for row in group_rows if row["target"] > 0.5]
        non_rule = [row for row in group_rows if not row["is_rule"]]
        selected_group = [row for row in non_rule if row["prob"] >= threshold]
        selected_non_rule += len(selected_group)
        total_non_rule += len(non_rule)
        if group_positives:
            group_recall_den += 1
            if any(row["prob"] >= threshold for row in group_positives):
                group_recall_num += 1
    return {
        "selected_count": len(selected),
        "candidate_keep_rate": selected_non_rule / total_non_rule if total_non_rule else 0.0,
        "precision": len(true_selected) / len(selected) if selected else None,
        "candidate_recall": len(true_selected) / len(positives) if positives else None,
        "positive_group_recall": group_recall_num / group_recall_den if group_recall_den else None,
        "accepted_candidate_true_adv": mean(row["adv"] for row in selected) if selected else None,
    }


def topk_metrics(by_group: dict[str, list[dict[str, Any]]], k: int) -> dict[str, Any]:
    selected_non_rule = 0
    total_non_rule = 0
    hit_candidates = 0
    positive_candidates = 0
    hit_groups = 0
    positive_groups = 0
    for group_rows in by_group.values():
        non_rule = [row for row in group_rows if not row["is_rule"]]
        selected = sorted(non_rule, key=lambda row: row["prob"], reverse=True)[:k]
        selected_set = {id(row) for row in selected}
        selected_non_rule += len(selected)
        total_non_rule += len(non_rule)
        positives = [row for row in group_rows if row["target"] > 0.5]
        positive_candidates += len(positives)
        hit_candidates += sum(1 for row in positives if id(row) in selected_set)
        if positives:
            positive_groups += 1
            if any(id(row) in selected_set for row in positives):
                hit_groups += 1
    return {
        "candidate_keep_rate": selected_non_rule / total_non_rule if total_non_rule else 0.0,
        "candidate_recall": hit_candidates / positive_candidates if positive_candidates else None,
        "positive_group_recall": hit_groups / positive_groups if positive_groups else None,
        "selected_count": selected_non_rule,
        "precision": hit_candidates / selected_non_rule if selected_non_rule else None,
        "accepted_candidate_true_adv": mean(row["adv"] for rows in by_group.values() for row in sorted([item for item in rows if not item["is_rule"]], key=lambda item: item["prob"], reverse=True)[:k]) if selected_non_rule else None,
    }


def hybrid_metrics(
    by_group: dict[str, list[dict[str, Any]]],
    k: int,
    threshold: float,
) -> dict[str, Any]:
    selected_non_rule = 0
    total_non_rule = 0
    hit_candidates = 0
    positive_candidates = 0
    hit_groups = 0
    positive_groups = 0
    selected_adv: list[float] = []
    for group_rows in by_group.values():
        non_rule = [row for row in group_rows if not row["is_rule"]]
        topk = sorted(non_rule, key=lambda row: row["prob"], reverse=True)[:k]
        selected = {id(row): row for row in topk}
        for row in non_rule:
            if row["prob"] >= threshold:
                selected[id(row)] = row
        selected_rows = list(selected.values())
        selected_set = set(selected)
        selected_non_rule += len(selected_rows)
        total_non_rule += len(non_rule)
        selected_adv.extend(row["adv"] for row in selected_rows)
        positives = [row for row in group_rows if row["target"] > 0.5]
        positive_candidates += len(positives)
        hit_candidates += sum(1 for row in positives if id(row) in selected_set)
        if positives:
            positive_groups += 1
            if any(id(row) in selected_set for row in positives):
                hit_groups += 1
    return {
        "selected_count": selected_non_rule,
        "candidate_keep_rate": selected_non_rule / total_non_rule if total_non_rule else 0.0,
        "precision": hit_candidates / selected_non_rule if selected_non_rule else None,
        "candidate_recall": hit_candidates / positive_candidates if positive_candidates else None,
        "positive_group_recall": hit_groups / positive_groups if positive_groups else None,
        "accepted_candidate_true_adv": mean(selected_adv) if selected_adv else None,
    }


def best_for_target_recall(candidates: list[dict[str, Any]], target_recall: float) -> dict[str, Any] | None:
    viable = [
        row
        for row in candidates
        if row.get("positive_group_recall") is not None
        and float(row["positive_group_recall"]) >= target_recall
    ]
    if not viable:
        return None
    best = min(
        viable,
        key=lambda row: (
            float(row.get("candidate_keep_rate") or 1.0),
            -float(row.get("candidate_recall") or 0.0),
        ),
    )
    return dict(best)


def dot(weights: dict[int, float], sparse: dict[int, float]) -> float:
    return sum(weights.get(idx, 0.0) * value for idx, value in sparse.items())


def sigmoid(value: float) -> float:
    if value >= 0:
        z = math.exp(-value)
        return 1.0 / (1.0 + z)
    z = math.exp(value)
    return z / (1.0 + z)


def average_precision(rows: list[dict[str, Any]]) -> float | None:
    sorted_rows = sorted(rows, key=lambda row: row["prob"], reverse=True)
    positives = sum(1 for row in sorted_rows if row["target"] > 0.5)
    if positives == 0:
        return None
    hit = 0
    precision_sum = 0.0
    for idx, row in enumerate(sorted_rows, start=1):
        if row["target"] > 0.5:
            hit += 1
            precision_sum += hit / idx
    return precision_sum / positives


def label_counts(examples: list[dict[str, Any]]) -> dict[str, int]:
    return {
        "positive": sum(1 for ex in examples if ex["target"] > 0.5),
        "negative": sum(1 for ex in examples if ex["target"] <= 0.5),
    }


def split_counts(examples: list[dict[str, Any]]) -> dict[str, int]:
    out = {"train": 0, "valid": 0, "test": 0}
    for ex in examples:
        if ex["split"] in out:
            out[ex["split"]] += 1
    return out


def parse_float_list(value: str) -> list[float]:
    return [float(item) for item in value.split(",") if item.strip()]


def parse_int_list(value: str) -> list[int]:
    return [int(item) for item in value.split(",") if item.strip()]


def mean(values: Any) -> float:
    values = list(values)
    return sum(values) / len(values) if values else 0.0


if __name__ == "__main__":
    main()
