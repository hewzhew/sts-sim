#!/usr/bin/env python3
"""Collect NeutralProbeEvaluator deliberation traces from full_run_env_driver.

This is an evidence/audit collector, not a training script. It follows a
behavior policy through the DecisionEnv, asks the driver for a neutral policy
trace at each decision point, writes those traces as JSONL, and emits aggregate
coverage/compression/fallback metrics.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import Counter
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


def update_summary(
    summary: dict[str, Any],
    trace_payload: dict[str, Any],
    behavior_action_id: int | None,
) -> None:
    summary["decision_count"] += 1
    supported = bool(trace_payload.get("supported"))
    if not supported:
        summary["unsupported_count"] += 1
        reason = trace_payload.get("reason") or "unknown"
        summary["unsupported_reasons"][reason] += 1
        return

    summary["supported_count"] += 1
    trace = trace_payload.get("trace") or {}
    trace_summary = trace_payload.get("summary") or {}
    decision = trace.get("decision") or {}
    evaluation = decision.get("payload") or {}
    mode = decision.get("mode") or "unknown"
    summary["mode_counts"][mode] += 1
    candidate_evaluations = evaluation.get("candidate_evaluations") or []
    for item in candidate_evaluations:
        reason_code = item.get("reason_code") or "missing"
        summary["candidate_reason_code_counts"][reason_code] += 1
        evidence_scope = item.get("evidence_scope") or "missing"
        summary["candidate_evidence_scope_counts"][evidence_scope] += 1
        for bucket in item.get("risk_buckets") or []:
            summary["candidate_risk_bucket_counts"][bucket] += 1
    summary["resource_representative_contamination_count"] += resource_representative_contamination_count(
        evaluation
    )
    if trace_summary.get("fallback"):
        summary["fallback_count"] += 1
    else:
        summary["selected_count"] += 1
    signal_candidate_id = trace_summary.get("short_horizon_signal_candidate_id")
    if signal_candidate_id is not None:
        summary["short_horizon_signal_count"] += 1
    if trace_summary.get("controller_decision") == "abstain":
        summary["controller_abstain_count"] += 1
    selected_eval = find_candidate_eval(candidate_evaluations, signal_candidate_id)
    if not trace_summary.get("fallback") and selected_eval and selected_eval.get("resource_action"):
        summary["selected_resource_count"] += 1
    if selected_eval and selected_eval.get("resource_action"):
        summary["short_horizon_signal_resource_count"] += 1
    if selected_eval:
        label_role = selected_eval.get("label_role") or "missing"
        summary["signal_label_role_counts"][label_role] += 1
    if signal_candidate_id is not None and behavior_action_id is not None:
        if int(signal_candidate_id) == int(behavior_action_id):
            summary["signal_agrees_with_behavior_count"] += 1
        else:
            summary["signal_disagrees_with_behavior_count"] += 1
            audit = trace_payload.get("disagreement_audit") or {}
            reason_code = audit.get("reason_code") or "missing"
            evidence_scope = audit.get("evidence_scope") or "missing"
            hypothesis_class = audit.get("hypothesis_class") or "missing"
            label_role = audit.get("label_role") or "missing"
            action_kind_pair = audit.get("action_kind_pair") or "missing"
            route = audit.get("route") or "missing"
            route_status = audit.get("route_status") or "missing"
            action_label = audit.get("action_label") or "missing"
            typed = audit.get("typed_comparability") or {}
            comparability = typed.get("comparability") or "missing"
            certificate_gate = typed.get("certificate_gate") or {}
            certificate = certificate_gate.get("certificate") or "missing"
            relation = disagreement_relation(trace_payload.get("commutation_probe_vs_reference"))
            summary["reason_code_counts"][reason_code] += 1
            summary["evidence_scope_counts"][evidence_scope] += 1
            summary["hypothesis_class_counts"][hypothesis_class] += 1
            summary["label_role_counts"][label_role] += 1
            summary["action_kind_confusion"][action_kind_pair] += 1
            summary["disagreement_relation_counts"][relation] += 1
            summary["reason_relation_counts"][f"{reason_code}|{relation}"] += 1
            if reason_code == "damage_delta_only":
                summary["damage_delta_relation_counts"][relation] += 1
            summary["route_counts"][route] += 1
            summary["route_status_counts"][route_status] += 1
            summary["route_action_label_counts"][action_label] += 1
            summary["typed_comparability_counts"][comparability] += 1
            summary["typed_comparability_relation_counts"][f"{comparability}|{relation}"] += 1
            summary["certificate_gate_counts"][certificate] += 1
            if certificate_gate.get("trainable_as_action_label") is not False:
                summary["trainable_certificate_label_count"] += 1
            if route == "equivalent_order_only":
                summary["equivalent_order_only_count"] += 1
            if route_status == "confirmed_positive":
                summary["confirmed_positive_count"] += 1
            if route_status == "refuted":
                summary["refuted_count"] += 1
            if route_status == "needs_aligned_confirmation":
                summary["needs_aligned_confirmation_count"] += 1
            if route_status == "needs_horizon_or_value":
                summary["needs_horizon_or_value_count"] += 1
            if reason_code == "missing":
                summary["missing_disagreement_reason_count"] += 1
            if audit.get("trainable_as_action_label") is not False:
                summary["trainable_disagreement_label_count"] += 1
            if action_label != "none":
                summary["non_none_action_label_count"] += 1
            ledger = audit.get("irreversible_resource_ledger") or {}
            for key, value in ledger.items():
                if isinstance(value, bool) and value:
                    summary["irreversible_ledger_counts"][key] += 1
            for bucket in audit.get("risk_buckets") or []:
                summary["risk_bucket_counts"][bucket] += 1
            paired = trace_payload.get("paired_compare_vs_reference")
            if paired is not None:
                summary["paired_compare_count"] += 1
                if paired.get("left_dead_right_alive"):
                    summary["paired_left_dead_right_alive_count"] += 1
                if paired.get("left_alive_right_dead"):
                    summary["paired_left_alive_right_dead_count"] += 1
                if paired.get("left_clears_right_not"):
                    summary["paired_left_clears_right_not_count"] += 1
                if paired.get("right_clears_left_not"):
                    summary["paired_right_clears_left_not_count"] += 1
                summary["paired_hp_lost_diff_sum"] += int(
                    paired.get("hp_lost_diff_left_minus_right") or 0
                )
                summary["paired_enemy_removed_diff_sum"] += int(
                    paired.get("enemy_removed_diff_left_minus_right") or 0
                )
                summary["paired_kill_diff_sum"] += int(
                    paired.get("kill_diff_left_minus_right") or 0
                )
                hp_diff = int(paired.get("hp_lost_diff_left_minus_right") or 0)
                enemy_diff = int(paired.get("enemy_removed_diff_left_minus_right") or 0)
                if hp_diff > 0:
                    summary["paired_hp_loss_worse_count"] += 1
                if hp_diff >= 3:
                    summary["paired_hp_loss_worse_ge_3_count"] += 1
                if hp_diff >= 5:
                    summary["paired_hp_loss_worse_ge_5_count"] += 1
                if enemy_diff < 0:
                    summary["paired_enemy_removed_worse_count"] += 1
            suffix = trace_payload.get("reference_suffix_replay_probe")
            if suffix is not None:
                summary["reference_suffix_replay_probe_count"] += 1
                if not bool(suffix.get("signal_then_reference_legal")):
                    summary["signal_suffix_replay_illegal_count"] += 1
                if not bool(suffix.get("reference_then_signal_legal")):
                    summary["reference_suffix_replay_illegal_count"] += 1
                if bool(suffix.get("summary_equal")):
                    summary["suffix_replay_summary_equal_count"] += 1
            isolated_enemy_response = trace_payload.get(
                "isolated_enemy_response_public_probe_vs_reference"
            )
            if isolated_enemy_response is not None:
                summary["isolated_enemy_response_public_probe_count"] += 1
                if bool(isolated_enemy_response.get("public_safe")):
                    summary["isolated_enemy_response_public_safe_count"] += 1
                hp_diff = int(
                    isolated_enemy_response.get("hp_lost_diff_left_minus_right") or 0
                )
                enemy_diff = int(
                    isolated_enemy_response.get("enemy_removed_diff_left_minus_right") or 0
                )
                if hp_diff > 0:
                    summary["isolated_enemy_response_hp_loss_worse_count"] += 1
                    summary["isolated_enemy_response_hp_worse_by_relation"][relation] += 1
                if enemy_diff < 0:
                    summary["isolated_enemy_response_enemy_removed_worse_count"] += 1
            aligned_enemy_response = trace_payload.get(
                "aligned_enemy_response_public_probe_vs_reference"
            )
            if aligned_enemy_response is not None:
                summary["aligned_enemy_response_public_probe_count"] += 1
                if bool(aligned_enemy_response.get("public_safe")):
                    summary["aligned_enemy_response_public_safe_count"] += 1
                if bool(aligned_enemy_response.get("summary_equal")):
                    summary["aligned_enemy_response_summary_equal_count"] += 1
                    summary["aligned_enemy_response_summary_equal_by_relation"][relation] += 1
                hp_diff = int(
                    aligned_enemy_response.get("hp_lost_diff_left_minus_right") or 0
                )
                enemy_diff = int(
                    aligned_enemy_response.get("enemy_removed_diff_left_minus_right") or 0
                )
                if hp_diff > 0:
                    summary["aligned_enemy_response_hp_loss_worse_count"] += 1
                    summary["aligned_enemy_response_hp_worse_by_relation"][relation] += 1
                    if reason_code == "damage_delta_only":
                        summary["damage_delta_aligned_hp_worse_by_relation"][relation] += 1
                if enemy_diff < 0:
                    summary["aligned_enemy_response_enemy_removed_worse_count"] += 1
            commutation = trace_payload.get("commutation_probe_vs_reference")
            if commutation is not None:
                summary["commutation_probe_count"] += 1
                left_legal = bool(commutation.get("left_then_right_legal"))
                right_legal = bool(commutation.get("right_then_left_legal"))
                order_only = bool(commutation.get("order_only_equivalent"))
                if order_only:
                    summary["order_only_disagreement_count"] += 1
                elif not left_legal or not right_legal:
                    summary["mutually_exclusive_disagreement_count"] += 1
                else:
                    summary["non_order_commutable_disagreement_count"] += 1
                if not left_legal:
                    summary["left_then_right_second_illegal_count"] += 1
                if not right_legal:
                    summary["right_then_left_second_illegal_count"] += 1

    for field in (
        "candidate_count",
        "evidence_count",
        "request_count",
        "group_count",
        "expanded_group_count",
        "unexpanded_group_count",
        "truncated_candidate_count",
        "dead_candidate_count",
    ):
        value = int(trace_summary.get(field) or 0)
        summary[f"total_{field}"] += value
        summary[f"max_{field}"] = max(summary[f"max_{field}"], value)


def find_candidate_eval(items: list[dict[str, Any]], action_id: Any) -> dict[str, Any] | None:
    if action_id is None:
        return None
    try:
        wanted = int(action_id)
    except (TypeError, ValueError):
        return None
    for item in items:
        try:
            if int(item.get("action_id")) == wanted:
                return item
        except (TypeError, ValueError):
            continue
    return None


def disagreement_relation(commutation: dict[str, Any] | None) -> str:
    if not commutation:
        return "missing"
    if bool(commutation.get("order_only_equivalent")):
        return "order_only"
    left_legal = bool(commutation.get("left_then_right_legal"))
    right_legal = bool(commutation.get("right_then_left_legal"))
    if not left_legal or not right_legal:
        return "mutually_exclusive"
    return "non_order_commutable"


def resource_representative_contamination_count(evaluation: dict[str, Any]) -> int:
    candidate_evaluations = evaluation.get("candidate_evaluations") or []
    eval_by_id: dict[int, dict[str, Any]] = {}
    for item in candidate_evaluations:
        try:
            eval_by_id[int(item.get("action_id"))] = item
        except (TypeError, ValueError):
            continue
    total = 0
    groups = list(evaluation.get("expanded_branch_groups") or []) + list(
        evaluation.get("unexpanded_branch_groups") or []
    )
    for group in groups:
        try:
            representative_id = int(group.get("representative_action_id"))
        except (TypeError, ValueError):
            continue
        representative_eval = eval_by_id.get(representative_id)
        if not representative_eval or not representative_eval.get("resource_action"):
            continue
        member_ids = []
        for action_id in group.get("action_ids") or []:
            try:
                member_ids.append(int(action_id))
            except (TypeError, ValueError):
                continue
        if any(eval_by_id.get(action_id, {}).get("dominance_eligible") for action_id in member_ids):
            total += 1
    return total


def collect_episode(
    client: DriverClient,
    *,
    seed: int,
    ascension: int,
    final_act: bool,
    max_steps: int,
    policy: str,
    time_budget_ms: int,
    max_branch_depth: int,
    max_candidates: int,
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
            "max_steps": max_steps,
            "reward_shaping_profile": "baseline",
        }
    )
    done = False
    records = 0
    total_reward = 0.0
    final_info: dict[str, Any] | None = None
    while not done and records < max_steps:
        preview = client.request(
            {
                "cmd": "preview_policy_action",
                "policy": policy,
                "include_state": False,
                "include_next_state": False,
                "check_live_env_unchanged": False,
            }
        )["payload"]
        action_id = preview.get("chosen_action_index")
        trace = client.request(
            {
                "cmd": "neutral_policy_trace",
                "time_budget_ms": time_budget_ms,
                "max_branch_depth": max_branch_depth,
                "max_candidates": max_candidates,
                "reference_action_id": action_id,
            }
        )["payload"]
        trace_record = {
            "schema_version": "neutral_policy_trace_record_v0",
            "seed": seed,
            "episode_step": records,
            "behavior_policy": policy,
            "behavior_action_id": action_id,
            "behavior_action_key": preview.get("chosen_action_key"),
            "trace": trace,
        }
        out.write(json.dumps(trace_record, separators=(",", ":")) + "\n")
        update_summary(summary, trace, action_id)
        records += 1

        if action_id is None:
            break
        step = client.request({"cmd": "decision_env_step", "action_id": action_id})
        total_reward += float(step.get("reward") or 0.0)
        done = bool(step.get("done"))
        final_info = step.get("info")
    return {
        "seed": seed,
        "records": records,
        "total_reward": total_reward,
        "done": done,
        "final_info": final_info,
    }


def finalize_summary(summary: dict[str, Any]) -> dict[str, Any]:
    supported = max(int(summary["supported_count"]), 1)
    for field in (
        "candidate_count",
        "evidence_count",
        "request_count",
        "group_count",
        "expanded_group_count",
        "unexpanded_group_count",
        "truncated_candidate_count",
        "dead_candidate_count",
    ):
        summary[f"avg_{field}"] = summary[f"total_{field}"] / supported
    summary["fallback_rate_supported"] = summary["fallback_count"] / supported
    summary["selected_rate_supported"] = summary["selected_count"] / supported
    selected = max(int(summary["selected_count"]), 1)
    summary["selected_agreement_rate_with_behavior"] = (
        summary["selected_agrees_with_behavior_count"] / selected
    )
    summary["selected_disagreement_rate_with_behavior"] = (
        summary["selected_disagrees_with_behavior_count"] / selected
    )
    signal = max(int(summary["short_horizon_signal_count"]), 1)
    summary["short_horizon_signal_rate_supported"] = (
        summary["short_horizon_signal_count"] / supported
    )
    summary["signal_agreement_rate_with_behavior"] = (
        summary["signal_agrees_with_behavior_count"] / signal
    )
    summary["signal_disagreement_rate_with_behavior"] = (
        summary["signal_disagrees_with_behavior_count"] / signal
    )
    paired = max(int(summary["paired_compare_count"]), 1)
    summary["paired_avg_hp_lost_diff_left_minus_behavior"] = (
        summary["paired_hp_lost_diff_sum"] / paired
    )
    summary["paired_avg_enemy_removed_diff_left_minus_behavior"] = (
        summary["paired_enemy_removed_diff_sum"] / paired
    )
    summary["paired_avg_kill_diff_left_minus_behavior"] = summary["paired_kill_diff_sum"] / paired
    summary["commutation_summary"] = {
        "probe_count": summary["commutation_probe_count"],
        "order_only_disagreement_count": summary["order_only_disagreement_count"],
        "mutually_exclusive_disagreement_count": summary[
            "mutually_exclusive_disagreement_count"
        ],
        "non_order_commutable_disagreement_count": summary[
            "non_order_commutable_disagreement_count"
        ],
        "left_then_right_second_illegal_count": summary[
            "left_then_right_second_illegal_count"
        ],
        "right_then_left_second_illegal_count": summary[
            "right_then_left_second_illegal_count"
        ],
    }
    summary["router_summary"] = {
        "confirmed_positive_count": summary["confirmed_positive_count"],
        "refuted_count": summary["refuted_count"],
        "equivalent_order_only_count": summary["equivalent_order_only_count"],
        "needs_aligned_confirmation_count": summary["needs_aligned_confirmation_count"],
        "needs_horizon_or_value_count": summary["needs_horizon_or_value_count"],
        "non_none_action_label_count": summary["non_none_action_label_count"],
        "trainable_certificate_label_count": summary["trainable_certificate_label_count"],
        "typed_comparability_counts": dict(summary["typed_comparability_counts"]),
        "typed_comparability_relation_counts": dict(
            summary["typed_comparability_relation_counts"]
        ),
        "certificate_gate_counts": dict(summary["certificate_gate_counts"]),
    }
    summary["signal_summary"] = {
        "short_horizon_signal_count": summary["short_horizon_signal_count"],
        "controller_abstain_count": summary["controller_abstain_count"],
        "short_horizon_signal_resource_count": summary[
            "short_horizon_signal_resource_count"
        ],
        "signal_agrees_with_behavior_count": summary[
            "signal_agrees_with_behavior_count"
        ],
        "signal_disagrees_with_behavior_count": summary[
            "signal_disagrees_with_behavior_count"
        ],
        "label_role_counts": dict(summary["label_role_counts"]),
        "reason_relation_counts": dict(summary["reason_relation_counts"]),
        "damage_delta_relation_counts": dict(summary["damage_delta_relation_counts"]),
        "damage_delta_aligned_hp_worse_by_relation": dict(
            summary["damage_delta_aligned_hp_worse_by_relation"]
        ),
    }
    summary["probe_summary"] = {
        "reference_suffix_replay_probe_count": summary[
            "reference_suffix_replay_probe_count"
        ],
        "signal_suffix_replay_illegal_count": summary[
            "signal_suffix_replay_illegal_count"
        ],
        "reference_suffix_replay_illegal_count": summary[
            "reference_suffix_replay_illegal_count"
        ],
        "suffix_replay_summary_equal_count": summary["suffix_replay_summary_equal_count"],
        "isolated_enemy_response_public_probe_count": summary[
            "isolated_enemy_response_public_probe_count"
        ],
        "isolated_enemy_response_public_safe_count": summary[
            "isolated_enemy_response_public_safe_count"
        ],
        "isolated_enemy_response_hp_loss_worse_count": summary[
            "isolated_enemy_response_hp_loss_worse_count"
        ],
        "isolated_enemy_response_enemy_removed_worse_count": summary[
            "isolated_enemy_response_enemy_removed_worse_count"
        ],
        "isolated_enemy_response_hp_worse_by_relation": dict(
            summary["isolated_enemy_response_hp_worse_by_relation"]
        ),
        "aligned_enemy_response_public_probe_count": summary[
            "aligned_enemy_response_public_probe_count"
        ],
        "aligned_enemy_response_public_safe_count": summary[
            "aligned_enemy_response_public_safe_count"
        ],
        "aligned_enemy_response_summary_equal_count": summary[
            "aligned_enemy_response_summary_equal_count"
        ],
        "aligned_enemy_response_hp_loss_worse_count": summary[
            "aligned_enemy_response_hp_loss_worse_count"
        ],
        "aligned_enemy_response_enemy_removed_worse_count": summary[
            "aligned_enemy_response_enemy_removed_worse_count"
        ],
        "aligned_enemy_response_hp_worse_by_relation": dict(
            summary["aligned_enemy_response_hp_worse_by_relation"]
        ),
        "aligned_enemy_response_summary_equal_by_relation": dict(
            summary["aligned_enemy_response_summary_equal_by_relation"]
        ),
    }
    return summary


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
    parser.add_argument("--max-steps", type=int, default=500)
    parser.add_argument("--policy", default="rule_baseline_v0")
    parser.add_argument("--time-budget-ms", type=int, default=25)
    parser.add_argument("--max-branch-depth", type=int, default=1)
    parser.add_argument("--max-candidates", type=int, default=64)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.driver.exists():
        raise SystemExit(f"driver binary not found: {args.driver}")
    args.out.parent.mkdir(parents=True, exist_ok=True)
    summary_out = args.summary_out or args.out.with_suffix(".summary.json")
    summary: dict[str, Any] = {
        "schema_version": "neutral_policy_trace_collection_summary_v0",
        "driver": str(args.driver),
        "out": str(args.out),
        "policy": args.policy,
        "decision_count": 0,
        "supported_count": 0,
        "unsupported_count": 0,
        "fallback_count": 0,
        "selected_count": 0,
        "short_horizon_signal_count": 0,
        "controller_abstain_count": 0,
        "selected_resource_count": 0,
        "short_horizon_signal_resource_count": 0,
        "selected_agrees_with_behavior_count": 0,
        "selected_disagrees_with_behavior_count": 0,
        "signal_agrees_with_behavior_count": 0,
        "signal_disagrees_with_behavior_count": 0,
        "missing_disagreement_reason_count": 0,
        "trainable_disagreement_label_count": 0,
        "trainable_certificate_label_count": 0,
        "non_none_action_label_count": 0,
        "confirmed_positive_count": 0,
        "refuted_count": 0,
        "equivalent_order_only_count": 0,
        "needs_aligned_confirmation_count": 0,
        "needs_horizon_or_value_count": 0,
        "paired_compare_count": 0,
        "paired_left_dead_right_alive_count": 0,
        "paired_left_alive_right_dead_count": 0,
        "paired_left_clears_right_not_count": 0,
        "paired_right_clears_left_not_count": 0,
        "paired_hp_lost_diff_sum": 0,
        "paired_enemy_removed_diff_sum": 0,
        "paired_kill_diff_sum": 0,
        "paired_hp_loss_worse_count": 0,
        "paired_hp_loss_worse_ge_3_count": 0,
        "paired_hp_loss_worse_ge_5_count": 0,
        "paired_enemy_removed_worse_count": 0,
        "commutation_probe_count": 0,
        "order_only_disagreement_count": 0,
        "mutually_exclusive_disagreement_count": 0,
        "non_order_commutable_disagreement_count": 0,
        "left_then_right_second_illegal_count": 0,
        "right_then_left_second_illegal_count": 0,
        "resource_representative_contamination_count": 0,
        "reference_suffix_replay_probe_count": 0,
        "signal_suffix_replay_illegal_count": 0,
        "reference_suffix_replay_illegal_count": 0,
        "suffix_replay_summary_equal_count": 0,
        "isolated_enemy_response_public_probe_count": 0,
        "isolated_enemy_response_public_safe_count": 0,
        "isolated_enemy_response_hp_loss_worse_count": 0,
        "isolated_enemy_response_enemy_removed_worse_count": 0,
        "aligned_enemy_response_public_probe_count": 0,
        "aligned_enemy_response_public_safe_count": 0,
        "aligned_enemy_response_summary_equal_count": 0,
        "aligned_enemy_response_hp_loss_worse_count": 0,
        "aligned_enemy_response_enemy_removed_worse_count": 0,
        "unsupported_reasons": Counter(),
        "mode_counts": Counter(),
        "reason_code_counts": Counter(),
        "evidence_scope_counts": Counter(),
        "hypothesis_class_counts": Counter(),
        "label_role_counts": Counter(),
        "signal_label_role_counts": Counter(),
        "isolated_enemy_response_hp_worse_by_relation": Counter(),
        "aligned_enemy_response_hp_worse_by_relation": Counter(),
        "aligned_enemy_response_summary_equal_by_relation": Counter(),
        "damage_delta_aligned_hp_worse_by_relation": Counter(),
        "risk_bucket_counts": Counter(),
        "action_kind_confusion": Counter(),
        "disagreement_relation_counts": Counter(),
        "reason_relation_counts": Counter(),
        "damage_delta_relation_counts": Counter(),
        "irreversible_ledger_counts": Counter(),
        "route_counts": Counter(),
        "route_status_counts": Counter(),
        "route_action_label_counts": Counter(),
        "typed_comparability_counts": Counter(),
        "typed_comparability_relation_counts": Counter(),
        "certificate_gate_counts": Counter(),
        "candidate_reason_code_counts": Counter(),
        "candidate_evidence_scope_counts": Counter(),
        "candidate_risk_bucket_counts": Counter(),
        "episodes": [],
    }
    for field in (
        "candidate_count",
        "evidence_count",
        "request_count",
        "group_count",
        "expanded_group_count",
        "unexpanded_group_count",
        "truncated_candidate_count",
        "dead_candidate_count",
    ):
        summary[f"total_{field}"] = 0
        summary[f"max_{field}"] = 0

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
                        policy=args.policy,
                        time_budget_ms=args.time_budget_ms,
                        max_branch_depth=args.max_branch_depth,
                        max_candidates=args.max_candidates,
                        out=out,
                        summary=summary,
                    )
                )
    finally:
        client.close()

    summary["unsupported_reasons"] = dict(summary["unsupported_reasons"])
    summary["mode_counts"] = dict(summary["mode_counts"])
    summary["reason_code_counts"] = dict(summary["reason_code_counts"])
    summary["evidence_scope_counts"] = dict(summary["evidence_scope_counts"])
    summary["hypothesis_class_counts"] = dict(summary["hypothesis_class_counts"])
    summary["label_role_counts"] = dict(summary["label_role_counts"])
    summary["signal_label_role_counts"] = dict(summary["signal_label_role_counts"])
    summary["isolated_enemy_response_hp_worse_by_relation"] = dict(
        summary["isolated_enemy_response_hp_worse_by_relation"]
    )
    summary["aligned_enemy_response_hp_worse_by_relation"] = dict(
        summary["aligned_enemy_response_hp_worse_by_relation"]
    )
    summary["aligned_enemy_response_summary_equal_by_relation"] = dict(
        summary["aligned_enemy_response_summary_equal_by_relation"]
    )
    summary["damage_delta_aligned_hp_worse_by_relation"] = dict(
        summary["damage_delta_aligned_hp_worse_by_relation"]
    )
    summary["risk_bucket_counts"] = dict(summary["risk_bucket_counts"])
    summary["action_kind_confusion"] = dict(summary["action_kind_confusion"])
    summary["disagreement_relation_counts"] = dict(summary["disagreement_relation_counts"])
    summary["reason_relation_counts"] = dict(summary["reason_relation_counts"])
    summary["damage_delta_relation_counts"] = dict(summary["damage_delta_relation_counts"])
    summary["irreversible_ledger_counts"] = dict(summary["irreversible_ledger_counts"])
    summary["route_counts"] = dict(summary["route_counts"])
    summary["route_status_counts"] = dict(summary["route_status_counts"])
    summary["route_action_label_counts"] = dict(summary["route_action_label_counts"])
    summary["typed_comparability_counts"] = dict(summary["typed_comparability_counts"])
    summary["typed_comparability_relation_counts"] = dict(
        summary["typed_comparability_relation_counts"]
    )
    summary["certificate_gate_counts"] = dict(summary["certificate_gate_counts"])
    summary["candidate_reason_code_counts"] = dict(summary["candidate_reason_code_counts"])
    summary["candidate_evidence_scope_counts"] = dict(
        summary["candidate_evidence_scope_counts"]
    )
    summary["candidate_risk_bucket_counts"] = dict(summary["candidate_risk_bucket_counts"])
    summary = finalize_summary(summary)
    summary_out.parent.mkdir(parents=True, exist_ok=True)
    summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
