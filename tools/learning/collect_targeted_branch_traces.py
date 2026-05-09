#!/usr/bin/env python3
"""Collect deeper BranchTrace records for mined hard-state decisions.

This is targeted data recollection, not policy execution. The script follows a
behavior policy to previously mined decision points, then asks the Rust branch
evaluator to trace more candidates and/or a deeper horizon. It preserves the
regular BranchTrace collection record shape so existing dataset exporters can
consume the output.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable

from collect_branch_traces import DriverClient, action_kind, default_driver_path


def iter_jsonl(path: Path) -> Iterable[dict[str, Any]]:
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                yield json.loads(line)


def bump(counter: dict[str, int], key: str, amount: int = 1) -> None:
    counter[key] = int(counter.get(key) or 0) + amount


def candidate_action_key(candidate: dict[str, Any]) -> str | None:
    value = candidate.get("action_key")
    return value if isinstance(value, str) else None


def hard_state_pair_keys(row: dict[str, Any]) -> list[str]:
    keys: list[str] = []
    for side in ("left", "right"):
        candidate = (row.get(side) or {}).get("candidate") or {}
        key = candidate.get("action_key")
        if isinstance(key, str) and key not in keys:
            keys.append(key)
    return keys


def load_targets(path: Path, *, max_rows: int, max_decisions: int) -> dict[int, dict[int, list[dict[str, Any]]]]:
    by_seed_step: dict[int, dict[int, list[dict[str, Any]]]] = defaultdict(lambda: defaultdict(list))
    seen_decisions: set[tuple[int, int]] = set()
    rows_read = 0
    for row in iter_jsonl(path):
        if row.get("trainable_as_action_label") is not False:
            continue
        if (row.get("label_policy") or {}).get("action_label") is not False:
            continue
        seed = row.get("episode_seed")
        step = row.get("episode_step")
        if not isinstance(seed, int) or not isinstance(step, int):
            continue
        decision_key = (seed, step)
        if decision_key not in seen_decisions:
            if max_decisions > 0 and len(seen_decisions) >= max_decisions:
                break
            seen_decisions.add(decision_key)
        by_seed_step[seed][step].append(row)
        rows_read += 1
        if max_rows > 0 and rows_read >= max_rows:
            break
    return by_seed_step


def target_summary(rows: list[dict[str, Any]]) -> dict[str, Any]:
    reason_counts: Counter[str] = Counter()
    pair_kind_counts: Counter[str] = Counter()
    pair_card_counts: Counter[str] = Counter()
    pair_keys: list[str] = []
    for row in rows:
        reason_counts.update(row.get("reasons") or [])
        pair_kind = row.get("pair_kind")
        if isinstance(pair_kind, str):
            pair_kind_counts[pair_kind] += 1
        pair_card = row.get("pair_card")
        if isinstance(pair_card, str):
            pair_card_counts[pair_card] += 1
        for key in hard_state_pair_keys(row):
            if key not in pair_keys:
                pair_keys.append(key)
    return {
        "schema_version": "targeted_recollection_target_summary_v0",
        "target_row_count": len(rows),
        "target_pair_action_keys": pair_keys,
        "reason_counts": dict(reason_counts),
        "pair_kind_counts": dict(pair_kind_counts),
        "pair_card_counts": dict(pair_card_counts),
    }


def action_indices_for_target(
    candidates: list[dict[str, Any]],
    behavior_action_id: Any,
    target_rows: list[dict[str, Any]],
    *,
    max_candidates: int,
    candidate_index_mode: str,
) -> tuple[list[int], list[str], list[str]]:
    indices: list[int] = []
    if candidate_index_mode == "all_plus_targets":
        for index in range(len(candidates)):
            if max_candidates > 0 and len(indices) >= max_candidates:
                break
            indices.append(index)
    elif candidate_index_mode != "targets_plus_behavior":
        raise ValueError(f"unknown candidate_index_mode {candidate_index_mode}")

    if isinstance(behavior_action_id, int) and 0 <= behavior_action_id < len(candidates):
        if behavior_action_id not in indices:
            indices.append(behavior_action_id)

    key_to_index = {
        candidate_action_key(candidate): index for index, candidate in enumerate(candidates)
    }
    matched_keys: list[str] = []
    missing_keys: list[str] = []
    for key in target_summary(target_rows)["target_pair_action_keys"]:
        index = key_to_index.get(key)
        if isinstance(index, int):
            matched_keys.append(key)
            if index not in indices and (max_candidates <= 0 or len(indices) < max_candidates):
                indices.append(index)
        else:
            missing_keys.append(key)
    return sorted(indices), matched_keys, missing_keys


def collect_seed(
    client: DriverClient,
    *,
    seed: int,
    target_steps: dict[int, list[dict[str, Any]]],
    args: argparse.Namespace,
    out,
    summary: dict[str, Any],
) -> None:
    max_target_step = max(target_steps) if target_steps else -1
    env_max_steps = max(args.env_max_steps or 0, max_target_step + 2, args.max_steps)
    client.request(
        {
            "cmd": "reset",
            "seed": seed,
            "ascension": args.ascension,
            "final_act": args.final_act,
            "class": "ironclad",
            "max_steps": env_max_steps,
            "reward_shaping_profile": "baseline",
        }
    )
    records = 0
    done = False
    reached_steps: set[int] = set()
    while not done and records < env_max_steps and records <= max_target_step:
        policy_input = client.request({"cmd": "policy_input", "time_budget_ms": 25})["payload"]
        candidates = policy_input.get("candidates") or []
        if not candidates:
            break
        decision_type = (
            ((policy_input.get("observation") or {}).get("decision_type"))
            or ((policy_input.get("decision_id") or {}).get("decision_type"))
            or "unknown"
        )
        preview = client.request(
            {
                "cmd": "preview_policy_action",
                "policy": args.behavior_policy,
                "include_state": False,
                "include_next_state": False,
                "check_live_env_unchanged": False,
            }
        )["payload"]
        behavior_action_id = preview.get("chosen_action_index")

        target_rows = target_steps.get(records)
        if target_rows:
            reached_steps.add(records)
            target_info = target_summary(target_rows)
            action_indices, matched_keys, missing_keys = action_indices_for_target(
                candidates,
                behavior_action_id,
                target_rows,
                max_candidates=args.max_candidates,
                candidate_index_mode=args.candidate_index_mode,
            )
            for index in action_indices:
                bump(
                    summary["requested_candidate_action_kind_counts"],
                    action_kind(candidates, index),
                )
            branch_request = {
                "cmd": "branch_trace",
                "action_indices": action_indices,
                "candidate_scope": args.candidate_scope,
                "candidate_sampling_spec_id": (
                    f"hard_state_targeted_{args.candidate_index_mode}_v0"
                ),
                "candidate_index_mode": args.candidate_index_mode,
                "candidate_cap": args.max_candidates,
                "behavior_action_id": behavior_action_id,
                "sampling_seed": seed,
                "continuation_policy": args.continuation_policy,
                "horizon_decisions": args.horizon_decisions,
                "horizon_mode": args.horizon_mode,
                "sim_version": "full_run_env_branch_trace_v1",
                "content_version": "content_current",
                "include_comparisons": True,
            }
            branch_payload = client.request(branch_request)["payload"]
            record = {
                "schema_version": "branch_trace_collection_record_v1",
                "seed": seed,
                "episode_step": records,
                "decision_type": decision_type,
                "behavior_policy": args.behavior_policy,
                "behavior_action_id": behavior_action_id,
                "behavior_action_key": preview.get("chosen_action_key"),
                "targeted_recollection": {
                    "schema_version": "targeted_recollection_record_v0",
                    "source": "branch_hard_state_mining_v0",
                    "target": target_info,
                    "matched_target_pair_action_keys": matched_keys,
                    "missing_target_pair_action_keys": missing_keys,
                    "trainable_as_action_label": False,
                },
                "branch_trace_batch": branch_payload,
            }
            out.write(json.dumps(record, separators=(",", ":")) + "\n")

            summary["target_decisions_reached"] += 1
            summary["target_rows_reached"] += len(target_rows)
            summary["trace_count"] += int(branch_payload.get("trace_count") or 0)
            summary["comparison_count"] += int(branch_payload.get("comparison_count") or 0)
            summary["matched_target_pair_action_key_count"] += len(matched_keys)
            summary["missing_target_pair_action_key_count"] += len(missing_keys)
            summary["sampling_requested_action_count"] += len(action_indices)
            summary["sampling_included_candidate_count"] += int(
                ((branch_payload.get("candidate_sampling_spec") or {}).get("included_candidate_count"))
                or 0
            )
            if not branch_payload.get("live_env_unchanged"):
                summary["live_env_changed_count"] += 1
            validation = branch_payload.get("validation_report") or {}
            summary["validation_issue_count"] += int(validation.get("issue_count") or 0)
            if not validation.get("valid", False):
                summary["invalid_branch_batch_count"] += 1
            for issue in validation.get("issues") or []:
                code = issue.get("code") or "unknown"
                bump(summary["validation_issue_code_counts"], code)
                if (
                    "redaction" in code
                    or "hidden" in code
                    or "non_public" in code
                    or "model_input" in code
                ):
                    summary["redaction_violation_count"] += 1
            for trace in branch_payload.get("traces") or []:
                if trace.get("trainable_as_action_label") is not False:
                    summary["trainable_action_label_count"] += 1
                outcome = trace.get("outcome") or {}
                result = outcome.get("result") or "unknown"
                bump(summary["result_counts"], result)
                if outcome.get("outcome_censored"):
                    summary["outcome_censored_count"] += 1
                if outcome.get("truncated"):
                    summary["truncated_trace_count"] += 1
                if int(outcome.get("combat_win_delta") or 0) > 0:
                    summary["combat_win_delta_positive_count"] += 1
                if int(outcome.get("hp_delta") or 0) < 0:
                    summary["hp_loss_trace_count"] += 1
            for comparison in branch_payload.get("comparisons") or []:
                role = comparison.get("trainable_role") or ""
                if "action" in role:
                    summary["action_like_comparison_role_count"] += 1
                if any(
                    key in comparison
                    for key in (
                        "winner",
                        "preferred",
                        "preferred_action",
                        "selected_action",
                        "teacher_choice",
                    )
                ):
                    summary["winner_or_preference_field_count"] += 1
                if comparison.get("rng_diverged") is True:
                    summary["rng_diverged_comparison_count"] += 1
                elif comparison.get("rng_diverged") is False:
                    summary["rng_not_diverged_comparison_count"] += 1

        if behavior_action_id is None:
            break
        step = client.request({"cmd": "decision_env_step", "action_id": behavior_action_id})
        done = bool(step.get("done"))
        records += 1

    missed_steps = sorted(set(target_steps) - reached_steps)
    for step in missed_steps:
        summary["target_decisions_missed"] += 1
        summary["target_rows_missed"] += len(target_steps[step])
    summary["episodes"].append(
        {
            "seed": seed,
            "target_steps": len(target_steps),
            "reached_target_steps": len(reached_steps),
            "missed_target_steps": missed_steps,
            "records_replayed": records,
            "done": done,
        }
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver", type=Path, default=default_driver_path())
    parser.add_argument("--hard-states", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path)
    parser.add_argument("--max-target-rows", type=int, default=100)
    parser.add_argument("--max-target-decisions", type=int, default=50)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--behavior-policy", default="rule_baseline_v0")
    parser.add_argument("--continuation-policy", default="rule_baseline_v0")
    parser.add_argument("--horizon-decisions", type=int, default=16)
    parser.add_argument("--horizon-mode", default="combat_end_v1")
    parser.add_argument("--candidate-scope", default="controlled_v1")
    parser.add_argument(
        "--candidate-index-mode",
        choices=["all_plus_targets", "targets_plus_behavior"],
        default="all_plus_targets",
    )
    parser.add_argument("--max-candidates", type=int, default=64)
    parser.add_argument("--max-steps", type=int, default=300)
    parser.add_argument("--env-max-steps", type=int, default=0)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.driver.exists():
        raise SystemExit(f"driver binary not found: {args.driver}")
    targets = load_targets(
        args.hard_states,
        max_rows=args.max_target_rows,
        max_decisions=args.max_target_decisions,
    )
    args.out.parent.mkdir(parents=True, exist_ok=True)
    summary_out = args.summary_out or args.out.with_suffix(".summary.json")
    target_decision_count = sum(len(steps) for steps in targets.values())
    target_row_count = sum(len(rows) for steps in targets.values() for rows in steps.values())
    summary: dict[str, Any] = {
        "schema_version": "targeted_branch_trace_collection_summary_v0",
        "hard_states": str(args.hard_states),
        "out": str(args.out),
        "max_target_rows": args.max_target_rows,
        "max_target_decisions": args.max_target_decisions,
        "target_seed_count": len(targets),
        "target_decision_count": target_decision_count,
        "target_row_count": target_row_count,
        "target_decisions_reached": 0,
        "target_decisions_missed": 0,
        "target_rows_reached": 0,
        "target_rows_missed": 0,
        "trace_count": 0,
        "comparison_count": 0,
        "matched_target_pair_action_key_count": 0,
        "missing_target_pair_action_key_count": 0,
        "sampling_requested_action_count": 0,
        "sampling_included_candidate_count": 0,
        "live_env_changed_count": 0,
        "invalid_branch_batch_count": 0,
        "validation_issue_count": 0,
        "redaction_violation_count": 0,
        "trainable_action_label_count": 0,
        "action_like_comparison_role_count": 0,
        "winner_or_preference_field_count": 0,
        "rng_diverged_comparison_count": 0,
        "rng_not_diverged_comparison_count": 0,
        "outcome_censored_count": 0,
        "truncated_trace_count": 0,
        "combat_win_delta_positive_count": 0,
        "hp_loss_trace_count": 0,
        "requested_candidate_action_kind_counts": {},
        "validation_issue_code_counts": {},
        "result_counts": {},
        "horizon_decisions": args.horizon_decisions,
        "horizon_mode": args.horizon_mode,
        "candidate_scope": args.candidate_scope,
        "candidate_index_mode": args.candidate_index_mode,
        "max_candidates": args.max_candidates,
        "behavior_policy": args.behavior_policy,
        "continuation_policy": args.continuation_policy,
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "targeted_recollection_is_data_collection_not_policy": True,
        },
        "episodes": [],
    }

    client = DriverClient(args.driver)
    try:
        with args.out.open("w", encoding="utf-8") as out:
            for seed in sorted(targets):
                collect_seed(
                    client,
                    seed=seed,
                    target_steps=targets[seed],
                    args=args,
                    out=out,
                    summary=summary,
                )
    finally:
        client.close()

    summary_out.parent.mkdir(parents=True, exist_ok=True)
    summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
