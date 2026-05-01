#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import time
from pathlib import Path
from typing import Any

import numpy as np
import torch
from torch import nn

from build_structured_state_evaluator_dataset import BINARY_TARGETS, REGRESSION_TARGETS
from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from structured_combat_env import (
    CARD_ID_VOCAB,
    INTENT_KIND_IDS,
    MONSTER_ID_VOCAB,
    POTION_ID_VOCAB,
    POWER_ID_VOCAB,
)
from structured_policy import StructuredPolicyNet, to_device_obs
from train_structured_combat_ppo import index_obs


def safe_corr(y_true: np.ndarray, y_pred: np.ndarray) -> float:
    if y_true.size == 0 or np.std(y_true) == 0.0 or np.std(y_pred) == 0.0:
        return 0.0
    return float(np.corrcoef(y_true, y_pred)[0, 1])


def load_state_evaluator_dataset(path: Path) -> dict[str, Any]:
    with np.load(path, allow_pickle=False) as payload:
        obs = {
            key.removeprefix("obs__"): np.asarray(payload[key])
            for key in payload.files
            if key.startswith("obs__")
        }
        targets = {
            key.removeprefix("target__"): np.asarray(payload[key], dtype=np.float32)
            for key in payload.files
            if key.startswith("target__")
        }
    if not obs:
        raise SystemExit(f"state evaluator dataset has no obs arrays: {path}")
    missing = sorted(set(REGRESSION_TARGETS + BINARY_TARGETS) - set(targets))
    if missing:
        raise SystemExit(f"state evaluator dataset is missing targets {missing}: {path}")
    count = next(iter(obs.values())).shape[0]
    if any(value.shape[0] != count for value in obs.values()):
        raise SystemExit(f"state evaluator dataset obs arrays have inconsistent sample counts: {path}")
    if any(value.shape[0] != count for value in targets.values()):
        raise SystemExit(f"state evaluator dataset target arrays have inconsistent sample counts: {path}")
    return {"obs": obs, "targets": targets, "count": count}


def target_matrix(targets: dict[str, np.ndarray], names: list[str], indices: np.ndarray) -> np.ndarray:
    return np.stack([targets[name][indices] for name in names], axis=-1).astype(np.float32)


def batch_from_dataset(
    dataset: dict[str, Any],
    indices: np.ndarray,
    device: torch.device,
    regression_mean: torch.Tensor,
    regression_std: torch.Tensor,
) -> dict[str, Any]:
    regression = torch.as_tensor(
        target_matrix(dataset["targets"], REGRESSION_TARGETS, indices),
        device=device,
    ).float()
    binary = torch.as_tensor(
        target_matrix(dataset["targets"], BINARY_TARGETS, indices),
        device=device,
    ).float()
    return {
        "obs": to_device_obs(index_obs(dataset["obs"], indices), device),
        "regression_raw": regression,
        "regression": (regression - regression_mean) / regression_std.clamp_min(1e-6),
        "binary": binary,
    }


class StructuredStateEvaluator(nn.Module):
    def __init__(
        self,
        *,
        card_vocab: int,
        potion_vocab: int,
        power_vocab: int,
        monster_vocab: int,
        intent_vocab: int,
        latent_dim: int = 32,
    ) -> None:
        super().__init__()
        self.state_encoder = StructuredPolicyNet(
            card_vocab=card_vocab,
            potion_vocab=potion_vocab,
            power_vocab=power_vocab,
            monster_vocab=monster_vocab,
            intent_vocab=intent_vocab,
            latent_dim=latent_dim,
        )
        self.regression_head = nn.Sequential(
            nn.Linear(latent_dim, 64),
            nn.ReLU(),
            nn.Linear(64, 64),
            nn.ReLU(),
            nn.Linear(64, len(REGRESSION_TARGETS)),
        )
        self.binary_head = nn.Sequential(
            nn.Linear(latent_dim, 64),
            nn.ReLU(),
            nn.Linear(64, len(BINARY_TARGETS)),
        )

    def forward(self, obs: dict[str, torch.Tensor]) -> tuple[torch.Tensor, torch.Tensor]:
        state = self.state_encoder.encode(obs)
        return self.regression_head(state.tactical), self.binary_head(state.tactical)


def evaluate_split(
    model: StructuredStateEvaluator,
    dataset: dict[str, Any],
    indices: np.ndarray,
    device: torch.device,
    batch_size: int,
    regression_mean: np.ndarray,
    regression_std: np.ndarray,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    if len(indices) == 0:
        return {"samples": 0}, []
    mean_t = torch.as_tensor(regression_mean, device=device).float()
    std_t = torch.as_tensor(regression_std, device=device).float().clamp_min(1e-6)
    reg_predictions: list[np.ndarray] = []
    bin_predictions: list[np.ndarray] = []
    model.eval()
    with torch.no_grad():
        for start in range(0, len(indices), batch_size):
            batch_indices = indices[start : start + batch_size]
            batch = batch_from_dataset(dataset, batch_indices, device, mean_t, std_t)
            reg_norm, bin_logits = model(batch["obs"])
            reg_predictions.append((reg_norm * std_t + mean_t).detach().cpu().numpy())
            bin_predictions.append(torch.sigmoid(bin_logits).detach().cpu().numpy())
    reg_pred = np.concatenate(reg_predictions, axis=0)
    bin_prob = np.concatenate(bin_predictions, axis=0)
    reg_true = target_matrix(dataset["targets"], REGRESSION_TARGETS, indices)
    bin_true = target_matrix(dataset["targets"], BINARY_TARGETS, indices)
    metrics: dict[str, Any] = {"samples": int(len(indices))}
    for target_index, name in enumerate(REGRESSION_TARGETS):
        y_true = reg_true[:, target_index].astype(np.float32)
        y_pred = reg_pred[:, target_index].astype(np.float32)
        metrics[f"{name}_mae"] = float(np.mean(np.abs(y_true - y_pred)))
        metrics[f"{name}_corr"] = safe_corr(y_true, y_pred)
    for target_index, name in enumerate(BINARY_TARGETS):
        y_true = bin_true[:, target_index].astype(np.float32)
        y_prob = bin_prob[:, target_index].astype(np.float32)
        y_pred = (y_prob >= 0.5).astype(np.float32)
        metrics[f"{name}_accuracy"] = float(np.mean(y_pred == y_true))
        metrics[f"{name}_brier"] = float(np.mean((y_prob - y_true) ** 2))
        metrics[f"{name}_positive_rate"] = float(np.mean(y_true))
        metrics[f"{name}_predicted_positive_rate"] = float(np.mean(y_pred))
    prediction_rows: list[dict[str, Any]] = []
    for local_index, source_index in enumerate(indices):
        row: dict[str, Any] = {"sample_index": int(source_index)}
        for target_index, name in enumerate(REGRESSION_TARGETS):
            row[f"target::{name}"] = float(reg_true[local_index, target_index])
            row[f"pred::{name}"] = float(reg_pred[local_index, target_index])
        for target_index, name in enumerate(BINARY_TARGETS):
            row[f"target::{name}"] = float(bin_true[local_index, target_index])
            row[f"pred::{name}"] = float(bin_prob[local_index, target_index])
        prediction_rows.append(row)
    return metrics, prediction_rows


def main() -> None:
    parser = argparse.ArgumentParser(description="Train a structured combat state evaluator.")
    parser.add_argument("--dataset", required=True, type=Path)
    parser.add_argument("--device", choices=["auto", "cpu", "cuda"], default="auto")
    parser.add_argument("--epochs", default=80, type=int)
    parser.add_argument("--batch-size", default=32, type=int)
    parser.add_argument("--learning-rate", default=3e-4, type=float)
    parser.add_argument("--binary-loss-coef", default=0.5, type=float)
    parser.add_argument("--train-percent", default=80, type=int)
    parser.add_argument("--seed", default=7, type=int)
    parser.add_argument("--output-prefix", default="structured_state_evaluator")
    parser.add_argument("--model-out", default=None, type=Path)
    parser.add_argument("--metrics-out", default=None, type=Path)
    parser.add_argument("--predictions-out", default=None, type=Path)
    args = parser.parse_args()

    np.random.seed(args.seed)
    torch.manual_seed(args.seed)
    if args.device == "auto":
        device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    elif args.device == "cuda":
        if not torch.cuda.is_available():
            raise SystemExit("requested --device cuda but torch.cuda.is_available() is false")
        device = torch.device("cuda")
    else:
        device = torch.device("cpu")

    dataset = load_state_evaluator_dataset(args.dataset)
    indices = np.arange(int(dataset["count"]))
    rng = np.random.default_rng(args.seed)
    rng.shuffle(indices)
    train_count = max(1, int(len(indices) * max(min(args.train_percent, 99), 1) / 100))
    train_indices = indices[:train_count]
    val_indices = indices[train_count:] if train_count < len(indices) else indices[:0]

    train_regression = target_matrix(dataset["targets"], REGRESSION_TARGETS, train_indices)
    regression_mean = train_regression.mean(axis=0).astype(np.float32)
    regression_std = train_regression.std(axis=0).astype(np.float32)
    regression_std = np.where(regression_std < 1e-6, 1.0, regression_std).astype(np.float32)
    mean_t = torch.as_tensor(regression_mean, device=device).float()
    std_t = torch.as_tensor(regression_std, device=device).float()

    model = StructuredStateEvaluator(
        card_vocab=max(len(CARD_ID_VOCAB), 1),
        potion_vocab=max(len(POTION_ID_VOCAB), 1),
        power_vocab=max(len(POWER_ID_VOCAB), 1),
        monster_vocab=max(len(MONSTER_ID_VOCAB), 1),
        intent_vocab=max(len(INTENT_KIND_IDS), 1),
    ).to(device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=args.learning_rate)
    timer = time.perf_counter()
    losses: list[float] = []
    regression_losses: list[float] = []
    binary_losses: list[float] = []
    for _ in range(int(args.epochs)):
        rng.shuffle(train_indices)
        model.train()
        for start in range(0, len(train_indices), int(args.batch_size)):
            batch_indices = train_indices[start : start + int(args.batch_size)]
            batch = batch_from_dataset(dataset, batch_indices, device, mean_t, std_t)
            reg_pred, bin_logits = model(batch["obs"])
            regression_loss = nn.functional.mse_loss(reg_pred, batch["regression"])
            binary_loss = nn.functional.binary_cross_entropy_with_logits(bin_logits, batch["binary"])
            loss = regression_loss + float(args.binary_loss_coef) * binary_loss
            optimizer.zero_grad()
            loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
            losses.append(float(loss.detach().cpu()))
            regression_losses.append(float(regression_loss.detach().cpu()))
            binary_losses.append(float(binary_loss.detach().cpu()))

    train_metrics, train_predictions = evaluate_split(
        model,
        dataset,
        train_indices,
        device,
        int(args.batch_size),
        regression_mean,
        regression_std,
    )
    val_metrics, val_predictions = evaluate_split(
        model,
        dataset,
        val_indices,
        device,
        int(args.batch_size),
        regression_mean,
        regression_std,
    )
    elapsed = time.perf_counter() - timer

    dataset_dir = REPO_ROOT / "tools" / "artifacts" / "learning_dataset"
    prefix = str(args.output_prefix or "").strip()
    model_out = args.model_out or dataset_dir / f"{prefix}_model.pt"
    metrics_out = args.metrics_out or dataset_dir / f"{prefix}_metrics.json"
    predictions_out = args.predictions_out or dataset_dir / f"{prefix}_predictions.jsonl"
    model_out.parent.mkdir(parents=True, exist_ok=True)
    metrics_out.parent.mkdir(parents=True, exist_ok=True)
    predictions_out.parent.mkdir(parents=True, exist_ok=True)
    torch.save(
        {
            "model_state": model.state_dict(),
            "config": {
                "card_vocab": max(len(CARD_ID_VOCAB), 1),
                "potion_vocab": max(len(POTION_ID_VOCAB), 1),
                "power_vocab": max(len(POWER_ID_VOCAB), 1),
                "monster_vocab": max(len(MONSTER_ID_VOCAB), 1),
                "intent_vocab": max(len(INTENT_KIND_IDS), 1),
                "regression_targets": REGRESSION_TARGETS,
                "binary_targets": BINARY_TARGETS,
                "regression_mean": regression_mean.tolist(),
                "regression_std": regression_std.tolist(),
            },
        },
        model_out,
    )
    prediction_rows = []
    for row in train_predictions:
        row["split"] = "train"
        prediction_rows.append(row)
    for row in val_predictions:
        row["split"] = "val"
        prediction_rows.append(row)
    write_jsonl(predictions_out, prediction_rows)
    metrics = {
        "model": "structured_state_evaluator",
        "dataset": str(args.dataset),
        "dataset_samples": int(dataset["count"]),
        "train_samples": int(len(train_indices)),
        "val_samples": int(len(val_indices)),
        "epochs": int(args.epochs),
        "batch_size": int(args.batch_size),
        "device": str(device),
        "torch": {
            "version": torch.__version__,
            "cuda_available": bool(torch.cuda.is_available()),
            "cuda_version": torch.version.cuda,
            "cuda_device": torch.cuda.get_device_name(0) if torch.cuda.is_available() else None,
        },
        "loss": {
            "mean_total": float(np.mean(losses)) if losses else 0.0,
            "mean_regression": float(np.mean(regression_losses)) if regression_losses else 0.0,
            "mean_binary": float(np.mean(binary_losses)) if binary_losses else 0.0,
        },
        "target_scaling": {
            "regression_targets": REGRESSION_TARGETS,
            "regression_mean": regression_mean.tolist(),
            "regression_std": regression_std.tolist(),
            "binary_targets": BINARY_TARGETS,
        },
        "offline_train": train_metrics,
        "offline_val": val_metrics,
        "timing": {
            "total_seconds": float(elapsed),
        },
        "outputs": {
            "model": str(model_out),
            "metrics": str(metrics_out),
            "predictions": str(predictions_out),
        },
        "notes": [
            "this evaluator predicts short teacher-continuation state outcomes",
            "targets are state values and safety labels, not direct action labels",
            "the encoder is the same structured observation encoder used by the PPO policy",
        ],
    }
    write_json(metrics_out, metrics)
    print(json.dumps(metrics, indent=2, ensure_ascii=False), flush=True)
    print(f"wrote structured state evaluator metrics to {metrics_out}", flush=True)


if __name__ == "__main__":
    main()
