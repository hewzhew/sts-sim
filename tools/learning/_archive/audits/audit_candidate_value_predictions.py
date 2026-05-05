#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from statistics import mean
from typing import Any

from combat_rl_common import REPO_ROOT, write_json, write_jsonl

RETURN_KEY = "discounted_return"
DIAGNOSTIC_TARGETS = [
    "discounted_return",
    "hp_delta",
    "enemy_hp_delta",
    "final_player_hp",
    "final_enemy_hp",
    "final_visible_unblocked",
    "immediate_reward",
    "root_hp_delta",
    "root_enemy_hp_delta",
    "root_visible_unblocked",
    "terminal_defeat",
    "root_terminal_defeat",
]


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def target(row: dict[str, Any], name: str) -> float:
    if f"target::{name}" in row:
        return float(row.get(f"target::{name}") or 0.0)
    return float((row.get("targets") or {}).get(name) or 0.0)


def pred(row: dict[str, Any], name: str) -> float:
    return float(row.get(f"pred::{name}") or 0.0)


def candidate_label(row: dict[str, Any]) -> str:
    return str(row.get("candidate_label") or f"candidate#{int(row.get('candidate_index') or 0)}")


def class_name(row: dict[str, Any]) -> str:
    return str(row.get("candidate_class") or "unknown")


def classify_error(predicted: dict[str, Any], best: dict[str, Any], *, small_gap: bool, return_epsilon: float) -> list[str]:
    if small_gap:
        return ["small_gap"]
    tags: list[str] = []
    predicted_class = class_name(predicted)
    best_class = class_name(best)
    if predicted_class != best_class:
        tags.append(f"class::{predicted_class}->best::{best_class}")
    if predicted_class == "end_turn" and best_class != "end_turn":
        tags.append("end_turn_over_non_end_turn")
    if predicted_class == "damage" and best_class in {"mitigation", "end_turn"}:
        tags.append("damage_over_safety")
    if predicted_class in {"mitigation", "end_turn"} and best_class == "damage":
        tags.append("safety_over_damage")

    predicted_defeat = target(predicted, "terminal_defeat") > 0.5
    best_defeat = target(best, "terminal_defeat") > 0.5
    if predicted_defeat and not best_defeat:
        tags.append("missed_survival")
    predicted_root_defeat = target(predicted, "root_terminal_defeat") > 0.5
    best_root_defeat = target(best, "root_terminal_defeat") > 0.5
    if predicted_root_defeat and not best_root_defeat:
        tags.append("root_suicide_miss")

    enemy_delta = target(predicted, "root_enemy_hp_delta") - target(best, "root_enemy_hp_delta")
    hp_delta = target(predicted, "root_hp_delta") - target(best, "root_hp_delta")
    if enemy_delta > 3.0 and hp_delta < -3.0:
        tags.append("root_damage_for_hp_tradeoff")
    if enemy_delta < -3.0 and hp_delta > 3.0:
        tags.append("root_hp_for_damage_tradeoff")

    pred_gap = pred(best, RETURN_KEY) - pred(predicted, RETURN_KEY)
    if pred_gap > return_epsilon:
        tags.append("model_underestimated_true_best")
    if not tags:
        tags.append("unclassified_large_gap")
    return tags


def summarize_run(
    *,
    rows_by_sample: dict[int, dict[str, Any]],
    prediction_rows: list[dict[str, Any]],
    split: str,
    return_epsilon: float,
    large_gap: float,
    top_examples: int,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    joined: list[dict[str, Any]] = []
    missing_rows = 0
    for prediction in prediction_rows:
        if split != "all" and str(prediction.get("split") or "") != split:
            continue
        sample_index = int(prediction.get("sample_index") or 0)
        source = rows_by_sample.get(sample_index)
        if source is None:
            missing_rows += 1
            continue
        row = dict(source)
        row.update(prediction)
        joined.append(row)

    groups: dict[int, list[dict[str, Any]]] = defaultdict(list)
    for row in joined:
        groups[int(row.get("group_index") or 0)].append(row)

    group_summaries: list[dict[str, Any]] = []
    confusion = Counter()
    error_tags = Counter()
    mistake_label_pairs = Counter()
    chosen_class_counts = Counter()
    best_class_counts = Counter()
    regression_deltas: dict[str, list[float]] = defaultdict(list)
    prediction_deltas: dict[str, list[float]] = defaultdict(list)
    top1_hits = 0
    within_eps_hits = 0
    large_gap_groups = 0
    large_gap_hits = 0
    large_gap_within_eps = 0
    regrets: list[float] = []
    top2_gaps: list[float] = []

    for group_id, group_rows in sorted(groups.items()):
        if not group_rows:
            continue
        true_sorted = sorted(group_rows, key=lambda row: target(row, RETURN_KEY), reverse=True)
        pred_sorted = sorted(group_rows, key=lambda row: pred(row, RETURN_KEY), reverse=True)
        best = true_sorted[0]
        chosen = pred_sorted[0]
        true_best = target(best, RETURN_KEY)
        chosen_true = target(chosen, RETURN_KEY)
        regret = true_best - chosen_true
        top2_gap = true_best - target(true_sorted[1], RETURN_KEY) if len(true_sorted) > 1 else 0.0
        is_hit = bool(chosen.get("candidate_is_best")) or regret <= 1e-9
        is_within_eps = regret <= return_epsilon
        is_large_gap = top2_gap >= large_gap

        top1_hits += 1 if is_hit else 0
        within_eps_hits += 1 if is_within_eps else 0
        if is_large_gap:
            large_gap_groups += 1
            large_gap_hits += 1 if is_hit else 0
            large_gap_within_eps += 1 if is_within_eps else 0
        regrets.append(float(regret))
        top2_gaps.append(float(top2_gap))
        chosen_class = class_name(chosen)
        best_class = class_name(best)
        chosen_class_counts[chosen_class] += 1
        best_class_counts[best_class] += 1
        if not is_hit:
            confusion[f"{chosen_class}->{best_class}"] += 1
            mistake_label_pairs[f"{candidate_label(chosen)} -> {candidate_label(best)}"] += 1
            for tag in classify_error(chosen, best, small_gap=top2_gap <= return_epsilon, return_epsilon=return_epsilon):
                error_tags[tag] += 1
            for name in DIAGNOSTIC_TARGETS:
                regression_deltas[name].append(target(chosen, name) - target(best, name))
                prediction_deltas[name].append(pred(chosen, name) - pred(best, name))

        group_summaries.append(
            {
                "group_index": group_id,
                "split": split,
                "candidate_count": len(group_rows),
                "hit": is_hit,
                "within_epsilon": is_within_eps,
                "large_gap": is_large_gap,
                "regret": float(regret),
                "top2_gap": float(top2_gap),
                "predicted": {
                    "sample_index": int(chosen.get("sample_index") or 0),
                    "candidate_index": int(chosen.get("candidate_index") or 0),
                    "label": candidate_label(chosen),
                    "class": chosen_class,
                    "target_return": target(chosen, RETURN_KEY),
                    "pred_return": pred(chosen, RETURN_KEY),
                    "target_hp_delta": target(chosen, "hp_delta"),
                    "target_enemy_hp_delta": target(chosen, "enemy_hp_delta"),
                    "target_terminal_defeat": target(chosen, "terminal_defeat"),
                },
                "best": {
                    "sample_index": int(best.get("sample_index") or 0),
                    "candidate_index": int(best.get("candidate_index") or 0),
                    "label": candidate_label(best),
                    "class": best_class,
                    "target_return": target(best, RETURN_KEY),
                    "pred_return": pred(best, RETURN_KEY),
                    "target_hp_delta": target(best, "hp_delta"),
                    "target_enemy_hp_delta": target(best, "enemy_hp_delta"),
                    "target_terminal_defeat": target(best, "terminal_defeat"),
                },
            }
        )

    group_count = len(group_summaries)
    mistakes = [row for row in group_summaries if not row["hit"]]
    mistakes.sort(key=lambda row: (float(row["regret"]), float(row["top2_gap"])), reverse=True)
    small_gap_count = sum(1 for value in top2_gaps if value <= return_epsilon)
    summary = {
        "split": split,
        "prediction_rows": len(prediction_rows),
        "joined_rows": len(joined),
        "missing_source_rows": missing_rows,
        "groups": group_count,
        "top1_group_match": float(top1_hits / group_count) if group_count else 0.0,
        "top1_within_epsilon": float(within_eps_hits / group_count) if group_count else 0.0,
        "mean_regret": float(mean(regrets)) if regrets else 0.0,
        "median_regret": float(sorted(regrets)[len(regrets) // 2]) if regrets else 0.0,
        "mean_top2_gap": float(mean(top2_gaps)) if top2_gaps else 0.0,
        "median_top2_gap": float(sorted(top2_gaps)[len(top2_gaps) // 2]) if top2_gaps else 0.0,
        "small_gap_groups": int(small_gap_count),
        "large_gap_threshold": float(large_gap),
        "large_gap_groups": int(large_gap_groups),
        "large_gap_top1_match": float(large_gap_hits / large_gap_groups) if large_gap_groups else 0.0,
        "large_gap_within_epsilon": float(large_gap_within_eps / large_gap_groups) if large_gap_groups else 0.0,
        "chosen_class_counts": dict(chosen_class_counts),
        "best_class_counts": dict(best_class_counts),
        "mistake_count": len(mistakes),
        "mistake_confusion": dict(confusion.most_common()),
        "mistake_error_tags": dict(error_tags.most_common()),
        "mistake_label_pairs_top": dict(mistake_label_pairs.most_common(20)),
        "mistake_true_delta_mean": {
            key: float(mean(values)) for key, values in sorted(regression_deltas.items()) if values
        },
        "mistake_pred_delta_mean": {
            key: float(mean(values)) for key, values in sorted(prediction_deltas.items()) if values
        },
        "worst_mistakes": mistakes[:top_examples],
    }
    return summary, group_summaries


def main() -> None:
    parser = argparse.ArgumentParser(description="Audit structured candidate value prediction failures.")
    parser.add_argument("--rows", required=True, type=Path)
    parser.add_argument("--predictions", action="append", required=True, type=Path)
    parser.add_argument("--split", default="val", help="Prediction split to audit, or 'all'.")
    parser.add_argument("--return-epsilon", default=0.05, type=float)
    parser.add_argument("--large-gap", default=0.10, type=float)
    parser.add_argument("--top-examples", default=12, type=int)
    parser.add_argument("--out", default=None, type=Path)
    parser.add_argument("--groups-out", default=None, type=Path)
    args = parser.parse_args()

    source_rows = load_jsonl(args.rows)
    rows_by_sample = {int(row.get("sample_index") or 0): row for row in source_rows}
    run_summaries: dict[str, Any] = {}
    all_group_rows: list[dict[str, Any]] = []
    for predictions_path in args.predictions:
        prediction_rows = load_jsonl(predictions_path)
        summary, group_rows = summarize_run(
            rows_by_sample=rows_by_sample,
            prediction_rows=prediction_rows,
            split=str(args.split),
            return_epsilon=float(args.return_epsilon),
            large_gap=float(args.large_gap),
            top_examples=int(args.top_examples),
        )
        key = predictions_path.stem
        summary["predictions"] = str(predictions_path)
        run_summaries[key] = summary
        for row in group_rows:
            row["run"] = key
            all_group_rows.append(row)

    output = {
        "rows": str(args.rows),
        "split": args.split,
        "return_epsilon": float(args.return_epsilon),
        "large_gap": float(args.large_gap),
        "runs": run_summaries,
    }
    out = args.out or REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "candidate_value_prediction_audit.json"
    groups_out = args.groups_out or out.with_suffix(".groups.jsonl")
    write_json(out, output)
    write_jsonl(groups_out, all_group_rows)
    print(json.dumps(output, ensure_ascii=False, indent=2), flush=True)
    print(f"wrote candidate value audit to {out}", flush=True)


if __name__ == "__main__":
    main()
