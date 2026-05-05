#!/usr/bin/env python3
from __future__ import annotations

import argparse
import copy
import json
import random
from collections import defaultdict
from pathlib import Path
from typing import Any

import numpy as np
import torch
from sklearn.feature_extraction import DictVectorizer
from sklearn.preprocessing import StandardScaler
from torch import nn

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


class MacroChoiceNet(nn.Module):
    def __init__(self, in_features: int, hidden: int = 192, dropout: float = 0.1) -> None:
        super().__init__()
        self.encoder = nn.Sequential(
            nn.Linear(in_features, hidden),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(hidden, hidden),
            nn.ReLU(),
            nn.Dropout(dropout),
        )
        self.score_head = nn.Linear(hidden, 1)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        hidden = self.encoder(x)
        return self.score_head(hidden).squeeze(-1)


def load_rows(path: Path) -> list[dict[str, Any]]:
    return iter_jsonl_rows(path) if path.exists() else []


def split_decision_ids(option_rows: list[dict[str, Any]]) -> dict[str, str]:
    decision_ids = sorted({str(row.get("decision_id") or "") for row in option_rows})
    return {decision_id: stable_split(decision_id) for decision_id in decision_ids}


def build_pairwise_targets(
    option_rows: list[dict[str, Any]],
    edge_rows: list[dict[str, Any]],
) -> tuple[dict[str, dict[str, float]], dict[str, set[str]], dict[str, list[dict[str, Any]]]]:
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
    return target_scores, positive_sets, grouped_edges


def make_option_lookup(option_rows: list[dict[str, Any]]) -> dict[tuple[str, str], dict[str, Any]]:
    return {
        (str(row.get("decision_id") or ""), str(row.get("option_id") or "")): row
        for row in option_rows
    }


def baseline_score(row: dict[str, Any]) -> float:
    return 1.0 if bool(row.get("baseline_matches_option")) else 0.0


def encode_option_rows(
    option_rows: list[dict[str, Any]],
    *,
    vectorizer: DictVectorizer,
    scaler: StandardScaler,
    fit: bool,
) -> tuple[np.ndarray, list[dict[str, Any]]]:
    grouped = group_option_rows(option_rows)
    ordered_rows: list[dict[str, Any]] = []
    for decision_id in sorted(grouped):
        ordered_rows.extend(grouped[decision_id])
    feature_dicts = [macro_option_feature_dict(row) for row in ordered_rows]
    matrix = vectorizer.fit_transform(feature_dicts).astype(np.float32) if fit else vectorizer.transform(feature_dicts).astype(np.float32)
    dense = matrix.toarray() if hasattr(matrix, "toarray") else np.asarray(matrix, dtype=np.float32)
    scaled = scaler.fit_transform(dense).astype(np.float32) if fit else scaler.transform(dense).astype(np.float32)
    return scaled, ordered_rows


def build_group_examples(
    *,
    option_rows: list[dict[str, Any]],
    scaled_features: np.ndarray,
    target_scores: dict[str, dict[str, float]],
    positive_sets: dict[str, set[str]],
) -> list[dict[str, Any]]:
    grouped = group_option_rows(option_rows)
    row_by_key = {
        (str(row.get("decision_id") or ""), str(row.get("option_id") or "")): (index, row)
        for index, row in enumerate(option_rows)
    }
    examples: list[dict[str, Any]] = []
    for decision_id in sorted(grouped):
        rows = grouped[decision_id]
        feature_rows = []
        target = np.zeros(len(rows), dtype=np.float32)
        option_ids = []
        for idx, row in enumerate(rows):
            key = (decision_id, str(row.get("option_id") or ""))
            source_index, _ = row_by_key[key]
            feature_rows.append(scaled_features[source_index])
            option_id = str(row.get("option_id") or "")
            option_ids.append(option_id)
            if option_id in positive_sets.get(decision_id, set()):
                target[idx] = 1.0
        if target.sum() <= 0:
            continue
        target /= target.sum()
        score_values = sorted(
            (float((target_scores.get(decision_id) or {}).get(option_id, 0.0)) for option_id in option_ids),
            reverse=True,
        )
        top_score = score_values[0] if score_values else 0.0
        second_score = next((value for value in score_values if value < top_score - 1e-6), top_score)
        examples.append(
            {
                "decision_id": decision_id,
                "rows": rows,
                "x": np.stack(feature_rows, axis=0).astype(np.float32),
                "target": target.astype(np.float32),
                "positive_option_ids": positive_sets.get(decision_id, set()),
                "decision_weight": 1.0 + max(top_score - second_score, 0.0) * 0.1,
            }
        )
    return examples


def score_fn_factory(
    model: MacroChoiceNet,
    vectorizer: DictVectorizer,
    scaler: StandardScaler,
    device: torch.device,
):
    cache: dict[tuple[str, str], float] = {}

    def score(row: dict[str, Any]) -> float:
        key = (str(row.get("decision_id") or ""), str(row.get("option_id") or ""))
        cached = cache.get(key)
        if cached is not None:
            return cached
        features = vectorizer.transform([macro_option_feature_dict(row)]).astype(np.float32)
        dense = features.toarray() if hasattr(features, "toarray") else np.asarray(features, dtype=np.float32)
        scaled = scaler.transform(dense).astype(np.float32)
        tensor = torch.as_tensor(scaled, dtype=torch.float32, device=device)
        with torch.no_grad():
            value = float(model(tensor).item())
        cache[key] = value
        return value

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
    probability_fn,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    grouped = group_option_rows(option_rows)
    predictions: list[dict[str, Any]] = []
    decision_count = 0
    top1_match = 0
    baseline_top1_match = 0
    choice_nll_total = 0.0
    positive_mass_total = 0.0
    source_correct: dict[str, int] = defaultdict(int)
    source_total: dict[str, int] = defaultdict(int)
    source_baseline_correct: dict[str, int] = defaultdict(int)

    for decision_id, rows in grouped.items():
        decision_count += 1
        source_kind = str(rows[0].get("source_kind") or "unknown")
        source_total[source_kind] += 1
        probabilities = probability_fn(rows)
        scored_rows = sorted(
            (
                {
                    "option_id": str(row.get("option_id") or ""),
                    "label": str(row.get("label") or ""),
                    "option_kind": str(row.get("option_kind") or ""),
                    "score": round(float(score_fn(row)), 6),
                    "probability": round(float(probabilities[str(row.get("option_id") or "")]), 6),
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

        positive_ids = positive_sets.get(decision_id, set())
        positive_mass = sum(item["probability"] for item in scored_rows if item["option_id"] in positive_ids)
        positive_mass_total += positive_mass
        target_prob = 1.0 / max(len(positive_ids), 1)
        choice_nll = 0.0
        for item in scored_rows:
            if item["option_id"] in positive_ids:
                choice_nll += -target_prob * np.log(max(item["probability"], 1e-8))
        choice_nll_total += choice_nll

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
                "positive_option_ids": sorted(positive_ids),
                "top1_match": top1_hit,
                "baseline_top1_match": baseline_hit,
                "positive_mass": round(float(positive_mass), 6),
                "choice_nll": round(float(choice_nll), 6),
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
        "mean_choice_nll": round(choice_nll_total / float(max(decision_count, 1)), 6),
        "mean_positive_mass": round(positive_mass_total / float(max(decision_count, 1)), 6),
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
    probability_fn,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    pairwise = evaluate_pairwise(edge_rows, option_lookup, score_fn)
    decision_metrics, predictions = evaluate_decisions(
        option_rows=option_rows,
        target_scores=target_scores,
        positive_sets=positive_sets,
        score_fn=score_fn,
        probability_fn=probability_fn,
    )
    return {
        "split": name,
        "option_rows": len(option_rows),
        "edge_rows": len(edge_rows),
        **pairwise,
        **decision_metrics,
    }, predictions


def probability_fn_factory(
    model: MacroChoiceNet,
    vectorizer: DictVectorizer,
    scaler: StandardScaler,
    device: torch.device,
):
    cache: dict[str, dict[str, float]] = {}

    def probability(rows: list[dict[str, Any]]) -> dict[str, float]:
        decision_id = str(rows[0].get("decision_id") or "")
        cached = cache.get(decision_id)
        if cached is not None:
            return cached
        features = vectorizer.transform([macro_option_feature_dict(row) for row in rows]).astype(np.float32)
        dense = features.toarray() if hasattr(features, "toarray") else np.asarray(features, dtype=np.float32)
        scaled = scaler.transform(dense).astype(np.float32)
        tensor = torch.as_tensor(scaled, dtype=torch.float32, device=device)
        with torch.no_grad():
            logits = model(tensor)
            probs = torch.softmax(logits, dim=0).cpu().numpy()
        result = {
            str(row.get("option_id") or ""): float(prob)
            for row, prob in zip(rows, probs, strict=False)
        }
        cache[decision_id] = result
        return result

    return probability


def choice_loss_for_group(
    logits: torch.Tensor,
    target: torch.Tensor,
    *,
    pairwise_aux_weight: float,
) -> tuple[torch.Tensor, dict[str, float]]:
    log_probs = torch.log_softmax(logits, dim=0)
    choice_loss = -(target * log_probs).sum()
    pairwise_loss = torch.tensor(0.0, device=logits.device)
    positive_mask = target > 0.0
    negative_mask = ~positive_mask
    if pairwise_aux_weight > 0 and positive_mask.any() and negative_mask.any():
        pos_logits = logits[positive_mask]
        neg_logits = logits[negative_mask]
        pairwise_loss = torch.nn.functional.softplus(-(pos_logits.unsqueeze(1) - neg_logits.unsqueeze(0))).mean()
    total = choice_loss + pairwise_aux_weight * pairwise_loss
    return total, {
        "choice_loss": float(choice_loss.item()),
        "pairwise_loss": float(pairwise_loss.item()) if pairwise_aux_weight > 0 else 0.0,
    }


def validate_groups(
    model: MacroChoiceNet,
    examples: list[dict[str, Any]],
    device: torch.device,
    pairwise_aux_weight: float,
) -> dict[str, float]:
    if not examples:
        return {"mean_total_loss": 0.0, "mean_choice_loss": 0.0, "mean_pairwise_loss": 0.0}
    model.eval()
    total_loss = 0.0
    choice_loss = 0.0
    pair_loss = 0.0
    with torch.no_grad():
        for example in examples:
            x = torch.as_tensor(example["x"], dtype=torch.float32, device=device)
            target = torch.as_tensor(example["target"], dtype=torch.float32, device=device)
            logits = model(x)
            loss, parts = choice_loss_for_group(logits, target, pairwise_aux_weight=pairwise_aux_weight)
            total_loss += float(loss.item())
            choice_loss += parts["choice_loss"]
            pair_loss += parts["pairwise_loss"]
    count = float(len(examples))
    return {
        "mean_total_loss": round(total_loss / count, 6),
        "mean_choice_loss": round(choice_loss / count, 6),
        "mean_pairwise_loss": round(pair_loss / count, 6),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Train a grouped macro counterfactual choice model.")
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--dataset-prefix", default="macro_counterfactual_pilot")
    parser.add_argument("--output-prefix", default="macro_counterfactual_choice_torch")
    parser.add_argument("--metrics-out", default=None, type=Path)
    parser.add_argument("--predictions-out", default=None, type=Path)
    parser.add_argument("--review-out", default=None, type=Path)
    parser.add_argument("--model-out", default=None, type=Path)
    parser.add_argument("--epochs", default=180, type=int)
    parser.add_argument("--hidden", default=192, type=int)
    parser.add_argument("--dropout", default=0.1, type=float)
    parser.add_argument("--lr", default=8e-4, type=float)
    parser.add_argument("--weight-decay", default=1e-4, type=float)
    parser.add_argument("--pairwise-aux-weight", default=0.2, type=float)
    parser.add_argument("--device", default="cpu")
    args = parser.parse_args()

    options_path = args.dataset_dir / f"{args.dataset_prefix}_options.jsonl"
    pairwise_path = args.dataset_dir / f"{args.dataset_prefix}_pairwise.jsonl"
    option_rows = load_rows(options_path)
    edge_rows = load_rows(pairwise_path)
    if not option_rows or not edge_rows:
        raise SystemExit(f"missing macro counterfactual dataset rows for prefix '{args.dataset_prefix}'")

    split_map = split_decision_ids(option_rows)
    option_lookup = make_option_lookup(option_rows)
    target_scores, positive_sets, _grouped_edges = build_pairwise_targets(option_rows, edge_rows)

    train_option_rows = [row for row in option_rows if split_map[str(row.get("decision_id") or "")] == "train"]
    val_option_rows = [row for row in option_rows if split_map[str(row.get("decision_id") or "")] == "val"]
    test_option_rows = [row for row in option_rows if split_map[str(row.get("decision_id") or "")] == "test"]
    train_edge_rows = [row for row in edge_rows if split_map[str(row.get("decision_id") or "")] == "train"]
    val_edge_rows = [row for row in edge_rows if split_map[str(row.get("decision_id") or "")] == "val"]
    test_edge_rows = [row for row in edge_rows if split_map[str(row.get("decision_id") or "")] == "test"]

    vectorizer = DictVectorizer(sparse=True)
    scaler = StandardScaler()
    x_train_scaled, train_rows_ordered = encode_option_rows(train_option_rows, vectorizer=vectorizer, scaler=scaler, fit=True)
    x_val_scaled, val_rows_ordered = encode_option_rows(val_option_rows, vectorizer=vectorizer, scaler=scaler, fit=False)
    x_test_scaled, test_rows_ordered = encode_option_rows(test_option_rows, vectorizer=vectorizer, scaler=scaler, fit=False)

    train_examples = build_group_examples(
        option_rows=train_rows_ordered,
        scaled_features=x_train_scaled,
        target_scores=target_scores,
        positive_sets=positive_sets,
    )
    val_examples = build_group_examples(
        option_rows=val_rows_ordered,
        scaled_features=x_val_scaled,
        target_scores=target_scores,
        positive_sets=positive_sets,
    )
    test_examples = build_group_examples(
        option_rows=test_rows_ordered,
        scaled_features=x_test_scaled,
        target_scores=target_scores,
        positive_sets=positive_sets,
    )

    device = torch.device(args.device)
    model = MacroChoiceNet(x_train_scaled.shape[1], hidden=int(args.hidden), dropout=float(args.dropout)).to(device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=float(args.lr), weight_decay=float(args.weight_decay))

    best_state = copy.deepcopy(model.state_dict())
    best_val = float("inf")
    best_epoch = 0
    train_rng = random.Random(0)

    for epoch in range(int(args.epochs)):
        model.train()
        shuffled = train_examples.copy()
        train_rng.shuffle(shuffled)
        for example in shuffled:
            x = torch.as_tensor(example["x"], dtype=torch.float32, device=device)
            target = torch.as_tensor(example["target"], dtype=torch.float32, device=device)
            logits = model(x)
            loss, _parts = choice_loss_for_group(
                logits,
                target,
                pairwise_aux_weight=float(args.pairwise_aux_weight),
            )
            weighted_loss = loss * float(example.get("decision_weight") or 1.0)
            optimizer.zero_grad()
            weighted_loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()

        val_loss = validate_groups(
            model,
            val_examples,
            device,
            pairwise_aux_weight=float(args.pairwise_aux_weight),
        )["mean_total_loss"]
        if val_loss < best_val:
            best_val = val_loss
            best_state = copy.deepcopy(model.state_dict())
            best_epoch = epoch + 1

    model.load_state_dict(best_state)

    score_fn = score_fn_factory(model, vectorizer, scaler, device)
    probability_fn = probability_fn_factory(model, vectorizer, scaler, device)
    baseline_fn = baseline_score

    train_metrics, _ = evaluate_split(
        name="train",
        option_rows=train_option_rows,
        edge_rows=train_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        score_fn=score_fn,
        probability_fn=probability_fn,
    )
    val_metrics, _ = evaluate_split(
        name="val",
        option_rows=val_option_rows,
        edge_rows=val_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        score_fn=score_fn,
        probability_fn=probability_fn,
    )
    test_metrics, test_predictions = evaluate_split(
        name="test",
        option_rows=test_option_rows,
        edge_rows=test_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        score_fn=score_fn,
        probability_fn=probability_fn,
    )
    train_baseline_metrics, _ = evaluate_split(
        name="train",
        option_rows=train_option_rows,
        edge_rows=train_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        score_fn=baseline_fn,
        probability_fn=lambda rows: {
            str(row.get("option_id") or ""): (1.0 if bool(row.get("baseline_matches_option")) else 0.0)
            for row in rows
        },
    )
    val_baseline_metrics, _ = evaluate_split(
        name="val",
        option_rows=val_option_rows,
        edge_rows=val_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        score_fn=baseline_fn,
        probability_fn=lambda rows: {
            str(row.get("option_id") or ""): (1.0 if bool(row.get("baseline_matches_option")) else 0.0)
            for row in rows
        },
    )
    test_baseline_metrics, baseline_predictions = evaluate_split(
        name="test",
        option_rows=test_option_rows,
        edge_rows=test_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        score_fn=baseline_fn,
        probability_fn=lambda rows: {
            str(row.get("option_id") or ""): (1.0 if bool(row.get("baseline_matches_option")) else 0.0)
            for row in rows
        },
    )

    metrics_out = args.metrics_out or (args.dataset_dir / f"{args.output_prefix}_metrics.json")
    predictions_out = args.predictions_out or (args.dataset_dir / f"{args.output_prefix}_predictions.jsonl")
    review_out = args.review_out or (args.dataset_dir / f"{args.output_prefix}_review.json")
    model_out = args.model_out or (args.dataset_dir / f"{args.output_prefix}_model.pt")

    torch.save(
        {
            "state_dict": model.state_dict(),
            "input_dim": int(x_train_scaled.shape[1]),
            "hidden": int(args.hidden),
            "dropout": float(args.dropout),
            "vectorizer_feature_names": vectorizer.feature_names_,
            "x_scaler_mean": scaler.mean_.tolist(),
            "x_scaler_scale": scaler.scale_.tolist(),
        },
        model_out,
    )

    metrics = {
        "model": "macro_grouped_choice_torch",
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
        "train_decision_examples": len(train_examples),
        "val_decision_examples": len(val_examples),
        "test_decision_examples": len(test_examples),
        "best_epoch": best_epoch,
        "best_val_loss": best_val,
        "train_loss_snapshot": validate_groups(
            model,
            train_examples,
            device,
            pairwise_aux_weight=float(args.pairwise_aux_weight),
        ),
        "val_loss_snapshot": validate_groups(
            model,
            val_examples,
            device,
            pairwise_aux_weight=float(args.pairwise_aux_weight),
        ),
        "test_loss_snapshot": validate_groups(
            model,
            test_examples,
            device,
            pairwise_aux_weight=float(args.pairwise_aux_weight),
        ),
        "train": train_metrics,
        "val": val_metrics,
        "test": test_metrics,
        "baseline_controls": {
            "train": train_baseline_metrics,
            "val": val_baseline_metrics,
            "test": test_baseline_metrics,
        },
        "config": {
            "epochs": int(args.epochs),
            "hidden": int(args.hidden),
            "dropout": float(args.dropout),
            "lr": float(args.lr),
            "weight_decay": float(args.weight_decay),
            "pairwise_aux_weight": float(args.pairwise_aux_weight),
        },
        "notes": [
            "grouped choice model trains on one macro state at a time with a softmax over legal options",
            "targets are uniform over the top macro counterfactual bucket inside each decision graph",
            "a light pairwise auxiliary term is kept to preserve ordering pressure between positive and negative options",
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
            "hard_mistakes": top_scoring_macro_mistakes(test_predictions),
            "baseline_control_mistakes": top_scoring_macro_mistakes(baseline_predictions),
        },
    )

    print(json.dumps(metrics, indent=2, ensure_ascii=False))
    print(f"wrote macro grouped choice metrics to {metrics_out}")
    print(f"wrote macro grouped choice predictions to {predictions_out}")
    print(f"wrote macro grouped choice model to {model_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
