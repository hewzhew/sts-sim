#!/usr/bin/env python3
"""Evaluate a DecisionRecord scorer as a conservative gated override policy."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from train_decision_record_pairwise_scorer import (
    action_id_value,
    candidate_by_id,
    dot,
    features_for,
    label_returns,
)


def iter_jsonl(paths: list[Path]):
    for path in paths:
        with path.open("r", encoding="utf-8") as handle:
            for line_no, line in enumerate(handle, start=1):
                line = line.strip()
                if line:
                    yield path, line_no, json.loads(line)


def load_model(path: Path) -> tuple[int, list[float]]:
    model = json.loads(path.read_text(encoding="utf-8"))
    dim = int(model["feature_dim"])
    weights = [0.0] * dim
    for key, value in (model.get("weights") or {}).items():
        weights[int(key)] = float(value)
    return dim, weights


def evaluate(paths: list[Path], model_path: Path, threshold: float, harmful_margin: float) -> dict[str, Any]:
    dim, weights = load_model(model_path)
    records = 0
    evaluable = 0
    override_count = 0
    harmful_override_count = 0
    true_adv_sum = 0.0
    score_margin_sum = 0.0
    examples: list[dict[str, Any]] = []

    for path, line_no, record in iter_jsonl(paths):
        records += 1
        candidates = candidate_by_id(record)
        returns = label_returns(record)
        reference = action_id_value(record.get("behavior_action"))
        if reference is None or reference not in candidates or reference not in returns:
            continue
        scores = {
            action_id: dot(weights, features_for(record, candidate, dim))
            for action_id, candidate in candidates.items()
        }
        if not scores:
            continue
        best_action, best_score = max(scores.items(), key=lambda item: item[1])
        ref_score = scores.get(reference, 0.0)
        score_margin = best_score - ref_score
        evaluable += 1
        if best_action == reference or score_margin < threshold:
            continue
        if best_action not in returns:
            continue
        override_count += 1
        true_adv = returns[best_action] - returns[reference]
        true_adv_sum += true_adv
        score_margin_sum += score_margin
        harmful = true_adv < -harmful_margin
        harmful_override_count += int(harmful)
        if len(examples) < 16:
            examples.append(
                {
                    "path": str(path),
                    "line": line_no,
                    "decision_id": record.get("decision_id"),
                    "reference_action": reference,
                    "override_action": best_action,
                    "reference_action_key": candidates[reference].get("action_key"),
                    "override_action_key": candidates[best_action].get("action_key"),
                    "score_margin": score_margin,
                    "true_adv": true_adv,
                    "harmful": harmful,
                }
            )

    return {
        "schema_version": "decision_record_gated_scorer_eval_v0",
        "inputs": [str(path) for path in paths],
        "model": str(model_path),
        "threshold": threshold,
        "harmful_margin": harmful_margin,
        "records": records,
        "evaluable_records": evaluable,
        "override_count": override_count,
        "override_rate": override_count / evaluable if evaluable else 0.0,
        "mean_score_margin_on_overrides": score_margin_sum / override_count if override_count else 0.0,
        "accepted_override_true_adv": true_adv_sum / override_count if override_count else 0.0,
        "harmful_override_count": harmful_override_count,
        "harmful_override_rate": harmful_override_count / override_count if override_count else 0.0,
        "override_examples": examples,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inputs", nargs="+", type=Path, required=True)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--threshold", type=float, default=0.5)
    parser.add_argument("--harmful-margin", type=float, default=0.5)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    report = evaluate(args.inputs, args.model, args.threshold, args.harmful_margin)
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
