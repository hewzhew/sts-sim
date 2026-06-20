#!/usr/bin/env python3
"""Export CombatTacticalEpisodeV1 records from turn-plan guidance-lab reports.

This is a diagnostic/learning handoff, not a policy script.  It keeps the
simulator report as the fact source, derives tactical deltas deterministically,
and records counterfactual fields only relative to the candidate set available
in the same root report.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any, Iterable


EPISODE_SCHEMA = "CombatTacticalEpisodeV1"
EPISODE_VERSION = 1
LABEL_ROLE = "diagnostic_tactical_trace_not_policy_label"
PUBLIC_MONSTER_FIELDS = (
    "slot",
    "enemy_id",
    "hp",
    "max_hp",
    "block",
    "alive",
    "escaped",
    "dying",
    "half_dead",
    "visible_intent",
    "preview_damage_per_hit",
)


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def iter_labs(path: Path, payload: Any) -> Iterable[tuple[dict[str, Any], dict[str, Any]]]:
    if not isinstance(payload, dict):
        return
    schema = payload.get("schema_name")
    if schema == "CombatTurnPlanGuidanceLabV1Report":
        yield (
            {
                "source_file": str(path),
                "benchmark_name": None,
                "case_id": None,
                "input_kind": None,
                "input_path": None,
            },
            payload,
        )
        return
    if schema == "CombatTurnPlanGuidanceLabBenchmarkV1Report":
        benchmark_name = payload.get("benchmark_name")
        for case in payload.get("cases", []):
            if not isinstance(case, dict) or not isinstance(case.get("lab"), dict):
                continue
            yield (
                {
                    "source_file": str(path),
                    "benchmark_name": benchmark_name,
                    "case_id": case.get("id"),
                    "input_kind": case.get("input_kind"),
                    "input_path": case.get("input_path"),
                },
                case["lab"],
            )


def resolve_input_path(report_path: Path, input_path: Any) -> Path | None:
    if not isinstance(input_path, str) or not input_path:
        return None
    path = Path(input_path)
    if path.exists() or path.is_absolute():
        return path
    candidate = report_path.parent / path
    if candidate.exists():
        return candidate
    return path


def public_enemy_slots_from_capture(path: Path | None) -> list[dict[str, Any]]:
    if path is None or not path.exists():
        return []
    try:
        payload = load_json(path)
    except (OSError, json.JSONDecodeError):
        return []
    summary = payload.get("summary") if isinstance(payload, dict) else {}
    monsters = summary.get("monsters") if isinstance(summary, dict) else []
    if not isinstance(monsters, list):
        return []
    out: list[dict[str, Any]] = []
    for monster in monsters:
        if not isinstance(monster, dict):
            continue
        public = {field: monster.get(field) for field in PUBLIC_MONSTER_FIELDS if field in monster}
        if public:
            out.append(public)
    return out


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def int_value(value: Any, default: int = 0) -> int:
    return value if isinstance(value, int) else default


def int_or_none(value: Any) -> int | None:
    return value if isinstance(value, int) else None


def bool_value(value: Any) -> bool:
    return value if isinstance(value, bool) else False


def target_sort_key(candidate: dict[str, Any]) -> tuple[int, int, int, int]:
    target = as_dict(candidate.get("target"))
    terminal = target.get("terminal")
    tier = 0
    if target.get("complete_win") and terminal == "win":
        tier = 3
    elif terminal == "win":
        tier = 2
    elif terminal == "unresolved":
        tier = 1
    final_hp = int_value(target.get("final_hp"), -10**9)
    child_hp_loss = int_value(target.get("child_search_hp_loss"), 10**9)
    nodes = int_value(target.get("nodes_expanded"), 10**9)
    return (tier, final_hp, -child_hp_loss, -nodes)


def plan_index(candidate: dict[str, Any]) -> int:
    return int_value(as_dict(candidate.get("plan")).get("plan_index"), 10**9)


def state_delta(before: dict[str, Any], after: dict[str, Any]) -> dict[str, Any]:
    fields = (
        "player_hp",
        "player_block",
        "energy",
        "turn_count",
        "living_enemy_count",
        "total_enemy_hp",
        "visible_incoming_damage",
        "hand_count",
        "draw_count",
        "discard_count",
        "exhaust_count",
        "limbo_count",
        "queued_cards_count",
    )
    return {
        field: int_value(after.get(field)) - int_value(before.get(field))
        for field in fields
        if isinstance(before.get(field), int) and isinstance(after.get(field), int)
    }


def sum_exact_deltas(action_facts: list[dict[str, Any]]) -> dict[str, int]:
    fields = (
        "player_hp_delta",
        "player_block_delta",
        "energy_delta",
        "hand_delta",
        "draw_delta",
        "discard_delta",
        "exhaust_delta",
        "limbo_delta",
        "queued_cards_delta",
        "total_enemy_hp_delta",
        "total_enemy_block_delta",
    )
    out = {field: 0 for field in fields}
    for facts in action_facts:
        exact = as_dict(facts.get("exact_one_step_delta"))
        for field in fields:
            out[field] += int_value(exact.get(field))
    return out


def action_kind_counts(action_facts: list[dict[str, Any]], actions: list[dict[str, Any]]) -> Counter[str]:
    counts: Counter[str] = Counter()
    for index, action in enumerate(actions):
        facts = action_facts[index] if index < len(action_facts) else {}
        kind = facts.get("action_kind") or action_kind_from_key(str(action.get("action_key") or ""))
        counts[str(kind)] += 1
    return counts


def action_kind_from_key(action_key: str) -> str:
    if "/play_card/" in action_key or action_key.startswith("combat/play_card"):
        return "play_card"
    if "/use_potion/" in action_key or action_key.startswith("combat/use_potion"):
        return "use_potion"
    if "/discard_potion/" in action_key or action_key.startswith("combat/discard_potion"):
        return "discard_potion"
    if action_key.endswith("/end_turn") or action_key == "combat/end_turn":
        return "end_turn"
    return "unknown"


def plan_tactical_summary(
    root_state: dict[str, Any],
    plan: dict[str, Any],
    action_facts: list[dict[str, Any]],
) -> dict[str, Any]:
    end_state = as_dict(plan.get("end_state"))
    root_to_end = state_delta(root_state, end_state)
    exact_sums = sum_exact_deltas(action_facts)
    actions = [action for action in as_list(plan.get("actions")) if isinstance(action, dict)]
    counts = action_kind_counts(action_facts, actions)
    player_hp_delta = root_to_end.get("player_hp", exact_sums.get("player_hp_delta", 0))
    total_enemy_hp_delta = root_to_end.get(
        "total_enemy_hp",
        exact_sums.get("total_enemy_hp_delta", 0),
    )
    living_enemy_delta = root_to_end.get("living_enemy_count", 0)
    target_slots = []
    damage_hint_total = 0
    block_hint_total = 0
    mitigation_hint_total = 0
    for facts in action_facts:
        target = as_dict(facts.get("target"))
        if isinstance(target.get("target_slot"), int):
            target_slots.append(target["target_slot"])
        immediate = as_dict(facts.get("immediate"))
        mechanics = as_dict(facts.get("mechanics"))
        damage_hint_total += int_value(immediate.get("action_payload_damage_hint"))
        block_hint_total += int_value(immediate.get("block_hint"))
        mitigation_hint_total += int_value(mechanics.get("visible_attack_mitigation_hint"))
    return {
        "data_role": "DerivedDeterministic",
        "availability": "EndOfPlan",
        "root_to_end_delta": root_to_end,
        "exact_step_delta_sum": exact_sums if action_facts else None,
        "action_kind_counts": dict(counts),
        "cards_played": counts.get("play_card", 0),
        "potion_actions": counts.get("use_potion", 0) + counts.get("discard_potion", 0),
        "hp_lost_to_plan_boundary": max(0, -player_hp_delta),
        "enemy_hp_removed_to_plan_boundary": max(0, -total_enemy_hp_delta),
        "enemy_kill_count_to_plan_boundary": max(0, -living_enemy_delta),
        "visible_incoming_boundary_delta": root_to_end.get("visible_incoming_damage"),
        "damage_hint_total": damage_hint_total,
        "block_hint_total": block_hint_total,
        "visible_attack_mitigation_hint_total": mitigation_hint_total,
        "target_slots": target_slots,
        "unique_target_slots": sorted(set(target_slots)),
        "all_enemies_dead_at_plan_boundary": end_state.get("terminal") == "win"
        or int_value(end_state.get("living_enemy_count")) == 0,
        "energy_unspent_at_plan_boundary": int_or_none(end_state.get("energy")),
    }


def step_trace(
    action: dict[str, Any],
    facts: dict[str, Any] | None,
    state_before: dict[str, Any] | None = None,
    state_after: dict[str, Any] | None = None,
) -> dict[str, Any]:
    exact = as_dict(facts.get("exact_one_step_delta")) if isinstance(facts, dict) else {}
    has_state_summary = isinstance(state_before, dict) and isinstance(state_after, dict)
    return {
        "step_index": action.get("step_index"),
        "action": {
            "data_role": "ObservedExact",
            "availability": "BeforeStep",
            "action_id": action.get("action_id"),
            "action_key": action.get("action_key"),
            "action_debug": action.get("action_debug"),
            "input": action.get("input"),
        },
        "state_before_ref": None,
        "state_after_ref": None,
        "state_before_summary": state_before if isinstance(state_before, dict) else None,
        "state_after_summary": state_after if isinstance(state_after, dict) else None,
        "state_snapshot_availability": (
            "summary_recorded_exact_state_ref_not_exported"
            if has_state_summary
            else "not_recorded_in_current_turn_plan_report"
        ),
        "action_facts": facts,
        "tactical_delta": {
            "data_role": "DerivedDeterministic" if exact else "Unavailable",
            "availability": "AfterStep" if exact else "not_recorded",
            "exact_one_step_delta": exact or None,
        },
    }


def candidate_trace(
    root_state: dict[str, Any],
    candidate: dict[str, Any],
) -> dict[str, Any]:
    plan = as_dict(candidate.get("plan"))
    source_steps = [step for step in as_list(plan.get("steps")) if isinstance(step, dict)]
    if source_steps:
        actions = [as_dict(step.get("action")) for step in source_steps]
        action_facts = [as_dict(step.get("action_facts")) for step in source_steps]
        state_pairs = [
            (as_dict(step.get("state_before")), as_dict(step.get("state_after")))
            for step in source_steps
        ]
    else:
        actions = [action for action in as_list(plan.get("actions")) if isinstance(action, dict)]
        action_facts = [facts for facts in as_list(plan.get("action_facts")) if isinstance(facts, dict)]
        state_pairs = [(None, None) for _ in actions]
    target = as_dict(candidate.get("target"))
    child_search = as_dict(candidate.get("child_search"))
    limitations = []
    if not action_facts:
        limitations.append("action_facts_not_available_in_source_report")
    if len(action_facts) != len(actions):
        limitations.append("action_facts_count_does_not_match_action_count")
    if source_steps and len(state_pairs) == len(actions):
        limitations.append("exact_state_refs_hashes_not_available_for_steps")
    else:
        limitations.append("state_before_after_refs_not_available_in_current_turn_plan_report")
    plan_id = f"plan:{plan.get('plan_index')}"
    steps = []
    for index, action in enumerate(actions):
        facts = action_facts[index] if index < len(action_facts) else None
        state_before, state_after = state_pairs[index] if index < len(state_pairs) else (None, None)
        steps.append(step_trace(action, facts, state_before, state_after))
    return {
        "plan_id": plan_id,
        "plan_index": plan.get("plan_index"),
        "generation": {
            "source": "TurnPlanEnumerator",
            "bucket": plan.get("bucket"),
            "stop_reason": plan.get("stop_reason"),
            "outcome_class": plan.get("outcome_class"),
            "survival_bucket": plan.get("survival_bucket"),
            "progress_bucket": plan.get("progress_bucket"),
        },
        "steps": steps,
        "plan_summary": plan_tactical_summary(root_state, plan, action_facts),
        "final_state_ref": None,
        "final_state_hash": None,
        "final_state_summary": plan.get("end_state"),
        "outcome_attachment": {
            "data_role": "SearchLabel",
            "availability": "PostSearch",
            "source": target.get("source"),
            "target_kind": target.get("target_kind"),
            "terminal": target.get("terminal"),
            "complete_win": target.get("complete_win"),
            "post_root_player_hp": target.get("post_root_player_hp"),
            "child_search_hp_loss": target.get("child_search_hp_loss"),
            "final_hp": target.get("final_hp"),
            "nodes_expanded": target.get("nodes_expanded"),
            "limitations": target.get("limitations") or [],
            "child_search": child_search or None,
        },
        "counterfactual": {},
        "limitations": limitations,
    }


def pareto_plan_ids(traces: list[dict[str, Any]]) -> list[str]:
    frontier = []
    summaries = [(trace, as_dict(trace.get("plan_summary"))) for trace in traces]
    for trace, summary in summaries:
        dominated = False
        hp_loss = int_value(summary.get("hp_lost_to_plan_boundary"))
        enemy_hp_removed = int_value(summary.get("enemy_hp_removed_to_plan_boundary"))
        kills = int_value(summary.get("enemy_kill_count_to_plan_boundary"))
        potion_actions = int_value(summary.get("potion_actions"))
        final_hp = int_value(
            as_dict(trace.get("outcome_attachment")).get("final_hp"),
            -10**9,
        )
        for other, other_summary in summaries:
            if other is trace:
                continue
            other_hp_loss = int_value(other_summary.get("hp_lost_to_plan_boundary"))
            other_enemy_hp_removed = int_value(other_summary.get("enemy_hp_removed_to_plan_boundary"))
            other_kills = int_value(other_summary.get("enemy_kill_count_to_plan_boundary"))
            other_potion_actions = int_value(other_summary.get("potion_actions"))
            other_final_hp = int_value(
                as_dict(other.get("outcome_attachment")).get("final_hp"),
                -10**9,
            )
            at_least_as_good = (
                other_hp_loss <= hp_loss
                and other_enemy_hp_removed >= enemy_hp_removed
                and other_kills >= kills
                and other_potion_actions <= potion_actions
                and other_final_hp >= final_hp
            )
            strictly_better = (
                other_hp_loss < hp_loss
                or other_enemy_hp_removed > enemy_hp_removed
                or other_kills > kills
                or other_potion_actions < potion_actions
                or other_final_hp > final_hp
            )
            if at_least_as_good and strictly_better:
                dominated = True
                break
        if not dominated:
            frontier.append(str(trace.get("plan_id")))
    return frontier


def root_tactical_context(traces: list[dict[str, Any]]) -> dict[str, Any]:
    summaries = [as_dict(trace.get("plan_summary")) for trace in traces]
    outcomes = [as_dict(trace.get("outcome_attachment")) for trace in traces]
    hp_losses = [int_value(summary.get("hp_lost_to_plan_boundary")) for summary in summaries]
    enemy_removed = [int_value(summary.get("enemy_hp_removed_to_plan_boundary")) for summary in summaries]
    kills = [int_value(summary.get("enemy_kill_count_to_plan_boundary")) for summary in summaries]
    final_hps = [
        outcome.get("final_hp")
        for outcome in outcomes
        if isinstance(outcome.get("final_hp"), int)
    ]
    pareto = pareto_plan_ids(traces)
    best_hp_loss = min(hp_losses) if hp_losses else None
    best_enemy_removed = max(enemy_removed) if enemy_removed else None
    best_kills = max(kills) if kills else None
    best_final_hp = max(final_hps) if final_hps else None
    for trace in traces:
        summary = as_dict(trace.get("plan_summary"))
        outcome = as_dict(trace.get("outcome_attachment"))
        counterfactual = {
            "data_role": "Counterfactual",
            "availability": "PostSearch",
            "candidate_set_scope": "same_root_bounded_turn_plan_candidates",
            "is_on_simple_pareto_frontier": trace.get("plan_id") in pareto,
        }
        if best_hp_loss is not None:
            counterfactual["hp_loss_regret_vs_best_boundary"] = int_value(
                summary.get("hp_lost_to_plan_boundary")
            ) - best_hp_loss
        if best_enemy_removed is not None:
            counterfactual["enemy_hp_progress_gap_vs_best_boundary"] = best_enemy_removed - int_value(
                summary.get("enemy_hp_removed_to_plan_boundary")
            )
        if best_kills is not None:
            counterfactual["kill_count_gap_vs_best_boundary"] = best_kills - int_value(
                summary.get("enemy_kill_count_to_plan_boundary")
            )
        if best_final_hp is not None and isinstance(outcome.get("final_hp"), int):
            counterfactual["final_hp_regret_vs_best_labeled"] = best_final_hp - outcome["final_hp"]
        trace["counterfactual"] = counterfactual
    return {
        "data_role": "Counterfactual",
        "availability": "PostSearch",
        "candidate_count": len(traces),
        "terminal_win_plan_exists": any(
            as_dict(trace.get("plan_summary")).get("all_enemies_dead_at_plan_boundary")
            for trace in traces
        ),
        "complete_win_label_exists": any(outcome.get("complete_win") for outcome in outcomes),
        "no_hp_loss_to_boundary_candidate_exists": any(loss == 0 for loss in hp_losses),
        "no_potion_candidate_exists": any(
            int_value(summary.get("potion_actions")) == 0 for summary in summaries
        ),
        "best_hp_loss_to_boundary": best_hp_loss,
        "best_enemy_hp_removed_to_boundary": best_enemy_removed,
        "best_enemy_kill_count_to_boundary": best_kills,
        "best_final_hp_labeled": best_final_hp,
        "pareto_frontier_plan_ids": pareto,
        "limitations": [
            "counterfactuals_are_relative_to_bounded_candidate_set_not_global_optimum",
        ],
    }


def episode_from_lab(meta: dict[str, Any], lab: dict[str, Any]) -> dict[str, Any]:
    report_path = Path(str(meta.get("source_file") or ""))
    input_path = resolve_input_path(report_path, meta.get("input_path"))
    enemy_slots = public_enemy_slots_from_capture(input_path)
    root = as_dict(lab.get("root"))
    initial_context = as_dict(root.get("initial_context"))
    root_state = as_dict(initial_context.get("state"))
    candidates = [candidate for candidate in as_list(lab.get("candidates")) if isinstance(candidate, dict)]
    traces = [candidate_trace(root_state, candidate) for candidate in candidates]
    context = root_tactical_context(traces)
    limitations = [
        "exact_state_refs_and_hashes_not_exported_by_current_turn_plan_report",
    ]
    if not enemy_slots:
        limitations.append("enemy_slot_public_view_not_available_from_capture")
    if any("action_facts_not_available_in_source_report" in trace["limitations"] for trace in traces):
        limitations.append("some_candidate_action_facts_missing")
    return {
        "schema_name": EPISODE_SCHEMA,
        "schema_version": EPISODE_VERSION,
        "label_role": LABEL_ROLE,
        "source": {
            **meta,
            "input_label": lab.get("input_label"),
        },
        "provenance": {
            "data_role": "ObservedExact",
            "candidate_generator_id": as_dict(root.get("enumeration")).get("planning_policy"),
            "search_config": root.get("config"),
            "root_report_schema": root.get("schema_name"),
            "lab_schema": lab.get("schema_name"),
            "policy_quality_claim": lab.get("policy_quality_claim"),
            "notes": lab.get("notes") or [],
        },
        "root": {
            "exact_state_ref": None,
            "exact_state_hash": None,
            "public_view": {
                "data_role": "ObservedExact",
                "availability": "RootOnly",
                "state": root_state,
                "phase_profile": initial_context.get("phase_profile"),
                "frontier_value": initial_context.get("frontier_value"),
                "enemy_slots": enemy_slots,
            },
            "legal_action_mask": None,
        },
        "candidate_plans": traces,
        "root_tactical_context": context,
        "label_bundle": {
            "data_role": "SearchLabel",
            "availability": "PostSearch",
            "source": "bounded_child_search_targets_in_turn_plan_guidance_lab",
            "summary": lab.get("summary"),
            "limitations": [
                "labels_are_oracle_under_current_simulator_and_budget_not_human_policy",
            ],
        },
        "limitations": limitations,
    }


def extract(
    inputs: list[Path],
    out_jsonl: Path | None,
    *,
    summary_only: bool,
    case_limit: int,
) -> None:
    episodes: list[dict[str, Any]] = []
    for path in inputs:
        for meta, lab in iter_labs(path, load_json(path)):
            episodes.append(episode_from_lab(meta, lab))

    if out_jsonl:
        out_jsonl.parent.mkdir(parents=True, exist_ok=True)
        with out_jsonl.open("w", encoding="utf-8") as handle:
            for episode in episodes:
                handle.write(json.dumps(episode, ensure_ascii=False, separators=(",", ":")))
                handle.write("\n")

    counters: Counter[str] = Counter()
    total_candidates = 0
    for episode in episodes:
        candidates = as_list(episode.get("candidate_plans"))
        total_candidates += len(candidates)
        if as_dict(as_dict(episode.get("root")).get("public_view")).get("enemy_slots"):
            counters["episodes_with_enemy_slots"] += 1
        if any(as_list(plan.get("steps")) and as_dict(as_list(plan.get("steps"))[0]).get("action_facts") for plan in candidates):
            counters["episodes_with_action_facts"] += 1
        context = as_dict(episode.get("root_tactical_context"))
        if context.get("no_hp_loss_to_boundary_candidate_exists"):
            counters["episodes_with_no_hp_loss_candidate"] += 1
        if context.get("complete_win_label_exists"):
            counters["episodes_with_complete_win_label"] += 1
    print("CombatTacticalTraceExtract")
    print(f"  episodes={len(episodes)} candidates={total_candidates}")
    print(f"  episodes_with_enemy_slots={counters['episodes_with_enemy_slots']}")
    print(f"  episodes_with_action_facts={counters['episodes_with_action_facts']}")
    print(f"  episodes_with_no_hp_loss_candidate={counters['episodes_with_no_hp_loss_candidate']}")
    print(f"  episodes_with_complete_win_label={counters['episodes_with_complete_win_label']}")
    if out_jsonl:
        print(f"  jsonl={out_jsonl}")
    if summary_only:
        return
    print("  cases:")
    for episode in episodes[:case_limit]:
        source = as_dict(episode.get("source"))
        context = as_dict(episode.get("root_tactical_context"))
        print(f"    case={source.get('case_id') or source.get('input_label')}")
        print(
            "      "
            f"candidates={context.get('candidate_count')} "
            f"best_hp_loss={context.get('best_hp_loss_to_boundary')} "
            f"best_enemy_removed={context.get('best_enemy_hp_removed_to_boundary')} "
            f"best_final_hp={context.get('best_final_hp_labeled')} "
            f"pareto={len(as_list(context.get('pareto_frontier_plan_ids')))}"
        )
    if len(episodes) > case_limit:
        print(f"    ... {len(episodes) - case_limit} more episode(s)")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="+", type=Path)
    parser.add_argument("--out-jsonl", type=Path)
    parser.add_argument("--summary-only", action="store_true")
    parser.add_argument("--case-limit", type=int, default=12)
    args = parser.parse_args()
    extract(
        args.inputs,
        args.out_jsonl,
        summary_only=args.summary_only,
        case_limit=max(0, args.case_limit),
    )


if __name__ == "__main__":
    main()
