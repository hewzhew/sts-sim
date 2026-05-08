#!/usr/bin/env python3
"""Audit DecisionRecord JSONL for contract and public-observation hygiene."""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any, Iterable


BANNED_PUBLIC_KEYS = {
    "dominated",
    "dominated_by_index",
    "estimated_role_scores",
    "likely_waste",
    "plan_delta",
    "plan_profile",
    "reward_structure",
}
BANNED_PUBLIC_KEY_SUBSTRINGS = ("score",)


def iter_jsonl(paths: list[Path]) -> Iterable[tuple[Path, int, dict[str, Any]]]:
    for path in paths:
        with path.open("r", encoding="utf-8") as handle:
            for line_no, line in enumerate(handle, start=1):
                line = line.strip()
                if not line:
                    continue
                yield path, line_no, json.loads(line)


def action_id_value(value: Any) -> int | None:
    if value is None:
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, dict) and "0" in value and isinstance(value["0"], int):
        return value["0"]
    if isinstance(value, list) and len(value) == 1 and isinstance(value[0], int):
        return value[0]
    return None


def public_payload_violations(value: Any, path: str = "$") -> list[str]:
    violations: list[str] = []
    if isinstance(value, dict):
        for key, child in value.items():
            key_text = str(key)
            key_lower = key_text.lower()
            child_path = f"{path}.{key_text}"
            if key_lower in BANNED_PUBLIC_KEYS or any(
                needle in key_lower for needle in BANNED_PUBLIC_KEY_SUBSTRINGS
            ):
                violations.append(child_path)
            violations.extend(public_payload_violations(child, child_path))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            violations.extend(public_payload_violations(child, f"{path}[{index}]"))
    return violations


def add_failure(
    report: dict[str, Any],
    *,
    path: Path,
    line_no: int,
    record: dict[str, Any],
    kind: str,
    details: dict[str, Any],
    max_examples: int,
) -> None:
    report["failure_count"] += 1
    report["failure_kind_counts"][kind] += 1
    if len(report["failure_examples"]) < max_examples:
        report["failure_examples"].append(
            {
                "path": str(path),
                "line": line_no,
                "kind": kind,
                "decision_id": record.get("decision_id"),
                "seed": record.get("seed"),
                "details": details,
            }
        )


def audit_record(
    report: dict[str, Any],
    *,
    path: Path,
    line_no: int,
    record: dict[str, Any],
    max_examples: int,
) -> bool:
    ok = True

    if record.get("schema_version") != "decision_record_v0":
        ok = False
        add_failure(
            report,
            path=path,
            line_no=line_no,
            record=record,
            kind="unexpected_record_schema",
            details={"schema_version": record.get("schema_version")},
            max_examples=max_examples,
        )

    observation = record.get("observation") or {}
    if observation.get("visibility") != "public":
        ok = False
        add_failure(
            report,
            path=path,
            line_no=line_no,
            record=record,
            kind="non_public_observation",
            details={"visibility": observation.get("visibility")},
            max_examples=max_examples,
        )

    observation_schema = observation.get("schema_version")
    if record.get("observation_schema_version") != observation_schema:
        ok = False
        add_failure(
            report,
            path=path,
            line_no=line_no,
            record=record,
            kind="observation_schema_mismatch",
            details={
                "record": record.get("observation_schema_version"),
                "observation": observation_schema,
            },
            max_examples=max_examples,
        )

    observation_violations = public_payload_violations(observation.get("payload"))
    if observation_violations:
        ok = False
        add_failure(
            report,
            path=path,
            line_no=line_no,
            record=record,
            kind="public_observation_leakage_key",
            details={"paths": observation_violations[:20]},
            max_examples=max_examples,
        )

    candidate_ids: set[int] = set()
    action_schema = record.get("action_schema_version")
    for candidate_index, candidate in enumerate(record.get("candidates") or []):
        candidate_id = action_id_value(candidate.get("id"))
        if candidate_id is not None:
            candidate_ids.add(candidate_id)
        if candidate.get("action_schema_version") != action_schema:
            ok = False
            add_failure(
                report,
                path=path,
                line_no=line_no,
                record=record,
                kind="candidate_schema_mismatch",
                details={
                    "candidate_index": candidate_index,
                    "record": action_schema,
                    "candidate": candidate.get("action_schema_version"),
                },
                max_examples=max_examples,
            )
        candidate_violations = public_payload_violations(candidate.get("payload"))
        if candidate_violations:
            ok = False
            add_failure(
                report,
                path=path,
                line_no=line_no,
                record=record,
                kind="public_candidate_leakage_key",
                details={
                    "candidate_index": candidate_index,
                    "paths": candidate_violations[:20],
                },
                max_examples=max_examples,
            )

    behavior_action = action_id_value(record.get("behavior_action"))
    if behavior_action is not None and behavior_action not in candidate_ids:
        ok = False
        add_failure(
            report,
            path=path,
            line_no=line_no,
            record=record,
            kind="behavior_action_not_in_candidates",
            details={"behavior_action": behavior_action, "candidate_ids": sorted(candidate_ids)[:20]},
            max_examples=max_examples,
        )

    if record.get("terminated") and record.get("truncated"):
        ok = False
        add_failure(
            report,
            path=path,
            line_no=line_no,
            record=record,
            kind="terminated_and_truncated",
            details={"terminated": record.get("terminated"), "truncated": record.get("truncated")},
            max_examples=max_examples,
        )

    return ok


def audit(paths: list[Path], max_examples: int) -> dict[str, Any]:
    report: dict[str, Any] = {
        "schema_version": "decision_record_contract_audit_v0",
        "inputs": [str(path) for path in paths],
        "record_count": 0,
        "passed_record_count": 0,
        "failure_count": 0,
        "failure_kind_counts": Counter(),
        "failure_examples": [],
    }
    for path, line_no, record in iter_jsonl(paths):
        report["record_count"] += 1
        if audit_record(report, path=path, line_no=line_no, record=record, max_examples=max_examples):
            report["passed_record_count"] += 1
    report["failure_kind_counts"] = dict(sorted(report["failure_kind_counts"].items()))
    report["passed_rate"] = (
        report["passed_record_count"] / report["record_count"] if report["record_count"] else 0.0
    )
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--inputs", nargs="+", type=Path, required=True)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--max-examples", type=int, default=20)
    parser.add_argument("--fail-on-violation", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    report = audit(args.inputs, args.max_examples)
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if args.fail_on_violation and report["failure_count"]:
        raise SystemExit(f"contract audit failed with {report['failure_count']} failures")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
