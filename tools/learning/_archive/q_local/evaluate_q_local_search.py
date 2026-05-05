#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, iter_jsonl, write_json, write_jsonl


def load_rows(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [row for _, row in iter_jsonl(path)]


def main() -> int:
    parser = argparse.ArgumentParser(description="Evaluate Q_local predictions as an offline search prior.")
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--predictions", default=None, type=Path)
    parser.add_argument("--rows", default=None, type=Path)
    parser.add_argument("--eval-out", default=None, type=Path)
    parser.add_argument("--samples-out", default=None, type=Path)
    parser.add_argument("--retained-threshold", default=0.35, type=float)
    parser.add_argument("--topk", default=2, type=int)
    parser.add_argument("--hard-gap", default=1.0, type=float)
    args = parser.parse_args()

    predictions_path = args.predictions or (args.dataset_dir / "q_local_predictions.jsonl")
    rows_path = args.rows or (args.dataset_dir / "combat_q_local_test.jsonl")
    eval_out = args.eval_out or (args.dataset_dir / "q_local_search_eval.json")
    samples_out = args.samples_out or (args.dataset_dir / "q_local_search_eval_samples.jsonl")

    preds = load_rows(predictions_path)
    rows = load_rows(rows_path)
    pred_by_key = {
        (str(row.get("group_id") or ""), str(row.get("candidate_move") or "")): row
        for row in preds
    }
    merged_groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        key = (str(row.get("group_id") or ""), str(row.get("candidate_move") or ""))
        merged = dict(row)
        merged.update(pred_by_key.get(key, {}))
        merged_groups[str(row.get("group_id") or "")].append(merged)

    sample_rows: list[dict[str, Any]] = []
    eligible = 0
    baseline_top1 = 0
    qlocal_top1 = 0
    baseline_topk = 0
    qlocal_topk = 0
    corrected_hard = 0
    hard_cases = 0
    retained_counts: list[int] = []
    legal_counts: list[int] = []
    by_bucket: dict[str, dict[str, float]] = defaultdict(lambda: {
        "groups": 0,
        "baseline_top1": 0,
        "qlocal_top1": 0,
        "baseline_topk": 0,
        "qlocal_topk": 0,
        "hard_cases": 0,
        "hard_corrected": 0,
    })

    for group_id, group_rows in merged_groups.items():
        if not group_rows or any(bool(row.get("uncertain")) for row in group_rows):
            continue
        positives = [row for row in group_rows if bool(row.get("candidate_is_best"))]
        if not positives:
            continue
        eligible += 1
        bucket = str(group_rows[0].get("eval_bucket") or "unknown")
        baseline_sorted = sorted(group_rows, key=lambda row: (int(row.get("candidate_rank") or 10_000), -float(row.get("candidate_score_hint") or 0.0)))
        qlocal_sorted = sorted(group_rows, key=lambda row: float(row.get("pred::aggregate") or 0.0), reverse=True)
        if bool(baseline_sorted[0].get("candidate_is_best")):
            baseline_top1 += 1
            by_bucket[bucket]["baseline_top1"] += 1
        if bool(qlocal_sorted[0].get("candidate_is_best")):
            qlocal_top1 += 1
            by_bucket[bucket]["qlocal_top1"] += 1
        if any(bool(row.get("candidate_is_best")) for row in baseline_sorted[: args.topk]):
            baseline_topk += 1
            by_bucket[bucket]["baseline_topk"] += 1
        if any(bool(row.get("candidate_is_best")) for row in qlocal_sorted[: args.topk]):
            qlocal_topk += 1
            by_bucket[bucket]["qlocal_topk"] += 1

        best_row = qlocal_sorted[0]
        baseline_best = baseline_sorted[0]
        best_teacher = max(float(row.get("mean_return") or 0.0) for row in group_rows)
        baseline_gap = best_teacher - float(baseline_best.get("mean_return") or 0.0)
        hard_case = (not bool(baseline_best.get("candidate_is_best"))) and baseline_gap >= float(args.hard_gap)
        if hard_case:
            hard_cases += 1
            by_bucket[bucket]["hard_cases"] += 1
            if bool(best_row.get("candidate_is_best")):
                corrected_hard += 1
                by_bucket[bucket]["hard_corrected"] += 1

        threshold = float(best_row.get("pred::aggregate") or 0.0) - float(args.retained_threshold)
        retained = [row for row in qlocal_sorted if float(row.get("pred::aggregate") or 0.0) >= threshold]
        retained_counts.append(len(retained))
        legal_counts.append(len(group_rows))
        by_bucket[bucket]["groups"] += 1

        sample_rows.append(
            {
                "group_id": group_id,
                "eval_bucket": bucket,
                "curriculum_tag": group_rows[0].get("curriculum_tag"),
                "baseline_move": baseline_best.get("candidate_move"),
                "qlocal_move": best_row.get("candidate_move"),
                "teacher_best_move": next(row.get("candidate_move") for row in group_rows if bool(row.get("candidate_is_best"))),
                "baseline_top1_correct": bool(baseline_best.get("candidate_is_best")),
                "qlocal_top1_correct": bool(best_row.get("candidate_is_best")),
                "hard_case": hard_case,
                "baseline_gap": baseline_gap,
                "retained_candidates": len(retained),
                "legal_candidates": len(group_rows),
                "baseline_order": [
                    {
                        "move": row.get("candidate_move"),
                        "rank": row.get("candidate_rank"),
                        "score_hint": row.get("candidate_score_hint"),
                        "teacher_best": bool(row.get("candidate_is_best")),
                    }
                    for row in baseline_sorted
                ],
                "qlocal_order": [
                    {
                        "move": row.get("candidate_move"),
                        "aggregate": row.get("pred::aggregate"),
                        "teacher_best": bool(row.get("candidate_is_best")),
                    }
                    for row in qlocal_sorted
                ],
            }
        )

    bucket_summary = {}
    for bucket, stats in by_bucket.items():
        groups = max(int(stats["groups"]), 1)
        hard = max(int(stats["hard_cases"]), 1)
        bucket_summary[bucket] = {
            "groups": int(stats["groups"]),
            "root_top1_match": float(stats["qlocal_top1"] / groups),
            "baseline_root_top1_match": float(stats["baseline_top1"] / groups),
            "topk_recall": float(stats["qlocal_topk"] / groups),
            "baseline_topk_recall": float(stats["baseline_topk"] / groups),
            "hard_mistake_suppression": float(stats["hard_corrected"] / hard) if int(stats["hard_cases"]) else 0.0,
        }

    avg_retained = sum(retained_counts) / len(retained_counts) if retained_counts else 0.0
    avg_legal = sum(legal_counts) / len(legal_counts) if legal_counts else 0.0
    summary = {
        "groups_evaluated": eligible,
        "root_top1_match": float(qlocal_top1 / eligible) if eligible else 0.0,
        "baseline_root_top1_match": float(baseline_top1 / eligible) if eligible else 0.0,
        "topk_recall": float(qlocal_topk / eligible) if eligible else 0.0,
        "baseline_topk_recall": float(baseline_topk / eligible) if eligible else 0.0,
        "node_reduction_proxy": float(1.0 - (avg_retained / avg_legal)) if avg_legal else 0.0,
        "avg_retained_candidates": avg_retained,
        "avg_legal_candidates": avg_legal,
        "hard_mistake_suppression": float(corrected_hard / hard_cases) if hard_cases else 0.0,
        "hard_case_count": hard_cases,
        "retained_threshold": args.retained_threshold,
        "topk": args.topk,
        "hard_gap": args.hard_gap,
        "by_bucket": bucket_summary,
        "notes": [
            "offline search replay eval compares current candidate order against Q_local aggregate prior",
            "node_reduction_proxy is derived from retained-candidate thresholding, not live node counts",
            "hard mistake suppression counts cases where baseline top1 misses a clearly better local-oracle action",
        ],
    }
    write_json(eval_out, summary)
    write_jsonl(samples_out, sample_rows)
    print(json.dumps(summary, indent=2, ensure_ascii=False))
    print(f"wrote Q_local search eval to {eval_out}")
    print(f"wrote Q_local search eval samples to {samples_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
