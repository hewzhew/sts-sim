#!/usr/bin/env python3
"""
Review combat reranker rows before training.

This stays offline and lightweight. It does not train a model. It surfaces:
- oracle-strong vs oracle-preference buckets
- equivalent-best tie states
- high-value baseline/oracle disagreements
- a recommended first-pass training subset size
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any

from combat_reranker_common import baseline_outcome, iter_jsonl, positive_move_set, write_json, write_jsonl

REPO_ROOT = Path(__file__).resolve().parents[2]


def top_candidate_label(row: dict[str, Any]) -> str | None:
    top_candidates = row.get("top_candidates") or []
    if not top_candidates:
        return None
    first = top_candidates[0] or {}
    label = first.get("move_label")
    if isinstance(label, str) and label:
        return label
    return None


def row_priority(row: dict[str, Any]) -> tuple[float, float, int]:
    top_gap = abs(float(row.get("top_gap") or 0.0))
    downside = abs(float(row.get("sequence_downside_penalty") or 0.0))
    weight = float(row.get("sample_weight") or 0.0)
    reasons = row.get("reasons") or []
    bonus = 0
    if row.get("heuristic_search_gap"):
        bonus += 2
    if "sequencing_conflict" in reasons:
        bonus += 2
    if "branch_opening_conflict" in reasons:
        bonus += 2
    if row.get("snapshot_normalized_state") is not None:
        bonus += 1
    return (top_gap + downside / 1000.0, weight, bonus)


def sample_tags(row: dict[str, Any]) -> list[str]:
    tags: list[str] = []
    baseline_move = str(row.get("baseline_chosen_move") or "")
    best_moves = positive_move_set(row)
    baseline_in_best_bucket = bool(baseline_move and baseline_move in best_moves)
    baseline_die_oracle_live = (
        baseline_outcome(row) == "dies"
        and str(row.get("oracle_outcome_bucket") or "") in {"survives", "lethal_win"}
    )
    if baseline_die_oracle_live:
        tags.append("baseline_dies_oracle_lives")
    if row.get("oracle_disagrees_with_baseline"):
        tags.append("hard_disagreement")
    if int(row.get("oracle_best_bucket_size") or 0) > 1:
        tags.append("equivalent_best_tie")
    baseline_row = row.get("baseline_row") or {}
    reasons = set(str(reason) for reason in (baseline_row.get("reasons") or []))
    if "sequencing_conflict" in reasons and "Flex" in " ".join(best_moves):
        tags.append("setup_flex_missed")
    pressure = int(baseline_row.get("value_incoming") or 0)
    if "Defend" in baseline_move and pressure <= 5 and not baseline_in_best_bucket:
        tags.append("overdefend_light_pressure")
    lowest_hp = 999
    snapshot = baseline_row.get("snapshot_normalized_state") or {}
    for monster in snapshot.get("monsters") or []:
        hp = int(monster.get("current_hp") or 0)
        if hp > 0:
            lowest_hp = min(lowest_hp, hp)
    if lowest_hp <= 12 and not baseline_in_best_bucket and not baseline_move.startswith("Play #"):
        tags.append("possible_kill_now_missed")
    if lowest_hp <= 12 and not baseline_in_best_bucket and baseline_move.startswith("Play #") and "Defend" in baseline_move:
        tags.append("possible_kill_now_missed")
    return tags


def main() -> int:
    parser = argparse.ArgumentParser(description="Review combat reranker rows from a sidecar dataset.")
    parser.add_argument(
        "--dataset-dir",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset",
        type=Path,
        help="Directory that contains combat_rows.jsonl and oracle rows.",
    )
    parser.add_argument(
        "--out",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "combat_reranker_review.json",
        type=Path,
        help="Where to write the review summary JSON.",
    )
    parser.add_argument(
        "--sample-out",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "combat_reranker_samples.jsonl",
        type=Path,
        help="Where to write the highest-value review samples JSONL.",
    )
    parser.add_argument(
        "--sample-limit",
        default=40,
        type=int,
        help="How many high-value rows to keep in the review sample bundle.",
    )
    parser.add_argument(
        "--packed-prefixes",
        default="",
        help="Optional comma-separated packed dataset prefixes to summarize alongside raw/oracle review.",
    )
    args = parser.parse_args()

    combat_rows_path = args.dataset_dir / "combat_rows.jsonl"
    if not combat_rows_path.exists():
        raise SystemExit(f"missing combat rows: {combat_rows_path}")

    rows = [row for _, row in iter_jsonl(combat_rows_path)]
    oracle_rows_path = args.dataset_dir / "oracle_labeled_combat_rows.jsonl"
    oracle_rows = [row for _, row in iter_jsonl(oracle_rows_path)] if oracle_rows_path.exists() else []
    oracle_summary_path = args.dataset_dir / "oracle_labeled_combat_summary.json"
    oracle_summary = {}
    if oracle_summary_path.exists():
        with oracle_summary_path.open("r", encoding="utf-8") as handle:
            oracle_summary = json.load(handle)
    oracle_profile_path = args.dataset_dir / "oracle_labeled_combat_profile.json"
    oracle_profile = {}
    if oracle_profile_path.exists():
        with oracle_profile_path.open("r", encoding="utf-8") as handle:
            oracle_profile = json.load(handle)
    packed_prefixes = [item.strip() for item in str(args.packed_prefixes or "").split(",") if item.strip()]

    reason_counts: Counter[str] = Counter()
    branch_family_counts: Counter[str] = Counter()
    rationale_counts: Counter[str] = Counter()
    top_candidate_rows = 0
    disagreement_top1 = 0
    strong_label_rows = 0
    snapshot_rows = 0
    oracle_hard_disagreements = 0
    oracle_strong_hard_disagreements = 0
    oracle_tie_rows = 0
    oracle_label_strength_counts: Counter[str] = Counter()
    sample_tag_counts: Counter[str] = Counter()
    recommended_training_rows = 0
    recommended_strong_rows = 0
    recommended_preference_rows = 0
    packed_dataset_summaries: dict[str, Any] = {}

    for row in rows:
        for reason in row.get("reasons") or []:
            reason_counts[reason] += 1
        if row.get("branch_family"):
            branch_family_counts[str(row["branch_family"])] += 1
        if row.get("sequencing_rationale_key"):
            rationale_counts[str(row["sequencing_rationale_key"])] += 1
        if row.get("strong_label"):
            strong_label_rows += 1
        if row.get("snapshot_normalized_state") is not None:
            snapshot_rows += 1
        top_label = top_candidate_label(row)
        if top_label is not None:
            top_candidate_rows += 1
            if row.get("chosen_move") != top_label:
                disagreement_top1 += 1

    ranked_rows = sorted(rows, key=row_priority, reverse=True)
    oracle_by_key = {
        f'{row.get("run_id")}::{row.get("frame_count")}': row
        for row in oracle_rows
    }
    review_samples = []
    for row in ranked_rows:
        key = f'{row.get("run_id")}::{row.get("frame_count")}'
        oracle = oracle_by_key.get(key)
        sample = {
            "run_id": row.get("run_id"),
            "frame_count": row.get("frame_count"),
            "chosen_move": row.get("chosen_move"),
            "heuristic_move": row.get("heuristic_move"),
            "search_move": row.get("search_move"),
            "reasons": row.get("reasons") or [],
            "top_gap": row.get("top_gap"),
            "sequence_bonus": row.get("sequence_bonus"),
            "sequence_defer_bonus": row.get("sequence_defer_bonus"),
            "sequence_downside_penalty": row.get("sequence_downside_penalty"),
            "snapshot_trigger_kind": row.get("snapshot_trigger_kind"),
            "snapshot_reasons": row.get("snapshot_reasons") or [],
            "snapshot_normalized_state": row.get("snapshot_normalized_state"),
            "oracle": oracle,
            "sample_tags": sample_tags(oracle) if oracle else [],
        }
        for tag in sample["sample_tags"]:
            sample_tag_counts[tag] += 1
        review_samples.append(sample)
        if len(review_samples) >= args.sample_limit:
            break

    baseline_die_oracle_live = 0
    for row in oracle_rows:
        label_strength = str(row.get("label_strength") or "baseline_weak")
        oracle_label_strength_counts[label_strength] += 1
        bucket_size = int(row.get("oracle_best_bucket_size") or 0)
        if bucket_size > 1:
            oracle_tie_rows += 1
        equivalent = positive_move_set(row)
        baseline = row.get("baseline_chosen_move")
        if baseline and equivalent and baseline not in equivalent:
            oracle_hard_disagreements += 1
            if label_strength == "oracle_strong":
                oracle_strong_hard_disagreements += 1
        if baseline_outcome(row) == "dies" and str(row.get("oracle_outcome_bucket") or "") in {"survives", "lethal_win"}:
            baseline_die_oracle_live += 1
        if label_strength == "oracle_strong":
            recommended_training_rows += 1
            recommended_strong_rows += 1
        elif label_strength == "oracle_preference":
            recommended_training_rows += 1
            recommended_preference_rows += 1

    for prefix in packed_prefixes:
        packed_rows = []
        for split in ("train", "val", "test"):
            packed_path = args.dataset_dir / f"{prefix}_{split}.jsonl"
            if packed_path.exists():
                packed_rows.extend(row for _, row in iter_jsonl(packed_path))
        source_counts: Counter[str] = Counter()
        tag_counts: Counter[str] = Counter()
        label_counts: Counter[str] = Counter()
        for row in packed_rows:
            source_counts[str(row.get("sample_origin") or "unknown")] += 1
            label_counts[str(row.get("label_strength") or "baseline_weak")] += 1
            for tag in row.get("sample_tags") or []:
                tag_counts[str(tag)] += 1
        packed_dataset_summaries[prefix] = {
            "candidate_rows": len(packed_rows),
            "sample_origin_counts": dict(source_counts),
            "label_strength_counts": dict(label_counts),
            "sample_tag_counts": dict(tag_counts),
        }

    review = {
        "dataset_dir": str(args.dataset_dir),
        "combat_rows": len(rows),
        "strong_label_rows": strong_label_rows,
        "snapshot_rows": snapshot_rows,
        "top_candidate_rows": top_candidate_rows,
        "top1_disagreement_rows": disagreement_top1,
        "top1_disagreement_rate": (
            float(disagreement_top1) / float(top_candidate_rows)
            if top_candidate_rows
            else 0.0
        ),
        "reason_counts": reason_counts.most_common(),
        "branch_family_counts": branch_family_counts.most_common(),
        "sequencing_rationale_counts": rationale_counts.most_common(12),
        "oracle_rows": len(oracle_rows),
        "oracle_mode": oracle_summary.get("mode"),
        "oracle_audit_invocation_mode": oracle_summary.get("audit_invocation_mode"),
        "oracle_hard_disagreements": oracle_hard_disagreements,
        "oracle_strong_hard_disagreements": oracle_strong_hard_disagreements,
        "oracle_tie_rows": oracle_tie_rows,
        "oracle_baseline_dies_oracle_lives": baseline_die_oracle_live,
        "oracle_label_strength_counts": dict(oracle_label_strength_counts),
        "sample_tag_counts": dict(sample_tag_counts),
        "recommended_training_subset": {
            "rows": recommended_training_rows,
            "oracle_strong_rows": recommended_strong_rows,
            "oracle_preference_rows": recommended_preference_rows,
            "oracle_strong_hard_disagreement_rows": oracle_strong_hard_disagreements,
            "tie_rows": oracle_tie_rows,
        },
        "oracle_profile_summary": {
            "audit_invocation_mode": oracle_summary.get("audit_invocation_mode"),
            "elapsed_total_seconds": oracle_summary.get("elapsed_total_seconds"),
            "avg_seconds_per_audited_frame": oracle_summary.get("avg_seconds_per_audited_frame"),
            "batch_count": oracle_summary.get("batch_count"),
            "profile_batches": len(oracle_profile.get("batches") or []),
        },
        "packed_dataset_summaries": packed_dataset_summaries,
        "notes": [
            "top1_disagreement_rate only uses serialized top_candidates from live rows",
            "equivalent-best buckets are reviewed separately and not treated as hard disagreement",
            "recommended_training_subset counts rows suitable for first-pass offline reranker training",
            "review samples are sorted by top_gap, downside magnitude, sample_weight, and conflict-rich reasons",
            "packed_dataset_summaries are optional and let review act as an offline trainer control panel",
        ],
    }

    write_json(args.out, review)
    write_jsonl(args.sample_out, review_samples)

    print(json.dumps(review, indent=2, ensure_ascii=False))
    print(f"wrote review summary to {args.out}")
    print(f"wrote review samples to {args.sample_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
