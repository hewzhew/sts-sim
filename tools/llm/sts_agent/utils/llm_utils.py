"""Shared small helpers for the LLM run controller.

This module intentionally contains low-level formatting and lookup helpers only.
It should not own controller policy, prompt construction, or simulator semantics.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any


def json_safe(value: Any) -> Any:
    if isinstance(value, Path):
        return str(value)
    if isinstance(value, dict):
        return {str(key): json_safe(item) for key, item in value.items()}
    if isinstance(value, list):
        return [json_safe(item) for item in value]
    if isinstance(value, tuple):
        return [json_safe(item) for item in value]
    if isinstance(value, set):
        return [json_safe(item) for item in sorted(value, key=str)]
    return value

def compact_json(value: Any, *, limit: int = 900) -> str:
    text = json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    if len(text) <= limit:
        return text
    return text[: limit - 3] + "..."

def map_room_label(room_type: Any, burning_elite: bool = False) -> str:
    mapping = {
        "MonsterRoom": "Monster",
        "MonsterRoomElite": "Elite",
        "MonsterRoomBoss": "Boss",
        "RestRoom": "Rest",
        "ShopRoom": "Shop",
        "TreasureRoom": "Chest",
        "EventRoom": "Event",
        "TrueVictoryRoom": "Victory",
    }
    label = mapping.get(str(room_type or ""), str(room_type or "Unknown"))
    if burning_elite:
        return f"{label} [Burning Elite]"
    return label

def map_route_context_lines(context: dict[str, Any] | None, *, max_choices: int = 6) -> list[str]:
    if not isinstance(context, dict):
        return []
    choices = context.get("route_choices") or []
    if not choices:
        return []
    lines = [
        (
            "Map route context: "
            + f"current=({context.get('current_x')},{context.get('current_y')}) "
            + f"boss={context.get('act_boss')} "
            + f"authority={context.get('decision_authority')} "
            + f"scope={context.get('map_scope')}"
        )
    ]
    warnings = context.get("truth_warnings") or []
    if warnings:
        lines.append("  warning=" + "; ".join(str(w) for w in warnings[:1]))
    starts = [
        choice
        for choice in choices
        if isinstance(choice, dict)
    ]
    if starts:
        earliest_shop = min(
            (int(choice.get("earliest_shop_floor")) for choice in starts if choice.get("earliest_shop_floor") is not None),
            default=None,
        )
        low_pressure = [
            str(choice.get("action_key") or f"x={choice.get('next_x')}")
            for choice in starts
            if ((choice.get("risk_vector") or {}).get("early_pressure") == "low")
        ]
        guaranteed_recovery = all(
            ((choice.get("risk_vector") or {}).get("recovery_access") == "guaranteed")
            for choice in starts
            if isinstance(choice.get("risk_vector"), dict)
        )
        summary = []
        if low_pressure:
            summary.append("low early pressure: " + ", ".join(low_pressure[:3]))
        if earliest_shop is not None:
            summary.append(f"earliest shop floor: {earliest_shop}")
        if guaranteed_recovery:
            summary.append("rest before boss reachable on all shown starts")
        if summary:
            lines.append("  route_summary=" + "; ".join(summary))
    for choice in choices[:max_choices]:
        if not isinstance(choice, dict):
            continue
        notes = choice.get("notes") or []
        risk_vector = choice.get("risk_vector") if isinstance(choice.get("risk_vector"), dict) else {}
        lines.append(
            "  "
            + f"{choice.get('action_key')} -> x={choice.get('next_x')} y={choice.get('next_y')} "
            + f"{choice.get('room_label') or map_room_label(choice.get('room_type'), bool(choice.get('burning_elite')))}; "
            + f"paths={choice.get('reachable_paths_to_boss')} "
            + f"elites={choice.get('min_elites')}-{choice.get('max_elites')} "
            + f"fires={choice.get('min_fires')}-{choice.get('max_fires')} "
            + f"shops={choice.get('min_shops')}-{choice.get('max_shops')} "
            + f"forced_fights_next_3={choice.get('forced_fights_next_3')} "
            + f"first_shop={choice.get('earliest_shop_floor')} "
            + f"first_fire={choice.get('earliest_fire_floor')} "
            + f"local_flex={choice.get('local_flex')} "
            + f"global_flex={choice.get('global_path_flex')} "
            + "risk="
            + (
                f"early:{risk_vector.get('early_pressure')} "
                f"elite:{risk_vector.get('elite_ceiling')} "
                f"shop:{risk_vector.get('shop_access')} "
                f"recovery:{risk_vector.get('recovery_access')} "
                f"boss_prep:{risk_vector.get('boss_prep_support')}"
                if risk_vector
                else str(choice.get("risk_label"))
            )
        )
        if notes:
            lines.append("    notes=" + "; ".join(str(note) for note in notes[:2]))
    return lines

def short_action_label(candidate: dict[str, Any]) -> str:
    key = str(candidate.get("action_key") or "")
    if key == "combat/end_turn":
        return "End turn"
    match = re.match(r"combat/play_card/card:([^/]+)/hand:([^/]+)/target:(.+)", key)
    if match:
        card, hand, target = match.groups()
        return f"Play {card} h{hand} -> {target}"
    match = re.match(r"event/choice/(\d+)", key)
    if match:
        return f"Event choice {match.group(1)}"
    match = re.match(r"map/choose/x:([^/]+)/y:(.+)", key)
    if match:
        return f"Map node x={match.group(1)} y={match.group(2)}"
    match = re.match(r"reward/card/([^/]+)", key)
    if match:
        return f"Take card {match.group(1)}"
    if key == "reward/proceed":
        return "Proceed"
    if key.startswith("campfire/"):
        return key.replace("campfire/", "Campfire ")
    if key.startswith("shop/"):
        return key.replace("shop/", "Shop ")
    return key or str((candidate.get("payload") or {}).get("action") or "unknown")

def find_candidate(candidates: list[dict[str, Any]], action_id: int) -> dict[str, Any] | None:
    for candidate in candidates:
        try:
            candidate_id = int(candidate.get("id"))
        except (TypeError, ValueError):
            continue
        if candidate_id == action_id:
            return candidate
    return None
