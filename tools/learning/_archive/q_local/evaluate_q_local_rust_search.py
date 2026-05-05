#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT
from combat_reranker_common import iter_jsonl, write_json, write_jsonl


def normalize_replay_path(path: Any) -> str:
    return str(path or "").replace("/", "\\").lower()


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [row for _, row in iter_jsonl(path)]


def key_to_string(key: dict[str, Any]) -> str:
    kind = str(key.get("kind") or "unknown")
    if kind == "spec_episode_step":
        return f"spec_episode_step::{key.get('spec_name')}::{int(key.get('episode_id') or 0)}::{int(key.get('step_index') or 0)}"
    if kind == "replay_frame":
        return f"replay_frame::{normalize_replay_path(key.get('source_path'))}::{int(key.get('frame') or 0)}"
    return json.dumps(key, sort_keys=True, ensure_ascii=False)


def key_kind_from_string(key: str) -> str:
    if key.startswith("spec_episode_step::"):
        return "spec_episode_step"
    if key.startswith("replay_frame::"):
        return "replay_frame"
    return "unknown"


def record_key(record: dict[str, Any]) -> str:
    root_key = record.get("root_prior_key")
    if root_key:
        return str(root_key)
    return key_to_string(
        {
            "kind": "replay_frame",
            "source_path": normalize_replay_path(record.get("source_path")),
            "frame": record.get("frame"),
        }
    )


def top_move(record: dict[str, Any]) -> str | None:
    top_moves = list(record.get("top_moves") or [])
    if not top_moves:
        return None
    return str(top_moves[0].get("move_text") or "")


def topk_contains(record: dict[str, Any], teacher_moves: set[str], topk: int) -> bool:
    for move in list(record.get("top_moves") or [])[:topk]:
        if str(move.get("move_text") or "") in teacher_moves:
            return True
    return False


def main() -> int:
    parser = argparse.ArgumentParser(description="Evaluate Rust root-prior search exports against Q_local teacher rows.")
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--prior", required=True, type=Path)
    parser.add_argument("--prior-artifact", default=None, type=Path)
    parser.add_argument("--eval-out", default=None, type=Path)
    parser.add_argument("--samples-out", default=None, type=Path)
    parser.add_argument("--retained-threshold", default=0.35, type=float)
    parser.add_argument("--topk", default=2, type=int)
    parser.add_argument("--hard-gap", default=1.0, type=float)
    parser.add_argument(
        "--key-kind",
        choices=["all", "replay_frame", "spec_episode_step"],
        default="replay_frame",
        help="Restrict evaluation to a single root prior key kind. Defaults to replay_frame for replay/raw shadow validation.",
    )
    args = parser.parse_args()

    prior_artifact_path = args.prior_artifact or (args.dataset_dir / "q_local_root_prior.jsonl")
    eval_out = args.eval_out or (args.dataset_dir / "q_local_rust_search_eval.json")
    samples_out = args.samples_out or (args.dataset_dir / "q_local_rust_search_eval_samples.jsonl")

    baseline_records = load_json(args.baseline)
    prior_records = load_json(args.prior)
    prior_rows = load_jsonl(prior_artifact_path)

    baseline_by_key = {record_key(row): row for row in baseline_records}
    prior_by_key = {record_key(row): row for row in prior_records}
    teacher_by_key: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in prior_rows:
        teacher_by_key[key_to_string(dict(row.get("root_prior_key") or {}))].append(row)

    groups = sorted(set(baseline_by_key) & set(prior_by_key) & set(teacher_by_key))
    if args.key_kind != "all":
        groups = [key for key in groups if key_kind_from_string(key) == args.key_kind]
    sample_rows: list[dict[str, Any]] = []
    by_bucket: dict[str, dict[str, float]] = defaultdict(
        lambda: {
            "groups": 0,
            "baseline_top1": 0,
            "root_prior_top1": 0,
            "baseline_topk": 0,
            "root_prior_topk": 0,
            "hard_cases": 0,
            "hard_corrected": 0,
        }
    )
    eligible = 0
    baseline_top1 = 0
    prior_top1 = 0
    baseline_topk = 0
    prior_topk = 0
    hard_cases = 0
    corrected_hard = 0
    retained_counts: list[int] = []
    legal_counts: list[int] = []

    for key in groups:
        baseline = baseline_by_key[key]
        prior = prior_by_key[key]
        teacher_rows = teacher_by_key[key]
        teacher_best_rows = [row for row in teacher_rows if bool(row.get("teacher_best"))]
        if not teacher_best_rows:
            continue
        teacher_moves = {str(row.get("candidate_move") or "") for row in teacher_best_rows}
        bucket = str(teacher_rows[0].get("eval_bucket") or "unknown")
        teacher_best_score = max(float(row.get("teacher_score") or 0.0) for row in teacher_rows)
        baseline_move = top_move(baseline)
        prior_move = top_move(prior)
        if not baseline_move or not prior_move:
            continue
        eligible += 1
        by_bucket[bucket]["groups"] += 1
        baseline_correct = baseline_move in teacher_moves
        prior_correct = prior_move in teacher_moves
        if baseline_correct:
            baseline_top1 += 1
            by_bucket[bucket]["baseline_top1"] += 1
        if prior_correct:
            prior_top1 += 1
            by_bucket[bucket]["root_prior_top1"] += 1
        if topk_contains(baseline, teacher_moves, args.topk):
            baseline_topk += 1
            by_bucket[bucket]["baseline_topk"] += 1
        if topk_contains(prior, teacher_moves, args.topk):
            prior_topk += 1
            by_bucket[bucket]["root_prior_topk"] += 1

        baseline_teacher = next(
            (row for row in teacher_rows if str(row.get("candidate_move") or "") == baseline_move),
            None,
        )
        baseline_score = float(baseline_teacher.get("teacher_score") or 0.0) if baseline_teacher else 0.0
        hard_case = (not baseline_correct) and (teacher_best_score - baseline_score) >= float(args.hard_gap)
        if hard_case:
            hard_cases += 1
            by_bucket[bucket]["hard_cases"] += 1
            if prior_correct:
                corrected_hard += 1
                by_bucket[bucket]["hard_corrected"] += 1

        best_aggregate = max(float(row.get("aggregate_score") or 0.0) for row in teacher_rows)
        retained = sum(
            1
            for row in teacher_rows
            if float(row.get("aggregate_score") or 0.0) >= best_aggregate - float(args.retained_threshold)
        )
        retained_counts.append(retained)
        legal_counts.append(int(prior.get("legal_moves") or baseline.get("legal_moves") or len(teacher_rows)))

        sample_rows.append(
            {
                "root_prior_key": key,
                "eval_bucket": bucket,
                "curriculum_tag": teacher_rows[0].get("curriculum_tag"),
                "baseline_move": baseline_move,
                "root_prior_move": prior_move,
                "teacher_best_moves": sorted(teacher_moves),
                "baseline_top1_correct": baseline_correct,
                "root_prior_top1_correct": prior_correct,
                "hard_case": hard_case,
                "legal_candidates": legal_counts[-1],
                "retained_candidates": retained,
                "baseline_top_moves": baseline.get("top_moves") or [],
                "root_prior_top_moves": prior.get("top_moves") or [],
            }
        )

    bucket_summary = {}
    for bucket, stats in by_bucket.items():
        groups_count = max(int(stats["groups"]), 1)
        hard_count = max(int(stats["hard_cases"]), 1)
        bucket_summary[bucket] = {
            "groups": int(stats["groups"]),
            "baseline_root_top1_match": float(stats["baseline_top1"] / groups_count),
            "root_prior_top1_match": float(stats["root_prior_top1"] / groups_count),
            "topk_recall": float(stats["root_prior_topk"] / groups_count),
            "baseline_topk_recall": float(stats["baseline_topk"] / groups_count),
            "hard_mistake_suppression": float(stats["hard_corrected"] / hard_count)
            if int(stats["hard_cases"])
            else 0.0,
        }

    avg_retained = sum(retained_counts) / len(retained_counts) if retained_counts else 0.0
    avg_legal = sum(legal_counts) / len(legal_counts) if legal_counts else 0.0
    summary = {
        "groups_evaluated": eligible,
        "baseline_root_top1_match": float(baseline_top1 / eligible) if eligible else 0.0,
        "root_prior_top1_match": float(prior_top1 / eligible) if eligible else 0.0,
        "topk_recall": float(prior_topk / eligible) if eligible else 0.0,
        "baseline_topk_recall": float(baseline_topk / eligible) if eligible else 0.0,
        "node_reduction_proxy": float(1.0 - (avg_retained / avg_legal)) if avg_legal else 0.0,
        "avg_retained_candidates": avg_retained,
        "avg_legal_candidates": avg_legal,
        "hard_mistake_suppression": float(corrected_hard / hard_cases) if hard_cases else 0.0,
        "hard_case_count": hard_cases,
        "retained_threshold": args.retained_threshold,
        "topk": args.topk,
        "hard_gap": args.hard_gap,
        "key_kind_filter": args.key_kind,
        "by_bucket": bucket_summary,
        "notes": [
            "baseline and root-prior records must come from identical replay/spec states",
            "teacher labels come from q_local_root_prior rows, not from live runtime behavior",
        ],
    }
    write_json(eval_out, summary)
    write_jsonl(samples_out, sample_rows)
    print(json.dumps(summary, indent=2, ensure_ascii=False))
    print(f"wrote Rust root-prior eval to {eval_out}")
    print(f"wrote Rust root-prior eval samples to {samples_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
