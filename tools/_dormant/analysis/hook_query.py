#!/usr/bin/env python3
"""
Hook Query Tool — focused dependency extraction for a single Java hook.

Usage:
    python tools/hook_query.py <hook_name> [--cache-dir PATH]

Example:
    python tools/hook_query.py onApplyPower

Output:
    tools/artifacts/hook_query_output/<hook_name>.md
"""

from __future__ import annotations

import argparse
from pathlib import Path

try:
    from tools.analysis.cache_builder import build_analysis_cache, load_cache
    from tools.analysis.common import ANALYSIS_CACHE_DIR, HOOK_QUERY_OUTPUT_DIR
    from tools.analysis.hook_reports import render_hook_report
except ImportError:
    from analysis.cache_builder import build_analysis_cache, load_cache  # type: ignore
    from analysis.common import ANALYSIS_CACHE_DIR, HOOK_QUERY_OUTPUT_DIR  # type: ignore
    from analysis.hook_reports import render_hook_report  # type: ignore


SCRIPT_DIR = Path(__file__).parent
OUTPUT_DIR = HOOK_QUERY_OUTPUT_DIR


def main() -> int:
    parser = argparse.ArgumentParser(description="Render a hook report from the structured analysis cache.")
    parser.add_argument("hook_name")
    parser.add_argument("--cache-dir", default=str(ANALYSIS_CACHE_DIR))
    parser.add_argument("--rebuild", action="store_true", help="Force rebuilding the structured cache first.")
    args = parser.parse_args()

    cache_dir = Path(args.cache_dir)
    if args.rebuild:
        build_analysis_cache(out_dir=cache_dir)
    else:
        load_cache(cache_dir)
    output_path = OUTPUT_DIR / f"{args.hook_name}.md"
    render_hook_report(args.hook_name, output_path, cache_dir)
    print(f"Report: {output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
