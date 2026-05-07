#!/usr/bin/env python3
"""Train a group/listwise Torch proposer for verified-H candidate filtering.

This model is a proposer, not a policy.  It scores non-rule candidates so the
exact H-step verifier can evaluate fewer candidates while keeping the rule
candidate as the fallback.
"""
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

from return_q_common import ADV_OVERRIDE_FEATURE_SETS, adv_override_features, read_jsonl, write_json
from train_verified_proposer_linear import (
    average_precision,
    best_for_target_recall,
    hybrid_metrics,
    label_counts,
    parse_float_list,
    parse_int_list,
    threshold_metrics,
    topk_metrics,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--model-out", type=Path, required=True)
    parser.add_argument("--state-dict-out", type=Path)
    parser.add_argument("--report-out", type=Path)
    parser.add_argument("--feature-set", default="full_decision_plus_choice", choices=ADV_OVERRIDE_FEATURE_SETS)
    parser.add_argument("--target-mode", default="oracle_chosen", choices=["oracle_chosen", "margin_positive"])
    parser.add_argument("--feature-dim", type=int, default=32768)
    parser.add_argument("--hidden-dim", type=int, default=96)
    parser.add_argument("--dropout-p", type=float, default=0.05)
    parser.add_argument("--epochs", type=int, default=12)
    parser.add_argument("--learning-rate", type=float, default=0.0015)
    parser.add_argument("--weight-decay", type=float, default=0.0001)
    parser.add_argument("--listwise-weight", type=float, default=1.0)
    parser.add_argument("--bce-weight", type=float, default=0.25)
    parser.add_argument("--negative-group-bce-weight", type=float, default=0.03)
    parser.add_argument("--positive-groups-only", action="store_true")
    parser.add_argument("--max-pos-weight", type=float, default=20.0)
    parser.add_argument("--seed", type=int, default=47)
    parser.add_argument("--device", default="cpu")
    parser.add_argument("--thresholds", default="0.01,0.02,0.05,0.1,0.2,0.3,0.5,0.7,0.9")
    parser.add_argument("--top-k", default="1,2,3,4,6,8")
    parser.add_argument("--target-recalls", default="0.8,0.9,0.95")
    parser.add_argument("--report-splits", default="train,valid,test")
    return parser.parse_args()


class SparseProposer(nn.Module):
    def __init__(self, feature_dim: int, hidden_dim: int, dropout_p: float) -> None:
        super().__init__()
        layers: list[nn.Module] = [
            nn.LayerNorm(hidden_dim),
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
        ]
        if dropout_p > 0.0:
            layers.append(nn.Dropout(p=dropout_p))
        layers.append(nn.Linear(hidden_dim, 1))
        self.embedding = nn.EmbeddingBag(
            feature_dim,
            hidden_dim,
            mode="sum",
            include_last_offset=True,
        )
        self.net = nn.Sequential(*layers)

    def forward(self, indices: torch.Tensor, offsets: torch.Tensor, weights: torch.Tensor) -> torch.Tensor:
        embedded = self.embedding(indices, offsets, per_sample_weights=weights)
        return self.net(embedded).squeeze(-1)


def main() -> None:
    args = parse_args()
    start = time.perf_counter()
    rng = random.Random(args.seed)
    torch.manual_seed(args.seed)
    rows = read_jsonl(args.input)
    groups = prepare_groups(rows, args.feature_set, args.target_mode, args.device)
    if not any(groups[split] for split in groups):
        raise SystemExit("no trainable proposer groups")
    train_examples = [ex for group in groups["train"] for ex in group["examples"]]
    positives = sum(1 for ex in train_examples if ex["target"] > 0.5)
    negatives = max(len(train_examples) - positives, 1)
    pos_weight = min(math.sqrt(negatives / max(positives, 1)), args.max_pos_weight)

    model = SparseProposer(args.feature_dim, args.hidden_dim, args.dropout_p).to(torch.device(args.device))
    optimizer = torch.optim.AdamW(
        model.parameters(),
        lr=args.learning_rate,
        weight_decay=args.weight_decay,
    )
    train_groups = [
        group
        for group in groups["train"]
        if not args.positive_groups_only or group_positive_count(group) > 0
    ]
    epoch_reports = []
    for epoch in range(max(args.epochs, 0)):
        rng.shuffle(train_groups)
        model.train()
        total_loss = 0.0
        updated = 0
        for group in train_groups:
            loss = group_loss(model, group, args, pos_weight)
            if loss is None:
                continue
            optimizer.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 5.0)
            optimizer.step()
            total_loss += float(loss.detach().cpu())
            updated += 1
        epoch_reports.append({"epoch": epoch + 1, "updated_groups": updated, "loss": total_loss / updated if updated else None})
    model.eval()

    state_dict_out = args.state_dict_out or args.model_out.with_suffix(".pt")
    state_dict_out.parent.mkdir(parents=True, exist_ok=True)
    torch.save(model.to("cpu").state_dict(), state_dict_out)
    model_payload = {
        "schema_version": "verified_proposer_torch_embedding_mlp_v0",
        "model_type": "verified_proposer_torch_embedding_mlp_v0",
        "feature_set": args.feature_set,
        "target_mode": f"verified_h_{args.target_mode}_listwise_candidate_proposer",
        "state_dict_path": str(state_dict_out),
        "config": {
            "feature_dim": args.feature_dim,
            "hidden_dim": args.hidden_dim,
            "dropout_p": args.dropout_p,
            "epochs": args.epochs,
            "learning_rate": args.learning_rate,
            "weight_decay": args.weight_decay,
            "listwise_weight": args.listwise_weight,
            "bce_weight": args.bce_weight,
            "negative_group_bce_weight": args.negative_group_bce_weight,
            "positive_groups_only": args.positive_groups_only,
            "pos_weight": pos_weight,
            "seed": args.seed,
            "target_mode": args.target_mode,
        },
    }
    write_json(args.model_out, model_payload)

    thresholds = parse_float_list(args.thresholds)
    top_ks = parse_int_list(args.top_k)
    target_recalls = parse_float_list(args.target_recalls)
    report_splits = [split for split in parse_split_list(args.report_splits) if split in groups]
    metrics = {
        split: evaluate_groups(model, groups[split], thresholds, top_ks, target_recalls)
        for split in report_splits
    }
    examples = [ex for split in ["train", "valid", "test"] for group in groups[split] for ex in group["examples"]]
    report = {
        "schema_version": "verified_proposer_torch_listwise_report_v0",
        "input": str(args.input),
        "model_out": str(args.model_out),
        "state_dict_out": str(state_dict_out),
        "row_count": len(rows),
        "example_count": len(examples),
        "group_counts": {split: len(groups[split]) for split in ["train", "valid", "test"]},
        "label_counts": label_counts(examples),
        "metrics": metrics,
        "epoch_reports": epoch_reports,
        "runtime_seconds": time.perf_counter() - start,
        "torch": {
            "version": torch.__version__,
            "device": args.device,
            "cuda_available": torch.cuda.is_available(),
        },
    }
    write_json(args.report_out or args.model_out.with_suffix(".report.json"), report)
    print(json.dumps(render_compact(report), indent=2, sort_keys=True))


def prepare_groups(
    rows: list[dict[str, Any]],
    feature_set: str,
    target_mode: str,
    device: str,
) -> dict[str, list[dict[str, Any]]]:
    by_group: defaultdict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        by_group[str(row.get("group_key") or "")].append(row)
    groups: dict[str, list[dict[str, Any]]] = {"train": [], "valid": [], "test": []}
    for group_key, group_rows in by_group.items():
        split = str(group_rows[0].get("split") or "train")
        if split not in groups:
            continue
        examples = []
        for row in group_rows:
            if bool(row.get("is_rule_choice")):
                continue
            target = target_value(row, target_mode)
            if target is None:
                continue
            sparse = adv_override_features(row, feature_set)
            examples.append(
                {
                    "indices": torch.tensor(list(sparse.keys()) or [0], dtype=torch.long, device=device),
                    "weights": torch.tensor(list(sparse.values()) or [0.0], dtype=torch.float32, device=device),
                    "target": float(target),
                    "adv": float(row.get("adv_vs_rule_mean") or 0.0),
                    "is_rule": False,
                    "group_key": group_key,
                }
            )
        if examples:
            groups[split].append({"group_key": group_key, "examples": examples})
    return groups


def target_value(row: dict[str, Any], target_mode: str) -> float | None:
    if target_mode == "oracle_chosen":
        return 1.0 if bool(row.get("is_full_verified_choice")) else 0.0
    label = str(row.get("safe_override_label") or "")
    if label not in {"positive", "negative"}:
        return None
    return 1.0 if label == "positive" else 0.0


def group_loss(
    model: SparseProposer,
    group: dict[str, Any],
    args: argparse.Namespace,
    pos_weight: float,
) -> torch.Tensor | None:
    scores, targets = score_group(model, group)
    positive_count = float(targets.sum().item())
    losses = []
    if positive_count > 0:
        target_probs = targets / positive_count
        losses.append(args.listwise_weight * (-(target_probs * F.log_softmax(scores, dim=0)).sum()))
        if args.bce_weight > 0:
            pos_weight_tensor = torch.tensor(pos_weight, dtype=torch.float32, device=scores.device)
            losses.append(
                args.bce_weight
                * F.binary_cross_entropy_with_logits(scores, targets, pos_weight=pos_weight_tensor)
            )
    elif args.negative_group_bce_weight > 0:
        losses.append(
            args.negative_group_bce_weight
            * F.binary_cross_entropy_with_logits(scores, targets)
        )
    if not losses:
        return None
    return sum(losses)


def group_positive_count(group: dict[str, Any]) -> int:
    return sum(1 for example in group["examples"] if float(example["target"]) > 0.5)


def score_group(model: SparseProposer, group: dict[str, Any]) -> tuple[torch.Tensor, torch.Tensor]:
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
    model: SparseProposer,
    groups: list[dict[str, Any]],
    thresholds: list[float],
    top_ks: list[int],
    target_recalls: list[float],
) -> dict[str, Any]:
    rows = []
    by_group: defaultdict[str, list[dict[str, Any]]] = defaultdict(list)
    with torch.no_grad():
        for group in groups:
            scores, _targets = score_group(model, group)
            probs = torch.sigmoid(scores).detach().cpu().tolist()
            for example, prob in zip(group["examples"], probs):
                row = {
                    "group_key": example["group_key"],
                    "prob": float(prob),
                    "target": float(example["target"]),
                    "adv": float(example["adv"]),
                    "is_rule": False,
                }
                rows.append(row)
                by_group[row["group_key"]].append(row)
    if not rows:
        return {"count": 0}
    threshold_rows = {str(threshold): threshold_metrics(rows, by_group, threshold) for threshold in thresholds}
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
        "count": len(rows),
        "positive_count": sum(1 for row in rows if row["target"] > 0.5),
        "positive_rate": mean(row["target"] for row in rows),
        "average_precision": average_precision(rows),
        "thresholds": threshold_rows,
        "top_k": topk_rows,
        "hybrid": hybrid_rows,
        "best_for_target_recall": {
            str(target): best_for_target_recall(candidates, target)
            for target in target_recalls
        },
    }


def render_compact(report: dict[str, Any]) -> dict[str, Any]:
    return {
        "model_out": report["model_out"],
        "row_count": report["row_count"],
        "example_count": report["example_count"],
        "group_counts": report["group_counts"],
        "label_counts": report["label_counts"],
        "runtime_seconds": report["runtime_seconds"],
        "torch": report["torch"],
        "test": {
            "average_precision": (report["metrics"].get("test") or {}).get("average_precision"),
            "best_for_target_recall": (report["metrics"].get("test") or {}).get("best_for_target_recall"),
        },
    }


def mean(values: Any) -> float:
    values = list(values)
    return sum(values) / len(values) if values else 0.0


def parse_split_list(value: str) -> list[str]:
    return [item.strip() for item in value.split(",") if item.strip()]


if __name__ == "__main__":
    main()
