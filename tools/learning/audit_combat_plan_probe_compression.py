#!/usr/bin/env python3
"""Batch audit combat turn plan-probe search compression.

This is a diagnostic harness, not a policy trainer.  It runs the Rust
`combat plan-probe` command over selected full-run combat decision points and
aggregates how much work was pruned by exact equivalence, abstract equivalence,
budget, and optimistic bounds.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from combat_reranker_common import write_json, write_jsonl

REPO_ROOT = Path(__file__).resolve().parents[2]
REPORT_VERSION = "combat_plan_probe_compression_audit_v0"

ACTION_SPACE_CARD_IDS = {
    "Acrobatics",
    "Adrenaline",
    "Backflip",
    "BattleTrance",
    "BurningPact",
    "DeepBreath",
    "Discovery",
    "DaggerThrow",
    "Dropkick",
    "Finesse",
    "FlashOfSteel",
    "Forethought",
    "Impatience",
    "InfernalBlade",
    "JackOfAllTrades",
    "MasterOfStrategy",
    "Offering",
    "PommelStrike",
    "Prepared",
    "SecretTechnique",
    "SecretWeapon",
    "ShrugItOff",
    "ThinkingAhead",
    "Violence",
    "Warcry",
}

ZONE_MUTATION_SEMANTICS = {
    "creates_cards",
    "produces_status",
    "self_replicating",
    "draw",
}

PRESSURE_ORDER = [
    "medium_pressure",
    "chip_pressure",
    "no_attack",
    "blocked_attack",
    "high_pressure",
    "lethal_pressure",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run combat turn plan-probe over a small batch of full-run combat decisions "
            "and summarize search compression/pruning behavior."
        )
    )
    source = parser.add_mutually_exclusive_group(required=False)
    source.add_argument("--trace-file", type=Path)
    source.add_argument("--trace-dir", type=Path)
    parser.add_argument(
        "--generate-episodes",
        type=int,
        default=0,
        help="If no trace source is provided, generate this many run-batch traces first.",
    )
    parser.add_argument("--generate-seed", type=int, default=76000)
    parser.add_argument("--generate-policy", default="random_masked")
    parser.add_argument("--generate-max-steps", type=int, default=160)
    parser.add_argument("--max-cases", type=int, default=50)
    parser.add_argument("--per-trace-limit", type=int, default=10)
    parser.add_argument("--min-candidates", type=int, default=2)
    parser.add_argument("--min-step-gap", type=int, default=3)
    parser.add_argument(
        "--case-strategy",
        default="balanced_pressure",
        choices=["trace_order", "danger", "balanced_pressure"],
    )
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=5000)
    parser.add_argument("--max-depth", type=int, default=4)
    parser.add_argument("--max-nodes", type=int, default=500)
    parser.add_argument(
        "--expansion-rerun-nodes",
        type=int,
        default=1000,
        help=(
            "If a budget-pruned case has action-space/zone-mutation cards, rerun that case "
            "with this max-node budget and report plan-query deltas. Set to 0 to disable."
        ),
    )
    parser.add_argument("--beam-width", type=int, default=16)
    parser.add_argument("--max-engine-steps-per-action", type=int, default=200)
    parser.add_argument("--sts-dev-tool", type=Path)
    parser.add_argument("--force-cargo-run", action="store_true")
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "combat_plan_probe_compression",
    )
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def sts_dev_tool_cmd(args: argparse.Namespace) -> list[str]:
    if args.sts_dev_tool:
        return [str(args.sts_dev_tool)]
    exe_name = "sts_dev_tool.exe" if sys.platform.startswith("win") else "sts_dev_tool"
    debug_exe = REPO_ROOT / "target" / "debug" / exe_name
    release_exe = REPO_ROOT / "target" / "release" / exe_name
    if not args.force_cargo_run and debug_exe.exists():
        return [str(debug_exe)]
    if not args.force_cargo_run and release_exe.exists():
        return [str(release_exe)]
    return ["cargo", "run", "--quiet", "--bin", "sts_dev_tool", "--"]


def run_generated_traces(args: argparse.Namespace, run_dir: Path) -> Path:
    trace_dir = run_dir / "generated_traces"
    summary_path = run_dir / "run_batch_summary.json"
    cmd = [
        *sts_dev_tool_cmd(args),
        "run-batch",
        "--episodes",
        str(args.generate_episodes),
        "--seed",
        str(args.generate_seed),
        "--policy",
        args.generate_policy,
        "--max-steps",
        str(args.generate_max_steps),
        "--trace-dir",
        str(trace_dir),
        "--summary-out",
        str(summary_path),
    ]
    subprocess.run(cmd, cwd=str(REPO_ROOT), check=True, text=True)
    return trace_dir


def trace_files(args: argparse.Namespace, run_dir: Path) -> list[Path]:
    if args.trace_file:
        return [resolve_path(args.trace_file)]
    if args.trace_dir:
        root = resolve_path(args.trace_dir)
    elif args.generate_episodes > 0:
        root = run_generated_traces(args, run_dir)
    else:
        raise SystemExit("provide --trace-file/--trace-dir or set --generate-episodes > 0")
    files = sorted(root.glob("episode_*.json"))
    if not files:
        files = sorted(root.rglob("episode_*.json"))
    if not files:
        raise SystemExit(f"no episode_*.json files found in {root}")
    return files


def resolve_path(path: Path) -> Path:
    return path if path.is_absolute() else REPO_ROOT / path


def combat_obs(step: dict[str, Any]) -> dict[str, Any]:
    obs = step.get("observation") or {}
    return obs.get("combat") or {}


def legal_count(step: dict[str, Any]) -> int:
    return int(step.get("legal_action_count") or len(step.get("action_mask") or []))


def hp(step: dict[str, Any]) -> int:
    obs = step.get("observation") or {}
    combat = combat_obs(step)
    return int(obs.get("current_hp") or combat.get("player_hp") or step.get("hp") or 0)


def visible_incoming(step: dict[str, Any]) -> int:
    return int(combat_obs(step).get("visible_incoming_damage") or 0)


def unblocked_damage(step: dict[str, Any]) -> int:
    combat = combat_obs(step)
    incoming = int(combat.get("visible_incoming_damage") or 0)
    block = int(combat.get("player_block") or 0)
    return max(incoming - block, 0)


def pressure_class(step: dict[str, Any]) -> str:
    incoming = visible_incoming(step)
    unblocked = unblocked_damage(step)
    current_hp = hp(step)
    if incoming <= 0:
        return "no_attack"
    if unblocked <= 0:
        return "blocked_attack"
    if current_hp > 0 and unblocked >= current_hp:
        return "lethal_pressure"
    if current_hp > 0 and unblocked >= max(current_hp // 2, 1):
        return "high_pressure"
    if unblocked >= 6:
        return "medium_pressure"
    return "chip_pressure"


def case_priority(step: dict[str, Any]) -> tuple[float, int, int, int]:
    current_hp = max(hp(step), 1)
    unblocked = unblocked_damage(step)
    combat = combat_obs(step)
    return (
        unblocked / current_hp,
        unblocked,
        legal_count(step),
        int(combat.get("total_monster_hp") or 0),
    )


def ordered_steps(candidates: list[tuple[tuple[float, int, int, int], dict[str, Any]]], strategy: str) -> list[dict[str, Any]]:
    if strategy == "danger":
        return [step for _priority, step in sorted(candidates, key=lambda item: item[0], reverse=True)]
    if strategy == "balanced_pressure":
        buckets: dict[str, list[dict[str, Any]]] = {name: [] for name in PRESSURE_ORDER}
        for priority, step in sorted(candidates, key=lambda item: item[0], reverse=True):
            buckets.setdefault(pressure_class(step), []).append(step)
        ordered: list[dict[str, Any]] = []
        while any(buckets.values()):
            for name in PRESSURE_ORDER:
                if buckets.get(name):
                    ordered.append(buckets[name].pop(0))
        return ordered
    return [step for _priority, step in candidates]


def step_to_case(path: Path, trace: dict[str, Any], step: dict[str, Any]) -> dict[str, Any]:
    step_index = int(step.get("step_index") or 0)
    combat = combat_obs(step)
    return {
        "case_id": f"{path.stem}_step_{step_index:04}",
        "trace_file": str(path),
        "seed": int((trace.get("summary") or {}).get("seed") or trace.get("seed") or 0),
        "step_index": step_index,
        "floor": int(step.get("floor") or 0),
        "act": int(step.get("act") or 0),
        "hp": hp(step),
        "incoming": visible_incoming(step),
        "unblocked": unblocked_damage(step),
        "pressure_class": pressure_class(step),
        "turn_count": int(combat.get("turn_count") or 0),
        "monster_hp": int(combat.get("total_monster_hp") or 0),
        "living_monsters": int(combat.get("alive_monster_count") or 0),
        "candidate_count": legal_count(step),
        "chosen_action_key": str(step.get("chosen_action_key") or ""),
    }


def select_cases(args: argparse.Namespace, files: list[Path]) -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []
    for path in files:
        trace = read_json(path)
        candidates = []
        for step in trace.get("steps") or []:
            if str(step.get("decision_type") or "") != "combat":
                continue
            if str(step.get("engine_state") or "") != "combat_player_turn":
                continue
            if legal_count(step) < args.min_candidates:
                continue
            candidates.append((case_priority(step), step))
        selected_steps: list[int] = []
        per_trace = 0
        for step in ordered_steps(candidates, args.case_strategy):
            step_index = int(step.get("step_index") or 0)
            if any(abs(step_index - selected) < args.min_step_gap for selected in selected_steps):
                continue
            cases.append(step_to_case(path, trace, step))
            selected_steps.append(step_index)
            per_trace += 1
            if per_trace >= args.per_trace_limit or len(cases) >= args.max_cases:
                break
        if len(cases) >= args.max_cases:
            break
    return cases


def run_plan_probe(
    args: argparse.Namespace,
    case: dict[str, Any],
    report_path: Path,
    max_nodes: int,
) -> dict[str, Any]:
    cmd = [
        *sts_dev_tool_cmd(args),
        "combat",
        "plan-probe",
        "--trace-file",
        case["trace_file"],
        "--step-index",
        str(case["step_index"]),
        "--out",
        str(report_path),
        "--ascension",
        str(args.ascension),
        "--class",
        args.player_class,
        "--max-steps",
        str(args.max_steps),
        "--max-depth",
        str(args.max_depth),
        "--max-nodes",
        str(max_nodes),
        "--beam-width",
        str(args.beam_width),
        "--max-engine-steps-per-action",
        str(args.max_engine_steps_per_action),
    ]
    if args.final_act:
        cmd.append("--final-act")
    proc = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    if proc.returncode != 0:
        return {
            "case": case,
            "status": "failed",
            "error": proc.stderr.strip() or proc.stdout.strip(),
            "report_path": str(report_path),
            "max_nodes": max_nodes,
        }
    return {
        "case": case,
        "status": "ok",
        "report_path": str(report_path),
        "report": read_json(report_path),
        "max_nodes": max_nodes,
    }


def report_action_space_cards(report: dict[str, Any]) -> list[str]:
    cards = []
    for card in report.get("hand_cards") or []:
        card_id = str(card.get("card_id") or "")
        semantics = {str(item) for item in card.get("base_semantics") or []}
        transient = {str(item) for item in card.get("transient_tags") or []}
        if card_id in ACTION_SPACE_CARD_IDS or semantics & ZONE_MUTATION_SEMANTICS or transient & ZONE_MUTATION_SEMANTICS:
            cards.append(card_id)
    return sorted(set(cards))


def should_expansion_rerun(args: argparse.Namespace, report: dict[str, Any]) -> tuple[bool, list[str]]:
    if args.expansion_rerun_nodes <= 0 or args.expansion_rerun_nodes <= args.max_nodes:
        return (False, [])
    limits = report.get("probe_limits") or {}
    if int(limits.get("pruned_by_budget") or 0) <= 0:
        return (False, [])
    cards = report_action_space_cards(report)
    return (bool(cards), cards)


def query_summary(query: dict[str, Any]) -> dict[str, Any]:
    outcome = query.get("outcome") or {}
    return {
        "status": query.get("status"),
        "best_action_keys": list(query.get("best_action_keys") or []),
        "needs_deeper_search": bool(query.get("needs_deeper_search")),
        "failed_constraints": list(query.get("failed_constraints") or []),
        "notes": list(query.get("notes") or []),
        "outcome": {
            key: outcome.get(key)
            for key in [
                "damage_done",
                "projected_unblocked_damage",
                "remaining_energy",
                "enemy_deaths",
                "living_monster_count",
                "total_monster_hp",
                "played_setup_or_scaling",
            ]
            if key in outcome
        },
    }


def plan_query_deltas(base_report: dict[str, Any], rerun_report: dict[str, Any]) -> list[dict[str, Any]]:
    base = {str(query.get("query_name")): query_summary(query) for query in base_report.get("plan_queries") or []}
    rerun = {str(query.get("query_name")): query_summary(query) for query in rerun_report.get("plan_queries") or []}
    out = []
    for name in sorted(set(base) | set(rerun)):
        before = base.get(name)
        after = rerun.get(name)
        changed_fields = []
        if before != after:
            keys = sorted(set((before or {}).keys()) | set((after or {}).keys()))
            changed_fields = [key for key in keys if (before or {}).get(key) != (after or {}).get(key)]
        if changed_fields:
            out.append(
                {
                    "query_name": name,
                    "changed_fields": changed_fields,
                    "before": before,
                    "after": after,
                }
            )
    return out


def run_case(args: argparse.Namespace, case: dict[str, Any], reports_dir: Path) -> dict[str, Any]:
    report_path = reports_dir / f"{case['case_id']}.json"
    result = run_plan_probe(args, case, report_path, args.max_nodes)
    if result["status"] != "ok":
        return result
    should_rerun, cards = should_expansion_rerun(args, result["report"])
    if not should_rerun:
        return result

    rerun_path = reports_dir / f"{case['case_id']}.expansion_nodes_{args.expansion_rerun_nodes}.json"
    rerun = run_plan_probe(args, case, rerun_path, args.expansion_rerun_nodes)
    expansion: dict[str, Any] = {
        "triggered": True,
        "reason": "budget_prune_with_action_space_cards",
        "action_space_cards": cards,
        "base_max_nodes": args.max_nodes,
        "rerun_max_nodes": args.expansion_rerun_nodes,
        "rerun_status": rerun["status"],
        "rerun_report_path": rerun.get("report_path"),
    }
    if rerun["status"] == "ok":
        expansion["query_deltas"] = plan_query_deltas(result["report"], rerun["report"])
        expansion["rerun_probe_limits"] = rerun["report"].get("probe_limits") or {}
    else:
        expansion["error"] = rerun.get("error")
    result["expansion_rerun"] = expansion
    return result


def card_semantic_blockers(report: dict[str, Any]) -> list[str]:
    blockers: set[str] = set()
    for card in report.get("hand_cards") or []:
        semantics = {str(item) for item in card.get("base_semantics") or []}
        transient = {str(item) for item in card.get("transient_tags") or []}
        labels = semantics | transient
        if "draw" in labels:
            blockers.add("draw_card_in_hand")
        if "exhaust" in labels or "exhaust_random" in labels:
            blockers.add("exhaust_card_in_hand")
        if "discard" in labels:
            blockers.add("discard_card_in_hand")
        if "random" in labels:
            blockers.add("random_effect_card_in_hand")
        if "energy" in labels or "gain_energy" in labels:
            blockers.add("energy_change_card_in_hand")
        if "apply_vulnerable" in labels or "apply_weak" in labels or "debuff" in labels:
            blockers.add("debuff_card_in_hand")
        if "power" in labels or "setup" in labels or "scaling" in labels:
            blockers.add("setup_or_scaling_card_in_hand")
    return sorted(blockers)


def flatten_result(result: dict[str, Any]) -> dict[str, Any]:
    case = result["case"]
    row: dict[str, Any] = {
        **case,
        "status": result["status"],
        "report_path": result.get("report_path"),
    }
    if result["status"] != "ok":
        row["error"] = result.get("error")
        return row
    report = result["report"]
    limits = report.get("probe_limits") or {}
    rows = report.get("sequence_classes") or []
    compression_notes = Counter()
    abstract_reject_diffs = Counter()
    order_reasons = Counter()
    risk_note_kinds = Counter()
    projection_sequence_count = 0
    projection_total = Counter()
    for sequence in rows:
        for note in sequence.get("compression_notes") or []:
            note = str(note)
            compression_notes[note] += 1
            if note.startswith("abstract_reject_diff:"):
                abstract_reject_diffs[note.removeprefix("abstract_reject_diff:")] += 1
        for reason in sequence.get("order_sensitive_reasons") or []:
            order_reasons[str(reason)] += 1
        diag = sequence.get("diagnostics") or {}
        strength = int(diag.get("strength_projection") or 0)
        dex = int(diag.get("dex_projection") or 0)
        vuln = int(diag.get("vulnerable_projection") or 0)
        if strength or dex or vuln:
            projection_sequence_count += 1
        projection_total["strength_projection"] += strength
        projection_total["dex_projection"] += dex
        projection_total["vulnerable_projection"] += vuln
    for note in report.get("risk_notes") or []:
        risk_note_kinds[str(note.get("kind") or "unknown")] += 1

    pruned_exact = int(limits.get("pruned_as_equivalent") or 0)
    pruned_abstract = int(limits.get("pruned_by_abstract_equivalence") or 0)
    abstract_candidates = int(limits.get("abstract_equivalence_candidates") or 0)
    abstract_blocked_context = int(limits.get("abstract_equivalence_blocked_by_context") or 0)
    abstract_blocked_action = int(limits.get("abstract_equivalence_blocked_by_action_semantics") or 0)
    abstract_rejected_engine = int(limits.get("abstract_equivalence_rejected_by_engine") or 0)
    verified_abstract_pruned = int(limits.get("pruned_by_verified_abstract_equivalence") or pruned_abstract)
    generation_candidates = int(limits.get("generation_canonical_candidates") or 0)
    generation_blocked_context = int(limits.get("generation_canonical_blocked_by_context") or 0)
    generation_blocked_action = int(limits.get("generation_canonical_blocked_by_action_semantics") or 0)
    pruned_generation = int(limits.get("pruned_by_generation_canonical_order") or 0)
    pruned_generation_duplicate = int(limits.get("pruned_by_generation_duplicate_card") or 0)
    pruned_generation_same_lane = int(limits.get("pruned_by_generation_same_lane_order") or 0)
    pruned_generation_target = int(limits.get("pruned_by_generation_target_order") or 0)
    pruned_generation_lane = int(limits.get("pruned_by_generation_lane_order") or 0)
    generation_duplicate_effects = limits.get("generation_duplicate_prune_effects") or {}
    pruned_plan_gate = int(limits.get("pruned_by_plan_expansion_gate") or 0)
    plan_gate_reasons = limits.get("plan_expansion_gate_reasons") or {}
    plan_gate_examples = limits.get("plan_expansion_gate_examples") or []
    expansion_rerun = result.get("expansion_rerun") or {}
    expansion_query_deltas = expansion_rerun.get("query_deltas") or []
    expansion_changed_queries = [str(delta.get("query_name") or "") for delta in expansion_query_deltas]
    expansion_rerun_limits = expansion_rerun.get("rerun_probe_limits") or {}
    pruned_bound = int(limits.get("pruned_by_optimistic_bound") or 0)
    pruned_budget = int(limits.get("pruned_by_budget") or 0)
    nodes = int(limits.get("nodes_expanded") or 0)
    actions_considered = int(limits.get("actions_considered") or 0)
    actions_simulated = int(limits.get("actions_simulated") or 0)
    kept = int(limits.get("sequence_classes_kept") or 0)
    observed_branch_events = (
        nodes
        + pruned_exact
        + pruned_abstract
        + pruned_generation
        + pruned_plan_gate
        + pruned_bound
        + pruned_budget
    )
    total_pruned = (
        pruned_exact
        + pruned_abstract
        + pruned_generation
        + pruned_plan_gate
        + pruned_bound
        + pruned_budget
    )

    no_compression_reasons: list[str] = []
    if pruned_generation == 0:
        no_compression_reasons.append("no_generation_canonical_pruned")
    if pruned_abstract == 0:
        no_compression_reasons.append("no_abstract_equivalence_pruned")
    if not compression_notes.get("generation_canonical_candidate"):
        no_compression_reasons.append("no_generation_canonical_candidate_sequence_kept")
    if not compression_notes.get("abstract_candidate"):
        no_compression_reasons.append("no_abstract_candidate_sequence_kept")
    if generation_blocked_context:
        no_compression_reasons.append("generation_canonical_blocked_by_context")
    if generation_blocked_action:
        no_compression_reasons.append("generation_canonical_blocked_by_action_semantics")
    if abstract_blocked_context:
        no_compression_reasons.append("abstract_blocked_by_context")
    if abstract_blocked_action:
        no_compression_reasons.append("abstract_blocked_by_action_semantics")
    if abstract_rejected_engine:
        no_compression_reasons.append("abstract_rejected_by_engine")
    if pruned_plan_gate:
        no_compression_reasons.append("plan_expansion_gate_pruned")
    if order_reasons:
        no_compression_reasons.append("order_sensitive_sequences_present")
    no_compression_reasons.extend(card_semantic_blockers(report))

    row.update(
        {
            "schema_version": report.get("schema_version"),
            "nodes_expanded": nodes,
            "actions_considered": actions_considered,
            "actions_simulated": actions_simulated,
            "sequence_classes_kept": kept,
            "pruned_as_equivalent": pruned_exact,
            "pruned_by_abstract_equivalence": pruned_abstract,
            "abstract_equivalence_candidates": abstract_candidates,
            "abstract_equivalence_blocked_by_context": abstract_blocked_context,
            "abstract_equivalence_blocked_by_action_semantics": abstract_blocked_action,
            "abstract_equivalence_rejected_by_engine": abstract_rejected_engine,
            "pruned_by_verified_abstract_equivalence": verified_abstract_pruned,
            "generation_canonical_candidates": generation_candidates,
            "generation_canonical_blocked_by_context": generation_blocked_context,
            "generation_canonical_blocked_by_action_semantics": generation_blocked_action,
            "pruned_by_generation_canonical_order": pruned_generation,
            "pruned_by_generation_duplicate_card": pruned_generation_duplicate,
            "pruned_by_generation_same_lane_order": pruned_generation_same_lane,
            "pruned_by_generation_target_order": pruned_generation_target,
            "pruned_by_generation_lane_order": pruned_generation_lane,
            "generation_duplicate_prune_effects": generation_duplicate_effects,
            "pruned_by_plan_expansion_gate": pruned_plan_gate,
            "plan_expansion_gate_reasons": plan_gate_reasons,
            "plan_expansion_gate_examples": plan_gate_examples,
            "pruned_by_optimistic_bound": pruned_bound,
            "pruned_by_budget": pruned_budget,
            "pruned_by_dominated_state": int(limits.get("pruned_by_dominated_state") or 0),
            "total_pruned_observed": total_pruned,
            "observed_branch_events": observed_branch_events,
            "observed_prune_share": (total_pruned / observed_branch_events) if observed_branch_events else 0.0,
            "simulation_skip_share": (pruned_generation / actions_considered) if actions_considered else 0.0,
            "kept_per_node": (kept / nodes) if nodes else 0.0,
            "compression_notes": dict(compression_notes),
            "abstract_reject_diffs": dict(abstract_reject_diffs),
            "order_sensitive_reasons": dict(order_reasons),
            "risk_note_kinds": dict(risk_note_kinds),
            "projection_sequence_count": projection_sequence_count,
            "projection_totals": dict(projection_total),
            "truth_warnings": list(report.get("truth_warnings") or []),
            "no_compression_reasons": sorted(set(no_compression_reasons)),
            "expansion_rerun_triggered": bool(expansion_rerun.get("triggered")),
            "expansion_rerun_status": expansion_rerun.get("rerun_status"),
            "expansion_rerun_reason": expansion_rerun.get("reason"),
            "expansion_rerun_report_path": expansion_rerun.get("rerun_report_path"),
            "expansion_rerun_cards": list(expansion_rerun.get("action_space_cards") or []),
            "expansion_rerun_query_delta_count": len(expansion_query_deltas),
            "expansion_rerun_changed_queries": expansion_changed_queries,
            "expansion_rerun_query_deltas": expansion_query_deltas,
            "expansion_rerun_pruned_by_budget": int(expansion_rerun_limits.get("pruned_by_budget") or 0),
            "expansion_rerun_nodes_expanded": int(expansion_rerun_limits.get("nodes_expanded") or 0),
        }
    )
    return row


def sum_int(rows: list[dict[str, Any]], key: str) -> int:
    return sum(int(row.get(key) or 0) for row in rows)


def aggregate_counter(rows: list[dict[str, Any]], key: str) -> Counter:
    counter: Counter = Counter()
    for row in rows:
        data = row.get(key) or {}
        if isinstance(data, dict):
            counter.update({str(k): int(v) for k, v in data.items()})
        elif isinstance(data, list):
            counter.update(str(item) for item in data)
    return counter


def aggregate_plan_gate_examples(rows: list[dict[str, Any]], top: int) -> list[tuple[str, int]]:
    counter: Counter = Counter()
    for row in rows:
        for example in row.get("plan_expansion_gate_examples") or []:
            reason = str(example.get("reason") or "unknown")
            action = str(example.get("pruned_action_key") or "unknown")
            prefix = " -> ".join(str(item) for item in example.get("partial_action_keys") or [])
            label = f"{reason} | {prefix + ' -> ' if prefix else ''}{action}"
            counter[label] += 1
    return counter.most_common(top)


def summarize(rows: list[dict[str, Any]], top: int = 15) -> dict[str, Any]:
    ok_rows = [row for row in rows if row.get("status") == "ok"]
    failed_rows = [row for row in rows if row.get("status") != "ok"]
    total_nodes = sum_int(ok_rows, "nodes_expanded")
    total_kept = sum_int(ok_rows, "sequence_classes_kept")
    total_pruned = sum_int(ok_rows, "total_pruned_observed")
    total_events = sum_int(ok_rows, "observed_branch_events")
    totals = {
        "cases": len(rows),
        "ok_cases": len(ok_rows),
        "failed_cases": len(failed_rows),
        "nodes_expanded": total_nodes,
        "actions_considered": sum_int(ok_rows, "actions_considered"),
        "actions_simulated": sum_int(ok_rows, "actions_simulated"),
        "sequence_classes_kept": total_kept,
        "pruned_as_equivalent": sum_int(ok_rows, "pruned_as_equivalent"),
        "pruned_by_abstract_equivalence": sum_int(ok_rows, "pruned_by_abstract_equivalence"),
        "abstract_equivalence_candidates": sum_int(ok_rows, "abstract_equivalence_candidates"),
        "abstract_equivalence_blocked_by_context": sum_int(ok_rows, "abstract_equivalence_blocked_by_context"),
        "abstract_equivalence_blocked_by_action_semantics": sum_int(
            ok_rows, "abstract_equivalence_blocked_by_action_semantics"
        ),
        "abstract_equivalence_rejected_by_engine": sum_int(ok_rows, "abstract_equivalence_rejected_by_engine"),
        "pruned_by_verified_abstract_equivalence": sum_int(ok_rows, "pruned_by_verified_abstract_equivalence"),
        "generation_canonical_candidates": sum_int(ok_rows, "generation_canonical_candidates"),
        "generation_canonical_blocked_by_context": sum_int(ok_rows, "generation_canonical_blocked_by_context"),
        "generation_canonical_blocked_by_action_semantics": sum_int(
            ok_rows, "generation_canonical_blocked_by_action_semantics"
        ),
        "pruned_by_generation_canonical_order": sum_int(ok_rows, "pruned_by_generation_canonical_order"),
        "pruned_by_generation_duplicate_card": sum_int(ok_rows, "pruned_by_generation_duplicate_card"),
        "pruned_by_generation_same_lane_order": sum_int(ok_rows, "pruned_by_generation_same_lane_order"),
        "pruned_by_generation_target_order": sum_int(ok_rows, "pruned_by_generation_target_order"),
        "pruned_by_generation_lane_order": sum_int(ok_rows, "pruned_by_generation_lane_order"),
        "pruned_by_plan_expansion_gate": sum_int(ok_rows, "pruned_by_plan_expansion_gate"),
        "pruned_by_optimistic_bound": sum_int(ok_rows, "pruned_by_optimistic_bound"),
        "pruned_by_budget": sum_int(ok_rows, "pruned_by_budget"),
        "total_pruned_observed": total_pruned,
        "observed_branch_events": total_events,
        "observed_prune_share": (total_pruned / total_events) if total_events else 0.0,
        "simulation_skip_share": (
            sum_int(ok_rows, "pruned_by_generation_canonical_order")
            / sum_int(ok_rows, "actions_considered")
        )
        if sum_int(ok_rows, "actions_considered")
        else 0.0,
        "kept_per_node": (total_kept / total_nodes) if total_nodes else 0.0,
        "cases_with_abstract_prune": sum(1 for row in ok_rows if int(row.get("pruned_by_abstract_equivalence") or 0) > 0),
        "cases_with_generation_prune": sum(
            1 for row in ok_rows if int(row.get("pruned_by_generation_canonical_order") or 0) > 0
        ),
        "cases_with_generation_candidates": sum(
            1 for row in ok_rows if int(row.get("generation_canonical_candidates") or 0) > 0
        ),
        "cases_with_abstract_candidates": sum(
            1 for row in ok_rows if int(row.get("abstract_equivalence_candidates") or 0) > 0
        ),
        "cases_with_abstract_rejections": sum(
            1 for row in ok_rows if int(row.get("abstract_equivalence_rejected_by_engine") or 0) > 0
        ),
        "cases_with_plan_expansion_gate": sum(
            1 for row in ok_rows if int(row.get("pruned_by_plan_expansion_gate") or 0) > 0
        ),
        "cases_with_bound_prune": sum(1 for row in ok_rows if int(row.get("pruned_by_optimistic_bound") or 0) > 0),
        "cases_with_budget_prune": sum(1 for row in ok_rows if int(row.get("pruned_by_budget") or 0) > 0),
        "expansion_reruns": sum(1 for row in ok_rows if row.get("expansion_rerun_triggered")),
        "expansion_rerun_success": sum(
            1
            for row in ok_rows
            if row.get("expansion_rerun_triggered") and row.get("expansion_rerun_status") == "ok"
        ),
        "expansion_rerun_query_delta_cases": sum(
            1 for row in ok_rows if int(row.get("expansion_rerun_query_delta_count") or 0) > 0
        ),
        "expansion_rerun_residual_budget_cases": sum(
            1
            for row in ok_rows
            if row.get("expansion_rerun_triggered") and int(row.get("expansion_rerun_pruned_by_budget") or 0) > 0
        ),
    }
    pressure_counts = Counter(str(row.get("pressure_class") or "unknown") for row in ok_rows)
    schema_counts = Counter(str(row.get("schema_version") or "unknown") for row in ok_rows)
    no_compression_reasons = aggregate_counter(ok_rows, "no_compression_reasons")
    return {
        "totals": totals,
        "pressure_counts": dict(pressure_counts),
        "schema_counts": dict(schema_counts),
        "compression_notes": aggregate_counter(ok_rows, "compression_notes").most_common(top),
        "abstract_reject_diffs": aggregate_counter(ok_rows, "abstract_reject_diffs").most_common(top),
        "order_sensitive_reasons": aggregate_counter(ok_rows, "order_sensitive_reasons").most_common(top),
        "risk_note_kinds": aggregate_counter(ok_rows, "risk_note_kinds").most_common(top),
        "no_compression_reasons": no_compression_reasons.most_common(top),
        "generation_duplicate_prune_effects": aggregate_counter(
            ok_rows, "generation_duplicate_prune_effects"
        ).most_common(top),
        "plan_expansion_gate_reasons": aggregate_counter(ok_rows, "plan_expansion_gate_reasons").most_common(top),
        "plan_expansion_gate_example_actions": aggregate_plan_gate_examples(ok_rows, top),
        "expansion_rerun_changed_queries": aggregate_counter(ok_rows, "expansion_rerun_changed_queries").most_common(top),
        "top_expansion_rerun_cases": top_expansion_rerun_cases(ok_rows, top),
        "top_exact_prune_cases": top_cases(ok_rows, "pruned_as_equivalent", top),
        "top_generation_prune_cases": top_cases(ok_rows, "pruned_by_generation_canonical_order", top),
        "top_generation_duplicate_cases": top_cases(ok_rows, "pruned_by_generation_duplicate_card", top),
        "top_generation_same_lane_cases": top_cases(ok_rows, "pruned_by_generation_same_lane_order", top),
        "top_generation_target_order_cases": top_cases(ok_rows, "pruned_by_generation_target_order", top),
        "top_generation_lane_order_cases": top_cases(ok_rows, "pruned_by_generation_lane_order", top),
        "top_plan_expansion_gate_cases": top_cases(ok_rows, "pruned_by_plan_expansion_gate", top),
        "top_generation_candidate_cases": top_cases(ok_rows, "generation_canonical_candidates", top),
        "top_generation_context_blocked_cases": top_cases(ok_rows, "generation_canonical_blocked_by_context", top),
        "top_generation_action_blocked_cases": top_cases(ok_rows, "generation_canonical_blocked_by_action_semantics", top),
        "top_abstract_prune_cases": top_cases(ok_rows, "pruned_by_abstract_equivalence", top),
        "top_abstract_candidate_cases": top_cases(ok_rows, "abstract_equivalence_candidates", top),
        "top_abstract_context_blocked_cases": top_cases(ok_rows, "abstract_equivalence_blocked_by_context", top),
        "top_abstract_action_blocked_cases": top_cases(ok_rows, "abstract_equivalence_blocked_by_action_semantics", top),
        "top_abstract_rejected_cases": top_cases(ok_rows, "abstract_equivalence_rejected_by_engine", top),
        "top_bound_prune_cases": top_cases(ok_rows, "pruned_by_optimistic_bound", top),
        "top_budget_prune_cases": top_cases(ok_rows, "pruned_by_budget", top),
    }


def top_expansion_rerun_cases(rows: list[dict[str, Any]], top: int) -> list[dict[str, Any]]:
    out = []
    candidates = [
        row
        for row in rows
        if row.get("expansion_rerun_triggered")
    ]
    candidates.sort(
        key=lambda row: (
            int(row.get("expansion_rerun_query_delta_count") or 0),
            int(row.get("pruned_by_budget") or 0),
        ),
        reverse=True,
    )
    for row in candidates[:top]:
        out.append(
            {
                "case_id": row.get("case_id"),
                "pressure_class": row.get("pressure_class"),
                "pruned_by_budget": row.get("pruned_by_budget"),
                "rerun_pruned_by_budget": row.get("expansion_rerun_pruned_by_budget"),
                "query_delta_count": row.get("expansion_rerun_query_delta_count"),
                "changed_queries": row.get("expansion_rerun_changed_queries") or [],
                "cards": row.get("expansion_rerun_cards") or [],
                "report_path": row.get("report_path"),
                "rerun_report_path": row.get("expansion_rerun_report_path"),
            }
        )
    return out


def top_cases(rows: list[dict[str, Any]], key: str, top: int) -> list[dict[str, Any]]:
    out = []
    for row in sorted(rows, key=lambda item: int(item.get(key) or 0), reverse=True)[:top]:
        if int(row.get(key) or 0) <= 0:
            continue
        out.append(
            {
                "case_id": row.get("case_id"),
                "seed": row.get("seed"),
                "step_index": row.get("step_index"),
                "pressure_class": row.get("pressure_class"),
                "candidate_count": row.get("candidate_count"),
                "nodes_expanded": row.get("nodes_expanded"),
                "actions_considered": row.get("actions_considered"),
                "actions_simulated": row.get("actions_simulated"),
                "sequence_classes_kept": row.get("sequence_classes_kept"),
                key: row.get(key),
                "report_path": row.get("report_path"),
            }
        )
    return out


def markdown_report(report: dict[str, Any]) -> str:
    totals = report["summary"]["totals"]
    lines = [
        "# Combat Plan Probe Compression Audit",
        "",
        "This report measures current-turn plan-probe search compression. It is not a policy score.",
        "",
        "## Summary",
        "",
    ]
    for key in [
        "cases",
        "ok_cases",
        "failed_cases",
        "nodes_expanded",
        "actions_considered",
        "actions_simulated",
        "sequence_classes_kept",
        "pruned_as_equivalent",
        "pruned_by_abstract_equivalence",
        "abstract_equivalence_candidates",
        "abstract_equivalence_blocked_by_context",
        "abstract_equivalence_blocked_by_action_semantics",
        "abstract_equivalence_rejected_by_engine",
        "pruned_by_verified_abstract_equivalence",
        "generation_canonical_candidates",
        "generation_canonical_blocked_by_context",
        "generation_canonical_blocked_by_action_semantics",
        "pruned_by_generation_canonical_order",
        "pruned_by_generation_duplicate_card",
        "pruned_by_generation_same_lane_order",
        "pruned_by_generation_target_order",
        "pruned_by_generation_lane_order",
        "pruned_by_plan_expansion_gate",
        "pruned_by_optimistic_bound",
        "pruned_by_budget",
        "observed_prune_share",
        "simulation_skip_share",
        "cases_with_generation_prune",
        "cases_with_generation_candidates",
        "cases_with_abstract_prune",
        "cases_with_abstract_candidates",
        "cases_with_abstract_rejections",
        "cases_with_plan_expansion_gate",
        "cases_with_bound_prune",
        "cases_with_budget_prune",
        "expansion_reruns",
        "expansion_rerun_success",
        "expansion_rerun_query_delta_cases",
        "expansion_rerun_residual_budget_cases",
    ]:
        value = totals.get(key)
        if isinstance(value, float):
            lines.append(f"- {key}: `{value:.4f}`")
        else:
            lines.append(f"- {key}: `{value}`")

    def table(title: str, rows: list[Any], headers: tuple[str, str]) -> None:
        lines.extend(["", f"## {title}", ""])
        if not rows:
            lines.append("_none_")
            return
        lines.append(f"| {headers[0]} | {headers[1]} |")
        lines.append("| --- | ---: |")
        for key, value in rows:
            lines.append(f"| `{key}` | {value} |")

    table("Compression Notes", report["summary"]["compression_notes"], ("note", "n"))
    table("Abstract Rejection Diffs", report["summary"]["abstract_reject_diffs"], ("diff", "n"))
    table("Order-Sensitive Reasons", report["summary"]["order_sensitive_reasons"], ("reason", "n"))
    table("Risk Note Kinds", report["summary"]["risk_note_kinds"], ("kind", "n"))
    table("No-Compression Reasons", report["summary"]["no_compression_reasons"], ("reason", "n"))
    table(
        "Generation Duplicate Prune Effects",
        report["summary"]["generation_duplicate_prune_effects"],
        ("effect key", "n"),
    )
    table(
        "Plan Expansion Gate Reasons",
        report["summary"]["plan_expansion_gate_reasons"],
        ("reason", "n"),
    )
    table(
        "Plan Expansion Gate Example Actions",
        report["summary"]["plan_expansion_gate_example_actions"],
        ("example", "n"),
    )
    table(
        "Expansion Rerun Changed Queries",
        report["summary"]["expansion_rerun_changed_queries"],
        ("query", "n"),
    )

    lines.extend(["", "## Top Expansion Rerun Cases", ""])
    rerun_rows = report["summary"]["top_expansion_rerun_cases"]
    if not rerun_rows:
        lines.append("_none_")
    else:
        lines.append("| case | pressure | budget 500→rerun | query deltas | changed queries | cards | reports |")
        lines.append("| --- | --- | ---: | ---: | --- | --- | --- |")
        for row in rerun_rows:
            changed = ", ".join(f"`{item}`" for item in row.get("changed_queries") or []) or "_none_"
            cards = ", ".join(f"`{item}`" for item in row.get("cards") or []) or "_none_"
            lines.append(
                f"| `{row['case_id']}` | `{row['pressure_class']}` | "
                f"{row.get('pruned_by_budget')}→{row.get('rerun_pruned_by_budget')} | "
                f"{row.get('query_delta_count')} | {changed} | {cards} | "
                f"`{row.get('report_path')}` / `{row.get('rerun_report_path')}` |"
            )

    for title, key in [
        ("Top Exact-Prune Cases", "top_exact_prune_cases"),
        ("Top Generation Canonical-Prune Cases", "top_generation_prune_cases"),
        ("Top Generation Duplicate-Card Cases", "top_generation_duplicate_cases"),
        ("Top Generation Same-Lane Order Cases", "top_generation_same_lane_cases"),
        ("Top Generation Target-Order Cases", "top_generation_target_order_cases"),
        ("Top Generation Lane-Order Cases", "top_generation_lane_order_cases"),
        ("Top Plan Expansion Gate Cases", "top_plan_expansion_gate_cases"),
        ("Top Generation Canonical-Candidate Cases", "top_generation_candidate_cases"),
        ("Top Generation Context-Blocked Cases", "top_generation_context_blocked_cases"),
        ("Top Generation Action-Blocked Cases", "top_generation_action_blocked_cases"),
        ("Top Abstract-Prune Cases", "top_abstract_prune_cases"),
        ("Top Abstract-Candidate Cases", "top_abstract_candidate_cases"),
        ("Top Abstract Context-Blocked Cases", "top_abstract_context_blocked_cases"),
        ("Top Abstract Action-Blocked Cases", "top_abstract_action_blocked_cases"),
        ("Top Abstract Engine-Rejected Cases", "top_abstract_rejected_cases"),
        ("Top Bound-Prune Cases", "top_bound_prune_cases"),
        ("Top Budget-Prune Cases", "top_budget_prune_cases"),
    ]:
        lines.extend(["", f"## {title}", ""])
        rows = report["summary"][key]
        if not rows:
            lines.append("_none_")
            continue
        lines.append("| case | pressure | candidates | nodes | sim | kept | pruned | report |")
        lines.append("| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |")
        prune_key = {
            "top_exact_prune_cases": "pruned_as_equivalent",
            "top_generation_prune_cases": "pruned_by_generation_canonical_order",
            "top_generation_duplicate_cases": "pruned_by_generation_duplicate_card",
            "top_generation_same_lane_cases": "pruned_by_generation_same_lane_order",
            "top_generation_target_order_cases": "pruned_by_generation_target_order",
            "top_generation_lane_order_cases": "pruned_by_generation_lane_order",
            "top_plan_expansion_gate_cases": "pruned_by_plan_expansion_gate",
            "top_generation_candidate_cases": "generation_canonical_candidates",
            "top_generation_context_blocked_cases": "generation_canonical_blocked_by_context",
            "top_generation_action_blocked_cases": "generation_canonical_blocked_by_action_semantics",
            "top_abstract_prune_cases": "pruned_by_abstract_equivalence",
            "top_abstract_candidate_cases": "abstract_equivalence_candidates",
            "top_abstract_context_blocked_cases": "abstract_equivalence_blocked_by_context",
            "top_abstract_action_blocked_cases": "abstract_equivalence_blocked_by_action_semantics",
            "top_abstract_rejected_cases": "abstract_equivalence_rejected_by_engine",
            "top_bound_prune_cases": "pruned_by_optimistic_bound",
            "top_budget_prune_cases": "pruned_by_budget",
        }[key]
        for row in rows:
            lines.append(
                f"| `{row['case_id']}` | `{row['pressure_class']}` | {row['candidate_count']} | "
                f"{row['nodes_expanded']} | {row.get('actions_simulated')} | "
                f"{row['sequence_classes_kept']} | {row[prune_key]} | "
                f"`{row['report_path']}` |"
            )

    lines.extend(
        [
            "",
            "## Interpretation",
            "",
            "- `pruned_as_equivalent` is exact state equivalence and is the safest compression signal.",
            "- `pruned_by_generation_canonical_order` is V2 pre-simulation canonical ordering for safe pure damage/block permutations.",
            "- Generation canonical subclasses split that count into duplicate-card, same-lane, target-order, and lane-order prunes.",
            "- `pruned_by_plan_expansion_gate` is V2.3 query-oriented expansion gating for surplus block and repeated action-space changes.",
            "- `pruned_by_abstract_equivalence` is the engine-verified pure damage/block permutation compression fallback.",
            "- `abstract_equivalence_rejected_by_engine` means the heuristic abstract key matched but exact verification refused to prune.",
            "- `abstract_reject_diff:*` notes classify which verified state fields differed across an abstract-key collision.",
            "- `pruned_by_optimistic_bound` is a V0 bound-prune diagnostic; zero means the bound is likely too loose or disabled by safety gates.",
            "- High `order_sensitive_reasons` or blocker counts mean the probe correctly refused to collapse risky sequences.",
        ]
    )
    return "\n".join(lines) + "\n"


def main() -> None:
    args = parse_args()
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    run_dir = resolve_path(args.out_dir) / stamp
    reports_dir = run_dir / "case_reports"
    reports_dir.mkdir(parents=True, exist_ok=True)

    files = trace_files(args, run_dir)
    cases = select_cases(args, files)
    results = [run_case(args, case, reports_dir) for case in cases]
    rows = [flatten_result(result) for result in results]
    summary = summarize(rows)
    report = {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {
            "trace_file": str(args.trace_file) if args.trace_file else None,
            "trace_dir": str(args.trace_dir) if args.trace_dir else None,
            "generate_episodes": args.generate_episodes,
            "generate_seed": args.generate_seed,
            "generate_policy": args.generate_policy,
            "max_cases": args.max_cases,
            "per_trace_limit": args.per_trace_limit,
            "case_strategy": args.case_strategy,
            "max_depth": args.max_depth,
            "max_nodes": args.max_nodes,
            "expansion_rerun_nodes": args.expansion_rerun_nodes,
            "beam_width": args.beam_width,
            "trace_files": [str(path) for path in files],
        },
        "summary": summary,
        "cases": rows,
    }
    json_path = run_dir / "compression_audit_report.json"
    jsonl_path = run_dir / "compression_audit_cases.jsonl"
    md_path = run_dir / "compression_audit_report.md"
    write_json(json_path, report)
    write_jsonl(jsonl_path, rows)
    md_path.write_text(markdown_report(report), encoding="utf-8")
    print(f"Wrote {json_path}")
    print(f"Wrote {jsonl_path}")
    print(f"Wrote {md_path}")
    print(json.dumps(summary["totals"], indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
