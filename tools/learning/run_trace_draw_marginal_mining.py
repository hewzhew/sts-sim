#!/usr/bin/env python3
"""Mine real full-run traces for draw/search marginal-value probe cases.

This is the trace-derived counterpart to `run_draw_marginal_value_probe_batch.py`.
It does not synthesize combat states. It replays full-run traces to combat
decision points where a draw/search/resource card is actually in hand, then asks
the Rust probe to compare:

  no-target-action branch vs forced-target-action branch.

The output intentionally matches the existing draw marginal drilldown inputs
(`branch_outcomes.jsonl` and `marginal_examples.jsonl`).
"""
from __future__ import annotations

import argparse
import json
import subprocess
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from statistics import mean
from typing import Any

from combat_reranker_common import write_json, write_jsonl

REPO_ROOT = Path(__file__).resolve().parents[2]
REPORT_VERSION = "trace_draw_marginal_mining_v0"

DEFAULT_TARGET_CARDS = [
    "BattleTrance",
    "PommelStrike",
    "Offering",
    "SecretTechnique",
    "SecretWeapon",
    "ShrugItOff",
    "BurningPact",
    "Dropkick",
    "Warcry",
    "MasterOfStrategy",
]

CARD_ALIASES = {
    "Battle Trance": "BattleTrance",
    "Pommel Strike": "PommelStrike",
    "Secret Technique": "SecretTechnique",
    "Secret Weapon": "SecretWeapon",
    "Shrug It Off": "ShrugItOff",
    "Burning Pact": "BurningPact",
    "War Cry": "Warcry",
    "Master Of Strategy": "MasterOfStrategy",
    "Master of Strategy": "MasterOfStrategy",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run draw marginal probes on trace-derived combat states.")
    parser.add_argument(
        "--trace-dir",
        type=Path,
        default=None,
        help="Directory containing full-run episode_*.json traces. Defaults to latest generated_traces artifact.",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "draw_marginal_value" / "trace_v0",
    )
    parser.add_argument("--cards", default=",".join(DEFAULT_TARGET_CARDS))
    parser.add_argument("--max-cases", type=int, default=80)
    parser.add_argument("--max-cases-per-card", type=int, default=20)
    parser.add_argument("--tool-path", type=Path, default=REPO_ROOT / "target" / "debug" / "sts_dev_tool.exe")
    parser.add_argument("--max-depth", type=int, default=4)
    parser.add_argument("--max-nodes", type=int, default=1200)
    parser.add_argument("--beam-width", type=int, default=16)
    parser.add_argument("--skip-run", action="store_true")
    return parser.parse_args()


def resolve(path: Path) -> Path:
    return path if path.is_absolute() else REPO_ROOT / path


def latest_generated_trace_dir() -> Path | None:
    root = REPO_ROOT / "tools" / "artifacts" / "combat_plan_probe_compression"
    if not root.exists():
        return None
    dirs = [path for path in root.glob("*/generated_traces") if path.is_dir()]
    if not dirs:
        return None
    return max(dirs, key=lambda path: path.stat().st_mtime)


def load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def card_key(card_id: Any) -> str:
    text = str(card_id or "").strip()
    return CARD_ALIASES.get(text, text)


def as_int(value: Any, default: int = 0) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def trace_files(trace_dir: Path) -> list[Path]:
    files = sorted(trace_dir.glob("episode_*.json"))
    if files:
        return files
    return sorted(trace_dir.glob("*.json"))


def hand_target_cards(step: dict[str, Any], target_cards: set[str]) -> list[dict[str, Any]]:
    combat = ((step.get("observation") or {}).get("combat") or {})
    result = []
    for hand_card in combat.get("hand_cards") or []:
        normalized = card_key(hand_card.get("card_id"))
        if normalized in target_cards and bool(hand_card.get("playable", True)):
            result.append({**hand_card, "normalized_card_id": normalized})
    return result


def has_legal_play_for_card(step: dict[str, Any], target_card: str, hand_index: int | None) -> bool:
    for action in step.get("action_mask") or []:
        if not isinstance(action, dict):
            continue
        if (action.get("action") or {}).get("type") != "play_card":
            continue
        if hand_index is not None and as_int((action.get("action") or {}).get("card_index"), -1) != hand_index:
            continue
        card = action.get("card") or {}
        if card_key(card.get("card_id")) == target_card:
            return True
    return False


def scan_candidates(trace_dir: Path, target_cards: set[str], *, max_cases_per_card: int, max_cases: int) -> list[dict[str, Any]]:
    candidates: list[dict[str, Any]] = []
    for path in trace_files(trace_dir):
        try:
            data = load_json(path)
        except Exception as err:  # pragma: no cover - diagnostic script
            print(f"warning: failed to read trace {path}: {err}")
            continue
        summary = data.get("summary") or {}
        for step in data.get("steps") or []:
            if step.get("decision_type") != "combat":
                continue
            if step.get("engine_state") != "combat_player_turn":
                continue
            combat = ((step.get("observation") or {}).get("combat") or {})
            if combat.get("combat_phase") not in {None, "player_turn"}:
                continue
            for hand_card in hand_target_cards(step, target_cards):
                target_card = str(hand_card["normalized_card_id"])
                hand_index = as_int(hand_card.get("hand_index"), -1)
                if not has_legal_play_for_card(step, target_card, hand_index):
                    continue
                incoming = as_int(combat.get("visible_incoming_damage"))
                hp = as_int(combat.get("player_hp"), as_int(step.get("hp")))
                energy = as_int(combat.get("energy"))
                pressure_score = incoming * 4 + max(0, incoming - hp // 4) * 6
                draw_count = as_int(combat.get("draw_count"))
                legal_count = as_int(step.get("legal_action_count"))
                chosen_key = str(step.get("chosen_action_key") or "")
                chosen_bonus = 20 if target_card in chosen_key else 0
                score = pressure_score + legal_count + min(draw_count, 8) + chosen_bonus
                case_id = f"{path.stem}_step_{as_int(step.get('step_index')):04d}_{target_card}"
                candidates.append(
                    {
                        "case_id": case_id,
                        "trace_file": str(path),
                        "trace_dir": str(trace_dir),
                        "episode_seed": summary.get("seed"),
                        "episode_result": summary.get("result"),
                        "episode_floor": summary.get("floor"),
                        "step_index": as_int(step.get("step_index")),
                        "act": as_int(step.get("act")),
                        "floor": as_int(step.get("floor")),
                        "hp": hp,
                        "max_hp": as_int(step.get("max_hp"), as_int((step.get("observation") or {}).get("max_hp"))),
                        "energy": energy,
                        "visible_incoming_damage": incoming,
                        "draw_count": draw_count,
                        "discard_count": as_int(combat.get("discard_count")),
                        "exhaust_count": as_int(combat.get("exhaust_count")),
                        "alive_monster_count": as_int(combat.get("alive_monster_count")),
                        "total_monster_hp": as_int(combat.get("total_monster_hp")),
                        "target_action_card": target_card,
                        "hand_index": hand_index,
                        "card_instance_id": hand_card.get("card_instance_id"),
                        "target_cost_for_turn": hand_card.get("cost_for_turn"),
                        "chosen_action_key": chosen_key,
                        "legal_action_count": legal_count,
                        "selection_score": score,
                        "selection_reasons": selection_reasons(incoming, draw_count, legal_count, chosen_key, target_card),
                    }
                )
    candidates.sort(
        key=lambda row: (
            -as_int(row.get("selection_score")),
            str(row.get("target_action_card")),
            str(row.get("trace_file")),
            as_int(row.get("step_index")),
        )
    )
    chosen: list[dict[str, Any]] = []
    per_card: Counter[str] = Counter()
    seen: set[tuple[str, int, str]] = set()
    for row in candidates:
        card = str(row["target_action_card"])
        key = (str(row["trace_file"]), as_int(row["step_index"]), card)
        if key in seen or per_card[card] >= max_cases_per_card:
            continue
        seen.add(key)
        per_card[card] += 1
        chosen.append(row)
        if len(chosen) >= max_cases:
            break
    return chosen


def selection_reasons(incoming: int, draw_count: int, legal_count: int, chosen_key: str, target_card: str) -> list[str]:
    reasons = ["target_card_playable_in_trace_hand"]
    if incoming > 0:
        reasons.append("visible_incoming_pressure")
    if incoming >= 18:
        reasons.append("high_incoming_pressure")
    if draw_count > 0:
        reasons.append("nonempty_draw_pile")
    if legal_count >= 8:
        reasons.append("wide_action_mask")
    if target_card in chosen_key:
        reasons.append("trace_policy_chose_target_card")
    return reasons


def run_probe(
    tool_path: Path,
    report_path: Path,
    case: dict[str, Any],
    *,
    max_depth: int,
    max_nodes: int,
    beam_width: int,
) -> None:
    report_path.parent.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(tool_path),
        "combat",
        "draw-marginal-probe",
        "--trace-file",
        str(case["trace_file"]),
        "--step-index",
        str(case["step_index"]),
        "--action-card",
        str(case["target_action_card"]),
        "--hand-index",
        str(case["hand_index"]),
        "--out",
        str(report_path),
        "--max-depth",
        str(max_depth),
        "--max-nodes",
        str(max_nodes),
        "--beam-width",
        str(beam_width),
    ]
    subprocess.run(cmd, cwd=REPO_ROOT, check=True, stdout=subprocess.DEVNULL)


def branch_rows(case: dict[str, Any], report: dict[str, Any]) -> list[dict[str, Any]]:
    rows = []
    for branch in report.get("branches") or []:
        for query in branch.get("plan_queries") or []:
            outcome = query.get("outcome") or {}
            rows.append(
                {
                    **case,
                    "branch_name": branch.get("branch_name"),
                    "branch_status": branch.get("status"),
                    "query_name": query.get("query_name"),
                    "query_status": query.get("status"),
                    "damage_done": outcome.get("damage_done"),
                    "block_after": outcome.get("block_after"),
                    "projected_unblocked_damage": outcome.get("projected_unblocked_damage"),
                    "hp_loss_actual": outcome.get("hp_loss_actual"),
                    "remaining_energy": outcome.get("remaining_energy"),
                    "remaining_hand_count": outcome.get("remaining_hand_count"),
                    "played_setup_or_scaling": outcome.get("played_setup_or_scaling"),
                    "best_action_keys": query.get("best_action_keys") or [],
                }
            )
    return rows


def marginal_row(case: dict[str, Any], report: dict[str, Any]) -> dict[str, Any]:
    marginal = report.get("marginal") or {}
    return {
        **case,
        "report_status": report.get("status"),
        "target_granularity": report.get("target_granularity"),
        "target_card_uuid": report.get("target_card_uuid"),
        "target_hand_index": report.get("target_hand_index"),
        "target_action_keys": [
            key
            for branch in report.get("branches") or []
            if branch.get("branch_name") == "forced_draw_best"
            for key in (branch.get("target_action_keys") or [])
        ],
        "damage_delta": as_int(marginal.get("damage_delta")),
        "block_delta": as_int(marginal.get("block_delta")),
        "unblocked_reduction": as_int(marginal.get("unblocked_reduction")),
        "hp_loss_reduction": as_int(marginal.get("hp_loss_reduction")),
        "remaining_energy_delta": as_int(marginal.get("remaining_energy_delta")),
        "remaining_hand_delta": as_int(marginal.get("remaining_hand_delta")),
        "setup_gain": bool(marginal.get("setup_gain")),
        "lethal_gain": bool(marginal.get("lethal_gain")),
        "full_block_gain": bool(marginal.get("full_block_gain")),
        "marginal_score": as_int(marginal.get("marginal_score")),
        "label_strength": str(marginal.get("label_strength") or "not_applicable"),
    }


def cvar_bad(scores: list[int], fraction: float = 0.2) -> float:
    if not scores:
        return 0.0
    ordered = sorted(scores)
    count = max(1, int(len(ordered) * fraction))
    return round(mean(ordered[:count]), 3)


def aggregate(rows: list[dict[str, Any]], candidates: list[dict[str, Any]]) -> dict[str, Any]:
    by_card: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        by_card[str(row["target_action_card"])].append(row)
    card_rows = []
    for card, items in sorted(by_card.items()):
        scores = [as_int(row.get("marginal_score")) for row in items]
        card_rows.append(
            {
                "target_action_card": card,
                "sample_count": len(items),
                "label_counts": dict(sorted(Counter(str(row.get("label_strength")) for row in items).items())),
                "mean_delta": round(mean(scores), 3) if scores else 0.0,
                "p_positive": round(
                    sum(
                        1
                        for row in items
                        if row.get("label_strength") in {"robust_positive", "conditional_positive"}
                    )
                    / max(len(items), 1),
                    4,
                ),
                "p_harmful": round(
                    sum(1 for row in items if row.get("label_strength") == "harmful") / max(len(items), 1),
                    4,
                ),
                "cvar_bad_20pct": cvar_bad(scores),
            }
        )
    all_scores = [as_int(row.get("marginal_score")) for row in rows]
    return {
        "candidate_count": len(candidates),
        "sample_count": len(rows),
        "candidate_cards": dict(sorted(Counter(str(row["target_action_card"]) for row in candidates).items())),
        "label_counts": dict(sorted(Counter(str(row.get("label_strength")) for row in rows).items())),
        "mean_delta": round(mean(all_scores), 3) if all_scores else 0.0,
        "cvar_bad_20pct": cvar_bad(all_scores),
        "by_card": card_rows,
    }


def markdown(report: dict[str, Any]) -> str:
    summary = report["summary"]
    lines = [
        "# Trace-Derived Draw Marginal Mining",
        "",
        f"Generated: `{report['generated_at_utc']}`",
        "",
        "This mines real full-run trace combat states where a draw/search/resource card is playable.",
        "It compares no-target-action vs forced-target-action with current-turn plan queries.",
        "The labels are middle evidence, not card-choice truth.",
        "",
        "## Summary",
        "",
        f"- trace dir: `{report['config']['trace_dir']}`",
        f"- candidates selected: `{summary['candidate_count']}`",
        f"- completed samples: `{summary['sample_count']}`",
        f"- labels: `{summary['label_counts']}`",
        f"- mean marginal score: `{summary['mean_delta']}`",
        f"- CVaR bad 20%: `{summary['cvar_bad_20pct']}`",
        "",
        "| card | samples | labels | mean | p+ | p harmful | cvar bad |",
        "| --- | ---: | --- | ---: | ---: | ---: | ---: |",
    ]
    for row in summary["by_card"]:
        lines.append(
            f"| `{row['target_action_card']}` | {row['sample_count']} | `{row['label_counts']}` | "
            f"{row['mean_delta']} | {row['p_positive']:.2f} | {row['p_harmful']:.2f} | {row['cvar_bad_20pct']} |"
        )
    lines.extend(["", "## Top Positive/Negative Cases", ""])
    top = sorted(report["cases"], key=lambda row: as_int(row.get("marginal_score")), reverse=True)[:8]
    bottom = sorted(report["cases"], key=lambda row: as_int(row.get("marginal_score")))[:8]
    for title, rows in [("Positive", top), ("Negative", bottom)]:
        lines.extend([f"### {title}", "", "| case | card | label | score | floor | hp/incoming | reasons |"])
        lines.append("| --- | --- | --- | ---: | ---: | --- | --- |")
        for row in rows:
            lines.append(
                f"| `{row['case_id']}` | `{row['target_action_card']}` | `{row['label_strength']}` | "
                f"{row['marginal_score']} | {row['floor']} | {row['hp']}/{row['visible_incoming_damage']} | "
                f"`{','.join(row.get('selection_reasons') or [])}` |"
            )
        lines.append("")
    return "\n".join(lines) + "\n"


def main() -> None:
    args = parse_args()
    out_dir = resolve(args.out_dir)
    tool_path = resolve(args.tool_path)
    trace_dir = resolve(args.trace_dir) if args.trace_dir else latest_generated_trace_dir()
    if trace_dir is None:
        raise SystemExit("no trace dir provided and no generated_traces artifact found")
    if not trace_dir.exists():
        raise SystemExit(f"trace dir not found: {trace_dir}")
    if not args.skip_run and not tool_path.exists():
        raise SystemExit(f"tool path not found: {tool_path}")

    target_cards = {card_key(card.strip()) for card in args.cards.split(",") if card.strip()}
    candidates = scan_candidates(
        trace_dir,
        target_cards,
        max_cases_per_card=args.max_cases_per_card,
        max_cases=args.max_cases,
    )
    report_dir = out_dir / "reports"
    branch_output_rows: list[dict[str, Any]] = []
    marginal_rows: list[dict[str, Any]] = []
    manifest_rows: list[dict[str, Any]] = []
    for case in candidates:
        report_path = report_dir / f"{case['case_id']}.report.json"
        case_meta = {**case, "report_path": str(report_path)}
        manifest_rows.append(case_meta)
        if args.skip_run:
            continue
        run_probe(
            tool_path,
            report_path,
            case,
            max_depth=args.max_depth,
            max_nodes=args.max_nodes,
            beam_width=args.beam_width,
        )
        probe_report = load_json(report_path)
        branch_output_rows.extend(branch_rows(case_meta, probe_report))
        marginal_rows.append(marginal_row(case_meta, probe_report))

    report = {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {
            "trace_dir": str(trace_dir),
            "out_dir": str(out_dir),
            "cards": sorted(target_cards),
            "max_cases": args.max_cases,
            "max_cases_per_card": args.max_cases_per_card,
            "tool_path": str(tool_path),
            "max_depth": args.max_depth,
            "max_nodes": args.max_nodes,
            "beam_width": args.beam_width,
            "skip_run": bool(args.skip_run),
        },
        "summary": aggregate(marginal_rows, candidates),
        "cases": marginal_rows,
        "limitations": [
            "current_turn_only_horizon",
            "trace_policy_occupancy_distribution_not_balanced",
            "target_hand_instance_probe_tracks_uuid_but_forced_branch_may_choose_any_legal_target_for_that_instance",
            "labels_are_plan_query_deltas_not_card_choice_truth",
        ],
    }
    write_json(out_dir / "trace_draw_marginal_value_report.json", report)
    write_jsonl(out_dir / "branch_outcomes.jsonl", branch_output_rows)
    write_jsonl(out_dir / "marginal_examples.jsonl", marginal_rows)
    write_jsonl(out_dir / "manifest.jsonl", manifest_rows)
    (out_dir / "trace_draw_marginal_value_report.md").write_text(markdown(report), encoding="utf-8")
    print(json.dumps({"summary": report["summary"], "out_dir": str(out_dir)}, indent=2))


if __name__ == "__main__":
    main()
