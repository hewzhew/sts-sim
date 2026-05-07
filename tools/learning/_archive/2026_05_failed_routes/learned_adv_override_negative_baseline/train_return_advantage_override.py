#!/usr/bin/env python3
"""Train a conservative safe-override classifier for adv-vs-rule rows."""
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

from return_q_common import (
    ADV_OVERRIDE_FEATURE_SETS,
    adv_override_features,
    read_jsonl,
    stable_group_split,
    write_json,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--model-out", type=Path, required=True)
    parser.add_argument("--state-dict-out", type=Path)
    parser.add_argument("--report-out", type=Path)
    parser.add_argument("--epochs", type=int, default=30)
    parser.add_argument("--learning-rate", type=float, default=0.002)
    parser.add_argument("--weight-decay", type=float, default=0.0001)
    parser.add_argument("--seed", type=int, default=71)
    parser.add_argument("--feature-set", default="full_decision_plus_choice", choices=ADV_OVERRIDE_FEATURE_SETS)
    parser.add_argument("--feature-dim", type=int, default=32768)
    parser.add_argument("--hidden-dim", type=int, default=96)
    parser.add_argument("--device", default="cpu")
    parser.add_argument("--thresholds", default="0.5,0.7,0.8,0.9")
    parser.add_argument("--include-gray", action="store_true")
    parser.add_argument("--train-all-feature-sets", action="store_true")
    return parser.parse_args()


class SparseOverrideMlp(nn.Module):
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
            nn.Dropout(0.05),
            nn.Linear(hidden_dim, hidden_dim // 2),
            nn.ReLU(),
            nn.Linear(hidden_dim // 2, 1),
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
    thresholds = [float(item) for item in args.thresholds.split(",") if item.strip()]
    rows = read_jsonl(args.input)
    if not rows:
        raise SystemExit(f"no rows found in {args.input}")
    for row in rows:
        row.setdefault("split", stable_group_split(str(row.get("group_key") or "")))
    trainable_rows = [
        row
        for row in rows
        if label_to_target(str(row.get("safe_override_label") or ""), args.include_gray) is not None
    ]
    if not trainable_rows:
        raise SystemExit("no positive/negative rows available after gray filtering")

    feature_sets = ADV_OVERRIDE_FEATURE_SETS if args.train_all_feature_sets else [args.feature_set]
    prepared = {
        feature_set: prepare_splits(trainable_rows, feature_set, args.device)
        for feature_set in feature_sets
    }
    models: dict[str, SparseOverrideMlp] = {}
    metrics: dict[str, Any] = {}
    for feature_set in feature_sets:
        model, feature_metrics = train_feature_set(args, prepared[feature_set], thresholds)
        models[feature_set] = model
        metrics[feature_set] = feature_metrics

    chosen = models[args.feature_set].to("cpu")
    state_dict_out = args.state_dict_out or args.model_out.with_suffix(".pt")
    state_dict_out.parent.mkdir(parents=True, exist_ok=True)
    torch.save(chosen.state_dict(), state_dict_out)
    model_payload = {
        "schema_version": "return_advantage_override_model_v1",
        "model_type": "adv_override_torch_embedding_mlp",
        "feature_set": args.feature_set,
        "target_mode": "safe_override_vs_rule",
        "state_dict_path": str(state_dict_out),
        "config": {
            "epochs": args.epochs,
            "learning_rate": args.learning_rate,
            "weight_decay": args.weight_decay,
            "seed": args.seed,
            "feature_dim": args.feature_dim,
            "hidden_dim": args.hidden_dim,
            "include_gray": args.include_gray,
        },
    }
    write_json(args.model_out, model_payload)

    elapsed = time.perf_counter() - start
    report = {
        "schema_version": "return_advantage_override_train_report_v1",
        "input": str(args.input),
        "model_out": str(args.model_out),
        "state_dict_out": str(state_dict_out),
        "row_count": len(rows),
        "trainable_row_count": len(trainable_rows),
        "label_counts": label_counts(rows),
        "split_counts": split_counts(trainable_rows),
        "metrics_by_feature_set": metrics,
        "gate": gate(metrics, args.feature_set),
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
    groups = {"train": [], "valid": [], "test": []}
    by_group: defaultdict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        by_group[str(row.get("group_key") or "")].append(row)
    for group_key, group_rows in by_group.items():
        split = str(group_rows[0].get("split") or stable_group_split(group_key))
        if split not in groups:
            continue
        examples = []
        for row in group_rows:
            target = label_to_target(str(row.get("safe_override_label") or ""), False)
            if target is None:
                continue
            sparse = adv_override_features(row, feature_set)
            examples.append(
                {
                    "indices": torch.tensor(list(sparse.keys()) or [0], dtype=torch.long, device=device),
                    "weights": torch.tensor(list(sparse.values()) or [0.0], dtype=torch.float32, device=device),
                    "target": float(target),
                    "adv": float(row.get("adv_vs_rule_mean") or 0.0),
                    "is_rule": bool(row.get("is_rule_choice")),
                }
            )
        if examples:
            groups[split].append({"group_key": group_key, "examples": examples})
    return groups


def label_to_target(label: str, include_gray: bool) -> float | None:
    if label == "positive":
        return 1.0
    if label == "negative":
        return 0.0
    if include_gray and label == "gray":
        return 0.0
    return None


def train_feature_set(
    args: argparse.Namespace,
    splits: dict[str, list[dict[str, Any]]],
    thresholds: list[float],
) -> tuple[SparseOverrideMlp, dict[str, Any]]:
    device = torch.device(args.device)
    model = SparseOverrideMlp(args.feature_dim, args.hidden_dim).to(device)
    optimizer = torch.optim.AdamW(
        model.parameters(),
        lr=args.learning_rate,
        weight_decay=args.weight_decay,
    )
    train_examples = [example for group in splits["train"] for example in group["examples"]]
    if not train_examples:
        return model, {split: evaluate_groups(model, groups, thresholds) for split, groups in splits.items()}
    pos = sum(1 for example in train_examples if example["target"] > 0.5)
    neg = max(len(train_examples) - pos, 1)
    pos_weight = torch.tensor([neg / max(pos, 1)], dtype=torch.float32, device=device)
    rng = random.Random(args.seed)
    train_groups = list(splits["train"])
    for _epoch in range(max(args.epochs, 0)):
        rng.shuffle(train_groups)
        model.train()
        for group in train_groups:
            logits, targets, _advs, _is_rule = score_group(model, group)
            loss = F.binary_cross_entropy_with_logits(
                logits,
                targets,
                pos_weight=pos_weight,
            )
            optimizer.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 5.0)
            optimizer.step()
    model.eval()
    return model, {
        split: evaluate_groups(model, groups, thresholds)
        for split, groups in splits.items()
    }


def score_group(
    model: SparseOverrideMlp,
    group: dict[str, Any],
) -> tuple[torch.Tensor, torch.Tensor, list[float], list[bool]]:
    examples = group["examples"]
    all_indices = torch.cat([example["indices"] for example in examples])
    all_weights = torch.cat([example["weights"] for example in examples])
    offsets = [0]
    total = 0
    for example in examples:
        total += int(example["indices"].numel())
        offsets.append(total)
    offsets_tensor = torch.tensor(offsets, dtype=torch.long, device=all_indices.device)
    logits = model(all_indices, offsets_tensor, all_weights)
    targets = torch.tensor(
        [example["target"] for example in examples],
        dtype=torch.float32,
        device=all_indices.device,
    )
    advs = [float(example["adv"]) for example in examples]
    is_rule = [bool(example["is_rule"]) for example in examples]
    return logits, targets, advs, is_rule


def evaluate_groups(
    model: SparseOverrideMlp,
    groups: list[dict[str, Any]],
    thresholds: list[float],
) -> dict[str, Any]:
    rows = []
    with torch.no_grad():
        for group in groups:
            logits, targets_tensor, advs, is_rule = score_group(model, group)
            probs = torch.sigmoid(logits).detach().cpu().tolist()
            targets = targets_tensor.detach().cpu().tolist()
            for prob, target, adv, rule_flag in zip(probs, targets, advs, is_rule):
                rows.append(
                    {
                        "prob": float(prob),
                        "target": float(target),
                        "adv": float(adv),
                        "is_rule": bool(rule_flag),
                    }
                )
    if not rows:
        return {"count": 0}
    return {
        "count": len(rows),
        "positive_count": sum(1 for row in rows if row["target"] > 0.5),
        "positive_rate": mean(row["target"] for row in rows),
        "average_precision": average_precision(rows),
        "roc_auc": roc_auc(rows),
        "thresholds": {
            str(threshold): threshold_metrics(rows, threshold)
            for threshold in thresholds
        },
    }


def threshold_metrics(rows: list[dict[str, Any]], threshold: float) -> dict[str, Any]:
    selected = [row for row in rows if row["prob"] >= threshold and not row["is_rule"]]
    positives = [row for row in rows if row["target"] > 0.5]
    true_selected = [row for row in selected if row["target"] > 0.5]
    harmful = [row for row in selected if row["adv"] < 0.0]
    return {
        "selected_count": len(selected),
        "override_rate": len(selected) / len(rows) if rows else 0.0,
        "precision": len(true_selected) / len(selected) if selected else None,
        "recall": len(true_selected) / len(positives) if positives else None,
        "false_positive_rate": (
            (len(selected) - len(true_selected))
            / max(sum(1 for row in rows if row["target"] <= 0.5), 1)
        ),
        "harmful_override_count": len(harmful),
        "harmful_override_rate": len(harmful) / len(selected) if selected else None,
        "accepted_override_real_adv": mean(row["adv"] for row in selected) if selected else None,
    }


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


def roc_auc(rows: list[dict[str, Any]]) -> float | None:
    positives = [row for row in rows if row["target"] > 0.5]
    negatives = [row for row in rows if row["target"] <= 0.5]
    if not positives or not negatives:
        return None
    wins = 0.0
    total = 0
    for pos in positives:
        for neg in negatives:
            total += 1
            if pos["prob"] > neg["prob"]:
                wins += 1.0
            elif pos["prob"] == neg["prob"]:
                wins += 0.5
    return wins / total if total else None


def label_counts(rows: list[dict[str, Any]]) -> dict[str, int]:
    counts: defaultdict[str, int] = defaultdict(int)
    for row in rows:
        counts[str(row.get("safe_override_label") or "missing")] += 1
    return dict(sorted(counts.items()))


def split_counts(rows: list[dict[str, Any]]) -> dict[str, int]:
    counts = {"train": 0, "valid": 0, "test": 0}
    for row in rows:
        split = str(row.get("split") or "")
        if split in counts:
            counts[split] += 1
    return counts


def gate(metrics: dict[str, Any], feature_set: str) -> dict[str, Any]:
    test = (metrics.get(feature_set) or {}).get("test") or {}
    threshold_08 = (test.get("thresholds") or {}).get("0.8") or {}
    failures = []
    if not finite_metric(test.get("average_precision")):
        failures.append("test average_precision is unavailable")
    if int(threshold_08.get("selected_count") or 0) == 0:
        failures.append("no heldout overrides selected at threshold 0.8")
    adv = threshold_08.get("accepted_override_real_adv")
    if finite_metric(adv) and float(adv) <= 0.0:
        failures.append("heldout selected overrides have non-positive true advantage")
    return {
        "return_advantage_override_offline_gate_passed": not failures,
        "failures": failures,
    }


def finite_metric(value: Any) -> bool:
    return isinstance(value, (int, float)) and math.isfinite(float(value))


def mean(values: Any) -> float:
    values = list(values)
    return sum(values) / len(values) if values else 0.0


if __name__ == "__main__":
    main()
