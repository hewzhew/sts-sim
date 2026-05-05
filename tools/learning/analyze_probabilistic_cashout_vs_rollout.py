#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json


DEFAULT_CASHOUT_REPORT = (
    REPO_ROOT
    / "tools"
    / "artifacts"
    / "probabilistic_cashout_lab"
    / "v0_2_archetype_cashout_v0_5"
    / "cashout_distribution_report.json"
)
DEFAULT_LABEL_DIR = (
    REPO_ROOT
    / "tools"
    / "artifacts"
    / "card_cashout_rollout_labels"
    / "v1_cashout_v0_5_100case"
)
DEFAULT_OUT = (
    REPO_ROOT
    / "tools"
    / "artifacts"
    / "probabilistic_cashout_lab"
    / "v0_2_archetype_cashout_v0_5"
    / "cashout_rollout_alignment_report.json"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Compare probabilistic cashout lab scores against rollout labels/outcomes. "
            "This is an alignment audit: it does not create teacher labels by itself."
        )
    )
    parser.add_argument("--cashout-report", type=Path, default=DEFAULT_CASHOUT_REPORT)
    parser.add_argument("--label-dir", type=Path, default=DEFAULT_LABEL_DIR)
    parser.add_argument("--label-report", type=Path)
    parser.add_argument("--candidate-outcomes", type=Path)
    parser.add_argument("--pairwise-labels", type=Path)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--markdown-out", type=Path)
    parser.add_argument("--top-n", type=int, default=25)
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
    rows: list[dict[str, Any]] = []
    with real.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def num(value: Any) -> float:
    try:
        if value is None:
            return 0.0
        out = float(value)
        return out if math.isfinite(out) else 0.0
    except (TypeError, ValueError):
        return 0.0


def pct(numerator: int, denominator: int) -> float:
    return round(100.0 * numerator / denominator, 2) if denominator else 0.0


def mean(values: list[float]) -> float:
    return sum(values) / len(values) if values else 0.0


def pearson(pairs: list[tuple[float, float]]) -> float:
    if len(pairs) < 2:
        return 0.0
    xs = [x for x, _ in pairs]
    ys = [y for _, y in pairs]
    mx = mean(xs)
    my = mean(ys)
    vx = sum((x - mx) ** 2 for x in xs)
    vy = sum((y - my) ** 2 for y in ys)
    if vx <= 0 or vy <= 0:
        return 0.0
    cov = sum((x - mx) * (y - my) for x, y in pairs)
    return round(cov / math.sqrt(vx * vy), 4)


def ranks(values: list[float]) -> list[float]:
    ordered = sorted(enumerate(values), key=lambda item: item[1])
    out = [0.0] * len(values)
    i = 0
    while i < len(ordered):
        j = i + 1
        while j < len(ordered) and ordered[j][1] == ordered[i][1]:
            j += 1
        rank = (i + 1 + j) / 2.0
        for idx, _ in ordered[i:j]:
            out[idx] = rank
        i = j
    return out


def spearman(pairs: list[tuple[float, float]]) -> float:
    if len(pairs) < 2:
        return 0.0
    xs = ranks([x for x, _ in pairs])
    ys = ranks([y for _, y in pairs])
    return pearson(list(zip(xs, ys)))


def outcome_value(outcome: dict[str, Any]) -> float:
    terminal = str(outcome.get("end_result") or "")
    terminal_bonus = -45.0 if terminal == "defeat" else 0.0
    return (
        num(outcome.get("floor_delta")) * 14.0
        + num(outcome.get("combat_win_delta")) * 5.0
        + num(outcome.get("end_hp")) * 0.22
        + num(outcome.get("reward_total"))
        + terminal_bonus
    )


def context_key(row: dict[str, Any]) -> tuple[str, str, int]:
    return (
        str(row.get("case_id") or ""),
        str(row.get("continuation_policy") or "unknown"),
        int(row.get("horizon") or 0),
    )


def candidate_key(candidate: dict[str, Any]) -> str:
    return str(candidate.get("action_key") or candidate.get("candidate_key") or "")


def card_id(candidate: dict[str, Any]) -> str:
    return str(candidate.get("card_id") or candidate.get("action_key") or candidate.get("candidate_key") or "unknown")


def dominant_archetype(case: dict[str, Any]) -> str:
    archetypes = ((case.get("future_room_summary") or {}).get("encounter_archetypes") or {})
    if not archetypes:
        return "unknown"
    return max(archetypes.items(), key=lambda item: num(item[1]))[0]


def lab_eval_by_key(case: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {str(row.get("action_key") or ""): row for row in case.get("candidate_evals") or []}


@dataclass
class Bucket:
    total: int = 0
    lab_hits: int = 0
    static_hits: int = 0
    chosen_hits: int = 0
    lab_regrets: list[float] = field(default_factory=list)
    static_regrets: list[float] = field(default_factory=list)
    spearman_values: list[float] = field(default_factory=list)
    statuses: Counter[str] = field(default_factory=Counter)

    def add(self, row: dict[str, Any]) -> None:
        self.total += 1
        self.lab_hits += int(bool(row.get("lab_hit")))
        self.static_hits += int(bool(row.get("static_hit")))
        self.chosen_hits += int(bool(row.get("chosen_hit")))
        self.lab_regrets.append(num(row.get("lab_regret")))
        self.static_regrets.append(num(row.get("static_regret")))
        if row.get("spearman") is not None:
            self.spearman_values.append(num(row.get("spearman")))
        self.statuses[str(row.get("label_status") or "unknown")] += 1


def bucket_row(name: str, bucket: Bucket) -> dict[str, Any]:
    return {
        "name": name,
        "total": bucket.total,
        "lab_hit_rate": pct(bucket.lab_hits, bucket.total),
        "static_hit_rate": pct(bucket.static_hits, bucket.total),
        "chosen_hit_rate": pct(bucket.chosen_hits, bucket.total),
        "avg_lab_regret": round(mean(bucket.lab_regrets), 3),
        "avg_static_regret": round(mean(bucket.static_regrets), 3),
        "avg_spearman": round(mean(bucket.spearman_values), 4),
        "label_statuses": dict(sorted(bucket.statuses.items())),
    }


def winner_from_pairwise(
    context: tuple[str, str, int],
    pairwise_by_context: dict[tuple[str, str, int], list[dict[str, Any]]],
    outcomes: dict[str, dict[str, Any]],
) -> str:
    rows = pairwise_by_context.get(context) or []
    scores: Counter[str] = Counter()
    for row in rows:
        scores[str(row.get("preferred_key") or "")] += 1
        scores[str(row.get("rejected_key") or "")] -= 1
    valid = {key for key in outcomes.keys() if key}
    if scores:
        candidates = [(key, score) for key, score in scores.items() if key in valid]
        if candidates:
            return max(candidates, key=lambda item: (item[1], outcome_value(outcomes[item[0]])))[0]
    if outcomes:
        return max(outcomes.items(), key=lambda item: outcome_value(item[1]))[0]
    return ""


def build_analysis(
    cashout_report: dict[str, Any],
    label_report: dict[str, Any],
    candidate_rows: list[dict[str, Any]],
    pairwise_rows: list[dict[str, Any]],
    *,
    top_n: int,
) -> dict[str, Any]:
    cases = {str(case.get("case_id") or ""): case for case in cashout_report.get("cases") or []}
    label_status = {
        str(label.get("case_id") or ""): str(label.get("label_status") or "unknown")
        for label in label_report.get("labels") or []
    }
    outcomes_by_context: dict[tuple[str, str, int], dict[str, dict[str, Any]]] = defaultdict(dict)
    for row in candidate_rows:
        cand = row.get("candidate") or {}
        key = str(cand.get("candidate_key") or "")
        if key:
            outcomes_by_context[context_key(row)][key] = cand

    pairwise_by_context: dict[tuple[str, str, int], list[dict[str, Any]]] = defaultdict(list)
    for row in pairwise_rows:
        pairwise_by_context[context_key(row)].append(row)

    context_rows: list[dict[str, Any]] = []
    candidate_pairs: list[tuple[float, float]] = []
    floor_pairs: list[tuple[float, float]] = []
    hp_pairs: list[tuple[float, float]] = []
    reward_pairs: list[tuple[float, float]] = []

    for ctx, outcomes in sorted(outcomes_by_context.items()):
        case_id, policy, horizon = ctx
        case = cases.get(case_id)
        if not case:
            continue
        evals = lab_eval_by_key(case)
        winner_key = winner_from_pairwise(ctx, pairwise_by_context, outcomes)
        if not winner_key:
            continue
        lab_best_key = str((case.get("lab_best") or {}).get("action_key") or "")
        static_best_key = str((case.get("static_best") or {}).get("action_key") or "")
        chosen_key = str((case.get("chosen") or {}).get("action_key") or "")
        winner_value = outcome_value(outcomes.get(winner_key) or {})

        scored_pairs: list[tuple[float, float]] = []
        for key, outcome in outcomes.items():
            lab_score = num((evals.get(key) or {}).get("lab_expected_score"))
            value = outcome_value(outcome)
            scored_pairs.append((lab_score, value))
            candidate_pairs.append((lab_score, value))
            floor_pairs.append((lab_score, num(outcome.get("floor_delta"))))
            hp_pairs.append((lab_score, num(outcome.get("end_hp"))))
            reward_pairs.append((lab_score, num(outcome.get("reward_total"))))

        def regret(key: str) -> float:
            if key not in outcomes:
                return 0.0
            return max(0.0, winner_value - outcome_value(outcomes[key]))

        row = {
            "case_id": case_id,
            "source_policy": str(case.get("policy") or ""),
            "continuation_policy": policy,
            "horizon": horizon,
            "label_status": label_status.get(case_id, "unknown"),
            "act": int(case.get("act") or 0),
            "floor": int(case.get("floor") or 0),
            "dominant_archetype": dominant_archetype(case),
            "dominant_cashout": str((case.get("static_best") or {}).get("dominant_cashout") or "unknown"),
            "static_best_card": card_id(case.get("static_best") or {}),
            "lab_best_card": card_id(case.get("lab_best") or {}),
            "winner_card": card_id(outcomes.get(winner_key) or {}),
            "winner_key": winner_key,
            "lab_best_key": lab_best_key,
            "static_best_key": static_best_key,
            "chosen_key": chosen_key,
            "lab_hit": lab_best_key == winner_key,
            "static_hit": static_best_key == winner_key,
            "chosen_hit": chosen_key == winner_key,
            "lab_regret": round(regret(lab_best_key), 3),
            "static_regret": round(regret(static_best_key), 3),
            "chosen_regret": round(regret(chosen_key), 3),
            "winner_value": round(winner_value, 3),
            "lab_spearman": spearman(scored_pairs),
            "spearman": spearman(scored_pairs),
            "candidate_count": len(outcomes),
            "case_flags": case.get("case_flags") or [],
            "prior_impact": case.get("prior_impact") or {},
        }
        context_rows.append(row)

    buckets: dict[str, dict[str, Bucket]] = {
        "by_policy_horizon": defaultdict(Bucket),
        "by_static_best_card": defaultdict(Bucket),
        "by_dominant_cashout": defaultdict(Bucket),
        "by_archetype": defaultdict(Bucket),
        "by_label_status": defaultdict(Bucket),
    }
    for row in context_rows:
        buckets["by_policy_horizon"][f"{row['continuation_policy']}@{row['horizon']}"].add(row)
        buckets["by_static_best_card"][str(row["static_best_card"])].add(row)
        buckets["by_dominant_cashout"][str(row["dominant_cashout"])].add(row)
        buckets["by_archetype"][str(row["dominant_archetype"])].add(row)
        buckets["by_label_status"][str(row["label_status"])].add(row)

    aggregate_tables = {
        table_name: sorted(
            [bucket_row(name, bucket) for name, bucket in table.items()],
            key=lambda row: (-int(row["total"]), -float(row["avg_lab_regret"]), str(row["name"])),
        )
        for table_name, table in buckets.items()
    }

    lab_hits = sum(1 for row in context_rows if row["lab_hit"])
    static_hits = sum(1 for row in context_rows if row["static_hit"])
    chosen_hits = sum(1 for row in context_rows if row["chosen_hit"])
    lab_regrets = [num(row["lab_regret"]) for row in context_rows]
    static_regrets = [num(row["static_regret"]) for row in context_rows]
    spearman_values = [num(row["spearman"]) for row in context_rows]

    high_regret_misses = sorted(
        [row for row in context_rows if not row["lab_hit"]],
        key=lambda row: (-num(row["lab_regret"]), str(row["case_id"]), str(row["continuation_policy"]), int(row["horizon"])),
    )[:top_n]
    static_beats_lab = sorted(
        [row for row in context_rows if row["static_hit"] and not row["lab_hit"]],
        key=lambda row: (-num(row["lab_regret"]), str(row["case_id"])),
    )[:top_n]
    lab_beats_static = sorted(
        [row for row in context_rows if row["lab_hit"] and not row["static_hit"]],
        key=lambda row: (-num(row["static_regret"]), str(row["case_id"])),
    )[:top_n]

    summary = {
        "cashout_report_version": cashout_report.get("report_version"),
        "label_report_version": label_report.get("report_version"),
        "case_count": len(cases),
        "matched_context_count": len(context_rows),
        "matched_case_count": len({row["case_id"] for row in context_rows}),
        "lab_hit_rate": pct(lab_hits, len(context_rows)),
        "static_hit_rate": pct(static_hits, len(context_rows)),
        "chosen_hit_rate": pct(chosen_hits, len(context_rows)),
        "avg_lab_regret": round(mean(lab_regrets), 3),
        "avg_static_regret": round(mean(static_regrets), 3),
        "avg_context_spearman": round(mean(spearman_values), 4),
        "pearson_lab_vs_rollout_value": pearson(candidate_pairs),
        "pearson_lab_vs_floor_delta": pearson(floor_pairs),
        "pearson_lab_vs_end_hp": pearson(hp_pairs),
        "pearson_lab_vs_reward_total": pearson(reward_pairs),
    }

    recommendations: list[str] = []
    if summary["lab_hit_rate"] <= summary["static_hit_rate"]:
        recommendations.append(
            "probabilistic lab is not yet beating static cashout on rollout winners; use it as diagnosis, not as a card-choice labeler."
        )
    elif summary["lab_hit_rate"] < summary["static_hit_rate"] + 3.0:
        recommendations.append(
            "probabilistic lab is only slightly ahead of static cashout; keep weak-override calibration and treat outputs as audit signals, not labels."
        )
    if summary["avg_context_spearman"] < 0.20:
        recommendations.append(
            "candidate score ordering has weak context-level correlation with rollout outcomes; prioritize fixing bucket EV/context rules before training."
        )
    if any(row["name"] in {"scaling_cashout", "draw_cashout", "exhaust"} and row["avg_lab_regret"] > 5 for row in aggregate_tables["by_dominant_cashout"]):
        recommendations.append(
            "draw/scaling/exhaust cashout still needs card-specific and encounter-specific calibration; do not widen labels until these miss buckets are understood."
        )
    if not recommendations:
        recommendations.append(
            "alignment looks usable for a tiny fixture set; inspect high-regret misses before creating comparator data."
        )

    return {
        "report_version": "probabilistic_cashout_rollout_alignment_v0",
        "summary": summary,
        "recommendations": recommendations,
        "aggregates": aggregate_tables,
        "high_regret_lab_misses": high_regret_misses,
        "static_beats_lab_cases": static_beats_lab,
        "lab_beats_static_cases": lab_beats_static,
        "context_rows": context_rows,
    }


def write_markdown(path: Path, analysis: dict[str, Any], *, top_n: int) -> None:
    summary = analysis["summary"]
    lines = [
        "# Probabilistic Cashout vs Rollout Alignment",
        "",
        "This report compares diagnostic cashout scores against policy/horizon rollout outcomes. It is not a teacher label export.",
        "",
        "## Summary",
        "",
        f"- cashout report: `{summary['cashout_report_version']}`",
        f"- rollout report: `{summary['label_report_version']}`",
        f"- matched cases: `{summary['matched_case_count']}`",
        f"- matched policy/horizon contexts: `{summary['matched_context_count']}`",
        f"- lab hit rate: `{summary['lab_hit_rate']}%`",
        f"- static hit rate: `{summary['static_hit_rate']}%`",
        f"- chosen hit rate: `{summary['chosen_hit_rate']}%`",
        f"- avg lab regret: `{summary['avg_lab_regret']}`",
        f"- avg static regret: `{summary['avg_static_regret']}`",
        f"- avg context Spearman: `{summary['avg_context_spearman']}`",
        f"- Pearson lab vs rollout value: `{summary['pearson_lab_vs_rollout_value']}`",
        f"- Pearson lab vs floor: `{summary['pearson_lab_vs_floor_delta']}`",
        f"- Pearson lab vs end HP: `{summary['pearson_lab_vs_end_hp']}`",
        f"- Pearson lab vs reward: `{summary['pearson_lab_vs_reward_total']}`",
        "",
        "## Recommendations",
        "",
    ]
    lines.extend(f"- {item}" for item in analysis["recommendations"])

    def table(title: str, rows: list[dict[str, Any]], limit: int = top_n) -> None:
        lines.extend(["", f"## {title}", "", "| bucket | n | lab hit | static hit | lab regret | static regret | spearman | statuses |", "|---|---:|---:|---:|---:|---:|---:|---|"])
        for row in rows[:limit]:
            lines.append(
                f"| {row['name']} | {row['total']} | {row['lab_hit_rate']}% | {row['static_hit_rate']}% | "
                f"{row['avg_lab_regret']} | {row['avg_static_regret']} | {row['avg_spearman']} | `{row['label_statuses']}` |"
            )

    aggregates = analysis["aggregates"]
    table("By Policy Horizon", aggregates["by_policy_horizon"])
    table("By Dominant Cashout", aggregates["by_dominant_cashout"])
    table("By Encounter Archetype", aggregates["by_archetype"])
    table("By Static Best Card", aggregates["by_static_best_card"])

    def case_table(title: str, rows: list[dict[str, Any]]) -> None:
        lines.extend(["", f"## {title}", "", "| case | context | static | lab | winner | label | lab regret | static regret | archetype |", "|---|---|---|---|---|---|---:|---:|---|"])
        for row in rows[:top_n]:
            lines.append(
                f"| {row['case_id']} | {row['continuation_policy']}@{row['horizon']} | {row['static_best_card']} | "
                f"{row['lab_best_card']} | {row['winner_card']} | {row['label_status']} | {row['lab_regret']} | "
                f"{row['static_regret']} | {row['dominant_archetype']} |"
            )

    case_table("High-Regret Lab Misses", analysis["high_regret_lab_misses"])
    case_table("Static Beats Lab", analysis["static_beats_lab_cases"])
    case_table("Lab Beats Static", analysis["lab_beats_static_cases"])

    real = resolve(path)
    real.parent.mkdir(parents=True, exist_ok=True)
    real.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    label_dir = resolve(args.label_dir)
    label_report_path = args.label_report or label_dir / "cashout_rollout_label_report.json"
    candidate_path = args.candidate_outcomes or label_dir / "candidate_outcomes.jsonl"
    pairwise_path = args.pairwise_labels or label_dir / "pairwise_labels.jsonl"
    out_path = resolve(args.out)
    markdown_path = resolve(args.markdown_out) if args.markdown_out else out_path.with_suffix(".md")

    cashout_report = read_json(args.cashout_report)
    label_report = read_json(label_report_path)
    candidate_rows = read_jsonl(candidate_path)
    pairwise_rows = read_jsonl(pairwise_path)

    analysis = build_analysis(
        cashout_report,
        label_report,
        candidate_rows,
        pairwise_rows,
        top_n=args.top_n,
    )
    write_json(out_path, analysis)
    write_markdown(markdown_path, analysis, top_n=args.top_n)
    print(
        json.dumps(
            {
                "out": str(out_path.relative_to(REPO_ROOT) if out_path.is_relative_to(REPO_ROOT) else out_path),
                "markdown_out": str(markdown_path.relative_to(REPO_ROOT) if markdown_path.is_relative_to(REPO_ROOT) else markdown_path),
                "summary": analysis["summary"],
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
