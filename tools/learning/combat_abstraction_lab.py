#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import random
import time
from collections import Counter, defaultdict
from pathlib import Path
from statistics import mean
from typing import Any

from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from full_run_counterfactual_lab import (
    compare_response_to_trace_step,
    load_trace,
    observation_from_response,
    outcome_delta,
    parse_branch_indices,
    replay_to_step,
    step_continuation,
    summarize_response,
    summarize_state_response,
    trace_step,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Build combat-root abstractions for one full-run combat decision: candidate signatures, "
            "short-horizon outcome estimates, ranking, and near-equivalence groups."
        )
    )
    parser.add_argument("--trace-file", type=Path, required=True)
    parser.add_argument("--step-index", type=int, required=True)
    parser.add_argument("--seed", type=int)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=5000)
    parser.add_argument("--continuation-policy", default="rule_baseline_v0", choices=["rule_baseline_v0", "random_masked"])
    parser.add_argument("--horizon", type=int, default=12)
    parser.add_argument("--samples", type=int, default=1)
    parser.add_argument("--branch-indices", default="all")
    parser.add_argument("--max-branches", type=int, default=24)
    parser.add_argument("--driver-binary", type=Path)
    parser.add_argument("--allow-replay-mismatch", action="store_true")
    parser.add_argument("--out", type=Path, default=REPO_ROOT / "tools" / "artifacts" / "combat_abstraction_lab" / "combat_abstraction_report.json")
    parser.add_argument("--rows-out", type=Path)
    return parser.parse_args()


def combat_state(obs: dict[str, Any]) -> dict[str, Any]:
    return obs.get("combat") or {}


def unblocked_damage(obs: dict[str, Any]) -> int:
    combat = combat_state(obs)
    return max(int(combat.get("visible_incoming_damage") or 0) - int(combat.get("player_block") or 0), 0)


def hp(obs: dict[str, Any]) -> int:
    return int(obs.get("current_hp") or combat_state(obs).get("player_hp") or 0)


def monster_hp(obs: dict[str, Any]) -> int:
    return int(combat_state(obs).get("total_monster_hp") or 0)


def summarize_combat_obs(obs: dict[str, Any]) -> dict[str, Any]:
    combat = combat_state(obs)
    return {
        "decision_type": obs.get("decision_type"),
        "engine_state": obs.get("engine_state"),
        "hp": hp(obs),
        "max_hp": int(obs.get("max_hp") or 0),
        "block": int(combat.get("player_block") or 0),
        "energy": int(combat.get("energy") or 0),
        "turn_count": int(combat.get("turn_count") or 0),
        "hand_count": int(combat.get("hand_count") or 0),
        "draw_count": int(combat.get("draw_count") or 0),
        "discard_count": int(combat.get("discard_count") or 0),
        "exhaust_count": int(combat.get("exhaust_count") or 0),
        "alive_monster_count": int(combat.get("alive_monster_count") or 0),
        "monster_hp": monster_hp(obs),
        "incoming": int(combat.get("visible_incoming_damage") or 0),
        "unblocked": unblocked_damage(obs),
    }


def candidate_tags(candidate: dict[str, Any]) -> list[str]:
    action = candidate.get("action") or {}
    card = candidate.get("card") or {}
    tags: list[str] = []
    action_type = str(action.get("type") or "")
    if action_type == "end_turn":
        tags.append("end_turn")
    if action_type == "play_card":
        tags.append("play_card")
    if action_type == "use_potion":
        tags.append("use_potion")
    if isinstance(card, dict) and card:
        card_type = int(card.get("card_type_id") or 0)
        if card_type == 1:
            tags.append("attack")
        elif card_type == 2:
            tags.append("skill")
        elif card_type == 3:
            tags.append("power")
        if int(card.get("base_damage") or 0) > 0 or int(card.get("upgraded_damage") or 0) > 0:
            tags.append("damage")
        if int(card.get("base_block") or 0) > 0 or int(card.get("upgraded_block") or 0) > 0:
            tags.append("block")
        if card.get("aoe"):
            tags.append("aoe")
        if card.get("draws_cards"):
            tags.append("draw")
        if card.get("gains_energy"):
            tags.append("energy")
        if card.get("scaling_piece"):
            tags.append("scaling")
        if card.get("exhaust"):
            tags.append("exhaust")
        if int(card.get("cost") or 0) == 0:
            tags.append("zero_cost")
    return sorted(set(tags))


def branch_once(args: argparse.Namespace, trace: dict[str, Any], target_step: dict[str, Any], candidate_index: int, sample_index: int) -> dict[str, Any]:
    driver, target_response, prefix_checks = replay_to_step(args=args, trace=trace, target_step_index=args.step_index)
    rng = random.Random((int((trace.get("summary") or {}).get("seed") or 0) * 1009) + candidate_index * 101 + sample_index)
    try:
        target_check = compare_response_to_trace_step(target_response, target_step)
        if target_check["status"] != "ok" and not args.allow_replay_mismatch:
            raise RuntimeError(f"target replay mismatch: {target_check}")
        start_obs = observation_from_response(target_response)
        candidates = (target_response.get("payload") or {}).get("action_candidates") or []
        candidate = candidates[candidate_index]

        response = driver.request({"cmd": "step", "action_index": candidate_index})
        reward_total = float(response.get("reward") or 0.0)
        immediate = summarize_response(response)
        immediate_combat = summarize_combat_obs(observation_from_response(response))
        steps_taken = 1
        if not bool(response.get("done")):
            for _ in range(max(int(args.horizon) - 1, 0)):
                response = step_continuation(driver, response, args.continuation_policy, rng)
                reward_total += float(response.get("reward") or 0.0)
                steps_taken += 1
                if bool(response.get("done")):
                    break
        end = summarize_response(response)
        end_obs = observation_from_response(response)
        return {
            "sample_index": sample_index,
            "prefix_status": Counter(check["status"] for check in prefix_checks),
            "target_replay_check": target_check,
            "start_combat": summarize_combat_obs(start_obs),
            "immediate": immediate,
            "immediate_combat": immediate_combat,
            "end": end,
            "end_combat": summarize_combat_obs(end_obs),
            "delta": outcome_delta(summarize_state_response(target_response), end),
            "reward_total": reward_total,
            "steps_taken": steps_taken,
            "candidate_key": str(candidate.get("action_key") or ""),
            "candidate_action": candidate.get("action") or {},
            "candidate_card": candidate.get("card"),
        }
    finally:
        driver.close()


def estimate_candidate(args: argparse.Namespace, trace: dict[str, Any], target_step: dict[str, Any], candidate_index: int) -> dict[str, Any]:
    samples = max(int(args.samples), 1)
    if args.continuation_policy == "rule_baseline_v0":
        samples = 1
    sample_rows = [branch_once(args, trace, target_step, candidate_index, sample_index) for sample_index in range(samples)]
    candidate = sample_rows[0]
    start = candidate["start_combat"]
    card = candidate.get("candidate_card") or {}
    immediate_obs = candidate.get("immediate_combat") or {}
    estimates = aggregate_samples(sample_rows)
    tags = candidate_tags({"action": candidate["candidate_action"], "card": candidate.get("candidate_card")})
    abstraction = classify_candidate(start, immediate_obs, estimates, card, tags)
    return {
        "candidate_index": candidate_index,
        "candidate_key": candidate["candidate_key"],
        "candidate_action": candidate["candidate_action"],
        "candidate_card": candidate.get("candidate_card"),
        "candidate_tags": tags,
        "abstraction": abstraction,
        "estimates": estimates,
        "samples": sample_rows,
    }


def include_required_branch(indices: list[int], required_index: int, candidate_count: int, max_branches: int) -> list[int]:
    if required_index < 0 or required_index >= candidate_count or required_index in indices:
        return indices
    if max_branches > 0 and len(indices) >= max_branches:
        indices = indices[: max_branches - 1]
    return sorted(set(indices + [required_index]))


def pressure_class(start: dict[str, Any]) -> str:
    incoming = int(start.get("incoming") or 0)
    unblocked = int(start.get("unblocked") or 0)
    current_hp = int(start.get("hp") or 0)
    if incoming <= 0:
        return "no_attack"
    if unblocked <= 0:
        return "blocked_attack"
    if current_hp > 0 and unblocked >= current_hp:
        return "lethal_pressure"
    if current_hp > 0 and unblocked >= max(current_hp // 2, 1):
        return "high_pressure"
    if unblocked >= 6:
        return "medium_pressure"
    return "chip_pressure"


def root_plan(start: dict[str, Any], immediate: dict[str, Any], tags: list[str]) -> dict[str, Any]:
    tag_set = set(tags)
    start_unblocked = int(start.get("unblocked") or 0)
    immediate_unblocked = int(immediate.get("unblocked") or start_unblocked)
    start_monster_hp = int(start.get("monster_hp") or 0)
    immediate_monster_hp = int(immediate.get("monster_hp") or 0)
    start_block = int(start.get("block") or 0)
    immediate_block = int(immediate.get("block") or start_block)
    start_energy = int(start.get("energy") or 0)
    immediate_energy = int(immediate.get("energy") or start_energy)
    unblocked_reduction = max(start_unblocked - immediate_unblocked, 0)
    damage_delta = max(start_monster_hp - immediate_monster_hp, 0)
    block_delta = max(immediate_block - start_block, 0)
    energy_spent = max(start_energy - immediate_energy, 0)
    pressure = pressure_class(start)
    under_attack = pressure not in {"no_attack", "blocked_attack"}

    if "end_turn" in tag_set:
        role = "end_turn"
    elif start_monster_hp > 0 and immediate_monster_hp <= 0:
        role = "lethal"
    elif under_attack and unblocked_reduction >= start_unblocked and start_unblocked > 0:
        role = "full_defense"
    elif under_attack and unblocked_reduction > 0 and damage_delta > 0:
        role = "mixed_partial_defense"
    elif under_attack and unblocked_reduction > 0:
        role = "partial_defense"
    elif under_attack and damage_delta > 0:
        role = "damage_under_pressure"
    elif under_attack and ("power" in tag_set or "scaling" in tag_set):
        role = "scaling_under_pressure"
    elif under_attack and "use_potion" in tag_set:
        role = "emergency_resource"
    elif under_attack:
        role = "ignores_pressure"
    elif "block" in tag_set and damage_delta == 0:
        role = "defense_without_pressure"
    elif "power" in tag_set or "scaling" in tag_set:
        role = "setup_window"
    elif damage_delta > 0:
        role = "damage_window"
    else:
        role = "other_window"

    if role == "lethal":
        fit = "decisive"
    elif role == "full_defense":
        fit = "covers_attack"
    elif role in {"mixed_partial_defense", "partial_defense"}:
        fit = "reduces_attack"
    elif role == "damage_under_pressure":
        fit = "trades_hp_for_progress"
    elif role in {"scaling_under_pressure", "ignores_pressure"}:
        fit = "ignores_attack"
    elif role == "emergency_resource":
        fit = "external_resource"
    elif role in {"setup_window", "damage_window"}:
        fit = "uses_window"
    elif role == "defense_without_pressure":
        fit = "wastes_window"
    else:
        fit = "neutral"

    return {
        "pressure_class": pressure,
        "root_block_need": start_unblocked,
        "root_unblocked_after": immediate_unblocked,
        "root_unblocked_reduction": unblocked_reduction,
        "root_damage_delta": damage_delta,
        "root_block_delta": block_delta,
        "root_energy_spent": energy_spent,
        "root_plan_role": role,
        "root_plan_fit": fit,
    }


def aggregate_samples(samples: list[dict[str, Any]]) -> dict[str, Any]:
    defeat = []
    combat_win = []
    floor_delta = []
    hp_delta = []
    end_hp = []
    monster_hp_end = []
    reward_total = []
    for sample in samples:
        end = sample.get("end") or {}
        delta = sample.get("delta") or {}
        end_combat = sample.get("end_combat") or {}
        result = str(end.get("result") or "")
        terminal_reason = str(end.get("terminal_reason") or "")
        defeat.append(result == "defeat" or terminal_reason in {"engine_rejected_action", "no_progress_loop"})
        combat_win.append(int(delta.get("combat_win_delta") or 0) > 0)
        floor_delta.append(float(delta.get("floor_delta") or 0))
        hp_delta.append(float(delta.get("hp_delta") or 0))
        end_hp.append(float(end.get("current_hp") or 0))
        monster_hp_end.append(float(end_combat.get("monster_hp") or 0))
        reward_total.append(float(sample.get("reward_total") or 0))
    n = max(len(samples), 1)
    return {
        "sample_count": len(samples),
        "survive_prob": 1.0 - sum(defeat) / n,
        "defeat_prob": sum(defeat) / n,
        "combat_win_prob": sum(combat_win) / n,
        "expected_floor_delta": mean(floor_delta) if floor_delta else 0.0,
        "expected_hp_delta": mean(hp_delta) if hp_delta else 0.0,
        "expected_end_hp": mean(end_hp) if end_hp else 0.0,
        "expected_end_monster_hp": mean(monster_hp_end) if monster_hp_end else 0.0,
        "expected_reward_total": mean(reward_total) if reward_total else 0.0,
    }


def classify_candidate(
    start: dict[str, Any],
    immediate: dict[str, Any],
    estimates: dict[str, Any],
    card: dict[str, Any],
    tags: list[str],
) -> dict[str, Any]:
    survive_prob = float(estimates.get("survive_prob") or 0.0)
    combat_win_prob = float(estimates.get("combat_win_prob") or 0.0)
    expected_end_hp = float(estimates.get("expected_end_hp") or 0.0)
    immediate_unblocked = int(immediate.get("unblocked") or start.get("unblocked") or 0)
    immediate_hp = int(immediate.get("hp") or start.get("hp") or 0)

    if survive_prob <= 0.0:
        survival_class = "forced_loss"
    elif survive_prob < 1.0:
        survival_class = "stochastic_loss_risk"
    elif immediate_unblocked >= immediate_hp and immediate_hp > 0:
        survival_class = "severe_risk"
    elif immediate_unblocked >= max(immediate_hp // 2, 1):
        survival_class = "risky"
    else:
        survival_class = "stable"

    if combat_win_prob >= 1.0:
        kill_clock = "wins_within_horizon"
    elif combat_win_prob > 0.0:
        kill_clock = "can_win_within_horizon"
    elif float(estimates.get("expected_end_monster_hp") or 0.0) < float(start.get("monster_hp") or 0):
        kill_clock = "progress_no_kill"
    else:
        kill_clock = "no_progress"

    if immediate_unblocked == 0:
        risk_bucket = "no_unblocked"
    elif immediate_unblocked <= 5:
        risk_bucket = "chip"
    elif immediate_unblocked <= 12:
        risk_bucket = "medium"
    else:
        risk_bucket = "high"

    role = "other"
    tag_set = set(tags)
    if "end_turn" in tag_set:
        role = "end_turn"
    elif "block" in tag_set and "damage" in tag_set:
        role = "attack_block"
    elif "block" in tag_set:
        role = "defense"
    elif "damage" in tag_set:
        role = "attack"
    elif "scaling" in tag_set:
        role = "scaling"

    return {
        "survival_class": survival_class,
        "kill_clock": kill_clock,
        "risk_bucket": risk_bucket,
        "role": role,
        "expected_end_hp_bucket5": int(expected_end_hp) // 5,
        **root_plan(start, immediate, tags),
    }


def rank_key(row: dict[str, Any]) -> tuple[float, float, float, float, float]:
    estimates = row["estimates"]
    return (
        float(estimates.get("survive_prob") or 0.0),
        float(estimates.get("combat_win_prob") or 0.0),
        float(estimates.get("expected_end_hp") or 0.0),
        -float(estimates.get("expected_end_monster_hp") or 0.0),
        float(estimates.get("expected_reward_total") or 0.0),
    )


def equivalence_signature(row: dict[str, Any]) -> str:
    abstraction = row["abstraction"]
    estimates = row["estimates"]
    return "|".join(
        [
            abstraction["survival_class"],
            abstraction["kill_clock"],
            abstraction["risk_bucket"],
            f"hp5:{abstraction['expected_end_hp_bucket5']}",
            f"win:{round(float(estimates.get('combat_win_prob') or 0.0), 1)}",
        ]
    )


def main() -> None:
    args = parse_args()
    trace = load_trace(args.trace_file)
    target = trace_step(trace, args.step_index)
    if str(target.get("decision_type") or "") != "combat":
        raise SystemExit(f"step {args.step_index} is {target.get('decision_type')}, expected combat")

    driver, target_response, prefix_checks = replay_to_step(args=args, trace=trace, target_step_index=args.step_index)
    try:
        target_check = compare_response_to_trace_step(target_response, target)
        if target_check["status"] != "ok" and not args.allow_replay_mismatch:
            raise RuntimeError(f"target replay mismatch: {target_check}")
        candidates = (target_response.get("payload") or {}).get("action_candidates") or []
        chosen_index = int(target.get("chosen_action_index") or 0)
        branch_indices = include_required_branch(
            parse_branch_indices(args.branch_indices, len(candidates), args.max_branches),
            chosen_index,
            len(candidates),
            args.max_branches,
        )
        start_obs = observation_from_response(target_response)
        target_summary = {
            "trace_step": {
                "step_index": int(target.get("step_index") or 0),
                "decision_type": target.get("decision_type"),
                "floor": target.get("floor"),
                "act": target.get("act"),
                "chosen_action_index": target.get("chosen_action_index"),
                "chosen_action_key": target.get("chosen_action_key"),
            },
            "combat": summarize_combat_obs(start_obs),
            "candidate_count": len(candidates),
            "branch_indices": branch_indices,
            "target_replay_check": target_check,
            "prefix_replay_status": Counter(check["status"] for check in prefix_checks),
        }
    finally:
        driver.close()

    started = time.perf_counter()
    candidates_out = [estimate_candidate(args, trace, target, index) for index in branch_indices]
    elapsed = time.perf_counter() - started
    ranked = sorted(candidates_out, key=rank_key, reverse=True)
    rank_by_index = {row["candidate_index"]: rank for rank, row in enumerate(ranked, start=1)}
    groups: dict[str, list[int]] = defaultdict(list)
    rows = []
    for row in candidates_out:
        signature = equivalence_signature(row)
        groups[signature].append(row["candidate_index"])
        flat = {
            "candidate_index": row["candidate_index"],
            "candidate_key": row["candidate_key"],
            "candidate_card_id": (row.get("candidate_card") or {}).get("card_id"),
            "candidate_tags": row["candidate_tags"],
            "rank": rank_by_index[row["candidate_index"]],
            "equivalence_signature": signature,
            **row["abstraction"],
            **row["estimates"],
        }
        rows.append(flat)
    chosen_index = int(target.get("chosen_action_index") or 0)
    report = {
        "schema_version": "combat_abstraction_lab_v0",
        "source": {
            "trace_file": str(args.trace_file),
            "trace_observation_schema_version": trace.get("observation_schema_version"),
            "trace_action_schema_version": trace.get("action_schema_version"),
        },
        "config": {
            "seed": int(args.seed if args.seed is not None else (trace.get("summary") or {}).get("seed") or 0),
            "continuation_policy": args.continuation_policy,
            "horizon": args.horizon,
            "samples": 1 if args.continuation_policy == "rule_baseline_v0" else args.samples,
            "max_branches": args.max_branches,
        },
        "target": target_summary,
        "ranking": [
            {
                "rank": rank_by_index[row["candidate_index"]],
                "candidate_index": row["candidate_index"],
                "candidate_key": row["candidate_key"],
                "candidate_card_id": (row.get("candidate_card") or {}).get("card_id"),
                "equivalence_signature": equivalence_signature(row),
                "estimates": row["estimates"],
                "abstraction": row["abstraction"],
            }
            for row in ranked
        ],
        "equivalence_groups": [
            {"signature": signature, "candidate_indices": indices}
            for signature, indices in sorted(groups.items())
        ],
        "chosen_rank": rank_by_index.get(chosen_index),
        "chosen_equivalence_signature": next(
            (equivalence_signature(row) for row in candidates_out if row["candidate_index"] == chosen_index),
            None,
        ),
        "candidates": candidates_out,
        "summary": {
            "elapsed_seconds": elapsed,
            "candidate_count": len(candidates_out),
            "equivalence_group_count": len(groups),
            "chosen_was_top_rank": rank_by_index.get(chosen_index) == 1,
        },
    }
    write_json(args.out, report)
    rows_out = args.rows_out or args.out.with_suffix(".rows.jsonl")
    write_jsonl(rows_out, rows)
    print(json.dumps(report["summary"] | {"chosen_rank": report["chosen_rank"]}, indent=2, ensure_ascii=False))
    print(f"wrote {args.out}")
    print(f"wrote {rows_out}")


if __name__ == "__main__":
    main()
