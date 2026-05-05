from __future__ import annotations

import json
from pathlib import Path

try:
    from tools.analysis.cache_builder import build_analysis_cache, load_cache
    from tools.analysis.common import ANALYSIS_CACHE_DIR, normalize_alias
except ImportError:
    from analysis.cache_builder import build_analysis_cache, load_cache  # type: ignore
    from analysis.common import ANALYSIS_CACHE_DIR, normalize_alias  # type: ignore


def _entity_rust_status(entity: dict, rust_dispatch: dict) -> dict:
    category = entity.get("category")
    rust_variant = entity.get("rust_variant")
    if category == "power":
        dispatch_source = rust_dispatch.get("power_dispatch", {})
        variant_set = set(rust_dispatch.get("power_variants", []))
    elif category == "relic":
        dispatch_source = rust_dispatch.get("relic_dispatch", {})
        variant_set = set(rust_dispatch.get("relic_variants", []))
    elif category == "action":
        dispatch_source = {}
        variant_set = set(rust_dispatch.get("action_variants", []))
    else:
        dispatch_source = {}
        variant_set = set()

    dispatch_functions = sorted(
        fn_name for fn_name, variants in dispatch_source.items() if rust_variant in variants
    )
    if not entity.get("live", True):
        status = "dead"
    elif dispatch_functions:
        status = "implemented"
    elif rust_variant in variant_set:
        status = "variant_only"
    elif rust_variant:
        status = "missing"
    else:
        status = "unknown"
    return {
        "rust_variant": rust_variant,
        "dispatch_functions": dispatch_functions,
        "status": status,
    }


def _schema_aliases_for_entity(entity: dict, schema_aliases: dict) -> list[dict]:
    rust_variant = entity.get("rust_variant")
    category = entity.get("category")
    enum_map = {
        "card": "card_id",
        "power": "power_id",
        "relic": "relic_id",
        "monster": "monster_id",
        "potion": "potion_id",
    }
    target_enum = enum_map.get(category)
    if not target_enum or not rust_variant:
        return []
    return [
        alias
        for alias in schema_aliases.get("aliases", [])
        if alias["enum"] == target_enum and alias["rust"] == rust_variant
    ]


def _entity_match_score(entity: dict, term: str, schema_aliases: dict) -> int:
    normalized_term = normalize_alias(term)
    candidates = [
        entity.get("class_name", ""),
        entity.get("string_id", "") or "",
        entity.get("file", ""),
        entity.get("rust_variant", "") or "",
    ]
    candidates.extend(alias["raw"] for alias in _schema_aliases_for_entity(entity, schema_aliases))
    normalized_candidates = {normalize_alias(candidate) for candidate in candidates if candidate}
    if normalized_term in normalized_candidates:
        return 100
    if any(normalized_term and normalized_term in candidate for candidate in normalized_candidates):
        return 75
    raw_term = term.lower()
    if any(raw_term and raw_term in candidate.lower() for candidate in candidates if candidate):
        return 50
    return 0


def _match_entities(cache: dict, term: str) -> list[dict]:
    entities = cache["java_entities"]["entities"]
    schema_aliases = cache["schema_aliases"]
    scored = [
        (entity, _entity_match_score(entity, term, schema_aliases))
        for entity in entities
    ]
    return [
        entity
        for entity, score in sorted(
            (row for row in scored if row[1] > 0),
            key=lambda item: (-item[1], item[0]["class_name"]),
        )
    ]


def _methods_for_entity(cache: dict, class_name: str) -> list[dict]:
    return [
        method
        for method in cache["java_methods"]["methods"]
        if method["class_name"] == class_name
    ]


def build_query_result(name: str, cache_dir: Path = ANALYSIS_CACHE_DIR, rebuild: bool = False) -> dict:
    if rebuild:
        build_analysis_cache(out_dir=cache_dir)
    cache = load_cache(cache_dir)
    matched_entities = _match_entities(cache, name)
    rust_dispatch = cache["rust_dispatch"]
    results = []
    for entity in matched_entities:
        methods = _methods_for_entity(cache, entity["class_name"])
        results.append(
            {
                "entity": entity,
                "rust": _entity_rust_status(entity, rust_dispatch),
                "schema_aliases": _schema_aliases_for_entity(entity, cache["schema_aliases"]),
                "methods": methods,
            }
        )
    return {
        "query": name,
        "cache_version": cache["manifest"]["version"],
        "results": results,
    }


def render_query_markdown(result: dict) -> str:
    lines = [f"# Query: `{result['query']}`", ""]
    if not result["results"]:
        lines.append("*No matches found.*")
        return "\n".join(lines)
    for row in result["results"]:
        entity = row["entity"]
        rust = row["rust"]
        lines.append(f"## `{entity['class_name']}`")
        lines.append("")
        lines.append(f"- Category: `{entity['category']}`")
        lines.append(f"- File: `{entity['file']}`")
        if entity.get("string_id"):
            lines.append(f"- String ID: `{entity['string_id']}`")
        if entity.get("superclass"):
            lines.append(f"- Superclass: `{entity['superclass']}`")
        if entity.get("inheritance_chain"):
            lines.append(f"- Inheritance: `{ ' -> '.join(entity['inheritance_chain']) }`")
        lines.append(f"- Live-instantiated: `{entity.get('live', True)}`")
        lines.append(f"- Rust parity: `{rust['status']}`")
        if rust.get("rust_variant"):
            lines.append(f"- Rust variant: `{rust['rust_variant']}`")
        if rust["dispatch_functions"]:
            lines.append(f"- Dispatch: `{', '.join(rust['dispatch_functions'])}`")
        if entity.get("analysis"):
            for key, value in sorted(entity["analysis"].items()):
                lines.append(f"- {key}: `{value}`")
        if row["schema_aliases"]:
            lines.append("- Schema aliases:")
            for alias in row["schema_aliases"]:
                lines.append(f"  - `{alias['raw']}` -> `{alias['normalized']}`")
        lines.append("")
        lines.append("### Methods")
        lines.append("")
        for method in row["methods"]:
            lines.append(
                f"- `{method['signature']}` L{method['start_line']}-{method['end_line']} "
                f"tags={','.join(method.get('normalized_tags', [])) or 'none'}"
            )
        lines.append("")
    return "\n".join(lines)


def build_find_result(term: str, cache_dir: Path = ANALYSIS_CACHE_DIR, rebuild: bool = False) -> dict:
    query = build_query_result(term, cache_dir, rebuild)
    return {
        "query": term,
        "matches": [
            {
                "class_name": row["entity"]["class_name"],
                "category": row["entity"]["category"],
                "file": row["entity"]["file"],
                "string_id": row["entity"].get("string_id"),
            }
            for row in query["results"]
        ],
    }


def render_find_markdown(result: dict) -> str:
    lines = [f"# Find: `{result['query']}`", ""]
    if not result["matches"]:
        lines.append("*No matches found.*")
        return "\n".join(lines)
    for match in result["matches"]:
        line = f"- `{match['class_name']}` ({match['category']}) — `{match['file']}`"
        if match.get("string_id"):
            line += f" id=`{match['string_id']}`"
        lines.append(line)
    return "\n".join(lines)


def build_override_result(hook_name: str, cache_dir: Path = ANALYSIS_CACHE_DIR, rebuild: bool = False) -> dict:
    if rebuild:
        build_analysis_cache(out_dir=cache_dir)
    cache = load_cache(cache_dir)
    hook_entry = cache["java_hooks"]["hooks"].get(hook_name, {"base_definitions": [], "overrides": []})
    return {
        "hook_name": hook_name,
        "base_definitions": hook_entry.get("base_definitions", []),
        "overrides": hook_entry.get("overrides", []),
        "callsites": cache["java_callsites"]["callsites"].get(hook_name, []),
    }


def render_override_markdown(result: dict) -> str:
    lines = [f"# Overrides: `{result['hook_name']}`", ""]
    lines.append(f"- Base definitions: {len(result['base_definitions'])}")
    lines.append(f"- Overrides: {len(result['overrides'])}")
    lines.append(f"- Callsites: {len(result['callsites'])}")
    lines.append("")
    for override in result["overrides"]:
        lines.append(
            f"- `{override['class']}` ({override['category']}) — `{override['file']}` "
            f"L{override['start_line']}-{override['end_line']}"
        )
    return "\n".join(lines)


def build_inspect_result(
    name: str,
    method_name: str | None,
    cache_dir: Path = ANALYSIS_CACHE_DIR,
    rebuild: bool = False,
) -> dict:
    query = build_query_result(name, cache_dir, rebuild)
    if not query["results"]:
        return {"query": name, "matches": []}
    matches = []
    for row in query["results"]:
        methods = row["methods"]
        if method_name:
            methods = [method for method in methods if method["name"] == method_name]
        matches.append(
            {
                "entity": row["entity"],
                "methods": methods,
            }
        )
    return {"query": name, "method": method_name, "matches": matches}


def render_inspect_markdown(result: dict) -> str:
    lines = [f"# Inspect: `{result['query']}`", ""]
    if not result["matches"]:
        lines.append("*No matches found.*")
        return "\n".join(lines)
    for match in result["matches"]:
        entity = match["entity"]
        lines.append(f"## `{entity['class_name']}`")
        lines.append("")
        for method in match["methods"]:
            lines.append(f"### `{method['signature']}`")
            lines.append("")
            lines.append("```java")
            lines.append(method["body"])
            lines.append("```")
            lines.append("")
    return "\n".join(lines)


def print_result(result: dict, as_json: bool, renderer) -> None:
    if as_json:
        print(json.dumps(result, indent=2, ensure_ascii=False))
        return
    print(renderer(result))
