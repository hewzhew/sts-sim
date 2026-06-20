#!/usr/bin/env python3
"""Summarize turn-plan guidance-lab reports and export probe samples.

This reads CombatTurnPlanGuidanceLabV1Report or
CombatTurnPlanGuidanceLabBenchmarkV1Report JSON files.  It does not train a
model.  The JSONL export is a stable handoff for later offline turn-plan
ranking experiments.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any, Iterable


SAMPLE_SCHEMA = "CombatTurnPlanProbeSampleV1"
SAMPLE_VERSION = 1
LABEL_ROLE = "oracle_under_budget_turn_plan_child_search_target_not_human_policy"
PUBLIC_MONSTER_FIELDS = (
    "slot",
    "enemy_id",
    "hp",
    "max_hp",
    "block",
    "alive",
    "escaped",
    "dying",
    "half_dead",
    "visible_intent",
    "preview_damage_per_hit",
)


def load_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def iter_labs(path: Path, payload: Any) -> Iterable[tuple[dict[str, Any], dict[str, Any]]]:
    if not isinstance(payload, dict):
        return
    schema = payload.get("schema_name")
    if schema == "CombatTurnPlanGuidanceLabV1Report":
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
    if schema == "CombatTurnPlanGuidanceLabBenchmarkV1Report":
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


def resolve_input_path(report_path: Path, input_path: Any) -> Path | None:
    if not isinstance(input_path, str) or not input_path:
        return None
    path = Path(input_path)
    if path.exists():
        return path
    if path.is_absolute():
        return path
    candidate = report_path.parent / path
    if candidate.exists():
        return candidate
    return path


def public_enemy_slots_from_capture(path: Path | None) -> list[dict[str, Any]]:
    if path is None or not path.exists():
        return []
    try:
        payload = load_json(path)
    except (OSError, json.JSONDecodeError):
        return []
    summary = payload.get("summary") if isinstance(payload, dict) else {}
    monsters = summary.get("monsters") if isinstance(summary, dict) else []
    if not isinstance(monsters, list):
        return []
    out: list[dict[str, Any]] = []
    for monster in monsters:
        if not isinstance(monster, dict):
            continue
        public = {field: monster.get(field) for field in PUBLIC_MONSTER_FIELDS if field in monster}
        if public:
            out.append(public)
    return out


def int_or_min(value: Any) -> int:
    return value if isinstance(value, int) else -10**9


def int_or_max(value: Any) -> int:
    return value if isinstance(value, int) else 10**9


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


def ordered_index(candidate: dict[str, Any] | None) -> Any:
    plan = candidate.get("plan") if isinstance(candidate, dict) else None
    return plan.get("plan_index") if isinstance(plan, dict) else None


def best_target_candidate(lab: dict[str, Any]) -> dict[str, Any] | None:
    candidates = [candidate for candidate in lab.get("candidates", []) if isinstance(candidate, dict)]
    if not candidates:
        return None
    return max(
        candidates,
        key=lambda candidate: (target_sort_key(candidate), -int_or_max(ordered_index(candidate))),
    )


def current_first_candidate(lab: dict[str, Any]) -> dict[str, Any] | None:
    candidates = [candidate for candidate in lab.get("candidates", []) if isinstance(candidate, dict)]
    if not candidates:
        return None
    return min(candidates, key=lambda candidate: int_or_max(ordered_index(candidate)))


def plan_short(candidate: dict[str, Any] | None) -> str:
    if not candidate:
        return "-"
    plan = candidate.get("plan") if isinstance(candidate.get("plan"), dict) else {}
    target = candidate.get("target") if isinstance(candidate.get("target"), dict) else {}
    actions = plan.get("action_keys") or []
    preview = " | ".join(actions[:4])
    if len(actions) > 4:
        preview += " | ..."
    return (
        f"idx={plan.get('plan_index')} bucket={plan.get('bucket')} "
        f"stop={plan.get('stop_reason')} actions={len(actions)} "
        f"terminal={target.get('terminal')} win={target.get('complete_win')} "
        f"final_hp={target.get('final_hp')} nodes={target.get('nodes_expanded')} "
        f"seq=[{preview}]"
    )


def sample_from_candidate(
    meta: dict[str, Any],
    lab: dict[str, Any],
    candidate: dict[str, Any],
    best_index: Any,
    enemy_slots: list[dict[str, Any]],
) -> dict[str, Any]:
    plan = candidate.get("plan") if isinstance(candidate.get("plan"), dict) else {}
    target = candidate.get("target") if isinstance(candidate.get("target"), dict) else {}
    root = lab.get("root") if isinstance(lab.get("root"), dict) else {}
    return {
        "schema_name": SAMPLE_SCHEMA,
        "schema_version": SAMPLE_VERSION,
        "label_role": LABEL_ROLE,
        "source": {
            **meta,
            "input_label": lab.get("input_label"),
        },
        "root_context": {
            "config": root.get("config"),
            "initial_context": root.get("initial_context"),
            "enumeration": root.get("enumeration"),
            "enemy_slots": enemy_slots,
        },
        "plan": plan,
        "target": {
            "target_kind": target.get("target_kind"),
            "source": target.get("source"),
            "terminal": target.get("terminal"),
            "complete_win": target.get("complete_win"),
            "post_root_player_hp": target.get("post_root_player_hp"),
            "child_search_hp_loss": target.get("child_search_hp_loss"),
            "final_hp": target.get("final_hp"),
            "nodes_expanded": target.get("nodes_expanded"),
            "is_best_target_plan": plan.get("plan_index") == best_index,
            "limitations": target.get("limitations") or [],
        },
        "child_search": candidate.get("child_search"),
    }


def summarize(
    paths: list[Path],
    out_jsonl: Path | None,
    *,
    summary_only: bool,
    case_limit: int,
) -> None:
    labs: list[tuple[dict[str, Any], dict[str, Any]]] = []
    for path in paths:
        labs.extend(iter_labs(path, load_json(path)))

    counters: Counter[str] = Counter()
    bucket_counts: Counter[str] = Counter()
    stop_counts: Counter[str] = Counter()
    total_candidates = 0
    total_child = 0
    total_wins = 0
    case_rows = []
    if out_jsonl:
        out_jsonl.parent.mkdir(parents=True, exist_ok=True)
        handle = out_jsonl.open("w", encoding="utf-8")
    else:
        handle = None
    try:
        for meta, lab in labs:
            counters["cases"] += 1
            summary = lab.get("summary") if isinstance(lab.get("summary"), dict) else {}
            total_candidates += int_or_zero(summary.get("candidate_count"))
            total_child += int_or_zero(summary.get("child_searches_run"))
            total_wins += int_or_zero(summary.get("child_complete_wins"))
            current = current_first_candidate(lab)
            best = best_target_candidate(lab)
            if ordered_index(best) != ordered_index(current):
                counters["target_diff_current_first"] += 1
            case_rows.append((meta.get("case_id") or lab.get("input_label"), current, best))
            best_index = ordered_index(best)
            input_path = resolve_input_path(Path(str(meta.get("source_file") or "")), meta.get("input_path"))
            enemy_slots = public_enemy_slots_from_capture(input_path)
            for candidate in lab.get("candidates", []):
                if not isinstance(candidate, dict):
                    continue
                plan = candidate.get("plan") if isinstance(candidate.get("plan"), dict) else {}
                bucket_counts[str(plan.get("bucket") or "unknown")] += 1
                stop_counts[str(plan.get("stop_reason") or "unknown")] += 1
                if handle:
                    sample = sample_from_candidate(meta, lab, candidate, best_index, enemy_slots)
                    handle.write(json.dumps(sample, ensure_ascii=False, separators=(",", ":")))
                    handle.write("\n")
    finally:
        if handle:
            handle.close()

    print("CombatTurnPlanGuidanceLabExtract")
    print(
        f"  cases={counters['cases']} candidates={total_candidates} "
        f"child_searches={total_child} child_complete_wins={total_wins}"
    )
    print(f"  target_diff_current_first={counters['target_diff_current_first']}")
    print(f"  buckets={dict(bucket_counts)}")
    print(f"  stop_reasons={dict(stop_counts)}")
    if out_jsonl:
        print(f"  jsonl={out_jsonl}")
    if summary_only:
        return
    print("  cases:")
    for case_id, current, best in case_rows[:case_limit]:
        print(f"    case={case_id}")
        print(f"      current: {plan_short(current)}")
        print(f"      target:  {plan_short(best)}")
    if len(case_rows) > case_limit:
        print(f"    ... {len(case_rows) - case_limit} more case(s)")


def int_or_zero(value: Any) -> int:
    return value if isinstance(value, int) else 0


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="+", type=Path)
    parser.add_argument("--out-jsonl", type=Path)
    parser.add_argument(
        "--summary-only",
        action="store_true",
        help="Print aggregate counts only; still writes --out-jsonl when requested.",
    )
    parser.add_argument(
        "--case-limit",
        type=int,
        default=20,
        help="Maximum per-case previews to print unless --summary-only is set.",
    )
    args = parser.parse_args()
    summarize(
        args.inputs,
        args.out_jsonl,
        summary_only=args.summary_only,
        case_limit=max(args.case_limit, 0),
    )


if __name__ == "__main__":
    main()
