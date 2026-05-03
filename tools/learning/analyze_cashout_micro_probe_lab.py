#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT, write_json


DEFAULT_LABEL_DIR = (
    REPO_ROOT
    / "tools"
    / "artifacts"
    / "card_cashout_rollout_labels"
    / "v1_1_attribution_100case_cashout_v0_6"
)

REPORT_VERSION = "cashout_micro_probe_lab_v0"

CASE_BUCKETS = {
    "rollout_refuted": "true_static_fix_needed",
    "requires_cashout_policy": "policy_utilization_gap",
    "rollout_unstable": "continuation_bad_or_unknown",
    "robust_confirmed": "robust_training_candidate",
    "rollout_equivalent": "equivalent_or_low_signal",
    "rollout_confirmed": "weak_confirmed",
}

DRAW_CARDS = {
    "BattleTrance",
    "BurningPact",
    "DarkEmbrace",
    "Offering",
    "PommelStrike",
    "ShrugItOff",
    "Warcry",
}
SCALING_CARDS = {
    "Barricade",
    "Berserk",
    "Brutality",
    "Corruption",
    "DarkEmbrace",
    "DemonForm",
    "Evolve",
    "FeelNoPain",
    "FireBreathing",
    "Inflame",
    "Juggernaut",
    "LimitBreak",
    "Metallicize",
    "Rupture",
    "SpotWeakness",
}
EXHAUST_PAYOFF_CARDS = {"DarkEmbrace", "FeelNoPain", "Corruption", "SecondWind", "FiendFire"}
EXHAUST_CARDS = {
    "BurningPact",
    "Corruption",
    "DarkEmbrace",
    "FeelNoPain",
    "FiendFire",
    "Havoc",
    "SecondWind",
    "SeverSoul",
    "TrueGrit",
}
AOE_CARDS = {"Immolate", "Cleave", "Whirlwind", "ThunderClap", "Shockwave", "Reaper"}
RESOURCE_WINDOW_CARDS = {"Offering", "SeeingRed", "Bloodletting"}
KILL_WINDOW_CARDS = {"Feed", "HandOfGreed", "RitualDagger"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Build a small continuation micro-probe lab from cashout rollout labels. "
            "V0 reuses existing counterfactual reports; it does not run new engine probes."
        )
    )
    parser.add_argument("--label-dir", type=Path, default=DEFAULT_LABEL_DIR)
    parser.add_argument("--label-report", type=Path)
    parser.add_argument("--policy-utilization-report", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--markdown-out", type=Path)
    parser.add_argument("--queue-out", type=Path)
    parser.add_argument("--top-n", type=int, default=20)
    parser.add_argument(
        "--buckets",
        default="policy_utilization_gap,continuation_bad_or_unknown,true_static_fix_needed",
        help="Comma-separated case buckets to place in the probe queue, or 'all'.",
    )
    return parser.parse_args()


def resolve(path: Path) -> Path:
    return path if path.is_absolute() else REPO_ROOT / path


def read_json(path: Path) -> dict[str, Any]:
    with resolve(path).open("r", encoding="utf-8") as handle:
        return json.load(handle)


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    real = resolve(path)
    real.parent.mkdir(parents=True, exist_ok=True)
    with real.open("w", encoding="utf-8", newline="\n") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")


def parse_csv(text: str) -> list[str]:
    return [part.strip() for part in text.split(",") if part.strip()]


def num(value: Any) -> float:
    try:
        return float(value or 0.0)
    except (TypeError, ValueError):
        return 0.0


def card_id_from_card(card: dict[str, Any] | None) -> str:
    if not card:
        return "proceed"
    return str(card.get("card_id") or "unknown")


def card_id_from_outcome(outcome: dict[str, Any]) -> str:
    return card_id_from_card(outcome.get("candidate_card")) or str(outcome.get("candidate_key") or "unknown")


def card_tags(card: dict[str, Any] | None) -> list[str]:
    if not card:
        return ["skip_or_proceed"]
    cid = card_id_from_card(card)
    tags: list[str] = []
    if num(card.get("base_damage")) > 0 or num(card.get("upgraded_damage")) > 0:
        tags.append("frontload")
    if num(card.get("base_block")) > 0 or num(card.get("upgraded_block")) > 0:
        tags.append("block")
    if bool(card.get("aoe")) or bool(card.get("multi_damage")) or cid in AOE_CARDS:
        tags.append("aoe")
    if bool(card.get("draws_cards")) or cid in DRAW_CARDS:
        tags.append("draw")
    if bool(card.get("gains_energy")) or cid in RESOURCE_WINDOW_CARDS:
        tags.append("resource_window")
    if bool(card.get("scaling_piece")) or cid in SCALING_CARDS:
        tags.append("setup_or_scaling")
    if bool(card.get("exhaust")) or cid in EXHAUST_CARDS:
        tags.append("exhaust")
    if cid in EXHAUST_PAYOFF_CARDS:
        tags.append("exhaust_payoff")
    if cid in KILL_WINDOW_CARDS:
        tags.append("kill_window")
    return sorted(set(tags)) or ["low_signal"]


def probe_families(card: dict[str, Any] | None, cashout_kinds: list[str] | None = None) -> list[str]:
    cid = card_id_from_card(card)
    tags = set(card_tags(card))
    kinds = set(cashout_kinds or [])
    families: list[str] = []
    if cid in RESOURCE_WINDOW_CARDS or "missed_draw_cashout" in kinds:
        families.append("resource_window")
    if "exhaust_payoff" in tags or "missed_exhaust_cashout" in kinds:
        families.append("exhaust_payoff")
    if "aoe" in tags or "missed_aoe_damage_cashout" in kinds:
        families.append("aoe_kill_timing")
    if "block" in tags or "missed_block_cashout" in kinds:
        families.append("block_survival")
    if "draw" in tags:
        families.append("draw_payoff")
    if "setup_or_scaling" in tags or "missed_scaling_cashout" in kinds:
        families.append("scaling_window")
    if "kill_window" in tags or "missed_kill_window_cashout" in kinds:
        families.append("kill_window")
    if "frontload" in tags or "missed_frontload_cashout" in kinds:
        families.append("frontload_lethal")
    return sorted(set(families)) or ["generic_cashout"]


def outcome_score(outcome: dict[str, Any]) -> float:
    delta = outcome.get("outcome_delta") or {}
    end = outcome.get("end") or {}
    reward = num(outcome.get("reward_total"))
    result_bonus = 0.0
    if end.get("result") == "victory":
        result_bonus += 500.0
    elif end.get("result") == "defeat":
        result_bonus -= 250.0
    return (
        num(delta.get("floor_delta")) * 100.0
        + num(delta.get("combat_win_delta")) * 40.0
        + num(end.get("current_hp")) * 1.5
        + reward * 2.0
        + result_bonus
    )


def axis_evidence(outcome: dict[str, Any]) -> dict[str, Any]:
    attr = outcome.get("attribution") or {}
    delta = outcome.get("outcome_delta") or {}
    end = outcome.get("end") or {}
    hp_loss = num(attr.get("hp_loss_observed"))
    return {
        "progress_score": round(
            num(delta.get("floor_delta")) * 100.0
            + num(delta.get("combat_win_delta")) * 40.0
            + num(attr.get("monster_hp_reduction_observed")) * 0.15
            + num(attr.get("alive_monster_reduction_observed")) * 20.0,
            3,
        ),
        "hp_preservation_score": round(num(end.get("current_hp")) - max(hp_loss, 0.0) * 0.15, 3),
        "frontload_or_aoe_score": round(
            num(attr.get("monster_hp_reduction_observed"))
            + num(attr.get("alive_monster_reduction_observed")) * 40.0,
            3,
        ),
        "draw_signal": bool(attr.get("draw_played")) or bool(attr.get("draw_cards_played")),
        "exhaust_signal": bool(attr.get("exhaust_played")) or bool(attr.get("exhaust_cards_played")),
        "scaling_signal": bool(attr.get("scaling_played")) or bool(attr.get("setup_or_scaling_cards_played")),
        "energy_waste": round(num(attr.get("energy_unused_on_end_turn_total")), 3),
        "combat_turns": round(num(attr.get("combat_turns_observed")), 3),
        "max_unblocked": round(num(attr.get("max_visible_unblocked_damage")), 3),
    }


def aggregate_values(values: list[float]) -> dict[str, Any]:
    if not values:
        return {"n": 0, "avg": 0.0, "min": 0.0, "max": 0.0}
    return {
        "n": len(values),
        "avg": round(sum(values) / len(values), 3),
        "min": round(min(values), 3),
        "max": round(max(values), 3),
    }


def read_case_report(path_text: str) -> dict[str, Any] | None:
    if not path_text:
        return None
    path = Path(path_text)
    real = resolve(path)
    if not real.exists():
        return None
    return read_json(real)


def rank_outcomes(outcomes: list[dict[str, Any]]) -> list[dict[str, Any]]:
    rows = []
    for outcome in outcomes:
        rows.append(
            {
                "candidate_key": outcome.get("candidate_key"),
                "card_id": card_id_from_outcome(outcome),
                "score": round(outcome_score(outcome), 3),
                "outcome_delta": outcome.get("outcome_delta") or {},
                "end": outcome.get("end") or {},
                "axis_evidence": axis_evidence(outcome),
            }
        )
    return sorted(rows, key=lambda row: -num(row.get("score")))


def build_candidate_summaries(reports: list[dict[str, Any]]) -> list[dict[str, Any]]:
    by_key: dict[str, dict[str, Any]] = {}
    for report in reports:
        policy = str((report.get("config") or {}).get("continuation_policy") or "unknown_policy")
        horizon = int((report.get("config") or {}).get("continuation_steps") or 0)
        ranked = rank_outcomes(list(report.get("outcomes") or []))
        rank_by_key = {str(row["candidate_key"]): index + 1 for index, row in enumerate(ranked)}
        for outcome in report.get("outcomes") or []:
            key = str(outcome.get("candidate_key") or "")
            card = outcome.get("candidate_card")
            entry = by_key.setdefault(
                key,
                {
                    "candidate_key": key,
                    "card_id": card_id_from_outcome(outcome),
                    "tags": card_tags(card),
                    "probe_families": probe_families(card),
                    "contexts": [],
                    "scores": [],
                    "ranks": [],
                    "axis_samples": [],
                },
            )
            entry["contexts"].append(f"{policy}@{horizon}")
            entry["scores"].append(outcome_score(outcome))
            entry["ranks"].append(rank_by_key.get(key))
            entry["axis_samples"].append(axis_evidence(outcome))

    summaries = []
    for entry in by_key.values():
        axis_samples = entry.pop("axis_samples")
        tags = set(entry.get("tags") or [])
        signals = Counter()
        for axis in axis_samples:
            if axis.get("draw_signal"):
                signals["draw"] += 1
            if axis.get("exhaust_signal"):
                signals["exhaust"] += 1
            if axis.get("scaling_signal"):
                signals["scaling"] += 1
        progress_values = [num(axis.get("progress_score")) for axis in axis_samples]
        hp_values = [num(axis.get("hp_preservation_score")) for axis in axis_samples]
        aoe_values = [num(axis.get("frontload_or_aoe_score")) for axis in axis_samples]
        summaries.append(
            {
                **entry,
                "score_summary": aggregate_values([num(value) for value in entry.pop("scores")]),
                "rank_summary": aggregate_values([num(value) for value in entry.pop("ranks") if value is not None]),
                "axis_summary": {
                    "progress_score": aggregate_values(progress_values),
                    "hp_preservation_score": aggregate_values(hp_values),
                    "frontload_or_aoe_score": aggregate_values(aoe_values),
                    "draw_signal_rate": round(signals["draw"] / max(len(axis_samples), 1), 3),
                    "exhaust_signal_rate": round(signals["exhaust"] / max(len(axis_samples), 1), 3),
                    "scaling_signal_rate": round(signals["scaling"] / max(len(axis_samples), 1), 3),
                },
                "cashout_realization_flags": cashout_realization_flags(tags, signals, axis_samples),
            }
        )
    return sorted(
        summaries,
        key=lambda row: (-num((row.get("score_summary") or {}).get("avg")), str(row.get("card_id"))),
    )


def cashout_realization_flags(tags: set[str], signals: Counter[str], axis_samples: list[dict[str, Any]]) -> list[str]:
    flags: list[str] = []
    sample_count = max(len(axis_samples), 1)
    if "draw" in tags and signals["draw"] == 0:
        flags.append("draw_not_realized_in_continuation")
    if "exhaust_payoff" in tags and signals["exhaust"] == 0:
        flags.append("exhaust_payoff_not_realized_in_continuation")
    if "setup_or_scaling" in tags and signals["scaling"] / sample_count < 0.5:
        flags.append("scaling_inconsistently_realized")
    progress_avg = sum(num(axis.get("progress_score")) for axis in axis_samples) / sample_count
    hp_avg = sum(num(axis.get("hp_preservation_score")) for axis in axis_samples) / sample_count
    if progress_avg >= 300:
        flags.append("strong_progress_realization")
    if hp_avg >= 25:
        flags.append("hp_preservation_realization")
    return flags


def build_case_probe(label: dict[str, Any]) -> dict[str, Any]:
    source = label.get("source_case") or {}
    status = str(label.get("label_status") or "unknown")
    bucket = CASE_BUCKETS.get(status, "other")
    reports: list[dict[str, Any]] = []
    observation_summaries = []
    for obs in label.get("observations") or []:
        report = read_case_report(str(obs.get("case_report_path") or ""))
        classification = obs.get("classification") or {}
        observation_summaries.append(
            {
                "context": f"{obs.get('continuation_policy')}@{obs.get('horizon')}",
                "status": obs.get("status"),
                "verdict": classification.get("verdict"),
                "reason": classification.get("reason"),
                "chosen_rank": classification.get("chosen_rank"),
                "cashout_best_rank": classification.get("cashout_best_rank"),
                "ranked_keys": classification.get("ranked_keys") or [],
            }
        )
        if report:
            reports.append(report)

    best = source.get("best_by_cashout") or {}
    chosen = source.get("chosen") or {}
    best_card = best.get("card_id")
    best_key = best.get("action_key")
    chosen_key = chosen.get("action_key")
    candidate_summaries = build_candidate_summaries(reports)
    best_candidate_summary = next(
        (row for row in candidate_summaries if str(row.get("candidate_key")) == str(best_key)),
        None,
    )
    diagnosis = case_diagnosis(bucket=bucket, best_summary=best_candidate_summary, observation_summaries=observation_summaries)
    families = probe_families(
        {"card_id": best_card} if best_card and best_card != "Skip" else None,
        list(source.get("cashout_kinds") or []),
    )
    return {
        "case_id": label.get("case_id"),
        "bucket": bucket,
        "label_status": status,
        "source_policy": label.get("source_policy"),
        "seed": source.get("seed"),
        "step_index": source.get("step_index"),
        "act": source.get("act"),
        "floor": source.get("floor"),
        "hp": source.get("hp"),
        "trace_file": source.get("trace_file"),
        "chosen": {
            "card_id": chosen.get("card_id"),
            "action_key": chosen_key,
            "cashout_score": chosen.get("cashout_score"),
        },
        "cashout_best": {
            "card_id": best_card,
            "action_key": best_key,
            "cashout_score": best.get("cashout_score"),
            "dominant_cashout": best.get("dominant_cashout"),
            "primary_class": best.get("primary_class"),
        },
        "cashout_gap": source.get("cashout_gap"),
        "cashout_kinds": source.get("cashout_kinds") or [],
        "probe_families": families,
        "observations": observation_summaries,
        "candidate_summaries": candidate_summaries,
        "diagnosis": diagnosis,
    }


def case_diagnosis(
    *,
    bucket: str,
    best_summary: dict[str, Any] | None,
    observation_summaries: list[dict[str, Any]],
) -> dict[str, Any]:
    verdicts = Counter(str(row.get("verdict") or row.get("status") or "unknown") for row in observation_summaries)
    contexts_confirmed = [row["context"] for row in observation_summaries if row.get("verdict") == "rollout_confirmed"]
    contexts_refuted = [row["context"] for row in observation_summaries if row.get("verdict") == "rollout_refuted"]
    flags = []
    if bucket == "policy_utilization_gap":
        flags.append("requires_better_continuation_policy")
    if bucket == "continuation_bad_or_unknown":
        flags.append("policy_or_horizon_conflict")
    if bucket == "true_static_fix_needed":
        flags.append("static_cashout_false_positive")
    if best_summary:
        flags.extend(best_summary.get("cashout_realization_flags") or [])
    return {
        "verdict_counts": dict(sorted(verdicts.items())),
        "confirmed_contexts": contexts_confirmed,
        "refuted_contexts": contexts_refuted,
        "flags": sorted(set(flags)),
        "recommended_next_step": recommended_next_step(bucket, best_summary),
    }


def recommended_next_step(bucket: str, best_summary: dict[str, Any] | None) -> str:
    flags = set((best_summary or {}).get("cashout_realization_flags") or [])
    families = set((best_summary or {}).get("probe_families") or [])
    if bucket == "true_static_fix_needed":
        return "inspect_static_cashout_semantics_before_training"
    if "draw_not_realized_in_continuation" in flags or "resource_window" in families:
        return "resource_window_micro_probe"
    if "exhaust_payoff_not_realized_in_continuation" in flags or "exhaust_payoff" in families:
        return "exhaust_payoff_micro_probe"
    if "aoe_kill_timing" in families:
        return "aoe_kill_timing_micro_probe"
    if "block_survival" in families:
        return "block_survival_micro_probe"
    if "scaling_inconsistently_realized" in flags or "scaling_window" in families:
        return "scaling_window_micro_probe"
    if bucket == "continuation_bad_or_unknown":
        return "paired_horizon_stability_probe"
    return "review_only"


def build_queue(case_probes: list[dict[str, Any]], bucket_filter: set[str] | None) -> list[dict[str, Any]]:
    rows = []
    for case in case_probes:
        bucket = str(case.get("bucket") or "")
        if bucket_filter is not None and bucket not in bucket_filter:
            continue
        rows.append(
            {
                "queue_version": "cashout_micro_probe_queue_v0",
                "case_id": case.get("case_id"),
                "bucket": bucket,
                "recommended_next_step": (case.get("diagnosis") or {}).get("recommended_next_step"),
                "probe_families": case.get("probe_families") or [],
                "source": {
                    "trace_file": case.get("trace_file"),
                    "seed": case.get("seed"),
                    "step_index": case.get("step_index"),
                    "act": case.get("act"),
                    "floor": case.get("floor"),
                    "hp": case.get("hp"),
                },
                "candidate_under_test": case.get("cashout_best"),
                "control_candidate": case.get("chosen"),
                "question": probe_question(case),
                "evidence_flags": (case.get("diagnosis") or {}).get("flags") or [],
                "contract": "queue for targeted micro-probe design; not a label",
            }
        )
    return rows


def probe_question(case: dict[str, Any]) -> str:
    best = (case.get("cashout_best") or {}).get("card_id") or "cashout_best"
    step = (case.get("diagnosis") or {}).get("recommended_next_step") or "review"
    if step == "resource_window_micro_probe":
        return f"Does {best} convert same-turn draw/energy into damage, block, or setup under realistic hand/energy bottlenecks?"
    if step == "exhaust_payoff_micro_probe":
        return f"Does {best} produce exhaust payoff before the relevant damage/block window, or is the outlet/fodder timing missing?"
    if step == "aoe_kill_timing_micro_probe":
        return f"Does {best} change multi-enemy kill timing and incoming damage, rather than only adding AoE-tagged damage?"
    if step == "block_survival_micro_probe":
        return f"Does {best} preserve enough HP without delaying required frontload or lethal progress?"
    if step == "scaling_window_micro_probe":
        return f"Does {best} come online before the fight/run is already lost?"
    if step == "paired_horizon_stability_probe":
        return f"Why does {best} flip between policies or horizons under the same trace prefix?"
    if step == "inspect_static_cashout_semantics_before_training":
        return f"Why did static cashout overrate {best} compared with paired continuation outcomes?"
    return f"Review whether {best} has real cashout under this continuation context."


def build_report(label_report: dict[str, Any], policy_report: dict[str, Any] | None, bucket_filter: set[str] | None) -> dict[str, Any]:
    labels = list(label_report.get("labels") or [])
    case_probes = [build_case_probe(label) for label in labels]
    queue = build_queue(case_probes, bucket_filter)
    bucket_counts = Counter(str(case.get("bucket") or "unknown") for case in case_probes)
    next_step_counts = Counter(str((case.get("diagnosis") or {}).get("recommended_next_step") or "unknown") for case in case_probes)
    flag_counts = Counter(
        flag
        for case in case_probes
        for flag in ((case.get("diagnosis") or {}).get("flags") or [])
    )
    return {
        "report_version": REPORT_VERSION,
        "source_report_version": label_report.get("report_version"),
        "policy_utilization_report_version": (policy_report or {}).get("report_version"),
        "summary": {
            "case_count": len(case_probes),
            "queue_count": len(queue),
            "bucket_counts": dict(sorted(bucket_counts.items())),
            "recommended_next_step_counts": dict(sorted(next_step_counts.items())),
            "diagnosis_flag_counts": dict(sorted(flag_counts.items())),
            "contract": "V0 reuses existing rollout case reports; it does not run new canonical encounters.",
        },
        "case_probes": case_probes,
        "micro_probe_queue": queue,
        "limitations": [
            "Existing full-run continuation reports are reused as micro-probe evidence.",
            "Attribution fields are before/after observation deltas, not exact engine event logs.",
            "Queue rows are questions for targeted probe design, not labels.",
            "V0 does not yet instantiate canonical encounters or resample future RNG.",
        ],
    }


def fmt(value: Any) -> str:
    if isinstance(value, float):
        return f"{value:.3f}"
    if value is None:
        return ""
    return str(value)


def write_markdown(path: Path, report: dict[str, Any], *, top_n: int) -> None:
    summary = report["summary"]
    lines = [
        "# Cashout Micro-Probe Lab V0",
        "",
        "This report turns rollout residuals into targeted continuation/micro-probe questions.",
        "It does not run new canonical encounters yet; it reuses existing paired continuation reports.",
        "",
        "## Summary",
        "",
        f"- source report: `{report['source_report_version']}`",
        f"- cases: `{summary['case_count']}`",
        f"- queue rows: `{summary['queue_count']}`",
        f"- bucket counts: `{summary['bucket_counts']}`",
        f"- recommended next steps: `{summary['recommended_next_step_counts']}`",
        f"- diagnosis flags: `{summary['diagnosis_flag_counts']}`",
        f"- contract: `{summary['contract']}`",
        "",
        "## Probe Queue",
        "",
        "| case | bucket | card | next step | families | question | flags |",
        "|---|---|---|---|---|---|---|",
    ]
    for row in report["micro_probe_queue"][:top_n]:
        lines.append(
            "| {case} | {bucket} | {card} | {step} | `{families}` | {question} | `{flags}` |".format(
                case=row.get("case_id"),
                bucket=row.get("bucket"),
                card=(row.get("candidate_under_test") or {}).get("card_id"),
                step=row.get("recommended_next_step"),
                families=row.get("probe_families"),
                question=row.get("question"),
                flags=row.get("evidence_flags"),
            )
        )
    lines.extend(["", "## Case Probes", ""])
    for case in report["case_probes"][:top_n]:
        diagnosis = case.get("diagnosis") or {}
        lines.extend(
            [
                f"### {case.get('case_id')}",
                "",
                "- bucket: `{bucket}`; status `{status}`; source `{source}`; act/floor/hp `{act}/{floor}/{hp}`".format(
                    bucket=case.get("bucket"),
                    status=case.get("label_status"),
                    source=case.get("source_policy"),
                    act=case.get("act"),
                    floor=case.get("floor"),
                    hp=case.get("hp"),
                ),
                "- chosen: `{chosen}`; cashout best: `{best}`; gap `{gap}`; kinds `{kinds}`".format(
                    chosen=(case.get("chosen") or {}).get("card_id"),
                    best=(case.get("cashout_best") or {}).get("card_id"),
                    gap=fmt(case.get("cashout_gap")),
                    kinds=case.get("cashout_kinds"),
                ),
                f"- diagnosis: `{diagnosis}`",
                "",
                "| candidate | tags | rank avg | score avg | progress avg | hp avg | AoE/frontload avg | flags | contexts |",
                "|---|---|---:|---:|---:|---:|---:|---|---|",
            ]
        )
        for candidate in (case.get("candidate_summaries") or [])[:8]:
            axis = candidate.get("axis_summary") or {}
            lines.append(
                "| {card} | `{tags}` | {rank} | {score} | {progress} | {hp} | {aoe} | `{flags}` | `{contexts}` |".format(
                    card=candidate.get("card_id"),
                    tags=candidate.get("tags"),
                    rank=fmt((candidate.get("rank_summary") or {}).get("avg")),
                    score=fmt((candidate.get("score_summary") or {}).get("avg")),
                    progress=fmt((axis.get("progress_score") or {}).get("avg")),
                    hp=fmt((axis.get("hp_preservation_score") or {}).get("avg")),
                    aoe=fmt((axis.get("frontload_or_aoe_score") or {}).get("avg")),
                    flags=candidate.get("cashout_realization_flags"),
                    contexts=candidate.get("contexts"),
                )
            )
        lines.append("")
    real = resolve(path)
    real.parent.mkdir(parents=True, exist_ok=True)
    real.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    label_dir = resolve(args.label_dir)
    label_report_path = args.label_report or label_dir / "cashout_rollout_label_report.json"
    policy_report_path = args.policy_utilization_report or label_dir / "policy_utilization_gap_analysis.json"
    out_path = resolve(args.out) if args.out else label_dir / "cashout_micro_probe_lab_report.json"
    markdown_path = resolve(args.markdown_out) if args.markdown_out else out_path.with_suffix(".md")
    queue_path = resolve(args.queue_out) if args.queue_out else label_dir / "cashout_micro_probe_queue.jsonl"
    label_report = read_json(label_report_path)
    policy_report = read_json(policy_report_path) if resolve(policy_report_path).exists() else None
    buckets = None if args.buckets.strip().lower() == "all" else set(parse_csv(args.buckets))
    report = build_report(label_report, policy_report, buckets)
    write_json(out_path, report)
    write_markdown(markdown_path, report, top_n=int(args.top_n))
    write_jsonl(queue_path, report["micro_probe_queue"])
    print(
        json.dumps(
            {
                "out": str(out_path),
                "markdown_out": str(markdown_path),
                "queue_out": str(queue_path),
                "summary": report["summary"],
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
