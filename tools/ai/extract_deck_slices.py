import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("inputs", nargs="+", type=Path)
    parser.add_argument(
        "--source-kind",
        choices=["live", "replay", "authored_probe", "weak_policy"],
        default="replay",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=ROOT / "tools" / "artifacts" / "deck_slices.jsonl",
    )
    return parser.parse_args()


def iter_json_values(path: Path):
    text = path.read_text(encoding="utf-8")
    stripped = text.lstrip()
    if not stripped:
        return

    if stripped[0] in "[{":
        try:
            value = json.loads(text)
        except json.JSONDecodeError:
            value = None
        if isinstance(value, list):
            for idx, item in enumerate(value):
                yield idx, item
            return
        if isinstance(value, dict):
            yield 0, value
            return

    for idx, line in enumerate(text.splitlines()):
        line = line.strip()
        if not line:
            continue
        try:
            yield idx, json.loads(line)
        except json.JSONDecodeError:
            continue


def walk_dicts(value: Any):
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from walk_dicts(child)
    elif isinstance(value, list):
        for child in value:
            yield from walk_dicts(child)


def card_from_value(value: Any) -> dict[str, Any] | None:
    if isinstance(value, str):
        return {"id": value, "upgrades": 0}
    if not isinstance(value, dict):
        return None

    card_id = (
        value.get("id")
        or value.get("card_id")
        or value.get("cardId")
        or value.get("name")
    )
    if not isinstance(card_id, str):
        return None
    upgrades = value.get("upgrades", value.get("upgrade_count", value.get("times_upgraded", 0)))
    try:
        upgrades = int(upgrades)
    except (TypeError, ValueError):
        upgrades = 0
    return {"id": card_id, "upgrades": upgrades}


def deck_from_value(value: Any) -> list[dict[str, Any]] | None:
    if not isinstance(value, list) or len(value) < 5:
        return None
    cards = []
    for item in value:
        card = card_from_value(item)
        if card is None:
            return None
        cards.append(card)
    ids = {card["id"] for card in cards}
    if not ({"Strike_R", "Defend_R"} & ids or {"Strike", "Defend"} & ids):
        return None
    return cards


def find_deck_payload(root: Any) -> tuple[list[dict[str, Any]], dict[str, Any]] | None:
    for obj in walk_dicts(root):
        for key in ("deck", "master_deck", "masterDeck"):
            deck = deck_from_value(obj.get(key))
            if deck is not None:
                return deck, obj
    return None


def first_int(*values: Any) -> int | None:
    for value in values:
        try:
            if value is not None:
                return int(value)
        except (TypeError, ValueError):
            continue
    return None


def make_record(path: Path, frame: int, source_kind: str, root: Any):
    found = find_deck_payload(root)
    if found is None:
        return None
    deck, context = found
    return {
        "schema": "sts.deck_slice.v0",
        "source_kind": source_kind,
        "source_path": str(path),
        "source_frame": frame,
        "character": context.get("character") or context.get("class") or context.get("player_class"),
        "act": first_int(context.get("act"), context.get("act_num")),
        "floor": first_int(context.get("floor"), context.get("floor_num")),
        "hp": first_int(context.get("hp"), context.get("current_hp"), context.get("currentHp")),
        "max_hp": first_int(context.get("max_hp"), context.get("maxHp"), context.get("max_health")),
        "gold": first_int(context.get("gold")),
        "deck": deck,
        "notes": [],
    }


def main():
    args = parse_args()
    records = []
    for path in args.inputs:
        for frame, value in iter_json_values(path):
            record = make_record(path, frame, args.source_kind, value)
            if record is not None:
                records.append(record)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8", newline="\n") as handle:
        for record in records:
            handle.write(json.dumps(record, ensure_ascii=False, separators=(",", ":")) + "\n")

    print(f"wrote={len(records)} output={args.output}")


if __name__ == "__main__":
    main()
