#!/usr/bin/env python3
"""Persistent cache for branch_trace evidence payloads.

This cache is keyed by a debug-only state identity plus the exact branch_trace
request. It must not be used as model input. Its purpose is to avoid rerunning
identical simulator branch queries across A/B matrices and model experiments.
"""

from __future__ import annotations

import hashlib
import json
import os
import tempfile
from pathlib import Path
from typing import Any


def canonical_json(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True)


def branch_request_semantic_key(identity: dict[str, Any], request: dict[str, Any]) -> dict[str, Any]:
    clean_request = {key: value for key, value in request.items() if key != "cmd"}
    return {
        "schema_version": "branch_evidence_cache_key_v1",
        "state_identity": identity,
        "branch_request": clean_request,
    }


class BranchEvidenceCache:
    def __init__(self, cache_dir: Path) -> None:
        self.cache_dir = cache_dir
        self.cache_dir.mkdir(parents=True, exist_ok=True)
        self.hit_count = 0
        self.miss_count = 0
        self.write_count = 0
        self.read_error_count = 0
        self.identity_mismatch_count = 0

    def key_digest(self, key: dict[str, Any]) -> str:
        return hashlib.sha256(canonical_json(key).encode("utf-8")).hexdigest()

    def path_for_digest(self, digest: str) -> Path:
        return self.cache_dir / digest[:2] / f"{digest}.json"

    def get(self, key: dict[str, Any]) -> dict[str, Any] | None:
        digest = self.key_digest(key)
        path = self.path_for_digest(digest)
        if not path.exists():
            self.miss_count += 1
            return None
        try:
            record = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            self.read_error_count += 1
            self.miss_count += 1
            return None
        if record.get("key") != key:
            self.identity_mismatch_count += 1
            self.miss_count += 1
            return None
        payload = record.get("payload")
        if not isinstance(payload, dict):
            self.read_error_count += 1
            self.miss_count += 1
            return None
        self.hit_count += 1
        return payload

    def put(self, key: dict[str, Any], payload: dict[str, Any]) -> None:
        digest = self.key_digest(key)
        path = self.path_for_digest(digest)
        path.parent.mkdir(parents=True, exist_ok=True)
        record = {
            "schema_version": "branch_evidence_cache_record_v1",
            "key": key,
            "payload": payload,
        }
        fd, temp_name = tempfile.mkstemp(
            prefix=f".{digest}.", suffix=".tmp", dir=str(path.parent)
        )
        try:
            with os.fdopen(fd, "w", encoding="utf-8") as handle:
                json.dump(record, handle, separators=(",", ":"))
            os.replace(temp_name, path)
            self.write_count += 1
        finally:
            if os.path.exists(temp_name):
                os.unlink(temp_name)

    def summary(self) -> dict[str, Any]:
        requests = self.hit_count + self.miss_count
        return {
            "schema_version": "branch_evidence_cache_summary_v1",
            "cache_dir": str(self.cache_dir),
            "hit_count": self.hit_count,
            "miss_count": self.miss_count,
            "write_count": self.write_count,
            "read_error_count": self.read_error_count,
            "identity_mismatch_count": self.identity_mismatch_count,
            "hit_rate": self.hit_count / requests if requests else 0.0,
        }
