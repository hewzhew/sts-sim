#!/usr/bin/env python3
"""Closed-loop A/B for run-level counterfactual targets.

This runner differs from the older combat strict-evidence runner in one way:
it only requests branch evidence at decisions named by run_counterfactual_targets_v1.
The target file is not an action label. At a matching decision this runner asks
the simulator for branch outcomes and overrides only if complete evidence passes
the registered gate. Otherwise it abstains and records why.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from branch_evidence_cache import BranchEvidenceCache, branch_request_semantic_key
from collect_branch_traces import DriverClient, default_driver_path
from run_strict_evidence_policy_ab import (
    assert_no_label_leak,
    complete_trace,
    forced_action_id,
    safe_float,
    safe_int,
    strict_pair_set,
    trace_outcome,
)


POLICY_UNDER_TEST = "targeted_counterfactual_policy_v0"
BASELINE_POLICY = "baseline_behavior"


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def parse_seed_args(args: argparse.Namespace) -> list[int]:
    seeds: list[int] = []
    if args.seeds:
        seeds.extend(args.seeds)
    if args.seed_start is not None and args.episodes:
        seeds.extend(args.seed_start + index * args.seed_step for index in range(args.episodes))
    return sorted(dict.fromkeys(seeds))


def targets_by_seed_step(target_payload: dict[str, Any]) -> dict[int, dict[int, list[dict[str, Any]]]]:
    table: dict[int, dict[int, list[dict[str, Any]]]] = defaultdict(lambda: defaultdict(list))
    for target in target_payload.get("targets") or []:
        if target.get("target_type") != "decision_counterfactual_target":
            continue
        step = target.get("decision_step")
        seed = target.get("seed")
        if not isinstance(seed, int) or not isinstance(step, int):
            continue
        table[seed][step].append(target)
    return table


def normalize_decision_type(value: Any) -> str:
    return str(value or "unknown").strip().lower()


def candidate_keys(candidates: list[dict[str, Any]]) -> set[str]:
    keys: set[str] = set()
    for candidate in candidates:
        key = candidate.get("action_key")
        if key is not None:
            keys.add(str(key))
    return keys


def target_candidate_keys(target: dict[str, Any]) -> set[str]:
    return {str(key) for key in (target.get("candidate_action_keys") or []) if key is not None}


def candidate_key_allowed_by_target(target: dict[str, Any], key: str) -> bool:
    family = str(target.get("target_family") or "")
    failure_class = str(target.get("source_failure_class") or "")
    if family == "route_to_shop":
        return key.startswith("map/")
    if family == "shop_purchase":
        return key.startswith("shop/buy_") or key.startswith("shop/purge_card/")
    if family == "shop_card":
        return key.startswith("shop/buy_card/")
    if family == "card_reward":
        return key.startswith("reward/claim/")
    if family == "campfire_upgrade":
        return key.startswith("campfire/smith/")
    if family == "campfire_smith_rest_counterfactual":
        if failure_class in {
            "low_upgrade_conversion",
            "low_damage_readiness",
            "low_block_readiness",
        }:
            return key.startswith("campfire/smith/")
        return key == "campfire/rest" or key.startswith("campfire/smith/")
    return True


def candidate_key_allowed_by_any_target(targets: list[dict[str, Any]], key: str) -> bool:
    return any(candidate_key_allowed_by_target(target, key) for target in targets)


def compatible_targets_for_current_decision(
    targets: list[dict[str, Any]],
    *,
    decision_type: str,
    candidates: list[dict[str, Any]],
) -> tuple[list[dict[str, Any]], Counter[str]]:
    """Filter recorded open-loop targets against the current closed-loop decision.

    Step numbers are not a stable identity once an override changes the run. A
    target may only request evidence if the live decision still has the same
    decision type and at least one originally recorded candidate action key.
    """

    reasons: Counter[str] = Counter()
    current_type = normalize_decision_type(decision_type)
    current_keys = candidate_keys(candidates)
    compatible: list[dict[str, Any]] = []
    for target in targets:
        target_type = normalize_decision_type(target.get("decision_type"))
        if target_type != current_type:
            reasons["target_step_mismatch_after_closed_loop"] += 1
            continue
        keys = target_candidate_keys(target)
        if keys and current_keys.isdisjoint(keys):
            reasons["target_candidate_key_mismatch_after_closed_loop"] += 1
            continue
        compatible.append(target)
    return compatible, reasons


def action_indices_for_targets(
    candidates: list[dict[str, Any]],
    targets: list[dict[str, Any]],
    *,
    behavior_action_id: int,
    max_candidates: int,
) -> list[int]:
    target_keys: set[str] = set()
    for target in targets:
        target_keys.update(target_candidate_keys(target))

    indices: list[int] = []
    for index, candidate in enumerate(candidates):
        key = candidate.get("action_key")
        key_text = str(key)
        if target_keys and key_text not in target_keys:
            continue
        if not candidate_key_allowed_by_any_target(targets, key_text):
            continue
        indices.append(index)
        if len(indices) >= max_candidates:
            break
    if behavior_action_id not in indices and 0 <= behavior_action_id < len(candidates):
        indices.append(behavior_action_id)
    return sorted(dict.fromkeys(indices))


def has_non_behavior_candidate(indices: list[int], behavior_action_id: int) -> bool:
    return any(index != behavior_action_id for index in indices)


def validate_cached_payload_identity(payload: dict[str, Any], identity: dict[str, Any]) -> bool:
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


def candidate_action_summary(candidates: list[dict[str, Any]], action_id: int | None) -> dict[str, Any]:
    if not isinstance(action_id, int) or action_id < 0 or action_id >= len(candidates):
        return {"action_id": action_id, "action_kind": "unknown", "action_key": None}
    candidate = candidates[action_id]
    return {
        "action_id": action_id,
        "action_kind": candidate.get("action_kind"),
        "action_key": candidate.get("action_key"),
    }


def choose_targeted_action(
    branch_payload: dict[str, Any],
    *,
    behavior_action_id: int,
    min_hp_margin: int,
    min_reward_margin: float,
) -> dict[str, Any]:
    traces = branch_payload.get("traces") or []
    candidates = traces[0].get("candidates") if traces else []
    candidates = candidates if isinstance(candidates, list) else []
    behavior_trace = next(
        (trace for trace in traces if forced_action_id(trace) == behavior_action_id),
        None,
    )
    base = {
        "schema_version": "targeted_counterfactual_decision_v0",
        "trainable_as_action_label": False,
        "mode": "abstain",
        "reason": "no_strict_material_alternative",
        "behavior_action_id": behavior_action_id,
        "chosen_action_id": behavior_action_id,
        "behavior_action": candidate_action_summary(candidates, behavior_action_id),
        "candidate_action": None,
        "strict_candidate_count": 0,
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
        if not isinstance(behavior_branch_id, str) or not isinstance(action_id, int):
            continue
        if frozenset((branch_id, behavior_branch_id)) not in strict_pairs:
            continue
        strict_candidate_count += 1
        if not complete_trace(trace):
            continue
        candidate_outcome = trace.get("outcome") or {}
        behavior_outcome = behavior_trace.get("outcome") or {}
        cand_dead = candidate_outcome.get("result") == "defeat"
        beh_dead = behavior_outcome.get("result") == "defeat"
        if cand_dead and not beh_dead:
            continue
        hp_gain = safe_int(candidate_outcome.get("hp")) - safe_int(behavior_outcome.get("hp"))
        reward_gain = safe_float(candidate_outcome.get("total_reward")) - safe_float(
            behavior_outcome.get("total_reward")
        )
        combat_gain = safe_int(candidate_outcome.get("combat_win_count")) - safe_int(
            behavior_outcome.get("combat_win_count")
        )
        floor_gain = safe_int(candidate_outcome.get("floor")) - safe_int(
            behavior_outcome.get("floor")
        )
        if not cand_dead and beh_dead:
            reason = "survival_flip"
        elif combat_gain < 0 or floor_gain < 0:
            continue
        elif hp_gain >= min_hp_margin and reward_gain >= -abs(min_reward_margin):
            reason = "hp_margin"
        else:
            continue
        alternatives.append(
            {
                "action_id": action_id,
                "branch_id": branch_id,
                "material_reason": reason,
                "hp_gain_vs_behavior": hp_gain,
                "reward_gain_vs_behavior": reward_gain,
                "combat_win_count_gain_vs_behavior": combat_gain,
                "floor_gain_vs_behavior": floor_gain,
                "candidate_outcome": trace_outcome(trace),
            }
        )
    if not alternatives:
        return {**base, "strict_candidate_count": strict_candidate_count}
    alternatives.sort(
        key=lambda row: (
            -safe_int(row.get("floor_gain_vs_behavior")),
            -safe_int(row.get("combat_win_count_gain_vs_behavior")),
            -safe_int(row.get("hp_gain_vs_behavior")),
            -safe_float(row.get("reward_gain_vs_behavior")),
            safe_int(row.get("action_id"), 9999),
        )
    )
    best = alternatives[0]
    chosen_id = safe_int(best.get("action_id"), behavior_action_id)
    return {
        **base,
        "mode": "counterfactual_override",
        "reason": best.get("material_reason"),
        "chosen_action_id": chosen_id,
        "strict_candidate_count": strict_candidate_count,
        "material_alternative_count": len(alternatives),
        "candidate_action": candidate_action_summary(candidates, chosen_id),
        "hp_gain_vs_behavior": best.get("hp_gain_vs_behavior"),
        "reward_gain_vs_behavior": best.get("reward_gain_vs_behavior"),
        "combat_win_count_gain_vs_behavior": best.get("combat_win_count_gain_vs_behavior"),
        "floor_gain_vs_behavior": best.get("floor_gain_vs_behavior"),
        "behavior_outcome": trace_outcome(behavior_trace),
        "candidate_outcome": best.get("candidate_outcome"),
    }


def run_episode(
    client: DriverClient,
    *,
    seed: int,
    policy_kind: str,
    args: argparse.Namespace,
    target_table: dict[int, dict[int, list[dict[str, Any]]]],
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
    target_hit_count = 0
    target_miss_count = 0
    raw_target_step_count = 0
    target_step_mismatch_after_closed_loop_count = 0
    target_candidate_key_mismatch_after_closed_loop_count = 0
    abstain_count = 0
    override_count = 0
    branch_trace_count = 0
    comparison_count = 0
    validation_issue_count = 0
    censored_trace_count = 0
    truncated_trace_count = 0
    evidence_request_count = 0
    cache_hit_count = 0
    cache_miss_count = 0
    abstain_reasons: Counter[str] = Counter()
    override_reasons: Counter[str] = Counter()
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

        raw_matching_targets = target_table.get(seed, {}).get(steps, [])
        matching_targets: list[dict[str, Any]] = []
        if policy_kind == POLICY_UNDER_TEST and raw_matching_targets:
            raw_target_step_count += len(raw_matching_targets)
            mismatch_reasons: Counter[str] = Counter()
            matching_targets, mismatch_reasons = compatible_targets_for_current_decision(
                raw_matching_targets,
                decision_type=str(decision_type),
                candidates=candidates if isinstance(candidates, list) else [],
            )
            target_step_mismatch_after_closed_loop_count += mismatch_reasons[
                "target_step_mismatch_after_closed_loop"
            ]
            target_candidate_key_mismatch_after_closed_loop_count += mismatch_reasons[
                "target_candidate_key_mismatch_after_closed_loop"
            ]
            if not matching_targets:
                target_miss_count += len(raw_matching_targets)
                abstain_count += len(raw_matching_targets)
                abstain_reasons.update(mismatch_reasons)
                step_record = {
                    "schema_version": "targeted_counterfactual_step_record_v0",
                    "trainable_as_action_label": False,
                    "episode_seed": seed,
                    "episode_step": steps,
                    "decision_type": decision_type,
                    "policy_kind": policy_kind,
                    "target_ids": [target.get("target_id") for target in raw_matching_targets],
                    "target_families": sorted(
                        {str(target.get("target_family")) for target in raw_matching_targets}
                    ),
                    "behavior_action_id": behavior_action_id,
                    "behavior_action_key": preview.get("chosen_action_key"),
                    "decision": {
                        "schema_version": "targeted_counterfactual_decision_v0",
                        "trainable_as_action_label": False,
                        "mode": "abstain",
                        "reason": "closed_loop_target_identity_mismatch",
                        "behavior_action_id": behavior_action_id,
                        "chosen_action_id": behavior_action_id,
                        "behavior_action": candidate_action_summary(candidates, behavior_action_id),
                        "candidate_action": None,
                        "mismatch_reason_counts": dict(mismatch_reasons),
                    },
                    "branch_trace_count": 0,
                    "comparison_count": 0,
                    "validation_issue_count": 0,
                    "branch_evidence_cache_hit": None,
                }
                assert_no_label_leak(step_record, label="targeted counterfactual mismatch step")
                trace_out.write(json.dumps(step_record, separators=(",", ":")) + "\n")
            else:
                target_hit_count += len(matching_targets)

        if policy_kind == POLICY_UNDER_TEST and matching_targets:
            action_indices = action_indices_for_targets(
                candidates if isinstance(candidates, list) else [],
                matching_targets,
                behavior_action_id=behavior_action_id,
                max_candidates=args.max_candidates,
            )
            if not has_non_behavior_candidate(action_indices, behavior_action_id):
                target_miss_count += len(matching_targets)
                abstain_count += len(matching_targets)
                abstain_reasons["target_no_compatible_candidate_after_role_filter"] += len(
                    matching_targets
                )
                step_record = {
                    "schema_version": "targeted_counterfactual_step_record_v0",
                    "trainable_as_action_label": False,
                    "episode_seed": seed,
                    "episode_step": steps,
                    "decision_type": decision_type,
                    "policy_kind": policy_kind,
                    "target_ids": [target.get("target_id") for target in matching_targets],
                    "target_families": sorted(
                        {str(target.get("target_family")) for target in matching_targets}
                    ),
                    "behavior_action_id": behavior_action_id,
                    "behavior_action_key": preview.get("chosen_action_key"),
                    "decision": {
                        "schema_version": "targeted_counterfactual_decision_v0",
                        "trainable_as_action_label": False,
                        "mode": "abstain",
                        "reason": "target_no_compatible_candidate_after_role_filter",
                        "behavior_action_id": behavior_action_id,
                        "chosen_action_id": behavior_action_id,
                        "behavior_action": candidate_action_summary(candidates, behavior_action_id),
                        "candidate_action": None,
                    },
                    "branch_action_indices": action_indices,
                    "branch_trace_count": 0,
                    "comparison_count": 0,
                    "validation_issue_count": 0,
                    "branch_evidence_cache_hit": None,
                }
                assert_no_label_leak(
                    step_record,
                    label="targeted counterfactual role-filter abstain step",
                )
                trace_out.write(json.dumps(step_record, separators=(",", ":")) + "\n")
                matching_targets = []

        if policy_kind == POLICY_UNDER_TEST and matching_targets:
            action_indices = action_indices_for_targets(
                candidates if isinstance(candidates, list) else [],
                matching_targets,
                behavior_action_id=behavior_action_id,
                max_candidates=args.max_candidates,
            )
            branch_request = {
                "cmd": "branch_trace",
                "action_indices": action_indices,
                "candidate_scope": "all",
                "candidate_sampling_spec_id": "targeted_counterfactual_target_keys_v1",
                "candidate_cap": args.max_candidates,
                "behavior_action_id": behavior_action_id,
                "sampling_seed": seed,
                "continuation_policy": args.continuation_policy,
                "horizon_decisions": args.horizon_decisions,
                "horizon_mode": args.horizon_mode,
                "sim_version": "full_run_env_targeted_counterfactual_policy_v0",
                "content_version": "content_current",
                "include_comparisons": True,
            }
            evidence_request_count += 1
            branch_payload, cache_hit = request_branch_trace(
                client, branch_request, cache=branch_cache
            )
            if cache_hit:
                cache_hit_count += 1
            else:
                cache_miss_count += 1
            branch_trace_count += safe_int(branch_payload.get("trace_count"))
            comparison_count += safe_int(branch_payload.get("comparison_count"))
            validation = branch_payload.get("validation_report") or {}
            validation_issue_count += safe_int(validation.get("issue_count"))
            for trace in branch_payload.get("traces") or []:
                outcome = trace.get("outcome") or {}
                if outcome.get("outcome_censored"):
                    censored_trace_count += 1
                if outcome.get("truncated"):
                    truncated_trace_count += 1
            decision = choose_targeted_action(
                branch_payload,
                behavior_action_id=behavior_action_id,
                min_hp_margin=args.min_hp_margin,
                min_reward_margin=args.min_reward_margin,
            )
            chosen_action_id = safe_int(decision.get("chosen_action_id"), behavior_action_id)
            if decision.get("mode") == "counterfactual_override":
                override_count += 1
                override_reasons[str(decision.get("reason") or "unknown")] += 1
            else:
                abstain_count += 1
                abstain_reasons[str(decision.get("reason") or "unknown")] += 1
            step_record = {
                "schema_version": "targeted_counterfactual_step_record_v0",
                "trainable_as_action_label": False,
                "episode_seed": seed,
                "episode_step": steps,
                "decision_type": decision_type,
                "policy_kind": policy_kind,
                "target_ids": [target.get("target_id") for target in matching_targets],
                "target_families": sorted(
                    {str(target.get("target_family")) for target in matching_targets}
                ),
                "raw_target_ids_at_step": [
                    target.get("target_id") for target in raw_matching_targets
                ],
                "target_action_key_intersection_count": len(
                    candidate_keys(candidates if isinstance(candidates, list) else [])
                    & set().union(*(target_candidate_keys(target) for target in matching_targets))
                    if matching_targets
                    else set()
                ),
                "branch_action_indices": action_indices,
                "behavior_action_id": behavior_action_id,
                "behavior_action_key": preview.get("chosen_action_key"),
                "decision": decision,
                "branch_trace_count": safe_int(branch_payload.get("trace_count")),
                "comparison_count": safe_int(branch_payload.get("comparison_count")),
                "validation_issue_count": safe_int(validation.get("issue_count")),
                "branch_evidence_cache_hit": cache_hit,
            }
            assert_no_label_leak(step_record, label="targeted counterfactual step")
            trace_out.write(json.dumps(step_record, separators=(",", ":")) + "\n")
        elif policy_kind == POLICY_UNDER_TEST:
            if target_table.get(seed):
                target_miss_count += 0

        step = client.request({"cmd": "decision_env_step", "action_id": chosen_action_id})
        reward = safe_float(step.get("reward"))
        total_reward += reward
        done = bool(step.get("done"))
        final_info = step.get("info") or {}
        steps += 1

    return {
        "schema_version": "targeted_counterfactual_episode_result_v0",
        "trainable_as_action_label": False,
        "seed": seed,
        "policy_kind": policy_kind,
        "steps": steps,
        "done": done,
        "total_reward": total_reward,
        "target_count_for_seed": sum(len(v) for v in target_table.get(seed, {}).values()),
        "raw_target_step_count": raw_target_step_count,
        "target_hit_count": target_hit_count,
        "target_miss_count": target_miss_count,
        "target_step_mismatch_after_closed_loop_count": target_step_mismatch_after_closed_loop_count,
        "target_candidate_key_mismatch_after_closed_loop_count": target_candidate_key_mismatch_after_closed_loop_count,
        "evidence_request_count": evidence_request_count,
        "abstain_count": abstain_count,
        "abstain_reason_counts": dict(abstain_reasons),
        "override_count": override_count,
        "override_reason_counts": dict(override_reasons),
        "branch_trace_count": branch_trace_count,
        "comparison_count": comparison_count,
        "validation_issue_count": validation_issue_count,
        "censored_trace_count": censored_trace_count,
        "truncated_trace_count": truncated_trace_count,
        "branch_cache_hit_count": cache_hit_count,
        "branch_cache_miss_count": cache_miss_count,
        "final_info": final_info,
        "final_result": final_info.get("result"),
        "terminal_reason": final_info.get("terminal_reason"),
        "floor": safe_int(final_info.get("floor")),
        "act": safe_int(final_info.get("act")),
        "hp": safe_int(final_info.get("hp")),
        "max_hp": safe_int(final_info.get("max_hp")),
        "combat_win_count": safe_int(final_info.get("combat_win_count")),
        "gold": safe_int(final_info.get("gold")),
        "deck_size": safe_int(final_info.get("deck_size")),
    }


def aggregate(results: list[dict[str, Any]], target_payload: dict[str, Any]) -> dict[str, Any]:
    by_seed: dict[int, dict[str, dict[str, Any]]] = defaultdict(dict)
    for result in results:
        by_seed[safe_int(result.get("seed"))][str(result.get("policy_kind"))] = result
    paired = []
    for seed, policies in sorted(by_seed.items()):
        baseline = policies.get(BASELINE_POLICY)
        variant = policies.get(POLICY_UNDER_TEST)
        if baseline is None or variant is None:
            continue
        paired.append(
            {
                "seed": seed,
                "floor_delta": safe_int(variant.get("floor")) - safe_int(baseline.get("floor")),
                "hp_delta": safe_int(variant.get("hp")) - safe_int(baseline.get("hp")),
                "combat_win_delta": safe_int(variant.get("combat_win_count"))
                - safe_int(baseline.get("combat_win_count")),
                "reward_delta": safe_float(variant.get("total_reward"))
                - safe_float(baseline.get("total_reward")),
                "result_changed": baseline.get("final_result") != variant.get("final_result"),
                "override_count": safe_int(variant.get("override_count")),
                "target_hit_count": safe_int(variant.get("target_hit_count")),
                "abstain_count": safe_int(variant.get("abstain_count")),
                "baseline": {
                    "result": baseline.get("final_result"),
                    "act": baseline.get("act"),
                    "floor": baseline.get("floor"),
                    "hp": baseline.get("hp"),
                    "combat_win_count": baseline.get("combat_win_count"),
                },
                "variant": {
                    "result": variant.get("final_result"),
                    "act": variant.get("act"),
                    "floor": variant.get("floor"),
                    "hp": variant.get("hp"),
                    "combat_win_count": variant.get("combat_win_count"),
                    "override_count": variant.get("override_count"),
                },
            }
        )

    def result_rank(result: Any) -> int:
        return {"defeat": 1, "ongoing": 2, "victory": 3}.get(str(result), 0)

    bad = 0
    improved = 0
    death_regressions = 0
    for row in paired:
        baseline = row.get("baseline") or {}
        variant = row.get("variant") or {}
        worsened = result_rank(variant.get("result")) < result_rank(baseline.get("result"))
        if variant.get("result") == "defeat" and baseline.get("result") != "defeat":
            death_regressions += 1
        is_bad = (
            worsened
            or safe_int(row.get("floor_delta")) < 0
            or safe_int(row.get("combat_win_delta")) < 0
            or safe_float(row.get("reward_delta")) < -1.0
        )
        is_improved = (
            result_rank(variant.get("result")) > result_rank(baseline.get("result"))
            or safe_int(row.get("floor_delta")) > 0
            or safe_int(row.get("combat_win_delta")) > 0
            or safe_int(row.get("hp_delta")) > 0
            or safe_float(row.get("reward_delta")) > 1.0
        )
        if is_bad:
            bad += 1
        elif is_improved:
            improved += 1

    unavailable_target_count = safe_int(target_payload.get("unavailable_target_count"))
    total_evidence_requests = sum(
        safe_int(result.get("evidence_request_count"))
        for result in results
        if result.get("policy_kind") == POLICY_UNDER_TEST
    )
    total_overrides = sum(
        safe_int(result.get("override_count"))
        for result in results
        if result.get("policy_kind") == POLICY_UNDER_TEST
    )
    total_censored = sum(
        safe_int(result.get("censored_trace_count"))
        for result in results
        if result.get("policy_kind") == POLICY_UNDER_TEST
    )
    total_validation = sum(
        safe_int(result.get("validation_issue_count"))
        for result in results
        if result.get("policy_kind") == POLICY_UNDER_TEST
    )
    total_step_mismatch = sum(
        safe_int(result.get("target_step_mismatch_after_closed_loop_count"))
        for result in results
        if result.get("policy_kind") == POLICY_UNDER_TEST
    )
    total_candidate_key_mismatch = sum(
        safe_int(result.get("target_candidate_key_mismatch_after_closed_loop_count"))
        for result in results
        if result.get("policy_kind") == POLICY_UNDER_TEST
    )
    target_role_filter_abstain = sum(
        safe_int((result.get("abstain_reason_counts") or {}).get(
            "target_no_compatible_candidate_after_role_filter"
        ))
        for result in results
        if result.get("policy_kind") == POLICY_UNDER_TEST
    )
    failure_report = build_failure_report(
        paired=paired,
        target_payload=target_payload,
        total_evidence_requests=total_evidence_requests,
        total_overrides=total_overrides,
        total_censored=total_censored,
        total_validation=total_validation,
        total_step_mismatch=total_step_mismatch,
        total_candidate_key_mismatch=total_candidate_key_mismatch,
        target_role_filter_abstain=target_role_filter_abstain,
        bad_outcome_seed_count=bad,
    )
    return {
        "schema_version": "targeted_counterfactual_ab_paired_summary_v0",
        "paired_seed_count": len(paired),
        "paired_deltas": paired,
        "sum_floor_delta": sum(safe_int(row.get("floor_delta")) for row in paired),
        "sum_hp_delta": sum(safe_int(row.get("hp_delta")) for row in paired),
        "sum_combat_win_delta": sum(safe_int(row.get("combat_win_delta")) for row in paired),
        "sum_reward_delta": sum(safe_float(row.get("reward_delta")) for row in paired),
        "total_evidence_request_count": total_evidence_requests,
        "total_override_count": total_overrides,
        "target_step_mismatch_after_closed_loop_count": total_step_mismatch,
        "target_candidate_key_mismatch_after_closed_loop_count": total_candidate_key_mismatch,
        "target_no_compatible_candidate_after_role_filter_count": target_role_filter_abstain,
        "total_abstain_count": sum(
            safe_int(result.get("abstain_count"))
            for result in results
            if result.get("policy_kind") == POLICY_UNDER_TEST
        ),
        "bad_outcome_seed_count": bad,
        "improved_outcome_seed_count": improved,
        "death_regression_count": death_regressions,
        "unavailable_target_count": unavailable_target_count,
        "candidate_snapshot_missing_target_count": sum(
            1
            for row in target_payload.get("unavailable_targets") or []
            if row.get("reason") == "candidate_snapshot_missing"
        ),
        "failure_report": failure_report,
    }


def build_failure_report(
    *,
    paired: list[dict[str, Any]],
    target_payload: dict[str, Any],
    total_evidence_requests: int,
    total_overrides: int,
    total_censored: int,
    total_validation: int,
    total_step_mismatch: int,
    total_candidate_key_mismatch: int,
    target_role_filter_abstain: int,
    bad_outcome_seed_count: int,
) -> dict[str, Any]:
    buckets: dict[str, dict[str, Any]] = {}

    def add(bucket: str, evidence: dict[str, Any], repair: str) -> None:
        buckets[bucket] = {
            "bucket": bucket,
            "evidence": evidence,
            "repair_experiment": repair,
        }

    unavailable = target_payload.get("unavailable_targets") or []
    unavailable_by_reason = Counter(str(row.get("reason")) for row in unavailable)
    missing_snapshots = [
        row for row in unavailable if row.get("reason") == "candidate_snapshot_missing"
    ]
    if missing_snapshots:
        add(
            "candidate_snapshot_missing",
            {
                "count": len(missing_snapshots),
                "sample_seeds": [row.get("seed") for row in missing_snapshots[:12]],
                "reason_counts": dict(unavailable_by_reason),
            },
            "Regenerate readable runs with noncombat candidate snapshots enabled; do not infer missing candidates from chosen actions.",
        )
    target_unavailable = [
        row for row in unavailable if row.get("reason") == "counterfactual_target_unavailable"
    ]
    if target_unavailable:
        add(
            "counterfactual_target_unavailable",
            {
                "count": len(target_unavailable),
                "sample_seeds": [row.get("seed") for row in target_unavailable[:12]],
                "reason_counts": dict(unavailable_by_reason),
            },
            "Add target matching for broader opportunity windows or collect decisions that expose this target family; do not fabricate absent options.",
        )
    if total_evidence_requests == 0:
        add(
            "counterfactual_target_unavailable",
            {
                "target_count": safe_int(target_payload.get("target_count")),
                "paired_seed_count": len(paired),
            },
            "Inspect target step alignment against closed-loop decision steps and add target matching by action_key/decision identity, not only step index.",
        )
    if total_censored:
        add(
            "evidence_horizon_too_short",
            {"censored_trace_count": total_censored},
            "Increase or specialize horizon only for the affected target families; keep censored traces abstained.",
        )
    if total_step_mismatch or total_candidate_key_mismatch:
        add(
            "open_loop_to_closed_loop_mismatch",
            {
                "target_step_mismatch_after_closed_loop_count": total_step_mismatch,
                "target_candidate_key_mismatch_after_closed_loop_count": total_candidate_key_mismatch,
            },
            "Use stable decision fingerprints and opportunity-window matching before interpreting closed-loop target utility.",
        )
    if target_role_filter_abstain:
        add(
            "target_role_filter_gap",
            {
                "target_no_compatible_candidate_after_role_filter_count": target_role_filter_abstain,
            },
            "Split target families by candidate role before closed-loop use; examples include upgrade-targets that must not become rest-targets.",
        )
    if total_overrides and bad_outcome_seed_count:
        bad_rows = [row for row in paired if safe_int(row.get("floor_delta")) < 0 or safe_int(row.get("combat_win_delta")) < 0]
        add(
            "bad_override_tail",
            {
                "bad_outcome_seed_count": bad_outcome_seed_count,
                "sample_seeds": [row.get("seed") for row in bad_rows[:12]],
            },
            "Add a second confirmation horizon and tail-risk veto for target families that caused regressions.",
        )
    if total_evidence_requests and (
        total_overrides == 0 or total_overrides * 100 < total_evidence_requests
    ):
        add(
            "low_yield_high_cost",
            {
                "evidence_request_count": total_evidence_requests,
                "override_count": total_overrides,
                "override_rate_threshold_source": "preregistered_low_yield_gate: overrides * 100 < evidence_requests",
            },
            "Rank targets by historical material-yield bucket before requesting branch evidence; audit top-decile yield separately.",
        )
    if total_validation:
        add(
            "branch_validation_issue",
            {"validation_issue_count": total_validation},
            "Fix branch dataset validation issues before interpreting A/B utility.",
        )
    if not buckets:
        add(
            "no_blocking_failure_bucket_observed",
            {
                "evidence_request_count": total_evidence_requests,
                "override_count": total_overrides,
                "bad_outcome_seed_count": bad_outcome_seed_count,
            },
            "Scale seed count and compare target families independently before broadening gates.",
        )
    return {
        "schema_version": "targeted_counterfactual_failure_report_v0",
        "required_when_no_or_negative_benefit": True,
        "buckets": buckets,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver", type=Path)
    parser.add_argument("--targets", type=Path, required=True)
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
    parser.add_argument("--horizon-decisions", type=int, default=24)
    parser.add_argument("--max-candidates", type=int, default=16)
    parser.add_argument("--min-hp-margin", type=int, default=10)
    parser.add_argument("--min-reward-margin", type=float, default=0.25)
    parser.add_argument("--branch-cache-dir", type=Path)
    parser.add_argument("--disable-branch-cache", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    seeds = parse_seed_args(args)
    if not seeds:
        raise SystemExit("provide --seeds or --seed-start/--episodes")
    target_payload = load_json(args.targets)
    target_table = targets_by_seed_step(target_payload)
    driver = args.driver or default_driver_path()
    branch_cache = (
        BranchEvidenceCache(args.branch_cache_dir)
        if args.branch_cache_dir is not None and not args.disable_branch_cache
        else None
    )
    results: list[dict[str, Any]] = []
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8") as trace_out:
        for policy_kind in (BASELINE_POLICY, POLICY_UNDER_TEST):
            client = DriverClient(driver)
            try:
                for seed in seeds:
                    result = run_episode(
                        client,
                        seed=seed,
                        policy_kind=policy_kind,
                        args=args,
                        target_table=target_table,
                        trace_out=trace_out,
                        branch_cache=branch_cache if policy_kind == POLICY_UNDER_TEST else None,
                    )
                    assert_no_label_leak(result, label=f"{policy_kind} episode result")
                    results.append(result)
                    trace_out.write(json.dumps(result, separators=(",", ":")) + "\n")
            finally:
                client.close()

    final_results: Counter[str] = Counter()
    abstain_reasons: Counter[str] = Counter()
    override_reasons: Counter[str] = Counter()
    target_step_mismatch_count = 0
    target_candidate_key_mismatch_count = 0
    for result in results:
        final_results[f"{result.get('policy_kind')}|{result.get('final_result')}"] += 1
        if result.get("policy_kind") == POLICY_UNDER_TEST:
            abstain_reasons.update(result.get("abstain_reason_counts") or {})
            override_reasons.update(result.get("override_reason_counts") or {})
            target_step_mismatch_count += safe_int(
                result.get("target_step_mismatch_after_closed_loop_count")
            )
            target_candidate_key_mismatch_count += safe_int(
                result.get("target_candidate_key_mismatch_after_closed_loop_count")
            )

    summary = {
        "schema_version": "targeted_counterfactual_ab_summary_v0",
        "policy_under_test": POLICY_UNDER_TEST,
        "baseline_policy": args.behavior_policy,
        "seed_count": len(seeds),
        "seeds": seeds,
        "target_source": str(args.targets),
        "target_summary": {
            "target_count": safe_int(target_payload.get("target_count")),
            "unavailable_target_count": safe_int(target_payload.get("unavailable_target_count")),
            "target_family_counts": target_payload.get("target_family_counts") or {},
        },
        "config": {
            "horizon_mode": args.horizon_mode,
            "horizon_decisions": args.horizon_decisions,
            "max_candidates": args.max_candidates,
            "min_hp_margin": args.min_hp_margin,
            "min_reward_margin": args.min_reward_margin,
            "max_steps": args.max_steps,
            "env_max_steps": args.env_max_steps,
        },
        "episode_results": results,
        "paired_summary": aggregate(results, target_payload),
        "final_result_counts": dict(final_results),
        "abstain_reason_counts": dict(abstain_reasons),
        "override_reason_counts": dict(override_reasons),
        "target_step_mismatch_after_closed_loop_count": target_step_mismatch_count,
        "target_candidate_key_mismatch_after_closed_loop_count": target_candidate_key_mismatch_count,
        "branch_evidence_cache_summary": branch_cache.summary() if branch_cache else None,
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "failure_classes_used_as_action_labels": False,
            "closed_loop_test_is_utility_diagnostic_not_training_label": True,
        },
    }
    assert_no_label_leak(summary, label="targeted counterfactual summary")
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps({k: v for k, v in summary.items() if k != "episode_results"}, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
