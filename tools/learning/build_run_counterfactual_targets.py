#!/usr/bin/env python3
"""Build run-level counterfactual evidence targets from failure classes.

This is not a policy and does not select actions. A target says "this decision
family needs counterfactual evidence under a named horizon/gate." The closed-loop
runner may later evaluate candidates at matching decisions and still abstain.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


TARGET_SCHEMA_VERSION = "run_counterfactual_targets_v1"


def safe_int(value: Any, default: int = 0) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False), encoding="utf-8")


def snapshots_for(record: dict[str, Any], decision_types: set[str]) -> list[dict[str, Any]]:
    summary = record.get("candidate_snapshots") or {}
    out = []
    for snapshot in summary.get("snapshots") or []:
        if str(snapshot.get("decision_type") or "unknown") in decision_types:
            out.append(snapshot)
    return out


def candidate_keys(snapshot: dict[str, Any], limit: int = 24) -> list[str]:
    keys = []
    for candidate in snapshot.get("candidates") or []:
        key = candidate.get("action_key")
        if key is not None:
            keys.append(str(key))
    return keys[:limit]


def target_gate_for_family(family: str) -> dict[str, Any]:
    if family in {"route_to_shop", "shop_purchase", "campfire_smith_rest_counterfactual", "campfire_upgrade"}:
        return {
            "schema_version": "counterfactual_gate_v1",
            "horizon": "act1_boss_or_next_combat_complete",
            "survival_must_not_regress": True,
            "boss_clear_must_not_regress": True,
            "min_hp_or_outcome_margin_source": "audit_noise_gate_required_before_override",
            "incomplete_evidence_action": "abstain",
        }
    if family in {"card_reward", "shop_card"}:
        return {
            "schema_version": "counterfactual_gate_v1",
            "horizon": "act1_boss_or_act2_entry_gauntlet",
            "survival_must_not_regress": True,
            "boss_clear_must_not_regress": True,
            "min_hp_or_outcome_margin_source": "audit_noise_gate_required_before_override",
            "incomplete_evidence_action": "abstain",
        }
    return {
        "schema_version": "counterfactual_gate_v1",
        "horizon": "gauntlet_evidence_only",
        "survival_must_not_regress": True,
        "boss_clear_must_not_regress": True,
        "min_hp_or_outcome_margin_source": "not_applicable_diagnostic_target",
        "incomplete_evidence_action": "abstain",
    }


FAMILY_DECISION_TYPES: dict[str, set[str]] = {
    "route_to_shop": {"map"},
    "shop_purchase": {"shop"},
    "campfire_smith_rest_counterfactual": {"campfire"},
    "campfire_upgrade": {"campfire"},
    "card_reward": {"reward"},
    "shop_card": {"shop"},
}


def build_targets_for_record(record: dict[str, Any]) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    seed = safe_int(record.get("seed"))
    targets: list[dict[str, Any]] = []
    unavailable: list[dict[str, Any]] = []
    seen_keys: set[tuple[str, str, int | None]] = set()
    snapshot_summary = record.get("candidate_snapshots") or {}
    old_log_missing_snapshots = (
        safe_int(snapshot_summary.get("snapshot_count")) == 0
        and safe_int(snapshot_summary.get("missing_noncombat_snapshot_count")) > 0
    )
    for failure in record.get("failure_classes_v1") or []:
        failure_class = str(failure.get("class") or "unknown")
        for family in failure.get("target_families") or []:
            family = str(family)
            decision_types = FAMILY_DECISION_TYPES.get(family)
            if decision_types is None:
                target_id = f"seed:{seed}:class:{failure_class}:family:{family}:gauntlet"
                key = (failure_class, family, None)
                if key in seen_keys:
                    continue
                seen_keys.add(key)
                targets.append(
                    {
                        "schema_version": TARGET_SCHEMA_VERSION,
                        "target_id": target_id,
                        "seed": seed,
                        "source_failure_class": failure_class,
                        "target_family": family,
                        "target_type": "gauntlet_evidence_target",
                        "decision_step": None,
                        "decision_type": None,
                        "candidate_snapshot_available": False,
                        "candidate_count": 0,
                        "candidate_action_keys": [],
                        "counterfactual_question": (
                            "Does this run state need deeper run/gauntlet evidence for the named "
                            "failure class? This target does not name an action."
                        ),
                        "gate": target_gate_for_family(family),
                        "failure_basis": failure.get("basis") or {},
                        "trainable_as_action_label": False,
                        "describes_action_choice": False,
                    }
                )
                continue

            matches = snapshots_for(record, decision_types)
            if not matches:
                unavailable.append(
                    {
                        "seed": seed,
                        "source_failure_class": failure_class,
                        "target_family": family,
                        "reason": "candidate_snapshot_missing"
                        if old_log_missing_snapshots
                        else "counterfactual_target_unavailable",
                        "expected_decision_types": sorted(decision_types),
                        "candidate_snapshot_summary": {
                            "snapshot_count": safe_int(snapshot_summary.get("snapshot_count")),
                            "missing_noncombat_snapshot_count": safe_int(
                                snapshot_summary.get("missing_noncombat_snapshot_count")
                            ),
                        },
                        "failure_basis": failure.get("basis") or {},
                    }
                )
                continue
            for snapshot in matches:
                step = safe_int(snapshot.get("step"), -1)
                key = (failure_class, family, step)
                if key in seen_keys:
                    continue
                seen_keys.add(key)
                target_id = f"seed:{seed}:step:{step}:class:{failure_class}:family:{family}"
                targets.append(
                    {
                        "schema_version": TARGET_SCHEMA_VERSION,
                        "target_id": target_id,
                        "seed": seed,
                        "source_failure_class": failure_class,
                        "target_family": family,
                        "target_type": "decision_counterfactual_target",
                        "decision_step": step,
                        "decision_type": snapshot.get("decision_type"),
                        "candidate_snapshot_available": True,
                        "candidate_count": safe_int(snapshot.get("candidate_count")),
                        "candidate_action_keys": candidate_keys(snapshot),
                        "counterfactual_question": (
                            "At this decision, do any available candidates improve the named "
                            "run-level objective under the registered evidence horizon and gate?"
                        ),
                        "gate": target_gate_for_family(family),
                        "failure_basis": failure.get("basis") or {},
                        "trainable_as_action_label": False,
                        "describes_action_choice": False,
                    }
                )
    return targets, unavailable


def build_targets(audit: dict[str, Any]) -> dict[str, Any]:
    all_targets: list[dict[str, Any]] = []
    unavailable: list[dict[str, Any]] = []
    for record in audit.get("records") or []:
        targets, missing = build_targets_for_record(record)
        all_targets.extend(targets)
        unavailable.extend(missing)

    counts = Counter(str(target.get("target_family")) for target in all_targets)
    by_class: dict[str, Counter[str]] = defaultdict(Counter)
    for target in all_targets:
        by_class[str(target.get("source_failure_class"))][str(target.get("target_family"))] += 1
    unavailable_counts = Counter(str(item.get("reason")) for item in unavailable)
    return {
        "schema_version": TARGET_SCHEMA_VERSION,
        "source_audit_schema_version": audit.get("schema_version"),
        "run_count": safe_int(audit.get("run_count")),
        "target_count": len(all_targets),
        "unavailable_target_count": len(unavailable),
        "target_family_counts": dict(counts),
        "target_family_by_failure_class": {key: dict(value) for key, value in by_class.items()},
        "unavailable_reason_counts": dict(unavailable_counts),
        "targets": all_targets,
        "unavailable_targets": unavailable,
        "label_safety": {
            "trainable_as_action_label": False,
            "contains_policy_scores": False,
            "contains_winner_or_preference": False,
            "intended_use": "counterfactual_evidence_request_planning",
        },
    }


def markdown_report(payload: dict[str, Any], *, max_rows: int = 30) -> str:
    lines = [
        "# Run Counterfactual Targets V1",
        "",
        f"- targets: `{payload.get('target_count')}`",
        f"- unavailable targets: `{payload.get('unavailable_target_count')}`",
        f"- target families: `{payload.get('target_family_counts')}`",
        f"- unavailable reasons: `{payload.get('unavailable_reason_counts')}`",
        "",
        "## Targets",
        "",
        "| seed | step | class | family | decision | candidates | gate horizon |",
        "| --- | ---: | --- | --- | --- | ---: | --- |",
    ]
    for target in payload.get("targets", [])[:max_rows]:
        gate = target.get("gate") or {}
        lines.append(
            "| {seed} | {step} | {cls} | {family} | {decision} | {candidates} | {horizon} |".format(
                seed=target.get("seed"),
                step=target.get("decision_step"),
                cls=target.get("source_failure_class"),
                family=target.get("target_family"),
                decision=target.get("decision_type"),
                candidates=target.get("candidate_count"),
                horizon=gate.get("horizon"),
            )
        )
    lines.append("")
    return "\n".join(lines)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--audit", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--markdown-out", type=Path)
    parser.add_argument("--max-markdown-rows", type=int, default=40)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    payload = build_targets(read_json(args.audit))
    write_json(args.out, payload)
    if args.markdown_out:
        args.markdown_out.parent.mkdir(parents=True, exist_ok=True)
        args.markdown_out.write_text(
            markdown_report(payload, max_rows=args.max_markdown_rows), encoding="utf-8"
        )
    print(
        json.dumps(
            {key: value for key, value in payload.items() if key not in {"targets", "unavailable_targets"}},
            indent=2,
            ensure_ascii=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
