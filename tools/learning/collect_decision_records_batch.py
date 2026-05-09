#!/usr/bin/env python3
"""Parallel shard collector for DecisionRecord JSONL."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
COLLECT_SCRIPT = REPO_ROOT / "tools" / "learning" / "collect_decision_records.py"


def default_driver_path() -> Path:
    suffix = ".exe" if sys.platform.startswith("win") else ""
    release = REPO_ROOT / "target" / "release" / f"full_run_env_driver{suffix}"
    debug = REPO_ROOT / "target" / "debug" / f"full_run_env_driver{suffix}"
    return release if release.exists() else debug


def shard_plan(episodes: int, workers: int) -> list[tuple[int, int]]:
    workers = max(1, min(workers, episodes))
    base = episodes // workers
    extra = episodes % workers
    out: list[tuple[int, int]] = []
    start = 0
    for worker in range(workers):
        count = base + (1 if worker < extra else 0)
        out.append((start, count))
        start += count
    return [(start, count) for start, count in out if count > 0]


def run_shard(args: argparse.Namespace, shard_index: int, offset: int, episodes: int) -> dict[str, Any]:
    shard_dir = args.out_dir / "shards"
    shard_dir.mkdir(parents=True, exist_ok=True)
    records = shard_dir / f"records_shard_{shard_index:04d}.jsonl"
    summary = shard_dir / f"records_shard_{shard_index:04d}.summary.json"
    seed_start = args.seed_start + offset * args.seed_step
    cmd = [
        sys.executable,
        str(COLLECT_SCRIPT),
        "--driver",
        str(args.driver),
        "--out",
        str(records),
        "--summary-out",
        str(summary),
        "--seed-start",
        str(seed_start),
        "--episodes",
        str(episodes),
        "--seed-step",
        str(args.seed_step),
        "--ascension",
        str(args.ascension),
        "--class",
        args.player_class,
        "--max-steps",
        str(args.max_steps),
        "--selector",
        args.selector,
        "--sim-version",
        args.sim_version,
        "--return-spec-version",
        args.return_spec_version,
    ]
    if args.final_act:
        cmd.append("--final-act")
    completed = subprocess.run(cmd, cwd=REPO_ROOT, text=True, capture_output=True)
    if completed.returncode != 0:
        return {
            "shard_index": shard_index,
            "ok": False,
            "cmd": cmd,
            "stdout": completed.stdout,
            "stderr": completed.stderr,
        }
    shard_summary = json.loads(summary.read_text(encoding="utf-8"))
    return {
        "shard_index": shard_index,
        "ok": True,
        "records": str(records),
        "summary": str(summary),
        "seed_start": seed_start,
        "episodes": episodes,
        "total_records": shard_summary.get("total_records"),
        "stdout_tail": completed.stdout[-2000:],
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver", type=Path, default=default_driver_path())
    parser.add_argument("--out-dir", type=Path, required=True)
    parser.add_argument("--episodes", type=int, required=True)
    parser.add_argument("--workers", type=int, default=2)
    parser.add_argument("--seed-start", type=int, default=1)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--max-steps", type=int, default=500)
    parser.add_argument("--selector", default="random_masked", choices=["random_masked", "first_legal"])
    parser.add_argument("--sim-version", default="full_run_env")
    parser.add_argument("--return-spec-version", default="driver_reward_v0")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    args.out_dir.mkdir(parents=True, exist_ok=True)
    futures = []
    with ThreadPoolExecutor(max_workers=max(1, args.workers)) as executor:
        for shard_index, (offset, episodes) in enumerate(shard_plan(args.episodes, args.workers)):
            futures.append(executor.submit(run_shard, args, shard_index, offset, episodes))
    shards = [future.result() for future in as_completed(futures)]
    shards.sort(key=lambda item: item["shard_index"])
    manifest = {
        "schema_version": "decision_record_batch_collection_manifest_v0",
        "driver": str(args.driver),
        "out_dir": str(args.out_dir),
        "class": args.player_class,
        "selector": args.selector,
        "episodes": args.episodes,
        "workers": args.workers,
        "ok": all(shard.get("ok") for shard in shards),
        "total_records": sum(int(shard.get("total_records") or 0) for shard in shards),
        "shards": shards,
    }
    manifest_path = args.out_dir / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    print(json.dumps(manifest, indent=2))
    if not manifest["ok"]:
        raise SystemExit("one or more collection shards failed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
