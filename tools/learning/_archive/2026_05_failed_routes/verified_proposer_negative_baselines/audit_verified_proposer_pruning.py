#!/usr/bin/env python3
"""Audit model proposer pruning against full-H verified candidate rows."""
from __future__ import annotations

import argparse
import json
import pickle
from collections import defaultdict
from pathlib import Path
from typing import Any

from return_q_common import predict_adv_override_probability, read_jsonl, write_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--misses-out", type=Path)
    parser.add_argument("--thresholds", default="0.3,0.5,0.7")
    parser.add_argument("--top-k", default="1")
    parser.add_argument("--max-misses", type=int, default=200)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    rows = read_jsonl(args.input)
    model = load_model(args.model)
    thresholds = parse_float_list(args.thresholds)
    top_ks = parse_int_list(args.top_k)
    groups: defaultdict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        groups[str(row.get("group_key") or "")].append(row)

    cases = {
        case_key(top_k, threshold): new_case(top_k, threshold)
        for top_k in top_ks
        for threshold in thresholds
    }
    misses: list[dict[str, Any]] = []
    summary = {
        "schema_version": "verified_proposer_pruning_audit_v0",
        "input": str(args.input),
        "model": str(args.model),
        "group_count": len(groups),
        "row_count": len(rows),
        "scored_non_rule_count": 0,
        "full_override_group_count": 0,
        "positive_candidate_count": 0,
        "cases": cases,
    }

    for group_key, group_rows in groups.items():
        rule = next((row for row in group_rows if bool(row.get("is_rule_choice"))), None)
        if not rule:
            continue
        observation = rule.get("observation") or {}
        rule_candidate = rule.get("rule_candidate") or rule.get("candidate") or {}
        non_rule_rows = [row for row in group_rows if not bool(row.get("is_rule_choice"))]
        if not non_rule_rows:
            continue
        scored = []
        for row in non_rule_rows:
            prob = predict_adv_override_probability(
                model,
                observation,
                row.get("candidate") or {},
                rule_candidate,
                row.get("cheap_return_features") or {},
                row.get("candidate_delta_vs_start") or {},
                rule.get("rule_delta_vs_start") or {},
                row.get("delta_vs_rule_features") or {},
            )
            scored.append((float(prob), row))
        scored.sort(key=lambda item: (-item[0], int(item[1].get("candidate_index") or 0)))
        summary["scored_non_rule_count"] += len(scored)
        chosen_rows = [
            row
            for row in non_rule_rows
            if bool(row.get("is_full_verified_choice"))
        ]
        positive_rows = [
            row
            for row in non_rule_rows
            if bool(row.get("passes_margin")) or str(row.get("safe_override_label")) == "positive"
        ]
        if chosen_rows:
            summary["full_override_group_count"] += 1
        summary["positive_candidate_count"] += len(positive_rows)

        for stats in cases.values():
            selected = selected_rows(scored, int(stats["top_k"]), float(stats["threshold"]))
            selected_ids = {id(row) for row in selected}
            stats["group_count"] += 1
            stats["selected_candidate_count"] += len(selected)
            stats["total_non_rule_candidate_count"] += len(scored)
            if chosen_rows:
                stats["chosen_group_count"] += 1
                if any(id(row) in selected_ids for row in chosen_rows):
                    stats["chosen_group_kept_count"] += 1
                else:
                    stats["chosen_group_missed_count"] += 1
                    if len(misses) < args.max_misses:
                        chosen = chosen_rows[0]
                        misses.append(
                            {
                                "case": stats["case"],
                                "group_key": group_key,
                                "split": chosen.get("split"),
                                "seed": chosen.get("seed"),
                                "step": chosen.get("step"),
                                "decision_kind": chosen.get("decision_kind"),
                                "candidate_index": chosen.get("candidate_index"),
                                "action_key": (chosen.get("candidate") or {}).get("action_key"),
                                "adv_vs_rule_mean": chosen.get("adv_vs_rule_mean"),
                                "score": next(
                                    (
                                        prob
                                        for prob, row in scored
                                        if row.get("candidate_index") == chosen.get("candidate_index")
                                    ),
                                    None,
                                ),
                                "selected_action_keys": [
                                    (row.get("candidate") or {}).get("action_key")
                                    for row in selected[:8]
                                ],
                            }
                        )
            stats["positive_candidate_count"] += len(positive_rows)
            positive_kept = sum(1 for row in positive_rows if id(row) in selected_ids)
            stats["positive_candidate_kept_count"] += positive_kept

    for stats in cases.values():
        finalize_case(stats)
    if args.misses_out:
        args.misses_out.parent.mkdir(parents=True, exist_ok=True)
        with args.misses_out.open("w", encoding="utf-8") as handle:
            for row in misses:
                handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")
    write_json(args.out, summary)
    print(json.dumps(render_compact(summary), indent=2, sort_keys=True))


def load_model(path: Path) -> dict[str, Any]:
    if path.suffix.lower() in {".pkl", ".pickle"}:
        with path.open("rb") as handle:
            return pickle.load(handle)
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def selected_rows(
    scored: list[tuple[float, dict[str, Any]]],
    top_k: int,
    threshold: float,
) -> list[dict[str, Any]]:
    selected: dict[int, dict[str, Any]] = {}
    if threshold >= 0:
        for score, row in scored:
            if score >= threshold:
                selected[id(row)] = row
    if top_k > 0:
        for _score, row in scored[:top_k]:
            selected[id(row)] = row
    if threshold < 0 and top_k <= 0:
        for _score, row in scored:
            selected[id(row)] = row
    return list(selected.values())


def new_case(top_k: int, threshold: float) -> dict[str, Any]:
    return {
        "case": case_key(top_k, threshold),
        "top_k": top_k,
        "threshold": threshold,
        "group_count": 0,
        "selected_candidate_count": 0,
        "total_non_rule_candidate_count": 0,
        "chosen_group_count": 0,
        "chosen_group_kept_count": 0,
        "chosen_group_missed_count": 0,
        "positive_candidate_count": 0,
        "positive_candidate_kept_count": 0,
    }


def finalize_case(stats: dict[str, Any]) -> None:
    total = int(stats["total_non_rule_candidate_count"])
    chosen = int(stats["chosen_group_count"])
    positives = int(stats["positive_candidate_count"])
    stats["candidate_keep_rate"] = (
        int(stats["selected_candidate_count"]) / total if total else None
    )
    stats["chosen_group_recall"] = (
        int(stats["chosen_group_kept_count"]) / chosen if chosen else None
    )
    stats["positive_candidate_recall"] = (
        int(stats["positive_candidate_kept_count"]) / positives if positives else None
    )


def render_compact(summary: dict[str, Any]) -> dict[str, Any]:
    return {
        "group_count": summary["group_count"],
        "row_count": summary["row_count"],
        "scored_non_rule_count": summary["scored_non_rule_count"],
        "full_override_group_count": summary["full_override_group_count"],
        "positive_candidate_count": summary["positive_candidate_count"],
        "cases": {
            key: {
                "candidate_keep_rate": value["candidate_keep_rate"],
                "chosen_group_recall": value["chosen_group_recall"],
                "chosen_group_missed_count": value["chosen_group_missed_count"],
                "positive_candidate_recall": value["positive_candidate_recall"],
            }
            for key, value in summary["cases"].items()
        },
    }


def case_key(top_k: int, threshold: float) -> str:
    return f"top{top_k}_thr{threshold:g}"


def parse_float_list(value: str) -> list[float]:
    return [float(item) for item in value.split(",") if item.strip()]


def parse_int_list(value: str) -> list[int]:
    return [int(item) for item in value.split(",") if item.strip()]


if __name__ == "__main__":
    main()
