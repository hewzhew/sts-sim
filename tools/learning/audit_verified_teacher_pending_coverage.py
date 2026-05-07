#!/usr/bin/env python3
"""Audit counterfactual pending DecisionState coverage for the verified teacher.

This does not train a model. It walks rule-baseline trajectories, asks the Rust
driver to force root combat candidates one step, and records pending combat
DecisionStates that appear as separate groups.
"""
from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any

from return_q_common import FullRunDriver, write_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--groups-out", type=Path)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--episodes", type=int, default=20)
    parser.add_argument("--seed-start", type=int, default=98100)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=160)
    parser.add_argument("--candidate-scope", default="controlled_v1", choices=["all", "controlled_v0", "controlled_v1"])
    parser.add_argument("--horizon-decisions", type=int, default=8)
    parser.add_argument(
        "--horizon-mode",
        default="fixed_decisions",
        choices=["fixed_decisions", "adaptive_next_player_turn_v1", "adaptive_payoff_window_v1"],
    )
    parser.add_argument("--oracle-margin", type=float, default=1.0)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--continuation-policy", default="rule_baseline_v0")
    parser.add_argument("--max-roots", type=int, default=64)
    parser.add_argument("--max-groups-per-state", type=int, default=16)
    parser.add_argument("--parallelism", type=int, default=0)
    parser.add_argument("--include-observation", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    summary = {
        "schema_version": "verified_teacher_pending_coverage_audit_v0",
        "episodes": args.episodes,
        "seed_start": args.seed_start,
        "seed_step": args.seed_step,
        "max_steps": args.max_steps,
        "candidate_scope": args.candidate_scope,
        "horizon_decisions": args.horizon_decisions,
        "horizon_mode": args.horizon_mode,
        "oracle_margin": args.oracle_margin,
        "combat_decision_count": 0,
        "normal_combat_decision_count": 0,
        "trajectory_pending_decision_count": 0,
        "counterfactual_pending_group_count": 0,
        "counterfactual_positive_group_count": 0,
        "counterfactual_candidate_rows": 0,
        "pending_exact_dedup_count": 0,
        "root_scoped_candidate_count": 0,
        "roots_considered": 0,
        "decision_type_counts": {},
        "trajectory_pending_type_counts": {},
        "counterfactual_pending_type_counts": {},
        "parent_action_head_counts": {},
        "parent_card_counts": {},
        "best_adv_bucket_counts": {},
    }
    counters = {
        "decision_type_counts": Counter(),
        "trajectory_pending_type_counts": Counter(),
        "counterfactual_pending_type_counts": Counter(),
        "parent_action_head_counts": Counter(),
        "parent_card_counts": Counter(),
        "best_adv_bucket_counts": Counter(),
    }

    groups_handle = None
    if args.groups_out:
        args.groups_out.parent.mkdir(parents=True, exist_ok=True)
        groups_handle = args.groups_out.open("w", encoding="utf-8")

    driver = FullRunDriver(args.binary)
    try:
        for episode_index in range(args.episodes):
            seed = args.seed_start + episode_index * args.seed_step
            collect_episode(args, driver, seed, summary, counters, groups_handle)
    finally:
        if groups_handle:
            groups_handle.close()
        driver.close()

    for key, counter in counters.items():
        summary[key] = dict(sorted(counter.items()))
    write_json(args.out, summary)
    print(json.dumps(summary, indent=2, sort_keys=True))


def collect_episode(
    args: argparse.Namespace,
    driver: FullRunDriver,
    seed: int,
    summary: dict[str, Any],
    counters: dict[str, Counter[str]],
    groups_handle: Any,
) -> None:
    response = driver.request(
        {
            "cmd": "reset",
            "seed": seed,
            "ascension": args.ascension,
            "final_act": args.final_act,
            "class": args.player_class,
            "max_steps": args.max_steps,
            "reward_shaping_profile": "baseline",
        }
    )
    done = bool(response.get("done"))
    step = 0
    while not done and step < args.max_steps:
        payload = response.get("payload") or {}
        observation = payload.get("observation") or {}
        decision_type = str(observation.get("decision_type") or "")
        counters["decision_type_counts"][decision_type] += 1
        if decision_type.startswith("combat"):
            summary["combat_decision_count"] += 1
            if decision_type == "combat":
                summary["normal_combat_decision_count"] += 1
            else:
                summary["trajectory_pending_decision_count"] += 1
                pending_kind = pending_kind_from_observation(observation)
                counters["trajectory_pending_type_counts"][pending_kind] += 1
            inspect_payload = inspect_counterfactual_pending(args, driver)
            summary["pending_exact_dedup_count"] += int(inspect_payload.get("pending_exact_dedup_count") or 0)
            summary["root_scoped_candidate_count"] += int(inspect_payload.get("root_scoped_candidate_count") or 0)
            summary["roots_considered"] += int(inspect_payload.get("roots_considered") or 0)
            for group in inspect_payload.get("groups") or []:
                record_group(seed, step, group, summary, counters, groups_handle)
        response = driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
        done = bool(response.get("done"))
        step += 1


def inspect_counterfactual_pending(args: argparse.Namespace, driver: FullRunDriver) -> dict[str, Any]:
    response = driver.request(
        {
            "cmd": "inspect_counterfactual_pending",
            "candidate_scope": args.candidate_scope,
            "continuation_policy": args.continuation_policy,
            "horizon_decisions": args.horizon_decisions,
            "horizon_mode": args.horizon_mode,
            "oracle_margin": args.oracle_margin,
            "gamma": args.gamma,
            "max_roots": args.max_roots,
            "max_groups": args.max_groups_per_state,
            "parallelism": args.parallelism,
            "include_observation": args.include_observation,
        }
    )
    return response.get("payload") or {}


def record_group(
    seed: int,
    step: int,
    group: dict[str, Any],
    summary: dict[str, Any],
    counters: dict[str, Counter[str]],
    groups_handle: Any,
) -> None:
    summary["counterfactual_pending_group_count"] += 1
    summary["counterfactual_candidate_rows"] += len(group.get("candidates") or [])
    best_adv = float(group.get("best_adv_vs_rule_mean") or 0.0)
    if group.get("selected_action_index") != group.get("rule_index"):
        summary["counterfactual_positive_group_count"] += 1
    counters["counterfactual_pending_type_counts"][str(group.get("decision_type") or "unknown")] += 1
    counters["best_adv_bucket_counts"][best_adv_bucket(best_adv, float(group.get("oracle_margin") or 1.0))] += 1

    parent_key = str(group.get("parent_action_key") or "")
    counters["parent_action_head_counts"][parent_key.split("/", 2)[0] if parent_key else "unknown"] += 1
    card = extract_key_segment(parent_key, "card")
    if card:
        counters["parent_card_counts"][card] += 1

    if groups_handle:
        row = dict(group)
        row["seed"] = seed
        row["step"] = step
        row["group_key"] = f"verified_teacher_pending|seed:{seed}|step:{step}|parent:{parent_key}|decision:{group.get('decision_type')}"
        groups_handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")


def pending_kind_from_observation(observation: dict[str, Any]) -> str:
    combat = observation.get("combat") or {}
    return str(combat.get("pending_choice_kind") or observation.get("decision_type") or "unknown")


def extract_key_segment(key: str, name: str) -> str:
    marker = f"{name}:"
    for part in key.split("/"):
        if part.startswith(marker):
            return part[len(marker) :]
    return ""


def best_adv_bucket(adv: float, margin: float) -> str:
    if adv <= 0.0:
        return "adv_le_0"
    if adv <= margin:
        return "adv_0_to_margin"
    if adv <= margin * 2.0:
        return "adv_margin_to_2x_margin"
    return "adv_gt_2x_margin"


if __name__ == "__main__":
    main()
