#!/usr/bin/env python3
"""Audit DecisionRecord teacher labels before any training job consumes them."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any, Iterable


def iter_jsonl(path: Path) -> Iterable[tuple[int, dict[str, Any]]]:
    with path.open("r", encoding="utf-8") as handle:
        for line_no, line in enumerate(handle, start=1):
            line = line.strip()
            if not line:
                continue
            yield line_no, json.loads(line)


def teacher_payload(record: dict[str, Any]) -> dict[str, Any] | None:
    label = record.get("teacher_label")
    if not isinstance(label, dict):
        return None
    payload = label.get("payload")
    return payload if isinstance(payload, dict) else {}


def eligibility(record: dict[str, Any]) -> dict[str, Any]:
    payload = teacher_payload(record)
    if payload is None:
        return {
            "eligible_for_training": False,
            "label_use": "missing_teacher_label",
            "ineligibility_reasons": ["missing_teacher_label"],
        }
    gate = payload.get("training_eligibility")
    if isinstance(gate, dict):
        return gate
    return compute_fallback_eligibility(record, payload)


def compute_fallback_eligibility(record: dict[str, Any], payload: dict[str, Any]) -> dict[str, Any]:
    reasons: list[str] = []
    labels = record.get("teacher_label", {}).get("labels") or []
    pairwise = record.get("teacher_label", {}).get("pairwise_preferences") or []
    if payload.get("live_env_unchanged") is not True:
        reasons.append("live_env_changed_or_unchecked")
    if len(labels) < 2:
        reasons.append("fewer_than_two_candidates")
    if not pairwise:
        reasons.append("no_strict_pairwise_preferences")
    if payload.get("horizon_mode") != "combat_end_v1":
        reasons.append(f"horizon_mode_not_strict_trainable:{payload.get('horizon_mode')}")
    for label in labels:
        lp = label.get("payload") or {}
        if not lp.get("ok", True):
            reasons.append("candidate_evaluation_error")
        if lp.get("horizon_stop_reason") == "horizon_decision_cap":
            reasons.append("horizon_decision_cap_hit")
        final_info = lp.get("final_info") or {}
        if final_info.get("result") in {"truncated", "crash"}:
            reasons.append("truncated_or_crash_final_info")
    reasons = sorted(set(reasons))
    return {
        "eligible_for_training": not reasons,
        "label_use": "trainable_pairwise" if not reasons else "audit_or_screening_only",
        "ineligibility_reasons": reasons,
        "candidate_count": len(labels),
        "pairwise_count": len(pairwise),
        "computed_by": "audit_fallback",
    }


def audit(paths: list[Path]) -> dict[str, Any]:
    total_records = 0
    missing_teacher = 0
    eligible_records = 0
    label_use_counts: Counter[str] = Counter()
    reason_counts: Counter[str] = Counter()
    teacher_spec_counts: Counter[str] = Counter()
    horizon_mode_counts: Counter[str] = Counter()
    pairwise_count_total = 0
    trainable_pairwise_count_total = 0
    examples: list[dict[str, Any]] = []

    for path in paths:
        for line_no, record in iter_jsonl(path):
            total_records += 1
            label = record.get("teacher_label")
            if not isinstance(label, dict):
                missing_teacher += 1
                label_use_counts["missing_teacher_label"] += 1
                reason_counts["missing_teacher_label"] += 1
                continue
            payload = label.get("payload") or {}
            teacher_spec_counts[str(label.get("teacher_spec_version"))] += 1
            horizon_mode_counts[str(payload.get("horizon_mode"))] += 1
            gate = eligibility(record)
            label_use = str(gate.get("label_use") or "unknown")
            label_use_counts[label_use] += 1
            pairwise_count = int(gate.get("pairwise_count") or len(label.get("pairwise_preferences") or []))
            pairwise_count_total += pairwise_count
            if gate.get("eligible_for_training"):
                eligible_records += 1
                trainable_pairwise_count_total += pairwise_count
            else:
                for reason in gate.get("ineligibility_reasons") or ["unknown_ineligible"]:
                    reason_counts[str(reason)] += 1
                if len(examples) < 12:
                    examples.append(
                        {
                            "path": str(path),
                            "line": line_no,
                            "decision_id": record.get("decision_id"),
                            "label_use": label_use,
                            "reasons": gate.get("ineligibility_reasons") or [],
                            "horizon_mode": payload.get("horizon_mode"),
                            "candidate_count": gate.get("candidate_count"),
                            "pairwise_count": pairwise_count,
                        }
                    )

    return {
        "schema_version": "decision_record_teacher_quality_audit_v0",
        "inputs": [str(path) for path in paths],
        "total_records": total_records,
        "missing_teacher_label_count": missing_teacher,
        "teacher_labeled_record_count": total_records - missing_teacher,
        "eligible_record_count": eligible_records,
        "eligible_record_rate": eligible_records / total_records if total_records else 0.0,
        "pairwise_count_total": pairwise_count_total,
        "trainable_pairwise_count_total": trainable_pairwise_count_total,
        "label_use_counts": dict(sorted(label_use_counts.items())),
        "ineligibility_reason_counts": dict(sorted(reason_counts.items())),
        "teacher_spec_counts": dict(sorted(teacher_spec_counts.items())),
        "horizon_mode_counts": dict(sorted(horizon_mode_counts.items())),
        "ineligible_examples": examples,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inputs", nargs="+", type=Path, required=True)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--fail-if-no-trainable", action="store_true")
    parser.add_argument("--min-eligible-records", type=int, default=0)
    parser.add_argument("--min-trainable-pairs", type=int, default=0)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    report = audit(args.inputs)
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    fail_reasons = []
    if args.fail_if_no_trainable and report["eligible_record_count"] == 0:
        fail_reasons.append("no eligible teacher records")
    if report["eligible_record_count"] < args.min_eligible_records:
        fail_reasons.append(
            f"eligible_record_count {report['eligible_record_count']} < {args.min_eligible_records}"
        )
    if report["trainable_pairwise_count_total"] < args.min_trainable_pairs:
        fail_reasons.append(
            "trainable_pairwise_count_total "
            f"{report['trainable_pairwise_count_total']} < {args.min_trainable_pairs}"
        )
    if fail_reasons:
        raise SystemExit("; ".join(fail_reasons))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
