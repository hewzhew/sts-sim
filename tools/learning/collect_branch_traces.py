#!/usr/bin/env python3
"""Collect BranchTraceV1 records from full_run_env_driver.

This is a data collection tool, not a policy trainer. It follows a behavior
policy through the DecisionEnv, asks the Rust driver to evaluate candidate
branches from each decision point, and writes versioned branch outcome records.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]


def default_driver_path() -> Path:
    suffix = ".exe" if sys.platform.startswith("win") else ""
    release = REPO_ROOT / "target" / "release" / f"full_run_env_driver{suffix}"
    debug = REPO_ROOT / "target" / "debug" / f"full_run_env_driver{suffix}"
    return release if release.exists() else debug


class DriverClient:
    def __init__(self, driver_path: Path) -> None:
        self.proc = subprocess.Popen(
            [str(driver_path)],
            cwd=REPO_ROOT,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
        )

    def request(self, payload: dict[str, Any]) -> dict[str, Any]:
        assert self.proc.stdin is not None
        assert self.proc.stdout is not None
        self.proc.stdin.write(json.dumps(payload, separators=(",", ":")) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            stderr = self.proc.stderr.read() if self.proc.stderr else ""
            raise RuntimeError(f"driver closed stdout; stderr={stderr}")
        response = json.loads(line)
        if not response.get("ok"):
            raise RuntimeError(response.get("error") or f"driver request failed: {payload}")
        return response

    def close(self) -> None:
        if self.proc.poll() is None:
            try:
                self.request({"cmd": "close"})
            except Exception:
                pass
        if self.proc.poll() is None:
            self.proc.terminate()


def bump(counter: dict[str, Any], key: str, amount: int = 1) -> None:
    counter[key] = int(counter.get(key) or 0) + amount


def bump_nested(counter: dict[str, Any], outer: str, inner: str, amount: int = 1) -> None:
    bucket = counter.setdefault(outer, {})
    bucket[inner] = int(bucket.get(inner) or 0) + amount


def action_kind(candidates: list[dict[str, Any]], action_index: Any) -> str:
    if not isinstance(action_index, int):
        return "unknown"
    if action_index < 0 or action_index >= len(candidates):
        return "invalid_index"
    return str(candidates[action_index].get("action_kind") or "unknown")


def forced_action_index(trace: dict[str, Any]) -> int | None:
    forced = trace.get("forced_prefix") or []
    if not forced:
        return None
    value = forced[0]
    return value if isinstance(value, int) else None


def trace_action_kind(trace: dict[str, Any]) -> str:
    return action_kind(trace.get("candidates") or [], forced_action_index(trace))


def trace_data_role(trace: dict[str, Any]) -> str:
    outcome = trace.get("outcome") or {}
    requested = outcome.get("boundary_requested") or "unknown"
    reached = bool(outcome.get("boundary_reached"))
    censored = bool(outcome.get("outcome_censored"))
    result = outcome.get("result") or "unknown"
    if result == "defeat":
        return "death_observed_branch"
    if requested == "combat_end" and reached and not censored:
        return "combat_end_complete_branch"
    if requested == "combat_end" and censored:
        return "censored_partial_combat_branch"
    if requested == "fixed_decisions":
        return "fixed_horizon_branch"
    return "debug_or_unclassified_branch"


def comparison_data_role(
    comparison: dict[str, Any],
    left: dict[str, Any] | None,
    right: dict[str, Any] | None,
) -> str:
    if left is None or right is None:
        return "missing_trace_debug_only"
    if not comparison.get("pairing_valid", False):
        return "invalid_pairing_debug_only"
    left_outcome = left.get("outcome") or {}
    right_outcome = right.get("outcome") or {}
    if left_outcome.get("outcome_censored") or right_outcome.get("outcome_censored"):
        return "censored_partial_pair"
    requested = left_outcome.get("boundary_requested") or "unknown"
    reached = bool(left_outcome.get("boundary_reached")) and bool(
        right_outcome.get("boundary_reached")
    )
    if requested == "combat_end" and reached:
        if comparison.get("rng_diverged") is True:
            return "combat_end_complete_pair_rng_diverged"
        return "combat_end_complete_pair_rng_aligned"
    if requested == "fixed_decisions":
        if comparison.get("rng_diverged") is True:
            return "fixed_horizon_pair_rng_diverged"
        return "fixed_horizon_pair_rng_aligned"
    return "debug_or_unclassified_pair"


def int_delta_bucket(value: int) -> str:
    if value <= -20:
        return "<=-20"
    if value <= -10:
        return "-19..-10"
    if value <= -5:
        return "-9..-5"
    if value <= -1:
        return "-4..-1"
    if value == 0:
        return "0"
    if value <= 4:
        return "1..4"
    if value <= 9:
        return "5..9"
    if value <= 19:
        return "10..19"
    return ">=20"


def reward_delta_bucket(value: float) -> str:
    if value <= -10.0:
        return "<=-10"
    if value <= -2.0:
        return "-10..-2"
    if value <= -0.1:
        return "-2..-0.1"
    if value < 0.1:
        return "~0"
    if value < 2.0:
        return "0.1..2"
    if value < 10.0:
        return "2..10"
    return ">=10"


def collect_episode(
    client: DriverClient,
    *,
    seed: int,
    ascension: int,
    final_act: bool,
    max_steps: int,
    env_max_steps: int,
    behavior_policy: str,
    continuation_policy: str,
    horizon_decisions: int,
    horizon_mode: str,
    candidate_scope: str,
    max_candidates: int,
    decision_type_prefixes: list[str],
    determinism_check_limit: int,
    out,
    summary: dict[str, Any],
) -> dict[str, Any]:
    client.request(
        {
            "cmd": "reset",
            "seed": seed,
            "ascension": ascension,
            "final_act": final_act,
            "class": "ironclad",
            "max_steps": env_max_steps,
            "reward_shaping_profile": "baseline",
        }
    )
    done = False
    records = 0
    final_info: dict[str, Any] | None = None
    while not done and records < max_steps:
        policy_input = client.request(
            {"cmd": "policy_input", "time_budget_ms": 25}
        )["payload"]
        decision_type = (
            ((policy_input.get("observation") or {}).get("decision_type"))
            or ((policy_input.get("decision_id") or {}).get("decision_type"))
            or "unknown"
        )
        summary["decision_type_counts"][decision_type] = (
            summary["decision_type_counts"].get(decision_type, 0) + 1
        )
        candidates = policy_input.get("candidates") or []
        if not candidates:
            break

        preview = client.request(
            {
                "cmd": "preview_policy_action",
                "policy": behavior_policy,
                "include_state": False,
                "include_next_state": False,
                "check_live_env_unchanged": False,
            }
        )["payload"]
        behavior_action_id = preview.get("chosen_action_index")
        should_trace = (
            not decision_type_prefixes
            or any(decision_type.startswith(prefix) for prefix in decision_type_prefixes)
        )
        if should_trace:
            summary["traced_decision_type_counts"][decision_type] = (
                summary["traced_decision_type_counts"].get(decision_type, 0) + 1
            )
            summary["legal_candidate_count_total"] += len(candidates)
            for candidate in candidates:
                bump(
                    summary["legal_candidate_action_kind_counts"],
                    str(candidate.get("action_kind") or "unknown"),
                )
            bump(summary["behavior_action_kind_counts"], action_kind(candidates, behavior_action_id))
            action_indices = list(range(min(len(candidates), max_candidates)))
            if isinstance(behavior_action_id, int) and behavior_action_id not in action_indices:
                action_indices.append(behavior_action_id)
            for action_index in action_indices:
                bump(
                    summary["requested_candidate_action_kind_counts"],
                    action_kind(candidates, action_index),
                )
            branch_request = {
                "cmd": "branch_trace",
                "action_indices": action_indices,
                "candidate_scope": candidate_scope,
                "candidate_sampling_spec_id": "first_n_plus_behavior_v1",
                "candidate_cap": max_candidates,
                "behavior_action_id": behavior_action_id,
                "sampling_seed": seed,
                "continuation_policy": continuation_policy,
                "horizon_decisions": horizon_decisions,
                "horizon_mode": horizon_mode,
                "sim_version": "full_run_env_branch_trace_v1",
                "content_version": "content_current",
                "include_comparisons": True,
            }
            branch_payload = client.request(
                branch_request
            )["payload"]
            if summary["determinism_check_count"] < determinism_check_limit:
                repeat_payload = client.request(branch_request)["payload"]
                summary["determinism_check_count"] += 1
                comparable_keys = ("action_indices", "traces", "comparisons", "validation_report")
                if any(branch_payload.get(key) != repeat_payload.get(key) for key in comparable_keys):
                    summary["determinism_mismatch_count"] += 1
            record = {
                "schema_version": "branch_trace_collection_record_v1",
                "seed": seed,
                "episode_step": records,
                "decision_type": decision_type,
                "behavior_policy": behavior_policy,
                "behavior_action_id": behavior_action_id,
                "behavior_action_key": preview.get("chosen_action_key"),
                "branch_trace_batch": branch_payload,
            }
            out.write(json.dumps(record, separators=(",", ":")) + "\n")

            summary["decision_count"] += 1
            summary["trace_count"] += int(branch_payload.get("trace_count") or 0)
            summary["comparison_count"] += int(branch_payload.get("comparison_count") or 0)
            sampling = branch_payload.get("candidate_sampling_spec") or {}
            included_indices = set(branch_payload.get("action_indices") or [])
            requested_indices = branch_payload.get("requested_action_indices") or []
            for action_index in included_indices:
                bump(
                    summary["traced_candidate_action_kind_counts"],
                    action_kind(candidates, action_index),
                )
            for action_index in requested_indices:
                if action_index not in included_indices:
                    bump(
                        summary["sampling_excluded_action_kind_counts"],
                        action_kind(candidates, action_index),
                    )
            spec_id = sampling.get("candidate_sampling_spec_id") or "unknown"
            summary["candidate_sampling_spec_id_counts"][spec_id] = (
                summary["candidate_sampling_spec_id_counts"].get(spec_id, 0) + 1
            )
            if not sampling.get("include_behavior_action", False):
                summary["sampling_missing_behavior_action_count"] += 1
            summary["sampling_requested_action_count"] += int(
                sampling.get("requested_action_count") or 0
            )
            summary["sampling_included_candidate_count"] += int(
                sampling.get("included_candidate_count") or 0
            )
            summary["sampling_excluded_candidate_count"] += int(
                sampling.get("excluded_candidate_count") or 0
            )
            sampling_scope = sampling.get("scope") or "unknown"
            summary["candidate_sampling_scope_counts"][sampling_scope] = (
                summary["candidate_sampling_scope_counts"].get(sampling_scope, 0) + 1
            )
            for reason, count in (sampling.get("excluded_by_reason") or {}).items():
                summary["sampling_excluded_by_reason_counts"][reason] = (
                    summary["sampling_excluded_by_reason_counts"].get(reason, 0)
                    + int(count or 0)
                )
            continuation_key = branch_payload.get("continuation_policy") or "unknown"
            summary["continuation_policy_counts"][continuation_key] = (
                summary["continuation_policy_counts"].get(continuation_key, 0) + 1
            )
            horizon_key = f"{branch_payload.get('horizon_mode') or 'unknown'}:{branch_payload.get('horizon_decisions') or 0}"
            summary["horizon_spec_counts"][horizon_key] = (
                summary["horizon_spec_counts"].get(horizon_key, 0) + 1
            )
            summary["reward_spec_counts"]["baseline"] = (
                summary["reward_spec_counts"].get("baseline", 0) + 1
            )
            validation = branch_payload.get("validation_report") or {}
            issue_count = int(validation.get("issue_count") or 0)
            summary["validation_issue_count"] += issue_count
            if not validation.get("valid", False):
                summary["invalid_branch_batch_count"] += 1
            for issue in validation.get("issues") or []:
                code = issue.get("code") or "unknown"
                summary["validation_issue_code_counts"][code] = (
                    summary["validation_issue_code_counts"].get(code, 0) + 1
                )
                if (
                    "redaction" in code
                    or "hidden" in code
                    or "non_public" in code
                    or "model_input" in code
                ):
                    summary["redaction_violation_count"] += 1
            if not branch_payload.get("live_env_unchanged"):
                summary["live_env_changed_count"] += 1
            traces_by_id = {
                trace.get("branch_id"): trace
                for trace in branch_payload.get("traces") or []
                if trace.get("branch_id")
            }
            for trace in branch_payload.get("traces") or []:
                if trace.get("trainable_as_action_label") is not False:
                    summary["trainable_action_label_count"] += 1
                kind = trace_action_kind(trace)
                role = trace_data_role(trace)
                bump(summary["trace_action_kind_counts"], kind)
                bump(summary["trace_data_role_counts"], role)
                bump_nested(summary["trace_data_role_by_action_kind_counts"], role, kind)
                scenario_seed_id = trace.get("scenario_seed_id")
                if scenario_seed_id:
                    summary["_scenario_seed_ids"][scenario_seed_id] = 1
                if trace.get("rng_consumed"):
                    summary["rng_consumed_trace_count"] += 1
                outcome = trace.get("outcome") or {}
                result = outcome.get("result") or "unknown"
                stop_reason = (
                    outcome.get("stop_reason")
                    or outcome.get("horizon_stop_reason")
                    or "unknown"
                )
                boundary_requested = outcome.get("boundary_requested") or "unknown"
                boundary_reached = bool(outcome.get("boundary_reached"))
                truncation_reason = outcome.get("truncation_reason") or "none"
                summary["result_counts"][result] = summary["result_counts"].get(result, 0) + 1
                summary["horizon_stop_reason_counts"][stop_reason] = (
                    summary["horizon_stop_reason_counts"].get(stop_reason, 0) + 1
                )
                summary["boundary_requested_counts"][boundary_requested] = (
                    summary["boundary_requested_counts"].get(boundary_requested, 0) + 1
                )
                boundary_key = f"{boundary_requested}:{str(boundary_reached).lower()}"
                summary["boundary_reached_counts"][boundary_key] = (
                    summary["boundary_reached_counts"].get(boundary_key, 0) + 1
                )
                summary["truncation_reason_counts"][truncation_reason] = (
                    summary["truncation_reason_counts"].get(truncation_reason, 0) + 1
                )
                if boundary_requested == "combat_end" and not boundary_reached:
                    summary["combat_end_requested_but_not_reached_count"] += 1
                if boundary_requested == "fixed_decisions":
                    summary["fixed_horizon_partial_outcome_count"] += 1
                if outcome.get("outcome_censored"):
                    summary["outcome_censored_count"] += 1
                if outcome.get("terminated"):
                    summary["terminal_trace_count"] += 1
                if outcome.get("truncated"):
                    summary["truncated_trace_count"] += 1
                if result == "defeat":
                    summary["death_trace_count"] += 1
                if int(outcome.get("combat_win_delta") or 0) > 0:
                    summary["combat_win_delta_positive_count"] += 1
                if int(outcome.get("hp_delta") or 0) < 0:
                    summary["hp_loss_trace_count"] += 1
                bump(
                    summary["trace_hp_delta_histogram"],
                    int_delta_bucket(int(outcome.get("hp_delta") or 0)),
                )
                bump(
                    summary["trace_reward_histogram"],
                    reward_delta_bucket(float(outcome.get("total_reward") or 0.0)),
                )
            for comparison in branch_payload.get("comparisons") or []:
                if "censored" in (comparison.get("comparison_scope") or ""):
                    summary["censored_comparison_count"] += 1
                trainable_role = comparison.get("trainable_role") or ""
                if "action" in trainable_role:
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
                if not comparison.get("pairing_valid", False):
                    summary["pairing_invalid_count"] += 1
                pairing_schema_version = (
                    comparison.get("pairing_schema_version") or "unknown"
                )
                summary["pairing_schema_version_counts"][pairing_schema_version] = (
                    summary["pairing_schema_version_counts"].get(pairing_schema_version, 0) + 1
                )
                pairing_mode = comparison.get("pairing_mode") or "unknown"
                summary["pairing_mode_counts"][pairing_mode] = (
                    summary["pairing_mode_counts"].get(pairing_mode, 0) + 1
                )
                common_random_policy = comparison.get("common_random_policy") or "unknown"
                summary["common_random_policy_counts"][common_random_policy] = (
                    summary["common_random_policy_counts"].get(common_random_policy, 0) + 1
                )
                paired_status = comparison.get("paired_validity_status") or "unknown"
                summary["paired_validity_status_counts"][paired_status] = (
                    summary["paired_validity_status_counts"].get(paired_status, 0) + 1
                )
                if comparison.get("rng_diverged") is True:
                    summary["rng_diverged_comparison_count"] += 1
                    reason = comparison.get("rng_divergence_reason") or "unknown"
                    summary["rng_divergence_reason_counts"][reason] = (
                        summary["rng_divergence_reason_counts"].get(reason, 0) + 1
                    )
                elif comparison.get("rng_diverged") is False:
                    summary["rng_not_diverged_comparison_count"] += 1
                else:
                    summary["rng_divergence_unknown_count"] += 1
                left = traces_by_id.get(comparison.get("left_branch_id"))
                right = traces_by_id.get(comparison.get("right_branch_id"))
                data_role = comparison_data_role(comparison, left, right)
                bump(summary["comparison_data_role_counts"], data_role)
                if left is None or right is None:
                    summary["unpaired_comparison_count"] += 1
                    left_kind = "missing"
                    right_kind = "missing"
                else:
                    left_kind = trace_action_kind(left)
                    right_kind = trace_action_kind(right)
                kind_pair = f"{left_kind}->{right_kind}"
                bump(summary["comparison_action_kind_pair_counts"], kind_pair)
                bump_nested(
                    summary["comparison_data_role_by_action_kind_pair_counts"],
                    data_role,
                    kind_pair,
                )
                diff = comparison.get("outcome_diff") or {}
                hp_diff = int(diff.get("hp_left_minus_right") or 0)
                reward_diff = float(diff.get("total_reward_left_minus_right") or 0.0)
                combat_win_diff = int(diff.get("combat_wins_left_minus_right") or 0)
                if hp_diff != 0:
                    summary["comparison_nonzero_hp_diff_count"] += 1
                if hp_diff < 0:
                    summary["comparison_left_hp_lower_count"] += 1
                if hp_diff > 0:
                    summary["comparison_left_hp_higher_count"] += 1
                if reward_diff != 0.0:
                    summary["comparison_nonzero_reward_diff_count"] += 1
                if combat_win_diff != 0:
                    summary["comparison_nonzero_combat_win_diff_count"] += 1
                if diff.get("left_dead_right_alive"):
                    summary["comparison_left_dead_right_alive_count"] += 1
                if diff.get("left_alive_right_dead"):
                    summary["comparison_left_alive_right_dead_count"] += 1
                bump(summary["comparison_hp_diff_histogram"], int_delta_bucket(hp_diff))
                bump_nested(
                    summary["comparison_hp_diff_histogram_by_role"],
                    data_role,
                    int_delta_bucket(hp_diff),
                )
                bump(
                    summary["comparison_reward_diff_histogram"],
                    reward_delta_bucket(reward_diff),
                )
                bump_nested(
                    summary["comparison_reward_diff_histogram_by_role"],
                    data_role,
                    reward_delta_bucket(reward_diff),
                )
                bump(
                    summary["comparison_combat_win_diff_counts"],
                    str(combat_win_diff),
                )
        else:
            summary["skipped_decision_type_counts"][decision_type] = (
                summary["skipped_decision_type_counts"].get(decision_type, 0) + 1
            )
        records += 1

        if behavior_action_id is None:
            break
        step = client.request({"cmd": "decision_env_step", "action_id": behavior_action_id})
        done = bool(step.get("done"))
        final_info = step.get("info")

    return {
        "seed": seed,
        "records": records,
        "done": done,
        "final_info": final_info,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver", type=Path, default=default_driver_path())
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path)
    parser.add_argument("--seed-start", type=int, default=1)
    parser.add_argument("--episodes", type=int, default=1)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=200)
    parser.add_argument(
        "--env-max-steps",
        type=int,
        help="FullRunEnv step cap. Defaults to --max-steps for compatibility.",
    )
    parser.add_argument("--behavior-policy", default="rule_baseline_v0")
    parser.add_argument("--continuation-policy", default="rule_baseline_v0")
    parser.add_argument("--horizon-decisions", type=int, default=8)
    parser.add_argument("--horizon-mode", default="fixed_decisions")
    parser.add_argument("--candidate-scope", default="controlled_v1")
    parser.add_argument("--max-candidates", type=int, default=8)
    parser.add_argument(
        "--determinism-check-limit",
        type=int,
        default=20,
        help="Number of traced decisions to repeat immediately for deterministic branch trace checks.",
    )
    parser.add_argument(
        "--decision-type-prefixes",
        default="combat",
        help="Comma-separated decision_type prefixes to trace. Empty string traces all decisions.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.driver.exists():
        raise SystemExit(f"driver binary not found: {args.driver}")
    args.out.parent.mkdir(parents=True, exist_ok=True)
    summary_out = args.summary_out or args.out.with_suffix(".summary.json")
    summary: dict[str, Any] = {
        "schema_version": "branch_trace_collection_summary_v1",
        "driver": str(args.driver),
        "out": str(args.out),
        "behavior_policy": args.behavior_policy,
        "continuation_policy": args.continuation_policy,
        "horizon_decisions": args.horizon_decisions,
        "horizon_mode": args.horizon_mode,
        "candidate_scope": args.candidate_scope,
        "max_candidates": args.max_candidates,
        "max_steps": args.max_steps,
        "env_max_steps": args.env_max_steps or args.max_steps,
        "decision_type_prefixes": [
            item.strip()
            for item in args.decision_type_prefixes.split(",")
            if item.strip()
        ],
        "decision_count": 0,
        "trace_count": 0,
        "comparison_count": 0,
        "determinism_check_count": 0,
        "determinism_mismatch_count": 0,
        "legal_candidate_count_total": 0,
        "sampling_requested_action_count": 0,
        "sampling_included_candidate_count": 0,
        "sampling_excluded_candidate_count": 0,
        "sampling_missing_behavior_action_count": 0,
        "live_env_changed_count": 0,
        "invalid_force_count": 0,
        "branch_panic_count": 0,
        "unpaired_comparison_count": 0,
        "invalid_branch_batch_count": 0,
        "validation_issue_count": 0,
        "redaction_violation_count": 0,
        "trainable_action_label_count": 0,
        "rng_consumed_trace_count": 0,
        "outcome_censored_count": 0,
        "combat_end_requested_but_not_reached_count": 0,
        "fixed_horizon_partial_outcome_count": 0,
        "terminal_trace_count": 0,
        "truncated_trace_count": 0,
        "death_trace_count": 0,
        "combat_win_delta_positive_count": 0,
        "hp_loss_trace_count": 0,
        "comparison_nonzero_hp_diff_count": 0,
        "censored_comparison_count": 0,
        "action_like_comparison_role_count": 0,
        "winner_or_preference_field_count": 0,
        "pairing_invalid_count": 0,
        "rng_diverged_comparison_count": 0,
        "rng_not_diverged_comparison_count": 0,
        "rng_divergence_unknown_count": 0,
        "comparison_left_hp_lower_count": 0,
        "comparison_left_hp_higher_count": 0,
        "comparison_nonzero_reward_diff_count": 0,
        "comparison_nonzero_combat_win_diff_count": 0,
        "comparison_left_dead_right_alive_count": 0,
        "comparison_left_alive_right_dead_count": 0,
        "legal_candidate_action_kind_counts": {},
        "requested_candidate_action_kind_counts": {},
        "traced_candidate_action_kind_counts": {},
        "sampling_excluded_action_kind_counts": {},
        "behavior_action_kind_counts": {},
        "trace_action_kind_counts": {},
        "trace_data_role_counts": {},
        "trace_data_role_by_action_kind_counts": {},
        "trace_hp_delta_histogram": {},
        "trace_reward_histogram": {},
        "comparison_data_role_counts": {},
        "comparison_action_kind_pair_counts": {},
        "comparison_data_role_by_action_kind_pair_counts": {},
        "comparison_hp_diff_histogram": {},
        "comparison_hp_diff_histogram_by_role": {},
        "comparison_reward_diff_histogram": {},
        "comparison_reward_diff_histogram_by_role": {},
        "comparison_combat_win_diff_counts": {},
        "decision_type_counts": {},
        "traced_decision_type_counts": {},
        "skipped_decision_type_counts": {},
        "result_counts": {},
        "horizon_stop_reason_counts": {},
        "boundary_requested_counts": {},
        "boundary_reached_counts": {},
        "truncation_reason_counts": {},
        "validation_issue_code_counts": {},
        "candidate_sampling_scope_counts": {},
        "candidate_sampling_spec_id_counts": {},
        "sampling_excluded_by_reason_counts": {},
        "continuation_policy_counts": {},
        "horizon_spec_counts": {},
        "reward_spec_counts": {},
        "pairing_schema_version_counts": {},
        "pairing_mode_counts": {},
        "common_random_policy_counts": {},
        "paired_validity_status_counts": {},
        "rng_divergence_reason_counts": {},
        "_scenario_seed_ids": {},
        "episodes": [],
    }
    decision_type_prefixes = summary["decision_type_prefixes"]
    client = DriverClient(args.driver)
    try:
        with args.out.open("w", encoding="utf-8") as out:
            for episode in range(args.episodes):
                seed = args.seed_start + episode * args.seed_step
                summary["episodes"].append(
                    collect_episode(
                        client,
                        seed=seed,
                        ascension=args.ascension,
                        final_act=args.final_act,
                        max_steps=args.max_steps,
                        env_max_steps=args.env_max_steps or args.max_steps,
                        behavior_policy=args.behavior_policy,
                        continuation_policy=args.continuation_policy,
                        horizon_decisions=args.horizon_decisions,
                        horizon_mode=args.horizon_mode,
                        candidate_scope=args.candidate_scope,
                        max_candidates=args.max_candidates,
                        decision_type_prefixes=decision_type_prefixes,
                        determinism_check_limit=args.determinism_check_limit,
                        out=out,
                        summary=summary,
                    )
                )
    finally:
        client.close()

    summary["scenario_seed_id_count"] = len(summary.get("_scenario_seed_ids") or {})
    summary.pop("_scenario_seed_ids", None)
    legal_total = int(summary.get("legal_candidate_count_total") or 0)
    included_total = int(summary.get("sampling_included_candidate_count") or 0)
    comparison_total = int(summary.get("comparison_count") or 0)
    trace_total = int(summary.get("trace_count") or 0)
    summary["traced_over_legal_candidate_ratio"] = (
        included_total / legal_total if legal_total else 0.0
    )
    summary["complete_combat_end_pair_ratio"] = (
        (
            int(
                summary["comparison_data_role_counts"].get(
                    "combat_end_complete_pair_rng_aligned", 0
                )
            )
            + int(
                summary["comparison_data_role_counts"].get(
                    "combat_end_complete_pair_rng_diverged", 0
                )
            )
        )
        / comparison_total
        if comparison_total
        else 0.0
    )
    summary["rng_diverged_pair_ratio"] = (
        int(summary.get("rng_diverged_comparison_count") or 0) / comparison_total
        if comparison_total
        else 0.0
    )
    summary["censored_trace_ratio"] = (
        int(summary.get("outcome_censored_count") or 0) / trace_total if trace_total else 0.0
    )
    summary_out.parent.mkdir(parents=True, exist_ok=True)
    summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
