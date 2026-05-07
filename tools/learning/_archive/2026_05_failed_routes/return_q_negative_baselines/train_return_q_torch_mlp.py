#!/usr/bin/env python3
"""Train a small sparse-feature Torch MLP return-Q ranker."""
from __future__ import annotations

import argparse
import json
import math
import random
import time
from collections import defaultdict
from pathlib import Path
from typing import Any

import torch
import torch.nn as nn
import torch.nn.functional as F

from return_q_common import read_jsonl, row_features, stable_group_split, write_json

FEATURE_SETS = [
    "action_only",
    "candidate_only",
    "state_only",
    "full_state_plus_candidate",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--model-out", type=Path, required=True)
    parser.add_argument("--state-dict-out", type=Path)
    parser.add_argument("--report-out", type=Path)
    parser.add_argument("--epochs", type=int, default=40)
    parser.add_argument("--learning-rate", type=float, default=0.002)
    parser.add_argument("--weight-decay", type=float, default=0.0001)
    parser.add_argument("--seed", type=int, default=23)
    parser.add_argument("--feature-set", default="full_state_plus_candidate", choices=FEATURE_SETS)
    parser.add_argument("--feature-dim", type=int, default=32768)
    parser.add_argument("--hidden-dim", type=int, default=64)
    parser.add_argument("--target-temperature", type=float, default=0.35)
    parser.add_argument("--pair-margin", type=float, default=0.01)
    parser.add_argument("--device", default="cpu")
    return parser.parse_args()


class SparseMlpQ(nn.Module):
    def __init__(self, feature_dim: int, hidden_dim: int) -> None:
        super().__init__()
        self.embedding = nn.EmbeddingBag(
            feature_dim,
            hidden_dim,
            mode="sum",
            include_last_offset=True,
        )
        self.net = nn.Sequential(
            nn.LayerNorm(hidden_dim),
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, 1),
        )

    def forward(
        self,
        indices: torch.Tensor,
        offsets: torch.Tensor,
        weights: torch.Tensor,
    ) -> torch.Tensor:
        embedded = self.embedding(indices, offsets, per_sample_weights=weights)
        return self.net(embedded).squeeze(-1)


def main() -> None:
    args = parse_args()
    start = time.perf_counter()
    random.seed(args.seed)
    torch.manual_seed(args.seed)
    rows = read_jsonl(args.input)
    if not rows:
        raise SystemExit(f"no rows found in {args.input}")
    for row in rows:
        row.setdefault("split", stable_group_split(str(row.get("group_key") or "")))

    prepared = {
        feature_set: prepare_splits(rows, feature_set, args.device)
        for feature_set in FEATURE_SETS
    }
    models: dict[str, SparseMlpQ] = {}
    metrics: dict[str, Any] = {}
    for feature_set in FEATURE_SETS:
        model, feature_metrics = train_feature_set(args, prepared[feature_set])
        models[feature_set] = model
        metrics[feature_set] = feature_metrics

    chosen = models[args.feature_set].to("cpu")
    state_dict_out = args.state_dict_out or args.model_out.with_suffix(".pt")
    state_dict_out.parent.mkdir(parents=True, exist_ok=True)
    torch.save(chosen.state_dict(), state_dict_out)

    model_payload = {
        "schema_version": "return_q_torch_embedding_mlp_model_v0",
        "model_type": "torch_embedding_mlp",
        "feature_set": args.feature_set,
        "target_mode": "listwise_return",
        "target_mean": 0.0,
        "target_std": 1.0,
        "state_dict_path": str(state_dict_out),
        "config": {
            "epochs": args.epochs,
            "learning_rate": args.learning_rate,
            "weight_decay": args.weight_decay,
            "seed": args.seed,
            "feature_dim": args.feature_dim,
            "hidden_dim": args.hidden_dim,
            "target_temperature": args.target_temperature,
            "pair_margin": args.pair_margin,
            "target_mode": "listwise_return",
        },
    }
    write_json(args.model_out, model_payload)

    elapsed = time.perf_counter() - start
    report = {
        "schema_version": "return_q_torch_mlp_train_report_v0",
        "input": str(args.input),
        "model_out": str(args.model_out),
        "state_dict_out": str(state_dict_out),
        "row_count": len(rows),
        "split_counts": split_counts(rows),
        "metrics_by_feature_set": metrics,
        "gate": gate(metrics),
        "runtime_seconds": elapsed,
        "torch": {
            "version": torch.__version__,
            "device": args.device,
            "cuda_available": torch.cuda.is_available(),
        },
    }
    report_out = args.report_out or args.model_out.with_suffix(".report.json")
    write_json(report_out, report)
    print(json.dumps(report, indent=2, sort_keys=True))


def prepare_splits(
    rows: list[dict[str, Any]],
    feature_set: str,
    device: str,
) -> dict[str, list[dict[str, Any]]]:
    groups: dict[str, list[dict[str, Any]]] = {"train": [], "valid": [], "test": []}
    by_group: defaultdict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        by_group[str(row.get("group_key") or "")].append(row)
    for group_key, group_rows in by_group.items():
        split = str(group_rows[0].get("split") or stable_group_split(group_key))
        if split not in groups:
            continue
        examples = []
        for row in group_rows:
            sparse = row_features(row, feature_set)
            examples.append(
                {
                    "indices": torch.tensor(list(sparse.keys()) or [0], dtype=torch.long, device=device),
                    "weights": torch.tensor(list(sparse.values()) or [0.0], dtype=torch.float32, device=device),
                    "target": float(row.get("discounted_return") or 0.0),
                }
            )
        if len(examples) >= 2:
            groups[split].append({"group_key": group_key, "examples": examples})
    return groups


def train_feature_set(
    args: argparse.Namespace,
    splits: dict[str, list[dict[str, Any]]],
) -> tuple[SparseMlpQ, dict[str, Any]]:
    device = torch.device(args.device)
    model = SparseMlpQ(args.feature_dim, args.hidden_dim).to(device)
    optimizer = torch.optim.AdamW(
        model.parameters(),
        lr=args.learning_rate,
        weight_decay=args.weight_decay,
    )
    rng = random.Random(args.seed)
    train_groups = list(splits["train"])
    for _epoch in range(max(args.epochs, 0)):
        rng.shuffle(train_groups)
        model.train()
        for group in train_groups:
            scores, targets = score_group(model, group)
            if not has_signal(targets, args.pair_margin):
                continue
            target_probs = F.softmax(targets / max(args.target_temperature, 1e-6), dim=0)
            log_probs = F.log_softmax(scores, dim=0)
            loss = -(target_probs * log_probs).sum()
            optimizer.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 5.0)
            optimizer.step()
    model.eval()
    return model, {
        split: evaluate_groups(model, groups, args.pair_margin)
        for split, groups in splits.items()
    }


def score_group(model: SparseMlpQ, group: dict[str, Any]) -> tuple[torch.Tensor, torch.Tensor]:
    examples = group["examples"]
    all_indices = torch.cat([example["indices"] for example in examples])
    all_weights = torch.cat([example["weights"] for example in examples])
    offsets = [0]
    total = 0
    for example in examples:
        total += int(example["indices"].numel())
        offsets.append(total)
    offsets_tensor = torch.tensor(offsets, dtype=torch.long, device=all_indices.device)
    scores = model(all_indices, offsets_tensor, all_weights)
    targets = torch.tensor(
        [example["target"] for example in examples],
        dtype=torch.float32,
        device=all_indices.device,
    )
    return scores, targets


def evaluate_groups(
    model: SparseMlpQ,
    groups: list[dict[str, Any]],
    pair_margin: float,
) -> dict[str, Any]:
    if not groups:
        return {"group_count": 0, "count": 0, "pairwise_accuracy": None, "top1_regret": None}
    pair_correct = 0
    pair_total = 0
    regrets = []
    count = 0
    with torch.no_grad():
        for group in groups:
            scores, targets_tensor = score_group(model, group)
            scores_l = [float(value) for value in scores.detach().cpu()]
            targets = [float(value) for value in targets_tensor.detach().cpu()]
            count += len(targets)
            true_best = max(targets)
            predicted_idx = max(range(len(scores_l)), key=lambda idx: scores_l[idx])
            regrets.append(true_best - targets[predicted_idx])
            for left_idx in range(len(targets)):
                for right_idx in range(left_idx + 1, len(targets)):
                    delta = targets[left_idx] - targets[right_idx]
                    if abs(delta) < pair_margin:
                        continue
                    pair_total += 1
                    if (scores_l[left_idx] > scores_l[right_idx]) == (delta > 0):
                        pair_correct += 1
    return {
        "group_count": len(groups),
        "count": count,
        "pairwise_accuracy": pair_correct / pair_total if pair_total else None,
        "top1_regret": sum(regrets) / len(regrets) if regrets else None,
    }


def has_signal(targets: torch.Tensor, margin: float) -> bool:
    return bool((targets.max() - targets.min()).item() >= margin)


def split_counts(rows: list[dict[str, Any]]) -> dict[str, int]:
    counts = {"train": 0, "valid": 0, "test": 0}
    for row in rows:
        split = str(row.get("split") or "")
        if split in counts:
            counts[split] += 1
    return counts


def gate(metrics: dict[str, Any]) -> dict[str, Any]:
    test = {name: values.get("test") or {} for name, values in metrics.items()}
    full = test.get("full_state_plus_candidate") or {}
    failures = []
    for baseline in ["action_only", "candidate_only", "state_only"]:
        base = test.get(baseline) or {}
        if not beats_pairwise(full, base, margin=0.02):
            failures.append(f"full_state_plus_candidate pairwise_accuracy does not beat {baseline} by >= 0.02")
        if not beats_top1_regret(full, base):
            failures.append(f"full_state_plus_candidate top1_regret does not beat {baseline}")
    return {
        "offline_return_q_torch_gate_passed": not failures,
        "failures": failures,
    }


def beats_pairwise(full: dict[str, Any], baseline: dict[str, Any], *, margin: float) -> bool:
    if not finite_metric(full.get("pairwise_accuracy")) or not finite_metric(baseline.get("pairwise_accuracy")):
        return False
    return float(full["pairwise_accuracy"]) >= float(baseline["pairwise_accuracy"]) + margin


def beats_top1_regret(full: dict[str, Any], baseline: dict[str, Any]) -> bool:
    if not finite_metric(full.get("top1_regret")) or not finite_metric(baseline.get("top1_regret")):
        return False
    return float(full["top1_regret"]) <= float(baseline["top1_regret"])


def finite_metric(value: Any) -> bool:
    return isinstance(value, (int, float)) and math.isfinite(float(value))


if __name__ == "__main__":
    main()
