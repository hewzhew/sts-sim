#!/usr/bin/env python3
"""Collect canonical DecisionRecord JSONL from full_run_env_driver.

This script intentionally talks to the driver through the DecisionEnv contract
commands. It does not parse legacy observation payloads except to ask a behavior
policy for the legal action id to record.
"""

from __future__ import annotations

import argparse
import json
import random
import subprocess
import sys
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]


def default_driver_path() -> Path:
    suffix = ".exe" if sys.platform.startswith("win") else ""
    release = REPO_ROOT / "target" / "release" / f"full_run_env_driver{suffix}"
    debug = REPO_ROOT / "target" / "debug" / f"full_run_env_driver{suffix}"
    return release if release.exists() else debug


class DriverClient:
    def __init__(self, driver_path: Path) -> None:
        self.proc = subprocess.Popen(
            [str(driver_path)],
            cwd=REPO_ROOT,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
        )

    def request(self, payload: dict[str, Any]) -> dict[str, Any]:
        assert self.proc.stdin is not None
        assert self.proc.stdout is not None
        self.proc.stdin.write(json.dumps(payload, separators=(",", ":")) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            stderr = self.proc.stderr.read() if self.proc.stderr else ""
            raise RuntimeError(f"driver closed stdout; stderr={stderr}")
        response = json.loads(line)
        if not response.get("ok"):
            raise RuntimeError(response.get("error") or f"driver request failed: {payload}")
        return response

    def close(self) -> None:
        if self.proc.poll() is None:
            try:
                self.request({"cmd": "close"})
            except Exception:
                pass
        if self.proc.poll() is None:
            self.proc.terminate()


def collect_episode(
    client: DriverClient,
    *,
    seed: int,
    ascension: int,
    final_act: bool,
    player_class: str,
    max_steps: int,
    policy: str,
    sim_version: str,
    return_spec_version: str,
    out,
) -> dict[str, Any]:
    client.request(
        {
            "cmd": "reset",
            "seed": seed,
            "ascension": ascension,
            "final_act": final_act,
            "class": player_class,
            "max_steps": max_steps,
        }
    )
    written = 0
    total_reward = 0.0
    done = False
    last_info: dict[str, Any] | None = None
    rng = random.Random(seed ^ 0x9E3779B97F4A7C15)
    while not done and written < max_steps:
        decision = client.request({"cmd": "decision_env_observation"})["payload"]
        candidates = decision.get("candidates") or []
        if not candidates:
            raise RuntimeError(f"decision env observation did not expose candidates: {decision}")
        if policy == "random_masked":
            candidate = candidates[rng.randrange(len(candidates))]
        elif policy == "first_legal":
            candidate = candidates[0]
        else:
            raise RuntimeError(f"unsupported collection policy '{policy}'")
        action_id = candidate.get("id", candidate.get("action_index"))
        if action_id is None:
            raise RuntimeError(f"candidate did not contain action id: {candidate}")

        request = {
            "cmd": "decision_record_step",
            "action_id": action_id,
            "sim_version": sim_version,
            "return_spec_version": return_spec_version,
                "context": {
                    "collector": "collect_decision_records.py",
                    "behavior_policy": policy,
                    "seed": seed,
            },
        }
        step = client.request(request)
        record = step["payload"]
        out.write(json.dumps(record, separators=(",", ":")) + "\n")
        written += 1
        total_reward += float(step.get("reward") or 0.0)
        done = bool(step.get("done"))
        last_info = step.get("info")
    return {
        "seed": seed,
        "records": written,
        "total_reward": total_reward,
        "done": done,
        "final_info": last_info,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver", type=Path, default=default_driver_path())
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--seed-start", type=int, default=1)
    parser.add_argument("--episodes", type=int, default=1)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--max-steps", type=int, default=500)
    parser.add_argument("--policy", default="random_masked", choices=["random_masked", "first_legal"])
    parser.add_argument("--sim-version", default="full_run_env")
    parser.add_argument("--return-spec-version", default="driver_reward_v0")
    parser.add_argument("--summary-out", type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.driver.exists():
        raise SystemExit(f"driver binary not found: {args.driver}")
    args.out.parent.mkdir(parents=True, exist_ok=True)
    summary_out = args.summary_out or args.out.with_suffix(".summary.json")
    client = DriverClient(args.driver)
    summaries: list[dict[str, Any]] = []
    try:
        with args.out.open("w", encoding="utf-8") as out:
            for episode in range(args.episodes):
                seed = args.seed_start + episode * args.seed_step
                summaries.append(
                    collect_episode(
                        client,
                        seed=seed,
                        ascension=args.ascension,
                        final_act=args.final_act,
                        player_class=args.player_class,
                        max_steps=args.max_steps,
                        policy=args.policy,
                        sim_version=args.sim_version,
                        return_spec_version=args.return_spec_version,
                        out=out,
                    )
                )
    finally:
        client.close()

    summary = {
        "schema_version": "decision_record_collection_summary_v0",
        "out": str(args.out),
        "driver": str(args.driver),
        "policy": args.policy,
        "class": args.player_class,
        "episodes": summaries,
        "total_records": sum(item["records"] for item in summaries),
    }
    summary_out.parent.mkdir(parents=True, exist_ok=True)
    summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
