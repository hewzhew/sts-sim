#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json


DEFAULT_LABEL_DIR = (
    REPO_ROOT / "tools" / "artifacts" / "card_cashout_rollout_labels" / "v1_1_attribution_100case"
)

PROGRESS_REASONS = {"terminal_class", "floor_delta", "combat_win_delta"}
HP_PRESERVATION_REASONS = {"hp_margin"}
POLICY_SENSITIVE_LABELS = {"requires_cashout_policy", "rollout_unstable"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Select representative cashout rollout drilldown cases. This is a review queue, "
            "not a training dataset."
        )
    )
    parser.add_argument("--label-dir", type=Path, default=DEFAULT_LABEL_DIR)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--pairwise-labels", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--markdown-out", type=Path)
    parser.add_argument("--per-bucket", type=int, default=8)
    parser.add_argument("--allow-duplicate-cases", action="store_true")
    return parser.parse_args()


def resolve(path: Path) -> Path:
    return path if path.is_absolute() else REPO_ROOT / path


def read_json(path: Path) -> dict[str, Any]:
    with resolve(path).open("r", encoding="utf-8") as handle:
        return json.load(handle)


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    real = resolve(path)
    if not real.exists():
        return []
    rows = []
    with real.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def num(value: Any) -> float:
    try:
        return float(value or 0.0)
    except (TypeError, ValueError):
        return 0.0


def card_label(outcome: dict[str, Any]) -> str:
    return str(outcome.get("card_id") or outcome.get("candidate_key") or "unknown")


def reason_win_mode(reason: str) -> str:
    if reason in PROGRESS_REASONS:
        return "progress_win"
    if reason in HP_PRESERVATION_REASONS:
        return "hp_preservation_win"
    return "other_win"


def pair_shape(preferred: str, rejected: str) -> str:
    if preferred == "proceed":
        return "skip_or_proceed_preferred"
    if rejected == "proceed":
        return "take_card_over_skip"
    return "card_over_card"


def attr_diff(row: dict[str, Any]) -> dict[str, Any]:
    return ((row.get("outcome_diff_preferred_minus_rejected") or {}).get("attribution") or {})


def progress_units(attr: dict[str, Any]) -> float:
    return num(attr.get("monster_hp_reduction_observed")) + 40.0 * num(
        attr.get("alive_monster_reduction_observed")
    )


def interpretation_flags(
    *,
    attr: dict[str, Any],
    reason: str,
    preferred: str,
    rejected: str,
    label_status: str,
) -> list[str]:
    flags: list[str] = []
    mode = reason_win_mode(reason)
    hp_loss = num(attr.get("hp_loss_observed"))
    turns = num(attr.get("combat_turns_observed"))
    plays = num(attr.get("combat_play_card_count"))
    if mode == "progress_win" and hp_loss >= 10 and (turns >= 2 or plays >= 5):
        flags.append("survival_exposure_warning")
    if mode == "hp_preservation_win" and hp_loss <= -5:
        flags.append("hp_preservation_signal")
    if preferred == "proceed":
        flags.append("skip_or_proceed_preferred")
    if rejected == "proceed" and preferred != "proceed":
        flags.append("take_card_over_skip")
    if label_status in POLICY_SENSITIVE_LABELS:
        flags.append("policy_sensitive_case")
    if hp_loss >= 20:
        flags.append("high_hp_cost_trade")
    if progress_units(attr) >= 80:
        flags.append("large_progress_trade")
    return flags


def observation_index(report: dict[str, Any]) -> dict[tuple[str, str, int], dict[str, Any]]:
    out = {}
    for label in report.get("labels") or []:
        case_id = str(label.get("case_id") or "")
        for obs in label.get("observations") or []:
            if obs.get("status") != "ok":
                continue
            key = (case_id, str(obs.get("continuation_policy") or ""), int(obs.get("horizon") or 0))
            out[key] = obs
    return out


def label_maps(report: dict[str, Any]) -> tuple[dict[str, str], dict[str, dict[str, Any]]]:
    status_by_case = {}
    source_by_case = {}
    for label in report.get("labels") or []:
        case_id = str(label.get("case_id") or "")
        status_by_case[case_id] = str(label.get("label_status") or "unknown")
        source_by_case[case_id] = label.get("source_case") or {}
    return status_by_case, source_by_case


def compact_pair(
    row: dict[str, Any],
    *,
    label_status: str,
    source_case: dict[str, Any],
    observation: dict[str, Any] | None,
) -> dict[str, Any]:
    diff = row.get("outcome_diff_preferred_minus_rejected") or {}
    attr = diff.get("attribution") or {}
    preferred = card_label(row.get("preferred_outcome") or {})
    rejected = card_label(row.get("rejected_outcome") or {})
    reason = str(row.get("reason") or "unknown")
    flags = interpretation_flags(
        attr=attr,
        reason=reason,
        preferred=preferred,
        rejected=rejected,
        label_status=label_status,
    )
    return {
        "case_id": row.get("case_id"),
        "source_policy": row.get("source_policy"),
        "source_calibration_status": row.get("source_calibration_status"),
        "label_status": label_status,
        "continuation_policy": row.get("continuation_policy"),
        "horizon": int(row.get("horizon") or 0),
        "reason": reason,
        "win_mode": reason_win_mode(reason),
        "pair_shape": pair_shape(preferred, rejected),
        "preferred": preferred,
        "rejected": rejected,
        "preferred_key": row.get("preferred_key"),
        "rejected_key": row.get("rejected_key"),
        "outcome_diff": {
            "floor_delta": diff.get("floor_delta"),
            "combat_win_delta": diff.get("combat_win_delta"),
            "end_hp": diff.get("end_hp"),
            "reward_total": diff.get("reward_total"),
        },
        "attribution": attr,
        "derived": {
            "progress_units": round(progress_units(attr), 3),
            "hp_cost": round(max(num(attr.get("hp_loss_observed")), 0.0), 3),
            "hp_saved": round(max(-num(attr.get("hp_loss_observed")), 0.0), 3),
        },
        "interpretation_flags": flags,
        "source_case": {
            "seed": source_case.get("seed"),
            "step_index": source_case.get("step_index"),
            "act": source_case.get("act"),
            "floor": source_case.get("floor"),
            "hp": source_case.get("hp"),
            "cashout_kinds": source_case.get("cashout_kinds") or [],
            "cashout_gap": source_case.get("cashout_gap"),
            "chosen": (source_case.get("chosen") or {}).get("label")
            or (source_case.get("chosen") or {}).get("card_id"),
            "best_by_cashout": (source_case.get("best_by_cashout") or {}).get("label")
            or (source_case.get("best_by_cashout") or {}).get("card_id"),
            "trace_file": source_case.get("trace_file"),
        },
        "case_report_path": (observation or {}).get("case_report_path"),
    }


def drilldown_score(name: str, row: dict[str, Any]) -> float:
    attr = row.get("attribution") or {}
    diff = row.get("outcome_diff") or {}
    progress = num((row.get("derived") or {}).get("progress_units"))
    hp_saved = num((row.get("derived") or {}).get("hp_saved"))
    hp_cost = num((row.get("derived") or {}).get("hp_cost"))
    floor = num(diff.get("floor_delta"))
    combat = num(diff.get("combat_win_delta"))
    if name == "robust_progress":
        return progress + 60.0 * floor + 30.0 * combat - 0.25 * hp_cost
    if name == "hp_preservation":
        return hp_saved + 0.05 * max(progress, 0.0)
    if name == "skip_or_proceed_wins":
        return hp_saved + 0.2 * max(progress, 0.0) + 20.0 * floor
    if name == "policy_sensitive":
        return max(progress, 0.0) + hp_saved + 25.0 * abs(floor)
    if name == "rollout_refuted":
        return abs(progress) + hp_saved + hp_cost
    if name == "take_card_over_skip":
        return progress + 50.0 * floor + 20.0 * combat - 0.2 * hp_cost
    return progress + hp_saved


def bucket_candidates(rows: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    buckets: dict[str, list[dict[str, Any]]] = {
        "robust_progress": [],
        "hp_preservation": [],
        "skip_or_proceed_wins": [],
        "policy_sensitive": [],
        "rollout_refuted": [],
        "take_card_over_skip": [],
    }
    for row in rows:
        if row["label_status"] == "robust_confirmed" and row["win_mode"] == "progress_win" and row["preferred"] != "proceed":
            buckets["robust_progress"].append(row)
        if row["win_mode"] == "hp_preservation_win" and num((row.get("derived") or {}).get("hp_saved")) >= 5:
            buckets["hp_preservation"].append(row)
        if row["pair_shape"] == "skip_or_proceed_preferred":
            buckets["skip_or_proceed_wins"].append(row)
        if row["label_status"] in POLICY_SENSITIVE_LABELS:
            buckets["policy_sensitive"].append(row)
        if row["label_status"] == "rollout_refuted":
            buckets["rollout_refuted"].append(row)
        if row["pair_shape"] == "take_card_over_skip" and row["win_mode"] == "progress_win":
            buckets["take_card_over_skip"].append(row)
    return buckets


def select_bucket(
    name: str,
    rows: list[dict[str, Any]],
    *,
    limit: int,
    allow_duplicate_cases: bool,
) -> list[dict[str, Any]]:
    ranked = sorted(rows, key=lambda row: drilldown_score(name, row), reverse=True)
    selected: list[dict[str, Any]] = []
    seen_cases: set[str] = set()
    for row in ranked:
        case_id = str(row.get("case_id") or "")
        if not allow_duplicate_cases and case_id in seen_cases:
            continue
        selected.append(row)
        seen_cases.add(case_id)
        if len(selected) >= limit:
            break
    if not allow_duplicate_cases and len(selected) < limit:
        seen = {
            (
                str(row.get("case_id") or ""),
                str(row.get("continuation_policy") or ""),
                int(row.get("horizon") or 0),
                str(row.get("preferred") or ""),
                str(row.get("rejected") or ""),
            )
            for row in selected
        }
        for row in ranked:
            key = (
                str(row.get("case_id") or ""),
                str(row.get("continuation_policy") or ""),
                int(row.get("horizon") or 0),
                str(row.get("preferred") or ""),
                str(row.get("rejected") or ""),
            )
            if key in seen:
                continue
            selected.append(row)
            seen.add(key)
            if len(selected) >= limit:
                break
    return selected


def build_selection(
    report: dict[str, Any],
    pairwise_rows: list[dict[str, Any]],
    *,
    per_bucket: int,
    allow_duplicate_cases: bool,
) -> dict[str, Any]:
    status_by_case, source_by_case = label_maps(report)
    obs_index = observation_index(report)
    compact_rows = []
    for row in pairwise_rows:
        attr = attr_diff(row)
        if not attr:
            continue
        case_id = str(row.get("case_id") or "")
        policy = str(row.get("continuation_policy") or "")
        horizon = int(row.get("horizon") or 0)
        compact_rows.append(
            compact_pair(
                row,
                label_status=status_by_case.get(case_id, "unknown"),
                source_case=source_by_case.get(case_id, {}),
                observation=obs_index.get((case_id, policy, horizon)),
            )
        )

    buckets = bucket_candidates(compact_rows)
    selections = {
        name: select_bucket(
            name,
            rows,
            limit=per_bucket,
            allow_duplicate_cases=allow_duplicate_cases,
        )
        for name, rows in buckets.items()
    }
    return {
        "report_version": "cashout_rollout_drilldown_selection_v0",
        "source_report_version": report.get("report_version"),
        "summary": {
            "label_case_count": (report.get("summary") or {}).get("case_count"),
            "pairwise_rows": len(pairwise_rows),
            "pairwise_with_attribution": len(compact_rows),
            "per_bucket": per_bucket,
            "bucket_candidate_counts": {name: len(rows) for name, rows in buckets.items()},
            "bucket_selected_counts": {name: len(rows) for name, rows in selections.items()},
            "contract": "review queue only; selected rows are not training labels",
        },
        "buckets": selections,
    }


def write_markdown(path: Path, selection: dict[str, Any]) -> None:
    summary = selection["summary"]
    lines = [
        "# Cashout Rollout Drilldown Cases",
        "",
        "This is a review queue for residual mining. It is not a training dataset.",
        "",
        "## Summary",
        "",
        f"- source report: `{selection['source_report_version']}`",
        f"- label cases: `{summary['label_case_count']}`",
        f"- pairwise rows: `{summary['pairwise_rows']}`",
        f"- pairwise with attribution: `{summary['pairwise_with_attribution']}`",
        f"- bucket candidates: `{summary['bucket_candidate_counts']}`",
        f"- bucket selected: `{summary['bucket_selected_counts']}`",
        f"- contract: `{summary['contract']}`",
        "",
        "## Bucket Meaning",
        "",
        "- `robust_progress`: robust confirmed card choices that mainly win by progress.",
        "- `hp_preservation`: choices that mainly win by preserving HP at similar progress.",
        "- `skip_or_proceed_wins`: cases where skipping/continuing beat taking a card.",
        "- `policy_sensitive`: cases where continuation policy or horizon matters.",
        "- `rollout_refuted`: cases where static cashout was contradicted.",
        "- `take_card_over_skip`: card-taking progress wins against skip/proceed.",
        "",
    ]

    for bucket, rows in selection["buckets"].items():
        lines.extend(
            [
                f"## {bucket}",
                "",
                "| case | status | context | reason | preferred > rejected | floor/combat/hp | progress | hp saved | flags | source | report |",
                "|---|---|---|---|---|---|---:|---:|---|---|---|",
            ]
        )
        for row in rows:
            diff = row.get("outcome_diff") or {}
            derived = row.get("derived") or {}
            source = row.get("source_case") or {}
            source_text = "seed {seed} step {step} act {act} floor {floor} hp {hp} kinds {kinds}".format(
                seed=source.get("seed"),
                step=source.get("step_index"),
                act=source.get("act"),
                floor=source.get("floor"),
                hp=source.get("hp"),
                kinds=",".join(str(kind) for kind in source.get("cashout_kinds") or []),
            )
            lines.append(
                "| {case} | {status} | {ctx} | {reason} | {pref} > {rej} | {floor}/{combat}/{hp} | {progress} | {saved} | `{flags}` | {source} | `{report}` |".format(
                    case=row.get("case_id"),
                    status=row.get("label_status"),
                    ctx=f"{row.get('continuation_policy')}@{row.get('horizon')}",
                    reason=row.get("reason"),
                    pref=row.get("preferred"),
                    rej=row.get("rejected"),
                    floor=diff.get("floor_delta"),
                    combat=diff.get("combat_win_delta"),
                    hp=diff.get("end_hp"),
                    progress=derived.get("progress_units"),
                    saved=derived.get("hp_saved"),
                    flags=row.get("interpretation_flags") or [],
                    source=source_text,
                    report=row.get("case_report_path"),
                )
            )
        lines.append("")

    real = resolve(path)
    real.parent.mkdir(parents=True, exist_ok=True)
    real.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    label_dir = resolve(args.label_dir)
    report_path = args.report or label_dir / "cashout_rollout_label_report.json"
    pairwise_path = args.pairwise_labels or label_dir / "pairwise_labels.jsonl"
    out_path = resolve(args.out) if args.out else label_dir / "cashout_rollout_drilldown_cases.json"
    markdown_path = resolve(args.markdown_out) if args.markdown_out else out_path.with_suffix(".md")

    report = read_json(report_path)
    pairwise_rows = read_jsonl(pairwise_path)
    selection = build_selection(
        report,
        pairwise_rows,
        per_bucket=int(args.per_bucket),
        allow_duplicate_cases=bool(args.allow_duplicate_cases),
    )
    write_json(out_path, selection)
    write_markdown(markdown_path, selection)
    print(
        json.dumps(
            {
                "out": str(out_path),
                "markdown_out": str(markdown_path),
                "summary": selection["summary"],
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
