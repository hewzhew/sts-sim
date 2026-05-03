#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json


REPORT_VERSION = "cashout_micro_probe_runner_v0_1"
DEFAULT_QUEUE = (
    REPO_ROOT
    / "tools"
    / "artifacts"
    / "card_cashout_rollout_labels"
    / "v1_1_attribution_100case_cashout_v0_6"
    / "cashout_micro_probe_queue.jsonl"
)
DEFAULT_OUT_DIR = REPO_ROOT / "tools" / "artifacts" / "cashout_micro_probes" / "v0"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run trace-prefix cashout micro probes for selected queue rows. "
            "V0 supports AoE kill-timing and resource-window families by reusing "
            "full_run_counterfactual_lab; it does not generate standalone canonical encounters."
        )
    )
    parser.add_argument("--queue", type=Path, default=DEFAULT_QUEUE)
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument(
        "--families",
        default="aoe_kill_timing,resource_window",
        help="Comma-separated probe families to run, or 'all'.",
    )
    parser.add_argument("--continuation-policies", default="rule_baseline_v0,plan_query_v0")
    parser.add_argument("--horizons", default="40,80")
    parser.add_argument("--max-cases", type=int, default=6)
    parser.add_argument("--max-branches", type=int, default=8)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=5000)
    parser.add_argument("--driver-binary", type=Path)
    parser.add_argument("--allow-replay-mismatch", action="store_true")
    return parser.parse_args()


def resolve(path: Path) -> Path:
    return path if path.is_absolute() else REPO_ROOT / path


def parse_csv(text: str) -> list[str]:
    return [part.strip() for part in str(text or "").split(",") if part.strip()]


def parse_int_csv(text: str) -> list[int]:
    return [int(part) for part in parse_csv(text)]


def read_json(path: Path) -> dict[str, Any]:
    with resolve(path).open("r", encoding="utf-8") as handle:
        return json.load(handle)


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    real = resolve(path)
    rows = []
    with real.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    real = resolve(path)
    real.parent.mkdir(parents=True, exist_ok=True)
    with real.open("w", encoding="utf-8", newline="\n") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")


def num(value: Any) -> float:
    try:
        return float(value or 0.0)
    except (TypeError, ValueError):
        return 0.0


def safe_name(text: str) -> str:
    return "".join(ch if ch.isalnum() or ch in {"_", "-", "."} else "_" for ch in text)


def selected_rows(args: argparse.Namespace) -> list[dict[str, Any]]:
    rows = read_jsonl(args.queue)
    families = None if args.families.strip().lower() == "all" else set(parse_csv(args.families))
    selected = []
    for row in rows:
        row_families = set(row.get("probe_families") or [])
        if families is not None and not (row_families & families):
            continue
        selected.append(row)
        if len(selected) >= int(args.max_cases):
            break
    return selected


def run_counterfactual_report(
    *,
    args: argparse.Namespace,
    row: dict[str, Any],
    family: str,
    continuation_policy: str,
    horizon: int,
    out_path: Path,
) -> dict[str, Any]:
    source = row.get("source") or {}
    trace_file = resolve(Path(str(source.get("trace_file") or "")))
    if not trace_file.exists():
        return {"status": "failed", "error": f"missing trace file: {trace_file}"}
    cmd = [
        sys.executable,
        str(REPO_ROOT / "tools" / "learning" / "full_run_counterfactual_lab.py"),
        "--trace-file",
        str(trace_file),
        "--step-index",
        str(int(source.get("step_index") or 0)),
        "--continuation-policy",
        continuation_policy,
        "--continuation-steps",
        str(int(horizon)),
        "--branch-indices",
        "all",
        "--max-branches",
        str(int(args.max_branches)),
        "--ascension",
        str(int(args.ascension)),
        "--class",
        str(args.player_class),
        "--max-steps",
        str(int(args.max_steps)),
        "--out",
        str(out_path),
    ]
    if args.final_act:
        cmd.append("--final-act")
    if args.driver_binary:
        cmd.extend(["--driver-binary", str(resolve(args.driver_binary))])
    if args.allow_replay_mismatch:
        cmd.append("--allow-replay-mismatch")
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
            "command": cmd,
            "report_path": str(out_path),
        }
    report = read_json(out_path)
    return {
        "status": "ok",
        "report_path": str(out_path),
        "rows_path": str(out_path.with_suffix(".rows.jsonl")),
        "report": report,
    }


def outcome_by_key(report: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {str(row.get("candidate_key") or ""): row for row in report.get("outcomes") or []}


def branch_score(outcome: dict[str, Any] | None) -> float:
    if not outcome:
        return -1e9
    delta = outcome.get("outcome_delta") or {}
    end = outcome.get("end") or {}
    attr = outcome.get("attribution") or {}
    result = str(end.get("result") or "")
    terminal_bonus = 0.0
    if result == "victory":
        terminal_bonus += 500.0
    elif result == "defeat":
        terminal_bonus -= 250.0
    return (
        num(delta.get("floor_delta")) * 100.0
        + num(delta.get("combat_win_delta")) * 45.0
        + num(end.get("current_hp")) * 1.5
        + num(attr.get("alive_monster_reduction_observed")) * 10.0
        + num(outcome.get("reward_total")) * 2.0
        + terminal_bonus
    )


def compact_outcome(outcome: dict[str, Any] | None) -> dict[str, Any]:
    if not outcome:
        return {}
    attr = outcome.get("attribution") or {}
    delta = outcome.get("outcome_delta") or {}
    end = outcome.get("end") or {}
    card = outcome.get("candidate_card") or {}
    return {
        "candidate_key": outcome.get("candidate_key"),
        "card_id": card.get("card_id") or ("proceed" if outcome.get("candidate_key") == "proceed" else None),
        "score": round(branch_score(outcome), 3),
        "floor_delta": delta.get("floor_delta"),
        "combat_win_delta": delta.get("combat_win_delta"),
        "hp_delta": delta.get("hp_delta"),
        "end_floor": end.get("floor"),
        "end_hp": end.get("current_hp"),
        "result": end.get("result"),
        "reward_total": round(num(outcome.get("reward_total")), 3),
        "attribution": {
            "hp_loss": attr.get("hp_loss_observed"),
            "monster_hp": attr.get("monster_hp_reduction_observed"),
            "kills": attr.get("alive_monster_reduction_observed"),
            "combat_turns": attr.get("combat_turns_observed"),
            "plays": attr.get("combat_play_card_count"),
            "draw_played": attr.get("draw_played"),
            "scaling_played": attr.get("scaling_played"),
            "exhaust_played": attr.get("exhaust_played"),
            "draw_cards": [record.get("card_id") for record in attr.get("draw_cards_played") or []],
            "setup_cards": [
                record.get("card_id") for record in attr.get("setup_or_scaling_cards_played") or []
            ],
            "exhaust_cards": [record.get("card_id") for record in attr.get("exhaust_cards_played") or []],
            "energy_unused": attr.get("energy_unused_on_end_turn_total"),
            "energy_unused_max": attr.get("energy_unused_on_end_turn_max"),
            "max_unblocked": attr.get("max_visible_unblocked_damage"),
        },
    }


def diff_metrics(left: dict[str, Any] | None, right: dict[str, Any] | None) -> dict[str, Any]:
    if not left or not right:
        return {"missing_branch": True}
    left_attr = left.get("attribution") or {}
    right_attr = right.get("attribution") or {}
    left_delta = left.get("outcome_delta") or {}
    right_delta = right.get("outcome_delta") or {}
    left_end = left.get("end") or {}
    right_end = right.get("end") or {}
    return {
        "score": round(branch_score(left) - branch_score(right), 3),
        "floor": num(left_delta.get("floor_delta")) - num(right_delta.get("floor_delta")),
        "combat_wins": num(left_delta.get("combat_win_delta")) - num(right_delta.get("combat_win_delta")),
        "end_hp": num(left_end.get("current_hp")) - num(right_end.get("current_hp")),
        "hp_loss": num(left_attr.get("hp_loss_observed")) - num(right_attr.get("hp_loss_observed")),
        "monster_hp": num(left_attr.get("monster_hp_reduction_observed")) - num(right_attr.get("monster_hp_reduction_observed")),
        "kills": num(left_attr.get("alive_monster_reduction_observed")) - num(right_attr.get("alive_monster_reduction_observed")),
        "combat_turns": num(left_attr.get("combat_turns_observed")) - num(right_attr.get("combat_turns_observed")),
        "plays": num(left_attr.get("combat_play_card_count")) - num(right_attr.get("combat_play_card_count")),
        "energy_unused": num(left_attr.get("energy_unused_on_end_turn_total")) - num(right_attr.get("energy_unused_on_end_turn_total")),
        "draw_played_delta": int(bool(left_attr.get("draw_played"))) - int(bool(right_attr.get("draw_played"))),
        "scaling_played_delta": int(bool(left_attr.get("scaling_played"))) - int(bool(right_attr.get("scaling_played"))),
        "exhaust_played_delta": int(bool(left_attr.get("exhaust_played"))) - int(bool(right_attr.get("exhaust_played"))),
    }


def played_card_ids(attr: dict[str, Any], field: str) -> list[str]:
    return [str(record.get("card_id") or "") for record in attr.get(field) or [] if record.get("card_id")]


def resource_window_reason_notes(reason_codes: list[str]) -> list[str]:
    text = {
        "opened_resource_window_without_clear_progress": "Resource window opened but did not create a clear progress edge",
        "low_damage_conversion": "Extra resource did not convert into meaningful monster HP progress",
        "no_combat_win_edge": "Candidate did not win more combats than the control line",
        "control_wins_combat_candidate_does_not": "Control branch wins a combat that the resource-window branch does not",
        "unused_energy_after_window": "Resource line still ended turns with unused energy",
        "draw_without_observed_payoff_card": "Draw was played, but no setup/exhaust payoff was observed",
        "survival_gain_without_progress": "Line preserved HP but did not improve combat progress",
        "resource_line_costs_more_hp": "Resource line paid extra HP without enough compensation",
        "enemy_pressure_overwhelms_window": "Visible incoming pressure stayed high during the window",
        "long_fight_no_close": "Continuation saw a long fight without closing the combat",
        "payoff_with_resource_waste": "Resource window produced progress, but with resource waste",
        "resource_not_played_or_not_observed": "No resource-window execution was observed",
    }
    return [text[code] for code in reason_codes if code in text]


def resource_window_reason_breakdown(
    candidate: dict[str, Any] | None,
    control: dict[str, Any] | None,
    diff: dict[str, Any],
    verdict: str,
) -> dict[str, Any]:
    if not candidate or not control:
        return {"reason_codes": ["missing_branch"], "reason_details": {}}
    cand_attr = candidate.get("attribution") or {}
    ctrl_attr = control.get("attribution") or {}
    cand_delta = candidate.get("outcome_delta") or {}
    ctrl_delta = control.get("outcome_delta") or {}
    cand_wins = num(cand_delta.get("combat_win_delta"))
    ctrl_wins = num(ctrl_delta.get("combat_win_delta"))
    cand_monster_hp = num(cand_attr.get("monster_hp_reduction_observed"))
    ctrl_monster_hp = num(ctrl_attr.get("monster_hp_reduction_observed"))
    cand_energy_unused = num(cand_attr.get("energy_unused_on_end_turn_total"))
    ctrl_energy_unused = num(ctrl_attr.get("energy_unused_on_end_turn_total"))
    reason_codes: list[str] = []
    if verdict == "window_opened_without_payoff":
        reason_codes.append("opened_resource_window_without_clear_progress")
        if num(diff.get("monster_hp")) < 20:
            reason_codes.append("low_damage_conversion")
        if num(diff.get("combat_wins")) <= 0 and cand_wins <= ctrl_wins:
            reason_codes.append("no_combat_win_edge")
        if cand_wins < ctrl_wins:
            reason_codes.append("control_wins_combat_candidate_does_not")
        if num(diff.get("energy_unused")) > 2 or cand_energy_unused >= 3:
            reason_codes.append("unused_energy_after_window")
        if (
            bool(cand_attr.get("draw_played"))
            and not bool(cand_attr.get("scaling_played"))
            and not bool(cand_attr.get("exhaust_played"))
            and num(diff.get("monster_hp")) < 40
        ):
            reason_codes.append("draw_without_observed_payoff_card")
        if num(diff.get("end_hp")) > 0 and num(diff.get("combat_wins")) <= 0 and num(diff.get("monster_hp")) < 40:
            reason_codes.append("survival_gain_without_progress")
        if num(diff.get("hp_loss")) > 5:
            reason_codes.append("resource_line_costs_more_hp")
        if num(cand_attr.get("max_visible_unblocked_damage")) >= 20:
            reason_codes.append("enemy_pressure_overwhelms_window")
        if num(cand_attr.get("combat_turns_observed")) >= 6 and cand_wins <= ctrl_wins:
            reason_codes.append("long_fight_no_close")
    elif verdict == "partial_cashout_with_waste":
        reason_codes.append("payoff_with_resource_waste")
        if num(diff.get("energy_unused")) > 2 or cand_energy_unused >= 3:
            reason_codes.append("unused_energy_after_window")
    elif verdict == "not_realized":
        reason_codes.append("resource_not_played_or_not_observed")

    return {
        "reason_codes": reason_codes,
        "reason_details": {
            "candidate_combat_win_delta": cand_wins,
            "control_combat_win_delta": ctrl_wins,
            "candidate_monster_hp_reduction": cand_monster_hp,
            "control_monster_hp_reduction": ctrl_monster_hp,
            "candidate_energy_unused": cand_energy_unused,
            "control_energy_unused": ctrl_energy_unused,
            "candidate_draw_cards": played_card_ids(cand_attr, "draw_cards_played"),
            "candidate_setup_cards": played_card_ids(cand_attr, "setup_or_scaling_cards_played"),
            "candidate_exhaust_cards": played_card_ids(cand_attr, "exhaust_cards_played"),
            "candidate_max_unblocked": num(cand_attr.get("max_visible_unblocked_damage")),
            "candidate_combat_turns": num(cand_attr.get("combat_turns_observed")),
        },
    }


def family_verdict(family: str, candidate: dict[str, Any] | None, control: dict[str, Any] | None) -> dict[str, Any]:
    diff = diff_metrics(candidate, control)
    if diff.get("missing_branch"):
        return {"verdict": "missing_branch", "diff": diff, "notes": ["candidate/control branch missing"]}
    notes = []
    if family == "aoe_kill_timing":
        progress = num(diff.get("kills")) >= 1 or num(diff.get("monster_hp")) >= 80 or num(diff.get("combat_wins")) >= 1
        hp_ok = num(diff.get("hp_loss")) <= 15
        if progress and hp_ok:
            verdict = "cashout_realized"
        elif progress:
            verdict = "progress_with_hp_cost"
            notes.append("AoE/progress improved, but HP exposure increased")
        else:
            verdict = "not_realized"
            notes.append("No clear kill-timing or monster-HP progress edge")
    elif family == "resource_window":
        used_window = num(diff.get("draw_played_delta")) > 0 or num(diff.get("plays")) >= 3
        progress = num(diff.get("monster_hp")) >= 40 or num(diff.get("combat_wins")) >= 1 or num(diff.get("floor")) >= 1
        energy_waste = num(diff.get("energy_unused")) > 2
        if used_window and progress and not energy_waste:
            verdict = "cashout_realized"
        elif used_window and progress:
            verdict = "partial_cashout_with_waste"
            notes.append("Resource window produced progress, but unused energy/action waste increased")
        elif used_window:
            verdict = "window_opened_without_payoff"
            notes.append("Draw/resource signal appeared without clear progress")
        else:
            verdict = "not_realized"
            notes.append("No clear draw/resource execution signal")
        reason_breakdown = resource_window_reason_breakdown(candidate, control, diff, verdict)
        notes.extend(resource_window_reason_notes(reason_breakdown["reason_codes"]))
        return {
            "verdict": verdict,
            "diff": diff,
            "notes": notes,
            "reason_codes": reason_breakdown["reason_codes"],
            "reason_details": reason_breakdown["reason_details"],
        }
    else:
        if num(diff.get("score")) > 25:
            verdict = "cashout_realized"
        elif num(diff.get("score")) < -25:
            verdict = "not_realized"
        else:
            verdict = "inconclusive"
    return {"verdict": verdict, "diff": diff, "notes": notes}


def summarize_probe_context(
    *,
    row: dict[str, Any],
    family: str,
    continuation_policy: str,
    horizon: int,
    report: dict[str, Any],
) -> dict[str, Any]:
    by_key = outcome_by_key(report)
    candidate_key = str((row.get("candidate_under_test") or {}).get("action_key") or "")
    control_key = str((row.get("control_candidate") or {}).get("action_key") or "")
    candidate = by_key.get(candidate_key)
    control = by_key.get(control_key)
    ranked = sorted(report.get("outcomes") or [], key=branch_score, reverse=True)
    rank_by_key = {str(outcome.get("candidate_key") or ""): index + 1 for index, outcome in enumerate(ranked)}
    verdict = family_verdict(family, candidate, control)
    return {
        "context": f"{continuation_policy}@{horizon}",
        "continuation_policy": continuation_policy,
        "horizon": horizon,
        "family": family,
        "candidate_key": candidate_key,
        "control_key": control_key,
        "candidate_rank": rank_by_key.get(candidate_key),
        "control_rank": rank_by_key.get(control_key),
        "best_branch": compact_outcome(ranked[0] if ranked else None),
        "candidate_outcome": compact_outcome(candidate),
        "control_outcome": compact_outcome(control),
        "family_verdict": verdict,
    }


def aggregate_case_verdict(contexts: list[dict[str, Any]]) -> dict[str, Any]:
    verdict_counts = Counter(str((row.get("family_verdict") or {}).get("verdict") or "unknown") for row in contexts)
    reason_counts = Counter(
        str(code)
        for row in contexts
        for code in ((row.get("family_verdict") or {}).get("reason_codes") or [])
    )
    realized = verdict_counts.get("cashout_realized", 0)
    partial = verdict_counts.get("partial_cashout_with_waste", 0) + verdict_counts.get("progress_with_hp_cost", 0)
    failed = verdict_counts.get("not_realized", 0) + verdict_counts.get("window_opened_without_payoff", 0)
    if realized > 0 and failed > 0:
        label = "context_sensitive"
    elif realized >= 2 and partial == 0 and failed == 0:
        label = "consistently_realized"
    elif realized > 0:
        label = "context_sensitive"
    elif partial > 0 and realized == 0:
        label = "partial_or_costly"
    elif failed > 0 and realized == 0:
        label = "not_realized"
    else:
        label = "inconclusive"
    return {
        "case_verdict": label,
        "family_verdict_counts": dict(sorted(verdict_counts.items())),
        "family_reason_counts": dict(sorted(reason_counts.items())),
        "contract": "micro-probe verdict is diagnostic and trace-prefix conditional, not a card label",
    }


def run_case(args: argparse.Namespace, row: dict[str, Any], case_dir: Path) -> dict[str, Any]:
    families = [family for family in row.get("probe_families") or [] if family in {"aoe_kill_timing", "resource_window"}]
    contexts = []
    failures = []
    for family in families:
        for policy in parse_csv(args.continuation_policies):
            for horizon in parse_int_csv(args.horizons):
                report_name = "__".join(
                    [
                        safe_name(str(row.get("case_id") or "case")),
                        family,
                        policy,
                        f"h{horizon}",
                    ]
                )
                out_path = case_dir / f"{report_name}.json"
                result = run_counterfactual_report(
                    args=args,
                    row=row,
                    family=family,
                    continuation_policy=policy,
                    horizon=horizon,
                    out_path=out_path,
                )
                if result["status"] != "ok":
                    failures.append(
                        {
                            "family": family,
                            "continuation_policy": policy,
                            "horizon": horizon,
                            "error": result.get("error"),
                            "report_path": result.get("report_path"),
                        }
                    )
                    continue
                contexts.append(
                    summarize_probe_context(
                        row=row,
                        family=family,
                        continuation_policy=policy,
                        horizon=horizon,
                        report=result["report"],
                    )
                    | {
                        "report_path": result.get("report_path"),
                        "rows_path": result.get("rows_path"),
                    }
                )
    return {
        "case_id": row.get("case_id"),
        "bucket": row.get("bucket"),
        "recommended_next_step": row.get("recommended_next_step"),
        "probe_families": families,
        "source": row.get("source") or {},
        "candidate_under_test": row.get("candidate_under_test") or {},
        "control_candidate": row.get("control_candidate") or {},
        "question": row.get("question"),
        "contexts": contexts,
        "failures": failures,
        "aggregate": aggregate_case_verdict(contexts),
    }


def write_markdown(path: Path, report: dict[str, Any]) -> None:
    summary = report["summary"]
    lines = [
        "# Cashout Micro-Probe Runner V0.1",
        "",
        "This is a trace-prefix short-continuation probe report.",
        "It focuses on AoE kill timing and resource-window cashout; it is not a standalone canonical encounter generator.",
        "",
        "## Summary",
        "",
        f"- queue rows selected: `{summary['selected_case_count']}`",
        f"- cases completed: `{summary['completed_case_count']}`",
        f"- context count: `{summary['context_count']}`",
        f"- case verdicts: `{summary['case_verdict_counts']}`",
        f"- family verdicts: `{summary['family_verdict_counts']}`",
        f"- family reasons: `{summary['family_reason_counts']}`",
        f"- failures: `{summary['failure_count']}`",
        f"- contract: `{summary['contract']}`",
        "",
        "## Cases",
        "",
    ]
    for case in report["cases"]:
        aggregate = case.get("aggregate") or {}
        lines.extend(
            [
                f"### {case.get('case_id')}",
                "",
                "- bucket: `{bucket}`; next step `{step}`; verdict `{verdict}`".format(
                    bucket=case.get("bucket"),
                    step=case.get("recommended_next_step"),
                    verdict=aggregate.get("case_verdict"),
                ),
                "- candidate: `{candidate}`; control `{control}`; question: {question}".format(
                    candidate=(case.get("candidate_under_test") or {}).get("card_id"),
                    control=(case.get("control_candidate") or {}).get("card_id"),
                    question=case.get("question"),
                ),
                f"- family verdict counts: `{aggregate.get('family_verdict_counts')}`",
                f"- family reason counts: `{aggregate.get('family_reason_counts')}`",
                "",
                "| context | family | verdict | cand rank | ctrl rank | score | floor | combats | hp | hp loss | monster hp | kills | reasons | notes |",
                "|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---|",
            ]
        )
        for ctx in case.get("contexts") or []:
            verdict = ctx.get("family_verdict") or {}
            diff = verdict.get("diff") or {}
            lines.append(
                "| {context} | {family} | {verdict} | {cand_rank} | {ctrl_rank} | {score} | {floor} | {combats} | {hp} | {hp_loss} | {monster} | {kills} | `{reasons}` | `{notes}` |".format(
                    context=ctx.get("context"),
                    family=ctx.get("family"),
                    verdict=verdict.get("verdict"),
                    cand_rank=ctx.get("candidate_rank"),
                    ctrl_rank=ctx.get("control_rank"),
                    score=diff.get("score"),
                    floor=diff.get("floor"),
                    combats=diff.get("combat_wins"),
                    hp=diff.get("end_hp"),
                    hp_loss=diff.get("hp_loss"),
                    monster=diff.get("monster_hp"),
                    kills=diff.get("kills"),
                    reasons=verdict.get("reason_codes") or [],
                    notes=verdict.get("notes") or [],
                )
            )
        if case.get("failures"):
            lines.append("")
            lines.append(f"- failures: `{case.get('failures')}`")
        lines.append("")
    real = resolve(path)
    real.parent.mkdir(parents=True, exist_ok=True)
    real.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    out_dir = resolve(args.out_dir)
    case_dir = out_dir / "case_reports"
    case_dir.mkdir(parents=True, exist_ok=True)
    rows = selected_rows(args)
    cases = [run_case(args, row, case_dir) for row in rows]
    case_verdict_counts = Counter(str((case.get("aggregate") or {}).get("case_verdict") or "unknown") for case in cases)
    family_verdict_counts = Counter(
        str((ctx.get("family_verdict") or {}).get("verdict") or "unknown")
        for case in cases
        for ctx in case.get("contexts") or []
    )
    family_reason_counts = Counter(
        str(code)
        for case in cases
        for ctx in case.get("contexts") or []
        for code in ((ctx.get("family_verdict") or {}).get("reason_codes") or [])
    )
    report = {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {
            "queue": str(args.queue),
            "families": parse_csv(args.families),
            "continuation_policies": parse_csv(args.continuation_policies),
            "horizons": parse_int_csv(args.horizons),
            "max_cases": int(args.max_cases),
            "max_branches": int(args.max_branches),
            "allow_replay_mismatch": bool(args.allow_replay_mismatch),
        },
        "summary": {
            "selected_case_count": len(rows),
            "completed_case_count": len(cases),
            "context_count": sum(len(case.get("contexts") or []) for case in cases),
            "failure_count": sum(len(case.get("failures") or []) for case in cases),
            "case_verdict_counts": dict(sorted(case_verdict_counts.items())),
            "family_verdict_counts": dict(sorted(family_verdict_counts.items())),
            "family_reason_counts": dict(sorted(family_reason_counts.items())),
            "contract": "trace-prefix micro-probe; diagnostic only, not a training label",
        },
        "cases": cases,
        "limitations": [
            "V0 reuses full_run_counterfactual_lab and the current trace prefix.",
            "No standalone canonical encounters are generated yet.",
            "Future RNG is fixed-trace replay; results are policy/horizon conditional.",
            "AoE/resource verdict rules are heuristic routing signals, not card truth.",
            "Resource-window reason codes are attribution hints, not causal proof.",
        ],
    }
    write_json(out_dir / "cashout_micro_probe_runner_report.json", report)
    write_jsonl(out_dir / "cashout_micro_probe_runner_cases.jsonl", cases)
    write_markdown(out_dir / "cashout_micro_probe_runner_report.md", report)
    print(
        json.dumps(
            {
                "out": str(out_dir / "cashout_micro_probe_runner_report.json"),
                "markdown_out": str(out_dir / "cashout_micro_probe_runner_report.md"),
                "summary": report["summary"],
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
