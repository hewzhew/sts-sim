#!/usr/bin/env python
"""Run live A/B comparisons for CombatRootActionPriorHintV0 ordering hints.

This is intentionally a driver wrapper, not a trainer.  It answers a narrow
question: when the Rust search consumes root action prior hints, do the hints
actually hit states, and do search-level outcomes move under the same budget?
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any


def summarize_driver_report_pair(
    benchmark_name: str, baseline: dict[str, Any], prior: dict[str, Any]
) -> dict[str, Any]:
    baseline_cases = baseline.get("cases") or []
    prior_cases = prior.get("cases") or []
    pairs = list(zip(baseline_cases, prior_cases))

    prior_scored_states = 0
    prior_scored_actions = 0
    baseline_complete = 0
    prior_complete = 0
    baseline_nodes = 0
    prior_nodes = 0
    baseline_frontier_hp = 0
    prior_frontier_hp = 0
    frontier_terminal_changed = 0
    best_complete_first_action_changed = 0
    ordering_first_reorder_sample_changed = 0
    case_deltas: list[dict[str, Any]] = []

    for baseline_case, prior_case in pairs:
        baseline_ordering = (baseline_case.get("diagnostics") or {}).get("ordering") or {}
        prior_ordering = (prior_case.get("diagnostics") or {}).get("ordering") or {}
        prior_scored_states += int(prior_ordering.get("root_action_prior_scored_states") or 0)
        prior_scored_actions += int(prior_ordering.get("root_action_prior_scored_actions") or 0)

        baseline_complete += int(
            bool((baseline_case.get("outcome") or {}).get("complete_trajectory_found"))
        )
        prior_complete += int(
            bool((prior_case.get("outcome") or {}).get("complete_trajectory_found"))
        )
        baseline_nodes += int((baseline_case.get("stats") or {}).get("nodes_expanded") or 0)
        prior_nodes += int((prior_case.get("stats") or {}).get("nodes_expanded") or 0)

        baseline_frontier = baseline_case.get("best_frontier_value") or {}
        prior_frontier = prior_case.get("best_frontier_value") or {}
        baseline_frontier_hp += int(baseline_frontier.get("player_hp") or 0)
        prior_frontier_hp += int(prior_frontier.get("player_hp") or 0)
        if baseline_frontier.get("terminal") != prior_frontier.get("terminal"):
            frontier_terminal_changed += 1
        if first_trajectory_action_key(
            baseline_case.get("best_complete_trajectory")
        ) != first_trajectory_action_key(prior_case.get("best_complete_trajectory")):
            best_complete_first_action_changed += 1
        if first_ordering_reorder_action_key(baseline_ordering) != first_ordering_reorder_action_key(
            prior_ordering
        ):
            ordering_first_reorder_sample_changed += 1
        baseline_best_complete_first_action = first_trajectory_action_key(
            baseline_case.get("best_complete_trajectory")
        )
        prior_best_complete_first_action = first_trajectory_action_key(
            prior_case.get("best_complete_trajectory")
        )
        case_deltas.append(
            {
                "case_id": baseline_case.get("id") or prior_case.get("id"),
                "prior_scored_states": int(
                    prior_ordering.get("root_action_prior_scored_states") or 0
                ),
                "prior_scored_actions": int(
                    prior_ordering.get("root_action_prior_scored_actions") or 0
                ),
                "baseline_complete_found": bool(
                    (baseline_case.get("outcome") or {}).get("complete_trajectory_found")
                ),
                "prior_complete_found": bool(
                    (prior_case.get("outcome") or {}).get("complete_trajectory_found")
                ),
                "nodes_expanded_delta": int((prior_case.get("stats") or {}).get("nodes_expanded") or 0)
                - int((baseline_case.get("stats") or {}).get("nodes_expanded") or 0),
                "frontier_hp_delta": int((prior_case.get("best_frontier_value") or {}).get("player_hp") or 0)
                - int((baseline_case.get("best_frontier_value") or {}).get("player_hp") or 0),
                "baseline_best_complete_first_action": baseline_best_complete_first_action,
                "prior_best_complete_first_action": prior_best_complete_first_action,
                "best_complete_first_action_changed": baseline_best_complete_first_action
                != prior_best_complete_first_action,
                "baseline_ordering_first_reorder_sample": first_ordering_reorder_action_key(
                    baseline_ordering
                ),
                "prior_ordering_first_reorder_sample": first_ordering_reorder_action_key(
                    prior_ordering
                ),
            }
        )

    return {
        "schema_name": "CombatRootPriorLiveCompareBenchmarkV0",
        "benchmark_name": benchmark_name,
        "case_count": len(pairs),
        "baseline_case_count": len(baseline_cases),
        "prior_case_count": len(prior_cases),
        "prior_scored_states": prior_scored_states,
        "prior_scored_actions": prior_scored_actions,
        "baseline_complete_found": baseline_complete,
        "prior_complete_found": prior_complete,
        "complete_found_delta": prior_complete - baseline_complete,
        "baseline_nodes_expanded": baseline_nodes,
        "prior_nodes_expanded": prior_nodes,
        "nodes_expanded_delta": prior_nodes - baseline_nodes,
        "baseline_frontier_hp": baseline_frontier_hp,
        "prior_frontier_hp": prior_frontier_hp,
        "frontier_hp_delta": prior_frontier_hp - baseline_frontier_hp,
        "frontier_terminal_changed": frontier_terminal_changed,
        "best_complete_first_action_changed": best_complete_first_action_changed,
        "ordering_first_reorder_sample_changed": ordering_first_reorder_sample_changed,
        "case_deltas": case_deltas,
    }


def summarize_batch(benchmark_summaries: list[dict[str, Any]]) -> dict[str, Any]:
    totals = {
        "case_count": 0,
        "prior_scored_states": 0,
        "prior_scored_actions": 0,
        "baseline_complete_found": 0,
        "prior_complete_found": 0,
        "complete_found_delta": 0,
        "baseline_nodes_expanded": 0,
        "prior_nodes_expanded": 0,
        "nodes_expanded_delta": 0,
        "baseline_frontier_hp": 0,
        "prior_frontier_hp": 0,
        "frontier_hp_delta": 0,
        "frontier_terminal_changed": 0,
        "best_complete_first_action_changed": 0,
        "ordering_first_reorder_sample_changed": 0,
    }
    for summary in benchmark_summaries:
        for key in totals:
            totals[key] += int(summary.get(key) or 0)
    return {
        "schema_name": "CombatRootPriorLiveCompareSummaryV0",
        "benchmark_count": len(benchmark_summaries),
        **totals,
    }


def live_prior_effect_decision(summary: dict[str, Any]) -> dict[str, Any]:
    """Conservatively decide whether live search should consume this prior.

    The live prior is allowed to affect ordering, so the bar for enabling it in
    campaign/search defaults is higher than "the hints were loaded."  This
    helper intentionally reports readiness from search-level evidence only; it
    does not reinterpret card/game strategy.
    """

    prior_scored_states = int(summary.get("prior_scored_states") or 0)
    prior_scored_actions = int(summary.get("prior_scored_actions") or 0)
    complete_delta = int(summary.get("complete_found_delta") or 0)
    hp_delta = int(summary.get("frontier_hp_delta") or 0)
    nodes_delta = int(summary.get("nodes_expanded_delta") or 0)
    first_action_changes = int(summary.get("best_complete_first_action_changed") or 0)

    evidence: list[str] = []
    limitations: list[str] = []

    if prior_scored_states > 0 or prior_scored_actions > 0:
        evidence.append("prior_hits_observed")
    else:
        limitations.append("no_prior_hits")

    if first_action_changes > 0:
        evidence.append("prior_changed_search_path")

    if complete_delta > 0:
        evidence.append("more_complete_trajectories_found")
    if hp_delta > 0:
        evidence.append("higher_frontier_hp")
    if nodes_delta < 0:
        evidence.append("fewer_nodes_expanded")

    if prior_scored_states > 0 and complete_delta <= 0 and hp_delta <= 0:
        evidence.append("prior_hits_without_outcome_gain")
    if nodes_delta > 0:
        limitations.append("prior_increased_nodes")
    if complete_delta < 0:
        limitations.append("fewer_complete_trajectories_found")
    if hp_delta < 0:
        limitations.append("lower_frontier_hp")
    limitations.append("small_budget_live_ab")

    if prior_scored_states <= 0:
        recommendation = "no_signal_no_prior_hits"
    elif complete_delta < 0 or hp_delta < 0:
        recommendation = "regressed_do_not_enable"
    elif complete_delta > 0 or hp_delta > 0 or (nodes_delta < 0 and first_action_changes > 0):
        recommendation = "candidate_for_larger_live_ab"
    else:
        recommendation = "do_not_enable_live_prior_yet"

    return {
        "schema_name": "CombatRootPriorLiveEffectDecisionV0",
        "recommendation": recommendation,
        "evidence": evidence,
        "limitations": limitations,
        "metrics": {
            "prior_scored_states": prior_scored_states,
            "prior_scored_actions": prior_scored_actions,
            "complete_found_delta": complete_delta,
            "frontier_hp_delta": hp_delta,
            "nodes_expanded_delta": nodes_delta,
            "best_complete_first_action_changed": first_action_changes,
        },
    }


def first_trajectory_action_key(trajectory: Any) -> str | None:
    if not isinstance(trajectory, dict):
        return None
    actions = trajectory.get("actions") or []
    if not actions:
        return None
    first = actions[0]
    if not isinstance(first, dict):
        return None
    value = first.get("action_key")
    return value if isinstance(value, str) else None


def first_ordering_reorder_action_key(ordering: dict[str, Any]) -> str | None:
    samples = ordering.get("largest_reorders") or []
    if not samples:
        return None
    first = samples[0]
    if not isinstance(first, dict):
        return None
    value = first.get("first_action_key")
    return value if isinstance(value, str) else None


def discover_benchmarks(root: Path, pattern: str) -> list[Path]:
    return sorted(path / "benchmark.json" for path in root.glob(pattern) if (path / "benchmark.json").exists())


def run_driver(
    driver: Path,
    benchmark: Path,
    max_nodes: int,
    output: Path,
    prior_hints: Path | None,
    extra_driver_args: list[str],
) -> None:
    args = [
        str(driver),
        "--benchmark-spec",
        str(benchmark),
        "--max-nodes",
        str(max_nodes),
        "--output",
        str(output),
    ] + extra_driver_args
    if prior_hints is not None:
        args.extend(["--root-action-prior-hints", str(prior_hints)])
    subprocess.run(args, check=True)


def build_driver(build_profile: str) -> None:
    subprocess.run(
        ["cargo", "build", "--profile", build_profile, "--bin", "combat_search_v2_driver"],
        check=True,
    )


def read_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True), encoding="utf-8")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--driver", type=Path, default=Path("target/fast-run/combat_search_v2_driver.exe"))
    parser.add_argument("--build", action="store_true")
    parser.add_argument("--build-profile", default="fast-run")
    parser.add_argument("--prior-hints", type=Path, required=True)
    parser.add_argument("--benchmark", type=Path, action="append", default=[])
    parser.add_argument("--benchmark-root", type=Path, default=Path("tools/artifacts/tmp"))
    parser.add_argument("--benchmark-directory-pattern", default="ml_capture_seed*")
    parser.add_argument("--limit", type=int, default=0)
    parser.add_argument("--max-nodes", type=int, default=1000)
    parser.add_argument("--output-root", type=Path, default=Path("tools/artifacts/tmp/root_prior_live_compare"))
    parser.add_argument("--summary-json-out", type=Path, default=None)
    parser.add_argument("--driver-arg", action="append", default=[])
    parser.add_argument("--compact", action="store_true")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    benchmarks = list(args.benchmark)
    if not benchmarks:
        benchmarks = discover_benchmarks(args.benchmark_root, args.benchmark_directory_pattern)
    if args.limit > 0:
        benchmarks = benchmarks[: args.limit]
    if not benchmarks:
        raise SystemExit("no benchmark.json files selected")
    if args.build:
        build_driver(args.build_profile)
    if not args.driver.exists():
        raise SystemExit(f"driver not found: {args.driver}")
    if not args.prior_hints.exists():
        raise SystemExit(f"prior hints not found: {args.prior_hints}")

    benchmark_summaries: list[dict[str, Any]] = []
    for benchmark in benchmarks:
        name = benchmark.parent.name
        bench_dir = args.output_root / name
        baseline_out = bench_dir / "baseline.json"
        prior_out = bench_dir / "prior.json"
        run_driver(args.driver, benchmark, args.max_nodes, baseline_out, None, args.driver_arg)
        run_driver(
            args.driver,
            benchmark,
            args.max_nodes,
            prior_out,
            args.prior_hints,
            args.driver_arg,
        )
        summary = summarize_driver_report_pair(name, read_json(baseline_out), read_json(prior_out))
        benchmark_summaries.append(summary)
        write_json(bench_dir / "summary.json", summary)
        if args.compact:
            print(
                f"{name}: cases={summary['case_count']} "
                f"hits={summary['prior_scored_states']}/{summary['prior_scored_actions']} "
                f"complete_delta={summary['complete_found_delta']} "
                f"hp_delta={summary['frontier_hp_delta']} "
                f"nodes_delta={summary['nodes_expanded_delta']}"
            )

    batch_summary = summarize_batch(benchmark_summaries)
    report = {
        "schema_name": "CombatRootPriorLiveCompareReportV0",
        "driver": str(args.driver),
        "prior_hints": str(args.prior_hints),
        "max_nodes": args.max_nodes,
        "driver_args": args.driver_arg,
        "summary": batch_summary,
        "live_prior_effect_decision": live_prior_effect_decision(batch_summary),
        "benchmarks": benchmark_summaries,
    }
    out_path = args.summary_json_out or (args.output_root / "summary.json")
    write_json(out_path, report)
    if not args.compact:
        print(json.dumps(report["summary"], indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
