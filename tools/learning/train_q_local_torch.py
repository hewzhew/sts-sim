#!/usr/bin/env python3
from __future__ import annotations

import argparse
import copy
import json
from collections import defaultdict
from pathlib import Path
from typing import Any

import numpy as np
import torch
from sklearn.feature_extraction import DictVectorizer
from sklearn.preprocessing import StandardScaler
from torch import nn
from torch.utils.data import DataLoader, TensorDataset

from combat_rl_common import REPO_ROOT, iter_jsonl, write_json, write_jsonl
from q_local_common import aggregate_q_local_score, q_local_feature_dict

HEADS = [
    "survival_score",
    "tempo_score",
    "setup_payoff_score",
    "kill_window_score",
    "risk_score",
    "mean_return",
]


def load_rows(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [row for _, row in iter_jsonl(path)]


def safe_corr(y_true: np.ndarray, y_pred: np.ndarray) -> float:
    if y_true.size == 0 or np.std(y_true) == 0 or np.std(y_pred) == 0:
        return 0.0
    return float(np.corrcoef(y_true, y_pred)[0, 1])


class QLocalNet(nn.Module):
    def __init__(self, in_features: int, hidden: int = 256) -> None:
        super().__init__()
        self.trunk = nn.Sequential(
            nn.Linear(in_features, hidden),
            nn.ReLU(),
            nn.Linear(hidden, hidden),
            nn.ReLU(),
        )
        self.heads = nn.ModuleDict({name: nn.Linear(hidden, 1) for name in HEADS})

    def forward(self, x: torch.Tensor) -> dict[str, torch.Tensor]:
        hidden = self.trunk(x)
        return {name: layer(hidden) for name, layer in self.heads.items()}


def standardize_targets(rows: list[dict[str, Any]]) -> tuple[dict[str, np.ndarray], dict[str, StandardScaler]]:
    arrays = {
        head: np.asarray([float(row.get(head) or 0.0) for row in rows], dtype=np.float32).reshape(-1, 1)
        for head in HEADS
    }
    scalers = {head: StandardScaler().fit(values) for head, values in arrays.items()}
    transformed = {head: scalers[head].transform(values).astype(np.float32) for head, values in arrays.items()}
    return transformed, scalers


def transform_targets(rows: list[dict[str, Any]], scalers: dict[str, StandardScaler]) -> dict[str, np.ndarray]:
    return {
        head: scalers[head].transform(
            np.asarray([float(row.get(head) or 0.0) for row in rows], dtype=np.float32).reshape(-1, 1)
        ).astype(np.float32)
        for head in HEADS
    }


def inverse_predictions(predictions: dict[str, np.ndarray], scalers: dict[str, StandardScaler]) -> dict[str, np.ndarray]:
    return {
        head: scalers[head].inverse_transform(values.reshape(-1, 1)).reshape(-1)
        for head, values in predictions.items()
    }


def evaluate_heads(
    model: QLocalNet,
    x_scaled: np.ndarray,
    rows: list[dict[str, Any]],
    target_scalers: dict[str, StandardScaler],
    device: torch.device,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    if len(rows) == 0:
        return {"rows": 0}, []
    with torch.no_grad():
        tensor = torch.as_tensor(x_scaled, dtype=torch.float32, device=device)
        raw = {head: output.cpu().numpy().reshape(-1) for head, output in model(tensor).items()}
    preds = inverse_predictions(raw, target_scalers)
    metrics: dict[str, Any] = {"rows": len(rows)}
    prediction_rows: list[dict[str, Any]] = []
    for index, row in enumerate(rows):
        pred_row = {
            "group_id": row.get("group_id"),
            "candidate_move": row.get("candidate_move"),
            "baseline_action": row.get("baseline_action"),
            "curriculum_tag": row.get("curriculum_tag"),
            "eval_bucket": row.get("eval_bucket"),
            "sample_origin": row.get("sample_origin"),
            "key_kind": row.get("key_kind"),
            "uncertain": bool(row.get("uncertain")),
            "candidate_is_best": bool(row.get("candidate_is_best")),
            "candidate_is_teacher_top": bool(
                row.get("candidate_is_teacher_top", row.get("candidate_is_best"))
            ),
            "candidate_rank": row.get("candidate_rank"),
            "candidate_score_hint": float(row.get("candidate_score_hint") or 0.0),
            "split": row.get("split"),
        }
        for head in HEADS:
            pred_row[f"pred::{head}"] = float(preds[head][index])
        pred_row["pred::aggregate"] = aggregate_q_local_score(
            {
                "survival_score": pred_row["pred::survival_score"],
                "tempo_score": pred_row["pred::tempo_score"],
                "setup_payoff_score": pred_row["pred::setup_payoff_score"],
                "kill_window_score": pred_row["pred::kill_window_score"],
                "risk_score": pred_row["pred::risk_score"],
                "mean_return": pred_row["pred::mean_return"],
            }
        )
        prediction_rows.append(pred_row)
    for head in HEADS:
        y_true = np.asarray([float(row.get(head) or 0.0) for row in rows], dtype=np.float32)
        y_pred = preds[head].astype(np.float32)
        metrics[f"{head}_corr"] = safe_corr(y_true, y_pred)
        metrics[f"{head}_mae"] = float(np.mean(np.abs(y_true - y_pred)))
    risk_true = np.asarray([float(row.get("risk_score") or 0.0) for row in rows], dtype=np.float32)
    risk_pred = preds["risk_score"].astype(np.float32)
    metrics["risk_head_calibration"] = float(np.mean((risk_true - risk_pred) ** 2))
    return metrics, prediction_rows


def root_ordering_metrics(
    prediction_rows: list[dict[str, Any]],
    *,
    key_kind: str | None = None,
    include_uncertain: bool = False,
) -> dict[str, Any]:
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in prediction_rows:
        if key_kind and str(row.get("key_kind") or "") != key_kind:
            continue
        groups[str(row.get("group_id"))].append(row)
    baseline_correct = 0
    qlocal_correct = 0
    eligible = 0
    retained_counts = []
    legal_counts = []
    bucket_counts: dict[str, dict[str, float]] = defaultdict(lambda: {"groups": 0, "baseline_correct": 0, "qlocal_correct": 0})
    for group_rows in groups.values():
        if not group_rows:
            continue
        if (not include_uncertain) and any(bool(row.get("uncertain")) for row in group_rows):
            continue
        positives = [row for row in group_rows if bool(row.get("candidate_is_teacher_top", row.get("candidate_is_best")))]
        if not positives:
            continue
        eligible += 1
        baseline_sorted = sorted(group_rows, key=lambda row: (int(row.get("candidate_rank") or 10_000), -float(row.get("candidate_score_hint") or 0.0)))
        qlocal_sorted = sorted(group_rows, key=lambda row: float(row.get("pred::aggregate") or 0.0), reverse=True)
        if bool(baseline_sorted[0].get("candidate_is_teacher_top", baseline_sorted[0].get("candidate_is_best"))):
            baseline_correct += 1
        if bool(qlocal_sorted[0].get("candidate_is_teacher_top", qlocal_sorted[0].get("candidate_is_best"))):
            qlocal_correct += 1
        threshold = float(qlocal_sorted[0].get("pred::aggregate") or 0.0) - 0.35
        retained = sum(1 for row in qlocal_sorted if float(row.get("pred::aggregate") or 0.0) >= threshold)
        retained_counts.append(retained)
        legal_counts.append(len(group_rows))
        bucket = str(qlocal_sorted[0].get("eval_bucket") or "unknown")
        bucket_counts[bucket]["groups"] += 1
        bucket_counts[bucket]["baseline_correct"] += 1 if bool(baseline_sorted[0].get("candidate_is_teacher_top", baseline_sorted[0].get("candidate_is_best"))) else 0
        bucket_counts[bucket]["qlocal_correct"] += 1 if bool(qlocal_sorted[0].get("candidate_is_teacher_top", qlocal_sorted[0].get("candidate_is_best"))) else 0
    by_bucket = {}
    for bucket, stats in bucket_counts.items():
        groups_count = max(int(stats["groups"]), 1)
        by_bucket[bucket] = {
            "groups": int(stats["groups"]),
            "baseline_top1_match": float(stats["baseline_correct"] / groups_count),
            "q_local_top1_match": float(stats["qlocal_correct"] / groups_count),
            "improvement": float((stats["qlocal_correct"] - stats["baseline_correct"]) / groups_count),
        }
    avg_retained = float(np.mean(retained_counts)) if retained_counts else 0.0
    avg_legal = float(np.mean(legal_counts)) if legal_counts else 0.0
    return {
        "eligible_groups": eligible,
        "baseline_top1_match": float(baseline_correct / eligible) if eligible else 0.0,
        "q_local_top1_match": float(qlocal_correct / eligible) if eligible else 0.0,
        "root_ordering_improvement": float((qlocal_correct - baseline_correct) / eligible) if eligible else 0.0,
        "avg_retained_candidates": avg_retained,
        "avg_legal_candidates": avg_legal,
        "search_node_reduction": float(1.0 - (avg_retained / avg_legal)) if avg_legal else 0.0,
        "by_bucket": by_bucket,
    }


def filter_rows_and_predictions(
    rows: list[dict[str, Any]],
    prediction_rows: list[dict[str, Any]],
    *,
    key_kind: str | None = None,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    if key_kind is None:
        return rows, prediction_rows
    filtered_rows = [row for row in rows if str(row.get("key_kind") or "") == key_kind]
    groups = {str(row.get("group_id") or "") for row in filtered_rows}
    filtered_predictions = [
        row for row in prediction_rows if str(row.get("group_id") or "") in groups
    ]
    return filtered_rows, filtered_predictions


def main() -> int:
    parser = argparse.ArgumentParser(description="Train a multi-head Q_local tactical value model.")
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--dataset-prefix", default="combat_q_local")
    parser.add_argument("--epochs", default=60, type=int)
    parser.add_argument("--batch-size", default=128, type=int)
    parser.add_argument("--device", default="cpu")
    parser.add_argument("--metrics-out", default=None, type=Path)
    parser.add_argument("--predictions-out", default=None, type=Path)
    parser.add_argument("--model-out", default=None, type=Path)
    args = parser.parse_args()

    metrics_out = args.metrics_out or (args.dataset_dir / "q_local_metrics.json")
    predictions_out = args.predictions_out or (args.dataset_dir / "q_local_predictions.jsonl")
    model_out = args.model_out or (args.dataset_dir / "combat_q_local_torch_model.pt")

    train_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_train.jsonl")
    val_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_val.jsonl")
    test_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_test.jsonl")
    if not train_rows:
        raise SystemExit("no Q_local rows found")

    vectorizer = DictVectorizer(sparse=False)
    x_train = vectorizer.fit_transform([q_local_feature_dict(row) for row in train_rows]).astype(np.float32)
    x_val = vectorizer.transform([q_local_feature_dict(row) for row in val_rows]).astype(np.float32)
    x_test = vectorizer.transform([q_local_feature_dict(row) for row in test_rows]).astype(np.float32)
    x_scaler = StandardScaler()
    x_train_scaled = x_scaler.fit_transform(x_train).astype(np.float32)
    x_val_scaled = x_scaler.transform(x_val).astype(np.float32)
    x_test_scaled = x_scaler.transform(x_test).astype(np.float32)

    y_train, target_scalers = standardize_targets(train_rows)
    y_val = transform_targets(val_rows, target_scalers)
    device = torch.device(args.device)
    model = QLocalNet(x_train.shape[1]).to(device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=8e-4, weight_decay=1e-4)
    loss_fn = nn.SmoothL1Loss()

    dataset = TensorDataset(
        torch.as_tensor(x_train_scaled),
        torch.as_tensor(np.asarray([float(row.get("training_weight") or 1.0) for row in train_rows], dtype=np.float32)).unsqueeze(1),
        *[torch.as_tensor(y_train[head]) for head in HEADS],
    )
    loader = DataLoader(dataset, batch_size=args.batch_size, shuffle=True)
    best_state = copy.deepcopy(model.state_dict())
    best_val = float("inf")
    best_epoch = 0

    for epoch in range(args.epochs):
        model.train()
        for batch in loader:
            batch_x = batch[0].to(device=device, dtype=torch.float32)
            batch_weight = batch[1].to(device=device, dtype=torch.float32)
            outputs = model(batch_x)
            loss = torch.tensor(0.0, device=device)
            for head_index, head in enumerate(HEADS, start=2):
                target = batch[head_index].to(device=device, dtype=torch.float32)
                head_loss = torch.nn.functional.smooth_l1_loss(outputs[head], target, reduction="none")
                loss = loss + (head_loss * batch_weight).mean()
            optimizer.zero_grad()
            loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
        model.eval()
        with torch.no_grad():
            val_x = torch.as_tensor(x_val_scaled, dtype=torch.float32, device=device)
            outputs = model(val_x)
            val_loss = 0.0
            for head in HEADS:
                target = torch.as_tensor(y_val[head], dtype=torch.float32, device=device)
                val_loss += float(loss_fn(outputs[head], target).item())
        if val_loss < best_val:
            best_val = val_loss
            best_state = copy.deepcopy(model.state_dict())
            best_epoch = epoch + 1

    model.load_state_dict(best_state)
    torch.save(
        {
            "state_dict": model.state_dict(),
            "vectorizer_feature_names": vectorizer.feature_names_,
            "input_dim": x_train.shape[1],
            "x_scaler_mean": x_scaler.mean_.tolist(),
            "x_scaler_scale": x_scaler.scale_.tolist(),
            "target_scaler_mean": {head: scaler.mean_.tolist() for head, scaler in target_scalers.items()},
            "target_scaler_scale": {head: scaler.scale_.tolist() for head, scaler in target_scalers.items()},
            "heads": HEADS,
        },
        model_out,
    )

    train_metrics, train_preds = evaluate_heads(model, x_train_scaled, train_rows, target_scalers, device)
    val_metrics, val_preds = evaluate_heads(model, x_val_scaled, val_rows, target_scalers, device)
    test_metrics, test_preds = evaluate_heads(model, x_test_scaled, test_rows, target_scalers, device)
    ordering_metrics = root_ordering_metrics(test_preds)
    replay_test_rows, replay_test_preds = filter_rows_and_predictions(
        test_rows, test_preds, key_kind="replay_frame"
    )
    replay_test_metrics = {}
    if replay_test_rows:
        replay_test_metrics, _ = evaluate_heads(
            model,
            x_test_scaled[[index for index, row in enumerate(test_rows) if str(row.get("key_kind") or "") == "replay_frame"]],
            replay_test_rows,
            target_scalers,
            device,
        )
    replay_ordering_metrics = root_ordering_metrics(
        replay_test_preds,
        key_kind="replay_frame",
        include_uncertain=True,
    )

    metrics = {
        "model": "q_local_torch",
        "dataset_prefix": args.dataset_prefix,
        "feature_count": x_train.shape[1],
        "epochs": args.epochs,
        "best_epoch": best_epoch,
        "heads": HEADS,
        "train": train_metrics,
        "val": val_metrics,
        "test": test_metrics,
        "root_ordering_eval": ordering_metrics,
        "test_replay": replay_test_metrics,
        "root_ordering_eval_replay": replay_ordering_metrics,
        "notes": [
            "Q_local is a multi-head tactical evaluator over local rollout-oracle targets",
            "root ordering metrics are offline prior comparisons against current search candidate order",
            "search_node_reduction is an estimated selective-expansion proxy, not a live runtime node count",
            "replay-only metrics include uncertain groups and use candidate_is_teacher_top labels",
        ],
    }
    all_predictions = train_preds + val_preds + test_preds
    write_json(metrics_out, metrics)
    write_jsonl(predictions_out, all_predictions)
    print(json.dumps(metrics, indent=2, ensure_ascii=False))
    print(f"wrote Q_local metrics to {metrics_out}")
    print(f"wrote Q_local predictions to {predictions_out}")
    print(f"saved Q_local model to {model_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
