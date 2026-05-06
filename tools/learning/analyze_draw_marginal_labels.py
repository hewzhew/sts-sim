#!/usr/bin/env python3
"""Drill down draw/search marginal labels into axes and tradeoffs.

The input labels are current-turn plan-query deltas, not card-choice truth. This
script explains why a marginal label was positive/equivalent/harmful and marks
which rows are clean enough to become training-prep evidence.
"""
from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from statistics import mean
from typing import Any

from combat_reranker_common import iter_jsonl, write_json, write_jsonl

REPO_ROOT = Path(__file__).resolve().parents[2]
REPORT_VERSION = "draw_marginal_label_drilldown_v1"
QUERY_NAMES = [
    "CanLethal",
    "CanFullBlock",
    "CanFullBlockThenMaxDamage",
    "CanPlaySetupAndStillBlock",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Analyze draw marginal labels.")
    parser.add_argument(
        "--input-dir",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "draw_marginal_value" / "v0",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "draw_marginal_value" / "v0" / "label_drilldown.json",
    )
    return parser.parse_args()


def resolve(path: Path) -> Path:
    return path if path.is_absolute() else REPO_ROOT / path


def as_int(value: Any, default: int = 0) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def as_bool(value: Any) -> bool:
    return bool(value)


def maybe_int(value: Any, default: int = 0) -> int:
    return default if value is None else as_int(value, default)


def load_rows(path: Path) -> list[dict[str, Any]]:
    return [row for _, row in iter_jsonl(path)]


def branch_index(branch_rows: list[dict[str, Any]]) -> dict[tuple[str, str, str], dict[str, Any]]:
    index = {}
    for row in branch_rows:
        index[(str(row["case_id"]), str(row["branch_name"]), str(row["query_name"]))] = row
    return index


def query_row(
    index: dict[tuple[str, str, str], dict[str, Any]], case_id: str, branch: str, query: str
) -> dict[str, Any]:
    return index.get((case_id, branch, query), {})


def action_cards(row: dict[str, Any]) -> list[str]:
    cards = []
    for action in row.get("best_action_keys") or []:
        text = str(action)
        if "card:" in text:
            cards.append(text.split("card:", 1)[1].split("/", 1)[0])
        elif "grid_select" in text:
            cards.append("GridSelect")
    return cards


def status_rank(status: str) -> int | None:
    if status == "feasible":
        return 2
    if status == "partial":
        return 1
    if status == "not_feasible":
        return 0
    return None


def query_axis(query_name: str) -> str:
    return {
        "CanLethal": "lethal_or_damage",
        "CanFullBlock": "full_block_or_leak",
        "CanFullBlockThenMaxDamage": "block_then_damage",
        "CanPlaySetupAndStillBlock": "setup_and_block",
    }.get(query_name, query_name)


def classify_case(row: dict[str, Any], index: dict[tuple[str, str, str], dict[str, Any]]) -> dict[str, Any]:
    case_id = str(row["case_id"])
    positive_axes: list[str] = []
    negative_axes: list[str] = []
    tradeoffs: list[str] = []
    notes: list[str] = []

    damage_delta = as_int(row.get("damage_delta"))
    block_delta = as_int(row.get("block_delta"))
    unblocked_reduction = as_int(row.get("unblocked_reduction"))
    hp_loss_reduction = as_int(row.get("hp_loss_reduction"))
    remaining_energy_delta = as_int(row.get("remaining_energy_delta"))
    remaining_hand_delta = as_int(row.get("remaining_hand_delta"))
    marginal_score = as_int(row.get("marginal_score"))

    if damage_delta > 0:
        positive_axes.append("damage_gain")
    elif damage_delta < 0:
        negative_axes.append("damage_loss")
    if block_delta > 0 or unblocked_reduction > 0 or as_bool(row.get("full_block_gain")):
        positive_axes.append("block_or_leak_gain")
    if hp_loss_reduction > 0:
        positive_axes.append("hp_loss_reduction")
    elif hp_loss_reduction < 0:
        negative_axes.append("hp_cost_or_extra_loss")
    if as_bool(row.get("setup_gain")):
        positive_axes.append("setup_gain")
    if as_bool(row.get("lethal_gain")):
        positive_axes.append("lethal_gain")
    if remaining_hand_delta > 0:
        positive_axes.append("hand_resource_gain")
    elif remaining_hand_delta < 0:
        negative_axes.append("hand_resource_loss")
    if remaining_energy_delta > 0:
        positive_axes.append("energy_resource_gain")
    elif remaining_energy_delta < 0:
        negative_axes.append("energy_resource_loss")

    forced_fbd = query_row(index, case_id, "forced_draw_best", "CanFullBlockThenMaxDamage")
    no_fbd = query_row(index, case_id, "no_draw_best", "CanFullBlockThenMaxDamage")
    forced_cards = action_cards(forced_fbd)
    no_cards = action_cards(no_fbd)

    target = str(row.get("target_action_card") or "")
    if target and forced_cards and target not in forced_cards:
        notes.append("forced_branch_best_query_did_not_include_target_card")
    if damage_delta < 0 and ("block_or_leak_gain" in positive_axes):
        tradeoffs.append("trades_damage_for_block")
    if hp_loss_reduction < 0 and damage_delta > 0:
        tradeoffs.append("trades_hp_for_damage")
    if hp_loss_reduction < 0 and ("block_or_leak_gain" in positive_axes):
        tradeoffs.append("hp_cost_offsets_block_gain")
    if positive_axes and negative_axes:
        tradeoffs.append("mixed_axis_tradeoff")
    if not positive_axes and not negative_axes:
        notes.append("near_noop_delta")

    if as_bool(row.get("full_block_gain")) and damage_delta <= 0:
        dominant_axis = "draw_to_block"
    elif damage_delta > 0 and hp_loss_reduction < 0:
        dominant_axis = "resource_window_damage_with_hp_cost"
    elif damage_delta > 0:
        dominant_axis = "draw_to_damage"
    elif as_bool(row.get("setup_gain")):
        dominant_axis = "draw_to_setup"
    elif hp_loss_reduction < 0:
        dominant_axis = "hp_cost_penalty"
    elif marginal_score <= -25:
        dominant_axis = "harmful_tradeoff"
    elif marginal_score == 0 or not positive_axes:
        dominant_axis = "equivalent_or_no_clear_cashout"
    else:
        dominant_axis = positive_axes[0]

    label_strength = str(row.get("label_strength") or "")
    usable_as_hard_preference = False
    if label_strength == "robust_positive" and not negative_axes:
        usable_as_hard_preference = True
    elif label_strength == "conditional_positive" and positive_axes and len(negative_axes) == 0:
        usable_as_hard_preference = True
    elif label_strength == "harmful" and negative_axes and not positive_axes:
        usable_as_hard_preference = True
    usable_as_axis_evidence = bool(positive_axes or negative_axes)

    if label_strength == "conditional_positive" and negative_axes:
        notes.append("positive_label_has_negative_axis_do_not_use_as_hard_positive")
    if label_strength == "equivalent" and positive_axes:
        notes.append("equivalent_label_has_local_axis_gain_but_low_total_score")
    if target == "Offering" and hp_loss_reduction < 0:
        notes.append("offering_hp_cost_visible")
    if target == "BattleTrance" and "damage_loss" in negative_axes and "block_or_leak_gain" in positive_axes:
        notes.append("battle_trance_positive_is_defensive_not_setup_damage")
    if target == "PommelStrike" and damage_delta > 0 and label_strength == "equivalent":
        notes.append("pommel_body_damage_gain_not_enough_for_positive_label")
    if target == "SecretTechnique" and usable_as_hard_preference:
        notes.append("synthetic_template_may_be_too_clean_for_search_to_block")

    return {
        **row,
        "dominant_axis": dominant_axis,
        "positive_axes": positive_axes,
        "negative_axes": negative_axes,
        "tradeoff_notes": tradeoffs,
        "notes": notes,
        "usable_as_hard_preference": usable_as_hard_preference,
        "usable_as_axis_evidence": usable_as_axis_evidence,
        "forced_cards": forced_cards,
        "no_draw_cards": no_cards,
    }


def aggregate(rows: list[dict[str, Any]]) -> dict[str, Any]:
    def grouped(key: str) -> list[dict[str, Any]]:
        groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
        for row in rows:
            groups[str(row.get(key) or "")].append(row)
        out = []
        for name, items in sorted(groups.items()):
            out.append(
                {
                    key: name,
                    "count": len(items),
                    "label_counts": dict(sorted(Counter(str(row.get("label_strength")) for row in items).items())),
                    "dominant_axis_counts": dict(sorted(Counter(str(row.get("dominant_axis")) for row in items).items())),
                    "usable_as_hard_preference": sum(1 for row in items if row.get("usable_as_hard_preference")),
                    "usable_as_axis_evidence": sum(1 for row in items if row.get("usable_as_axis_evidence")),
                    "avg_score": round(mean([as_int(row.get("marginal_score")) for row in items]), 3),
                }
            )
        return out

    all_notes = Counter(note for row in rows for note in row.get("notes") or [])
    all_tradeoffs = Counter(note for row in rows for note in row.get("tradeoff_notes") or [])
    return {
        "case_count": len(rows),
        "label_counts": dict(sorted(Counter(str(row.get("label_strength")) for row in rows).items())),
        "dominant_axis_counts": dict(sorted(Counter(str(row.get("dominant_axis")) for row in rows).items())),
        "usable_as_hard_preference_count": sum(1 for row in rows if row.get("usable_as_hard_preference")),
        "usable_as_axis_evidence_count": sum(1 for row in rows if row.get("usable_as_axis_evidence")),
        "by_card": grouped("target_action_card"),
        "by_dominant_axis": grouped("dominant_axis"),
        "note_counts": dict(sorted(all_notes.items())),
        "tradeoff_counts": dict(sorted(all_tradeoffs.items())),
    }


def axis_examples(cases: list[dict[str, Any]]) -> list[dict[str, Any]]:
    examples = []
    for row in cases:
        examples.append(
            {
                "example_id": f"{row['case_id']}::{row['target_action_card']}",
                "source_case_id": row["case_id"],
                "target_action_card": row.get("target_action_card"),
                "label_mode": "current_turn_draw_marginal_axis_evidence",
                "hard_preference_allowed": False,
                "dominant_axis": row.get("dominant_axis"),
                "positive_axes": row.get("positive_axes") or [],
                "negative_axes": row.get("negative_axes") or [],
                "tradeoff_notes": row.get("tradeoff_notes") or [],
                "notes": row.get("notes") or [],
                "label_strength": row.get("label_strength"),
                "marginal_score": row.get("marginal_score"),
                "full_block_gain": row.get("full_block_gain"),
                "setup_gain": row.get("setup_gain"),
                "lethal_gain": row.get("lethal_gain"),
                "damage_delta": row.get("damage_delta"),
                "block_delta": row.get("block_delta"),
                "unblocked_reduction": row.get("unblocked_reduction"),
                "hp_loss_reduction": row.get("hp_loss_reduction"),
                "remaining_energy_delta": row.get("remaining_energy_delta"),
                "remaining_hand_delta": row.get("remaining_hand_delta"),
                "forced_cards": row.get("forced_cards") or [],
                "no_draw_cards": row.get("no_draw_cards") or [],
                "usable_as_axis_evidence": bool(row.get("usable_as_axis_evidence")),
                "limitations": [
                    "current_turn_only",
                    "axis_evidence_not_card_choice_truth",
                    "not_a_hard_preference_label",
                ],
            }
        )
    return examples


def compare_query_preference(
    case: dict[str, Any],
    index: dict[tuple[str, str, str], dict[str, Any]],
    query_name: str,
) -> dict[str, Any] | None:
    case_id = str(case["case_id"])
    forced = query_row(index, case_id, "forced_draw_best", query_name)
    no_draw = query_row(index, case_id, "no_draw_best", query_name)
    if not forced or not no_draw:
        return None
    forced_status = str(forced.get("query_status") or "")
    no_status = str(no_draw.get("query_status") or "")
    forced_rank = status_rank(forced_status)
    no_rank = status_rank(no_status)
    if forced_rank is None or no_rank is None:
        return None

    forced_damage = maybe_int(forced.get("damage_done"))
    no_damage = maybe_int(no_draw.get("damage_done"))
    forced_unblocked = maybe_int(forced.get("projected_unblocked_damage"))
    no_unblocked = maybe_int(no_draw.get("projected_unblocked_damage"))
    forced_block = maybe_int(forced.get("block_after"))
    no_block = maybe_int(no_draw.get("block_after"))
    forced_hp_loss = maybe_int(forced.get("hp_loss_actual"))
    no_hp_loss = maybe_int(no_draw.get("hp_loss_actual"))
    forced_setup = as_bool(forced.get("played_setup_or_scaling"))
    no_setup = as_bool(no_draw.get("played_setup_or_scaling"))

    damage_delta = forced_damage - no_damage
    unblocked_reduction = no_unblocked - forced_unblocked
    block_delta = forced_block - no_block
    hp_loss_reduction = no_hp_loss - forced_hp_loss
    status_delta = forced_rank - no_rank
    setup_gain = forced_setup and not no_setup
    setup_loss = no_setup and not forced_setup

    preferred_branch = "equivalent"
    reason = "within_margin"
    if query_name == "CanLethal":
        if status_delta > 0:
            preferred_branch, reason = "forced_draw_best", "better_lethal_status"
        elif status_delta < 0:
            preferred_branch, reason = "no_draw_best", "worse_lethal_status"
        elif damage_delta >= 6:
            preferred_branch, reason = "forced_draw_best", "higher_damage_under_lethal_query"
        elif damage_delta <= -6:
            preferred_branch, reason = "no_draw_best", "lower_damage_under_lethal_query"
    elif query_name == "CanFullBlock":
        if status_delta > 0:
            preferred_branch, reason = "forced_draw_best", "better_block_status"
        elif status_delta < 0:
            preferred_branch, reason = "no_draw_best", "worse_block_status"
        elif unblocked_reduction >= 1:
            preferred_branch, reason = "forced_draw_best", "less_unblocked_damage"
        elif unblocked_reduction <= -1:
            preferred_branch, reason = "no_draw_best", "more_unblocked_damage"
    elif query_name == "CanFullBlockThenMaxDamage":
        if status_delta > 0:
            preferred_branch, reason = "forced_draw_best", "better_block_then_damage_status"
        elif status_delta < 0:
            preferred_branch, reason = "no_draw_best", "worse_block_then_damage_status"
        elif unblocked_reduction >= 1:
            preferred_branch, reason = "forced_draw_best", "less_unblocked_damage_under_damage_plan"
        elif unblocked_reduction <= -1:
            preferred_branch, reason = "no_draw_best", "more_unblocked_damage_under_damage_plan"
        elif damage_delta >= 6:
            preferred_branch, reason = "forced_draw_best", "higher_damage_after_block"
        elif damage_delta <= -6:
            preferred_branch, reason = "no_draw_best", "lower_damage_after_block"
    elif query_name == "CanPlaySetupAndStillBlock":
        if setup_gain and unblocked_reduction >= 0:
            preferred_branch, reason = "forced_draw_best", "setup_enabled_without_more_leak"
        elif setup_loss and unblocked_reduction <= 0:
            preferred_branch, reason = "no_draw_best", "setup_lost_or_more_leak"
        elif status_delta > 0:
            preferred_branch, reason = "forced_draw_best", "better_setup_block_status"
        elif status_delta < 0:
            preferred_branch, reason = "no_draw_best", "worse_setup_block_status"

    target = str(case.get("target_action_card") or "")
    forced_cards = action_cards(forced)
    no_draw_cards = action_cards(no_draw)
    notes = []
    if target and target not in forced_cards:
        notes.append("forced_query_line_did_not_include_target_action")
    if damage_delta < 0 and preferred_branch == "forced_draw_best":
        notes.append("forced_preference_has_damage_tradeoff")
    if unblocked_reduction < 0 and preferred_branch == "forced_draw_best":
        notes.append("forced_preference_has_defense_tradeoff")
    if hp_loss_reduction < 0 and preferred_branch == "forced_draw_best":
        notes.append("forced_preference_has_hp_cost")
    if damage_delta > 0 and preferred_branch == "no_draw_best":
        notes.append("no_draw_preference_gives_up_damage")
    if unblocked_reduction > 0 and preferred_branch == "no_draw_best":
        notes.append("no_draw_preference_gives_up_defense")

    hard_preference_allowed = preferred_branch != "equivalent" and target in forced_cards
    label = "equivalent"
    if preferred_branch == "forced_draw_best":
        label = f"target_action_better_for_{query_axis(query_name)}"
    elif preferred_branch == "no_draw_best":
        label = f"target_action_worse_for_{query_axis(query_name)}"

    return {
        "label_id": f"{case_id}::{target}::{query_name}",
        "source_case_id": case_id,
        "target_action_card": target,
        "query_name": query_name,
        "query_axis": query_axis(query_name),
        "label_mode": "current_turn_query_specific_preference_v0",
        "label": label,
        "preferred_branch": preferred_branch,
        "reason": reason,
        "hard_preference_allowed": hard_preference_allowed,
        "global_preference_allowed": False,
        "forced_status": forced_status,
        "no_draw_status": no_status,
        "status_delta": status_delta,
        "damage_delta": damage_delta,
        "block_delta": block_delta,
        "unblocked_reduction": unblocked_reduction,
        "hp_loss_reduction": hp_loss_reduction,
        "setup_gain": setup_gain,
        "setup_loss": setup_loss,
        "forced_cards": forced_cards,
        "no_draw_cards": no_draw_cards,
        "notes": notes,
        "limitations": [
            "current_turn_only",
            "query_specific_not_global_preference",
            "forced_draw_vs_no_draw_counterfactual_only",
        ],
    }


def query_specific_labels(
    cases: list[dict[str, Any]],
    index: dict[tuple[str, str, str], dict[str, Any]],
) -> list[dict[str, Any]]:
    labels = []
    for case in cases:
        for query_name in QUERY_NAMES:
            label = compare_query_preference(case, index, query_name)
            if label is not None:
                labels.append(label)
    return labels


def query_label_summary(rows: list[dict[str, Any]]) -> dict[str, Any]:
    by_card: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        by_card[str(row.get("target_action_card") or "")].append(row)
    hard_rows = [row for row in rows if row.get("hard_preference_allowed")]
    return {
        "count": len(rows),
        "label_counts": dict(sorted(Counter(str(row.get("label")) for row in rows).items())),
        "query_counts": dict(sorted(Counter(str(row.get("query_name")) for row in rows).items())),
        "hard_preference_count": len(hard_rows),
        "clean_hard_preference_count": sum(1 for row in hard_rows if not row.get("notes")),
        "hard_preference_with_tradeoff_count": sum(1 for row in hard_rows if row.get("notes")),
        "global_preference_count": sum(1 for row in rows if row.get("global_preference_allowed")),
        "by_card": [
            {
                "target_action_card": card,
                "count": len(items),
                "hard_preference_count": sum(1 for row in items if row.get("hard_preference_allowed")),
                "clean_hard_preference_count": sum(
                    1 for row in items if row.get("hard_preference_allowed") and not row.get("notes")
                ),
                "label_counts": dict(sorted(Counter(str(row.get("label")) for row in items).items())),
            }
            for card, items in sorted(by_card.items())
        ],
    }


def markdown(report: dict[str, Any]) -> str:
    lines = [
        "# Draw Marginal Label Drilldown",
        "",
        f"Generated: `{report['generated_at_utc']}`",
        "",
        "This audits current-turn marginal labels. It does not create card-choice truth.",
        "",
        "## Summary",
        "",
        f"- cases: `{report['summary']['case_count']}`",
        f"- labels: `{report['summary']['label_counts']}`",
        f"- dominant axes: `{report['summary']['dominant_axis_counts']}`",
        f"- usable as hard preference: `{report['summary']['usable_as_hard_preference_count']}`",
        f"- usable as axis evidence: `{report['summary']['usable_as_axis_evidence_count']}`",
        f"- query-specific labels: `{report['query_label_summary']['count']}`",
        f"- query-specific hard preferences: `{report['query_label_summary']['hard_preference_count']}`",
        f"- clean query-specific hard preferences: `{report['query_label_summary']['clean_hard_preference_count']}`",
        "",
        "## By Card",
        "",
        "| card | count | labels | axes | hard pref | axis evidence | avg score |",
        "| --- | ---: | --- | --- | ---: | ---: | ---: |",
    ]
    for row in report["summary"]["by_card"]:
        lines.append(
            f"| `{row['target_action_card']}` | {row['count']} | `{row['label_counts']}` | "
            f"`{row['dominant_axis_counts']}` | {row['usable_as_hard_preference']} | "
            f"{row['usable_as_axis_evidence']} | {row['avg_score']} |"
        )
    lines.extend(
        [
            "",
            "## Query-Specific Labels",
            "",
            "These are hard labels only inside one current-turn query, not global action preferences.",
            "",
            f"- labels: `{report['query_label_summary']['label_counts']}`",
            f"- queries: `{report['query_label_summary']['query_counts']}`",
            f"- global preferences: `{report['query_label_summary']['global_preference_count']}`",
            "",
        "| card | count | hard query pref | clean hard pref | labels |",
        "| --- | ---: | ---: | ---: | --- |",
        ]
    )
    for row in report["query_label_summary"]["by_card"]:
        lines.append(
            f"| `{row['target_action_card']}` | {row['count']} | {row['hard_preference_count']} | "
            f"{row['clean_hard_preference_count']} | "
            f"`{row['label_counts']}` |"
        )
    lines.extend(
        [
            "",
            "## Frequent Notes",
            "",
            "| note | count |",
            "| --- | ---: |",
        ]
    )
    for note, count in sorted(report["summary"]["note_counts"].items(), key=lambda kv: (-kv[1], kv[0]))[:20]:
        lines.append(f"| `{note}` | {count} |")
    lines.extend(
        [
            "",
            "## Top Non-Training Positives",
            "",
            "| case | card | label | axis | score | positives | negatives | notes |",
            "| --- | --- | --- | --- | ---: | --- | --- | --- |",
        ]
    )
    noisy = [
        row
        for row in report["cases"]
        if row.get("label_strength") in {"conditional_positive", "robust_positive"}
        and not row.get("usable_as_hard_preference")
    ][:20]
    for row in noisy:
        lines.append(
            f"| `{row['case_id']}` | `{row['target_action_card']}` | `{row['label_strength']}` | "
            f"`{row['dominant_axis']}` | {row['marginal_score']} | `{row['positive_axes']}` | "
            f"`{row['negative_axes']}` | `{row['notes']}` |"
        )
    return "\n".join(lines) + "\n"


def main() -> None:
    args = parse_args()
    input_dir = resolve(args.input_dir)
    out = resolve(args.out)
    marginal_path = input_dir / "marginal_examples.jsonl"
    branch_path = input_dir / "branch_outcomes.jsonl"
    marginals = load_rows(marginal_path)
    branches = load_rows(branch_path)
    index = branch_index(branches)
    cases = [classify_case(row, index) for row in marginals]
    query_labels = query_specific_labels(cases, index)
    report = {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {
            "input_dir": str(input_dir),
            "marginal_examples": str(marginal_path),
            "branch_outcomes": str(branch_path),
        },
        "summary": aggregate(cases),
        "query_label_summary": query_label_summary(query_labels),
        "cases": cases,
        "limitations": [
            "current_turn_only_horizon",
            "reason_axes_are_heuristic_drilldown_not_truth",
            "target_card_body_and_draw_effect_are_not_separated",
        ],
    }
    write_json(out, report)
    write_jsonl(out.with_suffix(".cases.jsonl"), cases)
    write_jsonl(out.with_name("axis_examples.jsonl"), axis_examples(cases))
    write_jsonl(out.with_name("query_specific_labels.jsonl"), query_labels)
    out.with_suffix(".md").write_text(markdown(report), encoding="utf-8")
    print(
        json.dumps(
            {
                "summary": report["summary"],
                "query_label_summary": report["query_label_summary"],
                "out": str(out),
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
