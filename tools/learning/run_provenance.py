#!/usr/bin/env python3
from __future__ import annotations

import json
import subprocess
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def git_short_head(repo_root: Path = REPO_ROOT) -> str | None:
    try:
        completed = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=repo_root,
            check=False,
            capture_output=True,
            text=True,
            timeout=5,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if completed.returncode != 0:
        return None
    text = completed.stdout.strip()
    return text or None


def git_dirty_summary(repo_root: Path = REPO_ROOT) -> dict[str, Any]:
    try:
        completed = subprocess.run(
            ["git", "status", "--short"],
            cwd=repo_root,
            check=False,
            capture_output=True,
            text=True,
            timeout=5,
        )
    except (OSError, subprocess.SubprocessError):
        return {"dirty": None, "status_short": None}
    if completed.returncode != 0:
        return {"dirty": None, "status_short": None}
    lines = [line for line in completed.stdout.splitlines() if line.strip()]
    return {"dirty": bool(lines), "status_short": lines[:50], "status_line_count": len(lines)}


def current_repo_provenance(repo_root: Path = REPO_ROOT) -> dict[str, Any]:
    return {
        "repo_head_short_sha": git_short_head(repo_root),
        **git_dirty_summary(repo_root),
    }


def run_dir_from_source_path(source_path: Any) -> Path | None:
    if not source_path:
        return None
    path = Path(str(source_path))
    if not path.is_absolute():
        path = REPO_ROOT / path
    if path.name.lower() == "raw.jsonl":
        return path.parent
    if path.parent.name:
        candidate = path.parent
        if (candidate / "manifest.json").exists():
            return candidate
    return None


def load_run_manifest_from_source(source_path: Any) -> dict[str, Any] | None:
    run_dir = run_dir_from_source_path(source_path)
    if run_dir is None:
        return None
    manifest_path = run_dir / "manifest.json"
    if not manifest_path.exists():
        return None
    manifest = read_json(manifest_path)
    manifest["_manifest_path"] = str(manifest_path)
    return manifest


def summarize_manifest(manifest: dict[str, Any] | None) -> dict[str, Any]:
    if manifest is None:
        return {
            "manifest_present": False,
            "manifest_path": None,
            "run_id": None,
            "classification_label": None,
            "build_tag": None,
            "profile_name": None,
            "git_short_sha": None,
            "repo_head_short_sha_at_run": None,
            "binary_matches_head": None,
            "binary_is_fresh": None,
        }
    provenance = manifest.get("provenance") or {}
    profile = manifest.get("profile") or {}
    return {
        "manifest_present": True,
        "manifest_path": manifest.get("_manifest_path"),
        "run_id": manifest.get("run_id"),
        "classification_label": manifest.get("classification_label"),
        "session_exit_reason": manifest.get("session_exit_reason"),
        "build_tag": manifest.get("build_tag"),
        "profile_name": provenance.get("profile_name") or profile.get("profile_name"),
        "profile_purpose": profile.get("purpose"),
        "capture_policy": profile.get("capture_policy"),
        "git_short_sha": provenance.get("git_short_sha"),
        "repo_head_short_sha_at_run": provenance.get("repo_head_short_sha"),
        "binary_matches_head": provenance.get("binary_matches_head"),
        "binary_is_fresh": provenance.get("binary_is_fresh"),
        "exe_path": provenance.get("exe_path"),
        "exe_mtime_utc": provenance.get("exe_mtime_utc"),
        "build_time_utc": provenance.get("build_time_utc"),
    }


def classify_run_freshness(
    run_provenance: dict[str, Any],
    current: dict[str, Any],
) -> dict[str, Any]:
    current_head = current.get("repo_head_short_sha")
    run_git = run_provenance.get("git_short_sha")
    run_head = run_provenance.get("repo_head_short_sha_at_run")
    manifest_present = bool(run_provenance.get("manifest_present"))
    binary_matches_head = run_provenance.get("binary_matches_head")
    binary_is_fresh = run_provenance.get("binary_is_fresh")
    stale_reasons: list[str] = []
    if not manifest_present:
        stale_reasons.append("missing_manifest")
    if current_head is None:
        stale_reasons.append("missing_current_git_head")
    if run_git is None:
        stale_reasons.append("missing_run_git_short_sha")
    if current_head is not None and run_git is not None and current_head != run_git:
        stale_reasons.append("run_git_differs_from_current_head")
    if run_head is not None and run_git is not None and run_head != run_git:
        stale_reasons.append("run_binary_git_differs_from_run_repo_head")
    if binary_matches_head is False:
        stale_reasons.append("binary_did_not_match_head_at_run")
    if binary_is_fresh is False:
        stale_reasons.append("binary_was_stale_at_run")
    if current.get("dirty") is True:
        stale_reasons.append("current_worktree_dirty")
    fresh_for_current = not stale_reasons
    return {
        "fresh_for_current_head": fresh_for_current,
        "current_policy_conclusion_allowed": fresh_for_current,
        "stale_reasons": stale_reasons,
        "evidence_scope": "current_policy" if fresh_for_current else "historical_replay_only",
    }


def provenance_for_source(source_path: Any, current: dict[str, Any] | None = None) -> dict[str, Any]:
    current = current or current_repo_provenance()
    manifest = load_run_manifest_from_source(source_path)
    run = summarize_manifest(manifest)
    freshness = classify_run_freshness(run, current)
    return {
        "current": current,
        "run": run,
        "freshness": freshness,
    }
