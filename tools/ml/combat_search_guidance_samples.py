#!/usr/bin/env python3
"""Export positive-only combat search guidance samples from search reports.

This is intentionally not a policy trainer.  It extracts what current search
already found in best complete trajectories so we can inspect whether combat
search traces are worth turning into ordering guidance later.

Accepted inputs:
  * CombatSearchV2Report JSON
  * CombatSearchEvidenceV1 JSON envelopes
  * CombatSearchV2BenchmarkReport JSON with per-case trajectories

The emitted samples are positive-only.  They do not include sibling legal
actions, so they must not be treated as calibrated action-good/action-bad
labels.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


SCHEMA_NAME = "CombatSearchGuidanceSampleV1"
SCHEMA_VERSION = 1
LABEL_ROLE = "oracle_search_guidance_positive_only_not_human_policy"
TRAJECTORY_TARGET_KIND = "best_complete_trajectory_action_positive"
TRAJECTORY_CANDIDATE_COVERAGE = "positive_only_no_alternatives"
MICROSCOPE_TARGET_KIND = "initial_decision_candidate_selected_by_best_complete"
MICROSCOPE_CANDIDATE_COVERAGE = "root_legal_candidates_reported_limit"


@dataclass(frozen=True)
class ReportRef:
    source_file: Path
    source_schema: str
    source_case_id: str | None
    source_input_path: str | None
    report: dict[str, Any]


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as fh:
        return json.load(fh)


def iter_report_refs(path: Path, payload: Any) -> Iterable[ReportRef]:
    if not isinstance(payload, dict):
        return
    schema = str(payload.get("schema_name", "unknown"))
    decision = payload.get("decision")
    if (
        isinstance(decision, dict)
        and decision.get("schema_name") == "CombatSearchV2DecisionMicroscopeReport"
    ):
        yield ReportRef(
            source_file=path,
            source_schema="CombatSearchV2DecisionMicroscopeWrapper",
            source_case_id=string_or_none(payload.get("case_id"))
            or string_or_none(decision.get("input_label")),
            source_input_path=string_or_none(payload.get("input_path")),
            report=decision,
        )
        return
    if schema == "CombatSearchEvidenceV1":
        report = payload.get("report")
        if isinstance(report, dict):
            context = payload.get("context") if isinstance(payload.get("context"), dict) else {}
            yield ReportRef(
                source_file=path,
                source_schema=schema,
                source_case_id=string_or_none(context.get("capture_case_id"))
                or string_or_none(context.get("source_kind")),
                source_input_path=None,
                report=report,
            )
        return
    if schema == "CombatSearchV2Report":
        yield ReportRef(
            source_file=path,
            source_schema=schema,
            source_case_id=string_or_none(payload.get("input_label")),
            source_input_path=None,
            report=payload,
        )
        return
    if schema == "CombatSearchV2DecisionMicroscopeReport":
        yield ReportRef(
            source_file=path,
            source_schema=schema,
            source_case_id=string_or_none(payload.get("input_label")),
            source_input_path=None,
            report=payload,
        )
        return
    if schema == "CombatSearchV2BenchmarkReport":
        for case in payload.get("cases", []):
            if not isinstance(case, dict):
                continue
            yield ReportRef(
                source_file=path,
                source_schema=schema,
                source_case_id=string_or_none(case.get("id")),
                source_input_path=string_or_none(case.get("input_path")),
                report=benchmark_case_as_report(case),
            )
        return


def benchmark_case_as_report(case: dict[str, Any]) -> dict[str, Any]:
    """Normalize benchmark cases enough for the sample exporter."""
    return {
        "schema_name": "CombatSearchV2BenchmarkCaseAsReport",
        "schema_version": 1,
        "input_label": case.get("start_label") or case.get("id"),
        "policy_evidence": case.get("policy_evidence"),
        "search_policy": case.get("search_policy"),
        "budget": case.get("budget"),
        "outcome": case.get("outcome"),
        "best_complete_trajectory": case.get("best_complete_trajectory"),
        "best_frontier_trajectory": case.get("best_frontier_trajectory"),
        "stats": case.get("stats"),
        "performance": case.get("performance"),
        "evidence_reliability": case.get("evidence_reliability"),
        "input_fingerprints": case.get("input_fingerprints"),
    }


def string_or_none(value: Any) -> str | None:
    if isinstance(value, str) and value:
        return value
    return None


def terminal_label(value: Any) -> str:
    if isinstance(value, str):
        return value.lower()
    return str(value).lower() if value is not None else "unknown"


def trajectory_is_usable(trajectory: Any, require_win: bool) -> bool:
    if not isinstance(trajectory, dict):
        return False
    actions = trajectory.get("actions")
    if not isinstance(actions, list) or not actions:
        return False
    if require_win and terminal_label(trajectory.get("terminal")) != "win":
        return False
    return True


def policy_evidence_summary(report: dict[str, Any]) -> dict[str, Any]:
    evidence = report.get("policy_evidence")
    if isinstance(evidence, dict):
        return {
            "information_access": evidence.get("information_access", "unknown"),
            "public_safe": evidence.get("public_safe", False),
            "hidden_information_risks": evidence.get("hidden_information_risks", []),
        }
    return {
        "information_access": "legacy_or_unknown",
        "public_safe": False,
        "hidden_information_risks": ["unknown_legacy_report_boundary"],
    }


def search_context(report: dict[str, Any]) -> dict[str, Any]:
    outcome = report.get("outcome") if isinstance(report.get("outcome"), dict) else {}
    stats = report.get("stats") if isinstance(report.get("stats"), dict) else {}
    budget = report.get("budget") if isinstance(report.get("budget"), dict) else {}
    reliability = (
        report.get("evidence_reliability")
        if isinstance(report.get("evidence_reliability"), dict)
        else {}
    )
    return {
        "input_label": report.get("input_label"),
        "coverage_status": outcome.get("coverage_status")
        or outcome.get("proof_status")
        or outcome.get("terminal"),
        "coverage_reason": outcome.get("coverage_reason") or outcome.get("reason"),
        "complete_trajectory_found": outcome.get("complete_trajectory_found"),
        "exhaustive": outcome.get("exhaustive"),
        "nodes_expanded": stats.get("nodes_expanded"),
        "nodes_generated": stats.get("nodes_generated"),
        "nodes_to_first_win": stats.get("nodes_to_first_win"),
        "terminal_wins": stats.get("terminal_wins"),
        "deadline_hit": stats.get("deadline_hit"),
        "node_budget_hit": stats.get("node_budget_hit"),
        "elapsed_ms": stats.get("elapsed_ms"),
        "max_nodes": budget.get("max_nodes"),
        "wall_time_ms": budget.get("wall_time_ms"),
        "max_potions_used": budget.get("max_potions_used"),
        "reliability": reliability.get("reliability"),
    }


def microscope_search_context(report: dict[str, Any]) -> dict[str, Any]:
    outcome = report.get("search_outcome") if isinstance(report.get("search_outcome"), dict) else {}
    config = report.get("config") if isinstance(report.get("config"), dict) else {}
    return {
        "input_label": report.get("input_label"),
        "coverage_status": outcome.get("coverage_status"),
        "coverage_reason": outcome.get("coverage_reason"),
        "complete_trajectory_found": outcome.get("complete_trajectory_found"),
        "exhaustive": outcome.get("exhaustive"),
        "max_nodes": config.get("max_nodes"),
        "wall_time_ms": config.get("wall_time_ms"),
        "max_potions_used": config.get("max_potions_used"),
        "frontier_policy": config.get("frontier_policy"),
        "rollout_policy": config.get("rollout_policy"),
    }


def trajectory_outcome(trajectory: dict[str, Any]) -> dict[str, Any]:
    return {
        "terminal": trajectory.get("terminal"),
        "estimated": trajectory.get("estimated"),
        "final_hp": trajectory.get("final_hp"),
        "final_block": trajectory.get("final_block"),
        "hp_loss": trajectory.get("hp_loss"),
        "turns": trajectory.get("turns"),
        "potions_used": trajectory.get("potions_used"),
        "potions_discarded": trajectory.get("potions_discarded"),
        "cards_played": trajectory.get("cards_played"),
    }


def action_class(action_key: str) -> str:
    if not action_key:
        return "unknown"
    parts = action_key.split("/")
    if len(parts) >= 2 and parts[0] == "combat":
        return f"{parts[0]}/{parts[1]}"
    return parts[0]


def selected_action_key(report: dict[str, Any]) -> str | None:
    selected = report.get("selected_first_action")
    if isinstance(selected, dict):
        return string_or_none(selected.get("action_key"))
    return None


def build_microscope_candidate_samples(ref: ReportRef) -> list[dict[str, Any]]:
    report = ref.report
    candidates = report.get("candidates")
    if not isinstance(candidates, list) or not candidates:
        return []
    selected_key = selected_action_key(report)
    context = microscope_search_context(report)
    best_summary = report.get("best_complete_summary")
    samples = []
    for candidate in candidates:
        if not isinstance(candidate, dict):
            continue
        action_key = string_or_none(candidate.get("action_key")) or ""
        selected = bool(candidate.get("selected_by_best_complete"))
        samples.append(
            {
                "schema_name": SCHEMA_NAME,
                "schema_version": SCHEMA_VERSION,
                "label_role": LABEL_ROLE,
                "target_kind": MICROSCOPE_TARGET_KIND,
                "candidate_coverage": MICROSCOPE_CANDIDATE_COVERAGE,
                "source": {
                    "file": str(ref.source_file),
                    "source_schema": ref.source_schema,
                    "case_id": ref.source_case_id,
                    "input_path": ref.source_input_path,
                },
                "policy_evidence": {
                    "information_access": "privileged_simulator_or_legacy_microscope",
                    "public_safe": False,
                    "hidden_information_risks": [
                        "privileged_simulator_state",
                        "exact_rng_state",
                    ],
                },
                "search_context": context,
                "best_complete_summary": best_summary,
                "initial_context": report.get("initial_context"),
                "label": {
                    "selected_by_best_complete": selected,
                    "selected_action_key": selected_key,
                },
                "candidate": {
                    "original_action_id": candidate.get("original_action_id"),
                    "ordered_index": candidate.get("ordered_index"),
                    "action_key": action_key,
                    "action_class": action_class(action_key),
                    "action_debug": candidate.get("action_debug"),
                    "action_role": candidate.get("action_role"),
                    "input": candidate.get("input"),
                    "action_facts": candidate.get("action_facts"),
                    "one_step": candidate.get("one_step"),
                },
                "limitations": [
                    "root_decision_only",
                    "label_from_best_complete_trajectory_under_budget",
                    "not_calibrated_for_human_public_policy",
                ],
            }
        )
    return samples


def build_samples(
    ref: ReportRef,
    *,
    max_actions_per_report: int | None,
    require_win: bool,
    include_prefix: bool,
) -> list[dict[str, Any]]:
    report = ref.report
    trajectory = report.get("best_complete_trajectory")
    if not trajectory_is_usable(trajectory, require_win=require_win):
        return []
    assert isinstance(trajectory, dict)
    actions = trajectory.get("actions")
    assert isinstance(actions, list)
    if max_actions_per_report is not None:
        actions = actions[:max_actions_per_report]

    policy = policy_evidence_summary(report)
    context = search_context(report)
    outcome = trajectory_outcome(trajectory)
    samples = []
    prefix: list[str] = []
    for ordinal, action in enumerate(actions):
        if not isinstance(action, dict):
            continue
        action_key = string_or_none(action.get("action_key")) or ""
        sample = {
            "schema_name": SCHEMA_NAME,
            "schema_version": SCHEMA_VERSION,
            "label_role": LABEL_ROLE,
            "target_kind": TRAJECTORY_TARGET_KIND,
            "candidate_coverage": TRAJECTORY_CANDIDATE_COVERAGE,
            "source": {
                "file": str(ref.source_file),
                "source_schema": ref.source_schema,
                "case_id": ref.source_case_id,
                "input_path": ref.source_input_path,
            },
            "policy_evidence": policy,
            "search_context": context,
            "trajectory_outcome": outcome,
            "step": {
                "trajectory_action_ordinal": ordinal,
                "is_first_action": ordinal == 0,
                "step_index": action.get("step_index"),
                "action_id": action.get("action_id"),
                "action_key": action_key,
                "action_class": action_class(action_key),
                "action_debug": action.get("action_debug"),
                "input": action.get("input"),
                "prefix_len": len(prefix),
            },
            "limitations": [
                "positive_only_best_complete_trajectory",
                "no_sibling_legal_action_negatives",
                "not_calibrated_for_human_public_policy",
            ],
        }
        if include_prefix:
            sample["step"]["prefix_action_keys"] = list(prefix)
        samples.append(sample)
        prefix.append(action_key)
    return samples


def summarize(samples: list[dict[str, Any]], skipped_reports: int, seen_reports: int) -> str:
    first_actions = Counter()
    action_classes = Counter()
    source_schemas = Counter()
    coverage = Counter()
    target_kinds = Counter()
    selected_labels = Counter()
    for sample in samples:
        source_schemas[sample["source"]["source_schema"]] += 1
        target_kinds[sample["target_kind"]] += 1
        coverage[sample["search_context"].get("coverage_status") or "unknown"] += 1
        if "step" in sample:
            action_classes[sample["step"]["action_class"]] += 1
            if sample["step"]["is_first_action"]:
                first_actions[sample["step"]["action_key"]] += 1
        elif "candidate" in sample:
            action_classes[sample["candidate"]["action_class"]] += 1
            selected = bool(sample.get("label", {}).get("selected_by_best_complete"))
            selected_labels["selected" if selected else "not_selected"] += 1
            if selected:
                first_actions[sample["candidate"]["action_key"]] += 1

    lines = [
        "CombatSearchGuidanceSampleV1 export",
        f"  reports_seen={seen_reports} reports_without_usable_best_win={skipped_reports}",
        f"  samples={len(samples)} first_action_samples={sum(first_actions.values())}",
        "  readiness=inspection_or_root_candidate_ranking",
        "  usable_for=trajectory_pattern_inspection, root_first_action_ranking_experiments",
        "  not_usable_for=full_combat_policy_without_deeper_state/action_labels",
    ]
    if target_kinds:
        lines.append(
            "  target_kinds="
            + ", ".join(f"{key}:{value}" for key, value in target_kinds.most_common())
        )
    if selected_labels:
        lines.append(
            "  root_candidate_labels="
            + ", ".join(f"{key}:{value}" for key, value in selected_labels.most_common())
        )
    if source_schemas:
        lines.append(
            "  source_schemas="
            + ", ".join(f"{key}:{value}" for key, value in source_schemas.most_common())
        )
    if coverage:
        lines.append(
            "  coverage_status="
            + ", ".join(f"{key}:{value}" for key, value in coverage.most_common())
        )
    if action_classes:
        lines.append("  top_action_classes:")
        for key, value in action_classes.most_common(8):
            lines.append(f"    {key}: {value}")
    if first_actions:
        lines.append("  top_first_actions:")
        for key, value in first_actions.most_common(8):
            lines.append(f"    {key}: {value}")
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="+", type=Path, help="JSON reports/evidence to read")
    parser.add_argument("--out", type=Path, help="Write JSONL samples to this path")
    parser.add_argument(
        "--max-actions-per-report",
        type=int,
        default=None,
        help="Optional cap for long trajectories",
    )
    parser.add_argument(
        "--allow-non-win",
        action="store_true",
        help="Also export non-win best_complete_trajectory actions",
    )
    parser.add_argument(
        "--include-prefix",
        action="store_true",
        help="Include previous action keys in each step sample",
    )
    args = parser.parse_args()

    all_samples: list[dict[str, Any]] = []
    seen_reports = 0
    skipped_reports = 0
    for path in args.inputs:
        payload = load_json(path)
        refs = list(iter_report_refs(path, payload))
        if not refs:
            print(f"warning: no supported combat search reports in {path}")
        for ref in refs:
            seen_reports += 1
            if ref.report.get("schema_name") == "CombatSearchV2DecisionMicroscopeReport":
                samples = build_microscope_candidate_samples(ref)
            else:
                samples = build_samples(
                    ref,
                    max_actions_per_report=args.max_actions_per_report,
                    require_win=not args.allow_non_win,
                    include_prefix=args.include_prefix,
                )
            if samples:
                all_samples.extend(samples)
            else:
                skipped_reports += 1

    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        with args.out.open("w", encoding="utf-8") as fh:
            for sample in all_samples:
                fh.write(json.dumps(sample, ensure_ascii=False, separators=(",", ":")))
                fh.write("\n")

    print(summarize(all_samples, skipped_reports, seen_reports))
    if args.out:
        print(f"  output={args.out}")


if __name__ == "__main__":
    main()
