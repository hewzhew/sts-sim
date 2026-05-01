#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, iter_jsonl, write_json, write_jsonl


def default_sidecar(path: Path, suffix: str) -> Path:
    return path.with_name(f"{path.stem}{suffix}")


def load_jsonl(path: Path | None) -> list[dict[str, Any]]:
    if path is None or not path.exists():
        return []
    return [row for _, row in iter_jsonl(path)]


def failed_label(group: dict[str, Any]) -> str:
    return str((group.get("failed_action") or {}).get("label") or "")


def failed_source(group: dict[str, Any]) -> str:
    return str((group.get("failed_action") or {}).get("source") or "unknown")


def rescue_candidates(group: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        candidate
        for candidate in group.get("candidate_outcomes") or []
        if bool(candidate.get("is_rescue_candidate"))
    ]


def has_survival_rescue(group: dict[str, Any]) -> bool:
    for candidate in rescue_candidates(group):
        reasons = set(candidate.get("counterfactual_reasons") or [])
        if reasons & {"avoids_horizon_defeat", "restores_horizon_survival", "avoids_root_defeat"}:
            return True
    return False


def is_hard_survival(group: dict[str, Any]) -> bool:
    diagnostics = group.get("candidate_group") or {}
    return bool(has_survival_rescue(group)) and bool(
        diagnostics.get("survival_disagreement") or diagnostics.get("terminal_defeat_disagreement")
    )


def is_greedy_transition_bad(group: dict[str, Any]) -> bool:
    return failed_source(group) in {"teacher", "greedy_transition"}


def is_random_trivial(group: dict[str, Any]) -> bool:
    return failed_source(group) == "random" and failed_label(group) == "EndTurn"


def is_random_nontrivial(group: dict[str, Any]) -> bool:
    return failed_source(group) == "random" and failed_label(group) != "EndTurn"


def group_tags(group: dict[str, Any]) -> list[str]:
    tags: list[str] = []
    if is_hard_survival(group):
        tags.append("hard_survival")
    if is_greedy_transition_bad(group):
        tags.append("greedy_transition_bad")
    if is_random_trivial(group):
        tags.append("random_trivial")
    elif is_random_nontrivial(group):
        tags.append("random_nontrivial")
    if float((group.get("candidate_group") or {}).get("top2_gap") or 0.0) >= 0.10:
        tags.append("top2_gap_010")
    if group.get("confidence") == "root_defeat_counterfactual":
        tags.append("root_defeat")
    return tags


def best_rescue_delta(group: dict[str, Any], key: str) -> float:
    values = [
        float(((candidate.get("delta_vs_failed") or {}).get(key)) or 0.0)
        for candidate in rescue_candidates(group)
    ]
    return max(values) if values else 0.0


def summarize_groups(groups: list[dict[str, Any]], macro_rows: list[dict[str, Any]]) -> dict[str, Any]:
    tag_counter = Counter(tag for group in groups for tag in group_tags(group))
    source_counter = Counter(failed_source(group) for group in groups)
    label_counter = Counter(failed_label(group) for group in groups)
    confidence_counter = Counter(str(group.get("confidence") or "unknown") for group in groups)
    offset_counter = Counter(int(group.get("backtrack_offset") or 0) for group in groups)
    rescue_count_counter = Counter(len(rescue_candidates(group)) for group in groups)
    macro_source_counter = Counter(str(row.get("source_policy") or "unknown") for row in macro_rows)
    macro_seed_counter = Counter(int(row.get("seed") or 0) for row in macro_rows)

    recommended = [
        group
        for group in groups
        if is_hard_survival(group) and not is_random_trivial(group)
    ]
    random_trivial = [group for group in groups if is_random_trivial(group)]

    return {
        "group_count": len(groups),
        "macro_backtrack_count": len(macro_rows),
        "recommended_candidate_value_group_count": len(recommended),
        "random_trivial_holdout_count": len(random_trivial),
        "tags": dict(sorted(tag_counter.items())),
        "failed_source": dict(sorted(source_counter.items())),
        "failed_label_top": dict(label_counter.most_common(20)),
        "confidence": dict(sorted(confidence_counter.items())),
        "backtrack_offset": {str(key): value for key, value in sorted(offset_counter.items())},
        "rescue_candidates_per_group": {str(key): value for key, value in sorted(rescue_count_counter.items())},
        "macro_source_policy": dict(sorted(macro_source_counter.items())),
        "macro_seed_count": len(macro_seed_counter),
        "return_delta": {
            "min_best_rescue": min((best_rescue_delta(group, "discounted_return_delta") for group in groups), default=0.0),
            "max_best_rescue": max((best_rescue_delta(group, "discounted_return_delta") for group in groups), default=0.0),
        },
        "hp_delta": {
            "min_best_rescue": min((best_rescue_delta(group, "hp_delta_delta") for group in groups), default=0.0),
            "max_best_rescue": max((best_rescue_delta(group, "hp_delta_delta") for group in groups), default=0.0),
        },
    }


def add_tags(groups: list[dict[str, Any]]) -> list[dict[str, Any]]:
    tagged: list[dict[str, Any]] = []
    for group in groups:
        row = dict(group)
        row["audit_tags"] = group_tags(group)
        row["recommended_for_candidate_value"] = bool(is_hard_survival(group) and not is_random_trivial(group))
        row["random_trivial_holdout"] = bool(is_random_trivial(group))
        tagged.append(row)
    return tagged


def write_review(
    path: Path,
    *,
    summary: dict[str, Any],
    groups: list[dict[str, Any]],
    macro_rows: list[dict[str, Any]],
) -> None:
    lines: list[str] = []
    lines.append("# Combat Rescue Decision Group Audit")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append("| metric | value |")
    lines.append("|---|---:|")
    for key in [
        "group_count",
        "macro_backtrack_count",
        "recommended_candidate_value_group_count",
        "random_trivial_holdout_count",
    ]:
        lines.append(f"| {key} | {summary.get(key, 0)} |")
    lines.append("")
    lines.append("## Tags")
    lines.append("")
    lines.append("| tag | count |")
    lines.append("|---|---:|")
    for key, value in (summary.get("tags") or {}).items():
        lines.append(f"| {key} | {value} |")
    lines.append("")
    lines.append("## Groups")
    lines.append("")
    lines.append("| seed | step | offset | source | failed | confidence | tags | rescue count | best return delta | best hp delta |")
    lines.append("|---:|---:|---:|---|---|---|---|---:|---:|---:|")
    for group in groups:
        tags = ", ".join(group_tags(group))
        lines.append(
            f"| {group.get('seed')} | {group.get('decision_step')} | {group.get('backtrack_offset')} | "
            f"{failed_source(group)} | {failed_label(group)} | {group.get('confidence')} | {tags} | "
            f"{len(rescue_candidates(group))} | {best_rescue_delta(group, 'discounted_return_delta'):.3f} | "
            f"{best_rescue_delta(group, 'hp_delta_delta'):.1f} |"
        )
    lines.append("")
    lines.append("## Macro Backtrack")
    lines.append("")
    lines.append("| seed | steps | reject_reason | last_label | last_hp | last_unblocked |")
    lines.append("|---:|---:|---|---|---:|---:|")
    for row in macro_rows[:50]:
        last_steps = row.get("last_failed_steps") or []
        last = last_steps[-1] if last_steps else {}
        state = last.get("state") or {}
        lines.append(
            f"| {row.get('seed')} | {row.get('episode_steps')} | {row.get('reject_reason')} | "
            f"{last.get('label')} | {state.get('player_hp')} | {state.get('visible_unblocked')} |"
        )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description="Audit combat rescue decision groups and split high-signal buckets.")
    parser.add_argument("--groups", required=True, type=Path)
    parser.add_argument("--macro-manifest", default=None, type=Path)
    parser.add_argument("--out", default=None, type=Path)
    parser.add_argument("--review-out", default=None, type=Path)
    parser.add_argument("--bucket-prefix", default=None, type=Path)
    args = parser.parse_args()

    groups = load_jsonl(args.groups)
    macro_manifest = args.macro_manifest or default_sidecar(args.groups, ".macro_backtrack.jsonl")
    macro_rows = load_jsonl(macro_manifest)
    summary = {
        "groups": str(args.groups),
        "macro_backtrack_manifest": str(macro_manifest),
        **summarize_groups(groups, macro_rows),
    }

    out = args.out or default_sidecar(args.groups, ".audit.json")
    review_out = args.review_out or default_sidecar(args.groups, ".audit.md")
    bucket_prefix = args.bucket_prefix or args.groups.with_name(args.groups.stem)

    tagged_groups = add_tags(groups)
    write_json(out, summary)
    write_review(review_out, summary=summary, groups=tagged_groups, macro_rows=macro_rows)
    write_jsonl(default_sidecar(bucket_prefix, ".tagged.jsonl"), tagged_groups)
    write_jsonl(default_sidecar(bucket_prefix, ".hard_survival.jsonl"), [group for group in tagged_groups if "hard_survival" in group["audit_tags"]])
    write_jsonl(
        default_sidecar(bucket_prefix, ".greedy_transition_bad.jsonl"),
        [group for group in tagged_groups if "greedy_transition_bad" in group["audit_tags"]],
    )
    write_jsonl(default_sidecar(bucket_prefix, ".random_trivial.jsonl"), [group for group in tagged_groups if "random_trivial" in group["audit_tags"]])
    write_jsonl(
        default_sidecar(bucket_prefix, ".candidate_value_recommended.jsonl"),
        [group for group in tagged_groups if group["recommended_for_candidate_value"]],
    )

    print(json.dumps(summary, indent=2, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
