#!/usr/bin/env python3
from __future__ import annotations

import argparse
import html
import json
import subprocess
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json, write_jsonl


BATCH_SCHEMA_VERSION = "combat_plan_query_batch_audit_v0_1"
QUERY_NAMES = [
    "CanLethal",
    "CanFullBlock",
    "CanFullBlockThenMaxDamage",
    "CanPlaySetupAndStillBlock",
    "CanPreserveKillWindow",
]

SETUP_DOWNSIDE_CARDS = {
    "Berserk": "berserk_vulnerable_downside",
    "Blasphemy": "blasphemy_delayed_death_risk",
    "WraithForm": "wraith_form_dex_loss_downside",
    "BiasedCognition": "biased_cognition_focus_loss_downside",
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
            "Run combat turn plan-probe over a batch of full-run combat decision steps and "
            "summarize whether tactical plan queries are feasible. This is a diagnostic audit, "
            "not a policy trainer."
        )
    )
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--trace-file", type=Path)
    source.add_argument("--trace-dir", type=Path)
    parser.add_argument("--max-cases", type=int, default=30)
    parser.add_argument("--per-trace-limit", type=int, default=8)
    parser.add_argument("--min-candidates", type=int, default=2)
    parser.add_argument("--min-step-gap", type=int, default=3)
    parser.add_argument(
        "--case-strategy",
        default="balanced_pressure",
        choices=["trace_order", "danger", "balanced_pressure"],
    )
    parser.add_argument("--small-lethal-gap", type=int, default=8)
    parser.add_argument(
        "--damage-gap-threshold",
        type=int,
        default=6,
        help="Minimum current-turn damage gap before a different full-block+damage first action is flagged.",
    )
    parser.add_argument(
        "--leak-gap-threshold",
        type=int,
        default=1,
        help="Minimum projected unblocked damage improvement before a different block line is flagged.",
    )
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=5000)
    parser.add_argument("--max-depth", type=int, default=4)
    parser.add_argument("--max-nodes", type=int, default=500)
    parser.add_argument("--beam-width", type=int, default=16)
    parser.add_argument("--max-engine-steps-per-action", type=int, default=200)
    parser.add_argument("--sts-dev-tool", type=Path)
    parser.add_argument("--force-cargo-run", action="store_true")
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "combat_plan_query_batch",
    )
    return parser.parse_args()


def load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def trace_files(args: argparse.Namespace) -> list[Path]:
    if args.trace_file:
        return [args.trace_file]
    files = sorted(args.trace_dir.glob("episode_*.json"))
    if not files:
        files = sorted(args.trace_dir.rglob("episode_*.json"))
    if not files:
        raise SystemExit(f"no episode_*.json files found in {args.trace_dir}")
    return files


def combat_obs(step: dict[str, Any]) -> dict[str, Any]:
    obs = step.get("observation") or {}
    return obs.get("combat") or {}


def legal_count(step: dict[str, Any]) -> int:
    mask = step.get("action_mask") or []
    return int(step.get("legal_action_count") or len(mask))


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
        "seed": int((trace.get("summary") or {}).get("seed") or 0),
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
        "chosen_action_index": int(step.get("chosen_action_index") or 0),
        "chosen_action_key": str(step.get("chosen_action_key") or ""),
    }


def select_cases(args: argparse.Namespace) -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []
    for path in trace_files(args):
        trace = load_json(path)
        candidates = []
        for step in trace.get("steps") or []:
            if str(step.get("decision_type") or "") != "combat":
                continue
            if str(step.get("engine_state") or "") != "combat_player_turn":
                continue
            if legal_count(step) < args.min_candidates:
                continue
            candidates.append((case_priority(step), step))
        per_trace = 0
        selected_steps: list[int] = []
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


def run_case(args: argparse.Namespace, case: dict[str, Any], reports_dir: Path) -> dict[str, Any]:
    report_path = reports_dir / f"{case['case_id']}.json"
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
        str(args.max_nodes),
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
        }
    return {
        "case": case,
        "status": "ok",
        "report_path": str(report_path),
        "report": load_json(report_path),
    }


def action_label(action_key: Any) -> str:
    key = str(action_key or "")
    if key == "combat/end_turn":
        return "EndTurn"
    if key.startswith("combat/play_card/card:"):
        card = key.split("card:", 1)[1].split("/", 1)[0]
        hand = ""
        if "hand:" in key:
            hand_idx = key.split("hand:", 1)[1].split("/", 1)[0]
            if hand_idx:
                hand = f"[h{hand_idx}]"
        target = key.split("target:", 1)[1] if "target:" in key else "none"
        return f"{card}{hand}" if target in {"none", ""} else f"{card}{hand} -> {target}"
    if key.startswith("combat/use_potion/"):
        return key.replace("combat/use_potion/", "Potion ")
    return key


def first_action(keys: list[Any]) -> str:
    return str(keys[0]) if keys else ""


def query_by_name(report: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {str(query.get("query_name") or ""): query for query in report.get("plan_queries") or []}


def query_outcome(query: dict[str, Any] | None) -> dict[str, Any]:
    if not query:
        return {}
    return query.get("outcome") or {}


def missing_damage(query: dict[str, Any] | None) -> int | None:
    if not query:
        return None
    for constraint in query.get("failed_constraints") or []:
        text = str(constraint)
        if text.startswith("missing_damage:"):
            try:
                return int(text.split(":", 1)[1])
            except ValueError:
                return None
    outcome = query_outcome(query)
    if query.get("status") == "partial" and outcome:
        return int(outcome.get("total_monster_hp") or 0)
    return None


def query_line(query: dict[str, Any] | None) -> str:
    if not query:
        return "missing"
    outcome = query_outcome(query)
    status = str(query.get("status") or "")
    if not outcome:
        return status
    return (
        f"{status}: dmg {outcome.get('damage_done')}, block {outcome.get('block_after')}, "
        f"leak {outcome.get('projected_unblocked_damage')}, energy {outcome.get('remaining_energy')}"
    )


def query_best_first(query: dict[str, Any] | None) -> str:
    if not query:
        return ""
    return first_action(query.get("best_action_keys") or [])


def chosen_matches_query_first(case: dict[str, Any], query: dict[str, Any] | None) -> bool:
    best_first = query_best_first(query)
    return bool(best_first) and best_first == str(case.get("chosen_action_key") or "")


def card_id_from_action_key(action_key: Any) -> str:
    key = str(action_key or "")
    if "card:" not in key:
        return ""
    return key.split("card:", 1)[1].split("/", 1)[0]


def action_card_ids(action_keys: list[Any]) -> list[str]:
    return [card for card in (card_id_from_action_key(key) for key in action_keys) if card]


def setup_downside_notes(action_keys: list[Any]) -> list[str]:
    notes: list[str] = []
    for card_id in action_card_ids(action_keys):
        note = SETUP_DOWNSIDE_CARDS.get(card_id)
        if note and note not in notes:
            notes.append(note)
    return notes


def end_turn_outcome_from_state(report: dict[str, Any]) -> dict[str, Any]:
    state = report.get("state_summary") or {}
    incoming = int(state.get("visible_incoming_damage") or 0)
    block = int(state.get("player_block") or 0)
    return {
        "damage_done": 0,
        "block_after": block,
        "projected_unblocked_damage": max(incoming - block, 0),
        "hp_loss_actual": 0,
        "remaining_energy": int(state.get("energy") or 0),
        "remaining_hand_count": int(state.get("hand_count") or 0),
        "enemy_deaths": 0,
        "living_monster_count": int(state.get("alive_monster_count") or 0),
        "total_monster_hp": int(state.get("total_monster_hp") or 0),
        "played_setup_or_scaling": False,
        "played_kill_window_card": False,
        "random_risk_present": False,
        "ended_turn": True,
        "source": "state_summary_end_turn_projection",
    }


def outcome_int(outcome: dict[str, Any] | None, key: str) -> int:
    if not outcome:
        return 0
    return int(outcome.get(key) or 0)


def outcome_line(outcome: dict[str, Any] | None) -> str:
    if not outcome:
        return "none"
    return (
        f"dmg {outcome_int(outcome, 'damage_done')}, "
        f"block {outcome_int(outcome, 'block_after')}, "
        f"leak {outcome_int(outcome, 'projected_unblocked_damage')}, "
        f"energy {outcome_int(outcome, 'remaining_energy')}"
    )


def outcome_gap(best: dict[str, Any] | None, chosen: dict[str, Any] | None) -> dict[str, int] | None:
    if not best or not chosen:
        return None
    return {
        "damage": outcome_int(best, "damage_done") - outcome_int(chosen, "damage_done"),
        "leak": outcome_int(chosen, "projected_unblocked_damage") - outcome_int(best, "projected_unblocked_damage"),
        "block": outcome_int(best, "block_after") - outcome_int(chosen, "block_after"),
        "enemy_deaths": outcome_int(best, "enemy_deaths") - outcome_int(chosen, "enemy_deaths"),
    }


def sequence_outcomes_for_first(report: dict[str, Any], first_key: str) -> list[dict[str, Any]]:
    if first_key == "combat/end_turn":
        return [end_turn_outcome_from_state(report)]
    outcomes: list[dict[str, Any]] = []
    for sequence in report.get("sequence_classes") or []:
        action_keys = sequence.get("action_keys") or []
        if first_action(action_keys) != first_key:
            continue
        outcome = dict(sequence.get("outcome") or {})
        if not outcome:
            continue
        if bool(outcome.get("ended_turn")) and outcome_int(outcome, "living_monster_count") > 0:
            continue
        outcome["sequence_equivalence_key"] = sequence.get("sequence_equivalence_key")
        outcome["action_keys"] = list(action_keys)
        outcomes.append(outcome)
    return outcomes


def best_outcome_for_first(report: dict[str, Any], first_key: str, query_name: str) -> dict[str, Any] | None:
    outcomes = sequence_outcomes_for_first(report, first_key)
    if not outcomes:
        return None
    if query_name == "CanLethal":
        return max(
            outcomes,
            key=lambda outcome: (
                outcome_int(outcome, "living_monster_count") == 0,
                outcome_int(outcome, "damage_done"),
                -outcome_int(outcome, "total_monster_hp"),
                outcome_int(outcome, "remaining_energy"),
            ),
        )
    if query_name == "CanFullBlock":
        return max(
            outcomes,
            key=lambda outcome: (
                -outcome_int(outcome, "projected_unblocked_damage"),
                outcome_int(outcome, "block_after"),
                outcome_int(outcome, "damage_done"),
                outcome_int(outcome, "remaining_energy"),
            ),
        )
    if query_name == "CanFullBlockThenMaxDamage":
        return max(
            outcomes,
            key=lambda outcome: (
                -outcome_int(outcome, "projected_unblocked_damage"),
                outcome_int(outcome, "damage_done"),
                outcome_int(outcome, "enemy_deaths"),
                outcome_int(outcome, "remaining_energy"),
            ),
        )
    if query_name == "CanPlaySetupAndStillBlock":
        setup_outcomes = [outcome for outcome in outcomes if bool(outcome.get("played_setup_or_scaling"))]
        if not setup_outcomes:
            return None
        return max(
            setup_outcomes,
            key=lambda outcome: (
                -outcome_int(outcome, "projected_unblocked_damage"),
                outcome_int(outcome, "damage_done"),
                outcome_int(outcome, "remaining_energy"),
            ),
        )
    return max(
        outcomes,
        key=lambda outcome: (
            outcome_int(outcome, "damage_done"),
            -outcome_int(outcome, "projected_unblocked_damage"),
            outcome_int(outcome, "block_after"),
        ),
    )


def build_query_summary(
    report: dict[str, Any],
    chosen_key: str,
    name: str,
    query: dict[str, Any] | None,
) -> dict[str, Any]:
    best_outcome = query_outcome(query)
    chosen_outcome = best_outcome_for_first(report, chosen_key, name)
    return {
        "status": (query or {}).get("status"),
        "best_first": query_best_first(query),
        "best_first_label": action_label(query_best_first(query)),
        "line": query_line(query),
        "needs_deeper_search": bool((query or {}).get("needs_deeper_search")),
        "outcome": best_outcome,
        "chosen_first_outcome": chosen_outcome,
        "chosen_first_line": outcome_line(chosen_outcome),
        "query_vs_chosen_gap": outcome_gap(best_outcome, chosen_outcome),
        "failed_constraints": list((query or {}).get("failed_constraints") or []),
        "notes": list((query or {}).get("notes") or []),
    }


def flatten_result(args: argparse.Namespace, result: dict[str, Any]) -> dict[str, Any]:
    case = result["case"]
    if result["status"] != "ok":
        return {
            **case,
            "status": result["status"],
            "error": result.get("error"),
            "report_path": result.get("report_path"),
            "flags": ["probe_failed"],
            "interesting_score": 100,
        }

    report = result["report"]
    queries = query_by_name(report)
    flags: list[str] = []
    notes: list[str] = []
    chosen_key = str(case.get("chosen_action_key") or "")

    lethal = queries.get("CanLethal")
    full_block = queries.get("CanFullBlock")
    full_block_damage = queries.get("CanFullBlockThenMaxDamage")
    setup_block = queries.get("CanPlaySetupAndStillBlock")
    kill_window = queries.get("CanPreserveKillWindow")

    lethal_chosen = best_outcome_for_first(report, chosen_key, "CanLethal")
    if lethal and lethal.get("status") == "feasible":
        if not lethal_chosen or outcome_int(lethal_chosen, "living_monster_count") != 0:
            flags.append("lethal_available_missed_first")
        elif not chosen_matches_query_first(case, lethal):
            notes.append("lethal_available_but_chosen_first_also_lethal")
    gap = missing_damage(lethal)
    if lethal and lethal.get("status") == "partial" and gap is not None and gap <= args.small_lethal_gap:
        flags.append("near_lethal_small_gap")
        notes.append(f"missing_damage={gap}")

    full_block_chosen = best_outcome_for_first(report, chosen_key, "CanFullBlock")
    if full_block and full_block.get("status") == "feasible":
        full_block_gap = outcome_gap(query_outcome(full_block), full_block_chosen)
        if not full_block_chosen:
            notes.append("chosen_first_not_kept_for_full_block")
        elif outcome_int(full_block_chosen, "projected_unblocked_damage") > 0:
            leak_gap = (full_block_gap or {}).get("leak", 0)
            if leak_gap >= args.leak_gap_threshold:
                flags.append("missed_full_block_line")
                notes.append(f"full_block_leak_gap={leak_gap}")
        elif not chosen_matches_query_first(case, full_block):
            notes.append("full_block_first_is_equivalent_or_safe")

    full_block_damage_chosen = best_outcome_for_first(report, chosen_key, "CanFullBlockThenMaxDamage")
    if full_block_damage and full_block_damage.get("status") == "feasible":
        full_damage_gap = outcome_gap(query_outcome(full_block_damage), full_block_damage_chosen)
        if not full_block_damage_chosen:
            notes.append("chosen_first_not_kept_for_full_block_damage")
        else:
            chosen_leak = outcome_int(full_block_damage_chosen, "projected_unblocked_damage")
            damage_gap = (full_damage_gap or {}).get("damage", 0)
            leak_gap = (full_damage_gap or {}).get("leak", 0)
            if chosen_leak > 0 and leak_gap >= args.leak_gap_threshold:
                flags.append("missed_full_block_damage_line")
                notes.append(f"full_block_damage_leak_gap={leak_gap}")
            elif damage_gap >= args.damage_gap_threshold:
                flags.append("full_block_damage_gap")
                notes.append(f"full_block_damage_gap={damage_gap}")
            elif not chosen_matches_query_first(case, full_block_damage):
                notes.append(f"full_block_damage_gap_below_threshold={damage_gap}")

    setup_notes = setup_downside_notes(list((setup_block or {}).get("best_action_keys") or []))
    if setup_block and setup_block.get("status") == "feasible":
        if setup_notes:
            flags.append("setup_and_block_available_with_downside")
        else:
            flags.append("setup_and_block_available_clean")
    if setup_block and setup_block.get("status") == "partial":
        flags.append("setup_available_but_leaks")
    if setup_notes:
        flags.append("setup_downside_risk")
        notes.extend(f"setup_downside={note}" for note in setup_notes)
    if kill_window and kill_window.get("status") == "feasible":
        flags.append("kill_window_preservable")
    if any(bool(query.get("needs_deeper_search")) for query in queries.values()):
        flags.append("needs_deeper_search")
    if case.get("incoming", 0) > 0 and full_block and full_block.get("status") != "feasible":
        flags.append("no_full_block_line_under_pressure")
    if chosen_key == "combat/end_turn":
        end_turn_outcome = end_turn_outcome_from_state(report)
        end_turn_leak = outcome_int(end_turn_outcome, "projected_unblocked_damage")
        fb_damage = query_outcome(full_block_damage)
        fb_damage_gain = outcome_int(fb_damage, "damage_done")
        if full_block_damage and full_block_damage.get("status") == "feasible" and fb_damage_gain >= args.damage_gap_threshold:
            flags.append("end_turn_with_damage_plan_available")
        if setup_block and setup_block.get("status") == "feasible":
            if setup_notes:
                flags.append("end_turn_with_risky_setup_available")
            else:
                flags.append("end_turn_with_clean_setup_available")
        if end_turn_leak > 0 and full_block and full_block.get("status") == "feasible":
            flags.append("end_turn_missed_full_block")

    flags = list(dict.fromkeys(flags))
    notes = list(dict.fromkeys(notes))

    flag_weights = {
        "probe_failed": 100,
        "lethal_available_missed_first": 30,
        "near_lethal_small_gap": 20,
        "missed_full_block_damage_line": 18,
        "missed_full_block_line": 16,
        "full_block_damage_gap": 12,
        "setup_and_block_available_clean": 8,
        "setup_and_block_available_with_downside": 3,
        "setup_downside_risk": 2,
        "setup_available_but_leaks": 8,
        "kill_window_preservable": 8,
        "needs_deeper_search": 7,
        "no_full_block_line_under_pressure": 6,
        "end_turn_with_damage_plan_available": 16,
        "end_turn_with_clean_setup_available": 8,
        "end_turn_with_risky_setup_available": 3,
        "end_turn_missed_full_block": 20,
    }
    interesting_score = sum(flag_weights.get(flag, 1) for flag in flags)

    query_summaries = {
        name: build_query_summary(report, chosen_key, name, queries.get(name))
        for name in QUERY_NAMES
    }
    return {
        **case,
        "status": "ok",
        "report_path": result.get("report_path"),
        "schema_version": report.get("schema_version"),
        "chosen_action_label": action_label(chosen_key),
        "flags": flags,
        "notes": notes,
        "interesting_score": interesting_score,
        "query_summaries": query_summaries,
        "probe_limits": report.get("probe_limits") or {},
        "truth_warnings": report.get("truth_warnings") or [],
    }


def build_summary(rows: list[dict[str, Any]]) -> dict[str, Any]:
    status_counts = Counter(row.get("status") for row in rows)
    pressure_counts = Counter(row.get("pressure_class") for row in rows)
    flag_counts = Counter(flag for row in rows for flag in row.get("flags") or [])
    query_status_counts: dict[str, dict[str, int]] = {}
    for name in QUERY_NAMES:
        counter = Counter(
            ((row.get("query_summaries") or {}).get(name) or {}).get("status", "missing")
            for row in rows
            if row.get("status") == "ok"
        )
        query_status_counts[name] = dict(sorted(counter.items()))
    needs_deeper = sum(
        1
        for row in rows
        if row.get("status") == "ok"
        and any(((row.get("query_summaries") or {}).get(name) or {}).get("needs_deeper_search") for name in QUERY_NAMES)
    )
    return {
        "case_count": len(rows),
        "ok_count": status_counts.get("ok", 0),
        "failed_count": len(rows) - status_counts.get("ok", 0),
        "status_counts": dict(sorted(status_counts.items())),
        "pressure_counts": dict(sorted(pressure_counts.items())),
        "query_status_counts": query_status_counts,
        "flag_counts": dict(sorted(flag_counts.items())),
        "needs_deeper_search_cases": needs_deeper,
    }


def esc(value: Any) -> str:
    return html.escape(str(value if value is not None else ""))


def rel_link(target: str | None, base: Path) -> str:
    if not target:
        return ""
    try:
        return Path(target).resolve().relative_to(base.resolve()).as_posix()
    except ValueError:
        return Path(target).resolve().as_uri()


def render_html(report: dict[str, Any], out_path: Path) -> str:
    rows = sorted(report.get("cases") or [], key=lambda row: int(row.get("interesting_score") or 0), reverse=True)
    interesting = [row for row in rows if int(row.get("interesting_score") or 0) > 0]
    summary = report.get("summary") or {}
    query_counts = summary.get("query_status_counts") or {}
    flag_counts = summary.get("flag_counts") or {}

    status_rows = []
    for name in QUERY_NAMES:
        counts = query_counts.get(name) or {}
        status_rows.append(
            "<tr>"
            f"<td>{esc(name)}</td>"
            f"<td>{esc(counts.get('feasible', 0))}</td>"
            f"<td>{esc(counts.get('partial', 0))}</td>"
            f"<td>{esc(counts.get('not_feasible', 0))}</td>"
            f"<td>{esc(counts.get('not_applicable', 0))}</td>"
            "</tr>"
        )

    case_rows = []
    for row in rows:
        query_summaries = row.get("query_summaries") or {}
        compact_lines = []
        for name in QUERY_NAMES:
            query = query_summaries.get(name) or {}
            gap = query.get("query_vs_chosen_gap") or {}
            gap_text = ""
            if gap:
                gap_text = f"; gap dmg {gap.get('damage')}, leak {gap.get('leak')}"
            compact_lines.append(
                f"<strong>{esc(name.replace('Can', ''))}</strong>: {esc(query.get('line'))}"
                f"<div class='muted'>chosen-first: {esc(query.get('chosen_first_line'))}{esc(gap_text)}</div>"
            )
        compact_queries = "<br>".join(compact_lines)
        flags = " ".join(f"<span class='chip'>{esc(flag)}</span>" for flag in row.get("flags") or [])
        notes = "; ".join(str(note) for note in row.get("notes") or [])
        link = rel_link(row.get("report_path"), out_path.parent)
        report_link = f"<a href='{esc(link)}'>json</a>" if link else ""
        case_rows.append(
            "<tr>"
            f"<td><strong>{esc(row.get('case_id'))}</strong><div class='muted'>floor {esc(row.get('floor'))}, step {esc(row.get('step_index'))}</div></td>"
            f"<td>{esc(row.get('pressure_class'))}<div class='muted'>HP {esc(row.get('hp'))}, in {esc(row.get('incoming'))}, leak {esc(row.get('unblocked'))}</div></td>"
            f"<td>{esc(row.get('chosen_action_label') or action_label(row.get('chosen_action_key')))}</td>"
            f"<td>{compact_queries}</td>"
            f"<td>{flags or '<span class=\"muted\">none</span>'}<div class='muted'>score {esc(row.get('interesting_score'))}</div><div class='muted'>{esc(notes)}</div></td>"
            f"<td>{report_link}</td>"
            "</tr>"
        )

    flag_html = " ".join(f"<span class='chip'>{esc(k)}: {esc(v)}</span>" for k, v in flag_counts.items()) or "<span class='muted'>none</span>"
    css = """
    body { margin: 0; font-family: Segoe UI, Arial, sans-serif; color: #111827; background: #f5f7fb; }
    main { max-width: 1440px; margin: 0 auto; padding: 24px; }
    h1 { margin: 0 0 8px; font-size: 28px; }
    h2 { margin: 0 0 12px; font-size: 18px; }
    .intro, .muted { color: #64748b; font-size: 12px; }
    .panel { background: #fff; border: 1px solid #dbe3ef; border-radius: 8px; padding: 16px; margin: 14px 0; }
    .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 12px; }
    .stat { background: #f8fafc; border: 1px solid #e5e7eb; border-radius: 6px; padding: 10px; }
    .stat-label { color: #64748b; font-size: 12px; }
    .stat-value { font-size: 20px; font-weight: 700; margin: 3px 0; overflow-wrap: anywhere; }
    .chip { display: inline-block; border: 1px solid #a5b4fc; background: #eef2ff; border-radius: 999px; padding: 3px 8px; font-size: 12px; margin: 2px; }
    table { border-collapse: collapse; width: 100%; }
    th, td { border-bottom: 1px solid #e5e7eb; padding: 8px; text-align: left; vertical-align: top; }
    th { background: #f8fafc; color: #475569; font-size: 12px; }
    a { color: #1d4ed8; text-decoration: none; }
    """
    return f"""<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <title>Combat Plan Query Batch Audit</title>
  <style>{css}</style>
</head>
<body>
<main>
  <h1>Combat Plan Query Batch Audit</h1>
  <p class="intro">This report asks whether tactical intents are currently feasible. It is not a teacher label and does not prove the chosen action is wrong.</p>
  <p class="muted">generated: {esc(report.get('generated_at'))}</p>
  <section class="panel">
    <h2>Summary</h2>
    <div class="stats">
      <div class="stat"><div class="stat-label">Cases</div><div class="stat-value">{esc(summary.get('case_count'))}</div></div>
      <div class="stat"><div class="stat-label">OK / Failed</div><div class="stat-value">{esc(summary.get('ok_count'))} / {esc(summary.get('failed_count'))}</div></div>
      <div class="stat"><div class="stat-label">Needs deeper search</div><div class="stat-value">{esc(summary.get('needs_deeper_search_cases'))}</div></div>
      <div class="stat"><div class="stat-label">Interesting</div><div class="stat-value">{esc(len(interesting))}</div></div>
    </div>
    <p>{flag_html}</p>
  </section>
  <section class="panel">
    <h2>Query Status Counts</h2>
    <table><thead><tr><th>Query</th><th>feasible</th><th>partial</th><th>not feasible</th><th>not applicable</th></tr></thead>
    <tbody>{''.join(status_rows)}</tbody></table>
  </section>
  <section class="panel">
    <h2>Cases</h2>
    <table><thead><tr><th>Case</th><th>Pressure</th><th>Chosen first action</th><th>Plan queries</th><th>Flags</th><th>Report</th></tr></thead>
    <tbody>{''.join(case_rows)}</tbody></table>
  </section>
</main>
</body>
</html>
"""


def render_markdown(report: dict[str, Any]) -> str:
    summary = report.get("summary") or {}
    lines = [
        "# Combat Plan Query Batch Audit",
        "",
        "This is a diagnostic report, not a teacher label.",
        "",
        f"- Cases: `{summary.get('case_count')}`",
        f"- OK / failed: `{summary.get('ok_count')}` / `{summary.get('failed_count')}`",
        f"- Needs deeper search cases: `{summary.get('needs_deeper_search_cases')}`",
        "",
        "## Query Status Counts",
    ]
    for name, counts in (summary.get("query_status_counts") or {}).items():
        lines.append(f"- `{name}`: {counts}")
    lines.extend(["", "## Flag Counts"])
    for flag, count in (summary.get("flag_counts") or {}).items():
        lines.append(f"- `{flag}`: `{count}`")
    lines.extend(["", "## Interesting Cases"])
    for row in sorted(report.get("cases") or [], key=lambda item: int(item.get("interesting_score") or 0), reverse=True):
        if int(row.get("interesting_score") or 0) <= 0:
            continue
        notes = "; ".join(row.get("notes") or [])
        lines.append(
            f"- `{row.get('case_id')}` floor `{row.get('floor')}` step `{row.get('step_index')}` "
            f"pressure `{row.get('pressure_class')}` chosen `{row.get('chosen_action_label')}` "
            f"flags `{', '.join(row.get('flags') or [])}` notes `{notes}` report `{row.get('report_path')}`"
        )
    return "\n".join(lines) + "\n"


def main() -> None:
    args = parse_args()
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    out_dir = args.out_dir / timestamp
    reports_dir = out_dir / "case_reports"
    reports_dir.mkdir(parents=True, exist_ok=True)

    cases = select_cases(args)
    if not cases:
        raise SystemExit("no combat cases selected")

    results = [run_case(args, case, reports_dir) for case in cases]
    rows = [flatten_result(args, result) for result in results]
    report = {
        "schema_version": BATCH_SCHEMA_VERSION,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "config": {
            "trace_file": None if args.trace_file is None else str(args.trace_file),
            "trace_dir": None if args.trace_dir is None else str(args.trace_dir),
            "max_cases": args.max_cases,
            "per_trace_limit": args.per_trace_limit,
            "min_candidates": args.min_candidates,
            "min_step_gap": args.min_step_gap,
            "case_strategy": args.case_strategy,
            "small_lethal_gap": args.small_lethal_gap,
            "damage_gap_threshold": args.damage_gap_threshold,
            "leak_gap_threshold": args.leak_gap_threshold,
            "max_depth": args.max_depth,
            "max_nodes": args.max_nodes,
            "beam_width": args.beam_width,
        },
        "summary": build_summary(rows),
        "cases": rows,
    }

    json_path = out_dir / "combat_plan_query_batch_report.json"
    jsonl_path = out_dir / "combat_plan_query_batch_cases.jsonl"
    html_path = out_dir / "combat_plan_query_batch_report.html"
    md_path = out_dir / "combat_plan_query_batch_report.md"
    write_json(json_path, report)
    write_jsonl(jsonl_path, rows)
    html_path.write_text(render_html(report, html_path), encoding="utf-8")
    md_path.write_text(render_markdown(report), encoding="utf-8")
    print(
        json.dumps(
            {
                "json": str(json_path),
                "jsonl": str(jsonl_path),
                "html": str(html_path),
                "markdown": str(md_path),
                "summary": report["summary"],
            },
            indent=2,
            ensure_ascii=False,
        )
    )


if __name__ == "__main__":
    main()
