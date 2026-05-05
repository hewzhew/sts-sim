#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
import time
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def iter_jsonl(path: Path):
    with path.open("r", encoding="utf-8") as handle:
        for line_no, line in enumerate(handle, start=1):
            text = line.strip()
            if text:
                yield line_no, json.loads(text)


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False) + "\n")


@dataclass(frozen=True)
class OracleConfig:
    name: str
    decision_depth: int
    top_k: int
    branch_cap: int
    max_seconds_per_frame: float | None
    max_seconds_per_batch: float | None
    batch_size: int
    quiet: bool


@dataclass(frozen=True)
class AuditInvocation:
    mode: str
    command_prefix: tuple[str, ...]


def state_priority(row: dict[str, Any]) -> tuple[float, float, int]:
    top_gap = abs(float(row.get("top_gap") or 0.0))
    downside = abs(float(row.get("sequence_downside_penalty") or 0.0))
    reasons = row.get("reasons") or []
    bonus = int(bool(row.get("heuristic_search_gap"))) * 3
    bonus += int("sequencing_conflict" in reasons) * 2
    bonus += int("branch_opening_conflict" in reasons) * 2
    bonus += int(row.get("snapshot_normalized_state") is not None)
    return (top_gap + downside / 1000.0, float(row.get("sample_weight") or 0.0), bonus)


def row_is_oracle_candidate(row: dict[str, Any]) -> bool:
    reasons = set(row.get("reasons") or [])
    if row.get("snapshot_normalized_state") is None:
        return False
    return (
        bool(row.get("heuristic_search_gap"))
        or "sequencing_conflict" in reasons
        or "branch_opening_conflict" in reasons
        or bool(row.get("large_sequence_bonus"))
        or row.get("snapshot_trigger_kind") in {"terminal_loss", "high_risk_suspect", "engine_bug"}
    )


def build_run_raw_index(baseline: dict[str, Any]) -> dict[str, str]:
    return {
        str(run["run_id"]): str(run["raw_path"])
        for run in baseline.get("selected_runs") or []
        if run.get("raw_path")
    }


def output_key(row: dict[str, Any]) -> str:
    return f'{row.get("run_id")}::{row.get("frame_count")}'


def default_release_audit_binary() -> Path | None:
    for candidate in (
        REPO_ROOT / "target" / "release" / "combat_decision_audit.exe",
        REPO_ROOT / "target" / "release" / "combat_decision_audit",
    ):
        if candidate.exists():
            return candidate
    return None


def release_binary_supports_batch(binary: Path) -> bool:
    try:
        completed = subprocess.run(
            [str(binary), "--help"],
            check=False,
            capture_output=True,
            text=True,
            timeout=5,
        )
    except (subprocess.SubprocessError, OSError):
        return False
    return completed.returncode == 0 and "audit-frame-batch" in completed.stdout


def resolve_audit_invocation(audit_binary: str | None) -> AuditInvocation:
    if audit_binary:
        binary = Path(audit_binary)
        if release_binary_supports_batch(binary):
            return AuditInvocation("release_binary", (str(binary),))
    release_binary = default_release_audit_binary()
    if release_binary is not None and release_binary_supports_batch(release_binary):
        return AuditInvocation("release_binary", (str(release_binary),))
    return AuditInvocation(
        "cargo_run_fallback",
        ("cargo", "run", "--quiet", "--bin", "combat_decision_audit", "--"),
    )


def cache_key(frame: int, config: OracleConfig, invocation: AuditInvocation) -> str:
    return (
        f"frame_{frame}_{config.name}_{invocation.mode}"
        f"_d{config.decision_depth}_k{config.top_k}_b{config.branch_cap}.json"
    )


def cache_path(cache_dir: Path, frame: int, config: OracleConfig, invocation: AuditInvocation) -> Path:
    return cache_dir / cache_key(frame, config, invocation)


def existing_row_matches(row: dict[str, Any], config: OracleConfig, invocation: AuditInvocation) -> bool:
    budget = row.get("oracle_compute_budget") or {}
    return (
        row.get("run_id") is not None
        and row.get("frame_count") is not None
        and "oracle_equivalent_best_moves" in row
        and "oracle_best_bucket_size" in row
        and budget.get("mode") == config.name
        and budget.get("audit_invocation_mode") == invocation.mode
        and int(budget.get("decision_depth") or -1) == config.decision_depth
        and int(budget.get("top_k") or -1) == config.top_k
        and int(budget.get("branch_cap") or -1) == config.branch_cap
    )


def load_existing_rows(
    path: Path,
    config: OracleConfig,
    invocation: AuditInvocation,
) -> dict[str, dict[str, Any]]:
    if not path.exists():
        return {}
    return {
        output_key(row): row
        for _, row in iter_jsonl(path)
        if existing_row_matches(row, config, invocation)
    }


def canonical_outcome(outcome: str | None) -> str:
    return str(outcome or "").lower()


def outcome_rank(outcome: str | None) -> int:
    return {"lethal_win": 4, "survives": 3, "timeout": 2, "dies": 1}.get(
        canonical_outcome(outcome), 0
    )


def score_candidate(candidate: dict[str, Any]) -> tuple[int, int, int, int, tuple[int, ...]]:
    return (
        outcome_rank(candidate.get("outcome")),
        int(candidate.get("score") or 0),
        int(candidate.get("final_player_hp") or 0),
        -int(candidate.get("final_incoming") or 0),
        tuple(int(v) for v in candidate.get("final_monster_hp") or []),
    )


def report_to_oracle_candidates(report: dict[str, Any]) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for move_report in report.get("first_move_reports") or []:
        trajectories = move_report.get("top_trajectories") or []
        if not trajectories:
            continue
        top = trajectories[0]
        rows.append(
            {
                "move_label": move_report.get("first_move"),
                "outcome": canonical_outcome(top.get("outcome")),
                "score": int(top.get("score") or 0),
                "final_player_hp": int(top.get("final_player_hp") or 0),
                "final_player_block": int(top.get("final_player_block") or 0),
                "final_incoming": int(top.get("final_incoming") or 0),
                "final_monster_hp": [int(v) for v in top.get("final_monster_hp") or []],
                "tags": [str(tag) for tag in top.get("tags") or []],
            }
        )
    rows.sort(key=score_candidate, reverse=True)
    return rows


def equivalent_best_moves(
    oracle_candidates: list[dict[str, Any]],
) -> tuple[list[str], int | None, str | None]:
    if not oracle_candidates:
        return [], None, None
    best = oracle_candidates[0]
    best_score = score_candidate(best)
    equivalent = [
        str(candidate.get("move_label"))
        for candidate in oracle_candidates
        if score_candidate(candidate) == best_score
    ]
    margin = None
    for candidate in oracle_candidates[len(equivalent) :]:
        margin = int(best.get("score") or 0) - int(candidate.get("score") or 0)
        break
    return equivalent, margin, canonical_outcome(best.get("outcome"))


def choose_label_strength(
    baseline_move: str | None,
    chosen_trajectory: dict[str, Any] | None,
    oracle_best: dict[str, Any] | None,
    oracle_equivalent_best_moves: list[str],
    oracle_margin: int | None,
) -> str:
    if oracle_best is None:
        return "baseline_weak"
    baseline_in_best_bucket = bool(
        baseline_move and baseline_move in set(oracle_equivalent_best_moves)
    )
    chosen_outcome = canonical_outcome((chosen_trajectory or {}).get("outcome"))
    best_outcome = canonical_outcome(oracle_best.get("outcome"))
    chosen_score = int((chosen_trajectory or {}).get("score") or 0)
    best_score = int(oracle_best.get("score") or 0)
    if not baseline_in_best_bucket:
        if chosen_outcome == "dies" and best_outcome in {"survives", "lethal_win"}:
            return "oracle_strong"
        if outcome_rank(best_outcome) > outcome_rank(chosen_outcome):
            return "oracle_strong"
        if best_score - chosen_score >= 500:
            return "oracle_strong"
        return "oracle_preference"
    if len(oracle_equivalent_best_moves) > 1:
        return "oracle_preference"
    if oracle_margin is not None and oracle_margin >= 250:
        return "oracle_strong"
    return "oracle_preference"


def compute_batch_timeout(config: OracleConfig, frame_count: int) -> float | None:
    candidates = []
    if config.max_seconds_per_batch is not None:
        candidates.append(config.max_seconds_per_batch)
    if config.max_seconds_per_frame is not None:
        candidates.append(config.max_seconds_per_frame * max(frame_count, 1))
    return max(1.0, min(candidates)) if candidates else None


def invoke_batch_audit(
    raw_path: str,
    frames: list[int],
    config: OracleConfig,
    invocation: AuditInvocation,
) -> tuple[dict[int, dict[str, Any]], dict[str, int], float]:
    if not frames:
        return {}, {"succeeded": 0, "timed_out": 0, "engine_failed": 0}, 0.0
    with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as handle:
        batch_out = Path(handle.name)
    cmd = [
        *invocation.command_prefix,
        "audit-frame-batch",
        "--raw",
        raw_path,
        "--frames",
        ",".join(str(frame) for frame in frames),
        "--json-out",
        str(batch_out),
        "--decision-depth",
        str(config.decision_depth),
        "--top-k",
        str(config.top_k),
        "--branch-cap",
        str(config.branch_cap),
    ]
    if config.quiet:
        cmd.append("--quiet")
    timeout = compute_batch_timeout(config, len(frames))
    started = time.perf_counter()
    try:
        completed = subprocess.run(
            cmd,
            check=False,
            timeout=timeout,
            stdout=subprocess.DEVNULL if config.quiet else None,
            stderr=subprocess.DEVNULL if config.quiet else None,
        )
        elapsed = time.perf_counter() - started
    except subprocess.TimeoutExpired:
        elapsed = time.perf_counter() - started
        return (
            {
                frame: {
                    "status": "timed_out",
                    "report": None,
                    "error": f"batch timed out after {timeout:.1f}s",
                }
                for frame in frames
            },
            {"succeeded": 0, "timed_out": len(frames), "engine_failed": 0},
            elapsed,
        )
    if completed.returncode != 0 or not batch_out.exists():
        return (
            {
                frame: {
                    "status": "engine_failed",
                    "report": None,
                    "error": f"combat_decision_audit exited with code {completed.returncode}",
                }
                for frame in frames
            },
            {"succeeded": 0, "timed_out": 0, "engine_failed": len(frames)},
            elapsed,
        )
    payload = read_json(batch_out)
    results = {}
    stats = {"succeeded": 0, "timed_out": 0, "engine_failed": 0}
    for item in payload.get("results") or []:
        frame = int(item.get("frame") or 0)
        status = str(item.get("status") or "error")
        stats["succeeded" if status == "ok" else "engine_failed"] += 1
        results[frame] = {
            "status": status,
            "report": item.get("report"),
            "error": item.get("error"),
        }
    for frame in frames:
        results.setdefault(
            frame,
            {
                "status": "engine_failed",
                "report": None,
                "error": "batch result missing frame",
            },
        )
    return results, stats, elapsed


def save_cached_report(
    cache_dir: Path,
    frame: int,
    config: OracleConfig,
    invocation: AuditInvocation,
    report: dict[str, Any],
) -> Path:
    path = cache_path(cache_dir, frame, config, invocation)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2, ensure_ascii=False)
        handle.write("\n")
    return path


def chunked(values: list[int], size: int) -> list[list[int]]:
    size = max(size, 1)
    return [values[i : i + size] for i in range(0, len(values), size)]


def record_batch_profile(
    profile_batches: list[dict[str, Any]],
    *,
    run_id: str,
    raw_path: str,
    stage: str,
    config: OracleConfig,
    invocation: AuditInvocation,
    frames: list[int],
    elapsed_seconds: float,
    status_counts: dict[str, int],
    degraded_from_batch_size: int | None,
) -> None:
    profile_batches.append(
        {
            "run_id": run_id,
            "raw_path": raw_path,
            "stage": stage,
            "config_name": config.name,
            "audit_invocation_mode": invocation.mode,
            "frame_count": len(frames),
            "frames": frames,
            "elapsed_seconds": round(elapsed_seconds, 3),
            "status_counts": status_counts,
            "batch_size_used": len(frames),
            "degraded_from_batch_size": degraded_from_batch_size,
            "avg_seconds_per_frame_estimate": round(
                elapsed_seconds / float(max(len(frames), 1)), 3
            ),
        }
    )


def audit_frame_group(
    *,
    run_id: str,
    raw_path: str,
    frames: list[int],
    config: OracleConfig,
    invocation: AuditInvocation,
    stage: str,
    profile_batches: list[dict[str, Any]],
) -> tuple[dict[int, dict[str, Any]], dict[str, int]]:
    aggregate_results: dict[int, dict[str, Any]] = {}
    aggregate_stats = {"succeeded": 0, "timed_out": 0, "engine_failed": 0, "batch_count": 0}

    def process_chunk(chunk: list[int], degraded_from: int | None = None) -> None:
        nonlocal aggregate_results, aggregate_stats
        results, stats, elapsed = invoke_batch_audit(raw_path, chunk, config, invocation)
        for key, value in stats.items():
            aggregate_stats[key] += value
        aggregate_stats["batch_count"] += 1
        status_counts = Counter(result.get("status") or "error" for result in results.values())
        record_batch_profile(
            profile_batches,
            run_id=run_id,
            raw_path=raw_path,
            stage=stage,
            config=config,
            invocation=invocation,
            frames=chunk,
            elapsed_seconds=elapsed,
            status_counts=dict(status_counts),
            degraded_from_batch_size=degraded_from,
        )
        all_failed = all(result.get("status") != "ok" for result in results.values())
        all_timed_out = all(result.get("status") == "timed_out" for result in results.values())
        if len(chunk) > 1 and (all_failed or all_timed_out):
            smaller = max(1, len(chunk) // 2)
            for subchunk in chunked(chunk, smaller):
                process_chunk(subchunk, degraded_from=len(chunk))
            return
        aggregate_results.update(results)

    for batch in chunked(frames, config.batch_size):
        process_chunk(batch)
    return aggregate_results, aggregate_stats


def should_rerun_in_stage_two(
    row: dict[str, Any],
    oracle_best: dict[str, Any] | None,
    equivalent_moves: list[str],
    oracle_margin: int | None,
    chosen_trajectory: dict[str, Any] | None,
) -> bool:
    if oracle_best is None:
        return False
    baseline_move = row.get("chosen_move")
    baseline_in_best_bucket = bool(baseline_move and baseline_move in set(equivalent_moves))
    chosen_outcome = canonical_outcome((chosen_trajectory or {}).get("outcome"))
    best_outcome = canonical_outcome(oracle_best.get("outcome"))
    if chosen_outcome == "dies" and best_outcome in {"survives", "lethal_win"}:
        return True
    if not baseline_in_best_bucket:
        return True
    return len(equivalent_moves) > 1 and (oracle_margin is None or oracle_margin <= 200)


def make_stage_config(
    *,
    name: str,
    decision_depth: int | None,
    top_k: int | None,
    branch_cap: int | None,
    max_seconds_per_frame: float | None,
    max_seconds_per_batch: float | None,
    batch_size: int | None,
    quiet: bool,
) -> OracleConfig:
    defaults = {
        "fast": dict(
            decision_depth=4,
            top_k=2,
            branch_cap=4,
            max_seconds_per_frame=8.0,
            max_seconds_per_batch=45.0,
            batch_size=4,
        ),
        "slow": dict(
            decision_depth=5,
            top_k=3,
            branch_cap=8,
            max_seconds_per_frame=45.0,
            max_seconds_per_batch=180.0,
            batch_size=8,
        ),
    }[name]
    return OracleConfig(
        name=name,
        decision_depth=decision_depth or defaults["decision_depth"],
        top_k=top_k or defaults["top_k"],
        branch_cap=branch_cap or defaults["branch_cap"],
        max_seconds_per_frame=(
            max_seconds_per_frame
            if max_seconds_per_frame is not None
            else defaults["max_seconds_per_frame"]
        ),
        max_seconds_per_batch=(
            max_seconds_per_batch
            if max_seconds_per_batch is not None
            else defaults["max_seconds_per_batch"]
        ),
        batch_size=batch_size or defaults["batch_size"],
        quiet=quiet,
    )


def make_oracle_row(
    *,
    row: dict[str, Any],
    report: dict[str, Any] | None,
    report_path: Path | None,
    audit_error: str | None,
    config: OracleConfig,
    invocation: AuditInvocation,
) -> tuple[dict[str, Any], bool]:
    oracle_candidates = report_to_oracle_candidates(report or {})
    oracle_best = oracle_candidates[0] if oracle_candidates else None
    equivalent_moves, oracle_margin, oracle_outcome_bucket = equivalent_best_moves(
        oracle_candidates
    )
    baseline_move = row.get("chosen_move")
    chosen_trajectory = (report or {}).get("chosen_trajectory")
    label_strength = choose_label_strength(
        baseline_move=baseline_move,
        chosen_trajectory=chosen_trajectory,
        oracle_best=oracle_best,
        oracle_equivalent_best_moves=equivalent_moves,
        oracle_margin=oracle_margin,
    )
    if report is None:
        label_strength = "baseline_weak"
    disagrees = bool(
        baseline_move and equivalent_moves and baseline_move not in set(equivalent_moves)
    )
    oracle_best_move = oracle_best.get("move_label") if oracle_best else baseline_move
    return (
        {
            "dataset_kind": "oracle_combat_label",
            "state_source": row.get("state_source", "validated_livecomm_audit"),
            "run_id": row.get("run_id"),
            "frame_count": row.get("frame_count"),
            "response_id": row.get("response_id"),
            "state_frame_id": row.get("state_frame_id"),
            "baseline_chosen_move": baseline_move,
            "heuristic_move": row.get("heuristic_move"),
            "search_move": row.get("search_move"),
            "oracle_best_move": oracle_best_move,
            "oracle_equivalent_best_moves": equivalent_moves,
            "oracle_top_candidates": oracle_candidates,
            "oracle_value_estimate": oracle_best.get("score") if oracle_best else None,
            "oracle_margin": oracle_margin,
            "oracle_outcome_bucket": oracle_outcome_bucket,
            "oracle_best_bucket_size": len(equivalent_moves),
            "oracle_report_path": str(report_path) if report_path is not None else None,
            "label_source": (
                "offline_decision_audit_search" if oracle_best else "baseline_bot_choice"
            ),
            "label_strength": label_strength,
            "oracle_disagrees_with_baseline": disagrees,
            "oracle_compute_budget": {
                "mode": config.name,
                "decision_depth": config.decision_depth,
                "top_k": config.top_k,
                "branch_cap": config.branch_cap,
                "batch_size": config.batch_size,
                "max_seconds_per_frame": config.max_seconds_per_frame,
                "max_seconds_per_batch": config.max_seconds_per_batch,
                "audit_invocation_mode": invocation.mode,
                "oracle_binary": "combat_decision_audit audit-frame-batch",
            },
            "oracle_error": audit_error,
            "baseline_row": row,
        },
        disagrees,
    )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Oracle-label combat rows using offline decision audit search."
    )
    parser.add_argument("--baseline", default=REPO_ROOT / "tools" / "artifacts" / "learning_baseline.json", type=Path)
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--out", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "oracle_labeled_combat_rows.jsonl", type=Path)
    parser.add_argument("--summary-out", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "oracle_labeled_combat_summary.json", type=Path)
    parser.add_argument("--profile-out", default=None, type=Path)
    parser.add_argument("--audit-binary", default=None)
    parser.add_argument("--mode", choices=["fast", "slow"], default="fast")
    parser.add_argument("--decision-depth", default=None, type=int)
    parser.add_argument("--top-k", default=None, type=int)
    parser.add_argument("--branch-cap", default=None, type=int)
    parser.add_argument("--limit", default=64, type=int)
    parser.add_argument("--cache-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "oracle_cache", type=Path)
    parser.add_argument("--batch-size", default=None, type=int)
    parser.add_argument("--max-seconds-per-frame", default=None, type=float)
    parser.add_argument("--max-seconds-per-batch", default=None, type=float)
    parser.add_argument("--quiet", action="store_true")
    parser.add_argument("--resume", action="store_true")
    args = parser.parse_args()

    started_total = time.perf_counter()
    invocation = resolve_audit_invocation(args.audit_binary)
    primary_config = make_stage_config(
        name=args.mode,
        decision_depth=args.decision_depth,
        top_k=args.top_k,
        branch_cap=args.branch_cap,
        max_seconds_per_frame=args.max_seconds_per_frame,
        max_seconds_per_batch=args.max_seconds_per_batch,
        batch_size=args.batch_size,
        quiet=args.quiet,
    )
    secondary_config = make_stage_config(
        name="slow",
        decision_depth=None,
        top_k=None,
        branch_cap=None,
        max_seconds_per_frame=None,
        max_seconds_per_batch=None,
        batch_size=None,
        quiet=args.quiet,
    )

    baseline = read_json(args.baseline)
    raw_by_run = build_run_raw_index(baseline)
    candidate_rows = [
        row
        for _, row in iter_jsonl(args.dataset_dir / "combat_rows.jsonl")
        if row_is_oracle_candidate(row) and row.get("run_id") in raw_by_run
    ]
    candidate_rows.sort(key=state_priority, reverse=True)
    candidate_rows = candidate_rows[: args.limit]

    existing_rows = load_existing_rows(args.out, primary_config, invocation) if args.resume else {}
    grouped_rows: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    output_rows: list[dict[str, Any]] = []
    label_strength_counts: Counter[str] = Counter()
    equivalent_best_bucket_counts: Counter[int] = Counter()
    disagreement_count = 0
    cache_hits = 0
    resumed_rows = 0
    timed_out = 0
    engine_failed = 0
    skipped = 0
    profile_batches: list[dict[str, Any]] = []

    for row in candidate_rows:
        key = output_key(row)
        if key in existing_rows:
            reused = existing_rows[key]
            output_rows.append(reused)
            label_strength_counts[str(reused.get("label_strength") or "baseline_weak")] += 1
            equivalent_best_bucket_counts[int(reused.get("oracle_best_bucket_size") or 0)] += 1
            if reused.get("oracle_disagrees_with_baseline"):
                disagreement_count += 1
            resumed_rows += 1
            continue
        grouped_rows[(str(row["run_id"]), raw_by_run[str(row["run_id"])])].append(row)

    primary_results: dict[tuple[str, int], tuple[dict[str, Any] | None, Path | None, str | None]] = {}
    batch_stats = {"succeeded": 0, "timed_out": 0, "engine_failed": 0, "batch_count": 0}

    for (run_id, raw_path), rows in grouped_rows.items():
        run_cache_dir = args.cache_dir / run_id
        uncached_frames: list[int] = []
        for row in rows:
            frame = int(row["frame_count"])
            cached_path = cache_path(run_cache_dir, frame, primary_config, invocation)
            if cached_path.exists():
                cache_hits += 1
                primary_results[(run_id, frame)] = (read_json(cached_path), cached_path, None)
            else:
                uncached_frames.append(frame)
        if not uncached_frames:
            continue
        frame_results, stats = audit_frame_group(
            run_id=run_id,
            raw_path=raw_path,
            frames=uncached_frames,
            config=primary_config,
            invocation=invocation,
            stage="primary",
            profile_batches=profile_batches,
        )
        for stat_key, value in stats.items():
            batch_stats[stat_key] = batch_stats.get(stat_key, 0) + value
        for frame in uncached_frames:
            result = frame_results.get(frame) or {}
            status = str(result.get("status") or "engine_failed")
            if status == "ok":
                report = result.get("report") or {}
                cached_path = save_cached_report(run_cache_dir, frame, primary_config, invocation, report)
                primary_results[(run_id, frame)] = (report, cached_path, None)
            else:
                timed_out += int(status == "timed_out")
                engine_failed += int(status != "timed_out")
                primary_results[(run_id, frame)] = (None, None, str(result.get("error") or status))

    stage_two_rows: list[dict[str, Any]] = []
    if args.mode == "fast":
        for row in candidate_rows:
            if output_key(row) in existing_rows and args.resume:
                continue
            frame = row.get("frame_count")
            if frame is None:
                skipped += 1
                continue
            run_id = str(row["run_id"])
            report, _, _ = primary_results.get((run_id, int(frame)), (None, None, None))
            oracle_candidates = report_to_oracle_candidates(report or {})
            oracle_best = oracle_candidates[0] if oracle_candidates else None
            equivalent_moves, oracle_margin, _ = equivalent_best_moves(oracle_candidates)
            if should_rerun_in_stage_two(
                row, oracle_best, equivalent_moves, oracle_margin, (report or {}).get("chosen_trajectory")
            ):
                stage_two_rows.append(row)

    stage_two_results: dict[tuple[str, int], tuple[dict[str, Any] | None, Path | None, str | None]] = {}
    if stage_two_rows:
        grouped_stage_two: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
        for row in stage_two_rows:
            grouped_stage_two[(str(row["run_id"]), raw_by_run[str(row["run_id"])])].append(row)
        for (run_id, raw_path), rows in grouped_stage_two.items():
            run_cache_dir = args.cache_dir / run_id
            uncached_frames: list[int] = []
            for row in rows:
                frame = int(row["frame_count"])
                cached_path = cache_path(run_cache_dir, frame, secondary_config, invocation)
                if cached_path.exists():
                    cache_hits += 1
                    stage_two_results[(run_id, frame)] = (read_json(cached_path), cached_path, None)
                else:
                    uncached_frames.append(frame)
            if not uncached_frames:
                continue
            frame_results, stats = audit_frame_group(
                run_id=run_id,
                raw_path=raw_path,
                frames=uncached_frames,
                config=secondary_config,
                invocation=invocation,
                stage="refine",
                profile_batches=profile_batches,
            )
            for stat_key, value in stats.items():
                batch_stats[stat_key] = batch_stats.get(stat_key, 0) + value
            for frame in uncached_frames:
                result = frame_results.get(frame) or {}
                status = str(result.get("status") or "engine_failed")
                if status == "ok":
                    report = result.get("report") or {}
                    cached_path = save_cached_report(run_cache_dir, frame, secondary_config, invocation, report)
                    stage_two_results[(run_id, frame)] = (report, cached_path, None)
                else:
                    timed_out += int(status == "timed_out")
                    engine_failed += int(status != "timed_out")
                    stage_two_results[(run_id, frame)] = (None, None, str(result.get("error") or status))

    for row in candidate_rows:
        key = output_key(row)
        if key in existing_rows and args.resume:
            continue
        frame = row.get("frame_count")
        if frame is None:
            continue
        run_id = str(row["run_id"])
        selected_config = primary_config
        report, report_path, audit_error = primary_results.get((run_id, int(frame)), (None, None, None))
        if (run_id, int(frame)) in stage_two_results and stage_two_results[(run_id, int(frame))][0] is not None:
            report, report_path, audit_error = stage_two_results[(run_id, int(frame))]
            selected_config = secondary_config
        oracle_row, disagrees = make_oracle_row(
            row=row,
            report=report,
            report_path=report_path,
            audit_error=audit_error,
            config=selected_config,
            invocation=invocation,
        )
        output_rows.append(oracle_row)
        label_strength_counts[str(oracle_row.get("label_strength") or "baseline_weak")] += 1
        equivalent_best_bucket_counts[int(oracle_row.get("oracle_best_bucket_size") or 0)] += 1
        if disagrees:
            disagreement_count += 1

    output_rows.sort(key=lambda row: (str(row.get("run_id") or ""), int(row.get("frame_count") or 0)))
    write_jsonl(args.out, output_rows)

    elapsed_total_seconds = time.perf_counter() - started_total
    audited_frame_count = int(batch_stats.get("succeeded", 0))
    max_seconds_per_batch_observed = max((float(batch["elapsed_seconds"]) for batch in profile_batches), default=0.0)
    profile = {
        "mode": args.mode,
        "audit_invocation_mode": invocation.mode,
        "batches": profile_batches,
        "slowest_batches": sorted(profile_batches, key=lambda batch: float(batch.get("elapsed_seconds") or 0.0), reverse=True)[:10],
        "slowest_frame_candidates": sorted(profile_batches, key=lambda batch: float(batch.get("avg_seconds_per_frame_estimate") or 0.0), reverse=True)[:10],
    }
    raw_profile = {}
    for batch in profile_batches:
        raw_key = f'{batch["run_id"]}::{batch["raw_path"]}'
        summary = raw_profile.setdefault(raw_key, {"run_id": batch["run_id"], "raw_path": batch["raw_path"], "batch_count": 0, "elapsed_seconds": 0.0, "succeeded": 0, "timed_out": 0, "engine_failed": 0})
        summary["batch_count"] += 1
        summary["elapsed_seconds"] += float(batch.get("elapsed_seconds") or 0.0)
        status_counts = batch.get("status_counts") or {}
        summary["succeeded"] += int(status_counts.get("ok") or 0)
        summary["timed_out"] += int(status_counts.get("timed_out") or 0)
        summary["engine_failed"] += sum(int(v) for k, v in status_counts.items() if k not in {"ok", "timed_out"})
    profile["raw_summaries"] = list(raw_profile.values())

    summary = {
        "baseline": str(args.baseline),
        "dataset_dir": str(args.dataset_dir),
        "mode": args.mode,
        "candidate_rows": len(candidate_rows),
        "oracle_labeled_rows": len(output_rows),
        "oracle_disagreement_rows": disagreement_count,
        "oracle_disagreement_rate": float(disagreement_count) / float(len(output_rows)) if output_rows else 0.0,
        "label_strength_counts": dict(label_strength_counts),
        "equivalent_best_bucket_counts": {str(size): count for size, count in sorted(equivalent_best_bucket_counts.items())},
        "cache_hits": cache_hits,
        "resume_hits": resumed_rows,
        "newly_audited_rows": audited_frame_count,
        "timeouts": timed_out,
        "engine_failed": engine_failed,
        "batch_stats": batch_stats,
        "batch_count": int(batch_stats.get("batch_count", 0)),
        "audited_frame_count": audited_frame_count,
        "elapsed_total_seconds": round(elapsed_total_seconds, 3),
        "avg_seconds_per_audited_frame": round(elapsed_total_seconds / float(max(audited_frame_count, 1)), 3),
        "max_seconds_per_batch_observed": round(max_seconds_per_batch_observed, 3),
        "audit_invocation_mode": invocation.mode,
        "skipped_rows": skipped,
        "notes": [
            "oracle labels come from offline decision_audit search over livecomm state rows",
            "baseline choices are preserved as weak labels inside baseline_row",
            "equivalent-best buckets downgrade tie states to preference labels",
            "fast mode may refine a disagreement-heavy subset with a slow second stage",
        ],
    }
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    with args.summary_out.open("w", encoding="utf-8") as handle:
        json.dump(summary, handle, indent=2, ensure_ascii=False)
        handle.write("\n")
    if args.profile_out is not None:
        args.profile_out.parent.mkdir(parents=True, exist_ok=True)
        with args.profile_out.open("w", encoding="utf-8") as handle:
            json.dump(profile, handle, indent=2, ensure_ascii=False)
            handle.write("\n")

    print(json.dumps(summary, indent=2, ensure_ascii=False))
    print(f"wrote oracle combat labels to {args.out}")
    print(f"wrote oracle summary to {args.summary_out}")
    if args.profile_out is not None:
        print(f"wrote oracle profile to {args.profile_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
