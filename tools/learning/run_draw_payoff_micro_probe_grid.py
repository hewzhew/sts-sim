#!/usr/bin/env python3
"""Generate and run small draw/search payoff micro-probe grids.

This is intentionally still a diagnostic lab, not a training-label generator.
It perturbs a few controlled author specs and measures whether current-turn
plan-probe evidence shows draw/search cashing out into damage, block, or setup.
"""
from __future__ import annotations

import argparse
import json
import subprocess
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from statistics import mean
from typing import Any

from combat_reranker_common import write_json
from build_draw_payoff_author_specs import monster, spec

REPO_ROOT = Path(__file__).resolve().parents[2]
REPORT_VERSION = "draw_payoff_micro_probe_grid_v0"


@dataclass(frozen=True)
class Template:
    name: str
    draw_card: str
    payoff_kind: str
    payoff_card: str
    base_hand: tuple[str, ...]
    base_hp: int
    monster_hp: int
    tags: tuple[str, ...]


TEMPLATES = [
    Template(
        name="battle_trance_setup_damage",
        draw_card="BattleTrance",
        payoff_kind="setup_damage",
        payoff_card="Inflame",
        base_hand=("Battle Trance", "Wild Strike", "Strike_R", "Defend_R"),
        base_hp=70,
        monster_hp=64,
        tags=("battle_trance", "setup_payoff", "damage_payoff"),
    ),
    Template(
        name="pommel_strike_block",
        draw_card="PommelStrike",
        payoff_kind="block",
        payoff_card="Defend",
        base_hand=("Pommel Strike", "Strike_R", "Defend_R"),
        base_hp=45,
        monster_hp=48,
        tags=("pommel_strike", "block_payoff", "pressure"),
    ),
    Template(
        name="offering_resource_window",
        draw_card="Offering",
        payoff_kind="resource_window",
        payoff_card="Inflame",
        base_hand=("Offering", "Strike_R", "Defend_R"),
        base_hp=55,
        monster_hp=72,
        tags=("offering", "resource_window", "damage_payoff", "block_payoff"),
    ),
    Template(
        name="secret_technique_block_search",
        draw_card="SecretTechnique",
        payoff_kind="search_block",
        payoff_card="ShrugItOff",
        base_hand=("Secret Technique", "Strike_R", "Defend_R"),
        base_hp=48,
        monster_hp=56,
        tags=("secret_technique", "search_payoff", "block_payoff", "draw_payoff"),
    ),
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run synthetic draw/search payoff micro-probe grid.")
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "draw_payoff_micro_probe_grid" / "v0",
    )
    parser.add_argument("--tool-path", type=Path, default=REPO_ROOT / "target" / "debug" / "sts_dev_tool.exe")
    parser.add_argument("--max-cases", type=int, default=0, help="Optional cap; 0 means all generated cases.")
    parser.add_argument("--max-depth", type=int, default=4)
    parser.add_argument("--max-nodes", type=int, default=1200)
    parser.add_argument("--beam-width", type=int, default=16)
    parser.add_argument("--skip-run", action="store_true", help="Only write specs and manifest.")
    return parser.parse_args()


def resolve(path: Path) -> Path:
    return path if path.is_absolute() else REPO_ROOT / path


def card_key(java_id: str) -> str:
    return java_id.replace(" ", "")


def payoff_draw_pile(template: Template, payoff_offset: int, *, skill_distractor: bool) -> list[str]:
    filler = ["Strike_R", "Strike_R", "Defend_R", "Strike_R", "Defend_R"]
    if template.name == "battle_trance_setup_damage":
        pile = filler[:payoff_offset] + ["Inflame"] + filler[payoff_offset:]
        return pile[:5]
    if template.name == "pommel_strike_block":
        pile = filler[:payoff_offset] + ["Defend_R"] + filler[payoff_offset:]
        return pile[:4]
    if template.name == "offering_resource_window":
        # Offering wants both setup and playable payoff. Offset controls setup reachability.
        pile = filler[:payoff_offset] + ["Inflame", "Strike_R", "Defend_R"] + filler[payoff_offset:]
        return pile[:6]
    if template.name == "secret_technique_block_search":
        attack_filler = ["Strike_R", "Strike_R", "Strike_R", "Strike_R", "Strike_R"]
        distractors = ["Defend_R"] if skill_distractor else []
        pile = attack_filler[:payoff_offset] + ["Shrug It Off"] + distractors + attack_filler[payoff_offset:]
        return pile[:6]
    raise ValueError(template.name)


def build_cases() -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []
    incoming_values = [6, 18, 30]
    energy_values = [2, 3]
    payoff_offsets = [0, 1, 3]
    for template in TEMPLATES:
        for incoming in incoming_values:
            for energy in energy_values:
                for payoff_offset in payoff_offsets:
                    distractor_options = [False, True] if template.name == "secret_technique_block_search" else [False]
                    for skill_distractor in distractor_options:
                        case_id = (
                            f"{template.name}_inc{incoming}_e{energy}_off{payoff_offset}"
                            + ("_distractor" if skill_distractor else "")
                        )
                        description = (
                            f"{template.name}: draw/search card {template.draw_card}, payoff {template.payoff_kind}, "
                            f"incoming={incoming}, energy={energy}, payoff_offset={payoff_offset}, "
                            f"skill_distractor={skill_distractor}."
                        )
                        payload = spec(
                            name=case_id,
                            description=description,
                            hand=list(template.base_hand),
                            draw_pile=payoff_draw_pile(template, payoff_offset, skill_distractor=skill_distractor),
                            monsters=[monster(hp=template.monster_hp, damage=incoming)],
                            hp=template.base_hp,
                            energy=energy,
                            tags=[
                                *template.tags,
                                "draw_payoff_micro_grid",
                                f"incoming_{incoming}",
                                f"energy_{energy}",
                                f"payoff_offset_{payoff_offset}",
                            ],
                        )
                        payload["provenance"]["micro_probe"] = {
                            "template": template.name,
                            "draw_card": template.draw_card,
                            "payoff_kind": template.payoff_kind,
                            "payoff_card": template.payoff_card,
                            "incoming": incoming,
                            "energy": energy,
                            "payoff_offset": payoff_offset,
                            "skill_distractor": skill_distractor,
                        }
                        cases.append(payload)
    return cases


def as_int(value: Any, default: int = 0) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def actions_contain(actions: list[str], card: str) -> bool:
    needle = f"card:{card}"
    return any(needle in action for action in actions)


def action_has_grid_select(actions: list[str]) -> bool:
    return any("grid_select" in action for action in actions)


def summarize_report(report: dict[str, Any], meta: dict[str, Any]) -> dict[str, Any]:
    sequences = report.get("sequence_classes") or []
    draw_card = str(meta["draw_card"])
    payoff_card = str(meta["payoff_card"])
    payoff_kind = str(meta["payoff_kind"])
    with_draw = []
    with_draw_payoff = []
    without_draw = []
    for seq in sequences:
        action_keys = [str(action) for action in (seq.get("action_keys") or [])]
        outcome = seq.get("outcome") or {}
        row = {
            "actions": action_keys,
            "damage": as_int(outcome.get("damage_done")),
            "block": as_int(outcome.get("block_after")),
            "unblocked": as_int(outcome.get("projected_unblocked_damage")),
            "setup": bool(outcome.get("played_setup_or_scaling")),
        }
        has_draw = actions_contain(action_keys, draw_card)
        has_payoff = actions_contain(action_keys, payoff_card)
        if draw_card == "SecretTechnique":
            has_payoff = has_payoff or action_has_grid_select(action_keys)
        if has_draw:
            with_draw.append(row)
            if has_payoff:
                with_draw_payoff.append(row)
        else:
            without_draw.append(row)

    def max_field(rows: list[dict[str, Any]], field: str) -> int:
        return max((as_int(row.get(field)) for row in rows), default=0)

    def min_field(rows: list[dict[str, Any]], field: str) -> int:
        return min((as_int(row.get(field)) for row in rows), default=999)

    best_damage_any = max_field(sequences_to_rows(sequences), "damage")
    best_damage_without_draw = max_field(without_draw, "damage")
    best_damage_with_draw_payoff = max_field(with_draw_payoff, "damage")
    best_block_without_draw = max_field(without_draw, "block")
    best_block_with_draw_payoff = max_field(with_draw_payoff, "block")
    min_unblocked_without_draw = min_field(without_draw, "unblocked")
    min_unblocked_with_draw_payoff = min_field(with_draw_payoff, "unblocked")
    damage_delta = best_damage_with_draw_payoff - best_damage_without_draw if with_draw_payoff else 0
    block_delta = best_block_with_draw_payoff - best_block_without_draw if with_draw_payoff else 0
    unblocked_reduction = (
        min_unblocked_without_draw - min_unblocked_with_draw_payoff
        if with_draw_payoff and min_unblocked_without_draw != 999 and min_unblocked_with_draw_payoff != 999
        else 0
    )

    success = False
    reason = "no_payoff_sequence"
    if payoff_kind == "setup_damage":
        success = bool(
            with_draw_payoff
            and any(row["setup"] for row in with_draw_payoff)
            and best_damage_with_draw_payoff > best_damage_without_draw
        )
        reason = "draw_to_setup_increases_damage" if success else reason
    elif payoff_kind == "block":
        success = bool(with_draw_payoff and min_unblocked_with_draw_payoff < min_unblocked_without_draw)
        reason = "draw_to_block_reduces_leak" if success else reason
    elif payoff_kind == "resource_window":
        success = bool(
            with_draw
            and (
                best_damage_with_draw_payoff > best_damage_without_draw
                or best_block_with_draw_payoff > best_block_without_draw
                or any(row["setup"] for row in with_draw)
            )
        )
        reason = "resource_window_cashout" if success else reason
    elif payoff_kind == "search_block":
        success = bool(with_draw_payoff and min_unblocked_with_draw_payoff < min_unblocked_without_draw)
        reason = "search_to_block_reduces_leak" if success else reason

    queries = {str(q.get("query_name")): q for q in (report.get("plan_queries") or [])}
    return {
        "case_id": (report.get("source_trace") or {}).get("case_id") or meta.get("case_id"),
        "template": meta["template"],
        "payoff_kind": payoff_kind,
        "draw_card": draw_card,
        "payoff_card": payoff_card,
        "incoming": meta["incoming"],
        "energy": meta["energy"],
        "payoff_offset": meta["payoff_offset"],
        "skill_distractor": meta["skill_distractor"],
        "sequence_count": len(sequences),
        "draw_sequence_count": len(with_draw),
        "draw_payoff_sequence_count": len(with_draw_payoff),
        "payoff_success": success,
        "payoff_reason": reason,
        "damage_delta": damage_delta,
        "block_delta": block_delta,
        "unblocked_reduction": unblocked_reduction,
        "best_damage_any": best_damage_any,
        "best_damage_without_draw": best_damage_without_draw,
        "best_damage_with_draw_payoff": best_damage_with_draw_payoff,
        "best_block_without_draw": best_block_without_draw,
        "best_block_with_draw_payoff": best_block_with_draw_payoff,
        "min_unblocked_without_draw": None if min_unblocked_without_draw == 999 else min_unblocked_without_draw,
        "min_unblocked_with_draw_payoff": None if min_unblocked_with_draw_payoff == 999 else min_unblocked_with_draw_payoff,
        "query_statuses": {name: query.get("status") for name, query in queries.items()},
    }


def sequences_to_rows(sequences: list[dict[str, Any]]) -> list[dict[str, Any]]:
    rows = []
    for seq in sequences:
        outcome = seq.get("outcome") or {}
        rows.append(
            {
                "damage": as_int(outcome.get("damage_done")),
                "block": as_int(outcome.get("block_after")),
                "unblocked": as_int(outcome.get("projected_unblocked_damage")),
                "setup": bool(outcome.get("played_setup_or_scaling")),
            }
        )
    return rows


def aggregate(rows: list[dict[str, Any]]) -> dict[str, Any]:
    def summarize_group(items: list[dict[str, Any]], extra: dict[str, Any] | None = None) -> dict[str, Any]:
        success_items = [row for row in items if row["payoff_success"]]
        payload = {
            "case_count": len(items),
            "payoff_success_count": sum(1 for row in items if row["payoff_success"]),
            "payoff_success_rate": round(sum(1 for row in items if row["payoff_success"]) / max(len(items), 1), 4),
            "avg_damage_delta": round(mean([float(row["damage_delta"]) for row in items]), 3),
            "avg_block_delta": round(mean([float(row["block_delta"]) for row in items]), 3),
            "avg_unblocked_reduction": round(mean([float(row["unblocked_reduction"]) for row in items]), 3),
            "avg_success_damage_delta": round(mean([float(row["damage_delta"]) for row in success_items]), 3)
            if success_items
            else 0.0,
            "avg_success_block_delta": round(mean([float(row["block_delta"]) for row in success_items]), 3)
            if success_items
            else 0.0,
            "avg_success_unblocked_reduction": round(
                mean([float(row["unblocked_reduction"]) for row in success_items])
            )
            if success_items
            else 0.0,
        }
        if extra:
            payload.update(extra)
        return payload

    by_template: dict[str, list[dict[str, Any]]] = {}
    by_template_offset: dict[tuple[str, int], list[dict[str, Any]]] = {}
    by_template_incoming: dict[tuple[str, int], list[dict[str, Any]]] = {}
    for row in rows:
        by_template.setdefault(str(row["template"]), []).append(row)
        by_template_offset.setdefault((str(row["template"]), as_int(row["payoff_offset"])), []).append(row)
        by_template_incoming.setdefault((str(row["template"]), as_int(row["incoming"])), []).append(row)
    template_rows = []
    for template, items in sorted(by_template.items()):
        template_rows.append(summarize_group(items, {"template": template}))
    offset_rows = [
        summarize_group(items, {"template": template, "payoff_offset": payoff_offset})
        for (template, payoff_offset), items in sorted(by_template_offset.items())
    ]
    incoming_rows = [
        summarize_group(items, {"template": template, "incoming": incoming})
        for (template, incoming), items in sorted(by_template_incoming.items())
    ]
    return {
        "case_count": len(rows),
        "payoff_success_count": sum(1 for row in rows if row["payoff_success"]),
        "payoff_success_rate": round(sum(1 for row in rows if row["payoff_success"]) / max(len(rows), 1), 4),
        "by_template": template_rows,
        "by_template_offset": offset_rows,
        "by_template_incoming": incoming_rows,
    }


def markdown(report: dict[str, Any]) -> str:
    lines = [
        "# Draw Payoff Micro-Probe Grid",
        "",
        f"Generated: `{report['generated_at_utc']}`",
        "",
        "This is diagnostic evidence, not a training label. A success means the current-turn probe found",
        "a sequence where the draw/search card cashes out into the configured payoff axis.",
        "",
        "## Summary",
        "",
        f"- cases: `{report['summary']['case_count']}`",
        f"- payoff successes: `{report['summary']['payoff_success_count']}`",
        f"- payoff success rate: `{report['summary']['payoff_success_rate']}`",
        "",
        "| template | cases | success | rate | avg dmg delta | success dmg delta | success block delta | success leak reduction |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for row in report["summary"]["by_template"]:
        lines.append(
            f"| `{row['template']}` | {row['case_count']} | {row['payoff_success_count']} | "
            f"{row['payoff_success_rate']:.2f} | {row['avg_damage_delta']} | "
            f"{row['avg_success_damage_delta']} | {row['avg_success_block_delta']} | "
            f"{row['avg_success_unblocked_reduction']} |"
        )
    lines.extend(
        [
            "",
            "## Payoff Offset Sensitivity",
            "",
            "| template | offset | cases | success rate | success dmg delta | success block delta |",
            "| --- | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for row in report["summary"].get("by_template_offset") or []:
        lines.append(
            f"| `{row['template']}` | {row['payoff_offset']} | {row['case_count']} | "
            f"{row['payoff_success_rate']:.2f} | {row['avg_success_damage_delta']} | "
            f"{row['avg_success_block_delta']} |"
        )
    lines.extend(["", "## Failed Or Weak Cases", ""])
    failed = [row for row in report["cases"] if not row["payoff_success"]][:20]
    if not failed:
        lines.append("No failed cases in this run.")
    else:
        lines.append("| case | template | incoming | energy | offset | reason | seqs |")
        lines.append("| --- | --- | ---: | ---: | ---: | --- | ---: |")
        for row in failed:
            lines.append(
                f"| `{row['case_id']}` | `{row['template']}` | {row['incoming']} | {row['energy']} | "
                f"{row['payoff_offset']} | `{row['payoff_reason']}` | {row['sequence_count']} |"
            )
    return "\n".join(lines) + "\n"


def run_probe(tool_path: Path, spec_path: Path, out_path: Path, *, max_depth: int, max_nodes: int, beam_width: int) -> None:
    out_path.parent.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(tool_path),
        "combat",
        "plan-probe-author-spec",
        "--author-spec",
        str(spec_path),
        "--out",
        str(out_path),
        "--max-depth",
        str(max_depth),
        "--max-nodes",
        str(max_nodes),
        "--beam-width",
        str(beam_width),
    ]
    subprocess.run(cmd, cwd=REPO_ROOT, check=True, stdout=subprocess.DEVNULL)


def main() -> None:
    args = parse_args()
    out_dir = resolve(args.out_dir)
    tool_path = resolve(args.tool_path)
    if not args.skip_run and not tool_path.exists():
        raise SystemExit(f"tool path not found: {tool_path}")

    cases = build_cases()
    if args.max_cases and args.max_cases > 0:
        cases = cases[: args.max_cases]

    spec_dir = out_dir / "specs"
    report_dir = out_dir / "reports"
    spec_dir.mkdir(parents=True, exist_ok=True)
    report_dir.mkdir(parents=True, exist_ok=True)

    case_rows = []
    summaries = []
    for payload in cases:
        case_id = str(payload["name"])
        spec_path = spec_dir / f"{case_id}.json"
        report_path = report_dir / f"{case_id}.report.json"
        write_json(spec_path, payload)
        meta = dict((payload.get("provenance") or {}).get("micro_probe") or {})
        meta["case_id"] = case_id
        if not args.skip_run:
            run_probe(
                tool_path,
                spec_path,
                report_path,
                max_depth=args.max_depth,
                max_nodes=args.max_nodes,
                beam_width=args.beam_width,
            )
            with report_path.open("r", encoding="utf-8") as handle:
                probe_report = json.load(handle)
            summaries.append(summarize_report(probe_report, meta))
        case_rows.append({"case_id": case_id, "spec_path": str(spec_path), "report_path": str(report_path), **meta})

    payload = {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {
            "out_dir": str(out_dir),
            "tool_path": str(tool_path),
            "max_depth": args.max_depth,
            "max_nodes": args.max_nodes,
            "beam_width": args.beam_width,
            "skip_run": bool(args.skip_run),
        },
        "manifest": case_rows,
        "summary": aggregate(summaries) if summaries else {"case_count": len(case_rows), "payoff_success_count": 0},
        "cases": summaries,
    }
    write_json(out_dir / "draw_payoff_micro_probe_grid.json", payload)
    (out_dir / "draw_payoff_micro_probe_grid.md").write_text(markdown(payload), encoding="utf-8")
    print(json.dumps({"case_count": len(case_rows), "summary": payload["summary"], "out_dir": str(out_dir)}, indent=2))


if __name__ == "__main__":
    main()
