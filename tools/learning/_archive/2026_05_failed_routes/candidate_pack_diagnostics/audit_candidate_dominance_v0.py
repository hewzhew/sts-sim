#!/usr/bin/env python3
"""Audit conservative one-step candidate dominance against the full-H verifier.

This is an audit only.  It does not change the runtime policy.  The goal is to
measure whether engine-derived one-step deltas can safely prune obvious
candidate noise before an H-step verifier.
"""
from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

from return_q_common import FullRunDriver, legal_candidate_indices, stable_group_split, write_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--pairs-out", type=Path)
    parser.add_argument("--bad-prunes-out", type=Path)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--episodes", type=int, default=20)
    parser.add_argument("--seed-start", type=int, default=98500)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=160)
    parser.add_argument("--max-groups", type=int, default=0)
    parser.add_argument("--candidate-scope", default="controlled_v1", choices=["all", "controlled_v0", "controlled_v1"])
    parser.add_argument("--horizon-decisions", type=int, default=8)
    parser.add_argument("--horizon-mode", default="fixed_decisions")
    parser.add_argument("--oracle-margin", type=float, default=1.0)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--continuation-policy", default="rule_baseline_v0")
    parser.add_argument("--parallelism", type=int, default=0)
    parser.add_argument("--max-pairs-out", type=int, default=5000)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    summary = {
        "schema_version": "candidate_dominance_audit_v0",
        "config": serializable_config(args),
        "episodes_started": 0,
        "groups": 0,
        "skipped_groups": 0,
        "skip_reasons": {},
        "full_candidate_evaluation_count": 0,
        "one_step_candidate_evaluation_count": 0,
        "full_policy_step_eval_count": 0,
        "one_step_policy_step_eval_count": 0,
        "full_override_count": 0,
        "positive_candidate_count": 0,
        "decision_type_counts": {},
        "modes": {
            "strict": new_mode_stats("strict"),
            "relaxed": new_mode_stats("relaxed"),
        },
    }
    counters = {
        "skip_reasons": Counter(),
        "decision_type_counts": Counter(),
    }
    pair_rows: list[dict[str, Any]] = []
    bad_prune_rows: list[dict[str, Any]] = []

    driver = FullRunDriver(args.binary)
    try:
        for episode_index in range(args.episodes):
            if args.max_groups and int(summary["groups"]) >= args.max_groups:
                break
            seed = args.seed_start + episode_index * args.seed_step
            summary["episodes_started"] += 1
            collect_episode(args, driver, seed, summary, counters, pair_rows, bad_prune_rows)
    finally:
        driver.close()

    for key, counter in counters.items():
        summary[key] = dict(sorted(counter.items()))
    for mode_stats in summary["modes"].values():
        finalize_mode_stats(mode_stats)
    if args.pairs_out:
        write_jsonl(args.pairs_out, pair_rows[: args.max_pairs_out])
    if args.bad_prunes_out:
        write_jsonl(args.bad_prunes_out, bad_prune_rows)
    write_json(args.out, summary)
    print(json.dumps(render_compact(summary), indent=2, sort_keys=True))


def collect_episode(
    args: argparse.Namespace,
    driver: FullRunDriver,
    seed: int,
    summary: dict[str, Any],
    counters: dict[str, Counter[str]],
    pair_rows: list[dict[str, Any]],
    bad_prune_rows: list[dict[str, Any]],
) -> None:
    response = driver.request(
        {
            "cmd": "reset",
            "seed": seed,
            "ascension": args.ascension,
            "final_act": args.final_act,
            "class": args.player_class,
            "max_steps": args.max_steps,
            "reward_shaping_profile": "baseline",
        }
    )
    done = bool(response.get("done"))
    step = 0
    while not done and step < args.max_steps:
        if args.max_groups and int(summary["groups"]) >= args.max_groups:
            return
        payload = response.get("payload") or {}
        observation = payload.get("observation") or {}
        decision_type = str(observation.get("decision_type") or "")
        if decision_type.startswith("combat"):
            selected = audit_decision(
                args,
                driver,
                seed,
                step,
                response,
                summary,
                counters,
                pair_rows,
                bad_prune_rows,
            )
            if selected is None:
                response = driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
            else:
                response = driver.request({"cmd": "step", "action_index": selected})
        else:
            response = driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
        done = bool(response.get("done"))
        step += 1


def audit_decision(
    args: argparse.Namespace,
    driver: FullRunDriver,
    seed: int,
    step: int,
    response: dict[str, Any],
    summary: dict[str, Any],
    counters: dict[str, Counter[str]],
    pair_rows: list[dict[str, Any]],
    bad_prune_rows: list[dict[str, Any]],
) -> int | None:
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    candidates = payload.get("action_candidates") or []
    decision_type = str(observation.get("decision_type") or "")
    counters["decision_type_counts"][decision_type] += 1

    scoped = legal_candidate_indices(response, args.candidate_scope)
    if not scoped:
        record_skip(summary, counters, "no_scoped_candidates")
        return None
    rule_index = preview_rule_index(driver)
    legal_all = set(legal_candidate_indices(response, "all"))
    if rule_index is None or rule_index not in legal_all:
        record_skip(summary, counters, "missing_rule_action")
        return scoped[0]
    if not any(index != rule_index for index in scoped):
        record_skip(summary, counters, "only_rule_candidate")
        return rule_index

    eval_indices = sorted({rule_index, *scoped})
    full_payload = evaluate_indices(
        args,
        driver,
        eval_indices,
        args.horizon_decisions,
        args.horizon_mode,
        include_next_state=False,
    )
    one_step_payload = evaluate_indices(
        args,
        driver,
        eval_indices,
        0,
        "fixed_decisions",
        include_next_state=True,
    )
    full_by_index = evaluation_by_index(full_payload)
    step_by_index = evaluation_by_index(one_step_payload)
    rule_full = full_by_index.get(rule_index)
    if not rule_full:
        record_skip(summary, counters, "missing_rule_full_evaluation")
        return rule_index
    scoped_evaluated = [idx for idx in scoped if idx in full_by_index and idx in step_by_index]
    if not scoped_evaluated:
        record_skip(summary, counters, "missing_scoped_evaluations")
        return rule_index

    summary["groups"] += 1
    summary["full_candidate_evaluation_count"] += len(full_by_index)
    summary["one_step_candidate_evaluation_count"] += len(step_by_index)
    summary["full_policy_step_eval_count"] += int(full_payload.get("policy_step_eval_count") or 0)
    summary["one_step_policy_step_eval_count"] += int(one_step_payload.get("policy_step_eval_count") or 0)

    rule_return = float(rule_full.get("discounted_return") or 0.0)
    best_index = max(scoped_evaluated, key=lambda idx: float(full_by_index[idx].get("discounted_return") or 0.0))
    best_return = float(full_by_index[best_index].get("discounted_return") or 0.0)
    selected_index = best_index if best_index != rule_index and best_return - rule_return > args.oracle_margin else rule_index
    positives = {
        idx
        for idx in scoped_evaluated
        if idx != rule_index and float(full_by_index[idx].get("discounted_return") or 0.0) - rule_return > args.oracle_margin
    }
    if selected_index != rule_index:
        summary["full_override_count"] += 1
    summary["positive_candidate_count"] += len(positives)

    group_key = f"dominance|seed:{seed}|step:{step}|decision:{decision_type}"
    split = stable_group_split(group_key)
    facts = {
        idx: candidate_fact(observation, candidates[idx], step_by_index[idx])
        for idx in scoped_evaluated
        if idx < len(candidates)
    }
    for mode in ["strict", "relaxed"]:
        dominated_by = find_dominated_candidates(facts, rule_index, mode)
        stats = summary["modes"][mode]
        update_mode_stats(
            stats,
            group_key,
            split,
            seed,
            step,
            decision_type,
            candidates,
            facts,
            dominated_by,
            selected_index,
            positives,
            full_by_index,
            pair_rows,
            bad_prune_rows,
        )
    return selected_index


def candidate_fact(
    before_observation: dict[str, Any],
    candidate: dict[str, Any],
    evaluation: dict[str, Any],
) -> dict[str, Any]:
    next_state = evaluation.get("next_state") or {}
    next_observation = next_state.get("observation") or {}
    before_combat = before_observation.get("combat") or {}
    after_combat = next_observation.get("combat") or {}
    return {
        "candidate": candidate,
        "evaluation": evaluation,
        "ok": bool(evaluation.get("ok")),
        "action_key": str(candidate.get("action_key") or ""),
        "card": candidate.get("card") or {},
        "before_decision_type": before_observation.get("decision_type"),
        "after_decision_type": next_observation.get("decision_type"),
        "after_engine_state": next_observation.get("engine_state"),
        "done": bool(evaluation.get("done")),
        "before": summarize_combat(before_observation, before_combat),
        "after": summarize_combat(next_observation, after_combat),
        "target": extract_segment(str(candidate.get("action_key") or ""), "target"),
        "card_id": ((candidate.get("card") or {}).get("card_id") or card_from_key(str(candidate.get("action_key") or ""))),
        "is_play_card": str(candidate.get("action_key") or "").startswith("combat/play_card"),
        "is_end_turn": str(candidate.get("action_key") or "").startswith("combat/end_turn"),
        "opens_pending": str(next_observation.get("decision_type") or "").startswith("combat_"),
    }


def summarize_combat(observation: dict[str, Any], combat: dict[str, Any]) -> dict[str, int | str | None]:
    return {
        "decision_type": observation.get("decision_type"),
        "current_hp": as_int(observation.get("current_hp")),
        "energy": as_int(combat.get("energy")),
        "player_block": as_int(combat.get("player_block")),
        "visible_incoming_damage": as_int(combat.get("visible_incoming_damage")),
        "total_monster_hp": as_int(combat.get("total_monster_hp")),
        "alive_monster_count": as_int(combat.get("alive_monster_count")),
        "hand_count": as_int(combat.get("hand_count")),
        "draw_count": as_int(combat.get("draw_count")),
        "discard_count": as_int(combat.get("discard_count")),
        "exhaust_count": as_int(combat.get("exhaust_count")),
    }


def find_dominated_candidates(
    facts: dict[int, dict[str, Any]],
    rule_index: int,
    mode: str,
) -> dict[int, dict[str, Any]]:
    dominated: dict[int, dict[str, Any]] = {}
    for loser_idx, loser in facts.items():
        if loser_idx == rule_index:
            continue
        for winner_idx, winner in facts.items():
            if winner_idx == loser_idx:
                continue
            ok, reasons = dominates(winner, loser, mode)
            if ok:
                dominated[loser_idx] = {
                    "dominated_by": winner_idx,
                    "mode": mode,
                    "reasons": reasons,
                }
                break
    return dominated


def dominates(winner: dict[str, Any], loser: dict[str, Any], mode: str) -> tuple[bool, list[str]]:
    if not winner["ok"] or not loser["ok"]:
        return False, ["not_ok"]
    if winner["done"] or loser["done"]:
        return False, ["terminal_skipped"]
    if winner["after_decision_type"] != "combat" or loser["after_decision_type"] != "combat":
        return False, ["pending_or_noncombat_skipped"]
    if winner["before_decision_type"] != loser["before_decision_type"]:
        return False, ["different_before_decision"]
    if not comparable_scope(winner, loser):
        return False, ["not_same_scope"]
    if card_has_protected_long_horizon_value(loser):
        return False, ["protected_loser"]
    if mode == "strict" and (card_has_complex_effect(winner) or card_has_complex_effect(loser)):
        return False, ["strict_complex_card_skipped"]
    for key in ["draw_count", "discard_count", "exhaust_count"]:
        if winner["after"].get(key) != loser["after"].get(key):
            return False, [f"{key}_changed"]

    wa = winner["after"]
    la = loser["after"]
    checks = [
        ("current_hp", ">="),
        ("energy", ">="),
        ("player_block", ">="),
        ("visible_incoming_damage", "<="),
        ("total_monster_hp", "<="),
        ("alive_monster_count", "<="),
    ]
    checks.append(("hand_count", ">="))
    better = []
    for key, op in checks:
        wv = wa.get(key)
        lv = la.get(key)
        if wv is None or lv is None:
            return False, [f"missing_{key}"]
        if op == ">=" and int(wv) < int(lv):
            return False, [f"{key}_worse"]
        if op == "<=" and int(wv) > int(lv):
            return False, [f"{key}_worse"]
        if int(wv) != int(lv):
            better.append(key)
    if not better:
        return False, ["no_strict_improvement"]
    if mode == "relaxed":
        # Extra guard: do not claim dominance when the winner draws into a full
        # or overflowing hand and the loser does not.  This catches a common
        # draw-burn risk without inspecting full card order.
        before_hand = int(winner["before"].get("hand_count") or 0)
        winner_hand_gain = int(wa.get("hand_count") or 0) - before_hand
        loser_hand_gain = int(la.get("hand_count") or 0) - before_hand
        if winner_hand_gain > loser_hand_gain and before_hand >= 9:
            return False, ["hand_overflow_risk"]
    return True, better


def comparable_scope(winner: dict[str, Any], loser: dict[str, Any]) -> bool:
    if winner["is_end_turn"] or loser["is_end_turn"]:
        return winner["is_end_turn"] and loser["is_end_turn"]
    if not winner["is_play_card"] or not loser["is_play_card"]:
        return False
    winner_target = winner.get("target") or "none"
    loser_target = loser.get("target") or "none"
    if winner_target != loser_target:
        return False
    if winner_target == "none":
        return True
    return True


def card_has_complex_effect(fact: dict[str, Any]) -> bool:
    card = fact.get("card") or {}
    return any(
        bool(card.get(key))
        for key in [
            "draws_cards",
            "gains_energy",
            "exhaust",
            "ethereal",
            "applies_weak",
            "applies_vulnerable",
            "scaling_piece",
        ]
    )


def card_has_protected_long_horizon_value(fact: dict[str, Any]) -> bool:
    card = fact.get("card") or {}
    card_id = str(card.get("card_id") or fact.get("card_id") or "")
    # Powers, debuffs, draw/energy/exhaust and special-growth attacks have
    # value that one-step block/damage summaries cannot certify away.
    if int(card.get("card_type_id") or 0) == 3:
        return True
    if card_id in {
        "BodySlam",
        "Rampage",
        "Headbutt",
        "SearingBlow",
        "PerfectedStrike",
        "Feed",
        "HandOfGreed",
        "RitualDagger",
    }:
        return True
    return card_has_complex_effect(fact)


def update_mode_stats(
    stats: dict[str, Any],
    group_key: str,
    split: str,
    seed: int,
    step: int,
    decision_type: str,
    candidates: list[dict[str, Any]],
    facts: dict[int, dict[str, Any]],
    dominated_by: dict[int, dict[str, Any]],
    selected_index: int,
    positives: set[int],
    full_by_index: dict[int, dict[str, Any]],
    pair_rows: list[dict[str, Any]],
    bad_prune_rows: list[dict[str, Any]],
) -> None:
    non_rule_count = sum(1 for idx in facts if idx not in dominated_by)
    pruned = set(dominated_by)
    stats["group_count"] += 1
    stats["candidate_count"] += len(facts)
    stats["pruned_candidate_count"] += len(pruned)
    if pruned:
        stats["pruned_group_count"] += 1
    if selected_index in pruned:
        stats["full_selected_pruned_count"] += 1
    stats["positive_candidate_count"] += len(positives)
    stats["positive_pruned_count"] += len(positives & pruned)
    for idx in pruned:
        candidate = candidates[idx] if idx < len(candidates) else {}
        card = (candidate.get("card") or {}).get("card_id") or card_from_key(str(candidate.get("action_key") or ""))
        stats["pruned_card_counts"][card or "unknown"] += 1
        if idx == selected_index or idx in positives:
            row = {
                "mode": stats["mode"],
                "group_key": group_key,
                "split": split,
                "seed": seed,
                "step": step,
                "decision_type": decision_type,
                "candidate_index": idx,
                "candidate_action_key": candidate.get("action_key"),
                "dominated_by": dominated_by[idx]["dominated_by"],
                "dominated_by_action_key": (candidates[dominated_by[idx]["dominated_by"]] or {}).get("action_key")
                if dominated_by[idx]["dominated_by"] < len(candidates)
                else None,
                "reasons": dominated_by[idx]["reasons"],
                "is_full_selected": idx == selected_index,
                "is_positive": idx in positives,
                "adv_vs_rule": full_adv(idx, full_by_index, selected_index),
            }
            bad_prune_rows.append(row)
    for loser_idx, detail in dominated_by.items():
        winner_idx = int(detail["dominated_by"])
        if loser_idx >= len(candidates) or winner_idx >= len(candidates):
            continue
        pair_rows.append(
            {
                "mode": stats["mode"],
                "group_key": group_key,
                "split": split,
                "seed": seed,
                "step": step,
                "decision_type": decision_type,
                "loser_index": loser_idx,
                "winner_index": winner_idx,
                "loser_action_key": candidates[loser_idx].get("action_key"),
                "winner_action_key": candidates[winner_idx].get("action_key"),
                "loser_card": (candidates[loser_idx].get("card") or {}).get("card_id"),
                "winner_card": (candidates[winner_idx].get("card") or {}).get("card_id"),
                "reasons": detail["reasons"],
                "loser_is_full_selected": loser_idx == selected_index,
                "loser_is_positive": loser_idx in positives,
            }
        )
    # Keep the local variable referenced so audits can inspect future changes
    # without changing output schema.
    _ = non_rule_count


def full_adv(idx: int, full_by_index: dict[int, dict[str, Any]], selected_index: int) -> float | None:
    # Only used for debugging bad prunes.  If the selected row is not the rule,
    # this is still useful as an absolute H-return signal.
    row = full_by_index.get(idx)
    return float(row.get("discounted_return") or 0.0) if row else None


def evaluate_indices(
    args: argparse.Namespace,
    driver: FullRunDriver,
    indices: list[int],
    horizon: int,
    horizon_mode: str,
    include_next_state: bool,
) -> dict[str, Any]:
    return driver.request(
        {
            "cmd": "evaluate_candidates",
            "action_indices": indices,
            "continuation_policy": args.continuation_policy,
            "horizon_decisions": horizon,
            "horizon_mode": horizon_mode,
            "gamma": args.gamma,
            "evaluation_mode": "independent",
            "parallelism": args.parallelism,
            "exact_root_dedup": False,
            "include_state": False,
            "include_next_state": include_next_state,
            "include_continuation_trace": False,
            "check_live_env_unchanged": False,
        }
    ).get("payload") or {}


def evaluation_by_index(payload: dict[str, Any]) -> dict[int, dict[str, Any]]:
    return {
        int(item.get("action_index")): item
        for item in payload.get("evaluations") or []
        if item.get("ok") and item.get("action_index") is not None
    }


def preview_rule_index(driver: FullRunDriver) -> int | None:
    payload = driver.request(
        {
            "cmd": "preview_policy_action",
            "policy": "rule_baseline_v0",
            "include_state": False,
            "include_next_state": False,
            "check_live_env_unchanged": False,
        }
    ).get("payload") or {}
    value = payload.get("chosen_action_index")
    return int(value) if value is not None else None


def new_mode_stats(mode: str) -> dict[str, Any]:
    return {
        "mode": mode,
        "group_count": 0,
        "candidate_count": 0,
        "pruned_group_count": 0,
        "pruned_candidate_count": 0,
        "full_selected_pruned_count": 0,
        "positive_candidate_count": 0,
        "positive_pruned_count": 0,
        "pruned_card_counts": Counter(),
    }


def finalize_mode_stats(stats: dict[str, Any]) -> None:
    stats["pruned_candidate_rate"] = (
        stats["pruned_candidate_count"] / stats["candidate_count"]
        if stats["candidate_count"]
        else None
    )
    stats["pruned_group_rate"] = (
        stats["pruned_group_count"] / stats["group_count"] if stats["group_count"] else None
    )
    stats["positive_pruned_rate"] = (
        stats["positive_pruned_count"] / stats["positive_candidate_count"]
        if stats["positive_candidate_count"]
        else None
    )
    stats["pruned_card_counts"] = dict(stats["pruned_card_counts"].most_common(30))


def record_skip(summary: dict[str, Any], counters: dict[str, Counter[str]], reason: str) -> None:
    summary["skipped_groups"] += 1
    counters["skip_reasons"][reason] += 1


def render_compact(summary: dict[str, Any]) -> dict[str, Any]:
    return {
        "episodes_started": summary["episodes_started"],
        "groups": summary["groups"],
        "skipped_groups": summary["skipped_groups"],
        "full_override_count": summary["full_override_count"],
        "positive_candidate_count": summary["positive_candidate_count"],
        "full_candidate_evaluation_count": summary["full_candidate_evaluation_count"],
        "one_step_candidate_evaluation_count": summary["one_step_candidate_evaluation_count"],
        "modes": {
            mode: {
                "pruned_candidate_count": stats["pruned_candidate_count"],
                "pruned_candidate_rate": stats["pruned_candidate_rate"],
                "pruned_group_count": stats["pruned_group_count"],
                "full_selected_pruned_count": stats["full_selected_pruned_count"],
                "positive_pruned_count": stats["positive_pruned_count"],
                "positive_pruned_rate": stats["positive_pruned_rate"],
                "top_pruned_cards": list(stats["pruned_card_counts"].items())[:12],
            }
            for mode, stats in summary["modes"].items()
        },
    }


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n")


def serializable_config(args: argparse.Namespace) -> dict[str, Any]:
    out: dict[str, Any] = {}
    for key, value in vars(args).items():
        out[key] = str(value) if isinstance(value, Path) else value
    return out


def as_int(value: Any) -> int | None:
    if value is None:
        return None
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def extract_segment(key: str, name: str) -> str:
    marker = f"{name}:"
    for part in key.split("/"):
        if part.startswith(marker):
            return part[len(marker) :]
    return ""


def card_from_key(key: str) -> str | None:
    match = re.search(r"card:([^/]+)", key)
    return match.group(1) if match else None


if __name__ == "__main__":
    main()
