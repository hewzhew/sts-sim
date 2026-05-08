#!/usr/bin/env python3
"""Run verified advantage override fully inside the Rust full-run driver."""
from __future__ import annotations

import argparse
import json
import time
from pathlib import Path
from typing import Any

from return_q_common import FullRunDriver, write_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--episodes", type=int, default=10)
    parser.add_argument("--seed-start", type=int, default=10000)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=500)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--candidate-scope", default="controlled_v1", choices=["all", "controlled_v0", "controlled_v1"])
    parser.add_argument("--horizon-decisions", type=int, default=8)
    parser.add_argument(
        "--horizon-mode",
        default="fixed_decisions",
        choices=["fixed_decisions", "adaptive_next_player_turn_v1", "adaptive_payoff_window_v1", "combat_end_v1"],
    )
    parser.add_argument("--oracle-margin", type=float, default=1.0)
    parser.add_argument(
        "--verifier-strategy",
        default="single_stage",
        choices=["single_stage", "two_stage_prefilter_v1", "model_proposer_v1"],
    )
    parser.add_argument("--prefilter-horizon-decisions", type=int)
    parser.add_argument(
        "--prefilter-horizon-mode",
        choices=["fixed_decisions", "adaptive_next_player_turn_v1", "adaptive_payoff_window_v1", "combat_end_v1"],
    )
    parser.add_argument("--prefilter-margin", type=float)
    parser.add_argument("--prefilter-top-k", type=int)
    parser.add_argument("--proposer-model-path")
    parser.add_argument("--proposer-top-k", type=int)
    parser.add_argument("--proposer-threshold", type=float)
    parser.add_argument(
        "--evidence-gate",
        default="horizon_cap_no_payoff_v1",
        choices=["none", "horizon_cap_no_payoff_v1", "horizon_cap_any_v1"],
    )
    parser.add_argument("--low-evidence-margin", type=float)
    parser.add_argument("--confirm-low-evidence-horizon-decisions", type=int)
    parser.add_argument(
        "--confirm-low-evidence-horizon-mode",
        choices=["fixed_decisions", "adaptive_next_player_turn_v1", "adaptive_payoff_window_v1", "combat_end_v1"],
    )
    parser.add_argument("--confirm-low-evidence-margin", type=float)
    parser.add_argument("--oracle-continuation-policy", default="rule_baseline_v0")
    parser.add_argument("--verified-evaluation-mode", default="independent", choices=["independent", "bellman_cached_v1"])
    parser.add_argument("--verified-value-cache-scope", default="episode", choices=["request", "episode"])
    parser.add_argument("--verified-value-cache-max-entries", type=int, default=4096)
    parser.add_argument("--verified-parallelism", type=int, default=0)
    parser.add_argument("--verified-exact-root-dedup", action=argparse.BooleanOptionalAction, default=False)
    parser.add_argument(
        "--summary-only",
        action="store_true",
        help="Ask the Rust runner to omit per-episode rows and write only aggregate summaries.",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    driver = FullRunDriver(args.binary)
    started = time.perf_counter()
    try:
        response = driver.request(
            {
                "cmd": "run_verified_adv_override_batch",
                "episodes": args.episodes,
                "seed_start": args.seed_start,
                "seed_step": args.seed_step,
                "ascension": args.ascension,
                "final_act": args.final_act,
                "class": args.player_class,
                "max_steps": args.max_steps,
                "reward_shaping_profile": "baseline",
                "candidate_scope": args.candidate_scope,
                "continuation_policy": args.oracle_continuation_policy,
                "horizon_decisions": args.horizon_decisions,
                "horizon_mode": args.horizon_mode,
                "oracle_margin": args.oracle_margin,
                "verifier_strategy": args.verifier_strategy,
                "prefilter_horizon_decisions": args.prefilter_horizon_decisions,
                "prefilter_horizon_mode": args.prefilter_horizon_mode,
                "prefilter_margin": args.prefilter_margin,
                "prefilter_top_k": args.prefilter_top_k,
                "proposer_model_path": args.proposer_model_path,
                "proposer_top_k": args.proposer_top_k,
                "proposer_threshold": args.proposer_threshold,
                "evidence_gate": args.evidence_gate,
                "low_evidence_margin": args.low_evidence_margin,
                "confirm_low_evidence_horizon_decisions": args.confirm_low_evidence_horizon_decisions,
                "confirm_low_evidence_horizon_mode": args.confirm_low_evidence_horizon_mode,
                "confirm_low_evidence_margin": args.confirm_low_evidence_margin,
                "gamma": args.gamma,
                "evaluation_mode": args.verified_evaluation_mode,
                "value_cache_scope": args.verified_value_cache_scope,
                "value_cache_max_entries": args.verified_value_cache_max_entries,
                "parallelism": args.verified_parallelism,
                "exact_root_dedup": args.verified_exact_root_dedup,
                "summary_only": args.summary_only,
            }
        )
    finally:
        driver.close()
    runtime_seconds = time.perf_counter() - started
    payload: dict[str, Any] = response.get("payload") or {}
    payload.setdefault("runtime", {})["python_wall_seconds"] = runtime_seconds
    if args.summary_only:
        payload.pop("episodes", None)
    write_json(args.out, payload)
    print(json.dumps(payload, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
