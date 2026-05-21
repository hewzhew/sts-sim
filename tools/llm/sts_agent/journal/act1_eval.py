"""Act 1 boss evaluation summary helpers."""

from __future__ import annotations

import argparse
from typing import Any


def new_act1_eval_summary(args: argparse.Namespace) -> dict[str, Any]:
    return {
        "schema_name": "Act1BossEvalSummary",
        "schema_version": 1,
        "seed": args.seed,
        "ascension": args.ascension,
        "class": args.player_class,
        "provider": args.provider,
        "model": args.model if args.provider == "openai_compatible" else None,
        "run_mode": args.run_mode,
        "context_ablation": args.context_ablation,
        "target": "act1_boss_kill",
        "stop_reason": None,
        "boss_id": None,
        "boss_seen": False,
        "boss_killed": False,
        "elite_floors": [],
        "elite_count": 0,
        "death_floor": None,
        "final_floor": None,
        "final_act": None,
        "final_hp": None,
        "max_hp": None,
        "llm_calls": 0,
        "illegal_actions": 0,
        "override_count": 0,
        "steps_executed": 0,
        "timed_out": False,
        "policy_quality_claim": False,
        "label_role": "not_a_label",
    }

def update_act1_eval_from_observation(summary: dict[str, Any], payload: dict[str, Any]) -> None:
    act = payload.get("act")
    floor = payload.get("floor")
    room = payload.get("current_room")
    decision_type = payload.get("decision_type")
    summary["final_act"] = act
    summary["final_floor"] = floor
    summary["final_hp"] = payload.get("current_hp")
    summary["max_hp"] = payload.get("max_hp")
    combat = payload.get("combat") if isinstance(payload.get("combat"), dict) else {}
    monsters = combat.get("monsters") or []
    if act == 1 and room == "MonsterRoomElite" and decision_type == "combat":
        elite_floors = set(summary.get("elite_floors") or [])
        if floor is not None:
            elite_floors.add(floor)
        summary["elite_floors"] = sorted(elite_floors)
        summary["elite_count"] = len(elite_floors)
    if act == 1 and floor == 16 and room == "MonsterRoomBoss" and monsters:
        summary["boss_seen"] = True
        summary["boss_id"] = "+".join(
            str(monster.get("monster_id") or monster.get("name") or "unknown")
            for monster in monsters
            if isinstance(monster, dict)
        )
    if summary.get("boss_seen") and (
        (isinstance(act, int) and act > 1)
        or (isinstance(floor, int) and floor > 16)
        or (
            floor == 16
            and decision_type
            in {"reward", "reward_card_choice", "card_reward", "boss_reward", "map"}
        )
    ):
        summary["boss_killed"] = True
        summary["stop_reason"] = "act1_boss_killed"

def update_act1_eval_from_record(summary: dict[str, Any], record: dict[str, Any]) -> None:
    summary["steps_executed"] = int(summary.get("steps_executed") or 0) + 1
    summary["final_floor"] = (record.get("public_state_before") or {}).get("floor")
    summary["final_hp"] = (record.get("info") or {}).get("hp", record.get("post_hp"))
    if not record.get("choice_was_legal"):
        summary["illegal_actions"] = int(summary.get("illegal_actions") or 0) + 1
    if (record.get("override") or {}).get("applied"):
        summary["override_count"] = int(summary.get("override_count") or 0) + 1
    info = record.get("info") if isinstance(record.get("info"), dict) else {}
    post_result = str(info.get("result") or record.get("post_result") or "").lower()
    post_hp = info.get("hp")
    if (
        record.get("done")
        and ("defeat" in post_result or "game_over" in post_result or post_hp == 0)
    ):
        state = record.get("public_state_before") or {}
        summary["stop_reason"] = "player_death"
        summary["death_floor"] = state.get("floor")
        summary["final_floor"] = state.get("floor")
        summary["final_hp"] = post_hp
