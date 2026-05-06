#!/usr/bin/env python3
"""Extract exact draw/setup cashout evidence from plan-probe expansion deltas.

This consumes `analyze_plan_probe_expansion_deltas.py` output. It is a
diagnostic event extractor, not a policy trainer and not a probabilistic draw
model. V0 only explains cases where a larger current-turn search budget changed
plan-query answers, then classifies whether draw/search/setup evidence appeared
in the new best line.
"""
from __future__ import annotations

import argparse
import json
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from combat_reranker_common import write_json

REPO_ROOT = Path(__file__).resolve().parents[2]
REPORT_VERSION = "draw_setup_cashout_event_analysis_v0"

DRAW_CARDS = {
    "Acrobatics",
    "Adrenaline",
    "Backflip",
    "BattleTrance",
    "BurningPact",
    "DaggerThrow",
    "DeepBreath",
    "Finesse",
    "FlashOfSteel",
    "Impatience",
    "MasterOfStrategy",
    "Offering",
    "PommelStrike",
    "Prepared",
    "ShrugItOff",
    "Skim",
    "ThinkingAhead",
    "Warcry",
}

SEARCH_CARDS = {
    "SecretTechnique",
    "SecretWeapon",
    "Seek",
    "Violence",
}

SETUP_OR_SCALING_CARDS = {
    "Barricade",
    "DemonForm",
    "DarkEmbrace",
    "FeelNoPain",
    "Footwork",
    "Inflame",
    "Metallicize",
    "NoxiousFumes",
    "Panache",
    "Rupture",
}

ZONE_MUTATION_CARDS = {
    "Anger",
    "Immolate",
    "PowerThrough",
    "RecklessCharge",
    "WildStrike",
}

BLOCK_CARDS = {
    "Defend",
    "FlameBarrier",
    "GhostlyArmor",
    "Impervious",
    "PowerThrough",
    "ShrugItOff",
    "TrueGrit",
}

KILL_WINDOW_CARDS = {
    "Feed",
    "HandOfGreed",
    "RitualDagger",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Classify draw/setup cashout evidence from plan-probe expansion deltas."
    )
    parser.add_argument("--delta-report", type=Path, required=True)
    parser.add_argument(
        "--out",
        type=Path,
        help="Output JSON path. Markdown is written next to it with .md suffix.",
    )
    return parser.parse_args()


def resolve_path(path: str | Path | None) -> Path | None:
    if not path:
        return None
    p = Path(path)
    return p if p.is_absolute() else REPO_ROOT / p


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def try_read_json(path: str | Path | None) -> tuple[dict[str, Any] | None, str | None]:
    resolved = resolve_path(path)
    if resolved is None:
        return None, "missing_path"
    try:
        return read_json(resolved), None
    except FileNotFoundError:
        return None, f"file_not_found:{resolved}"
    except json.JSONDecodeError as exc:
        return None, f"json_error:{resolved}:{exc}"


def action_card_id(action_key: str) -> str | None:
    for part in str(action_key).split("/"):
        if part.startswith("card:"):
            return part.removeprefix("card:")
    return None


def action_card_ids(action_keys: list[str]) -> list[str]:
    return [card for key in action_keys if (card := action_card_id(key))]


def lcs_added_indices(before_cards: list[str], after_cards: list[str]) -> list[int]:
    """Return indices in after_cards not consumed by a simple LCS match."""
    n = len(before_cards)
    m = len(after_cards)
    dp = [[0] * (m + 1) for _ in range(n + 1)]
    for i in range(n - 1, -1, -1):
        for j in range(m - 1, -1, -1):
            if before_cards[i] == after_cards[j]:
                dp[i][j] = dp[i + 1][j + 1] + 1
            else:
                dp[i][j] = max(dp[i + 1][j], dp[i][j + 1])

    matched_after: set[int] = set()
    i = 0
    j = 0
    while i < n and j < m:
        if before_cards[i] == after_cards[j]:
            matched_after.add(j)
            i += 1
            j += 1
        elif dp[i + 1][j] >= dp[i][j + 1]:
            i += 1
        else:
            j += 1
    return [idx for idx in range(m) if idx not in matched_after]


def plan_query(report: dict[str, Any] | None, query_name: str | None) -> dict[str, Any] | None:
    if not report or not query_name:
        return None
    for query in report.get("plan_queries") or []:
        if query.get("query_name") == query_name:
            return query
    return None


def outcome_value(query: dict[str, Any] | None, field: str) -> int | float | bool | None:
    outcome = (query or {}).get("outcome") or {}
    value = outcome.get(field)
    if isinstance(value, (int, float, bool)):
        return value
    return None


def numeric_delta(
    before_query: dict[str, Any] | None,
    after_query: dict[str, Any] | None,
    row: dict[str, Any],
    field: str,
    fallback_row_key: str | None = None,
) -> int | float | None:
    before = outcome_value(before_query, field)
    after = outcome_value(after_query, field)
    if isinstance(before, (int, float)) and isinstance(after, (int, float)):
        return after - before
    if fallback_row_key and isinstance(row.get(fallback_row_key), (int, float)):
        return row.get(fallback_row_key)
    return None


def bool_transition(
    before_query: dict[str, Any] | None,
    after_query: dict[str, Any] | None,
    row: dict[str, Any],
    field: str,
    fallback_row_key: str | None = None,
) -> str | None:
    before = outcome_value(before_query, field)
    after = outcome_value(after_query, field)
    if isinstance(before, bool) and isinstance(after, bool) and before != after:
        return f"{str(before).lower()}->{str(after).lower()}"
    fallback = row.get(fallback_row_key or "")
    return str(fallback) if fallback else None


def card_role(card: str) -> str:
    if card in DRAW_CARDS:
        return "draw"
    if card in SEARCH_CARDS:
        return "search"
    if card in SETUP_OR_SCALING_CARDS:
        return "setup_or_scaling"
    if card in ZONE_MUTATION_CARDS:
        return "zone_mutation"
    if card in BLOCK_CARDS:
        return "block"
    if card in KILL_WINDOW_CARDS:
        return "kill_window"
    return "other_payoff"


def card_roles(cards: list[str]) -> dict[str, str]:
    return {card: card_role(card) for card in cards}


def event_confidence(
    base_report: dict[str, Any] | None,
    rerun_report: dict[str, Any] | None,
    before_query: dict[str, Any] | None,
    after_query: dict[str, Any] | None,
    errors: list[str],
) -> str:
    if errors:
        return "missing_report"
    if base_report is not None and rerun_report is not None and before_query is not None and after_query is not None:
        return "exact_line_delta"
    return "inferred_from_action_delta"


def classify_event(
    *,
    row: dict[str, Any],
    added_cards: list[str],
    payoff_cards: list[str],
    damage_delta: int | float | None,
    block_delta: int | float | None,
    unblocked_delta: int | float | None,
    setup_transition: str | None,
) -> str:
    delta_kind = str(row.get("delta_kind") or "")
    query_name = str(row.get("query_name") or "")
    added_roles = {card_role(card) for card in added_cards}
    payoff_roles = {card_role(card) for card in payoff_cards}
    has_draw = "draw" in added_roles or "draw" in row.get("mechanism_tags", [])
    has_search = "search" in added_roles
    has_zone = "zone_mutation" in added_roles or "zone_mutation" in row.get("mechanism_tags", [])
    has_setup_payoff = "setup_or_scaling" in payoff_roles or setup_transition == "false->true"

    if delta_kind == "confidence_changed":
        return "confidence_only"
    if has_draw and has_setup_payoff and (setup_transition == "false->true" or (damage_delta or 0) > 0):
        return "draw_to_setup_to_damage"
    if has_draw and query_name in {"CanFullBlock", "CanFullBlockThenMaxDamage"}:
        if (block_delta or 0) > 0 or (unblocked_delta or 0) < 0:
            return "draw_to_block"
    if has_draw and query_name == "CanLethal" and delta_kind == "status_changed":
        return "draw_to_lethal"
    if has_search and payoff_cards:
        return "search_to_payoff"
    if has_zone and has_draw:
        return "zone_mutation_then_draw"
    if added_cards:
        return "line_changed_without_clear_draw"
    return "unclassified"


def event_notes(
    row: dict[str, Any],
    added_cards: list[str],
    payoff_cards: list[str],
    errors: list[str],
    event_class: str,
) -> list[str]:
    notes = [f"delta_kind:{row.get('delta_kind')}", f"query:{row.get('query_name')}"]
    if errors:
        notes.extend(errors)
    if added_cards:
        notes.append(f"added_cards:{','.join(added_cards)}")
    if payoff_cards:
        notes.append(f"payoff_cards:{','.join(payoff_cards)}")
    if event_class == "confidence_only":
        notes.append("not_actionable:best_line_or_outcome_not_improved")
    if event_class == "line_changed_without_clear_draw":
        notes.append("line_changed_but_no_clear_draw_search_payoff_pattern")
    return notes


def build_event(row: dict[str, Any]) -> dict[str, Any]:
    base_report, base_error = try_read_json(row.get("report_path"))
    rerun_report, rerun_error = try_read_json(row.get("rerun_report_path"))
    errors = [error for error in [base_error, rerun_error] if error]
    before_query = plan_query(base_report, row.get("query_name"))
    after_query = plan_query(rerun_report, row.get("query_name"))

    before_actions = list(row.get("before_actions") or [])
    after_actions = list(row.get("after_actions") or [])
    before_cards = action_card_ids(before_actions)
    after_cards = action_card_ids(after_actions)
    added_indices = lcs_added_indices(before_cards, after_cards)
    added_actions = [after_actions[idx] for idx in added_indices if idx < len(after_actions)]
    added_cards = [after_cards[idx] for idx in added_indices if idx < len(after_cards)]
    payoff_cards = [
        card
        for card in added_cards
        if card_role(card) not in {"draw", "search", "zone_mutation"}
    ]

    damage_delta = numeric_delta(before_query, after_query, row, "damage_done", "damage_delta")
    block_delta = numeric_delta(before_query, after_query, row, "block_after")
    unblocked_delta = numeric_delta(
        before_query,
        after_query,
        row,
        "projected_unblocked_damage",
        "unblocked_delta",
    )
    setup_transition = bool_transition(
        before_query,
        after_query,
        row,
        "played_setup_or_scaling",
        "setup_transition",
    )

    event_class = classify_event(
        row=row,
        added_cards=added_cards,
        payoff_cards=payoff_cards,
        damage_delta=damage_delta,
        block_delta=block_delta,
        unblocked_delta=unblocked_delta,
        setup_transition=setup_transition,
    )
    confidence = event_confidence(base_report, rerun_report, before_query, after_query, errors)
    notes = event_notes(row, added_cards, payoff_cards, errors, event_class)

    return {
        "case_id": row.get("case_id"),
        "seed": row.get("seed"),
        "step_index": row.get("step_index"),
        "act": row.get("act"),
        "floor": row.get("floor"),
        "pressure_class": row.get("pressure_class"),
        "query_name": row.get("query_name"),
        "delta_kind": row.get("delta_kind"),
        "before_actions": before_actions,
        "after_actions": after_actions,
        "before_cards": before_cards,
        "after_cards": after_cards,
        "added_actions": added_actions,
        "added_cards": added_cards,
        "added_card_roles": card_roles(added_cards),
        "payoff_cards": payoff_cards,
        "payoff_card_roles": card_roles(payoff_cards),
        "damage_delta": damage_delta,
        "block_delta": block_delta,
        "unblocked_delta": unblocked_delta,
        "setup_transition": setup_transition,
        "event_class": event_class,
        "confidence": confidence,
        "notes": notes,
        "source": {
            "report_path": row.get("report_path"),
            "rerun_report_path": row.get("rerun_report_path"),
        },
    }


def is_actionable(event: dict[str, Any]) -> bool:
    if event.get("confidence") == "missing_report":
        return False
    if event.get("event_class") == "confidence_only":
        return False
    if event.get("delta_kind") in {"status_changed", "damage_improved", "setup_changed"}:
        return True
    return False


def build_report(delta_report: dict[str, Any], delta_path: Path) -> dict[str, Any]:
    events = [build_event(row) for row in delta_report.get("deltas") or []]
    actionable = [event for event in events if is_actionable(event)]
    class_counts = Counter(event["event_class"] for event in events)
    actionable_class_counts = Counter(event["event_class"] for event in actionable)
    confidence_counts = Counter(event["confidence"] for event in events)
    added_card_counts = Counter(card for event in events for card in event["added_cards"])
    payoff_card_counts = Counter(card for event in events for card in event["payoff_cards"])
    query_counts = Counter(event["query_name"] for event in events)
    return {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "delta_report": str(delta_path),
        "summary": {
            "delta_rows": len(events),
            "actionable_events": len(actionable),
            "event_class_counts": dict(class_counts),
            "actionable_event_class_counts": dict(actionable_class_counts),
            "confidence_counts": dict(confidence_counts),
            "query_counts": dict(query_counts),
            "added_card_counts": dict(added_card_counts),
            "payoff_card_counts": dict(payoff_card_counts),
        },
        "events": events,
        "actionable_events": actionable,
        "limitations": [
            "V0 infers cashout evidence from best-line deltas only.",
            "V0 does not model draw probabilities, reshuffle, or future-turn value.",
            "Event classes are diagnostic evidence, not policy labels or teacher truth.",
            "added_cards are derived by LCS over card ids, so repeated identical cards can be ambiguous.",
        ],
    }


def markdown_table_counter(lines: list[str], title: str, items: dict[str, int]) -> None:
    lines.extend(["", f"## {title}", ""])
    if not items:
        lines.append("_none_")
        return
    lines.append("| item | n |")
    lines.append("| --- | ---: |")
    for key, value in sorted(items.items(), key=lambda item: (-item[1], item[0])):
        lines.append(f"| `{key}` | {value} |")


def markdown_report(report: dict[str, Any]) -> str:
    summary = report["summary"]
    lines = [
        "# Draw/Setup Cashout Event Analysis",
        "",
        "This report extracts exact diagnostic events from plan-probe expansion deltas.",
        "It does not train a model and does not claim a true card value.",
        "",
        "## Summary",
        "",
        f"- delta_rows: `{summary['delta_rows']}`",
        f"- actionable_events: `{summary['actionable_events']}`",
    ]
    markdown_table_counter(lines, "Event Classes", summary["event_class_counts"])
    markdown_table_counter(lines, "Actionable Event Classes", summary["actionable_event_class_counts"])
    markdown_table_counter(lines, "Confidence", summary["confidence_counts"])
    markdown_table_counter(lines, "Added Cards", summary["added_card_counts"])
    markdown_table_counter(lines, "Payoff Cards", summary["payoff_card_counts"])

    lines.extend(["", "## Actionable Events", ""])
    if not report["actionable_events"]:
        lines.append("_none_")
    else:
        lines.append("| case | query | class | confidence | damage Δ | block Δ | unblocked Δ | setup | added | payoff | before | after |")
        lines.append("| --- | --- | --- | --- | ---: | ---: | ---: | --- | --- | --- | --- | --- |")
        for event in report["actionable_events"]:
            lines.append(
                f"| `{event['case_id']}` | `{event['query_name']}` | `{event['event_class']}` | "
                f"`{event['confidence']}` | "
                f"{event['damage_delta'] if event['damage_delta'] is not None else ''} | "
                f"{event['block_delta'] if event['block_delta'] is not None else ''} | "
                f"{event['unblocked_delta'] if event['unblocked_delta'] is not None else ''} | "
                f"`{event['setup_transition'] or ''}` | "
                f"`{' -> '.join(event['added_cards'])}` | "
                f"`{' -> '.join(event['payoff_cards'])}` | "
                f"`{' -> '.join(event['before_cards'])}` | "
                f"`{' -> '.join(event['after_cards'])}` |"
            )

    lines.extend(["", "## All Events", ""])
    if not report["events"]:
        lines.append("_none_")
    else:
        lines.append("| case | query | delta | class | actionable | notes |")
        lines.append("| --- | --- | --- | --- | --- | --- |")
        actionable_ids = {(event["case_id"], event["query_name"]) for event in report["actionable_events"]}
        for event in report["events"]:
            actionable = (event["case_id"], event["query_name"]) in actionable_ids
            lines.append(
                f"| `{event['case_id']}` | `{event['query_name']}` | `{event['delta_kind']}` | "
                f"`{event['event_class']}` | `{str(actionable).lower()}` | "
                f"{'; '.join(event['notes'])} |"
            )

    lines.extend(["", "## Limitations", ""])
    for item in report["limitations"]:
        lines.append(f"- {item}")
    return "\n".join(lines) + "\n"


def main() -> None:
    args = parse_args()
    delta_path = resolve_path(args.delta_report)
    if delta_path is None:
        raise SystemExit("--delta-report is required")
    delta_report = read_json(delta_path)
    report = build_report(delta_report, delta_path)
    out_path = resolve_path(args.out) if args.out else delta_path.with_name("draw_setup_cashout_events.json")
    if out_path is None:
        raise SystemExit("could not resolve output path")
    md_path = out_path.with_suffix(".md")
    write_json(out_path, report)
    md_path.write_text(markdown_report(report), encoding="utf-8")
    print(f"Wrote {out_path}")
    print(f"Wrote {md_path}")
    print(json.dumps(report["summary"], indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
