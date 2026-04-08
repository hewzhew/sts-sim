"""
sts_tool — Unified Java→Rust porting assistant for Slay the Spire.

Usage:
    python -m sts_tool query ApplyPower      # Full report: AST + call chain + Rust parity
    python -m sts_tool ast ApplyPower        # Just the structured AST extraction
    python -m sts_tool overrides onApplyPower # Find all overrides of a hook method
    python -m sts_tool find Corruption       # Find Java files matching a name
    python -m sts_tool cache                 # Build structured analysis cache
    python -m sts_tool family guardian       # Emit JSON + Markdown for a bug family

Java source:  d:\\rust\\cardcrawl
Rust source:  d:\\rust\\sts_simulator\\src
"""

import sys
import os
import argparse
from pathlib import Path

# Fix Windows console encoding
if sys.platform == "win32":
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

from .java_parser import parse_file, extract_class, find_java_files, find_action_file, JAVA_SRC, node_text
from .ast_format import format_method_body
from .call_chain import analyze_method, format_call_chain, resolve_overrides, CallChainResult
from .rust_scan import check_rust_parity, format_rust_parity, RUST_SRC
try:
    from tools.analysis.cache_builder import build_analysis_cache
    from tools.analysis.common import ANALYSIS_CACHE_DIR
    from tools.analysis.family_audit import FAMILY_CONFIG, build_family_audit
except ImportError:
    from analysis.cache_builder import build_analysis_cache  # type: ignore
    from analysis.common import ANALYSIS_CACHE_DIR  # type: ignore
    from analysis.family_audit import FAMILY_CONFIG, build_family_audit  # type: ignore


OUTPUT_DIR = Path(r"d:\rust\sts_simulator\tools\sts_tool\output")


# ── query command ──────────────────────────────────────────────────────────

def cmd_query(args):
    """Combined query: AST + call chain + Rust parity for a Java entity."""
    name = args.name
    java_dir = Path(args.java_dir)
    rust_dir = Path(args.rust_dir)

    # Find Java files
    files = find_java_files(name, java_dir)
    if not files:
        print(f"No Java file found matching '{name}'.")
        print(f"Searched in: {java_dir}")
        return

    report_parts = []
    
    for fpath in files:
        ctx = parse_file(fpath)
        cls = extract_class(ctx)
        if not cls:
            continue
        
        rel_path = fpath.relative_to(java_dir) if fpath.is_relative_to(java_dir) else fpath
        report_parts.append(f"# {cls.name}")
        report_parts.append(f"")
        report_parts.append(f"**File**: `{rel_path}`")
        report_parts.append(f"**Category**: {cls.category}")
        if cls.string_id:
            report_parts.append(f"**ID**: `\"{cls.string_id}\"`")
        if cls.superclass:
            report_parts.append(f"**Extends**: `{cls.superclass}`")
        report_parts.append("")

        # ── Structured AST for key methods ──
        target_methods = _pick_methods(cls)
        
        for method_name in target_methods:
            if method_name not in cls.methods:
                continue
            mi = cls.methods[method_name]
            report_parts.append(f"## Method: `{method_name}{mi.params}`")
            report_parts.append(f"Lines {mi.start_line}–{mi.end_line}")
            report_parts.append("")

            # Layer 1: Structured markdown
            report_parts.append("### Structured Logic")
            report_parts.append("")
            md_lines = format_method_body(mi.body_node, ctx.source)
            report_parts.extend(md_lines)
            report_parts.append("")

            # Layer 2: Call chain
            if mi.body_node:
                chain = analyze_method(mi.body_node, ctx.source)
                chain.class_name = cls.name
                chain.method_name = method_name
                chain.file_path = str(rel_path)

                if chain.guards or chain.creates or chain.virtual_dispatches or chain.entity_checks:
                    report_parts.append("### Call Chain")
                    report_parts.append("")
                    report_parts.append("```")
                    report_parts.append(format_call_chain(chain, java_dir))
                    report_parts.append("```")
                    report_parts.append("")

        # ── Layer 3: Rust parity ──
        parity = check_rust_parity(cls.name, cls.category, rust_dir)
        report_parts.append("## Rust Parity")
        report_parts.append("")
        report_parts.append(format_rust_parity(parity))
        report_parts.append("")

    combined = "\n".join(report_parts)

    # Print to stdout
    print(combined)

    # Save to file
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    out_file = OUTPUT_DIR / f"{name.lower()}_report.md"
    out_file.write_text(combined, encoding="utf-8")
    print(f"\n{'='*60}")
    print(f"Saved: {out_file}")


def _pick_methods(cls) -> list[str]:
    """Choose which methods to extract based on entity category."""
    if cls.category == "action":
        return ["update"]
    elif cls.category == "card":
        return ["use", "upgrade"]
    elif cls.category == "power":
        # All non-trivial methods
        skip = {"makeCopy", "updateDescription"}
        return [m for m in cls.methods if m not in skip]
    elif cls.category == "relic":
        skip = {"makeCopy", "getUpdatedDescription"}
        return [m for m in cls.methods if m not in skip]
    elif cls.category == "monster":
        return ["takeTurn", "getMove", "usePreBattleAction"]
    else:
        return list(cls.methods.keys())


# ── ast command ────────────────────────────────────────────────────────────

def cmd_ast(args):
    """Extract structured AST for a specific method."""
    name = args.name
    method = args.method or "update"
    java_dir = Path(args.java_dir)

    files = find_java_files(name, java_dir)
    if not files:
        print(f"No Java file found matching '{name}'.")
        return

    for fpath in files:
        ctx = parse_file(fpath)
        cls = extract_class(ctx)
        if not cls:
            continue

        rel_path = fpath.relative_to(java_dir) if fpath.is_relative_to(java_dir) else fpath
        print(f"# {cls.name}  (`{rel_path}`)")

        if method in cls.methods:
            mi = cls.methods[method]
            print(f"## `{method}()`")
            for line in format_method_body(mi.body_node, ctx.source):
                print(line)
            print()
        else:
            available = ", ".join(cls.methods.keys())
            print(f"Method `{method}` not found. Available: {available}")


# ── overrides command ──────────────────────────────────────────────────────

def cmd_overrides(args):
    """Find all classes that override a given method."""
    hook_name = args.hook
    java_dir = Path(args.java_dir)

    print(f"Searching for overrides of `{hook_name}`...")
    overrides = resolve_overrides(hook_name, java_dir)

    print(f"\nFound {len(overrides)} overrides:\n")
    for ov in sorted(overrides, key=lambda x: x["class_name"]):
        print(f"  {ov['class_name']} ({ov['category']}) — {ov['file']} L{ov['start_line']}")


# ── find command ───────────────────────────────────────────────────────────

def cmd_find(args):
    """Find Java files matching a name."""
    name = args.name
    java_dir = Path(args.java_dir)

    files = find_java_files(name, java_dir)
    if not files:
        print(f"No files found matching '{name}'.")
        return

    print(f"Found {len(files)} file(s):")
    for f in files:
        rel = f.relative_to(java_dir) if f.is_relative_to(java_dir) else f
        ctx = parse_file(f)
        cls = extract_class(ctx)
        category = cls.category if cls else "?"
        sid = f' ID="{cls.string_id}"' if cls and cls.string_id else ""
        methods = ", ".join(cls.methods.keys()) if cls else ""
        print(f"  {rel} [{category}]{sid}")
        if methods:
            print(f"    Methods: {methods}")


def cmd_cache(args):
    paths = build_analysis_cache(Path(args.java_dir), Path(args.rust_dir), ANALYSIS_CACHE_DIR)
    for label, path in paths.items():
        print(f"{label}: {path}")


def cmd_family(args):
    json_path, md_path = build_family_audit(args.family, ANALYSIS_CACHE_DIR, Path(args.java_dir), Path(args.rust_dir))
    print(f"JSON: {json_path}")
    print(f"Markdown: {md_path}")


# ── Main ───────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        prog="sts_tool",
        description="Unified Java→Rust porting assistant for Slay the Spire",
    )
    parser.add_argument("--java-dir", default=str(JAVA_SRC), help="Java source directory")
    parser.add_argument("--rust-dir", default=str(RUST_SRC), help="Rust source directory")

    sub = parser.add_subparsers(dest="command", required=True)

    # query
    p_query = sub.add_parser("query", help="Full report: AST + call chain + Rust parity")
    p_query.add_argument("name", help="Entity name (e.g., ApplyPower, Combust, SneckoEye)")
    p_query.set_defaults(func=cmd_query)

    # ast
    p_ast = sub.add_parser("ast", help="Structured AST extraction")
    p_ast.add_argument("name", help="Entity name")
    p_ast.add_argument("--method", "-m", default=None, help="Method name (default: auto-pick)")
    p_ast.set_defaults(func=cmd_ast)

    # overrides
    p_ov = sub.add_parser("overrides", help="Find all overrides of a hook method")
    p_ov.add_argument("hook", help="Hook method name (e.g., onApplyPower)")
    p_ov.set_defaults(func=cmd_overrides)

    # find
    p_find = sub.add_parser("find", help="Find Java files matching a name")
    p_find.add_argument("name", help="Search term")
    p_find.set_defaults(func=cmd_find)

    # cache
    p_cache = sub.add_parser("cache", help="Build the structured analysis cache")
    p_cache.set_defaults(func=cmd_cache)

    # family
    p_family = sub.add_parser("family", help="Build a bug-family audit report")
    p_family.add_argument("family", choices=sorted(FAMILY_CONFIG))
    p_family.set_defaults(func=cmd_family)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
