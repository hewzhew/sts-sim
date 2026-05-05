from __future__ import annotations

import argparse
import copy
import json
import re
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any


DEFAULT_RAW_PATH = Path(r"D:\rust\sts_simulator\logs\current\live_comm_raw.jsonl")
DEFAULT_DEBUG_PATH = Path(r"D:\rust\sts_simulator\logs\current\live_comm_debug.txt")
DEFAULT_WATCH_AUDIT_PATH = Path(r"D:\rust\sts_simulator\logs\current\live_comm_watch_audit.jsonl")
DEFAULT_WATCH_NONCOMBAT_AUDIT_PATH = Path(
    r"D:\rust\sts_simulator\logs\current\live_comm_watch_noncombat.jsonl"
)

FRAME_RE = re.compile(r"^\[F(\d+)\]")
DIFF_RE = re.compile(
    r"^\s+(?P<field>.+?) : Rust=(?P<rust>.+), Java=(?P<java>.+?)\s+\[(?P<kind>ENGINE_BUG|CONTENT_GAP|TIMING)\]$"
)
MONSTER_FIELD_RE = re.compile(r"^monster\[(\d+)\]")
PLAYER_POWER_FIELD_RE = re.compile(r"^player\.power\[([^\]]+)\]")
MONSTER_POWER_FIELD_RE = re.compile(r"^monster\[(\d+)\]\.power\[([^\]]+)\]")
FIXTURE_MISMATCH_RE = re.compile(r"mismatch on (.+?) \[response_id=", re.MULTILINE)

END_RUNTIME_CARRY_RELIC_IDS = {
    "Art of War",
}

DEFAULT_FIELD_LOOKBACK = 3

ENERGY_RELIC_IDS = {
    "Busted Crown",
    "Coffee Dripper",
    "Cursed Key",
    "Ectoplasm",
    "Fusion Hammer",
    "Mark of Pain",
    "Philosopher's Stone",
    "Runic Dome",
    "Sozu",
    "Velvet Choker",
    "Slaver's Collar",
    "Happy Flower",
    "Ice Cream",
}

DAMAGE_MOD_RELIC_IDS = {
    "Akabeko",
    "Boot",
    "The Boot",
    "Pen Nib",
    "Vajra",
    "Red Skull",
    "Paper Frog",
    "Champion Belt",
    "Necronomicon",
}

DRAW_EXHAUST_RELIC_IDS = {
    "Pocketwatch",
    "Runic Cube",
    "Medical Kit",
    "Dead Branch",
    "Charon's Ashes",
    "Sundial",
}

DRAW_EXHAUST_POWER_IDS = {
    "Corruption",
    "Dark Embrace",
    "Feel No Pain",
    "Evolve",
    "Fire Breathing",
    "NoDraw",
    "No Draw",
}

BOSS_MONSTER_IDS = {
    "Champ",
    "TimeEater",
    "Donu",
    "Deca",
    "AwakenedOne",
    "TheGuardian",
    "Hexaghost",
    "SlimeBoss",
    "Collector",
    "BronzeAutomaton",
}


def normalize_aspect_id(value: str) -> str:
    return "".join(ch for ch in value.lower() if ch.isalnum())


def ids_matching_catalog(ids: set[str], catalog: set[str]) -> list[str]:
    normalized_catalog = {normalize_aspect_id(item) for item in catalog}
    return sorted(
        value for value in ids if normalize_aspect_id(value) in normalized_catalog
    )


def aspect_summary_from_fixture(fixture: dict[str, Any]) -> dict[str, Any]:
    provenance = fixture.get("provenance")
    if not isinstance(provenance, dict):
        return {}
    aspect_summary = provenance.get("aspect_summary")
    if not isinstance(aspect_summary, dict):
        return {}
    return aspect_summary


def debug_context_summary_from_fixture(fixture: dict[str, Any]) -> dict[str, Any]:
    provenance = fixture.get("provenance")
    if not isinstance(provenance, dict):
        return {}
    context = provenance.get("debug_context_summary")
    if not isinstance(context, dict):
        return {}
    return context


def fixture_response_range(fixture: dict[str, Any]) -> tuple[int, int]:
    start = fixture.get("initial_response_id")
    end = fixture.get("final_response_id")
    if isinstance(start, int) and isinstance(end, int):
        return start, end

    provenance = fixture.get("provenance")
    if isinstance(provenance, dict):
        response_range = provenance.get("response_id_range")
        if (
            isinstance(response_range, (list, tuple))
            and len(response_range) == 2
            and all(isinstance(v, int) for v in response_range)
        ):
            return int(response_range[0]), int(response_range[1])

    initial_response_id = (
        fixture.get("initial_protocol_meta", {}).get("response_id")
        if isinstance(fixture.get("initial_protocol_meta"), dict)
        else None
    )
    if isinstance(initial_response_id, int):
        steps = fixture.get("steps", [])
        if isinstance(steps, list):
            return initial_response_id, initial_response_id + len(steps)

    raise SystemExit(
        "fixture does not contain an inferable response window; "
        "expected initial_response_id/final_response_id or provenance.response_id_range"
    )


def normalize_fixture_response_metadata(fixture: dict[str, Any]) -> dict[str, Any]:
    start, end = fixture_response_range(fixture)
    normalized = copy.deepcopy(fixture)
    normalized.setdefault("initial_response_id", start)
    normalized.setdefault("final_response_id", end)
    return normalized


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            obj = json.loads(line)
            if isinstance(obj, dict):
                rows.append(obj)
    return rows


def sort_watch_rows(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return sorted(
        rows,
        key=lambda row: (
            row.get("logged_at_unix_ms", 0),
            row.get("frame", 0),
            row.get("response_id", 0),
        ),
        reverse=True,
    )


def row_tags(row: dict[str, Any]) -> list[str]:
    tags = row.get("tags")
    if isinstance(tags, list):
        return [str(tag) for tag in tags]
    return []


def watch_row_matches_filters(
    row: dict[str, Any],
    *,
    tag_filters: list[str] | None = None,
    response_id: int | None = None,
    path_contains: str | None = None,
    include_noncombat: bool = False,
    require_existing_path: bool = False,
) -> bool:
    if not include_noncombat and row.get("kind") == "noncombat_sidecar":
        return False

    out_path = row.get("out_path")
    if not isinstance(out_path, str) or not out_path.strip():
        return False

    if require_existing_path and not Path(out_path).exists():
        return False

    if response_id is not None and row.get("response_id") != response_id:
        return False

    if path_contains and path_contains.lower() not in out_path.lower():
        return False

    tags_lower = [tag.lower() for tag in row_tags(row)]
    for tag in tag_filters or []:
        if tag.lower() not in tags_lower:
            return False

    return True


def filtered_watch_rows(
    rows: list[dict[str, Any]],
    *,
    tag_filters: list[str] | None = None,
    response_id: int | None = None,
    path_contains: str | None = None,
    include_noncombat: bool = False,
    require_existing_path: bool = False,
) -> list[dict[str, Any]]:
    return [
        row
        for row in sort_watch_rows(rows)
        if watch_row_matches_filters(
            row,
            tag_filters=tag_filters,
            response_id=response_id,
            path_contains=path_contains,
            include_noncombat=include_noncombat,
            require_existing_path=require_existing_path,
        )
    ]


def latest_watch_capture(
    audit_path: Path,
    *,
    tag_filters: list[str] | None = None,
    response_id: int | None = None,
    path_contains: str | None = None,
) -> dict[str, Any] | None:
    rows = filtered_watch_rows(
        load_jsonl(audit_path),
        tag_filters=tag_filters,
        response_id=response_id,
        path_contains=path_contains,
        include_noncombat=False,
        require_existing_path=True,
    )
    return rows[0] if rows else None


def derive_minimized_fixture_path(fixture_path: Path) -> Path:
    if fixture_path.suffix:
        return fixture_path.with_suffix(f".min{fixture_path.suffix}")
    return fixture_path.with_name(f"{fixture_path.name}.min.json")


def load_raw_records(path: Path) -> dict[int, dict[str, Any]]:
    records: dict[int, dict[str, Any]] = {}
    with path.open("r", encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            obj = json.loads(line)
            response_id = (
                obj.get("protocol_meta", {}).get("response_id")
                if isinstance(obj, dict)
                else None
            )
            if isinstance(response_id, int):
                records[response_id] = obj
    return records


def parse_java_expected(field: str, java_value: str) -> dict[str, Any]:
    java_value = java_value.strip()
    if java_value == "MISSING":
        return {"expected_kind": "missing"}

    if java_value.startswith("amount="):
        if re.fullmatch(r".+\.power\[[^\]]+\]$", field):
            field = f"{field}.amount"
        try:
            return {
                "field": field,
                "expected_kind": "number",
                "expected_value": int(java_value.split("=", 1)[1]),
            }
        except ValueError:
            return {"field": field, "expected_kind": "string", "expected_value": java_value}

    if re.fullmatch(r"-?\d+", java_value):
        return {"expected_kind": "number", "expected_value": int(java_value)}

    return {"expected_kind": "string", "expected_value": java_value}


def format_available_fields(assertions: list[dict[str, Any]], limit: int = 12) -> str:
    fields = sorted(
        {
            str(assertion.get("field"))
            for assertion in assertions
            if isinstance(assertion.get("field"), str)
        }
    )
    if not fields:
        return "<none>"
    if len(fields) <= limit:
        return ", ".join(fields)
    head = ", ".join(fields[:limit])
    return f"{head}, ... ({len(fields)} total)"


def dedupe_assertions(assertions: list[dict[str, Any]]) -> list[dict[str, Any]]:
    seen: set[tuple[Any, ...]] = set()
    deduped: list[dict[str, Any]] = []
    for assertion in assertions:
        key = (
            assertion.get("field"),
            assertion.get("response_id"),
            assertion.get("frame_id"),
            assertion.get("expected_kind"),
            json.dumps(assertion.get("expected_value"), sort_keys=True, ensure_ascii=False),
        )
        if key in seen:
            continue
        seen.add(key)
        deduped.append(assertion)
    return deduped


def with_context_guard_assertions(
    repo_root: Path, fixture: dict[str, Any], raw_path: Path, field: str | None
) -> dict[str, Any]:
    augmented = copy.deepcopy(fixture)
    if field is None:
        return augmented
    guards = build_context_guard_assertions(augmented, raw_path)
    if not guards:
        return augmented
    stable_guards: list[dict[str, Any]] = []
    for guard in guards:
        candidate = copy.deepcopy(fixture)
        candidate["assertions"] = [guard]
        if run_fixture(repo_root, write_temp_fixture(candidate)).passed:
            stable_guards.append(guard)
    if not stable_guards:
        return augmented
    combined = list(augmented.get("assertions", []))
    combined.extend(stable_guards)
    augmented["assertions"] = dedupe_assertions(combined)
    return augmented


def frame_ids_for_response_window(
    records: dict[int, dict[str, Any]],
    start_response_id: int,
    end_response_id: int,
) -> set[int]:
    frame_ids: set[int] = set()
    for rid in range(start_response_id, end_response_id + 1):
        root = records[rid]
        meta = root.get("protocol_meta", {})
        state_frame_id = meta.get("state_frame_id")
        if isinstance(state_frame_id, int):
            frame_ids.add(state_frame_id)
        else:
            frame_ids.add(rid)
    return frame_ids


def response_ids_for_response_window(
    start_response_id: int,
    end_response_id: int,
) -> list[int]:
    return list(range(start_response_id, end_response_id + 1))


def frame_to_latest_response_id(
    records: dict[int, dict[str, Any]],
    start_response_id: int,
    end_response_id: int,
) -> dict[int, int]:
    mapping: dict[int, int] = {}
    for rid in range(start_response_id, end_response_id + 1):
        meta = records[rid].get("protocol_meta", {})
        frame_id = meta.get("state_frame_id")
        if isinstance(frame_id, int):
            mapping[frame_id] = rid
    return mapping


def response_window_for_field_assertions(
    assertions: list[dict[str, Any]],
    frame_to_response: dict[int, int],
    start_response_id: int,
    end_response_id: int,
    lookback: int = DEFAULT_FIELD_LOOKBACK,
) -> tuple[int, int, list[int]]:
    target_response_ids = sorted(
        {
            frame_to_response[int(assertion["source_frame"])]
            for assertion in assertions
            if isinstance(assertion.get("source_frame"), int)
            and int(assertion["source_frame"]) in frame_to_response
        }
    )
    if not target_response_ids:
        return start_response_id, end_response_id, []

    cropped_end = max(target_response_ids)
    cropped_start = max(start_response_id, min(target_response_ids) - max(lookback, 0))
    return cropped_start, cropped_end, target_response_ids


def summarize_powers(powers: Any) -> list[dict[str, Any]]:
    if not isinstance(powers, list):
        return []
    summary: list[dict[str, Any]] = []
    for power in powers:
        if not isinstance(power, dict):
            continue
        item: dict[str, Any] = {}
        if "id" in power:
            item["id"] = power["id"]
        if "amount" in power:
            item["amount"] = power["amount"]
        if item:
            summary.append(item)
    return summary


def summarize_debug_context(root: dict[str, Any]) -> dict[str, Any]:
    gs = root.get("game_state", {}) if isinstance(root, dict) else {}
    combat = gs.get("combat_state", {}) if isinstance(gs, dict) else {}
    monsters = combat.get("monsters", []) if isinstance(combat, dict) else []
    relics = gs.get("relics", []) if isinstance(gs, dict) else []
    player = combat.get("player", {}) if isinstance(combat, dict) else {}

    monster_summary: list[dict[str, Any]] = []
    if isinstance(monsters, list):
        for idx, monster in enumerate(monsters):
            if not isinstance(monster, dict):
                continue
            monster_summary.append(
                {
                    "monster": idx,
                    "id": monster.get("id"),
                    "powers": summarize_powers(monster.get("powers")),
                }
            )

    return {
        "response_id": root.get("protocol_meta", {}).get("response_id")
        if isinstance(root, dict)
        else None,
        "frame_id": root.get("protocol_meta", {}).get("state_frame_id")
        if isinstance(root, dict)
        else None,
        "room_type": gs.get("room_type") if isinstance(gs, dict) else None,
        "screen_type": gs.get("screen_type") if isinstance(gs, dict) else None,
        "monster_ids": [
            monster.get("id")
            for monster in monsters
            if isinstance(monster, dict) and isinstance(monster.get("id"), str)
        ]
        if isinstance(monsters, list)
        else [],
        "relic_ids": [
            relic.get("id")
            for relic in relics
            if isinstance(relic, dict) and isinstance(relic.get("id"), str)
        ]
        if isinstance(relics, list)
        else [],
        "player_powers": summarize_powers(player.get("powers") if isinstance(player, dict) else []),
        "monster_powers": monster_summary,
    }


def summarize_aspects(root: dict[str, Any]) -> dict[str, Any]:
    context = summarize_debug_context(root)
    relic_ids = {
        relic_id
        for relic_id in context.get("relic_ids", [])
        if isinstance(relic_id, str)
    }
    player_power_ids = {
        power["id"]
        for power in context.get("player_powers", [])
        if isinstance(power, dict) and isinstance(power.get("id"), str)
    }
    monster_ids = {
        monster_id
        for monster_id in context.get("monster_ids", [])
        if isinstance(monster_id, str)
    }

    return {
        "energy_relics": ids_matching_catalog(relic_ids, ENERGY_RELIC_IDS),
        "damage_mod_relics": ids_matching_catalog(relic_ids, DAMAGE_MOD_RELIC_IDS),
        "draw_exhaust_engine": {
            "relics": ids_matching_catalog(relic_ids, DRAW_EXHAUST_RELIC_IDS),
            "powers": ids_matching_catalog(player_power_ids, DRAW_EXHAUST_POWER_IDS),
        },
        "boss_mechanics": {
            "room_type": context.get("room_type"),
            "monster_ids": ids_matching_catalog(monster_ids, BOSS_MONSTER_IDS)
            if context.get("room_type") == "MonsterRoomBoss"
            else [],
        },
    }


def collect_debug_assertions(
    path: Path,
    *,
    frame_ids: set[int] | None = None,
) -> list[dict[str, Any]]:
    assertions: list[dict[str, Any]] = []
    current_frame: int | None = None
    current_caused_by: str | None = None
    with path.open("r", encoding="utf-8") as fh:
        for raw_line in fh:
            line = raw_line.rstrip("\n")
            m = FRAME_RE.match(line)
            if m:
                current_frame = int(m.group(1))
                current_caused_by = None
                continue
            if current_frame is None:
                continue
            stripped = line.strip()
            if stripped.startswith("CAUSED BY:"):
                current_caused_by = stripped.removeprefix("CAUSED BY:").strip()
                continue
            if frame_ids is not None and current_frame not in frame_ids:
                continue
            d = DIFF_RE.match(line)
            if not d:
                continue
            assertion = {
                "field": d.group("field"),
                "source_kind": d.group("kind"),
                "java_value_raw": d.group("java"),
                "source_frame": current_frame,
            }
            if current_caused_by:
                assertion["caused_by"] = current_caused_by
            parsed = parse_java_expected(d.group("field"), d.group("java"))
            if "field" in parsed:
                assertion["field"] = parsed.pop("field")
            assertion.update(parsed)
            assertions.append(assertion)
    return assertions


def latest_matching_assertions(
    assertions: list[dict[str, Any]],
    field_filters: list[str],
) -> list[dict[str, Any]]:
    selected: list[dict[str, Any]] = []
    missing_fields: list[str] = []
    for field in field_filters:
        matches = [a for a in assertions if a.get("field") == field]
        if not matches:
            missing_fields.append(field)
            continue
        selected.append(max(matches, key=lambda a: int(a.get("source_frame", -1))))
    if missing_fields:
        available = format_available_fields(assertions)
        missing = ", ".join(missing_fields)
        raise SystemExit(
            f"no debug assertion with field(s) {missing!r} in selected window; "
            f"available fields: {available}"
        )
    return selected


def extract_fixture(
    raw_path: Path,
    debug_path: Path,
    start_response_id: int,
    end_response_id: int,
    failure_frame: int | None,
    field_filters: list[str] | None = None,
    field_lookback: int = DEFAULT_FIELD_LOOKBACK,
) -> dict[str, Any]:
    records = load_raw_records(raw_path)
    original_start_response_id = start_response_id
    original_end_response_id = end_response_id
    missing = [
        rid for rid in range(start_response_id, end_response_id + 1) if rid not in records
    ]
    if missing:
        raise SystemExit(f"missing response_ids in raw log: {missing}")

    original_frame_ids = frame_ids_for_response_window(records, start_response_id, end_response_id)
    original_debug_assertions = (
        collect_debug_assertions(debug_path, frame_ids=original_frame_ids)
        if failure_frame is not None or field_filters
        else []
    )
    original_frame_to_response = frame_to_latest_response_id(
        records, start_response_id, end_response_id
    )

    selected_assertions: list[dict[str, Any]] = []
    target_response_ids: list[int] = []
    if field_filters:
        selected_assertions = latest_matching_assertions(original_debug_assertions, field_filters)
        (
            start_response_id,
            end_response_id,
            target_response_ids,
        ) = response_window_for_field_assertions(
            selected_assertions,
            original_frame_to_response,
            start_response_id,
            end_response_id,
            lookback=field_lookback,
        )

    provenance_notes: list[str] = []
    if (
        field_filters
        and (
            start_response_id != original_start_response_id
            or end_response_id != original_end_response_id
        )
    ):
        provenance_notes.append(
            f"field_auto_crop from {original_start_response_id}..{original_end_response_id} "
            f"to {start_response_id}..{end_response_id}"
        )

    initial = records[start_response_id]
    final = records[end_response_id]
    steps = []
    previous_command = initial.get("protocol_meta", {}).get("last_command")
    if not isinstance(previous_command, str):
        previous_command = None
    for rid in range(start_response_id + 1, end_response_id + 1):
        root = records[rid]
        meta = root.get("protocol_meta", {})
        human_choice = meta.get("recent_human_card_reward_choice")
        frame_id = meta.get("state_frame_id")
        if isinstance(human_choice, dict):
            choice_kind = human_choice.get("choice_kind")
            if choice_kind == "card":
                choice_index = human_choice.get("choice_index")
                if isinstance(choice_index, int):
                    steps.append(
                        {
                            "response_id": rid,
                            "frame_id": frame_id,
                            "command": f"HUMAN_CARD_REWARD {choice_index}",
                            "command_kind": "human_card_reward",
                        }
                    )
            elif choice_kind in {"skip", "bowl"}:
                steps.append(
                    {
                        "response_id": rid,
                        "frame_id": frame_id,
                        "command": "HUMAN_CARD_REWARD SKIP",
                        "command_kind": "human_card_reward",
                    }
                )
        command = meta.get("last_command")
        if not isinstance(command, str):
            raise SystemExit(f"response_id={rid} has no protocol_meta.last_command")
        if (
            isinstance(human_choice, dict)
            and previous_command is not None
            and command == previous_command
        ):
            continue
        steps.append(
            {
                "response_id": rid,
                "frame_id": frame_id,
                "command": command,
                "command_kind": meta.get("last_command_kind"),
            }
        )
        previous_command = command

    frame_ids = frame_ids_for_response_window(records, start_response_id, end_response_id)
    frame_to_response = frame_to_latest_response_id(records, start_response_id, end_response_id)
    debug_assertions = (
        collect_debug_assertions(debug_path, frame_ids=frame_ids)
        if failure_frame is not None or field_filters
        else []
    )
    if field_filters:
        assertions = latest_matching_assertions(debug_assertions, field_filters)
    elif failure_frame is not None:
        assertions = [
            assertion
            for assertion in debug_assertions
            if assertion.get("source_frame") == failure_frame
        ]
    else:
        assertions = []

    for assertion in assertions:
        source_frame = assertion.get("source_frame")
        if isinstance(source_frame, int):
            assertion["frame_id"] = source_frame
            response_id = frame_to_response.get(source_frame)
            if response_id is not None:
                assertion["response_id"] = response_id

    context_response_id = (
        max(
            [
                assertion["response_id"]
                for assertion in assertions
                if isinstance(assertion.get("response_id"), int)
            ],
            default=max(target_response_ids) if target_response_ids else end_response_id,
        )
    )
    context_root = records[context_response_id]
    debug_context_summary = summarize_debug_context(context_root)
    aspect_summary = summarize_aspects(context_root)

    fixture = {
        "name": f"live_window_{start_response_id}_{end_response_id}",
        "initial_response_id": start_response_id,
        "final_response_id": end_response_id,
        "initial_game_state": initial.get("game_state"),
        "initial_protocol_meta": {
            "last_command": initial.get("protocol_meta", {}).get("last_command"),
            "last_command_kind": initial.get("protocol_meta", {}).get("last_command_kind"),
            "response_id": initial.get("protocol_meta", {}).get("response_id"),
            "state_frame_id": initial.get("protocol_meta", {}).get("state_frame_id"),
        },
        "final_game_state": final.get("game_state"),
        "steps": steps,
        "assertions": assertions,
        "provenance": {
            "response_id_range": [start_response_id, end_response_id],
            "assertion_source_frames": sorted(
                {
                    int(assertion["source_frame"])
                    for assertion in assertions
                    if isinstance(assertion.get("source_frame"), int)
                }
            ),
            "assertion_source_response_ids": sorted(
                {
                    int(assertion["response_id"])
                    for assertion in assertions
                    if isinstance(assertion.get("response_id"), int)
                }
            ),
            "debug_context_summary": debug_context_summary,
            "aspect_summary": aspect_summary,
            "notes": provenance_notes,
        },
    }
    if field_filters:
        fixture["provenance"]["field_filters"] = field_filters
    return fixture


def cmd_extract(args: argparse.Namespace) -> int:
    fixture = extract_fixture(
        raw_path=args.raw_path,
        debug_path=args.debug_path,
        start_response_id=args.from_response_id,
        end_response_id=args.to_response_id,
        failure_frame=args.failure_frame,
        field_filters=args.field or None,
        field_lookback=args.field_lookback,
    )
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(fixture, indent=2), encoding="utf-8")
    print(f"wrote {args.out}")
    return 0


def sanitize_fixture_for_local_lab(fixture: dict[str, Any]) -> dict[str, Any]:
    sanitized = copy.deepcopy(fixture)
    name = sanitized.get("name")
    if isinstance(name, str) and not name.endswith("_start"):
        sanitized["name"] = f"{name}_start"
    sanitized["steps"] = []
    sanitized["assertions"] = []
    tags = sanitized.get("tags")
    if not isinstance(tags, list):
        tags = []
    tags.append("local_lab_start")
    sanitized["tags"] = sorted({str(tag) for tag in tags})
    provenance = sanitized.get("provenance")
    if not isinstance(provenance, dict):
        provenance = {}
    notes = provenance.get("notes")
    if not isinstance(notes, list):
        notes = []
    notes.append("sanitized_for_local_lab")
    provenance["notes"] = sorted({str(note) for note in notes})
    sanitized["provenance"] = provenance
    return sanitized


def cmd_sanitize_lab(args: argparse.Namespace) -> int:
    fixture = json.loads(args.fixture.read_text(encoding="utf-8"))
    sanitized = sanitize_fixture_for_local_lab(fixture)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(sanitized, indent=2), encoding="utf-8")
    print(f"wrote sanitized local-lab fixture {args.out}")
    return 0


def repo_root_from_path(path: Path) -> Path:
    path = path.resolve()
    if path.name == "tools":
        return path.parent
    if path.name == "sts_simulator":
        return path
    for parent in [path] + list(path.parents):
        if (parent / "Cargo.toml").exists() and (parent / "src").exists():
            return parent
    raise SystemExit(f"could not find sts_simulator repo root from {path}")


@dataclass(frozen=True)
class FixtureRunResult:
    passed: bool
    output: str


def run_fixture(repo_root: Path, fixture_path: Path) -> FixtureRunResult:
    env = dict(**__import__("os").environ)
    env["LIVE_REGRESSION_FIXTURE"] = str(fixture_path)
    proc = subprocess.run(
        [
            "cargo",
            "test",
            "--test",
            "live_regression_driver",
            "replay_single_fixture_from_env",
            "--",
            "--nocapture",
        ],
        cwd=repo_root,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )
    return FixtureRunResult(passed=proc.returncode == 0, output=proc.stdout)


def mismatch_fields_from_output(output: str) -> list[str]:
    return [match.group(1) for match in FIXTURE_MISMATCH_RE.finditer(output)]


def result_matches_target(
    result: FixtureRunResult, target_passed: bool, field: str | None
) -> bool:
    if result.passed != target_passed:
        return False
    if target_passed or field is None:
        return True
    return field in mismatch_fields_from_output(result.output)


def resolve_target_outcome(mode: str, baseline_passed: bool) -> bool:
    if mode == "same-outcome":
        return baseline_passed
    if mode == "pass-only":
        if not baseline_passed:
            raise SystemExit("requested --mode pass-only but original fixture currently fails")
        return True
    if mode == "fail-only":
        if baseline_passed:
            raise SystemExit("requested --mode fail-only but original fixture currently passes")
        return False
    raise SystemExit(f"unsupported mode {mode!r}")


def write_temp_fixture(fixture: dict[str, Any]) -> Path:
    tmp_dir = Path(tempfile.mkdtemp(prefix="live_regression_min_"))
    path = tmp_dir / "candidate.json"
    path.write_text(json.dumps(fixture, indent=2), encoding="utf-8")
    return path


def slim_card(card: Any) -> Any:
    if not isinstance(card, dict):
        return card
    slim = {}
    for key in ("id", "uuid", "upgrades", "cost"):
        if key in card:
            slim[key] = card[key]
    return slim


def slim_power(power: Any) -> Any:
    if not isinstance(power, dict):
        return power
    slim = {}
    for key in ("id", "amount", "misc", "just_applied"):
        if key in power:
            slim[key] = power[key]
    return slim


def slim_monster(monster: Any) -> Any:
    if not isinstance(monster, dict):
        return monster
    slim = {}
    for key in (
        "id",
        "current_hp",
        "hp",
        "max_hp",
        "block",
        "intent",
        "move_base_damage",
        "move_adjusted_damage",
        "move_hits",
        "move_id",
        "is_gone",
        "half_dead",
    ):
        if key in monster:
            slim[key] = monster[key]
    if "powers" in monster and isinstance(monster["powers"], list):
        slim["powers"] = [slim_power(power) for power in monster["powers"]]
    return slim


def slim_player(player: Any) -> Any:
    if not isinstance(player, dict):
        return player
    slim = {}
    for key in ("current_hp", "hp", "max_hp", "block", "energy"):
        if key in player:
            slim[key] = player[key]
    if "powers" in player and isinstance(player["powers"], list):
        slim["powers"] = [slim_power(power) for power in player["powers"]]
    return slim


def slim_potion(potion: Any) -> Any:
    if not isinstance(potion, dict):
        return potion
    slim = {}
    for key in ("id",):
        if key in potion:
            slim[key] = potion[key]
    return slim


def slim_relic(relic: Any) -> Any:
    if not isinstance(relic, dict):
        return relic
    slim = {}
    for key in ("id", "counter"):
        if key in relic:
            slim[key] = relic[key]
    return slim


def slim_rng_state(rng_state: Any) -> Any:
    if not isinstance(rng_state, dict):
        return rng_state
    slim: dict[str, Any] = {}
    for name in (
        "ai_rng",
        "shuffle_rng",
        "card_rng",
        "misc_rng",
        "monster_hp_rng",
        "potion_rng",
    ):
        value = rng_state.get(name)
        if not isinstance(value, dict):
            continue
        slim[name] = {}
        for key in ("seed0", "seed1", "counter"):
            if key in value:
                slim[name][key] = value[key]
    return slim


def slim_combat_state(snapshot: Any) -> Any:
    if not isinstance(snapshot, dict):
        return snapshot
    slim: dict[str, Any] = {}
    for key in ("turn",):
        if key in snapshot:
            slim[key] = snapshot[key]
    if "player" in snapshot:
        slim["player"] = slim_player(snapshot["player"])
    if "monsters" in snapshot and isinstance(snapshot["monsters"], list):
        slim["monsters"] = [slim_monster(monster) for monster in snapshot["monsters"]]
    for key in ("hand", "draw_pile", "discard_pile", "exhaust_pile", "limbo"):
        if key in snapshot and isinstance(snapshot[key], list):
            slim[key] = [slim_card(card) for card in snapshot[key]]
    if "potions" in snapshot and isinstance(snapshot["potions"], list):
        slim["potions"] = [slim_potion(potion) for potion in snapshot["potions"]]
    if "relics" in snapshot and isinstance(snapshot["relics"], list):
        slim["relics"] = [slim_relic(relic) for relic in snapshot["relics"]]
    if "rng_state" in snapshot and isinstance(snapshot["rng_state"], dict):
        slim["rng_state"] = slim_rng_state(snapshot["rng_state"])
    return slim


def slim_game_state(game_state: Any) -> Any:
    if not isinstance(game_state, dict):
        return game_state
    slim: dict[str, Any] = {}
    for key in ("screen_type", "room_type", "potions", "relics", "choice_list"):
        if key in game_state:
            value = game_state[key]
            if key == "potions" and isinstance(value, list):
                slim[key] = [slim_potion(potion) for potion in value]
            elif key == "relics" and isinstance(value, list):
                slim[key] = [slim_relic(relic) for relic in value]
            else:
                slim[key] = value
    if (
        game_state.get("screen_type") == "CARD_REWARD"
        and isinstance(game_state.get("screen_state"), dict)
    ):
        screen_state = game_state["screen_state"]
        slim_screen_state: dict[str, Any] = {}
        if isinstance(screen_state.get("cards"), list):
            slim_screen_state["cards"] = [slim_card(card) for card in screen_state["cards"]]
        for key in ("skip_available", "bowl_available"):
            if key in screen_state:
                slim_screen_state[key] = screen_state[key]
        if slim_screen_state:
            slim["screen_state"] = slim_screen_state
    if "combat_state" in game_state:
        slim["combat_state"] = slim_combat_state(game_state["combat_state"])
    return slim


def minimize_state(repo_root: Path, fixture: dict[str, Any], baseline_passed: bool) -> dict[str, Any]:
    if any(
        assertion.get("source_kind") == "CONTEXT_GUARD"
        for assertion in fixture.get("assertions", [])
        if isinstance(assertion, dict)
    ):
        return fixture
    candidate = dict(fixture)
    if "initial_game_state" in candidate:
        candidate["initial_game_state"] = slim_game_state(candidate["initial_game_state"])
    if "final_game_state" in candidate:
        candidate["final_game_state"] = slim_game_state(candidate["final_game_state"])
    if run_fixture(repo_root, write_temp_fixture(candidate)).passed == baseline_passed:
        return candidate
    return fixture


def get_in(root: Any, path: tuple[Any, ...]) -> Any:
    current = root
    for key in path:
        if isinstance(current, dict):
            current = current.get(key)
        elif isinstance(current, list) and isinstance(key, int):
            if 0 <= key < len(current):
                current = current[key]
            else:
                return None
        else:
            return None
    return current


def shrink_list_path(
    repo_root: Path,
    fixture: dict[str, Any],
    path: tuple[Any, ...],
    target_passed: bool,
    field: str | None,
    protected_indices: set[int] | None = None,
) -> dict[str, Any]:
    protected_indices = protected_indices or set()
    current = fixture
    while True:
        items = get_in(current, path)
        if not isinstance(items, list) or not items:
            return current
        changed = False
        for idx in range(len(items) - 1, -1, -1):
            if idx in protected_indices:
                continue
            candidate = copy.deepcopy(current)
            candidate_items = get_in(candidate, path)
            if not isinstance(candidate_items, list) or idx >= len(candidate_items):
                continue
            del candidate_items[idx]
            if result_matches_target(
                run_fixture(repo_root, write_temp_fixture(candidate)),
                target_passed,
                field,
            ):
                current = candidate
                changed = True
                break
        if not changed:
            return current


def protected_hand_indices(fixture: dict[str, Any]) -> set[int]:
    protected: set[int] = set()
    highest_referenced = -1
    for step in fixture.get("steps", []):
        command = step.get("command")
        if not isinstance(command, str):
            continue
        parts = command.split()
        if len(parts) >= 2 and parts[0] == "PLAY":
            try:
                highest_referenced = max(highest_referenced, max(int(parts[1]) - 1, 0))
            except ValueError:
                pass
    if highest_referenced >= 0:
        protected.update(range(highest_referenced + 1))
    return protected


def protected_monster_indices(fixture: dict[str, Any]) -> set[int]:
    protected: set[int] = set()
    highest_referenced = -1
    for step in fixture.get("steps", []):
        command = step.get("command")
        if not isinstance(command, str):
            continue
        parts = command.split()
        if len(parts) >= 3 and parts[0] == "PLAY":
            try:
                highest_referenced = max(highest_referenced, int(parts[2]))
            except ValueError:
                pass
        if len(parts) >= 4 and parts[0] == "POTION" and parts[1] == "USE":
            try:
                highest_referenced = max(highest_referenced, int(parts[3]))
            except ValueError:
                pass
    for assertion in fixture.get("assertions", []):
        field = assertion.get("field")
        if not isinstance(field, str):
            continue
        m = MONSTER_FIELD_RE.match(field)
        if m:
            highest_referenced = max(highest_referenced, int(m.group(1)))
    if highest_referenced >= 0:
        protected.update(range(highest_referenced + 1))
    return protected


def protected_aspect_monster_indices(fixture: dict[str, Any]) -> set[int]:
    context = debug_context_summary_from_fixture(fixture)
    aspect_summary = aspect_summary_from_fixture(fixture)
    boss_mechanics = aspect_summary.get("boss_mechanics")
    if not isinstance(boss_mechanics, dict):
        return set()
    boss_monster_ids = {
        normalize_aspect_id(monster_id)
        for monster_id in boss_mechanics.get("monster_ids", [])
        if isinstance(monster_id, str)
    }
    if not boss_monster_ids:
        return set()

    protected: set[int] = set()
    for monster in context.get("monster_powers", []):
        if not isinstance(monster, dict):
            continue
        monster_idx = monster.get("monster")
        monster_id = monster.get("id")
        if not isinstance(monster_idx, int) or not isinstance(monster_id, str):
            continue
        if normalize_aspect_id(monster_id) in boss_monster_ids:
            protected.add(monster_idx)
    return protected


def protected_player_power_names(fixture: dict[str, Any]) -> set[str]:
    protected: set[str] = set()
    for assertion in fixture.get("assertions", []):
        field = assertion.get("field")
        if not isinstance(field, str):
            continue
        m = PLAYER_POWER_FIELD_RE.match(field)
        if m:
            protected.add(m.group(1))
    return protected


def protected_aspect_player_power_names(fixture: dict[str, Any]) -> set[str]:
    protected: set[str] = set()
    aspect_summary = aspect_summary_from_fixture(fixture)
    draw_exhaust_engine = aspect_summary.get("draw_exhaust_engine")
    if not isinstance(draw_exhaust_engine, dict):
        return protected
    for power_name in draw_exhaust_engine.get("powers", []):
        if isinstance(power_name, str):
            protected.add(power_name)
    return protected


def protected_monster_power_names(fixture: dict[str, Any]) -> dict[int, set[str]]:
    protected: dict[int, set[str]] = {}
    for assertion in fixture.get("assertions", []):
        field = assertion.get("field")
        if not isinstance(field, str):
            continue
        m = MONSTER_POWER_FIELD_RE.match(field)
        if m:
            monster_idx = int(m.group(1))
            protected.setdefault(monster_idx, set()).add(m.group(2))
    return protected


def protected_relic_ids(fixture: dict[str, Any]) -> set[str]:
    protected: set[str] = set()
    aspect_summary = aspect_summary_from_fixture(fixture)
    for key in ("energy_relics", "damage_mod_relics"):
        for relic_id in aspect_summary.get(key, []):
            if isinstance(relic_id, str):
                protected.add(relic_id)
    draw_exhaust_engine = aspect_summary.get("draw_exhaust_engine")
    if isinstance(draw_exhaust_engine, dict):
        for relic_id in draw_exhaust_engine.get("relics", []):
            if isinstance(relic_id, str):
                protected.add(relic_id)

    has_end_step = any(
        isinstance(step.get("command"), str) and step["command"].split()[:1] == ["END"]
        for step in fixture.get("steps", [])
    )
    if not has_end_step:
        return protected

    relics = get_in(fixture, ("initial_game_state", "relics"))
    if not isinstance(relics, list):
        return protected

    for relic in relics:
        if isinstance(relic, dict) and relic.get("id") in END_RUNTIME_CARRY_RELIC_IDS:
            protected.add(relic["id"])
    return protected


def shrink_relics_by_id(
    repo_root: Path,
    fixture: dict[str, Any],
    path: tuple[Any, ...],
    target_passed: bool,
    field: str | None,
    protected_ids: set[str],
) -> dict[str, Any]:
    current = fixture
    while True:
        relics = get_in(current, path)
        if not isinstance(relics, list) or not relics:
            return current
        changed = False
        for idx in range(len(relics) - 1, -1, -1):
            relic = relics[idx]
            if isinstance(relic, dict) and relic.get("id") in protected_ids:
                continue
            candidate = copy.deepcopy(current)
            candidate_relics = get_in(candidate, path)
            if not isinstance(candidate_relics, list) or idx >= len(candidate_relics):
                continue
            del candidate_relics[idx]
            if result_matches_target(
                run_fixture(repo_root, write_temp_fixture(candidate)),
                target_passed,
                field,
            ):
                current = candidate
                changed = True
                break
        if not changed:
            return current


def shrink_monsters_by_id(
    repo_root: Path,
    fixture: dict[str, Any],
    path: tuple[Any, ...],
    target_passed: bool,
    field: str | None,
    protected_indices: set[int],
) -> dict[str, Any]:
    current = fixture
    while True:
        monsters = get_in(current, path)
        if not isinstance(monsters, list) or not monsters:
            return current
        changed = False
        for idx in range(len(monsters) - 1, -1, -1):
            if idx in protected_indices:
                continue
            candidate = copy.deepcopy(current)
            candidate_monsters = get_in(candidate, path)
            if not isinstance(candidate_monsters, list) or idx >= len(candidate_monsters):
                continue
            del candidate_monsters[idx]
            if result_matches_target(
                run_fixture(repo_root, write_temp_fixture(candidate)),
                target_passed,
                field,
            ):
                current = candidate
                changed = True
                break
        if not changed:
            return current


def shrink_powers_by_name(
    repo_root: Path,
    fixture: dict[str, Any],
    path: tuple[Any, ...],
    target_passed: bool,
    field: str | None,
    protected_names: set[str],
) -> dict[str, Any]:
    current = fixture
    while True:
        powers = get_in(current, path)
        if not isinstance(powers, list) or not powers:
            return current
        changed = False
        for idx in range(len(powers) - 1, -1, -1):
            power = powers[idx]
            if isinstance(power, dict) and power.get("id") in protected_names:
                continue
            candidate = copy.deepcopy(current)
            candidate_powers = get_in(candidate, path)
            if not isinstance(candidate_powers, list) or idx >= len(candidate_powers):
                continue
            del candidate_powers[idx]
            if result_matches_target(
                run_fixture(repo_root, write_temp_fixture(candidate)),
                target_passed,
                field,
            ):
                current = candidate
                changed = True
                break
        if not changed:
            return current


def shrink_state_lists(
    repo_root: Path, fixture: dict[str, Any], target_passed: bool, field: str | None
) -> dict[str, Any]:
    current = fixture
    protected_relics = protected_relic_ids(current)
    current = shrink_relics_by_id(
        repo_root,
        current,
        ("initial_game_state", "relics"),
        target_passed,
        field,
        protected_relics,
    )
    current = shrink_relics_by_id(
        repo_root,
        current,
        ("initial_game_state", "combat_state", "relics"),
        target_passed,
        field,
        protected_relics,
    )

    protected_monsters = protected_monster_indices(current) | protected_aspect_monster_indices(
        current
    )
    current = shrink_monsters_by_id(
        repo_root,
        current,
        ("initial_game_state", "combat_state", "monsters"),
        target_passed,
        field,
        protected_monsters,
    )

    for path, protected in (
        (("initial_game_state", "combat_state", "draw_pile"), set()),
        (("initial_game_state", "combat_state", "discard_pile"), set()),
        (("initial_game_state", "combat_state", "exhaust_pile"), set()),
        (("initial_game_state", "combat_state", "limbo"), set()),
        (("initial_game_state", "combat_state", "hand"), protected_hand_indices(current)),
    ):
        current = shrink_list_path(
            repo_root=repo_root,
            fixture=current,
            path=path,
            target_passed=target_passed,
            field=field,
            protected_indices=protected,
        )

    current = shrink_powers_by_name(
        repo_root,
        current,
        ("initial_game_state", "combat_state", "player", "powers"),
        target_passed,
        field,
        protected_player_power_names(current) | protected_aspect_player_power_names(current),
    )

    monster_power_protected = protected_monster_power_names(current)
    monsters = get_in(current, ("initial_game_state", "combat_state", "monsters"))
    if isinstance(monsters, list):
        for monster_idx in range(len(monsters)):
            current = shrink_powers_by_name(
                repo_root,
                current,
                ("initial_game_state", "combat_state", "monsters", monster_idx, "powers"),
                target_passed,
                field,
                monster_power_protected.get(monster_idx, set()),
            )

    candidate = copy.deepcopy(current)
    candidate.pop("final_game_state", None)
    if result_matches_target(
        run_fixture(repo_root, write_temp_fixture(candidate)),
        target_passed,
        field,
    ):
        current = candidate

    return current


def minimize_assertions(
    repo_root: Path, fixture: dict[str, Any], target_passed: bool, field: str | None
) -> dict[str, Any]:
    assertions = fixture.get("assertions", [])
    if len(assertions) <= 1:
        return fixture

    anchors = [
        assertion
        for assertion in assertions
        if assertion.get("source_kind") != "CONTEXT_GUARD"
    ]
    guards = [
        assertion
        for assertion in assertions
        if assertion.get("source_kind") == "CONTEXT_GUARD"
    ]
    if anchors and guards:
        current = dict(fixture)
        current["assertions"] = anchors + guards
        for guard in list(guards):
            candidate = dict(current)
            candidate["assertions"] = [
                assertion
                for assertion in current["assertions"]
                if assertion is not guard
            ]
            if result_matches_target(
                run_fixture(repo_root, write_temp_fixture(candidate)),
                target_passed,
                field,
            ):
                current = candidate
        return current

    for assertion in assertions:
        candidate = dict(fixture)
        candidate["assertions"] = [assertion]
        if result_matches_target(
            run_fixture(repo_root, write_temp_fixture(candidate)),
            target_passed,
            field,
        ):
            return candidate
    return fixture


def build_context_guard_assertions(
    fixture: dict[str, Any], raw_path: Path
) -> list[dict[str, Any]]:
    provenance = fixture.get("provenance")
    if not isinstance(provenance, dict):
        return []

    context = provenance.get("debug_context_summary")
    if not isinstance(context, dict):
        return []

    response_id = context.get("response_id")
    frame_id = context.get("frame_id")
    if not isinstance(response_id, int):
        return []

    records = load_raw_records(raw_path)
    record = records.get(response_id)
    if not isinstance(record, dict):
        return []

    game_state = record.get("game_state")
    if not isinstance(game_state, dict):
        return []

    combat_state = game_state.get("combat_state")
    if not isinstance(combat_state, dict):
        return []

    guards: list[dict[str, Any]] = []

    def add_number(field: str, value: Any) -> None:
        if isinstance(value, bool) or not isinstance(value, int):
            return
        guards.append(
            {
                "field": field,
                "expected_kind": "number",
                "expected_value": value,
                "response_id": response_id,
                "frame_id": frame_id,
                "source_kind": "CONTEXT_GUARD",
            }
        )

    player = combat_state.get("player")
    if isinstance(player, dict):
        add_number("player.hp", player.get("current_hp"))
        add_number("player.energy", player.get("energy"))
        add_number("player.block", player.get("block"))
        if isinstance(player.get("powers"), list):
            for power in player["powers"]:
                if not isinstance(power, dict):
                    continue
                power_id = power.get("id")
                amount = power.get("amount")
                if isinstance(power_id, str) and isinstance(amount, int):
                    guards.append(
                        {
                            "field": f"player.power[{power_id}].amount",
                            "expected_kind": "number",
                            "expected_value": amount,
                            "response_id": response_id,
                            "frame_id": frame_id,
                            "source_kind": "CONTEXT_GUARD",
                        }
                    )

    if isinstance(combat_state.get("hand"), list):
        add_number("hand_size", len(combat_state["hand"]))
    if isinstance(combat_state.get("draw_pile"), list):
        add_number("draw_pile_size", len(combat_state["draw_pile"]))
    if isinstance(combat_state.get("discard_pile"), list):
        add_number("discard_pile_size", len(combat_state["discard_pile"]))
    if isinstance(combat_state.get("exhaust_pile"), list):
        add_number("exhaust_pile_size", len(combat_state["exhaust_pile"]))

    monsters = combat_state.get("monsters")
    if isinstance(monsters, list):
        add_number("monsters_count", len(monsters))
        for idx, monster in enumerate(monsters):
            if not isinstance(monster, dict):
                continue
            add_number(f"monster[{idx}].hp", monster.get("current_hp"))
            add_number(f"monster[{idx}].block", monster.get("block"))
            if isinstance(monster.get("powers"), list):
                for power in monster["powers"]:
                    if not isinstance(power, dict):
                        continue
                    power_id = power.get("id")
                    amount = power.get("amount")
                    if isinstance(power_id, str) and isinstance(amount, int):
                        guards.append(
                            {
                                "field": f"monster[{idx}].power[{power_id}].amount",
                                "expected_kind": "number",
                                "expected_value": amount,
                                "response_id": response_id,
                                "frame_id": frame_id,
                                "source_kind": "CONTEXT_GUARD",
                            }
                        )

    return dedupe_assertions(guards)


def minimize_fixture(
    repo_root: Path,
    fixture: dict[str, Any],
    raw_path: Path,
    debug_path: Path,
    failure_frame: int | None,
    field: str | None,
    mode: str,
    preserve_at_least_one_step: bool,
) -> tuple[dict[str, Any], FixtureRunResult]:
    fixture = normalize_fixture_response_metadata(fixture)

    if field:
        filtered = [a for a in fixture.get("assertions", []) if a.get("field") == field]
        if not filtered:
            available = format_available_fields(fixture.get("assertions", []))
            raise SystemExit(
                f"no assertion with field={field!r} in fixture; "
                f"available fields: {available}"
            )
        fixture["assertions"] = filtered
    fixture = with_context_guard_assertions(repo_root, fixture, raw_path, field)

    baseline = run_fixture(repo_root, write_temp_fixture(fixture))
    target_passed = resolve_target_outcome(mode, baseline.passed)

    fixture = minimize_assertions(repo_root, fixture, target_passed, field)

    start = fixture["initial_response_id"]
    end = fixture["final_response_id"]

    best = fixture
    current_start = start
    while current_start < end:
        if preserve_at_least_one_step and len(best.get("steps", [])) <= 1:
            break
        candidate_start = current_start + 1
        try:
            candidate = extract_fixture(
                raw_path=raw_path,
                debug_path=debug_path,
                start_response_id=candidate_start,
                end_response_id=end,
                failure_frame=failure_frame,
                field_filters=[field] if field else None,
                field_lookback=DEFAULT_FIELD_LOOKBACK,
            )
        except SystemExit as exc:
            if field and "no debug assertion with field" in str(exc):
                break
            raise
        if not field:
            candidate["assertions"] = best.get("assertions", candidate.get("assertions", []))
        candidate = with_context_guard_assertions(repo_root, candidate, raw_path, field)
        if result_matches_target(
            run_fixture(repo_root, write_temp_fixture(candidate)),
            target_passed,
            field,
        ):
            best = candidate
            current_start = candidate_start
        else:
            break

    if not result_matches_target(
        run_fixture(repo_root, write_temp_fixture(best)), target_passed, field
    ):
        return fixture, baseline

    pre_slim = best
    slimmed = minimize_state(repo_root, best, target_passed)
    if result_matches_target(
        run_fixture(repo_root, write_temp_fixture(slimmed)), target_passed, field
    ):
        best = slimmed
    else:
        best = pre_slim

    pre_lists = best
    shrunk = shrink_state_lists(repo_root, best, target_passed, field)
    if result_matches_target(
        run_fixture(repo_root, write_temp_fixture(shrunk)), target_passed, field
    ):
        best = shrunk
    else:
        best = pre_lists

    return best, baseline


def cmd_watch_list(args: argparse.Namespace) -> int:
    rows = load_jsonl(args.audit_path)
    if args.include_noncombat:
        rows.extend(load_jsonl(args.noncombat_audit_path))
    rows = filtered_watch_rows(
        rows,
        tag_filters=args.tag or [],
        response_id=args.response_id,
        path_contains=args.path_contains,
        include_noncombat=args.include_noncombat,
        require_existing_path=args.require_existing_path,
    )
    if not rows:
        print("no watch captures found")
        return 0

    limit = max(args.limit, 1)
    for row in rows[:limit]:
        path = row.get("out_path", "<unknown>")
        tags = ",".join(row.get("tags", [])) if isinstance(row.get("tags"), list) else ""
        frame = row.get("frame")
        response_id = row.get("response_id")
        start_response_id = row.get("start_response_id")
        kind = row.get("kind", "combat_capture")
        print(
            f"{kind} response_id={response_id} start={start_response_id} frame={frame} "
            f"tags=[{tags}] path={path}"
        )
    return 0


def cmd_minimize(args: argparse.Namespace) -> int:
    repo_root = repo_root_from_path(args.repo_root)
    fixture = json.loads(args.fixture.read_text(encoding="utf-8"))
    best, baseline = minimize_fixture(
        repo_root=repo_root,
        fixture=fixture,
        raw_path=args.raw_path,
        debug_path=args.debug_path,
        failure_frame=args.failure_frame,
        field=args.field,
        mode=args.mode,
        preserve_at_least_one_step=args.preserve_at_least_one_step,
    )

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(best, indent=2), encoding="utf-8")
    print(
        f"wrote minimized fixture {args.out} "
        f"(responses {best['initial_response_id']}..{best['final_response_id']}, "
        f"assertions={len(best.get('assertions', []))}, "
        f"baseline={'pass' if baseline.passed else 'fail'}, "
        f"mode={args.mode})"
    )
    return 0


def cmd_watch_minimize_latest(args: argparse.Namespace) -> int:
    row = latest_watch_capture(
        args.audit_path,
        tag_filters=args.tag or [],
        response_id=args.response_id,
        path_contains=args.path_contains,
    )
    if row is None:
        print("no replayable watch captures found")
        return 0

    fixture_path = Path(row["out_path"])
    if not fixture_path.exists():
        raise SystemExit(f"latest watch fixture does not exist: {fixture_path}")

    repo_root = repo_root_from_path(args.repo_root)
    out_path = args.out or derive_minimized_fixture_path(fixture_path)
    fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
    best, baseline = minimize_fixture(
        repo_root=repo_root,
        fixture=fixture,
        raw_path=args.raw_path,
        debug_path=args.debug_path,
        failure_frame=args.failure_frame,
        field=args.field,
        mode=args.mode,
        preserve_at_least_one_step=args.preserve_at_least_one_step,
    )

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(best, indent=2), encoding="utf-8")
    print(
        f"wrote minimized fixture {out_path} from latest watch {fixture_path} "
        f"(responses {best['initial_response_id']}..{best['final_response_id']}, "
        f"assertions={len(best.get('assertions', []))}, "
        f"baseline={'pass' if baseline.passed else 'fail'}, "
        f"mode={args.mode})"
    )
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Extract a live_comm combat window into a replayable regression fixture."
    )
    sub = parser.add_subparsers(dest="command", required=True)

    extract = sub.add_parser("extract", help="extract a combat window fixture")
    extract.add_argument("--raw-path", type=Path, default=DEFAULT_RAW_PATH)
    extract.add_argument("--debug-path", type=Path, default=DEFAULT_DEBUG_PATH)
    extract.add_argument("--from-response-id", type=int, required=True)
    extract.add_argument("--to-response-id", type=int, required=True)
    extract.add_argument(
        "--failure-frame",
        type=int,
        help="optional debug frame id used to auto-populate Java-side assertions",
    )
    extract.add_argument(
        "--field",
        action="append",
        help=(
            "optional debug diff field to target; when set, the extractor picks the "
            "latest matching assertion in the selected response window"
        ),
    )
    extract.add_argument(
        "--field-lookback",
        type=int,
        default=DEFAULT_FIELD_LOOKBACK,
        help=(
            "when --field is used, automatically crop the extracted response window to the "
            "latest matching diff plus this many prior responses"
        ),
    )
    extract.add_argument("--out", type=Path, required=True)
    extract.set_defaults(func=cmd_extract)

    sanitize_lab = sub.add_parser(
        "sanitize-lab",
        help="strip live handoff/wait steps and assertions, keeping only the initial boss state",
    )
    sanitize_lab.add_argument("--fixture", type=Path, required=True)
    sanitize_lab.add_argument("--out", type=Path, required=True)
    sanitize_lab.set_defaults(func=cmd_sanitize_lab)

    minimize = sub.add_parser("minimize", help="shrink a failing fixture by narrowing assertions and trimming leading steps")
    minimize.add_argument("--fixture", type=Path, required=True)
    minimize.add_argument("--raw-path", type=Path, default=DEFAULT_RAW_PATH)
    minimize.add_argument("--debug-path", type=Path, default=DEFAULT_DEBUG_PATH)
    minimize.add_argument(
        "--repo-root",
        type=Path,
        default=Path(r"D:\rust\sts_simulator"),
        help="sts_simulator repo root used to run the replay harness",
    )
    minimize.add_argument(
        "--failure-frame",
        type=int,
        help="debug frame id for re-extracting assertions while trimming the window",
    )
    minimize.add_argument(
        "--field",
        help="optional assertion field to preserve, e.g. 'player.energy'",
    )
    minimize.add_argument(
        "--mode",
        choices=("same-outcome", "pass-only", "fail-only"),
        default="same-outcome",
        help="which outcome to preserve while minimizing",
    )
    minimize.add_argument(
        "--preserve-at-least-one-step",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="do not trim the response window below a single replay step",
    )
    minimize.add_argument("--out", type=Path, required=True)
    minimize.set_defaults(func=cmd_minimize)

    watch_list = sub.add_parser(
        "watch-list", help="list recent live watch captures from the audit jsonl files"
    )
    watch_list.add_argument(
        "--audit-path", type=Path, default=DEFAULT_WATCH_AUDIT_PATH
    )
    watch_list.add_argument(
        "--noncombat-audit-path", type=Path, default=DEFAULT_WATCH_NONCOMBAT_AUDIT_PATH
    )
    watch_list.add_argument(
        "--include-noncombat",
        action=argparse.BooleanOptionalAction,
        default=True,
    )
    watch_list.add_argument(
        "--tag",
        action="append",
        help="require a tag match; may be passed multiple times",
    )
    watch_list.add_argument(
        "--response-id",
        type=int,
        help="only show captures for a specific response id",
    )
    watch_list.add_argument(
        "--path-contains",
        help="only show captures whose out_path contains this substring",
    )
    watch_list.add_argument(
        "--require-existing-path",
        action=argparse.BooleanOptionalAction,
        default=False,
        help="skip audit rows whose out_path no longer exists on disk",
    )
    watch_list.add_argument("--limit", type=int, default=10)
    watch_list.set_defaults(func=cmd_watch_list)

    watch_minimize_latest = sub.add_parser(
        "watch-minimize-latest",
        help="minimize the most recent replayable combat watch capture",
    )
    watch_minimize_latest.add_argument(
        "--audit-path", type=Path, default=DEFAULT_WATCH_AUDIT_PATH
    )
    watch_minimize_latest.add_argument("--raw-path", type=Path, default=DEFAULT_RAW_PATH)
    watch_minimize_latest.add_argument("--debug-path", type=Path, default=DEFAULT_DEBUG_PATH)
    watch_minimize_latest.add_argument(
        "--repo-root",
        type=Path,
        default=Path(r"D:\rust\sts_simulator"),
        help="sts_simulator repo root used to run the replay harness",
    )
    watch_minimize_latest.add_argument(
        "--failure-frame",
        type=int,
        help="debug frame id for re-extracting assertions while trimming the window",
    )
    watch_minimize_latest.add_argument(
        "--field",
        help="optional assertion field to preserve, e.g. 'player.energy'",
    )
    watch_minimize_latest.add_argument(
        "--mode",
        choices=("same-outcome", "pass-only", "fail-only"),
        default="same-outcome",
        help="which outcome to preserve while minimizing",
    )
    watch_minimize_latest.add_argument(
        "--preserve-at-least-one-step",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="do not trim the response window below a single replay step",
    )
    watch_minimize_latest.add_argument(
        "--out",
        type=Path,
        help="output path for the minimized fixture; defaults to a .min.json sibling",
    )
    watch_minimize_latest.add_argument(
        "--tag",
        action="append",
        help="require a tag match; may be passed multiple times",
    )
    watch_minimize_latest.add_argument(
        "--response-id",
        type=int,
        help="only consider captures for a specific response id",
    )
    watch_minimize_latest.add_argument(
        "--path-contains",
        help="only consider captures whose out_path contains this substring",
    )
    watch_minimize_latest.set_defaults(func=cmd_watch_minimize_latest)
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
