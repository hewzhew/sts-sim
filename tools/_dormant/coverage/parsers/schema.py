"""Parse schema_dump.json to get Java class → Java ID → Rust enum mapping."""
import json
from pathlib import Path
from ..models import EntityCategory


def _category_from_str(s: str) -> EntityCategory | None:
    mapping = {
        "power": EntityCategory.POWER,
        "relic": EntityCategory.RELIC,
        "card": EntityCategory.CARD,
        "potion": EntityCategory.POTION,
        "monster": EntityCategory.MONSTER,
    }
    return mapping.get(s.lower())


def parse_schema_dump(schema_path: Path) -> list[dict]:
    """Parse schema_dump.json → list of entity dicts.

    Each entry has: class_name, id_field, category, file
    Returns raw list for flexible downstream use.
    """
    if not schema_path.exists():
        return []

    text = schema_path.read_text(encoding="utf-8-sig")
    data = json.loads(text)
    return data.get("entities", [])


def build_class_to_id_map(entities: list[dict], category: str | None = None) -> dict[str, str]:
    """Build {Java class_name: Java id_field} map, optionally filtered by category."""
    result = {}
    for e in entities:
        if category and e.get("category", "").lower() != category.lower():
            continue
        result[e["class_name"]] = e.get("id_field", e["class_name"])
    return result


def build_id_to_class_map(entities: list[dict], category: str | None = None) -> dict[str, str]:
    """Build {Java id_field: Java class_name} map."""
    result = {}
    for e in entities:
        if category and e.get("category", "").lower() != category.lower():
            continue
        result[e.get("id_field", e["class_name"])] = e["class_name"]
    return result


def build_class_to_file_map(entities: list[dict], category: str | None = None) -> dict[str, str]:
    """Build {Java class_name: relative file path} map."""
    result = {}
    for e in entities:
        if category and e.get("category", "").lower() != category.lower():
            continue
        result[e["class_name"]] = e.get("file", "")
    return result
