#!/usr/bin/env python3
"""
Build reward/event/combat sidecar datasets from the approved audit truth sources.

This script intentionally consumes the existing audit artifacts instead of reviving
any legacy exporter pipeline. It is designed for the "freeze baseline -> build
dataset -> offline eval" workflow.
"""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def iter_jsonl(path: Path):
    with path.open("r", encoding="utf-8") as handle:
        for line_no, line in enumerate(handle, start=1):
            text = line.strip()
            if not text:
                continue
            try:
                yield line_no, json.loads(text)
            except json.JSONDecodeError as exc:
                raise RuntimeError(f"{path}:{line_no}: invalid JSONL: {exc}") from exc


def safe_read_json(path_str: str | None) -> dict[str, Any] | None:
    if not path_str:
        return None
    path = Path(path_str)
    if not path.exists():
        return None
    return read_json(path)


def sample_weight_from_combat_row(row: dict[str, Any]) -> float:
    if row.get("tight_root_gap"):
        return 0.15
    if abs(float(row.get("top_gap") or 0.0)) < 1.0:
        return 0.25
    if row.get("heuristic_search_gap"):
        return 1.0
    return 0.5


@dataclass
class RunFilterVerdict:
    accepted: bool
    reason: str


def run_filter_verdict(manifest: dict[str, Any], validation: dict[str, Any] | None) -> RunFilterVerdict:
    if validation is None:
        return RunFilterVerdict(False, "missing_validation")
    if not str(validation.get("status", "")).startswith("ok"):
        return RunFilterVerdict(False, "validation_not_ok")
    if validation.get("trace_incomplete"):
        return RunFilterVerdict(False, "trace_incomplete")
    if validation.get("reward_loop_detected"):
        return RunFilterVerdict(False, "reward_loop_detected")
    if validation.get("bootstrap_protocol_ok") is False:
        return RunFilterVerdict(False, "bootstrap_protocol_bad")

    counts = manifest.get("counts") or {}
    if int(counts.get("engine_bugs") or 0) > 0:
        return RunFilterVerdict(False, "engine_bugs_present")
    if int(counts.get("replay_failures") or 0) > 0:
        return RunFilterVerdict(False, "replay_failures_present")

    label = str(manifest.get("classification_label") or "")
    if "tainted" in label:
        return RunFilterVerdict(False, "tainted_classification")

    return RunFilterVerdict(True, "ok")


def build_reward_rows(run_id: str, reward_audit_path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if not reward_audit_path.exists():
        return rows

    for line_no, record in iter_jsonl(reward_audit_path):
        if record.get("kind") != "bot_reward_decision":
            continue
        evaluation = record.get("bot_evaluation") or {}
        candidates = evaluation.get("cards") or []
        choice = record.get("bot_choice") or {}
        rows.append(
            {
                "dataset_kind": "reward",
                "state_source": "validated_livecomm_audit",
                "label_source": "baseline_bot_choice",
                "label_strength": "baseline_weak",
                "run_id": run_id,
                "line_no": line_no,
                "frame": record.get("frame"),
                "response_id": record.get("response_id"),
                "state_frame_id": record.get("state_frame_id"),
                "floor": record.get("floor"),
                "act": record.get("act"),
                "class": record.get("class"),
                "current_hp": record.get("current_hp"),
                "max_hp": record.get("max_hp"),
                "gold": record.get("gold"),
                "deck_size": record.get("deck_size"),
                "offered_cards": record.get("offered_cards") or [],
                "candidates": candidates,
                "recommended_choice": evaluation.get("recommended_choice"),
                "label": choice,
                "skip_chosen": choice.get("kind") == "skip",
                "rule_context_summaries": sorted(
                    {
                        candidate.get("delta_rule_context_summary")
                        for candidate in candidates
                        if candidate.get("delta_rule_context_summary")
                    }
                ),
            }
        )
    return rows


def build_event_rows(run_id: str, event_audit_path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if not event_audit_path.exists():
        return rows

    for line_no, record in iter_jsonl(event_audit_path):
        decision = record.get("decision") or {}
        rows.append(
            {
                "dataset_kind": "event",
                "state_source": "validated_livecomm_audit",
                "label_source": "baseline_bot_choice",
                "label_strength": "baseline_weak",
                "run_id": run_id,
                "line_no": line_no,
                "frame": record.get("frame"),
                "room_phase": record.get("room_phase"),
                "screen": record.get("screen"),
                "command": record.get("command"),
                "event_id": decision.get("event_id"),
                "event_name": decision.get("event_name"),
                "family": decision.get("family"),
                "rationale_key": decision.get("rationale_key"),
                "screen_index": decision.get("screen_index"),
                "screen_key": decision.get("screen_key"),
                "screen_source": decision.get("screen_source"),
                "chosen_option_index": decision.get("chosen_option_index"),
                "chosen_option_label": decision.get("chosen_option_label"),
                "chosen_option_text": decision.get("chosen_option_text"),
                "command_index": decision.get("command_index"),
                "score": decision.get("score"),
                "safety_override_applied": decision.get("safety_override_applied"),
                "decision": decision,
            }
        )
    return rows


def build_combat_rows(run_id: str, combat_suspects_path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if not combat_suspects_path.exists():
        return rows

    for line_no, record in iter_jsonl(combat_suspects_path):
        rows.append(
            {
                "dataset_kind": "combat",
                "state_source": "validated_livecomm_audit",
                "run_id": run_id,
                "line_no": line_no,
                "frame_count": record.get("frame_count"),
                "response_id": record.get("response_id"),
                "state_frame_id": record.get("state_frame_id"),
                "chosen_move": record.get("chosen_move"),
                "heuristic_move": record.get("heuristic_move"),
                "search_move": record.get("search_move"),
                "top_candidates": record.get("top_candidates") or [],
                "top_gap": record.get("top_gap"),
                "sequence_bonus": record.get("sequence_bonus"),
                "sequence_frontload_bonus": record.get("sequence_frontload_bonus"),
                "sequence_defer_bonus": record.get("sequence_defer_bonus"),
                "sequence_branch_bonus": record.get("sequence_branch_bonus"),
                "sequence_downside_penalty": record.get("sequence_downside_penalty"),
                "branch_family": record.get("branch_family"),
                "sequencing_rationale_key": record.get("sequencing_rationale_key"),
                "branch_rationale_key": record.get("branch_rationale_key"),
                "downside_rationale_key": record.get("downside_rationale_key"),
                "heuristic_search_gap": bool(record.get("heuristic_search_gap")),
                "tight_root_gap": bool(record.get("tight_root_gap")),
                "large_sequence_bonus": bool(record.get("large_sequence_bonus")),
                "reasons": record.get("reasons") or [],
                "sample_weight": sample_weight_from_combat_row(record),
                "strong_label": not bool(record.get("tight_root_gap")),
                "label_source": "baseline_bot_choice",
                "label_strength": (
                    "baseline_weak"
                    if bool(record.get("tight_root_gap"))
                    else "filtered_low_weight"
                    if not bool(record.get("heuristic_search_gap"))
                    else "baseline_weak"
                ),
                "hidden_intent_active": record.get("hidden_intent_active"),
                "visible_incoming": record.get("visible_incoming"),
                "visible_unblocked": record.get("visible_unblocked"),
                "belief_expected_incoming": record.get("belief_expected_incoming"),
                "belief_expected_unblocked": record.get("belief_expected_unblocked"),
                "belief_max_incoming": record.get("belief_max_incoming"),
                "belief_max_unblocked": record.get("belief_max_unblocked"),
                "value_incoming": record.get("value_incoming"),
                "value_unblocked": record.get("value_unblocked"),
                "survival_guard_incoming": record.get("survival_guard_incoming"),
                "survival_guard_unblocked": record.get("survival_guard_unblocked"),
                "belief_attack_probability": record.get("belief_attack_probability"),
                "belief_lethal_probability": record.get("belief_lethal_probability"),
                "belief_urgent_probability": record.get("belief_urgent_probability"),
            }
        )
    return rows


def load_failure_snapshots(path: Path) -> dict[tuple[int | None, int | None, int | None], dict[str, Any]]:
    snapshots: dict[tuple[int | None, int | None, int | None], dict[str, Any]] = {}
    if not path.exists():
        return snapshots

    for _, record in iter_jsonl(path):
        key = (
            record.get("frame"),
            record.get("response_id"),
            record.get("state_frame_id"),
        )
        snapshots[key] = record
    return snapshots


def attach_snapshot_context(
    rows: list[dict[str, Any]],
    snapshots: dict[tuple[int | None, int | None, int | None], dict[str, Any]],
) -> None:
    for row in rows:
        key = (
            row.get("frame_count"),
            row.get("response_id"),
            row.get("state_frame_id"),
        )
        snapshot = snapshots.get(key)
        if snapshot is None and row.get("frame_count") is not None:
            fallback_key = (row.get("frame_count"), None, None)
            snapshot = snapshots.get(fallback_key)
        if snapshot is None:
            row["snapshot_id"] = None
            row["snapshot_trigger_kind"] = None
            row["snapshot_reasons"] = []
            row["snapshot_normalized_state"] = None
            row["snapshot_decision_context"] = None
            continue

        row["snapshot_id"] = snapshot.get("snapshot_id")
        row["snapshot_trigger_kind"] = snapshot.get("trigger_kind")
        row["snapshot_reasons"] = snapshot.get("reasons") or []
        row["snapshot_normalized_state"] = snapshot.get("normalized_state")
        row["snapshot_decision_context"] = snapshot.get("decision_context")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description="Build sidecar datasets from learning baseline manifests.")
    parser.add_argument(
        "--baseline",
        required=True,
        type=Path,
        help="Path to a baseline manifest created by `sts_dev_tool logs freeze-baseline`.",
    )
    parser.add_argument(
        "--out-dir",
        default=Path("tools/artifacts/learning_dataset"),
        type=Path,
        help="Directory to write reward/event/combat dataset files into.",
    )
    args = parser.parse_args()

    baseline = read_json(args.baseline)
    selected_runs = baseline.get("selected_runs") or []

    reward_rows: list[dict[str, Any]] = []
    event_rows: list[dict[str, Any]] = []
    combat_rows: list[dict[str, Any]] = []
    skipped_runs: list[dict[str, Any]] = []

    for run in selected_runs:
        run_id = str(run.get("run_id"))
        manifest_path = Path(run["manifest_path"])
        manifest = read_json(manifest_path)
        validation = safe_read_json(run.get("validation_path"))
        verdict = run_filter_verdict(manifest, validation)
        if not verdict.accepted:
            skipped_runs.append(
                {
                    "run_id": run_id,
                    "reason": verdict.reason,
                    "classification_label": manifest.get("classification_label"),
                }
            )
            continue

        reward_path = run.get("reward_audit_path")
        if reward_path:
            reward_rows.extend(build_reward_rows(run_id, Path(reward_path)))

        event_path = run.get("event_audit_path")
        if event_path:
            event_rows.extend(build_event_rows(run_id, Path(event_path)))

        combat_path = run.get("combat_suspects_path")
        if combat_path:
            run_combat_rows = build_combat_rows(run_id, Path(combat_path))
            failure_snapshots_path = run.get("failure_snapshots_path")
            if failure_snapshots_path:
                attach_snapshot_context(
                    run_combat_rows, load_failure_snapshots(Path(failure_snapshots_path))
                )
            combat_rows.extend(run_combat_rows)

    out_dir: Path = args.out_dir
    write_jsonl(out_dir / "reward_rows.jsonl", reward_rows)
    write_jsonl(out_dir / "event_rows.jsonl", event_rows)
    write_jsonl(out_dir / "combat_rows.jsonl", combat_rows)

    summary = {
        "baseline": str(args.baseline),
        "selected_runs": len(selected_runs),
        "accepted_runs": len(selected_runs) - len(skipped_runs),
        "accepted_run_ids": [
            str(run.get("run_id"))
            for run in selected_runs
            if str(run.get("run_id")) not in {entry["run_id"] for entry in skipped_runs}
        ],
        "skipped_runs": skipped_runs,
        "row_counts": {
            "reward": len(reward_rows),
            "event": len(event_rows),
            "combat": len(combat_rows),
        },
        "notes": [
            "reward rows only consume `kind=bot_reward_decision` entries",
            "combat rows down-weight tight_root_gap samples instead of discarding them entirely",
            "combat rows attach failure snapshot context when `failure_snapshots.jsonl` is present",
            "runs with engine bugs, replay failures, tainted classification, or non-ok validation are excluded",
        ],
    }
    with (out_dir / "summary.json").open("w", encoding="utf-8") as handle:
        json.dump(summary, handle, indent=2, ensure_ascii=False)
        handle.write("\n")

    print(f"Wrote sidecar datasets to {out_dir}")
    print(json.dumps(summary["row_counts"], indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
