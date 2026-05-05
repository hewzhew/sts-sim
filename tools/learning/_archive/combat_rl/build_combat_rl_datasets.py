#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
import tempfile
from typing import Any

from combat_rl_common import (
    REPO_ROOT,
    curriculum_tag_from_spec_name,
    discounted_returns,
    find_release_binary,
    horizon_return,
    load_oracle_policy_rows,
    load_policy_seed_rows,
    read_json,
    reward_breakdown_from_trace_step,
    run_combat_lab_spec,
    snapshot_from_trace_step,
    snapshot_state_features,
    write_json,
    write_jsonl,
    chosen_move_from_trace_step,
    normalized_candidates_from_trace_step,
    stable_split,
)


def transition_row(
    spec_name: str,
    trace: dict[str, Any],
    step: dict[str, Any],
    reward_breakdown: dict[str, float],
) -> dict[str, Any]:
    state_before = snapshot_from_trace_step(step, "before")
    state_after = snapshot_from_trace_step(step, "after")
    before_features = snapshot_state_features(state_before)
    after_features = snapshot_state_features(state_after)
    outcome = str(trace.get("outcome") or "").lower()
    done = bool(step.get("episode_outcome")) and step.get("step_index") == len(trace.get("steps") or []) - 1
    return {
        "dataset_kind": "combat_transition",
        "split": stable_split(f"{spec_name}::episode::{trace.get('episode_id')}"),
        "group_id": f"{spec_name}::episode::{trace.get('episode_id')}::step::{step.get('step_index')}",
        "sample_origin": "combat_lab_spec",
        "teacher_source": "combat_env_rollout",
        "curriculum_tag": curriculum_tag_from_spec_name(spec_name),
        "state_source": "combat_lab_trace",
        "label_source": "combat_env_rollout",
        "label_strength": "oracle_strong",
        "spec_name": spec_name,
        "sample_id": f"{spec_name}::{trace.get('episode_id')}::{step.get('step_index')}",
        "episode_id": trace.get("episode_id"),
        "seed": trace.get("seed"),
        "turn_index": step.get("turn_index"),
        "step_index": step.get("step_index"),
        "action_label": chosen_move_from_trace_step(step),
        "action_kind": step.get("action_kind"),
        "state_before": state_before,
        "state_after": state_after,
        "state_before_features": before_features,
        "state_after_features": after_features,
        "reward_breakdown": reward_breakdown,
        "reward_total": reward_breakdown["total"],
        "done": done,
        "terminal_outcome": outcome,
        "terminal_victory": outcome == "victory",
        "terminal_defeat": outcome == "defeat",
        "notes": ["transition row derived from combat_lab trace step"],
    }


def value_row(
    base_row: dict[str, Any],
    discounted_return: float,
    short_horizon_return: float,
    kill_within_horizon: bool,
) -> dict[str, Any]:
    row = dict(base_row)
    row["dataset_kind"] = "combat_value"
    row["discounted_return"] = round(discounted_return, 4)
    row["short_horizon_return"] = round(short_horizon_return, 4)
    row["survives_episode"] = bool(row.get("terminal_victory"))
    row["dies_episode"] = bool(row.get("terminal_defeat"))
    row["kill_within_horizon"] = bool(kill_within_horizon)
    row["notes"] = ["value row derived from combat_lab rollout return"]
    return row


def trace_policy_rows(spec_name: str, trace: dict[str, Any], step: dict[str, Any]) -> list[dict[str, Any]]:
    candidates = normalized_candidates_from_trace_step(step)
    if not candidates:
        return []
    chosen_move = chosen_move_from_trace_step(step)
    state_before = snapshot_from_trace_step(step, "before")
    sample_tags = list(step.get("bad_action_tags") or [])
    group_id = f"{spec_name}::policy::{trace.get('episode_id')}::{step.get('step_index')}"
    rows: list[dict[str, Any]] = []
    for index, candidate in enumerate(candidates):
        rows.append(
            {
                "dataset_kind": "combat_policy",
                "split": stable_split(f"{spec_name}::episode::{trace.get('episode_id')}"),
                "group_id": group_id,
                "sample_origin": "combat_lab_spec",
                "teacher_source": "combat_lab_policy_trace",
                "curriculum_tag": curriculum_tag_from_spec_name(spec_name),
                "state_source": "combat_lab_trace",
                "label_source": "combat_lab_policy_trace",
                "label_strength": "baseline_weak",
                "sample_tags": sample_tags,
                "training_weight": 0.2,
                "snapshot_normalized_state": state_before,
                "state_before": state_before,
                "candidate_move": candidate.get("move_label"),
                "candidate_rank": candidate.get("search_rank"),
                "candidate_score_hint": float(candidate.get("score") or 0.0),
                "candidate_is_positive": candidate.get("move_label") == chosen_move,
                "baseline_action": chosen_move,
                "preferred_action": chosen_move,
                "sample_id": group_id,
                "spec_name": spec_name,
                "episode_id": trace.get("episode_id"),
                "turn_index": step.get("turn_index"),
                "step_index": step.get("step_index"),
            }
        )
    return rows


def build_datasets_from_specs(
    combat_lab_binary: Path,
    spec_dir: Path,
    episodes: int,
    depth: int,
    base_seed: int,
    gamma: float,
    horizon: int,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], list[dict[str, Any]], dict[str, Any]]:
    transition_rows: list[dict[str, Any]] = []
    value_rows: list[dict[str, Any]] = []
    policy_rows: list[dict[str, Any]] = []
    spec_counts: Counter[str] = Counter()
    with tempfile.TemporaryDirectory(prefix="combat_rl_datasets_", dir=str(REPO_ROOT / "tmp")) as tmp_dir_name:
        tmp_dir = Path(tmp_dir_name)
        for spec_path in sorted(spec_dir.glob("*.json")):
            spec_name = spec_path.stem
            spec_out = tmp_dir / spec_name
            run_combat_lab_spec(combat_lab_binary, spec_path, episodes, depth, base_seed, spec_out)
            for trace_path in sorted(spec_out.glob("trace_*.json")):
                trace = read_json(trace_path)
                steps = list(trace.get("steps") or [])
                rewards = [
                    reward_breakdown_from_trace_step(step, step.get("step_index") == len(steps) - 1)["total"]
                    for step in steps
                ]
                returns = discounted_returns(rewards, gamma)
                monster_counts = [
                    snapshot_state_features(snapshot_from_trace_step(step, "after"))["living_monster_count"]
                    for step in steps
                ]
                for index, step in enumerate(steps):
                    transition = transition_row(
                        spec_name=spec_name,
                        trace=trace,
                        step=step,
                        reward_breakdown=reward_breakdown_from_trace_step(step, index == len(steps) - 1),
                    )
                    kill_within_horizon = any(count == 0 for count in monster_counts[index : index + max(horizon, 1)])
                    value = value_row(
                        transition,
                        discounted_return=returns[index],
                        short_horizon_return=horizon_return(rewards, index, gamma, horizon),
                        kill_within_horizon=kill_within_horizon,
                    )
                    transition_rows.append(transition)
                    value_rows.append(value)
                    policy_rows.extend(trace_policy_rows(spec_name, trace, step))
                spec_counts[spec_name] += len(steps)
    summary = {
        "spec_dir": str(spec_dir),
        "spec_row_counts": dict(spec_counts),
        "transition_rows": len(transition_rows),
        "value_rows": len(value_rows),
        "policy_rows_from_specs": len(policy_rows),
    }
    return transition_rows, value_rows, policy_rows, summary


def write_split_rows(out_dir: Path, prefix: str, rows: list[dict[str, Any]]) -> dict[str, int]:
    split_counts: dict[str, int] = {}
    for split in ("train", "val", "test"):
        split_rows = [row for row in rows if row.get("split") == split]
        write_jsonl(out_dir / f"{prefix}_{split}.jsonl", split_rows)
        split_counts[split] = len(split_rows)
    return split_counts


def main() -> int:
    parser = argparse.ArgumentParser(description="Build combat transition/value/policy datasets from local offline sources.")
    parser.add_argument("--out-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--spec-dir", default=REPO_ROOT / "data" / "combat_lab" / "specs", type=Path)
    parser.add_argument("--combat-lab-binary", default=None, type=Path)
    parser.add_argument("--episodes", default=6, type=int)
    parser.add_argument("--depth", default=6, type=int)
    parser.add_argument("--base-seed", default=1, type=int)
    parser.add_argument("--gamma", default=0.97, type=float)
    parser.add_argument("--horizon", default=3, type=int)
    parser.add_argument(
        "--seed-glob",
        default=str(REPO_ROOT / "data" / "combat_lab" / "policy_seed_set_*.jsonl"),
        help="Comma-separated glob list for policy seed set JSONL files.",
    )
    parser.add_argument(
        "--oracle-policy-path",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "oracle_labeled_combat_rows.jsonl",
        type=Path,
        help="Optional oracle-labeled combat rows used to append archived policy rows.",
    )
    args = parser.parse_args()

    combat_lab_binary = find_release_binary(args.combat_lab_binary, "combat_lab")
    transition_rows, value_rows, policy_rows, spec_summary = build_datasets_from_specs(
        combat_lab_binary=combat_lab_binary,
        spec_dir=args.spec_dir,
        episodes=args.episodes,
        depth=args.depth,
        base_seed=args.base_seed,
        gamma=args.gamma,
        horizon=args.horizon,
    )
    seed_policy_rows = load_policy_seed_rows(args.seed_glob)
    oracle_policy_rows = load_oracle_policy_rows(args.oracle_policy_path)
    policy_rows.extend(seed_policy_rows)
    policy_rows.extend(oracle_policy_rows)

    transition_splits = write_split_rows(args.out_dir, "combat_transition", transition_rows)
    value_splits = write_split_rows(args.out_dir, "combat_value", value_rows)
    policy_splits = write_split_rows(args.out_dir, "combat_policy", policy_rows)

    summary = {
        "combat_lab_binary": str(combat_lab_binary),
        "spec_dir": str(args.spec_dir),
        "episodes_per_spec": args.episodes,
        "depth": args.depth,
        "base_seed": args.base_seed,
        "gamma": args.gamma,
        "horizon": args.horizon,
        "seed_glob": args.seed_glob,
        "oracle_policy_path": str(args.oracle_policy_path) if args.oracle_policy_path else None,
        "transition_split_counts": transition_splits,
        "value_split_counts": value_splits,
        "policy_split_counts": policy_splits,
        "policy_source_counts": dict(Counter(str(row.get("sample_origin") or "unknown") for row in policy_rows)),
        "policy_label_strength_counts": dict(Counter(str(row.get("label_strength") or "unknown") for row in policy_rows)),
        "notes": [
            "transition/value rows are derived from local combat_lab traces and simulator outcomes",
            "policy rows mix combat_lab trace warm-start data, policy seed teachers, and archived oracle policy supplements",
            "livecomm is intentionally excluded from the primary combat RL dataset path",
        ],
    }
    summary.update(spec_summary)
    write_json(args.out_dir / "combat_rl_dataset_summary.json", summary)

    print(json.dumps(summary, indent=2, ensure_ascii=False))
    print(f"wrote combat transition/value/policy datasets to {args.out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
