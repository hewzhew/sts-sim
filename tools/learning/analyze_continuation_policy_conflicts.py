#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json


DEFAULT_LABEL_DIR = (
    REPO_ROOT / "tools" / "artifacts" / "card_cashout_rollout_labels" / "v1"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Explain rollout label continuation_policy_conflict cases. The report is "
            "diagnostic only: it attributes why a continuation policy realized or failed "
            "a static cashout-best candidate."
        )
    )
    parser.add_argument("--label-dir", type=Path, default=DEFAULT_LABEL_DIR)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--substatus", default="continuation_policy_conflict")
    parser.add_argument("--out", type=Path)
    parser.add_argument("--json-out", type=Path)
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def as_int(value: Any) -> int:
    try:
        return int(value or 0)
    except (TypeError, ValueError):
        return 0


def as_float(value: Any) -> float:
    try:
        return float(value or 0.0)
    except (TypeError, ValueError):
        return 0.0


def card_name(candidate: dict[str, Any]) -> str:
    return str(candidate.get("card_id") or candidate.get("action_key") or "Skip")


def outcome_tags(outcome: dict[str, Any], other: dict[str, Any], source: dict[str, Any]) -> list[str]:
    tags: list[str] = []
    attr = outcome.get("attribution") or {}
    other_attr = other.get("attribution") or {}
    max_unblocked = as_int(attr.get("max_visible_unblocked_damage"))
    max_incoming = as_int(attr.get("max_visible_incoming_damage"))
    hp_loss = as_int(attr.get("hp_loss_observed"))
    monster_damage = as_int(attr.get("monster_hp_reduction_observed"))
    other_monster_damage = as_int(other_attr.get("monster_hp_reduction_observed"))
    combat_wins = as_int(outcome.get("combat_win_delta"))
    other_combat_wins = as_int(other.get("combat_win_delta"))
    floor = as_int(outcome.get("end_floor"))
    result = str(outcome.get("end_result") or "")

    if max_unblocked >= 24 or max_incoming >= 32:
        tags.append("high_incoming_leak")
    if floor in {16, 33, 50} or (as_int(source.get("act")) == 1 and floor == 16):
        tags.append("boss_or_ramp_window")
    if result == "defeat":
        tags.append("terminal_defeat")
    if result == "defeat" and combat_wins <= other_combat_wins:
        tags.append("all_in_without_combat_win")
    if monster_damage >= other_monster_damage + 40 and hp_loss >= as_int(other_attr.get("hp_loss_observed")) + 15:
        tags.append("local_damage_over_survival")
    if as_int(outcome.get("hp_delta")) <= as_int(other.get("hp_delta")) - 20:
        tags.append("hp_budget_failure")
    return sorted(set(tags))


def compact_outcome(outcome: dict[str, Any], other: dict[str, Any], source: dict[str, Any]) -> dict[str, Any]:
    attr = outcome.get("attribution") or {}
    return {
        "candidate_key": outcome.get("candidate_key"),
        "card_id": outcome.get("card_id"),
        "end_result": outcome.get("end_result"),
        "terminal_reason": outcome.get("terminal_reason"),
        "end_floor": as_int(outcome.get("end_floor")),
        "end_hp": as_int(outcome.get("end_hp")),
        "floor_delta": as_int(outcome.get("floor_delta")),
        "combat_win_delta": as_int(outcome.get("combat_win_delta")),
        "hp_delta": as_int(outcome.get("hp_delta")),
        "reward_total": round(as_float(outcome.get("reward_total")), 3),
        "hp_loss_observed": as_int(attr.get("hp_loss_observed")),
        "max_single_transition_hp_loss": as_int(attr.get("max_single_transition_hp_loss")),
        "max_visible_incoming_damage": as_int(attr.get("max_visible_incoming_damage")),
        "max_visible_unblocked_damage": as_int(attr.get("max_visible_unblocked_damage")),
        "monster_hp_reduction_observed": as_int(attr.get("monster_hp_reduction_observed")),
        "combat_turns_observed": as_int(attr.get("combat_turns_observed")),
        "tags": outcome_tags(outcome, other, source),
    }


def observation_row(label: dict[str, Any], observation: dict[str, Any]) -> dict[str, Any]:
    classification = observation.get("classification") or {}
    chosen = classification.get("chosen_outcome") or {}
    best = classification.get("cashout_best_outcome") or {}
    source = label.get("source_case") or {}
    verdict = str(classification.get("verdict") or "unknown")
    return {
        "continuation_policy": observation.get("continuation_policy"),
        "horizon": observation.get("horizon"),
        "verdict": verdict,
        "reason": classification.get("reason"),
        "chosen": compact_outcome(chosen, best, source),
        "cashout_best": compact_outcome(best, chosen, source),
        "cashout_minus_chosen": classification.get("outcome_diff_cashout_minus_chosen") or {},
        "case_report_path": observation.get("case_report_path"),
    }


def analyze_label(label: dict[str, Any]) -> dict[str, Any]:
    source = label.get("source_case") or {}
    rows = [
        observation_row(label, observation)
        for observation in label.get("observations") or []
        if observation.get("status") == "ok"
    ]
    tag_counts: Counter[str] = Counter()
    for row in rows:
        tag_counts.update(row["cashout_best"]["tags"])

    policy_rows: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        policy_rows[str(row.get("continuation_policy") or "unknown")].append(row)

    return {
        "case_id": label.get("case_id"),
        "label_status": label.get("label_status"),
        "label_substatus": label.get("label_substatus"),
        "source_policy": label.get("source_policy"),
        "seed": source.get("seed"),
        "step_index": source.get("step_index"),
        "act": source.get("act"),
        "floor": source.get("floor"),
        "chosen": card_name(source.get("chosen") or {}),
        "cashout_best": card_name(source.get("best_by_cashout") or {}),
        "cashout_gap": as_float(source.get("cashout_gap")),
        "verdict_counts": label.get("verdict_counts") or {},
        "policy_verdict_counts": label.get("policy_verdict_counts") or {},
        "horizon_verdict_counts": label.get("horizon_verdict_counts") or {},
        "death_decomposition_tags": dict(sorted(tag_counts.items())),
        "observations": rows,
        "by_policy": {
            policy: {
                "verdicts": dict(Counter(str(row["verdict"]) for row in items)),
                "cashout_best_tags": dict(
                    Counter(tag for row in items for tag in row["cashout_best"]["tags"])
                ),
            }
            for policy, items in sorted(policy_rows.items())
        },
    }


def build_report(labels: list[dict[str, Any]], substatus: str) -> dict[str, Any]:
    selected = [
        analyze_label(label)
        for label in labels
        if str(label.get("label_substatus") or "") == substatus
    ]
    tag_counts: Counter[str] = Counter()
    card_counts: Counter[str] = Counter()
    policy_verdicts: dict[str, Counter[str]] = defaultdict(Counter)
    for case in selected:
        tag_counts.update(case["death_decomposition_tags"])
        card_counts[str(case["cashout_best"])] += 1
        for policy, payload in case["by_policy"].items():
            policy_verdicts[policy].update(payload["verdicts"])

    return {
        "report_version": "continuation_policy_conflict_analysis_v0",
        "substatus": substatus,
        "summary": {
            "case_count": len(selected),
            "cashout_best_cards": dict(sorted(card_counts.items())),
            "death_decomposition_tags": dict(sorted(tag_counts.items())),
            "policy_verdicts": {
                policy: dict(sorted(counter.items()))
                for policy, counter in sorted(policy_verdicts.items())
            },
        },
        "cases": selected,
    }


def render_markdown(report: dict[str, Any]) -> str:
    lines: list[str] = []
    lines.append("# Continuation Policy Conflict Analysis")
    lines.append("")
    summary = report["summary"]
    lines.append(f"- cases: `{summary['case_count']}`")
    lines.append(f"- cashout-best cards: `{summary['cashout_best_cards']}`")
    lines.append(f"- death tags: `{summary['death_decomposition_tags']}`")
    lines.append(f"- policy verdicts: `{summary['policy_verdicts']}`")
    lines.append("")
    for case in report["cases"]:
        lines.append(f"## `{case['case_id']}`")
        lines.append(
            f"- source: `{case['source_policy']}` seed `{case['seed']}` step `{case['step_index']}` floor `{case['floor']}`"
        )
        lines.append(
            f"- choice: `{case['chosen']}` -> cashout-best `{case['cashout_best']}`, gap `{case['cashout_gap']:.2f}`"
        )
        lines.append(f"- verdicts: `{case['verdict_counts']}`")
        lines.append(f"- tags: `{case['death_decomposition_tags']}`")
        lines.append("")
        lines.append("| policy | horizon | verdict | best result | best floor/hp | best combat wins | best hp loss | max incoming/unblocked | best tags | chosen result | chosen floor/hp |")
        lines.append("|---|---:|---|---|---:|---:|---:|---:|---|---|---:|")
        for row in case["observations"]:
            best = row["cashout_best"]
            chosen = row["chosen"]
            lines.append(
                "| "
                + " | ".join(
                    [
                        str(row["continuation_policy"]),
                        str(row["horizon"]),
                        str(row["verdict"]),
                        str(best["end_result"]),
                        f"{best['end_floor']}/{best['end_hp']}",
                        str(best["combat_win_delta"]),
                        str(best["hp_loss_observed"]),
                        f"{best['max_visible_incoming_damage']}/{best['max_visible_unblocked_damage']}",
                        "`" + ",".join(best["tags"]) + "`",
                        str(chosen["end_result"]),
                        f"{chosen['end_floor']}/{chosen['end_hp']}",
                    ]
                )
                + " |"
            )
        lines.append("")
    return "\n".join(lines) + "\n"


def main() -> None:
    args = parse_args()
    report_path = args.report or (args.label_dir / "cashout_rollout_label_report.json")
    labels = read_json(report_path).get("labels") or []
    report = build_report(labels, args.substatus)
    json_out = args.json_out or report_path.with_name("continuation_policy_conflict_analysis.json")
    md_out = args.out or report_path.with_name("continuation_policy_conflict_analysis.md")
    write_json(json_out, report)
    md_out.parent.mkdir(parents=True, exist_ok=True)
    md_out.write_text(render_markdown(report), encoding="utf-8")
    print(f"wrote {json_out}")
    print(f"wrote {md_out}")


if __name__ == "__main__":
    main()
