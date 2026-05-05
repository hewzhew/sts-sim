#!/usr/bin/env python3
from __future__ import annotations

import argparse
from collections import Counter
from pathlib import Path
from typing import Any

from combat_reranker_common import parse_move_label, stable_split, write_json, write_jsonl
from curriculum_dynamic_teacher import TEACHER_SOURCE, dynamic_teacher_for_row

REPO_ROOT = Path(__file__).resolve().parents[2]


def iter_rows(path: Path):
    import json

    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            text = line.strip()
            if text:
                yield json.loads(text)


def load_rows(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return list(iter_rows(path))


def sample_weight(label_strength: str) -> float:
    if label_strength == "oracle_strong":
        return 0.5
    if label_strength == "oracle_preference":
        return 0.25
    return 0.0


def curriculum_sample_weight(label_strength: str, tags: set[str], margin: float) -> float:
    weight = sample_weight(label_strength)
    if margin >= 0.35:
        weight = max(weight, 0.5)
    elif margin >= 0.20:
        weight = max(weight, 0.4)
    elif margin >= 0.12:
        weight = max(weight, 0.3)
    if "dynamic_semantic_disagreement" in tags:
        weight = max(weight, 0.35)
    if "oracle_save" in tags or "kill_now_missed" in tags:
        weight = max(weight, 0.45)
    return round(min(weight, 0.65), 4)


def build_candidate_rows(row: dict[str, Any]) -> list[dict[str, Any]]:
    candidates = list(row.get("normalized_candidates") or [])
    if len(candidates) < 2:
        return []

    dynamic = dynamic_teacher_for_row(row)
    preferred_moves = list(row.get("dynamic_teacher_preferred_moves") or dynamic.get("preferred_moves") or [])
    teacher_source = str(row.get("dynamic_teacher_source") or dynamic.get("teacher_source") or TEACHER_SOURCE)
    label_strength = str(row.get("dynamic_teacher_label_strength") or dynamic.get("label_strength") or "oracle_preference")
    chosen_move = str(row.get("chosen_move") or "")
    preferred_set = {move for move in preferred_moves if move}
    if not preferred_set or chosen_move in preferred_set:
        return []
    if not bool(row.get("dynamic_teacher_active") or dynamic.get("active")):
        return []

    score_by_move = {str(candidate.get("move_label") or ""): float(candidate.get("score") or 0.0) for candidate in candidates}
    preferred_score = max((score_by_move.get(move, float("-inf")) for move in preferred_set), default=float("-inf"))
    chosen_score = score_by_move.get(chosen_move, float("-inf"))
    margin = float(row.get("dynamic_teacher_margin") or dynamic.get("oracle_margin") or 0.0)
    tags = set(str(tag) for tag in (row.get("sample_tags") or []))
    row_weight = curriculum_sample_weight(label_strength, tags, margin)
    detail_by_move = {
        str(detail.get("move_label") or ""): detail
        for detail in (dynamic.get("candidate_details") or [])
    }

    split = stable_split(str(row.get("sample_id") or row.get("spec_name") or "curriculum"))
    snapshot = row.get("snapshot_normalized_state") or {}
    preview = row.get("state_features_preview") or {}
    full = row.get("state_features_full") or {}
    incoming = int(preview.get("incoming_damage") or 0)
    unblocked = int(full.get("unblocked_incoming") or max(incoming - int(preview.get("player_block") or 0), 0))
    oracle_outcome_bucket = "survives" if str(row.get("outcome") or "").lower() == "victory" else str(row.get("outcome") or "").lower()
    result: list[dict[str, Any]] = []
    for index, candidate in enumerate(candidates):
        move = str(candidate.get("move_label") or "")
        parsed = parse_move_label(move)
        detail = detail_by_move.get(move) or {}
        result.append(
            {
                "dataset_kind": "combat_reranker_candidate",
                "split": split,
                "group_id": f"combat_lab_spec::{row.get('sample_id')}",
                "run_id": None,
                "frame_count": None,
                "response_id": None,
                "state_frame_id": None,
                "state_source": str(row.get("state_source") or "combat_lab_trace"),
                "label_source": teacher_source,
                "label_strength": label_strength,
                "sample_origin": "combat_lab_spec",
                "teacher_source": teacher_source,
                "curriculum_tag": row.get("curriculum_tag"),
                "sample_tags": row.get("sample_tags") or [],
                "training_eligible": True,
                "sample_weight": row_weight,
                "candidate_index": index,
                "candidate_count": len(candidates),
                "candidate_source": "combat_lab_curriculum_candidates",
                "candidate_move": move,
                "candidate_move_family": parsed["move_family"],
                "candidate_card_name": parsed["card_name"],
                "candidate_slot_index": parsed["slot_index"],
                "candidate_has_target": parsed["has_target"],
                "candidate_target_index": parsed["target_index"],
                "candidate_is_positive": move in preferred_set,
                "candidate_is_equivalent_best": move in preferred_set,
                "candidate_is_teacher_top": move in preferred_set,
                "oracle_equivalent_best_moves": sorted(preferred_set),
                "oracle_best_bucket_size": len(preferred_set),
                "oracle_margin": round(margin, 3),
                "oracle_outcome_bucket": oracle_outcome_bucket,
                "oracle_disagrees_with_baseline": True,
                "oracle_best_move": sorted(preferred_set)[0],
                "baseline_chosen_move": chosen_move,
                "baseline_in_best_bucket": False,
                "baseline_outcome": oracle_outcome_bucket,
                "heuristic_move": None,
                "search_move": row.get("top_candidate_move"),
                "top_gap": float(
                    max(float(candidate.get("score") or 0.0) for candidate in candidates)
                    - min(float(candidate.get("score") or 0.0) for candidate in candidates)
                ),
                "sequence_bonus": 0.0,
                "sequence_frontload_bonus": 0.0,
                "sequence_defer_bonus": 0.0,
                "sequence_branch_bonus": 0.0,
                "sequence_downside_penalty": 0.0,
                "branch_family": None,
                "sequencing_rationale_key": row.get("curriculum_tag"),
                "branch_rationale_key": None,
                "downside_rationale_key": None,
                "heuristic_search_gap": chosen_move != row.get("top_candidate_move"),
                "tight_root_gap": False,
                "large_sequence_bonus": False,
                "reasons": list(row.get("sample_tags") or []),
                "snapshot_id": row.get("sample_id"),
                "snapshot_trigger_kind": "combat_lab_curriculum",
                "snapshot_reasons": row.get("sample_tags") or [],
                "snapshot_normalized_state": snapshot,
                "snapshot_decision_context": {
                    "spec_name": row.get("spec_name"),
                    "episode_id": row.get("episode_id"),
                    "turn_index": row.get("turn_index"),
                    "step_index": row.get("step_index"),
                    "chosen_move": chosen_move,
                    "top_candidate_move": row.get("top_candidate_move"),
                },
                "hidden_intent_active": False,
                "visible_incoming": incoming,
                "visible_unblocked": unblocked,
                "belief_expected_incoming": float(incoming),
                "belief_expected_unblocked": float(unblocked),
                "belief_max_incoming": incoming,
                "belief_max_unblocked": unblocked,
                "value_incoming": incoming,
                "value_unblocked": unblocked,
                "survival_guard_incoming": incoming,
                "survival_guard_unblocked": unblocked,
                "belief_attack_probability": 1.0 if incoming > 0 else 0.0,
                "belief_lethal_probability": 1.0 if int(preview.get("player_hp") or 0) <= unblocked else 0.0,
                "belief_urgent_probability": 1.0 if unblocked >= max(int(preview.get("player_hp") or 0) // 4, 8) else 0.0,
                "candidate_search_rank": int(candidate.get("search_rank") or index),
                "candidate_search_avg_score": float(candidate.get("avg_score") or candidate.get("score") or 0.0),
                "candidate_search_order_score": float(candidate.get("score") or 0.0),
                "candidate_search_leaf_score": float(candidate.get("score") or 0.0),
                "candidate_search_sequence_bonus": 0.0,
                "candidate_search_sequence_frontload_bonus": 0.0,
                "candidate_search_sequence_defer_bonus": 0.0,
                "candidate_search_sequence_branch_bonus": 0.0,
                "candidate_search_sequence_downside_penalty": 0.0,
                "candidate_projected_unblocked": 0.0,
                "candidate_projected_enemy_total": float(preview.get("remaining_monster_hp") or 0.0),
                "candidate_survives": True,
                "candidate_branch_family": None,
                "candidate_semantics": detail.get("candidate_semantics"),
                "chance_features": detail.get("chance_features"),
                "curriculum_teacher_targets": detail.get("teacher_targets"),
                "curriculum_teacher_score": detail.get("teacher_score"),
            }
        )
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description="Pack combat_lab curriculum rows into a weak offline reranker dataset.")
    parser.add_argument(
        "--dataset-dir",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset",
        type=Path,
        help="Directory containing combat_lab_curriculum_rows.jsonl.",
    )
    parser.add_argument(
        "--dataset-prefix",
        default="combat_lab_curriculum_reranker",
        help="Output prefix for curriculum reranker train/val/test files.",
    )
    parser.add_argument(
        "--append-dataset-prefix",
        default="",
        help="Optional existing packed dataset prefix to append and emit a mixed dataset.",
    )
    parser.add_argument(
        "--mixed-dataset-prefix",
        default="offline_teacher_plus_curriculum_reranker",
        help="Output prefix used when --append-dataset-prefix is provided.",
    )
    args = parser.parse_args()

    source_path = args.dataset_dir / "combat_lab_curriculum_rows.jsonl"
    if not source_path.exists():
        raise SystemExit(f"missing curriculum rows: {source_path}")

    source_rows = load_rows(source_path)
    packed_rows: list[dict[str, Any]] = []
    source_count = len(source_rows)
    selected_samples = 0
    tag_counts: Counter[str] = Counter()
    teacher_source_counts: Counter[str] = Counter()
    for row in source_rows:
        candidate_rows = build_candidate_rows(row)
        if not candidate_rows:
            continue
        packed_rows.extend(candidate_rows)
        selected_samples += 1
        teacher_source_counts[str(candidate_rows[0].get("teacher_source") or "unknown")] += 1
        for tag in row.get("sample_tags") or []:
            tag_counts[str(tag)] += 1

    split_rows = {
        "train": [row for row in packed_rows if row["split"] == "train"],
        "val": [row for row in packed_rows if row["split"] == "val"],
        "test": [row for row in packed_rows if row["split"] == "test"],
    }
    for split, values in split_rows.items():
        write_jsonl(args.dataset_dir / f"{args.dataset_prefix}_{split}.jsonl", values)

    summary = {
        "dataset_prefix": args.dataset_prefix,
        "source_rows": source_count,
        "selected_frame_count": selected_samples,
        "candidate_rows": len(packed_rows),
        "split_row_counts": {split: len(values) for split, values in split_rows.items()},
        "teacher_source_counts": dict(teacher_source_counts),
        "sample_tag_counts": dict(tag_counts),
        "notes": [
            "combat_lab curriculum dataset is a weak offline teacher derived from local tactical fixtures",
            "only rows with multi-candidate choice and dynamic semantic disagreement against baseline are packed",
            "these rows are intended to complement offline_teacher, not replace stronger archived oracle labels",
        ],
    }
    write_json(args.dataset_dir / f"{args.dataset_prefix}_dataset_summary.json", summary)

    if args.append_dataset_prefix:
        mixed_counts = {}
        for split, values in split_rows.items():
            appended = load_rows(args.dataset_dir / f"{args.append_dataset_prefix}_{split}.jsonl")
            mixed = appended + values
            write_jsonl(args.dataset_dir / f"{args.mixed_dataset_prefix}_{split}.jsonl", mixed)
            mixed_counts[split] = len(mixed)
        write_json(
            args.dataset_dir / f"{args.mixed_dataset_prefix}_dataset_summary.json",
            {
                "dataset_prefix": args.mixed_dataset_prefix,
                "components": [args.append_dataset_prefix, args.dataset_prefix],
                "split_row_counts": mixed_counts,
                "notes": [
                    "mixed dataset concatenates existing packed offline teacher rows with combat_lab curriculum weak teacher rows"
                ],
            },
        )

    import json

    print(json.dumps(summary, indent=2, ensure_ascii=False))
    print(f"wrote combat_lab curriculum reranker dataset to {args.dataset_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
