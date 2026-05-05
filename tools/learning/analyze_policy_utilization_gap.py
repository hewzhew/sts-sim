#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json


DEFAULT_LABEL_DIR = (
    REPO_ROOT
    / "tools"
    / "artifacts"
    / "card_cashout_rollout_labels"
    / "v1_1_attribution_100case_cashout_v0_6"
)

CASE_BUCKETS = {
    "rollout_refuted": "true_static_fix_needed",
    "requires_cashout_policy": "policy_utilization_gap",
    "rollout_unstable": "continuation_bad_or_unknown",
    "robust_confirmed": "robust_training_candidate",
    "rollout_equivalent": "equivalent_or_low_signal",
    "rollout_confirmed": "weak_confirmed",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Classify cashout rollout labels by policy utilization gap. "
            "This is a diagnostic report, not a training command."
        )
    )
    parser.add_argument("--label-dir", type=Path, default=DEFAULT_LABEL_DIR)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--pairwise-labels", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--markdown-out", type=Path)
    parser.add_argument("--robust-pairwise-out", type=Path)
    parser.add_argument("--top-n", type=int, default=20)
    return parser.parse_args()


def resolve(path: Path) -> Path:
    return path if path.is_absolute() else REPO_ROOT / path


def read_json(path: Path) -> dict[str, Any]:
    with resolve(path).open("r", encoding="utf-8") as handle:
        return json.load(handle)


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    real = resolve(path)
    if not real.exists():
        return []
    rows = []
    with real.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


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


def card_name(outcome: dict[str, Any] | None, key: str = "") -> str:
    if not outcome:
        return "proceed" if key == "proceed" else str(key or "unknown")
    return str(outcome.get("card_id") or outcome.get("candidate_key") or key or "unknown")


def verdict_matrix(label: dict[str, Any]) -> dict[str, dict[str, int]]:
    matrix: dict[str, Counter[str]] = defaultdict(Counter)
    for obs in label.get("observations") or []:
        policy = str(obs.get("continuation_policy") or "unknown_policy")
        horizon = str(obs.get("horizon") or "unknown_horizon")
        verdict = str((obs.get("classification") or {}).get("verdict") or obs.get("status") or "unknown")
        matrix[f"{policy}@{horizon}"][verdict] += 1
        matrix[policy][verdict] += 1
    return {key: dict(sorted(counter.items())) for key, counter in sorted(matrix.items())}


def observation_notes(label: dict[str, Any]) -> list[str]:
    confirmed_by = set()
    refuted_by = set()
    equivalent_by = set()
    for obs in label.get("observations") or []:
        policy = str(obs.get("continuation_policy") or "")
        verdict = str((obs.get("classification") or {}).get("verdict") or "")
        if verdict == "rollout_confirmed":
            confirmed_by.add(policy)
        elif verdict == "rollout_refuted":
            refuted_by.add(policy)
        elif verdict == "rollout_equivalent":
            equivalent_by.add(policy)
    notes: list[str] = []
    if "plan_query_v0" in confirmed_by and "rule_baseline_v0" not in confirmed_by:
        notes.append("plan_query confirms but rule_baseline does not; likely policy utilization gap")
    if confirmed_by and refuted_by:
        notes.append("confirmed and refuted verdicts coexist; do not use as hard label")
    if equivalent_by and not confirmed_by and not refuted_by:
        notes.append("all observed continuations are below margin")
    return notes


def compact_case(label: dict[str, Any]) -> dict[str, Any]:
    source = label.get("source_case") or {}
    chosen = source.get("chosen") or {}
    best = source.get("best_by_cashout") or {}
    status = str(label.get("label_status") or "unknown")
    return {
        "case_id": label.get("case_id"),
        "bucket": CASE_BUCKETS.get(status, "other"),
        "label_status": status,
        "source_policy": label.get("source_policy"),
        "seed": source.get("seed"),
        "step_index": source.get("step_index"),
        "act": source.get("act"),
        "floor": source.get("floor"),
        "hp": source.get("hp"),
        "source_calibration_status": source.get("calibration_status"),
        "cashout_kinds": source.get("cashout_kinds") or [],
        "chosen_card": chosen.get("card_id"),
        "chosen_key": chosen.get("action_key"),
        "chosen_cashout_score": chosen.get("cashout_score"),
        "cashout_best_card": best.get("card_id"),
        "cashout_best_key": best.get("action_key"),
        "cashout_best_score": best.get("cashout_score"),
        "cashout_gap": source.get("cashout_gap"),
        "verdict_matrix": verdict_matrix(label),
        "notes": observation_notes(label),
    }


def pairwise_context(row: dict[str, Any], case_by_id: dict[str, dict[str, Any]]) -> dict[str, Any]:
    case_id = str(row.get("case_id") or "")
    case = case_by_id.get(case_id) or {}
    preferred = row.get("preferred_outcome") or {}
    rejected = row.get("rejected_outcome") or {}
    diff = row.get("outcome_diff_preferred_minus_rejected") or {}
    attr = diff.get("attribution") or {}
    source = case.get("source_case") or {}
    best_key = str((source.get("best_by_cashout") or {}).get("action_key") or "")
    chosen_key = str((source.get("chosen") or {}).get("action_key") or "")
    preferred_key = str(row.get("preferred_key") or "")
    rejected_key = str(row.get("rejected_key") or "")
    preferred_card = card_name(preferred, preferred_key)
    rejected_card = card_name(rejected, rejected_key)
    return {
        "case_id": case_id,
        "case_label_status": str(case.get("label_status") or "unknown"),
        "case_bucket": CASE_BUCKETS.get(str(case.get("label_status") or ""), "other"),
        "source_policy": row.get("source_policy"),
        "continuation_policy": row.get("continuation_policy"),
        "horizon": row.get("horizon"),
        "reason": row.get("reason"),
        "preferred_key": preferred_key,
        "rejected_key": rejected_key,
        "preferred_card": preferred_card,
        "rejected_card": rejected_card,
        "preferred_is_static_best": preferred_key == best_key,
        "rejected_is_static_best": rejected_key == best_key,
        "preferred_is_source_chosen": preferred_key == chosen_key,
        "rejected_is_source_chosen": rejected_key == chosen_key,
        "floor_delta": diff.get("floor_delta"),
        "combat_win_delta": diff.get("combat_win_delta"),
        "end_hp_delta": diff.get("end_hp"),
        "reward_total_delta": diff.get("reward_total"),
        "attr_hp_loss": attr.get("hp_loss_observed"),
        "attr_monster_hp": attr.get("monster_hp_reduction_observed"),
        "attr_kills": attr.get("alive_monster_reduction_observed"),
    }


def robust_fixture_row(row: dict[str, Any], case_by_id: dict[str, dict[str, Any]]) -> dict[str, Any] | None:
    ctx = pairwise_context(row, case_by_id)
    if ctx["case_label_status"] != "robust_confirmed":
        return None
    if not ctx["preferred_is_static_best"]:
        return None
    if ctx["preferred_key"] == ctx["rejected_key"]:
        return None
    return {
        "label_mode": "cashout_policy_utilization_gap_v0_fixture",
        "source_label_mode": row.get("label_mode"),
        "game_rng_mode": row.get("game_rng_mode"),
        "case_id": row.get("case_id"),
        "source_policy": row.get("source_policy"),
        "continuation_policy": row.get("continuation_policy"),
        "horizon": row.get("horizon"),
        "preferred_key": row.get("preferred_key"),
        "preferred_card": ctx["preferred_card"],
        "rejected_key": row.get("rejected_key"),
        "rejected_card": ctx["rejected_card"],
        "reason": row.get("reason"),
        "suggested_weight": 1.0,
        "contract": "fixture/smoke only; robust within current policy/horizon rollout labeler",
        "outcome_diff_preferred_minus_rejected": row.get("outcome_diff_preferred_minus_rejected") or {},
    }


def build_analysis(report: dict[str, Any], pairwise_rows: list[dict[str, Any]]) -> dict[str, Any]:
    labels = list(report.get("labels") or [])
    case_by_id = {str(label.get("case_id") or ""): label for label in labels}
    compact_cases = [compact_case(label) for label in labels]
    bucket_counts = Counter(row["bucket"] for row in compact_cases)
    status_counts = Counter(row["label_status"] for row in compact_cases)

    pair_contexts = [pairwise_context(row, case_by_id) for row in pairwise_rows]
    proceed_beats_static_best = [
        row
        for row in pair_contexts
        if row["preferred_key"] == "proceed" and row["rejected_is_static_best"]
    ]
    proceed_beats_non_best = [
        row
        for row in pair_contexts
        if row["preferred_key"] == "proceed" and not row["rejected_is_static_best"]
    ]
    static_best_wins = [row for row in pair_contexts if row["preferred_is_static_best"]]
    static_best_loses = [row for row in pair_contexts if row["rejected_is_static_best"]]

    robust_rows = [
        fixture
        for row in pairwise_rows
        if (fixture := robust_fixture_row(row, case_by_id)) is not None
    ]

    by_bucket: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in compact_cases:
        by_bucket[row["bucket"]].append(row)

    policy_case_counts: dict[str, Counter[str]] = defaultdict(Counter)
    for label in labels:
        status = str(label.get("label_status") or "unknown")
        for obs in label.get("observations") or []:
            policy = str(obs.get("continuation_policy") or "unknown")
            verdict = str((obs.get("classification") or {}).get("verdict") or obs.get("status") or "unknown")
            policy_case_counts[policy][f"{status}:{verdict}"] += 1

    return {
        "report_version": "policy_utilization_gap_v0",
        "source_report_version": report.get("report_version"),
        "summary": {
            "case_count": len(labels),
            "pairwise_count": len(pairwise_rows),
            "case_bucket_counts": dict(sorted(bucket_counts.items())),
            "label_status_counts": dict(sorted(status_counts.items())),
            "static_best_pairwise_wins": len(static_best_wins),
            "static_best_pairwise_losses": len(static_best_loses),
            "proceed_beats_static_best_count": len(proceed_beats_static_best),
            "proceed_beats_non_best_count": len(proceed_beats_non_best),
            "robust_pairwise_fixture_count": len(robust_rows),
            "contract": (
                "This is residual routing. Only robust_pairwise_fixture rows are exported, "
                "and even those are fixture/smoke labels rather than a production dataset."
            ),
        },
        "policy_verdict_cross_counts": {
            key: dict(sorted(counter.items())) for key, counter in sorted(policy_case_counts.items())
        },
        "cases_by_bucket": {key: rows for key, rows in sorted(by_bucket.items())},
        "skip_or_proceed": {
            "proceed_beats_static_best": proceed_beats_static_best,
            "proceed_beats_non_best": proceed_beats_non_best[:100],
        },
        "static_best_pairwise": {
            "wins": static_best_wins[:100],
            "losses": static_best_loses[:100],
        },
        "robust_pairwise_fixtures": robust_rows,
    }


def fmt(value: Any) -> str:
    if isinstance(value, float):
        return f"{value:.3f}"
    if value is None:
        return ""
    return str(value)


def write_markdown(path: Path, analysis: dict[str, Any], *, top_n: int) -> None:
    summary = analysis["summary"]
    lines = [
        "# Policy Utilization Gap Analysis",
        "",
        "This report routes cashout rollout residuals into actionable buckets.",
        "It is not a training report and does not claim policy-independent card value.",
        "",
        "## Summary",
        "",
        f"- source report: `{analysis['source_report_version']}`",
        f"- cases: `{summary['case_count']}`",
        f"- pairwise rows: `{summary['pairwise_count']}`",
        f"- case buckets: `{summary['case_bucket_counts']}`",
        f"- label statuses: `{summary['label_status_counts']}`",
        f"- static-best pairwise wins/losses: `{summary['static_best_pairwise_wins']}` / `{summary['static_best_pairwise_losses']}`",
        f"- proceed beats static-best: `{summary['proceed_beats_static_best_count']}`",
        f"- proceed beats non-best: `{summary['proceed_beats_non_best_count']}`",
        f"- robust pairwise fixture rows: `{summary['robust_pairwise_fixture_count']}`",
        f"- contract: `{summary['contract']}`",
        "",
        "## Bucket Meaning",
        "",
        "- `true_static_fix_needed`: rollout refuted the static cashout-best; inspect static semantics first.",
        "- `policy_utilization_gap`: plan-query continuation can realize value that rule baseline does not; improve continuation before treating as card truth.",
        "- `continuation_bad_or_unknown`: policy/horizon conflicts; send to probe/oracle, not training.",
        "- `robust_training_candidate`: multiple policy/horizon settings agree; usable as fixture/smoke pairwise evidence.",
        "- `equivalent_or_low_signal`: below effect margins.",
        "- `weak_confirmed`: one-sided confirmation that is not yet robust.",
        "",
        "## Policy Verdict Cross Counts",
        "",
        "| policy | counts |",
        "|---|---|",
    ]
    for policy, counts in analysis["policy_verdict_cross_counts"].items():
        lines.append(f"| {policy} | `{counts}` |")
    lines.append("")

    def add_case_table(title: str, rows: list[dict[str, Any]]) -> None:
        lines.extend(
            [
                f"## {title}",
                "",
                "| case | status | source | act/floor/hp | chosen | cashout best | gap | kinds | notes |",
                "|---|---|---|---|---|---|---:|---|---|",
            ]
        )
        for row in rows[:top_n]:
            lines.append(
                "| {case} | {status} | {source} | {act}/{floor}/{hp} | {chosen} | {best} | {gap} | `{kinds}` | `{notes}` |".format(
                    case=row.get("case_id"),
                    status=row.get("label_status"),
                    source=row.get("source_policy"),
                    act=row.get("act"),
                    floor=row.get("floor"),
                    hp=row.get("hp"),
                    chosen=row.get("chosen_card"),
                    best=row.get("cashout_best_card"),
                    gap=fmt(row.get("cashout_gap")),
                    kinds=row.get("cashout_kinds"),
                    notes=row.get("notes"),
                )
            )
        lines.append("")

    cases_by_bucket = analysis["cases_by_bucket"]
    for bucket in [
        "true_static_fix_needed",
        "policy_utilization_gap",
        "continuation_bad_or_unknown",
        "robust_training_candidate",
        "weak_confirmed",
        "equivalent_or_low_signal",
    ]:
        add_case_table(bucket, cases_by_bucket.get(bucket, []))

    def add_pair_table(title: str, rows: list[dict[str, Any]]) -> None:
        lines.extend(
            [
                f"## {title}",
                "",
                "| case | status | context | preferred > rejected | reason | floor/combat/hp | attr hp/monster/kills |",
                "|---|---|---|---|---|---|---|",
            ]
        )
        for row in rows[:top_n]:
            lines.append(
                "| {case} | {status} | {policy}@{horizon} | {preferred} > {rejected} | {reason} | {floor}/{combat}/{hp} | {attr_hp}/{attr_monster}/{attr_kills} |".format(
                    case=row.get("case_id"),
                    status=row.get("case_label_status"),
                    policy=row.get("continuation_policy"),
                    horizon=row.get("horizon"),
                    preferred=row.get("preferred_card"),
                    rejected=row.get("rejected_card"),
                    reason=row.get("reason"),
                    floor=fmt(row.get("floor_delta")),
                    combat=fmt(row.get("combat_win_delta")),
                    hp=fmt(row.get("end_hp_delta")),
                    attr_hp=fmt(row.get("attr_hp_loss")),
                    attr_monster=fmt(row.get("attr_monster_hp")),
                    attr_kills=fmt(row.get("attr_kills")),
                )
            )
        lines.append("")

    add_pair_table("Proceed Beats Static Best", analysis["skip_or_proceed"]["proceed_beats_static_best"])
    add_pair_table("Proceed Beats Non-Best", analysis["skip_or_proceed"]["proceed_beats_non_best"])
    add_pair_table("Static Best Pairwise Losses", analysis["static_best_pairwise"]["losses"])

    real = resolve(path)
    real.parent.mkdir(parents=True, exist_ok=True)
    real.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    label_dir = resolve(args.label_dir)
    report_path = args.report or label_dir / "cashout_rollout_label_report.json"
    pairwise_path = args.pairwise_labels or label_dir / "pairwise_labels.jsonl"
    out_path = resolve(args.out) if args.out else label_dir / "policy_utilization_gap_analysis.json"
    markdown_path = resolve(args.markdown_out) if args.markdown_out else out_path.with_suffix(".md")
    robust_path = (
        resolve(args.robust_pairwise_out)
        if args.robust_pairwise_out
        else label_dir / "robust_pairwise_labels.jsonl"
    )

    report = read_json(report_path)
    pairwise_rows = read_jsonl(pairwise_path)
    analysis = build_analysis(report, pairwise_rows)
    write_json(out_path, analysis)
    write_markdown(markdown_path, analysis, top_n=args.top_n)
    write_jsonl(robust_path, analysis["robust_pairwise_fixtures"])
    print(
        json.dumps(
            {
                "out": str(out_path),
                "markdown_out": str(markdown_path),
                "robust_pairwise_out": str(robust_path),
                "summary": analysis["summary"],
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
