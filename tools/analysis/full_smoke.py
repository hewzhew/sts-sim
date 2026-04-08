from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

try:
    from tools.analysis.cache_builder import build_analysis_cache
    from tools.analysis.common import ANALYSIS_CACHE_DIR, HOOK_QUERY_OUTPUT_DIR
    from tools.analysis.family_audit import build_family_audit
    from tools.analysis.hook_reports import render_hook_report
except ImportError:
    from analysis.cache_builder import build_analysis_cache  # type: ignore
    from analysis.common import ANALYSIS_CACHE_DIR, HOOK_QUERY_OUTPUT_DIR  # type: ignore
    from analysis.family_audit import build_family_audit  # type: ignore
    from analysis.hook_reports import render_hook_report  # type: ignore


BASE_DIR = Path(__file__).resolve().parents[1]
REQUIRED_DOC_PATHS = [
    BASE_DIR / "sts_tool" / "__main__.py",
    BASE_DIR / "hook_query.py",
    BASE_DIR / "source_extractor" / "sts_extractor.py",
    BASE_DIR / "source_extractor" / "AGENT_GUIDE.md",
]


def main() -> int:
    for path in REQUIRED_DOC_PATHS:
        if not path.exists():
            raise SystemExit(f"Missing documented tool path: {path}")

    subprocess.run(
        [sys.executable, "-m", "sts_tool", "query", "Corruption"],
        cwd=BASE_DIR,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    build_analysis_cache(out_dir=ANALYSIS_CACHE_DIR)
    subprocess.run(
        [sys.executable, "hook_query.py", "onExhaust"],
        cwd=BASE_DIR,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    render_hook_report("onExhaust", HOOK_QUERY_OUTPUT_DIR / "onExhaust.md", ANALYSIS_CACHE_DIR)
    for family in ("exhaust", "guardian", "vulnerable"):
        build_family_audit(family, ANALYSIS_CACHE_DIR)
    subprocess.run(
        [sys.executable, "-m", "tools.coverage.main"],
        cwd=BASE_DIR.parent,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    subprocess.run(
        [
            sys.executable,
            str(BASE_DIR / "source_extractor" / "sts_extractor.py"),
            str(BASE_DIR.parent.parent / "cardcrawl"),
            str(BASE_DIR / "source_extractor" / "output"),
        ],
        cwd=BASE_DIR.parent,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )

    entities = json.loads((ANALYSIS_CACHE_DIR / "java_entities.json").read_text(encoding="utf-8"))
    hooks = json.loads((ANALYSIS_CACHE_DIR / "java_hooks.json").read_text(encoding="utf-8"))
    dispatch = json.loads((ANALYSIS_CACHE_DIR / "rust_dispatch.json").read_text(encoding="utf-8"))

    classes = {entity["class_name"] for entity in entities["entities"]}
    assert "VulnerablePower" in classes
    assert "PaperFrog" in classes
    assert "TheGuardian" in classes
    assert "onExhaust" in hooks["hooks"]
    assert "resolve_power_on_exhaust" in dispatch["power_dispatch"]
    assert (BASE_DIR / "source_extractor" / "output" / "hooks.json").exists()
    assert (BASE_DIR / "source_extractor" / "output" / "scattered_logic.json").exists()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
