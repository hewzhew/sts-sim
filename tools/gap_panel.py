#!/usr/bin/env python3
"""Run a small branch_tiny seed panel and collect capsule summaries."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path


def main() -> int:
    args = parse_args()
    root = Path(args.capsule_root)
    root.mkdir(parents=True, exist_ok=True)
    branch_tiny = Path(args.branch_tiny)
    rows = []
    had_process_failure = False
    for seed in parse_seeds(args.seeds):
        capsule = root / str(seed)
        if capsule.exists():
            shutil.rmtree(capsule)
        capsule.mkdir(parents=True, exist_ok=True)
        run_log = capsule / "run.log"
        err_log = capsule / "stderr.log"
        print(f"run seed={seed}", flush=True)
        run_exit = run_command(
            branch_tiny_command(branch_tiny)
            + [
                "--seed",
                str(seed),
                "--run-capsule",
                str(capsule),
                "--generations",
                str(args.generations),
                "--max-branches",
                str(args.max_branches),
                "--wall-ms",
                str(args.wall_ms),
            ],
            run_log,
            err_log,
        )
        if run_exit != 0:
            had_process_failure = True
            row = read_capsule_summary(capsule)
            mark_process_failure(row, "run_failed", run_exit, err_log)
            row["seed"] = seed
            row["capsule_path"] = str(capsule)
            rows.append(row)
            print(render_row(row), flush=True)
            continue
        failed_continue_row = None
        for _ in range(args.continue_soft_wall):
            summary = read_capsule_summary(capsule)
            if summary.get("blocker_kind") != "wall_deadline":
                break
            print(f"continue seed={seed}", flush=True)
            continue_exit = run_command(
                branch_tiny_command(branch_tiny)
                + [
                    "--continue-capsule",
                    str(capsule),
                    "--continue-slices",
                    "1",
                ],
                run_log,
                err_log,
                append=True,
            )
            if continue_exit != 0:
                had_process_failure = True
                row = read_capsule_summary(capsule)
                mark_process_failure(row, "continue_failed", continue_exit, err_log)
                failed_continue_row = row
                break
        row = failed_continue_row or read_capsule_summary(capsule)
        row["seed"] = seed
        row["capsule_path"] = str(capsule)
        rows.append(row)
        print(render_row(row), flush=True)
    payload = {
        "schema": "branch_tiny_gap_panel_summary",
        "capsule_root": str(root),
        "rows": rows,
    }
    write_json(root / "panel_summary.json", payload)
    print("\nsummary")
    print_table(rows)
    print(f"\nwrote {root / 'panel_summary.json'}")
    return 1 if had_process_failure else 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--seeds", nargs="+", required=True)
    parser.add_argument("--capsule-root", required=True)
    parser.add_argument("--wall-ms", type=int, default=60_000)
    parser.add_argument("--generations", type=int, default=80)
    parser.add_argument("--max-branches", type=int, default=1)
    parser.add_argument("--continue-soft-wall", type=int, default=0)
    parser.add_argument(
        "--branch-tiny",
        default=str(default_branch_tiny()),
        help="Path to branch_tiny executable; falls back to cargo run if missing.",
    )
    return parser.parse_args()


def default_branch_tiny() -> Path:
    exe = "branch_tiny.exe" if sys.platform.startswith("win") else "branch_tiny"
    return Path("target") / "debug" / exe


def parse_seeds(values: list[str]) -> list[int]:
    seeds: list[int] = []
    for value in values:
        for part in value.split(","):
            part = part.strip()
            if not part:
                continue
            if ".." in part:
                start, end = part.split("..", 1)
                seeds.extend(range(int(start), int(end) + 1))
            else:
                seeds.append(int(part))
    return seeds


def branch_tiny_command(path: Path) -> list[str]:
    if path.exists():
        return [str(path)]
    return ["cargo", "run", "--quiet", "--bin", "branch_tiny", "--"]


def run_command(
    command: list[str], stdout_path: Path, stderr_path: Path, append: bool = False
) -> int:
    mode = "ab" if append else "wb"
    with stdout_path.open(mode) as out, stderr_path.open(mode) as err:
        if append:
            out.write(b"\n--- continue ---\n")
            err.write(b"\n--- continue ---\n")
        completed = subprocess.run(command, stdout=out, stderr=err)
    return completed.returncode


def read_capsule_summary(capsule: Path) -> dict:
    summary = capsule / "summary.json"
    if summary.exists():
        row = read_json(summary)
        row.setdefault("summary_source", "summary_json")
        return row
    row = fallback_summary(capsule)
    row.setdefault("summary_source", "fallback")
    return row


def fallback_summary(capsule: Path) -> dict:
    manifest = read_json(capsule / "manifest.json") if (capsule / "manifest.json").exists() else {}
    row = {
        "schema": "branch_tiny_gap_panel_fallback_summary",
        "capsule_status": manifest.get("status"),
        "reason": manifest.get("reason"),
        "blocker_kind": manifest.get("reason") or manifest.get("status"),
    }
    if not manifest:
        row["blocker_kind"] = "missing_summary"
    result = capsule / "result.json"
    if result.exists():
        value = read_json(result)
        state = value.get("state") or {}
        row.update(
            {
                "generation": value.get("generation"),
                "status": value.get("status"),
                "blocker_kind": (value.get("status") or {}).get("kind"),
                "act": state.get("act"),
                "floor": state.get("floor"),
                "hp": state.get("hp"),
                "max_hp": state.get("max_hp"),
                "gold": state.get("gold"),
                "deck_size": state.get("deck_size"),
                "combat_case": value.get("combat_case"),
            }
        )
        combat = value.get("combat") or {}
        row["enemies"] = combat.get("enemies")
        row["subject"] = ",".join(combat.get("enemies") or []) or None
    return row


def mark_process_failure(
    row: dict, failure_kind: str, exit_code: int, stderr_path: Path
) -> None:
    row["panel_status"] = failure_kind
    row["process_exit"] = exit_code
    row["stderr_tail"] = tail_text(stderr_path)
    row["blocker_kind"] = failure_kind


def tail_text(path: Path, limit: int = 1200) -> str:
    if not path.exists():
        return ""
    text = path.read_text(encoding="utf-8", errors="replace")
    return text[-limit:]


def render_row(row: dict) -> str:
    hp = "-"
    if row.get("hp") is not None and row.get("max_hp") is not None:
        hp = f"{row['hp']}/{row['max_hp']}"
    floor = "-"
    if row.get("act") is not None and row.get("floor") is not None:
        floor = f"A{row['act']}F{row['floor']}"
    return (
        f"{row.get('seed')} {row.get('blocker_kind') or '-'} "
        f"{floor} hp={hp} deck={row.get('deck_size') or '-'} "
        f"subject={row.get('subject') or '-'}"
    )


def print_table(rows: list[dict]) -> None:
    print("seed        kind          floor  hp       deck  subject")
    for row in rows:
        hp = (
            f"{row['hp']}/{row['max_hp']}"
            if row.get("hp") is not None and row.get("max_hp") is not None
            else "-"
        )
        floor = (
            f"A{row['act']}F{row['floor']}"
            if row.get("act") is not None and row.get("floor") is not None
            else "-"
        )
        print(
            f"{row.get('seed')!s:<11} "
            f"{(row.get('blocker_kind') or '-'):<13} "
            f"{floor:<6} "
            f"{hp:<8} "
            f"{str(row.get('deck_size') or '-'):<5} "
            f"{row.get('subject') or '-'}"
        )


def read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, value: dict) -> None:
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding="utf-8")


if __name__ == "__main__":
    raise SystemExit(main())
