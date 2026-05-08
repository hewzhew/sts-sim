#!/usr/bin/env python3
"""Collect NeutralCompressedPolicyRunner deliberation traces from full_run_env_driver.

This is an evidence/audit collector, not a training script. It follows a
behavior policy through the DecisionEnv, asks the driver for a neutral policy
trace at each decision point, writes those traces as JSONL, and emits aggregate
coverage/compression/fallback metrics.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections import Counter
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


def update_summary(
    summary: dict[str, Any],
    trace_payload: dict[str, Any],
    behavior_action_id: int | None,
) -> None:
    summary["decision_count"] += 1
    supported = bool(trace_payload.get("supported"))
    if not supported:
        summary["unsupported_count"] += 1
        reason = trace_payload.get("reason") or "unknown"
        summary["unsupported_reasons"][reason] += 1
        return

    summary["supported_count"] += 1
    trace = trace_payload.get("trace") or {}
    trace_summary = trace_payload.get("summary") or {}
    decision = trace.get("decision") or {}
    mode = decision.get("mode") or "unknown"
    summary["mode_counts"][mode] += 1
    if trace_summary.get("fallback"):
        summary["fallback_count"] += 1
    else:
        summary["selected_count"] += 1
        selected_action_id = trace_summary.get("selected_action_id")
        if selected_action_id is not None and behavior_action_id is not None:
            if int(selected_action_id) == int(behavior_action_id):
                summary["selected_agrees_with_behavior_count"] += 1
            else:
                summary["selected_disagrees_with_behavior_count"] += 1

    for field in (
        "candidate_count",
        "evidence_count",
        "request_count",
        "group_count",
        "expanded_group_count",
        "unexpanded_group_count",
        "truncated_candidate_count",
        "dead_candidate_count",
    ):
        value = int(trace_summary.get(field) or 0)
        summary[f"total_{field}"] += value
        summary[f"max_{field}"] = max(summary[f"max_{field}"], value)


def collect_episode(
    client: DriverClient,
    *,
    seed: int,
    ascension: int,
    final_act: bool,
    max_steps: int,
    policy: str,
    time_budget_ms: int,
    max_branch_depth: int,
    max_candidates: int,
    out,
    summary: dict[str, Any],
) -> dict[str, Any]:
    client.request(
        {
            "cmd": "reset",
            "seed": seed,
            "ascension": ascension,
            "final_act": final_act,
            "class": "ironclad",
            "max_steps": max_steps,
            "reward_shaping_profile": "baseline",
        }
    )
    done = False
    records = 0
    total_reward = 0.0
    final_info: dict[str, Any] | None = None
    while not done and records < max_steps:
        trace = client.request(
            {
                "cmd": "neutral_policy_trace",
                "time_budget_ms": time_budget_ms,
                "max_branch_depth": max_branch_depth,
                "max_candidates": max_candidates,
            }
        )["payload"]
        preview = client.request(
            {
                "cmd": "preview_policy_action",
                "policy": policy,
                "include_state": False,
                "include_next_state": False,
                "check_live_env_unchanged": False,
            }
        )["payload"]
        action_id = preview.get("chosen_action_index")
        trace_record = {
            "schema_version": "neutral_policy_trace_record_v0",
            "seed": seed,
            "episode_step": records,
            "behavior_policy": policy,
            "behavior_action_id": action_id,
            "behavior_action_key": preview.get("chosen_action_key"),
            "trace": trace,
        }
        out.write(json.dumps(trace_record, separators=(",", ":")) + "\n")
        update_summary(summary, trace, action_id)
        records += 1

        if action_id is None:
            break
        step = client.request({"cmd": "decision_env_step", "action_id": action_id})
        total_reward += float(step.get("reward") or 0.0)
        done = bool(step.get("done"))
        final_info = step.get("info")
    return {
        "seed": seed,
        "records": records,
        "total_reward": total_reward,
        "done": done,
        "final_info": final_info,
    }


def finalize_summary(summary: dict[str, Any]) -> dict[str, Any]:
    supported = max(int(summary["supported_count"]), 1)
    for field in (
        "candidate_count",
        "evidence_count",
        "request_count",
        "group_count",
        "expanded_group_count",
        "unexpanded_group_count",
        "truncated_candidate_count",
        "dead_candidate_count",
    ):
        summary[f"avg_{field}"] = summary[f"total_{field}"] / supported
    summary["fallback_rate_supported"] = summary["fallback_count"] / supported
    summary["selected_rate_supported"] = summary["selected_count"] / supported
    selected = max(int(summary["selected_count"]), 1)
    summary["selected_agreement_rate_with_behavior"] = (
        summary["selected_agrees_with_behavior_count"] / selected
    )
    summary["selected_disagreement_rate_with_behavior"] = (
        summary["selected_disagrees_with_behavior_count"] / selected
    )
    return summary


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver", type=Path, default=default_driver_path())
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--summary-out", type=Path)
    parser.add_argument("--seed-start", type=int, default=1)
    parser.add_argument("--episodes", type=int, default=1)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=500)
    parser.add_argument("--policy", default="rule_baseline_v0")
    parser.add_argument("--time-budget-ms", type=int, default=25)
    parser.add_argument("--max-branch-depth", type=int, default=1)
    parser.add_argument("--max-candidates", type=int, default=64)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.driver.exists():
        raise SystemExit(f"driver binary not found: {args.driver}")
    args.out.parent.mkdir(parents=True, exist_ok=True)
    summary_out = args.summary_out or args.out.with_suffix(".summary.json")
    summary: dict[str, Any] = {
        "schema_version": "neutral_policy_trace_collection_summary_v0",
        "driver": str(args.driver),
        "out": str(args.out),
        "policy": args.policy,
        "decision_count": 0,
        "supported_count": 0,
        "unsupported_count": 0,
        "fallback_count": 0,
        "selected_count": 0,
        "selected_agrees_with_behavior_count": 0,
        "selected_disagrees_with_behavior_count": 0,
        "unsupported_reasons": Counter(),
        "mode_counts": Counter(),
        "episodes": [],
    }
    for field in (
        "candidate_count",
        "evidence_count",
        "request_count",
        "group_count",
        "expanded_group_count",
        "unexpanded_group_count",
        "truncated_candidate_count",
        "dead_candidate_count",
    ):
        summary[f"total_{field}"] = 0
        summary[f"max_{field}"] = 0

    client = DriverClient(args.driver)
    try:
        with args.out.open("w", encoding="utf-8") as out:
            for episode in range(args.episodes):
                seed = args.seed_start + episode * args.seed_step
                summary["episodes"].append(
                    collect_episode(
                        client,
                        seed=seed,
                        ascension=args.ascension,
                        final_act=args.final_act,
                        max_steps=args.max_steps,
                        policy=args.policy,
                        time_budget_ms=args.time_budget_ms,
                        max_branch_depth=args.max_branch_depth,
                        max_candidates=args.max_candidates,
                        out=out,
                        summary=summary,
                    )
                )
    finally:
        client.close()

    summary["unsupported_reasons"] = dict(summary["unsupported_reasons"])
    summary["mode_counts"] = dict(summary["mode_counts"])
    summary = finalize_summary(summary)
    summary_out.parent.mkdir(parents=True, exist_ok=True)
    summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
