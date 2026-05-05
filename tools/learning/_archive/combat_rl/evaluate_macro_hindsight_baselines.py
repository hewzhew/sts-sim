#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any

import numpy as np

from combat_reranker_common import iter_jsonl, write_json
from combat_rl_common import REPO_ROOT


def load_rows(path: Path) -> list[dict[str, Any]]:
    return [row for _, row in iter_jsonl(path)] if path.exists() else []


def safe_corr(xs: list[float], ys: list[float]) -> float:
    if not xs or not ys:
        return 0.0
    x = np.asarray(xs, dtype=np.float32)
    y = np.asarray(ys, dtype=np.float32)
    if np.std(x) == 0 or np.std(y) == 0:
        return 0.0
    return float(np.corrcoef(x, y)[0, 1])


def bucket_rate(rows: list[dict[str, Any]], positive_buckets: set[str]) -> float:
    if not rows:
        return 0.0
    hits = sum(1 for row in rows if str(row.get("deck_quality_bucket") or "") in positive_buckets)
    return float(hits / len(rows))


def reward_metrics(rows: list[dict[str, Any]]) -> dict[str, Any]:
    scores = [float(row.get("baseline_score") or 0.0) for row in rows]
    hindsight = [float(row.get("future_window_score") or 0.0) for row in rows]
    return {
        "rows": len(rows),
        "baseline_hindsight_corr": safe_corr(scores, hindsight),
        "future_window_positive_rate": bucket_rate(rows, {"positive", "survival_positive", "neutral_positive"}),
        "recommended_match_rate": float(sum(1 for row in rows if row.get("recommended_matches_choice")) / len(rows)) if rows else 0.0,
        "skip_choice_rate": float(sum(1 for row in rows if row.get("skip_chosen")) / len(rows)) if rows else 0.0,
    }


def event_metrics(rows: list[dict[str, Any]]) -> dict[str, Any]:
    scores = [float(row.get("baseline_score") or 0.0) for row in rows]
    hindsight = [float(row.get("future_window_score") or 0.0) for row in rows]
    family_counts = Counter(str((row.get("decision") or {}).get("family") or "unknown") for row in rows)
    return {
        "rows": len(rows),
        "baseline_hindsight_corr": safe_corr(scores, hindsight),
        "future_window_positive_rate": bucket_rate(rows, {"positive", "survival_positive", "neutral_positive"}),
        "family_counts": dict(family_counts),
    }


def shop_metrics(rows: list[dict[str, Any]]) -> dict[str, Any]:
    action_counts = Counter(str(row.get("baseline_choice_kind") or "unknown") for row in rows)
    avg_by_action = {}
    for action_kind in action_counts:
        subset = [row for row in rows if str(row.get("baseline_choice_kind") or "unknown") == action_kind]
        avg_by_action[action_kind] = float(np.mean([float(row.get("future_window_score") or 0.0) for row in subset])) if subset else 0.0
    buy_rows = [row for row in rows if str(row.get("baseline_choice_kind") or "").startswith("buy_")]
    leave_rows = [row for row in rows if str(row.get("baseline_choice_kind") or "") == "leave"]
    return {
        "rows": len(rows),
        "action_counts": dict(action_counts),
        "avg_future_window_score_by_action": avg_by_action,
        "buy_positive_rate": bucket_rate(buy_rows, {"positive", "survival_positive", "neutral_positive"}),
        "leave_positive_rate": bucket_rate(leave_rows, {"positive", "survival_positive", "neutral_positive"}),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Evaluate baseline macro hindsight datasets.")
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--metrics-out", default=None, type=Path)
    args = parser.parse_args()

    metrics_out = args.metrics_out or (args.dataset_dir / "macro_hindsight_metrics.json")
    reward_rows = load_rows(args.dataset_dir / "reward_hindsight_rows.jsonl")
    event_rows = load_rows(args.dataset_dir / "event_hindsight_rows.jsonl")
    shop_rows = load_rows(args.dataset_dir / "shop_hindsight_rows.jsonl")

    metrics = {
        "reward": reward_metrics(reward_rows),
        "event": event_metrics(event_rows),
        "shop": shop_metrics(shop_rows),
        "notes": [
            "macro baseline metrics are descriptive offline baselines over future-window hindsight rows",
            "they intentionally do not treat current bot choices as strong labels",
        ],
    }
    write_json(metrics_out, metrics)
    print(json.dumps(metrics, indent=2, ensure_ascii=False))
    print(f"wrote macro hindsight metrics to {metrics_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
