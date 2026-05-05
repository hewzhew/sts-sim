#!/usr/bin/env python3
"""Build a conservative card-choice pretraining dataset from cashout rollout labels.

All labels are policy-conditional preferences, not absolute truth.
Only high_confidence_candidate status enters the training set.
rollout_unstable / needs_rollout / continuation_policy_conflict are excluded.

Usage:
    python build_card_choice_pretrain_dataset.py \
        --cashout-report tools/artifacts/card_cashout_lab/.../cashout_report.json \
        --candidate-outcomes tools/artifacts/.../candidate_outcomes.jsonl \
        --pairwise-labels tools/artifacts/.../pairwise_labels.jsonl \
        --out-dir tools/artifacts/card_choice_pretrain/
"""
from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json, write_jsonl


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Build conservative card-choice pretrain dataset")
    p.add_argument("--cashout-report", type=Path, required=True)
    p.add_argument("--candidate-outcomes", type=Path, required=True)
    p.add_argument("--pairwise-labels", type=Path, required=True)
    p.add_argument("--out-dir", type=Path, required=True)
    p.add_argument("--min-effect-hp", type=int, default=2,
                   help="Minimum HP delta for an edge to count as meaningful")
    p.add_argument("--min-effect-floor", type=int, default=0,
                   help="Minimum floor delta for an edge to count as meaningful")
    return p.parse_args()


# ---------------------------------------------------------------------------
# Loading
# ---------------------------------------------------------------------------

def load_cashout_report(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


# ---------------------------------------------------------------------------
# Classification
# ---------------------------------------------------------------------------

TRAIN_STATUSES = {"high_confidence_candidate"}
EXCLUDE_STATUSES = {
    "needs_rollout",
    "rollout_unstable",
    "continuation_policy_conflict",
    "cashout_disagreement_with_rule_baseline",
}
EQUIVALENT_STATUSES = {"rollout_equivalent"}


def classify_pairwise_row(row: dict[str, Any]) -> str:
    """Return 'train' | 'exclude' | 'equivalent' for a pairwise label row."""
    status = row.get("source_calibration_status", "unknown")
    if status in TRAIN_STATUSES:
        return "train"
    if status in EXCLUDE_STATUSES:
        return "exclude"
    if status in EQUIVALENT_STATUSES:
        return "equivalent"
    return "exclude"


# ---------------------------------------------------------------------------
# Group building
# ---------------------------------------------------------------------------

def build_candidate_groups(
    outcomes: list[dict[str, Any]],
    pairwise: list[dict[str, Any]],
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    """Build candidate groups and pairwise edges from outcomes and labels.

    A "group" is all candidates for a single reward decision (same case_id).
    """
    # Index outcomes by case_id
    outcomes_by_case: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in outcomes:
        outcomes_by_case[row["case_id"]].append(row)

    # Index pairwise by case_id
    pairwise_by_case: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in pairwise:
        pairwise_by_case[row["case_id"]].append(row)

    all_case_ids = set(outcomes_by_case.keys()) | set(pairwise_by_case.keys())

    groups: list[dict[str, Any]] = []
    edges: list[dict[str, Any]] = []
    stats = Counter()

    for case_id in sorted(all_case_ids):
        case_outcomes = outcomes_by_case.get(case_id, [])
        case_pairwise = pairwise_by_case.get(case_id, [])

        # Build candidate list: one entry per unique candidate_key
        candidates: dict[str, dict[str, Any]] = {}
        for row in case_outcomes:
            c = row.get("candidate") or {}
            key = c.get("candidate_key", "unknown")
            if key not in candidates:
                candidates[key] = {
                    "candidate_key": key,
                    "candidate_index": c.get("candidate_index"),
                    "card_id": c.get("card_id"),
                    "outcomes": [],
                }
            candidates[key]["outcomes"].append({
                "continuation_policy": row.get("continuation_policy"),
                "horizon": row.get("horizon"),
                "end_hp": c.get("end_hp"),
                "end_floor": c.get("end_floor"),
                "end_result": c.get("end_result"),
                "hp_delta": c.get("hp_delta"),
                "floor_delta": c.get("floor_delta"),
                "combat_win_delta": c.get("combat_win_delta"),
                "source_calibration_status": row.get("source_calibration_status"),
            })

        # Classify pairwise edges
        train_edges = []
        equivalent_edges = []
        excluded_edges = []

        for edge in case_pairwise:
            klass = classify_pairwise_row(edge)
            if klass == "train":
                train_edges.append(edge)
                stats["train_edges"] += 1
            elif klass == "equivalent":
                equivalent_edges.append(edge)
                stats["equivalent_edges"] += 1
            else:
                excluded_edges.append(edge)
                stats["excluded_edges"] += 1

        # Only emit group if it has at least one train edge
        if train_edges:
            groups.append({
                "case_id": case_id,
                "candidates": list(candidates.values()),
                "train_edge_count": len(train_edges),
                "equivalent_edge_count": len(equivalent_edges),
                "excluded_edge_count": len(excluded_edges),
            })
            edges.extend(train_edges)
            stats["train_groups"] += 1
        else:
            stats["skipped_groups"] += 1

    print(f"Groups: {stats['train_groups']} train, {stats['skipped_groups']} skipped")
    print(f"Edges: {stats['train_edges']} train, {stats['equivalent_edges']} equivalent, {stats['excluded_edges']} excluded")
    return groups, edges


# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------

def build_report(
    groups: list[dict[str, Any]],
    edges: list[dict[str, Any]],
    cashout_report: dict[str, Any],
    args: argparse.Namespace,
) -> dict[str, Any]:
    # Count per continuation policy
    policy_counts = Counter(e.get("continuation_policy", "?") for e in edges)
    reason_counts = Counter(e.get("reason", "?") for e in edges)
    horizon_counts = Counter(str(e.get("horizon", "?")) for e in edges)

    # Count unique cards
    all_card_ids: set[str] = set()
    for g in groups:
        for c in g["candidates"]:
            cid = c.get("card_id")
            if cid:
                all_card_ids.add(cid)

    return {
        "report_version": "card_choice_pretrain_v0",
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {
            "cashout_report": str(args.cashout_report),
            "candidate_outcomes": str(args.candidate_outcomes),
            "pairwise_labels": str(args.pairwise_labels),
            "min_effect_hp": args.min_effect_hp,
            "min_effect_floor": args.min_effect_floor,
            "train_statuses": sorted(TRAIN_STATUSES),
            "excluded_statuses": sorted(EXCLUDE_STATUSES),
        },
        "summary": {
            "train_groups": len(groups),
            "train_edges": len(edges),
            "unique_cards": len(all_card_ids),
            "policy_distribution": dict(policy_counts),
            "reason_distribution": dict(reason_counts),
            "horizon_distribution": dict(horizon_counts),
        },
    }


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    args = parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)

    print(f"Loading cashout report: {args.cashout_report}")
    cashout = load_cashout_report(args.cashout_report)

    print(f"Loading candidate outcomes: {args.candidate_outcomes}")
    outcomes = load_jsonl(args.candidate_outcomes)
    print(f"  {len(outcomes)} rows")

    print(f"Loading pairwise labels: {args.pairwise_labels}")
    pairwise = load_jsonl(args.pairwise_labels)
    print(f"  {len(pairwise)} rows")

    groups, edges = build_candidate_groups(outcomes, pairwise)

    # Write outputs
    groups_path = args.out_dir / "card_choice_candidate_groups.jsonl"
    write_jsonl(groups_path, groups)
    print(f"Wrote {groups_path}")

    edges_path = args.out_dir / "card_choice_pairwise_edges.jsonl"
    write_jsonl(edges_path, edges)
    print(f"Wrote {edges_path}")

    report = build_report(groups, edges, cashout, args)
    report_path = args.out_dir / "pretrain_report.json"
    write_json(report_path, report)
    print(f"Wrote {report_path}")

    # Quick summary
    s = report["summary"]
    print(f"\nDone. {s['train_groups']} groups, {s['train_edges']} edges, {s['unique_cards']} cards.")


if __name__ == "__main__":
    main()
