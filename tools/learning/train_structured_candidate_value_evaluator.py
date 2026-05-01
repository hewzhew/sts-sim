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

from build_structured_candidate_value_dataset import CANDIDATE_BINARY_TARGETS, CANDIDATE_REGRESSION_TARGETS
from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from structured_candidate_ranker_common import CANDIDATE_FEATURE_DIM
from structured_combat_env import CARD_ID_VOCAB, INTENT_KIND_IDS, MONSTER_ID_VOCAB, POTION_ID_VOCAB, POWER_ID_VOCAB
from structured_policy import StructuredPolicyNet, to_device_obs
from train_structured_combat_ppo import index_obs


def safe_corr(y_true: np.ndarray, y_pred: np.ndarray) -> float:
    if y_true.size == 0 or np.std(y_true) == 0.0 or np.std(y_pred) == 0.0:
        return 0.0
    return float(np.corrcoef(y_true, y_pred)[0, 1])


def load_candidate_value_dataset(path: Path) -> dict[str, Any]:
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
        data = {key: np.asarray(payload[key]) for key in payload.files if not key.startswith("obs__") and not key.startswith("target__")}
    if not obs:
        raise SystemExit(f"candidate value dataset has no obs arrays: {path}")
    missing = sorted(set(CANDIDATE_REGRESSION_TARGETS + CANDIDATE_BINARY_TARGETS) - set(targets))
    if missing:
        raise SystemExit(f"candidate value dataset is missing targets {missing}: {path}")
    required = {"candidate_features", "candidate_actions", "candidate_class", "group_index", "candidate_index", "candidate_is_best"}
    missing_arrays = sorted(required - set(data))
    if missing_arrays:
        raise SystemExit(f"candidate value dataset is missing arrays {missing_arrays}: {path}")
    count = next(iter(obs.values())).shape[0]
    if any(value.shape[0] != count for value in obs.values()):
        raise SystemExit(f"candidate value dataset obs arrays have inconsistent sample counts: {path}")
    if any(value.shape[0] != count for value in targets.values()):
        raise SystemExit(f"candidate value dataset target arrays have inconsistent sample counts: {path}")
    if any(value.shape[0] != count for value in data.values()):
        raise SystemExit(f"candidate value dataset candidate arrays have inconsistent sample counts: {path}")
    if data["candidate_features"].shape[-1] != CANDIDATE_FEATURE_DIM:
        raise SystemExit(
            f"candidate feature dim mismatch {data['candidate_features'].shape[-1]} != {CANDIDATE_FEATURE_DIM}"
        )
    return {"obs": obs, "targets": targets, **data, "count": count}


def target_matrix(targets: dict[str, np.ndarray], names: list[str], indices: np.ndarray) -> np.ndarray:
    return np.stack([targets[name][indices] for name in names], axis=-1).astype(np.float32)


def split_indices_by_group(dataset: dict[str, Any], train_percent: int, seed: int) -> tuple[np.ndarray, np.ndarray]:
    group_ids = np.unique(dataset["group_index"].astype(np.int64))
    rng = np.random.default_rng(seed)
    rng.shuffle(group_ids)
    train_group_count = max(1, int(len(group_ids) * max(min(train_percent, 99), 1) / 100))
    train_groups = set(int(value) for value in group_ids[:train_group_count])
    all_indices = np.arange(int(dataset["count"]))
    train_mask = np.asarray([int(group) in train_groups for group in dataset["group_index"].astype(np.int64)])
    return all_indices[train_mask], all_indices[~train_mask]


def batch_from_dataset(
    dataset: dict[str, Any],
    indices: np.ndarray,
    device: torch.device,
    regression_mean: torch.Tensor,
    regression_std: torch.Tensor,
) -> dict[str, Any]:
    regression = torch.as_tensor(target_matrix(dataset["targets"], CANDIDATE_REGRESSION_TARGETS, indices), device=device).float()
    binary = torch.as_tensor(target_matrix(dataset["targets"], CANDIDATE_BINARY_TARGETS, indices), device=device).float()
    return {
        "obs": to_device_obs(index_obs(dataset["obs"], indices), device),
        "candidate_features": torch.as_tensor(dataset["candidate_features"][indices], device=device).float(),
        "regression_raw": regression,
        "regression": (regression - regression_mean) / regression_std.clamp_min(1e-6),
        "binary": binary,
    }


class StructuredCandidateValueEvaluator(nn.Module):
    def __init__(
        self,
        *,
        card_vocab: int,
        potion_vocab: int,
        power_vocab: int,
        monster_vocab: int,
        intent_vocab: int,
        candidate_feature_dim: int = CANDIDATE_FEATURE_DIM,
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
        self.trunk = nn.Sequential(
            nn.Linear(latent_dim + candidate_feature_dim, 96),
            nn.ReLU(),
            nn.Linear(96, 64),
            nn.ReLU(),
        )
        self.regression_head = nn.Linear(64, len(CANDIDATE_REGRESSION_TARGETS))
        self.binary_head = nn.Linear(64, len(CANDIDATE_BINARY_TARGETS))

    def forward(self, obs: dict[str, torch.Tensor], candidate_features: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        state = self.state_encoder.encode(obs)
        hidden = self.trunk(torch.cat([state.tactical, candidate_features.float()], dim=-1))
        return self.regression_head(hidden), self.binary_head(hidden)


def evaluate_split(
    model: StructuredCandidateValueEvaluator,
    dataset: dict[str, Any],
    indices: np.ndarray,
    device: torch.device,
    batch_size: int,
    regression_mean: np.ndarray,
    regression_std: np.ndarray,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    if len(indices) == 0:
        return {"samples": 0, "groups": 0}, []
    mean_t = torch.as_tensor(regression_mean, device=device).float()
    std_t = torch.as_tensor(regression_std, device=device).float().clamp_min(1e-6)
    reg_predictions: list[np.ndarray] = []
    bin_predictions: list[np.ndarray] = []
    model.eval()
    with torch.no_grad():
        for start in range(0, len(indices), batch_size):
            batch_indices = indices[start : start + batch_size]
            batch = batch_from_dataset(dataset, batch_indices, device, mean_t, std_t)
            reg_norm, bin_logits = model(batch["obs"], batch["candidate_features"])
            reg_predictions.append((reg_norm * std_t + mean_t).detach().cpu().numpy())
            bin_predictions.append(torch.sigmoid(bin_logits).detach().cpu().numpy())
    reg_pred = np.concatenate(reg_predictions, axis=0)
    bin_prob = np.concatenate(bin_predictions, axis=0)
    reg_true = target_matrix(dataset["targets"], CANDIDATE_REGRESSION_TARGETS, indices)
    bin_true = target_matrix(dataset["targets"], CANDIDATE_BINARY_TARGETS, indices)
    metrics: dict[str, Any] = {
        "samples": int(len(indices)),
        "groups": int(len(np.unique(dataset["group_index"][indices].astype(np.int64)))),
    }
    for target_index, name in enumerate(CANDIDATE_REGRESSION_TARGETS):
        y_true = reg_true[:, target_index].astype(np.float32)
        y_pred = reg_pred[:, target_index].astype(np.float32)
        metrics[f"{name}_mae"] = float(np.mean(np.abs(y_true - y_pred)))
        metrics[f"{name}_corr"] = safe_corr(y_true, y_pred)
    for target_index, name in enumerate(CANDIDATE_BINARY_TARGETS):
        y_true = bin_true[:, target_index].astype(np.float32)
        y_prob = bin_prob[:, target_index].astype(np.float32)
        y_pred = (y_prob >= 0.5).astype(np.float32)
        metrics[f"{name}_accuracy"] = float(np.mean(y_pred == y_true))
        metrics[f"{name}_brier"] = float(np.mean((y_prob - y_true) ** 2))
        metrics[f"{name}_positive_rate"] = float(np.mean(y_true))
    score_index = CANDIDATE_REGRESSION_TARGETS.index("discounted_return")
    index_to_local = {int(source_index): local for local, source_index in enumerate(indices)}
    top1_hits = 0
    top1_within_002 = 0
    top1_within_005 = 0
    top1_regrets: list[float] = []
    top2_gaps: list[float] = []
    group_rows = 0
    prediction_rows: list[dict[str, Any]] = []
    for group_id in np.unique(dataset["group_index"][indices].astype(np.int64)):
        group_indices = [int(index) for index in indices if int(dataset["group_index"][index]) == int(group_id)]
        if not group_indices:
            continue
        group_rows += 1
        local_positions = [index_to_local[index] for index in group_indices]
        pred_scores = reg_pred[local_positions, score_index]
        true_scores = reg_true[local_positions, score_index]
        best_local = int(np.argmax(pred_scores))
        true_best = float(np.max(true_scores))
        sorted_true = sorted((float(value) for value in true_scores), reverse=True)
        if len(sorted_true) > 1:
            top2_gaps.append(float(sorted_true[0] - sorted_true[1]))
        chosen_global = group_indices[best_local]
        regret = float(true_best - true_scores[best_local])
        if bool(dataset["candidate_is_best"][chosen_global] > 0.5):
            top1_hits += 1
        if regret <= 0.02:
            top1_within_002 += 1
        if regret <= 0.05:
            top1_within_005 += 1
        top1_regrets.append(regret)
    metrics["top1_group_match"] = float(top1_hits / group_rows) if group_rows else 0.0
    metrics["top1_within_0_02"] = float(top1_within_002 / group_rows) if group_rows else 0.0
    metrics["top1_within_0_05"] = float(top1_within_005 / group_rows) if group_rows else 0.0
    metrics["top1_mean_regret"] = float(np.mean(top1_regrets)) if top1_regrets else 0.0
    metrics["mean_true_top2_gap"] = float(np.mean(top2_gaps)) if top2_gaps else 0.0
    for local_index, source_index in enumerate(indices):
        row: dict[str, Any] = {
            "sample_index": int(source_index),
            "group_index": int(dataset["group_index"][source_index]),
            "candidate_index": int(dataset["candidate_index"][source_index]),
            "candidate_is_best": bool(dataset["candidate_is_best"][source_index] > 0.5),
        }
        for target_index, name in enumerate(CANDIDATE_REGRESSION_TARGETS):
            row[f"target::{name}"] = float(reg_true[local_index, target_index])
            row[f"pred::{name}"] = float(reg_pred[local_index, target_index])
        for target_index, name in enumerate(CANDIDATE_BINARY_TARGETS):
            row[f"target::{name}"] = float(bin_true[local_index, target_index])
            row[f"pred::{name}"] = float(bin_prob[local_index, target_index])
        prediction_rows.append(row)
    return metrics, prediction_rows


def main() -> None:
    parser = argparse.ArgumentParser(description="Train a structured candidate value evaluator.")
    parser.add_argument("--dataset", required=True, type=Path)
    parser.add_argument("--device", choices=["auto", "cpu", "cuda"], default="auto")
    parser.add_argument("--epochs", default=80, type=int)
    parser.add_argument("--batch-size", default=32, type=int)
    parser.add_argument("--learning-rate", default=3e-4, type=float)
    parser.add_argument("--binary-loss-coef", default=0.5, type=float)
    parser.add_argument("--train-percent", default=80, type=int)
    parser.add_argument("--seed", default=7, type=int)
    parser.add_argument("--output-prefix", default="structured_candidate_value_evaluator")
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

    dataset = load_candidate_value_dataset(args.dataset)
    train_indices, val_indices = split_indices_by_group(dataset, int(args.train_percent), int(args.seed))
    rng = np.random.default_rng(args.seed)
    train_regression = target_matrix(dataset["targets"], CANDIDATE_REGRESSION_TARGETS, train_indices)
    regression_mean = train_regression.mean(axis=0).astype(np.float32)
    regression_std = train_regression.std(axis=0).astype(np.float32)
    regression_std = np.where(regression_std < 1e-6, 1.0, regression_std).astype(np.float32)
    mean_t = torch.as_tensor(regression_mean, device=device).float()
    std_t = torch.as_tensor(regression_std, device=device).float()

    model = StructuredCandidateValueEvaluator(
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
            reg_pred, bin_logits = model(batch["obs"], batch["candidate_features"])
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
                "candidate_feature_dim": CANDIDATE_FEATURE_DIM,
                "regression_targets": CANDIDATE_REGRESSION_TARGETS,
                "binary_targets": CANDIDATE_BINARY_TARGETS,
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
        "model": "structured_candidate_value_evaluator",
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
            "regression_targets": CANDIDATE_REGRESSION_TARGETS,
            "regression_mean": regression_mean.tolist(),
            "regression_std": regression_std.tolist(),
            "binary_targets": CANDIDATE_BINARY_TARGETS,
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
            "this evaluator predicts candidate-after-teacher-continuation outcomes",
            "group ranking metrics split by root state group, not by individual candidate row",
            "ranking uses predicted discounted_return over candidates from the same root state",
        ],
    }
    write_json(metrics_out, metrics)
    print(json.dumps(metrics, indent=2, ensure_ascii=False), flush=True)
    print(f"wrote structured candidate value metrics to {metrics_out}", flush=True)


if __name__ == "__main__":
    main()
