"""
sts_tool — cache-first Java→Rust analysis entrypoint for Slay the Spire.

Primary workflow:
    python -m sts_tool cache
    python -m sts_tool query ApplyPower
    python -m sts_tool query ApplyPower --json
    python -m sts_tool overrides onApplyPower
    python -m sts_tool find Corruption
    python -m sts_tool family power_lifecycle
    python -m sts_tool inspect ApplyPower --method update
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

if sys.platform == "win32":
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

from .java_parser import JAVA_SRC
from .rust_scan import RUST_SRC

try:
    from tools.analysis.cache_builder import build_analysis_cache
    from tools.analysis.common import ANALYSIS_CACHE_DIR
    from tools.analysis.family_audit import FAMILY_CONFIG, build_family_audit
    from tools.analysis.query_engine import (
        build_find_result,
        build_inspect_result,
        build_override_result,
        build_query_result,
        print_result,
        render_find_markdown,
        render_inspect_markdown,
        render_override_markdown,
        render_query_markdown,
    )
except ImportError:
    from analysis.cache_builder import build_analysis_cache  # type: ignore
    from analysis.common import ANALYSIS_CACHE_DIR  # type: ignore
    from analysis.family_audit import FAMILY_CONFIG, build_family_audit  # type: ignore
    from analysis.query_engine import (  # type: ignore
        build_find_result,
        build_inspect_result,
        build_override_result,
        build_query_result,
        print_result,
        render_find_markdown,
        render_inspect_markdown,
        render_override_markdown,
        render_query_markdown,
    )


def cmd_query(args) -> None:
    result = build_query_result(args.name, Path(args.cache_dir), rebuild=args.rebuild)
    print_result(result, args.json, render_query_markdown)


def cmd_find(args) -> None:
    result = build_find_result(args.name, Path(args.cache_dir), rebuild=args.rebuild)
    print_result(result, args.json, render_find_markdown)


def cmd_overrides(args) -> None:
    result = build_override_result(args.hook, Path(args.cache_dir), rebuild=args.rebuild)
    print_result(result, args.json, render_override_markdown)


def cmd_inspect(args) -> None:
    result = build_inspect_result(args.name, args.method, Path(args.cache_dir), rebuild=args.rebuild)
    print_result(result, args.json, render_inspect_markdown)


def cmd_cache(args) -> None:
    paths = build_analysis_cache(Path(args.java_dir), Path(args.rust_dir), Path(args.cache_dir))
    for label, path in paths.items():
        print(f"{label}: {path}")


def cmd_family(args) -> None:
    json_path, md_path = build_family_audit(args.family, Path(args.cache_dir), Path(args.java_dir), Path(args.rust_dir))
    print(f"JSON: {json_path}")
    print(f"Markdown: {md_path}")


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="sts_tool",
        description="Cache-first Java→Rust analysis assistant for Slay the Spire",
    )
    parser.add_argument("--java-dir", default=str(JAVA_SRC), help="Java source directory")
    parser.add_argument("--rust-dir", default=str(RUST_SRC), help="Rust source directory")
    parser.add_argument("--cache-dir", default=str(ANALYSIS_CACHE_DIR), help="Analysis cache directory")

    sub = parser.add_subparsers(dest="command", required=True)

    p_cache = sub.add_parser("cache", help="Build or rebuild the structured analysis cache")
    p_cache.set_defaults(func=cmd_cache)

    p_query = sub.add_parser("query", help="Query a Java entity from the structured cache")
    p_query.add_argument("name")
    p_query.add_argument("--json", action="store_true", help="Emit structured JSON instead of Markdown")
    p_query.add_argument("--md", action="store_true", help="Accepted for explicitness; Markdown is default")
    p_query.add_argument("--rebuild", action="store_true", help="Force rebuilding the cache first")
    p_query.set_defaults(func=cmd_query)

    p_find = sub.add_parser("find", help="Find Java entities by class name, string id, file, or schema alias")
    p_find.add_argument("name")
    p_find.add_argument("--json", action="store_true", help="Emit structured JSON instead of Markdown")
    p_find.add_argument("--rebuild", action="store_true", help="Force rebuilding the cache first")
    p_find.set_defaults(func=cmd_find)

    p_overrides = sub.add_parser("overrides", help="Query hook definitions, overrides, and callsites from cache")
    p_overrides.add_argument("hook")
    p_overrides.add_argument("--json", action="store_true", help="Emit structured JSON instead of Markdown")
    p_overrides.add_argument("--rebuild", action="store_true", help="Force rebuilding the cache first")
    p_overrides.set_defaults(func=cmd_overrides)

    p_family = sub.add_parser("family", help="Build a cache-backed bug-family report")
    p_family.add_argument("family", choices=sorted(FAMILY_CONFIG))
    p_family.set_defaults(func=cmd_family)

    p_inspect = sub.add_parser("inspect", help="Low-level cache-backed method inspection")
    p_inspect.add_argument("name")
    p_inspect.add_argument("--method", "-m", default=None)
    p_inspect.add_argument("--json", action="store_true", help="Emit structured JSON instead of Markdown")
    p_inspect.add_argument("--rebuild", action="store_true", help="Force rebuilding the cache first")
    p_inspect.set_defaults(func=cmd_inspect)

    p_ast = sub.add_parser("ast", help="Deprecated alias for inspect")
    p_ast.add_argument("name")
    p_ast.add_argument("--method", "-m", default=None)
    p_ast.add_argument("--json", action="store_true")
    p_ast.add_argument("--rebuild", action="store_true")
    p_ast.set_defaults(func=cmd_inspect)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
