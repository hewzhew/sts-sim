#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path
from statistics import median
from typing import Any

from combat_rl_common import iter_jsonl, write_json, write_jsonl
from run_provenance import current_repo_provenance, provenance_for_source


SURVIVAL_RANK = {
    "forced_loss": 0,
    "severe_risk": 1,
    "risky_but_playable": 2,
    "stable": 3,
    "safe": 4,
}
ORACLE_PRIORITY_RANK = {"high": 0, "medium": 1, "low": 2, "none": 3}

PLAY_CARD_RE = re.compile(r"PlayCard \{ card_index: (?P<index>\d+), target: (?P<target>[^}]+) \}")


def default_sidecar(path: Path, suffix: str) -> Path:
    return path.with_name(f"{path.stem}{suffix}")


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    return [row for _, row in iter_jsonl(path)]


def safe_int(value: Any, default: int = 0) -> int:
    try:
        if value is None:
            return default
        return int(value)
    except (TypeError, ValueError):
        return default


def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        if value is None:
            return default
        return float(value)
    except (TypeError, ValueError):
        return default


def survival_rank(value: Any) -> int | None:
    text = str(value or "")
    return SURVIVAL_RANK.get(text)


def hp_bucket(current_hp: int, max_hp: int) -> str:
    if max_hp <= 0:
        return "unknown"
    ratio = current_hp / max_hp
    if ratio <= 0.25:
        return "hp_000_025"
    if ratio <= 0.50:
        return "hp_026_050"
    if ratio <= 0.75:
        return "hp_051_075"
    return "hp_076_100"


def card_cost(card: dict[str, Any]) -> int:
    if card.get("free_to_play_once"):
        return 0
    cost_for_turn = card.get("cost_for_turn")
    if cost_for_turn is not None:
        return safe_int(cost_for_turn)
    return safe_int(card.get("cost"))


def playable_card_count(hand: list[dict[str, Any]], energy: int) -> int:
    count = 0
    for card in hand:
        cost = card_cost(card)
        if cost == -2:
            continue
        if cost == -1:
            if energy > 0:
                count += 1
            continue
        if cost <= energy:
            count += 1
    return count


def unplayable_due_energy_count(hand: list[dict[str, Any]], energy: int) -> int:
    count = 0
    for card in hand:
        cost = card_cost(card)
        if cost <= 0:
            continue
        if cost > energy:
            count += 1
    return count


def hand_card_ids(row: dict[str, Any]) -> list[str]:
    snapshot = row.get("combat_snapshot") or {}
    zones = snapshot.get("zones") or {}
    hand = zones.get("hand") or []
    return [str(card.get("id") or "unknown") for card in hand]


def normalize_action_label(action: Any, row: dict[str, Any]) -> str:
    text = str(action or "")
    if text == "EndTurn":
        return "EndTurn"
    if text.startswith("UsePotion"):
        return "UsePotion"
    match = PLAY_CARD_RE.match(text)
    if match:
        index = safe_int(match.group("index"), -1)
        cards = hand_card_ids(row)
        card_id = cards[index] if 0 <= index < len(cards) else f"card_{index}"
        return f"Play {card_id}"
    if not text:
        return "None"
    return text.split(" ", 1)[0]


def action_class(action: Any) -> str:
    text = str(action or "")
    if text == "EndTurn":
        return "end_turn"
    if text.startswith("PlayCard"):
        return "play_card"
    if text.startswith("UsePotion"):
        return "use_potion"
    if text in {"", "None"}:
        return "none"
    return "other"


def decision_audit(row: dict[str, Any]) -> dict[str, Any]:
    return row.get("decision_audit") or {}


def exact_turn_verdict(row: dict[str, Any]) -> dict[str, Any]:
    return decision_audit(row).get("exact_turn_verdict") or {}


def exact_turn_shadow(row: dict[str, Any]) -> dict[str, Any]:
    return decision_audit(row).get("exact_turn_shadow") or {}


def frontier_outcome(row: dict[str, Any]) -> dict[str, Any]:
    return decision_audit(row).get("frontier_outcome") or {}


def decision_trace(row: dict[str, Any]) -> dict[str, Any]:
    return decision_audit(row).get("decision_trace") or {}


def root_pipeline(row: dict[str, Any]) -> dict[str, Any]:
    return decision_audit(row).get("root_pipeline") or {}


def extract_features(row: dict[str, Any]) -> dict[str, Any]:
    snapshot = row.get("combat_snapshot") or {}
    player = snapshot.get("player") or {}
    zones = snapshot.get("zones") or {}
    turn = snapshot.get("turn") or {}
    monsters = snapshot.get("monsters") or []
    hand = zones.get("hand") or []
    living_monsters = [
        monster
        for monster in monsters
        if safe_int(monster.get("current_hp")) > 0
        and not bool(monster.get("is_dying"))
        and not bool(monster.get("is_escaped"))
    ]
    current_hp = safe_int(player.get("current_hp"))
    max_hp = safe_int(player.get("max_hp"))
    energy = safe_int(turn.get("energy"))
    draw_count = safe_int(zones.get("draw_count"))
    discard_count = safe_int(zones.get("discard_count"))
    exhaust_count = safe_int(zones.get("exhaust_count"))
    hand_size = len(hand)
    deck_visible_size = hand_size + draw_count + discard_count + exhaust_count
    powers = player.get("powers") or []
    relics = player.get("relics") or []
    potions = [potion for potion in (player.get("potions") or []) if potion]
    exact = exact_turn_verdict(row)
    shadow = exact_turn_shadow(row)
    frontier = frontier_outcome(row)
    trace = decision_trace(row)
    exact_survival = exact.get("survival")
    frontier_survival = frontier.get("survival")
    exact_rank = survival_rank(exact_survival)
    frontier_rank = survival_rank(frontier_survival)
    ranks = [rank for rank in [exact_rank, frontier_rank] if rank is not None]
    min_survival_rank = min(ranks) if ranks else None
    survival_gap = (
        abs(exact_rank - frontier_rank)
        if exact_rank is not None and frontier_rank is not None
        else None
    )
    chosen_action = trace.get("chosen_action")
    exact_best = shadow.get("best_first_input")
    return {
        "run_id": row.get("run_id"),
        "sample_id": row.get("sample_id"),
        "source_kind": row.get("source_kind"),
        "source_path": row.get("source_path"),
        "frame_id": row.get("frame_id"),
        "response_id": row.get("response_id"),
        "encounter_signature": row.get("encounter_signature") or [],
        "encounter_key": f"{row.get('run_id')}::{','.join(row.get('encounter_signature') or [])}",
        "screen_type": row.get("screen_type"),
        "engine_state": row.get("engine_state"),
        "player_class": row.get("player_class"),
        "ascension_level": row.get("ascension_level"),
        "regime": row.get("regime"),
        "curriculum_buckets": row.get("curriculum_buckets") or [],
        "legal_moves": safe_int(row.get("legal_moves")),
        "reduced_legal_moves": safe_int(row.get("reduced_legal_moves")),
        "screened_out_count": safe_int(row.get("screened_out_count")),
        "needs_exact_trigger_target": bool(row.get("needs_exact_trigger_target")),
        "has_screening_activity_target": bool(row.get("has_screening_activity_target")),
        "timed_out": bool(row.get("timed_out")),
        "current_hp": current_hp,
        "max_hp": max_hp,
        "hp_bucket": hp_bucket(current_hp, max_hp),
        "block": safe_int(player.get("block")),
        "energy": energy,
        "turn_count": safe_int(turn.get("turn_count")),
        "cards_played_this_turn": safe_int(turn.get("cards_played_this_turn")),
        "attacks_played_this_turn": safe_int(turn.get("attacks_played_this_turn")),
        "hand_size": hand_size,
        "draw_count": draw_count,
        "discard_count": discard_count,
        "exhaust_count": exhaust_count,
        "deck_visible_size": deck_visible_size,
        "playable_cards": playable_card_count(hand, energy),
        "unplayable_due_energy": unplayable_due_energy_count(hand, energy),
        "hand_card_ids": hand_card_ids(row),
        "living_monsters": len(living_monsters),
        "enemy_total_hp": sum(safe_int(monster.get("current_hp")) for monster in living_monsters),
        "relic_count": len(relics),
        "potion_count": len(potions),
        "power_ids": [str(power.get("id") or "") for power in powers],
        "frontier_survival": frontier_survival,
        "frontier_survival_rank": frontier_rank,
        "frontier_terminality": frontier.get("terminality"),
        "exact_survival": exact_survival,
        "exact_survival_rank": exact_rank,
        "exact_confidence": exact.get("confidence"),
        "exact_dominance": exact.get("dominance"),
        "exact_lethal_window": bool(exact.get("lethal_window")),
        "exact_truncated": bool(exact.get("truncated")),
        "min_survival_rank": min_survival_rank,
        "exact_frontier_survival_gap": survival_gap,
        "exact_agrees_with_frontier": shadow.get("agrees_with_frontier"),
        "exact_best_first_input": exact_best,
        "exact_best_label": normalize_action_label(exact_best, row),
        "chosen_action": chosen_action,
        "chosen_label": normalize_action_label(chosen_action, row),
        "chosen_action_class": action_class(chosen_action),
        "exact_nodes": safe_int(shadow.get("explored_nodes")),
        "exact_elapsed_ms": safe_float(shadow.get("elapsed_ms")),
        "proposal_count": safe_int(root_pipeline(row).get("proposal_count")),
        "screened_count": safe_int(root_pipeline(row).get("screened_count")),
    }


def triage_record(row: dict[str, Any]) -> dict[str, Any]:
    features = extract_features(row)
    tags: list[str] = []
    reject_reasons: list[str] = []
    buckets = set(features["curriculum_buckets"])
    legal = int(features["legal_moves"])
    reduced_legal = int(features["reduced_legal_moves"])
    screen_type = str(features["screen_type"] or "")
    regime = str(features["regime"] or "")
    min_rank = features["min_survival_rank"]
    exact_rank = features["exact_survival_rank"]
    frontier_rank = features["frontier_survival_rank"]
    survival_gap = features["exact_frontier_survival_gap"]
    exact_confidence = str(features["exact_confidence"] or "")
    exact_dominance = str(features["exact_dominance"] or "")
    exact_agrees = features["exact_agrees_with_frontier"]

    if "elite" in buckets:
        tags.append("elite")
    if "boss" in buckets:
        tags.append("boss")
    if regime in {"crisis", "fragile"}:
        tags.append(f"regime_{regime}")
    if features["chosen_action_class"] == "end_turn":
        tags.append("baseline_end_turn")
    if exact_agrees is False:
        tags.append("frontier_exact_disagreement")
    if survival_gap is not None and survival_gap >= 2:
        tags.append("survival_rank_disagreement")
    if exact_dominance in {"strictly_better_in_window", "strictly_worse_in_window"}:
        tags.append(f"exact_{exact_dominance}")
    if features["has_screening_activity_target"] or features["screened_out_count"] > 0:
        tags.append("screening_activity")
    if features["playable_cards"] == 0:
        tags.append("no_playable_cards")
    if features["unplayable_due_energy"] > 0:
        tags.append("energy_tension")

    oracle_needed = False
    macro_backtrack_needed = False
    calibration_only = False
    priority = "none"

    if screen_type != "NONE":
        primary = "pending_choice_or_non_root_state"
        reject_reasons.append("screen_type_not_none")
    elif legal <= 1 or reduced_legal <= 1:
        primary = "trivial_forced_state"
        reject_reasons.append("single_candidate_after_reduction")
    elif exact_confidence == "unavailable" or features["exact_truncated"] or features["timed_out"]:
        primary = "oracle_unstable_state"
        calibration_only = True
        priority = "low"
        reject_reasons.append("exact_unavailable_or_truncated")
    elif exact_rank == 0 and frontier_rank == 0:
        primary = "already_lost_state"
        macro_backtrack_needed = True
        priority = "high"
        tags.append("macro_suspect")
    elif min_rank is not None and min_rank <= 1:
        primary = "tactical_survival_state"
        oracle_needed = True
        priority = "high"
    elif regime in {"crisis", "fragile"} or buckets & {"regime_crisis", "regime_fragile"}:
        primary = "tactical_survival_state"
        oracle_needed = True
        priority = "high" if ("elite" in buckets or "boss" in buckets) else "medium"
    elif (
        exact_agrees is False
        or exact_dominance in {"strictly_better_in_window", "strictly_worse_in_window"}
        or features["has_screening_activity_target"]
        or features["screened_out_count"] > 0
        or buckets & {"elite", "boss", "setup_window"}
    ):
        primary = "high_regret_combat_state"
        oracle_needed = True
        priority = "medium"
    else:
        primary = "trivial_or_low_signal_state"
        reject_reasons.append("low_signal_by_cheap_triage")

    if calibration_only and (
        regime in {"crisis", "fragile"}
        or buckets & {"elite", "boss", "regime_crisis", "regime_fragile"}
    ):
        tags.append("unstable_but_important")

    row_out = {
        "sample_id": features["sample_id"],
        "run_id": features["run_id"],
        "frame_id": features["frame_id"],
        "response_id": features["response_id"],
        "source_kind": features["source_kind"],
        "source_path": features["source_path"],
        "encounter_signature": features["encounter_signature"],
        "encounter_key": features["encounter_key"],
        "primary_bucket": primary,
        "triage_tags": sorted(set(tags)),
        "counterfactual_candidate": oracle_needed,
        "oracle_needed": oracle_needed,
        "oracle_priority": priority,
        "macro_backtrack_needed": macro_backtrack_needed,
        "calibration_only": calibration_only,
        "reject_reasons": reject_reasons,
        "features": features,
    }
    return row_out


def oracle_selection_score(row: dict[str, Any]) -> float:
    features = row["features"]
    tags = set(row.get("triage_tags") or [])
    score = 0.0
    if row.get("primary_bucket") == "tactical_survival_state":
        score += 10.0
    if row.get("primary_bucket") == "high_regret_combat_state":
        score += 4.0
    if "regime_crisis" in tags:
        score += 6.0
    if "regime_fragile" in tags:
        score += 3.0
    if "survival_rank_disagreement" in tags:
        score += 6.0
    if "frontier_exact_disagreement" in tags:
        score += 4.0
    if "exact_strictly_worse_in_window" in tags or "exact_strictly_better_in_window" in tags:
        score += 3.0
    if "baseline_end_turn" in tags:
        score += 2.0
    if "boss" in tags:
        score += 2.0
    if "elite" in tags:
        score += 1.5
    if "screening_activity" in tags:
        score += 1.0
    max_hp = safe_int(features.get("max_hp"))
    current_hp = safe_int(features.get("current_hp"))
    if max_hp > 0:
        score += max(0.0, 1.0 - current_hp / max_hp) * 4.0
    score += min(safe_int(features.get("reduced_legal_moves")), 8) * 0.15
    return round(score, 4)


def apply_oracle_selection(
    triage_rows: list[dict[str, Any]],
    *,
    min_priority: str,
    max_per_encounter: int,
) -> None:
    threshold_rank = ORACLE_PRIORITY_RANK[min_priority]
    candidate_rows: list[dict[str, Any]] = []
    for row in triage_rows:
        row["oracle_selection_threshold"] = min_priority
        row["oracle_encounter_cap"] = max_per_encounter
        row["oracle_selection_score"] = oracle_selection_score(row)
        row["oracle_needed"] = False
        row["oracle_suppressed_reason"] = None
        priority_rank = ORACLE_PRIORITY_RANK.get(str(row.get("oracle_priority")), 99)
        if bool(row["counterfactual_candidate"]) and priority_rank <= threshold_rank:
            candidate_rows.append(row)

    if max_per_encounter <= 0:
        for row in candidate_rows:
            row["oracle_needed"] = True
        return

    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in candidate_rows:
        grouped[str(row.get("encounter_key"))].append(row)
    for rows in grouped.values():
        rows.sort(
            key=lambda row: (
                -safe_float(row.get("oracle_selection_score")),
                safe_int(row.get("frame_id")),
            )
        )
        for index, row in enumerate(rows):
            if index < max_per_encounter:
                row["oracle_needed"] = True
            else:
                row["oracle_suppressed_reason"] = "encounter_cap"


def count_values(rows: list[dict[str, Any]], key: str) -> dict[str, int]:
    return dict(sorted(Counter(str(row.get(key)) for row in rows).items()))


def count_nested(rows: list[dict[str, Any]], key: str) -> dict[str, int]:
    counter: Counter[str] = Counter()
    for row in rows:
        values = row.get(key) or []
        if not values:
            counter["<none>"] += 1
        for value in values:
            counter[str(value)] += 1
    return dict(counter.most_common())


def summarize_numeric(values: list[int | float]) -> dict[str, float | int | None]:
    if not values:
        return {"min": None, "median": None, "max": None}
    return {
        "min": min(values),
        "median": median(values),
        "max": max(values),
    }


def summarize_triage(triage_rows: list[dict[str, Any]], input_path: Path) -> dict[str, Any]:
    features = [row["features"] for row in triage_rows]
    encounter_groups: dict[str, int] = defaultdict(int)
    for row in triage_rows:
        encounter_groups[str(row.get("encounter_key"))] += 1
    group_sizes = list(encounter_groups.values())
    oracle_rows = [row for row in triage_rows if row["oracle_needed"]]
    counterfactual_rows = [row for row in triage_rows if row["counterfactual_candidate"]]
    macro_rows = [row for row in triage_rows if row["macro_backtrack_needed"]]
    calibration_rows = [row for row in triage_rows if row["calibration_only"]]
    oracle_suppressed = [row for row in triage_rows if row.get("oracle_suppressed_reason")]
    stale_rows = [
        row
        for row in triage_rows
        if not bool((row.get("provenance_freshness") or {}).get("fresh_for_current_head"))
    ]
    run_provenance = {}
    for row in triage_rows:
        run_id = str(row.get("run_id") or "unknown")
        run_provenance.setdefault(
            run_id,
            {
                "run": row.get("run_provenance"),
                "freshness": row.get("provenance_freshness"),
            },
        )
    return {
        "input": str(input_path),
        "oracle_selection": "oracle_needed uses the configured priority threshold; counterfactual_candidate is the broad cheap-triage set",
        "current_repo_provenance": triage_rows[0].get("current_repo_provenance") if triage_rows else {},
        "current_policy_conclusion_allowed": len(stale_rows) == 0,
        "stale_state_count": len(stale_rows),
        "stale_state_rate": len(stale_rows) / len(triage_rows) if triage_rows else 0.0,
        "run_provenance": run_provenance,
        "evidence_scope_counts": dict(
            Counter(str((row.get("provenance_freshness") or {}).get("evidence_scope")) for row in triage_rows).most_common()
        ),
        "stale_reason_counts": dict(
            Counter(
                reason
                for row in stale_rows
                for reason in ((row.get("provenance_freshness") or {}).get("stale_reasons") or [])
            ).most_common()
        ),
        "raw_states": len(triage_rows),
        "counterfactual_candidate_states": len(counterfactual_rows),
        "oracle_needed_states": len(oracle_rows),
        "oracle_selected_encounter_groups": len({str(row.get("encounter_key")) for row in oracle_rows}),
        "oracle_suppressed_by_encounter_cap": len(oracle_suppressed),
        "macro_backtrack_states": len(macro_rows),
        "calibration_only_states": len(calibration_rows),
        "counterfactual_candidate_rate": len(counterfactual_rows) / len(triage_rows) if triage_rows else 0.0,
        "oracle_needed_rate": len(oracle_rows) / len(triage_rows) if triage_rows else 0.0,
        "macro_backtrack_rate": len(macro_rows) / len(triage_rows) if triage_rows else 0.0,
        "effective_encounter_groups": len(encounter_groups),
        "near_duplicate_state_count": max(len(triage_rows) - len(encounter_groups), 0),
        "states_per_encounter_group": summarize_numeric(group_sizes),
        "primary_bucket_counts": count_values(triage_rows, "primary_bucket"),
        "oracle_priority_counts": count_values(triage_rows, "oracle_priority"),
        "oracle_suppressed_reason_counts": count_values(oracle_suppressed, "oracle_suppressed_reason"),
        "triage_tag_counts": count_nested(triage_rows, "triage_tags"),
        "reject_reason_counts": count_nested(triage_rows, "reject_reasons"),
        "run_id_counts": count_values(features, "run_id"),
        "source_kind_counts": count_values(features, "source_kind"),
        "regime_counts": count_values(features, "regime"),
        "screen_type_counts": count_values(features, "screen_type"),
        "curriculum_bucket_counts": count_nested(features, "curriculum_buckets"),
        "hp_bucket_counts": count_values(features, "hp_bucket"),
        "chosen_action_class_counts": count_values(features, "chosen_action_class"),
        "chosen_label_top": dict(Counter(str(row["features"]["chosen_label"]) for row in triage_rows).most_common(30)),
        "exact_best_label_top": dict(Counter(str(row["features"]["exact_best_label"]) for row in triage_rows).most_common(30)),
        "legal_moves": summarize_numeric([int(row["features"]["legal_moves"]) for row in triage_rows]),
        "reduced_legal_moves": summarize_numeric([int(row["features"]["reduced_legal_moves"]) for row in triage_rows]),
        "hand_size": summarize_numeric([int(row["features"]["hand_size"]) for row in triage_rows]),
        "deck_visible_size": summarize_numeric([int(row["features"]["deck_visible_size"]) for row in triage_rows]),
        "current_hp": summarize_numeric([int(row["features"]["current_hp"]) for row in triage_rows]),
        "notes": [
            "live states are a candidate pool, not training labels",
            "oracle_needed states are selected by cheap tactical/provenance signals only",
            "already_lost states should go to macro provenance before combat-policy training",
            "near_duplicate_state_count is raw states minus run+encounter groups, so it is a correlation warning not deduplication",
        ],
    }


def write_review(path: Path, summary: dict[str, Any], triage_rows: list[dict[str, Any]]) -> None:
    lines: list[str] = []
    lines.append("# State Corpus Triage")
    lines.append("")
    if not summary.get("current_policy_conclusion_allowed", False):
        lines.append("**STALE: this corpus is historical replay evidence, not current-policy evidence.**")
        lines.append("")
        lines.append(f"- stale_state_count: {summary.get('stale_state_count', 0)}")
        lines.append(f"- evidence_scope_counts: {json.dumps(summary.get('evidence_scope_counts') or {}, ensure_ascii=False)}")
        lines.append(f"- stale_reason_counts: {json.dumps(summary.get('stale_reason_counts') or {}, ensure_ascii=False)}")
        lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append("| metric | value |")
    lines.append("|---|---:|")
    for key in [
        "raw_states",
        "counterfactual_candidate_states",
        "oracle_needed_states",
        "macro_backtrack_states",
        "calibration_only_states",
        "effective_encounter_groups",
        "near_duplicate_state_count",
    ]:
        lines.append(f"| {key} | {summary.get(key)} |")
    lines.append("")
    lines.append("## Primary Buckets")
    lines.append("")
    lines.append("| bucket | count |")
    lines.append("|---|---:|")
    for key, value in (summary.get("primary_bucket_counts") or {}).items():
        lines.append(f"| {key} | {value} |")
    lines.append("")
    lines.append("## Oracle Priority")
    lines.append("")
    lines.append("| priority | count |")
    lines.append("|---|---:|")
    for key, value in (summary.get("oracle_priority_counts") or {}).items():
        lines.append(f"| {key} | {value} |")
    lines.append("")
    lines.append("## Top Oracle Candidates")
    lines.append("")
    lines.append("| priority | bucket | run | frame | encounter | hp | regime | legal | chosen | exact best | tags |")
    lines.append("|---|---|---|---:|---|---:|---|---:|---|---|---|")
    oracle_rows = [row for row in triage_rows if row["oracle_needed"]]
    priority_rank = {"high": 0, "medium": 1, "low": 2, "none": 3}
    oracle_rows.sort(
        key=lambda row: (
            priority_rank.get(str(row.get("oracle_priority")), 9),
            str(row.get("run_id")),
            safe_int(row.get("frame_id")),
        )
    )
    for row in oracle_rows[:60]:
        features = row["features"]
        encounter = ",".join(features["encounter_signature"])
        tags = ",".join(row["triage_tags"])
        lines.append(
            f"| {row['oracle_priority']} | {row['primary_bucket']} | {row['run_id']} | {row['frame_id']} | "
            f"{encounter} | {features['current_hp']} | {features['regime']} | {features['reduced_legal_moves']} | "
            f"{features['chosen_label']} | {features['exact_best_label']} | {tags} |"
        )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Triage run-built combat state corpus rows before expensive counterfactual labeling."
    )
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--out", default=None, type=Path)
    parser.add_argument("--summary-out", default=None, type=Path)
    parser.add_argument("--review-out", default=None, type=Path)
    parser.add_argument("--oracle-out", default=None, type=Path)
    parser.add_argument("--counterfactual-candidate-out", default=None, type=Path)
    parser.add_argument("--macro-out", default=None, type=Path)
    parser.add_argument("--calibration-out", default=None, type=Path)
    parser.add_argument("--reject-out", default=None, type=Path)
    parser.add_argument(
        "--oracle-min-priority",
        choices=["high", "medium", "low"],
        default="high",
        help="Minimum cheap-triage priority to write to oracle-needed outputs. Default keeps only high-priority states.",
    )
    parser.add_argument(
        "--max-oracle-per-encounter",
        default=4,
        type=int,
        help="Maximum oracle-needed rows to keep per run+encounter group. Use 0 to disable capping.",
    )
    args = parser.parse_args()

    source_rows = load_jsonl(args.input)
    triage_rows = [triage_record(row) for row in source_rows]
    current_provenance = current_repo_provenance()
    provenance_cache: dict[str, dict[str, Any]] = {}
    for row in triage_rows:
        source_path = str(row.get("source_path") or "")
        provenance_cache.setdefault(source_path, provenance_for_source(source_path, current_provenance))
        provenance = provenance_cache[source_path]
        row["current_repo_provenance"] = provenance["current"]
        row["run_provenance"] = provenance["run"]
        row["provenance_freshness"] = provenance["freshness"]
    apply_oracle_selection(
        triage_rows,
        min_priority=args.oracle_min_priority,
        max_per_encounter=int(args.max_oracle_per_encounter),
    )
    summary = summarize_triage(triage_rows, args.input)
    summary["oracle_min_priority"] = args.oracle_min_priority
    summary["max_oracle_per_encounter"] = int(args.max_oracle_per_encounter)

    out = args.out or default_sidecar(args.input, ".triage.jsonl")
    summary_out = args.summary_out or default_sidecar(args.input, ".triage.summary.json")
    review_out = args.review_out or default_sidecar(args.input, ".triage.md")
    oracle_out = args.oracle_out or default_sidecar(args.input, ".oracle_needed.jsonl")
    counterfactual_candidate_out = args.counterfactual_candidate_out or default_sidecar(
        args.input, ".counterfactual_candidates.jsonl"
    )
    macro_out = args.macro_out or default_sidecar(args.input, ".macro_backtrack.jsonl")
    calibration_out = args.calibration_out or default_sidecar(args.input, ".calibration_only.jsonl")
    reject_out = args.reject_out or default_sidecar(args.input, ".rejected_or_background.jsonl")

    oracle_rows = [row for row in triage_rows if row["oracle_needed"]]
    counterfactual_candidate_rows = [row for row in triage_rows if row["counterfactual_candidate"]]
    macro_rows = [row for row in triage_rows if row["macro_backtrack_needed"]]
    calibration_rows = [row for row in triage_rows if row["calibration_only"]]
    reject_rows = [
        row
        for row in triage_rows
        if not row["oracle_needed"] and not row["macro_backtrack_needed"] and not row["calibration_only"]
    ]

    write_jsonl(out, triage_rows)
    write_jsonl(oracle_out, oracle_rows)
    write_jsonl(counterfactual_candidate_out, counterfactual_candidate_rows)
    write_jsonl(macro_out, macro_rows)
    write_jsonl(calibration_out, calibration_rows)
    write_jsonl(reject_out, reject_rows)
    write_json(summary_out, summary)
    write_review(review_out, summary, triage_rows)
    print(json.dumps(summary, indent=2, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
