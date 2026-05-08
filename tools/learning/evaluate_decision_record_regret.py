#!/usr/bin/env python3
"""Evaluate behavior/model action regret from DecisionRecord teacher labels."""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable


def iter_jsonl(paths: list[Path]) -> Iterable[tuple[Path, int, dict[str, Any]]]:
    for path in paths:
        with path.open("r", encoding="utf-8") as handle:
            for line_no, line in enumerate(handle, start=1):
                line = line.strip()
                if not line:
                    continue
                yield path, line_no, json.loads(line)


def action_id_value(value: Any) -> int | None:
    if value is None:
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, dict) and "0" in value and isinstance(value["0"], int):
        return value["0"]
    if isinstance(value, list) and len(value) == 1 and isinstance(value[0], int):
        return value[0]
    return None


def finite_float(value: Any) -> float | None:
    try:
        out = float(value)
    except (TypeError, ValueError):
        return None
    return out if math.isfinite(out) else None


def candidate_returns(record: dict[str, Any]) -> dict[int, float]:
    label = record.get("teacher_label")
    if not isinstance(label, dict):
        return {}
    returns: dict[int, float] = {}
    for candidate in label.get("labels") or []:
        action_id = action_id_value(candidate.get("action_id"))
        mean_return = finite_float(candidate.get("mean_return"))
        if action_id is not None and mean_return is not None:
            returns[action_id] = mean_return
    return returns


def eligibility(record: dict[str, Any]) -> dict[str, Any]:
    label = record.get("teacher_label")
    if not isinstance(label, dict):
        return {
            "eligible_for_training": False,
            "label_use": "missing_teacher_label",
            "ineligibility_reasons": ["missing_teacher_label"],
        }
    payload = label.get("payload") or {}
    gate = payload.get("training_eligibility")
    if isinstance(gate, dict):
        return gate
    return {
        "eligible_for_training": False,
        "label_use": "legacy_or_ungated_teacher_label",
        "ineligibility_reasons": ["legacy_or_ungated_teacher_label"],
    }


def bucket_for_regret(regret: float) -> str:
    if regret <= 1e-6:
        return "best_or_tied"
    if regret < 0.25:
        return "lt_0_25"
    if regret < 0.5:
        return "0_25_to_0_5"
    if regret < 1.0:
        return "0_5_to_1"
    if regret < 2.0:
        return "1_to_2"
    return "gte_2"


def update_group_stats(stats: dict[str, Any], regret: float, harmful: bool) -> None:
    stats["count"] += 1
    stats["regret_sum"] += regret
    stats["regret_max"] = max(stats["regret_max"], regret)
    if harmful:
        stats["harmful_count"] += 1


def new_group_stats() -> dict[str, Any]:
    return {"count": 0, "regret_sum": 0.0, "regret_max": 0.0, "harmful_count": 0}


def finalize_group_stats(stats: dict[str, Any]) -> dict[str, Any]:
    count = stats["count"]
    return {
        "count": count,
        "mean_regret": stats["regret_sum"] / count if count else 0.0,
        "max_regret": stats["regret_max"],
        "harmful_count": stats["harmful_count"],
        "harmful_rate": stats["harmful_count"] / count if count else 0.0,
    }


def evaluate(paths: list[Path], harmful_margin: float, eligible_only: bool) -> dict[str, Any]:
    total_records = 0
    teacher_labeled = 0
    evaluated_records = 0
    skipped_counts: Counter[str] = Counter()
    regret_sum = 0.0
    max_regret = 0.0
    harmful_count = 0
    exact_best_count = 0
    regret_buckets: Counter[str] = Counter()
    by_decision_type: dict[str, dict[str, Any]] = defaultdict(new_group_stats)
    by_chosen_kind: dict[str, dict[str, Any]] = defaultdict(new_group_stats)
    examples: list[dict[str, Any]] = []

    for path, line_no, record in iter_jsonl(paths):
        total_records += 1
        if not isinstance(record.get("teacher_label"), dict):
            skipped_counts["missing_teacher_label"] += 1
            continue
        teacher_labeled += 1
        gate = eligibility(record)
        if eligible_only and not gate.get("eligible_for_training"):
            skipped_counts[f"ineligible:{gate.get('label_use')}"] += 1
            continue
        returns = candidate_returns(record)
        if not returns:
            skipped_counts["no_numeric_candidate_returns"] += 1
            continue
        chosen = action_id_value(record.get("behavior_action"))
        if chosen is None:
            skipped_counts["missing_behavior_action"] += 1
            continue
        if chosen not in returns:
            skipped_counts["chosen_action_missing_from_teacher_labels"] += 1
            continue

        best_action, best_return = max(returns.items(), key=lambda item: item[1])
        chosen_return = returns[chosen]
        regret = max(0.0, best_return - chosen_return)
        harmful = regret > harmful_margin
        evaluated_records += 1
        regret_sum += regret
        max_regret = max(max_regret, regret)
        harmful_count += int(harmful)
        exact_best_count += int(regret <= 1e-6)
        regret_buckets[bucket_for_regret(regret)] += 1

        decision_type = str((record.get("decision_id") or {}).get("decision_type") or "unknown")
        candidate_by_id = {
            action_id_value(candidate.get("id")): candidate for candidate in record.get("candidates") or []
        }
        chosen_kind = str((candidate_by_id.get(chosen) or {}).get("action_kind") or "unknown")
        update_group_stats(by_decision_type[decision_type], regret, harmful)
        update_group_stats(by_chosen_kind[chosen_kind], regret, harmful)

        if harmful and len(examples) < 16:
            examples.append(
                {
                    "path": str(path),
                    "line": line_no,
                    "decision_id": record.get("decision_id"),
                    "chosen_action": chosen,
                    "chosen_action_key": (candidate_by_id.get(chosen) or {}).get("action_key"),
                    "chosen_return": chosen_return,
                    "best_action": best_action,
                    "best_action_key": (candidate_by_id.get(best_action) or {}).get("action_key"),
                    "best_return": best_return,
                    "regret": regret,
                    "eligibility": gate,
                }
            )

    return {
        "schema_version": "decision_record_regret_eval_v0",
        "inputs": [str(path) for path in paths],
        "eligible_only": eligible_only,
        "harmful_margin": harmful_margin,
        "total_records": total_records,
        "teacher_labeled_records": teacher_labeled,
        "evaluated_records": evaluated_records,
        "skipped_counts": dict(sorted(skipped_counts.items())),
        "mean_regret": regret_sum / evaluated_records if evaluated_records else 0.0,
        "max_regret": max_regret,
        "exact_best_count": exact_best_count,
        "exact_best_rate": exact_best_count / evaluated_records if evaluated_records else 0.0,
        "harmful_count": harmful_count,
        "harmful_rate": harmful_count / evaluated_records if evaluated_records else 0.0,
        "regret_buckets": dict(sorted(regret_buckets.items())),
        "by_decision_type": {
            key: finalize_group_stats(value) for key, value in sorted(by_decision_type.items())
        },
        "by_chosen_action_kind": {
            key: finalize_group_stats(value) for key, value in sorted(by_chosen_kind.items())
        },
        "harmful_examples": examples,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inputs", nargs="+", type=Path, required=True)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--harmful-margin", type=float, default=0.5)
    parser.add_argument("--eligible-only", action="store_true")
    parser.add_argument("--max-harmful-rate", type=float)
    parser.add_argument("--max-mean-regret", type=float)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    report = evaluate(args.inputs, args.harmful_margin, args.eligible_only)
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    failures: list[str] = []
    if args.max_harmful_rate is not None and report["harmful_rate"] > args.max_harmful_rate:
        failures.append(f"harmful_rate {report['harmful_rate']} > {args.max_harmful_rate}")
    if args.max_mean_regret is not None and report["mean_regret"] > args.max_mean_regret:
        failures.append(f"mean_regret {report['mean_regret']} > {args.max_mean_regret}")
    if failures:
        raise SystemExit("; ".join(failures))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
