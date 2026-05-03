#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json


DEFAULT_LABEL_DIR = REPO_ROOT / "tools" / "artifacts" / "card_cashout_rollout_labels" / "v1"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Analyze cashout rollout labels by card, cashout class, label status, "
            "continuation policy, and horizon. This is a decision report for the "
            "next data/modeling step, not a training-data exporter."
        )
    )
    parser.add_argument("--label-dir", type=Path, default=DEFAULT_LABEL_DIR)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--candidate-outcomes", type=Path)
    parser.add_argument("--pairwise-labels", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--json-out", type=Path)
    parser.add_argument("--top-n", type=int, default=20)
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def iter_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def avg(values: list[float]) -> float:
    return sum(values) / len(values) if values else 0.0


def pct(numerator: int, denominator: int) -> float:
    return 100.0 * numerator / denominator if denominator else 0.0


def fmt_float(value: float, digits: int = 2) -> str:
    return f"{value:.{digits}f}"


@dataclass
class BucketStats:
    total: int = 0
    statuses: Counter[str] = field(default_factory=Counter)
    source_statuses: Counter[str] = field(default_factory=Counter)
    source_policies: Counter[str] = field(default_factory=Counter)
    chosen_cards: Counter[str] = field(default_factory=Counter)
    best_cards: Counter[str] = field(default_factory=Counter)
    cashout_kinds: Counter[str] = field(default_factory=Counter)
    gaps: list[float] = field(default_factory=list)

    def add_label(self, label: dict[str, Any]) -> None:
        source = label.get("source_case") or {}
        chosen = source.get("chosen") or {}
        best = source.get("best_by_cashout") or {}
        self.total += 1
        self.statuses[str(label.get("label_status") or "unknown")] += 1
        self.source_statuses[str(source.get("calibration_status") or "uncalibrated")] += 1
        self.source_policies[str(label.get("source_policy") or "unknown")] += 1
        self.chosen_cards[card_label(chosen)] += 1
        self.best_cards[card_label(best)] += 1
        for kind in source.get("cashout_kinds") or []:
            self.cashout_kinds[str(kind)] += 1
        dominant = str(best.get("dominant_cashout") or "")
        if dominant:
            self.cashout_kinds[dominant] += 1
        self.gaps.append(float(source.get("cashout_gap") or 0.0))


def card_label(candidate: dict[str, Any]) -> str:
    return str(candidate.get("card_id") or candidate.get("action_key") or "unknown")


def best_card(label: dict[str, Any]) -> str:
    return card_label(((label.get("source_case") or {}).get("best_by_cashout") or {}))


def chosen_card(label: dict[str, Any]) -> str:
    return card_label(((label.get("source_case") or {}).get("chosen") or {}))


def primary_class(label: dict[str, Any]) -> str:
    best = (label.get("source_case") or {}).get("best_by_cashout") or {}
    return str(best.get("primary_class") or "unknown")


def dominant_cashout(label: dict[str, Any]) -> str:
    best = (label.get("source_case") or {}).get("best_by_cashout") or {}
    return str(best.get("dominant_cashout") or "unknown")


def label_reason(label: dict[str, Any]) -> str:
    counts = label.get("verdict_counts") or {}
    return ", ".join(f"{key}:{value}" for key, value in sorted(counts.items()))


def aggregate_labels(labels: list[dict[str, Any]]) -> dict[str, Any]:
    by_best: dict[str, BucketStats] = defaultdict(BucketStats)
    by_chosen: dict[str, BucketStats] = defaultdict(BucketStats)
    by_primary: dict[str, BucketStats] = defaultdict(BucketStats)
    by_dominant: dict[str, BucketStats] = defaultdict(BucketStats)
    by_pair: dict[str, BucketStats] = defaultdict(BucketStats)
    by_status: dict[str, BucketStats] = defaultdict(BucketStats)

    for label in labels:
        by_best[best_card(label)].add_label(label)
        by_chosen[chosen_card(label)].add_label(label)
        by_primary[primary_class(label)].add_label(label)
        by_dominant[dominant_cashout(label)].add_label(label)
        by_pair[f"{chosen_card(label)} -> {best_card(label)}"].add_label(label)
        by_status[str(label.get("label_status") or "unknown")].add_label(label)

    return {
        "by_best_card": bucket_table(by_best),
        "by_chosen_card": bucket_table(by_chosen),
        "by_primary_class": bucket_table(by_primary),
        "by_dominant_cashout": bucket_table(by_dominant),
        "by_chosen_to_best_pair": bucket_table(by_pair),
        "by_label_status": bucket_table(by_status),
    }


def bucket_table(buckets: dict[str, BucketStats]) -> list[dict[str, Any]]:
    rows = []
    for name, stats in buckets.items():
        rows.append(
            {
                "name": name,
                "total": stats.total,
                "statuses": dict(sorted(stats.statuses.items())),
                "robust_confirmed": stats.statuses.get("robust_confirmed", 0),
                "requires_cashout_policy": stats.statuses.get("requires_cashout_policy", 0),
                "refuted": stats.statuses.get("rollout_refuted", 0),
                "unstable": stats.statuses.get("rollout_unstable", 0),
                "equivalent": stats.statuses.get("rollout_equivalent", 0),
                "confirmed_rate": pct(
                    stats.statuses.get("robust_confirmed", 0)
                    + stats.statuses.get("rollout_confirmed", 0)
                    + stats.statuses.get("requires_cashout_policy", 0),
                    stats.total,
                ),
                "bad_static_rate": pct(
                    stats.statuses.get("rollout_refuted", 0)
                    + stats.statuses.get("rollout_unstable", 0),
                    stats.total,
                ),
                "avg_cashout_gap": avg(stats.gaps),
                "source_statuses": dict(sorted(stats.source_statuses.items())),
                "source_policies": dict(sorted(stats.source_policies.items())),
                "top_chosen_cards": stats.chosen_cards.most_common(5),
                "top_best_cards": stats.best_cards.most_common(5),
                "top_cashout_kinds": stats.cashout_kinds.most_common(5),
            }
        )
    return sorted(
        rows,
        key=lambda row: (
            -int(row["total"]),
            -int(row["robust_confirmed"]),
            -float(row["bad_static_rate"]),
            str(row["name"]),
        ),
    )


def aggregate_observations(labels: list[dict[str, Any]]) -> dict[str, Any]:
    by_policy_horizon: dict[str, Counter[str]] = defaultdict(Counter)
    by_policy: dict[str, Counter[str]] = defaultdict(Counter)
    by_horizon: dict[str, Counter[str]] = defaultdict(Counter)
    disagreement_cases = []
    for label in labels:
        verdicts = []
        for obs in label.get("observations") or []:
            policy = str(obs.get("continuation_policy") or "unknown")
            horizon = str(obs.get("horizon") or "unknown")
            verdict = str((obs.get("classification") or {}).get("verdict") or obs.get("status") or "unknown")
            by_policy_horizon[f"{policy}@{horizon}"][verdict] += 1
            by_policy[policy][verdict] += 1
            by_horizon[horizon][verdict] += 1
            verdicts.append(verdict)
        non_equivalent = {verdict for verdict in verdicts if verdict not in {"rollout_equivalent", "inconclusive"}}
        if len(non_equivalent) > 1:
            disagreement_cases.append(label_summary(label))
    return {
        "by_policy_horizon": counter_table(by_policy_horizon),
        "by_policy": counter_table(by_policy),
        "by_horizon": counter_table(by_horizon),
        "disagreement_cases": disagreement_cases,
    }


def counter_table(buckets: dict[str, Counter[str]]) -> list[dict[str, Any]]:
    rows = []
    for name, counter in buckets.items():
        total = sum(counter.values())
        rows.append(
            {
                "name": name,
                "total": total,
                "counts": dict(sorted(counter.items())),
                "confirmed": counter.get("rollout_confirmed", 0),
                "refuted": counter.get("rollout_refuted", 0),
                "equivalent": counter.get("rollout_equivalent", 0),
                "confirmed_rate": pct(counter.get("rollout_confirmed", 0), total),
                "refuted_rate": pct(counter.get("rollout_refuted", 0), total),
            }
        )
    return sorted(rows, key=lambda row: (-int(row["total"]), str(row["name"])))


def label_summary(label: dict[str, Any]) -> dict[str, Any]:
    source = label.get("source_case") or {}
    return {
        "case_id": label.get("case_id"),
        "label_status": label.get("label_status"),
        "source_policy": label.get("source_policy"),
        "seed": source.get("seed"),
        "step_index": source.get("step_index"),
        "floor": source.get("floor"),
        "chosen": chosen_card(label),
        "best": best_card(label),
        "primary_class": primary_class(label),
        "dominant_cashout": dominant_cashout(label),
        "cashout_gap": float(source.get("cashout_gap") or 0.0),
        "verdict_counts": label.get("verdict_counts") or {},
        "reason": label_reason(label),
    }


def aggregate_pairwise(rows: list[dict[str, Any]]) -> dict[str, Any]:
    card_pref: dict[str, Counter[str]] = defaultdict(Counter)
    card_reject: dict[str, Counter[str]] = defaultdict(Counter)
    reason_counts: Counter[str] = Counter()
    policy_horizon_reasons: dict[str, Counter[str]] = defaultdict(Counter)
    for row in rows:
        preferred = (row.get("preferred_outcome") or {}).get("card_id") or row.get("preferred_key") or "unknown"
        rejected = (row.get("rejected_outcome") or {}).get("card_id") or row.get("rejected_key") or "unknown"
        reason = str(row.get("reason") or "unknown")
        key = f"{row.get('continuation_policy')}@{row.get('horizon')}"
        card_pref[str(preferred)][reason] += 1
        card_reject[str(rejected)][reason] += 1
        reason_counts[reason] += 1
        policy_horizon_reasons[key][reason] += 1

    cards = sorted(set(card_pref) | set(card_reject))
    card_rows = []
    for card in cards:
        pref_total = sum(card_pref[card].values())
        reject_total = sum(card_reject[card].values())
        total = pref_total + reject_total
        card_rows.append(
            {
                "card": card,
                "preferred": pref_total,
                "rejected": reject_total,
                "net": pref_total - reject_total,
                "preference_rate": pct(pref_total, total),
                "preferred_reasons": dict(card_pref[card].most_common(5)),
                "rejected_reasons": dict(card_reject[card].most_common(5)),
            }
        )
    return {
        "card_pairwise": sorted(card_rows, key=lambda row: (-abs(int(row["net"])), str(row["card"]))),
        "reason_counts": dict(reason_counts.most_common()),
        "policy_horizon_reasons": {
            key: dict(counter.most_common()) for key, counter in sorted(policy_horizon_reasons.items())
        },
    }


def aggregate_candidate_outcomes(rows: list[dict[str, Any]]) -> dict[str, Any]:
    buckets: dict[str, list[dict[str, Any]]] = defaultdict(list)
    by_policy_horizon: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        candidate = row.get("candidate") or {}
        card = str(candidate.get("card_id") or candidate.get("candidate_key") or "unknown")
        buckets[card].append(candidate)
        by_policy_horizon[f"{row.get('continuation_policy')}@{row.get('horizon')}"].append(candidate)
    return {
        "by_card": outcome_table(buckets),
        "by_policy_horizon": outcome_table(by_policy_horizon),
    }


def outcome_table(buckets: dict[str, list[dict[str, Any]]]) -> list[dict[str, Any]]:
    rows = []
    for name, values in buckets.items():
        rows.append(
            {
                "name": name,
                "count": len(values),
                "avg_floor_delta": avg([float(row.get("floor_delta") or 0.0) for row in values]),
                "avg_combat_win_delta": avg([float(row.get("combat_win_delta") or 0.0) for row in values]),
                "avg_end_hp": avg([float(row.get("end_hp") or 0.0) for row in values]),
                "avg_reward_total": avg([float(row.get("reward_total") or 0.0) for row in values]),
                "defeat_count": sum(1 for row in values if str(row.get("end_result") or "") == "defeat"),
            }
        )
    return sorted(rows, key=lambda row: (-int(row["count"]), -float(row["avg_floor_delta"]), str(row["name"])))


def recommendations(analysis: dict[str, Any]) -> list[str]:
    summary = analysis.get("rollout_summary") or {}
    labels = summary.get("label_status_counts") or {}
    robust = int(labels.get("robust_confirmed", 0))
    refuted = int(labels.get("rollout_refuted", 0))
    unstable = int(labels.get("rollout_unstable", 0))
    equivalent = int(labels.get("rollout_equivalent", 0))
    total = int(summary.get("case_count") or sum(int(v) for v in labels.values()))
    out = []

    if refuted + unstable > robust:
        out.append(
            "修 cashout 静态模型优先：refuted+unstable 明显多于 robust_confirmed，静态 cashout 仍在系统性过度乐观。"
        )
    if robust < 20:
        out.append(
            "暂时不要训练正式 comparator：robust_confirmed 数量太少，只适合做 smoke/fixture，不适合主训练集。"
        )
    if total >= 20 and robust > 0:
        out.append(
            "可以扩大 rollout 到 100 cases，但目的应是定位静态模型错误分布，而不是马上堆训练标签。"
        )
    if equivalent > 0:
        out.append(
            "保留 equivalent 为校准样本：这些 case 可以帮助设置 margin/不确定性，不应转成偏好标签。"
        )
    policy_rows = analysis.get("observation_aggregates", {}).get("by_policy") or []
    plan_row = next((row for row in policy_rows if row.get("name") == "plan_query_v0"), None)
    rule_row = next((row for row in policy_rows if row.get("name") == "rule_baseline_v0"), None)
    if plan_row and rule_row:
        if float(plan_row.get("confirmed_rate") or 0.0) > float(rule_row.get("confirmed_rate") or 0.0):
            out.append(
                "继续保留 plan_query_v0 作为 continuation 对照：它确认率高于 rule_baseline，但仍需按 case 检查是否只是 horizon/RNG 偶然。"
            )
        else:
            out.append(
                "不要假设 plan_query_v0 更聪明：当前聚合确认率未明显超过 rule_baseline，应先分析 requires_cashout_policy 个案。"
            )
    return out


def build_analysis(report: dict[str, Any], candidate_rows: list[dict[str, Any]], pairwise_rows: list[dict[str, Any]]) -> dict[str, Any]:
    labels = list(report.get("labels") or [])
    analysis = {
        "source_report_version": report.get("report_version"),
        "source_generated_at_utc": report.get("generated_at_utc"),
        "rollout_summary": report.get("summary") or {},
        "limitations": report.get("limitations") or [],
        "label_aggregates": aggregate_labels(labels),
        "observation_aggregates": aggregate_observations(labels),
        "pairwise_aggregates": aggregate_pairwise(pairwise_rows),
        "candidate_outcome_aggregates": aggregate_candidate_outcomes(candidate_rows),
        "robust_confirmed_cases": [
            label_summary(label) for label in labels if label.get("label_status") == "robust_confirmed"
        ],
        "requires_cashout_policy_cases": [
            label_summary(label) for label in labels if label.get("label_status") == "requires_cashout_policy"
        ],
        "refuted_cases": [
            label_summary(label) for label in labels if label.get("label_status") == "rollout_refuted"
        ],
        "unstable_cases": [
            label_summary(label) for label in labels if label.get("label_status") == "rollout_unstable"
        ],
    }
    analysis["recommendations"] = recommendations(analysis)
    return analysis


def md_table(rows: list[dict[str, Any]], columns: list[tuple[str, str]], *, limit: int | None = None) -> list[str]:
    shown = rows[:limit] if limit is not None else rows
    if not shown:
        return ["_none_"]
    lines = [
        "| " + " | ".join(label for label, _ in columns) + " |",
        "| " + " | ".join("---" for _ in columns) + " |",
    ]
    for row in shown:
        values = []
        for _, key in columns:
            value = row.get(key)
            if isinstance(value, float):
                values.append(fmt_float(value))
            elif isinstance(value, (dict, list, tuple)):
                values.append("`" + str(value).replace("|", "/") + "`")
            else:
                values.append(str(value))
        lines.append("| " + " | ".join(values) + " |")
    return lines


def write_markdown(path: Path, analysis: dict[str, Any], top_n: int) -> None:
    summary = analysis.get("rollout_summary") or {}
    lines = [
        "# Cashout Rollout Label Analysis",
        "",
        f"- source version: `{analysis.get('source_report_version')}`",
        f"- source generated: `{analysis.get('source_generated_at_utc')}`",
        f"- cases: `{summary.get('case_count')}`",
        f"- label counts: `{summary.get('label_status_counts')}`",
        f"- candidate outcomes: `{summary.get('candidate_outcome_row_count')}`",
        f"- pairwise labels: `{summary.get('pairwise_label_count')}`",
        "",
        "## Decision",
        "",
    ]
    for recommendation in analysis.get("recommendations") or []:
        lines.append(f"- {recommendation}")
    lines.extend(["", "## Label Status By Cashout Best Card", ""])
    lines.extend(
        md_table(
            analysis["label_aggregates"]["by_best_card"],
            [
                ("card", "name"),
                ("n", "total"),
                ("statuses", "statuses"),
                ("confirmed %", "confirmed_rate"),
                ("bad static %", "bad_static_rate"),
                ("avg gap", "avg_cashout_gap"),
                ("kinds", "top_cashout_kinds"),
            ],
            limit=top_n,
        )
    )
    lines.extend(["", "## Label Status By Primary Class", ""])
    lines.extend(
        md_table(
            analysis["label_aggregates"]["by_primary_class"],
            [
                ("class", "name"),
                ("n", "total"),
                ("statuses", "statuses"),
                ("confirmed %", "confirmed_rate"),
                ("bad static %", "bad_static_rate"),
                ("avg gap", "avg_cashout_gap"),
                ("best cards", "top_best_cards"),
            ],
            limit=top_n,
        )
    )
    lines.extend(["", "## Continuation Policy / Horizon", ""])
    lines.extend(
        md_table(
            analysis["observation_aggregates"]["by_policy_horizon"],
            [
                ("policy@h", "name"),
                ("n", "total"),
                ("counts", "counts"),
                ("confirmed %", "confirmed_rate"),
                ("refuted %", "refuted_rate"),
            ],
        )
    )
    lines.extend(["", "## Pairwise Card Net", ""])
    lines.extend(
        md_table(
            analysis["pairwise_aggregates"]["card_pairwise"],
            [
                ("card", "card"),
                ("preferred", "preferred"),
                ("rejected", "rejected"),
                ("net", "net"),
                ("pref %", "preference_rate"),
                ("pref reasons", "preferred_reasons"),
                ("reject reasons", "rejected_reasons"),
            ],
            limit=top_n,
        )
    )
    lines.extend(["", "## Robust Confirmed Cases", ""])
    lines.extend(
        md_table(
            analysis["robust_confirmed_cases"],
            [
                ("case", "case_id"),
                ("chosen", "chosen"),
                ("best", "best"),
                ("class", "primary_class"),
                ("cashout", "dominant_cashout"),
                ("gap", "cashout_gap"),
                ("verdicts", "verdict_counts"),
            ],
        )
    )
    lines.extend(["", "## Requires Cashout Policy Cases", ""])
    lines.extend(
        md_table(
            analysis["requires_cashout_policy_cases"],
            [
                ("case", "case_id"),
                ("chosen", "chosen"),
                ("best", "best"),
                ("class", "primary_class"),
                ("cashout", "dominant_cashout"),
                ("gap", "cashout_gap"),
                ("verdicts", "verdict_counts"),
            ],
        )
    )
    lines.extend(["", "## Refuted Cases", ""])
    lines.extend(
        md_table(
            analysis["refuted_cases"],
            [
                ("case", "case_id"),
                ("chosen", "chosen"),
                ("best", "best"),
                ("class", "primary_class"),
                ("cashout", "dominant_cashout"),
                ("gap", "cashout_gap"),
                ("verdicts", "verdict_counts"),
            ],
            limit=top_n,
        )
    )
    lines.extend(["", "## Unstable Cases", ""])
    lines.extend(
        md_table(
            analysis["unstable_cases"],
            [
                ("case", "case_id"),
                ("chosen", "chosen"),
                ("best", "best"),
                ("class", "primary_class"),
                ("cashout", "dominant_cashout"),
                ("gap", "cashout_gap"),
                ("verdicts", "verdict_counts"),
            ],
            limit=top_n,
        )
    )
    lines.extend(["", "## Limitations", ""])
    for limitation in analysis.get("limitations") or []:
        lines.append(f"- {limitation}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    args = parse_args()
    label_dir = args.label_dir
    report_path = args.report or label_dir / "cashout_rollout_label_report.json"
    candidate_path = args.candidate_outcomes or label_dir / "candidate_outcomes.jsonl"
    pairwise_path = args.pairwise_labels or label_dir / "pairwise_labels.jsonl"
    out_path = args.out or label_dir / "cashout_rollout_label_analysis.md"
    json_out_path = args.json_out or label_dir / "cashout_rollout_label_analysis.json"

    report = read_json(report_path)
    candidate_rows = iter_jsonl(candidate_path)
    pairwise_rows = iter_jsonl(pairwise_path)
    analysis = build_analysis(report, candidate_rows, pairwise_rows)
    write_json(json_out_path, analysis)
    write_markdown(out_path, analysis, top_n=max(args.top_n, 1))


if __name__ == "__main__":
    main()
