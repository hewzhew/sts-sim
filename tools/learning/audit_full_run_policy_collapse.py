#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from statistics import mean
from typing import Any

from combat_rl_common import REPO_ROOT, write_json


REPORT_VERSION = "full_run_policy_collapse_audit_v0"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Audit full-run policy collapse symptoms from saved traces: reward/card-choice "
            "behavior and optional combat plan-query diagnostic signals. This is an eval report, "
            "not a trainer."
        )
    )
    parser.add_argument(
        "--trace-dir",
        action="append",
        default=[],
        metavar="POLICY=PATH",
        help="Saved full-run trace directory for one policy. Can be repeated.",
    )
    parser.add_argument(
        "--plan-query-report",
        action="append",
        default=[],
        metavar="POLICY=PATH",
        help="Optional combat_plan_query_batch_report.json for the same policy. Can be repeated.",
    )
    parser.add_argument("--top-cases", type=int, default=20)
    parser.add_argument(
        "--out",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "full_run_policy_collapse" / "collapse_audit.json",
    )
    parser.add_argument("--markdown-out", type=Path)
    return parser.parse_args()


def parse_named_paths(values: list[str], label: str) -> dict[str, Path]:
    out: dict[str, Path] = {}
    for raw in values:
        if "=" not in raw:
            raise SystemExit(f"{label} must use POLICY=PATH, got {raw!r}")
        name, path = raw.split("=", 1)
        name = name.strip()
        if not name:
            raise SystemExit(f"{label} has empty policy name: {raw!r}")
        out[name] = Path(path.strip())
    return out


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def trace_files(path: Path) -> list[Path]:
    files = sorted(path.glob("episode_*.json"))
    if not files:
        files = sorted(path.rglob("episode_*.json"))
    if not files:
        raise SystemExit(f"no episode_*.json traces found in {path}")
    return files


def chosen_candidate(step: dict[str, Any]) -> dict[str, Any]:
    candidate = step.get("chosen_candidate")
    if isinstance(candidate, dict) and candidate:
        return candidate
    candidates = step.get("action_mask") or []
    index = int(step.get("chosen_action_index") or 0)
    if 0 <= index < len(candidates) and isinstance(candidates[index], dict):
        return candidates[index]
    key = str(step.get("chosen_action_key") or "")
    for candidate in candidates:
        if isinstance(candidate, dict) and str(candidate.get("action_key") or "") == key:
            return candidate
    return {}


def card_candidates(step: dict[str, Any]) -> list[dict[str, Any]]:
    cards = []
    for candidate in step.get("action_mask") or []:
        if not isinstance(candidate, dict):
            continue
        key = str(candidate.get("action_key") or "")
        card = candidate.get("card") or {}
        plan_delta = candidate.get("plan_delta") or {}
        if "reward/select_card" not in key or not card:
            continue
        cards.append(
            {
                "action_key": key,
                "card_id": str(card.get("card_id") or ""),
                "rule_score": float(card.get("rule_score") or 0),
                "plan_adjusted_score": float(
                    plan_delta.get("plan_adjusted_score", card.get("rule_score") or 0) or 0
                ),
                "draws_cards": bool(card.get("draws_cards")),
                "scaling_piece": bool(card.get("scaling_piece")),
                "card_type_id": int(card.get("card_type_id") or 0),
                "rarity_id": int(card.get("rarity_id") or 0),
            }
        )
    return cards


def is_card_reward_step(step: dict[str, Any]) -> bool:
    if str(step.get("decision_type") or "") == "reward_card_choice":
        return True
    return bool(card_candidates(step))


def selected_card(step: dict[str, Any]) -> dict[str, Any] | None:
    key = str(step.get("chosen_action_key") or "")
    if key == "proceed":
        return None
    candidate = chosen_candidate(step)
    card = candidate.get("card") or {}
    plan_delta = candidate.get("plan_delta") or {}
    if not card:
        return None
    return {
        "action_key": key,
        "card_id": str(card.get("card_id") or ""),
        "rule_score": float(card.get("rule_score") or 0),
        "plan_adjusted_score": float(
            plan_delta.get("plan_adjusted_score", card.get("rule_score") or 0) or 0
        ),
        "draws_cards": bool(card.get("draws_cards")),
        "scaling_piece": bool(card.get("scaling_piece")),
        "card_type_id": int(card.get("card_type_id") or 0),
        "rarity_id": int(card.get("rarity_id") or 0),
    }


def summarize_trace_policy(policy: str, path: Path, top_cases: int) -> dict[str, Any]:
    decisions: list[dict[str, Any]] = []
    for trace_path in trace_files(path):
        trace = read_json(trace_path)
        seed = int((trace.get("summary") or {}).get("seed") or 0)
        for step in trace.get("steps") or []:
            if not is_card_reward_step(step):
                continue
            cards = card_candidates(step)
            if not cards:
                continue
            best = max(cards, key=lambda card: card["rule_score"])
            best_plan = max(cards, key=lambda card: card["plan_adjusted_score"])
            selected = selected_card(step)
            chosen_key = str(step.get("chosen_action_key") or "")
            chosen_score = selected["rule_score"] if selected else 5.0
            chosen_plan_score = selected["plan_adjusted_score"] if selected else 5.0
            gap = max(best["rule_score"] - chosen_score, 0.0)
            plan_gap = max(best_plan["plan_adjusted_score"] - chosen_plan_score, 0.0)
            offer_has_draw = any(card["draws_cards"] for card in cards)
            offer_has_scaling = any(card["scaling_piece"] for card in cards)
            decisions.append(
                {
                    "trace_file": str(trace_path),
                    "seed": seed,
                    "step_index": int(step.get("step_index") or step.get("step") or 0),
                    "floor": int(step.get("floor") or (step.get("observation") or {}).get("floor") or 0),
                    "act": int(step.get("act") or (step.get("observation") or {}).get("act") or 0),
                    "hp": int(step.get("hp") or (step.get("observation") or {}).get("current_hp") or 0),
                    "deck": (step.get("observation") or {}).get("deck") or {},
                    "chosen_action_key": chosen_key,
                    "selected_card": selected,
                    "best_card": best,
                    "best_plan_card": best_plan,
                    "best_gap": gap,
                    "plan_adjusted_gap": plan_gap,
                    "offer_has_draw": offer_has_draw,
                    "offer_has_scaling": offer_has_scaling,
                    "cards": cards,
                }
            )

    selects = [row for row in decisions if row["selected_card"]]
    skips = [row for row in decisions if not row["selected_card"]]
    draw_offers = [row for row in decisions if row["offer_has_draw"]]
    scaling_offers = [row for row in decisions if row["offer_has_scaling"]]
    draw_selects = [row for row in selects if row["selected_card"]["draws_cards"]]
    scaling_selects = [row for row in selects if row["selected_card"]["scaling_piece"]]
    skipped_good = [row for row in skips if row["best_card"]["rule_score"] >= 70]
    large_gap = [row for row in decisions if row["best_gap"] >= 30]
    large_plan_gap = [row for row in decisions if row["plan_adjusted_gap"] >= 30]

    return {
        "policy": policy,
        "trace_dir": str(path),
        "card_choice_decision_count": len(decisions),
        "card_select_count": len(selects),
        "card_skip_count": len(skips),
        "card_skip_share": ratio(len(skips), len(decisions)),
        "skipped_good_offer_count": len(skipped_good),
        "large_best_gap_count": len(large_gap),
        "selected_rule_score_average": average([row["selected_card"]["rule_score"] for row in selects]),
        "best_offer_rule_score_average": average([row["best_card"]["rule_score"] for row in decisions]),
        "best_gap_average": average([row["best_gap"] for row in decisions]),
        "plan_adjusted_best_offer_score_average": average(
            [row["best_plan_card"]["plan_adjusted_score"] for row in decisions]
        ),
        "plan_adjusted_gap_average": average([row["plan_adjusted_gap"] for row in decisions]),
        "plan_adjusted_large_gap_count": len(large_plan_gap),
        "draw_card_offer_count": len(draw_offers),
        "draw_card_select_count": len(draw_selects),
        "draw_card_select_share_when_offered": ratio(len(draw_selects), len(draw_offers)),
        "scaling_card_offer_count": len(scaling_offers),
        "scaling_card_select_count": len(scaling_selects),
        "scaling_card_select_share_when_offered": ratio(len(scaling_selects), len(scaling_offers)),
        "selected_card_type_counts": dict(Counter(str(row["selected_card"]["card_type_id"]) for row in selects)),
        "collapse_flags": collapse_flags(decisions, skips, skipped_good, large_gap, draw_offers, draw_selects, scaling_offers, scaling_selects),
        "top_regret_cases": compact_cases(sorted(large_gap, key=lambda row: row["best_gap"], reverse=True)[:top_cases]),
        "top_plan_adjusted_regret_cases": compact_cases(
            sorted(large_plan_gap, key=lambda row: row["plan_adjusted_gap"], reverse=True)[:top_cases]
        ),
        "top_skipped_good_cases": compact_cases(
            sorted(skipped_good, key=lambda row: row["best_card"]["rule_score"], reverse=True)[:top_cases]
        ),
    }


def collapse_flags(
    decisions: list[dict[str, Any]],
    skips: list[dict[str, Any]],
    skipped_good: list[dict[str, Any]],
    large_gap: list[dict[str, Any]],
    draw_offers: list[dict[str, Any]],
    draw_selects: list[dict[str, Any]],
    scaling_offers: list[dict[str, Any]],
    scaling_selects: list[dict[str, Any]],
) -> list[str]:
    flags = []
    if ratio(len(skips), len(decisions)) >= 0.20:
        flags.append("reward_card_skip_nontrivial")
    if skipped_good:
        flags.append("skipped_good_card_offer")
    if large_gap:
        flags.append("reward_card_large_rule_score_regret")
    if len(draw_offers) >= 10 and ratio(len(draw_selects), len(draw_offers)) < 0.20:
        flags.append("reward_card_draw_avoidance")
    if len(scaling_offers) >= 10 and ratio(len(scaling_selects), len(scaling_offers)) < 0.20:
        flags.append("reward_card_scaling_avoidance")
    return flags


def compact_cases(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    out = []
    for row in rows:
        selected = row["selected_card"]
        out.append(
            {
                "trace_file": row["trace_file"],
                "seed": row["seed"],
                "step_index": row["step_index"],
                "floor": row["floor"],
                "act": row["act"],
                "hp": row["hp"],
                "chosen": selected["card_id"] if selected else "Skip",
                "chosen_rule_score": selected["rule_score"] if selected else 5.0,
                "chosen_plan_adjusted_score": selected["plan_adjusted_score"] if selected else 5.0,
                "best": row["best_card"]["card_id"],
                "best_rule_score": row["best_card"]["rule_score"],
                "best_plan_adjusted": row["best_plan_card"]["card_id"],
                "best_plan_adjusted_score": row["best_plan_card"]["plan_adjusted_score"],
                "best_gap": row["best_gap"],
                "plan_adjusted_gap": row["plan_adjusted_gap"],
                "offer_has_draw": row["offer_has_draw"],
                "offer_has_scaling": row["offer_has_scaling"],
                "cards": [
                    {
                        "card_id": card["card_id"],
                        "rule_score": card["rule_score"],
                        "plan_adjusted_score": card["plan_adjusted_score"],
                        "draws_cards": card["draws_cards"],
                        "scaling_piece": card["scaling_piece"],
                    }
                    for card in row["cards"]
                ],
            }
        )
    return out


def plan_query_signal(policy: str, path: Path) -> dict[str, Any]:
    report = read_json(path)
    summary = report.get("summary") or {}
    return {
        "policy": policy,
        "report_path": str(path),
        "case_count": int(summary.get("case_count") or 0),
        "pressure_counts": summary.get("pressure_counts") or {},
        "query_status_counts": summary.get("query_status_counts") or {},
        "flag_counts": summary.get("flag_counts") or {},
        "needs_deeper_search_cases": int(summary.get("needs_deeper_search_cases") or 0),
    }


def average(values: list[float]) -> float:
    return float(mean(values)) if values else 0.0


def ratio(num: int | float, den: int | float) -> float:
    den_f = float(den)
    if den_f == 0.0:
        return 0.0
    return float(num) / den_f


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# Full-Run Policy Collapse Audit",
        "",
        f"Generated: `{report['generated_at_utc']}`",
        "",
        "This report is diagnostic. `rule_score` is a heuristic baseline, not a teacher label.",
        "",
        "## Reward/Card Choice",
        "",
        "| policy | choices | skip % | skipped good | large gap | plan gap | selected score | best offer | plan best | draw select % | scaling select % | flags |",
        "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|",
    ]
    for policy in report["policies"]:
        lines.append(
            "| {policy} | {choices} | {skip:.1%} | {skipped_good} | {large_gap} | {plan_gap} | {selected:.1f} | {best:.1f} | {plan_best:.1f} | {draw:.1%} | {scaling:.1%} | {flags} |".format(
                policy=policy["policy"],
                choices=policy["card_choice_decision_count"],
                skip=policy["card_skip_share"],
                skipped_good=policy["skipped_good_offer_count"],
                large_gap=policy["large_best_gap_count"],
                plan_gap=policy["plan_adjusted_large_gap_count"],
                selected=policy["selected_rule_score_average"],
                best=policy["best_offer_rule_score_average"],
                plan_best=policy["plan_adjusted_best_offer_score_average"],
                draw=policy["draw_card_select_share_when_offered"],
                scaling=policy["scaling_card_select_share_when_offered"],
                flags=", ".join(policy["collapse_flags"]) or "-",
            )
        )
    lines.extend(["", "## Worst Regret Cases", ""])
    for policy in report["policies"]:
        lines.extend([f"### {policy['policy']}", ""])
        for case in policy["top_regret_cases"][:8]:
            cards = ", ".join(
                f"{card['card_id']}:{card['rule_score']:.0f}/{card['plan_adjusted_score']:.0f}"
                f"{'D' if card['draws_cards'] else ''}{'S' if card['scaling_piece'] else ''}"
                for card in case["cards"]
            )
            lines.append(
                "- seed `{seed}` step `{step}` floor `{floor}` hp `{hp}`: chose `{chosen}` ({chosen_score:.0f}, plan {chosen_plan:.0f}), best `{best}` ({best_score:.0f}), plan-best `{plan_best}` ({plan_best_score:.0f}), gaps `{gap:.0f}` / plan `{plan_gap:.0f}`; [{cards}]".format(
                    seed=case["seed"],
                    step=case["step_index"],
                    floor=case["floor"],
                    hp=case["hp"],
                    chosen=case["chosen"],
                    chosen_score=case["chosen_rule_score"],
                    chosen_plan=case["chosen_plan_adjusted_score"],
                    best=case["best"],
                    best_score=case["best_rule_score"],
                    plan_best=case["best_plan_adjusted"],
                    plan_best_score=case["best_plan_adjusted_score"],
                    gap=case["best_gap"],
                    plan_gap=case["plan_adjusted_gap"],
                    cards=cards,
                )
            )
        lines.append("")
    if report["plan_query_signals"]:
        lines.extend(["## Combat Plan-Query Eval Signal", ""])
        lines.append("| policy | cases | missed full block | full-block damage gaps | no full-block under pressure | clean setup+block | deeper-search cases |")
        lines.append("|---|---:|---:|---:|---:|---:|---:|")
        for signal in report["plan_query_signals"]:
            flags = signal.get("flag_counts") or {}
            lines.append(
                "| {policy} | {cases} | {miss_block} | {damage_gap} | {no_block} | {setup} | {deeper} |".format(
                    policy=signal["policy"],
                    cases=signal["case_count"],
                    miss_block=flags.get("missed_full_block_line", 0),
                    damage_gap=flags.get("full_block_damage_gap", 0),
                    no_block=flags.get("no_full_block_line_under_pressure", 0),
                    setup=flags.get("setup_and_block_available_clean", 0),
                    deeper=signal.get("needs_deeper_search_cases", 0),
                )
            )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    trace_dirs = parse_named_paths(args.trace_dir, "--trace-dir")
    plan_reports = parse_named_paths(args.plan_query_report, "--plan-query-report")
    if not trace_dirs:
        raise SystemExit("at least one --trace-dir POLICY=PATH is required")

    policies = [
        summarize_trace_policy(policy, path, args.top_cases)
        for policy, path in sorted(trace_dirs.items())
    ]
    report = {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "purpose": "diagnose reward/card-choice collapse and use combat plan-query as eval signal",
        "policies": policies,
        "plan_query_signals": [
            plan_query_signal(policy, path)
            for policy, path in sorted(plan_reports.items())
        ],
    }
    write_json(args.out, report)
    markdown_out = args.markdown_out or args.out.with_suffix(".md")
    write_markdown(markdown_out, report)
    print(json.dumps({"out": str(args.out), "markdown_out": str(markdown_out)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
