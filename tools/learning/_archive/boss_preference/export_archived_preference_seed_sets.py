#!/usr/bin/env python3
from __future__ import annotations

import argparse
import glob
import json
import subprocess
from collections import defaultdict
from pathlib import Path
from typing import Any

from combat_reranker_common import iter_jsonl, write_json

REPO_ROOT = Path(__file__).resolve().parents[2]


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def find_audit_binary(explicit: Path | None) -> list[str]:
    if explicit and explicit.exists():
        return [str(explicit)]
    exe = REPO_ROOT / "target" / "release" / "combat_decision_audit.exe"
    if exe.exists():
        return [str(exe)]
    bin_path = REPO_ROOT / "target" / "release" / "combat_decision_audit"
    if bin_path.exists():
        return [str(bin_path)]
    return ["cargo", "run", "--release", "--bin", "combat_decision_audit", "--"]


def suspect_priority(row: dict[str, Any]) -> float:
    score = 0.0
    if row.get("heuristic_search_gap"):
        score += 5.0
    if row.get("large_sequence_bonus"):
        score += 3.0
    if row.get("snapshot_normalized_state") is not None:
        score += 1.0
    reasons = set(str(reason) for reason in (row.get("reasons") or []))
    if "sequencing_conflict" in reasons:
        score += 3.0
    if "branch_opening_conflict" in reasons:
        score += 2.0
    if "tight_root_gap" in reasons:
        score -= 3.0
    score += min(abs(float(row.get("top_gap") or 0.0)) / 1000.0, 10.0)
    score += min(abs(float(row.get("sequence_downside_penalty") or 0.0)) / 4000.0, 5.0)
    return score


def select_frames(combat_suspects_path: Path, per_run_limit: int, min_top_gap: float) -> list[int]:
    candidates: list[dict[str, Any]] = []
    for _, row in iter_jsonl(combat_suspects_path):
        if not row.get("heuristic_search_gap"):
            continue
        top_gap = abs(float(row.get("top_gap") or 0.0))
        reasons = set(str(reason) for reason in (row.get("reasons") or []))
        if top_gap < min_top_gap and not ({"sequencing_conflict", "branch_opening_conflict"} & reasons):
            continue
        if row.get("tight_root_gap") and len(reasons) == 1:
            continue
        candidates.append(row)
    candidates.sort(key=suspect_priority, reverse=True)
    chosen = []
    seen = set()
    for row in candidates:
        frame = int(row.get("frame_count") or 0)
        if frame <= 0 or frame in seen:
            continue
        seen.add(frame)
        chosen.append(frame)
        if len(chosen) >= per_run_limit:
            break
    return chosen


def main() -> int:
    parser = argparse.ArgumentParser(description="Export offline preference seed sets from archived clean baseline runs.")
    parser.add_argument(
        "--baseline",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_baseline.json",
        type=Path,
        help="Frozen baseline manifest.",
    )
    parser.add_argument(
        "--out-dir",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "generated_policy_seed_sets",
        type=Path,
        help="Directory for generated policy seed sets and summaries.",
    )
    parser.add_argument(
        "--audit-binary",
        default=None,
        type=Path,
        help="Optional explicit combat_decision_audit binary.",
    )
    parser.add_argument("--per-run-limit", default=8, type=int, help="Max selected frames per archived clean run.")
    parser.add_argument("--min-top-gap", default=100.0, type=float, help="Minimum top_gap to treat a suspect as export-worthy.")
    parser.add_argument("--decision-depth", default=4, type=int)
    parser.add_argument("--top-k", default=3, type=int)
    parser.add_argument("--branch-cap", default=6, type=int)
    args = parser.parse_args()

    baseline = read_json(args.baseline)
    audit_cmd_prefix = find_audit_binary(args.audit_binary)
    args.out_dir.mkdir(parents=True, exist_ok=True)

    run_summaries = []
    generated_files = []
    total_frames = 0
    total_samples = 0

    for run in baseline.get("selected_runs") or []:
        run_id = str(run.get("run_id") or "")
        raw_path = Path(str(run.get("raw_path") or ""))
        suspects_path = Path(str(run.get("combat_suspects_path") or ""))
        if not run_id or not raw_path.exists() or not suspects_path.exists():
            continue
        frames = select_frames(suspects_path, args.per_run_limit, args.min_top_gap)
        if not frames:
            continue
        out_path = args.out_dir / f"policy_seed_set_{run_id}.jsonl"
        summary_out = args.out_dir / f"policy_seed_set_{run_id}.summary.json"
        cmd = audit_cmd_prefix + [
            "export-preference-seed-set",
            "--raw",
            str(raw_path),
            "--out",
            str(out_path),
            "--summary-out",
            str(summary_out),
            "--frames",
            ",".join(str(frame) for frame in frames),
            "--decision-depth",
            str(args.decision_depth),
            "--top-k",
            str(args.top_k),
            "--branch-cap",
            str(args.branch_cap),
        ]
        subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        summary = read_json(summary_out)
        run_summaries.append(
            {
                "run_id": run_id,
                "raw_path": str(raw_path),
                "combat_suspects_path": str(suspects_path),
                "selected_frames": frames,
                "exported_samples": int(summary.get("exported_samples") or 0),
                "audited_frames": int(summary.get("audited_frames") or 0),
                "summary_path": str(summary_out),
                "seed_set_path": str(out_path),
            }
        )
        generated_files.append(str(out_path))
        total_frames += len(frames)
        total_samples += int(summary.get("exported_samples") or 0)

    manifest = {
        "baseline_path": str(args.baseline),
        "audit_invocation": audit_cmd_prefix,
        "per_run_limit": args.per_run_limit,
        "min_top_gap": args.min_top_gap,
        "decision_depth": args.decision_depth,
        "top_k": args.top_k,
        "branch_cap": args.branch_cap,
        "generated_seed_files": generated_files,
        "run_summaries": run_summaries,
        "totals": {
            "runs": len(run_summaries),
            "selected_frames": total_frames,
            "exported_samples": total_samples,
        },
        "notes": [
            "generated policy seed sets come from archived clean baseline runs, not new livecomm sessions",
            "only high-value heuristic/search suspect frames are selected for export",
            "these outputs can be folded into the policy_seed teacher dataset",
        ],
    }
    write_json(args.out_dir / "generated_policy_seed_manifest.json", manifest)

    print(json.dumps(manifest, indent=2, ensure_ascii=False))
    print(f"wrote generated archived preference seed sets to {args.out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
