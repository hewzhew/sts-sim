#!/usr/bin/env python3
"""Audit harmful and low-confidence verified-teacher overrides.

This consumes summary JSON files emitted by run_verified_teacher_diagnostics.py.
It does not rerun the simulator. The goal is to make the teacher's accepted
overrides auditable: which overrides hurt seed-level outcomes, and which accepted
overrides had weak evidence such as horizon-capped returns near the margin.
"""
from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any

from return_q_common import write_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inputs", type=Path, nargs="+", required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--report-out", type=Path)
    parser.add_argument(
        "--low-adv-threshold",
        type=float,
        default=2.5,
        help="Accepted cap-stopped overrides below this adv are flagged low-confidence.",
    )
    parser.add_argument(
        "--top-examples",
        type=int,
        default=8,
        help="Maximum examples to include per report section.",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    audits = [audit_file(path, args.low_adv_threshold, args.top_examples) for path in args.inputs]
    payload = {
        "schema_version": "verified_teacher_override_audit_v0",
        "low_adv_threshold": args.low_adv_threshold,
        "files": audits,
        "aggregate": aggregate_audits(audits),
    }
    write_json(args.out, payload)
    if args.report_out:
        args.report_out.write_text(render_markdown(payload, args.top_examples), encoding="utf-8")


def audit_file(path: Path, low_adv_threshold: float, top_examples: int) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    row = verified_row(data)
    events = collect_override_events(row)
    harmed_examples = collect_seed_delta_examples(data, "harmed_examples")
    top_worsened = collect_seed_delta_examples(data, "top_worsened")

    low_margin_cap = [
        event
        for event in events
        if event.get("horizon_stop_reason") == "horizon_decision_cap"
        and float(event.get("adv_vs_rule") or 0.0) < low_adv_threshold
    ]
    low_margin_cap_no_payoff = [event for event in low_margin_cap if not event.get("payoff_reasons")]

    return {
        "input": str(path),
        "case": row.get("case") or path.stem,
        "summary": {
            "average_total_reward": row.get("average_total_reward"),
            "reward_stderr": row.get("reward_stderr"),
            "defeat_count": row.get("defeat_count"),
            "episodes": row.get("episodes"),
            "override_count": row.get("verified_override_count"),
            "override_rate": row.get("verified_override_rate"),
            "low_evidence_reject_count": row.get("verified_low_evidence_reject_count"),
            "confirm_decision_count": row.get("verified_confirm_decision_count"),
            "confirm_accept_count": row.get("verified_confirm_accept_count"),
            "confirm_reject_count": row.get("verified_confirm_reject_count"),
        },
        "accepted_override_audit": {
            "count": len(events),
            "avg_adv_vs_rule": mean([float(event.get("adv_vs_rule") or 0.0) for event in events]),
            "horizon_stop_reason_counts": counter_to_dict(
                Counter(event.get("horizon_stop_reason") or "<missing>" for event in events)
            ),
            "adv_bucket_counts": counter_to_dict(Counter(adv_bucket(event.get("adv_vs_rule")) for event in events)),
            "payoff_reason_counts": counter_to_dict(Counter(payoff_key(event) for event in events)),
            "confirmation_kind_counts": counter_to_dict(
                Counter(event.get("confirmation_kind") or "<none>" for event in events)
            ),
            "artifact_reason_counts": counter_to_dict(artifact_reason_counts(events).most_common(24)),
            "context_key_counts": counter_to_dict(context_counts(events).most_common(24)),
            "action_pair_kind_counts": counter_to_dict(Counter(action_pair_kind(event) for event in events).most_common(24)),
        },
        "low_confidence_accepted": {
            "cap_stopped_below_threshold_count": len(low_margin_cap),
            "cap_stopped_below_threshold_no_payoff_count": len(low_margin_cap_no_payoff),
            "examples": event_examples(low_margin_cap_no_payoff[:top_examples]),
        },
        "harmed_examples": simplify_delta_examples(harmed_examples, top_examples),
        "worsened_examples": simplify_delta_examples(
            [example for example in top_worsened if float(example.get("reward_delta") or 0.0) < 0.0],
            top_examples,
        ),
    }


def verified_row(data: dict[str, Any]) -> dict[str, Any]:
    for row in data.get("rows") or []:
        if row.get("kind") == "verified_teacher":
            return row
    policy_summary = data.get("policy_summary") or {}
    if policy_summary:
        policy, summary = next(iter(policy_summary.items()))
        row = dict(summary)
        row["kind"] = "verified_teacher"
        row["case"] = policy
        row["episodes_detail"] = data.get("episodes") or []
        return row
    return {}


def collect_override_events(row: dict[str, Any]) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    for episode in row.get("episodes_detail") or []:
        seed = episode.get("seed")
        result = episode.get("result")
        for event in episode.get("verified_override_events") or []:
            enriched = dict(event)
            enriched["seed"] = seed
            enriched["episode_result"] = result
            events.append(enriched)
    return events


def collect_seed_delta_examples(data: dict[str, Any], key: str) -> list[dict[str, Any]]:
    examples: list[dict[str, Any]] = []
    for report in (data.get("seed_deltas") or {}).values():
        examples.extend(report.get(key) or [])
    return examples


def simplify_delta_examples(examples: list[dict[str, Any]], limit: int) -> list[dict[str, Any]]:
    rows = sorted(examples, key=lambda item: float(item.get("reward_delta") or 0.0))[:limit]
    simplified = []
    for example in rows:
        simplified.append(
            {
                "seed": example.get("seed"),
                "reward_delta": example.get("reward_delta"),
                "rule_result": example.get("rule_result"),
                "teacher_result": example.get("teacher_result"),
                "rule_floor": example.get("rule_floor"),
                "teacher_floor": example.get("teacher_floor"),
                "teacher_overrides": example.get("teacher_overrides"),
                "override_events": event_examples(
                    attach_seed(example.get("teacher_override_events") or [], example.get("seed"))
                ),
            }
        )
    return simplified


def attach_seed(events: list[dict[str, Any]], seed: Any) -> list[dict[str, Any]]:
    enriched = []
    for event in events:
        row = dict(event)
        row.setdefault("seed", seed)
        enriched.append(row)
    return enriched


def event_examples(events: list[dict[str, Any]]) -> list[dict[str, Any]]:
    rows = []
    for event in events:
        rows.append(
            {
                "seed": event.get("seed"),
                "step": event.get("step"),
                "floor": event.get("floor"),
                "hp": event.get("hp"),
                "max_hp": event.get("max_hp"),
                "adv_vs_rule": event.get("adv_vs_rule"),
                "horizon_decisions": event.get("horizon_decisions"),
                "horizon_stop_reason": event.get("horizon_stop_reason"),
                "payoff_reasons": event.get("payoff_reasons") or [],
                "confirmation_kind": event.get("confirmation_kind"),
                "artifact_reasons": event.get("artifact_reasons") or [],
                "context_keys": event.get("context_keys") or [],
                "rule_action_key": event.get("rule_action_key"),
                "selected_action_key": event.get("selected_action_key"),
            }
        )
    return rows


def context_counts(events: list[dict[str, Any]]) -> Counter[str]:
    counts: Counter[str] = Counter()
    prefixes = ("pressure:", "hp_band:", "combat_shape:", "room:", "decision_type:", "pending_choice:")
    for event in events:
        for key in event.get("context_keys") or []:
            if key.startswith(prefixes):
                counts[key] += 1
    return counts


def artifact_reason_counts(events: list[dict[str, Any]]) -> Counter[str]:
    counts: Counter[str] = Counter()
    for event in events:
        for reason in event.get("artifact_reasons") or []:
            counts[reason] += 1
    return counts


def action_pair_kind(event: dict[str, Any]) -> str:
    return f"{action_kind(event.get('rule_action_key'))} -> {action_kind(event.get('selected_action_key'))}"


def action_kind(action_key: Any) -> str:
    if not isinstance(action_key, str) or not action_key:
        return "<missing>"
    if action_key == "combat/end_turn":
        return "end_turn"
    if action_key.startswith("choice/") or "grid_select" in action_key or "hand_select" in action_key:
        return "pending_choice"
    if "card:" in action_key:
        card = action_key.split("card:", 1)[1].split("/", 1)[0]
        if "target:monster_slot" in action_key:
            return f"target:{card}"
        return f"no_target:{card}"
    return action_key.split("/", 1)[0]


def payoff_key(event: dict[str, Any]) -> str:
    reasons = event.get("payoff_reasons") or []
    return "+".join(reasons) if reasons else "<none>"


def adv_bucket(value: Any) -> str:
    adv = float(value or 0.0)
    if adv < 2.0:
        return "<2"
    if adv < 2.25:
        return "2-2.25"
    if adv < 2.5:
        return "2.25-2.5"
    if adv < 3.0:
        return "2.5-3"
    if adv < 5.0:
        return "3-5"
    if adv < 9.0:
        return "5-9"
    return ">=9"


def mean(values: list[float]) -> float | None:
    if not values:
        return None
    return sum(values) / len(values)


def counter_to_dict(counter: Counter[Any] | list[tuple[Any, int]]) -> dict[str, int]:
    items = counter.items() if isinstance(counter, Counter) else counter
    return {str(key): int(value) for key, value in items}


def aggregate_audits(audits: list[dict[str, Any]]) -> dict[str, Any]:
    return {
        "file_count": len(audits),
        "override_count": sum(int(audit["accepted_override_audit"]["count"] or 0) for audit in audits),
        "low_confidence_accepted_count": sum(
            int(audit["low_confidence_accepted"]["cap_stopped_below_threshold_count"] or 0) for audit in audits
        ),
        "low_confidence_no_payoff_count": sum(
            int(audit["low_confidence_accepted"]["cap_stopped_below_threshold_no_payoff_count"] or 0)
            for audit in audits
        ),
        "harmed_example_count": sum(len(audit["harmed_examples"]) for audit in audits),
        "worsened_example_count": sum(len(audit["worsened_examples"]) for audit in audits),
    }


def render_markdown(payload: dict[str, Any], top_examples: int) -> str:
    lines = [
        "# Verified Teacher Override Audit",
        "",
        f"- low_adv_threshold: `{payload['low_adv_threshold']}`",
        f"- files: `{payload['aggregate']['file_count']}`",
        f"- accepted overrides: `{payload['aggregate']['override_count']}`",
        f"- low-confidence accepted: `{payload['aggregate']['low_confidence_accepted_count']}`",
        f"- low-confidence accepted with no payoff: `{payload['aggregate']['low_confidence_no_payoff_count']}`",
        "",
    ]
    for audit in payload["files"]:
        summary = audit["summary"]
        accepted = audit["accepted_override_audit"]
        low = audit["low_confidence_accepted"]
        lines.extend(
            [
                f"## {Path(audit['input']).name}",
                "",
                (
                    f"- reward `{summary.get('average_total_reward')}`, defeats `{summary.get('defeat_count')}`, "
                    f"overrides `{summary.get('override_count')}`, low rejects `{summary.get('low_evidence_reject_count')}`"
                ),
                (
                    f"- confirm decisions `{summary.get('confirm_decision_count')}`, accepts "
                    f"`{summary.get('confirm_accept_count')}`, rejects `{summary.get('confirm_reject_count')}`"
                ),
                f"- stop reasons: `{accepted['horizon_stop_reason_counts']}`",
                f"- adv buckets: `{accepted['adv_bucket_counts']}`",
                f"- payoff top: `{top_items(accepted['payoff_reason_counts'])}`",
                f"- confirmation kinds: `{accepted['confirmation_kind_counts']}`",
                f"- artifact reasons: `{top_items(accepted['artifact_reason_counts'])}`",
                (
                    f"- low-confidence accepted below threshold: "
                    f"`{low['cap_stopped_below_threshold_count']}`; no-payoff "
                    f"`{low['cap_stopped_below_threshold_no_payoff_count']}`"
                ),
                "",
            ]
        )
        if audit["harmed_examples"]:
            lines.append("### Harmed Examples")
            for example in audit["harmed_examples"][:top_examples]:
                lines.append(render_example(example))
            lines.append("")
        if low["examples"]:
            lines.append("### Low-Confidence Accepted Examples")
            for event in low["examples"][:top_examples]:
                lines.append(render_event(event))
            lines.append("")
    return "\n".join(lines) + "\n"


def top_items(mapping: dict[str, int], limit: int = 6) -> dict[str, int]:
    return dict(sorted(mapping.items(), key=lambda item: item[1], reverse=True)[:limit])


def render_example(example: dict[str, Any]) -> str:
    lines = [
        (
            f"- seed `{example.get('seed')}` delta `{example.get('reward_delta')}` "
            f"{example.get('rule_result')} -> {example.get('teacher_result')}"
        )
    ]
    for event in example.get("override_events") or []:
        lines.append(f"  - {render_event(event)}")
    return "\n".join(lines)


def render_event(event: dict[str, Any]) -> str:
    return (
        f"seed `{event.get('seed')}` step `{event.get('step')}` floor `{event.get('floor')}` "
        f"hp `{event.get('hp')}/{event.get('max_hp')}` adv `{event.get('adv_vs_rule')}` "
        f"stop `{event.get('horizon_stop_reason')}` payoff `{event.get('payoff_reasons')}` "
        f"confirm `{event.get('confirmation_kind')}` artifact `{event.get('artifact_reasons')}` "
        f"rule `{event.get('rule_action_key')}` selected `{event.get('selected_action_key')}`"
    )


if __name__ == "__main__":
    main()
