#!/usr/bin/env python3
"""Diagnose Pareto dominance hypotheses against rollout pairwise labels.

This is a diagnostic, not a trainer.  A Pareto relation means:

    "Under the current heuristic vector, B should not beat A."

Rollout pairwise labels are policy/horizon conditional evidence.  The script
cross-references rollout labels with candidate plan_delta vectors when those
vectors are available.  If vectors are missing, it reports that explicitly
instead of pretending the hypothesis was tested.
"""
from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from combat_reranker_common import iter_jsonl, write_json

REPO_ROOT = Path(__file__).resolve().parents[2]

DOMINANCE_DIMS: list[tuple[str, bool]] = [
    ("frontload_delta", True),
    ("block_delta", True),
    ("draw_delta", True),
    ("scaling_delta", True),
    ("aoe_delta", True),
    ("exhaust_delta", True),
    ("kill_window_delta", True),
    ("starter_basic_burden_delta", True),
    ("setup_cashout_risk_delta", False),
    ("duplicate_penalty", True),
]


def parse_args() -> argparse.Namespace:
    default_label_dir = (
        REPO_ROOT
        / "tools"
        / "artifacts"
        / "card_cashout_rollout_labels"
        / "current_trace_corpus_v0_10_50eps_plan_query_guard"
    )
    p = argparse.ArgumentParser(description="Diagnose Pareto feedback from rollout labels")
    p.add_argument(
        "--label-report",
        type=Path,
        default=default_label_dir / "cashout_rollout_label_report.json",
        help="Rollout label report JSON containing case-level label_status metadata.",
    )
    p.add_argument(
        "--pairwise-labels",
        type=Path,
        default=default_label_dir / "pairwise_labels.jsonl",
        help="Pairwise rollout labels JSONL.",
    )
    p.add_argument(
        "--cashout-report",
        type=Path,
        default=REPO_ROOT
        / "tools"
        / "artifacts"
        / "card_cashout_lab"
        / "current_trace_corpus_v0_10_50eps"
        / "cashout_report.json",
        help="Optional cashout report; used only if it contains candidate plan_delta fields.",
    )
    p.add_argument(
        "--trace-root",
        type=Path,
        action="append",
        default=[],
        help="Optional root containing full run trace episode_*.json files with action_mask.plan_delta.",
    )
    p.add_argument(
        "--trace-glob",
        type=str,
        action="append",
        default=[],
        help="Optional glob for trace files, relative to repo root unless absolute.",
    )
    p.add_argument(
        "--out",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "pareto_feedback" / "pareto_feedback_report.json",
    )
    p.add_argument("--top", type=int, default=20)
    return p.parse_args()


def resolve_path(path: Path | str | None) -> Path | None:
    if path is None:
        return None
    p = Path(path)
    if p.is_absolute():
        return p
    return REPO_ROOT / p


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    return [row for _, row in iter_jsonl(path)] if path.exists() else []


def dim_value(delta: dict[str, Any], dim: str, higher_better: bool) -> int:
    raw = int(delta.get(dim) or 0)
    return raw if higher_better else -raw


def is_vectorizable(delta: dict[str, Any] | None) -> bool:
    if not delta:
        return False
    return any(dim_value(delta, dim, True) != 0 for dim, _ in DOMINANCE_DIMS)


def pareto_dominates_with_dims(
    a: dict[str, Any],
    b: dict[str, Any],
    dims: list[tuple[str, bool]] = DOMINANCE_DIMS,
) -> bool:
    has_strict = False
    for dim, higher_better in dims:
        va = dim_value(a, dim, higher_better)
        vb = dim_value(b, dim, higher_better)
        if va < vb:
            return False
        if va > vb:
            has_strict = True
    return has_strict


def strict_advantage_dims(a: dict[str, Any], b: dict[str, Any]) -> list[str]:
    out = []
    for dim, higher_better in DOMINANCE_DIMS:
        if dim_value(a, dim, higher_better) > dim_value(b, dim, higher_better):
            out.append(dim)
    return out


def compact_delta(delta: dict[str, Any] | None) -> dict[str, int]:
    if not delta:
        return {}
    return {dim: int(delta.get(dim) or 0) for dim, _ in DOMINANCE_DIMS}


def candidate_card_from_outcome(outcome: dict[str, Any] | None) -> str:
    if not outcome:
        return "unknown"
    return str(outcome.get("card_id") or outcome.get("candidate_key") or "skip")


def case_key(seed: Any, step_index: Any) -> str:
    return f"{seed}:{step_index}"


class VectorIndex:
    def __init__(self) -> None:
        self.by_case_id: dict[str, dict[str, dict[str, Any]]] = {}
        self.by_seed_step: dict[str, dict[str, dict[str, Any]]] = {}
        self.sources = Counter()
        self.missing_trace_files: set[str] = set()
        self.trace_files_scanned = 0
        self.reward_steps_scanned = 0

    def add_candidate(
        self,
        *,
        case_id: str | None,
        seed: Any,
        step_index: Any,
        action_key: str | None,
        delta: dict[str, Any] | None,
        source: str,
        card_id: str | None = None,
    ) -> None:
        if not action_key or not delta:
            return
        record = {
            "plan_delta": delta,
            "source": source,
            "card_id": card_id,
        }
        if case_id:
            self.by_case_id.setdefault(str(case_id), {})[str(action_key)] = record
        if seed is not None and step_index is not None:
            self.by_seed_step.setdefault(case_key(seed, step_index), {})[str(action_key)] = record
        self.sources[source] += 1

    def vectors_for(self, row: dict[str, Any]) -> tuple[dict[str, dict[str, Any]] | None, str]:
        case_id = str(row.get("case_id") or "")
        if case_id and case_id in self.by_case_id:
            return self.by_case_id[case_id], "case_id"
        seed_step = case_key(row.get("seed"), row.get("step_index"))
        if seed_step in self.by_seed_step:
            return self.by_seed_step[seed_step], "seed_step"
        return None, "missing_vector_source"


def action_key(candidate: dict[str, Any]) -> str | None:
    return (
        candidate.get("action_key")
        or candidate.get("candidate_key")
        or candidate.get("key")
        or candidate.get("debug_key")
    )


def candidate_plan_delta(candidate: dict[str, Any]) -> dict[str, Any] | None:
    delta = candidate.get("plan_delta")
    if isinstance(delta, dict):
        return delta
    return None


def load_vectors_from_cashout_report(index: VectorIndex, path: Path | None) -> None:
    if not path or not path.exists():
        return
    report = read_json(path)
    for policy in report.get("policies") or []:
        for comparison in policy.get("comparisons") or []:
            seed = comparison.get("seed")
            step = comparison.get("step_index")
            case_id = comparison.get("case_id")
            for candidate in comparison.get("candidates") or []:
                index.add_candidate(
                    case_id=case_id,
                    seed=seed,
                    step_index=step,
                    action_key=action_key(candidate),
                    delta=candidate_plan_delta(candidate),
                    source="cashout_report",
                    card_id=candidate.get("card_id"),
                )


def load_vectors_from_label_report(index: VectorIndex, label_report: dict[str, Any]) -> None:
    for label in label_report.get("labels") or []:
        case_id = label.get("case_id")
        source_case = label.get("source_case") or {}
        seed = source_case.get("seed")
        step = source_case.get("step_index")
        for key in ("chosen", "best_by_cashout"):
            candidate = source_case.get(key) or {}
            index.add_candidate(
                case_id=case_id,
                seed=seed,
                step_index=step,
                action_key=action_key(candidate),
                delta=candidate_plan_delta(candidate),
                source="label_report_source_case",
                card_id=candidate.get("card_id"),
            )


def step_index_value(step: dict[str, Any], fallback: int) -> Any:
    for key in ("step_index", "step"):
        if key in step:
            return step.get(key)
    return fallback


def iter_trace_steps(trace: dict[str, Any]) -> list[dict[str, Any]]:
    steps = trace.get("steps")
    return steps if isinstance(steps, list) else []


def load_trace_file(index: VectorIndex, path: Path, case_by_trace_step: dict[str, str]) -> None:
    try:
        trace = read_json(path)
    except Exception:
        return
    index.trace_files_scanned += 1
    seed = (trace.get("summary") or {}).get("seed") or trace.get("seed")
    trace_key_prefix = str(path).lower()
    for i, step in enumerate(iter_trace_steps(trace)):
        if step.get("decision_type") != "reward_card_choice":
            continue
        candidates = step.get("action_mask") or []
        if not isinstance(candidates, list):
            continue
        step_idx = step_index_value(step, i)
        case_id = case_by_trace_step.get(f"{trace_key_prefix}:{step_idx}") or case_by_trace_step.get(
            case_key(seed, step_idx)
        )
        index.reward_steps_scanned += 1
        for candidate in candidates:
            index.add_candidate(
                case_id=case_id,
                seed=seed,
                step_index=step_idx,
                action_key=action_key(candidate),
                delta=candidate_plan_delta(candidate),
                source="trace",
                card_id=candidate.get("card_id"),
            )


def collect_trace_paths(args: argparse.Namespace, label_report: dict[str, Any]) -> list[Path]:
    paths: list[Path] = []
    seen: set[str] = set()

    def add(path: Path | None) -> None:
        if not path:
            return
        key = str(path.resolve() if path.exists() else path).lower()
        if key not in seen:
            seen.add(key)
            paths.append(path)

    for label in label_report.get("labels") or []:
        trace_file = (label.get("source_case") or {}).get("trace_file")
        p = resolve_path(trace_file)
        if p and p.exists():
            add(p)

    for root in args.trace_root or []:
        root_path = resolve_path(root)
        if root_path and root_path.exists():
            for path in root_path.rglob("episode_*.json"):
                add(path)

    for pattern in args.trace_glob or []:
        glob_root = Path(pattern)
        if not glob_root.is_absolute():
            glob_root = REPO_ROOT / glob_root
        for path in glob_root.parent.glob(glob_root.name):
            add(path)

    return paths


def build_case_metadata(label_report: dict[str, Any]) -> tuple[dict[str, dict[str, Any]], dict[str, str]]:
    meta: dict[str, dict[str, Any]] = {}
    trace_step_to_case: dict[str, str] = {}
    for label in label_report.get("labels") or []:
        case_id = str(label.get("case_id") or "")
        source_case = label.get("source_case") or {}
        if not case_id:
            continue
        meta[case_id] = {
            "label_status": label.get("label_status"),
            "label_substatus": label.get("label_substatus"),
            "strong_training_signal": bool(label.get("strong_training_signal")),
            "source_policy": label.get("source_policy"),
            "act": source_case.get("act"),
            "floor": source_case.get("floor"),
            "seed": source_case.get("seed"),
            "step_index": source_case.get("step_index"),
            "chosen_key": action_key(source_case.get("chosen") or {}),
            "cashout_best_key": action_key(source_case.get("best_by_cashout") or {}),
            "trace_file": source_case.get("trace_file"),
        }
        trace_file = resolve_path(source_case.get("trace_file"))
        if trace_file is not None:
            trace_step_to_case[f"{str(trace_file).lower()}:{source_case.get('step_index')}"] = case_id
        if source_case.get("seed") is not None and source_case.get("step_index") is not None:
            trace_step_to_case[case_key(source_case.get("seed"), source_case.get("step_index"))] = case_id
    return meta, trace_step_to_case


def diagnose_row(row: dict[str, Any], vectors: dict[str, dict[str, Any]] | None, vector_lookup: str) -> dict[str, Any]:
    preferred_key = str(row.get("preferred_key") or "")
    rejected_key = str(row.get("rejected_key") or "")
    preferred_card = candidate_card_from_outcome(row.get("preferred_outcome"))
    rejected_card = candidate_card_from_outcome(row.get("rejected_outcome"))

    base = {
        "case_id": row.get("case_id"),
        "continuation_policy": row.get("continuation_policy"),
        "horizon": row.get("horizon"),
        "source_calibration_status": row.get("source_calibration_status"),
        "reason": row.get("reason"),
        "preferred_key": preferred_key,
        "rejected_key": rejected_key,
        "preferred_card": preferred_card,
        "rejected_card": rejected_card,
        "vector_lookup": vector_lookup,
    }

    if not vectors:
        return {**base, "status": "missing_vector_source"}

    preferred = vectors.get(preferred_key)
    rejected = vectors.get(rejected_key)
    if not preferred or not rejected:
        return {
            **base,
            "status": "missing_candidate_vector",
            "available_keys": sorted(vectors.keys()),
        }

    preferred_delta = preferred.get("plan_delta") or {}
    rejected_delta = rejected.get("plan_delta") or {}
    if not is_vectorizable(preferred_delta) or not is_vectorizable(rejected_delta):
        return {
            **base,
            "status": "not_vectorizable",
            "preferred_delta": compact_delta(preferred_delta),
            "rejected_delta": compact_delta(rejected_delta),
        }

    preferred_dominates = pareto_dominates_with_dims(preferred_delta, rejected_delta)
    rejected_dominates = pareto_dominates_with_dims(rejected_delta, preferred_delta)
    if preferred_dominates:
        status = "agree"
        pareto_winner = "preferred"
        strict_dims = strict_advantage_dims(preferred_delta, rejected_delta)
    elif rejected_dominates:
        status = "contradict"
        pareto_winner = "rejected"
        strict_dims = strict_advantage_dims(rejected_delta, preferred_delta)
    else:
        status = "no_opinion"
        pareto_winner = "none"
        strict_dims = []

    return {
        **base,
        "status": status,
        "pareto_winner": pareto_winner,
        "strict_advantage_dims": strict_dims,
        "preferred_delta": compact_delta(preferred_delta),
        "rejected_delta": compact_delta(rejected_delta),
        "preferred_vector_source": preferred.get("source"),
        "rejected_vector_source": rejected.get("source"),
    }


def summarize(rows: list[dict[str, Any]], case_meta: dict[str, dict[str, Any]], top: int) -> dict[str, Any]:
    status_counts = Counter(row["status"] for row in rows)
    by_card = Counter()
    by_dim = Counter()
    by_policy = Counter()
    by_act_floor = Counter()
    by_label_substatus = Counter()

    for row in rows:
        meta = case_meta.get(str(row.get("case_id") or ""), {})
        key_status = row["status"]
        if key_status == "contradict":
            by_card[f"{row['preferred_card']} over vector:{row['rejected_card']}"] += 1
            for dim in row.get("strict_advantage_dims") or []:
                by_dim[dim] += 1
            by_policy[f"{row.get('continuation_policy')}@{row.get('horizon')}"] += 1
            by_act_floor[f"act{meta.get('act')}_floor{meta.get('floor')}"] += 1
            by_label_substatus[str(meta.get("label_substatus"))] += 1

    sensitivity = []
    for dim, _ in DOMINANCE_DIMS:
        dims_without = [(d, hb) for d, hb in DOMINANCE_DIMS if d != dim]
        contradictions_resolved = 0
        agreements_lost = 0
        for row in rows:
            if row["status"] not in {"agree", "contradict"}:
                continue
            pd = row.get("preferred_delta") or {}
            rd = row.get("rejected_delta") or {}
            if row["status"] == "contradict":
                if not pareto_dominates_with_dims(rd, pd, dims_without):
                    contradictions_resolved += 1
            elif row["status"] == "agree":
                if not pareto_dominates_with_dims(pd, rd, dims_without):
                    agreements_lost += 1
        sensitivity.append(
            {
                "dimension": dim,
                "contradictions_resolved_if_removed": contradictions_resolved,
                "agreements_lost_if_removed": agreements_lost,
                "net": contradictions_resolved - agreements_lost,
            }
        )
    sensitivity.sort(key=lambda item: (item["net"], item["contradictions_resolved_if_removed"]), reverse=True)

    return {
        "status_counts": dict(status_counts),
        "contradictions_by_card_pair": by_card.most_common(top),
        "contradictions_by_strict_dimension": by_dim.most_common(top),
        "contradictions_by_policy_horizon": by_policy.most_common(top),
        "contradictions_by_act_floor": by_act_floor.most_common(top),
        "contradictions_by_label_substatus": by_label_substatus.most_common(top),
        "dimension_sensitivity": sensitivity,
    }


def markdown_report(report: dict[str, Any], top: int) -> str:
    lines = [
        "# Pareto Feedback Diagnostic",
        "",
        "This report treats Pareto dominance as a heuristic hypothesis, not truth.",
        "",
        "## Summary",
        "",
    ]
    for key, value in report["summary"].items():
        lines.append(f"- {key}: `{value}`")
    lines.extend(["", "## Status Counts", ""])
    for key, value in report["diagnostics"]["status_counts"].items():
        lines.append(f"- {key}: `{value}`")

    def table(title: str, rows: list[Any], headers: tuple[str, str]) -> None:
        lines.extend(["", f"## {title}", ""])
        if not rows:
            lines.append("_none_")
            return
        lines.append(f"| {headers[0]} | {headers[1]} |")
        lines.append("| --- | --- |")
        for key, value in rows[:top]:
            lines.append(f"| `{key}` | {value} |")

    table("Contradictions By Card Pair", report["diagnostics"]["contradictions_by_card_pair"], ("pair", "n"))
    table(
        "Contradictions By Strict Dimension",
        report["diagnostics"]["contradictions_by_strict_dimension"],
        ("dimension", "n"),
    )
    table(
        "Contradictions By Policy/Horizon",
        report["diagnostics"]["contradictions_by_policy_horizon"],
        ("policy@horizon", "n"),
    )
    table("Contradictions By Act/Floor", report["diagnostics"]["contradictions_by_act_floor"], ("act/floor", "n"))

    lines.extend(["", "## Dimension Sensitivity", ""])
    lines.append("| dimension | contradictions resolved if removed | agreements lost if removed | net |")
    lines.append("| --- | ---: | ---: | ---: |")
    for row in report["diagnostics"]["dimension_sensitivity"][:top]:
        lines.append(
            f"| `{row['dimension']}` | {row['contradictions_resolved_if_removed']} | "
            f"{row['agreements_lost_if_removed']} | {row['net']} |"
        )

    lines.extend(["", "## Limitations", ""])
    for note in report["limitations"]:
        lines.append(f"- {note}")
    return "\n".join(lines) + "\n"


def main() -> None:
    args = parse_args()
    label_report = read_json(args.label_report)
    pairwise = load_jsonl(args.pairwise_labels)
    case_meta, trace_step_to_case = build_case_metadata(label_report)

    index = VectorIndex()
    load_vectors_from_label_report(index, label_report)
    load_vectors_from_cashout_report(index, args.cashout_report if args.cashout_report.exists() else None)

    trace_paths = collect_trace_paths(args, label_report)
    existing_trace_files = 0
    for path in trace_paths:
        if path.exists():
            existing_trace_files += 1
            load_trace_file(index, path, trace_step_to_case)
        else:
            index.missing_trace_files.add(str(path))

    rows = []
    for row in pairwise:
        vectors, lookup = index.vectors_for(row)
        diagnosed = diagnose_row(row, vectors, lookup)
        meta = case_meta.get(str(row.get("case_id") or ""), {})
        diagnosed.update(
            {
                "case_label_status": meta.get("label_status"),
                "case_label_substatus": meta.get("label_substatus"),
                "strong_training_signal": meta.get("strong_training_signal"),
                "act": meta.get("act"),
                "floor": meta.get("floor"),
            }
        )
        rows.append(diagnosed)

    diagnostics = summarize(rows, case_meta, args.top)
    report = {
        "report_version": "pareto_feedback_v0",
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {
            "label_report": str(args.label_report),
            "pairwise_labels": str(args.pairwise_labels),
            "cashout_report": str(args.cashout_report),
            "trace_roots": [str(p) for p in args.trace_root or []],
            "trace_globs": list(args.trace_glob or []),
            "dominance_dims": [{"name": dim, "higher_better": hb} for dim, hb in DOMINANCE_DIMS],
        },
        "summary": {
            "pairwise_rows": len(pairwise),
            "cases_in_label_report": len(case_meta),
            "diagnosed_rows": len(rows),
            "vector_records_by_source": dict(index.sources),
            "trace_files_requested": len(trace_paths),
            "trace_files_existing": existing_trace_files,
            "trace_files_scanned": index.trace_files_scanned,
            "reward_steps_scanned": index.reward_steps_scanned,
        },
        "diagnostics": diagnostics,
        "rows": rows,
        "limitations": [
            "Pairwise rollout labels are conditional on continuation policy and horizon.",
            "Pareto dominance is a heuristic hypothesis, not a policy-independent card-value truth.",
            "Rows with missing_vector_source or missing_candidate_vector were not tested.",
            "If original full-run traces are absent, candidate plan_delta vectors cannot be reconstructed from current compact rollout artifacts.",
        ],
    }
    write_json(args.out, report)
    md_path = args.out.with_suffix(".md")
    md_path.write_text(markdown_report(report, args.top), encoding="utf-8")
    print(f"Wrote {args.out}")
    print(f"Wrote {md_path}")
    print(json.dumps(report["summary"], indent=2, ensure_ascii=False))
    print(json.dumps(report["diagnostics"]["status_counts"], indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
