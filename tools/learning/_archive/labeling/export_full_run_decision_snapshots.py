#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import time
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, find_release_binary, write_json, write_jsonl


SNAPSHOT_SCHEMA_VERSION = "full_run_decision_snapshot_v0"
IMPORTANT_CONTEXT_FLAGS = {
    "missing_full_deck_list",
    "missing_current_relics",
    "missing_potion_state",
}
TRAINING_CONTEXT_FLAGS = IMPORTANT_CONTEXT_FLAGS | {
    "missing_replay_clone_state",
    "missing_rollout_randomness_contract",
}
REWARD_DECISION_TYPES = {"reward_card_choice", "combat_card_reward"}
MAP_DECISION_TYPES = {"map"}
ROUTE_SENSITIVE_DECISION_TYPES = {"map", "campfire", "shop", "reward_card_choice"}
BOSS_SENSITIVE_DECISION_TYPES = {"reward_card_choice", "campfire", "map"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Export full-run run-batch traces as decision snapshots with schema readiness flags. "
            "This is a state pool/export contract, not a training-label generator."
        )
    )
    parser.add_argument("--trace-dir", type=Path, help="Read existing run-batch episode_*.json traces.")
    parser.add_argument("--episodes", type=int, default=5)
    parser.add_argument("--seed", type=int, default=70000)
    parser.add_argument("--policy", default="rule_baseline_v0", choices=["random_masked", "rule_baseline_v0"])
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=5000)
    parser.add_argument("--sts-dev-tool-binary", type=Path)
    parser.add_argument(
        "--artifact-dir",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "full_run_decision_snapshots",
    )
    parser.add_argument("--out", type=Path)
    parser.add_argument("--summary-out", type=Path)
    parser.add_argument(
        "--include-trace-summary",
        action="store_true",
        help="Include each episode trace summary in snapshot source metadata.",
    )
    return parser.parse_args()


def default_output_paths(args: argparse.Namespace) -> tuple[Path, Path]:
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    stem = f"{args.policy}_seed_{args.seed}_eps_{args.episodes}_{stamp}"
    out = args.out or args.artifact_dir / f"{stem}.jsonl"
    summary_out = args.summary_out or args.artifact_dir / f"{stem}.summary.json"
    return out, summary_out


def run_batch_trace(args: argparse.Namespace) -> Path:
    binary = find_release_binary(args.sts_dev_tool_binary, "sts_dev_tool")
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    run_dir = args.artifact_dir / "source_traces" / f"{args.policy}_seed_{args.seed}_eps_{args.episodes}_{stamp}"
    trace_dir = run_dir / "traces"
    summary_path = run_dir / "run_batch_summary.json"
    cmd = [
        str(binary),
        "run-batch",
        "--episodes",
        str(args.episodes),
        "--seed",
        str(args.seed),
        "--policy",
        args.policy,
        "--ascension",
        str(args.ascension),
        "--class",
        args.player_class,
        "--max-steps",
        str(args.max_steps),
        "--determinism-check",
        "--summary-out",
        str(summary_path),
        "--trace-dir",
        str(trace_dir),
    ]
    if args.final_act:
        cmd.append("--final-act")

    start = time.perf_counter()
    proc = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    elapsed = time.perf_counter() - start
    if proc.returncode != 0:
        raise RuntimeError(f"run-batch failed with code {proc.returncode}: {proc.stderr}")
    print(f"generated run-batch traces in {trace_dir} ({elapsed:.2f}s)")
    return trace_dir


def load_trace_files(trace_dir: Path) -> list[Path]:
    trace_files = sorted(trace_dir.glob("episode_*.json"))
    if not trace_files:
        raise SystemExit(f"no episode_*.json traces found in {trace_dir}")
    return trace_files


def load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def export_snapshots(args: argparse.Namespace, trace_dir: Path) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    trace_files = load_trace_files(trace_dir)
    snapshots: list[dict[str, Any]] = []
    readiness_counts: Counter[str] = Counter()
    quality_flag_counts: Counter[str] = Counter()
    decision_type_counts: Counter[str] = Counter()
    episode_count = 0
    step_count = 0

    for trace_path in trace_files:
        trace = load_json(trace_path)
        summary = trace.get("summary") or {}
        episode_count += 1
        steps = trace.get("steps") or []
        step_count += len(steps)
        for step in steps:
            snapshot = snapshot_from_step(
                trace_path=trace_path,
                trace=trace,
                summary=summary,
                step=step,
                include_trace_summary=bool(args.include_trace_summary),
            )
            snapshots.append(snapshot)
            readiness_counts[snapshot["readiness_level"]] += 1
            decision_type_counts[snapshot["decision_type"]] += 1
            quality_flag_counts.update(snapshot["quality_flags"])

    summary_payload = {
        "snapshot_schema_version": SNAPSHOT_SCHEMA_VERSION,
        "created_at_utc": datetime.now(timezone.utc).isoformat(),
        "source": {
            "kind": "rust_run_batch_trace",
            "trace_dir": str(trace_dir),
            "trace_file_count": len(trace_files),
        },
        "config": {
            "source_mode": "existing_trace_dir" if args.trace_dir else "generated_run_batch",
            "run_batch": None
            if args.trace_dir
            else {
                "episodes": args.episodes,
                "seed": args.seed,
                "policy": args.policy,
                "ascension": args.ascension,
                "player_class": args.player_class,
                "final_act": bool(args.final_act),
                "max_steps": args.max_steps,
            },
        },
        "counts": {
            "episodes": episode_count,
            "steps": step_count,
            "snapshots": len(snapshots),
        },
        "decision_type_counts": dict(sorted(decision_type_counts.items())),
        "readiness_counts": dict(sorted(readiness_counts.items())),
        "quality_flag_counts": dict(sorted(quality_flag_counts.items())),
        "interpretation": {
            "smoke_ready": "Has enough data to inspect policy behavior and legal candidate shape.",
            "eval_ready": "Has full visible run context for human/model preference evaluation.",
            "training_ready": "Has replay/clone/randomness contract needed for rollout-backed labels.",
            "current_expected_gap": (
                "Rust run-batch traces expose visible deck/relic/potion/map context, but they "
                "do not yet include a replay clone state or paired rollout randomness contract."
            ),
        },
    }
    return snapshots, summary_payload


def snapshot_from_step(
    *,
    trace_path: Path,
    trace: dict[str, Any],
    summary: dict[str, Any],
    step: dict[str, Any],
    include_trace_summary: bool,
) -> dict[str, Any]:
    observation = step.get("observation") or {}
    candidates = step.get("action_mask") or []
    chosen_index = int(step.get("chosen_action_index") or 0)
    chosen_candidate = candidates[chosen_index] if 0 <= chosen_index < len(candidates) else {}
    decision_type = str(step.get("decision_type") or observation.get("decision_type") or "unknown")
    source = {
        "kind": "rust_run_batch_trace",
        "trace_path": str(trace_path),
        "observation_schema_version": trace.get("observation_schema_version"),
        "action_schema_version": trace.get("action_schema_version"),
    }
    if include_trace_summary:
        source["episode_summary"] = summary

    quality_flags = quality_flags_for_snapshot(observation, candidates, decision_type)
    readiness_level = readiness_level_for_flags(quality_flags, observation, candidates)
    return {
        "snapshot_schema_version": SNAPSHOT_SCHEMA_VERSION,
        "source": source,
        "episode_id": int(summary.get("episode_id") or 0),
        "seed": int(summary.get("seed") or 0),
        "step_index": int(step.get("step_index") or 0),
        "floor": int(step.get("floor") or observation.get("floor") or 0),
        "act": int(step.get("act") or observation.get("act") or 0),
        "decision_type": decision_type,
        "engine_state": str(step.get("engine_state") or observation.get("engine_state") or "unknown"),
        "hp": int(step.get("hp") or observation.get("current_hp") or 0),
        "max_hp": int(step.get("max_hp") or observation.get("max_hp") or 0),
        "gold": int(step.get("gold") or observation.get("gold") or 0),
        "deck_size": int(step.get("deck_size") or observation.get("deck_size") or 0),
        "relic_count": int(step.get("relic_count") or observation.get("relic_count") or 0),
        "legal_action_count": int(step.get("legal_action_count") or len(candidates)),
        "observation": observation,
        "legal_candidates": candidates,
        "chosen_action_index": chosen_index,
        "chosen_action_id": int(step.get("chosen_action_id") or chosen_candidate.get("action_id") or 0),
        "chosen_action_key": str(step.get("chosen_action_key") or chosen_candidate.get("action_key") or ""),
        "chosen_action": step.get("chosen_action") or chosen_candidate.get("action") or {},
        "chosen_candidate": chosen_candidate,
        "quality_flags": quality_flags,
        "readiness_level": readiness_level,
        "can_use_for_eval": readiness_level in {"eval_ready", "training_ready"},
        "can_use_for_training": readiness_level == "training_ready",
        "recommended_use": recommended_use(readiness_level, quality_flags),
    }


def quality_flags_for_snapshot(
    observation: dict[str, Any],
    candidates: list[dict[str, Any]],
    decision_type: str,
) -> list[str]:
    flags: list[str] = []

    if not candidates:
        flags.append("missing_candidate_list")

    if not isinstance(observation.get("deck"), dict):
        flags.append("missing_deck_summary")
    if not has_nonempty_list(observation, "deck.cards") and not has_nonempty_list(observation, "deck_cards"):
        flags.append("missing_full_deck_list")
    if not has_list(observation, "relics"):
        flags.append("missing_current_relics")
    if not has_list(observation, "potions"):
        flags.append("missing_potion_state")

    if decision_type in REWARD_DECISION_TYPES and not observation.get("reward_source"):
        flags.append("missing_reward_source")
    is_boss_reward = observation.get("reward_source") == "boss_combat_reward"
    if decision_type in MAP_DECISION_TYPES and not observation.get("map"):
        flags.append("missing_map_graph")
    if decision_type in ROUTE_SENSITIVE_DECISION_TYPES and not is_boss_reward and not (
        observation.get("next_nodes") or nested_get(observation, "map.next_nodes")
    ):
        flags.append("missing_next_nodes")
    if decision_type in BOSS_SENSITIVE_DECISION_TYPES and not is_boss_reward and not (
        observation.get("boss") or observation.get("act_boss") or observation.get("boss_id")
    ):
        flags.append("missing_boss_info")

    # These are deliberately marked missing until traces can replay/clone every candidate branch.
    flags.append("missing_replay_clone_state")
    flags.append("missing_rollout_randomness_contract")

    if IMPORTANT_CONTEXT_FLAGS.intersection(flags):
        flags.append("insufficient_context")
    if TRAINING_CONTEXT_FLAGS.intersection(flags):
        flags.append("not_training_ready")

    return sorted(set(flags))


def readiness_level_for_flags(
    quality_flags: list[str],
    observation: dict[str, Any],
    candidates: list[dict[str, Any]],
) -> str:
    flag_set = set(quality_flags)
    has_minimal_shape = bool(observation) and bool(candidates)
    if not has_minimal_shape:
        return "not_ready"
    if not flag_set.intersection(TRAINING_CONTEXT_FLAGS):
        return "training_ready"
    if not flag_set.intersection(IMPORTANT_CONTEXT_FLAGS):
        return "eval_ready"
    return "smoke_ready"


def recommended_use(readiness_level: str, quality_flags: list[str]) -> list[str]:
    uses: list[str] = []
    if readiness_level == "training_ready":
        uses.extend(["candidate_rollout_training", "policy_evaluation", "schema_regression"])
    elif readiness_level == "eval_ready":
        uses.extend(["human_audit", "policy_evaluation", "schema_regression"])
    elif readiness_level == "smoke_ready":
        uses.extend(["capability_smoke", "schema_lint", "exporter_regression"])
    else:
        uses.append("reject")
    if "missing_candidate_list" in quality_flags:
        uses = ["reject"]
    return uses


def has_nonempty_list(data: dict[str, Any], dotted_key: str) -> bool:
    value = nested_get(data, dotted_key)
    return isinstance(value, list) and bool(value)


def has_list(data: dict[str, Any], dotted_key: str) -> bool:
    value = nested_get(data, dotted_key)
    return isinstance(value, list)


def nested_get(data: dict[str, Any], dotted_key: str) -> Any:
    current: Any = data
    for part in dotted_key.split("."):
        if not isinstance(current, dict) or part not in current:
            return None
        current = current[part]
    return current


def main() -> None:
    args = parse_args()
    out_path, summary_path = default_output_paths(args)
    trace_dir = args.trace_dir or run_batch_trace(args)
    snapshots, summary = export_snapshots(args, trace_dir)
    write_jsonl(out_path, snapshots)
    summary["outputs"] = {"snapshots": str(out_path), "summary": str(summary_path)}
    write_json(summary_path, summary)
    print(json.dumps(summary, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
