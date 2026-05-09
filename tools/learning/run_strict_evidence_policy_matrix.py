#!/usr/bin/env python3
"""Run strict-evidence closed-loop A/B configurations in parallel chunks."""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import subprocess
import sys
import time
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from collect_branch_traces import default_driver_path
from run_strict_evidence_policy_ab import aggregate, assert_no_label_leak, safe_int
from strict_evidence_report import markdown_table, summarize_summary


REPO_ROOT = Path(__file__).resolve().parents[2]
RUNNER = Path(__file__).with_name("run_strict_evidence_policy_ab.py")


@dataclass(frozen=True)
class MatrixConfig:
    config_id: str
    min_hp_margin: int
    max_steps: int
    hp_margin_only: bool
    horizon_mode: str
    horizon_decisions: int
    max_candidates: int
    candidate_scope: str
    min_reward_margin: float
    allow_progress_flip: bool
    preset: str = "none"


@dataclass(frozen=True)
class ChunkJob:
    config: MatrixConfig
    seeds: tuple[int, ...]
    chunk_index: int
    out_jsonl: Path
    summary_json: Path
    log_path: Path
    driver: Path
    ascension: int
    final_act: bool
    env_max_steps: int
    behavior_policy: str
    continuation_policy: str
    branch_cache_dir: Path | None
    disable_branch_cache: bool


def chunked(items: list[int], size: int) -> list[tuple[int, ...]]:
    return [tuple(items[index : index + size]) for index in range(0, len(items), size)]


def build_configs(args: argparse.Namespace) -> list[MatrixConfig]:
    configs: list[MatrixConfig] = []
    modes = [True] if args.hp_margin_only else [False]
    if args.both_hp_modes:
        modes = [True, False]
    for margin in args.margins:
        for max_steps in args.max_steps_list:
            for hp_only in modes:
                hp_label = "hponly" if hp_only else "rewardok"
                config_id = f"m{margin}_{hp_label}_steps{max_steps}_h{args.horizon_decisions}_c{args.max_candidates}"
                preset = "conservative_v1" if (
                    margin == 25
                    and hp_only
                    and max_steps == args.max_steps_list[0]
                    and args.horizon_mode == "combat_end_v1"
                    and args.horizon_decisions == 16
                    and args.max_candidates == 12
                    and args.candidate_scope == "controlled_v1"
                    and not args.allow_progress_flip
                ) else "none"
                configs.append(
                    MatrixConfig(
                        config_id=config_id,
                        min_hp_margin=margin,
                        max_steps=max_steps,
                        hp_margin_only=hp_only,
                        horizon_mode=args.horizon_mode,
                        horizon_decisions=args.horizon_decisions,
                        max_candidates=args.max_candidates,
                        candidate_scope=args.candidate_scope,
                        min_reward_margin=args.min_reward_margin,
                        allow_progress_flip=args.allow_progress_flip,
                        preset=preset,
                    )
                )
    return configs


def run_chunk(job: ChunkJob) -> dict[str, Any]:
    cmd = [
        sys.executable,
        str(RUNNER),
        "--driver",
        str(job.driver),
        "--seeds",
        *[str(seed) for seed in job.seeds],
        "--out",
        str(job.out_jsonl),
        "--summary-out",
        str(job.summary_json),
        "--ascension",
        str(job.ascension),
        "--max-steps",
        str(job.config.max_steps),
        "--env-max-steps",
        str(job.env_max_steps),
        "--behavior-policy",
        job.behavior_policy,
        "--continuation-policy",
        job.continuation_policy,
        "--horizon-mode",
        job.config.horizon_mode,
        "--horizon-decisions",
        str(job.config.horizon_decisions),
        "--candidate-scope",
        job.config.candidate_scope,
        "--max-candidates",
        str(job.config.max_candidates),
        "--min-hp-margin",
        str(job.config.min_hp_margin),
        "--min-reward-margin",
        str(job.config.min_reward_margin),
    ]
    if job.config.hp_margin_only:
        cmd.append("--hp-margin-only")
    if job.config.allow_progress_flip:
        cmd.append("--allow-progress-flip")
    if job.final_act:
        cmd.append("--final-act")
    if job.branch_cache_dir is not None and not job.disable_branch_cache:
        cmd.extend(["--branch-cache-dir", str(job.branch_cache_dir)])
    if job.disable_branch_cache:
        cmd.append("--disable-branch-cache")
    started = time.time()
    job.out_jsonl.parent.mkdir(parents=True, exist_ok=True)
    job.log_path.parent.mkdir(parents=True, exist_ok=True)
    proc = subprocess.run(
        cmd,
        cwd=REPO_ROOT,
        text=True,
        encoding="utf-8",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    elapsed = time.time() - started
    job.log_path.write_text(
        "COMMAND:\n"
        + " ".join(cmd)
        + "\n\nSTDOUT:\n"
        + proc.stdout
        + "\n\nSTDERR:\n"
        + proc.stderr,
        encoding="utf-8",
    )
    return {
        "config_id": job.config.config_id,
        "chunk_index": job.chunk_index,
        "seeds": list(job.seeds),
        "summary_json": str(job.summary_json),
        "out_jsonl": str(job.out_jsonl),
        "log_path": str(job.log_path),
        "returncode": proc.returncode,
        "elapsed_seconds": elapsed,
    }


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def merge_config_summaries(
    *,
    config: MatrixConfig,
    chunk_results: list[dict[str, Any]],
    out_dir: Path,
    seeds: list[int],
    behavior_policy: str,
) -> dict[str, Any]:
    episode_results: list[dict[str, Any]] = []
    final_results: Counter[str] = Counter()
    override_reasons: Counter[str] = Counter()
    behavior_reasons: Counter[str] = Counter()
    sampling_excluded_by_reason: Counter[str] = Counter()
    sampling_totals: Counter[str] = Counter()
    cache_totals: Counter[str] = Counter()
    child_summaries: list[str] = []
    for result in chunk_results:
        summary = load_json(Path(result["summary_json"]))
        child_summaries.append(result["summary_json"])
        episode_results.extend(summary.get("episode_results") or [])
        final_results.update(summary.get("final_result_counts") or {})
        override_reasons.update(summary.get("override_reason_counts") or {})
        behavior_reasons.update(summary.get("behavior_reason_counts") or {})
        sampling = summary.get("candidate_sampling_summary") or {}
        for key in (
            "legal_candidate_count_total",
            "sampling_requested_action_count",
            "sampling_included_candidate_count",
            "sampling_excluded_candidate_count",
            "sampling_missing_behavior_action_count",
        ):
            sampling_totals[key] += safe_int(sampling.get(key))
        sampling_excluded_by_reason.update(sampling.get("sampling_excluded_by_reason_counts") or {})
        cache_summary = summary.get("branch_evidence_cache_summary") or {}
        for key in (
            "hit_count",
            "miss_count",
            "write_count",
            "read_error_count",
            "identity_mismatch_count",
        ):
            cache_totals[key] += safe_int(cache_summary.get(key))
    paired_summary = aggregate(episode_results)
    summary = {
        "schema_version": "strict_evidence_policy_ab_matrix_config_summary_v1",
        "matrix_config_id": config.config_id,
        "policy_under_test": "strict_evidence_policy_v0",
        "baseline_policy": behavior_policy,
        "seed_count": len(seeds),
        "seeds": seeds,
        "config": {
            "preset": config.preset,
            "horizon_mode": config.horizon_mode,
            "horizon_decisions": config.horizon_decisions,
            "candidate_scope": config.candidate_scope,
            "max_candidates": config.max_candidates,
            "min_hp_margin": config.min_hp_margin,
            "min_reward_margin": config.min_reward_margin,
            "hp_margin_only": config.hp_margin_only,
            "allow_progress_flip": config.allow_progress_flip,
            "max_steps": config.max_steps,
        },
        "episode_results": episode_results,
        "paired_summary": paired_summary,
        "final_result_counts": dict(final_results),
        "override_reason_counts": dict(override_reasons),
        "behavior_reason_counts": dict(behavior_reasons),
        "candidate_sampling_summary": {
            **dict(sampling_totals),
            "sampling_excluded_by_reason_counts": dict(sampling_excluded_by_reason),
            "traced_over_legal_candidate_ratio": (
                safe_int(sampling_totals.get("sampling_included_candidate_count"))
                / safe_int(sampling_totals.get("legal_candidate_count_total"))
                if sampling_totals.get("legal_candidate_count_total")
                else 0.0
            ),
        },
        "branch_evidence_cache_summary": {
            "schema_version": "branch_evidence_cache_summary_v1",
            **dict(cache_totals),
            "hit_rate": (
                safe_int(cache_totals.get("hit_count"))
                / (
                    safe_int(cache_totals.get("hit_count"))
                    + safe_int(cache_totals.get("miss_count"))
                )
                if safe_int(cache_totals.get("hit_count"))
                + safe_int(cache_totals.get("miss_count"))
                else 0.0
            ),
        },
        "chunk_summaries": child_summaries,
        "label_safety": {
            "trainable_as_action_label": False,
            "winner_or_preference_label_used": False,
            "closed_loop_test_is_utility_diagnostic_not_training_label": True,
        },
    }
    assert_no_label_leak(summary, label=f"matrix config summary {config.config_id}")
    out_path = out_dir / f"{config.config_id}.summary.json"
    out_path.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    return summary


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--driver", type=Path, default=default_driver_path())
    parser.add_argument("--out-dir", type=Path, default=REPO_ROOT / "tools" / "artifacts" / "strict_evidence_matrix")
    parser.add_argument("--seed-start", type=int, default=5001)
    parser.add_argument("--episodes", type=int, default=20)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--seeds", type=int, nargs="*")
    parser.add_argument("--chunk-size", type=int, default=25)
    parser.add_argument("--workers", type=int, default=2)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--env-max-steps", type=int, default=260)
    parser.add_argument("--behavior-policy", default="rule_baseline_v0")
    parser.add_argument("--continuation-policy", default="rule_baseline_v0")
    parser.add_argument("--horizon-mode", default="combat_end_v1")
    parser.add_argument("--horizon-decisions", type=int, default=16)
    parser.add_argument("--candidate-scope", default="controlled_v1")
    parser.add_argument("--max-candidates", type=int, default=12)
    parser.add_argument("--margins", type=int, nargs="+", default=[25])
    parser.add_argument("--max-steps-list", type=int, nargs="+", default=[120])
    parser.add_argument("--min-reward-margin", type=float, default=0.25)
    parser.add_argument("--hp-margin-only", action="store_true", default=True)
    parser.add_argument("--both-hp-modes", action="store_true")
    parser.add_argument("--allow-progress-flip", action="store_true")
    parser.add_argument(
        "--branch-cache-dir",
        type=Path,
        help="Shared persistent branch evidence cache directory. Defaults to OUT_DIR/branch_evidence_cache.",
    )
    parser.add_argument("--disable-branch-cache", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.driver.exists():
        raise SystemExit(f"driver binary not found: {args.driver}")
    seeds = sorted(
        dict.fromkeys(
            args.seeds
            if args.seeds
            else [args.seed_start + index * args.seed_step for index in range(args.episodes)]
        )
    )
    configs = build_configs(args)
    args.out_dir.mkdir(parents=True, exist_ok=True)
    branch_cache_dir = args.branch_cache_dir or (args.out_dir / "branch_evidence_cache")
    jobs: list[ChunkJob] = []
    for config in configs:
        config_dir = args.out_dir / config.config_id
        for chunk_index, seed_chunk in enumerate(chunked(seeds, max(1, args.chunk_size))):
            stem = f"chunk{chunk_index:03d}"
            jobs.append(
                ChunkJob(
                    config=config,
                    seeds=seed_chunk,
                    chunk_index=chunk_index,
                    out_jsonl=config_dir / f"{stem}.jsonl",
                    summary_json=config_dir / f"{stem}.summary.json",
                    log_path=config_dir / f"{stem}.log",
                    driver=args.driver,
                    ascension=args.ascension,
                    final_act=args.final_act,
                    env_max_steps=args.env_max_steps,
                    behavior_policy=args.behavior_policy,
                    continuation_policy=args.continuation_policy,
                    branch_cache_dir=branch_cache_dir,
                    disable_branch_cache=args.disable_branch_cache,
                )
            )
    started = time.time()
    results: list[dict[str, Any]] = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=max(1, args.workers)) as executor:
        future_to_job = {executor.submit(run_chunk, job): job for job in jobs}
        for future in concurrent.futures.as_completed(future_to_job):
            result = future.result()
            results.append(result)
            if result["returncode"] != 0:
                raise SystemExit(
                    f"chunk failed: {result['config_id']} chunk {result['chunk_index']} log={result['log_path']}"
                )
    by_config: dict[str, list[dict[str, Any]]] = {}
    for result in results:
        by_config.setdefault(result["config_id"], []).append(result)
    merged_summaries: list[dict[str, Any]] = []
    for config in configs:
        merged_summaries.append(
            merge_config_summaries(
                config=config,
                chunk_results=sorted(
                    by_config.get(config.config_id, []), key=lambda row: row["chunk_index"]
                ),
                out_dir=args.out_dir,
                seeds=seeds,
                behavior_policy=args.behavior_policy,
            )
        )
    rows = [summarize_summary(summary, label=summary["matrix_config_id"]) for summary in merged_summaries]
    matrix_summary = {
        "schema_version": "strict_evidence_policy_matrix_summary_v1",
        "elapsed_seconds": time.time() - started,
        "seed_count": len(seeds),
        "config_count": len(configs),
        "chunk_count": len(jobs),
        "workers": args.workers,
        "branch_cache_dir": None if args.disable_branch_cache else str(branch_cache_dir),
        "chunk_results": sorted(results, key=lambda row: (row["config_id"], row["chunk_index"])),
        "report_rows": rows,
    }
    (args.out_dir / "matrix_summary.json").write_text(
        json.dumps(matrix_summary, indent=2), encoding="utf-8"
    )
    table = markdown_table(rows)
    (args.out_dir / "matrix_report.md").write_text(table, encoding="utf-8")
    print(table, end="")
    print(f"matrix_summary={args.out_dir / 'matrix_summary.json'}")
    print(f"matrix_report={args.out_dir / 'matrix_report.md'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
