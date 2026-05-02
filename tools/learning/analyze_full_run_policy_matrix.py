#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from statistics import mean
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Audit behavior signals from a full-run PPO sanity matrix."
    )
    parser.add_argument(
        "--matrix",
        type=Path,
        help="Matrix JSON from benchmark_full_run_ppo_sanity.py. Defaults to latest full_matrix*.json.",
    )
    parser.add_argument("--out", type=Path, help="JSON audit output path.")
    parser.add_argument("--markdown-out", type=Path, help="Markdown audit output path.")
    parser.add_argument("--print-markdown", action="store_true")
    parser.add_argument("--seed-floor-spread-threshold", type=float, default=3.0)
    parser.add_argument("--under-rule-floor-threshold", type=float, default=2.0)
    parser.add_argument("--low-card-select-rate", type=float, default=0.05)
    parser.add_argument("--high-shop-potion-buy-rate", type=float, default=0.20)
    parser.add_argument("--high-combat-potion-use-rate", type=float, default=0.12)
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def default_matrix_path() -> Path:
    artifact_dir = REPO_ROOT / "tools" / "artifacts" / "full_run_rl_matrix"
    candidates = [
        path
        for path in artifact_dir.glob("full_matrix*.json")
        if "behavior_audit" not in path.stem
    ]
    candidates.sort(key=lambda path: path.stat().st_mtime, reverse=True)
    if not candidates:
        raise SystemExit(
            "missing matrix JSON; run tools/learning/benchmark_full_run_ppo_sanity.py first"
        )
    return candidates[0]


def as_counter(value: Any) -> Counter[str]:
    if not isinstance(value, dict):
        return Counter()
    counts: Counter[str] = Counter()
    for key, raw in value.items():
        try:
            counts[str(key)] += int(raw)
        except (TypeError, ValueError):
            continue
    return counts


def ratio(numerator: float, denominator: float) -> float:
    if denominator <= 0:
        return 0.0
    return float(numerator) / float(denominator)


def pct(value: float) -> str:
    return f"{value * 100.0:.1f}%"


def count_per_episode(count: int, episodes: int) -> float:
    return ratio(float(count), float(episodes))


def policy_summary(item: dict[str, Any]) -> dict[str, Any]:
    return {
        "policy": item.get("policy"),
        "episodes": int(item.get("episodes") or item.get("episodes_completed") or 0),
        "average_floor": float(item.get("average_floor") or 0.0),
        "average_reward": float(item.get("average_reward") or 0.0),
        "average_steps": float(item.get("average_steps") or 0.0),
        "crash_count": int(item.get("crash_count") or 0),
        "illegal_action_count": int(item.get("illegal_action_count") or 0),
        "no_progress_count": int(item.get("no_progress_count") or 0),
        "contract_failure_count": int(item.get("contract_failure_count") or 0),
        "anomaly_flags": list(item.get("anomaly_flags") or []),
    }


def merge_eval_counts(evals: list[dict[str, Any]], key: str) -> Counter[str]:
    merged: Counter[str] = Counter()
    for item in evals:
        merged.update(as_counter(item.get(key)))
    return merged


def audit_ppo_seed(
    run: dict[str, Any],
    *,
    rule_average_floor: float | None,
    ppo_peer_average_floor: float,
    args: argparse.Namespace,
) -> dict[str, Any]:
    evals = list(run.get("evals") or [])
    aggregate = dict(run.get("aggregate") or {})
    episodes = int(aggregate.get("episodes") or sum(int(item.get("episodes") or 0) for item in evals))
    decision_counts = merge_eval_counts(evals, "decision_type_counts")
    action_counts = merge_eval_counts(evals, "action_type_counts")
    prefix_counts = merge_eval_counts(evals, "action_key_prefix_counts")
    combat_decisions = decision_counts["combat"]
    reward_decisions = decision_counts["reward"]
    shop_decisions = decision_counts["shop"]
    campfire_decisions = decision_counts["campfire"]

    average_floor = float(aggregate.get("average_floor") or 0.0)
    flags = list(aggregate.get("anomaly_flags") or [])
    if rule_average_floor is not None and average_floor + args.under_rule_floor_threshold < rule_average_floor:
        flags.append("under_rule_baseline")
    if average_floor + args.under_rule_floor_threshold < ppo_peer_average_floor:
        flags.append("under_ppo_peer_mean")

    select_card_rate = ratio(action_counts["select_card"], reward_decisions)
    if reward_decisions >= 100 and select_card_rate < args.low_card_select_rate:
        flags.append("reward_card_selection_collapse")

    shop_potion_buy_rate = ratio(action_counts["buy_potion"], shop_decisions)
    if shop_decisions >= 20 and shop_potion_buy_rate > args.high_shop_potion_buy_rate:
        flags.append("high_shop_potion_buy_rate")

    combat_potion_use_rate = ratio(action_counts["use_potion"], combat_decisions)
    if combat_decisions >= 100 and combat_potion_use_rate > args.high_combat_potion_use_rate:
        flags.append("high_combat_potion_use_rate")

    top_action_type = action_counts.most_common(1)[0] if action_counts else ("unknown", 0)
    top_action_share = ratio(top_action_type[1], sum(action_counts.values()))

    return {
        "train_seed": int(run.get("train_seed") or 0),
        "summary": policy_summary(aggregate),
        "decision_type_counts": dict(decision_counts),
        "action_type_counts": dict(action_counts),
        "action_key_prefix_counts": dict(prefix_counts),
        "rates": {
            "play_card_per_combat_decision": ratio(action_counts["play_card"], combat_decisions),
            "end_turn_per_combat_decision": ratio(action_counts["end_turn"], combat_decisions),
            "use_potion_per_combat_decision": combat_potion_use_rate,
            "select_card_per_reward_decision": select_card_rate,
            "claim_reward_per_reward_decision": ratio(action_counts["claim_reward"], reward_decisions),
            "buy_card_per_shop_decision": ratio(action_counts["buy_card"], shop_decisions),
            "buy_relic_per_shop_decision": ratio(action_counts["buy_relic"], shop_decisions),
            "buy_potion_per_shop_decision": shop_potion_buy_rate,
            "purge_card_per_shop_decision": ratio(action_counts["purge_card"], shop_decisions),
            "campfire_smith_per_campfire": ratio(prefix_counts["campfire/smith"], campfire_decisions),
            "campfire_rest_per_campfire": ratio(prefix_counts["campfire/rest"], campfire_decisions),
            "campfire_toke_per_campfire": ratio(prefix_counts["campfire/toke"], campfire_decisions),
            "buy_potion_per_episode": count_per_episode(action_counts["buy_potion"], episodes),
            "use_potion_per_episode": count_per_episode(action_counts["use_potion"], episodes),
            "select_card_per_episode": count_per_episode(action_counts["select_card"], episodes),
            "top_action_type_share": top_action_share,
        },
        "top_action_type": {"name": top_action_type[0], "count": int(top_action_type[1])},
        "flags": sorted(set(flags)),
    }


def audit_baselines(matrix: dict[str, Any]) -> dict[str, Any]:
    return {
        name: policy_summary(summary)
        for name, summary in dict(matrix.get("baseline_aggregates") or {}).items()
    }


def build_findings(
    matrix: dict[str, Any],
    baseline_audit: dict[str, Any],
    ppo_seed_audit: list[dict[str, Any]],
    args: argparse.Namespace,
) -> list[dict[str, Any]]:
    findings: list[dict[str, Any]] = []
    issue_flags = list(matrix.get("issue_flags") or [])
    if issue_flags:
        findings.append(
            {
                "severity": "high",
                "title": "Matrix contract issue flags are present",
                "detail": ", ".join(issue_flags),
            }
        )
    else:
        findings.append(
            {
                "severity": "info",
                "title": "No matrix-level contract issue flags",
                "detail": "The benchmark reported no crash, illegal-action, or no-progress flags.",
            }
        )

    floors = [float(item["summary"].get("average_floor") or 0.0) for item in ppo_seed_audit]
    if floors:
        spread = max(floors) - min(floors)
        if spread >= args.seed_floor_spread_threshold:
            findings.append(
                {
                    "severity": "medium",
                    "title": "PPO seed spread is large",
                    "detail": f"average_floor range is {spread:.2f} across PPO train seeds.",
                }
            )

    rule = baseline_audit.get("rule_baseline_v0")
    if rule:
        rule_floor = float(rule.get("average_floor") or 0.0)
        weak = [
            item
            for item in ppo_seed_audit
            if float(item["summary"].get("average_floor") or 0.0)
            + args.under_rule_floor_threshold
            < rule_floor
        ]
        for item in weak:
            floor = float(item["summary"].get("average_floor") or 0.0)
            findings.append(
                {
                    "severity": "medium",
                    "title": f"PPO seed {item['train_seed']} underperforms rule baseline",
                    "detail": f"average_floor={floor:.2f}, rule_baseline_v0={rule_floor:.2f}.",
                }
            )

    for item in ppo_seed_audit:
        if "reward_card_selection_collapse" in item["flags"]:
            rate = float(item["rates"].get("select_card_per_reward_decision") or 0.0)
            count = int((item.get("action_type_counts") or {}).get("select_card") or 0)
            findings.append(
                {
                    "severity": "high",
                    "title": f"PPO seed {item['train_seed']} almost never selects reward cards",
                    "detail": f"select_card_count={count}, select_card/reward={pct(rate)}.",
                }
            )

    return findings


def markdown_table(headers: list[str], rows: list[list[str]]) -> str:
    lines = [
        "| " + " | ".join(headers) + " |",
        "| " + " | ".join(["---"] * len(headers)) + " |",
    ]
    for row in rows:
        lines.append("| " + " | ".join(row) + " |")
    return "\n".join(lines)


def render_markdown(report: dict[str, Any]) -> str:
    lines: list[str] = []
    lines.append("# Full-Run Policy Behavior Audit")
    lines.append("")
    lines.append(f"- Matrix: `{report['matrix_path']}`")
    lines.append(f"- Generated: `{report['generated_at_utc']}`")
    lines.append("")
    lines.append("## Contract Summary")
    baseline_rows = []
    for name, item in report["baseline_audit"].items():
        baseline_rows.append(
            [
                name,
                str(item["episodes"]),
                f"{item['average_floor']:.2f}",
                f"{item['average_reward']:.2f}",
                str(item["crash_count"]),
                str(item["illegal_action_count"]),
                str(item["no_progress_count"]),
                ", ".join(item["anomaly_flags"]) or "-",
            ]
        )
    ppo = report["ppo_aggregate"]
    baseline_rows.append(
        [
            "ppo_all_seeds",
            str(ppo["episodes"]),
            f"{ppo['average_floor']:.2f}",
            f"{ppo['average_reward']:.2f}",
            str(ppo["crash_count"]),
            str(ppo["illegal_action_count"]),
            str(ppo["no_progress_count"]),
            ", ".join(ppo["anomaly_flags"]) or "-",
        ]
    )
    lines.append(
        markdown_table(
            ["policy", "episodes", "avg_floor", "avg_reward", "crash", "illegal", "no_progress", "flags"],
            baseline_rows,
        )
    )
    lines.append("")
    lines.append("## PPO Seed Behavior")
    seed_rows = []
    for item in report["ppo_seed_audit"]:
        rates = item["rates"]
        seed_rows.append(
            [
                str(item["train_seed"]),
                f"{item['summary']['average_floor']:.2f}",
                f"{item['summary']['average_reward']:.2f}",
                pct(float(rates["select_card_per_reward_decision"])),
                f"{rates['select_card_per_episode']:.2f}",
                pct(float(rates["buy_potion_per_shop_decision"])),
                pct(float(rates["use_potion_per_combat_decision"])),
                pct(float(rates["campfire_rest_per_campfire"])),
                ", ".join(item["flags"]) or "-",
            ]
        )
    lines.append(
        markdown_table(
            [
                "seed",
                "avg_floor",
                "avg_reward",
                "select_card/reward",
                "select_card/ep",
                "buy_potion/shop",
                "use_potion/combat",
                "rest/campfire",
                "flags",
            ],
            seed_rows,
        )
    )
    lines.append("")
    lines.append("## Findings")
    for finding in report["findings"]:
        lines.append(
            f"- **{finding['severity']}**: {finding['title']} - {finding['detail']}"
        )
    lines.append("")
    lines.append("## Limitations")
    for limitation in report["limitations"]:
        lines.append(f"- {limitation}")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    args = parse_args()
    matrix_path = (args.matrix or default_matrix_path()).resolve()
    matrix = read_json(matrix_path)
    if not matrix.get("ppo_runs"):
        raise SystemExit(f"{matrix_path} does not look like a full-run PPO matrix")

    baseline_audit = audit_baselines(matrix)
    ppo_aggregate = policy_summary(dict(matrix.get("ppo_aggregate") or {}))
    ppo_runs = list(matrix.get("ppo_runs") or [])
    ppo_peer_average_floor = mean(
        float((run.get("aggregate") or {}).get("average_floor") or 0.0) for run in ppo_runs
    )
    rule_summary = baseline_audit.get("rule_baseline_v0")
    rule_average_floor = (
        float(rule_summary.get("average_floor") or 0.0) if rule_summary else None
    )
    ppo_seed_audit = [
        audit_ppo_seed(
            run,
            rule_average_floor=rule_average_floor,
            ppo_peer_average_floor=ppo_peer_average_floor,
            args=args,
        )
        for run in ppo_runs
    ]
    findings = build_findings(matrix, baseline_audit, ppo_seed_audit, args)
    report = {
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "matrix_path": str(matrix_path),
        "matrix_config": matrix.get("config") or {},
        "baseline_audit": baseline_audit,
        "ppo_aggregate": ppo_aggregate,
        "ppo_seed_audit": ppo_seed_audit,
        "findings": findings,
        "limitations": [
            "Rust baseline entries in the current matrix include decision counts but not action-type counts, so action behavior comparisons are PPO-focused.",
            "The audit detects behavioral symptoms, not policy quality; it should gate longer training rather than replace evaluation.",
            "Reward-card selection rates are coarse because reward screens include claim/proceed/card-choice phases.",
        ],
    }
    out_path = args.out or matrix_path.with_name(f"{matrix_path.stem}_behavior_audit.json")
    markdown_path = args.markdown_out or matrix_path.with_name(
        f"{matrix_path.stem}_behavior_audit.md"
    )
    write_json(out_path, report)
    markdown = render_markdown(report)
    markdown_path.parent.mkdir(parents=True, exist_ok=True)
    markdown_path.write_text(markdown, encoding="utf-8")
    if args.print_markdown:
        print(markdown)
    else:
        print(json.dumps({"json": str(out_path), "markdown": str(markdown_path)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
