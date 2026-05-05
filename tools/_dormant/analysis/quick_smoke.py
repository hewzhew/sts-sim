from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

try:
    from tools.analysis.cache_builder import load_cache
    from tools.analysis.common import ANALYSIS_CACHE_DIR, HOOK_QUERY_OUTPUT_DIR
    from tools.analysis.family_audit import build_family_audit
    from tools.analysis.hook_reports import render_hook_report
except ImportError:
    from analysis.cache_builder import load_cache  # type: ignore
    from analysis.common import ANALYSIS_CACHE_DIR, HOOK_QUERY_OUTPUT_DIR  # type: ignore
    from analysis.family_audit import build_family_audit  # type: ignore
    from analysis.hook_reports import render_hook_report  # type: ignore


BASE_DIR = Path(__file__).resolve().parents[1]


def main() -> int:
    load_cache(ANALYSIS_CACHE_DIR)
    subprocess.run(
        [sys.executable, "-m", "sts_tool", "query", "Corruption", "--json"],
        cwd=BASE_DIR,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    subprocess.run(
        [sys.executable, "hook_query.py", "onExhaust"],
        cwd=BASE_DIR,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    build_family_audit("power_lifecycle", ANALYSIS_CACHE_DIR)
    render_hook_report("onExhaust", HOOK_QUERY_OUTPUT_DIR / "onExhaust.md", ANALYSIS_CACHE_DIR)

    entities = json.loads((ANALYSIS_CACHE_DIR / "java_entities.json").read_text(encoding="utf-8"))
    hooks = json.loads((ANALYSIS_CACHE_DIR / "java_hooks.json").read_text(encoding="utf-8"))
    methods = json.loads((ANALYSIS_CACHE_DIR / "java_methods.json").read_text(encoding="utf-8"))
    aliases = json.loads((ANALYSIS_CACHE_DIR / "schema_aliases.json").read_text(encoding="utf-8"))
    classes = {entity["class_name"] for entity in entities["entities"]}
    assert "CorruptionPower" in classes
    assert "TheGuardian" in classes
    assert "onExhaust" in hooks["hooks"]
    assert any(method["class_name"] == "ApplyPowerAction" and method["name"] == "update" for method in methods["methods"])
    assert any(alias["raw"] == "No Draw" and alias["normalized"] == "nodraw" for alias in aliases["aliases"])
    assert any(alias["raw"] == "ClockworkSouvenir" and alias["normalized"] == "clockworksouvenir" for alias in aliases["aliases"])
    assert any(alias["raw"] == "Strike_R" and alias["normalized"] == "striker" for alias in aliases["aliases"])
    assert (ANALYSIS_CACHE_DIR / "family_audit" / "power_lifecycle.json").exists()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
