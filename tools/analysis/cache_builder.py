from __future__ import annotations

import json
from pathlib import Path

try:
    from tools.analysis.common import ANALYSIS_CACHE_DIR, ensure_dir, now_iso
    from tools.analysis.java_facts import (
        BASE_CLASSES,
        detect_side_effects,
        extract_hardcoded_checks,
        is_class_instantiated,
        iter_java_classes,
    )
    from tools.sts_tool.java_parser import JAVA_SRC, find_descendants
    from tools.sts_tool.rust_scan import (
        RUST_SRC,
        scan_action_variants,
        scan_dispatch_arms,
        scan_power_variants,
        scan_relic_variants,
    )
except ImportError:
    from analysis.common import ANALYSIS_CACHE_DIR, ensure_dir, now_iso  # type: ignore
    from analysis.java_facts import (  # type: ignore
        BASE_CLASSES,
        detect_side_effects,
        extract_hardcoded_checks,
        is_class_instantiated,
        iter_java_classes,
    )
    from sts_tool.java_parser import JAVA_SRC, find_descendants  # type: ignore
    from sts_tool.rust_scan import (  # type: ignore
        RUST_SRC,
        scan_action_variants,
        scan_dispatch_arms,
        scan_power_variants,
        scan_relic_variants,
    )


def _write_json(path: Path, payload) -> Path:
    ensure_dir(path.parent)
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False), encoding="utf-8")
    return path


def _scan_java_entities(java_dir: Path) -> tuple[list[dict], dict[str, list[dict]], dict[str, list[dict]]]:
    entities: list[dict] = []
    hook_bases: dict[str, list[dict]] = {}
    hook_overrides: dict[str, list[dict]] = {}

    for fpath, ctx, cls in iter_java_classes(java_dir):
        rel_path = str(fpath.relative_to(java_dir))
        entity = {
            "class_name": cls.name,
            "superclass": cls.superclass,
            "category": cls.category,
            "file": rel_path,
            "string_id": cls.string_id,
            "methods": [],
        }
        is_live = is_class_instantiated(cls.name, java_dir)

        for method in cls.methods.values():
            side_effects = detect_side_effects(method.node, ctx.source)
            method_fact = {
                "name": method.name,
                "params": method.params,
                "return_type": method.return_type,
                "start_line": method.start_line,
                "end_line": method.end_line,
                "side_effects": side_effects,
            }
            entity["methods"].append(method_fact)

            hook_row = {
                "class": cls.name,
                "superclass": cls.superclass,
                "category": cls.category,
                "file": rel_path,
                "params": method.params,
                "return_type": method.return_type,
                "start_line": method.start_line,
                "end_line": method.end_line,
                "side_effects": side_effects,
                "body": ctx.source[method.node.start_byte:method.node.end_byte],
                "string_id": cls.string_id,
                "live": is_live,
            }

            if cls.name in BASE_CLASSES:
                hook_bases.setdefault(method.name, []).append(hook_row)
            elif method.name in hook_bases or method.name.startswith(("on", "at", "can", "trigger")):
                hook_overrides.setdefault(method.name, []).append(hook_row)

        entities.append(entity)

    return entities, hook_bases, hook_overrides


def _scan_hook_callsites(java_dir: Path, hook_names: set[str]) -> dict[str, list[dict]]:
    callsites: dict[str, list[dict]] = {name: [] for name in sorted(hook_names)}

    for fpath, ctx, cls in iter_java_classes(java_dir):
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


def build_analysis_cache(
    java_dir: Path = JAVA_SRC,
    rust_dir: Path = RUST_SRC,
    out_dir: Path = ANALYSIS_CACHE_DIR,
) -> dict[str, Path]:
    ensure_dir(out_dir)

    entities, hook_bases, hook_overrides = _scan_java_entities(java_dir)
    hook_names = set(hook_bases) | set(hook_overrides)

    entity_payload = {
        "generated_at": now_iso(),
        "java_dir": str(java_dir),
        "entities": sorted(entities, key=lambda item: item["class_name"]),
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
        "callsites": _scan_hook_callsites(java_dir, hook_names),
    }
    rust_payload = _scan_rust_dispatch(rust_dir)
    manifest = {
        "generated_at": now_iso(),
        "java_dir": str(java_dir),
        "rust_dir": str(rust_dir),
        "files": {
            "java_entities": "java_entities.json",
            "java_hooks": "java_hooks.json",
            "java_callsites": "java_callsites.json",
            "rust_dispatch": "rust_dispatch.json",
        },
    }

    return {
        "java_entities": _write_json(out_dir / "java_entities.json", entity_payload),
        "java_hooks": _write_json(out_dir / "java_hooks.json", hook_payload),
        "java_callsites": _write_json(out_dir / "java_callsites.json", callsite_payload),
        "rust_dispatch": _write_json(out_dir / "rust_dispatch.json", rust_payload),
        "manifest": _write_json(out_dir / "manifest.json", manifest),
    }


def load_cache(cache_dir: Path = ANALYSIS_CACHE_DIR) -> dict:
    files = {
        "java_entities": cache_dir / "java_entities.json",
        "java_hooks": cache_dir / "java_hooks.json",
        "java_callsites": cache_dir / "java_callsites.json",
        "rust_dispatch": cache_dir / "rust_dispatch.json",
    }
    if not all(path.exists() for path in files.values()):
        build_analysis_cache(out_dir=cache_dir)
    return {
        key: json.loads(path.read_text(encoding="utf-8"))
        for key, path in files.items()
    }
