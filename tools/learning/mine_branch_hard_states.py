#!/usr/bin/env python3
"""Mine branch-outcome hard states for targeted data collection/search.

This consumes pair prediction audit rows. It does not create action labels,
winner labels, or policy preferences. Rows are prioritized as data/search
allocation candidates when the current value/risk model underestimates large
outcome differences or sees high tail risk.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Iterable


def iter_jsonl(path: Path) -> Iterable[dict[str, Any]]:
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                yield json.loads(line)


def bump(counter: dict[str, int], key: str, amount: int = 1) -> None:
    counter[key] = int(counter.get(key) or 0) + amount


def candidate(row: dict[str, Any], side: str) -> dict[str, Any]:
    return ((row.get(side) or {}).get("candidate") or {})


def context(row: dict[str, Any], side: str) -> dict[str, Any]:
    return ((row.get(side) or {}).get("decision_context") or {})


def pair_kind(row: dict[str, Any]) -> str:
    return f"{candidate(row, 'left').get('action_kind')}->{candidate(row, 'right').get('action_kind')}"


def pair_card(row: dict[str, Any]) -> str:
    return f"{candidate(row, 'left').get('card_id')}->{candidate(row, 'right').get('card_id')}"


def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        return float(value)
    except (TypeError, ValueError):
        return default


def mine_reasons(row: dict[str, Any], *, tail_threshold: float) -> list[str]:
    targets = row.get("targets") or {}
    outputs = row.get("model_outputs") or {}
    errors = row.get("errors") or {}
    signals = row.get("search_allocation_signals") or {}
    obs = row.get("observation") or {}
    left_ctx = context(row, "left")
    right_ctx = context(row, "right")
    left_is_end_turn = candidate(row, "left").get("action_kind") == "end_turn"
    right_is_end_turn = candidate(row, "right").get("action_kind") == "end_turn"
    true_hp = safe_float(targets.get("hp_left_minus_right"))
    branch_pred = safe_float(outputs.get("branch_model_hp_left_minus_right"))
    residual_pred = safe_float(outputs.get("residual_corrected_hp_left_minus_right"))
    tail_abs10 = signals.get("tail_abs_hp_diff_ge_10_probability")
    tail_left_worse10 = signals.get("tail_left_worse_ge_10_probability")
    reasons: list[str] = []
    if abs(true_hp) >= 10:
        reasons.append("true_abs_hp_diff_ge_10")
    if abs(true_hp) >= 15:
        reasons.append("true_abs_hp_diff_ge_15")
    if errors.get("branch_model_severe_underestimate_abs_ge_10_pred_abs_lt_5"):
        reasons.append("branch_model_severe_underestimate_ge_10")
    if errors.get("residual_corrected_severe_underestimate_abs_ge_10_pred_abs_lt_5"):
        reasons.append("residual_model_severe_underestimate_ge_10")
    if abs(true_hp) >= 10 and abs(residual_pred) >= 5 and abs(branch_pred) < 5:
        reasons.append("residual_model_recovers_tail_magnitude")
    if isinstance(tail_abs10, (int, float)) and tail_abs10 >= tail_threshold:
        reasons.append("tail_model_high_abs_diff_ge_10")
    if isinstance(tail_left_worse10, (int, float)) and tail_left_worse10 >= tail_threshold:
        reasons.append("tail_model_high_left_worse_ge_10")
    if pair_kind(row) in {"end_turn->play_card", "play_card->end_turn"}:
        reasons.append("end_turn_play_card_pair")
    if (left_is_end_turn and left_ctx.get("end_turn_with_playable_cards")) or (
        right_is_end_turn and right_ctx.get("end_turn_with_playable_cards")
    ):
        reasons.append("end_turn_with_playable_cards")
    if (left_is_end_turn and left_ctx.get("end_turn_with_unspent_energy")) or (
        right_is_end_turn and right_ctx.get("end_turn_with_unspent_energy")
    ):
        reasons.append("end_turn_with_unspent_energy")
    if safe_float(obs.get("visible_incoming_damage")) >= 10:
        reasons.append("high_incoming_damage")
    if safe_float(obs.get("current_hp")) <= 50:
        reasons.append("low_current_hp")
    if safe_float(obs.get("alive_monster_count")) >= 2:
        reasons.append("multi_enemy")
    if safe_float(left_ctx.get("playable_unique_damage_sum")) >= 20 or safe_float(
        right_ctx.get("playable_unique_damage_sum")
    ) >= 20:
        reasons.append("large_public_damage_opportunity")
    return reasons


def priority_score(row: dict[str, Any], reasons: list[str]) -> float:
    targets = row.get("targets") or {}
    outputs = row.get("model_outputs") or {}
    signals = row.get("search_allocation_signals") or {}
    true_hp = safe_float(targets.get("hp_left_minus_right"))
    branch_pred = safe_float(outputs.get("branch_model_hp_left_minus_right"))
    residual_pred = safe_float(outputs.get("residual_corrected_hp_left_minus_right"))
    tail_abs10 = signals.get("tail_abs_hp_diff_ge_10_probability")
    score = abs(true_hp)
    score += 5.0 * int("branch_model_severe_underestimate_ge_10" in reasons)
    score += 3.0 * int("end_turn_play_card_pair" in reasons)
    score += 2.0 * int("high_incoming_damage" in reasons)
    score += 2.0 * int("multi_enemy" in reasons)
    score += abs(residual_pred - branch_pred) * 0.25
    if isinstance(tail_abs10, (int, float)):
        score += 5.0 * float(tail_abs10)
    return score


def mine_rows(
    rows: list[dict[str, Any]],
    *,
    tail_threshold: float,
    max_rows: int,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    mined: list[dict[str, Any]] = []
    summary: dict[str, Any] = {
        "schema_version": "branch_hard_state_mining_summary_v0",
        "input_pair_prediction_count": len(rows),
        "candidate_pair_count_before_cap": 0,
        "mined_pair_count": 0,
        "reason_counts": {},
        "pair_kind_counts": {},
        "pair_card_counts": {},
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "hard_states_are_sampling_targets": True,
        },
    }
    for row in rows:
        if row.get("trainable_as_action_label") is not False:
            continue
        if (row.get("label_policy") or {}).get("action_label") is not False:
            continue
        reasons = mine_reasons(row, tail_threshold=tail_threshold)
        if not reasons:
            continue
        targets = row.get("targets") or {}
        outputs = row.get("model_outputs") or {}
        errors = row.get("errors") or {}
        signals = row.get("search_allocation_signals") or {}
        item = {
            "schema_version": "branch_hard_state_candidate_v0",
            "trainable_role": "hard_state_sampling_target",
            "trainable_as_action_label": False,
            "episode_seed": row.get("episode_seed"),
            "episode_step": row.get("episode_step"),
            "decision_id": row.get("decision_id"),
            "comparison_id": row.get("comparison_id"),
            "pairing": row.get("pairing"),
            "priority_score": priority_score(row, reasons),
            "reasons": reasons,
            "pair_kind": pair_kind(row),
            "pair_card": pair_card(row),
            "left": row.get("left"),
            "right": row.get("right"),
            "observation": row.get("observation"),
            "targets": {
                "hp_left_minus_right": targets.get("hp_left_minus_right"),
                "total_reward_left_minus_right": targets.get(
                    "total_reward_left_minus_right"
                ),
            },
            "model_outputs": {
                "branch_model_hp_left_minus_right": outputs.get(
                    "branch_model_hp_left_minus_right"
                ),
                "residual_corrected_hp_left_minus_right": outputs.get(
                    "residual_corrected_hp_left_minus_right"
                ),
                "tail_probabilities": outputs.get("tail_probabilities"),
            },
            "errors": errors,
            "search_allocation_signals": signals,
            "label_policy": {
                "action_label": False,
                "source": "branch_hard_state_mining_v0",
            },
        }
        mined.append(item)
    mined.sort(key=lambda item: (-float(item["priority_score"]), item["episode_seed"], item["episode_step"]))
    summary["candidate_pair_count_before_cap"] = len(mined)
    if max_rows > 0:
        mined = mined[:max_rows]
    summary["reason_counts"] = {}
    summary["pair_kind_counts"] = {}
    summary["pair_card_counts"] = {}
    for item in mined:
        for reason in item["reasons"]:
            bump(summary["reason_counts"], reason)
        bump(summary["pair_kind_counts"], item["pair_kind"])
        bump(summary["pair_card_counts"], item["pair_card"])
    summary["mined_pair_count"] = len(mined)
    summary["top_reasons"] = sorted(
        summary["reason_counts"].items(), key=lambda item: (-item[1], item[0])
    )[:20]
    summary["top_pair_kinds"] = sorted(
        summary["pair_kind_counts"].items(), key=lambda item: (-item[1], item[0])
    )[:20]
    return mined, summary


def assert_no_action_label_leak(rows: list[dict[str, Any]]) -> None:
    forbidden = {"winner", "preferred_action", "selected_action", "teacher_choice"}
    for index, row in enumerate(rows):
        if row.get("trainable_as_action_label") is not False:
            raise ValueError(f"mined row {index} is action-label-like")
        if (row.get("label_policy") or {}).get("action_label") is not False:
            raise ValueError(f"mined row {index} has action_label=true")
        for key in forbidden:
            if key in row:
                raise ValueError(f"mined row {index} has forbidden key {key}")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pair-predictions", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--tail-threshold", type=float, default=0.35)
    parser.add_argument("--max-rows", type=int, default=500)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    rows = list(iter_jsonl(args.pair_predictions))
    mined, summary = mine_rows(
        rows, tail_threshold=args.tail_threshold, max_rows=args.max_rows
    )
    assert_no_action_label_leak(mined)
    write_jsonl(args.out, mined)
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
