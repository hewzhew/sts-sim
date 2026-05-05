#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import time
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from combat_rl_common import iter_jsonl, write_json, write_jsonl
from oracle_label_combat_rows import (
    audit_frame_group,
    canonical_outcome,
    choose_label_strength,
    equivalent_best_moves,
    make_stage_config,
    outcome_rank,
    report_to_oracle_candidates,
    resolve_audit_invocation,
    score_candidate,
)
from run_provenance import current_repo_provenance, provenance_for_source


REPO_ROOT = Path(__file__).resolve().parents[2]
OUTCOME_EPSILON = 250
STRONG_SCORE_GAP = 500


def default_sidecar(path: Path, suffix: str) -> Path:
    return path.with_name(f"{path.stem}{suffix}")


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    return [row for _, row in iter_jsonl(path)]


def safe_int(value: Any, default: int = 0) -> int:
    try:
        if value is None:
            return default
        return int(value)
    except (TypeError, ValueError):
        return default


def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        if value is None:
            return default
        return float(value)
    except (TypeError, ValueError):
        return default


def row_frame(row: dict[str, Any]) -> int:
    return safe_int(row.get("frame_id") or row.get("frame_count"))


def row_run_id(row: dict[str, Any]) -> str:
    return str(row.get("run_id") or "unknown")


def row_source_path(row: dict[str, Any]) -> str:
    path = row.get("source_path")
    if path:
        return str(path)
    raise ValueError(f"triage row has no source_path: {row.get('sample_id')}")


def audit_key(row: dict[str, Any]) -> str:
    return f"{row_run_id(row)}::{row_frame(row)}"


def candidate_with_rank(candidates: list[dict[str, Any]]) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    best = candidates[0] if candidates else None
    best_score = score_candidate(best) if best else None
    for rank, candidate in enumerate(candidates, start=1):
        enriched = dict(candidate)
        enriched["rank"] = rank
        enriched["score_gap_vs_best"] = (
            int(best.get("score") or 0) - int(candidate.get("score") or 0)
            if best is not None
            else None
        )
        enriched["is_exact_best_equivalent"] = bool(
            best_score is not None and score_candidate(candidate) == best_score
        )
        rows.append(enriched)
    return rows


def find_candidate(candidates: list[dict[str, Any]], move_label: str | None) -> dict[str, Any] | None:
    if not move_label:
        return None
    for candidate in candidates:
        if str(candidate.get("move_label") or "") == move_label:
            return candidate
    return None


def recommendation_for(
    *,
    bot_candidate: dict[str, Any] | None,
    best_candidate: dict[str, Any] | None,
    equivalent_best_moves: list[str],
    oracle_margin: int | None,
    label_strength: str,
) -> tuple[str, dict[str, Any], list[str]]:
    facts: dict[str, Any] = {
        "bot_in_best_bucket": False,
        "bot_within_epsilon": False,
        "clearly_better_candidate": False,
        "terminal_or_survival_bad": False,
        "regression_case": False,
        "safety_rule_candidate": False,
        "macro_provenance_candidate": False,
        "weak_training_candidate": False,
    }
    reasons: list[str] = []
    if best_candidate is None:
        reasons.append("no_oracle_best")
        return "reject", facts, reasons
    if bot_candidate is None:
        reasons.append("bot_candidate_not_found")
        return "reject", facts, reasons

    bot_move = str(bot_candidate.get("move_label") or "")
    best_move = str(best_candidate.get("move_label") or "")
    bot_outcome = canonical_outcome(bot_candidate.get("outcome"))
    best_outcome = canonical_outcome(best_candidate.get("outcome"))
    bot_rank = safe_int(bot_candidate.get("rank"), 9999)
    score_gap = safe_int(bot_candidate.get("score_gap_vs_best"))
    both_die = bot_outcome == "dies" and best_outcome == "dies"
    facts["bot_in_best_bucket"] = bot_move in set(equivalent_best_moves)
    facts["bot_within_epsilon"] = (
        outcome_rank(bot_outcome) == outcome_rank(best_outcome)
        and score_gap <= OUTCOME_EPSILON
    )
    facts["terminal_or_survival_bad"] = (
        bot_outcome == "dies" and best_outcome in {"survives", "lethal_win"}
    )
    facts["clearly_better_candidate"] = (
        not facts["bot_in_best_bucket"]
        and not both_die
        and (
            facts["terminal_or_survival_bad"]
            or outcome_rank(best_outcome) > outcome_rank(bot_outcome)
            or score_gap >= STRONG_SCORE_GAP
        )
    )
    facts["safety_rule_candidate"] = bool(facts["terminal_or_survival_bad"])
    facts["regression_case"] = bool(facts["clearly_better_candidate"])
    facts["macro_provenance_candidate"] = (
        not facts["bot_in_best_bucket"] and both_die and score_gap >= STRONG_SCORE_GAP
    )
    facts["weak_training_candidate"] = (
        not facts["regression_case"]
        and not facts["macro_provenance_candidate"]
        and not facts["bot_in_best_bucket"]
        and not both_die
        and score_gap > OUTCOME_EPSILON
    )

    if facts["bot_in_best_bucket"]:
        reasons.append("bot_in_best_bucket")
        if len(equivalent_best_moves) > 1 or (oracle_margin is not None and oracle_margin <= OUTCOME_EPSILON):
            reasons.append("best_bucket_or_gap_tie")
            return "calibration_or_tie", facts, reasons
        return "reject_bot_already_best", facts, reasons
    if facts["safety_rule_candidate"]:
        reasons.append("bot_dies_best_survives")
        return "safety_rule_candidate", facts, reasons
    if facts["regression_case"]:
        reasons.append("clearly_better_candidate")
        if label_strength == "oracle_strong":
            reasons.append("oracle_strong")
        return "regression_case", facts, reasons
    if facts["macro_provenance_candidate"]:
        reasons.append("all_candidates_die")
        reasons.append("best_only_reduces_loss")
        return "macro_provenance_candidate", facts, reasons
    if facts["weak_training_candidate"]:
        reasons.append("score_gap_preference")
        return "weak_training_candidate", facts, reasons
    if bot_rank <= 2:
        reasons.append("bot_rank_near_top")
    if score_gap <= OUTCOME_EPSILON:
        reasons.append("small_score_gap")
    if best_move:
        reasons.append("audit_only")
    return "reject_small_or_unclear_gap", facts, reasons


def make_audit_row(
    *,
    triage_row: dict[str, Any],
    report: dict[str, Any] | None,
    audit_error: str | None,
    config_name: str,
    invocation_mode: str,
    primary_recommendation: str | None = None,
    refined_from_fast: bool = False,
) -> dict[str, Any]:
    features = triage_row.get("features") or {}
    base = {
        "state_id": triage_row.get("sample_id"),
        "run_id": row_run_id(triage_row),
        "frame_id": row_frame(triage_row),
        "response_id": triage_row.get("response_id"),
        "source_path": triage_row.get("source_path"),
        "encounter_signature": triage_row.get("encounter_signature") or [],
        "encounter_key": triage_row.get("encounter_key"),
        "triage_primary_bucket": triage_row.get("primary_bucket"),
        "triage_tags": triage_row.get("triage_tags") or [],
        "triage_oracle_priority": triage_row.get("oracle_priority"),
        "triage_score": triage_row.get("oracle_selection_score"),
        "triage_features": {
            "hp": features.get("current_hp"),
            "max_hp": features.get("max_hp"),
            "regime": features.get("regime"),
            "legal_moves": features.get("legal_moves"),
            "reduced_legal_moves": features.get("reduced_legal_moves"),
            "chosen_action": features.get("chosen_action"),
            "chosen_label": features.get("chosen_label"),
            "exact_best_label": features.get("exact_best_label"),
            "frontier_survival": features.get("frontier_survival"),
            "exact_survival": features.get("exact_survival"),
            "exact_dominance": features.get("exact_dominance"),
        },
        "judge_protocol": {
            "tool": "combat_decision_audit audit-frame-batch",
            "mode": config_name,
            "audit_invocation_mode": invocation_mode,
            "refined_from_fast": refined_from_fast,
            "primary_recommendation": primary_recommendation,
        },
        "audit_error": audit_error,
        "current_repo_provenance": triage_row.get("current_repo_provenance"),
        "run_provenance": triage_row.get("run_provenance"),
        "provenance_freshness": triage_row.get("provenance_freshness"),
        "recommendation_scope": (triage_row.get("provenance_freshness") or {}).get(
            "evidence_scope", "unknown"
        ),
    }
    if report is None:
        return {
            **base,
            "bot_action": None,
            "legal_candidates": [],
            "candidate_outcomes": [],
            "best_candidate": None,
            "bot_candidate": None,
            "bot_rank": None,
            "top2_gap": None,
            "oracle_equivalent_best_moves": [],
            "oracle_best_bucket_size": 0,
            "recommendation": "reject",
            "recommendation_reasons": ["audit_error_or_missing_report"],
            "bot_in_best_bucket": False,
            "bot_within_epsilon": False,
            "clearly_better_candidate": False,
            "terminal_or_survival_bad": False,
            "regression_case": False,
            "safety_rule_candidate": False,
            "weak_training_candidate": False,
            "macro_provenance_candidate": False,
        }

    candidates = candidate_with_rank(report_to_oracle_candidates(report))
    best_candidate = candidates[0] if candidates else None
    equivalent_moves, oracle_margin, oracle_outcome_bucket = equivalent_best_moves(candidates)
    bot_action = str(report.get("chosen_first_move") or "")
    bot_candidate = find_candidate(candidates, bot_action)
    if bot_candidate is None and report.get("chosen_trajectory"):
        chosen = report.get("chosen_trajectory") or {}
        bot_candidate = {
            "move_label": bot_action,
            "outcome": canonical_outcome(chosen.get("outcome")),
            "score": safe_int(chosen.get("score")),
            "final_player_hp": safe_int(chosen.get("final_player_hp")),
            "final_player_block": safe_int(chosen.get("final_player_block")),
            "final_incoming": safe_int(chosen.get("final_incoming")),
            "final_monster_hp": [safe_int(value) for value in chosen.get("final_monster_hp") or []],
            "tags": [str(tag) for tag in chosen.get("tags") or []],
            "rank": None,
            "score_gap_vs_best": (
                safe_int(best_candidate.get("score")) - safe_int(chosen.get("score"))
                if best_candidate is not None
                else None
            ),
            "is_exact_best_equivalent": False,
        }
    label_strength = choose_label_strength(
        baseline_move=bot_action,
        chosen_trajectory=report.get("chosen_trajectory"),
        oracle_best=best_candidate,
        oracle_equivalent_best_moves=equivalent_moves,
        oracle_margin=oracle_margin,
    )
    recommendation, facts, reasons = recommendation_for(
        bot_candidate=bot_candidate,
        best_candidate=best_candidate,
        equivalent_best_moves=equivalent_moves,
        oracle_margin=oracle_margin,
        label_strength=label_strength,
    )
    bot_rank = bot_candidate.get("rank") if bot_candidate else None
    return {
        **base,
        "bot_action": bot_action,
        "legal_candidates": report.get("legal_first_moves") or [],
        "candidate_outcomes": candidates,
        "best_candidate": best_candidate,
        "bot_candidate": bot_candidate,
        "bot_rank": bot_rank,
        "top2_gap": oracle_margin,
        "oracle_outcome_bucket": oracle_outcome_bucket,
        "oracle_equivalent_best_moves": equivalent_moves,
        "oracle_best_bucket_size": len(equivalent_moves),
        "label_strength": label_strength,
        "recommendation": recommendation,
        "recommendation_reasons": reasons,
        **facts,
    }


def summarize(rows: list[dict[str, Any]], triage_count: int, elapsed_seconds: float, profile_batches: list[dict[str, Any]]) -> dict[str, Any]:
    ok_rows = [row for row in rows if not row.get("audit_error")]
    stale_rows = [
        row
        for row in rows
        if not bool((row.get("provenance_freshness") or {}).get("fresh_for_current_head"))
    ]
    run_provenance = {}
    for row in rows:
        run_id = str(row.get("run_id") or "unknown")
        run_provenance.setdefault(
            run_id,
            {
                "run": row.get("run_provenance"),
                "freshness": row.get("provenance_freshness"),
            },
        )
    return {
        "triage_rows": triage_count,
        "current_repo_provenance": rows[0].get("current_repo_provenance") if rows else {},
        "current_policy_conclusion_allowed": len(stale_rows) == 0,
        "stale_row_count": len(stale_rows),
        "stale_row_rate": len(stale_rows) / len(rows) if rows else 0.0,
        "run_provenance": run_provenance,
        "evidence_scope_counts": dict(
            Counter(str((row.get("provenance_freshness") or {}).get("evidence_scope")) for row in rows).most_common()
        ),
        "stale_reason_counts": dict(
            Counter(
                reason
                for row in stale_rows
                for reason in ((row.get("provenance_freshness") or {}).get("stale_reasons") or [])
            ).most_common()
        ),
        "audited_rows": len(rows),
        "ok_rows": len(ok_rows),
        "audit_error_rows": len(rows) - len(ok_rows),
        "recommendation_counts": dict(Counter(str(row.get("recommendation")) for row in rows).most_common()),
        "label_strength_counts": dict(Counter(str(row.get("label_strength") or "none") for row in rows).most_common()),
        "bot_rank_counts": dict(Counter(str(row.get("bot_rank")) for row in ok_rows).most_common()),
        "bot_in_best_bucket": sum(1 for row in ok_rows if row.get("bot_in_best_bucket")),
        "bot_within_epsilon": sum(1 for row in ok_rows if row.get("bot_within_epsilon")),
        "clearly_better_candidate": sum(1 for row in ok_rows if row.get("clearly_better_candidate")),
        "terminal_or_survival_bad": sum(1 for row in ok_rows if row.get("terminal_or_survival_bad")),
        "regression_cases": sum(1 for row in ok_rows if row.get("regression_case")),
        "safety_rule_candidates": sum(1 for row in ok_rows if row.get("safety_rule_candidate")),
        "weak_training_candidates": sum(1 for row in ok_rows if row.get("weak_training_candidate")),
        "macro_provenance_candidates": sum(1 for row in ok_rows if row.get("macro_provenance_candidate")),
        "run_id_counts": dict(Counter(str(row.get("run_id")) for row in rows).most_common()),
        "encounter_counts": dict(
            Counter(",".join(row.get("encounter_signature") or []) for row in rows).most_common()
        ),
        "triage_primary_bucket_counts": dict(
            Counter(str(row.get("triage_primary_bucket")) for row in rows).most_common()
        ),
        "elapsed_seconds": round(elapsed_seconds, 3),
        "batch_count": len(profile_batches),
        "refined_rows": sum(1 for row in rows if (row.get("judge_protocol") or {}).get("refined_from_fast")),
        "batch_profile": profile_batches,
        "notes": [
            "This is an audit consumer, not a training dataset builder.",
            "Recommendations identify locally fixable live-run decisions before any model training.",
            "A regression case means the audited best candidate clearly beats the bot action under the configured decision-audit protocol.",
            "A safety rule candidate means the bot action dies while an audited alternative survives or wins.",
        ],
    }


def write_review(path: Path, summary: dict[str, Any], rows: list[dict[str, Any]]) -> None:
    lines: list[str] = []
    lines.append("# Oracle Needed State Audit")
    lines.append("")
    if not summary.get("current_policy_conclusion_allowed", False):
        lines.append("**STALE: this audit is historical replay evidence, not current-policy evidence.**")
        lines.append("")
        lines.append(f"- stale_row_count: {summary.get('stale_row_count', 0)}")
        lines.append(f"- evidence_scope_counts: {json.dumps(summary.get('evidence_scope_counts') or {}, ensure_ascii=False)}")
        lines.append(f"- stale_reason_counts: {json.dumps(summary.get('stale_reason_counts') or {}, ensure_ascii=False)}")
        lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append("| metric | value |")
    lines.append("|---|---:|")
    for key in [
        "triage_rows",
        "audited_rows",
        "ok_rows",
        "clearly_better_candidate",
        "terminal_or_survival_bad",
        "regression_cases",
        "safety_rule_candidates",
        "weak_training_candidates",
        "macro_provenance_candidates",
    ]:
        lines.append(f"| {key} | {summary.get(key, 0)} |")
    lines.append("")
    lines.append("## Recommendations")
    lines.append("")
    lines.append("| recommendation | count |")
    lines.append("|---|---:|")
    for key, value in (summary.get("recommendation_counts") or {}).items():
        lines.append(f"| {key} | {value} |")
    lines.append("")
    lines.append("## Cases")
    lines.append("")
    lines.append("| rec | run | frame | encounter | hp | regime | bot rank | bot action | bot outcome | best action | best outcome | gap | reasons |")
    lines.append("|---|---|---:|---|---:|---|---:|---|---|---|---|---:|---|")
    priority = {
        "safety_rule_candidate": 0,
        "regression_case": 1,
        "weak_training_candidate": 2,
        "macro_provenance_candidate": 3,
        "calibration_or_tie": 4,
        "reject_small_or_unclear_gap": 5,
        "reject_bot_already_best": 6,
        "reject": 7,
    }
    rows_sorted = sorted(
        rows,
        key=lambda row: (
            priority.get(str(row.get("recommendation")), 99),
            str(row.get("run_id")),
            safe_int(row.get("frame_id")),
        ),
    )
    for row in rows_sorted[:80]:
        features = row.get("triage_features") or {}
        bot = row.get("bot_candidate") or {}
        best = row.get("best_candidate") or {}
        reasons = ",".join(row.get("recommendation_reasons") or [])
        encounter = ",".join(row.get("encounter_signature") or [])
        lines.append(
            f"| {row.get('recommendation')} | {row.get('run_id')} | {row.get('frame_id')} | "
            f"{encounter} | {features.get('hp')} | {features.get('regime')} | "
            f"{row.get('bot_rank')} | {row.get('bot_action')} | {bot.get('outcome')} | "
            f"{best.get('move_label')} | {best.get('outcome')} | {row.get('top2_gap')} | {reasons} |"
        )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Audit triaged oracle-needed live states without producing training tensors."
    )
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--out", default=None, type=Path)
    parser.add_argument("--summary-out", default=None, type=Path)
    parser.add_argument("--review-out", default=None, type=Path)
    parser.add_argument("--regression-out", default=None, type=Path)
    parser.add_argument("--safety-out", default=None, type=Path)
    parser.add_argument("--cache-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "oracle_needed_audit_cache", type=Path)
    parser.add_argument("--audit-binary", default=None)
    parser.add_argument("--mode", choices=["fast", "slow"], default="fast")
    parser.add_argument("--decision-depth", default=None, type=int)
    parser.add_argument("--top-k", default=None, type=int)
    parser.add_argument("--branch-cap", default=None, type=int)
    parser.add_argument("--batch-size", default=None, type=int)
    parser.add_argument("--max-seconds-per-frame", default=None, type=float)
    parser.add_argument("--max-seconds-per-batch", default=None, type=float)
    parser.add_argument("--limit", default=0, type=int)
    parser.add_argument("--quiet", action="store_true")
    parser.add_argument(
        "--require-current",
        action="store_true",
        help="Fail instead of auditing when any input row is not fresh for the current HEAD.",
    )
    parser.add_argument(
        "--no-refine-actionable",
        action="store_true",
        help="Do not rerun fast actionable cases with the slow audit budget.",
    )
    args = parser.parse_args()

    triage_rows = load_jsonl(args.input)
    if args.limit > 0:
        triage_rows = triage_rows[: args.limit]
    current_provenance = current_repo_provenance()
    provenance_cache: dict[str, dict[str, Any]] = {}
    for row in triage_rows:
        if row.get("provenance_freshness") is not None:
            continue
        source_path = row_source_path(row)
        provenance_cache.setdefault(source_path, provenance_for_source(source_path, current_provenance))
        provenance = provenance_cache[source_path]
        row["current_repo_provenance"] = provenance["current"]
        row["run_provenance"] = provenance["run"]
        row["provenance_freshness"] = provenance["freshness"]
    stale_inputs = [
        row
        for row in triage_rows
        if not bool((row.get("provenance_freshness") or {}).get("fresh_for_current_head"))
    ]
    if args.require_current and stale_inputs:
        stale_reasons = Counter(
            reason
            for row in stale_inputs
            for reason in ((row.get("provenance_freshness") or {}).get("stale_reasons") or [])
        )
        raise SystemExit(
            "input contains stale rows; rerun live_comm on current HEAD or omit --require-current "
            f"for historical replay audit only. stale_reasons={dict(stale_reasons)}"
        )
    invocation = resolve_audit_invocation(args.audit_binary)
    config = make_stage_config(
        name=args.mode,
        decision_depth=args.decision_depth,
        top_k=args.top_k,
        branch_cap=args.branch_cap,
        max_seconds_per_frame=args.max_seconds_per_frame,
        max_seconds_per_batch=args.max_seconds_per_batch,
        batch_size=args.batch_size,
        quiet=args.quiet,
    )

    started = time.perf_counter()
    grouped: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    for row in triage_rows:
        grouped[(row_run_id(row), row_source_path(row))].append(row)

    profile_batches: list[dict[str, Any]] = []
    report_by_key: dict[str, tuple[dict[str, Any] | None, str | None]] = {}
    for (run_id, raw_path), rows in grouped.items():
        frames = sorted({row_frame(row) for row in rows})
        frame_results, _stats = audit_frame_group(
            run_id=run_id,
            raw_path=raw_path,
            frames=frames,
            config=config,
            invocation=invocation,
            stage="oracle_needed",
            profile_batches=profile_batches,
        )
        for frame in frames:
            result = frame_results.get(frame) or {}
            status = str(result.get("status") or "missing")
            report = result.get("report") if status == "ok" else None
            error = None if status == "ok" else str(result.get("error") or status)
            report_by_key[f"{run_id}::{frame}"] = (report, error)

    output_rows: list[dict[str, Any]] = []
    rows_by_key = {audit_key(row): row for row in triage_rows}
    for row in triage_rows:
        report, error = report_by_key.get(audit_key(row), (None, "audit_result_missing"))
        output_rows.append(
            make_audit_row(
                triage_row=row,
                report=report,
                audit_error=error,
                config_name=config.name,
                invocation_mode=invocation.mode,
            )
        )

    if config.name == "fast" and not args.no_refine_actionable:
        actionable = {
            "safety_rule_candidate",
            "regression_case",
            "weak_training_candidate",
            "macro_provenance_candidate",
        }
        primary_by_key = {f"{row.get('run_id')}::{row.get('frame_id')}": row for row in output_rows}
        refine_triage_rows = [
            rows_by_key[key]
            for key, audit_row in primary_by_key.items()
            if audit_row.get("recommendation") in actionable and key in rows_by_key
        ]
        if refine_triage_rows:
            slow_config = make_stage_config(
                name="slow",
                decision_depth=None,
                top_k=None,
                branch_cap=None,
                max_seconds_per_frame=None,
                max_seconds_per_batch=None,
                batch_size=None,
                quiet=args.quiet,
            )
            refine_grouped: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
            for row in refine_triage_rows:
                refine_grouped[(row_run_id(row), row_source_path(row))].append(row)
            refined_reports: dict[str, tuple[dict[str, Any] | None, str | None]] = {}
            for (run_id, raw_path), rows in refine_grouped.items():
                frames = sorted({row_frame(row) for row in rows})
                frame_results, _stats = audit_frame_group(
                    run_id=run_id,
                    raw_path=raw_path,
                    frames=frames,
                    config=slow_config,
                    invocation=invocation,
                    stage="oracle_needed_refine",
                    profile_batches=profile_batches,
                )
                for frame in frames:
                    result = frame_results.get(frame) or {}
                    status = str(result.get("status") or "missing")
                    report = result.get("report") if status == "ok" else None
                    error = None if status == "ok" else str(result.get("error") or status)
                    refined_reports[f"{run_id}::{frame}"] = (report, error)
            refined_output = []
            for audit_row in output_rows:
                key = f"{audit_row.get('run_id')}::{audit_row.get('frame_id')}"
                if key not in refined_reports:
                    refined_output.append(audit_row)
                    continue
                report, error = refined_reports[key]
                refined_output.append(
                    make_audit_row(
                        triage_row=rows_by_key[key],
                        report=report,
                        audit_error=error,
                        config_name=slow_config.name,
                        invocation_mode=invocation.mode,
                        primary_recommendation=str(audit_row.get("recommendation")),
                        refined_from_fast=True,
                    )
                )
            output_rows = refined_output

    elapsed = time.perf_counter() - started
    summary = summarize(output_rows, len(triage_rows), elapsed, profile_batches)
    summary["input"] = str(args.input)
    summary["mode"] = config.name
    summary["refine_actionable"] = bool(config.name == "fast" and not args.no_refine_actionable)
    summary["decision_depth"] = config.decision_depth
    summary["top_k"] = config.top_k
    summary["branch_cap"] = config.branch_cap
    summary["audit_invocation_mode"] = invocation.mode

    out = args.out or default_sidecar(args.input, ".audit.jsonl")
    summary_out = args.summary_out or default_sidecar(args.input, ".audit.summary.json")
    review_out = args.review_out or default_sidecar(args.input, ".audit.md")
    regression_out = args.regression_out or default_sidecar(args.input, ".regression_cases.jsonl")
    safety_out = args.safety_out or default_sidecar(args.input, ".safety_rule_cases.jsonl")

    write_jsonl(out, output_rows)
    write_json(summary_out, summary)
    write_review(review_out, summary, output_rows)
    write_jsonl(regression_out, [row for row in output_rows if row.get("regression_case")])
    write_jsonl(safety_out, [row for row in output_rows if row.get("safety_rule_candidate")])
    print(json.dumps(summary, indent=2, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
