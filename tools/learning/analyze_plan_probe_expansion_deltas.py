#!/usr/bin/env python3
"""Analyze plan-query changes caused by expansion reruns.

This consumes `audit_combat_plan_probe_compression.py` JSON output. It is a
diagnostic report: it does not judge policy quality, it only explains which
query answers changed when a budget-pruned case was rerun with a larger node
budget.
"""
from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from combat_reranker_common import write_json

REPO_ROOT = Path(__file__).resolve().parents[2]
REPORT_VERSION = "plan_probe_expansion_delta_analysis_v0"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Analyze plan-query deltas from expansion reruns.")
    parser.add_argument("--audit-report", type=Path, required=True)
    parser.add_argument(
        "--out",
        type=Path,
        help="Output JSON path. Markdown is written next to it with .md suffix.",
    )
    return parser.parse_args()


def resolve_path(path: Path) -> Path:
    return path if path.is_absolute() else REPO_ROOT / path


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def numeric_delta(before: dict[str, Any], after: dict[str, Any], field: str) -> int | float | None:
    old = before.get("outcome", {}).get(field)
    new = after.get("outcome", {}).get(field)
    if isinstance(old, (int, float)) and isinstance(new, (int, float)):
        return new - old
    return None


def bool_transition(before: dict[str, Any], after: dict[str, Any], field: str) -> str | None:
    old = before.get("outcome", {}).get(field)
    new = after.get("outcome", {}).get(field)
    if isinstance(old, bool) and isinstance(new, bool) and old != new:
        return f"{str(old).lower()}->{str(new).lower()}"
    return None


def action_card_ids(action_keys: list[str]) -> list[str]:
    cards = []
    for key in action_keys:
        for part in str(key).split("/"):
            if part.startswith("card:"):
                cards.append(part.removeprefix("card:"))
                break
    return cards


def mechanism_tags(cards: list[str]) -> list[str]:
    tags = set()
    for card in cards:
        if card in {"BattleTrance", "PommelStrike", "ShrugItOff", "Offering", "Acrobatics", "Backflip"}:
            tags.add("draw")
        if card in {"SecretTechnique", "SecretWeapon"}:
            tags.add("search")
        if card in {"Inflame", "DemonForm", "Metallicize", "DarkEmbrace", "FeelNoPain"}:
            tags.add("setup_or_scaling")
        if card in {"WildStrike", "PowerThrough", "Immolate", "Anger"}:
            tags.add("zone_mutation")
        if card in {"Bash", "ThunderClap", "Uppercut", "Shockwave"}:
            tags.add("debuff_ordering")
    return sorted(tags)


def classify_delta(delta: dict[str, Any]) -> str:
    before = delta.get("before") or {}
    after = delta.get("after") or {}
    if before.get("status") != after.get("status"):
        return "status_changed"
    if numeric_delta(before, after, "damage_done") not in (None, 0):
        return "damage_improved"
    if bool_transition(before, after, "played_setup_or_scaling"):
        return "setup_changed"
    if before.get("needs_deeper_search") != after.get("needs_deeper_search"):
        return "confidence_changed"
    if before.get("best_action_keys") != after.get("best_action_keys"):
        return "line_changed"
    return "other"


def delta_summary(case: dict[str, Any], delta: dict[str, Any]) -> dict[str, Any]:
    before = delta.get("before") or {}
    after = delta.get("after") or {}
    before_actions = before.get("best_action_keys") or []
    after_actions = after.get("best_action_keys") or []
    before_cards = action_card_ids(before_actions)
    after_cards = action_card_ids(after_actions)
    new_cards = [card for card in after_cards if card not in before_cards]
    all_cards = sorted(set(before_cards + after_cards + list(case.get("expansion_rerun_cards") or [])))
    return {
        "case_id": case.get("case_id"),
        "seed": case.get("seed"),
        "step_index": case.get("step_index"),
        "act": case.get("act"),
        "floor": case.get("floor"),
        "pressure_class": case.get("pressure_class"),
        "query_name": delta.get("query_name"),
        "delta_kind": classify_delta(delta),
        "changed_fields": delta.get("changed_fields") or [],
        "trigger_cards": case.get("expansion_rerun_cards") or [],
        "before_cards": before_cards,
        "after_cards": after_cards,
        "new_after_cards": new_cards,
        "mechanism_tags": mechanism_tags(all_cards),
        "damage_delta": numeric_delta(before, after, "damage_done"),
        "unblocked_delta": numeric_delta(before, after, "projected_unblocked_damage"),
        "remaining_energy_delta": numeric_delta(before, after, "remaining_energy"),
        "total_monster_hp_delta": numeric_delta(before, after, "total_monster_hp"),
        "setup_transition": bool_transition(before, after, "played_setup_or_scaling"),
        "before_status": before.get("status"),
        "after_status": after.get("status"),
        "before_needs_deeper_search": before.get("needs_deeper_search"),
        "after_needs_deeper_search": after.get("needs_deeper_search"),
        "before_actions": before_actions,
        "after_actions": after_actions,
        "report_path": case.get("report_path"),
        "rerun_report_path": case.get("expansion_rerun_report_path"),
    }


def build_report(audit: dict[str, Any], audit_path: Path) -> dict[str, Any]:
    rows = []
    for case in audit.get("cases") or []:
        if not case.get("expansion_rerun_triggered"):
            continue
        for delta in case.get("expansion_rerun_query_deltas") or []:
            rows.append(delta_summary(case, delta))

    query_counts = Counter(row["query_name"] for row in rows)
    kind_counts = Counter(row["delta_kind"] for row in rows)
    mechanism_counts = Counter(tag for row in rows for tag in row["mechanism_tags"])
    card_counts = Counter(card for row in rows for card in row["trigger_cards"])
    pressure_counts = Counter(row["pressure_class"] for row in rows)

    actionable = []
    for row in rows:
        if row["delta_kind"] in {"status_changed", "damage_improved", "setup_changed"}:
            actionable.append(row)

    return {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "audit_report": str(audit_path),
        "summary": {
            "cases": len(audit.get("cases") or []),
            "expansion_rerun_cases": sum(1 for case in audit.get("cases") or [] if case.get("expansion_rerun_triggered")),
            "delta_rows": len(rows),
            "actionable_delta_rows": len(actionable),
            "query_counts": dict(query_counts),
            "delta_kind_counts": dict(kind_counts),
            "mechanism_counts": dict(mechanism_counts),
            "trigger_card_counts": dict(card_counts),
            "pressure_counts": dict(pressure_counts),
        },
        "deltas": rows,
        "actionable_deltas": actionable,
    }


def table(lines: list[str], title: str, rows: list[tuple[Any, Any]], left: str, right: str) -> None:
    lines.extend(["", f"## {title}", ""])
    if not rows:
        lines.append("_none_")
        return
    lines.append(f"| {left} | {right} |")
    lines.append("| --- | ---: |")
    for key, value in rows:
        lines.append(f"| `{key}` | {value} |")


def markdown_report(report: dict[str, Any]) -> str:
    summary = report["summary"]
    lines = [
        "# Plan Probe Expansion Delta Analysis",
        "",
        "This report explains which plan queries changed when budget-pruned cases were rerun with a larger node budget.",
        "",
        "## Summary",
        "",
    ]
    for key in [
        "cases",
        "expansion_rerun_cases",
        "delta_rows",
        "actionable_delta_rows",
    ]:
        lines.append(f"- {key}: `{summary.get(key)}`")

    table(lines, "Changed Queries", sorted(summary["query_counts"].items()), "query", "n")
    table(lines, "Delta Kinds", sorted(summary["delta_kind_counts"].items()), "kind", "n")
    table(lines, "Mechanisms", sorted(summary["mechanism_counts"].items()), "mechanism", "n")
    table(lines, "Trigger Cards", sorted(summary["trigger_card_counts"].items()), "card", "n")
    table(lines, "Pressure Classes", sorted(summary["pressure_counts"].items()), "pressure", "n")

    lines.extend(["", "## Actionable Deltas", ""])
    if not report["actionable_deltas"]:
        lines.append("_none_")
    else:
        lines.append("| case | query | kind | mechanism | damage Δ | setup | before | after | reports |")
        lines.append("| --- | --- | --- | --- | ---: | --- | --- | --- | --- |")
        for row in report["actionable_deltas"]:
            lines.append(
                f"| `{row['case_id']}` | `{row['query_name']}` | `{row['delta_kind']}` | "
                f"`{','.join(row['mechanism_tags']) or 'unknown'}` | "
                f"{row['damage_delta'] if row['damage_delta'] is not None else ''} | "
                f"`{row['setup_transition'] or ''}` | "
                f"`{' -> '.join(row['before_cards'])}` | "
                f"`{' -> '.join(row['after_cards'])}` | "
                f"`{row['report_path']}` / `{row['rerun_report_path']}` |"
            )

    lines.extend(
        [
            "",
            "## Interpretation",
            "",
            "- `status_changed` means the larger budget changed feasibility, which is the highest priority.",
            "- `damage_improved` means the same query stayed feasible/partial but found a better line.",
            "- `setup_changed` means expansion exposed a setup/scaling line that the base budget missed.",
            "- `confidence_changed` usually means the rerun cleared budget-prune uncertainty without changing the best line.",
        ]
    )
    return "\n".join(lines) + "\n"


def main() -> None:
    args = parse_args()
    audit_path = resolve_path(args.audit_report)
    audit = read_json(audit_path)
    report = build_report(audit, audit_path)
    if args.out:
        out_path = resolve_path(args.out)
    else:
        out_path = audit_path.with_name("expansion_delta_report.json")
    md_path = out_path.with_suffix(".md")
    write_json(out_path, report)
    md_path.write_text(markdown_report(report), encoding="utf-8")
    print(f"Wrote {out_path}")
    print(f"Wrote {md_path}")
    print(json.dumps(report["summary"], indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
