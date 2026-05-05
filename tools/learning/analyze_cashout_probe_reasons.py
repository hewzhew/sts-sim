#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json


REPORT_VERSION = "cashout_probe_reason_analysis_v0"
DEFAULT_REPORT = (
    REPO_ROOT
    / "tools"
    / "artifacts"
    / "cashout_micro_probes"
    / "v0"
    / "cashout_micro_probe_runner_report.json"
)

STATIC_FALSE_POSITIVE_REASONS = {
    "opened_resource_window_without_clear_progress",
    "low_damage_conversion",
    "no_combat_win_edge",
    "survival_gain_without_progress",
    "control_wins_combat_candidate_does_not",
}
POLICY_UTILIZATION_REASONS = {
    "draw_without_observed_payoff_card",
    "resource_not_played_or_not_observed",
}
PRESSURE_GATE_REASONS = {
    "enemy_pressure_overwhelms_window",
    "resource_line_costs_more_hp",
    "unused_energy_after_window",
    "long_fight_no_close",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Analyze cashout micro-probe reason codes into repair queues. "
            "This is a diagnostic router; it does not modify cashout scores or train a model."
        )
    )
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--markdown-out", type=Path)
    parser.add_argument("--queue-out", type=Path)
    parser.add_argument("--top-n", type=int, default=30)
    return parser.parse_args()


def resolve(path: Path) -> Path:
    return path if path.is_absolute() else REPO_ROOT / path


def read_json(path: Path) -> dict[str, Any]:
    with resolve(path).open("r", encoding="utf-8") as handle:
        return json.load(handle)


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    real = resolve(path)
    real.parent.mkdir(parents=True, exist_ok=True)
    with real.open("w", encoding="utf-8", newline="\n") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")


def num(value: Any) -> float:
    try:
        return float(value or 0.0)
    except (TypeError, ValueError):
        return 0.0


def reason_counts_for_contexts(contexts: list[dict[str, Any]]) -> Counter[str]:
    counts: Counter[str] = Counter()
    for context in contexts:
        verdict = context.get("family_verdict") or {}
        for code in verdict.get("reason_codes") or []:
            counts[str(code)] += 1
    return counts


def family_counts_for_contexts(contexts: list[dict[str, Any]]) -> Counter[str]:
    counts: Counter[str] = Counter()
    for context in contexts:
        verdict = context.get("family_verdict") or {}
        counts[str(verdict.get("verdict") or "unknown")] += 1
    return counts


def context_compact(context: dict[str, Any]) -> dict[str, Any]:
    verdict = context.get("family_verdict") or {}
    diff = verdict.get("diff") or {}
    return {
        "context": context.get("context"),
        "family": context.get("family"),
        "verdict": verdict.get("verdict"),
        "candidate_rank": context.get("candidate_rank"),
        "control_rank": context.get("control_rank"),
        "diff": {
            "score": diff.get("score"),
            "floor": diff.get("floor"),
            "combat_wins": diff.get("combat_wins"),
            "end_hp": diff.get("end_hp"),
            "hp_loss": diff.get("hp_loss"),
            "monster_hp": diff.get("monster_hp"),
            "kills": diff.get("kills"),
        },
        "reason_codes": verdict.get("reason_codes") or [],
        "reason_details": verdict.get("reason_details") or {},
    }


def classify_reason_bucket(reason_counts: Counter[str], family_counts: Counter[str]) -> tuple[str, list[str]]:
    total_reasons = sum(reason_counts.values())
    if total_reasons == 0:
        return "no_reason_codes", ["no reason-coded probe evidence"]
    static_hits = sum(reason_counts.get(code, 0) for code in STATIC_FALSE_POSITIVE_REASONS)
    policy_hits = sum(reason_counts.get(code, 0) for code in POLICY_UTILIZATION_REASONS)
    pressure_hits = sum(reason_counts.get(code, 0) for code in PRESSURE_GATE_REASONS)
    no_payoff = family_counts.get("window_opened_without_payoff", 0)
    notes: list[str] = []
    if reason_counts.get("control_wins_combat_candidate_does_not", 0) > 0:
        notes.append("control branch sometimes wins a combat that the candidate branch misses")
    if reason_counts.get("draw_without_observed_payoff_card", 0) > 0:
        notes.append("draw/resource card was played without observed payoff cards")
    if reason_counts.get("enemy_pressure_overwhelms_window", 0) >= max(no_payoff, 1):
        notes.append("all no-payoff windows occurred under high enemy pressure")
    if reason_counts.get("survival_gain_without_progress", 0) > 0:
        notes.append("some lines preserve HP but fail to improve combat progress")

    if no_payoff > 0 and reason_counts.get("enemy_pressure_overwhelms_window", 0) >= no_payoff:
        return "pressure_gate_static_cashout", notes
    if no_payoff > 0 and pressure_hits >= static_hits and pressure_hits >= policy_hits:
        return "pressure_gate_static_cashout", notes
    if no_payoff > 0 and policy_hits > 0 and policy_hits >= static_hits / 2:
        return "policy_payoff_missing", notes
    if no_payoff > 0 and static_hits > 0:
        return "static_resource_window_false_positive", notes
    return "mixed_or_inconclusive", notes


def recommended_actions(bucket: str, reason_counts: Counter[str]) -> list[str]:
    if bucket == "pressure_gate_static_cashout":
        return [
            "add_pressure_gate_to_resource_window_cashout",
            "downweight Offering/resource-window candidates when enemy pressure is high and no immediate kill/block payoff is visible",
            "reject as positive training label until rollout shows combat-win or kill-timing edge",
        ]
    if bucket == "policy_payoff_missing":
        return [
            "route to payoff-affordance probe instead of static score tuning",
            "require observed playable payoff density before high draw/resource cashout",
            "keep policy-conditional label separate from card-potential label",
        ]
    if bucket == "static_resource_window_false_positive":
        return [
            "cap static resource-window score without monster-hp or combat-win evidence",
            "increase low_damage/no_combat_win penalties in cashout diagnostics",
            "prefer paired rollout verification before using as comparator data",
        ]
    if bucket == "no_reason_codes":
        return ["no repair action from reason analysis"]
    return [
        "inspect case manually or expand probe context",
        "do not use as hard label",
    ]


def confidence(reason_counts: Counter[str], context_count: int) -> str:
    if context_count <= 0:
        return "none"
    top = reason_counts.most_common(1)[0][1] if reason_counts else 0
    if top >= context_count and context_count >= 4:
        return "high"
    if top >= max(2, context_count // 2):
        return "medium"
    return "low"


def analyze_case(case: dict[str, Any]) -> dict[str, Any]:
    contexts = list(case.get("contexts") or [])
    reason_counts = reason_counts_for_contexts(contexts)
    family_counts = family_counts_for_contexts(contexts)
    bucket, notes = classify_reason_bucket(reason_counts, family_counts)
    candidate = case.get("candidate_under_test") or {}
    control = case.get("control_candidate") or {}
    return {
        "case_id": case.get("case_id"),
        "bucket": case.get("bucket"),
        "reason_bucket": bucket,
        "recommended_actions": recommended_actions(bucket, reason_counts),
        "confidence": confidence(reason_counts, len(contexts)),
        "candidate_under_test": {
            "card_id": candidate.get("card_id"),
            "action_key": candidate.get("action_key"),
            "cashout_score": candidate.get("cashout_score"),
            "dominant_cashout": candidate.get("dominant_cashout"),
        },
        "control_candidate": {
            "card_id": control.get("card_id"),
            "action_key": control.get("action_key"),
            "cashout_score": control.get("cashout_score"),
            "dominant_cashout": control.get("dominant_cashout"),
        },
        "family_counts": dict(sorted(family_counts.items())),
        "reason_counts": dict(sorted(reason_counts.items())),
        "notes": notes,
        "context_count": len(contexts),
        "source": case.get("source") or {},
        "contexts": [context_compact(context) for context in contexts],
        "contract": "repair routing only; not a label and not causal proof",
    }


def build_repair_queue(case_rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    rows = []
    for case in case_rows:
        if case["reason_bucket"] == "no_reason_codes":
            continue
        rows.append(
            {
                "queue_version": "cashout_reason_repair_queue_v0",
                "case_id": case.get("case_id"),
                "reason_bucket": case.get("reason_bucket"),
                "confidence": case.get("confidence"),
                "candidate_card": (case.get("candidate_under_test") or {}).get("card_id"),
                "control_card": (case.get("control_candidate") or {}).get("card_id"),
                "recommended_actions": case.get("recommended_actions") or [],
                "reason_counts": case.get("reason_counts") or {},
                "family_counts": case.get("family_counts") or {},
                "source": case.get("source") or {},
                "training_use": "reject_as_hard_label",
                "next_use": "cashout_static_patch_or_targeted_affordance_probe",
            }
        )
    return sorted(
        rows,
        key=lambda row: (
            str(row.get("reason_bucket")),
            str(row.get("candidate_card")),
            str(row.get("case_id")),
        ),
    )


def build_report(source_report: dict[str, Any]) -> dict[str, Any]:
    case_rows = [analyze_case(case) for case in source_report.get("cases") or []]
    repair_queue = build_repair_queue(case_rows)
    reason_counts = Counter()
    bucket_counts = Counter()
    card_bucket_counts = Counter()
    for row in case_rows:
        bucket_counts[str(row.get("reason_bucket") or "unknown")] += 1
        card = str((row.get("candidate_under_test") or {}).get("card_id") or "unknown")
        card_bucket_counts[f"{card}:{row.get('reason_bucket')}"] += 1
        reason_counts.update(row.get("reason_counts") or {})
    return {
        "report_version": REPORT_VERSION,
        "source_report_version": source_report.get("report_version"),
        "summary": {
            "case_count": len(case_rows),
            "repair_queue_count": len(repair_queue),
            "reason_bucket_counts": dict(sorted(bucket_counts.items())),
            "reason_counts": dict(sorted(reason_counts.items())),
            "card_reason_bucket_counts": dict(sorted(card_bucket_counts.items())),
            "contract": "reason-code routing for cashout repair; not a training label",
        },
        "case_reason_analysis": case_rows,
        "repair_queue": repair_queue,
        "limitations": [
            "Reason codes come from trace-prefix micro-probes, not canonical encounter distributions.",
            "The repair queue suggests cashout/affordance work; it does not prove card value.",
            "Policy-conditional failures must not be folded into card truth without a stronger continuation policy.",
        ],
    }


def write_markdown(path: Path, report: dict[str, Any], top_n: int) -> None:
    summary = report["summary"]
    lines = [
        "# Cashout Probe Reason Analysis V0",
        "",
        "This report converts micro-probe reason codes into cashout repair routing.",
        "It is diagnostic only; rows are not labels.",
        "",
        "## Summary",
        "",
        f"- source report: `{report.get('source_report_version')}`",
        f"- cases: `{summary['case_count']}`",
        f"- repair queue rows: `{summary['repair_queue_count']}`",
        f"- reason buckets: `{summary['reason_bucket_counts']}`",
        f"- reason counts: `{summary['reason_counts']}`",
        f"- card buckets: `{summary['card_reason_bucket_counts']}`",
        f"- contract: `{summary['contract']}`",
        "",
        "## Repair Queue",
        "",
        "| case | bucket | confidence | candidate | control | recommended actions | reasons |",
        "|---|---|---|---|---|---|---|",
    ]
    for row in report["repair_queue"][:top_n]:
        lines.append(
            "| {case} | {bucket} | {confidence} | {candidate} | {control} | `{actions}` | `{reasons}` |".format(
                case=row.get("case_id"),
                bucket=row.get("reason_bucket"),
                confidence=row.get("confidence"),
                candidate=row.get("candidate_card"),
                control=row.get("control_card"),
                actions=row.get("recommended_actions") or [],
                reasons=row.get("reason_counts") or {},
            )
        )
    lines.extend(["", "## Cases", ""])
    for case in report["case_reason_analysis"][:top_n]:
        lines.extend(
            [
                f"### {case.get('case_id')}",
                "",
                "- reason bucket: `{bucket}` confidence `{confidence}`".format(
                    bucket=case.get("reason_bucket"),
                    confidence=case.get("confidence"),
                ),
                "- candidate `{candidate}` vs control `{control}`".format(
                    candidate=(case.get("candidate_under_test") or {}).get("card_id"),
                    control=(case.get("control_candidate") or {}).get("card_id"),
                ),
                f"- family counts: `{case.get('family_counts')}`",
                f"- reason counts: `{case.get('reason_counts')}`",
                f"- actions: `{case.get('recommended_actions')}`",
                f"- notes: `{case.get('notes')}`",
                "",
            ]
        )
    real = resolve(path)
    real.parent.mkdir(parents=True, exist_ok=True)
    real.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    source = read_json(args.report)
    report = build_report(source)
    out_path = resolve(args.out) if args.out else resolve(args.report).with_name("cashout_probe_reason_analysis.json")
    markdown_path = (
        resolve(args.markdown_out)
        if args.markdown_out
        else out_path.with_suffix(".md")
    )
    queue_path = (
        resolve(args.queue_out)
        if args.queue_out
        else out_path.with_name("cashout_reason_repair_queue.jsonl")
    )
    write_json(out_path, report)
    write_markdown(markdown_path, report, int(args.top_n))
    write_jsonl(queue_path, report["repair_queue"])
    print(
        json.dumps(
            {
                "out": str(out_path),
                "markdown_out": str(markdown_path),
                "queue_out": str(queue_path),
                "summary": report["summary"],
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
