#!/usr/bin/env python3
from __future__ import annotations

import argparse
import glob
import json
from pathlib import Path
from typing import Any

from combat_reranker_common import (
    curriculum_tag_from_spec_name,
    iter_jsonl,
    preference_label_strength,
    sample_tags_from_preference_sample,
    write_json,
    write_jsonl,
)

REPO_ROOT = Path(__file__).resolve().parents[2]


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def normalize_path(path: str | Path | None) -> str | None:
    if not path:
        return None
    return str(Path(path))


def build_seed_rows(seed_paths: list[Path]) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for seed_path in seed_paths:
        for _, sample in iter_jsonl(seed_path):
            rows.append(
                {
                    "sample_origin": "policy_seed_set",
                    "teacher_source": str(sample.get("preferred_source") or "offline_audit_search"),
                    "curriculum_tag": str(sample.get("preference_kind") or "policy_seed"),
                    "state_source": str(sample.get("state_source") or "reconstructed_live_replay_state"),
                    "label_source": str(sample.get("preferred_source") or "offline_audit_search"),
                    "label_strength": preference_label_strength(sample),
                    "sample_id": sample.get("sample_id"),
                    "spec_name": None,
                    "run_id": None,
                    "source_path": str(seed_path),
                    "raw_source_path": normalize_path(sample.get("source_path")),
                    "chosen_action": sample.get("chosen_action"),
                    "preferred_action": sample.get("preferred_action"),
                    "chosen_outcome": sample.get("chosen_outcome"),
                    "preferred_outcome": sample.get("preferred_outcome"),
                    "score_gap": sample.get("score_gap"),
                    "sample_tags": sample_tags_from_preference_sample(sample),
                }
            )
    return rows


def build_spec_rows(spec_paths: list[Path]) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for spec_path in spec_paths:
        spec_name = spec_path.stem
        rows.append(
            {
                "sample_origin": "combat_lab_spec",
                "teacher_source": "combat_lab_curriculum",
                "curriculum_tag": curriculum_tag_from_spec_name(spec_name),
                "state_source": "combat_lab_spec",
                "label_source": "combat_lab_policy_trace",
                "label_strength": "baseline_weak",
                "sample_id": spec_name,
                "spec_name": spec_name,
                "run_id": None,
                "source_path": str(spec_path),
                "raw_source_path": None,
                "sample_tags": [],
            }
        )
    return rows


def build_archived_rows(baseline: dict[str, Any]) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for run in baseline.get("selected_runs") or []:
        rows.append(
            {
                "sample_origin": "archived_clean_run",
                "teacher_source": "offline_decision_audit_search",
                "curriculum_tag": "archived_real_state",
                "state_source": "validated_livecomm_audit",
                "label_source": "offline_decision_audit_search",
                "label_strength": "baseline_weak",
                "sample_id": run.get("run_id"),
                "spec_name": None,
                "run_id": run.get("run_id"),
                "source_path": normalize_path(run.get("combat_suspects_path")),
                "raw_source_path": normalize_path(run.get("raw_path")),
                "failure_snapshots_path": normalize_path(run.get("failure_snapshots_path")),
                "validation_path": normalize_path(run.get("validation_path")),
                "classification_label": run.get("classification_label"),
                "validation_status": run.get("validation_status"),
                "sample_tags": [],
            }
        )
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description="Build a unified local combat source manifest for offline learning.")
    parser.add_argument(
        "--baseline",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_baseline.json",
        type=Path,
        help="Frozen baseline manifest used for archived clean run sources.",
    )
    parser.add_argument(
        "--spec-dir",
        default=REPO_ROOT / "data" / "combat_lab" / "specs",
        type=Path,
        help="Directory containing combat_lab author specs.",
    )
    parser.add_argument(
        "--seed-glob",
        default=str(REPO_ROOT / "data" / "combat_lab" / "policy_seed_set_*.jsonl"),
        help="Glob used to find policy seed set JSONL files.",
    )
    parser.add_argument(
        "--out-dir",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset",
        type=Path,
        help="Where to write the local source manifest rows.",
    )
    args = parser.parse_args()

    baseline = read_json(args.baseline)
    seed_paths = sorted(Path(path) for path in glob.glob(args.seed_glob))
    spec_paths = sorted(args.spec_dir.glob("*.json"))

    seed_rows = build_seed_rows(seed_paths)
    spec_rows = build_spec_rows(spec_paths)
    archived_rows = build_archived_rows(baseline)

    manifest = {
        "baseline_path": str(args.baseline),
        "spec_dir": str(args.spec_dir),
        "seed_glob": args.seed_glob,
        "accepted_run_ids": baseline.get("accepted_run_ids") or [],
        "source_counts": {
            "policy_seed_set": len(seed_rows),
            "combat_lab_spec": len(spec_rows),
            "archived_clean_run": len(archived_rows),
        },
        "seed_files": [str(path) for path in seed_paths],
        "spec_files": [str(path) for path in spec_paths],
        "notes": [
            "livecomm is not a primary training loop source in this manifest",
            "policy_seed_set rows are the preferred early offline teacher",
            "archived clean runs contribute real state distribution but not strong labels by default",
        ],
    }

    write_json(args.out_dir / "local_combat_source_manifest.json", manifest)
    write_jsonl(args.out_dir / "local_combat_seed_rows.jsonl", seed_rows)
    write_jsonl(args.out_dir / "local_combat_spec_rows.jsonl", spec_rows)
    write_jsonl(args.out_dir / "local_combat_archived_rows.jsonl", archived_rows)

    print(json.dumps(manifest, indent=2, ensure_ascii=False))
    print(f"wrote local combat source rows to {args.out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
