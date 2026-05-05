#!/usr/bin/env python3
from __future__ import annotations

import argparse
import copy
import json
from pathlib import Path
from typing import Any

import numpy as np
import torch
from sklearn.feature_extraction import DictVectorizer
from sklearn.metrics import accuracy_score, mean_absolute_error
from sklearn.preprocessing import StandardScaler
from torch import nn
from torch.utils.data import DataLoader, TensorDataset

from combat_rl_common import REPO_ROOT, iter_jsonl, value_feature_dict, write_json


def load_rows(path: Path) -> list[dict[str, Any]]:
    return [row for _, row in iter_jsonl(path)]


def safe_corr(y_true: np.ndarray, y_pred: np.ndarray) -> float:
    if y_true.size == 0 or np.std(y_true) == 0 or np.std(y_pred) == 0:
        return 0.0
    return float(np.corrcoef(y_true, y_pred)[0, 1])


def brier_score(y_true: np.ndarray, y_prob: np.ndarray) -> float:
    if y_true.size == 0:
        return 0.0
    return float(np.mean((y_prob - y_true) ** 2))


class ValueNet(nn.Module):
    def __init__(self, in_features: int, hidden: int = 256) -> None:
        super().__init__()
        self.input = nn.Sequential(nn.Linear(in_features, hidden), nn.ReLU())
        self.block1 = nn.Sequential(nn.Linear(hidden, hidden), nn.ReLU(), nn.Dropout(0.1))
        self.block2 = nn.Sequential(nn.Linear(hidden, hidden), nn.ReLU(), nn.Dropout(0.1))
        self.discounted_head = nn.Linear(hidden, 1)
        self.short_head = nn.Linear(hidden, 1)
        self.survival_head = nn.Linear(hidden, 1)
        self.kill_head = nn.Linear(hidden, 1)

    def forward(self, x: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor]:
        hidden = self.input(x)
        hidden = hidden + self.block1(hidden)
        hidden = hidden + self.block2(hidden)
        return (
            self.discounted_head(hidden),
            self.short_head(hidden),
            self.survival_head(hidden),
            self.kill_head(hidden),
        )


def evaluate(
    model: ValueNet,
    x_scaled: np.ndarray,
    y_discounted: np.ndarray,
    y_short: np.ndarray,
    y_survival: np.ndarray,
    y_kill: np.ndarray,
    discounted_scaler: StandardScaler,
    short_scaler: StandardScaler,
    device: torch.device,
) -> dict[str, Any]:
    if len(x_scaled) == 0:
        return {"rows": 0}
    with torch.no_grad():
        tensor = torch.as_tensor(x_scaled, dtype=torch.float32, device=device)
        pred_discounted, pred_short, pred_survival, pred_kill = model(tensor)
        pred_discounted_np = discounted_scaler.inverse_transform(pred_discounted.cpu().numpy()).reshape(-1)
        pred_short_np = short_scaler.inverse_transform(pred_short.cpu().numpy()).reshape(-1)
        survival_prob = torch.sigmoid(pred_survival).cpu().numpy().reshape(-1)
        kill_prob = torch.sigmoid(pred_kill).cpu().numpy().reshape(-1)
        pred_survival_np = (survival_prob >= 0.5).astype(np.int32)
        pred_kill_np = (kill_prob >= 0.5).astype(np.int32)
    return {
        "rows": len(x_scaled),
        "discounted_return_mae": float(mean_absolute_error(y_discounted, pred_discounted_np)),
        "discounted_return_corr": safe_corr(y_discounted, pred_discounted_np),
        "short_horizon_return_mae": float(mean_absolute_error(y_short, pred_short_np)),
        "short_horizon_return_corr": safe_corr(y_short, pred_short_np),
        "survival_accuracy": float(accuracy_score(y_survival, pred_survival_np)),
        "survival_brier": brier_score(y_survival.astype(np.float32), survival_prob.astype(np.float32)),
        "kill_accuracy": float(accuracy_score(y_kill, pred_kill_np)),
        "kill_brier": brier_score(y_kill.astype(np.float32), kill_prob.astype(np.float32)),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Train a PyTorch combat value model.")
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--dataset-prefix", default="combat_value")
    parser.add_argument("--epochs", default=80, type=int)
    parser.add_argument("--batch-size", default=128, type=int)
    parser.add_argument("--device", default="cpu")
    parser.add_argument("--metrics-out", default=None, type=Path)
    parser.add_argument("--model-out", default=None, type=Path)
    args = parser.parse_args()

    metrics_out = args.metrics_out or (args.dataset_dir / "value_torch_metrics.json")
    model_out = args.model_out or (args.dataset_dir / "combat_value_torch_model.pt")
    baseline_metrics_path = args.dataset_dir / "value_metrics.json"
    train_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_train.jsonl")
    val_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_val.jsonl")
    test_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_test.jsonl")
    if not train_rows:
        raise SystemExit("no combat value rows found")

    vectorizer = DictVectorizer(sparse=False)
    x_train = vectorizer.fit_transform([value_feature_dict(row) for row in train_rows]).astype(np.float32)
    x_val = vectorizer.transform([value_feature_dict(row) for row in val_rows]).astype(np.float32)
    x_test = vectorizer.transform([value_feature_dict(row) for row in test_rows]).astype(np.float32)
    x_scaler = StandardScaler()
    x_train_scaled = x_scaler.fit_transform(x_train).astype(np.float32)
    x_val_scaled = x_scaler.transform(x_val).astype(np.float32)
    x_test_scaled = x_scaler.transform(x_test).astype(np.float32)

    y_train_discounted = np.asarray([float(row.get("discounted_return") or 0.0) for row in train_rows], dtype=np.float32)
    y_val_discounted = np.asarray([float(row.get("discounted_return") or 0.0) for row in val_rows], dtype=np.float32)
    y_test_discounted = np.asarray([float(row.get("discounted_return") or 0.0) for row in test_rows], dtype=np.float32)
    y_train_short = np.asarray([float(row.get("short_horizon_return") or 0.0) for row in train_rows], dtype=np.float32)
    y_val_short = np.asarray([float(row.get("short_horizon_return") or 0.0) for row in val_rows], dtype=np.float32)
    y_test_short = np.asarray([float(row.get("short_horizon_return") or 0.0) for row in test_rows], dtype=np.float32)
    y_train_survival = np.asarray([1 if row.get("survives_episode") else 0 for row in train_rows], dtype=np.float32)
    y_val_survival = np.asarray([1 if row.get("survives_episode") else 0 for row in val_rows], dtype=np.float32)
    y_test_survival = np.asarray([1 if row.get("survives_episode") else 0 for row in test_rows], dtype=np.float32)
    y_train_kill = np.asarray([1 if row.get("kill_within_horizon") else 0 for row in train_rows], dtype=np.float32)
    y_val_kill = np.asarray([1 if row.get("kill_within_horizon") else 0 for row in val_rows], dtype=np.float32)
    y_test_kill = np.asarray([1 if row.get("kill_within_horizon") else 0 for row in test_rows], dtype=np.float32)

    discounted_scaler = StandardScaler()
    short_scaler = StandardScaler()
    y_train_discounted_scaled = discounted_scaler.fit_transform(y_train_discounted.reshape(-1, 1)).astype(np.float32)
    y_val_discounted_scaled = discounted_scaler.transform(y_val_discounted.reshape(-1, 1)).astype(np.float32)
    y_train_short_scaled = short_scaler.fit_transform(y_train_short.reshape(-1, 1)).astype(np.float32)
    y_val_short_scaled = short_scaler.transform(y_val_short.reshape(-1, 1)).astype(np.float32)

    device = torch.device(args.device)
    model = ValueNet(x_train.shape[1]).to(device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=1e-3, weight_decay=1e-4)
    huber = nn.SmoothL1Loss()
    survival_pos_weight = float(max((len(y_train_survival) - y_train_survival.sum()) / max(y_train_survival.sum(), 1.0), 1.0))
    kill_pos_weight = float(max((len(y_train_kill) - y_train_kill.sum()) / max(y_train_kill.sum(), 1.0), 1.0))
    survival_bce = nn.BCEWithLogitsLoss(pos_weight=torch.tensor([survival_pos_weight], dtype=torch.float32, device=device))
    kill_bce = nn.BCEWithLogitsLoss(pos_weight=torch.tensor([kill_pos_weight], dtype=torch.float32, device=device))

    dataset = TensorDataset(
        torch.as_tensor(x_train_scaled),
        torch.as_tensor(y_train_discounted_scaled),
        torch.as_tensor(y_train_short_scaled),
        torch.as_tensor(y_train_survival).unsqueeze(1),
        torch.as_tensor(y_train_kill).unsqueeze(1),
    )
    loader = DataLoader(dataset, batch_size=args.batch_size, shuffle=True)
    best_state = copy.deepcopy(model.state_dict())
    best_val = float("inf")
    best_epoch = 0

    for epoch in range(args.epochs):
        model.train()
        for batch_x, batch_discounted, batch_short, batch_survival, batch_kill in loader:
            batch_x = batch_x.to(device=device, dtype=torch.float32)
            batch_discounted = batch_discounted.to(device=device, dtype=torch.float32)
            batch_short = batch_short.to(device=device, dtype=torch.float32)
            batch_survival = batch_survival.to(device=device, dtype=torch.float32)
            batch_kill = batch_kill.to(device=device, dtype=torch.float32)
            pred_discounted, pred_short, pred_survival, pred_kill = model(batch_x)
            loss = (
                huber(pred_discounted, batch_discounted)
                + huber(pred_short, batch_short)
                + 0.4 * survival_bce(pred_survival, batch_survival)
                + 0.4 * kill_bce(pred_kill, batch_kill)
            )
            optimizer.zero_grad()
            loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
        model.eval()
        with torch.no_grad():
            val_x = torch.as_tensor(x_val_scaled, dtype=torch.float32, device=device)
            pred_discounted, pred_short, pred_survival, pred_kill = model(val_x)
            val_loss = (
                huber(pred_discounted, torch.as_tensor(y_val_discounted_scaled, dtype=torch.float32, device=device))
                + huber(pred_short, torch.as_tensor(y_val_short_scaled, dtype=torch.float32, device=device))
                + 0.4 * survival_bce(pred_survival, torch.as_tensor(y_val_survival, dtype=torch.float32, device=device).unsqueeze(1))
                + 0.4 * kill_bce(pred_kill, torch.as_tensor(y_val_kill, dtype=torch.float32, device=device).unsqueeze(1))
            ).item()
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
            "discounted_scaler_mean": discounted_scaler.mean_.tolist(),
            "discounted_scaler_scale": discounted_scaler.scale_.tolist(),
            "short_scaler_mean": short_scaler.mean_.tolist(),
            "short_scaler_scale": short_scaler.scale_.tolist(),
        },
        model_out,
    )

    metrics = {
        "model": "torch_value_model",
        "dataset_prefix": args.dataset_prefix,
        "feature_count": x_train.shape[1],
        "epochs": args.epochs,
        "best_epoch": best_epoch,
        "train": evaluate(model, x_train_scaled, y_train_discounted, y_train_short, y_train_survival, y_train_kill, discounted_scaler, short_scaler, device),
        "val": evaluate(model, x_val_scaled, y_val_discounted, y_val_short, y_val_survival, y_val_kill, discounted_scaler, short_scaler, device),
        "test": evaluate(model, x_test_scaled, y_test_discounted, y_test_short, y_test_survival, y_test_kill, discounted_scaler, short_scaler, device),
        "notes": [
            "PyTorch value model uses standardized return targets and class-weighted survival/kill heads",
            "intended to beat or replace the sklearn value baseline, not the old reranker",
        ],
    }
    if baseline_metrics_path.exists():
        with baseline_metrics_path.open("r", encoding="utf-8") as handle:
            baseline = json.load(handle)
        metrics["baseline_compare"] = {
            "test_discounted_corr_delta_vs_sklearn": float(metrics["test"]["discounted_return_corr"] - baseline["test"]["discounted_return_corr"]),
            "test_short_corr_delta_vs_sklearn": float(metrics["test"]["short_horizon_return_corr"] - baseline["test"]["short_horizon_return_corr"]),
            "test_survival_acc_delta_vs_sklearn": float(metrics["test"]["survival_accuracy"] - baseline["test"]["survival_accuracy"]),
            "test_kill_acc_delta_vs_sklearn": float(metrics["test"]["kill_accuracy"] - baseline["test"]["kill_accuracy"]),
        }
    write_json(metrics_out, metrics)
    print(json.dumps(metrics, indent=2, ensure_ascii=False))
    print(f"wrote torch value metrics to {metrics_out}")
    print(f"saved torch value model to {model_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
