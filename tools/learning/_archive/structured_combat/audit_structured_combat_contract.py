#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from structured_combat_env import StructuredGymCombatEnv, discover_spec_paths

REQUIRED_OBS_KEYS = {
    "global_numeric",
    "player_power_ids",
    "player_power_numeric",
    "player_power_mask",
    "hand_ids",
    "hand_numeric",
    "hand_mask",
    "potion_ids",
    "potion_numeric",
    "potion_mask",
    "monster_ids",
    "monster_numeric",
    "monster_mask",
    "monster_power_ids",
    "monster_power_numeric",
    "monster_power_mask",
    "choice_kind_id",
    "choice_card_ids",
    "choice_numeric",
    "choice_mask",
    "action_type_mask",
    "play_card_mask",
    "play_card_target_mask",
    "use_potion_mask",
    "use_potion_target_mask",
    "choice_option_mask",
}


def audit_spec(
    spec_path: Path,
    *,
    driver_binary: Path | None,
    step_limit: int,
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
        obs, info = env.reset(options={"spec_path": spec_path, "seed_hint": seed})
        missing_keys = sorted(REQUIRED_OBS_KEYS - set(obs.keys()))
        if missing_keys:
            return {"spec_name": spec_path.stem, "status": "fail", "error": f"missing obs keys: {missing_keys}"}
        payload = (env._last_response or {}).get("payload") or {}
        legal_candidates = [
            candidate
            for candidate, legal in zip(payload.get("action_candidates") or [], payload.get("action_mask") or [])
            if legal
        ]
        for candidate in legal_candidates:
            canonical = env.candidate_to_canonical(candidate)
            decoded_index, invalid_action, decoder_failure = env.decode_canonical_action(canonical)
            if invalid_action or decoder_failure:
                return {
                    "spec_name": spec_path.stem,
                    "status": "fail",
                    "error": f"decoder rejected canonical action for candidate {candidate}",
                }
            if decoded_index != int(candidate.get("index")):
                return {
                    "spec_name": spec_path.stem,
                    "status": "fail",
                    "error": f"decoder roundtrip mismatch: expected {candidate.get('index')} got {decoded_index}",
                }
        steps = 0
        decoder_failures = 0
        invalid_actions = 0
        terminated = False
        truncated = False
        while steps < step_limit and not terminated and not truncated:
            action = env.sample_random_legal_action()
            _, _, terminated, truncated, step_info = env.step(action)
            decoder_failures += 1 if step_info.get("decoder_failure") else 0
            invalid_actions += 1 if step_info.get("invalid_action") else 0
            steps += 1
        return {
            "spec_name": spec_path.stem,
            "status": "pass",
            "steps": steps,
            "decoder_failures": decoder_failures,
            "invalid_actions": invalid_actions,
            "pending_choice_seen": bool(info.get("pending_choice_kind")) or bool(
                any(
                    (row.get("action_family") or "") in {"use_potion"}
                    for row in (payload.get("action_candidates") or [])
                )
            ),
        }
    except Exception as exc:
        return {"spec_name": spec_path.stem, "status": "fail", "error": str(exc)}
    finally:
        env.close()


def main() -> None:
    parser = argparse.ArgumentParser(description="Audit structured combat env contract and decoder roundtrip.")
    parser.add_argument("--spec-dir", type=Path, default=REPO_ROOT / "data" / "combat_lab" / "specs")
    parser.add_argument("--driver-binary", type=Path, default=None)
    parser.add_argument("--step-limit", type=int, default=8)
    parser.add_argument("--max-episode-steps", type=int, default=32)
    parser.add_argument("--seed", type=int, default=7)
    parser.add_argument("--summary-out", type=Path, default=None)
    parser.add_argument("--rows-out", type=Path, default=None)
    args = parser.parse_args()

    rows = [
        audit_spec(
            spec_path,
            driver_binary=args.driver_binary,
            step_limit=args.step_limit,
            max_episode_steps=args.max_episode_steps,
            seed=args.seed,
        )
        for spec_path in discover_spec_paths(args.spec_dir)
    ]
    failures = [row for row in rows if row.get("status") != "pass"]
    summary = {
        "spec_count": len(rows),
        "pass_count": len(rows) - len(failures),
        "fail_count": len(failures),
        "step_limit": int(args.step_limit),
        "max_episode_steps": int(args.max_episode_steps),
        "seed": int(args.seed),
        "failures": failures,
    }

    dataset_dir = REPO_ROOT / "tools" / "artifacts" / "learning_dataset"
    summary_out = args.summary_out or (dataset_dir / "structured_combat_contract_audit_summary.json")
    rows_out = args.rows_out or (dataset_dir / "structured_combat_contract_audit_rows.jsonl")
    write_json(summary_out, summary)
    write_jsonl(rows_out, rows)
    print(json.dumps(summary, indent=2, ensure_ascii=False))
    print(f"wrote summary to {summary_out}")


if __name__ == "__main__":
    main()
