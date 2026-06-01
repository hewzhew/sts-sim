#!/usr/bin/env python3
"""Replay DecisionRecord JSONL through full_run_env_driver and verify hashes.

This verifier checks the most basic training-data invariant:

    same env config + same seed + same behavior_action trace
    -> same observations, candidates, rewards, terminal flags, and state hashes.

It intentionally uses only the DecisionEnv contract commands. Teacher labels are
ignored except as payload data on the record; they are not trusted as replay
authority.
"""

from __future__ import annotations

import argparse
import json
import math
import subprocess
import sys
from collections import Counter
from pathlib import Path
from typing import Any, Iterable


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


def iter_jsonl(paths: list[Path]) -> Iterable[tuple[Path, int, dict[str, Any]]]:
    for path in paths:
        with path.open("r", encoding="utf-8") as handle:
            for line_no, line in enumerate(handle, start=1):
                line = line.strip()
                if not line:
                    continue
                yield path, line_no, json.loads(line)


def action_id_value(value: Any) -> int | None:
    if value is None:
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, dict) and "0" in value:
        raw = value["0"]
        return raw if isinstance(raw, int) else None
    if isinstance(value, list) and len(value) == 1 and isinstance(value[0], int):
        return value[0]
    return None


def float_equal(a: Any, b: Any, tolerance: float) -> bool:
    try:
        left = float(a)
        right = float(b)
    except (TypeError, ValueError):
        return False
    if math.isnan(left) or math.isnan(right):
        return math.isnan(left) and math.isnan(right)
    return abs(left - right) <= tolerance


def candidate_signature(candidates: list[dict[str, Any]]) -> list[tuple[int | None, str, str]]:
    out: list[tuple[int | None, str, str]] = []
    for candidate in candidates:
        out.append(
            (
                action_id_value(candidate.get("id")),
                str(candidate.get("action_key")),
                str(candidate.get("action_kind")),
            )
        )
    return out


def reset_driver(client: DriverClient, args: argparse.Namespace, seed: int) -> None:
    client.request(
        {
            "cmd": "reset",
            "seed": seed,
            "ascension": args.ascension,
            "final_act": args.final_act,
            "class": args.player_class,
            "max_steps": args.max_steps,
        }
    )


def add_failure(
    report: dict[str, Any],
    *,
    path: Path,
    line_no: int,
    record: dict[str, Any],
    kind: str,
    details: dict[str, Any],
    max_examples: int,
) -> None:
    report["mismatch_count"] += 1
    report["mismatch_kind_counts"][kind] += 1
    if len(report["mismatch_examples"]) < max_examples:
        report["mismatch_examples"].append(
            {
                "path": str(path),
                "line": line_no,
                "kind": kind,
                "decision_id": record.get("decision_id"),
                "seed": record.get("seed"),
                "details": details,
            }
        )


def verify_record(
    client: DriverClient,
    args: argparse.Namespace,
    report: dict[str, Any],
    *,
    path: Path,
    line_no: int,
    record: dict[str, Any],
) -> bool:
    current = client.request({"cmd": "decision_env_observation"})["payload"]
    ok = True

    expected_before = record.get("state_hash_before")
    actual_before = current.get("info", {}).get("state_hash")
    if expected_before != actual_before:
        ok = False
        add_failure(
            report,
            path=path,
            line_no=line_no,
            record=record,
            kind="state_hash_before_mismatch",
            details={"expected": expected_before, "actual": actual_before},
            max_examples=args.max_examples,
        )

    expected_decision = record.get("decision_id") or {}
    actual_decision = current.get("decision_id") or {}
    if expected_decision != actual_decision:
        ok = False
        add_failure(
            report,
            path=path,
            line_no=line_no,
            record=record,
            kind="decision_id_mismatch",
            details={"expected": expected_decision, "actual": actual_decision},
            max_examples=args.max_examples,
        )

    expected_candidates = candidate_signature(record.get("candidates") or [])
    actual_candidates = candidate_signature(current.get("candidates") or [])
    if expected_candidates != actual_candidates:
        ok = False
        add_failure(
            report,
            path=path,
            line_no=line_no,
            record=record,
            kind="candidate_list_mismatch",
            details={
                "expected_count": len(expected_candidates),
                "actual_count": len(actual_candidates),
                "expected_head": expected_candidates[:8],
                "actual_head": actual_candidates[:8],
            },
            max_examples=args.max_examples,
        )

    action_id = action_id_value(record.get("behavior_action"))
    if action_id is None:
        ok = False
        add_failure(
            report,
            path=path,
            line_no=line_no,
            record=record,
            kind="missing_behavior_action",
            details={"behavior_action": record.get("behavior_action")},
            max_examples=args.max_examples,
        )
        return ok

    outcome = client.request({"cmd": "decision_env_step", "action_id": action_id})["payload"]

    expected_reward = (record.get("reward_since_prev") or {}).get("scalar_reward")
    actual_reward = (outcome.get("reward") or {}).get("scalar_reward")
    if not float_equal(expected_reward, actual_reward, args.float_tolerance):
        ok = False
        add_failure(
            report,
            path=path,
            line_no=line_no,
            record=record,
            kind="reward_mismatch",
            details={"expected": expected_reward, "actual": actual_reward},
            max_examples=args.max_examples,
        )

    for field in ("terminated", "truncated"):
        expected = bool(record.get(field))
        actual = bool(outcome.get(field))
        if expected != actual:
            ok = False
            add_failure(
                report,
                path=path,
                line_no=line_no,
                record=record,
                kind=f"{field}_mismatch",
                details={"expected": expected, "actual": actual},
                max_examples=args.max_examples,
            )

    expected_after = record.get("state_hash_after")
    actual_after = (outcome.get("info") or {}).get("state_hash")
    if expected_after != actual_after:
        ok = False
        add_failure(
            report,
            path=path,
            line_no=line_no,
            record=record,
            kind="state_hash_after_mismatch",
            details={"expected": expected_after, "actual": actual_after},
            max_examples=args.max_examples,
        )

    return ok


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver", type=Path, default=default_driver_path())
    parser.add_argument("--inputs", nargs="+", type=Path, required=True)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=500)
    parser.add_argument("--player-class", default="ironclad")
    parser.add_argument("--float-tolerance", type=float, default=1e-6)
    parser.add_argument("--max-examples", type=int, default=20)
    parser.add_argument("--fail-on-mismatch", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.driver.exists():
        raise SystemExit(f"driver binary not found: {args.driver}")

    client = DriverClient(args.driver)
    report: dict[str, Any] = {
        "schema_version": "decision_record_replay_verification_v0",
        "inputs": [str(path) for path in args.inputs],
        "driver": str(args.driver),
        "env_config": {
            "ascension": args.ascension,
            "final_act": args.final_act,
            "max_steps": args.max_steps,
            "player_class": args.player_class,
        },
        "record_count": 0,
        "verified_record_count": 0,
        "episode_reset_count": 0,
        "mismatch_count": 0,
        "mismatch_kind_counts": Counter(),
        "mismatch_examples": [],
    }

    current_seed: int | None = None
    try:
        for path, line_no, record in iter_jsonl(args.inputs):
            seed = record.get("seed")
            if not isinstance(seed, int):
                add_failure(
                    report,
                    path=path,
                    line_no=line_no,
                    record=record,
                    kind="missing_seed",
                    details={"seed": seed},
                    max_examples=args.max_examples,
                )
                continue
            if seed != current_seed:
                reset_driver(client, args, seed)
                current_seed = seed
                report["episode_reset_count"] += 1

            report["record_count"] += 1
            if verify_record(client, args, report, path=path, line_no=line_no, record=record):
                report["verified_record_count"] += 1
    finally:
        client.close()

    report["mismatch_kind_counts"] = dict(sorted(report["mismatch_kind_counts"].items()))
    report["verified_rate"] = (
        report["verified_record_count"] / report["record_count"]
        if report["record_count"]
        else 0.0
    )

    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))

    if args.fail_on_mismatch and report["mismatch_count"]:
        raise SystemExit(f"replay verification failed with {report['mismatch_count']} mismatches")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
