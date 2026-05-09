#!/usr/bin/env python3
"""Run a closed-loop strict branch-evidence policy A/B test.

This is an offline utility test, not a training-label generator. It compares:

  baseline:
    execute a behavior policy directly

  strict_evidence_policy_v0:
    preview the same behavior action
    for combat decisions, collect branch traces for current legal candidates
    override only when complete, RNG-aligned combat-end evidence shows a
    material improvement under strict rules
    otherwise execute the behavior action

The point is to test whether branch evidence can improve closed-loop outcomes.
It does not emit action labels, winners, or teacher preferences.
"""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable

from branch_evidence_cache import BranchEvidenceCache, branch_request_semantic_key
from collect_branch_traces import DriverClient, action_kind, default_driver_path


FORBIDDEN_LABEL_KEYS = {
    "winner",
    "preferred",
    "preferred_action",
    "selected_action",
    "teacher_choice",
}


def safe_int(value: Any, default: int = 0) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return default
    return number if math.isfinite(number) else default


def assert_no_label_leak(row: dict[str, Any], *, label: str) -> None:
    serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
    for key in FORBIDDEN_LABEL_KEYS:
        if f'"{key}"' in serialized:
            raise ValueError(f"{label} contains forbidden key {key}")


def forced_action_id(trace: dict[str, Any]) -> int | None:
    forced = trace.get("forced_prefix") or []
    if not forced:
        return None
    value = forced[0]
    return value if isinstance(value, int) else None


def complete_trace(trace: dict[str, Any]) -> bool:
    outcome = trace.get("outcome") or {}
    if outcome.get("outcome_censored"):
        return False
    if outcome.get("truncated") or trace.get("truncated"):
        return False
    if outcome.get("result") == "defeat":
        return True
    if outcome.get("boundary_requested") == "combat_end":
        return bool(outcome.get("boundary_reached"))
    return True


def trace_outcome(trace: dict[str, Any]) -> dict[str, Any]:
    outcome = trace.get("outcome") or {}
    return {
        "result": outcome.get("result"),
        "hp": safe_int(outcome.get("hp")),
        "hp_delta": safe_int(outcome.get("hp_delta")),
        "floor": safe_int(outcome.get("floor")),
        "floor_delta": safe_int(outcome.get("floor_delta")),
        "combat_win_count": safe_int(outcome.get("combat_win_count")),
        "combat_win_delta": safe_int(outcome.get("combat_win_delta")),
        "total_reward": safe_float(outcome.get("total_reward")),
        "step_count": safe_int(outcome.get("step_count")),
        "boundary_reached": bool(outcome.get("boundary_reached")),
        "outcome_censored": bool(outcome.get("outcome_censored")),
        "truncated": bool(outcome.get("truncated")),
        "stop_reason": outcome.get("stop_reason"),
    }


def strict_pair_set(branch_payload: dict[str, Any]) -> set[frozenset[str]]:
    traces = {
        trace.get("branch_id"): trace
        for trace in branch_payload.get("traces") or []
        if isinstance(trace.get("branch_id"), str)
    }
    out: set[frozenset[str]] = set()
    for comparison in branch_payload.get("comparisons") or []:
        if comparison.get("pairing_valid") is not True:
            continue
        if comparison.get("rng_diverged") is not False:
            continue
        left_id = comparison.get("left_branch_id")
        right_id = comparison.get("right_branch_id")
        if not isinstance(left_id, str) or not isinstance(right_id, str):
            continue
        left = traces.get(left_id)
        right = traces.get(right_id)
        if left is None or right is None:
            continue
        if not complete_trace(left) or not complete_trace(right):
            continue
        out.add(frozenset((left_id, right_id)))
    return out


def candidate_action_summary(candidates: list[dict[str, Any]], action_id: int | None) -> dict[str, Any]:
    if not isinstance(action_id, int) or action_id < 0 or action_id >= len(candidates):
        return {"action_id": action_id, "action_kind": "unknown", "action_key": None, "card_id": None}
    candidate = candidates[action_id]
    payload = candidate.get("payload") or {}
    card = payload.get("card") if isinstance(payload.get("card"), dict) else {}
    return {
        "action_id": action_id,
        "action_kind": candidate.get("action_kind"),
        "action_key": candidate.get("action_key"),
        "card_id": card.get("card_id"),
    }


def is_cross_combat_resource_action(candidates: list[dict[str, Any]], action_id: int) -> bool:
    """Reject obvious non-card resource actions.

    `controlled_v1` should already exclude potions and screen actions for combat
    branch traces, but keep this guard explicit for closed-loop override tests.
    Exhausting a card inside combat is not treated as cross-combat resource here;
    it remains represented in the branch outcome and trace audit.
    """
    if action_id < 0 or action_id >= len(candidates):
        return True
    return (candidates[action_id].get("action_kind") or "unknown") not in {"play_card", "end_turn"}


def material_reason(
    candidate: dict[str, Any],
    behavior: dict[str, Any],
    *,
    min_hp_margin: int,
    min_reward_margin: float,
    hp_margin_only: bool,
    allow_progress_flip: bool,
) -> str | None:
    cand = candidate.get("outcome") or {}
    beh = behavior.get("outcome") or {}
    cand_dead = cand.get("result") == "defeat"
    beh_dead = beh.get("result") == "defeat"
    if not cand_dead and beh_dead:
        return "survival_flip"
    if cand_dead and not beh_dead:
        return None
    cand_combat = safe_int(cand.get("combat_win_count"))
    beh_combat = safe_int(beh.get("combat_win_count"))
    if cand_combat > beh_combat:
        return "combat_progress_flip" if allow_progress_flip else None
    if cand_combat < beh_combat:
        return None
    hp_gain = safe_int(cand.get("hp")) - safe_int(beh.get("hp"))
    reward_gain = safe_float(cand.get("total_reward")) - safe_float(beh.get("total_reward"))
    if hp_gain >= min_hp_margin and reward_gain >= -abs(min_reward_margin):
        return "hp_margin"
    if hp_margin_only:
        return None
    if reward_gain >= min_reward_margin and hp_gain >= 0:
        return "reward_margin"
    return None


def choose_strict_evidence_action(
    branch_payload: dict[str, Any],
    *,
    behavior_action_id: int,
    min_hp_margin: int,
    min_reward_margin: float,
    hp_margin_only: bool,
    allow_progress_flip: bool,
) -> dict[str, Any]:
    traces = branch_payload.get("traces") or []
    candidates = traces[0].get("candidates") if traces else []
    candidates = candidates if isinstance(candidates, list) else []
    behavior_trace = next(
        (trace for trace in traces if forced_action_id(trace) == behavior_action_id),
        None,
    )
    base = {
        "schema_version": "strict_evidence_policy_decision_v0",
        "trainable_as_action_label": False,
        "mode": "behavior",
        "reason": "no_strict_material_alternative",
        "behavior_action_id": behavior_action_id,
        "chosen_action_id": behavior_action_id,
        "strict_candidate_count": 0,
        "candidate_action": None,
        "behavior_action": candidate_action_summary(candidates, behavior_action_id),
    }
    if behavior_trace is None:
        return {**base, "reason": "missing_behavior_trace"}
    if not complete_trace(behavior_trace):
        return {**base, "reason": "incomplete_behavior_trace"}
    strict_pairs = strict_pair_set(branch_payload)
    behavior_branch_id = behavior_trace.get("branch_id")
    alternatives: list[dict[str, Any]] = []
    strict_candidate_count = 0
    for trace in traces:
        branch_id = trace.get("branch_id")
        action_id = forced_action_id(trace)
        if branch_id == behavior_branch_id or not isinstance(branch_id, str):
            continue
        if not isinstance(behavior_branch_id, str):
            continue
        if not isinstance(action_id, int):
            continue
        if is_cross_combat_resource_action(candidates, action_id):
            continue
        if frozenset((branch_id, behavior_branch_id)) not in strict_pairs:
            continue
        strict_candidate_count += 1
        reason = material_reason(
            trace,
            behavior_trace,
            min_hp_margin=min_hp_margin,
            min_reward_margin=min_reward_margin,
            hp_margin_only=hp_margin_only,
            allow_progress_flip=allow_progress_flip,
        )
        if reason is None:
            continue
        cand = trace.get("outcome") or {}
        beh = behavior_trace.get("outcome") or {}
        alternatives.append(
            {
                "action_id": action_id,
                "branch_id": branch_id,
                "material_reason": reason,
                "hp_gain_vs_behavior": safe_int(cand.get("hp")) - safe_int(beh.get("hp")),
                "reward_gain_vs_behavior": safe_float(cand.get("total_reward"))
                - safe_float(beh.get("total_reward")),
                "combat_win_count_gain_vs_behavior": safe_int(cand.get("combat_win_count"))
                - safe_int(beh.get("combat_win_count")),
                "candidate_outcome": trace_outcome(trace),
            }
        )
    if not alternatives:
        return {**base, "strict_candidate_count": strict_candidate_count}
    alternatives.sort(
        key=lambda item: (
            -safe_int(item.get("combat_win_count_gain_vs_behavior")),
            -safe_int(item.get("hp_gain_vs_behavior")),
            -safe_float(item.get("reward_gain_vs_behavior")),
            safe_int(item.get("action_id"), 9999),
        )
    )
    best = alternatives[0]
    chosen_id = safe_int(best.get("action_id"), behavior_action_id)
    return {
        **base,
        "mode": "strict_evidence_override",
        "reason": best.get("material_reason"),
        "chosen_action_id": chosen_id,
        "strict_candidate_count": strict_candidate_count,
        "candidate_action": candidate_action_summary(candidates, chosen_id),
        "material_alternative_count": len(alternatives),
        "hp_gain_vs_behavior": best.get("hp_gain_vs_behavior"),
        "reward_gain_vs_behavior": best.get("reward_gain_vs_behavior"),
        "combat_win_count_gain_vs_behavior": best.get("combat_win_count_gain_vs_behavior"),
        "behavior_outcome": trace_outcome(behavior_trace),
        "candidate_outcome": best.get("candidate_outcome"),
    }


def preview_behavior(client: DriverClient, policy: str) -> dict[str, Any]:
    return client.request(
        {
            "cmd": "preview_policy_action",
            "policy": policy,
            "include_state": False,
            "include_next_state": False,
            "check_live_env_unchanged": False,
        }
    )["payload"]


def validate_cached_payload_identity(
    payload: dict[str, Any],
    identity: dict[str, Any],
) -> bool:
    traces = payload.get("traces") or []
    if not traces:
        return False
    state_hash = identity.get("state_hash")
    rng_hash = identity.get("rng_state_hash")
    for trace in traces:
        if trace.get("state_hash_before") != state_hash:
            return False
        if trace.get("rng_state_before_hash") != rng_hash:
            return False
    return True


def request_branch_trace(
    client: DriverClient,
    branch_request: dict[str, Any],
    *,
    cache: BranchEvidenceCache | None,
) -> tuple[dict[str, Any], bool]:
    if cache is None:
        return client.request(branch_request)["payload"], False
    identity = client.request({"cmd": "branch_trace_cache_identity"})["payload"]
    key = branch_request_semantic_key(identity, branch_request)
    cached = cache.get(key)
    if cached is not None and validate_cached_payload_identity(cached, identity):
        return cached, True
    if cached is not None:
        cache.identity_mismatch_count += 1
    payload = client.request(branch_request)["payload"]
    if validate_cached_payload_identity(payload, identity):
        cache.put(key, payload)
    else:
        cache.identity_mismatch_count += 1
    return payload, False


def run_episode(
    client: DriverClient,
    *,
    seed: int,
    policy_kind: str,
    args: argparse.Namespace,
    trace_out,
    branch_cache: BranchEvidenceCache | None,
) -> dict[str, Any]:
    client.request(
        {
            "cmd": "reset",
            "seed": seed,
            "ascension": args.ascension,
            "final_act": args.final_act,
            "class": "ironclad",
            "max_steps": args.env_max_steps,
            "reward_shaping_profile": "baseline",
        }
    )
    done = False
    steps = 0
    combat_decisions = 0
    branch_trace_count = 0
    comparison_count = 0
    override_count = 0
    override_reason_counts: Counter[str] = Counter()
    behavior_reason_counts: Counter[str] = Counter()
    validation_issue_count = 0
    censored_trace_count = 0
    truncated_trace_count = 0
    branch_cache_hit_count = 0
    branch_cache_miss_count = 0
    legal_candidate_count_total = 0
    sampling_requested_action_count = 0
    sampling_included_candidate_count = 0
    sampling_excluded_candidate_count = 0
    sampling_missing_behavior_action_count = 0
    sampling_excluded_by_reason_counts: Counter[str] = Counter()
    final_info: dict[str, Any] = {}
    total_reward = 0.0

    while not done and steps < args.max_steps:
        policy_input = client.request({"cmd": "policy_input", "time_budget_ms": 25})["payload"]
        candidates = policy_input.get("candidates") or []
        if not candidates:
            break
        decision_type = (
            ((policy_input.get("observation") or {}).get("decision_type"))
            or ((policy_input.get("decision_id") or {}).get("decision_type"))
            or "unknown"
        )
        preview = preview_behavior(client, args.behavior_policy)
        behavior_action_id = preview.get("chosen_action_index")
        if not isinstance(behavior_action_id, int):
            break
        chosen_action_id = behavior_action_id
        decision_record: dict[str, Any] | None = None

        if policy_kind == "strict_evidence_policy_v0" and decision_type.startswith("combat"):
            combat_decisions += 1
            action_indices = list(range(min(len(candidates), args.max_candidates)))
            if behavior_action_id not in action_indices:
                action_indices.append(behavior_action_id)
            branch_request = {
                "cmd": "branch_trace",
                "action_indices": action_indices,
                "candidate_scope": args.candidate_scope,
                "candidate_sampling_spec_id": "strict_evidence_policy_current_all_controlled_v0",
                "candidate_cap": args.max_candidates,
                "behavior_action_id": behavior_action_id,
                "sampling_seed": seed,
                "continuation_policy": args.continuation_policy,
                "horizon_decisions": args.horizon_decisions,
                "horizon_mode": args.horizon_mode,
                "sim_version": "full_run_env_strict_evidence_policy_v0",
                "content_version": "content_current",
                "include_comparisons": True,
            }
            branch_payload, cache_hit = request_branch_trace(
                client, branch_request, cache=branch_cache
            )
            if cache_hit:
                branch_cache_hit_count += 1
            else:
                branch_cache_miss_count += 1
            branch_trace_count += int(branch_payload.get("trace_count") or 0)
            comparison_count += int(branch_payload.get("comparison_count") or 0)
            sampling = branch_payload.get("candidate_sampling_spec") or {}
            legal_candidate_count_total += int(
                sampling.get("legal_candidate_count") or len(candidates)
            )
            sampling_requested_action_count += int(sampling.get("requested_action_count") or 0)
            sampling_included_candidate_count += int(
                sampling.get("included_candidate_count") or 0
            )
            sampling_excluded_candidate_count += int(
                sampling.get("excluded_candidate_count") or 0
            )
            if not sampling.get("include_behavior_action", False):
                sampling_missing_behavior_action_count += 1
            for reason, count in (sampling.get("excluded_by_reason") or {}).items():
                sampling_excluded_by_reason_counts[str(reason)] += int(count or 0)
            validation = branch_payload.get("validation_report") or {}
            validation_issue_count += int(validation.get("issue_count") or 0)
            for trace in branch_payload.get("traces") or []:
                outcome = trace.get("outcome") or {}
                if outcome.get("outcome_censored"):
                    censored_trace_count += 1
                if outcome.get("truncated"):
                    truncated_trace_count += 1
            decision = choose_strict_evidence_action(
                branch_payload,
                behavior_action_id=behavior_action_id,
                min_hp_margin=args.min_hp_margin,
                min_reward_margin=args.min_reward_margin,
                hp_margin_only=args.hp_margin_only,
                allow_progress_flip=args.allow_progress_flip,
            )
            chosen_action_id = safe_int(decision.get("chosen_action_id"), behavior_action_id)
            if decision.get("mode") == "strict_evidence_override":
                override_count += 1
                override_reason_counts[str(decision.get("reason") or "unknown")] += 1
            else:
                behavior_reason_counts[str(decision.get("reason") or "unknown")] += 1
            decision_record = {
                "schema_version": "strict_evidence_policy_step_record_v0",
                "trainable_as_action_label": False,
                "episode_seed": seed,
                "episode_step": steps,
                "decision_type": decision_type,
                "policy_kind": policy_kind,
                "behavior_action_id": behavior_action_id,
                "behavior_action_key": preview.get("chosen_action_key"),
                "decision": decision,
                "branch_trace_count": int(branch_payload.get("trace_count") or 0),
                "comparison_count": int(branch_payload.get("comparison_count") or 0),
                "validation_issue_count": int(validation.get("issue_count") or 0),
                "candidate_sampling": sampling,
                "branch_evidence_cache_hit": cache_hit,
            }
        elif policy_kind == "baseline_behavior":
            if decision_type.startswith("combat"):
                combat_decisions += 1

        step = client.request({"cmd": "decision_env_step", "action_id": chosen_action_id})
        reward = safe_float(step.get("reward"))
        total_reward += reward
        done = bool(step.get("done"))
        final_info = step.get("info") or {}
        if decision_record is not None:
            decision_record["executed_action_key"] = step.get("chosen_action_key")
            decision_record["step_reward"] = reward
            decision_record["done_after_step"] = done
            assert_no_label_leak(decision_record, label="strict evidence step record")
            trace_out.write(json.dumps(decision_record, separators=(",", ":")) + "\n")
        steps += 1

    return {
        "schema_version": "strict_evidence_policy_episode_result_v0",
        "trainable_as_action_label": False,
        "seed": seed,
        "policy_kind": policy_kind,
        "steps": steps,
        "done": done,
        "total_reward": total_reward,
        "combat_decisions": combat_decisions,
        "branch_trace_count": branch_trace_count,
        "comparison_count": comparison_count,
        "override_count": override_count,
        "override_reason_counts": dict(override_reason_counts),
        "behavior_reason_counts": dict(behavior_reason_counts),
        "validation_issue_count": validation_issue_count,
        "censored_trace_count": censored_trace_count,
        "truncated_trace_count": truncated_trace_count,
        "branch_cache_hit_count": branch_cache_hit_count,
        "branch_cache_miss_count": branch_cache_miss_count,
        "legal_candidate_count_total": legal_candidate_count_total,
        "sampling_requested_action_count": sampling_requested_action_count,
        "sampling_included_candidate_count": sampling_included_candidate_count,
        "sampling_excluded_candidate_count": sampling_excluded_candidate_count,
        "sampling_missing_behavior_action_count": sampling_missing_behavior_action_count,
        "sampling_excluded_by_reason_counts": dict(sampling_excluded_by_reason_counts),
        "final_info": final_info,
        "final_result": final_info.get("result"),
        "terminal_reason": final_info.get("terminal_reason"),
        "floor": safe_int(final_info.get("floor")),
        "hp": safe_int(final_info.get("hp")),
        "max_hp": safe_int(final_info.get("max_hp")),
        "combat_win_count": safe_int(final_info.get("combat_win_count")),
        "gold": safe_int(final_info.get("gold")),
        "deck_size": safe_int(final_info.get("deck_size")),
    }


def aggregate(results: list[dict[str, Any]]) -> dict[str, Any]:
    by_seed: dict[int, dict[str, dict[str, Any]]] = defaultdict(dict)
    for result in results:
        by_seed[safe_int(result.get("seed"))][str(result.get("policy_kind"))] = result

    paired: list[dict[str, Any]] = []
    for seed, policies in sorted(by_seed.items()):
        baseline = policies.get("baseline_behavior")
        evidence = policies.get("strict_evidence_policy_v0")
        if baseline is None or evidence is None:
            continue
        paired.append(
            {
                "seed": seed,
                "result_changed": baseline.get("final_result") != evidence.get("final_result"),
                "floor_delta": safe_int(evidence.get("floor")) - safe_int(baseline.get("floor")),
                "hp_delta": safe_int(evidence.get("hp")) - safe_int(baseline.get("hp")),
                "combat_win_delta": safe_int(evidence.get("combat_win_count"))
                - safe_int(baseline.get("combat_win_count")),
                "reward_delta": safe_float(evidence.get("total_reward"))
                - safe_float(baseline.get("total_reward")),
                "override_count": safe_int(evidence.get("override_count")),
                "baseline": {
                    "result": baseline.get("final_result"),
                    "floor": baseline.get("floor"),
                    "hp": baseline.get("hp"),
                    "combat_win_count": baseline.get("combat_win_count"),
                    "steps": baseline.get("steps"),
                },
                "strict_evidence": {
                    "result": evidence.get("final_result"),
                    "floor": evidence.get("floor"),
                    "hp": evidence.get("hp"),
                    "combat_win_count": evidence.get("combat_win_count"),
                    "steps": evidence.get("steps"),
                    "override_count": evidence.get("override_count"),
                },
            }
        )

    def result_rank(result: Any) -> int:
        return {"defeat": 1, "ongoing": 2, "victory": 3}.get(str(result), 0)

    def sum_field(field: str) -> float:
        return sum(safe_float(row.get(field)) for row in paired)

    override_seed_count = sum(1 for row in paired if safe_int(row.get("override_count")) > 0)
    bad_outcome_count = 0
    improved_outcome_count = 0
    death_regression_count = 0
    for row in paired:
        baseline = row.get("baseline") or {}
        evidence = row.get("strict_evidence") or {}
        worsened_result = result_rank(evidence.get("result")) < result_rank(baseline.get("result"))
        improved_result = result_rank(evidence.get("result")) > result_rank(baseline.get("result"))
        if evidence.get("result") == "defeat" and baseline.get("result") != "defeat":
            death_regression_count += 1
        is_bad = (
            worsened_result
            or safe_int(row.get("combat_win_delta")) < 0
            or safe_int(row.get("floor_delta")) < 0
            or safe_float(row.get("reward_delta")) < -1.0
        )
        is_improved = (
            improved_result
            or safe_int(row.get("combat_win_delta")) > 0
            or safe_int(row.get("floor_delta")) > 0
            or safe_float(row.get("reward_delta")) > 1.0
            or safe_int(row.get("hp_delta")) > 0
        )
        if is_bad:
            bad_outcome_count += 1
        elif is_improved:
            improved_outcome_count += 1

    return {
        "paired_seed_count": len(paired),
        "paired_deltas": paired,
        "sum_floor_delta": sum_field("floor_delta"),
        "sum_hp_delta": sum_field("hp_delta"),
        "sum_combat_win_delta": sum_field("combat_win_delta"),
        "sum_reward_delta": sum_field("reward_delta"),
        "total_override_count": sum(safe_int(row.get("override_count")) for row in paired),
        "override_seed_count": override_seed_count,
        "bad_outcome_seed_count": bad_outcome_count,
        "improved_outcome_seed_count": improved_outcome_count,
        "neutral_seed_count": len(paired) - bad_outcome_count - improved_outcome_count,
        "death_regression_count": death_regression_count,
        "result_change_count": sum(1 for row in paired if row.get("result_changed")),
    }


def parse_seed_args(args: argparse.Namespace) -> list[int]:
    seeds: list[int] = []
    if args.seeds:
        seeds.extend(args.seeds)
    if args.seed_start is not None and args.episodes:
        seeds.extend(args.seed_start + index * args.seed_step for index in range(args.episodes))
    return sorted(dict.fromkeys(seeds))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--preset",
        choices=("none", "conservative_v1"),
        default="none",
        help="Named closed-loop gate preset. conservative_v1 is margin=25 + hp-margin-only + complete combat-end evidence.",
    )
    parser.add_argument("--driver", type=Path, default=None)
    parser.add_argument("--seeds", type=int, nargs="*")
    parser.add_argument("--seed-start", type=int)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--episodes", type=int, default=0)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=220)
    parser.add_argument("--env-max-steps", type=int, default=260)
    parser.add_argument("--behavior-policy", default="rule_baseline_v0")
    parser.add_argument("--continuation-policy", default="rule_baseline_v0")
    parser.add_argument("--horizon-mode", default="combat_end_v1")
    parser.add_argument("--horizon-decisions", type=int, default=16)
    parser.add_argument("--candidate-scope", default="controlled_v1")
    parser.add_argument("--max-candidates", type=int, default=12)
    parser.add_argument("--min-hp-margin", type=int, default=8)
    parser.add_argument("--min-reward-margin", type=float, default=0.25)
    parser.add_argument(
        "--hp-margin-only",
        action="store_true",
        help="Disable reward-margin-only overrides; require the configured HP margin unless survival/progress gates apply.",
    )
    parser.add_argument("--allow-progress-flip", action="store_true")
    parser.add_argument(
        "--branch-cache-dir",
        type=Path,
        help="Optional persistent branch evidence cache directory keyed by debug state hash and branch request.",
    )
    parser.add_argument("--disable-branch-cache", action="store_true")
    return parser.parse_args()


def apply_preset(args: argparse.Namespace) -> None:
    if args.preset == "conservative_v1":
        args.horizon_mode = "combat_end_v1"
        args.horizon_decisions = 16
        args.candidate_scope = "controlled_v1"
        args.max_candidates = 12
        args.min_hp_margin = 25
        args.min_reward_margin = 0.25
        args.hp_margin_only = True
        args.allow_progress_flip = False


def main() -> int:
    args = parse_args()
    apply_preset(args)
    seeds = parse_seed_args(args)
    if not seeds:
        raise SystemExit("provide --seeds or --seed-start/--episodes")
    driver = args.driver or default_driver_path()
    branch_cache = (
        BranchEvidenceCache(args.branch_cache_dir)
        if args.branch_cache_dir is not None and not args.disable_branch_cache
        else None
    )
    results: list[dict[str, Any]] = []
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8") as trace_out:
        for policy_kind in ("baseline_behavior", "strict_evidence_policy_v0"):
            client = DriverClient(driver)
            try:
                for seed in seeds:
                    result = run_episode(
                        client,
                        seed=seed,
                        policy_kind=policy_kind,
                        args=args,
                        trace_out=trace_out,
                        branch_cache=branch_cache if policy_kind == "strict_evidence_policy_v0" else None,
                    )
                    assert_no_label_leak(result, label=f"{policy_kind} episode result")
                    results.append(result)
                    trace_out.write(json.dumps(result, separators=(",", ":")) + "\n")
            finally:
                client.close()

    override_reasons: Counter[str] = Counter()
    behavior_reasons: Counter[str] = Counter()
    final_results: Counter[str] = Counter()
    sampling_excluded_by_reason_counts: Counter[str] = Counter()
    sampling_totals: Counter[str] = Counter()
    for result in results:
        final_results[f"{result.get('policy_kind')}|{result.get('final_result')}"] += 1
        override_reasons.update(result.get("override_reason_counts") or {})
        behavior_reasons.update(result.get("behavior_reason_counts") or {})
        if result.get("policy_kind") == "strict_evidence_policy_v0":
            sampling_totals["legal_candidate_count_total"] += safe_int(
                result.get("legal_candidate_count_total")
            )
            sampling_totals["sampling_requested_action_count"] += safe_int(
                result.get("sampling_requested_action_count")
            )
            sampling_totals["sampling_included_candidate_count"] += safe_int(
                result.get("sampling_included_candidate_count")
            )
            sampling_totals["sampling_excluded_candidate_count"] += safe_int(
                result.get("sampling_excluded_candidate_count")
            )
            sampling_totals["sampling_missing_behavior_action_count"] += safe_int(
                result.get("sampling_missing_behavior_action_count")
            )
            sampling_excluded_by_reason_counts.update(
                result.get("sampling_excluded_by_reason_counts") or {}
            )

    summary = {
        "schema_version": "strict_evidence_policy_ab_summary_v0",
        "policy_under_test": "strict_evidence_policy_v0",
        "baseline_policy": args.behavior_policy,
        "seed_count": len(seeds),
        "seeds": seeds,
        "config": {
            "preset": args.preset,
            "horizon_mode": args.horizon_mode,
            "horizon_decisions": args.horizon_decisions,
            "candidate_scope": args.candidate_scope,
            "max_candidates": args.max_candidates,
            "min_hp_margin": args.min_hp_margin,
            "min_reward_margin": args.min_reward_margin,
            "hp_margin_only": args.hp_margin_only,
            "allow_progress_flip": args.allow_progress_flip,
            "max_steps": args.max_steps,
            "env_max_steps": args.env_max_steps,
        },
        "episode_results": results,
        "paired_summary": aggregate(results),
        "final_result_counts": dict(final_results),
        "override_reason_counts": dict(override_reasons),
        "behavior_reason_counts": dict(behavior_reasons),
        "candidate_sampling_summary": {
            **dict(sampling_totals),
            "sampling_excluded_by_reason_counts": dict(sampling_excluded_by_reason_counts),
            "traced_over_legal_candidate_ratio": (
                safe_float(sampling_totals.get("sampling_included_candidate_count"))
                / safe_float(sampling_totals.get("legal_candidate_count_total"))
                if sampling_totals.get("legal_candidate_count_total")
                else 0.0
            ),
        },
        "branch_evidence_cache_summary": branch_cache.summary() if branch_cache else None,
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "closed_loop_test_is_utility_diagnostic_not_training_label": True,
        },
    }
    assert_no_label_leak(summary, label="strict evidence ab summary")
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
