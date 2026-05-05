#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import random
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from gym_combat_env import GymCombatEnv, discover_spec_paths


def choose_action(mask: list[bool], policy: str, rng: random.Random) -> int:
    legal = [idx for idx, allowed in enumerate(mask) if allowed]
    if not legal:
        raise RuntimeError("no legal actions available")
    if policy == "random_legal":
        return rng.choice(legal)
    return legal[0]


def audit_spec(
    spec_path: Path,
    spec_source: str,
    step_limit: int,
    max_episode_steps: int,
    seed_hint: int,
    policy: str,
    rng: random.Random,
) -> dict[str, Any]:
    env = GymCombatEnv(
        spec_paths=[spec_path],
        spec_source=spec_source,
        max_episode_steps=max_episode_steps,
        seed=0,
    )
    row: dict[str, Any] = {
        "spec_name": spec_path.stem,
        "spec_path": str(spec_path),
        "spec_source": spec_source,
        "seed_hint": int(seed_hint),
        "policy": policy,
    }
    try:
        _, info = env.reset(options={"spec_path": str(spec_path), "seed_hint": int(seed_hint)})
        row["reset_ok"] = True
        row["initial_turn_count"] = info.get("turn_count")
        row["initial_player_hp"] = info.get("player_hp")
        row["initial_visible_incoming"] = info.get("visible_incoming")
        steps = 0
        done = False
        truncated = False
        final_info = info
        while not done and not truncated and steps < step_limit:
            mask = env.action_masks().tolist()
            action_index = choose_action(mask, policy, rng)
            _, _, done, truncated, final_info = env.step(action_index)
            steps += 1
        row["step_ok"] = True
        row["steps_executed"] = steps
        row["done"] = bool(done)
        row["truncated"] = bool(truncated)
        row["outcome"] = final_info.get("outcome")
        row["final_turn_count"] = final_info.get("turn_count")
        row["final_player_hp"] = final_info.get("player_hp")
        row["final_visible_incoming"] = final_info.get("visible_incoming")
    except Exception as err:
        row["reset_ok"] = bool(row.get("reset_ok"))
        row["step_ok"] = False
        row["error"] = f"{type(err).__name__}: {err}"
    finally:
        env.close()
    return row


def main() -> int:
    parser = argparse.ArgumentParser(description="Smoke-audit combat RL specs through GymCombatEnv.")
    parser.add_argument("--spec-dir", default=REPO_ROOT / "data" / "combat_lab" / "specs", type=Path)
    parser.add_argument("--spec-source", default="author_spec", choices=["author_spec", "start_spec"])
    parser.add_argument("--policy", default="first_legal", choices=["first_legal", "random_legal"])
    parser.add_argument("--seed-hint", default=1, type=int)
    parser.add_argument("--rng-seed", default=17, type=int)
    parser.add_argument("--step-limit", default=8, type=int)
    parser.add_argument("--max-episode-steps", default=32, type=int)
    parser.add_argument("--report-out", default=None, type=Path)
    parser.add_argument("--rows-out", default=None, type=Path)
    args = parser.parse_args()

    rows_out = args.rows_out or (REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "combat_env_spec_audit_rows.jsonl")
    report_out = args.report_out or (REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "combat_env_spec_audit_summary.json")

    spec_paths = discover_spec_paths(args.spec_dir)
    if not spec_paths:
        raise SystemExit(f"no specs found under {args.spec_dir}")

    rng = random.Random(args.rng_seed)
    rows = [
        audit_spec(
            spec_path=spec_path,
            spec_source=args.spec_source,
            step_limit=args.step_limit,
            max_episode_steps=args.max_episode_steps,
            seed_hint=args.seed_hint,
            policy=args.policy,
            rng=rng,
        )
        for spec_path in spec_paths
    ]
    failures = [row for row in rows if not row.get("step_ok")]
    summary = {
        "spec_count": len(rows),
        "spec_source": args.spec_source,
        "policy": args.policy,
        "seed_hint": int(args.seed_hint),
        "rng_seed": int(args.rng_seed),
        "step_limit": int(args.step_limit),
        "max_episode_steps": int(args.max_episode_steps),
        "pass_count": len(rows) - len(failures),
        "fail_count": len(failures),
        "failed_specs": [
            {
                "spec_name": row["spec_name"],
                "error": row.get("error"),
            }
            for row in failures
        ],
    }

    write_json(report_out, summary)
    write_jsonl(rows_out, rows)
    print(json.dumps(summary, indent=2, ensure_ascii=False))
    print(f"wrote audit summary to {report_out}")
    print(f"wrote audit rows to {rows_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
