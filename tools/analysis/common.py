from __future__ import annotations

from datetime import UTC, datetime
from pathlib import Path


TOOLS_DIR = Path(__file__).resolve().parents[1]
PROJECT_ROOT = TOOLS_DIR.parent
ANALYSIS_CACHE_DIR = TOOLS_DIR / "analysis_cache"
FAMILY_AUDIT_DIR = ANALYSIS_CACHE_DIR / "family_audit"
ARTIFACTS_DIR = TOOLS_DIR / "artifacts"
HOOK_QUERY_OUTPUT_DIR = ARTIFACTS_DIR / "hook_query_output"
COVERAGE_REPORT_PATH = ARTIFACTS_DIR / "coverage_report.html"
INTERACTION_COVERAGE_PATH = ARTIFACTS_DIR / "interaction_coverage.json"
INTERACTION_COVERAGE_REPORT_PATH = ARTIFACTS_DIR / "interaction_coverage_report.json"
OBSERVED_IDS_PATH = ARTIFACTS_DIR / "observed_ids.json"


def now_iso() -> str:
    return datetime.now(UTC).replace(microsecond=0).isoformat()


def ensure_dir(path: Path) -> Path:
    path.mkdir(parents=True, exist_ok=True)
    return path
