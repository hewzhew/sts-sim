#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import random
import time
from collections import Counter
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from full_run_env import FullRunEnvDriver


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Replay a full-run trace to one decision point, branch each legal candidate, "
            "and continue with a fixed policy to produce counterfactual outcomes."
        )
    )
    parser.add_argument("--trace-file", type=Path, required=True)
    parser.add_argument("--step-index", type=int, required=True)
    parser.add_argument("--seed", type=int, help="Override trace summary seed.")
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=5000)
    parser.add_argument("--continuation-policy", default="rule_baseline_v0", choices=["rule_baseline_v0", "random_masked"])
    parser.add_argument("--continuation-steps", type=int, default=40)
    parser.add_argument(
        "--branch-indices",
        default="all",
        help="Comma-separated candidate indices, or 'all'. Applied after --max-branches.",
    )
    parser.add_argument("--max-branches", type=int, default=16)
    parser.add_argument("--driver-binary", type=Path)
    parser.add_argument("--out", type=Path, default=REPO_ROOT / "tools" / "artifacts" / "full_run_counterfactual_lab" / "counterfactual_report.json")
    parser.add_argument("--rows-out", type=Path)
    parser.add_argument("--allow-replay-mismatch", action="store_true")
    return parser.parse_args()


def load_trace(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def trace_step(trace: dict[str, Any], step_index: int) -> dict[str, Any]:
    for step in trace.get("steps") or []:
        if int(step.get("step_index") or 0) == step_index:
            return step
    raise SystemExit(f"trace has no step_index={step_index}")


def action_keys_from_response(response: dict[str, Any]) -> list[str]:
    payload = response.get("payload") or {}
    return [str(candidate.get("action_key") or "") for candidate in payload.get("action_candidates") or []]


def action_keys_from_trace_step(step: dict[str, Any]) -> list[str]:
    return [str(candidate.get("action_key") or "") for candidate in step.get("action_mask") or []]


def observation_from_response(response: dict[str, Any]) -> dict[str, Any]:
    payload = response.get("payload") or {}
    return payload.get("observation") or {}


def info_from_response(response: dict[str, Any]) -> dict[str, Any]:
    return response.get("info") or {}


def replay_to_step(
    *,
    args: argparse.Namespace,
    trace: dict[str, Any],
    target_step_index: int,
) -> tuple[FullRunEnvDriver, dict[str, Any], list[dict[str, Any]]]:
    summary = trace.get("summary") or {}
    seed = int(args.seed if args.seed is not None else summary.get("seed") or 0)
    driver = FullRunEnvDriver(args.driver_binary)
    response = driver.request(
        {
            "cmd": "reset",
            "seed": seed,
            "ascension": int(args.ascension),
            "final_act": bool(args.final_act),
            "class": str(args.player_class),
            "max_steps": int(args.max_steps),
        }
    )
    checks: list[dict[str, Any]] = []
    for step in trace.get("steps") or []:
        step_index = int(step.get("step_index") or 0)
        if step_index >= target_step_index:
            break
        check = compare_response_to_trace_step(response, step)
        checks.append(check)
        if check["status"] != "ok" and not args.allow_replay_mismatch:
            driver.close()
            raise RuntimeError(f"prefix replay mismatch at step {step_index}: {check}")
        response = driver.request({"cmd": "step", "action_index": int(step.get("chosen_action_index") or 0)})
    return driver, response, checks


def compare_response_to_trace_step(response: dict[str, Any], step: dict[str, Any]) -> dict[str, Any]:
    obs = observation_from_response(response)
    response_keys = action_keys_from_response(response)
    trace_keys = action_keys_from_trace_step(step)
    mismatches: list[str] = []
    if int(obs.get("floor") or 0) != int(step.get("floor") or 0):
        mismatches.append("floor")
    if int(obs.get("act") or 0) != int(step.get("act") or 0):
        mismatches.append("act")
    if str(obs.get("decision_type") or "") != str(step.get("decision_type") or ""):
        mismatches.append("decision_type")
    if response_keys != trace_keys:
        mismatches.append("action_keys")
    return {
        "step_index": int(step.get("step_index") or 0),
        "status": "ok" if not mismatches else "mismatch",
        "mismatches": mismatches,
        "response_action_count": len(response_keys),
        "trace_action_count": len(trace_keys),
    }


def parse_branch_indices(text: str, candidate_count: int, max_branches: int) -> list[int]:
    if text.strip().lower() == "all":
        return list(range(min(candidate_count, max_branches)))
    out = []
    for part in text.split(","):
        value = part.strip()
        if not value:
            continue
        index = int(value)
        if index < 0 or index >= candidate_count:
            raise SystemExit(f"branch index {index} out of range for {candidate_count} candidates")
        out.append(index)
    return out[:max_branches]


def run_candidate_branch(
    *,
    args: argparse.Namespace,
    trace: dict[str, Any],
    target_step: dict[str, Any],
    candidate_index: int,
) -> dict[str, Any]:
    driver, target_response, prefix_checks = replay_to_step(
        args=args,
        trace=trace,
        target_step_index=int(target_step.get("step_index") or 0),
    )
    rng = random.Random((int((trace.get("summary") or {}).get("seed") or 0) * 1009) + candidate_index)
    try:
        target_check = compare_response_to_trace_step(target_response, target_step)
        if target_check["status"] != "ok" and not args.allow_replay_mismatch:
            raise RuntimeError(f"target replay mismatch: {target_check}")

        target_payload = target_response.get("payload") or {}
        candidates = target_payload.get("action_candidates") or []
        candidate = candidates[candidate_index]
        start = summarize_state_response(target_response)
        decision_counts: Counter[str] = Counter()
        reward_total = 0.0

        response = driver.request({"cmd": "step", "action_index": candidate_index})
        reward_total += float(response.get("reward") or 0.0)
        immediate = summarize_response(response)
        steps_taken = 1
        if not bool(response.get("done")):
            for _ in range(max(int(args.continuation_steps) - 1, 0)):
                decision_counts[str(observation_from_response(response).get("decision_type") or "unknown")] += 1
                response = step_continuation(driver, response, args.continuation_policy, rng)
                reward_total += float(response.get("reward") or 0.0)
                steps_taken += 1
                if bool(response.get("done")):
                    break

        end = summarize_response(response)
        return {
            "candidate_index": candidate_index,
            "candidate_key": str(candidate.get("action_key") or ""),
            "candidate_action": candidate.get("action") or {},
            "candidate_card": candidate.get("card"),
            "prefix_replay_checks": prefix_checks,
            "target_replay_check": target_check,
            "start": start,
            "immediate_after_branch": immediate,
            "end": end,
            "outcome_delta": outcome_delta(start, end),
            "steps_taken": steps_taken,
            "continuation_policy": args.continuation_policy,
            "continuation_decision_counts": dict(sorted(decision_counts.items())),
            "reward_total": reward_total,
        }
    finally:
        driver.close()


def step_continuation(
    driver: FullRunEnvDriver,
    response: dict[str, Any],
    policy: str,
    rng: random.Random,
) -> dict[str, Any]:
    if policy == "rule_baseline_v0":
        return driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
    if policy == "random_masked":
        candidates = (response.get("payload") or {}).get("action_candidates") or []
        if not candidates:
            return driver.request({"cmd": "step", "action_index": 0})
        return driver.request({"cmd": "step", "action_index": rng.randrange(len(candidates))})
    raise ValueError(f"unsupported continuation policy: {policy}")


def summarize_response(response: dict[str, Any]) -> dict[str, Any]:
    return summarize_state_response(response) | {
        "done": bool(response.get("done")),
        "reward": float(response.get("reward") or 0.0),
        "chosen_action_key": response.get("chosen_action_key"),
    }


def summarize_state_response(response: dict[str, Any]) -> dict[str, Any]:
    payload = response.get("payload") or {}
    summary = summarize_observation(observation_from_response(response), info_from_response(response))
    summary["legal_action_count"] = int(payload.get("legal_action_count") or summary["legal_action_count"])
    return summary


def summarize_observation(obs: dict[str, Any], info: dict[str, Any]) -> dict[str, Any]:
    return {
        "result": info.get("result"),
        "terminal_reason": info.get("terminal_reason"),
        "crash": info.get("crash"),
        "step": int(info.get("step") or 0),
        "act": int(obs.get("act") or info.get("act") or 0),
        "floor": int(obs.get("floor") or info.get("floor") or 0),
        "decision_type": obs.get("decision_type"),
        "engine_state": obs.get("engine_state"),
        "current_hp": int(obs.get("current_hp") or info.get("current_hp") or 0),
        "max_hp": int(obs.get("max_hp") or info.get("max_hp") or 0),
        "gold": int(obs.get("gold") or info.get("gold") or 0),
        "deck_size": int(obs.get("deck_size") or info.get("deck_size") or 0),
        "relic_count": int(obs.get("relic_count") or info.get("relic_count") or 0),
        "combat_win_count": int(info.get("combat_win_count") or 0),
        "legal_action_count": int((info.get("legal_action_count") or 0)),
    }


def outcome_delta(start: dict[str, Any], end: dict[str, Any]) -> dict[str, Any]:
    return {
        "floor_delta": int(end.get("floor") or 0) - int(start.get("floor") or 0),
        "act_delta": int(end.get("act") or 0) - int(start.get("act") or 0),
        "hp_delta": int(end.get("current_hp") or 0) - int(start.get("current_hp") or 0),
        "gold_delta": int(end.get("gold") or 0) - int(start.get("gold") or 0),
        "deck_size_delta": int(end.get("deck_size") or 0) - int(start.get("deck_size") or 0),
        "combat_win_delta": int(end.get("combat_win_count") or 0) - int(start.get("combat_win_count") or 0),
    }


def main() -> None:
    args = parse_args()
    trace = load_trace(args.trace_file)
    target = trace_step(trace, args.step_index)

    driver, target_response, prefix_checks = replay_to_step(args=args, trace=trace, target_step_index=args.step_index)
    try:
        target_check = compare_response_to_trace_step(target_response, target)
        if target_check["status"] != "ok" and not args.allow_replay_mismatch:
            raise RuntimeError(f"target replay mismatch: {target_check}")
        candidates = (target_response.get("payload") or {}).get("action_candidates") or []
        branch_indices = parse_branch_indices(args.branch_indices, len(candidates), args.max_branches)
        target_summary = {
            "trace_step": {
                "step_index": int(target.get("step_index") or 0),
                "decision_type": target.get("decision_type"),
                "floor": target.get("floor"),
                "act": target.get("act"),
                "chosen_action_index": target.get("chosen_action_index"),
                "chosen_action_key": target.get("chosen_action_key"),
            },
            "current_observation": summarize_state_response(target_response),
            "candidate_count": len(candidates),
            "branch_indices": branch_indices,
            "target_replay_check": target_check,
            "prefix_replay_status": Counter(check["status"] for check in prefix_checks),
        }
    finally:
        driver.close()

    start_time = time.perf_counter()
    outcomes = [
        run_candidate_branch(args=args, trace=trace, target_step=target, candidate_index=index)
        for index in branch_indices
    ]
    elapsed = time.perf_counter() - start_time
    report = {
        "schema_version": "full_run_counterfactual_lab_v0",
        "source": {
            "trace_file": str(args.trace_file),
            "trace_observation_schema_version": trace.get("observation_schema_version"),
            "trace_action_schema_version": trace.get("action_schema_version"),
        },
        "config": {
            "seed": int(args.seed if args.seed is not None else (trace.get("summary") or {}).get("seed") or 0),
            "ascension": args.ascension,
            "player_class": args.player_class,
            "final_act": bool(args.final_act),
            "max_steps": args.max_steps,
            "continuation_policy": args.continuation_policy,
            "continuation_steps": args.continuation_steps,
            "max_branches": args.max_branches,
        },
        "target": target_summary,
        "outcomes": outcomes,
        "summary": summarize_outcomes(outcomes, elapsed),
    }
    write_json(args.out, report)
    rows_out = args.rows_out or args.out.with_suffix(".rows.jsonl")
    write_jsonl(rows_out, outcomes)
    print(json.dumps(report["summary"], indent=2, ensure_ascii=False))
    print(f"wrote {args.out}")
    print(f"wrote {rows_out}")


def summarize_outcomes(outcomes: list[dict[str, Any]], elapsed: float) -> dict[str, Any]:
    best_by_floor = max(outcomes, key=lambda row: (row["outcome_delta"]["floor_delta"], row["end"]["current_hp"]), default=None)
    best_by_hp = max(outcomes, key=lambda row: (row["end"]["current_hp"], row["outcome_delta"]["floor_delta"]), default=None)
    return {
        "candidate_count": len(outcomes),
        "elapsed_seconds": elapsed,
        "result_counts": dict(sorted(Counter(row["end"].get("result") or "unknown" for row in outcomes).items())),
        "terminal_reason_counts": dict(sorted(Counter(row["end"].get("terminal_reason") or "unknown" for row in outcomes).items())),
        "best_by_floor": compact_best(best_by_floor),
        "best_by_hp": compact_best(best_by_hp),
    }


def compact_best(row: dict[str, Any] | None) -> dict[str, Any] | None:
    if row is None:
        return None
    return {
        "candidate_index": row["candidate_index"],
        "candidate_key": row["candidate_key"],
        "end_floor": row["end"]["floor"],
        "end_hp": row["end"]["current_hp"],
        "floor_delta": row["outcome_delta"]["floor_delta"],
        "hp_delta": row["outcome_delta"]["hp_delta"],
        "result": row["end"].get("result"),
        "terminal_reason": row["end"].get("terminal_reason"),
    }


if __name__ == "__main__":
    main()
