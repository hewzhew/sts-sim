#!/usr/bin/env python3
"""Summarize PlannerQuestionEvent JSONL streams into a tool-needs backlog.

This script is intentionally offline. It does not call the simulator, does not
call an LLM, and does not make policy-quality claims. It only groups harness
tool-design observations for human review.
"""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


def normalized_tool_name(value: Any) -> str:
    text = str(value or "").strip()
    if not text:
        return "unnamed_tool_need"
    return re.sub(r"[^a-zA-Z0-9_]+", "_", text).strip("_").lower() or "unnamed_tool_need"


def read_events(paths: list[Path]) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    for path in paths:
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if not line.strip():
                continue
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if not isinstance(event, dict):
                continue
            if event.get("schema_name") != "PlannerQuestionEvent":
                continue
            if event.get("status") != "ok":
                continue
            event["_source_file"] = str(path)
            event["_source_line"] = line_number
            events.append(event)
    return events


def summarize(events: list[dict[str, Any]], *, max_examples: int) -> dict[str, Any]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for event in events:
        proposed_tool = event.get("proposed_tool") if isinstance(event.get("proposed_tool"), dict) else {}
        grouped[normalized_tool_name(proposed_tool.get("name"))].append(event)

    clusters = []
    for tool_name, items in sorted(grouped.items(), key=lambda pair: (-len(pair[1]), pair[0])):
        priority_counts = Counter(str(item.get("priority_guess") or "unknown") for item in items)
        answer_counts = Counter(str(item.get("current_tools_can_answer") or "unknown") for item in items)
        decision_counts = Counter(str(item.get("decision_type") or "unknown") for item in items)
        representative = items[0]
        proposed_tool = representative.get("proposed_tool") if isinstance(representative.get("proposed_tool"), dict) else {}
        examples = []
        for item in items[:max_examples]:
            examples.append(
                {
                    "step": item.get("step"),
                    "floor": item.get("floor"),
                    "decision_type": item.get("decision_type"),
                    "question": item.get("question"),
                    "why_action_relevant": item.get("why_action_relevant"),
                    "missing_measurement": item.get("missing_measurement"),
                    "source_file": item.get("_source_file"),
                    "source_line": item.get("_source_line"),
                }
            )
        clusters.append(
            {
                "need_cluster": tool_name,
                "count": len(items),
                "priority_counts": dict(priority_counts),
                "current_tools_can_answer_counts": dict(answer_counts),
                "decision_type_counts": dict(decision_counts),
                "candidate_tool_contract": {
                    "name": proposed_tool.get("name") or tool_name,
                    "inputs": proposed_tool.get("inputs") or [],
                    "outputs": proposed_tool.get("outputs") or [],
                },
                "examples": examples,
                "human_review_status": "unreviewed",
            }
        )
    return {
        "schema_name": "ToolNeedsSummary",
        "schema_version": 1,
        "event_count": len(events),
        "cluster_count": len(clusters),
        "clusters": clusters,
        "policy_quality_claim": False,
        "label_role": "not_a_label",
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("paths", type=Path, nargs="+")
    parser.add_argument("--out", type=Path, default=None)
    parser.add_argument("--max-examples", type=int, default=3)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    events = read_events(args.paths)
    summary = summarize(events, max_examples=args.max_examples)
    text = json.dumps(summary, ensure_ascii=False, indent=2)
    if args.out is not None:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(text + "\n", encoding="utf-8")
    else:
        print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
