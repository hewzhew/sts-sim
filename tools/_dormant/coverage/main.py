"""CLI entry point for coverage dashboard."""
import sys
import os
from pathlib import Path

if sys.platform == "win32":
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    sys.stderr.reconfigure(encoding="utf-8", errors="replace")

# Ensure the tools directory is on the path
SCRIPT_DIR = Path(__file__).parent
TOOLS_DIR = SCRIPT_DIR.parent
if str(TOOLS_DIR) not in sys.path:
    sys.path.insert(0, str(TOOLS_DIR))

from coverage.matcher import CoverageAnalyzer
from coverage.renderer import render_html

try:
    from tools.analysis.common import COVERAGE_REPORT_PATH
except ImportError:
    from analysis.common import COVERAGE_REPORT_PATH  # type: ignore


def main():
    # Auto-detect project root (2 levels up from tools/coverage/)
    project_root = SCRIPT_DIR.parent.parent
    if not (project_root / "src").exists():
        print(f"ERROR: Cannot find src/ under {project_root}")
        sys.exit(1)

    print(f"Project root: {project_root}")
    print(f"Analysis cache: {project_root / 'tools' / 'analysis_cache'}")
    print(f"Legacy extractor output: {project_root / 'tools' / 'source_extractor' / 'output'}")
    print(f"Rust src: {project_root / 'src' / 'content'}")
    print()

    analyzer = CoverageAnalyzer(project_root)

    print("Analyzing Powers...")
    results = analyzer.analyze_all()

    for cat, summary in results.items():
        print(f"\n{'='*60}")
        print(f"  {cat.value.upper()}: {summary.fully_covered}/{summary.total_java} fully covered")
        print(f"  Files: {summary.has_rust_file} | Partial: {summary.partially_covered} | Missing: {summary.not_covered}")

        # Show incomplete entries
        incomplete = [e for e in summary.entries if e.coverage_pct < 100 and e.total_hooks > 0]
        if incomplete:
            print(f"\n  Incomplete ({len(incomplete)}):")
            for e in incomplete[:10]:
                missing = [h.name for h in e.hook_details if h.status.value == "missing"]
                print(f"    {e.status_icon} {e.java.class_name}: {e.implemented_hooks}/{e.total_hooks} — missing: {', '.join(missing)}")
            if len(incomplete) > 10:
                print(f"    ... and {len(incomplete) - 10} more")

    # Generate HTML
    output_path = COVERAGE_REPORT_PATH
    java_root = str(project_root.parent / "cardcrawl").replace("\\", "/")
    render_html(results, output_path, java_root)
    print(f"\n✅ HTML report: {output_path}")
    print(f"   Open in browser: file:///{str(output_path).replace(chr(92), '/')}")


if __name__ == "__main__":
    main()
