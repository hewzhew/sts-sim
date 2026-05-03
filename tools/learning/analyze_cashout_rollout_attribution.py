#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json


DEFAULT_LABEL_DIR = REPO_ROOT / "tools" / "artifacts" / "card_cashout_rollout_labels" / "v1_1_attribution_10case"


ATTR_FIELDS = [
    "hp_loss_observed",
    "monster_hp_reduction_observed",
    "alive_monster_reduction_observed",
    "combat_turns_observed",
    "combat_play_card_count",
    "energy_unused_on_end_turn_total",
    "draw_pile_decrease_observed",
    "exhaust_count_increase_observed",
    "discard_count_increase_observed",
    "max_visible_incoming_damage",
    "max_visible_unblocked_damage",
    "scaling_played_delta",
    "draw_played_delta",
    "exhaust_played_delta",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Aggregate Rollout Attribution V0 fields from cashout rollout labels. "
            "This explains pairwise rollout wins with observed combat/run deltas; "
            "it is not exact engine event attribution."
        )
    )
    parser.add_argument("--label-dir", type=Path, default=DEFAULT_LABEL_DIR)
    parser.add_argument("--candidate-outcomes", type=Path)
    parser.add_argument("--pairwise-labels", type=Path)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--markdown-out", type=Path)
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


def num(value: Any) -> float:
    try:
        return float(value or 0.0)
    except (TypeError, ValueError):
        return 0.0


@dataclass
class AttrBucket:
    total: int = 0
    sums: dict[str, float] = field(default_factory=lambda: defaultdict(float))

    def add(self, attr: dict[str, Any]) -> None:
        self.total += 1
        for field in ATTR_FIELDS:
            self.sums[field] += num(attr.get(field))

    def row(self, name: str) -> dict[str, Any]:
        return {
            "name": name,
            "total": self.total,
            "avg": {
                field: round(self.sums[field] / self.total, 3) if self.total else 0.0
                for field in ATTR_FIELDS
            },
        }


def card_label(outcome: dict[str, Any]) -> str:
    return str(outcome.get("card_id") or outcome.get("candidate_key") or "unknown")


def build_analysis(
    report: dict[str, Any],
    candidate_rows: list[dict[str, Any]],
    pairwise_rows: list[dict[str, Any]],
    *,
    top_n: int,
) -> dict[str, Any]:
    by_reason: dict[str, AttrBucket] = defaultdict(AttrBucket)
    by_policy_horizon: dict[str, AttrBucket] = defaultdict(AttrBucket)
    by_preferred_card: dict[str, AttrBucket] = defaultdict(AttrBucket)
    by_rejected_card: dict[str, AttrBucket] = defaultdict(AttrBucket)
    by_pair: dict[str, AttrBucket] = defaultdict(AttrBucket)
    high_hp_loss_trades: list[dict[str, Any]] = []
    high_progress_trades: list[dict[str, Any]] = []

    for row in pairwise_rows:
        diff = row.get("outcome_diff_preferred_minus_rejected") or {}
        attr = diff.get("attribution") or {}
        if not attr:
            continue
        reason = str(row.get("reason") or "unknown")
        preferred = row.get("preferred_outcome") or {}
        rejected = row.get("rejected_outcome") or {}
        preferred_card = card_label(preferred)
        rejected_card = card_label(rejected)
        policy_horizon = f"{row.get('continuation_policy')}@{row.get('horizon')}"
        pair_name = f"{preferred_card} > {rejected_card}"
        for table, key in [
            (by_reason, reason),
            (by_policy_horizon, policy_horizon),
            (by_preferred_card, preferred_card),
            (by_rejected_card, rejected_card),
            (by_pair, pair_name),
        ]:
            table[key].add(attr)
        compact = {
            "case_id": row.get("case_id"),
            "policy_horizon": policy_horizon,
            "reason": reason,
            "preferred": preferred_card,
            "rejected": rejected_card,
            "outcome_diff": diff,
        }
        if num(attr.get("hp_loss_observed")) >= 20:
            high_hp_loss_trades.append(compact)
        if num(attr.get("monster_hp_reduction_observed")) >= 80 or num(attr.get("alive_monster_reduction_observed")) >= 2:
            high_progress_trades.append(compact)

    candidate_by_card: dict[str, AttrBucket] = defaultdict(AttrBucket)
    candidate_by_policy: dict[str, AttrBucket] = defaultdict(AttrBucket)
    for row in candidate_rows:
        candidate = row.get("candidate") or {}
        attr = candidate.get("attribution") or {}
        if not attr:
            continue
        candidate_by_card[card_label(candidate)].add(attr)
        candidate_by_policy[f"{row.get('continuation_policy')}@{row.get('horizon')}"].add(attr)

    def table(bucket: dict[str, AttrBucket]) -> list[dict[str, Any]]:
        return sorted(
            [value.row(key) for key, value in bucket.items()],
            key=lambda row: (-int(row["total"]), str(row["name"])),
        )

    return {
        "report_version": "cashout_rollout_attribution_analysis_v0",
        "source_report_version": report.get("report_version"),
        "summary": {
            "label_case_count": (report.get("summary") or {}).get("case_count"),
            "candidate_rows": len(candidate_rows),
            "pairwise_rows": len(pairwise_rows),
            "pairwise_with_attribution": sum(
                1
                for row in pairwise_rows
                if ((row.get("outcome_diff_preferred_minus_rejected") or {}).get("attribution") or {})
            ),
            "observability": "before/after observation deltas, not exact engine events",
        },
        "aggregates": {
            "pairwise_by_reason": table(by_reason),
            "pairwise_by_policy_horizon": table(by_policy_horizon),
            "pairwise_by_preferred_card": table(by_preferred_card),
            "pairwise_by_rejected_card": table(by_rejected_card),
            "pairwise_by_card_pair": table(by_pair),
            "candidate_by_card": table(candidate_by_card),
            "candidate_by_policy_horizon": table(candidate_by_policy),
        },
        "high_hp_loss_trades": sorted(
            high_hp_loss_trades,
            key=lambda row: -num(((row.get("outcome_diff") or {}).get("attribution") or {}).get("hp_loss_observed")),
        )[:top_n],
        "high_progress_trades": sorted(
            high_progress_trades,
            key=lambda row: -num(((row.get("outcome_diff") or {}).get("attribution") or {}).get("monster_hp_reduction_observed")),
        )[:top_n],
    }


def write_markdown(path: Path, analysis: dict[str, Any], *, top_n: int) -> None:
    summary = analysis["summary"]
    lines = [
        "# Cashout Rollout Attribution Analysis",
        "",
        "This report explains pairwise rollout preferences using Rollout Attribution V0 observation deltas.",
        "",
        "## Summary",
        "",
        f"- source report: `{analysis['source_report_version']}`",
        f"- label cases: `{summary['label_case_count']}`",
        f"- candidate rows: `{summary['candidate_rows']}`",
        f"- pairwise rows: `{summary['pairwise_rows']}`",
        f"- pairwise with attribution: `{summary['pairwise_with_attribution']}`",
        f"- observability: `{summary['observability']}`",
        "",
    ]

    def add_table(title: str, rows: list[dict[str, Any]]) -> None:
        lines.extend(
            [
                f"## {title}",
                "",
                "| bucket | n | hp loss | monster hp | kills | turns | plays | unused energy | draw delta | exhaust delta |",
                "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
            ]
        )
        for row in rows[:top_n]:
            avg = row["avg"]
            lines.append(
                "| {name} | {total} | {hp} | {monster} | {kills} | {turns} | {plays} | {energy} | {draw} | {exhaust} |".format(
                    name=row["name"],
                    total=row["total"],
                    hp=avg["hp_loss_observed"],
                    monster=avg["monster_hp_reduction_observed"],
                    kills=avg["alive_monster_reduction_observed"],
                    turns=avg["combat_turns_observed"],
                    plays=avg["combat_play_card_count"],
                    energy=avg["energy_unused_on_end_turn_total"],
                    draw=avg["draw_pile_decrease_observed"],
                    exhaust=avg["exhaust_count_increase_observed"],
                )
            )
        lines.append("")

    aggregates = analysis["aggregates"]
    add_table("Pairwise By Reason", aggregates["pairwise_by_reason"])
    add_table("Pairwise By Preferred Card", aggregates["pairwise_by_preferred_card"])
    add_table("Candidate By Card", aggregates["candidate_by_card"])

    def add_cases(title: str, rows: list[dict[str, Any]]) -> None:
        lines.extend(["", f"## {title}", "", "| case | context | reason | preferred | rejected | attr diff |", "|---|---|---|---|---|---|"])
        for row in rows[:top_n]:
            attr = ((row.get("outcome_diff") or {}).get("attribution") or {})
            lines.append(
                f"| {row.get('case_id')} | {row.get('policy_horizon')} | {row.get('reason')} | "
                f"{row.get('preferred')} | {row.get('rejected')} | `{attr}` |"
            )

    add_cases("High HP-Loss Trades", analysis["high_hp_loss_trades"])
    add_cases("High Progress Trades", analysis["high_progress_trades"])

    real = resolve(path)
    real.parent.mkdir(parents=True, exist_ok=True)
    real.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    label_dir = resolve(args.label_dir)
    report_path = args.report or label_dir / "cashout_rollout_label_report.json"
    candidate_path = args.candidate_outcomes or label_dir / "candidate_outcomes.jsonl"
    pairwise_path = args.pairwise_labels or label_dir / "pairwise_labels.jsonl"
    out_path = resolve(args.out) if args.out else label_dir / "cashout_rollout_attribution_analysis.json"
    markdown_path = resolve(args.markdown_out) if args.markdown_out else out_path.with_suffix(".md")

    report = read_json(report_path)
    candidate_rows = read_jsonl(candidate_path)
    pairwise_rows = read_jsonl(pairwise_path)
    analysis = build_analysis(report, candidate_rows, pairwise_rows, top_n=args.top_n)
    write_json(out_path, analysis)
    write_markdown(markdown_path, analysis, top_n=args.top_n)
    print(
        json.dumps(
            {
                "out": str(out_path),
                "markdown_out": str(markdown_path),
                "summary": analysis["summary"],
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
