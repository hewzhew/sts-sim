#!/usr/bin/env python3
"""Inspect saved DecisionCaseV1 artifacts.

This is intentionally a diagnostic helper. It does not run evaluators, replay,
or generate labels.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
CASES_DIR = REPO_ROOT / "tools" / "artifacts" / "cases"


def compact(value: Any, limit: int = 160) -> str:
    text = json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    if len(text) <= limit:
        return text
    return text[: limit - 3] + "..."


def case_files() -> list[Path]:
    if not CASES_DIR.exists():
        return []
    return sorted(CASES_DIR.glob("*.json"), key=lambda path: path.stat().st_mtime, reverse=True)


def load_case(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def resolve_case(ref: str) -> Path:
    raw = Path(ref)
    if raw.exists():
        return raw
    candidates = case_files()
    for path in candidates:
        if path.name == ref or path.stem == ref:
            return path
    matches = [path for path in candidates if ref in path.name or ref in path.stem]
    if len(matches) == 1:
        return matches[0]
    if not matches:
        raise SystemExit(f"case not found: {ref}")
    raise SystemExit("ambiguous case ref:\n" + "\n".join(str(path) for path in matches[:20]))


def selected_action_summary(case: dict[str, Any]) -> str:
    selected = case.get("selected_action") or {}
    action_id = selected.get("action_id")
    action_key = selected.get("action_key")
    candidate = selected.get("candidate") or {}
    descriptor = candidate.get("action_descriptor") or (candidate.get("payload") or {}).get("semantic_descriptor") or {}
    label = descriptor.get("label") or action_key
    return f"id={action_id} {label}"


def list_cases(limit: int) -> None:
    files = case_files()[:limit]
    if not files:
        print(f"no cases found in {CASES_DIR}")
        return
    print("case_id | step | decision | floor | selected | note")
    print("-" * 110)
    for path in files:
        try:
            case = load_case(path)
        except Exception as err:
            print(f"{path.name} | unreadable: {err}")
            continue
        state = case.get("public_state_before") or {}
        print(
            f"{case.get('case_id') or path.stem} | "
            f"{case.get('step')} | "
            f"{case.get('decision_type')} | "
            f"{state.get('floor')} | "
            f"{selected_action_summary(case)} | "
            f"{case.get('human_note') or ''}"
        )


def show_case(ref: str, as_json: bool) -> None:
    path = resolve_case(ref)
    case = load_case(path)
    if as_json:
        print(json.dumps(case, ensure_ascii=False, indent=2))
        return
    state = case.get("public_state_before") or {}
    print(f"Path: {path}")
    print(f"Case: {case.get('case_id')}")
    print(f"Role: {case.get('case_role')} label={case.get('label_role')}")
    print(f"Step: {case.get('step')} decision={case.get('decision_type')} floor={state.get('floor')}")
    print(f"Run: {compact(case.get('run_metadata') or {}, 420)}")
    print(f"Selected: {selected_action_summary(case)}")
    if case.get("human_note"):
        print(f"Human note: {case.get('human_note')}")
    if case.get("suspected_issue"):
        print(f"Suspected issue: {case.get('suspected_issue')}")
    rewards = case.get("reward_card_choices") or []
    if rewards:
        print("Reward card choices:")
        for option in rewards[:12]:
            descriptor = option.get("semantic_descriptor") or {}
            label = descriptor.get("label") or option.get("card_name") or option.get("card_id")
            print(
                "  "
                + f"{option.get('option_index')} {label} "
                + f"{option.get('card_type')} {option.get('rarity')} cost={option.get('cost')}"
            )
    route = case.get("map_route_context") or {}
    choices = route.get("route_choices") or []
    if choices:
        print("Route choices:")
        for choice in choices[:8]:
            print(
                "  "
                + f"{choice.get('action_key')} {choice.get('room_label')} "
                + f"paths={choice.get('reachable_paths_to_boss')} "
                + f"risk={compact(choice.get('risk_vector') or choice.get('risk_label'), 220)}"
            )
    print("Warnings:")
    for warning in case.get("truth_warnings") or []:
        print(f"  - {warning}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Inspect DecisionCaseV1 artifacts.")
    sub = parser.add_subparsers(dest="cmd", required=True)
    list_parser = sub.add_parser("list", help="List recent saved cases.")
    list_parser.add_argument("--limit", type=int, default=20)
    show_parser = sub.add_parser("show", help="Show a saved case summary.")
    show_parser.add_argument("case")
    show_parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    if args.cmd == "list":
        list_cases(args.limit)
    elif args.cmd == "show":
        show_case(args.case, args.json)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
