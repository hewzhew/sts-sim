#!/usr/bin/env python3
"""Verify Pareto dominance: do dominated candidates ever win?"""
from __future__ import annotations

import json
from collections import defaultdict
from pathlib import Path

# Same dimensions as Rust side
DOMS = [
    ("frontload_delta", True),
    ("block_delta", True),
    ("draw_delta", True),
    ("scaling_delta", True),
    ("aoe_delta", True),
    ("exhaust_delta", True),
    ("kill_window_delta", True),
    ("starter_basic_burden_delta", True),
    ("setup_cashout_risk_delta", False),
]


def delta_dim(delta: dict, dim: str, higher_better: bool) -> int:
    raw = delta.get(dim, 0) or 0
    return raw if higher_better else -raw


def pareto_dominates(a: dict, b: dict) -> bool:
    has_strict = False
    for dim, higher_better in DOMS:
        va = delta_dim(a, dim, higher_better)
        vb = delta_dim(b, dim, higher_better)
        if va < vb:
            return False
        if va > vb:
            has_strict = True
    return has_strict


def main():
    # Load candidate outcomes
    base = Path(r"D:\rust\sts_simulator\tools\artifacts\card_cashout_rollout_labels")
    path = base / "current_trace_corpus_v0_10_50eps_plan_query_guard" / "candidate_outcomes.jsonl"

    if not path.exists():
        print(f"Not found: {path}")
        return

    rows = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                rows.append(json.loads(line))

    print(f"Loaded {len(rows)} outcome rows")

    # Group by case_id
    by_case = defaultdict(list)
    for r in rows:
        by_case[r["case_id"]].append(r)

    # For each case, check dominance vs outcomes
    dominated_wins = 0
    dominated_total = 0
    frontier_wins = 0
    frontier_total = 0
    total_comparable = 0

    for case_id, case_rows in by_case.items():
        # Group candidates by candidate_key (there may be multiple outcomes per key)
        candidates: dict[str, dict] = {}
        for r in case_rows:
            c = r["candidate"]
            key = c.get("candidate_key", "?")
            if key not in candidates:
                # Extract plan_delta from the candidate - may need to find it elsewhere
                candidates[key] = {
                    "key": key,
                    "outcomes": [],
                    "delta": None,  # We'll try to find this
                }
            candidates[key]["outcomes"].append(c)

        # Try to get plan_delta from the raw data
        # The candidate_outcomes.jsonl doesn't include plan_delta directly.
        # We need another source. Let's try the cashout_report or traces.

    print("candidate_outcomes.jsonl doesn't include plan_delta vectors.")
    print("Need to cross-reference with trace data or the cashout lab output.")

    # Alternative: read from existing trace files
    trace_dir = Path(r"D:\rust\sts_simulator\tools\artifacts\card_cashout_lab\current_trace_corpus_v0_10_50eps")
    if not trace_dir.exists():
        print(f"Trace dir not found: {trace_dir}")
        return

    # Look for trace files with candidates
    import glob
    trace_files = list(trace_dir.glob("**/traces/episode_*.json"))
    print(f"Found {len(trace_files)} trace files")

    # Read traces and extract reward_card_choice steps
    dominated_wins = 0
    dominated_total = 0
    frontier_wins = 0
    frontier_total = 0
    cases_analyzed = 0

    for tf in trace_files[:10]:  # Start with 10
        try:
            with open(tf) as f:
                trace = json.load(f)
        except Exception:
            continue

        for step in trace.get("steps", []):
            if step.get("decision_type") != "reward_card_choice":
                continue
            candidates = step.get("action_mask", [])
            if len(candidates) < 2:
                continue

            # Extract deltas
            deltas = {}
            for c in candidates:
                key = c.get("action_key", "?")
                pd = c.get("plan_delta")
                if pd and isinstance(pd, dict):
                    deltas[key] = pd

            if len(deltas) < 2:
                continue

            # Compute dominance
            keys = list(deltas.keys())
            dominated = set()
            for i in range(len(keys)):
                for j in range(len(keys)):
                    if i != j and pareto_dominates(deltas[keys[i]], deltas[keys[j]]):
                        dominated.add(keys[j])

            frontier = [k for k in keys if k not in dominated]

            # Check if chosen action was dominated
            chosen_key = step.get("chosen_action_key", "")
            if chosen_key in dominated:
                dominated_wins += 1
            if chosen_key in deltas:
                dominated_total += 1
            if chosen_key in frontier:
                frontier_wins += 1
            if chosen_key in deltas:
                frontier_total += 1

            cases_analyzed += 1
            if cases_analyzed <= 3:
                print(f"\nCase: {trace.get('summary', {}).get('seed', '?')}")
                print(f"  Candidates: {keys}")
                print(f"  Dominated: {dominated}")
                print(f"  Frontier: {frontier}")
                print(f"  Chosen: {chosen_key}")

    print(f"\n=== Results ({cases_analyzed} cases) ===")
    print(f"Dominated candidates chosen: {dominated_wins}/{dominated_total}")
    print(f"Frontier candidates chosen: {frontier_wins}/{frontier_total}")
    if dominated_total > 0:
        print(f"Dominated win rate: {dominated_wins/dominated_total*100:.1f}%")


if __name__ == "__main__":
    main()
