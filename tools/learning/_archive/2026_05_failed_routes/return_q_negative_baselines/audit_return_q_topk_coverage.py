#!/usr/bin/env python3
"""Audit whether a return-Q model's top-K covers useful combat actions."""
from __future__ import annotations

import argparse
import json
import random
from collections import Counter
from pathlib import Path
from typing import Any

from return_q_common import FullRunDriver, legal_candidate_indices, model_scores_are_rank_only, predict_model, write_json


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--episodes", type=int, default=20)
    parser.add_argument("--seed-start", type=int, default=95000)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=120)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--horizon-decisions", type=int, default=4)
    parser.add_argument("--top-k", type=int, default=4)
    parser.add_argument("--candidate-scope", default="all", choices=["all", "controlled_v0", "controlled_v1"])
    parser.add_argument("--max-candidates", type=int, default=0)
    parser.add_argument("--trajectory-policy", default="learned_q_direct")
    parser.add_argument("--fallback-count", type=int, default=1)
    parser.add_argument("--selective-horizon-decisions", type=int, default=4)
    parser.add_argument("--max-records", type=int, default=500)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    model = json.loads(args.model.read_text(encoding="utf-8"))
    rng = random.Random(args.seed_start)
    driver = FullRunDriver(args.binary)
    episodes = []
    records = []
    try:
        for episode_index in range(args.episodes):
            seed = args.seed_start + episode_index * args.seed_step
            episode, episode_records = audit_episode(args, driver, rng, seed, model)
            episodes.append(episode)
            records.extend(episode_records)
    finally:
        driver.close()

    report = {
        "schema_version": "return_q_topk_coverage_audit_v0",
        "model": str(args.model),
        "config": {
            "episodes": args.episodes,
            "seed_start": args.seed_start,
            "seed_step": args.seed_step,
            "max_steps": args.max_steps,
            "horizon_decisions": args.horizon_decisions,
            "gamma": args.gamma,
            "top_k": args.top_k,
            "candidate_scope": args.candidate_scope,
            "max_candidates": args.max_candidates,
            "trajectory_policy": args.trajectory_policy,
        },
        "summary": summarize(records, episodes),
        "episodes": episodes,
        "records": records[: args.max_records],
    }
    write_json(args.out, report)
    print(json.dumps(report["summary"], indent=2, sort_keys=True))


def audit_episode(
    args: argparse.Namespace,
    driver: FullRunDriver,
    rng: random.Random,
    seed: int,
    model: dict[str, Any],
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    try:
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
    except Exception as err:
        return episode_crash(seed, 0, err), []

    records = []
    done = bool(response.get("done"))
    total_reward = float(response.get("reward") or 0.0)
    last_info = response.get("info") or {}
    steps = 0
    combat_decisions = 0
    crash = None

    while not done and steps < args.max_steps:
        payload = response.get("payload") or {}
        if is_combat_payload(payload):
            combat_decisions += 1
            try:
                record = audit_current_state(args, driver, response, model)
                if record is not None:
                    records.append(record)
                response = step_trajectory(args, driver, rng, response, model)
            except Exception as err:
                crash = str(err)
                break
        else:
            try:
                response = driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
            except Exception as err:
                crash = str(err)
                break

        total_reward += float(response.get("reward") or 0.0)
        last_info = response.get("info") or last_info
        done = bool(response.get("done"))
        steps += 1

    return {
        "seed": seed,
        "steps": steps,
        "done": done,
        "crash": crash,
        "result": "crash" if crash else last_info.get("result"),
        "terminal_reason": "script_error" if crash else last_info.get("terminal_reason"),
        "combat_win_count": int(last_info.get("combat_win_count") or 0),
        "total_reward": total_reward,
        "combat_decisions": combat_decisions,
        "records": len(records),
    }, records


def episode_crash(seed: int, steps: int, err: Exception) -> dict[str, Any]:
    return {
        "seed": seed,
        "steps": steps,
        "done": True,
        "crash": str(err),
        "result": "crash",
        "terminal_reason": "reset_error",
        "combat_win_count": 0,
        "total_reward": 0.0,
        "combat_decisions": 0,
        "records": 0,
    }


def audit_current_state(
    args: argparse.Namespace,
    driver: FullRunDriver,
    response: dict[str, Any],
    model: dict[str, Any],
) -> dict[str, Any] | None:
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    candidates = payload.get("action_candidates") or []
    legal = scoped_legal_indices(args, response)
    if not legal:
        return None

    scored = [(idx, predict_model(model, observation, candidates[idx])) for idx in legal]
    ranked = [idx for idx, _score in sorted(scored, key=lambda item: item[1], reverse=True)]
    ranks = {idx: rank for rank, idx in enumerate(ranked, start=1)}
    top_k = set(ranked[: max(args.top_k, 1)])
    top1 = ranked[0]

    rule_index = preview_rule_index(driver)
    eval_indices = ranked
    capped = False
    if args.max_candidates > 0 and len(eval_indices) > args.max_candidates:
        capped = True
        eval_indices = eval_indices[: args.max_candidates]
    rule_in_scope = rule_index is not None and rule_index in legal
    if rule_in_scope:
        eval_indices = sorted(set(eval_indices + [rule_index]))
    else:
        eval_indices = sorted(set(eval_indices))

    eval_payload = driver.request(
        {
            "cmd": "evaluate_candidates",
            "action_indices": eval_indices,
            "continuation_policy": "rule_baseline_v0",
            "horizon_decisions": args.horizon_decisions,
            "gamma": args.gamma,
            "include_state": False,
            "include_next_state": False,
            "include_continuation_trace": False,
            "check_live_env_unchanged": False,
        }
    ).get("payload") or {}
    by_index = {
        int(item.get("action_index")): item
        for item in (eval_payload.get("evaluations") or [])
        if item.get("ok")
    }
    if not by_index:
        return None

    best_index = max(
        by_index,
        key=lambda idx: float(by_index[idx].get("discounted_return") or 0.0),
    )
    best_return = float(by_index[best_index].get("discounted_return") or 0.0)
    top1_return = float(by_index.get(top1, {}).get("discounted_return") or 0.0)
    rule_return = (
        float(by_index.get(rule_index, {}).get("discounted_return") or 0.0)
        if rule_index is not None and rule_index in by_index
        else None
    )
    return {
        "seed": (response.get("info") or {}).get("seed"),
        "step": (response.get("info") or {}).get("step"),
        "floor": observation.get("floor"),
        "current_hp": observation.get("current_hp"),
        "incoming": (observation.get("combat") or {}).get("visible_incoming_damage"),
        "monster_hp": (observation.get("combat") or {}).get("total_monster_hp"),
        "energy": (observation.get("combat") or {}).get("energy"),
        "legal_count": len(legal),
        "evaluated_count": len(by_index),
        "evaluation_capped": capped,
        "top_k": args.top_k,
        "top1_index": top1,
        "top1_action_key": action_key(candidates[top1]),
        "top1_action_family": action_family(candidates[top1]),
        "top1_score": dict(scored).get(top1),
        "top1_short_return": top1_return,
        "best_index": best_index,
        "best_action_key": action_key(candidates[best_index]),
        "best_action_family": action_family(candidates[best_index]),
        "best_rank": ranks.get(best_index),
        "best_short_return": best_return,
        "best_in_topk": best_index in top_k,
        "top1_is_best": best_index == top1,
        "top1_regret": best_return - top1_return,
        "rule_index": rule_index,
        "rule_action_key": action_key(candidates[rule_index]) if rule_in_scope else None,
        "rule_action_family": action_family(candidates[rule_index]) if rule_in_scope else None,
        "rule_rank": ranks.get(rule_index) if rule_in_scope else None,
        "rule_short_return": rule_return,
        "rule_in_scope": rule_in_scope,
        "rule_in_topk": rule_index in top_k if rule_in_scope else None,
        "top1_is_rule": top1 == rule_index if rule_in_scope else False,
    }


def preview_rule_index(driver: FullRunDriver) -> int | None:
    try:
        payload = driver.request(
            {
                "cmd": "preview_policy_action",
                "policy": "rule_baseline_v0",
                "include_state": False,
                "include_next_state": False,
                "check_live_env_unchanged": False,
            }
        ).get("payload") or {}
    except Exception:
        return None
    value = payload.get("chosen_action_index")
    return int(value) if value is not None else None


def step_trajectory(
    args: argparse.Namespace,
    driver: FullRunDriver,
    rng: random.Random,
    response: dict[str, Any],
    model: dict[str, Any],
) -> dict[str, Any]:
    if args.trajectory_policy == "random":
        legal = scoped_legal_indices(args, response)
        if legal:
            return driver.request({"cmd": "step", "action_index": rng.choice(legal)})
        return driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
    if args.trajectory_policy in {"rule_baseline_v0", "plan_query_v0"}:
        return driver.request({"cmd": "step_policy", "policy": args.trajectory_policy})
    if args.trajectory_policy == "learned_q_direct":
        return driver.request({"cmd": "step", "action_index": choose_direct(args, response, model)})
    if args.trajectory_policy == "learned_q_selective_1ply":
        return driver.request({"cmd": "step", "action_index": choose_selective(args, driver, response, model)})
    raise RuntimeError(f"unknown trajectory policy {args.trajectory_policy}")


def choose_direct(args: argparse.Namespace, response: dict[str, Any], model: dict[str, Any]) -> int:
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    candidates = payload.get("action_candidates") or []
    legal = scoped_legal_indices(args, response)
    if not legal:
        return 0
    return max(legal, key=lambda idx: predict_model(model, observation, candidates[idx]))


def choose_selective(
    args: argparse.Namespace,
    driver: FullRunDriver,
    response: dict[str, Any],
    model: dict[str, Any],
) -> int:
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    candidates = payload.get("action_candidates") or []
    legal = scoped_legal_indices(args, response)
    if not legal:
        return 0
    ranked = sorted(
        legal,
        key=lambda idx: predict_model(model, observation, candidates[idx]),
        reverse=True,
    )
    selected = ranked[: max(args.top_k, 1)]
    if args.fallback_count > 0:
        selected.extend(ranked[-args.fallback_count :])
    selected = sorted(set(selected))
    eval_payload = driver.request(
        {
            "cmd": "evaluate_candidates",
            "action_indices": selected,
            "continuation_policy": "rule_baseline_v0",
            "horizon_decisions": args.selective_horizon_decisions,
            "gamma": args.gamma,
            "include_state": False,
            "include_next_state": True,
            "include_continuation_trace": False,
            "check_live_env_unchanged": False,
        }
    ).get("payload") or {}

    rank_only_model = model_scores_are_rank_only(model)
    best_index = selected[0]
    best_score = float("-inf")
    for evaluation in eval_payload.get("evaluations") or []:
        if not evaluation.get("ok"):
            continue
        action_index = int(evaluation.get("action_index"))
        if rank_only_model:
            score = float(evaluation.get("discounted_return") or 0.0)
        else:
            score = float(evaluation.get("one_step_reward") or 0.0)
            if not evaluation.get("done"):
                next_state = evaluation.get("next_state") or {}
                next_observation = next_state.get("observation") or {}
                next_candidates = next_state.get("action_candidates") or []
                next_legal = legal_candidate_indices({"payload": next_state}, args.candidate_scope)
                next_scores = [
                    predict_model(model, next_observation, next_candidates[idx])
                    for idx in next_legal
                ]
                if next_scores:
                    score += args.gamma * max(next_scores)
        if score > best_score:
            best_score = score
            best_index = action_index
    return best_index


def legal_indices(response: dict[str, Any]) -> list[int]:
    return legal_candidate_indices(response, "all")


def scoped_legal_indices(args: argparse.Namespace, response: dict[str, Any]) -> list[int]:
    return legal_candidate_indices(response, args.candidate_scope)


def is_combat_payload(payload: dict[str, Any]) -> bool:
    observation = payload.get("observation") or {}
    return str(observation.get("decision_type") or "").startswith("combat")


def action_key(candidate: dict[str, Any]) -> str:
    return str(candidate.get("action_key") or "")


def action_family(candidate: dict[str, Any]) -> str:
    key = action_key(candidate)
    if key.startswith("combat/end_turn"):
        return "end_turn"
    if key.startswith("combat/play_card"):
        card = segment(key, "card")
        return f"play:{card or 'unknown'}"
    if key.startswith("combat/use_potion"):
        return "use_potion"
    return key.split("/", 1)[0] if key else "unknown"


def segment(key: str, name: str) -> str:
    marker = f"{name}:"
    for part in key.split("/"):
        if part.startswith(marker):
            return part[len(marker) :]
    return ""


def summarize(records: list[dict[str, Any]], episodes: list[dict[str, Any]]) -> dict[str, Any]:
    missed_best = [record for record in records if not record.get("best_in_topk")]
    rule_known = [record for record in records if record.get("rule_in_topk") is not None]
    best_ranks = [int(record["best_rank"]) for record in records if record.get("best_rank") is not None]
    rule_ranks = [int(record["rule_rank"]) for record in rule_known if record.get("rule_rank") is not None]
    candidate_count_histogram = Counter(int(record.get("legal_count") or 0) for record in records)
    random_topk_expected = mean(
        min(float(record.get("top_k") or 0.0), float(record.get("legal_count") or 0.0))
        / max(float(record.get("legal_count") or 0.0), 1.0)
        for record in records
    )
    rank_percentiles = [
        (float(record["best_rank"]) - 1.0) / max(float(record.get("legal_count") or 1) - 1.0, 1.0)
        for record in records
        if record.get("best_rank") is not None
    ]
    by_count = summarize_by_candidate_count(records)
    top_missed = Counter(
        (record.get("top1_action_family"), record.get("best_action_family"))
        for record in missed_best
    )
    top_regret = Counter(
        (record.get("top1_action_family"), record.get("best_action_family"))
        for record in records
        if float(record.get("top1_regret") or 0.0) > 0.05
    )
    return {
        "episodes": len(episodes),
        "episode_crashes": sum(1 for episode in episodes if episode.get("crash")),
        "average_total_reward": mean(float(episode.get("total_reward") or 0.0) for episode in episodes),
        "average_combat_win_count": mean(float(episode.get("combat_win_count") or 0.0) for episode in episodes),
        "combat_states": len(records),
        "average_legal_count": mean(float(record.get("legal_count") or 0.0) for record in records),
        "average_evaluated_count": mean(float(record.get("evaluated_count") or 0.0) for record in records),
        "candidate_count_histogram": {
            str(count): value for count, value in sorted(candidate_count_histogram.items())
        },
        "random_topk_expected_coverage": random_topk_expected,
        "average_best_rank_percentile": mean(rank_percentiles),
        "coverage_by_candidate_count": by_count,
        "capped_state_count": sum(1 for record in records if record.get("evaluation_capped")),
        "best_short_return_in_topk_rate": rate(record.get("best_in_topk") for record in records),
        "rule_in_topk_rate": rate(record.get("rule_in_topk") for record in rule_known),
        "top1_best_rate": rate(record.get("top1_is_best") for record in records),
        "top1_rule_rate": rate(record.get("top1_is_rule") for record in rule_known),
        "average_best_rank": mean(best_ranks),
        "average_rule_rank": mean(rule_ranks),
        "average_top1_regret": mean(float(record.get("top1_regret") or 0.0) for record in records),
        "missed_best_count": len(missed_best),
        "top_missed_best_pairs": [
            {"top1": pair[0], "best": pair[1], "count": count}
            for pair, count in top_missed.most_common(12)
        ],
        "top_regret_pairs": [
            {"top1": pair[0], "best": pair[1], "count": count}
            for pair, count in top_regret.most_common(12)
        ],
        "worst_regret_examples": sorted(
            records,
            key=lambda record: float(record.get("top1_regret") or 0.0),
            reverse=True,
        )[:12],
    }


def summarize_by_candidate_count(records: list[dict[str, Any]]) -> dict[str, dict[str, float]]:
    buckets: dict[str, list[dict[str, Any]]] = {}
    for record in records:
        legal_count = int(record.get("legal_count") or 0)
        key = str(legal_count if legal_count <= 8 else "9+")
        buckets.setdefault(key, []).append(record)
    summary = {}
    for key in sorted(buckets, key=lambda item: int(item.rstrip("+"))):
        rows = buckets[key]
        summary[key] = {
            "count": len(rows),
            "best_short_return_in_topk_rate": rate(row.get("best_in_topk") for row in rows),
            "random_topk_expected_coverage": mean(
                min(float(row.get("top_k") or 0.0), float(row.get("legal_count") or 0.0))
                / max(float(row.get("legal_count") or 0.0), 1.0)
                for row in rows
            ),
            "top1_best_rate": rate(row.get("top1_is_best") for row in rows),
            "average_best_rank_percentile": mean(
                (float(row["best_rank"]) - 1.0) / max(float(row.get("legal_count") or 1) - 1.0, 1.0)
                for row in rows
                if row.get("best_rank") is not None
            ),
        }
    return summary


def rate(values: Any) -> float:
    values = [bool(value) for value in values]
    return sum(1 for value in values if value) / len(values) if values else 0.0


def mean(values: Any) -> float:
    values = list(values)
    return sum(values) / len(values) if values else 0.0


if __name__ == "__main__":
    main()
