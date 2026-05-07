#!/usr/bin/env python3
"""Build a narrow dataset from draw marginal query-specific labels.

This is a training-prep consumer, not a model trainer. It separates clean
query-local preferences from tradeoff-heavy rows and audits whether the labels
are balanced enough to train anything more than an axis/reason head.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from combat_reranker_common import iter_jsonl, write_json, write_jsonl

REPO_ROOT = Path(__file__).resolve().parents[2]
REPORT_VERSION = "draw_query_axis_dataset_v0_2"
TRACE_STEP_RE = re.compile(r"^(?P<trace_step>episode_\d+_seed_\d+_step_\d+)")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build draw query-axis dataset prep artifacts.")
    parser.add_argument(
        "--input-dir",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "draw_marginal_value" / "v0",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=REPO_ROOT / "tools" / "artifacts" / "draw_query_axis_dataset" / "v0",
    )
    return parser.parse_args()


def resolve(path: Path) -> Path:
    return path if path.is_absolute() else REPO_ROOT / path


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    return [row for _, row in iter_jsonl(path)]


def split_group(row: dict[str, Any]) -> tuple[str, str]:
    """Return the leakage-safe split group for a query-axis label row.

    Action-key mining intentionally creates several target-action cases from the
    same combat state. Splitting by source_case_id would allow near-duplicate
    rows from the same trace step into train and heldout sets. Grouping by the
    trace step keeps all actions/queries from one combat state together.
    """
    trace_file = str(row.get("trace_file") or "")
    step_index = row.get("step_index")
    if trace_file and step_index is not None:
        return f"{trace_file}::step_{step_index}", "trace_file_step"

    source_case_id = str(row.get("source_case_id") or "")
    match = TRACE_STEP_RE.match(source_case_id)
    if match:
        return match.group("trace_step"), "trace_step_from_source_case_id"

    label_id = str(row.get("label_id") or "")
    match = TRACE_STEP_RE.match(label_id)
    if match:
        return match.group("trace_step"), "trace_step_from_label_id"

    if source_case_id:
        return source_case_id, "source_case_id"
    if label_id:
        return label_id, "label_id"
    return "unknown", "unknown"


def stable_group_split(group_key: str) -> str:
    digest = hashlib.sha256(group_key.encode("utf-8")).digest()
    bucket = int.from_bytes(digest[:8], "big") % 20
    if bucket < 14:
        return "train"
    if bucket < 17:
        return "val"
    return "test"


def row_split(row: dict[str, Any]) -> str:
    group_key, _ = split_group(row)
    return stable_group_split(group_key)


def normalize_label(row: dict[str, Any]) -> str:
    label = str(row.get("label") or "")
    if label == "equivalent":
        return "equivalent"
    if label.startswith("target_action_better_for_"):
        return "target_better"
    if label.startswith("target_action_worse_for_"):
        return "target_worse"
    return "unknown"


def compact_row(row: dict[str, Any], bucket: str) -> dict[str, Any]:
    split_group_key, split_group_key_kind = split_group(row)
    return {
        "label_id": row.get("label_id"),
        "source_case_id": row.get("source_case_id"),
        "split": stable_group_split(split_group_key),
        "split_group_key": split_group_key,
        "split_group_key_kind": split_group_key_kind,
        "bucket": bucket,
        "target_action_card": row.get("target_action_card"),
        "query_name": row.get("query_name"),
        "query_axis": row.get("query_axis"),
        "label_mode": row.get("label_mode"),
        "label": row.get("label"),
        "normalized_label": normalize_label(row),
        "preferred_branch": row.get("preferred_branch"),
        "reason": row.get("reason"),
        "hard_preference_allowed": bool(row.get("hard_preference_allowed")),
        "global_preference_allowed": bool(row.get("global_preference_allowed")),
        "status_delta": row.get("status_delta"),
        "damage_delta": row.get("damage_delta"),
        "block_delta": row.get("block_delta"),
        "unblocked_reduction": row.get("unblocked_reduction"),
        "hp_loss_reduction": row.get("hp_loss_reduction"),
        "setup_gain": bool(row.get("setup_gain")),
        "setup_loss": bool(row.get("setup_loss")),
        "forced_cards": row.get("forced_cards") or [],
        "no_draw_cards": row.get("no_draw_cards") or [],
        "notes": row.get("notes") or [],
        "training_use": (
            "query_axis_clean_preference"
            if bucket == "clean_hard_preferences"
            else "query_axis_tradeoff_or_calibration"
        ),
        "limitations": row.get("limitations") or [],
    }


def partition(rows: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    buckets: dict[str, list[dict[str, Any]]] = {
        "clean_hard_preferences": [],
        "tradeoff_hard_preferences": [],
        "equivalent_or_calibration": [],
        "rejected": [],
    }
    for row in rows:
        if row.get("hard_preference_allowed") and not row.get("notes"):
            buckets["clean_hard_preferences"].append(compact_row(row, "clean_hard_preferences"))
        elif row.get("hard_preference_allowed"):
            buckets["tradeoff_hard_preferences"].append(compact_row(row, "tradeoff_hard_preferences"))
        elif normalize_label(row) == "equivalent":
            buckets["equivalent_or_calibration"].append(compact_row(row, "equivalent_or_calibration"))
        else:
            buckets["rejected"].append(compact_row(row, "rejected"))
    return buckets


def binary_examples(buckets: dict[str, list[dict[str, Any]]]) -> list[dict[str, Any]]:
    rows = []
    for row in buckets["clean_hard_preferences"]:
        target_improves = row.get("normalized_label") == "target_better"
        rows.append(
            {
                **row,
                "binary_label_mode": "query_axis_improvement_v0",
                "target_improves_query": target_improves,
                "binary_training_use": (
                    "positive_clean_query_improvement"
                    if target_improves
                    else "negative_clean_query_degradation"
                ),
            }
        )
    for row in buckets["equivalent_or_calibration"]:
        rows.append(
            {
                **row,
                "binary_label_mode": "query_axis_improvement_v0",
                "target_improves_query": False,
                "binary_training_use": "negative_equivalent_or_calibration",
            }
        )
    return rows


def group_counts(rows: list[dict[str, Any]], key: str) -> dict[str, int]:
    return dict(sorted(Counter(str(row.get(key) or "") for row in rows).items()))


def split_counts(rows: list[dict[str, Any]]) -> dict[str, int]:
    return group_counts(rows, "split")


def split_group_counts(rows: list[dict[str, Any]]) -> dict[str, int]:
    by_split: dict[str, set[str]] = defaultdict(set)
    for row in rows:
        by_split[str(row.get("split") or "")].add(str(row.get("split_group_key") or ""))
    return dict(sorted((split, len(groups)) for split, groups in by_split.items()))


def split_group_diagnostics(rows: list[dict[str, Any]]) -> dict[str, Any]:
    groups: dict[str, set[str]] = defaultdict(set)
    for row in rows:
        groups[str(row.get("split_group_key") or "")].add(str(row.get("split") or ""))
    leakage = {key: sorted(splits) for key, splits in groups.items() if len(splits) > 1}
    examples = [
        {"split_group_key": key, "splits": splits}
        for key, splits in sorted(leakage.items())[:10]
    ]
    return {
        "unique_group_count": len(groups),
        "group_counts_by_split": split_group_counts(rows),
        "group_key_kind_counts": group_counts(rows, "split_group_key_kind"),
        "potential_leakage_group_count": len(leakage),
        "potential_leakage_examples": examples,
    }


def summarize_bucket(rows: list[dict[str, Any]]) -> dict[str, Any]:
    return {
        "count": len(rows),
        "split_counts": split_counts(rows),
        "split_group_counts": split_group_counts(rows),
        "label_counts": group_counts(rows, "normalized_label"),
        "query_counts": group_counts(rows, "query_name"),
        "card_counts": group_counts(rows, "target_action_card"),
    }


def majority_predictor_accuracy(rows: list[dict[str, Any]], feature_keys: tuple[str, ...]) -> dict[str, Any]:
    train = [row for row in rows if row.get("split") == "train"]
    test = [row for row in rows if row.get("split") in {"val", "test"}]
    global_majority = Counter(str(row.get("normalized_label")) for row in train).most_common(1)
    default_label = global_majority[0][0] if global_majority else "equivalent"
    by_key: dict[tuple[str, ...], Counter[str]] = defaultdict(Counter)
    for row in train:
        key = tuple(str(row.get(part) or "") for part in feature_keys)
        by_key[key][str(row.get("normalized_label"))] += 1
    correct = 0
    predictions = []
    for row in test:
        key = tuple(str(row.get(part) or "") for part in feature_keys)
        label = by_key[key].most_common(1)[0][0] if key in by_key else default_label
        truth = str(row.get("normalized_label"))
        correct += int(label == truth)
        predictions.append(
            {
                "label_id": row.get("label_id"),
                "key": list(key),
                "truth": truth,
                "prediction": label,
                "correct": label == truth,
            }
        )
    return {
        "feature_keys": list(feature_keys),
        "train_count": len(train),
        "test_count": len(test),
        "accuracy": round(correct / len(test), 4) if test else None,
        "default_label": default_label,
        "sample_predictions": predictions[:5],
    }


def binary_majority_accuracy(rows: list[dict[str, Any]], feature_keys: tuple[str, ...]) -> dict[str, Any]:
    normalized = [
        {
            **row,
            "normalized_label": "improves_query" if row.get("target_improves_query") else "does_not_improve_query",
        }
        for row in rows
    ]
    return majority_predictor_accuracy(normalized, feature_keys)


def build_report(input_path: Path, buckets: dict[str, list[dict[str, Any]]]) -> dict[str, Any]:
    all_rows = [row for rows in buckets.values() for row in rows]
    clean = buckets["clean_hard_preferences"]
    hard_pref = [*buckets["clean_hard_preferences"], *buckets["tradeoff_hard_preferences"]]
    binary_rows = binary_examples(buckets)
    label_balance_ok = bool(
        group_counts(hard_pref, "normalized_label").get("target_better", 0)
        and group_counts(hard_pref, "normalized_label").get("target_worse", 0)
    )
    return {
        "report_version": REPORT_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "config": {"input": str(input_path)},
        "summary": {
            "total_rows": len(all_rows),
            "bucket_counts": {name: len(rows) for name, rows in buckets.items()},
            "all_rows": summarize_bucket(all_rows),
            "clean_hard_preferences": summarize_bucket(clean),
            "hard_preference_label_balance_ok": label_balance_ok,
            "split_grouping": split_group_diagnostics(all_rows),
            "binary_query_improvement": {
                "count": len(binary_rows),
                "positive_count": sum(1 for row in binary_rows if row.get("target_improves_query")),
                "negative_count": sum(1 for row in binary_rows if not row.get("target_improves_query")),
                "split_counts": split_counts(binary_rows),
                "split_group_counts": split_group_counts(binary_rows),
                "split_grouping": split_group_diagnostics(binary_rows),
                "query_counts": group_counts(binary_rows, "query_name"),
                "card_counts": group_counts(binary_rows, "target_action_card"),
                "majority_baselines": [
                    binary_majority_accuracy(binary_rows, ("query_name",)),
                    binary_majority_accuracy(binary_rows, ("target_action_card",)),
                    binary_majority_accuracy(binary_rows, ("target_action_card", "query_name")),
                ],
            },
            "majority_baselines": [
                majority_predictor_accuracy(all_rows, ("query_name",)),
                majority_predictor_accuracy(all_rows, ("target_action_card",)),
                majority_predictor_accuracy(all_rows, ("target_action_card", "query_name")),
            ],
        },
        "warnings": [
            "clean hard preferences are query-local, not global action preferences",
            "current batch may have source distribution bias; check synthetic-template or trace-occupancy provenance",
            "splits are grouped by trace step when trace-derived ids are available",
            "do not train a scalar action-good classifier unless target_worse negatives exist",
        ],
    }


def markdown(report: dict[str, Any]) -> str:
    lines = [
        "# Draw Query-Axis Dataset Prep",
        "",
        f"Generated: `{report['generated_at_utc']}`",
        "",
        "This prepares query-local labels from draw marginal probes. It does not create global action-good labels.",
        "",
        "## Summary",
        "",
        f"- bucket counts: `{report['summary']['bucket_counts']}`",
        f"- clean hard preferences: `{report['summary']['clean_hard_preferences']['count']}`",
        f"- hard preference label balance ok: `{report['summary']['hard_preference_label_balance_ok']}`",
        f"- binary query-improvement rows: `{report['summary']['binary_query_improvement']['count']}`",
        f"- binary positives/negatives: `{report['summary']['binary_query_improvement']['positive_count']}` / "
        f"`{report['summary']['binary_query_improvement']['negative_count']}`",
        f"- split groups: `{report['summary']['split_grouping']['unique_group_count']}`",
        f"- split leakage groups: `{report['summary']['split_grouping']['potential_leakage_group_count']}`",
        "",
        "## Clean Hard Preferences",
        "",
        f"- labels: `{report['summary']['clean_hard_preferences']['label_counts']}`",
        f"- queries: `{report['summary']['clean_hard_preferences']['query_counts']}`",
        f"- cards: `{report['summary']['clean_hard_preferences']['card_counts']}`",
        "",
        "## Split Hygiene",
        "",
        "Rows are split by combat-state group when trace-derived ids are available, not by individual action-key case.",
        "",
        f"- all row groups by split: `{report['summary']['split_grouping']['group_counts_by_split']}`",
        f"- all row group key kinds: `{report['summary']['split_grouping']['group_key_kind_counts']}`",
        f"- binary row groups by split: "
        f"`{report['summary']['binary_query_improvement']['split_grouping']['group_counts_by_split']}`",
        f"- potential leakage groups: "
        f"`{report['summary']['split_grouping']['potential_leakage_group_count']}`",
        "",
        "## Binary Query-Improvement Baselines",
        "",
        "This uses clean hard preferences as positives and equivalent/calibration rows as negatives.",
        "",
        "| features | train | heldout | accuracy | default |",
        "| --- | ---: | ---: | ---: | --- |",
    ]
    for row in report["summary"]["binary_query_improvement"]["majority_baselines"]:
        lines.append(
            f"| `{row['feature_keys']}` | {row['train_count']} | {row['test_count']} | "
            f"{row['accuracy']} | `{row['default_label']}` |"
        )
    lines.extend(
        [
            "",
            "## Raw Query-Label Baselines",
            "",
            "These include tradeoff rows and are mainly a bias check.",
            "",
            "| features | train | heldout | accuracy | default |",
            "| --- | ---: | ---: | ---: | --- |",
        ]
    )
    for row in report["summary"]["majority_baselines"]:
        lines.append(
            f"| `{row['feature_keys']}` | {row['train_count']} | {row['test_count']} | "
            f"{row['accuracy']} | `{row['default_label']}` |"
        )
    lines.extend(["", "## Warnings", ""])
    for warning in report["warnings"]:
        lines.append(f"- {warning}")
    return "\n".join(lines) + "\n"


def main() -> None:
    args = parse_args()
    input_dir = resolve(args.input_dir)
    out_dir = resolve(args.out_dir)
    input_path = input_dir / "query_specific_labels.jsonl"
    rows = load_jsonl(input_path)
    buckets = partition(rows)
    out_dir.mkdir(parents=True, exist_ok=True)
    for name, bucket_rows in buckets.items():
        write_jsonl(out_dir / f"{name}.jsonl", bucket_rows)
    write_jsonl(out_dir / "query_axis_binary_examples.jsonl", binary_examples(buckets))
    report = build_report(input_path, buckets)
    write_json(out_dir / "draw_query_axis_dataset_report.json", report)
    (out_dir / "draw_query_axis_dataset_report.md").write_text(markdown(report), encoding="utf-8")
    print(
        json.dumps(
            {
                "bucket_counts": report["summary"]["bucket_counts"],
                "clean_hard_preferences": report["summary"]["clean_hard_preferences"],
                "binary_query_improvement": {
                    key: value
                    for key, value in report["summary"]["binary_query_improvement"].items()
                    if key != "majority_baselines"
                },
                "binary_majority_baselines": report["summary"]["binary_query_improvement"]["majority_baselines"],
                "out_dir": str(out_dir),
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
