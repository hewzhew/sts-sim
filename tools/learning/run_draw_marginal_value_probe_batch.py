#!/usr/bin/env python3
"""Run draw/search marginal-value probes over sampled synthetic author specs.

This batch is a training-prep diagnostic, not a card-choice oracle. Each sample
compares a no-target-action branch against a forced-target-action branch using
the Rust current-turn plan probe.
"""
from __future__ import annotations

import argparse
import json
import random
import subprocess
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from statistics import mean
from typing import Any

from combat_reranker_common import write_json, write_jsonl
from build_draw_payoff_author_specs import monster, spec

REPO_ROOT = Path(__file__).resolve().parents[2]
REPORT_VERSION = "draw_marginal_value_probe_batch_v0"


TEMPLATES: dict[str, dict[str, Any]] = {
    "BattleTrance": {
        "action_card": "BattleTrance",
        "template": "battle_trance_setup_damage",
        "hand": ["Battle Trance", "Wild Strike", "Strike_R", "Defend_R"],
        "draw_pool": ["Inflame", "Strike_R", "Strike_R", "Defend_R", "Strike_R", "Defend_R"],
        "monster_hp": 64,
        "hp": 70,
        "incoming": [6, 18, 30],
        "energy": [3, 4],
        "tags": ["battle_trance", "setup_payoff", "damage_payoff"],
    },
    "PommelStrike": {
        "action_card": "PommelStrike",
        "template": "pommel_strike_block",
        "hand": ["Pommel Strike", "Strike_R", "Defend_R"],
        "draw_pool": ["Defend_R", "Defend_R", "Strike_R", "Strike_R", "Strike_R"],
        "monster_hp": 48,
        "hp": 45,
        "incoming": [6, 18, 30],
        "energy": [2, 3],
        "tags": ["pommel_strike", "block_payoff", "pressure"],
    },
    "PommelStrikeNoPayoff": {
        "action_card": "PommelStrike",
        "template": "pommel_strike_spends_block_energy",
        "hand": ["Pommel Strike", "Defend_R", "Defend_R"],
        "draw_pool": ["Strike_R", "Strike_R", "Strike_R", "Strike_R", "Defend_R"],
        "monster_hp": 64,
        "hp": 45,
        "incoming": [18, 24, 30],
        "energy": [2],
        "tags": ["pommel_strike", "negative_payoff", "energy_spent_blocks_full_block"],
    },
    "Offering": {
        "action_card": "Offering",
        "template": "offering_resource_window",
        "hand": ["Offering", "Strike_R", "Defend_R"],
        "draw_pool": ["Inflame", "Strike_R", "Defend_R", "Strike_R", "Defend_R", "Strike_R"],
        "monster_hp": 72,
        "hp": 55,
        "incoming": [6, 18, 30],
        "energy": [2, 3],
        "tags": ["offering", "resource_window", "damage_payoff", "block_payoff"],
    },
    "OfferingNoPayoff": {
        "action_card": "Offering",
        "template": "offering_hp_cost_no_payoff",
        "hand": ["Offering", "Strike_R", "Defend_R"],
        "draw_pool": ["Strike_R", "Defend_R", "Strike_R", "Defend_R", "Strike_R", "Defend_R"],
        "monster_hp": 90,
        "hp": 18,
        "incoming": [0, 6],
        "energy": [3],
        "tags": ["offering", "negative_payoff", "hp_cost_without_query_gain"],
    },
    "SecretTechnique": {
        "action_card": "SecretTechnique",
        "template": "secret_technique_search_block",
        "hand": ["Secret Technique", "Strike_R", "Defend_R"],
        "draw_pool": ["Shrug It Off", "Inflame", "Strike_R", "Strike_R", "Strike_R", "Defend_R"],
        "monster_hp": 56,
        "hp": 48,
        "incoming": [6, 18, 30],
        "energy": [2, 3],
        "tags": ["secret_technique", "search_payoff", "block_payoff", "draw_payoff"],
    },
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run draw marginal value probe batch.")
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "draw_marginal_value" / "v0",
    )
    parser.add_argument(
        "--cards",
        default="BattleTrance,PommelStrike,PommelStrikeNoPayoff,Offering,OfferingNoPayoff,SecretTechnique",
    )
    parser.add_argument("--samples-per-template", type=int, default=32)
    parser.add_argument("--seed", type=int, default=91000)
    parser.add_argument("--tool-path", type=Path, default=REPO_ROOT / "target" / "debug" / "sts_dev_tool.exe")
    parser.add_argument("--max-depth", type=int, default=4)
    parser.add_argument("--max-nodes", type=int, default=1200)
    parser.add_argument("--beam-width", type=int, default=16)
    parser.add_argument("--skip-run", action="store_true")
    return parser.parse_args()


def resolve(path: Path) -> Path:
    return path if path.is_absolute() else REPO_ROOT / path


def build_sample(template_key: str, template: dict[str, Any], sample_index: int, rng: random.Random) -> dict[str, Any]:
    action_card = str(template.get("action_card") or template_key)
    incoming = rng.choice(template["incoming"])
    energy = rng.choice(template["energy"])
    draw_pile = list(template["draw_pool"])
    rng.shuffle(draw_pile)
    case_id = f"{template['template']}_{sample_index:04d}_inc{incoming}_e{energy}"
    payload = spec(
        name=case_id,
        description=(
            f"Draw marginal value sample for {action_card}; compare no-target-action vs forced-target-action. "
            f"incoming={incoming}, energy={energy}, draw_pile_sample={draw_pile}."
        ),
        hand=list(template["hand"]),
        draw_pile=draw_pile,
        monsters=[monster(hp=int(template["monster_hp"]), damage=incoming)],
        hp=int(template["hp"]),
        energy=energy,
        tags=[*template["tags"], "draw_marginal_value_probe"],
    )
    payload["provenance"]["draw_marginal"] = {
        "template": template["template"],
        "template_key": template_key,
        "target_action_card": action_card,
        "sample_index": sample_index,
        "incoming": incoming,
        "energy": energy,
        "draw_pile_sample": draw_pile,
    }
    return payload


def run_probe(
    tool_path: Path,
    spec_path: Path,
    report_path: Path,
    card: str,
    *,
    max_depth: int,
    max_nodes: int,
    beam_width: int,
) -> None:
    report_path.parent.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(tool_path),
        "combat",
        "draw-marginal-probe-author-spec",
        "--author-spec",
        str(spec_path),
        "--action-card",
        card,
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


def load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


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
        "damage_delta": int(marginal.get("damage_delta") or 0),
        "block_delta": int(marginal.get("block_delta") or 0),
        "unblocked_reduction": int(marginal.get("unblocked_reduction") or 0),
        "hp_loss_reduction": int(marginal.get("hp_loss_reduction") or 0),
        "remaining_energy_delta": int(marginal.get("remaining_energy_delta") or 0),
        "remaining_hand_delta": int(marginal.get("remaining_hand_delta") or 0),
        "setup_gain": bool(marginal.get("setup_gain")),
        "lethal_gain": bool(marginal.get("lethal_gain")),
        "full_block_gain": bool(marginal.get("full_block_gain")),
        "marginal_score": int(marginal.get("marginal_score") or 0),
        "label_strength": str(marginal.get("label_strength") or "not_applicable"),
    }


def cvar_bad(scores: list[int], fraction: float = 0.2) -> float:
    if not scores:
        return 0.0
    ordered = sorted(scores)
    count = max(1, int(len(ordered) * fraction))
    return round(mean(ordered[:count]), 3)


def aggregate(rows: list[dict[str, Any]]) -> dict[str, Any]:
    by_card: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        by_card[str(row["target_action_card"])].append(row)

    card_rows = []
    for card, items in sorted(by_card.items()):
        labels = Counter(str(row["label_strength"]) for row in items)
        scores = [int(row["marginal_score"]) for row in items]
        card_rows.append(
            {
                "target_action_card": card,
                "sample_count": len(items),
                "label_counts": dict(sorted(labels.items())),
                "mean_delta": round(mean(scores), 3) if scores else 0.0,
                "p_positive": round(
                    sum(1 for row in items if row["label_strength"] in {"robust_positive", "conditional_positive"})
                    / max(len(items), 1),
                    4,
                ),
                "p_harmful": round(sum(1 for row in items if row["label_strength"] == "harmful") / max(len(items), 1), 4),
                "p_equivalent": round(
                    sum(1 for row in items if row["label_strength"] == "equivalent") / max(len(items), 1),
                    4,
                ),
                "cvar_bad_20pct": cvar_bad(scores),
                "p_full_block_gain": round(sum(1 for row in items if row["full_block_gain"]) / max(len(items), 1), 4),
                "p_setup_gain": round(sum(1 for row in items if row["setup_gain"]) / max(len(items), 1), 4),
                "p_damage_gain": round(sum(1 for row in items if int(row["damage_delta"]) > 0) / max(len(items), 1), 4),
                "p_lethal_gain": round(sum(1 for row in items if row["lethal_gain"]) / max(len(items), 1), 4),
            }
        )
    all_scores = [int(row["marginal_score"]) for row in rows]
    return {
        "sample_count": len(rows),
        "label_counts": dict(sorted(Counter(str(row["label_strength"]) for row in rows).items())),
        "mean_delta": round(mean(all_scores), 3) if all_scores else 0.0,
        "cvar_bad_20pct": cvar_bad(all_scores),
        "by_card": card_rows,
    }


def markdown(report: dict[str, Any]) -> str:
    lines = [
        "# Draw Marginal Value Probe Batch",
        "",
        f"Generated: `{report['generated_at_utc']}`",
        "",
        "This report compares `no_draw_best` against `forced_draw_best` for the target action.",
        "Labels are current-turn, policy/probe-conditional middle evidence; they are not card-choice truth.",
        "The marginal includes the target card's full body, not only its draw/search text.",
        "",
        "## Summary",
        "",
        f"- samples: `{report['summary']['sample_count']}`",
        f"- labels: `{report['summary']['label_counts']}`",
        f"- mean marginal score: `{report['summary']['mean_delta']}`",
        f"- CVaR bad 20%: `{report['summary']['cvar_bad_20pct']}`",
        "",
        "| card | samples | labels | mean | p+ | p harmful | p equiv | cvar bad | p setup | p damage | p full block |",
        "| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for row in report["summary"]["by_card"]:
        lines.append(
            f"| `{row['target_action_card']}` | {row['sample_count']} | `{row['label_counts']}` | "
            f"{row['mean_delta']} | {row['p_positive']:.2f} | {row['p_harmful']:.2f} | "
            f"{row['p_equivalent']:.2f} | {row['cvar_bad_20pct']} | {row['p_setup_gain']:.2f} | "
            f"{row['p_damage_gain']:.2f} | {row['p_full_block_gain']:.2f} |"
        )
    lines.extend(["", "## Strongest Positive Examples", ""])
    positives = sorted(report["cases"], key=lambda row: int(row["marginal_score"]), reverse=True)[:12]
    lines.append("| case | card | label | score | dmg | block | leak | hp | setup |")
    lines.append("| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | --- |")
    for row in positives:
        lines.append(
            f"| `{row['case_id']}` | `{row['target_action_card']}` | `{row['label_strength']}` | "
            f"{row['marginal_score']} | {row['damage_delta']} | {row['block_delta']} | "
            f"{row['unblocked_reduction']} | {row['hp_loss_reduction']} | {row['setup_gain']} |"
        )
    return "\n".join(lines) + "\n"


def main() -> None:
    args = parse_args()
    out_dir = resolve(args.out_dir)
    tool_path = resolve(args.tool_path)
    if not args.skip_run and not tool_path.exists():
        raise SystemExit(f"tool path not found: {tool_path}")
    cards = [card.strip() for card in args.cards.split(",") if card.strip()]
    unknown = [card for card in cards if card not in TEMPLATES]
    if unknown:
        raise SystemExit(f"unknown cards: {unknown}; known={sorted(TEMPLATES)}")

    spec_dir = out_dir / "specs"
    report_dir = out_dir / "reports"
    spec_dir.mkdir(parents=True, exist_ok=True)
    report_dir.mkdir(parents=True, exist_ok=True)

    branch_output_rows: list[dict[str, Any]] = []
    marginal_rows: list[dict[str, Any]] = []
    manifest_rows: list[dict[str, Any]] = []
    rng = random.Random(args.seed)
    for template_key in cards:
        template = TEMPLATES[template_key]
        action_card = str(template.get("action_card") or template_key)
        for sample_index in range(args.samples_per_template):
            payload = build_sample(template_key, template, sample_index, rng)
            case_id = str(payload["name"])
            spec_path = spec_dir / f"{case_id}.json"
            report_path = report_dir / f"{case_id}.report.json"
            write_json(spec_path, payload)
            case_meta = {
                "case_id": case_id,
                "template": template["template"],
                "template_key": template_key,
                "target_action_card": action_card,
                "sample_index": sample_index,
                "incoming": payload["monsters"][0]["move_adjusted_damage"],
                "energy": payload["player"]["energy"],
                "spec_path": str(spec_path),
                "report_path": str(report_path),
            }
            manifest_rows.append(case_meta)
            if args.skip_run:
                continue
            run_probe(
                tool_path,
                spec_path,
                report_path,
                action_card,
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
            "out_dir": str(out_dir),
            "cards": cards,
            "samples_per_template": args.samples_per_template,
            "seed": args.seed,
            "tool_path": str(tool_path),
            "max_depth": args.max_depth,
            "max_nodes": args.max_nodes,
            "beam_width": args.beam_width,
            "skip_run": bool(args.skip_run),
        },
        "summary": aggregate(marginal_rows) if marginal_rows else {"sample_count": 0, "by_card": []},
        "cases": marginal_rows,
        "limitations": [
            "current_turn_only_horizon",
            "sampled_draw_pile_orders_are_synthetic",
            "labels_are_plan_query_deltas_not_card_choice_truth",
            "marginal_delta_includes_target_card_body_not_only_draw_text",
            "no_multi_turn_demand_decay_or_reshuffle_model",
        ],
    }
    write_json(out_dir / "draw_marginal_value_report.json", report)
    write_jsonl(out_dir / "branch_outcomes.jsonl", branch_output_rows)
    write_jsonl(out_dir / "marginal_examples.jsonl", marginal_rows)
    write_jsonl(out_dir / "manifest.jsonl", manifest_rows)
    (out_dir / "draw_marginal_value_report.md").write_text(markdown(report), encoding="utf-8")
    print(json.dumps({"summary": report["summary"], "out_dir": str(out_dir)}, indent=2))


if __name__ == "__main__":
    main()
