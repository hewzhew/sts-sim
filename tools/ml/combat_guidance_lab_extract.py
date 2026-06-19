#!/usr/bin/env python3
"""Summarize guidance-lab reports and export candidate probe samples.

This reads CombatSearchGuidanceLabV1Report or
CombatSearchGuidanceLabBenchmarkV1Report JSON files.  It does not train a
model.  The JSONL export is meant as a stable handoff between Rust search
diagnostics and later offline ranking experiments.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any, Iterable


SAMPLE_SCHEMA = "CombatActionProbeSampleV1"
SAMPLE_VERSION = 1
LABEL_ROLE = "oracle_under_budget_child_search_target_not_human_policy"


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def iter_labs(path: Path, payload: Any) -> Iterable[tuple[dict[str, Any], dict[str, Any]]]:
    if not isinstance(payload, dict):
        return
    schema = payload.get("schema_name")
    if schema == "CombatSearchGuidanceLabV1Report":
        yield (
            {
                "source_file": str(path),
                "benchmark_name": None,
                "case_id": None,
                "input_kind": None,
                "input_path": None,
            },
            payload,
        )
        return
    if schema == "CombatSearchGuidanceLabBenchmarkV1Report":
        benchmark_name = payload.get("benchmark_name")
        for case in payload.get("cases", []):
            if not isinstance(case, dict) or not isinstance(case.get("lab"), dict):
                continue
            yield (
                {
                    "source_file": str(path),
                    "benchmark_name": benchmark_name,
                    "case_id": case.get("id"),
                    "input_kind": case.get("input_kind"),
                    "input_path": case.get("input_path"),
                },
                case["lab"],
            )


def target_tier(target: dict[str, Any]) -> int:
    terminal = target.get("terminal")
    if target.get("complete_win") and terminal == "win":
        return 3
    if terminal == "win":
        return 2
    if terminal == "unresolved":
        return 1
    return 0


def target_sort_key(candidate: dict[str, Any]) -> tuple[int, int, int, int]:
    target = candidate.get("target") if isinstance(candidate.get("target"), dict) else {}
    return (
        target_tier(target),
        int_or_min(target.get("final_hp")),
        -int_or_max(target.get("child_search_hp_loss")),
        -int_or_max(target.get("nodes_expanded")),
    )


def outcome_sort_key(candidate: dict[str, Any] | None) -> tuple[int, int, int]:
    if not candidate:
        return (-1, -10**9, -10**9)
    target = candidate.get("target") if isinstance(candidate.get("target"), dict) else {}
    return (
        target_tier(target),
        int_or_min(target.get("final_hp")),
        -int_or_max(target.get("child_search_hp_loss")),
    )


def int_or_min(value: Any) -> int:
    return value if isinstance(value, int) else -10**9


def int_or_max(value: Any) -> int:
    return value if isinstance(value, int) else 10**9


def best_target_candidate(lab: dict[str, Any]) -> dict[str, Any] | None:
    candidates = [candidate for candidate in lab.get("candidates", []) if isinstance(candidate, dict)]
    if not candidates:
        return None
    return max(
        candidates,
        key=lambda candidate: (target_sort_key(candidate), -int_or_max(candidate.get("ordered_index"))),
    )


def current_first_candidate(lab: dict[str, Any]) -> dict[str, Any] | None:
    candidates = [candidate for candidate in lab.get("candidates", []) if isinstance(candidate, dict)]
    if not candidates:
        return None
    return min(candidates, key=lambda candidate: int_or_max(candidate.get("ordered_index")))


def selected_best_complete_candidate(lab: dict[str, Any]) -> dict[str, Any] | None:
    for candidate in lab.get("candidates", []):
        if isinstance(candidate, dict) and candidate.get("selected_by_best_complete"):
            return candidate
    return None


def candidate_short(candidate: dict[str, Any] | None) -> str:
    if not candidate:
        return "-"
    target = candidate.get("target") if isinstance(candidate.get("target"), dict) else {}
    return (
        f"idx={candidate.get('ordered_index')} role={candidate.get('action_role')} "
        f"terminal={target.get('terminal')} win={target.get('complete_win')} "
        f"final_hp={target.get('final_hp')} child_loss={target.get('child_search_hp_loss')} "
        f"nodes={target.get('nodes_expanded')} action={candidate.get('action_key')}"
    )


def lab_summary(meta: dict[str, Any], lab: dict[str, Any]) -> dict[str, Any]:
    current = current_first_candidate(lab)
    best = best_target_candidate(lab)
    selected = selected_best_complete_candidate(lab)
    summary = lab.get("summary") if isinstance(lab.get("summary"), dict) else {}
    return {
        "case_id": meta.get("case_id") or lab.get("input_label"),
        "candidate_count": summary.get("candidate_count"),
        "child_searches_run": summary.get("child_searches_run"),
        "child_complete_wins": summary.get("child_complete_wins"),
        "current_first": candidate_short(current),
        "best_target": candidate_short(best),
        "best_complete_first": candidate_short(selected),
        "best_target_differs_from_current_first": ordered_index(best) != ordered_index(current),
        "best_target_differs_from_best_complete_first": ordered_index(best) != ordered_index(selected),
        "best_target_outcome_differs_from_current_first": outcome_sort_key(best) != outcome_sort_key(current),
        "best_target_outcome_differs_from_best_complete_first": outcome_sort_key(best)
        != outcome_sort_key(selected),
    }


def ordered_index(candidate: dict[str, Any] | None) -> Any:
    return candidate.get("ordered_index") if candidate else None


def sample_from_candidate(
    meta: dict[str, Any],
    lab: dict[str, Any],
    candidate: dict[str, Any],
    best_index: Any,
) -> dict[str, Any]:
    target = candidate.get("target") if isinstance(candidate.get("target"), dict) else {}
    microscope = ((lab.get("root") or {}).get("microscope") or {})
    return {
        "schema_name": SAMPLE_SCHEMA,
        "schema_version": SAMPLE_VERSION,
        "label_role": LABEL_ROLE,
        "source": {
            **meta,
            "input_label": lab.get("input_label"),
        },
        "root_context": {
            "config": microscope.get("config"),
            "search_outcome": microscope.get("search_outcome"),
            "best_complete_summary": microscope.get("best_complete_summary"),
            "initial_context": microscope.get("initial_context"),
        },
        "candidate": {
            "original_action_id": candidate.get("original_action_id"),
            "ordered_index": candidate.get("ordered_index"),
            "action_key": candidate.get("action_key"),
            "action_debug": candidate.get("action_debug"),
            "action_role": candidate.get("action_role"),
            "selected_by_best_complete": candidate.get("selected_by_best_complete"),
            "one_step_status": candidate.get("one_step_status"),
            "one_step_terminal": candidate.get("one_step_terminal"),
        },
        "target": {
            "target_kind": target.get("target_kind"),
            "source": target.get("source"),
            "terminal": target.get("terminal"),
            "complete_win": target.get("complete_win"),
            "post_root_player_hp": target.get("post_root_player_hp"),
            "child_search_hp_loss": target.get("child_search_hp_loss"),
            "final_hp": target.get("final_hp"),
            "nodes_expanded": target.get("nodes_expanded"),
            "is_best_target_candidate": candidate.get("ordered_index") == best_index,
            "limitations": target.get("limitations") or [],
        },
        "child_search": candidate.get("child_search"),
    }


def summarize(paths: list[Path], out_jsonl: Path | None) -> None:
    labs: list[tuple[dict[str, Any], dict[str, Any]]] = []
    for path in paths:
        labs.extend(iter_labs(path, load_json(path)))

    case_summaries = [lab_summary(meta, lab) for meta, lab in labs]
    counters = Counter()
    total_candidates = 0
    total_child = 0
    total_wins = 0
    for meta, lab in labs:
        summary = lab.get("summary") if isinstance(lab.get("summary"), dict) else {}
        counters["cases"] += 1
        total_candidates += int_or_zero(summary.get("candidate_count"))
        total_child += int_or_zero(summary.get("child_searches_run"))
        total_wins += int_or_zero(summary.get("child_complete_wins"))
        if lab_summary(meta, lab)["best_target_differs_from_current_first"]:
            counters["target_diff_current_first"] += 1
        if lab_summary(meta, lab)["best_target_differs_from_best_complete_first"]:
            counters["target_diff_best_complete_first"] += 1
        if lab_summary(meta, lab)["best_target_outcome_differs_from_current_first"]:
            counters["outcome_diff_current_first"] += 1
        if lab_summary(meta, lab)["best_target_outcome_differs_from_best_complete_first"]:
            counters["outcome_diff_best_complete_first"] += 1

    if out_jsonl:
        out_jsonl.parent.mkdir(parents=True, exist_ok=True)
        with out_jsonl.open("w", encoding="utf-8") as handle:
            for meta, lab in labs:
                best = best_target_candidate(lab)
                best_index = ordered_index(best)
                for candidate in lab.get("candidates", []):
                    if isinstance(candidate, dict):
                        sample = sample_from_candidate(meta, lab, candidate, best_index)
                        handle.write(json.dumps(sample, ensure_ascii=False, separators=(",", ":")))
                        handle.write("\n")

    print("CombatGuidanceLabExtract")
    print(
        f"  cases={counters['cases']} candidates={total_candidates} "
        f"child_searches={total_child} child_complete_wins={total_wins}"
    )
    print(
        f"  target_diff_current_first={counters['target_diff_current_first']} "
        f"target_diff_best_complete_first={counters['target_diff_best_complete_first']}"
    )
    print(
        f"  outcome_diff_current_first={counters['outcome_diff_current_first']} "
        f"outcome_diff_best_complete_first={counters['outcome_diff_best_complete_first']}"
    )
    if out_jsonl:
        print(f"  jsonl={out_jsonl}")
    print("  cases:")
    for summary in case_summaries[:20]:
        print(f"    case={summary['case_id']}")
        print(f"      current: {summary['current_first']}")
        print(f"      target:  {summary['best_target']}")
        print(f"      best_complete_first: {summary['best_complete_first']}")
    if len(case_summaries) > 20:
        print(f"    ... {len(case_summaries) - 20} more case(s)")


def int_or_zero(value: Any) -> int:
    return value if isinstance(value, int) else 0


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="+", type=Path)
    parser.add_argument("--out-jsonl", type=Path)
    args = parser.parse_args()
    summarize(args.inputs, args.out_jsonl)


if __name__ == "__main__":
    main()
