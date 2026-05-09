#!/usr/bin/env python3
"""Export BranchTrace JSONL into value/risk/search-allocation datasets.

This exporter does not create action labels. It filters branch outcome records
into branch-level value/risk examples and ordered pairwise outcome-diff examples
for later model-guided search work.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Iterable


BRANCH_ROW_SCHEMA = "branch_value_risk_example_v0"
PAIR_ROW_SCHEMA = "branch_pair_outcome_diff_example_v0"


def iter_jsonl(paths: list[Path]) -> Iterable[dict[str, Any]]:
    for path in paths:
        with path.open("r", encoding="utf-8") as handle:
            for line in handle:
                line = line.strip()
                if line:
                    yield json.loads(line)


def bump(counter: dict[str, int], key: str, amount: int = 1) -> None:
    counter[key] = int(counter.get(key) or 0) + amount


def forced_action_id(trace: dict[str, Any]) -> int | None:
    forced = trace.get("forced_prefix") or []
    if not forced:
        return None
    value = forced[0]
    return value if isinstance(value, int) else None


def action_candidate(trace: dict[str, Any]) -> dict[str, Any]:
    action_id = forced_action_id(trace)
    if action_id is None:
        return {}
    candidates = trace.get("candidates") or []
    if action_id < 0 or action_id >= len(candidates):
        return {}
    return candidates[action_id] or {}


def observation_payload(trace: dict[str, Any]) -> dict[str, Any]:
    return (trace.get("observation") or {}).get("payload") or {}


def candidate_features(candidate: dict[str, Any]) -> dict[str, Any]:
    payload = candidate.get("payload") or {}
    action = payload.get("action") or {}
    card = payload.get("card") or {}
    return {
        "action_id": candidate.get("id"),
        "action_index": candidate.get("action_index"),
        "action_key": candidate.get("action_key"),
        "action_kind": candidate.get("action_kind"),
        "action_type": action.get("type") if isinstance(action, dict) else None,
        "target": action.get("target") if isinstance(action, dict) else None,
        "card_id": card.get("card_id") if isinstance(card, dict) else None,
        "card_type_id": card.get("card_type_id") if isinstance(card, dict) else None,
        "card_cost": card.get("cost") if isinstance(card, dict) else None,
        "card_base_damage": card.get("base_damage") if isinstance(card, dict) else None,
        "card_base_block": card.get("base_block") if isinstance(card, dict) else None,
        "card_draws_cards": card.get("draws_cards") if isinstance(card, dict) else None,
        "card_exhaust": card.get("exhaust") if isinstance(card, dict) else None,
        "card_applies_vulnerable": card.get("applies_vulnerable")
        if isinstance(card, dict)
        else None,
        "card_applies_weak": card.get("applies_weak") if isinstance(card, dict) else None,
        "card_scaling_piece": card.get("scaling_piece") if isinstance(card, dict) else None,
        "card_starter_basic": card.get("starter_basic") if isinstance(card, dict) else None,
    }


def card_payload(candidate: dict[str, Any]) -> dict[str, Any]:
    payload = candidate.get("payload") or {}
    card = payload.get("card")
    return card if isinstance(card, dict) else {}


def action_payload(candidate: dict[str, Any]) -> dict[str, Any]:
    payload = candidate.get("payload") or {}
    action = payload.get("action")
    return action if isinstance(action, dict) else {}


def numeric_card_value(card: dict[str, Any], key: str) -> int:
    value = card.get(key)
    return value if isinstance(value, int) else 0


def decision_context_features(trace: dict[str, Any]) -> dict[str, Any]:
    candidates = trace.get("candidates") or []
    observation = observation_payload(trace)
    combat = observation.get("combat") or {}
    energy = combat.get("energy")
    playable_by_card_index: dict[Any, dict[str, Any]] = {}
    end_turn_legal = False
    playable_candidate_count = 0
    attack_candidate_count = 0
    block_candidate_count = 0
    draw_candidate_count = 0
    debuff_candidate_count = 0
    exhaust_candidate_count = 0
    setup_candidate_count = 0
    zero_cost_candidate_count = 0
    one_cost_candidate_count = 0
    two_plus_cost_candidate_count = 0
    candidate_damage_sum = 0
    candidate_block_sum = 0
    candidate_max_damage = 0
    candidate_max_block = 0
    for candidate in candidates:
        kind = candidate.get("action_kind")
        if kind == "end_turn":
            end_turn_legal = True
            continue
        if kind != "play_card":
            continue
        card = card_payload(candidate)
        action = action_payload(candidate)
        playable_candidate_count += 1
        base_damage = numeric_card_value(card, "base_damage")
        base_block = numeric_card_value(card, "base_block")
        cost = card.get("cost")
        candidate_damage_sum += base_damage
        candidate_block_sum += base_block
        candidate_max_damage = max(candidate_max_damage, base_damage)
        candidate_max_block = max(candidate_max_block, base_block)
        if base_damage > 0:
            attack_candidate_count += 1
        if base_block > 0:
            block_candidate_count += 1
        if card.get("draws_cards"):
            draw_candidate_count += 1
        if card.get("applies_vulnerable") or card.get("applies_weak"):
            debuff_candidate_count += 1
        if card.get("exhaust"):
            exhaust_candidate_count += 1
        if card.get("scaling_piece") or card.get("gains_energy"):
            setup_candidate_count += 1
        if cost == 0:
            zero_cost_candidate_count += 1
        elif cost == 1:
            one_cost_candidate_count += 1
        elif isinstance(cost, int) and cost >= 2:
            two_plus_cost_candidate_count += 1

        # Collapse target variants of the same visible hand card for opportunity-cost totals.
        card_key = action.get("card_index")
        if card_key is None:
            card_key = candidate.get("action_key")
        existing = playable_by_card_index.get(card_key)
        if existing is None:
            playable_by_card_index[card_key] = dict(card)
        else:
            existing["base_damage"] = max(
                numeric_card_value(existing, "base_damage"),
                numeric_card_value(card, "base_damage"),
            )
            existing["base_block"] = max(
                numeric_card_value(existing, "base_block"),
                numeric_card_value(card, "base_block"),
            )
    unique_cards = list(playable_by_card_index.values())
    unique_damage_sum = sum(numeric_card_value(card, "base_damage") for card in unique_cards)
    unique_block_sum = sum(numeric_card_value(card, "base_block") for card in unique_cards)
    try:
        energy_int = int(energy)
    except (TypeError, ValueError):
        energy_int = 0
    return {
        "schema_version": "decision_context_features_v0",
        "legal_candidate_count": len(candidates),
        "end_turn_legal": end_turn_legal,
        "playable_candidate_count": playable_candidate_count,
        "playable_unique_card_count": len(unique_cards),
        "playable_attack_candidate_count": attack_candidate_count,
        "playable_block_candidate_count": block_candidate_count,
        "playable_draw_candidate_count": draw_candidate_count,
        "playable_debuff_candidate_count": debuff_candidate_count,
        "playable_exhaust_candidate_count": exhaust_candidate_count,
        "playable_setup_candidate_count": setup_candidate_count,
        "playable_zero_cost_candidate_count": zero_cost_candidate_count,
        "playable_one_cost_candidate_count": one_cost_candidate_count,
        "playable_two_plus_cost_candidate_count": two_plus_cost_candidate_count,
        "playable_candidate_damage_sum": candidate_damage_sum,
        "playable_candidate_block_sum": candidate_block_sum,
        "playable_candidate_max_damage": candidate_max_damage,
        "playable_candidate_max_block": candidate_max_block,
        "playable_unique_damage_sum": unique_damage_sum,
        "playable_unique_block_sum": unique_block_sum,
        "end_turn_with_playable_cards": end_turn_legal and playable_candidate_count > 0,
        "end_turn_with_unspent_energy": end_turn_legal and energy_int > 0,
        "incoming_minus_current_block": max(
            0,
            int(combat.get("visible_incoming_damage") or 0)
            - int(combat.get("player_block") or 0),
        ),
    }


def observation_features(trace: dict[str, Any]) -> dict[str, Any]:
    obs = observation_payload(trace)
    combat = obs.get("combat") or {}
    deck = obs.get("deck") or {}
    return {
        "act": obs.get("act"),
        "floor": obs.get("floor"),
        "current_hp": obs.get("current_hp"),
        "max_hp": obs.get("max_hp"),
        "gold": obs.get("gold"),
        "deck_size": obs.get("deck_size"),
        "relic_count": obs.get("relic_count"),
        "filled_potion_slots": obs.get("filled_potion_slots"),
        "combat_turn_count": combat.get("turn_count"),
        "combat_energy": combat.get("energy"),
        "visible_incoming_damage": combat.get("visible_incoming_damage"),
        "player_block": combat.get("player_block"),
        "alive_monster_count": combat.get("alive_monster_count"),
        "total_monster_hp": combat.get("total_monster_hp"),
        "hand_count": combat.get("hand_count"),
        "draw_count": combat.get("draw_count"),
        "discard_count": combat.get("discard_count"),
        "exhaust_count": combat.get("exhaust_count"),
        "deck_attack_count": deck.get("attack_count"),
        "deck_skill_count": deck.get("skill_count"),
        "deck_power_count": deck.get("power_count"),
        "deck_damage_card_count": deck.get("damage_card_count"),
        "deck_block_card_count": deck.get("block_card_count"),
        "deck_draw_card_count": deck.get("draw_card_count"),
        "deck_exhaust_card_count": deck.get("exhaust_card_count"),
        "deck_scaling_card_count": deck.get("scaling_card_count"),
    }


def safe_number(value: Any, default: float = 0.0) -> float:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return default
    return number if number == number else default


def action_key_kind(action_key: Any) -> str:
    if not isinstance(action_key, str):
        return "unknown"
    if action_key == "combat/end_turn":
        return "end_turn"
    if action_key.startswith("combat/play_card/"):
        return "play_card"
    if action_key.startswith("combat/use_potion/"):
        return "potion"
    return "other"


def first_public_summary(trace: dict[str, Any]) -> dict[str, Any]:
    summaries = trace.get("public_summaries") or []
    return summaries[0] if summaries and isinstance(summaries[0], dict) else {}


def evidence_features(trace: dict[str, Any]) -> dict[str, Any]:
    """Public-safe one-step evidence features.

    This deliberately uses only the first public transition summary after the
    forced branch action. It must stay shorter than the combat-end label horizon.
    """

    first = first_public_summary(trace)
    info = first.get("info") or {}
    env_info = info.get("env_info") or {}
    reward = first.get("reward") or {}
    observation = observation_features(trace)
    current_hp = safe_number(observation.get("current_hp"))
    after_hp = safe_number(env_info.get("hp"), current_hp)
    action_key = first.get("chosen_action_key") or info.get("chosen_action_key")
    return {
        "schema_version": "one_step_public_evidence_features_v0",
        "evidence_scope": "one_step_public_transition",
        "evidence_horizon_lt_label_horizon": True,
        "one_step_available": bool(first),
        "one_step_action_kind": action_key_kind(action_key),
        "one_step_decision_type_after": (first.get("decision_id") or {}).get("decision_type"),
        "one_step_reward": reward.get("scalar_reward"),
        "one_step_hp": env_info.get("hp"),
        "one_step_hp_delta_from_observation": after_hp - current_hp,
        "one_step_combat_win_count": env_info.get("combat_win_count"),
        "one_step_floor": env_info.get("floor"),
        "one_step_legal_action_count": info.get("legal_action_count"),
        "one_step_forced_engine_ticks": env_info.get("forced_engine_ticks"),
        "one_step_result": env_info.get("result"),
        "one_step_terminal_reason": env_info.get("terminal_reason"),
        "one_step_terminated": first.get("terminated"),
        "one_step_truncated": first.get("truncated"),
        "one_step_state_hash_present": isinstance(first.get("state_hash"), str),
    }


def is_complete_combat_end_trace(trace: dict[str, Any]) -> bool:
    outcome = trace.get("outcome") or {}
    return (
        trace.get("schema_version") == "branch_trace_v1"
        and trace.get("trainable_as_action_label") is False
        and outcome.get("boundary_requested") == "combat_end"
        and outcome.get("boundary_reached") is True
        and outcome.get("outcome_censored") is False
        and outcome.get("truncated") is False
        and (trace.get("redaction_report") or {}).get("model_input_uses_public_observation")
        is True
        and (trace.get("redaction_report") or {}).get("hidden_state_in_observation") is False
        and (trace.get("redaction_report") or {}).get("hidden_future_in_public_summary") is False
    )


def trace_skip_reason(trace: dict[str, Any]) -> str | None:
    outcome = trace.get("outcome") or {}
    if trace.get("trainable_as_action_label") is not False:
        return "action_label_trace"
    if outcome.get("boundary_requested") != "combat_end":
        return "not_combat_end"
    if outcome.get("boundary_reached") is not True:
        return "combat_end_not_reached"
    if outcome.get("outcome_censored"):
        return "outcome_censored"
    if outcome.get("truncated"):
        return "truncated"
    redaction = trace.get("redaction_report") or {}
    if redaction.get("model_input_uses_public_observation") is not True:
        return "model_input_not_public"
    if redaction.get("hidden_state_in_observation") or redaction.get(
        "hidden_future_in_public_summary"
    ):
        return "redaction_violation"
    return None


def branch_row(record: dict[str, Any], trace: dict[str, Any]) -> dict[str, Any]:
    decision = trace.get("decision_id") or {}
    outcome = trace.get("outcome") or {}
    candidate = action_candidate(trace)
    return {
        "schema_version": BRANCH_ROW_SCHEMA,
        "source_schema_version": trace.get("schema_version"),
        "trainable_role": "branch_value_risk",
        "trainable_as_action_label": False,
        "episode_seed": record.get("seed"),
        "episode_step": record.get("episode_step"),
        "episode_id": trace.get("episode_id"),
        "decision_id": decision,
        "decision_type": decision.get("decision_type"),
        "state_hash_before": trace.get("state_hash_before"),
        "behavior_action_id": record.get("behavior_action_id"),
        "behavior_action_key": record.get("behavior_action_key"),
        "scenario_seed_id": trace.get("scenario_seed_id"),
        "rng_state_before_hash": trace.get("rng_state_before_hash"),
        "rng_state_after_hash": trace.get("rng_state_after_hash"),
        "rng_consumed": trace.get("rng_consumed"),
        "branch_id": trace.get("branch_id"),
        "forced_prefix": trace.get("forced_prefix"),
        "forced_action_keys": trace.get("forced_action_keys"),
        "continuation_policy": trace.get("continuation_policy"),
        "horizon": trace.get("horizon"),
        "candidate": candidate_features(candidate),
        "observation_features": observation_features(trace),
        "decision_context_features": decision_context_features(trace),
        "evidence_features": evidence_features(trace),
        "targets": {
            "total_reward": outcome.get("total_reward"),
            "hp_delta": outcome.get("hp_delta"),
            "final_hp": outcome.get("hp"),
            "max_hp": outcome.get("max_hp"),
            "combat_win_delta": outcome.get("combat_win_delta"),
            "floor_delta": outcome.get("floor_delta"),
            "death": outcome.get("result") == "defeat",
            "boundary_reached": outcome.get("boundary_reached"),
            "outcome_censored": outcome.get("outcome_censored"),
            "step_count": outcome.get("step_count"),
        },
        "label_policy": {
            "dataset": "branch_value_risk_dataset_v0",
            "action_label": False,
            "source": "complete_combat_end_branch",
        },
    }


def comparison_skip_reason(
    comparison: dict[str, Any],
    left: dict[str, Any] | None,
    right: dict[str, Any] | None,
    *,
    allow_rng_diverged: bool,
) -> str | None:
    if left is None or right is None:
        return "missing_branch_trace"
    if comparison.get("pairing_valid") is not True:
        return "invalid_pairing"
    if comparison.get("paired_validity_status") != "valid_shared_initial_scenario":
        return "invalid_pairing_status"
    if comparison.get("trainable_role") and "action" in str(comparison.get("trainable_role")):
        return "action_like_role"
    if not is_complete_combat_end_trace(left):
        return f"left_{trace_skip_reason(left) or 'not_complete'}"
    if not is_complete_combat_end_trace(right):
        return f"right_{trace_skip_reason(right) or 'not_complete'}"
    if not allow_rng_diverged and comparison.get("rng_diverged") is True:
        return "rng_diverged"
    return None


def pair_row(
    record: dict[str, Any],
    comparison: dict[str, Any],
    left: dict[str, Any],
    right: dict[str, Any],
) -> dict[str, Any]:
    return {
        "schema_version": PAIR_ROW_SCHEMA,
        "source_schema_version": comparison.get("schema_version"),
        "trainable_role": "pairwise_outcome_diff",
        "trainable_as_action_label": False,
        "episode_seed": record.get("seed"),
        "episode_step": record.get("episode_step"),
        "decision_id": comparison.get("decision_id"),
        "state_hash_before": left.get("state_hash_before"),
        "scenario_seed_id": comparison.get("scenario_seed_id"),
        "pairing": {
            "pairing_schema_version": comparison.get("pairing_schema_version"),
            "pairing_mode": comparison.get("pairing_mode"),
            "paired_seed_id": comparison.get("paired_seed_id"),
            "paired_validity_status": comparison.get("paired_validity_status"),
            "common_random_policy": comparison.get("common_random_policy"),
            "rng_diverged": comparison.get("rng_diverged"),
            "rng_divergence_reason": comparison.get("rng_divergence_reason"),
        },
        "left": {
            "branch_id": left.get("branch_id"),
            "forced_prefix": left.get("forced_prefix"),
            "forced_action_keys": left.get("forced_action_keys"),
            "candidate": candidate_features(action_candidate(left)),
            "targets": (branch_row(record, left).get("targets") or {}),
            "decision_context_features": decision_context_features(left),
            "evidence_features": evidence_features(left),
        },
        "right": {
            "branch_id": right.get("branch_id"),
            "forced_prefix": right.get("forced_prefix"),
            "forced_action_keys": right.get("forced_action_keys"),
            "candidate": candidate_features(action_candidate(right)),
            "targets": (branch_row(record, right).get("targets") or {}),
            "decision_context_features": decision_context_features(right),
            "evidence_features": evidence_features(right),
        },
        "outcome_diff": comparison.get("outcome_diff"),
        "label_policy": {
            "dataset": "branch_value_risk_dataset_v0",
            "action_label": False,
            "source": "ordered_pair_outcome_diff",
            "preference_label": False,
        },
    }


def export_records(
    inputs: list[Path],
    *,
    allow_rng_diverged_pairs: bool,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], dict[str, Any]]:
    branch_rows: list[dict[str, Any]] = []
    pair_rows: list[dict[str, Any]] = []
    summary: dict[str, Any] = {
        "schema_version": "branch_value_risk_dataset_export_summary_v0",
        "input_record_count": 0,
        "input_trace_count": 0,
        "input_comparison_count": 0,
        "branch_row_count": 0,
        "pair_row_count": 0,
        "skipped_trace_reasons": {},
        "skipped_comparison_reasons": {},
        "branch_action_kind_counts": {},
        "pair_action_kind_counts": {},
        "pair_rng_diverged_count": 0,
        "pair_rng_aligned_count": 0,
    }
    for record in iter_jsonl(inputs):
        summary["input_record_count"] += 1
        batch = record.get("branch_trace_batch") or {}
        traces = batch.get("traces") or []
        comparisons = batch.get("comparisons") or []
        summary["input_trace_count"] += len(traces)
        summary["input_comparison_count"] += len(comparisons)
        traces_by_id = {
            trace.get("branch_id"): trace
            for trace in traces
            if isinstance(trace.get("branch_id"), str)
        }
        for trace in traces:
            skip = trace_skip_reason(trace)
            if skip is not None:
                bump(summary["skipped_trace_reasons"], skip)
                continue
            row = branch_row(record, trace)
            branch_rows.append(row)
            bump(
                summary["branch_action_kind_counts"],
                str((row.get("candidate") or {}).get("action_kind") or "unknown"),
            )
        for comparison in comparisons:
            left = traces_by_id.get(comparison.get("left_branch_id"))
            right = traces_by_id.get(comparison.get("right_branch_id"))
            skip = comparison_skip_reason(
                comparison,
                left,
                right,
                allow_rng_diverged=allow_rng_diverged_pairs,
            )
            if skip is not None:
                bump(summary["skipped_comparison_reasons"], skip)
                continue
            assert left is not None and right is not None
            row = pair_row(record, comparison, left, right)
            pair_rows.append(row)
            left_kind = ((row.get("left") or {}).get("candidate") or {}).get("action_kind")
            right_kind = ((row.get("right") or {}).get("candidate") or {}).get("action_kind")
            bump(summary["pair_action_kind_counts"], f"{left_kind}->{right_kind}")
            if (row.get("pairing") or {}).get("rng_diverged") is True:
                summary["pair_rng_diverged_count"] += 1
            else:
                summary["pair_rng_aligned_count"] += 1
    summary["branch_row_count"] = len(branch_rows)
    summary["pair_row_count"] = len(pair_rows)
    summary["allow_rng_diverged_pairs"] = allow_rng_diverged_pairs
    return branch_rows, pair_rows, summary


def assert_no_action_label_leak(rows: list[dict[str, Any]], *, row_kind: str) -> None:
    forbidden = {"winner", "preferred", "preferred_action", "selected_action", "teacher_choice"}
    for index, row in enumerate(rows):
        serialized = json.dumps(row, separators=(",", ":"), sort_keys=True)
        if row.get("trainable_as_action_label") is not False:
            raise ValueError(f"{row_kind} row {index} is action-label-like")
        if row.get("label_policy", {}).get("action_label") is not False:
            raise ValueError(f"{row_kind} row {index} label_policy action_label is not false")
        for key in forbidden:
            if f'"{key}"' in serialized:
                raise ValueError(f"{row_kind} row {index} contains forbidden key {key}")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, separators=(",", ":")) + "\n")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inputs", nargs="+", type=Path, required=True)
    parser.add_argument("--branch-out", type=Path, required=True)
    parser.add_argument("--pair-out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path)
    parser.add_argument(
        "--allow-rng-diverged-pairs",
        action="store_true",
        help="Include valid complete-combat-end pairs even if branch RNG streams diverged.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    branch_rows, pair_rows, summary = export_records(
        args.inputs,
        allow_rng_diverged_pairs=args.allow_rng_diverged_pairs,
    )
    assert_no_action_label_leak(branch_rows, row_kind="branch")
    assert_no_action_label_leak(pair_rows, row_kind="pair")
    write_jsonl(args.branch_out, branch_rows)
    write_jsonl(args.pair_out, pair_rows)
    summary.update(
        {
            "inputs": [str(path) for path in args.inputs],
            "branch_out": str(args.branch_out),
            "pair_out": str(args.pair_out),
        }
    )
    summary_out = args.summary_out or args.branch_out.with_suffix(".summary.json")
    summary_out.parent.mkdir(parents=True, exist_ok=True)
    summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
