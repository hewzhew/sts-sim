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


REPORT_VERSION = "card_choice_comparison_audit_v0"


PLAN_FIELDS = (
    "frontload_delta",
    "block_delta",
    "draw_delta",
    "scaling_delta",
    "aoe_delta",
    "exhaust_delta",
    "kill_window_delta",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Audit reward-card choices as candidate comparisons. This is an attribution "
            "report over heuristic plan deltas, not a teacher label or trainer."
        )
    )
    parser.add_argument(
        "--trace-dir",
        action="append",
        default=[],
        metavar="POLICY=PATH",
        help="Saved full-run trace directory. Can be repeated.",
    )
    parser.add_argument("--min-gap", type=float, default=30.0)
    parser.add_argument("--top-cases", type=int, default=30)
    parser.add_argument(
        "--out",
        type=Path,
        default=REPO_ROOT
        / "tools"
        / "artifacts"
        / "card_choice_comparisons"
        / "comparison_report.json",
    )
    parser.add_argument("--markdown-out", type=Path)
    parser.add_argument("--top-cases-out", type=Path)
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


def is_card_reward_step(step: dict[str, Any]) -> bool:
    if str(step.get("decision_type") or "") == "reward_card_choice":
        return True
    return any(
        isinstance(candidate, dict)
        and str(candidate.get("action_key") or "").startswith("reward/select_card/")
        and isinstance(candidate.get("card"), dict)
        for candidate in step.get("action_mask") or []
    )


def candidate_rows(step: dict[str, Any], profile: dict[str, Any]) -> list[dict[str, Any]]:
    rows = []
    for candidate in step.get("action_mask") or []:
        if not isinstance(candidate, dict):
            continue
        key = str(candidate.get("action_key") or "")
        card = candidate.get("card") or {}
        if not key.startswith("reward/select_card/") or not card:
            continue
        delta = candidate.get("plan_delta") or {}
        row = {
            "action_key": key,
            "card_id": str(card.get("card_id") or ""),
            "rule_score": float(card.get("rule_score") or 0),
            "plan_adjusted_score": float(
                delta.get("plan_adjusted_score", card.get("rule_score") or 0) or 0
            ),
            "card_type_id": int(card.get("card_type_id") or 0),
            "rarity_id": int(card.get("rarity_id") or 0),
            "cost": int(card.get("cost") or 0),
            "draws_cards": bool(card.get("draws_cards")),
            "scaling_piece": bool(card.get("scaling_piece")),
            "aoe": bool(card.get("aoe") or card.get("multi_damage")),
            "starter_basic": bool(card.get("starter_basic")),
            "deck_copies": int(card.get("deck_copies") or 0),
            "plan_delta": {field: int(delta.get(field) or 0) for field in PLAN_FIELDS},
            "deck_deficit_bonus": int(delta.get("deck_deficit_bonus") or 0),
            "bloat_penalty": int(delta.get("bloat_penalty") or 0),
            "duplicate_penalty": int(delta.get("duplicate_penalty") or 0),
            "setup_cashout_risk_delta": int(delta.get("setup_cashout_risk_delta") or 0),
        }
        row["fills_deficit"] = fills_deficit(row, profile)
        row["worsens_risk"] = worsens_risk(row)
        row["likely_role"] = likely_role(row)
        rows.append(row)
    return rows


def fills_deficit(row: dict[str, Any], profile: dict[str, Any]) -> list[str]:
    delta = row["plan_delta"]
    out = []
    if delta["frontload_delta"] > 0 and int(profile.get("frontload_supply") or 0) < 70:
        out.append("frontload")
    if delta["block_delta"] > 0 and int(profile.get("block_supply") or 0) < 50:
        out.append("block")
    if delta["draw_delta"] > 0 and int(profile.get("draw_supply") or 0) < 35:
        out.append("draw")
    if delta["scaling_delta"] > 0 and int(profile.get("scaling_supply") or 0) < 35:
        out.append("scaling")
    if delta["aoe_delta"] > 0 and int(profile.get("aoe_supply") or 0) < 18:
        out.append("aoe")
    if delta["exhaust_delta"] > 0 and int(profile.get("exhaust_supply") or 0) < 12:
        out.append("exhaust")
    if delta["kill_window_delta"] > 0 and int(profile.get("kill_window_supply") or 0) <= 0:
        out.append("kill_window")
    return out


def worsens_risk(row: dict[str, Any]) -> list[str]:
    out = []
    if int(row.get("bloat_penalty") or 0) < 0:
        out.append("deck_bloat")
    if int(row.get("duplicate_penalty") or 0) < 0:
        out.append("duplicate")
    if int(row.get("setup_cashout_risk_delta") or 0) > 0:
        out.append("setup_cashout")
    if int(row.get("cost") or 0) >= 3:
        out.append("high_curve")
    if float(row.get("plan_adjusted_score") or 0) <= 20:
        out.append("low_plan_value")
    return out


def likely_role(row: dict[str, Any]) -> str:
    delta = row["plan_delta"]
    if delta["aoe_delta"] > 0:
        return "aoe_answer"
    if delta["kill_window_delta"] > 0:
        return "kill_window"
    if delta["scaling_delta"] > 0:
        return "scaling_piece"
    if delta["draw_delta"] > 0 and delta["block_delta"] > 0:
        return "block_draw"
    if delta["draw_delta"] > 0 and delta["frontload_delta"] > 0:
        return "frontload_draw"
    if delta["draw_delta"] > 0:
        return "draw_utility"
    if delta["frontload_delta"] >= 30:
        return "transition_attack"
    if delta["frontload_delta"] > 0:
        return "attack"
    if delta["block_delta"] > 0:
        return "block"
    if delta["exhaust_delta"] > 0:
        return "exhaust_tool"
    if row["card_type_id"] == 3:
        return "setup_power"
    return "low_value_or_speculative"


def selected_row(step: dict[str, Any], rows: list[dict[str, Any]]) -> dict[str, Any] | None:
    key = str(step.get("chosen_action_key") or "")
    if key == "proceed":
        return None
    for row in rows:
        if row["action_key"] == key:
            return row
    candidate = chosen_candidate(step)
    card = candidate.get("card") or {}
    if not card:
        return None
    card_id = str(card.get("card_id") or "")
    for row in rows:
        if row["card_id"] == card_id:
            return row
    return None


def score_gap(best: dict[str, Any], selected: dict[str, Any] | None, key: str) -> float:
    selected_score = float(selected.get(key) or 0) if selected else 5.0
    return max(float(best.get(key) or 0) - selected_score, 0.0)


def classify_regret(
    *,
    selected: dict[str, Any] | None,
    best_rule: dict[str, Any],
    best_plan: dict[str, Any],
    profile: dict[str, Any],
    act: int,
    floor: int,
    rule_gap: float,
    plan_gap: float,
    min_gap: float,
) -> tuple[list[str], list[str], bool, str]:
    if rule_gap < min_gap and plan_gap < min_gap:
        return ["small_gap_ignore"], ["gap below audit threshold"], False, "low"
    if selected is None:
        kinds = ["skip_good_offer"]
        if best_plan["plan_delta"]["draw_delta"] > 0:
            kinds.append("missed_draw")
        if best_plan["plan_delta"]["scaling_delta"] > 0:
            kinds.append("missed_scaling")
        if best_plan["plan_delta"]["aoe_delta"] > 0:
            kinds.append("missed_aoe")
        return kinds, ["policy skipped the card reward offer"], False, confidence(rule_gap, plan_gap)

    kinds = []
    notes = []
    best = best_plan if plan_gap >= rule_gap else best_rule
    selected_delta = selected["plan_delta"]
    best_delta = best["plan_delta"]

    if best_delta["aoe_delta"] > selected_delta["aoe_delta"] + 8 and (
        int(profile.get("aoe_supply") or 0) < 18 or act >= 2 or floor >= 7
    ):
        kinds.append("missed_aoe")
        notes.append("best candidate improves AoE/readiness more than chosen")
    if best_delta["scaling_delta"] > selected_delta["scaling_delta"] + 8:
        kinds.append("missed_scaling")
        notes.append("best candidate improves scaling more than chosen")
    if best_delta["draw_delta"] > selected_delta["draw_delta"] + 6:
        kinds.append("missed_draw")
        notes.append("best candidate improves draw/cashout more than chosen")
    if best_delta["block_delta"] > selected_delta["block_delta"] + 10:
        kinds.append("missed_block")
        notes.append("best candidate improves block plan more than chosen")
    if best_delta["frontload_delta"] > selected_delta["frontload_delta"] + 12:
        kinds.append("missed_frontload")
        notes.append("best candidate improves frontload more than chosen")
    if best_delta["kill_window_delta"] > selected_delta["kill_window_delta"]:
        kinds.append("missed_kill_window")
        notes.append("best candidate offers a kill-window payoff")

    if selected["bloat_penalty"] < best["bloat_penalty"]:
        kinds.append("picked_bloat")
        notes.append("chosen candidate has worse bloat penalty")
    if selected["duplicate_penalty"] < best["duplicate_penalty"]:
        kinds.append("picked_duplicate")
        notes.append("chosen candidate has worse duplicate penalty")
    if selected["plan_adjusted_score"] <= 20 or selected["rule_score"] <= 20:
        kinds.append("picked_low_synergy")
        notes.append("chosen candidate has very low heuristic value")
    if selected["likely_role"] in {"draw_utility", "exhaust_tool", "low_value_or_speculative"} and (
        "frontload" in best.get("fills_deficit", [])
        or "aoe" in best.get("fills_deficit", [])
        or "scaling" in best.get("fills_deficit", [])
    ):
        kinds.append("picked_speculative_over_core_need")
        notes.append("chosen utility card lost to a clearer deck need")

    needs_rollout = False
    if abs(rule_gap - plan_gap) >= 50:
        needs_rollout = True
        notes.append("rule and plan-adjusted gaps diverge substantially")
    if best["setup_cashout_risk_delta"] > 0 or selected["setup_cashout_risk_delta"] > 0:
        needs_rollout = True
        notes.append("setup value depends on future cashout")
    if best["likely_role"] in {"scaling_piece", "setup_power"} and plan_gap < 60:
        needs_rollout = True
        notes.append("scaling choice has medium gap and should be rollout-checked")

    if not kinds:
        kinds.append("score_gap_unclassified")
        needs_rollout = True
        notes.append("large score gap was not explained by V0 plan fields")

    return sorted(set(kinds)), notes, needs_rollout, confidence(rule_gap, plan_gap)


def confidence(rule_gap: float, plan_gap: float) -> str:
    gap = max(rule_gap, plan_gap)
    if gap >= 90:
        return "high"
    if gap >= 45:
        return "medium"
    return "low"


def compare_policy(policy: str, path: Path, min_gap: float, top_cases: int) -> dict[str, Any]:
    comparisons = []
    for trace_path in trace_files(path):
        trace = read_json(trace_path)
        seed = int((trace.get("summary") or {}).get("seed") or 0)
        for step in trace.get("steps") or []:
            if not is_card_reward_step(step):
                continue
            obs = step.get("observation") or {}
            profile = obs.get("plan_profile") or {}
            rows = candidate_rows(step, profile)
            if not rows:
                continue
            selected = selected_row(step, rows)
            best_rule = max(rows, key=lambda row: row["rule_score"])
            best_plan = max(rows, key=lambda row: row["plan_adjusted_score"])
            rule_gap = score_gap(best_rule, selected, "rule_score")
            plan_gap = score_gap(best_plan, selected, "plan_adjusted_score")
            act = int(step.get("act") or obs.get("act") or 0)
            floor = int(step.get("floor") or obs.get("floor") or 0)
            kinds, notes, needs_rollout, conf = classify_regret(
                selected=selected,
                best_rule=best_rule,
                best_plan=best_plan,
                profile=profile,
                act=act,
                floor=floor,
                rule_gap=rule_gap,
                plan_gap=plan_gap,
                min_gap=min_gap,
            )
            comparisons.append(
                {
                    "policy": policy,
                    "trace_file": str(trace_path),
                    "seed": seed,
                    "step_index": int(step.get("step_index") or step.get("step") or 0),
                    "act": act,
                    "floor": floor,
                    "hp": int(step.get("hp") or obs.get("current_hp") or 0),
                    "deck": obs.get("deck") or {},
                    "deck_plan_profile": profile,
                    "chosen": compact_candidate(selected) if selected else skip_candidate(),
                    "best_by_rule": compact_candidate(best_rule),
                    "best_by_plan_adjusted": compact_candidate(best_plan),
                    "candidate_cards": [compact_candidate(row) for row in rows],
                    "rule_gap": rule_gap,
                    "plan_adjusted_gap": plan_gap,
                    "regret_score": max(rule_gap, plan_gap),
                    "regret_kind": kinds,
                    "needs_rollout": needs_rollout,
                    "confidence": conf,
                    "notes": notes,
                }
            )

    actionable = [row for row in comparisons if "small_gap_ignore" not in row["regret_kind"]]
    regret_counts = Counter(kind for row in actionable for kind in row["regret_kind"])
    role_counts = Counter(row["chosen"]["likely_role"] for row in comparisons)
    best_role_counts = Counter(row["best_by_plan_adjusted"]["likely_role"] for row in comparisons)
    return {
        "policy": policy,
        "trace_dir": str(path),
        "decision_count": len(comparisons),
        "actionable_regret_count": len(actionable),
        "needs_rollout_count": sum(1 for row in actionable if row["needs_rollout"]),
        "average_rule_gap": average([row["rule_gap"] for row in comparisons]),
        "average_plan_adjusted_gap": average([row["plan_adjusted_gap"] for row in comparisons]),
        "regret_kind_counts": dict(regret_counts),
        "chosen_role_counts": dict(role_counts),
        "best_plan_role_counts": dict(best_role_counts),
        "top_regret_cases": [
            compact_case(row)
            for row in sorted(actionable, key=lambda row: row["regret_score"], reverse=True)[
                :top_cases
            ]
        ],
        "comparisons": comparisons,
    }


def compact_candidate(row: dict[str, Any]) -> dict[str, Any]:
    return {
        "card_id": row["card_id"],
        "action_key": row["action_key"],
        "rule_score": row["rule_score"],
        "plan_adjusted_score": row["plan_adjusted_score"],
        "likely_role": row["likely_role"],
        "fills_deficit": row["fills_deficit"],
        "worsens_risk": row["worsens_risk"],
        "plan_delta": row["plan_delta"],
        "deck_deficit_bonus": row["deck_deficit_bonus"],
        "bloat_penalty": row["bloat_penalty"],
        "duplicate_penalty": row["duplicate_penalty"],
        "setup_cashout_risk_delta": row["setup_cashout_risk_delta"],
    }


def skip_candidate() -> dict[str, Any]:
    return {
        "card_id": "Skip",
        "action_key": "proceed",
        "rule_score": 5.0,
        "plan_adjusted_score": 5.0,
        "likely_role": "skip",
        "fills_deficit": [],
        "worsens_risk": [],
        "plan_delta": {field: 0 for field in PLAN_FIELDS},
        "deck_deficit_bonus": 0,
        "bloat_penalty": 0,
        "duplicate_penalty": 0,
        "setup_cashout_risk_delta": 0,
    }


def compact_case(row: dict[str, Any]) -> dict[str, Any]:
    return {
        "seed": row["seed"],
        "step_index": row["step_index"],
        "act": row["act"],
        "floor": row["floor"],
        "hp": row["hp"],
        "chosen": row["chosen"],
        "best_by_rule": row["best_by_rule"],
        "best_by_plan_adjusted": row["best_by_plan_adjusted"],
        "rule_gap": row["rule_gap"],
        "plan_adjusted_gap": row["plan_adjusted_gap"],
        "regret_kind": row["regret_kind"],
        "needs_rollout": row["needs_rollout"],
        "confidence": row["confidence"],
        "notes": row["notes"],
        "candidate_cards": row["candidate_cards"],
        "deck_plan_profile": row["deck_plan_profile"],
        "trace_file": row["trace_file"],
    }


def average(values: list[float]) -> float:
    return float(mean(values)) if values else 0.0


def pct(value: float) -> str:
    return f"{value:.1%}"


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# Card Choice Comparison Audit",
        "",
        f"Generated: `{report['generated_at_utc']}`",
        "",
        "This report is heuristic attribution over plan deltas. It is not a teacher label.",
        "",
        "## Summary",
        "",
        "| policy | decisions | actionable | needs rollout | avg rule gap | avg plan gap | top regret kinds |",
        "|---|---:|---:|---:|---:|---:|---|",
    ]
    for policy in report["policies"]:
        top_kinds = ", ".join(
            f"{kind}:{count}"
            for kind, count in Counter(policy["regret_kind_counts"]).most_common(5)
        )
        lines.append(
            "| {policy} | {decisions} | {actionable} | {rollout} ({rollout_share}) | {rule_gap:.1f} | {plan_gap:.1f} | {kinds} |".format(
                policy=policy["policy"],
                decisions=policy["decision_count"],
                actionable=policy["actionable_regret_count"],
                rollout=policy["needs_rollout_count"],
                rollout_share=pct(
                    policy["needs_rollout_count"] / max(policy["actionable_regret_count"], 1)
                ),
                rule_gap=policy["average_rule_gap"],
                plan_gap=policy["average_plan_adjusted_gap"],
                kinds=top_kinds or "-",
            )
        )

    lines.extend(["", "## Top Regret Cases", ""])
    for policy in report["policies"]:
        lines.extend([f"### {policy['policy']}", ""])
        for case in policy["top_regret_cases"][:12]:
            cards = ", ".join(
                "{card}:{rule:.0f}/{plan:.0f}:{role}{tags}".format(
                    card=candidate["card_id"],
                    rule=candidate["rule_score"],
                    plan=candidate["plan_adjusted_score"],
                    role=candidate["likely_role"],
                    tags=(
                        "[" + ",".join(candidate["fills_deficit"]) + "]"
                        if candidate["fills_deficit"]
                        else ""
                    ),
                )
                for candidate in case["candidate_cards"]
            )
            lines.append(
                "- seed `{seed}` step `{step}` floor `{floor}` hp `{hp}`: chose `{chosen}` ({chosen_role}, {chosen_score:.0f}/{chosen_plan:.0f}) vs plan-best `{best}` ({best_role}, {best_score:.0f}/{best_plan:.0f}); gaps `{rule_gap:.0f}` / plan `{plan_gap:.0f}`; kinds `{kinds}`; rollout `{rollout}`; [{cards}]".format(
                    seed=case["seed"],
                    step=case["step_index"],
                    floor=case["floor"],
                    hp=case["hp"],
                    chosen=case["chosen"]["card_id"],
                    chosen_role=case["chosen"]["likely_role"],
                    chosen_score=case["chosen"]["rule_score"],
                    chosen_plan=case["chosen"]["plan_adjusted_score"],
                    best=case["best_by_plan_adjusted"]["card_id"],
                    best_role=case["best_by_plan_adjusted"]["likely_role"],
                    best_score=case["best_by_plan_adjusted"]["rule_score"],
                    best_plan=case["best_by_plan_adjusted"]["plan_adjusted_score"],
                    rule_gap=case["rule_gap"],
                    plan_gap=case["plan_adjusted_gap"],
                    kinds=", ".join(case["regret_kind"]),
                    rollout="yes" if case["needs_rollout"] else "no",
                    cards=cards,
                )
            )
            if case["notes"]:
                lines.append(f"  - notes: {'; '.join(case['notes'])}")
        lines.append("")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_top_cases_markdown(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# Top Card Choice Regret Cases",
        "",
        f"Generated: `{report['generated_at_utc']}`",
        "",
        "Each case compares the chosen card against the plan-adjusted best candidate.",
        "",
    ]
    for policy in report["policies"]:
        lines.extend([f"## {policy['policy']}", ""])
        for case in policy["top_regret_cases"]:
            lines.extend(
                [
                    "### seed `{seed}` step `{step}` floor `{floor}`".format(
                        seed=case["seed"],
                        step=case["step_index"],
                        floor=case["floor"],
                    ),
                    "",
                    "- chosen: `{}` ({}, {:.0f}/{:.0f})".format(
                        case["chosen"]["card_id"],
                        case["chosen"]["likely_role"],
                        case["chosen"]["rule_score"],
                        case["chosen"]["plan_adjusted_score"],
                    ),
                    "- plan-best: `{}` ({}, {:.0f}/{:.0f})".format(
                        case["best_by_plan_adjusted"]["card_id"],
                        case["best_by_plan_adjusted"]["likely_role"],
                        case["best_by_plan_adjusted"]["rule_score"],
                        case["best_by_plan_adjusted"]["plan_adjusted_score"],
                    ),
                    "- regret: `{}`; rule gap `{:.0f}`, plan gap `{:.0f}`, confidence `{}`, rollout `{}`".format(
                        ", ".join(case["regret_kind"]),
                        case["rule_gap"],
                        case["plan_adjusted_gap"],
                        case["confidence"],
                        "yes" if case["needs_rollout"] else "no",
                    ),
                ]
            )
            if case["notes"]:
                lines.append(f"- notes: {'; '.join(case['notes'])}")
            lines.append("")
            lines.append("| candidate | role | score | plan | fills deficit | risks |")
            lines.append("|---|---|---:|---:|---|---|")
            for candidate in case["candidate_cards"]:
                lines.append(
                    "| {card} | {role} | {rule:.0f} | {plan:.0f} | {fills} | {risks} |".format(
                        card=candidate["card_id"],
                        role=candidate["likely_role"],
                        rule=candidate["rule_score"],
                        plan=candidate["plan_adjusted_score"],
                        fills=", ".join(candidate["fills_deficit"]) or "-",
                        risks=", ".join(candidate["worsens_risk"]) or "-",
                    )
                )
            lines.append("")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    trace_dirs = parse_named_paths(args.trace_dir, "--trace-dir")
    if not trace_dirs:
        raise SystemExit("at least one --trace-dir POLICY=PATH is required")
    policies = [
        compare_policy(policy, path, args.min_gap, args.top_cases)
        for policy, path in sorted(trace_dirs.items())
    ]
    report = {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {
            "min_gap": args.min_gap,
            "top_cases": args.top_cases,
        },
        "policies": policies,
    }
    write_json(args.out, report)
    markdown_out = args.markdown_out or args.out.with_suffix(".md")
    write_markdown(markdown_out, report)
    top_cases_out = args.top_cases_out or args.out.parent / "top_regret_cases.md"
    write_top_cases_markdown(top_cases_out, report)
    print(
        json.dumps(
            {
                "out": str(args.out),
                "markdown_out": str(markdown_out),
                "top_cases_out": str(top_cases_out),
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
