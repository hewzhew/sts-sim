from __future__ import annotations

import json
import re
from collections import defaultdict
from pathlib import Path

try:
    from tools.analysis.common import ANALYSIS_CACHE_DIR, CACHE_VERSION, PROJECT_ROOT, ensure_dir, normalize_alias, now_iso
    from tools.analysis.java_facts import (
        BASE_CLASSES,
        detect_side_effects,
        extract_hardcoded_checks,
        iter_java_classes,
    )
    from tools.sts_tool.call_chain import analyze_method
    from tools.sts_tool.java_parser import JAVA_SRC, find_descendants, node_text
    from tools.sts_tool.rust_scan import (
        RUST_SRC,
        java_to_rust_name,
        scan_action_variants,
        scan_dispatch_arms,
        scan_power_variants,
        scan_relic_variants,
    )
except ImportError:
    from analysis.common import ANALYSIS_CACHE_DIR, CACHE_VERSION, PROJECT_ROOT, ensure_dir, normalize_alias, now_iso  # type: ignore
    from analysis.java_facts import (  # type: ignore
        BASE_CLASSES,
        detect_side_effects,
        extract_hardcoded_checks,
        iter_java_classes,
    )
    from sts_tool.call_chain import analyze_method  # type: ignore
    from sts_tool.java_parser import JAVA_SRC, find_descendants, node_text  # type: ignore
    from sts_tool.rust_scan import (  # type: ignore
        RUST_SRC,
        java_to_rust_name,
        scan_action_variants,
        scan_dispatch_arms,
        scan_power_variants,
        scan_relic_variants,
    )


COMPILED_SCHEMA_PATH = PROJECT_ROOT / "tools" / "compiled_protocol_schema.json"


def _write_json(path: Path, payload) -> Path:
    ensure_dir(path.parent)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False), encoding="utf-8")
    return path


def _load_compiled_schema(schema_path: Path = COMPILED_SCHEMA_PATH) -> dict:
    return json.loads(schema_path.read_text(encoding="utf-8"))


def _scan_rust_dispatch(rust_dir: Path) -> dict:
    relic_hooks_file = rust_dir / "content" / "relics" / "hooks.rs"
    power_file = rust_dir / "content" / "powers" / "mod.rs"

    return {
        "generated_at": now_iso(),
        "rust_dir": str(rust_dir),
        "action_variants": sorted(scan_action_variants(rust_dir)),
        "power_variants": sorted(scan_power_variants(rust_dir)),
        "relic_variants": sorted(scan_relic_variants(rust_dir)),
        "power_dispatch": {
            key: sorted(value)
            for key, value in scan_dispatch_arms(power_file, "resolve_power_").items()
        },
        "relic_dispatch": {
            key: sorted(value)
            for key, value in scan_dispatch_arms(relic_hooks_file, "on_").items()
        },
    }


def _inheritance_chain_for(class_name: str, superclass_by_class: dict[str, str | None]) -> list[str]:
    chain: list[str] = []
    current = superclass_by_class.get(class_name)
    seen: set[str] = set()
    while current and current not in seen:
        chain.append(current)
        seen.add(current)
        current = superclass_by_class.get(current)
    return chain


def _extract_power_analysis(class_text: str, methods: dict[str, dict]) -> dict:
    power_type_match = re.search(r"PowerType\.([A-Z_]+)", class_text)
    sentinel_amount = bool(
        re.search(r"(?:this\.)?amount\s*=\s*-1\b", class_text)
        or re.search(r"\bamount\s*=\s*-1\b", class_text)
        or re.search(r"\b-1\b", class_text) and "amount" in class_text
    )
    remove_patterns = (
        "RemoveSpecificPowerAction",
        ".removePower(",
        ".removeSpecificPower(",
        "removeDebuffs(",
    )
    removes_at_end_of_turn = any(
        method["name"] in ("atEndOfTurn", "atEndOfRound")
        and any(pattern in method["body"] for pattern in remove_patterns)
        for method in methods.values()
    )
    return {
        "power_type": power_type_match.group(1) if power_type_match else None,
        "uses_sentinel_amount": sentinel_amount,
        "removes_at_end_of_turn": removes_at_end_of_turn,
        "has_on_specific_trigger": "onSpecificTrigger" in methods,
        "has_stack_power": "stackPower" in methods,
    }


def _method_tags(
    cls,
    method_name: str,
    body_text: str,
    side_effects: list[str],
    hardcoded_checks: list[dict],
    call_chain,
) -> list[str]:
    tags: set[str] = set()
    if cls.category == "power" and "PowerType.DEBUFF" in body_text:
        tags.add("uses_debuff_gate")
    if "RemoveSpecificPowerAction" in body_text or ".removeSpecificPower(" in body_text:
        tags.add("uses_remove_specific_power")
    if re.search(r"\bamount\s*=\s*-1\b", body_text):
        tags.add("uses_sentinel_amount")
    if any(check["type"] == "hasPower" and normalize_alias(check["id"]) == "artifact" for check in hardcoded_checks):
        tags.add("checks_artifact")
    if any(creation.insertion == "addToTop" for creation in call_chain.creates):
        tags.add("queues_top")
    if any(creation.insertion == "addToBot" for creation in call_chain.creates):
        tags.add("queues_bottom")
    if method_name in {"stackPower", "onSpecificTrigger", "atEndOfTurn", "atEndOfRound"}:
        tags.add(f"hook:{method_name}")
    if any(effect.startswith("QUEUES_ACTIONS") for effect in side_effects):
        tags.add("has_side_effect_queue")
    return sorted(tags)


def _build_method_record(cls, ctx, rel_path: str, method, is_live: bool, java_dir: Path) -> dict:
    body_text = ctx.source[method.node.start_byte:method.node.end_byte]
    call_chain = analyze_method(method.body_node, ctx.source) if method.body_node else None
    if call_chain:
        call_chain.class_name = cls.name
        call_chain.method_name = method.name
        call_chain.file_path = rel_path
    side_effects = detect_side_effects(method.node, ctx.source)
    hardcoded_checks = extract_hardcoded_checks(method.node, ctx.source)
    creates = []
    virtual_dispatches = []
    entity_checks = []
    significant_calls = []
    guards = []
    if call_chain:
        creates = [
            {
                "class_name": creation.class_name,
                "full_text": creation.full_text,
                "line": creation.line,
                "insertion": creation.insertion,
            }
            for creation in call_chain.creates
        ]
        virtual_dispatches = [
            {
                "method_name": dispatch.method_name,
                "iterable": dispatch.iterable,
                "receiver_type": dispatch.receiver_type,
                "line": dispatch.line,
            }
            for dispatch in call_chain.virtual_dispatches
        ]
        entity_checks = [check.__dict__ for check in call_chain.entity_checks]
        significant_calls = [call.__dict__ for call in call_chain.significant_calls]
        guards = list(call_chain.guards)

    return {
        "class_name": cls.name,
        "category": cls.category,
        "superclass": cls.superclass,
        "file": rel_path,
        "string_id": cls.string_id,
        "live": is_live,
        "name": method.name,
        "signature": f"{method.name}{method.params}",
        "params": method.params,
        "return_type": method.return_type,
        "start_line": method.start_line,
        "end_line": method.end_line,
        "side_effects": side_effects,
        "hardcoded_checks": hardcoded_checks,
        "body": body_text,
        "call_chain": {
            "creates": creates,
            "virtual_dispatches": virtual_dispatches,
            "entity_checks": entity_checks,
            "significant_calls": significant_calls,
            "guards": guards,
            "formatted": _render_lightweight_call_chain(cls.name, method.name, creates, virtual_dispatches, entity_checks, significant_calls, guards),
        },
        "normalized_tags": _method_tags(cls, method.name, body_text, side_effects, hardcoded_checks, call_chain) if call_chain else [],
    }


def _render_lightweight_call_chain(
    class_name: str,
    method_name: str,
    creates: list[dict],
    virtual_dispatches: list[dict],
    entity_checks: list[dict],
    significant_calls: list[dict],
    guards: list[str],
) -> str:
    lines = [f"{class_name}.{method_name}()"]
    for guard in guards:
        lines.append(f"  ├─ guard: {guard}")
    for check in entity_checks:
        lines.append(f"  ├─ checks: {check['check_type']}(\"{check['entity_id']}\") L{check['line']}")
    for creation in creates:
        insertion = f" [{creation['insertion']}]" if creation["insertion"] != "unknown" else ""
        lines.append(f"  ├─ creates: {creation['class_name']}{insertion} L{creation['line']}")
    for dispatch in virtual_dispatches:
        lines.append(
            f"  ├─ iterates: {dispatch['iterable']} -> {dispatch['receiver_type']}.{dispatch['method_name']}()"
        )
    for call in significant_calls:
        lines.append(f"  ├─ calls: {call['full_text']} L{call['line']}")
    return "\n".join(lines)


def _scan_java_cache(
    java_dir: Path,
) -> tuple[list[tuple[Path, object, object]], list[dict], list[dict], dict[str, list[dict]], dict[str, list[dict]]]:
    raw_rows = []
    superclass_by_class: dict[str, str | None] = {}
    instantiated_classes: set[str] = set()
    for fpath, ctx, cls in iter_java_classes(java_dir):
        raw_rows.append((fpath, ctx, cls))
        superclass_by_class[cls.name] = cls.superclass
        instantiated_classes.update(re.findall(r"\bnew\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(", ctx.source))

    entities: list[dict] = []
    methods: list[dict] = []
    hook_bases: dict[str, list[dict]] = {}
    hook_overrides: dict[str, list[dict]] = {}

    for fpath, ctx, cls in raw_rows:
        rel_path = str(fpath.relative_to(java_dir))
        class_nodes = find_descendants(ctx.tree.root_node, "class_declaration")
        class_text = node_text(class_nodes[0], ctx.source) if class_nodes else ctx.source
        is_live = cls.name in instantiated_classes

        entity_methods: list[dict] = []
        for method in cls.methods.values():
            record = _build_method_record(cls, ctx, rel_path, method, is_live, java_dir)
            methods.append(record)
            entity_methods.append(
                {
                    "name": record["name"],
                    "signature": record["signature"],
                    "return_type": record["return_type"],
                    "start_line": record["start_line"],
                    "end_line": record["end_line"],
                    "side_effects": record["side_effects"],
                    "normalized_tags": record["normalized_tags"],
                }
            )

            hook_row = {
                "class": cls.name,
                "superclass": cls.superclass,
                "category": cls.category,
                "file": rel_path,
                "params": method.params,
                "return_type": method.return_type,
                "start_line": method.start_line,
                "end_line": method.end_line,
                "side_effects": record["side_effects"],
                "body": record["body"],
                "string_id": cls.string_id,
                "live": is_live,
            }
            if cls.name in BASE_CLASSES:
                hook_bases.setdefault(method.name, []).append(hook_row)
            elif method.name in hook_bases or method.name.startswith(("on", "at", "can", "trigger")):
                hook_overrides.setdefault(method.name, []).append(hook_row)

        analysis = {}
        if cls.category == "power":
            analysis = _extract_power_analysis(class_text, {method["name"]: method for method in methods if method["class_name"] == cls.name})
        entities.append(
            {
                "class_name": cls.name,
                "superclass": cls.superclass,
                "inheritance_chain": _inheritance_chain_for(cls.name, superclass_by_class),
                "category": cls.category,
                "file": rel_path,
                "string_id": cls.string_id,
                "live": is_live,
                "rust_variant": java_to_rust_name(cls.name, cls.category),
                "normalized_aliases": sorted(
                    {
                        alias
                        for alias in (
                            normalize_alias(cls.name),
                            normalize_alias(cls.string_id or ""),
                        )
                        if alias
                    }
                ),
                "analysis": analysis,
                "methods": entity_methods,
            }
        )

    return raw_rows, entities, methods, hook_bases, hook_overrides


def _scan_hook_callsites(raw_rows: list[tuple[Path, object, object]], java_dir: Path, hook_names: set[str]) -> dict[str, list[dict]]:
    callsites: dict[str, list[dict]] = {name: [] for name in sorted(hook_names)}

    for fpath, ctx, cls in raw_rows:
        rel_path = str(fpath.relative_to(java_dir))
        lines = ctx.source.splitlines()
        method_checks: dict[str, list[dict]] = {}
        for invocation in find_descendants(ctx.tree.root_node, "method_invocation"):
            hook_name = None
            for child in invocation.children:
                if child.type == "identifier":
                    candidate = ctx.source[child.start_byte:child.end_byte]
                    if candidate in hook_names:
                        hook_name = candidate
            if not hook_name:
                continue

            enclosing_node = invocation
            while enclosing_node and enclosing_node.type != "method_declaration":
                enclosing_node = enclosing_node.parent
            enclosing = "<unknown>"
            if enclosing_node:
                for child in enclosing_node.children:
                    if child.type == "identifier":
                        enclosing = ctx.source[child.start_byte:child.end_byte]
                        break
            if enclosing not in method_checks:
                method_checks[enclosing] = extract_hardcoded_checks(enclosing_node, ctx.source)

            current = invocation.parent
            iterable = None
            while current:
                if current.type == "enhanced_for_statement":
                    for i, child in enumerate(current.children):
                        if child.type == ":" and i + 1 < len(current.children):
                            for j in range(i + 1, len(current.children)):
                                nxt = current.children[j]
                                if nxt.type not in ("{", "}", "block"):
                                    iterable = ctx.source[nxt.start_byte:nxt.end_byte].strip()
                                    break
                    break
                if current.type == "method_declaration":
                    break
                current = current.parent

            line_no = invocation.start_point[0] + 1
            line_idx = line_no - 1
            ctx_start = max(0, line_idx - 2)
            ctx_end = min(len(lines), line_idx + 3)
            context = "\n".join(
                (">>> " if i == line_idx else "    ") + lines[i]
                for i in range(ctx_start, ctx_end)
            )

            callsites[hook_name].append(
                {
                    "class": cls.name,
                    "category": cls.category,
                    "file": rel_path,
                    "line": line_no,
                    "call_text": ctx.source[invocation.start_byte:invocation.end_byte],
                    "context": context,
                    "enclosing": enclosing,
                    "iterable": iterable,
                    "hardcoded_checks": method_checks[enclosing],
                }
            )

    return {name: rows for name, rows in callsites.items() if rows}


def _build_schema_aliases(schema: dict) -> dict:
    def display_alias(raw: str) -> str:
        raw = raw.replace("_", " ").replace("-", " ")
        raw = re.sub(r"([a-z0-9])([A-Z])", r"\1 \2", raw)
        return re.sub(r"\s+", " ", raw).strip()

    aliases: list[dict] = []
    normalized_index: dict[str, list[dict]] = defaultdict(list)
    for enum_key, enum_payload in schema.get("enums", {}).items():
        for rust_name, entry in enum_payload.get("entries", {}).items():
            raw_values = set(entry.get("java", []))
            raw_values.add(rust_name)
            raw_values.add(display_alias(rust_name))
            for raw in sorted(str(value) for value in raw_values if value):
                normalized = normalize_alias(raw)
                sources = list(entry.get("sources", []))
                if raw not in entry.get("java", []) and raw != rust_name:
                    sources = sources + ["generated_display_alias"]
                row = {
                    "enum": enum_key,
                    "rust": rust_name,
                    "raw": raw,
                    "normalized": normalized,
                    "sources": sources,
                    "status": entry.get("status"),
                }
                aliases.append(row)
                normalized_index[normalized].append(
                    {
                        "enum": enum_key,
                        "rust": rust_name,
                        "raw": raw,
                    }
                )

    conflicts = {
        normalized: rows
        for normalized, rows in sorted(normalized_index.items())
        if normalized and len({(row["enum"], row["rust"], row["raw"]) for row in rows}) > 1
    }

    return {
        "generated_at": now_iso(),
        "schema_path": str(COMPILED_SCHEMA_PATH),
        "aliases": sorted(aliases, key=lambda item: (item["enum"], item["rust"], item["raw"])),
        "normalized_index": dict(sorted(normalized_index.items())),
        "conflicts": conflicts,
    }


def build_analysis_cache(
    java_dir: Path = JAVA_SRC,
    rust_dir: Path = RUST_SRC,
    out_dir: Path = ANALYSIS_CACHE_DIR,
) -> dict[str, Path]:
    ensure_dir(out_dir)

    schema = _load_compiled_schema()
    raw_rows, entities, methods, hook_bases, hook_overrides = _scan_java_cache(java_dir)
    hook_names = set(hook_bases) | set(hook_overrides)
    hook_callsites = _scan_hook_callsites(raw_rows, java_dir, hook_names)
    rust_payload = _scan_rust_dispatch(rust_dir)
    schema_aliases = _build_schema_aliases(schema)

    entity_payload = {
        "generated_at": now_iso(),
        "java_dir": str(java_dir),
        "entities": sorted(entities, key=lambda item: item["class_name"]),
    }
    methods_payload = {
        "generated_at": now_iso(),
        "java_dir": str(java_dir),
        "methods": sorted(methods, key=lambda item: (item["class_name"], item["start_line"], item["name"])),
    }
    hook_payload = {
        "generated_at": now_iso(),
        "java_dir": str(java_dir),
        "hooks": {
            name: {
                "base_definitions": hook_bases.get(name, []),
                "overrides": hook_overrides.get(name, []),
            }
            for name in sorted(hook_names)
        },
    }
    callsite_payload = {
        "generated_at": now_iso(),
        "java_dir": str(java_dir),
        "callsites": hook_callsites,
    }
    manifest = {
        "version": CACHE_VERSION,
        "generated_at": now_iso(),
        "java_dir": str(java_dir),
        "rust_dir": str(rust_dir),
        "schema_path": str(COMPILED_SCHEMA_PATH),
        "files": {
            "java_entities": "java_entities.json",
            "java_methods": "java_methods.json",
            "java_hooks": "java_hooks.json",
            "java_callsites": "java_callsites.json",
            "rust_dispatch": "rust_dispatch.json",
            "schema_aliases": "schema_aliases.json",
        },
    }

    return {
        "java_entities": _write_json(out_dir / "java_entities.json", entity_payload),
        "java_methods": _write_json(out_dir / "java_methods.json", methods_payload),
        "java_hooks": _write_json(out_dir / "java_hooks.json", hook_payload),
        "java_callsites": _write_json(out_dir / "java_callsites.json", callsite_payload),
        "rust_dispatch": _write_json(out_dir / "rust_dispatch.json", rust_payload),
        "schema_aliases": _write_json(out_dir / "schema_aliases.json", schema_aliases),
        "manifest": _write_json(out_dir / "manifest.json", manifest),
    }


def load_cache(cache_dir: Path = ANALYSIS_CACHE_DIR) -> dict:
    files = {
        "manifest": cache_dir / "manifest.json",
        "java_entities": cache_dir / "java_entities.json",
        "java_methods": cache_dir / "java_methods.json",
        "java_hooks": cache_dir / "java_hooks.json",
        "java_callsites": cache_dir / "java_callsites.json",
        "rust_dispatch": cache_dir / "rust_dispatch.json",
        "schema_aliases": cache_dir / "schema_aliases.json",
    }
    manifest_ok = False
    if files["manifest"].exists():
        try:
            manifest = json.loads(files["manifest"].read_text(encoding="utf-8"))
            manifest_ok = manifest.get("version") == CACHE_VERSION
        except json.JSONDecodeError:
            manifest_ok = False
    if not manifest_ok or not all(path.exists() for key, path in files.items() if key != "manifest"):
        build_analysis_cache(out_dir=cache_dir)
    return {
        key: json.loads(path.read_text(encoding="utf-8"))
        for key, path in files.items()
    }
