#!/usr/bin/env python3
import json
import sys
from collections import defaultdict
from pathlib import Path


CARD_ARRAY_KEYS = {
    "hand",
    "draw_pile",
    "discard_pile",
    "exhaust_pile",
    "master_deck",
}
CARD_ID_ARRAY_KEYS = {
    "draw_pile_ids",
    "discard_pile_ids",
    "exhaust_pile_ids",
    "deck",
}


def add_observed(bucket, category: str, value: str, field: str, source: str):
    if not value:
        return
    entry = bucket[category][value]
    entry["count"] += 1
    entry["fields"].add(field)
    entry["sources"].add(source)


def walk_snapshot(obj, bucket, source: str, path: str = "root"):
    if isinstance(obj, dict):
        for key, value in obj.items():
            child_path = f"{path}.{key}"

            if key == "powers" and isinstance(value, list):
                for item in value:
                    if isinstance(item, dict):
                        add_observed(
                            bucket,
                            "power",
                            item.get("id", ""),
                            f"{child_path}.id",
                            source,
                        )
                        walk_snapshot(item, bucket, source, child_path)
                continue

            if key == "relics" and isinstance(value, list):
                for item in value:
                    if isinstance(item, dict):
                        add_observed(
                            bucket,
                            "relic",
                            item.get("id", ""),
                            f"{child_path}.id",
                            source,
                        )
                        walk_snapshot(item, bucket, source, child_path)
                continue

            if key == "potions" and isinstance(value, list):
                for item in value:
                    if isinstance(item, dict):
                        add_observed(
                            bucket,
                            "potion",
                            item.get("id", ""),
                            f"{child_path}.id",
                            source,
                        )
                        walk_snapshot(item, bucket, source, child_path)
                continue

            if key == "monsters" and isinstance(value, list):
                for item in value:
                    if isinstance(item, dict):
                        add_observed(
                            bucket,
                            "monster",
                            item.get("id", ""),
                            f"{child_path}.id",
                            source,
                        )
                        walk_snapshot(item, bucket, source, child_path)
                continue

            if key in CARD_ARRAY_KEYS and isinstance(value, list):
                for item in value:
                    if isinstance(item, dict):
                        add_observed(
                            bucket,
                            "card",
                            item.get("id", ""),
                            f"{child_path}.id",
                            source,
                        )
                        walk_snapshot(item, bucket, source, child_path)
                continue

            if key in CARD_ID_ARRAY_KEYS and isinstance(value, list):
                for item in value:
                    if isinstance(item, str):
                        add_observed(bucket, "card", item, child_path, source)
                continue

            if key in {"card_id", "cardId"} and isinstance(value, str):
                add_observed(bucket, "card", value, child_path, source)
            elif key in {"power_id", "powerId"} and isinstance(value, str):
                add_observed(bucket, "power", value, child_path, source)
            elif key in {"relic_id", "relicId"} and isinstance(value, str):
                add_observed(bucket, "relic", value, child_path, source)
            elif key in {"potion_id", "potionId"} and isinstance(value, str):
                add_observed(bucket, "potion", value, child_path, source)
            elif key in {"monster_id", "monsterId"} and isinstance(value, str):
                add_observed(bucket, "monster", value, child_path, source)

            walk_snapshot(value, bucket, source, child_path)

    elif isinstance(obj, list):
        for idx, item in enumerate(obj):
            walk_snapshot(item, bucket, source, f"{path}[{idx}]")


def scan_replay(path: Path, bucket):
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            line = line.strip()
            if not line:
                continue
            try:
                payload = json.loads(line)
            except json.JSONDecodeError:
                continue
            walk_snapshot(payload, bucket, path.name, f"line[{line_number}]")


def finalize(bucket):
    result = {}
    for category, values in bucket.items():
        category_out = {}
        for identifier, data in sorted(values.items()):
            category_out[identifier] = {
                "count": data["count"],
                "fields": sorted(data["fields"]),
                "sources": sorted(data["sources"]),
            }
        result[category] = category_out
    return result


def main():
    if len(sys.argv) < 3:
        print(
            "Usage: python observed_id_extractor.py <output.json> <input1.jsonl> [<input2.jsonl> ...]"
        )
        sys.exit(1)

    output = Path(sys.argv[1])
    inputs = [Path(arg) for arg in sys.argv[2:]]

    bucket = defaultdict(
        lambda: defaultdict(lambda: {"count": 0, "fields": set(), "sources": set()})
    )
    scanned = []
    for candidate in inputs:
        if candidate.exists() and candidate.is_file():
            scan_replay(candidate, bucket)
            scanned.append(str(candidate))

    payload = {
        "version": "1.0",
        "description": "Observed Java IDs extracted from replay/live_comm samples",
        "inputs": scanned,
        "categories": finalize(bucket),
    }
    output.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    total = sum(len(values) for values in payload["categories"].values())
    print(f"Observed ID extraction complete. {total} unique IDs written to {output}")


if __name__ == "__main__":
    main()
