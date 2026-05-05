#!/usr/bin/env python3
from __future__ import annotations

import argparse
from collections import Counter
import glob
from pathlib import Path
from typing import Any

from combat_reranker_common import (
    action_debug_to_move_label,
    curriculum_tag_from_spec_name,
    iter_jsonl,
    normalize_outcome,
    parse_move_label,
    preference_label_strength,
    preference_state_to_snapshot,
    sample_tags_from_preference_sample,
    stable_split,
    write_json,
    write_jsonl,
)

REPO_ROOT = Path(__file__).resolve().parents[2]


def sample_weight_for_label(label_strength: str) -> float:
    if label_strength == "oracle_strong":
        return 1.0
    if label_strength == "oracle_preference":
        return 0.4
    return 0.0


def build_candidate_row(
    sample: dict[str, Any],
    split: str,
    candidate_move: str,
    candidate_index: int,
    candidate_count: int,
    is_positive: bool,
    preferred_move: str,
    chosen_move: str,
    label_strength: str,
    state_snapshot: dict[str, Any],
    sample_tags: list[str],
) -> dict[str, Any]:
    parsed = parse_move_label(candidate_move)
    state = sample.get("state") or {}
    incoming = int(state.get("incoming") or 0)
    player_block = int(state.get("player_block") or 0)
    unblocked = max(incoming - player_block, 0)
    preference_kind = str(sample.get("preference_kind") or "policy_seed")
    sample_id = str(sample.get("sample_id") or f"policy-seed-{candidate_index}")
    return {
        "dataset_kind": "combat_reranker_candidate",
        "split": split,
        "group_id": f"policy_seed::{sample_id}",
        "run_id": None,
        "frame_count": None,
        "response_id": None,
        "state_frame_id": None,
        "state_source": str(sample.get("state_source") or "reconstructed_live_replay_state"),
        "label_source": str(sample.get("preferred_source") or "offline_audit_search"),
        "label_strength": label_strength,
        "sample_origin": "policy_seed_set",
        "teacher_source": str(sample.get("preferred_source") or "offline_audit_search"),
        "curriculum_tag": preference_kind or curriculum_tag_from_spec_name(sample.get("spec_name")),
        "sample_tags": sample_tags,
        "training_eligible": label_strength in {"oracle_strong", "oracle_preference"},
        "sample_weight": sample_weight_for_label(label_strength),
        "candidate_index": candidate_index,
        "candidate_count": candidate_count,
        "candidate_source": "policy_seed_preference_pair",
        "candidate_move": candidate_move,
        "candidate_move_family": parsed["move_family"],
        "candidate_card_name": parsed["card_name"],
        "candidate_slot_index": parsed["slot_index"],
        "candidate_has_target": parsed["has_target"],
        "candidate_target_index": parsed["target_index"],
        "candidate_is_positive": is_positive,
        "candidate_is_equivalent_best": is_positive,
        "oracle_equivalent_best_moves": [preferred_move],
        "oracle_best_bucket_size": 1,
        "oracle_margin": int(sample.get("score_gap") or 0),
        "oracle_outcome_bucket": normalize_outcome(sample.get("preferred_outcome")),
        "oracle_disagrees_with_baseline": chosen_move != preferred_move,
        "oracle_best_move": preferred_move,
        "baseline_chosen_move": chosen_move,
        "baseline_in_best_bucket": chosen_move == preferred_move,
        "baseline_outcome": normalize_outcome(sample.get("chosen_outcome")),
        "heuristic_move": None,
        "search_move": None,
        "top_gap": float(sample.get("score_gap") or 0.0),
        "sequence_bonus": 0.0,
        "sequence_frontload_bonus": 0.0,
        "sequence_defer_bonus": 0.0,
        "sequence_branch_bonus": 0.0,
        "sequence_downside_penalty": 0.0,
        "branch_family": None,
        "sequencing_rationale_key": str(sample.get("preference_kind") or "policy_seed"),
        "branch_rationale_key": None,
        "downside_rationale_key": None,
        "heuristic_search_gap": chosen_move != preferred_move,
        "tight_root_gap": False,
        "large_sequence_bonus": False,
        "reasons": [str(sample.get("preference_kind") or "policy_seed")],
        "snapshot_id": sample_id,
        "snapshot_trigger_kind": "policy_seed_preference",
        "snapshot_reasons": sample_tags,
        "snapshot_normalized_state": state_snapshot,
        "snapshot_decision_context": {
            "encounter_names": state.get("encounter_names") or [],
            "chosen_action": chosen_move,
            "preferred_action": preferred_move,
            "chosen_tags": sample.get("chosen_tags") or [],
            "preferred_tags": sample.get("preferred_tags") or [],
            "score_gap": sample.get("score_gap"),
        },
        "hidden_intent_active": any("Unknown" in str(monster.get("intent") or "") for monster in (state.get("monsters") or [])),
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
        "belief_lethal_probability": 1.0 if int(state.get("player_hp") or 0) <= unblocked else 0.0,
        "belief_urgent_probability": 1.0 if incoming >= max(int(state.get("player_hp") or 0) // 4, 8) else 0.0,
        "candidate_search_rank": 0 if is_positive else 1,
        "candidate_search_avg_score": float(sample.get("preferred_score") if is_positive else sample.get("chosen_score") or 0.0),
        "candidate_search_order_score": float(sample.get("preferred_score") if is_positive else sample.get("chosen_score") or 0.0),
        "candidate_search_leaf_score": float(sample.get("preferred_score") if is_positive else sample.get("chosen_score") or 0.0),
        "candidate_search_sequence_bonus": 0.0,
        "candidate_search_sequence_frontload_bonus": 0.0,
        "candidate_search_sequence_defer_bonus": 0.0,
        "candidate_search_sequence_branch_bonus": 0.0,
        "candidate_search_sequence_downside_penalty": 0.0,
        "candidate_projected_unblocked": 0.0,
        "candidate_projected_enemy_total": float(sum(int(monster.get("hp") or 0) for monster in (state.get("monsters") or []))),
        "candidate_survives": normalize_outcome(sample.get("preferred_outcome") if is_positive else sample.get("chosen_outcome")) in {"survives", "lethal_win"},
        "candidate_branch_family": None,
    }


def build_rows(seed_paths: list[Path]) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for seed_path in seed_paths:
        for _, sample in iter_jsonl(seed_path):
            chosen_move = action_debug_to_move_label(sample.get("chosen_action"))
            preferred_move = action_debug_to_move_label(sample.get("preferred_action"))
            if not chosen_move or not preferred_move:
                continue
            if chosen_move == preferred_move:
                continue
            label_strength = preference_label_strength(sample)
            split = stable_split(str(sample.get("sample_id") or preferred_move))
            state_snapshot = preference_state_to_snapshot(sample.get("state") or {})
            sample_tags = sample_tags_from_preference_sample(sample)
            candidates = [preferred_move, chosen_move]
            for candidate_index, candidate_move in enumerate(candidates):
                rows.append(
                    build_candidate_row(
                        sample=sample,
                        split=split,
                        candidate_move=candidate_move,
                        candidate_index=candidate_index,
                        candidate_count=len(candidates),
                        is_positive=(candidate_move == preferred_move),
                        preferred_move=preferred_move,
                        chosen_move=chosen_move,
                        label_strength=label_strength,
                        state_snapshot=state_snapshot,
                        sample_tags=sample_tags,
                    )
                )
    return rows


def load_rows(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [row for _, row in iter_jsonl(path)]


def main() -> int:
    parser = argparse.ArgumentParser(description="Convert offline policy seed preferences into candidate-level reranker datasets.")
    parser.add_argument(
        "--seed-glob",
        default=str(REPO_ROOT / "data" / "combat_lab" / "policy_seed_set_*.jsonl"),
        help="Comma-separated globs used to discover policy seed set JSONL files.",
    )
    parser.add_argument(
        "--out-dir",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset",
        type=Path,
        help="Directory where candidate-level datasets will be written.",
    )
    parser.add_argument(
        "--dataset-prefix",
        default="policy_seed_reranker",
        help="Output prefix for the converted policy seed dataset.",
    )
    parser.add_argument(
        "--append-dataset-prefix",
        default="",
        help="Optional existing dataset prefix to append and emit a mixed dataset alongside the seed-only one.",
    )
    parser.add_argument(
        "--mixed-dataset-prefix",
        default="mixed_combat_reranker",
        help="Output prefix used when --append-dataset-prefix is provided.",
    )
    args = parser.parse_args()

    seed_paths = []
    seen_paths = set()
    for pattern in [item.strip() for item in str(args.seed_glob or "").split(",") if item.strip()]:
        for path in sorted(glob.glob(pattern)):
            if path in seen_paths:
                continue
            seen_paths.add(path)
            seed_paths.append(Path(path))
    rows = build_rows(seed_paths)

    split_rows = {
        "train": [row for row in rows if row["split"] == "train"],
        "val": [row for row in rows if row["split"] == "val"],
        "test": [row for row in rows if row["split"] == "test"],
    }
    for split, split_values in split_rows.items():
        write_jsonl(args.out_dir / f"{args.dataset_prefix}_{split}.jsonl", split_values)

    label_strength_counts = Counter(str(row.get("label_strength") or "baseline_weak") for row in rows[::2])
    sample_tag_counts: Counter[str] = Counter()
    curriculum_counts: Counter[str] = Counter()
    for row in rows[::2]:
        curriculum_counts[str(row.get("curriculum_tag") or "policy_seed")] += 1
        for tag in row.get("sample_tags") or []:
            sample_tag_counts[str(tag)] += 1

    summary = {
        "dataset_prefix": args.dataset_prefix,
        "seed_glob": args.seed_glob,
        "seed_files": [str(path) for path in seed_paths],
        "frame_count": len(rows) // 2,
        "candidate_rows": len(rows),
        "split_row_counts": {key: len(value) for key, value in split_rows.items()},
        "label_strength_frame_counts": dict(label_strength_counts),
        "curriculum_tag_counts": dict(curriculum_counts),
        "sample_tag_counts": dict(sample_tag_counts),
        "notes": [
            "policy_seed_set is treated as the first offline preference teacher",
            "preferred_action is the positive candidate, chosen_action is the baseline negative/reference candidate",
            "rows keep the same candidate-level schema used by archived oracle-labeled combat datasets",
        ],
    }
    write_json(args.out_dir / f"{args.dataset_prefix}_dataset_summary.json", summary)

    if args.append_dataset_prefix:
        mixed_counts = {}
        for split, split_values in split_rows.items():
            appended = load_rows(args.out_dir / f"{args.append_dataset_prefix}_{split}.jsonl")
            mixed_rows = split_values + appended
            write_jsonl(args.out_dir / f"{args.mixed_dataset_prefix}_{split}.jsonl", mixed_rows)
            mixed_counts[split] = len(mixed_rows)
        mixed_summary = {
            "dataset_prefix": args.mixed_dataset_prefix,
            "components": [args.dataset_prefix, args.append_dataset_prefix],
            "split_row_counts": mixed_counts,
            "notes": [
                "mixed dataset concatenates policy_seed candidate rows with an existing packed candidate dataset",
                "use this to compare seed-only vs seed-plus-strong-archived trainer runs",
            ],
        }
        write_json(args.out_dir / f"{args.mixed_dataset_prefix}_dataset_summary.json", mixed_summary)

    import json
    print(json.dumps(summary, indent=2, ensure_ascii=False))
    print(f"wrote policy seed reranker dataset to {args.out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
