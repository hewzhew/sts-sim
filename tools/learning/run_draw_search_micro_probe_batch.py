#!/usr/bin/env python3
"""Run targeted draw/search current-turn plan-probe micro probes.

This is an active diagnostic sampler: instead of waiting for generic compression
audits to hit a budget-pruned draw/search state, it selects combat turns whose
legal action space contains draw/search/card-zone-changing cards, probes them
with a base and expanded node budget, and emits an expansion-delta-style report
that can be fed into `analyze_draw_setup_cashout_events.py`.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from combat_reranker_common import write_json, write_jsonl

REPO_ROOT = Path(__file__).resolve().parents[2]
REPORT_VERSION = "draw_search_micro_probe_batch_v0"

DRAW_CARDS = {
    "Acrobatics",
    "Adrenaline",
    "Backflip",
    "BattleTrance",
    "BurningPact",
    "DaggerThrow",
    "DarkEmbrace",
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

ZONE_MUTATION_CARDS = {
    "Anger",
    "Immolate",
    "PowerThrough",
    "RecklessCharge",
    "WildStrike",
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

PAYOFF_CARDS = SETUP_OR_SCALING_CARDS | {
    "Bash",
    "Cleave",
    "Clothesline",
    "Defend",
    "Feed",
    "FiendFire",
    "FlameBarrier",
    "HandOfGreed",
    "Hemokinesis",
    "Immolate",
    "Impervious",
    "Rampage",
    "RitualDagger",
    "SecondWind",
    "Strike",
    "ThunderClap",
    "TrueGrit",
    "Uppercut",
    "Whirlwind",
}

TRIGGER_CARDS = DRAW_CARDS | SEARCH_CARDS | ZONE_MUTATION_CARDS


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Target combat turns with draw/search/zone-mutation cards, run base/expanded "
            "plan-probe budgets, and emit compatible delta/event reports."
        )
    )
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--trace-file", type=Path)
    source.add_argument("--trace-dir", type=Path)
    parser.add_argument("--out-dir", type=Path, default=REPO_ROOT / "tools" / "artifacts" / "draw_search_micro_probes")
    parser.add_argument("--max-cases", type=int, default=50)
    parser.add_argument("--per-trace-limit", type=int, default=6)
    parser.add_argument("--min-step-gap", type=int, default=2)
    parser.add_argument("--min-candidates", type=int, default=2)
    parser.add_argument("--base-max-nodes", type=int, default=500)
    parser.add_argument("--expanded-max-nodes", type=int, default=2000)
    parser.add_argument("--max-depth", type=int, default=4)
    parser.add_argument("--beam-width", type=int, default=16)
    parser.add_argument("--max-engine-steps-per-action", type=int, default=200)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=5000)
    parser.add_argument("--sts-dev-tool", type=Path)
    parser.add_argument("--force-cargo-run", action="store_true")
    return parser.parse_args()


def resolve(path: Path) -> Path:
    return path if path.is_absolute() else REPO_ROOT / path


def read_json(path: Path) -> dict[str, Any]:
    with resolve(path).open("r", encoding="utf-8") as handle:
        return json.load(handle)


def parse_card_from_action_key(action_key: str) -> str | None:
    for part in str(action_key).split("/"):
        if part.startswith("card:"):
            return part.removeprefix("card:")
    return None


def num(value: Any) -> float:
    try:
        return float(value or 0.0)
    except (TypeError, ValueError):
        return 0.0


def trace_files(args: argparse.Namespace) -> list[Path]:
    if args.trace_file:
        return [resolve(args.trace_file)]
    root = resolve(args.trace_dir)
    files = sorted(root.glob("episode_*.json"))
    if not files:
        files = sorted(root.rglob("episode_*.json"))
    if not files:
        raise SystemExit(f"no episode_*.json files found in {root}")
    return files


def sts_dev_tool_cmd(args: argparse.Namespace) -> list[str]:
    if args.sts_dev_tool:
        return [str(resolve(args.sts_dev_tool))]
    exe_name = "sts_dev_tool.exe" if sys.platform.startswith("win") else "sts_dev_tool"
    debug_exe = REPO_ROOT / "target" / "debug" / exe_name
    release_exe = REPO_ROOT / "target" / "release" / exe_name
    if not args.force_cargo_run and debug_exe.exists():
        return [str(debug_exe)]
    if not args.force_cargo_run and release_exe.exists():
        return [str(release_exe)]
    return ["cargo", "run", "--quiet", "--bin", "sts_dev_tool", "--"]


def combat_obs(step: dict[str, Any]) -> dict[str, Any]:
    obs = step.get("observation") or {}
    return obs.get("combat") or {}


def visible_incoming(step: dict[str, Any]) -> int:
    return int(combat_obs(step).get("visible_incoming_damage") or 0)


def player_block(step: dict[str, Any]) -> int:
    return int(combat_obs(step).get("player_block") or 0)


def unblocked_damage(step: dict[str, Any]) -> int:
    return max(visible_incoming(step) - player_block(step), 0)


def hp(step: dict[str, Any]) -> int:
    obs = step.get("observation") or {}
    combat = combat_obs(step)
    return int(obs.get("current_hp") or combat.get("player_hp") or step.get("hp") or 0)


def pressure_class(step: dict[str, Any]) -> str:
    incoming = visible_incoming(step)
    unblocked = unblocked_damage(step)
    current_hp = max(hp(step), 1)
    if incoming <= 0:
        return "no_attack"
    if unblocked <= 0:
        return "blocked_attack"
    if unblocked >= current_hp:
        return "lethal_pressure"
    if unblocked >= max(current_hp // 2, 1):
        return "high_pressure"
    if unblocked >= 6:
        return "medium_pressure"
    return "chip_pressure"


def playable_cards(step: dict[str, Any]) -> list[str]:
    cards = []
    seen: set[tuple[int | None, str]] = set()
    for action in step.get("action_mask") or []:
        card = action.get("card") or {}
        card_id = str(card.get("card_id") or parse_card_from_action_key(str(action.get("action_key") or "")) or "")
        if not card_id:
            continue
        action_payload = action.get("action") or {}
        hand_index = action_payload.get("card_index")
        key = (hand_index if isinstance(hand_index, int) else None, card_id)
        if key in seen:
            continue
        seen.add(key)
        cards.append(card_id)
    return cards


def step_trigger_tags(cards: list[str]) -> list[str]:
    tags = set()
    if any(card in DRAW_CARDS for card in cards):
        tags.add("draw")
    if any(card in SEARCH_CARDS for card in cards):
        tags.add("search")
    if any(card in ZONE_MUTATION_CARDS for card in cards):
        tags.add("zone_mutation")
    if any(card in SETUP_OR_SCALING_CARDS for card in cards):
        tags.add("setup_or_scaling")
    if any(card in PAYOFF_CARDS for card in cards):
        tags.add("payoff_available")
    return sorted(tags)


def selection_score(step: dict[str, Any]) -> tuple[int, int, int, int, int]:
    cards = playable_cards(step)
    draw = sum(1 for card in cards if card in DRAW_CARDS)
    search = sum(1 for card in cards if card in SEARCH_CARDS)
    zone = sum(1 for card in cards if card in ZONE_MUTATION_CARDS)
    payoff = sum(1 for card in cards if card in PAYOFF_CARDS)
    pressure = unblocked_damage(step)
    return (search + draw, zone, payoff, pressure, len(cards))


def legal_count(step: dict[str, Any]) -> int:
    return int(step.get("legal_action_count") or len(step.get("action_mask") or []))


def step_is_candidate(step: dict[str, Any], min_candidates: int) -> bool:
    if str(step.get("decision_type") or "") != "combat":
        return False
    if str(step.get("engine_state") or "") != "combat_player_turn":
        return False
    if legal_count(step) < min_candidates:
        return False
    cards = playable_cards(step)
    return any(card in TRIGGER_CARDS for card in cards)


def select_cases(args: argparse.Namespace, files: list[Path]) -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []
    for path in files:
        trace = read_json(path)
        scored = []
        for step in trace.get("steps") or []:
            if not step_is_candidate(step, args.min_candidates):
                continue
            scored.append((selection_score(step), step))
        selected_steps: list[int] = []
        for _score, step in sorted(scored, key=lambda item: item[0], reverse=True):
            step_index = int(step.get("step_index") or 0)
            if any(abs(step_index - seen) < args.min_step_gap for seen in selected_steps):
                continue
            cards = playable_cards(step)
            cases.append(
                {
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
                    "candidate_count": legal_count(step),
                    "playable_cards": cards,
                    "trigger_cards": sorted(card for card in set(cards) if card in TRIGGER_CARDS),
                    "trigger_tags": step_trigger_tags(cards),
                    "chosen_action_key": str(step.get("chosen_action_key") or ""),
                }
            )
            selected_steps.append(step_index)
            if len(selected_steps) >= args.per_trace_limit or len(cases) >= args.max_cases:
                break
        if len(cases) >= args.max_cases:
            break
    return cases


def run_plan_probe(args: argparse.Namespace, case: dict[str, Any], report_path: Path, max_nodes: int) -> dict[str, Any]:
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
            "status": "failed",
            "error": proc.stderr.strip() or proc.stdout.strip(),
            "report_path": str(report_path),
            "max_nodes": max_nodes,
        }
    return {
        "status": "ok",
        "report_path": str(report_path),
        "report": read_json(report_path),
        "max_nodes": max_nodes,
    }


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
                "block_after",
                "projected_unblocked_damage",
                "remaining_energy",
                "enemy_deaths",
                "living_monster_count",
                "total_monster_hp",
                "played_setup_or_scaling",
                "played_kill_window_card",
                "random_risk_present",
            ]
            if key in outcome
        },
    }


def plan_query_deltas(base_report: dict[str, Any], expanded_report: dict[str, Any]) -> list[dict[str, Any]]:
    base = {str(query.get("query_name")): query_summary(query) for query in base_report.get("plan_queries") or []}
    expanded = {
        str(query.get("query_name")): query_summary(query)
        for query in expanded_report.get("plan_queries") or []
    }
    deltas = []
    for name in sorted(set(base) | set(expanded)):
        before = base.get(name)
        after = expanded.get(name)
        if before == after:
            continue
        keys = sorted(set((before or {}).keys()) | set((after or {}).keys()))
        changed_fields = [key for key in keys if (before or {}).get(key) != (after or {}).get(key)]
        deltas.append({"query_name": name, "changed_fields": changed_fields, "before": before, "after": after})
    return deltas


def action_card_ids(action_keys: list[str]) -> list[str]:
    return [card for key in action_keys if (card := parse_card_from_action_key(key))]


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


def classify_delta(delta: dict[str, Any]) -> str:
    before = delta.get("before") or {}
    after = delta.get("after") or {}
    if before.get("status") != after.get("status"):
        return "status_changed"
    if numeric_delta(before, after, "damage_done") not in (None, 0):
        return "damage_improved"
    if numeric_delta(before, after, "projected_unblocked_damage") not in (None, 0):
        return "block_or_leak_changed"
    if bool_transition(before, after, "played_setup_or_scaling"):
        return "setup_changed"
    if before.get("needs_deeper_search") != after.get("needs_deeper_search"):
        return "confidence_changed"
    if before.get("best_action_keys") != after.get("best_action_keys"):
        return "line_changed"
    return "other"


def mechanism_tags(cards: list[str]) -> list[str]:
    tags = set()
    if any(card in DRAW_CARDS for card in cards):
        tags.add("draw")
    if any(card in SEARCH_CARDS for card in cards):
        tags.add("search")
    if any(card in SETUP_OR_SCALING_CARDS for card in cards):
        tags.add("setup_or_scaling")
    if any(card in ZONE_MUTATION_CARDS for card in cards):
        tags.add("zone_mutation")
    if any(card in PAYOFF_CARDS for card in cards):
        tags.add("payoff_available")
    return sorted(tags)


def delta_summary(case: dict[str, Any], delta: dict[str, Any], base_path: Path, expanded_path: Path) -> dict[str, Any]:
    before = delta.get("before") or {}
    after = delta.get("after") or {}
    before_actions = list(before.get("best_action_keys") or [])
    after_actions = list(after.get("best_action_keys") or [])
    before_cards = action_card_ids(before_actions)
    after_cards = action_card_ids(after_actions)
    all_cards = sorted(set(before_cards + after_cards + list(case.get("trigger_cards") or [])))
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
        "trigger_cards": case.get("trigger_cards") or [],
        "before_cards": before_cards,
        "after_cards": after_cards,
        "new_after_cards": [card for card in after_cards if card not in before_cards],
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
        "report_path": str(base_path),
        "rerun_report_path": str(expanded_path),
    }


def run_case(args: argparse.Namespace, case: dict[str, Any], reports_dir: Path) -> dict[str, Any]:
    base_path = reports_dir / f"{case['case_id']}.base_nodes_{args.base_max_nodes}.json"
    expanded_path = reports_dir / f"{case['case_id']}.expanded_nodes_{args.expanded_max_nodes}.json"
    base = run_plan_probe(args, case, base_path, args.base_max_nodes)
    if base["status"] != "ok":
        return {**case, "status": "failed", "phase": "base", "error": base.get("error")}
    expanded = run_plan_probe(args, case, expanded_path, args.expanded_max_nodes)
    if expanded["status"] != "ok":
        return {**case, "status": "failed", "phase": "expanded", "error": expanded.get("error")}
    deltas = plan_query_deltas(base["report"], expanded["report"])
    limits_base = base["report"].get("probe_limits") or {}
    limits_expanded = expanded["report"].get("probe_limits") or {}
    return {
        **case,
        "status": "ok",
        "report_path": str(base_path),
        "expanded_report_path": str(expanded_path),
        "schema_version": base["report"].get("schema_version"),
        "base_nodes_expanded": int(limits_base.get("nodes_expanded") or 0),
        "expanded_nodes_expanded": int(limits_expanded.get("nodes_expanded") or 0),
        "base_pruned_by_budget": int(limits_base.get("pruned_by_budget") or 0),
        "expanded_pruned_by_budget": int(limits_expanded.get("pruned_by_budget") or 0),
        "query_delta_count": len(deltas),
        "changed_queries": [delta.get("query_name") for delta in deltas],
        "query_deltas": deltas,
        "delta_rows": [delta_summary(case, delta, base_path, expanded_path) for delta in deltas],
    }


def build_delta_report(report: dict[str, Any], out_path: Path) -> dict[str, Any]:
    rows = [row for case in report["cases"] for row in case.get("delta_rows") or []]
    actionable = [
        row
        for row in rows
        if row["delta_kind"] in {"status_changed", "damage_improved", "setup_changed", "block_or_leak_changed"}
    ]
    return {
        "report_version": "draw_search_micro_probe_delta_report_v0",
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "audit_report": str(out_path),
        "summary": {
            "cases": len(report["cases"]),
            "ok_cases": report["summary"]["ok_cases"],
            "delta_rows": len(rows),
            "actionable_delta_rows": len(actionable),
            "query_counts": dict(Counter(row["query_name"] for row in rows)),
            "delta_kind_counts": dict(Counter(row["delta_kind"] for row in rows)),
            "mechanism_counts": dict(Counter(tag for row in rows for tag in row["mechanism_tags"])),
            "trigger_card_counts": dict(Counter(card for row in rows for card in row["trigger_cards"])),
            "pressure_counts": dict(Counter(row["pressure_class"] for row in rows)),
        },
        "deltas": rows,
        "actionable_deltas": actionable,
    }


def build_report(args: argparse.Namespace, run_dir: Path, cases: list[dict[str, Any]]) -> dict[str, Any]:
    reports_dir = run_dir / "case_reports"
    reports_dir.mkdir(parents=True, exist_ok=True)
    results = [run_case(args, case, reports_dir) for case in cases]
    ok = [row for row in results if row.get("status") == "ok"]
    failed = [row for row in results if row.get("status") != "ok"]
    delta_rows = [delta for row in ok for delta in row.get("delta_rows") or []]
    return {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {
            "trace_file": str(args.trace_file) if args.trace_file else None,
            "trace_dir": str(args.trace_dir) if args.trace_dir else None,
            "max_cases": args.max_cases,
            "per_trace_limit": args.per_trace_limit,
            "base_max_nodes": args.base_max_nodes,
            "expanded_max_nodes": args.expanded_max_nodes,
            "max_depth": args.max_depth,
            "beam_width": args.beam_width,
        },
        "summary": {
            "cases": len(results),
            "ok_cases": len(ok),
            "failed_cases": len(failed),
            "cases_with_query_delta": sum(1 for row in ok if int(row.get("query_delta_count") or 0) > 0),
            "delta_rows": len(delta_rows),
            "base_budget_prune_cases": sum(1 for row in ok if int(row.get("base_pruned_by_budget") or 0) > 0),
            "expanded_budget_prune_cases": sum(1 for row in ok if int(row.get("expanded_pruned_by_budget") or 0) > 0),
            "trigger_tag_counts": dict(Counter(tag for row in ok for tag in row.get("trigger_tags") or [])),
            "trigger_card_counts": dict(Counter(card for row in ok for card in row.get("trigger_cards") or [])),
            "changed_query_counts": dict(Counter(query for row in ok for query in row.get("changed_queries") or [])),
            "delta_kind_counts": dict(Counter(delta["delta_kind"] for delta in delta_rows)),
        },
        "cases": results,
    }


def markdown_report(report: dict[str, Any], delta_report: dict[str, Any]) -> str:
    lines = [
        "# Draw/Search Micro-Probe Batch",
        "",
        "This report actively samples combat turns with draw/search/zone-mutation cards and compares base vs expanded current-turn plan-probe budgets.",
        "",
        "## Summary",
        "",
    ]
    for key, value in report["summary"].items():
        if isinstance(value, dict):
            continue
        lines.append(f"- {key}: `{value}`")
    for title, data in [
        ("Trigger Tags", report["summary"].get("trigger_tag_counts") or {}),
        ("Trigger Cards", report["summary"].get("trigger_card_counts") or {}),
        ("Changed Queries", report["summary"].get("changed_query_counts") or {}),
        ("Delta Kinds", report["summary"].get("delta_kind_counts") or {}),
    ]:
        lines.extend(["", f"## {title}", ""])
        if not data:
            lines.append("_none_")
        else:
            lines.append("| item | n |")
            lines.append("| --- | ---: |")
            for key, value in sorted(data.items(), key=lambda item: (-item[1], item[0])):
                lines.append(f"| `{key}` | {value} |")

    lines.extend(["", "## Delta Cases", ""])
    delta_cases = [case for case in report["cases"] if case.get("query_delta_count")]
    if not delta_cases:
        lines.append("_none_")
    else:
        lines.append("| case | trigger cards | changed queries | base budget | expanded budget | reports |")
        lines.append("| --- | --- | --- | ---: | ---: | --- |")
        for case in delta_cases[:40]:
            lines.append(
                f"| `{case['case_id']}` | `{','.join(case.get('trigger_cards') or [])}` | "
                f"`{','.join(case.get('changed_queries') or [])}` | "
                f"{case.get('base_pruned_by_budget')} | {case.get('expanded_pruned_by_budget')} | "
                f"`{case.get('report_path')}` / `{case.get('expanded_report_path')}` |"
            )

    lines.extend(["", "## Actionable Deltas", ""])
    if not delta_report.get("actionable_deltas"):
        lines.append("_none_")
    else:
        lines.append("| case | query | kind | cards | before | after |")
        lines.append("| --- | --- | --- | --- | --- | --- |")
        for row in delta_report["actionable_deltas"][:40]:
            lines.append(
                f"| `{row['case_id']}` | `{row['query_name']}` | `{row['delta_kind']}` | "
                f"`{','.join(row.get('trigger_cards') or [])}` | "
                f"`{' -> '.join(row.get('before_cards') or [])}` | "
                f"`{' -> '.join(row.get('after_cards') or [])}` |"
            )
    return "\n".join(lines) + "\n"


def main() -> None:
    args = parse_args()
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    run_dir = resolve(args.out_dir) / stamp
    run_dir.mkdir(parents=True, exist_ok=True)
    files = trace_files(args)
    cases = select_cases(args, files)
    if not cases:
        raise SystemExit("no draw/search micro-probe candidate cases found")
    report = build_report(args, run_dir, cases)
    report_path = run_dir / "draw_search_micro_probe_report.json"
    write_json(report_path, report)
    write_jsonl(run_dir / "draw_search_micro_probe_cases.jsonl", report["cases"])
    delta_report = build_delta_report(report, report_path)
    delta_path = run_dir / "expansion_delta_report.json"
    write_json(delta_path, delta_report)
    md_path = run_dir / "draw_search_micro_probe_report.md"
    md_path.write_text(markdown_report(report, delta_report), encoding="utf-8")
    print(f"Wrote {report_path}")
    print(f"Wrote {delta_path}")
    print(f"Wrote {md_path}")
    print(json.dumps(report["summary"], indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
