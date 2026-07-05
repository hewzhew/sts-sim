#!/usr/bin/env python3
"""Render branch_tiny path/capsule choices for human review."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


def main() -> int:
    args = parse_args()
    print(render_source(Path(args.source), max_candidates=args.max_candidates))
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", help="capsule directory or path/result/frontier/terminal json")
    parser.add_argument("--max-candidates", type=int, default=12)
    return parser.parse_args()


def render_source(source: Path, max_candidates: int = 12) -> str:
    paths = load_review_paths(source)
    if not paths:
        raise SystemExit(f"no path data found in {source}")
    chunks = []
    for index, review_path in enumerate(paths):
        if len(paths) > 1:
            chunks.append(f"== {review_path['title']} ==")
        chunks.extend(render_steps(review_path["steps"], max_candidates=max_candidates))
        if index + 1 < len(paths):
            chunks.append("")
    return "\n".join(chunks)


def load_review_paths(source: Path) -> list[dict[str, Any]]:
    if source.is_dir():
        for name in ("path.json", "result.json", "terminal.json", "frontier.json"):
            candidate = source / name
            if candidate.exists():
                paths = review_paths_from_value(read_json(candidate), candidate.name)
                if paths:
                    return paths
        return []
    return review_paths_from_value(read_json(source), source.name)


def review_paths_from_value(value: Any, title: str) -> list[dict[str, Any]]:
    if not isinstance(value, dict):
        return []
    if isinstance(value.get("steps"), list):
        return [{"title": title, "steps": value["steps"]}]
    path = value.get("path")
    if isinstance(path, dict) and isinstance(path.get("steps"), list):
        return [{"title": title, "steps": path["steps"]}]
    if isinstance(path, list):
        return [{"title": title, "steps": path}]
    frontier = value.get("frontier")
    if isinstance(frontier, list):
        paths = []
        for branch in frontier:
            if isinstance(branch, dict) and isinstance(branch.get("path"), list):
                paths.append(
                    {
                        "title": f"{title}:branch-{branch.get('id', len(paths))}",
                        "steps": branch["path"],
                    }
                )
        return paths
    terminals = value.get("terminals")
    if isinstance(terminals, list):
        paths = []
        for terminal in terminals:
            paths.extend(review_paths_from_value(terminal, title))
        return paths
    return []


def render_steps(steps: list[dict[str, Any]], max_candidates: int) -> list[str]:
    lines: list[str] = []
    for index, step in enumerate(steps):
        state = step.get("state_before") or {}
        prefix = state_line(index, state)
        selected = step.get("label") or "-"
        lines.append(prefix)
        lines.append(f"  selected: {selected}")
        pool = step.get("candidate_pool") or []
        if pool:
            lines.append("  candidates:")
            for candidate in pool[:max_candidates]:
                lines.extend(render_candidate(candidate))
            hidden = len(pool) - max_candidates
            if hidden > 0:
                lines.append(f"    ... {hidden} more candidates")
        annotation = step.get("annotation")
        if not pool and annotation and annotation != {"kind": "none"}:
            lines.append(f"  annotation: {compact_json(annotation)}")
        lines.append("")
    if lines and lines[-1] == "":
        lines.pop()
    return lines


def state_line(index: int, state: dict[str, Any]) -> str:
    act = state.get("act", "?")
    floor = state.get("floor", "?")
    hp = state.get("hp", "?")
    max_hp = state.get("max_hp", "?")
    gold = state.get("gold", "?")
    deck = state.get("deck_size", "?")
    boundary = state.get("boundary", "?")
    return f"{index:02} A{act}F{floor} hp={hp}/{max_hp} gold={gold} deck={deck} boundary={boundary}"


def render_candidate(candidate: dict[str, Any]) -> list[str]:
    selected = "[x]" if candidate.get("selected") else "[ ]"
    rank = candidate.get("rank", "?")
    label = candidate.get("label") or "-"
    auto = "auto" if candidate.get("auto_expand") else "hold"
    annotation = candidate.get("annotation") if isinstance(candidate.get("annotation"), dict) else {}
    lane = annotation.get("lane")
    score = annotation.get("score")
    parts = [selected, f"{rank}.", label, auto]
    if lane is not None:
        parts.append(f"lane={lane}")
    if score is not None:
        parts.append(f"score={score}")
    summary = candidate_summary(annotation.get("candidate"))
    if summary:
        parts.append(summary)
    lines = ["    " + " ".join(str(part) for part in parts)]
    inspect = candidate.get("inspect_only")
    if inspect:
        lines.append(f"      inspect={inspect}")
    return lines


def candidate_summary(candidate: Any) -> str:
    if not isinstance(candidate, dict):
        return ""
    kind = candidate.get("kind")
    if kind == "shop_buy_card":
        return "{} {}+{} {}g".format(
            kind,
            candidate.get("card", "?"),
            candidate.get("upgrades", "?"),
            candidate.get("price", "?"),
        )
    if kind == "card_reward_pick":
        return "{} {}+{}".format(
            kind, candidate.get("card", "?"), candidate.get("upgrades", "?")
        )
    if kind == "shop_buy_relic":
        return "{} {} {}g".format(kind, candidate.get("relic", "?"), candidate.get("price", "?"))
    if kind == "shop_buy_potion":
        return "{} {} {}g".format(
            kind, candidate.get("potion", "?"), candidate.get("price", "?")
        )
    if kind == "shop_purge":
        target = candidate.get("target")
        if isinstance(target, dict):
            return f"{kind} {target.get('label') or target.get('kind') or '?'}"
    return str(kind) if kind else compact_json(candidate)


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def dumps_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, indent=2)


def compact_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


if __name__ == "__main__":
    raise SystemExit(main())
