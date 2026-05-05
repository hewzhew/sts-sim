#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from structured_combat_env import StructuredGymCombatEnv, discover_spec_paths

GLOBAL_REQUIRED_KEYS = {
    "contract_version",
    "player_powers",
    "potions",
    "monsters",
    "turn_prefix",
    "pressure",
    "belief",
}

BASE_MONSTER_MECHANIC_KEYS = {
    "planned_move_id",
    "move_history",
}

MONSTER_SPECIFIC_KEYS = {
    "Lagavulin": {"sleeping", "idle_count", "debuff_turn_count", "wake_triggered"},
    "TheGuardian": {
        "guardian_threshold",
        "guardian_damage_taken",
        "guardian_open",
        "close_up_triggered",
    },
    "SlimeBoss": {"split_threshold", "split_ready"},
    "Darkling": {"half_dead", "first_move", "regrow_counter"},
}


def required_mechanic_keys(monster: dict[str, Any]) -> set[str]:
    required = set(BASE_MONSTER_MECHANIC_KEYS)
    monster_id = str(monster.get("monster_id") or "")
    required.update(MONSTER_SPECIFIC_KEYS.get(monster_id, set()))
    power_ids = {str(power.get("id") or "") for power in (monster.get("powers") or [])}
    if "Split" in power_ids:
        required.update({"split_threshold", "split_ready"})
    if "Regrow" in power_ids:
        required.add("regrow_counter")
    return required


def audit_spec(
    spec_path: Path,
    *,
    driver_binary: Path | None,
    max_episode_steps: int,
    seed: int,
) -> dict[str, Any]:
    env = StructuredGymCombatEnv(
        [spec_path],
        driver_binary=driver_binary,
        max_episode_steps=max_episode_steps,
        seed=seed,
    )
    try:
        _, info = env.reset(options={"spec_path": spec_path, "seed_hint": seed})
        raw = info.get("raw_observation") or {}
        missing_global = sorted(GLOBAL_REQUIRED_KEYS - set(raw.keys()))
        if missing_global:
            return {
                "spec_name": spec_path.stem,
                "status": "fail",
                "error": f"missing global observation keys: {missing_global}",
            }
        if raw.get("contract_version") != "combat_rl_v0":
            return {
                "spec_name": spec_path.stem,
                "status": "fail",
                "error": f"unexpected contract_version: {raw.get('contract_version')}",
            }
        monsters = list(raw.get("monsters") or [])
        monster_failures: list[dict[str, Any]] = []
        seen_monsters = []
        for monster in monsters:
            monster_id = str(monster.get("monster_id") or "")
            seen_monsters.append(monster_id)
            mechanic_state = monster.get("mechanic_state") or {}
            missing_mechanics = sorted(required_mechanic_keys(monster) - set(mechanic_state.keys()))
            if missing_mechanics:
                monster_failures.append(
                    {
                        "monster_id": monster_id,
                        "missing_mechanic_keys": missing_mechanics,
                        "mechanic_state": mechanic_state,
                    }
                )
        if monster_failures:
            return {
                "spec_name": spec_path.stem,
                "status": "fail",
                "error": "missing required monster mechanic_state fields",
                "monsters": monster_failures,
            }
        pending_choice = raw.get("pending_choice")
        if raw.get("pending_choice_kind") and pending_choice is None:
            return {
                "spec_name": spec_path.stem,
                "status": "fail",
                "error": "pending_choice_kind present but pending_choice payload missing",
            }
        return {
            "spec_name": spec_path.stem,
            "status": "pass",
            "monster_ids": seen_monsters,
            "pending_choice_kind": raw.get("pending_choice_kind"),
            "turn_prefix_keys": sorted((raw.get("turn_prefix") or {}).keys()),
        }
    except Exception as exc:
        return {"spec_name": spec_path.stem, "status": "fail", "error": str(exc)}
    finally:
        env.close()


def main() -> None:
    parser = argparse.ArgumentParser(description="Audit structured combat observation truth coverage.")
    parser.add_argument("--spec-dir", type=Path, default=REPO_ROOT / "data" / "combat_lab" / "specs")
    parser.add_argument("--driver-binary", type=Path, default=None)
    parser.add_argument("--max-episode-steps", type=int, default=32)
    parser.add_argument("--seed", type=int, default=7)
    parser.add_argument("--summary-out", type=Path, default=None)
    parser.add_argument("--rows-out", type=Path, default=None)
    args = parser.parse_args()

    rows = [
        audit_spec(
            spec_path,
            driver_binary=args.driver_binary,
            max_episode_steps=args.max_episode_steps,
            seed=args.seed,
        )
        for spec_path in discover_spec_paths(args.spec_dir)
    ]
    failures = [row for row in rows if row.get("status") != "pass"]
    monster_counter: Counter[str] = Counter()
    for row in rows:
        for monster_id in row.get("monster_ids") or []:
            monster_counter[str(monster_id)] += 1
    summary = {
        "spec_count": len(rows),
        "pass_count": len(rows) - len(failures),
        "fail_count": len(failures),
        "seed": int(args.seed),
        "max_episode_steps": int(args.max_episode_steps),
        "monsters_seen": dict(sorted(monster_counter.items())),
        "failures": failures,
    }

    dataset_dir = REPO_ROOT / "tools" / "artifacts" / "learning_dataset"
    summary_out = args.summary_out or (dataset_dir / "structured_truth_coverage_summary.json")
    rows_out = args.rows_out or (dataset_dir / "structured_truth_coverage_rows.jsonl")
    write_json(summary_out, summary)
    write_jsonl(rows_out, rows)
    print(json.dumps(summary, indent=2, ensure_ascii=False))
    print(f"wrote summary to {summary_out}")


if __name__ == "__main__":
    main()
