#!/usr/bin/env python3
"""Run combat_case_review over combat cases and summarize success-feedback reruns."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path


EXCLUDED_CASE_NAME_PARTS = (".review", ".classified", ".line")


def main() -> int:
    args = parse_args()
    output_root = Path(args.output_root)
    if args.clean and output_root.exists():
        shutil.rmtree(output_root)
    output_root.mkdir(parents=True, exist_ok=True)
    review_bin = Path(args.combat_case_review)
    cases = select_cases(args)
    rows: list[dict] = []
    had_process_failure = False
    feedback_count = 0

    for case_path in cases:
        case_id = case_path.stem
        review_path = output_root / f"{safe_name(case_id)}.review.json"
        stdout_path = output_root / f"{safe_name(case_id)}.stdout.log"
        stderr_path = output_root / f"{safe_name(case_id)}.stderr.log"
        print(f"review case={case_id}", flush=True)
        exit_code = run_command(
            review_command(review_bin, case_path, review_path, args),
            stdout_path,
            stderr_path,
        )
        if exit_code != 0:
            had_process_failure = True
            row = failed_row(case_path, review_path, exit_code, stderr_path)
        else:
            row = summarize_review(case_path, review_path)
        rows.append(row)
        if row.get("has_success_feedback"):
            feedback_count += 1
        print(render_row(row), flush=True)
        if feedback_count >= args.target_feedback_count:
            break

    payload = {
        "schema": "combat_success_feedback_panel_v0",
        "contract": "thin_experiment_runner_reads_combat_case_review_success_feedback_no_policy_decision",
        "case_root": str(Path(args.case_root)),
        "output_root": str(output_root),
        "reviewed_cases": len(rows),
        "feedback_cases": feedback_count,
        "target_feedback_count": args.target_feedback_count,
        "quality_lane_total_nodes": args.quality_lane_total_nodes,
        "quality_lane_total_ms": args.quality_lane_total_ms,
        "rows": rows,
    }
    write_json(output_root / "panel_summary.json", payload)
    print("\nsummary")
    print_table(rows)
    print(f"\nwrote {output_root / 'panel_summary.json'}")
    return 1 if had_process_failure else 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--case-root", default="target")
    parser.add_argument("--cases", nargs="*", default=[])
    parser.add_argument("--output-root", default="target/success-feedback-panel")
    parser.add_argument("--limit", type=int, default=30)
    parser.add_argument("--target-feedback-count", type=int, default=10)
    parser.add_argument("--quality-lane-total-nodes", type=int, default=800_000)
    parser.add_argument("--quality-lane-total-ms", type=int, default=8_000)
    parser.add_argument("--action-preview-limit", type=int, default=80)
    parser.add_argument("--clean", action="store_true")
    parser.add_argument(
        "--combat-case-review",
        default=str(default_combat_case_review()),
        help="Path to combat_case_review executable; falls back to cargo run if missing.",
    )
    return parser.parse_args()


def default_combat_case_review() -> Path:
    exe = "combat_case_review.exe" if sys.platform.startswith("win") else "combat_case_review"
    return Path("target") / "debug" / exe


def select_cases(args: argparse.Namespace) -> list[Path]:
    explicit = [Path(value) for value in args.cases]
    if explicit:
        return explicit[: args.limit]
    root = Path(args.case_root)
    candidates = sorted(
        path
        for path in root.rglob("combat_cases/*.json")
        if is_raw_combat_case(path)
    )
    return candidates[: args.limit]


def is_raw_combat_case(path: Path) -> bool:
    name = path.name.lower()
    return not any(part in name for part in EXCLUDED_CASE_NAME_PARTS)


def review_command(
    review_bin: Path, case_path: Path, review_path: Path, args: argparse.Namespace
) -> list[str]:
    command = (
        executable_command(review_bin)
        + [
            "--case",
            str(case_path),
            "--ladder",
            "--quality-lanes",
            "--quality-lane-total-nodes",
            str(args.quality_lane_total_nodes),
            "--quality-lane-total-ms",
            str(args.quality_lane_total_ms),
            "--action-preview-limit",
            str(args.action_preview_limit),
            "--compact",
            "--write-review",
            str(review_path),
        ]
    )
    return command


def executable_command(path: Path) -> list[str]:
    if path.exists():
        return [str(path)]
    return ["cargo", "run", "--quiet", "--bin", "combat_case_review", "--"]


def run_command(command: list[str], stdout_path: Path, stderr_path: Path) -> int:
    stdout_path.parent.mkdir(parents=True, exist_ok=True)
    with stdout_path.open("wb") as out, stderr_path.open("wb") as err:
        completed = subprocess.run(command, stdout=out, stderr=err)
    return completed.returncode


def summarize_review(case_path: Path, review_path: Path) -> dict:
    review = read_json(review_path)
    quality = review.get("quality_lanes") or {}
    feedback = quality.get("success_feedback_rerun")
    row = {
        "case_id": case_path.stem,
        "case_path": str(case_path),
        "review_path": str(review_path),
        "has_success_feedback": feedback is not None,
        "selected_lane": quality.get("selected_lane"),
    }
    row.update(case_context(review))
    if feedback is None:
        row["verdict"] = "no_feedback"
        row["reason"] = quality.get("selected_reason") or "no_success_feedback_rerun"
        return row

    comparison = feedback.get("comparison") or {}
    baseline = feedback.get("baseline") or {}
    rerun = feedback.get("rerun") or {}
    row.update(
        {
            "source_lane": feedback.get("source_lane"),
            "prior_states": feedback.get("prior_states"),
            "witness_action_count": feedback.get("witness_action_count"),
            "baseline_first_win": baseline.get("nodes_to_first_win"),
            "rerun_first_win": rerun.get("nodes_to_first_win"),
            "first_win_nodes_delta": comparison.get("first_win_nodes_delta"),
            "terminal_wins_delta": comparison.get("terminal_wins_delta"),
            "final_hp_delta": comparison.get("final_hp_delta"),
            "hp_loss_delta": comparison.get("hp_loss_delta"),
            "potions_used_delta": comparison.get("potions_used_delta"),
            "rerun_found_win": comparison.get("rerun_found_win"),
        }
    )
    row["verdict"] = classify_feedback(comparison)
    row["reason"] = feedback_reason(row)
    return row


def case_context(review: dict) -> dict:
    run = review.get("run") or {}
    combat = review.get("combat") or {}
    enemies = combat.get("enemies") or []
    return {
        "act": run.get("act"),
        "floor": run.get("floor"),
        "player_hp": run.get("hp"),
        "player_max_hp": run.get("max_hp"),
        "deck_size": run.get("deck_size"),
        "gold": run.get("gold"),
        "enemy_count": len(enemies),
        "enemies": enemies,
        "subject": ",".join(enemies) or None,
    }


def classify_feedback(comparison: dict) -> str:
    if not comparison.get("rerun_found_win"):
        return "harmful"
    first_delta = comparison.get("first_win_nodes_delta")
    hp_delta = comparison.get("final_hp_delta")
    hp_loss_delta = comparison.get("hp_loss_delta")
    potion_delta = comparison.get("potions_used_delta")
    quality_drop = (hp_delta is not None and hp_delta < 0) or (
        hp_loss_delta is not None and hp_loss_delta > 0
    ) or (potion_delta is not None and potion_delta > 0)
    if first_delta is not None and first_delta < 0:
        return "suspicious" if quality_drop else "positive"
    if first_delta is not None and first_delta > 0:
        return "harmful" if quality_drop else "neutral"
    return "neutral"


def feedback_reason(row: dict) -> str:
    verdict = row.get("verdict")
    if verdict == "positive":
        return "faster_first_win_without_quality_drop"
    if verdict == "suspicious":
        return "faster_first_win_but_quality_or_resource_worse"
    if verdict == "harmful":
        return "rerun_failed_or_worse_first_win"
    if verdict == "neutral":
        return "no_clear_first_win_improvement"
    return "no_success_feedback"


def failed_row(
    case_path: Path, review_path: Path, exit_code: int, stderr_path: Path
) -> dict:
    return {
        "case_id": case_path.stem,
        "case_path": str(case_path),
        "review_path": str(review_path),
        "has_success_feedback": False,
        "verdict": "process_failed",
        "reason": "combat_case_review_exit_nonzero",
        "process_exit": exit_code,
        "stderr_tail": tail_text(stderr_path),
    }


def render_row(row: dict) -> str:
    floor = "-"
    if row.get("act") is not None and row.get("floor") is not None:
        floor = f"A{row['act']}F{row['floor']}"
    delta = row.get("first_win_nodes_delta")
    delta_text = "-" if delta is None else str(delta)
    return (
        f"{row.get('case_id')} {row.get('verdict')} "
        f"{floor} delta={delta_text} subject={row.get('subject') or '-'}"
    )


def print_table(rows: list[dict]) -> None:
    print("verdict      delta   floor  subject")
    for row in rows:
        floor = (
            f"A{row['act']}F{row['floor']}"
            if row.get("act") is not None and row.get("floor") is not None
            else "-"
        )
        delta = row.get("first_win_nodes_delta")
        print(
            f"{row.get('verdict'):<12} "
            f"{str(delta if delta is not None else '-'):<7} "
            f"{floor:<6} "
            f"{row.get('subject') or row.get('case_id')}"
        )


def tail_text(path: Path, limit: int = 1200) -> str:
    if not path.exists():
        return ""
    return path.read_text(encoding="utf-8", errors="replace")[-limit:]


def safe_name(value: str) -> str:
    return "".join(ch if ch.isalnum() or ch in ("-", "_") else "_" for ch in value)


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, value: dict) -> None:
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding="utf-8")


if __name__ == "__main__":
    raise SystemExit(main())
