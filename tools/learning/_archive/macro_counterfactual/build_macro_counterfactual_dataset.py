#!/usr/bin/env python3
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any

import torch

from combat_rl_common import REPO_ROOT, iter_jsonl, write_json, write_jsonl
from structured_combat_env import StructuredGymCombatEnv
from view_structured_policy_trace import analyze_policy_step, load_model

PROBE_LIBRARY = {
    "jaw_worm": {"name": "jaw_worm", "encounter_id": "JawWorm", "room_type": "MonsterRoom"},
    "gremlin_nob": {"name": "gremlin_nob", "encounter_id": "GremlinNob", "room_type": "MonsterRoomElite"},
    "lagavulin": {"name": "lagavulin", "encounter_id": "Lagavulin", "room_type": "MonsterRoomElite"},
    "three_sentries": {"name": "three_sentries", "encounter_id": "ThreeSentries", "room_type": "MonsterRoomElite"},
    "guardian": {"name": "guardian", "encounter_id": "TheGuardian", "room_type": "MonsterRoomBoss"},
    "hexaghost": {"name": "hexaghost", "encounter_id": "Hexaghost", "room_type": "MonsterRoomBoss"},
    "slime_boss": {"name": "slime_boss", "encounter_id": "SlimeBoss", "room_type": "MonsterRoomBoss"},
    "book_of_stabbing": {"name": "book_of_stabbing", "encounter_id": "BookOfStabbing", "room_type": "MonsterRoomElite"},
    "collector": {"name": "collector", "encounter_id": "Collector", "room_type": "MonsterRoomBoss"},
    "the_champ": {"name": "the_champ", "encounter_id": "TheChamp", "room_type": "MonsterRoomBoss"},
    "automaton": {"name": "automaton", "encounter_id": "Automaton", "room_type": "MonsterRoomBoss"},
    "awakened_one": {"name": "awakened_one", "encounter_id": "AwakenedOne", "room_type": "MonsterRoomBoss"},
    "time_eater": {"name": "time_eater", "encounter_id": "TimeEater", "room_type": "MonsterRoomBoss"},
    "donu_and_deca": {"name": "donu_and_deca", "encounter_id": "DonuAndDeca", "room_type": "MonsterRoomBoss"},
    "shield_and_spear": {"name": "shield_and_spear", "encounter_id": "ShieldAndSpear", "room_type": "MonsterRoomBoss"},
    "the_heart": {"name": "the_heart", "encounter_id": "TheHeart", "room_type": "MonsterRoomBoss"},
}
ACT_BOSS_PROBE_BY_ID = {
    "slimeboss": "slime_boss",
    "theguardian": "guardian",
    "hexaghost": "hexaghost",
    "collector": "collector",
    "thechamp": "the_champ",
    "automaton": "automaton",
    "bronzeautomaton": "automaton",
    "awakenedone": "awakened_one",
    "timeeater": "time_eater",
    "donuanddeca": "donu_and_deca",
}
DIRECT_DECISION_SCREENS = {"CARD_REWARD", "SHOP_SCREEN", "REST"}
SUPPORTED_CLASS = "IRONCLAD"
COMPILED_SCHEMA_PATH = REPO_ROOT / "tools" / "compiled_protocol_schema.json"
DROP_ONLY_RELIC_SCOPES = {"non_combat", "card_reward", "ui"}
DROP_ONLY_POTION_SCOPES = {"ui"}
DEFAULT_FUTURE_WINDOW_FLOORS = 5


def _safe_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, sort_keys=True)


def _slugify(text: str) -> str:
    chars: list[str] = []
    for char in text.lower():
        chars.append(char if char.isalnum() else "-")
    return "".join(chars).strip("-") or "item"


def _stable_int(text: str) -> int:
    digest = hashlib.md5(text.encode("utf-8")).hexdigest()
    return int(digest[:8], 16)


def _normalize_java_identifier(text: str) -> str:
    return "".join(char.lower() for char in str(text or "") if char.isalnum())


def _probe_defs(names: list[str]) -> list[dict[str, Any]]:
    probes: list[dict[str, Any]] = []
    seen: set[str] = set()
    for name in names:
        if name in seen:
            continue
        probe = PROBE_LIBRARY.get(name)
        if probe is None:
            raise KeyError(f"unknown probe '{name}'")
        probes.append(copy.deepcopy(probe))
        seen.add(name)
    return probes


def _all_keys_ready(state: dict[str, Any]) -> bool:
    keys = dict(state.get("keys") or {})
    return bool(keys.get("ruby")) and bool(keys.get("emerald")) and bool(keys.get("sapphire"))


def probe_profile_for_decision(decision: dict[str, Any]) -> tuple[str, list[dict[str, Any]]]:
    state = dict(decision.get("state") or {})
    future = dict(decision.get("future_window") or {})
    act = int(state.get("act") or 0)
    floor = int(state.get("floor") or 0)
    act_boss = ACT_BOSS_PROBE_BY_ID.get(_normalize_java_identifier(state.get("act_boss") or ""))
    act_four_ready = _all_keys_ready(state)
    future_elite_count = int(future.get("future_elite_count") or 0)
    future_shop_count = int(future.get("future_shop_count") or 0)
    future_rest_count = int(future.get("future_rest_count") or 0)
    floors_to_boss = int(future.get("floors_to_boss") or 999)
    survival_to_boss = bool(future.get("survival_to_boss"))
    next_act_reached = bool(future.get("next_act_reached"))
    heart_path_ready = bool(future.get("heart_path_ready")) or act_four_ready

    if act >= 4:
        return "act4_finale", _probe_defs(["shield_and_spear", "the_heart"])

    if act <= 1:
        probe_names = ["jaw_worm", "gremlin_nob", "lagavulin", "three_sentries"]
        if act_boss:
            probe_names.append(act_boss)
            profile = "act1_boss_conditioned"
        else:
            probe_names.extend(["guardian", "hexaghost", "slime_boss"])
            profile = "act1_general"
        if floor >= 13 or floors_to_boss <= 3 or survival_to_boss:
            probe_names = [name for name in probe_names if name != "jaw_worm"]
            profile += "_late"
        if next_act_reached:
            probe_names.append("book_of_stabbing")
            profile += "_next_act"
        if future_elite_count == 0 and future_shop_count > 0 and future_rest_count > 0:
            probe_names = [name for name in probe_names if name not in {"gremlin_nob", "lagavulin", "three_sentries"}]
            profile += "_low_elite"
        return profile, _probe_defs(probe_names)

    if act == 2:
        probe_names = ["book_of_stabbing"]
        if act_boss in {"collector", "the_champ", "automaton"}:
            probe_names.append(act_boss)
            profile = "act2_boss_conditioned"
        else:
            probe_names.extend(["collector", "the_champ", "automaton"])
            profile = "act2_general"
        if floor >= 28 or floors_to_boss <= 4 or survival_to_boss:
            profile += "_late"
        if future_elite_count == 0 and future_rest_count > 0:
            probe_names = [name for name in probe_names if name != "book_of_stabbing"]
            profile += "_recover"
        return profile, _probe_defs(probe_names)

    probe_names = ["awakened_one", "time_eater", "donu_and_deca"]
    profile = "act3_general"
    if act_boss in {"awakened_one", "time_eater", "donu_and_deca"}:
        probe_names = [act_boss]
        profile = "act3_boss_conditioned"
    if heart_path_ready and (floor >= 45 or next_act_reached):
        probe_names.extend(["shield_and_spear", "the_heart"])
        profile += "_heart_path"
    return profile, _probe_defs(probe_names)


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def load_jsonl_rows(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [row for _, row in iter_jsonl(path)]


def load_json_if_exists(path: Path) -> dict[str, Any] | None:
    if not path.exists():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def canonical_player_class(raw: Any) -> str | None:
    text = str(raw or "").strip().upper()
    if text == "IRONCLAD":
        return "Ironclad"
    if text == "SILENT":
        return "Silent"
    if text == "DEFECT":
        return "Defect"
    if text == "WATCHER":
        return "Watcher"
    return None


def raw_record_map(raw_rows: list[dict[str, Any]]) -> dict[int, dict[str, Any]]:
    mapping: dict[int, dict[str, Any]] = {}
    for row in raw_rows:
        response_id = (row.get("protocol_meta") or {}).get("response_id")
        if response_id is not None:
            mapping[int(response_id)] = row
    return mapping


def valid_noncombat_record(record: dict[str, Any]) -> bool:
    game_state = record.get("game_state") or {}
    return (
        canonical_player_class(game_state.get("class")) == "Ironclad"
        and str(game_state.get("screen_type") or "") in DIRECT_DECISION_SCREENS
    )


def usable_potions(game_state: dict[str, Any]) -> list[str]:
    result: list[str] = []
    for potion in game_state.get("potions") or []:
        potion_id = str(potion.get("id") or potion.get("name") or "")
        if _normalize_java_identifier(potion_id) == "potionslot":
            continue
        result.append(potion_id)
    return result


def relic_specs(game_state: dict[str, Any]) -> list[str]:
    specs: list[str] = []
    for relic in game_state.get("relics") or []:
        specs.append(str(relic.get("id") or relic.get("name") or ""))
    return specs


def master_deck_specs(game_state: dict[str, Any]) -> list[dict[str, Any]]:
    deck_specs: list[dict[str, Any]] = []
    for card in game_state.get("deck") or []:
        deck_specs.append(
            {
                "id": str(card.get("id") or card.get("name") or ""),
                "upgrades": int(card.get("upgrades") or 0),
            }
        )
    return deck_specs


def base_macro_state(record: dict[str, Any]) -> dict[str, Any] | None:
    game_state = record.get("game_state")
    if not isinstance(game_state, dict):
        return None
    player_class = canonical_player_class(game_state.get("class"))
    if player_class != "Ironclad":
        return None
    return {
        "player_class": player_class,
        "ascension_level": int(game_state.get("ascension_level") or 0),
        "player_current_hp": int(game_state.get("current_hp") or 0),
        "player_max_hp": int(game_state.get("max_hp") or 0),
        "gold": int(game_state.get("gold") or 0),
        "keys": dict(game_state.get("keys") or {}),
        "act": int(game_state.get("act") or 0),
        "floor": int(game_state.get("floor") or 0),
        "seed": int(game_state.get("seed") or 0),
        "act_boss": str(game_state.get("act_boss") or ""),
        "room_type": str(game_state.get("room_type") or ""),
        "screen_type": str(game_state.get("screen_type") or ""),
        "choice_list": list(game_state.get("choice_list") or []),
        "screen_state": copy.deepcopy(game_state.get("screen_state") or {}),
        "master_deck": master_deck_specs(game_state),
        "deck_state": copy.deepcopy(game_state.get("deck") or []),
        "relics": relic_specs(game_state),
        "relic_state": copy.deepcopy(game_state.get("relics") or []),
        "potions": usable_potions(game_state),
        "potion_state": copy.deepcopy(game_state.get("potions") or []),
    }


def macro_state_context(state: dict[str, Any]) -> dict[str, Any]:
    deck_state = list(state.get("deck_state") or [])
    keys = dict(state.get("keys") or {})
    deck_card_counts = Counter(str(card.get("id") or card.get("name") or "") for card in deck_state)
    type_counts = Counter(str(card.get("type") or "").upper() for card in deck_state)
    upgraded_count = sum(1 for card in deck_state if int(card.get("upgrades") or 0) > 0)
    upgradable_count = sum(
        1
        for card in deck_state
        if str(card.get("type") or "").upper() in {"ATTACK", "SKILL", "POWER"} and int(card.get("upgrades") or 0) == 0
    )
    current_hp = int(state.get("player_current_hp") or 0)
    max_hp = int(state.get("player_max_hp") or 0)
    return {
        "floor": int(state.get("floor") or 0),
        "act": int(state.get("act") or 0),
        "gold": int(state.get("gold") or 0),
        "player_current_hp": current_hp,
        "player_max_hp": max_hp,
        "player_missing_hp": max(max_hp - current_hp, 0),
        "player_hp_pct": float(current_hp / max(max_hp, 1)),
        "deck_size": len(deck_state),
        "deck_upgraded_count": upgraded_count,
        "deck_upgradable_count": upgradable_count,
        "attack_count": int(type_counts.get("ATTACK", 0)),
        "skill_count": int(type_counts.get("SKILL", 0)),
        "power_count": int(type_counts.get("POWER", 0)),
        "curse_count": int(type_counts.get("CURSE", 0)),
        "status_count": int(type_counts.get("STATUS", 0)),
        "relic_count": len(state.get("relics") or []),
        "potion_count": len(state.get("potions") or []),
        "has_ruby_key": int(bool(keys.get("ruby"))),
        "has_emerald_key": int(bool(keys.get("emerald"))),
        "has_sapphire_key": int(bool(keys.get("sapphire"))),
        "relic_ids": sorted(str(value) for value in (state.get("relics") or [])),
        "deck_card_counts": dict(sorted(deck_card_counts.items())),
    }


def room_summary(record: dict[str, Any]) -> dict[str, Any] | None:
    game_state = record.get("game_state")
    if not isinstance(game_state, dict):
        return None
    protocol_meta = record.get("protocol_meta") or {}
    response_id = protocol_meta.get("response_id")
    return {
        "response_id": int(response_id) if response_id is not None else None,
        "floor": int(game_state.get("floor") or 0),
        "act": int(game_state.get("act") or 0),
        "room_type": str(game_state.get("room_type") or ""),
        "screen_type": str(game_state.get("screen_type") or ""),
        "current_hp": int(game_state.get("current_hp") or 0),
        "max_hp": int(game_state.get("max_hp") or 0),
        "gold": int(game_state.get("gold") or 0),
        "deck_size": len(game_state.get("deck") or []),
        "act_boss": str(game_state.get("act_boss") or ""),
        "keys": dict(game_state.get("keys") or {}),
    }


def future_window_context(
    summary_rows: list[dict[str, Any]],
    *,
    current_response_id: int | None,
    current_floor: int,
    current_act: int,
    horizon_floors: int,
) -> dict[str, Any]:
    if current_response_id is None:
        return {
            "future_window_floors": int(horizon_floors),
            "future_room_type_counts": {},
            "future_screen_type_counts": {},
            "future_elite_count": 0,
            "future_rest_count": 0,
            "future_shop_count": 0,
            "future_reward_count": 0,
            "survival_to_boss": False,
            "next_act_reached": False,
            "future_window_end_floor": current_floor,
            "future_window_end_act": current_act,
            "floors_to_boss": max({1: 16, 2: 33, 3: 50}.get(current_act, current_floor) - current_floor, 0),
            "heart_path_ready": False,
        }
    same_act_future = [
        row
        for row in summary_rows
        if int(row.get("response_id") or 0) > current_response_id
        and int(row.get("act") or 0) == current_act
        and int(row.get("floor") or 0) <= current_floor + horizon_floors
    ]
    all_future = [
        row
        for row in summary_rows
        if int(row.get("response_id") or 0) > current_response_id
        and int(row.get("floor") or 0) <= current_floor + horizon_floors
    ]
    target = same_act_future[-1] if same_act_future else (all_future[-1] if all_future else None)
    room_counts = Counter(str(row.get("room_type") or "") for row in same_act_future if row.get("room_type"))
    screen_counts = Counter(str(row.get("screen_type") or "") for row in same_act_future if row.get("screen_type"))
    act_boss_floor = {1: 16, 2: 33, 3: 50}.get(current_act, current_floor + horizon_floors + 1)
    survival_to_boss = any(int(row.get("floor") or 0) >= act_boss_floor for row in all_future)
    next_act_reached = any(int(row.get("act") or 0) > current_act for row in all_future)
    last_known = summary_rows[-1] if summary_rows else {}
    keys = dict(last_known.get("keys") or {})
    heart_path_ready = bool(keys.get("ruby")) and bool(keys.get("emerald")) and bool(keys.get("sapphire"))
    return {
        "future_window_floors": int(horizon_floors),
        "future_room_type_counts": dict(room_counts),
        "future_screen_type_counts": dict(screen_counts),
        "future_elite_count": sum(1 for row in same_act_future if "Elite" in str(row.get("room_type") or "")),
        "future_rest_count": sum(1 for row in same_act_future if str(row.get("room_type") or "") == "RestRoom"),
        "future_shop_count": sum(1 for row in same_act_future if str(row.get("room_type") or "") == "ShopRoom"),
        "future_reward_count": sum(1 for row in same_act_future if str(row.get("screen_type") or "") == "CARD_REWARD"),
        "survival_to_boss": survival_to_boss,
        "next_act_reached": next_act_reached,
        "future_window_end_floor": int((target or {}).get("floor") or current_floor),
        "future_window_end_act": int((target or {}).get("act") or current_act),
        "floors_to_boss": max(act_boss_floor - current_floor, 0),
        "heart_path_ready": heart_path_ready,
    }


def choice_label(choice_list: list[str], index: int) -> str | None:
    return choice_list[index] if 0 <= index < len(choice_list) else None


def chosen_shop_action(prev_record: dict[str, Any], curr_record: dict[str, Any]) -> dict[str, Any]:
    protocol_meta = curr_record.get("protocol_meta") or {}
    last_command = str(protocol_meta.get("last_command") or "")
    choice_list = list((prev_record.get("game_state") or {}).get("choice_list") or [])
    if last_command.startswith("LEAVE"):
        return {"kind": "leave", "label": None}
    if not last_command.startswith("CHOOSE "):
        return {"kind": "unknown", "label": None}
    try:
        index = int(last_command.split()[1])
    except (IndexError, ValueError):
        return {"kind": "unknown", "label": None}
    label = choice_label(choice_list, index)
    if label == "purge":
        return {"kind": "purge", "label": label}
    screen_state = ((prev_record.get("game_state") or {}).get("screen_state") or {})
    for key, kind in (("cards", "buy_card"), ("relics", "buy_relic"), ("potions", "buy_potion")):
        for item in screen_state.get(key) or []:
            name = str(item.get("name") or "").lower()
            item_id = str(item.get("id") or "").lower()
            if label and (label.lower() == name or label.lower() == item_id):
                return {"kind": kind, "label": str(item.get("name") or item.get("id") or label)}
    return {"kind": "choose", "label": label}


def chosen_campfire_action(prev_record: dict[str, Any], curr_record: dict[str, Any]) -> dict[str, Any]:
    protocol_meta = curr_record.get("protocol_meta") or {}
    last_command = str(protocol_meta.get("last_command") or "")
    choice_list = list((prev_record.get("game_state") or {}).get("choice_list") or [])
    if last_command.startswith("CHOOSE "):
        try:
            index = int(last_command.split()[1])
        except (IndexError, ValueError):
            return {"kind": "unknown", "label": None}
        label = str(choice_label(choice_list, index) or "")
        return {"kind": label or "unknown", "label": label or None}
    if last_command.startswith("REST"):
        return {"kind": "rest", "label": "rest"}
    if last_command.startswith("SMITH"):
        return {"kind": "smith", "label": "smith"}
    if last_command.startswith("RECALL"):
        return {"kind": "recall", "label": "recall"}
    return {"kind": "unknown", "label": None}


def remove_deck_card(state: dict[str, Any], deck_uuid: str) -> dict[str, Any]:
    next_state = copy.deepcopy(state)
    filtered_state = []
    filtered_specs = []
    removed_card: dict[str, Any] | None = None
    for raw_card, spec in zip(state.get("deck_state") or [], state.get("master_deck") or [], strict=False):
        if removed_card is None and str(raw_card.get("uuid") or "") == deck_uuid:
            removed_card = dict(raw_card)
            continue
        filtered_state.append(copy.deepcopy(raw_card))
        filtered_specs.append(copy.deepcopy(spec))
    next_state["deck_state"] = filtered_state
    next_state["master_deck"] = filtered_specs
    next_state["removed_card"] = removed_card
    return next_state


def add_deck_card(state: dict[str, Any], card_id: str, upgrades: int, card_name: str) -> dict[str, Any]:
    next_state = copy.deepcopy(state)
    next_state["master_deck"].append({"id": card_id, "upgrades": int(upgrades)})
    next_state["deck_state"].append(
        {
            "uuid": f"synthetic-{len(next_state['deck_state']) + 1}",
            "id": card_id,
            "name": card_name,
            "upgrades": int(upgrades),
            "type": "",
        }
    )
    return next_state


def upgrade_deck_card(state: dict[str, Any], deck_uuid: str) -> dict[str, Any]:
    next_state = copy.deepcopy(state)
    for raw_card, spec in zip(next_state.get("deck_state") or [], next_state.get("master_deck") or [], strict=False):
        if str(raw_card.get("uuid") or "") == deck_uuid:
            raw_card["upgrades"] = int(raw_card.get("upgrades") or 0) + 1
            spec["upgrades"] = int(spec.get("upgrades") or 0) + 1
            next_state["upgraded_card"] = dict(raw_card)
            break
    return next_state


def add_relic(state: dict[str, Any], relic_name: str) -> dict[str, Any]:
    next_state = copy.deepcopy(state)
    next_state["relics"].append(relic_name)
    return next_state


def add_potion(state: dict[str, Any], potion_name: str) -> dict[str, Any]:
    next_state = copy.deepcopy(state)
    if len(next_state["potions"]) < 3:
        next_state["potions"].append(potion_name)
    return next_state


def apply_rest(state: dict[str, Any]) -> dict[str, Any]:
    next_state = copy.deepcopy(state)
    heal_pct = 0.25 if int(state.get("ascension_level") or 0) >= 14 else 0.3
    heal = int(int(state.get("player_max_hp") or 0) * heal_pct)
    relic_ids = {_normalize_java_identifier(value) for value in (state.get("relics") or [])}
    if "regalpillow" in relic_ids:
        heal += 15
    if "markofthebloom" in relic_ids:
        heal = 0
    next_state["player_current_hp"] = min(
        int(state.get("player_max_hp") or 0),
        int(state.get("player_current_hp") or 0) + heal,
    )
    next_state["rest_heal"] = heal
    return next_state


def shop_buyable_options(state: dict[str, Any]) -> list[dict[str, Any]]:
    screen_state = state.get("screen_state") or {}
    gold = int(state.get("gold") or 0)
    options: list[dict[str, Any]] = [{"option_id": "shop::leave", "option_kind": "shop_leave", "label": "Leave"}]
    for item in screen_state.get("cards") or []:
        if not item.get("can_buy"):
            continue
        price = int(item.get("price") or 0)
        if price > gold:
            continue
        label = str(item.get("name") or item.get("id") or "card")
        options.append(
            {
                "option_id": f"shop::buy_card::{_slugify(label)}",
                "option_kind": "shop_buy_card",
                "label": f"Buy Card {label}",
                "card_id": str(item.get("id") or label),
                "card_name": label,
                "upgrades": int(item.get("upgrades") or 0),
                "price": price,
            }
        )
    for item in screen_state.get("relics") or []:
        if not item.get("can_buy"):
            continue
        price = int(item.get("price") or 0)
        if price > gold:
            continue
        label = str(item.get("name") or item.get("id") or "relic")
        options.append(
            {
                "option_id": f"shop::buy_relic::{_slugify(label)}",
                "option_kind": "shop_buy_relic",
                "label": f"Buy Relic {label}",
                "relic_id": str(item.get("id") or label),
                "relic_name": label,
                "price": price,
            }
        )
    for item in screen_state.get("potions") or []:
        if not item.get("can_buy"):
            continue
        price = int(item.get("price") or 0)
        if price > gold or len(state.get("potions") or []) >= 3:
            continue
        label = str(item.get("name") or item.get("id") or "potion")
        options.append(
            {
                "option_id": f"shop::buy_potion::{_slugify(label)}",
                "option_kind": "shop_buy_potion",
                "label": f"Buy Potion {label}",
                "potion_id": str(item.get("id") or label),
                "potion_name": label,
                "price": price,
            }
        )
    if bool(screen_state.get("purge_available")) and int(screen_state.get("purge_cost") or 0) <= gold:
        purge_cost = int(screen_state.get("purge_cost") or 0)
        for card in state.get("deck_state") or []:
            uuid = str(card.get("uuid") or "")
            card_name = str(card.get("name") or card.get("id") or "card")
            options.append(
                {
                    "option_id": f"shop::purge::{_slugify(card_name)}::{_slugify(uuid)}",
                    "option_kind": "shop_purge",
                    "label": f"Purge {card_name}",
                    "deck_uuid": uuid,
                    "card_name": card_name,
                    "price": purge_cost,
                }
            )
    return options


def reward_options(state: dict[str, Any]) -> list[dict[str, Any]]:
    screen_state = state.get("screen_state") or {}
    options: list[dict[str, Any]] = []
    for item in screen_state.get("cards") or []:
        label = str(item.get("name") or item.get("id") or "card")
        options.append(
            {
                "option_id": f"reward::take::{_slugify(label)}",
                "option_kind": "reward_take_card",
                "label": f"Take {label}",
                "card_id": str(item.get("id") or label),
                "card_name": label,
                "upgrades": int(item.get("upgrades") or 0),
            }
        )
    if bool(screen_state.get("skip_available")):
        options.append({"option_id": "reward::skip", "option_kind": "reward_skip", "label": "Skip"})
    return options


def campfire_options(state: dict[str, Any]) -> list[dict[str, Any]]:
    raw_options = [str(value) for value in ((state.get("screen_state") or {}).get("rest_options") or state.get("choice_list") or [])]
    options: list[dict[str, Any]] = []
    if "rest" in raw_options:
        options.append({"option_id": "campfire::rest", "option_kind": "campfire_rest", "label": "Rest"})
    if "smith" in raw_options:
        for card in state.get("deck_state") or []:
            if str(card.get("type") or "").upper() not in {"ATTACK", "SKILL", "POWER"}:
                continue
            uuid = str(card.get("uuid") or "")
            name = str(card.get("name") or card.get("id") or "card")
            options.append(
                {
                    "option_id": f"campfire::smith::{_slugify(name)}::{_slugify(uuid)}",
                    "option_kind": "campfire_smith",
                    "label": f"Smith {name}",
                    "deck_uuid": uuid,
                    "card_name": name,
                }
            )
    if "recall" in raw_options:
        options.append({"option_id": "campfire::recall", "option_kind": "campfire_recall", "label": "Recall"})
    return options


def apply_option(state: dict[str, Any], option: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    option_kind = str(option.get("option_kind") or "")
    next_state = copy.deepcopy(state)
    effect = {
        "option_kind": option_kind,
        "deck_delta": 0,
        "gold_delta": 0,
        "hp_delta": 0,
        "relic_delta": 0,
        "potion_delta": 0,
    }
    if option_kind == "reward_take_card":
        next_state = add_deck_card(next_state, str(option.get("card_id") or ""), int(option.get("upgrades") or 0), str(option.get("card_name") or ""))
        effect["deck_delta"] = 1
    elif option_kind == "reward_skip":
        pass
    elif option_kind == "shop_leave":
        pass
    elif option_kind == "shop_buy_card":
        next_state = add_deck_card(next_state, str(option.get("card_id") or ""), int(option.get("upgrades") or 0), str(option.get("card_name") or ""))
        price = int(option.get("price") or 0)
        next_state["gold"] = int(next_state.get("gold") or 0) - price
        effect["deck_delta"] = 1
        effect["gold_delta"] = -price
    elif option_kind == "shop_buy_relic":
        next_state = add_relic(next_state, str(option.get("relic_id") or option.get("relic_name") or ""))
        price = int(option.get("price") or 0)
        next_state["gold"] = int(next_state.get("gold") or 0) - price
        effect["relic_delta"] = 1
        effect["gold_delta"] = -price
    elif option_kind == "shop_buy_potion":
        next_state = add_potion(next_state, str(option.get("potion_id") or option.get("potion_name") or ""))
        price = int(option.get("price") or 0)
        next_state["gold"] = int(next_state.get("gold") or 0) - price
        effect["potion_delta"] = 1
        effect["gold_delta"] = -price
    elif option_kind == "shop_purge":
        next_state = remove_deck_card(next_state, str(option.get("deck_uuid") or ""))
        price = int(option.get("price") or 0)
        next_state["gold"] = int(next_state.get("gold") or 0) - price
        effect["deck_delta"] = -1
        effect["gold_delta"] = -price
    elif option_kind == "campfire_rest":
        before = int(next_state.get("player_current_hp") or 0)
        next_state = apply_rest(next_state)
        effect["hp_delta"] = int(next_state.get("player_current_hp") or 0) - before
    elif option_kind == "campfire_smith":
        next_state = upgrade_deck_card(next_state, str(option.get("deck_uuid") or ""))
    elif option_kind == "campfire_recall":
        keys = dict(next_state.get("keys") or {})
        keys["ruby"] = True
        next_state["keys"] = keys
    else:
        raise ValueError(f"unsupported option_kind '{option_kind}'")
    return next_state, effect


def _compiled_enum_support(enum_name: str) -> dict[str, dict[str, Any]]:
    payload = load_json(COMPILED_SCHEMA_PATH)
    entries = (((payload.get("enums") or {}).get(enum_name) or {}).get("entries") or {})
    mapping: dict[str, dict[str, Any]] = {}
    for entry in entries.values():
        aliases = [str(alias) for alias in (entry.get("java") or []) if str(alias)]
        supported = str(entry.get("status") or "mapped") != "unsupported"
        scope = str(entry.get("scope") or "")
        for alias in aliases:
            mapping[_normalize_java_identifier(alias)] = {
                "raw": alias,
                "supported": supported,
                "scope": scope,
            }
    return mapping


def relic_java_support() -> dict[str, dict[str, Any]]:
    if not hasattr(relic_java_support, "_cache"):
        relic_java_support._cache = _compiled_enum_support("relic_id")  # type: ignore[attr-defined]
    return relic_java_support._cache  # type: ignore[attr-defined]


def potion_java_support() -> dict[str, dict[str, Any]]:
    if not hasattr(potion_java_support, "_cache"):
        potion_java_support._cache = _compiled_enum_support("potion_id")  # type: ignore[attr-defined]
    return potion_java_support._cache  # type: ignore[attr-defined]


def sanitize_relic_ids(relic_ids: list[str]) -> tuple[list[str], list[dict[str, Any]]]:
    support = relic_java_support()
    sanitized: list[str] = []
    dropped: list[dict[str, Any]] = []
    for relic_id in relic_ids:
        normalized = _normalize_java_identifier(relic_id)
        meta = support.get(normalized)
        if meta is None:
            raise ValueError(f"unknown relic id '{relic_id}' for CombatStartSpec")
        if meta["supported"]:
            sanitized.append(relic_id)
            continue
        scope = str(meta.get("scope") or "")
        if scope in DROP_ONLY_RELIC_SCOPES:
            dropped.append({"kind": "relic", "id": relic_id, "scope": scope})
            continue
        raise ValueError(f"unsupported combat-affecting relic id '{relic_id}' (scope={scope or 'unknown'})")
    return sanitized, dropped


def sanitize_potion_ids(potion_ids: list[str]) -> tuple[list[str], list[dict[str, Any]]]:
    support = potion_java_support()
    sanitized: list[str] = []
    dropped: list[dict[str, Any]] = []
    for potion_id in potion_ids:
        normalized = _normalize_java_identifier(potion_id)
        meta = support.get(normalized)
        if meta is None:
            raise ValueError(f"unknown potion id '{potion_id}' for CombatStartSpec")
        if meta["supported"]:
            sanitized.append(potion_id)
            continue
        scope = str(meta.get("scope") or "")
        if scope in DROP_ONLY_POTION_SCOPES:
            dropped.append({"kind": "potion", "id": potion_id, "scope": scope})
            continue
        raise ValueError(f"unsupported combat-affecting potion id '{potion_id}' (scope={scope or 'unknown'})")
    return sanitized, dropped


def build_start_spec(state: dict[str, Any], probe: dict[str, Any], seed: int, name: str) -> dict[str, Any]:
    relics, dropped_relics = sanitize_relic_ids(list(state.get("relics") or []))
    potions, dropped_potions = sanitize_potion_ids(list(state.get("potions") or []))
    return {
        "name": name,
        "player_class": str(state.get("player_class") or "Ironclad"),
        "ascension_level": int(state.get("ascension_level") or 0),
        "encounter_id": str(probe["encounter_id"]),
        "room_type": str(probe["room_type"]),
        "seed": int(seed),
        "player_current_hp": int(state.get("player_current_hp") or 0),
        "player_max_hp": int(state.get("player_max_hp") or 0),
        "relics": relics,
        "potions": potions,
        "master_deck": copy.deepcopy(state.get("master_deck") or []),
        "_sanitized_meta": {
            "dropped_relics": dropped_relics,
            "dropped_potions": dropped_potions,
        },
    }


def shared_probe_seeds(decision_id: str, probe_name: str, count: int) -> list[int]:
    base = _stable_int(f"{decision_id}::{probe_name}")
    return [101 + ((base + index * 7919) % 1_000_000) for index in range(count)]


def rollout_structured_bundle(
    *,
    spec_path: Path,
    seeds: list[int],
    model: Any,
    device: torch.device,
    driver_binary: Path | None,
    max_steps: int,
    top_k: int,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    env = StructuredGymCombatEnv(
        [spec_path],
        spec_source="start_spec",
        driver_binary=driver_binary,
        max_episode_steps=max_steps,
        seed=0,
    )
    episodes: list[dict[str, Any]] = []
    try:
        for seed in seeds:
            obs, info = env.reset(options={"spec_path": str(spec_path), "seed_hint": int(seed)})
            done = False
            truncated = False
            steps = 0
            reward_total = 0.0
            while not done and not truncated and steps < max_steps:
                raw_before = info.get("raw_observation") or {}
                action, _ = analyze_policy_step(
                    model,
                    env,
                    obs,
                    raw_before,
                    deterministic=True,
                    device=device,
                    top_k=top_k,
                )
                obs, reward, done, truncated, info = env.step(action)
                reward_total += float(reward)
                steps += 1
            raw_after = info.get("raw_observation") or {}
            outcome = str(info.get("outcome") or "ongoing")
            episodes.append(
                {
                    "seed": int(seed),
                    "outcome": outcome,
                    "steps": steps,
                    "reward_total": reward_total,
                    "player_hp": int(raw_after.get("player_hp") or 0),
                    "player_max_hp": int(raw_after.get("player_max_hp") or 0),
                    "done": bool(done),
                    "truncated": bool(truncated),
                }
            )
    finally:
        env.close()
    victories = sum(1 for item in episodes if item.get("outcome") == "victory")
    summary = {
        "episodes": len(episodes),
        "win_rate": float(victories / max(len(episodes), 1)),
        "mean_player_hp": float(
            sum(float(item.get("player_hp") or 0.0) for item in episodes) / max(len(episodes), 1)
        ),
        "mean_reward_total": float(
            sum(float(item.get("reward_total") or 0.0) for item in episodes) / max(len(episodes), 1)
        ),
        "mean_steps": float(
            sum(float(item.get("steps") or 0.0) for item in episodes) / max(len(episodes), 1)
        ),
    }
    return episodes, summary


def compare_probe_summary(left: dict[str, Any], right: dict[str, Any], *, tol: float = 1e-6) -> int:
    left_key = (
        float(left.get("win_rate") or 0.0),
        float(left.get("mean_player_hp") or 0.0),
        -float(left.get("mean_steps") or 0.0),
        float(left.get("mean_reward_total") or 0.0),
    )
    right_key = (
        float(right.get("win_rate") or 0.0),
        float(right.get("mean_player_hp") or 0.0),
        -float(right.get("mean_steps") or 0.0),
        float(right.get("mean_reward_total") or 0.0),
    )
    for left_value, right_value in zip(left_key, right_key, strict=False):
        if left_value > right_value + tol:
            return 1
        if right_value > left_value + tol:
            return -1
    return 0


def aggregate_probe_summary(summary: dict[str, Any]) -> float:
    return (
        float(summary.get("win_rate") or 0.0) * 100.0
        + float(summary.get("mean_player_hp") or 0.0)
        + float(summary.get("mean_reward_total") or 0.0) * 0.5
        - float(summary.get("mean_steps") or 0.0) * 0.25
    )


def aggregate_probe_bundle_score(probe_summaries: dict[str, dict[str, Any]]) -> float:
    if not probe_summaries:
        return 0.0
    return float(
        sum(aggregate_probe_summary(summary) for summary in probe_summaries.values()) / max(len(probe_summaries), 1)
    )


def pairwise_edges(option_rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    edges: list[dict[str, Any]] = []
    for index, left in enumerate(option_rows):
        for right in option_rows[index + 1 :]:
            left_votes = 0
            right_votes = 0
            per_probe: dict[str, int] = {}
            for probe_name in sorted(set(left["probe_summaries"]) & set(right["probe_summaries"])):
                cmp = compare_probe_summary(left["probe_summaries"][probe_name], right["probe_summaries"][probe_name])
                per_probe[probe_name] = cmp
                if cmp > 0:
                    left_votes += 1
                elif cmp < 0:
                    right_votes += 1
            if left_votes == right_votes:
                continue
            preferred, rejected = (left, right) if left_votes > right_votes else (right, left)
            vote_margin = abs(left_votes - right_votes)
            edges.append(
                {
                    "dataset_kind": "macro_counterfactual_pairwise",
                    "decision_id": str(left["decision_id"]),
                    "preferred_option_id": str(preferred["option_id"]),
                    "rejected_option_id": str(rejected["option_id"]),
                    "preferred_label": str(preferred["label"]),
                    "rejected_label": str(rejected["label"]),
                    "preferred_kind": str(preferred["option_kind"]),
                    "rejected_kind": str(rejected["option_kind"]),
                    "preferred_probe_votes": max(left_votes, right_votes),
                    "rejected_probe_votes": min(left_votes, right_votes),
                    "vote_margin": vote_margin,
                    "strength": float(vote_margin / max(len(per_probe), 1)),
                    "per_probe": per_probe,
                }
            )
    return edges


def reward_decisions(
    *,
    run_id: str,
    raw_by_response: dict[int, dict[str, Any]],
    reward_rows: list[dict[str, Any]],
    summary_rows: list[dict[str, Any]],
    future_window_floors: int,
) -> list[dict[str, Any]]:
    decisions: list[dict[str, Any]] = []
    for row in reward_rows:
        response_id = row.get("response_id")
        if response_id is None:
            continue
        record = raw_by_response.get(int(response_id))
        if not record or not valid_noncombat_record(record):
            continue
        game_state = record.get("game_state") or {}
        if str(game_state.get("screen_type") or "") != "CARD_REWARD":
            continue
        if str(game_state.get("room_phase") or "") != "COMPLETE":
            continue
        state = base_macro_state(record)
        if state is None:
            continue
        options = reward_options(state)
        if len(options) < 2:
            continue
        bot_choice = row.get("bot_choice") or {}
        baseline_label = None
        if int(bot_choice.get("choice_index") or -1) >= 0:
            baseline_label = choice_label(list(game_state.get("choice_list") or []), int(bot_choice.get("choice_index") or 0))
        elif bot_choice.get("kind") == "skip":
            baseline_label = "skip"
        decisions.append(
            {
                "decision_id": f"{run_id}::reward::{int(response_id)}",
                "run_id": run_id,
                "source_kind": "reward",
                "response_id": int(response_id),
                "baseline_choice_label": baseline_label,
                "baseline_choice_kind": str(bot_choice.get("kind") or ""),
                "state": state,
                "future_window": future_window_context(
                    summary_rows,
                    current_response_id=int(response_id),
                    current_floor=int(state.get("floor") or 0),
                    current_act=int(state.get("act") or 0),
                    horizon_floors=future_window_floors,
                ),
                "options": options,
                "screen_type": "CARD_REWARD",
            }
        )
    return decisions


def raw_screen_decisions(
    *,
    run_id: str,
    raw_rows: list[dict[str, Any]],
    summary_rows: list[dict[str, Any]],
    future_window_floors: int,
) -> list[dict[str, Any]]:
    decisions: list[dict[str, Any]] = []
    for prev, curr in zip(raw_rows, raw_rows[1:], strict=False):
        if not valid_noncombat_record(prev):
            continue
        game_state = prev.get("game_state") or {}
        state = base_macro_state(prev)
        if state is None:
            continue
        screen_type = str(game_state.get("screen_type") or "")
        response_id = int(((prev.get("protocol_meta") or {}).get("response_id")) or 0)
        if screen_type == "SHOP_SCREEN":
            options = shop_buyable_options(state)
            baseline = chosen_shop_action(prev, curr)
            if len(options) >= 2:
                decisions.append(
                    {
                        "decision_id": f"{run_id}::shop::{response_id}",
                        "run_id": run_id,
                        "source_kind": "shop",
                        "response_id": response_id,
                        "baseline_choice_label": baseline.get("label"),
                        "baseline_choice_kind": baseline.get("kind"),
                        "state": state,
                        "future_window": future_window_context(
                            summary_rows,
                            current_response_id=response_id,
                            current_floor=int(state.get("floor") or 0),
                            current_act=int(state.get("act") or 0),
                            horizon_floors=future_window_floors,
                        ),
                        "options": options,
                        "screen_type": screen_type,
                    }
                )
        elif screen_type == "REST":
            options = campfire_options(state)
            baseline = chosen_campfire_action(prev, curr)
            if len(options) >= 2:
                decisions.append(
                    {
                        "decision_id": f"{run_id}::campfire::{response_id}",
                        "run_id": run_id,
                        "source_kind": "campfire",
                        "response_id": response_id,
                        "baseline_choice_label": baseline.get("label"),
                        "baseline_choice_kind": baseline.get("kind"),
                        "state": state,
                        "future_window": future_window_context(
                            summary_rows,
                            current_response_id=response_id,
                            current_floor=int(state.get("floor") or 0),
                            current_act=int(state.get("act") or 0),
                            horizon_floors=future_window_floors,
                        ),
                        "options": options,
                        "screen_type": screen_type,
                    }
                )
    return decisions


def collect_run_dirs(run_glob: str) -> list[Path]:
    pattern = Path(run_glob)
    return sorted(pattern.parent.glob(pattern.name))


def collect_baseline_run_dirs(path: Path) -> list[Path]:
    baseline = load_json_if_exists(path) or {}
    run_dirs: list[Path] = []
    for run in baseline.get("selected_runs") or []:
        raw_path = Path(str(run.get("raw_path") or ""))
        if raw_path.exists():
            run_dirs.append(raw_path.parent)
    return run_dirs


def evaluate_decision(
    *,
    decision: dict[str, Any],
    temp_dir: Path,
    model: Any,
    teacher_model: Any | None,
    device: torch.device,
    driver_binary: Path | None,
    max_steps: int,
    top_k: int,
    probe_seed_count: int,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    option_rows: list[dict[str, Any]] = []
    spec_cache: dict[tuple[str, str], tuple[Path, dict[str, Any]]] = {}
    probe_profile, probes = probe_profile_for_decision(decision)
    probe_names = [str(probe["name"]) for probe in probes]
    for option in decision["options"]:
        option_state, effect = apply_option(decision["state"], option)
        probe_summaries: dict[str, dict[str, Any]] = {}
        teacher_probe_summaries: dict[str, dict[str, Any]] = {}
        probe_episode_rows: list[dict[str, Any]] = []
        probe_sanitization: dict[str, Any] = {}
        for probe in probes:
            probe_name = str(probe["name"])
            spec_key = (str(option["option_id"]), probe_name)
            cached = spec_cache.get(spec_key)
            if cached is None:
                seed_for_name = shared_probe_seeds(str(decision["decision_id"]), probe_name, 1)[0]
                spec = build_start_spec(
                    option_state,
                    probe,
                    seed_for_name,
                    name=f"macro_probe::{decision['decision_id']}::{option['option_id']}::{probe_name}",
                )
                sanitized_meta = spec.pop("_sanitized_meta", {})
                spec_path = temp_dir / f"{_slugify(decision['decision_id'])}--{_slugify(option['option_id'])}--{probe_name}.json"
                spec_path.write_text(_safe_json(spec), encoding="utf-8")
                spec_cache[spec_key] = (spec_path, sanitized_meta)
            else:
                spec_path, sanitized_meta = cached
            probe_sanitization[probe_name] = sanitized_meta
            shared_seeds = shared_probe_seeds(str(decision["decision_id"]), probe_name, probe_seed_count)
            episodes, summary = rollout_structured_bundle(
                spec_path=spec_path,
                seeds=shared_seeds,
                model=model,
                device=device,
                driver_binary=driver_binary,
                max_steps=max_steps,
                top_k=top_k,
            )
            probe_summaries[probe_name] = summary
            if teacher_model is not None:
                _, teacher_summary = rollout_structured_bundle(
                    spec_path=spec_path,
                    seeds=shared_seeds,
                    model=teacher_model,
                    device=device,
                    driver_binary=driver_binary,
                    max_steps=max_steps,
                    top_k=top_k,
                )
                teacher_probe_summaries[probe_name] = teacher_summary
            for episode in episodes:
                probe_episode_rows.append(
                    {
                        "decision_id": str(decision["decision_id"]),
                        "option_id": str(option["option_id"]),
                        "probe_name": probe_name,
                        **episode,
                    }
                )
        option_rows.append(
            {
                "dataset_kind": "macro_counterfactual_option",
                "decision_id": str(decision["decision_id"]),
                "run_id": str(decision["run_id"]),
                "source_kind": str(decision["source_kind"]),
                "response_id": int(decision["response_id"]),
                "screen_type": str(decision["screen_type"]),
                "baseline_choice_label": decision.get("baseline_choice_label"),
                "baseline_choice_kind": decision.get("baseline_choice_kind"),
                "baseline_matches_option": str(decision.get("baseline_choice_label") or "").lower()
                in str(option.get("label") or "").lower(),
                "option_id": str(option["option_id"]),
                "option_kind": str(option["option_kind"]),
                "label": str(option["label"]),
                "option_payload": option,
                "effect": effect,
                "future_window": decision.get("future_window") or {},
                "probe_profile": probe_profile,
                "probe_names": probe_names,
                "probe_sanitization": probe_sanitization,
                "state_context": {
                    **macro_state_context(decision["state"]),
                    "future_window_floors": int((decision.get("future_window") or {}).get("future_window_floors") or 0),
                    "future_elite_count": int((decision.get("future_window") or {}).get("future_elite_count") or 0),
                    "future_rest_count": int((decision.get("future_window") or {}).get("future_rest_count") or 0),
                    "future_shop_count": int((decision.get("future_window") or {}).get("future_shop_count") or 0),
                    "future_reward_count": int((decision.get("future_window") or {}).get("future_reward_count") or 0),
                    "survival_to_boss": int(bool((decision.get("future_window") or {}).get("survival_to_boss"))),
                    "next_act_reached": int(bool((decision.get("future_window") or {}).get("next_act_reached"))),
                    "future_window_end_floor": int((decision.get("future_window") or {}).get("future_window_end_floor") or 0),
                    "future_window_end_act": int((decision.get("future_window") or {}).get("future_window_end_act") or 0),
                    "floors_to_boss": int((decision.get("future_window") or {}).get("floors_to_boss") or 0),
                    "heart_path_ready": int(bool((decision.get("future_window") or {}).get("heart_path_ready"))),
                },
                "state_preview": {
                    "floor": int(decision["state"].get("floor") or 0),
                    "act": int(decision["state"].get("act") or 0),
                    "gold": int(decision["state"].get("gold") or 0),
                    "player_current_hp": int(decision["state"].get("player_current_hp") or 0),
                    "player_max_hp": int(decision["state"].get("player_max_hp") or 0),
                    "deck_size": len(decision["state"].get("master_deck") or []),
                },
                "probe_summaries": probe_summaries,
                "current_policy_score": aggregate_probe_bundle_score(probe_summaries),
                "teacher_probe_summaries": teacher_probe_summaries,
                "teacher_policy_score": aggregate_probe_bundle_score(teacher_probe_summaries) if teacher_probe_summaries else None,
                "execution_gap": (
                    aggregate_probe_bundle_score(teacher_probe_summaries) - aggregate_probe_bundle_score(probe_summaries)
                    if teacher_probe_summaries
                    else 0.0
                ),
                "executor_disagreement_tag": (
                    "teacher_prefers" if teacher_probe_summaries and aggregate_probe_bundle_score(teacher_probe_summaries) > aggregate_probe_bundle_score(probe_summaries) + 1e-6
                    else "current_prefers" if teacher_probe_summaries and aggregate_probe_bundle_score(probe_summaries) > aggregate_probe_bundle_score(teacher_probe_summaries) + 1e-6
                    else "agree_or_unavailable"
                ),
            }
        )
    return option_rows, pairwise_edges(option_rows)


def collect_decisions(run_dir: Path, *, future_window_floors: int) -> list[dict[str, Any]]:
    raw_rows = load_jsonl_rows(run_dir / "raw.jsonl")
    if not raw_rows:
        return []
    raw_by_response = raw_record_map(raw_rows)
    summary_rows = [row for row in (room_summary(record) for record in raw_rows) if row is not None]
    reward_rows = load_jsonl_rows(run_dir / "reward_audit.jsonl")
    decisions = reward_decisions(
        run_id=run_dir.name,
        raw_by_response=raw_by_response,
        reward_rows=reward_rows,
        summary_rows=summary_rows,
        future_window_floors=future_window_floors,
    )
    decisions.extend(
        raw_screen_decisions(
            run_id=run_dir.name,
            raw_rows=raw_rows,
            summary_rows=summary_rows,
            future_window_floors=future_window_floors,
        )
    )
    return decisions


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Build macro counterfactual datasets by replaying noncombat choices into combat probe baskets."
    )
    parser.add_argument("--run-glob", default=str(REPO_ROOT / "logs" / "runs" / "*"))
    parser.add_argument("--include-current", action="store_true")
    parser.add_argument(
        "--model",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "smoke_structured_metrics_v2_structured_combat_ppo_model.pt",
        type=Path,
    )
    parser.add_argument("--teacher-model", default=None, type=Path)
    parser.add_argument("--driver-binary", default=None, type=Path)
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--out-prefix", default="macro_counterfactual")
    parser.add_argument("--probe-seed-count", default=2, type=int)
    parser.add_argument("--max-steps", default=96, type=int)
    parser.add_argument("--top-k", default=4, type=int)
    parser.add_argument("--limit-decisions", default=0, type=int)
    parser.add_argument("--future-window-floors", default=DEFAULT_FUTURE_WINDOW_FLOORS, type=int)
    parser.add_argument(
        "--baseline-manifest",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_baseline.json",
        type=Path,
    )
    parser.add_argument("--include-baseline-runs", action="store_true")
    args = parser.parse_args()

    device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    model = load_model(args.model, device)
    teacher_model = load_model(args.teacher_model, device) if args.teacher_model else None

    run_dirs = collect_run_dirs(str(args.run_glob))
    if args.include_baseline_runs:
        run_dirs.extend(collect_baseline_run_dirs(args.baseline_manifest))
    if args.include_current:
        current_dir = REPO_ROOT / "logs" / "current"
        if current_dir.exists():
            run_dirs.append(current_dir)
    deduped_run_dirs: list[Path] = []
    seen_run_dirs: set[Path] = set()
    for path in run_dirs:
        if not path.exists():
            continue
        resolved = path.resolve()
        if resolved in seen_run_dirs:
            continue
        seen_run_dirs.add(resolved)
        deduped_run_dirs.append(path)
    run_dirs = deduped_run_dirs

    decisions: list[dict[str, Any]] = []
    for run_dir in run_dirs:
        decisions.extend(collect_decisions(run_dir, future_window_floors=int(args.future_window_floors)))
    decisions.sort(
        key=lambda item: (
            str(item.get("source_kind") or ""),
            str(item.get("run_id") or ""),
            int(item.get("response_id") or 0),
        )
    )
    if args.limit_decisions > 0:
        decisions = decisions[: int(args.limit_decisions)]

    option_rows: list[dict[str, Any]] = []
    edge_rows: list[dict[str, Any]] = []
    with tempfile.TemporaryDirectory(prefix="macro_counterfactual_", dir=str(args.dataset_dir)) as temp_root:
        temp_dir = Path(temp_root)
        for decision in decisions:
            decision_option_rows, decision_edge_rows = evaluate_decision(
                decision=decision,
                temp_dir=temp_dir,
                model=model,
                teacher_model=teacher_model,
                device=device,
                driver_binary=args.driver_binary,
                max_steps=int(args.max_steps),
                top_k=int(args.top_k),
                probe_seed_count=int(args.probe_seed_count),
            )
            option_rows.extend(decision_option_rows)
            edge_rows.extend(decision_edge_rows)

    probe_profile_counts = Counter(str(row.get("probe_profile") or "unknown") for row in option_rows)
    used_probe_names = sorted({probe_name for row in option_rows for probe_name in (row.get("probe_names") or [])})

    summary = {
        "dataset_kind": "macro_counterfactual",
        "run_dirs": [str(path) for path in run_dirs],
        "decision_count": len(decisions),
        "option_row_count": len(option_rows),
        "pairwise_edge_count": len(edge_rows),
        "source_kind_counts": dict(Counter(str(row.get("source_kind") or "unknown") for row in option_rows)),
        "option_kind_counts": dict(Counter(str(row.get("option_kind") or "unknown") for row in option_rows)),
        "decision_source_counts": dict(Counter(str(row.get("source_kind") or "unknown") for row in decisions)),
        "probe_names": used_probe_names,
        "probe_profile_counts": dict(probe_profile_counts),
        "probe_seed_count": int(args.probe_seed_count),
        "model_path": str(args.model),
        "teacher_model_path": str(args.teacher_model) if args.teacher_model else None,
        "future_window_floors": int(args.future_window_floors),
        "notes": [
            "macro counterfactual rows are built from logged noncombat states, not human-authored scalar rewards",
            "each option is converted into a temporary CombatStartSpec and evaluated on a shared basket of combat probes",
            "pairwise edges are derived from per-probe lexicographic combat outcomes, not a hand-written aggregate deck score",
            "probe baskets are now conditioned on act, floor, known boss, and key state instead of using one fixed global basket",
            "future-window summaries from the originating run now condition late-act / near-boss encounter bundle selection",
            "if a teacher combat model is provided, option rows also record current-vs-teacher execution gap fields",
        ],
    }

    write_jsonl(args.dataset_dir / f"{args.out_prefix}_options.jsonl", option_rows)
    write_jsonl(args.dataset_dir / f"{args.out_prefix}_pairwise.jsonl", edge_rows)
    write_json(args.dataset_dir / f"{args.out_prefix}_summary.json", summary)
    print(json.dumps(summary, ensure_ascii=False, indent=2))
    print(f"wrote macro counterfactual rows to {args.dataset_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
