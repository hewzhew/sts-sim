#!/usr/bin/env python3
from __future__ import annotations

import argparse
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

from combat_rl_common import REPO_ROOT, iter_jsonl, transition_feature_dict, write_json

STATE_DELTA_TARGETS = [
    "delta_player_hp",
    "delta_player_block",
    "delta_energy",
    "delta_total_monster_hp",
    "delta_living_monster_count",
    "delta_hand_count",
    "delta_draw_count",
    "delta_discard_count",
]
REWARD_COMPONENT_TARGETS = [
    "enemy_hp_delta",
    "player_hp_delta",
    "incoming_relief",
    "kill_bonus",
    "stabilize_bonus",
    "idle_penalty",
]
STATE_AFTER_KEYS = [
    "player_current_hp",
    "player_block",
    "player_energy",
    "total_monster_hp",
    "living_monster_count",
    "hand_count",
    "draw_count",
    "discard_count",
]
STATE_BEFORE_KEYS = STATE_AFTER_KEYS


def load_rows(path: Path) -> list[dict[str, Any]]:
    return [row for _, row in iter_jsonl(path)]


def before_matrix(rows: list[dict[str, Any]]) -> np.ndarray:
    matrix = []
    for row in rows:
        before = row.get("state_before_features") or {}
        matrix.append([float(before.get(key) or 0.0) for key in STATE_BEFORE_KEYS])
    return np.asarray(matrix, dtype=np.float32)


def state_delta_targets(rows: list[dict[str, Any]]) -> np.ndarray:
    matrix = []
    for row in rows:
        before = row.get("state_before_features") or {}
        after = row.get("state_after_features") or {}
        matrix.append(
            [
                float(after.get("player_current_hp") or 0.0) - float(before.get("player_current_hp") or 0.0),
                float(after.get("player_block") or 0.0) - float(before.get("player_block") or 0.0),
                float(after.get("player_energy") or 0.0) - float(before.get("player_energy") or 0.0),
                float(after.get("total_monster_hp") or 0.0) - float(before.get("total_monster_hp") or 0.0),
                float(after.get("living_monster_count") or 0.0) - float(before.get("living_monster_count") or 0.0),
                float(after.get("hand_count") or 0.0) - float(before.get("hand_count") or 0.0),
                float(after.get("draw_count") or 0.0) - float(before.get("draw_count") or 0.0),
                float(after.get("discard_count") or 0.0) - float(before.get("discard_count") or 0.0),
            ]
        )
    return np.asarray(matrix, dtype=np.float32)


def reward_component_targets(rows: list[dict[str, Any]]) -> np.ndarray:
    matrix = []
    for row in rows:
        breakdown = row.get("reward_breakdown") or {}
        matrix.append([float(breakdown.get(key) or 0.0) for key in REWARD_COMPONENT_TARGETS])
    return np.asarray(matrix, dtype=np.float32)


def terminal_targets(rows: list[dict[str, Any]]) -> tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
    done = np.asarray([1 if row.get("done") else 0 for row in rows], dtype=np.float32)
    victory = np.asarray([1 if row.get("terminal_victory") else 0 for row in rows], dtype=np.float32)
    defeat = np.asarray([1 if row.get("terminal_defeat") else 0 for row in rows], dtype=np.float32)
    outcome = np.asarray(
        [2 if row.get("terminal_victory") else 1 if row.get("terminal_defeat") else 0 for row in rows],
        dtype=np.int64,
    )
    return done, victory, defeat, outcome


class MLP(nn.Module):
    def __init__(self, in_features: int, out_features: int, hidden: int = 256, dropout: float = 0.1) -> None:
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(in_features, hidden),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(hidden, hidden),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(hidden, out_features),
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.net(x)


def _pos_weight(labels: np.ndarray) -> torch.Tensor:
    positives = float(labels.sum())
    negatives = float(len(labels) - positives)
    weight = max(negatives / max(positives, 1.0), 1.0)
    return torch.tensor([weight], dtype=torch.float32)


def _class_weights(labels: np.ndarray, num_classes: int) -> torch.Tensor:
    counts = np.bincount(labels.astype(np.int64), minlength=num_classes).astype(np.float32)
    counts[counts == 0] = 1.0
    weights = counts.sum() / (counts * num_classes)
    return torch.as_tensor(weights, dtype=torch.float32)


def fit_regressor(
    x_train: np.ndarray,
    y_train: np.ndarray,
    x_val: np.ndarray,
    y_val: np.ndarray,
    device: torch.device,
    epochs: int,
    batch_size: int,
    lr: float,
) -> tuple[MLP, int]:
    model = MLP(x_train.shape[1], y_train.shape[1]).to(device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=lr, weight_decay=1e-4)
    loss_fn = nn.SmoothL1Loss()
    loader = DataLoader(
        TensorDataset(torch.as_tensor(x_train), torch.as_tensor(y_train)),
        batch_size=batch_size,
        shuffle=True,
    )
    best_state = model.state_dict()
    best_val = float("inf")
    best_epoch = 1
    for epoch in range(epochs):
        model.train()
        for batch_x, batch_y in loader:
            batch_x = batch_x.to(device=device, dtype=torch.float32)
            batch_y = batch_y.to(device=device, dtype=torch.float32)
            pred = model(batch_x)
            loss = loss_fn(pred, batch_y)
            optimizer.zero_grad()
            loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
        model.eval()
        with torch.no_grad():
            val_pred = model(torch.as_tensor(x_val, dtype=torch.float32, device=device))
            val_loss = loss_fn(val_pred, torch.as_tensor(y_val, dtype=torch.float32, device=device)).item()
        if val_loss < best_val:
            best_val = val_loss
            best_state = {key: value.detach().cpu().clone() for key, value in model.state_dict().items()}
            best_epoch = epoch + 1
    model.load_state_dict(best_state)
    return model, best_epoch


def fit_binary_classifier(
    x_train: np.ndarray,
    y_train: np.ndarray,
    x_val: np.ndarray,
    y_val: np.ndarray,
    device: torch.device,
    epochs: int,
    batch_size: int,
    lr: float,
) -> tuple[MLP, int]:
    model = MLP(x_train.shape[1], 1).to(device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=lr, weight_decay=1e-4)
    loss_fn = nn.BCEWithLogitsLoss(pos_weight=_pos_weight(y_train).to(device))
    loader = DataLoader(
        TensorDataset(torch.as_tensor(x_train), torch.as_tensor(y_train).unsqueeze(1)),
        batch_size=batch_size,
        shuffle=True,
    )
    best_state = model.state_dict()
    best_val = float("inf")
    best_epoch = 1
    for epoch in range(epochs):
        model.train()
        for batch_x, batch_y in loader:
            batch_x = batch_x.to(device=device, dtype=torch.float32)
            batch_y = batch_y.to(device=device, dtype=torch.float32)
            pred = model(batch_x)
            loss = loss_fn(pred, batch_y)
            optimizer.zero_grad()
            loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
        model.eval()
        with torch.no_grad():
            val_pred = model(torch.as_tensor(x_val, dtype=torch.float32, device=device))
            val_loss = loss_fn(val_pred, torch.as_tensor(y_val, dtype=torch.float32, device=device).unsqueeze(1)).item()
        if val_loss < best_val:
            best_val = val_loss
            best_state = {key: value.detach().cpu().clone() for key, value in model.state_dict().items()}
            best_epoch = epoch + 1
    model.load_state_dict(best_state)
    return model, best_epoch


def fit_outcome_classifier(
    x_train: np.ndarray,
    y_train: np.ndarray,
    x_val: np.ndarray,
    y_val: np.ndarray,
    device: torch.device,
    epochs: int,
    batch_size: int,
    lr: float,
) -> tuple[MLP, int]:
    model = MLP(x_train.shape[1], 3).to(device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=lr, weight_decay=1e-4)
    loss_fn = nn.CrossEntropyLoss(weight=_class_weights(y_train, 3).to(device))
    loader = DataLoader(
        TensorDataset(torch.as_tensor(x_train), torch.as_tensor(y_train)),
        batch_size=batch_size,
        shuffle=True,
    )
    best_state = model.state_dict()
    best_val = float("inf")
    best_epoch = 1
    for epoch in range(epochs):
        model.train()
        for batch_x, batch_y in loader:
            batch_x = batch_x.to(device=device, dtype=torch.float32)
            batch_y = batch_y.to(device=device, dtype=torch.long)
            pred = model(batch_x)
            loss = loss_fn(pred, batch_y)
            optimizer.zero_grad()
            loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
        model.eval()
        with torch.no_grad():
            val_pred = model(torch.as_tensor(x_val, dtype=torch.float32, device=device))
            val_loss = loss_fn(val_pred, torch.as_tensor(y_val, dtype=torch.long, device=device)).item()
        if val_loss < best_val:
            best_val = val_loss
            best_state = {key: value.detach().cpu().clone() for key, value in model.state_dict().items()}
            best_epoch = epoch + 1
    model.load_state_dict(best_state)
    return model, best_epoch


def evaluate(
    delta_model: MLP,
    reward_model: MLP,
    done_model: MLP,
    victory_model: MLP,
    defeat_model: MLP,
    outcome_model: MLP,
    x_scaled: np.ndarray,
    before: np.ndarray,
    y_delta: np.ndarray,
    y_reward_components: np.ndarray,
    y_done: np.ndarray,
    y_victory: np.ndarray,
    y_defeat: np.ndarray,
    y_outcome: np.ndarray,
    delta_scaler: StandardScaler,
    reward_scaler: StandardScaler,
    device: torch.device,
) -> dict[str, Any]:
    if len(x_scaled) == 0:
        return {"rows": 0}
    x_tensor = torch.as_tensor(x_scaled, dtype=torch.float32, device=device)
    with torch.no_grad():
        pred_delta = delta_scaler.inverse_transform(delta_model(x_tensor).cpu().numpy())
        pred_reward_components = reward_scaler.inverse_transform(reward_model(x_tensor).cpu().numpy())
        pred_done_np = (torch.sigmoid(done_model(x_tensor)).cpu().numpy().reshape(-1) >= 0.5).astype(np.int32)
        pred_victory_np = (torch.sigmoid(victory_model(x_tensor)).cpu().numpy().reshape(-1) >= 0.5).astype(np.int32)
        pred_defeat_np = (torch.sigmoid(defeat_model(x_tensor)).cpu().numpy().reshape(-1) >= 0.5).astype(np.int32)
        pred_outcome_np = torch.argmax(outcome_model(x_tensor), dim=1).cpu().numpy()

    pred_after = before.copy()
    pred_after[:, : len(STATE_AFTER_KEYS)] = before[:, : len(STATE_AFTER_KEYS)] + pred_delta
    actual_after = before.copy()
    actual_after[:, : len(STATE_AFTER_KEYS)] = before[:, : len(STATE_AFTER_KEYS)] + y_delta

    numeric_mae = {
        key if key != "player_current_hp" else "next_player_hp": float(mean_absolute_error(actual_after[:, index], pred_after[:, index]))
        for index, key in enumerate(STATE_AFTER_KEYS)
    }
    reward_component_mae = {
        key: float(mean_absolute_error(y_reward_components[:, index], pred_reward_components[:, index]))
        for index, key in enumerate(REWARD_COMPONENT_TARGETS)
    }
    actual_reward_total = y_reward_components.sum(axis=1)
    pred_reward_total = pred_reward_components.sum(axis=1)
    return {
        "rows": len(x_scaled),
        "numeric_mae": numeric_mae,
        "reward_component_mae": reward_component_mae,
        "reward_mae": float(mean_absolute_error(actual_reward_total, pred_reward_total)),
        "terminal_prediction_accuracy": float(accuracy_score(y_outcome, pred_outcome_np)),
        "done_prediction_accuracy": float(accuracy_score(y_done.astype(np.int32), pred_done_np)),
        "victory_prediction_accuracy": float(accuracy_score(y_victory.astype(np.int32), pred_victory_np)),
        "defeat_prediction_accuracy": float(accuracy_score(y_defeat.astype(np.int32), pred_defeat_np)),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Train separated PyTorch combat transition models.")
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--dataset-prefix", default="combat_transition")
    parser.add_argument("--epochs", default=80, type=int)
    parser.add_argument("--batch-size", default=128, type=int)
    parser.add_argument("--device", default="cpu")
    parser.add_argument("--lr", default=5e-4, type=float)
    parser.add_argument("--metrics-out", default=None, type=Path)
    parser.add_argument("--model-out", default=None, type=Path)
    args = parser.parse_args()

    metrics_out = args.metrics_out or (args.dataset_dir / "transition_torch_metrics.json")
    model_out = args.model_out or (args.dataset_dir / "combat_transition_torch_model.pt")
    baseline_metrics_path = args.dataset_dir / "transition_metrics.json"
    train_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_train.jsonl")
    val_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_val.jsonl")
    test_rows = load_rows(args.dataset_dir / f"{args.dataset_prefix}_test.jsonl")
    if not train_rows:
        raise SystemExit("no combat transition rows found")

    vectorizer = DictVectorizer(sparse=False)
    x_train = vectorizer.fit_transform([transition_feature_dict(row) for row in train_rows]).astype(np.float32)
    x_val = vectorizer.transform([transition_feature_dict(row) for row in val_rows]).astype(np.float32)
    x_test = vectorizer.transform([transition_feature_dict(row) for row in test_rows]).astype(np.float32)
    x_scaler = StandardScaler()
    x_train_scaled = x_scaler.fit_transform(x_train).astype(np.float32)
    x_val_scaled = x_scaler.transform(x_val).astype(np.float32)
    x_test_scaled = x_scaler.transform(x_test).astype(np.float32)

    before_train = before_matrix(train_rows)
    before_val = before_matrix(val_rows)
    before_test = before_matrix(test_rows)

    y_train_delta = state_delta_targets(train_rows)
    y_val_delta = state_delta_targets(val_rows)
    y_test_delta = state_delta_targets(test_rows)
    y_train_reward = reward_component_targets(train_rows)
    y_val_reward = reward_component_targets(val_rows)
    y_test_reward = reward_component_targets(test_rows)
    y_train_done, y_train_victory, y_train_defeat, y_train_outcome = terminal_targets(train_rows)
    y_val_done, y_val_victory, y_val_defeat, y_val_outcome = terminal_targets(val_rows)
    y_test_done, y_test_victory, y_test_defeat, y_test_outcome = terminal_targets(test_rows)

    delta_scaler = StandardScaler()
    reward_scaler = StandardScaler()
    y_train_delta_scaled = delta_scaler.fit_transform(y_train_delta).astype(np.float32)
    y_val_delta_scaled = delta_scaler.transform(y_val_delta).astype(np.float32)
    y_train_reward_scaled = reward_scaler.fit_transform(y_train_reward).astype(np.float32)
    y_val_reward_scaled = reward_scaler.transform(y_val_reward).astype(np.float32)

    device = torch.device(args.device)
    delta_model, delta_best_epoch = fit_regressor(
        x_train_scaled, y_train_delta_scaled, x_val_scaled, y_val_delta_scaled, device, args.epochs, args.batch_size, args.lr
    )
    reward_model, reward_best_epoch = fit_regressor(
        x_train_scaled, y_train_reward_scaled, x_val_scaled, y_val_reward_scaled, device, args.epochs, args.batch_size, args.lr
    )
    done_model, done_best_epoch = fit_binary_classifier(
        x_train_scaled, y_train_done, x_val_scaled, y_val_done, device, args.epochs, args.batch_size, args.lr
    )
    victory_model, victory_best_epoch = fit_binary_classifier(
        x_train_scaled, y_train_victory, x_val_scaled, y_val_victory, device, args.epochs, args.batch_size, args.lr
    )
    defeat_model, defeat_best_epoch = fit_binary_classifier(
        x_train_scaled, y_train_defeat, x_val_scaled, y_val_defeat, device, args.epochs, args.batch_size, args.lr
    )
    outcome_model, outcome_best_epoch = fit_outcome_classifier(
        x_train_scaled, y_train_outcome, x_val_scaled, y_val_outcome, device, args.epochs, args.batch_size, args.lr
    )

    torch.save(
        {
            "vectorizer_feature_names": vectorizer.feature_names_,
            "input_dim": x_train.shape[1],
            "x_scaler_mean": x_scaler.mean_.tolist(),
            "x_scaler_scale": x_scaler.scale_.tolist(),
            "delta_scaler_mean": delta_scaler.mean_.tolist(),
            "delta_scaler_scale": delta_scaler.scale_.tolist(),
            "reward_scaler_mean": reward_scaler.mean_.tolist(),
            "reward_scaler_scale": reward_scaler.scale_.tolist(),
            "delta_state_dict": delta_model.state_dict(),
            "reward_state_dict": reward_model.state_dict(),
            "done_state_dict": done_model.state_dict(),
            "victory_state_dict": victory_model.state_dict(),
            "defeat_state_dict": defeat_model.state_dict(),
            "outcome_state_dict": outcome_model.state_dict(),
        },
        model_out,
    )

    metrics = {
        "model": "torch_transition_stack",
        "dataset_prefix": args.dataset_prefix,
        "feature_count": x_train.shape[1],
        "epochs": args.epochs,
        "lr": args.lr,
        "best_epochs": {
            "state_delta": delta_best_epoch,
            "reward_components": reward_best_epoch,
            "done": done_best_epoch,
            "victory": victory_best_epoch,
            "defeat": defeat_best_epoch,
            "outcome": outcome_best_epoch,
        },
        "train": evaluate(delta_model, reward_model, done_model, victory_model, defeat_model, outcome_model, x_train_scaled, before_train, y_train_delta, y_train_reward, y_train_done, y_train_victory, y_train_defeat, y_train_outcome, delta_scaler, reward_scaler, device),
        "val": evaluate(delta_model, reward_model, done_model, victory_model, defeat_model, outcome_model, x_val_scaled, before_val, y_val_delta, y_val_reward, y_val_done, y_val_victory, y_val_defeat, y_val_outcome, delta_scaler, reward_scaler, device),
        "test": evaluate(delta_model, reward_model, done_model, victory_model, defeat_model, outcome_model, x_test_scaled, before_test, y_test_delta, y_test_reward, y_test_done, y_test_victory, y_test_defeat, y_test_outcome, delta_scaler, reward_scaler, device),
        "state_delta_targets": STATE_DELTA_TARGETS,
        "reward_component_targets": REWARD_COMPONENT_TARGETS,
        "notes": [
            "transition training is split into separate state-delta, reward-component, and terminal classifiers",
            "reward total is derived from predicted reward components rather than trained directly",
            "this keeps terminal/outcome supervision from dominating next-state regression",
        ],
    }
    if baseline_metrics_path.exists():
        with baseline_metrics_path.open("r", encoding="utf-8") as handle:
            baseline = json.load(handle)
        metrics["baseline_compare"] = {
            "test_reward_mae_delta_vs_sklearn": float(metrics["test"]["reward_mae"] - baseline["test"]["reward_mae"]),
            "test_done_acc_delta_vs_sklearn": float(metrics["test"]["done_prediction_accuracy"] - baseline["test"]["done_prediction_accuracy"]),
            "test_terminal_acc_delta_vs_sklearn": float(metrics["test"]["terminal_prediction_accuracy"] - baseline["test"]["terminal_prediction_accuracy"]),
        }
    write_json(metrics_out, metrics)
    print(json.dumps(metrics, indent=2, ensure_ascii=False))
    print(f"wrote torch transition metrics to {metrics_out}")
    print(f"saved torch transition model stack to {model_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
