#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import numpy as np

from build_structured_bc_teacher_dataset import (
    legal_candidates,
    make_env,
    score_teacher_candidates,
)
from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from structured_candidate_ranker_common import (
    ACTION_CLASS_IDS,
    MAX_RANKER_CANDIDATES,
    candidate_action_array,
    candidate_action_class,
    candidate_feature_vector,
    empty_candidate_arrays,
)
from train_structured_combat_ppo import load_start_spec_name, parse_seed_list


def append_state(
    obs_samples: dict[str, list[np.ndarray]],
    ranker_samples: dict[str, list[np.ndarray]],
    obs: dict[str, np.ndarray],
    arrays: dict[str, np.ndarray],
) -> None:
    for key, value in obs.items():
        obs_samples.setdefault(key, []).append(np.asarray(value))
    for key, value in arrays.items():
        ranker_samples.setdefault(key, []).append(np.asarray(value))


def write_npz_dataset(
    path: Path,
    obs_samples: dict[str, list[np.ndarray]],
    ranker_samples: dict[str, list[np.ndarray]],
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload: dict[str, np.ndarray] = {}
    for key, values in obs_samples.items():
        payload[f"obs__{key}"] = np.stack(values, axis=0)
    for key, values in ranker_samples.items():
        payload[key] = np.stack(values, axis=0)
    np.savez_compressed(path, **payload)


def build_ranker_arrays(
    *,
    raw_observation: dict[str, Any],
    scored_candidates: list[dict[str, Any]],
    tie_epsilon: float,
) -> tuple[dict[str, np.ndarray] | None, list[dict[str, Any]]]:
    valid = [row for row in scored_candidates if row.get("action") is not None and row.get("candidate") is not None]
    if not valid:
        return None, []
    valid = valid[:MAX_RANKER_CANDIDATES]
    best_score = max(float(row.get("score") or -1e9) for row in valid)
    best_indices = [index for index, row in enumerate(valid) if float(row.get("score") or -1e9) >= best_score - tie_epsilon]
    if not best_indices:
        return None, []
    arrays = empty_candidate_arrays()
    rows: list[dict[str, Any]] = []
    for index, row in enumerate(valid):
        candidate = dict(row.get("candidate") or {})
        action = dict(row.get("action") or {})
        action_class = candidate_action_class(candidate)
        arrays["candidate_features"][index] = np.asarray(
            candidate_feature_vector(raw_observation, candidate),
            dtype=np.float32,
        )
        arrays["candidate_mask"][index] = 1.0
        arrays["candidate_scores"][index] = float(row.get("score") or -1e9)
        arrays["candidate_survival_rank"][index] = float(row.get("survival_rank") or 0.0)
        arrays["candidate_class"][index] = int(ACTION_CLASS_IDS.get(action_class, 0))
        arrays["candidate_actions"][index] = np.asarray(candidate_action_array(action), dtype=np.int64)
        if index in best_indices:
            arrays["best_mask"][index] = 1.0
        rows.append(
            {
                "candidate_index": index,
                "label": row.get("candidate_label"),
                "score": row.get("score"),
                "survival_rank": row.get("survival_rank"),
                "survival_margin": row.get("survival_margin"),
                "action_class": action_class,
                "action": action,
                "is_teacher_top": index in best_indices,
            }
        )
    arrays["best_index"] = np.asarray(best_indices[0], dtype=np.int64)
    arrays["best_class"] = np.asarray(int(arrays["candidate_class"][best_indices[0]]), dtype=np.int64)
    for index in best_indices:
        arrays["best_class_mask"][int(arrays["candidate_class"][index])] = 1.0
    return arrays, rows


def main() -> None:
    parser = argparse.ArgumentParser(description="Build a structured candidate-ranking teacher dataset.")
    parser.add_argument("--spec-source", choices=["start_spec"], default="start_spec")
    parser.add_argument("--start-spec", action="append", required=True, type=Path)
    parser.add_argument("--seeds", default="2009,2010,2011,2012")
    parser.add_argument("--samples", default=128, type=int)
    parser.add_argument("--max-episode-steps", default=96, type=int)
    parser.add_argument("--draw-order-variant", choices=["exact", "reshuffle_draw"], default="reshuffle_draw")
    parser.add_argument("--reward-mode", choices=["legacy", "minimal_rl"], default="minimal_rl")
    parser.add_argument("--victory-reward", default=1.0, type=float)
    parser.add_argument("--defeat-reward", default=-1.0, type=float)
    parser.add_argument("--hp-loss-scale", default=0.02, type=float)
    parser.add_argument("--enemy-hp-delta-scale", default=0.01, type=float)
    parser.add_argument("--kill-bonus-scale", default=0.0, type=float)
    parser.add_argument("--catastrophe-unblocked-threshold", default=18.0, type=float)
    parser.add_argument("--catastrophe-penalty", default=0.25, type=float)
    parser.add_argument("--next-enemy-window-relief-scale", default=0.0, type=float)
    parser.add_argument("--persistent-attack-script-relief-scale", default=0.0, type=float)
    parser.add_argument("--tie-epsilon", default=1e-6, type=float)
    parser.add_argument("--driver-binary", default=None, type=Path)
    parser.add_argument(
        "--out",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "structured_candidate_ranker_dataset.npz",
        type=Path,
    )
    parser.add_argument("--rows-out", default=None, type=Path)
    parser.add_argument("--summary-out", default=None, type=Path)
    args = parser.parse_args()

    spec_paths = [Path(path) for path in args.start_spec]
    seeds = parse_seed_list(args.seeds)
    reward_config = {
        "victory_reward": float(args.victory_reward),
        "defeat_reward": float(args.defeat_reward),
        "hp_loss_scale": float(args.hp_loss_scale),
        "enemy_hp_delta_scale": float(args.enemy_hp_delta_scale),
        "kill_bonus_scale": float(args.kill_bonus_scale),
        "catastrophe_unblocked_threshold": float(args.catastrophe_unblocked_threshold),
        "catastrophe_penalty": float(args.catastrophe_penalty),
        "next_enemy_window_relief_scale": float(args.next_enemy_window_relief_scale),
        "persistent_attack_script_relief_scale": float(args.persistent_attack_script_relief_scale),
    }
    env_args = {
        "spec_paths": spec_paths,
        "spec_source": args.spec_source,
        "driver_binary": args.driver_binary,
        "max_episode_steps": args.max_episode_steps,
        "draw_order_variant": args.draw_order_variant,
        "reward_mode": args.reward_mode,
        "reward_config": reward_config,
        "seed": 0,
    }

    obs_samples: dict[str, list[np.ndarray]] = {}
    ranker_samples: dict[str, list[np.ndarray]] = {}
    rows: list[dict[str, Any]] = []
    episodes_started = 0
    candidate_evals = 0
    main_env = make_env(**env_args)
    try:
        for spec_path in spec_paths:
            for seed_hint in seeds:
                sample_count = len(next(iter(ranker_samples.values()), []))
                if sample_count >= args.samples:
                    break
                episodes_started += 1
                obs, info = main_env.reset(options={"spec_path": spec_path, "seed_hint": seed_hint})
                prefix_actions: list[dict[str, int]] = []
                done = False
                truncated = False
                step_index = 0
                while not done and not truncated and step_index < args.max_episode_steps:
                    sample_count = len(next(iter(ranker_samples.values()), []))
                    if sample_count >= args.samples:
                        break
                    candidates = legal_candidates(info)
                    if not candidates:
                        break
                    scored = score_teacher_candidates(
                        spec_path=spec_path,
                        seed_hint=seed_hint,
                        prefix_actions=prefix_actions,
                        candidates=candidates,
                        main_env=main_env,
                        env_args=env_args,
                    )
                    candidate_evals += len(candidates)
                    arrays, candidate_rows = build_ranker_arrays(
                        raw_observation=info.get("raw_observation") or {},
                        scored_candidates=scored,
                        tie_epsilon=float(args.tie_epsilon),
                    )
                    if arrays is None:
                        break
                    append_state(obs_samples, ranker_samples, obs, arrays)
                    best_index = int(arrays["best_index"])
                    best_action = {
                        key: int(value)
                        for key, value in zip(
                            ("action_type", "card_slot", "target_slot", "potion_slot", "choice_index"),
                            arrays["candidate_actions"][best_index],
                        )
                    }
                    rows.append(
                        {
                            "sample_index": len(rows),
                            "spec_path": str(spec_path.relative_to(REPO_ROOT) if spec_path.is_relative_to(REPO_ROOT) else spec_path),
                            "spec_name": load_start_spec_name(spec_path),
                            "seed": seed_hint,
                            "step_index": step_index,
                            "prefix_len": len(prefix_actions),
                            "best_index": best_index,
                            "best_class": int(arrays["best_class"]),
                            "legal_action_count": len(candidates),
                            "retained_candidate_count": len(candidate_rows),
                            "candidates": candidate_rows,
                        }
                    )
                    obs, _, done, truncated, info = main_env.step(best_action)
                    if info.get("invalid_action") or info.get("decoder_failure"):
                        break
                    prefix_actions.append(best_action)
                    step_index += 1
            if len(next(iter(ranker_samples.values()), [])) >= args.samples:
                break
    finally:
        main_env.close()

    sample_count = len(next(iter(ranker_samples.values()), []))
    if sample_count == 0:
        raise SystemExit("candidate ranker dataset generation produced no samples")
    write_npz_dataset(args.out, obs_samples, ranker_samples)
    rows_out = args.rows_out or args.out.with_suffix(".jsonl")
    summary_out = args.summary_out or args.out.with_suffix(".summary.json")
    write_jsonl(rows_out, rows)
    summary = {
        "dataset": str(args.out),
        "rows": str(rows_out),
        "sample_count": sample_count,
        "episodes_started": episodes_started,
        "candidate_evals": candidate_evals,
        "max_candidates": MAX_RANKER_CANDIDATES,
        "specs": [str(path) for path in spec_paths],
        "seeds": seeds,
        "draw_order_variant": args.draw_order_variant,
        "reward_mode": args.reward_mode,
        "reward": reward_config,
        "teacher": {
            "kind": "candidate_ranking_one_step_branch_score",
            "tie_epsilon": float(args.tie_epsilon),
            "action_classes": ACTION_CLASS_IDS,
            "notes": [
                "dataset preserves the legal candidate set rather than a single card-slot label",
                "candidate features contain static action abstraction and current target context",
                "teacher scores remain supervision only; branch outcomes are not candidate inputs",
            ],
        },
    }
    write_json(summary_out, summary)
    print(json.dumps(summary, indent=2, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
