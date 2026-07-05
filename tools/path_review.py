#!/usr/bin/env python3
"""Render branch_tiny path/capsule choices for human review."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def main() -> int:
    args = parse_args()
    print(
        render_source(
            Path(args.source),
            max_candidates=args.max_candidates,
            boundaries=parse_boundaries(args.boundary),
            contains=parse_contains(args.contains),
            interesting=args.interesting,
            show_summary=args.summary,
            inspect_summary=args.inspect_summary,
            max_summary_examples=args.max_summary_examples,
        )
    )
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", help="capsule directory or path/result/frontier/terminal json")
    parser.add_argument("--max-candidates", type=int, default=12)
    parser.add_argument(
        "--boundary",
        action="append",
        default=[],
        help="Filter boundary names; may be repeated or comma-separated.",
    )
    parser.add_argument(
        "--interesting",
        action="store_true",
        help="Only show steps with multiple candidates or inspect-only reasons.",
    )
    parser.add_argument(
        "--contains",
        action="append",
        default=[],
        help="Only show steps whose selected choice, candidates, or inspect reasons contain text.",
    )
    parser.add_argument(
        "--inspect-summary",
        action="store_true",
        help="Group inspect-only reasons instead of rendering full path steps.",
    )
    parser.add_argument("--max-summary-examples", type=int, default=3)
    parser.add_argument("--summary", action="store_true", help="Print a compact count header.")
    return parser.parse_args()


def render_source(
    source: Path,
    max_candidates: int = 12,
    boundaries: set[str] | None = None,
    contains: list[str] | None = None,
    interesting: bool = False,
    show_summary: bool = False,
    inspect_summary: bool = False,
    max_summary_examples: int = 3,
) -> str:
    paths = load_review_paths(source)
    if not paths:
        raise SystemExit(f"no path data found in {source}")
    if inspect_summary:
        return render_inspect_summary(
            paths, boundaries, contains, interesting, max_summary_examples
        )
    chunks = []
    summary = review_summary(paths, boundaries, contains, interesting)
    if show_summary:
        chunks.append(
            "summary: paths={paths} steps={steps} shown={shown} inspect_reasons={inspect}".format(
                paths=summary["paths"],
                steps=summary["steps"],
                shown=summary["shown"],
                inspect=summary["inspect_reasons"],
            )
        )
    for index, review_path in enumerate(paths):
        selected = select_steps(review_path["steps"], boundaries, contains, interesting)
        if not selected:
            continue
        if len(paths) > 1:
            chunks.append(f"== {review_path['title']} ==")
        chunks.extend(
            render_steps(
                selected,
                max_candidates=max_candidates,
                all_steps=review_path["steps"],
            )
        )
        if index + 1 < len(paths):
            chunks.append("")
    return "\n".join(chunks)


def render_inspect_summary(
    paths: list[dict[str, Any]],
    boundaries: set[str] | None,
    contains: list[str] | None,
    interesting: bool,
    max_examples: int,
) -> str:
    summary = review_summary(paths, boundaries, contains, interesting)
    grouped: dict[str, dict[str, Any]] = {}
    for title, index, step, candidate in inspect_records(
        paths, boundaries, contains, interesting
    ):
        reason = str(candidate.get("inspect_only") or "")
        group = grouped.setdefault(reason, {"count": 0, "examples": []})
        group["count"] += 1
        if len(group["examples"]) < max_examples:
            group["examples"].append(inspect_example(title, index, step, candidate))

    lines = [
        (
            "inspect_summary: paths={paths} steps={steps} "
            "shown={shown} inspect_reasons={inspect}"
        ).format(
            paths=summary["paths"],
            steps=summary["steps"],
            shown=summary["shown"],
            inspect=summary["inspect_reasons"],
        )
    ]
    if not grouped:
        lines.append("(no inspect reasons matched)")
        return "\n".join(lines)

    for reason, group in sorted(
        grouped.items(), key=lambda item: (-item[1]["count"], item[0])
    ):
        lines.append(f"- {group['count']}x {reason}")
        for example in group["examples"]:
            lines.append(f"  example: {example}")
    return "\n".join(lines)


def inspect_records(
    paths: list[dict[str, Any]],
    boundaries: set[str] | None,
    contains: list[str] | None,
    interesting: bool,
) -> list[tuple[str, int, dict[str, Any], dict[str, Any]]]:
    records = []
    for path in paths:
        title = path["title"]
        for index, step in select_steps(path["steps"], boundaries, contains, interesting):
            for candidate in step.get("candidate_pool") or []:
                if candidate.get("inspect_only"):
                    records.append((title, index, step, candidate))
    return records


def inspect_example(
    title: str,
    index: int,
    step: dict[str, Any],
    candidate: dict[str, Any],
) -> str:
    state = step.get("state_before") or {}
    act = state.get("act", "?")
    floor = state.get("floor", "?")
    boundary = state.get("boundary", "?")
    selected = step.get("label") or "-"
    candidate_label = candidate.get("label") or "-"
    return (
        f"{title}#{index:02} A{act}F{floor} {boundary}: "
        f"candidate={candidate_label!r} selected={selected!r}"
    )


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


def review_summary(
    paths: list[dict[str, Any]],
    boundaries: set[str] | None,
    contains: list[str] | None,
    interesting: bool,
) -> dict[str, int]:
    steps = sum(len(path["steps"]) for path in paths)
    selected = [
        item
        for path in paths
        for item in select_steps(path["steps"], boundaries, contains, interesting)
    ]
    return {
        "paths": len(paths),
        "steps": steps,
        "shown": len(selected),
        "inspect_reasons": sum(count_inspect_reasons(step) for _, step in selected),
    }


def select_steps(
    steps: list[dict[str, Any]],
    boundaries: set[str] | None,
    contains: list[str] | None,
    interesting: bool,
) -> list[tuple[int, dict[str, Any]]]:
    selected = []
    for index, step in enumerate(steps):
        if (
            boundaries
            and normalize_boundary((step.get("state_before") or {}).get("boundary"))
            not in boundaries
        ):
            continue
        if contains and not step_contains(step, contains):
            continue
        if interesting and not step_is_interesting(step):
            continue
        selected.append((index, step))
    return selected


def step_is_interesting(step: dict[str, Any]) -> bool:
    pool = step.get("candidate_pool") or []
    return len(pool) > 1 or count_inspect_reasons(step) > 0


def count_inspect_reasons(step: dict[str, Any]) -> int:
    return sum(1 for candidate in step.get("candidate_pool") or [] if candidate.get("inspect_only"))


def parse_boundaries(values: list[str]) -> set[str] | None:
    boundaries = {
        normalize_boundary(part)
        for value in values
        for part in value.split(",")
        if part.strip()
    }
    return boundaries or None


def parse_contains(values: list[str]) -> list[str] | None:
    terms = [
        normalize_search_text(part)
        for value in values
        for part in value.split(",")
        if part.strip()
    ]
    return terms or None


def normalize_boundary(value: Any) -> str:
    return str(value or "").strip().lower()


def normalize_search_text(value: Any) -> str:
    return str(value or "").strip().lower()


def step_contains(step: dict[str, Any], terms: list[str]) -> bool:
    text = "\n".join(step_search_text(step)).lower()
    return any(term in text for term in terms)


def step_search_text(step: dict[str, Any]) -> list[str]:
    values = [str(step.get("label") or "")]
    state = step.get("state_before") or {}
    values.append(str(state.get("boundary") or ""))
    for candidate in step.get("candidate_pool") or []:
        values.append(str(candidate.get("label") or ""))
        values.append(str(candidate.get("inspect_only") or ""))
        annotation = candidate.get("annotation") if isinstance(candidate.get("annotation"), dict) else {}
        values.append(candidate_summary(annotation.get("candidate")))
    return values


def render_steps(
    steps: list[tuple[int, dict[str, Any]]],
    max_candidates: int,
    all_steps: list[dict[str, Any]],
) -> list[str]:
    lines: list[str] = []
    for index, step in steps:
        state = step.get("state_before") or {}
        prefix = state_line(index, state)
        selected = step.get("label") or "-"
        lines.append(prefix)
        lines.append(f"  selected: {selected}")
        transition = hidden_auto_transition(all_steps, index, state)
        if transition:
            lines.append(f"  auto transition: {transition}")
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


def hidden_auto_transition(
    all_steps: list[dict[str, Any]], index: int, state: dict[str, Any]
) -> str:
    if index <= 0 or index >= len(all_steps):
        return ""
    previous = all_steps[index - 1]
    if previous.get("decision_delta"):
        return ""
    previous_state = previous.get("state_before") or {}
    if not isinstance(previous_state, dict) or not isinstance(state, dict):
        return ""

    changes = []
    previous_site = site_summary(previous_state)
    current_site = site_summary(state)
    if previous_site != current_site:
        changes.append(f"{previous_site} -> {current_site}")
    for key, label in (("hp", "hp"), ("gold", "gold"), ("deck_size", "deck")):
        before = previous_state.get(key)
        after = state.get(key)
        if before is not None and after is not None and before != after:
            changes.append(f"{label} {before}->{after}")
    return " ".join(changes) if len(changes) > 1 else ""


def site_summary(state: dict[str, Any]) -> str:
    return f"A{state.get('act', '?')}F{state.get('floor', '?')}"


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
