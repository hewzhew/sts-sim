#!/usr/bin/env python3
"""Deprecated compatibility wrapper over `branch_panel`.

Runtime scheduling semantics now live in Rust. This script only translates the
old gap-panel flags to the Rust panel CLI while callers migrate.
"""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path


def main() -> int:
    args = parse_args()
    root = Path(args.capsule_root)
    root.mkdir(parents=True, exist_ok=True)
    command = branch_panel_command(args)
    print(
        "tools/gap_panel.py is deprecated; delegating to branch_panel",
        flush=True,
    )
    exit_code = run_command(
        command,
        root / "branch_panel.log",
        root / "branch_panel.stderr.log",
    )
    print(f"wrote {root / 'panel_summary.json'}")
    return exit_code


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--seeds", nargs="+", required=True)
    parser.add_argument("--capsule-root", required=True)
    parser.add_argument("--wall-ms", type=int, default=60_000)
    parser.add_argument("--generations", type=int, default=80)
    parser.add_argument("--max-branches", type=int, default=1)
    parser.add_argument("--continue-soft-wall", type=int, default=0)
    parser.add_argument("--fresh", action="store_true")
    parser.add_argument(
        "--no-build",
        action="store_true",
        help="Accepted for compatibility; branch_panel is launched through cargo.",
    )
    parser.add_argument(
        "--branch-tiny",
        default=None,
        help="Ignored compatibility flag; branch_tiny is no longer invoked.",
    )
    return parser.parse_args()


def branch_panel_command(args: argparse.Namespace) -> list[str]:
    mode = "drain" if args.continue_soft_wall > 0 else "smoke"
    max_slices = 1 + max(args.continue_soft_wall, 0)
    command = [
        "cargo",
        "run",
        "--bin",
        "branch_panel",
        "--",
        "panel",
        mode,
        "--seeds",
        *args.seeds,
        "--capsule-root",
        str(Path(args.capsule_root)),
        "--slice-ms",
        str(args.wall_ms),
        "--generations",
        str(args.generations),
        "--max-branches",
        str(args.max_branches),
        "--max-slices",
        str(max_slices),
    ]
    if args.fresh:
        command.append("--fresh")
    return command


def run_command(command: list[str], stdout_path: Path, stderr_path: Path) -> int:
    with stdout_path.open("wb") as out, stderr_path.open("wb") as err:
        completed = subprocess.run(command, stdout=out, stderr=err)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
