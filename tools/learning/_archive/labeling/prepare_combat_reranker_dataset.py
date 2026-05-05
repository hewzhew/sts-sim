#!/usr/bin/env python3
from __future__ import annotations

import argparse
from collections import Counter
from pathlib import Path
from typing import Any

from combat_reranker_common import (
    baseline_in_best_bucket,
    baseline_outcome,
    curriculum_tag_from_spec_name,
    iter_jsonl,
    oracle_label_strength,
    parse_move_label,
    positive_move_set,
    sample_tags_from_oracle_row,
    training_weight,
    write_json,
    write_jsonl,
)

REPO_ROOT = Path(__file__).resolve().parents[2]


def oracle_row_key(row: dict[str, Any]) -> str:
    return f'{row.get("run_id")}::{row.get("frame_count")}'


def candidate_rows_from_oracle(row: dict[str, Any], split: str) -> list[dict[str, Any]]:
    baseline_row = row.get("baseline_row") or {}
    oracle_candidates = row.get("oracle_top_candidates") or []
    baseline_candidates = baseline_row.get("top_candidates") or []
    search_candidates_by_move = {
        str(candidate.get("move_label")): candidate
        for candidate in baseline_candidates
        if candidate.get("move_label")
    }
    best_moves = positive_move_set(row)
    group_id = oracle_row_key(row)
    frame_weight = training_weight(row)
    baseline_move = row.get("baseline_chosen_move")
    baseline_in_best_bucket = bool(baseline_move and baseline_move in best_moves)
    baseline_result = baseline_outcome(row)
    sample_origin = str(row.get("sample_origin") or "archived_clean_run")
    teacher_source = str(row.get("teacher_source") or row.get("label_source") or "offline_decision_audit_search")
    curriculum_tag = row.get("curriculum_tag")
    if not curriculum_tag:
        spec_name = row.get("spec_name") or baseline_row.get("spec_name")
        if spec_name:
            curriculum_tag = curriculum_tag_from_spec_name(spec_name)
    sample_tags = row.get("sample_tags") or sample_tags_from_oracle_row(row)
    raw_rows: list[dict[str, Any]] = []
    for index, oracle_candidate in enumerate(oracle_candidates):
        move = str(oracle_candidate.get("move_label") or "")
        parsed = parse_move_label(move)
        search_candidate = search_candidates_by_move.get(move) or {}
        candidate_row = {
            "dataset_kind": "combat_reranker_candidate",
            "split": split,
            "group_id": group_id,
            "run_id": row.get("run_id"),
            "frame_count": row.get("frame_count"),
            "response_id": row.get("response_id"),
            "state_frame_id": row.get("state_frame_id"),
            "state_source": row.get("state_source", "validated_livecomm_audit"),
            "label_source": row.get("label_source", "offline_decision_audit_search"),
            "label_strength": row.get("label_strength", "baseline_weak"),
            "sample_origin": sample_origin,
            "teacher_source": teacher_source,
            "curriculum_tag": curriculum_tag,
            "sample_tags": sample_tags,
            "training_eligible": frame_weight > 0.0,
            "sample_weight": frame_weight,
            "candidate_index": index,
            "candidate_count": len(oracle_candidates),
            "candidate_source": "search_top_candidates" if search_candidate else "oracle_top_candidates_fallback",
            "candidate_move": move,
            "candidate_move_family": parsed["move_family"],
            "candidate_card_name": parsed["card_name"],
            "candidate_slot_index": parsed["slot_index"],
            "candidate_has_target": parsed["has_target"],
            "candidate_target_index": parsed["target_index"],
            "candidate_is_positive": move in best_moves,
            "candidate_is_equivalent_best": move in best_moves,
            "oracle_equivalent_best_moves": sorted(best_moves),
            "oracle_best_bucket_size": int(row.get("oracle_best_bucket_size") or 0),
            "oracle_margin": row.get("oracle_margin"),
            "oracle_outcome_bucket": row.get("oracle_outcome_bucket"),
            "oracle_disagrees_with_baseline": bool(row.get("oracle_disagrees_with_baseline")),
            "oracle_best_move": row.get("oracle_best_move"),
            "baseline_chosen_move": baseline_move,
            "baseline_in_best_bucket": baseline_in_best_bucket,
            "baseline_outcome": baseline_result,
            "heuristic_move": row.get("heuristic_move"),
            "search_move": row.get("search_move"),
            "top_gap": baseline_row.get("top_gap"),
            "sequence_bonus": baseline_row.get("sequence_bonus"),
            "sequence_frontload_bonus": baseline_row.get("sequence_frontload_bonus"),
            "sequence_defer_bonus": baseline_row.get("sequence_defer_bonus"),
            "sequence_branch_bonus": baseline_row.get("sequence_branch_bonus"),
            "sequence_downside_penalty": baseline_row.get("sequence_downside_penalty"),
            "branch_family": baseline_row.get("branch_family"),
            "sequencing_rationale_key": baseline_row.get("sequencing_rationale_key"),
            "branch_rationale_key": baseline_row.get("branch_rationale_key"),
            "downside_rationale_key": baseline_row.get("downside_rationale_key"),
            "heuristic_search_gap": bool(baseline_row.get("heuristic_search_gap")),
            "tight_root_gap": bool(baseline_row.get("tight_root_gap")),
            "large_sequence_bonus": bool(baseline_row.get("large_sequence_bonus")),
            "reasons": baseline_row.get("reasons") or [],
            "snapshot_id": baseline_row.get("snapshot_id"),
            "snapshot_trigger_kind": baseline_row.get("snapshot_trigger_kind"),
            "snapshot_reasons": baseline_row.get("snapshot_reasons") or [],
            "snapshot_normalized_state": baseline_row.get("snapshot_normalized_state"),
            "snapshot_decision_context": baseline_row.get("snapshot_decision_context"),
            "hidden_intent_active": baseline_row.get("hidden_intent_active"),
            "visible_incoming": baseline_row.get("visible_incoming"),
            "visible_unblocked": baseline_row.get("visible_unblocked"),
            "belief_expected_incoming": baseline_row.get("belief_expected_incoming"),
            "belief_expected_unblocked": baseline_row.get("belief_expected_unblocked"),
            "belief_max_incoming": baseline_row.get("belief_max_incoming"),
            "belief_max_unblocked": baseline_row.get("belief_max_unblocked"),
            "value_incoming": baseline_row.get("value_incoming"),
            "value_unblocked": baseline_row.get("value_unblocked"),
            "survival_guard_incoming": baseline_row.get("survival_guard_incoming"),
            "survival_guard_unblocked": baseline_row.get("survival_guard_unblocked"),
            "belief_attack_probability": baseline_row.get("belief_attack_probability"),
            "belief_lethal_probability": baseline_row.get("belief_lethal_probability"),
            "belief_urgent_probability": baseline_row.get("belief_urgent_probability"),
            "candidate_search_rank": search_candidate.get("search_rank", index) if search_candidate else None,
            "candidate_search_avg_score": search_candidate.get("avg_score"),
            "candidate_search_order_score": search_candidate.get("order_score"),
            "candidate_search_leaf_score": search_candidate.get("leaf_score"),
            "candidate_search_sequence_bonus": search_candidate.get("sequence_bonus"),
            "candidate_search_sequence_frontload_bonus": search_candidate.get("sequence_frontload_bonus"),
            "candidate_search_sequence_defer_bonus": search_candidate.get("sequence_defer_bonus"),
            "candidate_search_sequence_branch_bonus": search_candidate.get("sequence_branch_bonus"),
            "candidate_search_sequence_downside_penalty": search_candidate.get("sequence_downside_penalty"),
            "candidate_projected_unblocked": search_candidate.get("projected_unblocked"),
            "candidate_projected_enemy_total": search_candidate.get("projected_enemy_total"),
            "candidate_survives": search_candidate.get("survives"),
            "candidate_branch_family": search_candidate.get("branch_family"),
        }
        raw_rows.append(candidate_row)
    return raw_rows


def parse_csv_set(text: str | None) -> set[str]:
    return {
        item.strip()
        for item in str(text or "").split(",")
        if item.strip()
    }


def row_selected(
    row: dict[str, Any],
    allowed_label_strengths: set[str],
    only_oracle_disagreements: bool,
) -> bool:
    if allowed_label_strengths and oracle_label_strength(row) not in allowed_label_strengths:
        return False
    if only_oracle_disagreements and not row.get("oracle_disagrees_with_baseline", is_oracle_disagreement_fallback(row)):
        return False
    return True


def is_oracle_disagreement_fallback(row: dict[str, Any]) -> bool:
    return not baseline_in_best_bucket(row)


def main() -> int:
    parser = argparse.ArgumentParser(description="Pack oracle-labeled combat rows into train/val/test reranker datasets.")
    parser.add_argument(
        "--dataset-dir",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset",
        type=Path,
        help="Directory that contains combat_rows.jsonl and oracle_labeled_combat_rows.jsonl.",
    )
    parser.add_argument(
        "--out-dir",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset",
        type=Path,
        help="Directory where packed reranker datasets will be written.",
    )
    parser.add_argument(
        "--dataset-prefix",
        default="combat_reranker",
        help="Output filename prefix for packed train/val/test files.",
    )
    parser.add_argument(
        "--only-label-strengths",
        default="oracle_strong,oracle_preference,baseline_weak",
        help="Comma-separated oracle label_strength values to keep.",
    )
    parser.add_argument(
        "--only-oracle-disagreements",
        action="store_true",
        help="Keep only rows where oracle equivalent-best bucket excludes baseline chosen move.",
    )
    parser.add_argument(
        "--sample-origins",
        default="",
        help="Optional comma-separated sample_origin values to keep.",
    )
    args = parser.parse_args()

    oracle_rows_path = args.dataset_dir / "oracle_labeled_combat_rows.jsonl"
    if not oracle_rows_path.exists():
        raise SystemExit(f"missing oracle rows: {oracle_rows_path}")

    allowed_label_strengths = parse_csv_set(args.only_label_strengths)
    allowed_sample_origins = parse_csv_set(args.sample_origins)
    source_oracle_rows = [row for _, row in iter_jsonl(oracle_rows_path)]
    oracle_rows = [
        row
        for row in source_oracle_rows
        if row_selected(
            row,
            allowed_label_strengths=allowed_label_strengths,
            only_oracle_disagreements=bool(args.only_oracle_disagreements),
        )
        and (
            not allowed_sample_origins
            or str(row.get("sample_origin") or "archived_clean_run") in allowed_sample_origins
        )
    ]
    group_ids = sorted(oracle_row_key(row) for row in oracle_rows)
    split_for_group: dict[str, str] = {}
    group_count = len(group_ids)
    val_group_count = 1 if group_count >= 3 else 0
    test_group_count = 1 if group_count >= 4 else 0
    train_group_count = max(group_count - val_group_count - test_group_count, 0)
    for index, group_id in enumerate(group_ids):
        if index < train_group_count:
            split_for_group[group_id] = "train"
        elif index < train_group_count + val_group_count:
            split_for_group[group_id] = "val"
        else:
            split_for_group[group_id] = "test"

    packed_rows: list[dict[str, Any]] = []
    split_counts: Counter[str] = Counter()
    label_counts: Counter[str] = Counter()
    candidate_source_counts: Counter[str] = Counter()
    group_sizes: Counter[str] = Counter()

    for row in oracle_rows:
        candidates = candidate_rows_from_oracle(row, split_for_group[oracle_row_key(row)])
        if not candidates:
            continue
        packed_rows.extend(candidates)
        key = oracle_row_key(row)
        group_sizes[key] = len(candidates)
        split_counts[candidates[0]["split"]] += len(candidates)
        label_counts[str(candidates[0]["label_strength"])] += 1
        candidate_source_counts[candidates[0]["candidate_source"]] += len(candidates)

    train_rows = [row for row in packed_rows if row["split"] == "train"]
    val_rows = [row for row in packed_rows if row["split"] == "val"]
    test_rows = [row for row in packed_rows if row["split"] == "test"]

    prefix = args.dataset_prefix
    write_jsonl(args.out_dir / f"{prefix}_train.jsonl", train_rows)
    write_jsonl(args.out_dir / f"{prefix}_val.jsonl", val_rows)
    write_jsonl(args.out_dir / f"{prefix}_test.jsonl", test_rows)

    summary = {
        "source_dataset_dir": str(args.dataset_dir),
        "out_dir": str(args.out_dir),
        "dataset_prefix": prefix,
        "source_oracle_rows": len(source_oracle_rows),
        "selected_oracle_rows": len(oracle_rows),
        "filter_config": {
            "only_label_strengths": sorted(allowed_label_strengths),
            "only_oracle_disagreements": bool(args.only_oracle_disagreements),
            "sample_origins": sorted(allowed_sample_origins),
        },
        "frame_count": len(group_sizes),
        "candidate_rows": len(packed_rows),
        "split_row_counts": dict(split_counts),
        "label_strength_frame_counts": dict(label_counts),
        "candidate_source_counts": dict(candidate_source_counts),
        "training_eligible_rows": sum(1 for row in packed_rows if row["training_eligible"]),
        "positive_rows": sum(1 for row in packed_rows if row["candidate_is_positive"]),
        "negative_rows": sum(1 for row in packed_rows if not row["candidate_is_positive"]),
        "notes": [
            "rows are candidate-level training examples grouped by run_id::frame_count",
            "oracle_strong rows get full weight, oracle_preference rows are down-weighted",
            "baseline_weak rows remain in packed outputs for analysis but are not training-eligible",
            "oracle_top_candidates are the candidate universe when live top_candidates are still missing",
            "use --only-label-strengths oracle_strong --only-oracle-disagreements for the first-pass strong-only set",
        ],
    }
    write_json(args.out_dir / f"{prefix}_dataset_summary.json", summary)

    import json
    print(json.dumps(summary, indent=2, ensure_ascii=False))
    print(f"wrote packed train/val/test reranker datasets to {args.out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
