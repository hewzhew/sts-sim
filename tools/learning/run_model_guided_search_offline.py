#!/usr/bin/env python3
"""Run offline model-guided search evidence collection.

This is an offline evidence runner:

    family model predictions
    -> K contrast-family evidence requests
    -> targeted branch traces
    -> decision evidence bundles + abstain/coverage report

It does not choose actions, train action labels, create winners/preferences, or
perform live takeover.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_DIR = Path(__file__).resolve().parent

FORBIDDEN_LABEL_KEYS = {
    "winner",
    "preferred",
    "preferred_action",
    "selected_action",
    "teacher_choice",
}


def iter_jsonl(path: Path) -> Iterable[dict[str, Any]]:
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                yield json.loads(line)


def assert_no_action_label_leak(
    row: dict[str, Any],
    *,
    source: str,
    index: int,
    require_explicit: bool = True,
) -> None:
    if require_explicit and row.get("trainable_as_action_label") is not False:
        raise ValueError(f"{source}:{index} is action-label-like")
    if (not require_explicit) and row.get("trainable_as_action_label") is True:
        raise ValueError(f"{source}:{index} is action-label-like")
    label_policy = row.get("label_policy")
    if require_explicit and (label_policy or {}).get("action_label") is not False:
        raise ValueError(f"{source}:{index} has action_label=true")
    if (not require_explicit) and isinstance(label_policy, dict):
        if label_policy.get("action_label") is not False:
            raise ValueError(f"{source}:{index} has action_label=true")
    serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
    for key in FORBIDDEN_LABEL_KEYS:
        if f'"{key}"' in serialized:
            raise ValueError(f"{source}:{index} contains forbidden key {key}")


def read_json(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"{path} did not contain a JSON object")
    return data


def run_command(command: list[str]) -> None:
    result = subprocess.run(command, cwd=REPO_ROOT, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"command failed with exit code {result.returncode}: {' '.join(command)}")


def decision_key(seed: Any, step: Any) -> str:
    return json.dumps(
        {"episode_seed": seed, "episode_step": step},
        sort_keys=True,
        separators=(",", ":"),
    )


def request_decision_key(row: dict[str, Any]) -> str:
    return decision_key(row.get("episode_seed"), row.get("episode_step"))


def record_decision_key(row: dict[str, Any]) -> str:
    return decision_key(row.get("seed"), row.get("episode_step"))


def load_safe_rows(
    path: Path,
    *,
    source: str,
    require_explicit: bool = True,
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for index, row in enumerate(iter_jsonl(path)):
        assert_no_action_label_leak(
            row,
            source=source,
            index=index,
            require_explicit=require_explicit,
        )
        rows.append(row)
    return rows


def count_trace_safety(record: dict[str, Any]) -> dict[str, int]:
    batch = record.get("branch_trace_batch") or {}
    validation = batch.get("validation_report") or {}
    out = {
        "trace_count": int(batch.get("trace_count") or 0),
        "comparison_count": int(batch.get("comparison_count") or 0),
        "validation_issue_count": int(validation.get("issue_count") or 0),
        "redaction_violation_count": 0,
        "trainable_action_label_count": 0,
        "winner_or_preference_field_count": 0,
        "outcome_censored_count": 0,
        "truncated_trace_count": 0,
    }
    for issue in validation.get("issues") or []:
        code = str(issue.get("code") or "unknown")
        if "redaction" in code or "hidden" in code or "non_public" in code or "model_input" in code:
            out["redaction_violation_count"] += 1
    for trace in batch.get("traces") or []:
        if trace.get("trainable_as_action_label") is not False:
            out["trainable_action_label_count"] += 1
        outcome = trace.get("outcome") or {}
        if outcome.get("outcome_censored"):
            out["outcome_censored_count"] += 1
        if outcome.get("truncated"):
            out["truncated_trace_count"] += 1
    for comparison in batch.get("comparisons") or []:
        if any(key in comparison for key in FORBIDDEN_LABEL_KEYS):
            out["winner_or_preference_field_count"] += 1
    return out


def bundle_status(record: dict[str, Any], counts: dict[str, int]) -> tuple[str, str | None]:
    target = (record.get("targeted_recollection") or {})
    missing = target.get("missing_target_pair_action_keys") or []
    batch = record.get("branch_trace_batch") or {}
    if missing:
        return "abstain", "missing_target_action_key"
    if not batch.get("live_env_unchanged", False):
        return "abstain", "live_env_changed"
    if counts["validation_issue_count"] > 0:
        return "abstain", "validation_issue"
    if counts["redaction_violation_count"] > 0:
        return "abstain", "redaction_violation"
    if counts["trainable_action_label_count"] > 0 or counts["winner_or_preference_field_count"] > 0:
        return "abstain", "label_safety_violation"
    if counts["trace_count"] == 0:
        return "abstain", "no_evidence"
    if counts["outcome_censored_count"] > 0 or counts["truncated_trace_count"] > 0:
        return "evidence_partial", "partial_or_censored_evidence"
    return "evidence_ready", None


def write_evidence_bundles(
    *,
    request_rows: list[dict[str, Any]],
    trace_records: list[dict[str, Any]],
    out: Path,
) -> dict[str, Any]:
    requests_by_decision: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in request_rows:
        requests_by_decision[request_decision_key(row)].append(row)

    status_counts: Counter[str] = Counter()
    reason_counts: Counter[str] = Counter()
    family_counts: Counter[str] = Counter()
    total_counts: Counter[str] = Counter()
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8") as handle:
        for index, record in enumerate(trace_records):
            assert_no_action_label_leak(
                record,
                source=str(out),
                index=index,
                require_explicit=False,
            )
            key = record_decision_key(record)
            requests = requests_by_decision.get(key, [])
            for request in requests:
                family = ((request.get("family_allocator") or {}).get("family")) or "unknown"
                family_counts[str(family)] += 1
            counts = count_trace_safety(record)
            status, reason = bundle_status(record, counts)
            status_counts[status] += 1
            if reason:
                reason_counts[reason] += 1
            total_counts.update(counts)
            bundle = {
                "schema_version": "model_guided_search_evidence_bundle_v0",
                "trainable_role": "model_guided_search_evidence_bundle",
                "trainable_as_action_label": False,
                "episode_seed": record.get("seed"),
                "episode_step": record.get("episode_step"),
                "decision_type": record.get("decision_type"),
                "behavior_policy": record.get("behavior_policy"),
                "behavior_action_id": record.get("behavior_action_id"),
                "behavior_action_key": record.get("behavior_action_key"),
                "request_count": len(requests),
                "requested_families": [
                    request.get("family_allocator") for request in requests
                ],
                "evidence_status": status,
                "abstain_reason": reason,
                "controller_decision": {
                    "mode": "abstain",
                    "reason": "offline_evidence_collection_only",
                    "selected_action_id": None,
                    "trainable_as_action_label": False,
                },
                "coverage": {
                    "matched_target_pair_action_key_count": len(
                        (record.get("targeted_recollection") or {}).get(
                            "matched_target_pair_action_keys"
                        )
                        or []
                    ),
                    "missing_target_pair_action_key_count": len(
                        (record.get("targeted_recollection") or {}).get(
                            "missing_target_pair_action_keys"
                        )
                        or []
                    ),
                    **counts,
                },
                "branch_trace_batch": record.get("branch_trace_batch"),
                "label_policy": {
                    "action_label": False,
                    "source": "model_guided_search_offline_v0",
                },
            }
            handle.write(json.dumps(bundle, separators=(",", ":")) + "\n")

    return {
        "bundle_count": len(trace_records),
        "status_counts": dict(status_counts),
        "abstain_reason_counts": dict(reason_counts),
        "requested_family_counts_top": dict(family_counts.most_common(30)),
        "aggregate_evidence_counts": dict(total_counts),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--family-predictions", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--request-out", type=Path)
    parser.add_argument("--trace-out", type=Path)
    parser.add_argument("--trace-summary-out", type=Path)
    parser.add_argument("--budget", type=int, default=3)
    parser.add_argument("--score-name", default="family_abs_ge_10_probability")
    parser.add_argument("--max-target-rows", type=int, default=0)
    parser.add_argument("--max-target-decisions", type=int, default=0)
    parser.add_argument("--horizon-decisions", type=int, default=16)
    parser.add_argument("--horizon-mode", default="combat_end_v1")
    parser.add_argument("--candidate-scope", default="controlled_v1")
    parser.add_argument("--candidate-index-mode", default="targets_plus_behavior")
    parser.add_argument("--max-candidates", type=int, default=64)
    parser.add_argument("--max-steps", type=int, default=360)
    parser.add_argument("--env-max-steps", type=int, default=360)
    parser.add_argument("--include-audit-targets", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    request_out = args.request_out or args.out.with_suffix(".requests.jsonl")
    trace_out = args.trace_out or args.out.with_suffix(".branch_traces.jsonl")
    trace_summary_out = args.trace_summary_out or args.out.with_suffix(".branch_traces.summary.json")

    build_cmd = [
        sys.executable,
        str(SCRIPT_DIR / "build_model_guided_family_evidence_requests.py"),
        "--family-predictions",
        str(args.family_predictions),
        "--out",
        str(request_out),
        "--summary-out",
        str(request_out.with_suffix(".summary.json")),
        "--budget",
        str(args.budget),
        "--score-name",
        args.score_name,
    ]
    if args.include_audit_targets:
        build_cmd.append("--include-audit-targets")
    run_command(build_cmd)

    collect_cmd = [
        sys.executable,
        str(SCRIPT_DIR / "collect_targeted_branch_traces.py"),
        "--hard-states",
        str(request_out),
        "--out",
        str(trace_out),
        "--summary-out",
        str(trace_summary_out),
        "--max-target-rows",
        str(args.max_target_rows),
        "--max-target-decisions",
        str(args.max_target_decisions),
        "--horizon-decisions",
        str(args.horizon_decisions),
        "--horizon-mode",
        args.horizon_mode,
        "--candidate-scope",
        args.candidate_scope,
        "--candidate-index-mode",
        args.candidate_index_mode,
        "--max-candidates",
        str(args.max_candidates),
        "--max-steps",
        str(args.max_steps),
        "--env-max-steps",
        str(args.env_max_steps),
    ]
    run_command(collect_cmd)

    request_rows = load_safe_rows(request_out, source=str(request_out))
    trace_records = load_safe_rows(trace_out, source=str(trace_out), require_explicit=False)
    bundle_summary = write_evidence_bundles(
        request_rows=request_rows,
        trace_records=trace_records,
        out=args.out,
    )
    request_summary = read_json(request_out.with_suffix(".summary.json"))
    trace_summary = read_json(trace_summary_out)
    summary = {
        "schema_version": "model_guided_search_offline_summary_v0",
        "family_predictions": str(args.family_predictions),
        "evidence_bundle_out": str(args.out),
        "request_out": str(request_out),
        "trace_out": str(trace_out),
        "budget": args.budget,
        "score_name": args.score_name,
        "candidate_index_mode": args.candidate_index_mode,
        "horizon_mode": args.horizon_mode,
        "horizon_decisions": args.horizon_decisions,
        "request_summary": request_summary,
        "trace_summary": trace_summary,
        "bundle_summary": bundle_summary,
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "offline_runner_is_search_evidence_not_policy": True,
            "controller_decision_is_abstain_only": True,
        },
    }
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
