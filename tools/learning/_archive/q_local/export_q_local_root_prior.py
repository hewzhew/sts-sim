#!/usr/bin/env python3
from __future__ import annotations

import argparse
from collections import Counter
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT
from combat_reranker_common import iter_jsonl, write_json, write_jsonl

HEAD_FIELDS = [
    "survival_score",
    "tempo_score",
    "setup_payoff_score",
    "kill_window_score",
    "risk_score",
    "mean_return",
]


def load_rows(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [row for _, row in iter_jsonl(path)]


def build_root_prior_key(row: dict[str, Any]) -> dict[str, Any] | None:
    spec_name = row.get("spec_name")
    episode_id = row.get("episode_id")
    step_index = row.get("step_index")
    if spec_name is not None and episode_id is not None and step_index is not None:
        return {
            "kind": "spec_episode_step",
            "spec_name": str(spec_name),
            "episode_id": int(episode_id),
            "step_index": int(step_index),
        }
    source_path = row.get("source_path")
    frame = row.get("before_frame_id") or row.get("frame") or row.get("state_frame_id")
    if source_path is not None and frame is not None:
        return {
            "kind": "replay_frame",
            "source_path": str(source_path),
            "frame": int(frame),
        }
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description="Export Q_local predictions into a Rust lookup artifact.")
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--predictions", default=None, type=Path)
    parser.add_argument("--rows-glob", default=None, help="Comma-separated JSONL paths or globs. Defaults to combat_q_local train/val/test.")
    parser.add_argument("--out", default=None, type=Path)
    parser.add_argument("--summary-out", default=None, type=Path)
    parser.add_argument("--include-uncertain", action="store_true")
    parser.add_argument(
        "--strict-certain-only",
        action="store_true",
        help="Skip uncertain replay_frame rows too. By default uncertain replay rows are retained for shadow coverage.",
    )
    args = parser.parse_args()

    predictions_path = args.predictions or (args.dataset_dir / "q_local_predictions.jsonl")
    out_path = args.out or (args.dataset_dir / "q_local_root_prior.jsonl")
    summary_path = args.summary_out or (args.dataset_dir / "q_local_root_prior_summary.json")
    if args.rows_glob:
        row_paths = []
        for token in args.rows_glob.split(","):
            path = Path(token.strip())
            if path.exists():
                row_paths.append(path)
            else:
                row_paths.extend(sorted(path.parent.glob(path.name)))
    else:
        row_paths = [
            args.dataset_dir / "combat_q_local_train.jsonl",
            args.dataset_dir / "combat_q_local_val.jsonl",
            args.dataset_dir / "combat_q_local_test.jsonl",
        ]

    prediction_rows = load_rows(predictions_path)
    candidate_rows: list[dict[str, Any]] = []
    for path in row_paths:
        candidate_rows.extend(load_rows(path))

    prediction_by_key = {
        (str(row.get("group_id") or ""), str(row.get("candidate_move") or "")): row
        for row in prediction_rows
    }
    group_best_teacher_score: dict[str, float] = {}
    for row in candidate_rows:
        group_id = str(row.get("group_id") or "")
        score = float(row.get("q_local_teacher_score") or 0.0)
        current = group_best_teacher_score.get(group_id)
        if current is None or score > current:
            group_best_teacher_score[group_id] = score
    exported_rows: list[dict[str, Any]] = []
    source_counts: Counter[str] = Counter()
    key_kind_counts: Counter[str] = Counter()
    skipped_uncertain = 0
    included_uncertain_replay = 0
    skipped_missing_prediction = 0
    skipped_missing_key = 0

    for row in candidate_rows:
        prior_key = build_root_prior_key(row)
        if prior_key is None:
            skipped_missing_key += 1
            continue
        key_kind = str(prior_key.get("kind") or "unknown")
        is_uncertain = bool(row.get("uncertain"))
        if is_uncertain and not args.include_uncertain:
            if args.strict_certain_only or key_kind != "replay_frame":
                skipped_uncertain += 1
                continue
            included_uncertain_replay += 1
        lookup_key = (str(row.get("group_id") or ""), str(row.get("candidate_move") or ""))
        prediction = prediction_by_key.get(lookup_key)
        if prediction is None:
            skipped_missing_prediction += 1
            continue
        group_id = str(row.get("group_id") or "")
        teacher_score = float(row.get("q_local_teacher_score") or 0.0)
        best_teacher_score = group_best_teacher_score.get(group_id, teacher_score)
        exported = {
            "root_prior_key": prior_key,
            "sample_origin": str(row.get("sample_origin") or "unknown"),
            "group_id": group_id,
            "curriculum_tag": row.get("curriculum_tag"),
            "eval_bucket": row.get("eval_bucket"),
            "candidate_move": str(row.get("candidate_move") or ""),
            "aggregate_score": float(prediction.get("pred::aggregate") or 0.0),
            "uncertain": bool(row.get("uncertain")),
            "teacher_best": teacher_score >= (best_teacher_score - 1e-6),
            "teacher_score": teacher_score,
            "candidate_rank": row.get("candidate_rank"),
            "candidate_score_hint": row.get("candidate_score_hint"),
        }
        for head in HEAD_FIELDS:
            exported[head] = float(prediction.get(f"pred::{head}") or 0.0)
        exported_rows.append(exported)
        source_counts[exported["sample_origin"]] += 1
        key_kind_counts[key_kind] += 1

    exported_rows.sort(
        key=lambda row: (
            str(row["root_prior_key"].get("kind") or ""),
            str(row["root_prior_key"].get("spec_name") or row["root_prior_key"].get("source_path") or ""),
            int(row["root_prior_key"].get("episode_id") or 0),
            int(row["root_prior_key"].get("step_index") or row["root_prior_key"].get("frame") or 0),
            str(row["candidate_move"]),
        )
    )
    summary = {
        "prediction_rows": len(prediction_rows),
        "candidate_rows": len(candidate_rows),
        "exported_rows": len(exported_rows),
        "source_counts": dict(source_counts),
        "key_kind_counts": dict(key_kind_counts),
        "skipped_uncertain": skipped_uncertain,
        "included_uncertain_replay": included_uncertain_replay,
        "skipped_missing_prediction": skipped_missing_prediction,
        "skipped_missing_key": skipped_missing_key,
        "include_uncertain": bool(args.include_uncertain),
        "strict_certain_only": bool(args.strict_certain_only),
        "notes": [
            "root prior artifact is a static lookup for Rust offline search shadow",
            "candidate_move labels must exactly match Rust describe_client_input formatting",
            "uncertain replay_frame rows are retained by default to maximize replay/raw shadow coverage",
        ],
    }
    write_jsonl(out_path, exported_rows)
    write_json(summary_path, summary)
    print(f"exported {len(exported_rows)} root prior rows to {out_path}")
    print(f"wrote root prior summary to {summary_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
