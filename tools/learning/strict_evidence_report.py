#!/usr/bin/env python3
"""Build compact reports for strict-evidence closed-loop A/B summaries."""

from __future__ import annotations

import argparse
import json
import math
from collections import Counter
from pathlib import Path
from typing import Any


def safe_int(value: Any, default: int = 0) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        number = float(value)
    except (TypeError, ValueError):
        return default
    return number if math.isfinite(number) else default


def load_summary(path: Path) -> dict[str, Any]:
    summary = json.loads(path.read_text(encoding="utf-8"))
    summary["_source_path"] = str(path)
    return summary


def _sum_episode_field(summary: dict[str, Any], policy_kind: str, field: str) -> int:
    total = 0
    for result in summary.get("episode_results") or []:
        if result.get("policy_kind") == policy_kind:
            total += safe_int(result.get(field))
    return total


def _sum_nested_counter(summary: dict[str, Any], field: str) -> dict[str, int]:
    counter: Counter[str] = Counter()
    for result in summary.get("episode_results") or []:
        if result.get("policy_kind") == "strict_evidence_policy_v0":
            counter.update(result.get(field) or {})
    return dict(counter)


def summarize_summary(summary: dict[str, Any], *, label: str | None = None) -> dict[str, Any]:
    config = summary.get("config") or {}
    paired = summary.get("paired_summary") or {}
    sampling = summary.get("candidate_sampling_summary") or {}
    cache = summary.get("branch_evidence_cache_summary") or {}
    if not sampling:
        legal = _sum_episode_field(summary, "strict_evidence_policy_v0", "legal_candidate_count_total")
        included = _sum_episode_field(
            summary, "strict_evidence_policy_v0", "sampling_included_candidate_count"
        )
        sampling = {
            "legal_candidate_count_total": legal,
            "sampling_included_candidate_count": included,
            "sampling_excluded_candidate_count": _sum_episode_field(
                summary, "strict_evidence_policy_v0", "sampling_excluded_candidate_count"
            ),
            "sampling_missing_behavior_action_count": _sum_episode_field(
                summary, "strict_evidence_policy_v0", "sampling_missing_behavior_action_count"
            ),
            "sampling_excluded_by_reason_counts": _sum_nested_counter(
                summary, "sampling_excluded_by_reason_counts"
            ),
            "traced_over_legal_candidate_ratio": (included / legal if legal else 0.0),
        }
    strict_combat_decisions = _sum_episode_field(
        summary, "strict_evidence_policy_v0", "combat_decisions"
    )
    trace_count = _sum_episode_field(summary, "strict_evidence_policy_v0", "branch_trace_count")
    comparison_count = _sum_episode_field(summary, "strict_evidence_policy_v0", "comparison_count")
    validation_issue_count = _sum_episode_field(
        summary, "strict_evidence_policy_v0", "validation_issue_count"
    )
    censored_trace_count = _sum_episode_field(
        summary, "strict_evidence_policy_v0", "censored_trace_count"
    )
    truncated_trace_count = _sum_episode_field(
        summary, "strict_evidence_policy_v0", "truncated_trace_count"
    )
    total_override_count = safe_int(paired.get("total_override_count"))
    return {
        "label": label
        or summary.get("matrix_config_id")
        or summary.get("label")
        or Path(str(summary.get("_source_path") or "summary")).stem,
        "source_path": summary.get("_source_path"),
        "preset": config.get("preset") or "none",
        "seed_count": safe_int(summary.get("seed_count") or paired.get("paired_seed_count")),
        "paired_seed_count": safe_int(paired.get("paired_seed_count")),
        "max_steps": safe_int(config.get("max_steps")),
        "horizon": f"{config.get('horizon_mode') or 'unknown'}:{safe_int(config.get('horizon_decisions'))}",
        "max_candidates": safe_int(config.get("max_candidates")),
        "min_hp_margin": safe_int(config.get("min_hp_margin")),
        "hp_margin_only": bool(config.get("hp_margin_only")),
        "allow_progress_flip": bool(config.get("allow_progress_flip")),
        "strict_combat_decisions": strict_combat_decisions,
        "branch_trace_count": trace_count,
        "comparison_count": comparison_count,
        "validation_issue_count": validation_issue_count,
        "censored_trace_count": censored_trace_count,
        "truncated_trace_count": truncated_trace_count,
        "total_override_count": total_override_count,
        "override_seed_count": safe_int(paired.get("override_seed_count")),
        "override_per_100_combat_decisions": (
            100.0 * total_override_count / strict_combat_decisions
            if strict_combat_decisions
            else 0.0
        ),
        "sum_floor_delta": safe_float(paired.get("sum_floor_delta")),
        "sum_hp_delta": safe_float(paired.get("sum_hp_delta")),
        "sum_combat_win_delta": safe_float(paired.get("sum_combat_win_delta")),
        "sum_reward_delta": safe_float(paired.get("sum_reward_delta")),
        "bad_outcome_seed_count": safe_int(paired.get("bad_outcome_seed_count")),
        "improved_outcome_seed_count": safe_int(paired.get("improved_outcome_seed_count")),
        "death_regression_count": safe_int(paired.get("death_regression_count")),
        "result_change_count": safe_int(paired.get("result_change_count")),
        "sampling_included_candidate_count": safe_int(
            sampling.get("sampling_included_candidate_count")
        ),
        "sampling_excluded_candidate_count": safe_int(
            sampling.get("sampling_excluded_candidate_count")
        ),
        "sampling_missing_behavior_action_count": safe_int(
            sampling.get("sampling_missing_behavior_action_count")
        ),
        "traced_over_legal_candidate_ratio": safe_float(
            sampling.get("traced_over_legal_candidate_ratio")
        ),
        "branch_cache_hit_count": safe_int(cache.get("hit_count")),
        "branch_cache_miss_count": safe_int(cache.get("miss_count")),
        "branch_cache_hit_rate": safe_float(cache.get("hit_rate")),
        "sampling_excluded_by_reason_counts": sampling.get("sampling_excluded_by_reason_counts")
        or {},
        "final_result_counts": summary.get("final_result_counts") or {},
        "override_reason_counts": summary.get("override_reason_counts") or {},
        "behavior_reason_counts": summary.get("behavior_reason_counts") or {},
    }


def markdown_table(rows: list[dict[str, Any]]) -> str:
    columns = [
        ("label", "config"),
        ("seed_count", "seeds"),
        ("max_steps", "steps"),
        ("min_hp_margin", "hp_margin"),
        ("hp_margin_only", "hp_only"),
        ("total_override_count", "overrides"),
        ("override_per_100_combat_decisions", "ovr/100cmb"),
        ("sum_floor_delta", "floorΔ"),
        ("sum_hp_delta", "hpΔ"),
        ("sum_combat_win_delta", "winΔ"),
        ("sum_reward_delta", "rewardΔ"),
        ("bad_outcome_seed_count", "bad"),
        ("death_regression_count", "death_reg"),
        ("validation_issue_count", "issues"),
        ("truncated_trace_count", "trunc"),
        ("censored_trace_count", "censored"),
        ("traced_over_legal_candidate_ratio", "cand_cov"),
        ("branch_cache_hit_rate", "cache_hit"),
    ]
    lines = [
        "| " + " | ".join(header for _, header in columns) + " |",
        "| " + " | ".join("---" for _ in columns) + " |",
    ]
    for row in rows:
        cells: list[str] = []
        for key, _ in columns:
            value = row.get(key)
            if isinstance(value, float):
                cells.append(f"{value:.3f}")
            else:
                cells.append(str(value))
        lines.append("| " + " | ".join(cells) + " |")
    return "\n".join(lines) + "\n"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("summaries", type=Path, nargs="+")
    parser.add_argument("--labels", nargs="*", help="Optional labels matching the summary paths.")
    parser.add_argument("--json-out", type=Path)
    parser.add_argument("--md-out", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    labels = args.labels or []
    if labels and len(labels) != len(args.summaries):
        raise SystemExit("--labels must match the number of summary paths")
    rows = [
        summarize_summary(load_summary(path), label=labels[index] if labels else None)
        for index, path in enumerate(args.summaries)
    ]
    report = {
        "schema_version": "strict_evidence_closed_loop_report_v1",
        "summary_count": len(rows),
        "rows": rows,
    }
    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(report, indent=2), encoding="utf-8")
    table = markdown_table(rows)
    if args.md_out:
        args.md_out.parent.mkdir(parents=True, exist_ok=True)
        args.md_out.write_text(table, encoding="utf-8")
    print(table, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
