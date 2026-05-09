#!/usr/bin/env python3
"""Build model-guided family evidence requests.

Input rows are family_search_allocation_model predictions. Output rows are
family_evidence_request_v0 records consumable by targeted branch trace
collection. This is search-allocation plumbing, not an action policy.
"""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any, Iterable


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


def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return default
    return number if math.isfinite(number) else default


def assert_no_action_label_leak(row: dict[str, Any], *, index: int) -> None:
    if row.get("trainable_as_action_label") is not False:
        raise ValueError(f"family row {index} is action-label-like")
    if (row.get("label_policy") or {}).get("action_label") is not False:
        raise ValueError(f"family row {index} has action_label=true")
    serialized = json.dumps(row, sort_keys=True, separators=(",", ":"))
    for key in FORBIDDEN_LABEL_KEYS:
        if f'"{key}"' in serialized:
            raise ValueError(f"family row {index} contains forbidden key {key}")


def load_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for index, row in enumerate(iter_jsonl(path)):
        assert_no_action_label_leak(row, index=index)
        rows.append(row)
    return rows


def score_value(row: dict[str, Any], score_name: str) -> float:
    signals = row.get("search_allocation_signals") or {}
    outputs = row.get("model_outputs") or {}
    if score_name in signals:
        return safe_float(signals.get(score_name))
    if score_name in outputs:
        return safe_float(outputs.get(score_name))
    return 0.0


def action_summary(candidate: dict[str, Any]) -> dict[str, Any]:
    return {
        "action_kind": candidate.get("action_kind"),
        "action_type": candidate.get("action_type"),
        "action_key": candidate.get("action_key"),
        "card_id": candidate.get("card_id"),
    }


def request_from_family(
    row: dict[str, Any],
    *,
    rank: int,
    score_name: str,
    score: float,
    include_audit_targets: bool,
) -> dict[str, Any]:
    pair = row.get("representative_pair") or {}
    left = pair.get("left") or {}
    right = pair.get("right") or {}
    out = {
        "schema_version": "family_evidence_request_v0",
        "trainable_role": "model_guided_contrast_family_evidence_request",
        "trainable_as_action_label": False,
        "episode_seed": row.get("episode_seed"),
        "episode_step": row.get("episode_step"),
        "decision_id": row.get("decision_id"),
        "family_allocator": {
            "schema_version": "family_search_allocation_model_v0",
            "family_mode": row.get("family_mode"),
            "family": row.get("family"),
            "family_rank": rank,
            "score_name": score_name,
            "score": score,
            "budget_item_role": "model_guided_family_representative_pair",
        },
        "priority_score": score,
        "reasons": [
            "model_guided_contrast_family_evidence_request",
            f"family_mode:{row.get('family_mode')}",
            f"family:{row.get('family')}",
            f"score:{score_name}",
        ],
        "pair_kind": (
            f"{(left.get('candidate') or {}).get('action_kind')}"
            f"->{(right.get('candidate') or {}).get('action_kind')}"
        ),
        "pair_card": (
            f"{(left.get('candidate') or {}).get('card_id')}"
            f"->{(right.get('candidate') or {}).get('card_id')}"
        ),
        "left": {
            "branch_id": left.get("branch_id"),
            "candidate": action_summary(left.get("candidate") or {}),
        },
        "right": {
            "branch_id": right.get("branch_id"),
            "candidate": action_summary(right.get("candidate") or {}),
        },
        "model_outputs": row.get("model_outputs") or {},
        "label_policy": {
            "action_label": False,
            "source": "family_search_allocation_model_v0",
        },
    }
    if include_audit_targets:
        out["audit_targets"] = row.get("targets") or {}
    return out


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--family-predictions", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    parser.add_argument("--budget", type=int, default=3)
    parser.add_argument("--score-name", default="family_abs_ge_10_probability")
    parser.add_argument("--include-audit-targets", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    rows = load_rows(args.family_predictions)
    by_decision: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        by_decision[str(row.get("decision_key"))].append(row)

    requests: list[dict[str, Any]] = []
    for decision_rows in by_decision.values():
        ranked = sorted(
            decision_rows,
            key=lambda row: (-score_value(row, args.score_name), str(row.get("family"))),
        )
        for rank, row in enumerate(ranked[: args.budget], start=1):
            requests.append(
                request_from_family(
                    row,
                    rank=rank,
                    score_name=args.score_name,
                    score=score_value(row, args.score_name),
                    include_audit_targets=args.include_audit_targets,
                )
            )

    family_counts = Counter((request.get("family_allocator") or {}).get("family") for request in requests)
    pair_kind_counts = Counter(request.get("pair_kind") for request in requests)
    target10_hits = 0
    target10_mass = 0.0
    for request in requests:
        targets = request.get("audit_targets") or {}
        if targets.get("high_regret_abs10"):
            target10_hits += 1
        target10_mass += safe_float(targets.get("regret_mass_abs10"))

    summary = {
        "schema_version": "model_guided_family_evidence_request_summary_v0",
        "family_predictions": str(args.family_predictions),
        "decision_count": len(by_decision),
        "request_count": len(requests),
        "budget": args.budget,
        "score_name": args.score_name,
        "family_counts_top": dict(family_counts.most_common(30)),
        "pair_kind_counts": dict(pair_kind_counts.most_common(30)),
        "audit_selected_target_family_rows_abs10": target10_hits if args.include_audit_targets else None,
        "audit_selected_regret_mass_abs10": target10_mass if args.include_audit_targets else None,
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "model_guided_requests_are_search_allocation_not_policy": True,
        },
    }

    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8") as handle:
        for request in requests:
            handle.write(json.dumps(request, separators=(",", ":")) + "\n")
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    args.summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
