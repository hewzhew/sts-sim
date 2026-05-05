#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT

DEFAULT_EVAL_SEEDS = "2009,2010,2011,2012,2013,2014,2015,2016"

STRONG_HEXAGHOST_SPEC: dict[str, Any] = {
    "name": "ironclad_hexaghost_op_v1_strong_deck",
    "player_class": "Ironclad",
    "ascension_level": 0,
    "encounter_id": "Hexaghost",
    "room_type": "MonsterRoomBoss",
    "seed": 7331,
    "player_current_hp": 80,
    "player_max_hp": 80,
    "relics": ["Burning Blood"],
    "potions": [],
    "master_deck": [
        {"id": "Bash", "upgrades": 1},
        {"id": "Disarm", "upgrades": 1},
        {"id": "Shockwave", "upgrades": 1},
        {"id": "Shrug It Off", "upgrades": 1},
        {"id": "Shrug It Off", "upgrades": 1},
        {"id": "Impervious", "upgrades": 1},
        {"id": "Power Through", "upgrades": 1},
        {"id": "Flame Barrier", "upgrades": 1},
        {"id": "Inflame", "upgrades": 1},
        {"id": "Demon Form", "upgrades": 1},
        {"id": "Offering", "upgrades": 1},
        {"id": "Reaper", "upgrades": 1},
        {"id": "Limit Break", "upgrades": 1},
        {"id": "Pommel Strike", "upgrades": 1},
        {"id": "Pommel Strike", "upgrades": 1},
        {"id": "Twin Strike", "upgrades": 1},
        {"id": "Clothesline", "upgrades": 1},
    ],
}


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as handle:
        json.dump(payload, handle, indent=2, ensure_ascii=False)
        handle.write("\n")


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def stage_specs(generated_spec_dir: Path) -> dict[str, Path]:
    strong_path = generated_spec_dir / "hexaghost_op_v1_start_spec.json"
    write_json(strong_path, STRONG_HEXAGHOST_SPEC)
    return {
        "hexaghost_op_v1": strong_path,
        "hexaghost_v3": REPO_ROOT / "data" / "boss_validation" / "hexaghost_v3" / "start_spec.json",
        "hexaghost_v2": REPO_ROOT / "data" / "boss_validation" / "hexaghost_v2" / "start_spec.json",
    }


def eval_summary(metrics: dict[str, Any], key: str) -> dict[str, Any]:
    section = metrics.get(key) or {}
    return {
        "win_rate": section.get("win_rate"),
        "mean_reward": section.get("mean_reward"),
        "mean_steps": section.get("mean_steps"),
        "first_action_end_turn_rate": section.get("first_action_end_turn_rate"),
        "first_action_survival_rate": section.get("first_action_survival_rate"),
        "first_action_attack_like_rate": section.get("first_action_attack_like_rate"),
    }


def timing_summary(metrics: dict[str, Any]) -> dict[str, Any]:
    timing = metrics.get("timing") or {}
    return {
        "total_seconds": timing.get("total_seconds"),
        "rollout_policy_seconds": timing.get("rollout_policy_seconds"),
        "rollout_env_seconds": timing.get("rollout_env_seconds"),
        "update_seconds": timing.get("update_seconds"),
        "eval_seconds": timing.get("eval_seconds"),
        "rollout_steps_per_second": timing.get("rollout_steps_per_second"),
        "env_step_milliseconds": timing.get("env_step_milliseconds"),
        "policy_step_milliseconds": timing.get("policy_step_milliseconds"),
        "update_step_milliseconds": timing.get("update_step_milliseconds"),
    }


def run_command(cmd: list[str], *, dry_run: bool) -> None:
    print(" ".join(cmd), flush=True)
    if dry_run:
        return
    subprocess.run(cmd, cwd=REPO_ROOT, check=True)


def main() -> None:
    parser = argparse.ArgumentParser(description="Run structured PPO over a small start-spec curriculum sweep.")
    parser.add_argument("--stages", default="hexaghost_op_v1,hexaghost_v3,hexaghost_v2")
    parser.add_argument("--timesteps", default=32768, type=int)
    parser.add_argument("--n-envs", default=4, type=int)
    parser.add_argument("--rollout-steps", default=64, type=int)
    parser.add_argument("--epochs", default=4, type=int)
    parser.add_argument("--minibatch-size", default=128, type=int)
    parser.add_argument("--max-episode-steps", default=128, type=int)
    parser.add_argument("--eval-seeds", default=DEFAULT_EVAL_SEEDS)
    parser.add_argument("--draw-order-variant", choices=["exact", "reshuffle_draw"], default="reshuffle_draw")
    parser.add_argument("--reward-mode", choices=["legacy", "minimal_rl"], default="minimal_rl")
    parser.add_argument("--device", choices=["auto", "cpu", "cuda"], default="auto")
    parser.add_argument("--victory-reward", default=1.0, type=float)
    parser.add_argument("--defeat-reward", default=-1.0, type=float)
    parser.add_argument("--hp-loss-scale", default=0.02, type=float)
    parser.add_argument("--enemy-hp-delta-scale", default=0.01, type=float)
    parser.add_argument("--kill-bonus-scale", default=0.0, type=float)
    parser.add_argument("--catastrophe-unblocked-threshold", default=18.0, type=float)
    parser.add_argument("--catastrophe-penalty", default=0.25, type=float)
    parser.add_argument("--next-enemy-window-relief-scale", default=0.0, type=float)
    parser.add_argument("--persistent-attack-script-relief-scale", default=0.0, type=float)
    parser.add_argument("--bc-dataset", default=None, type=Path)
    parser.add_argument("--bc-warmup-epochs", default=0, type=int)
    parser.add_argument("--bc-batch-size", default=128, type=int)
    parser.add_argument("--bc-max-samples", default=0, type=int)
    parser.add_argument("--bc-only", action="store_true")
    parser.add_argument("--output-prefix", default="structured_start_spec_curriculum")
    parser.add_argument("--driver-binary", default=None, type=Path)
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--fail-on-missing", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    dataset_dir = REPO_ROOT / "tools" / "artifacts" / "learning_dataset"
    generated_spec_dir = dataset_dir / "generated_start_specs"
    specs = stage_specs(generated_spec_dir)
    requested_stages = [stage.strip() for stage in str(args.stages).split(",") if stage.strip()]
    if not requested_stages:
        raise SystemExit("--stages must name at least one stage")

    if not args.skip_build:
        run_command(["cargo", "build", "--release", "--bin", "combat_env_driver"], dry_run=args.dry_run)

    stage_rows: list[dict[str, Any]] = []
    for stage in requested_stages:
        if stage not in specs:
            raise SystemExit(f"unknown stage '{stage}'; known stages: {', '.join(sorted(specs))}")
        spec_path = specs[stage]
        if not spec_path.exists():
            message = f"missing start spec for stage '{stage}': {spec_path}"
            if args.fail_on_missing:
                raise SystemExit(message)
            print(f"skipping: {message}", flush=True)
            continue

        prefix = f"{args.output_prefix}_{stage}"
        cmd = [
            sys.executable,
            str(REPO_ROOT / "tools" / "learning" / "train_structured_combat_ppo.py"),
            "--spec-source",
            "start_spec",
            "--start-spec",
            str(spec_path),
            "--draw-order-variant",
            args.draw_order_variant,
            "--reward-mode",
            args.reward_mode,
            "--device",
            args.device,
            "--victory-reward",
            str(args.victory_reward),
            "--defeat-reward",
            str(args.defeat_reward),
            "--hp-loss-scale",
            str(args.hp_loss_scale),
            "--enemy-hp-delta-scale",
            str(args.enemy_hp_delta_scale),
            "--kill-bonus-scale",
            str(args.kill_bonus_scale),
            "--catastrophe-unblocked-threshold",
            str(args.catastrophe_unblocked_threshold),
            "--catastrophe-penalty",
            str(args.catastrophe_penalty),
            "--next-enemy-window-relief-scale",
            str(args.next_enemy_window_relief_scale),
            "--persistent-attack-script-relief-scale",
            str(args.persistent_attack_script_relief_scale),
            "--timesteps",
            str(args.timesteps),
            "--n-envs",
            str(args.n_envs),
            "--rollout-steps",
            str(args.rollout_steps),
            "--epochs",
            str(args.epochs),
            "--minibatch-size",
            str(args.minibatch_size),
            "--max-episode-steps",
            str(args.max_episode_steps),
            "--eval-seeds",
            args.eval_seeds,
            "--output-prefix",
            prefix,
        ]
        if args.bc_dataset is not None:
            cmd.extend(
                [
                    "--bc-dataset",
                    str(args.bc_dataset),
                    "--bc-warmup-epochs",
                    str(args.bc_warmup_epochs),
                    "--bc-batch-size",
                    str(args.bc_batch_size),
                    "--bc-max-samples",
                    str(args.bc_max_samples),
                ]
            )
        if args.bc_only:
            cmd.append("--bc-only")
        if args.driver_binary is not None:
            cmd.extend(["--driver-binary", str(args.driver_binary)])
        run_command(cmd, dry_run=args.dry_run)
        metrics_path = dataset_dir / f"{prefix}_structured_combat_ppo_metrics.json"
        metrics = {} if args.dry_run else read_json(metrics_path)
        stage_rows.append(
            {
                "stage": stage,
                "start_spec": str(spec_path.relative_to(REPO_ROOT) if spec_path.is_relative_to(REPO_ROOT) else spec_path),
                "metrics_path": str(metrics_path.relative_to(REPO_ROOT)),
                "eval": eval_summary(metrics, "eval"),
                "random_benchmark": eval_summary(metrics, "random_benchmark"),
                "timing": timing_summary(metrics),
            }
        )

    summary = {
        "model": "structured_multi_head_ppo_start_spec_curriculum_sweep",
        "stages": stage_rows,
        "timesteps": args.timesteps,
        "n_envs": args.n_envs,
        "rollout_steps": args.rollout_steps,
        "epochs": args.epochs,
        "minibatch_size": args.minibatch_size,
        "max_episode_steps": args.max_episode_steps,
        "eval_seeds": args.eval_seeds,
        "draw_order_variant": args.draw_order_variant,
        "reward_mode": args.reward_mode,
        "device": args.device,
        "bc_dataset": str(args.bc_dataset) if args.bc_dataset else None,
        "bc_warmup_epochs": args.bc_warmup_epochs,
        "bc_batch_size": args.bc_batch_size,
        "bc_max_samples": args.bc_max_samples,
        "bc_only": bool(args.bc_only),
        "reward": {
            "victory_reward": args.victory_reward,
            "defeat_reward": args.defeat_reward,
            "hp_loss_scale": args.hp_loss_scale,
            "enemy_hp_delta_scale": args.enemy_hp_delta_scale,
            "kill_bonus_scale": args.kill_bonus_scale,
            "catastrophe_unblocked_threshold": args.catastrophe_unblocked_threshold,
            "catastrophe_penalty": args.catastrophe_penalty,
            "next_enemy_window_relief_scale": args.next_enemy_window_relief_scale,
            "persistent_attack_script_relief_scale": args.persistent_attack_script_relief_scale,
        },
    }
    summary_path = dataset_dir / f"{args.output_prefix}_summary.json"
    if not args.dry_run:
        write_json(summary_path, summary)
    print(json.dumps(summary, indent=2, ensure_ascii=False), flush=True)
    if not args.dry_run:
        print(f"wrote curriculum summary to {summary_path}", flush=True)


if __name__ == "__main__":
    main()
