#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from collections import Counter
from pathlib import Path
from typing import Any

from combat_reranker_common import iter_jsonl, write_json, write_jsonl
from combat_rl_common import REPO_ROOT, read_json

CHOOSE_INDEX_RE = re.compile(r"CHOOSE\s+(?P<index>\d+)")


def read_jsonl_rows(path: Path) -> list[dict[str, Any]]:
    return [row for _, row in iter_jsonl(path)] if path.exists() else []


def room_summary(record: dict[str, Any]) -> dict[str, Any] | None:
    game_state = record.get("game_state")
    if not isinstance(game_state, dict):
        return None
    protocol_meta = record.get("protocol_meta") or {}
    response_id = protocol_meta.get("response_id")
    deck = game_state.get("deck") or []
    return {
        "response_id": response_id,
        "floor": int(game_state.get("floor") or 0),
        "act": int(game_state.get("act") or 0),
        "current_hp": int(game_state.get("current_hp") or 0),
        "gold": int(game_state.get("gold") or 0),
        "deck_size": len(deck),
        "room_type": str(game_state.get("room_type") or ""),
        "screen_type": str(game_state.get("screen_type") or ""),
        "choice_list": list(game_state.get("choice_list") or []),
    }


def future_window(summary_rows: list[dict[str, Any]], current_response_id: int | None, current_floor: int, current_act: int, horizon_floors: int) -> dict[str, Any]:
    if current_response_id is None:
        return {
            "future_window_floors": horizon_floors,
            "future_hp_delta": 0,
            "future_gold_delta": 0,
            "future_elite_count": 0,
            "survival_to_boss": False,
            "deck_growth": 0,
            "deck_quality_bucket": "unknown",
            "future_window_score": 0.0,
        }
    future_candidates = [
        row
        for row in summary_rows
        if int(row.get("response_id") or 0) > current_response_id
        and int(row.get("act") or 0) == current_act
        and int(row.get("floor") or 0) <= current_floor + horizon_floors
    ]
    base = next((row for row in summary_rows if row.get("response_id") == current_response_id), None)
    target = future_candidates[-1] if future_candidates else summary_rows[-1]
    if base is None:
        base = target
    distinct_elite_floors = {
        int(row.get("floor") or 0)
        for row in future_candidates
        if "Elite" in str(row.get("room_type") or "")
    }
    future_hp_delta = int(target.get("current_hp") or 0) - int(base.get("current_hp") or 0)
    future_gold_delta = int(target.get("gold") or 0) - int(base.get("gold") or 0)
    deck_growth = int(target.get("deck_size") or 0) - int(base.get("deck_size") or 0)
    act_boss_floor = {1: 16, 2: 33, 3: 50}.get(current_act, current_floor + horizon_floors + 1)
    survival_to_boss = int(target.get("floor") or 0) >= act_boss_floor
    future_window_score = float(future_hp_delta) + float(future_gold_delta) * 0.1 + len(distinct_elite_floors) * 5.0 + (15.0 if survival_to_boss else 0.0) + deck_growth * 0.5
    if survival_to_boss and future_hp_delta >= 0:
        bucket = "positive"
    elif survival_to_boss:
        bucket = "survival_positive"
    elif future_window_score >= 5:
        bucket = "neutral_positive"
    elif future_window_score <= -10:
        bucket = "negative"
    else:
        bucket = "neutral"
    return {
        "future_window_floors": horizon_floors,
        "future_hp_delta": future_hp_delta,
        "future_gold_delta": future_gold_delta,
        "future_elite_count": len(distinct_elite_floors),
        "survival_to_boss": survival_to_boss,
        "deck_growth": deck_growth,
        "deck_quality_bucket": bucket,
        "future_window_score": round(future_window_score, 4),
    }


def enrich_reward_rows(reward_rows: list[dict[str, Any]], summaries_by_run: dict[str, list[dict[str, Any]]], horizon_floors: int) -> list[dict[str, Any]]:
    enriched = []
    for row in reward_rows:
        run_rows = summaries_by_run.get(str(row.get("run_id")), [])
        future = future_window(run_rows, row.get("response_id"), int(row.get("floor") or 0), int(row.get("act") or 0), horizon_floors)
        recommended = row.get("recommended_choice") or {}
        candidates = row.get("candidates") or []
        recommended_index = recommended.get("choice_index")
        recommended_candidate = candidates[recommended_index] if isinstance(recommended_index, int) and 0 <= recommended_index < len(candidates) else None
        enriched.append(
            {
                **row,
                "dataset_kind": "reward_hindsight",
                **future,
                "baseline_score": float((recommended_candidate or {}).get("combined_score") or 0.0),
                "baseline_choice_kind": (row.get("label") or {}).get("kind"),
                "recommended_matches_choice": bool((row.get("label") or {}).get("choice_index") == recommended_index),
            }
        )
    return enriched


def enrich_event_rows(event_rows: list[dict[str, Any]], summaries_by_run: dict[str, list[dict[str, Any]]], horizon_floors: int) -> list[dict[str, Any]]:
    enriched = []
    for row in event_rows:
        run_rows = summaries_by_run.get(str(row.get("run_id")), [])
        frame = row.get("frame")
        response_id = row.get("frame") if row.get("response_id") is None else row.get("response_id")
        current_summary = next((summary for summary in run_rows if summary.get("response_id") == response_id), None)
        current_floor = int((current_summary or {}).get("floor") or 0)
        current_act = int((current_summary or {}).get("act") or 1)
        future = future_window(run_rows, response_id, current_floor, current_act, horizon_floors)
        enriched.append(
            {
                **row,
                "dataset_kind": "event_hindsight",
                "baseline_score": float((row.get("decision") or {}).get("score") or 0.0),
                "baseline_choice_kind": "event_option",
                "frame": frame,
                "floor": current_floor,
                "act": current_act,
                **future,
            }
        )
    return enriched


def chosen_shop_action(prev_record: dict[str, Any], curr_record: dict[str, Any]) -> tuple[str, str | None]:
    protocol_meta = curr_record.get("protocol_meta") or {}
    last_command = str(protocol_meta.get("last_command") or "")
    choice_list = list((prev_record.get("game_state") or {}).get("choice_list") or [])
    if last_command.startswith("LEAVE"):
        return ("leave", None)
    match = CHOOSE_INDEX_RE.match(last_command)
    if not match:
        return ("unknown", None)
    index = int(match.group("index"))
    label = choice_list[index] if 0 <= index < len(choice_list) else None
    if label == "purge":
        return ("purge", label)
    screen_state = (prev_record.get("game_state") or {}).get("screen_state") or {}
    for key, kind in (("cards", "buy_card"), ("relics", "buy_relic"), ("potions", "buy_potion")):
        for item in screen_state.get(key) or []:
            name = str(item.get("name") or "").lower()
            item_id = str(item.get("id") or "").lower()
            if label and (label.lower() == name or label.lower() == item_id):
                return (kind, label)
    return ("choose", label)


def build_shop_rows(raw_records: list[dict[str, Any]], run_id: str, horizon_floors: int, summaries: list[dict[str, Any]]) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for prev, curr in zip(raw_records, raw_records[1:], strict=False):
        prev_state = prev.get("game_state") or {}
        if str(prev_state.get("screen_type") or "") != "SHOP_SCREEN":
            continue
        action_kind, chosen_label = chosen_shop_action(prev, curr)
        if action_kind == "unknown":
            continue
        summary = room_summary(prev) or {}
        future = future_window(summaries, summary.get("response_id"), summary.get("floor", 0), summary.get("act", 0), horizon_floors)
        screen_state = prev_state.get("screen_state") or {}
        rows.append(
            {
                "dataset_kind": "shop_hindsight",
                "run_id": run_id,
                "response_id": summary.get("response_id"),
                "frame": summary.get("response_id"),
                "floor": summary.get("floor"),
                "act": summary.get("act"),
                "current_hp": summary.get("current_hp"),
                "gold": summary.get("gold"),
                "deck_size": summary.get("deck_size"),
                "cards": screen_state.get("cards") or [],
                "relics": screen_state.get("relics") or [],
                "potions": screen_state.get("potions") or [],
                "purge_cost": screen_state.get("purge_cost"),
                "purge_available": screen_state.get("purge_available"),
                "baseline_choice_kind": action_kind,
                "baseline_choice_label": chosen_label,
                "baseline_score": float(summary.get("gold") or 0),
                **future,
            }
        )
    return rows


def main() -> int:
    parser = argparse.ArgumentParser(description="Build reward/shop/event hindsight rows from archived clean runs.")
    parser.add_argument("--baseline", default=REPO_ROOT / "tools" / "artifacts" / "learning_baseline.json", type=Path)
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--horizon-floors", default=3, type=int)
    args = parser.parse_args()

    baseline = read_json(args.baseline)
    reward_rows = read_jsonl_rows(args.dataset_dir / "reward_rows.jsonl")
    event_rows = read_jsonl_rows(args.dataset_dir / "event_rows.jsonl")

    summaries_by_run: dict[str, list[dict[str, Any]]] = {}
    raw_records_by_run: dict[str, list[dict[str, Any]]] = {}
    for run in baseline.get("selected_runs") or []:
        run_id = str(run.get("run_id"))
        raw_path = Path(str(run.get("raw_path")))
        raw_records = [record for _, record in iter_jsonl(raw_path)]
        raw_records_by_run[run_id] = raw_records
        summaries = [summary for record in raw_records if (summary := room_summary(record)) is not None]
        summaries_by_run[run_id] = summaries

    reward_hindsight = enrich_reward_rows(reward_rows, summaries_by_run, args.horizon_floors)
    event_hindsight = enrich_event_rows(event_rows, summaries_by_run, args.horizon_floors)
    shop_hindsight: list[dict[str, Any]] = []
    for run_id, raw_records in raw_records_by_run.items():
        shop_hindsight.extend(build_shop_rows(raw_records, run_id, args.horizon_floors, summaries_by_run[run_id]))

    write_jsonl(args.dataset_dir / "reward_hindsight_rows.jsonl", reward_hindsight)
    write_jsonl(args.dataset_dir / "event_hindsight_rows.jsonl", event_hindsight)
    write_jsonl(args.dataset_dir / "shop_hindsight_rows.jsonl", shop_hindsight)

    summary = {
        "baseline": str(args.baseline),
        "horizon_floors": args.horizon_floors,
        "row_counts": {
            "reward_hindsight": len(reward_hindsight),
            "event_hindsight": len(event_hindsight),
            "shop_hindsight": len(shop_hindsight),
        },
        "shop_action_counts": dict(Counter(str(row.get("baseline_choice_kind") or "unknown") for row in shop_hindsight)),
        "notes": [
            "macro hindsight rows are future-window enriched archived clean-run rows",
            "reward/event rows derive from existing audit datasets; shop rows derive from raw shop screen transitions",
            "these rows are for offline value/preference analysis only, not runtime learning",
        ],
    }
    write_json(args.dataset_dir / "macro_hindsight_summary.json", summary)

    print(json.dumps(summary, indent=2, ensure_ascii=False))
    print(f"wrote macro hindsight rows to {args.dataset_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
