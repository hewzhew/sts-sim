#!/usr/bin/env python3
"""A1_AGENT_HARNESS_FINAL evaluator.

This is the non-smoke evaluator target for the agent harness. It runs paired
baseline/target seeds, computes automated gates, and emits a human-audit sample.

It intentionally fails the final gate until human audit annotations are supplied.
"""

from __future__ import annotations

import argparse
import json
import statistics
import subprocess
import sys
import time
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
CONTROLLER = REPO_ROOT / "tools" / "llm" / "llm_full_run_controller.py"
SUMMARIZER = REPO_ROOT / "tools" / "llm" / "summarize_tool_needs.py"


def seed_suite(start: int, count: int) -> list[int]:
    return list(range(start, start + count))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    events = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        event = json.loads(line)
        if isinstance(event, dict):
            events.append(event)
    return events


def median(values: list[float]) -> float:
    return float(statistics.median(values)) if values else 0.0


def percentile(values: list[float], pct: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    index = min(len(ordered) - 1, max(0, int(round((pct / 100.0) * (len(ordered) - 1)))))
    return float(ordered[index])


def run_controller(
    *,
    seed: int,
    role: str,
    provider: str,
    agent_mode: str,
    run_mode: str,
    tool_design_mode: str,
    out_path: Path,
    tool_design_path: Path | None,
    args: argparse.Namespace,
) -> dict[str, Any]:
    command = [
        sys.executable,
        str(CONTROLLER),
        "--provider",
        provider,
        "--run-mode",
        run_mode,
        "--agent-mode",
        agent_mode,
        "--combat-decision-owner",
        "search",
        "--tool-policy",
        "risk_gated",
        "--trace-level",
        "compact",
        "--journal-format",
        "events",
        "--seed",
        str(seed),
        "--steps",
        str(args.decisions),
        "--max-steps",
        str(args.max_steps),
        "--out",
        str(out_path),
        "--timeout",
        str(args.timeout),
        "--planner-timeout",
        str(args.planner_timeout),
    ]
    if provider == "openai_compatible" and args.model:
        command.extend(["--model", args.model])
    if tool_design_mode == "observe":
        command.extend(
            [
                "--tool-design-mode",
                "observe",
                "--tool-design-max-events",
                str(args.tool_design_max_events),
                "--tool-design-max-questions",
                "1",
            ]
        )
        if tool_design_path is not None:
            command.extend(["--tool-design-out", str(tool_design_path)])
    else:
        command.extend(["--tool-design-mode", "off"])
    if args.combat_lab_with_search:
        command.append("--combat-lab-with-search")

    start = time.perf_counter()
    proc = subprocess.run(
        command,
        cwd=REPO_ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=args.process_timeout,
    )
    elapsed_s = time.perf_counter() - start
    return {
        "role": role,
        "run_mode": run_mode,
        "seed": seed,
        "returncode": proc.returncode,
        "elapsed_s": elapsed_s,
        "stdout_tail": proc.stdout[-1600:],
        "stderr_tail": proc.stderr[-1600:],
        "journal_path": str(out_path),
        "tool_design_path": str(tool_design_path) if tool_design_path else None,
    }


def analyze_run(run: dict[str, Any]) -> dict[str, Any]:
    path = Path(run["journal_path"])
    events = read_jsonl(path)
    verifier_events = [event for event in events if event.get("schema_name") == "VerifierDecisionEvent"]
    tool_events = [event for event in events if event.get("schema_name") == "ToolResultEvent"]
    question_events = read_jsonl(Path(run["tool_design_path"])) if run.get("tool_design_path") else []
    max_floor = 0
    final_result = None
    final_hp = None
    final_floor = None
    terminal_reason = None
    for event in verifier_events:
        for key in ["post_floor", "floor"]:
            if isinstance(event.get(key), int):
                max_floor = max(max_floor, int(event[key]))
        if event.get("post_result") is not None:
            final_result = event.get("post_result")
        if event.get("post_hp") is not None:
            final_hp = event.get("post_hp")
        if event.get("post_floor") is not None:
            final_floor = event.get("post_floor")
        if event.get("post_terminal_reason") is not None:
            terminal_reason = event.get("post_terminal_reason")

    illegal = sum(1 for event in verifier_events if event.get("legal") is False)
    combat_final_llm_calls = sum(
        1
        for event in verifier_events
        if event.get("decision_type") == "combat"
        and event.get("executed_action_owner") not in {"search_controller", "routine_policy"}
    )
    return {
        **run,
        "run_mode": run.get("run_mode"),
        "event_count": len(events),
        "journal_bytes": path.stat().st_size if path.exists() else 0,
        "tool_design_questions": len(question_events),
        "max_floor": max_floor,
        "final_floor": final_floor,
        "final_hp": final_hp,
        "final_result": final_result,
        "terminal_reason": terminal_reason,
        "illegal_actions": illegal,
        "harness_crash": run["returncode"] != 0,
        "combat_final_llm_calls": combat_final_llm_calls,
        "tool_counts": dict(Counter(str(event.get("tool") or "unknown") for event in tool_events)),
        "verifier_decisions": len(verifier_events),
        "tool_design_path": run.get("tool_design_path"),
    }


def summarize_tool_needs(question_paths: list[Path], out_path: Path) -> str | None:
    existing = [path for path in question_paths if path.exists() and path.stat().st_size > 0]
    if not existing:
        return None
    subprocess.run(
        [
            sys.executable,
            str(SUMMARIZER),
            *[str(path) for path in existing],
            "--out",
            str(out_path),
        ],
        cwd=REPO_ROOT,
        check=False,
    )
    return str(out_path) if out_path.exists() else None


def build_audit_sample(target_runs: list[dict[str, Any]], out_path: Path, limit: int) -> int:
    rows = []
    priority_decisions = {"campfire", "shop", "map", "reward_card_choice", "boss_reward", "event"}
    for run in target_runs:
        events = read_jsonl(Path(run["journal_path"]))
        by_step: dict[Any, dict[str, list[dict[str, Any]]]] = defaultdict(lambda: defaultdict(list))
        for event in events:
            by_step[event.get("step")][event.get("schema_name")].append(event)
        for step, grouped in by_step.items():
            verifier = (grouped.get("VerifierDecisionEvent") or [None])[-1]
            if not verifier:
                continue
            risk = set(verifier.get("risk_flags") or [])
            decision_type = verifier.get("decision_type")
            high_impact = bool(
                risk
                or decision_type in priority_decisions
                or verifier.get("done")
                or verifier.get("guardrail_override")
            )
            if not high_impact:
                continue
            rows.append(
                {
                    "schema_name": "HumanDecisionAuditSample",
                    "schema_version": 1,
                    "seed": run["seed"],
                    "step": step,
                    "floor": verifier.get("floor"),
                    "decision_type": decision_type,
                    "risk_flags": sorted(risk),
                    "decision_owner": verifier.get("decision_owner"),
                    "executed_action_owner": verifier.get("executed_action_owner"),
                    "final_action_key": verifier.get("final_action_key"),
                    "tool_summaries": [
                        {
                            "tool": item.get("tool"),
                            "summary": item.get("summary"),
                        }
                        for item in grouped.get("ToolResultEvent", [])[:4]
                    ],
                    "annotation": {
                        "reasonable_decision": None,
                        "tool_supported_reasoning": None,
                        "obviously_harmful_decision": None,
                        "unactionable_or_noisy_reasoning": None,
                        "notes": "",
                    },
                }
            )
    rows = rows[:limit]
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(
        "\n".join(json.dumps(row, ensure_ascii=False, separators=(",", ":")) for row in rows)
        + ("\n" if rows else ""),
        encoding="utf-8",
    )
    return len(rows)


def analyze_human_audit(path: Path | None) -> dict[str, Any]:
    if path is None or not path.exists():
        return {
            "status": "pending",
            "passed": False,
            "reason": "no human audit annotation file supplied",
        }
    rows = read_jsonl(path)
    annotated = [
        row
        for row in rows
        if isinstance(row.get("annotation"), dict)
        and row["annotation"].get("reasonable_decision") is not None
    ]
    if not annotated:
        return {"status": "pending", "passed": False, "reason": "no annotated rows"}
    count = len(annotated)

    def rate(field: str) -> float:
        return sum(1 for row in annotated if row["annotation"].get(field) is True) / count

    reasonable = rate("reasonable_decision")
    supported = rate("tool_supported_reasoning")
    harmful = rate("obviously_harmful_decision")
    noisy = rate("unactionable_or_noisy_reasoning")
    passed = (
        count >= 100
        and reasonable >= 0.80
        and supported >= 0.60
        and harmful <= 0.05
        and noisy <= 0.10
    )
    return {
        "status": "complete",
        "passed": passed,
        "sample_size": count,
        "reasonable_decision_rate": reasonable,
        "tool_supported_reasoning_rate": supported,
        "obviously_harmful_decision_rate": harmful,
        "unactionable_or_noisy_reasoning_rate": noisy,
        "thresholds": {
            "sample_size": 100,
            "reasonable_decision_rate": 0.80,
            "tool_supported_reasoning_rate": 0.60,
            "obviously_harmful_decision_rate_max": 0.05,
            "unactionable_or_noisy_reasoning_rate_max": 0.10,
        },
    }


def evaluate(args: argparse.Namespace) -> dict[str, Any]:
    seeds = args.seeds or seed_suite(args.seed_start, args.seed_count)
    out_dir = args.out_dir or (
        REPO_ROOT
        / "tools"
        / "artifacts"
        / "evals"
        / f"a1_final_{args.target_provider}_{int(time.time())}"
    )
    baseline_dir = out_dir / "baseline"
    target_dir = out_dir / "target"
    baseline_dir.mkdir(parents=True, exist_ok=True)
    target_dir.mkdir(parents=True, exist_ok=True)

    baseline_runs = []
    target_runs = []
    for seed in seeds:
        baseline_runs.append(
            analyze_run(
                run_controller(
                    seed=seed,
                    role="baseline_search_combat_routine_noncombat",
                    provider="routine",
                    agent_mode="off",
                    run_mode="search_final_baseline",
                    tool_design_mode="off",
                    out_path=baseline_dir / f"seed_{seed}.jsonl",
                    tool_design_path=None,
                    args=args,
                )
            )
        )
        target_runs.append(
            analyze_run(
                run_controller(
                    seed=seed,
                    role=f"target_{args.target_run_mode}",
                    provider=args.target_provider,
                    agent_mode="planner",
                    run_mode=args.target_run_mode,
                    tool_design_mode="observe",
                    out_path=target_dir / f"seed_{seed}.jsonl",
                    tool_design_path=target_dir / f"seed_{seed}_questions.jsonl",
                    args=args,
                )
            )
        )

    paired = []
    for baseline, target in zip(baseline_runs, target_runs):
        paired.append(
            {
                "seed": target["seed"],
                "baseline_max_floor": baseline["max_floor"],
                "target_max_floor": target["max_floor"],
                "floor_delta": target["max_floor"] - baseline["max_floor"],
                "baseline_final_hp": baseline["final_hp"],
                "target_final_hp": target["final_hp"],
                "hp_delta_at_end": (
                    target["final_hp"] - baseline["final_hp"]
                    if isinstance(target["final_hp"], int) and isinstance(baseline["final_hp"], int)
                    else None
                ),
            }
        )

    target_question_paths = [Path(run["tool_design_path"]) for run in target_runs if run.get("tool_design_path")]
    tool_needs_summary_path = summarize_tool_needs(
        target_question_paths,
        out_dir / "tool_needs_summary.json",
    )
    audit_sample_count = build_audit_sample(
        target_runs,
        out_dir / "human_audit_sample.jsonl",
        args.audit_sample_size,
    )
    human_audit = analyze_human_audit(args.human_audit_in)

    target_journal_sizes = [run["journal_bytes"] for run in target_runs]
    target_question_counts = [run["tool_design_questions"] for run in target_runs]
    wall_times = [run["elapsed_s"] for run in target_runs]
    llm_call_estimates = [
        sum(1 for event in read_jsonl(Path(run["journal_path"])) if event.get("schema_name") in {"PlannerRequestEvent", "RecommendationEvent"})
        for run in target_runs
    ]
    floor_deltas = [item["floor_delta"] for item in paired]
    act1_boss_baseline = sum(1 for run in baseline_runs if run["max_floor"] >= 16) / max(1, len(baseline_runs))
    act1_boss_target = sum(1 for run in target_runs if run["max_floor"] >= 16) / max(1, len(target_runs))
    death_before_10_baseline = sum(
        1 for run in baseline_runs if str(run["final_result"]) == "defeat" and run["max_floor"] < 10
    ) / max(1, len(baseline_runs))
    death_before_10_target = sum(
        1 for run in target_runs if str(run["final_result"]) == "defeat" and run["max_floor"] < 10
    ) / max(1, len(target_runs))
    hp_at_end_deltas = [
        item["hp_delta_at_end"]
        for item in paired
        if isinstance(item.get("hp_delta_at_end"), int)
    ]

    reliability = {
        "passed": all(
            run["returncode"] == 0
            and run["illegal_actions"] == 0
            and not run["harness_crash"]
            and (
                args.target_run_mode.startswith("llm_live")
                or run["combat_final_llm_calls"] == 0
            )
            for run in target_runs
        ),
        "illegal_action_count": sum(run["illegal_actions"] for run in target_runs),
        "harness_crash_count": sum(1 for run in target_runs if run["harness_crash"]),
        "combat_final_llm_calls": sum(run["combat_final_llm_calls"] for run in target_runs),
        "provider_failure_rate": sum(1 for run in target_runs if run["returncode"] != 0) / max(1, len(target_runs)),
        "sampled_replay_pass_rate": None,
        "stale_action_id_count": None,
    }

    outcome_conditions = {
        "paired_median_floor_delta": median([float(value) for value in floor_deltas]),
        "act1_boss_reached_rate_delta": act1_boss_target - act1_boss_baseline,
        "death_before_floor_10_rate_delta": death_before_10_target - death_before_10_baseline,
        "paired_median_hp_at_end_delta": median([float(value) for value in hp_at_end_deltas]),
    }
    outcome_utility = {
        **outcome_conditions,
        "passed": (
            outcome_conditions["paired_median_floor_delta"] >= 0
            and (
                outcome_conditions["act1_boss_reached_rate_delta"] >= 0.10
                or outcome_conditions["death_before_floor_10_rate_delta"] <= -0.15
                or outcome_conditions["paired_median_floor_delta"] >= 1.5
                or outcome_conditions["paired_median_hp_at_end_delta"] >= 5
            )
        ),
    }

    signal_noise = {
        "passed": percentile([float(value) for value in target_journal_sizes], 95) <= 250_000
        and percentile([float(value) for value in target_question_counts], 95) <= 20
        and audit_sample_count > 0,
        "p95_compact_journal_size": percentile([float(value) for value in target_journal_sizes], 95),
        "p95_tool_design_questions": percentile([float(value) for value in target_question_counts], 95),
        "death_attribution_available_rate": None,
        "unsupported_tool_request_refusal_rate": None,
        "audit_sample_count": audit_sample_count,
    }

    cost_latency = {
        "passed": median(wall_times) <= 120
        and percentile(wall_times, 95) <= 240
        and median([float(value) for value in llm_call_estimates]) <= 35,
        "median_wall_time_s": median(wall_times),
        "p95_wall_time_s": percentile(wall_times, 95),
        "median_llm_call_estimate": median([float(value) for value in llm_call_estimates]),
    }

    tool_summary = json.loads(Path(tool_needs_summary_path).read_text(encoding="utf-8")) if tool_needs_summary_path else {}
    clusters = tool_summary.get("clusters") or []
    top_clusters = clusters[:5]
    tool_discovery = {
        "passed": len(top_clusters) >= 3
        and all(int(cluster.get("count") or 0) >= 3 for cluster in top_clusters[: min(3, len(top_clusters))]),
        "top_cluster_count": len(top_clusters),
        "top_clusters": [
            {
                "need_cluster": cluster.get("need_cluster"),
                "count": cluster.get("count"),
                "human_review_status": cluster.get("human_review_status"),
            }
            for cluster in top_clusters
        ],
        "human_accepted_tool_proposals": None,
        "duplicate_noise_cluster_rate": None,
    }

    final_passed = all(
        [
            reliability["passed"],
            outcome_utility["passed"],
            human_audit["passed"],
            signal_noise["passed"],
            cost_latency["passed"],
            tool_discovery["passed"],
        ]
    )
    return {
        "schema_name": "A1AgentHarnessFinalEvaluation",
        "schema_version": 1,
        "metric_name": "A1_AGENT_HARNESS_FINAL",
        "suite_passed": final_passed,
        "target_provider": args.target_provider,
        "seed_count": len(seeds),
        "decisions_per_seed": args.decisions,
        "baseline": "search_combat + routine_noncombat",
        "target": f"planner + tools + {args.target_run_mode}",
        "reliability": reliability,
        "outcome_utility": outcome_utility,
        "human_decision_audit": human_audit,
        "signal_noise": signal_noise,
        "cost_latency": cost_latency,
        "tool_discovery_quality": tool_discovery,
        "paired_results": paired,
        "baseline_runs": baseline_runs,
        "target_runs": target_runs,
        "tool_needs_summary": tool_needs_summary_path,
        "human_audit_sample": str(out_dir / "human_audit_sample.jsonl"),
        "claim_level": "final_harness_usability_and_utility_eval",
        "policy_quality_claim": False,
        "notes": [
            "Final pass requires human audit annotations.",
            "sampled_replay_pass_rate and stale_action_id_count are reserved until replay checker is wired.",
        ],
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target-provider", choices=["mock", "openai_compatible"], default="mock")
    parser.add_argument(
        "--target-run-mode",
        choices=[
            "llm_live_controller",
            "llm_live_with_tactical_safety",
            "search_final_baseline",
            "llm_shadow_audit",
        ],
        default="llm_live_controller",
    )
    parser.add_argument("--seed-start", type=int, default=42)
    parser.add_argument("--seed-count", type=int, default=100)
    parser.add_argument("--seeds", type=int, nargs="*", default=None)
    parser.add_argument("--decisions", type=int, default=350)
    parser.add_argument("--max-steps", type=int, default=1200)
    parser.add_argument("--timeout", type=int, default=120)
    parser.add_argument("--planner-timeout", type=int, default=120)
    parser.add_argument("--process-timeout", type=int, default=600)
    parser.add_argument("--tool-design-max-events", type=int, default=8)
    parser.add_argument("--combat-lab-with-search", action="store_true", default=True)
    parser.add_argument("--audit-sample-size", type=int, default=100)
    parser.add_argument("--human-audit-in", type=Path, default=None)
    parser.add_argument("--model", default=None)
    parser.add_argument("--out-dir", type=Path, default=None)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    result = evaluate(args)
    out_dir = Path(result["human_audit_sample"]).parent
    out_dir.mkdir(parents=True, exist_ok=True)
    summary_path = out_dir / "a1_final_summary.json"
    summary_path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0 if result["suite_passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
