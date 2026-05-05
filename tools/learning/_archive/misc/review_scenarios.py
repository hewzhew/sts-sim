#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import tempfile
from datetime import datetime, timezone
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any
from urllib.parse import urlparse

from combat_rl_common import REPO_ROOT


DEFAULT_INPUT = REPO_ROOT / "tools" / "artifacts" / "scenario_bench" / "human_review_v0_with_ppo.json"
DEFAULT_OUT = REPO_ROOT / "tools" / "artifacts" / "scenario_bench" / "human_review_v0_labeled.json"

LABEL_MODES = {
    "hard_preference",
    "soft_preference",
    "acceptable_set",
    "no_signal",
    "needs_rollout",
    "abstain",
}

EVIDENCE_MODES = {"human_prior", "rollout_backed", "regression_case", "uncertain"}

RATIONALE_TAGS = {
    "early_act",
    "late_act",
    "deck_unformed",
    "needs_frontload",
    "lacks_draw",
    "lacks_block",
    "lacks_scaling",
    "needs_aoe",
    "hp_pressure",
    "boss_counter",
    "avoid_deck_bloat",
    "already_lost",
    "synergy_piece",
    "upgrade_value",
    "remove_basic",
    "route_risk",
    "potion_reliance",
    "low_curve",
    "high_curve",
    "small_gap",
    "uncertain",
}

ROLE_NAMES = {"preferred", "acceptable", "bad"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Local browser UI for scenario human review labels.")
    parser.add_argument("--input", type=Path, default=DEFAULT_INPUT)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument("--no-resume", action="store_true", help="Ignore existing --out and start from --input.")
    parser.add_argument("--check", action="store_true", help="Validate input/output paths and exit without serving.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    state = ReviewState(
        input_path=args.input,
        out_path=args.out,
        resume=not args.no_resume,
    )
    state.load()
    if args.check:
        state.validate()
        return 0

    handler = make_handler(state)
    server = ThreadingHTTPServer((args.host, args.port), handler)
    print(f"Scenario review UI: http://{args.host}:{args.port}/")
    print(f"Input: {state.loaded_from}")
    print(f"Output: {state.out_path}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nStopping scenario review UI.")
    finally:
        server.server_close()
    return 0


class ReviewState:
    def __init__(self, *, input_path: Path, out_path: Path, resume: bool) -> None:
        self.input_path = input_path
        self.out_path = out_path
        self.resume = resume
        self.loaded_from: Path | None = None
        self.data: dict[str, Any] = {}

    def load(self) -> None:
        source = self.out_path if self.resume and self.out_path.exists() else self.input_path
        if not source.exists():
            raise SystemExit(f"missing review JSON: {source}")
        with source.open("r", encoding="utf-8") as handle:
            self.data = json.load(handle)
        self.loaded_from = source
        self.validate()

    def validate(self) -> None:
        scenarios = self.data.get("scenarios")
        if not isinstance(scenarios, list) or not scenarios:
            raise SystemExit("review JSON must contain a non-empty scenarios[] list")
        for scenario in scenarios:
            if not isinstance(scenario, dict):
                raise SystemExit("each scenario must be an object")
            if not scenario.get("id"):
                raise SystemExit("each scenario must have id")
            candidates = scenario.get("candidates")
            if not isinstance(candidates, list) or not candidates:
                raise SystemExit(f"scenario {scenario.get('id')} has no candidates")
            candidate_ids = {str(candidate.get("id")) for candidate in candidates}
            label = normalize_label(scenario.get("human_label"), candidate_ids)
            scenario["human_label"] = label

    def replace_and_save(self, payload: dict[str, Any]) -> dict[str, Any]:
        merged = dict(self.data)
        incoming_scenarios = payload.get("scenarios")
        if not isinstance(incoming_scenarios, list):
            raise ValueError("payload.scenarios must be a list")
        current_by_id = {str(scenario.get("id")): scenario for scenario in self.data.get("scenarios") or []}
        for incoming in incoming_scenarios:
            scenario_id = str(incoming.get("id") or "")
            current = current_by_id.get(scenario_id)
            if current is None:
                raise ValueError(f"unknown scenario id: {scenario_id}")
            candidate_ids = {str(candidate.get("id")) for candidate in current.get("candidates") or []}
            current["human_label"] = normalize_label(incoming.get("human_label"), candidate_ids)
        merged["scenarios"] = list(current_by_id.values())
        metadata = dict(merged.get("review_metadata") or {})
        metadata.update(
            {
                "saved_at_utc": datetime.now(timezone.utc).isoformat(),
                "input_path": str(self.input_path),
                "output_path": str(self.out_path),
                "loaded_from": str(self.loaded_from) if self.loaded_from else None,
                "scenario_count": len(merged["scenarios"]),
            }
        )
        merged["review_metadata"] = metadata
        self.data = merged
        atomic_write_json(self.out_path, self.data)
        return {
            "ok": True,
            "saved_at_utc": metadata["saved_at_utc"],
            "out_path": str(self.out_path),
            "summary": review_summary(self.data),
        }

    def response_payload(self) -> dict[str, Any]:
        return {
            "input_path": str(self.input_path),
            "out_path": str(self.out_path),
            "loaded_from": str(self.loaded_from) if self.loaded_from else None,
            "summary": review_summary(self.data),
            "review": self.data,
        }


def normalize_label(raw: Any, candidate_ids: set[str]) -> dict[str, Any]:
    label = dict(raw or {})
    mode = str(label.get("mode") or "abstain")
    if mode not in LABEL_MODES:
        mode = "abstain"
    normalized = {
        "mode": mode,
        "preferred": normalize_id_list(label.get("preferred"), candidate_ids),
        "acceptable": normalize_id_list(label.get("acceptable"), candidate_ids),
        "bad": normalize_id_list(label.get("bad"), candidate_ids),
        "confidence": clamp_float(label.get("confidence"), 0.0, 1.0, 0.5),
        "rationale_tags": normalize_tag_list(label.get("rationale_tags")),
        "evidence": normalize_evidence(label.get("evidence")),
        "reason": str(label.get("reason") or ""),
    }
    return normalized


def normalize_id_list(raw: Any, candidate_ids: set[str]) -> list[str]:
    if not isinstance(raw, list):
        return []
    out: list[str] = []
    for item in raw:
        value = str(item)
        if value in candidate_ids and value not in out:
            out.append(value)
    return out


def normalize_tag_list(raw: Any) -> list[str]:
    if not isinstance(raw, list):
        return []
    out: list[str] = []
    for item in raw:
        value = str(item)
        if value in RATIONALE_TAGS and value not in out:
            out.append(value)
    return out


def normalize_evidence(raw: Any) -> str:
    value = str(raw or "human_prior")
    if value not in EVIDENCE_MODES:
        return "human_prior"
    return value


def clamp_float(raw: Any, low: float, high: float, default: float) -> float:
    try:
        value = float(raw)
    except (TypeError, ValueError):
        return default
    return max(low, min(high, value))


def review_summary(data: dict[str, Any]) -> dict[str, Any]:
    scenarios = data.get("scenarios") or []
    mode_counts: dict[str, int] = {}
    category_counts: dict[str, int] = {}
    tag_counts: dict[str, int] = {}
    evidence_counts: dict[str, int] = {}
    reviewed = 0
    needs_attention = 0
    for scenario in scenarios:
        label = scenario.get("human_label") or {}
        mode = str(label.get("mode") or "abstain")
        evidence = str(label.get("evidence") or "human_prior")
        category = str(scenario.get("category") or "unknown")
        mode_counts[mode] = mode_counts.get(mode, 0) + 1
        evidence_counts[evidence] = evidence_counts.get(evidence, 0) + 1
        category_counts[category] = category_counts.get(category, 0) + 1
        for tag in label.get("rationale_tags") or []:
            tag_counts[str(tag)] = tag_counts.get(str(tag), 0) + 1
        reason = str(label.get("reason") or "").strip()
        has_roles = any(label.get(role) for role in ROLE_NAMES)
        if mode in {"no_signal", "needs_rollout", "abstain"} or reason or has_roles:
            reviewed += 1
        if mode in {"needs_rollout", "abstain"}:
            needs_attention += 1
    return {
        "scenario_count": len(scenarios),
        "reviewed_count": reviewed,
        "needs_attention_count": needs_attention,
        "mode_counts": mode_counts,
        "category_counts": category_counts,
        "rationale_tag_counts": tag_counts,
        "evidence_counts": evidence_counts,
    }


def atomic_write_json(path: Path, data: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temp_name = tempfile.mkstemp(prefix=f".{path.name}.", suffix=".tmp", dir=str(path.parent))
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="\n") as handle:
            json.dump(data, handle, ensure_ascii=False, indent=2)
            handle.write("\n")
        os.replace(temp_name, path)
    except Exception:
        try:
            os.unlink(temp_name)
        except OSError:
            pass
        raise


def make_handler(state: ReviewState) -> type[BaseHTTPRequestHandler]:
    class ReviewHandler(BaseHTTPRequestHandler):
        server_version = "ScenarioReview/0.1"

        def do_GET(self) -> None:  # noqa: N802
            parsed = urlparse(self.path)
            if parsed.path == "/":
                self.send_bytes(HTML.encode("utf-8"), "text/html; charset=utf-8")
                return
            if parsed.path == "/api/review":
                self.send_json(state.response_payload())
                return
            self.send_error(HTTPStatus.NOT_FOUND, "not found")

        def do_POST(self) -> None:  # noqa: N802
            parsed = urlparse(self.path)
            if parsed.path != "/api/review":
                self.send_error(HTTPStatus.NOT_FOUND, "not found")
                return
            try:
                length = int(self.headers.get("Content-Length") or "0")
                raw = self.rfile.read(length)
                payload = json.loads(raw.decode("utf-8"))
                result = state.replace_and_save(payload)
                self.send_json(result)
            except Exception as err:  # pragma: no cover - visible to browser.
                self.send_json({"ok": False, "error": str(err)}, status=HTTPStatus.BAD_REQUEST)

        def log_message(self, fmt: str, *args: Any) -> None:
            print(f"{self.address_string()} - {fmt % args}")

        def send_json(self, data: dict[str, Any], status: HTTPStatus = HTTPStatus.OK) -> None:
            self.send_bytes(json.dumps(data, ensure_ascii=False, indent=2).encode("utf-8"), "application/json", status)

        def send_bytes(
            self,
            body: bytes,
            content_type: str,
            status: HTTPStatus = HTTPStatus.OK,
        ) -> None:
            self.send_response(status)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(body)

    return ReviewHandler


HTML = r"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Scenario Review</title>
  <style>
    :root {
      color-scheme: light;
      --bg: #f6f7f8;
      --panel: #ffffff;
      --ink: #192026;
      --muted: #65717d;
      --line: #d9dee3;
      --accent: #2364aa;
      --good: #1d7a46;
      --warn: #a76500;
      --bad: #b42318;
      --shadow: 0 1px 2px rgba(0,0,0,.06);
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      background: var(--bg);
      color: var(--ink);
      font-family: Inter, Segoe UI, system-ui, -apple-system, sans-serif;
      font-size: 14px;
      letter-spacing: 0;
    }
    button, input, select, textarea {
      font: inherit;
      letter-spacing: 0;
    }
    .app {
      display: grid;
      grid-template-columns: minmax(280px, 360px) minmax(0, 1fr);
      min-height: 100vh;
    }
    .sidebar {
      border-right: 1px solid var(--line);
      background: #eef1f4;
      display: flex;
      flex-direction: column;
      min-width: 0;
    }
    .side-head {
      padding: 14px;
      border-bottom: 1px solid var(--line);
      background: #f8fafb;
    }
    h1 {
      margin: 0 0 10px;
      font-size: 18px;
      line-height: 1.2;
    }
    .path {
      color: var(--muted);
      font-size: 12px;
      word-break: break-all;
    }
    .toolbar {
      display: grid;
      grid-template-columns: 1fr auto;
      gap: 8px;
      margin-top: 12px;
    }
    .search, .filter, .mode-select {
      width: 100%;
      min-height: 34px;
      border: 1px solid var(--line);
      border-radius: 6px;
      background: #fff;
      padding: 6px 8px;
    }
    .summary {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 6px;
      margin-top: 12px;
    }
    .metric {
      background: #fff;
      border: 1px solid var(--line);
      border-radius: 6px;
      padding: 8px;
      box-shadow: var(--shadow);
    }
    .metric b {
      display: block;
      font-size: 15px;
    }
    .metric span {
      display: block;
      color: var(--muted);
      font-size: 11px;
      margin-top: 2px;
    }
    .list {
      overflow: auto;
      padding: 8px;
    }
    .item {
      width: 100%;
      border: 1px solid transparent;
      border-radius: 6px;
      background: transparent;
      padding: 9px;
      text-align: left;
      display: grid;
      grid-template-columns: 1fr auto;
      gap: 6px;
      cursor: pointer;
    }
    .item:hover { background: #fff; }
    .item.active {
      background: #fff;
      border-color: #a9bdd3;
      box-shadow: var(--shadow);
    }
    .item-title {
      font-weight: 650;
      overflow-wrap: anywhere;
    }
    .item-meta {
      color: var(--muted);
      font-size: 12px;
      margin-top: 3px;
    }
    .badge {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-height: 22px;
      padding: 2px 7px;
      border-radius: 999px;
      border: 1px solid var(--line);
      background: #fff;
      font-size: 12px;
      white-space: nowrap;
    }
    .badge.good { color: var(--good); border-color: #9fd2b4; background: #eefaf3; }
    .badge.warn { color: var(--warn); border-color: #e4c27b; background: #fff8e8; }
    .badge.bad { color: var(--bad); border-color: #edaaa4; background: #fff1ef; }
    .main {
      min-width: 0;
      padding: 18px;
      overflow: auto;
    }
    .topbar {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      margin-bottom: 14px;
    }
    .actions {
      display: flex;
      align-items: center;
      gap: 8px;
      flex-wrap: wrap;
    }
    .btn {
      min-height: 34px;
      border: 1px solid var(--line);
      background: #fff;
      border-radius: 6px;
      padding: 6px 10px;
      cursor: pointer;
    }
    .btn:hover { border-color: #98a8b8; }
    .btn.primary {
      background: var(--accent);
      color: #fff;
      border-color: var(--accent);
    }
    .status {
      color: var(--muted);
      min-width: 130px;
      text-align: right;
      font-size: 13px;
    }
    .panel {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 8px;
      box-shadow: var(--shadow);
      margin-bottom: 14px;
      overflow: hidden;
    }
    .panel-head {
      padding: 12px 14px;
      border-bottom: 1px solid var(--line);
      background: #fbfcfd;
    }
    .panel-head h2 {
      margin: 0;
      font-size: 17px;
      line-height: 1.25;
    }
    .panel-body {
      padding: 14px;
    }
    .context {
      color: var(--muted);
      line-height: 1.5;
      margin-top: 8px;
    }
    .context-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 8px;
      margin-bottom: 12px;
    }
    .context-card {
      border: 1px solid var(--line);
      border-radius: 6px;
      background: #fbfcfd;
      padding: 9px;
      min-width: 0;
    }
    .context-title {
      color: var(--muted);
      font-size: 12px;
      font-weight: 650;
      margin-bottom: 6px;
    }
    .kv {
      display: grid;
      grid-template-columns: minmax(116px, 1fr) auto;
      gap: 4px 8px;
      font-size: 12px;
      line-height: 1.35;
    }
    .kv-key {
      color: var(--muted);
      word-break: keep-all;
    }
    .kv-value {
      font-weight: 600;
      text-align: right;
    }
    .pill-list {
      display: flex;
      flex-wrap: wrap;
      gap: 5px;
    }
    .pill {
      border: 1px solid var(--line);
      border-radius: 999px;
      background: #fff;
      padding: 3px 7px;
      font-size: 12px;
      overflow-wrap: anywhere;
    }
    .grid2 {
      display: grid;
      grid-template-columns: minmax(0, 1.2fr) minmax(280px, .8fr);
      gap: 14px;
    }
    table {
      width: 100%;
      border-collapse: collapse;
      table-layout: fixed;
    }
    th, td {
      border-bottom: 1px solid var(--line);
      padding: 8px 7px;
      text-align: left;
      vertical-align: top;
    }
    th {
      color: var(--muted);
      font-weight: 650;
      font-size: 12px;
      background: #fbfcfd;
    }
    tr:last-child td { border-bottom: 0; }
    .candidate-name {
      font-weight: 650;
      overflow-wrap: anywhere;
    }
    .candidate-sub {
      color: var(--muted);
      font-size: 12px;
      margin-top: 3px;
      overflow-wrap: anywhere;
    }
    .role-group {
      display: grid;
      grid-template-columns: repeat(4, minmax(48px, 1fr));
      gap: 4px;
    }
    .role-btn {
      border: 1px solid var(--line);
      background: #fff;
      border-radius: 6px;
      padding: 5px 4px;
      min-height: 30px;
      cursor: pointer;
      font-size: 12px;
    }
    .role-btn.selected { color: #fff; border-color: transparent; }
    .role-btn.preferred.selected { background: var(--good); }
    .role-btn.acceptable.selected { background: var(--accent); }
    .role-btn.bad.selected { background: var(--bad); }
    .role-btn.none.selected { background: #59636e; }
    .mode-row {
      display: grid;
      grid-template-columns: minmax(180px, 1fr) minmax(160px, .8fr) minmax(180px, 1fr) auto;
      gap: 10px;
      align-items: center;
    }
    .confidence-wrap {
      display: grid;
      grid-template-columns: 1fr 48px;
      gap: 8px;
      align-items: center;
    }
    .confidence-wrap input { width: 100%; }
    .tag-section {
      margin-top: 12px;
      padding-top: 10px;
      border-top: 1px solid var(--line);
    }
    .tag-title {
      color: var(--muted);
      font-size: 12px;
      font-weight: 650;
      margin-bottom: 8px;
    }
    .tag-grid {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
    }
    .tag-btn {
      min-height: 28px;
      border: 1px solid var(--line);
      background: #fff;
      border-radius: 999px;
      padding: 4px 9px;
      cursor: pointer;
      color: #2f3b46;
      font-size: 12px;
    }
    .tag-btn.selected {
      background: #e7f0fb;
      border-color: #8fb4db;
      color: #174f87;
      font-weight: 650;
    }
    textarea {
      width: 100%;
      min-height: 92px;
      resize: vertical;
      border: 1px solid var(--line);
      border-radius: 6px;
      padding: 8px;
      margin-top: 10px;
    }
    .ranking {
      display: grid;
      gap: 8px;
    }
    .rank-card {
      border: 1px solid var(--line);
      border-radius: 6px;
      overflow: hidden;
    }
    .rank-title {
      padding: 8px 10px;
      background: #fbfcfd;
      border-bottom: 1px solid var(--line);
      font-weight: 650;
    }
    .rank-list {
      padding: 8px 10px;
      display: grid;
      gap: 5px;
    }
    .rank-row {
      display: grid;
      grid-template-columns: 32px 1fr auto;
      gap: 6px;
      align-items: center;
      color: var(--muted);
    }
    .rank-row.top { color: var(--ink); font-weight: 650; }
    .empty {
      color: var(--muted);
      padding: 40px;
      text-align: center;
    }
    @media (max-width: 980px) {
      .app { grid-template-columns: 1fr; }
      .sidebar { min-height: 42vh; border-right: 0; border-bottom: 1px solid var(--line); }
      .grid2 { grid-template-columns: 1fr; }
      .mode-row { grid-template-columns: 1fr; }
    }
  </style>
</head>
<body>
  <div class="app">
    <aside class="sidebar">
      <div class="side-head">
        <h1>Scenario Review</h1>
        <div class="path" id="pathInfo">Loading...</div>
        <div class="toolbar">
          <input class="search" id="search" data-testid="search" placeholder="Search scenario or candidate">
          <select class="filter" id="filter" data-testid="filter">
            <option value="all">All</option>
            <option value="needs_attention">Needs attention</option>
            <option value="bad_model">Model bad choice</option>
            <option value="no_signal">No signal</option>
            <option value="needs_rollout">Needs rollout</option>
          </select>
        </div>
        <div class="summary">
          <div class="metric"><b id="metricTotal">0</b><span>scenarios</span></div>
          <div class="metric"><b id="metricReviewed">0</b><span>reviewed</span></div>
          <div class="metric"><b id="metricAttention">0</b><span>attention</span></div>
        </div>
      </div>
      <div class="list" id="scenarioList" data-testid="scenario-list"></div>
    </aside>
    <main class="main">
      <div class="topbar">
        <div>
          <span class="badge" id="positionBadge">0 / 0</span>
          <span class="badge" id="modeBadge">mode</span>
          <span class="badge" id="categoryBadge">category</span>
        </div>
        <div class="actions">
          <button class="btn" id="prevBtn" data-testid="prev">Prev</button>
          <button class="btn" id="nextBtn" data-testid="next">Next</button>
          <button class="btn primary" id="saveBtn" data-testid="save">Save</button>
          <span class="status" id="saveStatus">Not saved</span>
        </div>
      </div>
      <section id="content" data-testid="content" class="empty">Loading...</section>
    </main>
  </div>
  <script>
    const state = {
      review: null,
      summary: null,
      inputPath: "",
      outPath: "",
      selected: 0,
      filtered: [],
      dirty: false,
      saving: false,
      saveTimer: null
    };

    const labelModes = [
      "hard_preference",
      "soft_preference",
      "acceptable_set",
      "no_signal",
      "needs_rollout",
      "abstain"
    ];

    const evidenceModes = [
      "human_prior",
      "rollout_backed",
      "regression_case",
      "uncertain"
    ];

    const rationaleTags = [
      "early_act",
      "late_act",
      "deck_unformed",
      "needs_frontload",
      "lacks_draw",
      "lacks_block",
      "lacks_scaling",
      "needs_aoe",
      "hp_pressure",
      "boss_counter",
      "avoid_deck_bloat",
      "already_lost",
      "synergy_piece",
      "upgrade_value",
      "remove_basic",
      "route_risk",
      "potion_reliance",
      "low_curve",
      "high_curve",
      "small_gap",
      "uncertain"
    ];

    function byId(id) {
      return document.getElementById(id);
    }

    async function loadReview() {
      const response = await fetch("/api/review");
      const payload = await response.json();
      state.review = payload.review;
      state.summary = payload.summary;
      state.inputPath = payload.input_path;
      state.outPath = payload.out_path;
      renderAll();
      setStatus("Loaded");
    }

    function renderAll() {
      renderSummary();
      renderList();
      renderScenario();
    }

    function renderSummary() {
      const summary = computeSummary();
      byId("metricTotal").textContent = summary.total;
      byId("metricReviewed").textContent = summary.reviewed;
      byId("metricAttention").textContent = summary.attention;
      byId("pathInfo").textContent = `out: ${state.outPath}`;
    }

    function computeSummary() {
      const scenarios = getScenarios();
      let reviewed = 0;
      let attention = 0;
      for (const scenario of scenarios) {
        const label = scenario.human_label || {};
        const hasRoles = ["preferred", "acceptable", "bad"].some((role) => (label[role] || []).length > 0);
        if (["no_signal", "needs_rollout", "abstain"].includes(label.mode) || hasRoles || (label.reason || "").trim()) {
          reviewed += 1;
        }
        if (["needs_rollout", "abstain"].includes(label.mode)) {
          attention += 1;
        }
      }
      return { total: scenarios.length, reviewed, attention };
    }

    function getScenarios() {
      return (state.review && state.review.scenarios) || [];
    }

    function renderList() {
      const list = byId("scenarioList");
      const query = byId("search").value.trim().toLowerCase();
      const filter = byId("filter").value;
      const scenarios = getScenarios();
      state.filtered = scenarios
        .map((scenario, index) => ({ scenario, index }))
        .filter(({ scenario }) => matchesQuery(scenario, query) && matchesFilter(scenario, filter));
      list.innerHTML = "";
      if (!state.filtered.length) {
        list.innerHTML = '<div class="empty">No scenarios match.</div>';
        return;
      }
      for (const row of state.filtered) {
        const scenario = row.scenario;
        const label = scenario.human_label || {};
        const button = document.createElement("button");
        button.className = "item" + (row.index === state.selected ? " active" : "");
        button.dataset.testid = "scenario-item";
        button.onclick = () => {
          state.selected = row.index;
          renderAll();
        };
        const left = document.createElement("div");
        const title = document.createElement("div");
        title.className = "item-title";
        title.textContent = scenario.id;
        const meta = document.createElement("div");
        meta.className = "item-meta";
        meta.textContent = `${scenario.category} / ${scenario.decision_type}`;
        left.appendChild(title);
        left.appendChild(meta);
        const badge = document.createElement("span");
        badge.className = "badge " + badgeClassForMode(label.mode);
        badge.textContent = label.mode || "unlabeled";
        button.appendChild(left);
        button.appendChild(badge);
        list.appendChild(button);
      }
    }

    function matchesQuery(scenario, query) {
      if (!query) return true;
      const haystack = [
        scenario.id,
        scenario.category,
        scenario.decision_type,
        scenario.context_note,
        ...objectSearchValues(scenario.observation),
        ...objectSearchValues(scenario.deck),
        ...(scenario.relics || []),
        ...(scenario.potions || []),
        ...(scenario.feature_limitations || []),
        ...((scenario.human_label || {}).rationale_tags || []),
        (scenario.human_label || {}).evidence || "",
        ...(scenario.candidates || []).map((candidate) => `${candidate.id} ${candidate.label} ${candidate.card || ""}`)
      ].join(" ").toLowerCase();
      return haystack.includes(query);
    }

    function matchesFilter(scenario, filter) {
      const label = scenario.human_label || {};
      if (filter === "all") return true;
      if (filter === "needs_attention") return ["needs_rollout", "abstain"].includes(label.mode);
      if (filter === "no_signal") return label.mode === "no_signal";
      if (filter === "needs_rollout") return label.mode === "needs_rollout";
      if (filter === "bad_model") {
        const rankings = scenario.policy_rankings || {};
        const ppoKey = Object.keys(rankings).find((key) => key.includes("ppo") || key.includes("model"));
        const top = ppoKey && rankings[ppoKey] && rankings[ppoKey][0] && rankings[ppoKey][0].candidate_id;
        return (label.bad || []).includes(top);
      }
      return true;
    }

    function renderScenario() {
      const scenarios = getScenarios();
      const scenario = scenarios[state.selected];
      const content = byId("content");
      if (!scenario) {
        content.className = "empty";
        content.textContent = "No scenario selected.";
        return;
      }
      content.className = "";
      const label = ensureLabel(scenario);
      const position = state.selected + 1;
      byId("positionBadge").textContent = `${position} / ${scenarios.length}`;
      byId("modeBadge").textContent = label.mode;
      byId("modeBadge").className = "badge " + badgeClassForMode(label.mode);
      byId("categoryBadge").textContent = scenario.category || "unknown";
      content.innerHTML = `
        <section class="panel">
          <div class="panel-head">
            <h2>${escapeHtml(scenario.id)}</h2>
            <div class="context">${escapeHtml(scenario.context_note || "")}</div>
          </div>
          <div class="panel-body">
            ${renderScenarioContext(scenario)}
            <div class="mode-row">
              <select class="mode-select" id="labelMode" data-testid="label-mode">
                ${labelModes.map((mode) => `<option value="${mode}" ${mode === label.mode ? "selected" : ""}>${mode}</option>`).join("")}
              </select>
              <select class="mode-select" id="evidenceMode" data-testid="evidence-mode">
                ${evidenceModes.map((mode) => `<option value="${mode}" ${mode === label.evidence ? "selected" : ""}>${mode}</option>`).join("")}
              </select>
              <div class="confidence-wrap">
                <input id="confidence" data-testid="confidence" type="range" min="0" max="1" step="0.01" value="${Number(label.confidence || 0).toFixed(2)}">
                <span id="confidenceText">${Number(label.confidence || 0).toFixed(2)}</span>
              </div>
              <button class="btn" id="clearRoles" data-testid="clear-roles">Clear roles</button>
            </div>
            <div class="tag-section">
              <div class="tag-title">Rationale tags</div>
              <div class="tag-grid">${renderRationaleTags(label)}</div>
            </div>
            <textarea id="reason" data-testid="reason" placeholder="Reason / uncertainty / what rollout should check">${escapeHtml(label.reason || "")}</textarea>
          </div>
        </section>
        <div class="grid2">
          <section class="panel">
            <div class="panel-head"><h2>Candidates</h2></div>
            <div class="panel-body">${renderCandidateTable(scenario)}</div>
          </section>
          <section class="panel">
            <div class="panel-head"><h2>Policy Rankings</h2></div>
            <div class="panel-body"><div class="ranking">${renderRankings(scenario)}</div></div>
          </section>
        </div>
      `;
      wireScenarioControls(scenario);
    }

    function objectSearchValues(value) {
      if (!value || typeof value !== "object") return [];
      if (Array.isArray(value)) return value.map((item) => String(item));
      return Object.entries(value).map(([key, item]) => `${key} ${String(item)}`);
    }

    function renderScenarioContext(scenario) {
      const cards = [
        renderKvCard("Observation", scenario.observation),
        renderKvCard("Deck summary", scenario.deck),
        renderListCard("Relics", scenario.relics),
        renderListCard("Potions", scenario.potions),
        renderListCard("Limitations", scenario.feature_limitations),
      ].filter(Boolean);
      if (!cards.length) {
        return '<div class="context-card"><div class="context-title">Context</div><div class="kv-value">No structured context in this review file.</div></div>';
      }
      return `<div class="context-grid">${cards.join("")}</div>`;
    }

    function renderKvCard(title, value) {
      if (!value || typeof value !== "object" || Array.isArray(value)) return "";
      const entries = Object.entries(value).filter(([, item]) => item !== undefined && item !== null && item !== "");
      if (!entries.length) return "";
      return `
        <div class="context-card">
          <div class="context-title">${escapeHtml(title)}</div>
          <div class="kv">
            ${entries.map(([key, item]) => `
              <div class="kv-key">${escapeHtml(key)}</div>
              <div class="kv-value">${escapeHtml(formatContextValue(item))}</div>
            `).join("")}
          </div>
        </div>
      `;
    }

    function renderListCard(title, value) {
      const items = Array.isArray(value) ? value.filter((item) => item !== undefined && item !== null && item !== "") : [];
      if (!items.length) return "";
      return `
        <div class="context-card">
          <div class="context-title">${escapeHtml(title)}</div>
          <div class="pill-list">
            ${items.map((item) => `<span class="pill">${escapeHtml(formatContextValue(item))}</span>`).join("")}
          </div>
        </div>
      `;
    }

    function formatContextValue(value) {
      if (Array.isArray(value)) return value.join(", ");
      if (value && typeof value === "object") return JSON.stringify(value);
      return String(value);
    }

    function renderCandidateTable(scenario) {
      const label = ensureLabel(scenario);
      return `
        <table>
          <thead>
            <tr>
              <th style="width: 42%">Candidate</th>
              <th style="width: 18%">Rule score</th>
              <th style="width: 40%">Your label</th>
            </tr>
          </thead>
          <tbody>
            ${(scenario.candidates || []).map((candidate) => {
              const role = roleForCandidate(label, candidate.id);
              return `
                <tr>
                  <td>
                    <div class="candidate-name">${escapeHtml(candidate.label || candidate.id)}</div>
                    <div class="candidate-sub">${escapeHtml(candidate.id)}${candidate.card ? " / " + escapeHtml(candidate.card) : ""}</div>
                    <div class="candidate-sub">${escapeHtml(candidate.action_key || "")}</div>
                  </td>
                  <td>${formatScore(candidate.rule_score)}</td>
                  <td>
                    <div class="role-group" data-candidate-id="${escapeAttr(candidate.id)}">
                      ${roleButton(candidate.id, "preferred", role)}
                      ${roleButton(candidate.id, "acceptable", role)}
                      ${roleButton(candidate.id, "bad", role)}
                      ${roleButton(candidate.id, "none", role)}
                    </div>
                  </td>
                </tr>
              `;
            }).join("")}
          </tbody>
        </table>
      `;
    }

    function roleButton(candidateId, role, selectedRole) {
      const selected = role === selectedRole;
      const label = role === "preferred" ? "pref" : role === "acceptable" ? "ok" : role;
      return `<button class="role-btn ${role} ${selected ? "selected" : ""}" data-testid="role-${role}" data-role="${role}" data-candidate-id="${escapeAttr(candidateId)}">${label}</button>`;
    }

    function renderRationaleTags(label) {
      const selected = new Set(label.rationale_tags || []);
      return rationaleTags.map((tag) => {
        const isSelected = selected.has(tag);
        return `<button class="tag-btn ${isSelected ? "selected" : ""}" data-testid="rationale-tag" data-tag="${escapeAttr(tag)}">${escapeHtml(tag)}</button>`;
      }).join("");
    }

    function renderRankings(scenario) {
      const rankings = scenario.policy_rankings || {};
      const keys = Object.keys(rankings);
      if (!keys.length) return '<div class="empty">No policy rankings.</div>';
      return keys.map((key) => {
        const rows = rankings[key] || [];
        return `
          <div class="rank-card">
            <div class="rank-title">${escapeHtml(key)} top: ${escapeHtml((rows[0] && rows[0].candidate_id) || "none")}</div>
            <div class="rank-list">
              ${rows.map((row) => `
                <div class="rank-row ${row.rank === 1 ? "top" : ""}">
                  <span>#${row.rank}</span>
                  <span>${escapeHtml(row.label || row.candidate_id)}</span>
                  <span>${formatScore(row.score)}</span>
                </div>
              `).join("")}
            </div>
          </div>
        `;
      }).join("");
    }

    function wireScenarioControls(scenario) {
      byId("labelMode").onchange = (event) => {
        ensureLabel(scenario).mode = event.target.value;
        markDirty();
        renderAll();
      };
      byId("evidenceMode").onchange = (event) => {
        ensureLabel(scenario).evidence = event.target.value;
        markDirty();
        renderAll();
      };
      byId("confidence").oninput = (event) => {
        ensureLabel(scenario).confidence = Number(event.target.value);
        byId("confidenceText").textContent = Number(event.target.value).toFixed(2);
        markDirty();
      };
      byId("reason").oninput = (event) => {
        ensureLabel(scenario).reason = event.target.value;
        markDirty();
      };
      byId("clearRoles").onclick = () => {
        const label = ensureLabel(scenario);
        label.preferred = [];
        label.acceptable = [];
        label.bad = [];
        markDirty();
        renderAll();
      };
      for (const button of document.querySelectorAll(".role-btn")) {
        button.onclick = () => {
          setCandidateRole(scenario, button.dataset.candidateId, button.dataset.role);
          markDirty();
          renderAll();
        };
      }
      for (const button of document.querySelectorAll(".tag-btn")) {
        button.onclick = () => {
          toggleRationaleTag(scenario, button.dataset.tag);
          markDirty();
          renderAll();
        };
      }
    }

    function ensureLabel(scenario) {
      if (!scenario.human_label) {
        scenario.human_label = {};
      }
      const label = scenario.human_label;
      label.mode = label.mode || "abstain";
      label.preferred = Array.isArray(label.preferred) ? label.preferred : [];
      label.acceptable = Array.isArray(label.acceptable) ? label.acceptable : [];
      label.bad = Array.isArray(label.bad) ? label.bad : [];
      label.confidence = Number.isFinite(Number(label.confidence)) ? Number(label.confidence) : 0.5;
      label.rationale_tags = Array.isArray(label.rationale_tags) ? label.rationale_tags : [];
      label.evidence = label.evidence || "human_prior";
      label.reason = label.reason || "";
      return label;
    }

    function roleForCandidate(label, candidateId) {
      if ((label.preferred || []).includes(candidateId)) return "preferred";
      if ((label.acceptable || []).includes(candidateId)) return "acceptable";
      if ((label.bad || []).includes(candidateId)) return "bad";
      return "none";
    }

    function setCandidateRole(scenario, candidateId, role) {
      const label = ensureLabel(scenario);
      for (const key of ["preferred", "acceptable", "bad"]) {
        label[key] = (label[key] || []).filter((id) => id !== candidateId);
      }
      if (role !== "none") {
        label[role].push(candidateId);
      }
    }

    function toggleRationaleTag(scenario, tag) {
      const label = ensureLabel(scenario);
      const tags = new Set(label.rationale_tags || []);
      if (tags.has(tag)) {
        tags.delete(tag);
      } else {
        tags.add(tag);
      }
      label.rationale_tags = Array.from(tags);
    }

    function markDirty() {
      state.dirty = true;
      setStatus("Unsaved");
      clearTimeout(state.saveTimer);
      state.saveTimer = setTimeout(() => saveReview(false), 900);
    }

    async function saveReview(explicit) {
      if (state.saving || (!state.dirty && !explicit)) return;
      state.saving = true;
      setStatus("Saving...");
      try {
        const response = await fetch("/api/review", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ scenarios: getScenarios().map((scenario) => ({ id: scenario.id, human_label: scenario.human_label })) })
        });
        const result = await response.json();
        if (!result.ok) throw new Error(result.error || "save failed");
        state.dirty = false;
        state.summary = result.summary;
        renderSummary();
        setStatus("Saved " + new Date().toLocaleTimeString());
      } catch (error) {
        setStatus("Save failed: " + error.message);
      } finally {
        state.saving = false;
      }
    }

    function setStatus(text) {
      byId("saveStatus").textContent = text;
    }

    function selectRelative(delta) {
      const scenarios = getScenarios();
      state.selected = Math.max(0, Math.min(scenarios.length - 1, state.selected + delta));
      renderAll();
    }

    function badgeClassForMode(mode) {
      if (mode === "no_signal") return "good";
      if (mode === "needs_rollout" || mode === "abstain") return "warn";
      if (mode === "hard_preference") return "bad";
      return "";
    }

    function formatScore(value) {
      if (value === undefined || value === null || value === "") return "";
      const num = Number(value);
      if (!Number.isFinite(num)) return String(value);
      return num.toFixed(Math.abs(num) >= 10 ? 1 : 3);
    }

    function escapeHtml(value) {
      return String(value ?? "")
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;")
        .replaceAll("'", "&#39;");
    }

    function escapeAttr(value) {
      return escapeHtml(value);
    }

    byId("search").oninput = renderList;
    byId("filter").onchange = renderList;
    byId("saveBtn").onclick = () => saveReview(true);
    byId("prevBtn").onclick = () => selectRelative(-1);
    byId("nextBtn").onclick = () => selectRelative(1);
    window.addEventListener("keydown", (event) => {
      if (event.target && ["INPUT", "TEXTAREA", "SELECT"].includes(event.target.tagName)) return;
      if (event.key === "ArrowLeft") selectRelative(-1);
      if (event.key === "ArrowRight") selectRelative(1);
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
        event.preventDefault();
        saveReview(true);
      }
    });
    window.addEventListener("beforeunload", (event) => {
      if (!state.dirty) return;
      event.preventDefault();
      event.returnValue = "";
    });

    loadReview().catch((error) => {
      byId("content").className = "empty";
      byId("content").textContent = "Failed to load review: " + error.message;
      setStatus("Load failed");
    });
  </script>
</body>
</html>
"""


if __name__ == "__main__":
    raise SystemExit(main())
