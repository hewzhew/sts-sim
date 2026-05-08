#!/usr/bin/env python3
"""Export DecisionRecord JSONL to a flat candidate table.

The JSONL output is always available. Parquet output is supported when pyarrow
is installed; this keeps Arrow as an optional storage layer rather than a
runtime dependency for the simulator.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Iterable


def iter_jsonl(paths: list[Path]) -> Iterable[dict[str, Any]]:
    for path in paths:
        with path.open("r", encoding="utf-8") as handle:
            for line in handle:
                line = line.strip()
                if line:
                    yield json.loads(line)


def action_id_value(value: Any) -> int | None:
    if isinstance(value, int):
        return value
    if isinstance(value, dict) and "0" in value and isinstance(value["0"], int):
        return value["0"]
    if isinstance(value, list) and len(value) == 1 and isinstance(value[0], int):
        return value[0]
    return None


def teacher_returns(record: dict[str, Any]) -> dict[int, dict[str, Any]]:
    label = record.get("teacher_label")
    if not isinstance(label, dict):
        return {}
    out: dict[int, dict[str, Any]] = {}
    for item in label.get("labels") or []:
        action_id = action_id_value(item.get("action_id"))
        if action_id is not None:
            out[action_id] = item
    return out


def flatten_records(paths: list[Path]) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for record in iter_jsonl(paths):
        decision = record.get("decision_id") or {}
        obs = (record.get("observation") or {}).get("payload") or {}
        combat = obs.get("combat") or {}
        deck = obs.get("deck") or {}
        screen = obs.get("screen") or {}
        labels = teacher_returns(record)
        behavior_action = action_id_value(record.get("behavior_action"))
        eligibility = (((record.get("teacher_label") or {}).get("payload") or {}).get("training_eligibility") or {})
        for candidate in record.get("candidates") or []:
            action_id = action_id_value(candidate.get("id"))
            payload = candidate.get("payload") or {}
            action = payload.get("action") or {}
            card = payload.get("card") or {}
            label = labels.get(action_id or -1) or {}
            rows.append(
                {
                    "schema_version": "decision_record_candidate_table_v0",
                    "episode_id": decision.get("episode_id"),
                    "step_index": decision.get("step_index"),
                    "decision_type": decision.get("decision_type"),
                    "seed": record.get("seed"),
                    "state_hash_before": record.get("state_hash_before"),
                    "observation_schema_version": record.get("observation_schema_version"),
                    "action_schema_version": record.get("action_schema_version"),
                    "return_spec_version": record.get("return_spec_version"),
                    "action_id": action_id,
                    "action_index": candidate.get("action_index"),
                    "action_key": candidate.get("action_key"),
                    "action_kind": candidate.get("action_kind"),
                    "action_type": action.get("type") if isinstance(action, dict) else None,
                    "selected_by_behavior": action_id == behavior_action,
                    "teacher_mean_return": label.get("mean_return"),
                    "teacher_stderr": label.get("stderr"),
                    "teacher_sample_count": label.get("sample_count"),
                    "teacher_dominance": label.get("dominance"),
                    "teacher_label_use": eligibility.get("label_use"),
                    "teacher_eligible_for_training": eligibility.get("eligible_for_training"),
                    "floor": obs.get("floor"),
                    "act": obs.get("act"),
                    "current_hp": obs.get("current_hp"),
                    "max_hp": obs.get("max_hp"),
                    "gold": obs.get("gold"),
                    "deck_size": obs.get("deck_size"),
                    "visible_incoming_damage": combat.get("visible_incoming_damage"),
                    "energy": combat.get("energy"),
                    "alive_monster_count": combat.get("alive_monster_count"),
                    "total_monster_hp": combat.get("total_monster_hp"),
                    "deck_attack_count": deck.get("attack_count"),
                    "deck_skill_count": deck.get("skill_count"),
                    "deck_power_count": deck.get("power_count"),
                    "screen_reward_phase": screen.get("reward_phase"),
                    "card_id": card.get("card_id"),
                    "card_type_id": card.get("card_type_id"),
                    "card_cost": card.get("cost"),
                    "card_base_damage": card.get("base_damage"),
                    "card_base_block": card.get("base_block"),
                    "card_draws_cards": card.get("draws_cards"),
                    "card_applies_vulnerable": card.get("applies_vulnerable"),
                    "card_applies_weak": card.get("applies_weak"),
                    "card_scaling_piece": card.get("scaling_piece"),
                }
            )
    return rows


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


def write_parquet(path: Path, rows: list[dict[str, Any]]) -> None:
    try:
        import pyarrow as pa
        import pyarrow.parquet as pq
    except ModuleNotFoundError as exc:
        raise SystemExit("pyarrow is required for --parquet-out") from exc
    path.parent.mkdir(parents=True, exist_ok=True)
    table = pa.Table.from_pylist(rows)
    pq.write_table(table, path)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inputs", nargs="+", type=Path, required=True)
    parser.add_argument("--jsonl-out", type=Path, required=True)
    parser.add_argument("--parquet-out", type=Path)
    parser.add_argument("--summary-out", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    rows = flatten_records(args.inputs)
    write_jsonl(args.jsonl_out, rows)
    if args.parquet_out:
        write_parquet(args.parquet_out, rows)
    summary = {
        "schema_version": "decision_record_candidate_table_export_summary_v0",
        "inputs": [str(path) for path in args.inputs],
        "jsonl_out": str(args.jsonl_out),
        "parquet_out": str(args.parquet_out) if args.parquet_out else None,
        "row_count": len(rows),
    }
    summary_out = args.summary_out or args.jsonl_out.with_suffix(".summary.json")
    summary_out.parent.mkdir(parents=True, exist_ok=True)
    summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
