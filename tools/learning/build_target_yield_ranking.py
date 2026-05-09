#!/usr/bin/env python3
"""Build target-family yield ranking from targeted counterfactual A/B traces.

This is a search-allocation diagnostic, not a policy label. It estimates which
target family / candidate role buckets produced closed-loop overrides or useful
outcomes often enough to deserve branch evidence budget in the next run.
"""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


def safe_int(value: Any, default: int = 0) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            stripped = line.strip()
            if stripped:
                rows.append(json.loads(stripped))
    return rows


def yield_key(family: str, role: str, decision_type: str) -> str:
    return "|".join([family or "unknown", role or "none", decision_type or "unknown"])


def parse_yield_key(key: str, decision_type: str) -> tuple[str, str, str]:
    parts = str(key).split("|")
    if len(parts) >= 3:
        return parts[0] or "unknown", parts[1] or "none", parts[2] or decision_type
    if len(parts) == 2:
        return parts[0] or "unknown", parts[1] or "none", decision_type
    return str(key) or "unknown", "none", decision_type


def build_ranking(rows: list[dict[str, Any]]) -> dict[str, Any]:
    buckets: dict[str, dict[str, Any]] = defaultdict(
        lambda: {
            "yield_key": "",
            "target_family": "",
            "candidate_role": "",
            "decision_type": "",
            "evidence_request_count": 0,
            "override_count": 0,
            "bad_override_proxy_count": 0,
        }
    )
    for row in rows:
        if row.get("schema_version") != "targeted_counterfactual_step_record_v0":
            continue
        decision_type = str(row.get("decision_type") or "unknown")
        exact_keys = [str(item) for item in (row.get("target_family_role_keys") or [])]
        if exact_keys:
            bucket_specs = [parse_yield_key(key, decision_type) for key in exact_keys]
        else:
            families = [str(item) for item in (row.get("target_families") or [])]
            roles = [str(item) for item in (row.get("target_candidate_roles") or ["none"])]
            bucket_specs = [
                (family, role, decision_type)
                for family in families
                for role in roles
            ]
        if not bucket_specs:
            continue
        decision = row.get("decision") or {}
        mode = str(decision.get("mode") or "abstain")
        for family, role, key_decision_type in bucket_specs:
            key = yield_key(family, role, key_decision_type)
            bucket = buckets[key]
            bucket.update(
                {
                    "yield_key": key,
                    "target_family": family,
                    "candidate_role": role,
                    "decision_type": key_decision_type,
                }
            )
            # Count only actual branch evidence attempts, not identity mismatch
            # records without branch traces.
            if safe_int(row.get("branch_trace_count")) > 0:
                bucket["evidence_request_count"] += 1
            if mode == "counterfactual_override":
                bucket["override_count"] += 1
                if safe_int(decision.get("floor_gain_vs_behavior")) < 0 or safe_int(
                    decision.get("combat_win_count_gain_vs_behavior")
                ) < 0:
                    bucket["bad_override_proxy_count"] += 1

    rows_out = []
    for bucket in buckets.values():
        requests = safe_int(bucket.get("evidence_request_count"))
        if requests <= 0:
            continue
        overrides = safe_int(bucket.get("override_count"))
        bad = safe_int(bucket.get("bad_override_proxy_count"))
        score = 0.0
        if requests:
            score = max(0.0, (overrides - bad * 2) / requests)
        row = dict(bucket)
        row["yield_score"] = score
        row["trainable_as_action_label"] = False
        row["winner_or_preference_label_used"] = False
        rows_out.append(row)
    rows_out.sort(
        key=lambda item: (
            -float(item.get("yield_score") or 0.0),
            -safe_int(item.get("evidence_request_count")),
            str(item.get("yield_key")),
        )
    )
    return {
        "schema_version": "target_yield_ranking_v0",
        "source": "targeted_counterfactual_step_records",
        "yield_rows": rows_out,
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "intended_use": "evidence_budget_allocation",
        },
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--trace", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    payload = build_ranking(read_jsonl(args.trace))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(payload, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps(payload, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
