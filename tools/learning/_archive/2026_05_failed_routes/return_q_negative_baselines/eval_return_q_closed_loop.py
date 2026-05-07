#!/usr/bin/env python3
"""Evaluate a learned return-Q model in the full-run engine loop."""
from __future__ import annotations

import argparse
import json
import pickle
import random
from collections import Counter
from pathlib import Path
from typing import Any

from return_q_common import (
    FullRunDriver,
    legal_candidate_indices,
    model_scores_are_rank_only,
    predict_adv_override_probability,
    predict_model,
    write_json,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--episodes", type=int, default=10)
    parser.add_argument("--seed-start", type=int, default=10000)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=500)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--candidate-scope", default="all", choices=["all", "controlled_v0", "controlled_v1"])
    parser.add_argument("--top-k", type=int, default=4)
    parser.add_argument("--fallback-count", type=int, default=1)
    parser.add_argument("--selective-horizon-decisions", type=int, default=4)
    parser.add_argument("--oracle-margin", type=float, default=0.0)
    parser.add_argument("--oracle-continuation-policy", default="rule_baseline_v0")
    parser.add_argument(
        "--verified-evaluation-mode",
        default="independent",
        choices=["independent", "bellman_cached_v1"],
    )
    parser.add_argument("--verified-value-cache-scope", default="episode", choices=["request", "episode"])
    parser.add_argument("--verified-value-cache-max-entries", type=int, default=4096)
    parser.add_argument(
        "--verified-parallelism",
        type=int,
        default=0,
        help="Parallel candidate evaluations for verified exact H runs. 0 means auto; 1 means serial.",
    )
    parser.add_argument(
        "--verified-exact-root-dedup",
        action=argparse.BooleanOptionalAction,
        default=False,
        help="Reuse exact identical one-step candidate suffixes during verified evaluation.",
    )
    parser.add_argument("--verified-proposer-model", type=Path)
    parser.add_argument("--verified-proposer-top-k", type=int, default=0)
    parser.add_argument("--verified-proposer-threshold", type=float, default=-1.0)
    parser.add_argument(
        "--verified-proposer-feature-horizons",
        default="",
        help="Cheap horizons such as '1,2' used as proposer features before final H verification.",
    )
    parser.add_argument(
        "--verified-prefilter-horizon-decisions",
        type=int,
        default=-1,
        help="Optional shallow horizon used to reject verified override candidates before the final horizon. Disabled by default.",
    )
    parser.add_argument(
        "--verified-prefilter-margin",
        type=float,
        default=0.0,
        help="Candidate must beat rule by this shallow margin to reach final verification.",
    )
    parser.add_argument(
        "--verified-prefilter-top-k",
        type=int,
        default=0,
        help="Optional cap on candidates sent from shallow prefilter to final verification. 0 means no cap.",
    )
    parser.add_argument(
        "--policies",
        default="random,rule_baseline_v0,plan_query_v0,learned_q_direct,learned_q_selective_1ply",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    model = load_model(args.model) if args.model else None
    proposer_model = load_model(args.verified_proposer_model) if args.verified_proposer_model else None
    policies = [item.strip() for item in args.policies.split(",") if item.strip()]
    if model is None:
        policies = [policy for policy in policies if not policy.startswith("learned_q")]
    results = []
    driver = FullRunDriver(args.binary)
    rng = random.Random(args.seed_start)
    try:
        for policy in policies:
            for episode_index in range(args.episodes):
                seed = args.seed_start + episode_index * args.seed_step
                results.append(run_episode(args, driver, rng, seed, policy, model, proposer_model))
    finally:
        driver.close()

    summary = summarize(args, results)
    write_json(args.out, summary)
    print(json.dumps(summary, indent=2, sort_keys=True))


def load_model(path: Path | None) -> dict[str, Any] | None:
    if path is None:
        return None
    if path.suffix.lower() in {".pkl", ".pickle"}:
        with path.open("rb") as handle:
            return pickle.load(handle)
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def run_episode(
    args: argparse.Namespace,
    driver: FullRunDriver,
    rng: random.Random,
    seed: int,
    policy: str,
    model: dict[str, Any] | None,
    proposer_model: dict[str, Any] | None,
) -> dict[str, Any]:
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
        return {
            "policy": policy,
            "seed": seed,
            "steps": 0,
            "done": True,
            "crash": str(err),
            "result": "crash",
            "terminal_reason": "reset_error",
            "combat_win_count": 0,
            "total_reward": 0.0,
            "learned_decisions": 0,
        }
    done = bool(response.get("done"))
    total_reward = float(response.get("reward") or 0.0)
    steps = 0
    learned_decisions = 0
    last_info = response.get("info") or {}
    crash = None
    verified_stats = VerifiedOverrideStats()

    while not done and steps < args.max_steps:
        try:
            learned_combat_decision = (
                policy.startswith("learned_q")
                or policy.startswith("short_return_oracle")
                or policy.startswith("verified_adv_override_agent_v0")
            ) and is_combat_response(response)
            response = step_policy(args, driver, rng, response, policy, model, proposer_model, verified_stats)
        except Exception as err:
            crash = str(err)
            break
        total_reward += float(response.get("reward") or 0.0)
        info = response.get("info") or {}
        if info:
            last_info = info
        if learned_combat_decision:
            learned_decisions += 1
        done = bool(response.get("done"))
        steps += 1

    result = {
        "policy": policy,
        "seed": seed,
        "steps": steps,
        "done": done,
        "crash": crash,
        "result": last_info.get("result") if not crash else "crash",
        "terminal_reason": last_info.get("terminal_reason") if not crash else "script_error",
        "combat_win_count": int(last_info.get("combat_win_count") or 0),
        "total_reward": total_reward,
        "learned_decisions": learned_decisions,
    }
    result.update(verified_stats.as_episode_dict())
    return result


def step_policy(
    args: argparse.Namespace,
    driver: FullRunDriver,
    rng: random.Random,
    response: dict[str, Any],
    policy: str,
    model: dict[str, Any] | None,
    proposer_model: dict[str, Any] | None,
    verified_stats: "VerifiedOverrideStats | None" = None,
) -> dict[str, Any]:
    if policy == "random":
        legal = legal_indices(response)
        if not legal:
            return driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
        return driver.request({"cmd": "step", "action_index": rng.choice(legal)})
    if policy in {"rule_baseline_v0", "plan_query_v0"}:
        return driver.request({"cmd": "step_policy", "policy": policy})
    if policy.startswith("short_return_oracle_controlled_H"):
        if not is_combat_response(response):
            return driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
        action_index = choose_short_return_oracle(args, driver, response, policy)
        return driver.request({"cmd": "step", "action_index": action_index})
    if policy.startswith("short_return_oracle_shielded_vs_rule_H"):
        if not is_combat_response(response):
            return driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
        action_index = choose_short_return_oracle_shielded(args, driver, response, policy)
        return driver.request({"cmd": "step", "action_index": action_index})
    if policy.startswith("verified_adv_override_agent_v0_H"):
        if not is_combat_response(response):
            return driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
        action_index = choose_verified_adv_override_agent(
            args,
            driver,
            response,
            policy,
            verified_stats,
            proposer_model,
        )
        return driver.request({"cmd": "step", "action_index": action_index})
    if model is None:
        raise RuntimeError(f"policy {policy} requires --model")
    if not is_combat_response(response):
        return driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
    if policy == "learned_q_direct":
        action_index = choose_direct(args, response, model)
        return driver.request({"cmd": "step", "action_index": action_index})
    if policy == "learned_q_selective_1ply":
        action_index = choose_selective(args, driver, response, model)
        return driver.request({"cmd": "step", "action_index": action_index})
    raise RuntimeError(f"unknown policy {policy}")


def is_combat_response(response: dict[str, Any]) -> bool:
    observation = ((response.get("payload") or {}).get("observation") or {})
    return str(observation.get("decision_type") or "").startswith("combat")


def legal_indices(response: dict[str, Any]) -> list[int]:
    return legal_candidate_indices(response, "all")


def scoped_legal_indices(args: argparse.Namespace, response: dict[str, Any]) -> list[int]:
    return legal_candidate_indices(response, args.candidate_scope)


def choose_direct(args: argparse.Namespace, response: dict[str, Any], model: dict[str, Any]) -> int:
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    candidates = payload.get("action_candidates") or []
    legal = scoped_legal_indices(args, response)
    if not legal:
        return 0
    return max(
        legal,
        key=lambda idx: predict_model(model, observation, candidates[idx]),
    )


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

    best_index = selected[0]
    best_score = float("-inf")
    rank_only_model = model_scores_are_rank_only(model)
    for evaluation in eval_payload.get("evaluations") or []:
        if not evaluation.get("ok"):
            continue
        action_index = int(evaluation.get("action_index"))
        if rank_only_model:
            score = float(evaluation.get("discounted_return") or 0.0)
        else:
            score = float(evaluation.get("one_step_reward") or 0.0)
        if not evaluation.get("done") and not rank_only_model:
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


def choose_short_return_oracle(
    args: argparse.Namespace,
    driver: FullRunDriver,
    response: dict[str, Any],
    policy: str,
) -> int:
    legal = scoped_legal_indices(args, response)
    if not legal:
        return 0
    by_index = evaluate_action_indices(args, driver, legal, parse_oracle_horizon(policy))
    if not by_index:
        return legal[0]
    return max(
        by_index,
        key=lambda idx: float(by_index[idx].get("discounted_return") or 0.0),
    )


def choose_short_return_oracle_shielded(
    args: argparse.Namespace,
    driver: FullRunDriver,
    response: dict[str, Any],
    policy: str,
) -> int:
    scoped = scoped_legal_indices(args, response)
    if not scoped:
        return 0
    rule_index = preview_rule_index(driver)
    all_legal = set(legal_indices(response))
    eval_indices = set(scoped)
    if rule_index is not None and rule_index in all_legal:
        eval_indices.add(rule_index)
    by_index = evaluate_action_indices(args, driver, sorted(eval_indices), parse_oracle_horizon(policy))
    if not by_index:
        return rule_index if rule_index is not None else scoped[0]
    best_index = max(
        [idx for idx in scoped if idx in by_index],
        key=lambda idx: float(by_index[idx].get("discounted_return") or 0.0),
        default=scoped[0],
    )
    if rule_index is None or rule_index not in by_index:
        return best_index
    best_return = float(by_index[best_index].get("discounted_return") or 0.0)
    rule_return = float(by_index[rule_index].get("discounted_return") or 0.0)
    if best_return > rule_return + args.oracle_margin:
        return best_index
    return rule_index


def choose_verified_adv_override_agent(
    args: argparse.Namespace,
    driver: FullRunDriver,
    response: dict[str, Any],
    policy: str,
    stats: "VerifiedOverrideStats | None",
    proposer_model: dict[str, Any] | None = None,
) -> int:
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    decision_type = str(observation.get("decision_type") or "unknown")
    scoped = scoped_legal_indices(args, response)
    if not scoped:
        if stats:
            stats.record_missing(decision_type, "no_scoped_candidates")
        return 0
    if stats:
        stats.record_decision(decision_type, len(scoped))

    rule_index = preview_rule_index(driver)
    all_legal = set(legal_indices(response))
    if rule_index is None or rule_index not in all_legal:
        if stats:
            stats.record_missing(decision_type, "missing_rule_action")
        return scoped[0]
    if not any(index != rule_index for index in scoped):
        if stats:
            stats.record_reject(decision_type, 0.0)
        return rule_index

    scoped_for_eval = apply_verified_proposer(args, driver, response, scoped, rule_index, proposer_model, stats)
    if not any(index != rule_index for index in scoped_for_eval):
        if stats:
            stats.record_reject(decision_type, 0.0)
        return rule_index

    eval_indices = set(scoped_for_eval)
    eval_indices.add(rule_index)
    horizon = parse_oracle_horizon(policy)
    by_index = evaluate_verified_candidates(
        args,
        driver,
        sorted(eval_indices),
        scoped_for_eval,
        rule_index,
        horizon,
        decision_type,
        stats,
    )
    if stats:
        stats.evaluated_candidate_count += len(by_index)
    if rule_index not in by_index:
        if stats:
            stats.record_missing(decision_type, "missing_rule_evaluation")
        return rule_index
    scoped_evaluated = [idx for idx in scoped_for_eval if idx in by_index]
    if not scoped_evaluated:
        if stats:
            stats.record_missing(decision_type, "missing_scoped_evaluations")
        return rule_index

    best_index = max(
        scoped_evaluated,
        key=lambda idx: float(by_index[idx].get("discounted_return") or 0.0),
    )
    best_return = float(by_index[best_index].get("discounted_return") or 0.0)
    rule_return = float(by_index[rule_index].get("discounted_return") or 0.0)
    adv = best_return - rule_return
    if best_index != rule_index and adv > args.oracle_margin:
        if stats:
            stats.record_override(decision_type, adv)
        return best_index
    if stats:
        stats.record_reject(decision_type, adv)
    return rule_index


def apply_verified_proposer(
    args: argparse.Namespace,
    driver: FullRunDriver,
    response: dict[str, Any],
    scoped: list[int],
    rule_index: int,
    proposer_model: dict[str, Any] | None,
    stats: "VerifiedOverrideStats | None",
) -> list[int]:
    if proposer_model is None:
        return scoped
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    candidates = payload.get("action_candidates") or []
    if rule_index >= len(candidates):
        return scoped
    non_rule = [idx for idx in scoped if idx != rule_index and idx < len(candidates)]
    if not non_rule:
        return scoped
    cheap_features = proposer_cheap_features(args, driver, sorted({rule_index, *non_rule}), rule_index, stats)
    scored = [
        (
            predict_adv_override_probability(
                proposer_model,
                observation,
                candidates[idx],
                candidates[rule_index],
                cheap_features.get(idx, {}),
            ),
            idx,
        )
        for idx in non_rule
    ]
    selected: set[int] = set()
    threshold = float(args.verified_proposer_threshold)
    if threshold >= 0.0:
        selected.update(idx for score, idx in scored if score >= threshold)
    top_k = int(args.verified_proposer_top_k)
    if top_k > 0:
        selected.update(idx for _score, idx in sorted(scored, reverse=True)[:top_k])
    if threshold < 0.0 and top_k <= 0:
        selected.update(non_rule)
    kept = sorted({rule_index, *selected})
    if stats:
        stats.record_proposer(len(non_rule), len(selected))
    return kept


def proposer_cheap_features(
    args: argparse.Namespace,
    driver: FullRunDriver,
    indices: list[int],
    rule_index: int,
    stats: "VerifiedOverrideStats | None",
) -> dict[int, dict[str, float]]:
    horizons = parse_int_list(args.verified_proposer_feature_horizons)
    if not horizons:
        return {}
    out: dict[int, dict[str, float]] = {idx: {} for idx in indices}
    for horizon in horizons:
        by_index = evaluate_action_indices(
            args,
            driver,
            indices,
            horizon,
            evaluation_mode="independent",
            value_cache_scope=args.verified_value_cache_scope,
            value_cache_max_entries=args.verified_value_cache_max_entries,
            parallelism=args.verified_parallelism,
            exact_root_dedup=False,
            stats=None,
        )
        if stats:
            stats.record_proposer_feature_evaluations(len(by_index))
        rule_eval = by_index.get(rule_index)
        if not rule_eval:
            continue
        ranked = sorted(
            (
                (idx, float(evaluation.get("discounted_return") or 0.0))
                for idx, evaluation in by_index.items()
                if idx != rule_index
            ),
            key=lambda item: (-item[1], item[0]),
        )
        rank_by_index = {idx: rank + 1 for rank, (idx, _value) in enumerate(ranked)}
        best_non_rule = ranked[0][1] if ranked else None
        rule_return = float(rule_eval.get("discounted_return") or 0.0)
        for idx, evaluation in by_index.items():
            candidate_return = float(evaluation.get("discounted_return") or 0.0)
            out.setdefault(idx, {})[f"h{horizon}_return"] = candidate_return
            out[idx][f"h{horizon}_rule_return"] = rule_return
            out[idx][f"h{horizon}_adv_vs_rule"] = candidate_return - rule_return
            out[idx][f"h{horizon}_one_step_reward"] = float(evaluation.get("one_step_reward") or 0.0)
            candidate_rank = rank_by_index.get(idx)
            if candidate_rank is not None and ranked:
                out[idx][f"h{horizon}_rank"] = float(candidate_rank)
                out[idx][f"h{horizon}_rank_percentile"] = (
                    (candidate_rank - 1) / max(len(ranked) - 1, 1)
                )
                out[idx][f"h{horizon}_best_gap"] = float(best_non_rule) - candidate_return
    return out


def evaluate_verified_candidates(
    args: argparse.Namespace,
    driver: FullRunDriver,
    eval_indices: list[int],
    scoped_indices: list[int],
    rule_index: int,
    horizon_decisions: int,
    decision_type: str,
    stats: "VerifiedOverrideStats | None",
) -> dict[int, dict[str, Any]]:
    prefilter_horizon = int(args.verified_prefilter_horizon_decisions)
    if prefilter_horizon < 0 or prefilter_horizon >= horizon_decisions:
        return evaluate_action_indices(
            args,
            driver,
            eval_indices,
            horizon_decisions,
            evaluation_mode=args.verified_evaluation_mode,
            value_cache_scope=args.verified_value_cache_scope,
            value_cache_max_entries=args.verified_value_cache_max_entries,
            parallelism=args.verified_parallelism,
            exact_root_dedup=args.verified_exact_root_dedup,
            stats=stats,
        )

    shallow_by_index = evaluate_action_indices(
        args,
        driver,
        eval_indices,
        prefilter_horizon,
        evaluation_mode=args.verified_evaluation_mode,
        value_cache_scope=args.verified_value_cache_scope,
        value_cache_max_entries=args.verified_value_cache_max_entries,
        parallelism=args.verified_parallelism,
        exact_root_dedup=args.verified_exact_root_dedup,
        stats=stats,
    )
    if stats:
        stats.record_prefilter_evaluations(decision_type, len(shallow_by_index))
    shallow_rule = shallow_by_index.get(rule_index)
    if not shallow_rule:
        if stats:
            stats.record_prefilter_reject(decision_type, 0.0, "missing_rule")
        return {}

    rule_return = float(shallow_rule.get("discounted_return") or 0.0)
    passing: list[tuple[float, int]] = []
    best_adv = float("-inf")
    for index in scoped_indices:
        if index == rule_index:
            continue
        evaluation = shallow_by_index.get(index)
        if not evaluation:
            continue
        adv = float(evaluation.get("discounted_return") or 0.0) - rule_return
        best_adv = max(best_adv, adv)
        if adv > float(args.verified_prefilter_margin):
            passing.append((adv, index))

    if not passing:
        if stats:
            stats.record_prefilter_reject(
                decision_type,
                best_adv if best_adv != float("-inf") else 0.0,
                "no_candidate_passed",
            )
        return {rule_index: shallow_rule}

    passing.sort(reverse=True)
    top_k = int(args.verified_prefilter_top_k)
    if top_k > 0:
        passing = passing[:top_k]
    final_indices = sorted({rule_index, *[index for _adv, index in passing]})
    if stats:
        stats.record_prefilter_pass(decision_type, len(final_indices), passing[0][0])
    return evaluate_action_indices(
        args,
        driver,
        final_indices,
        horizon_decisions,
        evaluation_mode=args.verified_evaluation_mode,
        value_cache_scope=args.verified_value_cache_scope,
        value_cache_max_entries=args.verified_value_cache_max_entries,
        parallelism=args.verified_parallelism,
        exact_root_dedup=args.verified_exact_root_dedup,
        stats=stats,
    )


def evaluate_action_indices(
    args: argparse.Namespace,
    driver: FullRunDriver,
    action_indices: list[int],
    horizon_decisions: int,
    *,
    evaluation_mode: str = "independent",
    value_cache_scope: str = "request",
    value_cache_max_entries: int = 4096,
    parallelism: int = 1,
    exact_root_dedup: bool = False,
    stats: "VerifiedOverrideStats | None" = None,
) -> dict[int, dict[str, Any]]:
    payload = driver.request(
        {
            "cmd": "evaluate_candidates",
            "action_indices": action_indices,
            "continuation_policy": args.oracle_continuation_policy,
            "horizon_decisions": horizon_decisions,
            "gamma": args.gamma,
            "evaluation_mode": evaluation_mode,
            "value_cache_scope": value_cache_scope,
            "value_cache_max_entries": value_cache_max_entries,
            "parallelism": parallelism,
            "exact_root_dedup": exact_root_dedup,
            "include_state": False,
            "include_next_state": False,
            "include_continuation_trace": False,
            "check_live_env_unchanged": False,
        }
    ).get("payload") or {}
    if stats is not None:
        stats.record_evaluation_payload(payload)
    return {
        int(item.get("action_index")): item
        for item in (payload.get("evaluations") or [])
        if item.get("ok")
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


def parse_oracle_horizon(policy: str) -> int:
    try:
        return max(int(policy.rsplit("_H", 1)[1]), 0)
    except (IndexError, ValueError):
        raise RuntimeError(f"oracle policy must end with _H<int>: {policy}")


class VerifiedOverrideStats:
    def __init__(self) -> None:
        self.decision_count = 0
        self.override_count = 0
        self.reject_count = 0
        self.evaluated_candidate_count = 0
        self.scoped_candidate_count_sum = 0
        self.verified_adv_sum = 0.0
        self.harmful_override_count = 0
        self.decision_type_counts: Counter[str] = Counter()
        self.override_decision_type_counts: Counter[str] = Counter()
        self.missing_counts: Counter[str] = Counter()
        self.prefilter_evaluation_count = 0
        self.prefilter_pass_count = 0
        self.prefilter_reject_count = 0
        self.prefilter_final_candidate_count = 0
        self.prefilter_best_adv_sum = 0.0
        self.prefilter_reject_counts: Counter[str] = Counter()
        self.cached_root_candidate_count = 0
        self.cached_root_exact_dedup_count = 0
        self.root_rule_equivalent_prune_count = 0
        self.cached_value_hit_count = 0
        self.cached_value_miss_count = 0
        self.cached_policy_step_eval_count = 0
        self.cached_cache_entry_count_max = 0
        self.parallelism_used_max = 0
        self.candidate_eval_wall_ms = 0
        self.proposer_non_rule_candidate_count = 0
        self.proposer_kept_candidate_count = 0
        self.proposer_decision_count = 0
        self.proposer_feature_evaluation_count = 0
        self.max_verified_adv: float | None = None
        self.min_verified_adv: float | None = None

    def record_decision(self, decision_type: str, scoped_candidate_count: int) -> None:
        self.decision_count += 1
        self.scoped_candidate_count_sum += scoped_candidate_count
        self.decision_type_counts[decision_type] += 1

    def record_override(self, decision_type: str, adv: float) -> None:
        self.override_count += 1
        self.verified_adv_sum += adv
        if adv < 0.0:
            self.harmful_override_count += 1
        self.override_decision_type_counts[decision_type] += 1
        self.max_verified_adv = adv if self.max_verified_adv is None else max(self.max_verified_adv, adv)
        self.min_verified_adv = adv if self.min_verified_adv is None else min(self.min_verified_adv, adv)

    def record_reject(self, decision_type: str, _adv: float) -> None:
        self.reject_count += 1

    def record_missing(self, decision_type: str, reason: str) -> None:
        self.missing_counts[f"{decision_type}:{reason}"] += 1

    def record_prefilter_evaluations(self, _decision_type: str, count: int) -> None:
        self.prefilter_evaluation_count += count

    def record_prefilter_pass(self, _decision_type: str, final_candidate_count: int, best_adv: float) -> None:
        self.prefilter_pass_count += 1
        self.prefilter_final_candidate_count += final_candidate_count
        self.prefilter_best_adv_sum += best_adv

    def record_prefilter_reject(self, decision_type: str, best_adv: float, reason: str) -> None:
        self.prefilter_reject_count += 1
        self.prefilter_best_adv_sum += best_adv
        self.prefilter_reject_counts[f"{decision_type}:{reason}"] += 1

    def record_evaluation_payload(self, payload: dict[str, Any]) -> None:
        self.cached_root_candidate_count += int(payload.get("root_candidate_count") or 0)
        self.cached_root_exact_dedup_count += int(payload.get("root_exact_dedup_count") or 0)
        self.root_rule_equivalent_prune_count += int(payload.get("root_rule_equivalent_prune_count") or 0)
        self.cached_value_hit_count += int(payload.get("value_cache_hit_count") or 0)
        self.cached_value_miss_count += int(payload.get("value_cache_miss_count") or 0)
        self.cached_policy_step_eval_count += int(payload.get("policy_step_eval_count") or 0)
        self.cached_cache_entry_count_max = max(
            self.cached_cache_entry_count_max,
            int(payload.get("cache_entry_count") or 0),
        )
        self.parallelism_used_max = max(self.parallelism_used_max, int(payload.get("parallelism_used") or 0))
        self.candidate_eval_wall_ms += int(payload.get("candidate_eval_wall_ms") or 0)

    def record_proposer(self, non_rule_count: int, kept_count: int) -> None:
        self.proposer_decision_count += 1
        self.proposer_non_rule_candidate_count += non_rule_count
        self.proposer_kept_candidate_count += kept_count

    def record_proposer_feature_evaluations(self, count: int) -> None:
        self.proposer_feature_evaluation_count += count

    def as_episode_dict(self) -> dict[str, Any]:
        return {
            "verified_decision_count": self.decision_count,
            "verified_override_count": self.override_count,
            "verified_reject_count": self.reject_count,
            "verified_override_rate": self.override_count / self.decision_count if self.decision_count else 0.0,
            "verified_candidate_evaluation_count": self.evaluated_candidate_count,
            "verified_average_scoped_candidate_count": (
                self.scoped_candidate_count_sum / self.decision_count if self.decision_count else 0.0
            ),
            "verified_adv_mean_on_overrides": (
                self.verified_adv_sum / self.override_count if self.override_count else None
            ),
            "verified_harmful_override_count": self.harmful_override_count,
            "verified_harmful_override_rate": (
                self.harmful_override_count / self.override_count if self.override_count else None
            ),
            "verified_min_adv_on_overrides": self.min_verified_adv,
            "verified_max_adv_on_overrides": self.max_verified_adv,
            "verified_decision_type_counts": dict(sorted(self.decision_type_counts.items())),
            "verified_override_decision_type_counts": dict(sorted(self.override_decision_type_counts.items())),
            "verified_missing_counts": dict(sorted(self.missing_counts.items())),
            "verified_prefilter_evaluation_count": self.prefilter_evaluation_count,
            "verified_prefilter_pass_count": self.prefilter_pass_count,
            "verified_prefilter_reject_count": self.prefilter_reject_count,
            "verified_prefilter_final_candidate_count": self.prefilter_final_candidate_count,
            "verified_prefilter_average_final_candidate_count": (
                self.prefilter_final_candidate_count / self.prefilter_pass_count
                if self.prefilter_pass_count
                else None
            ),
            "verified_prefilter_average_best_adv": (
                self.prefilter_best_adv_sum / (self.prefilter_pass_count + self.prefilter_reject_count)
                if self.prefilter_pass_count + self.prefilter_reject_count
                else None
            ),
            "verified_prefilter_reject_counts": dict(sorted(self.prefilter_reject_counts.items())),
            "verified_cached_root_candidate_count": self.cached_root_candidate_count,
            "verified_cached_root_exact_dedup_count": self.cached_root_exact_dedup_count,
            "verified_root_rule_equivalent_prune_count": self.root_rule_equivalent_prune_count,
            "verified_cached_value_hit_count": self.cached_value_hit_count,
            "verified_cached_value_miss_count": self.cached_value_miss_count,
            "verified_cached_policy_step_eval_count": self.cached_policy_step_eval_count,
            "verified_cached_cache_entry_count_max": self.cached_cache_entry_count_max,
            "verified_parallelism_used_max": self.parallelism_used_max,
            "verified_candidate_eval_wall_ms": self.candidate_eval_wall_ms,
            "verified_proposer_decision_count": self.proposer_decision_count,
            "verified_proposer_non_rule_candidate_count": self.proposer_non_rule_candidate_count,
            "verified_proposer_kept_candidate_count": self.proposer_kept_candidate_count,
            "verified_proposer_feature_evaluation_count": self.proposer_feature_evaluation_count,
            "verified_proposer_keep_rate": (
                self.proposer_kept_candidate_count / self.proposer_non_rule_candidate_count
                if self.proposer_non_rule_candidate_count
                else None
            ),
        }


def summarize(args: argparse.Namespace, results: list[dict[str, Any]]) -> dict[str, Any]:
    by_policy: dict[str, list[dict[str, Any]]] = {}
    for result in results:
        by_policy.setdefault(str(result["policy"]), []).append(result)
    policy_summary = {}
    for policy, rows in by_policy.items():
        verified_decisions = sum(int(row.get("verified_decision_count") or 0) for row in rows)
        verified_overrides = sum(int(row.get("verified_override_count") or 0) for row in rows)
        verified_evaluations = sum(int(row.get("verified_candidate_evaluation_count") or 0) for row in rows)
        verified_adv_weighted_sum = sum(
            float(row.get("verified_adv_mean_on_overrides") or 0.0)
            * int(row.get("verified_override_count") or 0)
            for row in rows
        )
        verified_harmful = sum(int(row.get("verified_harmful_override_count") or 0) for row in rows)
        verified_prefilter_evaluations = sum(
            int(row.get("verified_prefilter_evaluation_count") or 0) for row in rows
        )
        verified_prefilter_passes = sum(
            int(row.get("verified_prefilter_pass_count") or 0) for row in rows
        )
        verified_prefilter_rejects = sum(
            int(row.get("verified_prefilter_reject_count") or 0) for row in rows
        )
        verified_prefilter_final_candidates = sum(
            int(row.get("verified_prefilter_final_candidate_count") or 0) for row in rows
        )
        verified_prefilter_best_adv_weighted_sum = sum(
            float(row.get("verified_prefilter_average_best_adv") or 0.0)
            * (
                int(row.get("verified_prefilter_pass_count") or 0)
                + int(row.get("verified_prefilter_reject_count") or 0)
            )
            for row in rows
        )
        verified_prefilter_total_decisions = verified_prefilter_passes + verified_prefilter_rejects
        decision_type_counts = sum_counter(row.get("verified_decision_type_counts") for row in rows)
        override_decision_type_counts = sum_counter(row.get("verified_override_decision_type_counts") for row in rows)
        missing_counts = sum_counter(row.get("verified_missing_counts") for row in rows)
        prefilter_reject_counts = sum_counter(row.get("verified_prefilter_reject_counts") for row in rows)
        cached_root_candidates = sum(
            int(row.get("verified_cached_root_candidate_count") or 0) for row in rows
        )
        cached_root_dedup = sum(
            int(row.get("verified_cached_root_exact_dedup_count") or 0) for row in rows
        )
        root_rule_equiv_prunes = sum(
            int(row.get("verified_root_rule_equivalent_prune_count") or 0) for row in rows
        )
        cached_value_hits = sum(
            int(row.get("verified_cached_value_hit_count") or 0) for row in rows
        )
        cached_value_misses = sum(
            int(row.get("verified_cached_value_miss_count") or 0) for row in rows
        )
        cached_policy_steps = sum(
            int(row.get("verified_cached_policy_step_eval_count") or 0) for row in rows
        )
        cached_entry_count_max = max(
            [int(row.get("verified_cached_cache_entry_count_max") or 0) for row in rows],
            default=0,
        )
        parallelism_used_max = max(
            [int(row.get("verified_parallelism_used_max") or 0) for row in rows],
            default=0,
        )
        candidate_eval_wall_ms = sum(
            int(row.get("verified_candidate_eval_wall_ms") or 0) for row in rows
        )
        proposer_decisions = sum(
            int(row.get("verified_proposer_decision_count") or 0) for row in rows
        )
        proposer_non_rule = sum(
            int(row.get("verified_proposer_non_rule_candidate_count") or 0) for row in rows
        )
        proposer_kept = sum(
            int(row.get("verified_proposer_kept_candidate_count") or 0) for row in rows
        )
        proposer_feature_evals = sum(
            int(row.get("verified_proposer_feature_evaluation_count") or 0) for row in rows
        )
        rewards = [float(row.get("total_reward") or 0.0) for row in rows]
        policy_summary[policy] = {
            "episodes": len(rows),
            "crash_count": sum(1 for row in rows if row.get("crash")),
            "result_counts": counts(row.get("result") for row in rows),
            "average_total_reward": mean(rewards),
            "reward_stderr": stderr(rewards),
            "average_combat_win_count": mean(float(row.get("combat_win_count") or 0.0) for row in rows),
            "average_steps": mean(float(row.get("steps") or 0.0) for row in rows),
            "verified_decision_count": verified_decisions,
            "verified_override_count": verified_overrides,
            "verified_override_rate": verified_overrides / verified_decisions if verified_decisions else 0.0,
            "verified_candidate_evaluation_count": verified_evaluations,
            "verified_adv_mean_on_overrides": (
                verified_adv_weighted_sum / verified_overrides if verified_overrides else None
            ),
            "verified_harmful_override_count": verified_harmful,
            "verified_harmful_override_rate": (
                verified_harmful / verified_overrides if verified_overrides else None
            ),
            "verified_decision_type_counts": dict(sorted(decision_type_counts.items())),
            "verified_override_decision_type_counts": dict(sorted(override_decision_type_counts.items())),
            "verified_missing_counts": dict(sorted(missing_counts.items())),
            "verified_prefilter_evaluation_count": verified_prefilter_evaluations,
            "verified_prefilter_pass_count": verified_prefilter_passes,
            "verified_prefilter_reject_count": verified_prefilter_rejects,
            "verified_prefilter_pass_rate": (
                verified_prefilter_passes / verified_prefilter_total_decisions
                if verified_prefilter_total_decisions
                else None
            ),
            "verified_prefilter_final_candidate_count": verified_prefilter_final_candidates,
            "verified_prefilter_average_final_candidate_count": (
                verified_prefilter_final_candidates / verified_prefilter_passes
                if verified_prefilter_passes
                else None
            ),
            "verified_prefilter_average_best_adv": (
                verified_prefilter_best_adv_weighted_sum / verified_prefilter_total_decisions
                if verified_prefilter_total_decisions
                else None
            ),
            "verified_prefilter_reject_counts": dict(sorted(prefilter_reject_counts.items())),
            "verified_cached_root_candidate_count": cached_root_candidates,
            "verified_cached_root_exact_dedup_count": cached_root_dedup,
            "verified_cached_root_exact_dedup_rate": (
                cached_root_dedup / cached_root_candidates if cached_root_candidates else None
            ),
            "verified_root_rule_equivalent_prune_count": root_rule_equiv_prunes,
            "verified_root_rule_equivalent_prune_rate": (
                root_rule_equiv_prunes / cached_root_candidates if cached_root_candidates else None
            ),
            "verified_cached_value_hit_count": cached_value_hits,
            "verified_cached_value_miss_count": cached_value_misses,
            "verified_cached_value_hit_rate": (
                cached_value_hits / (cached_value_hits + cached_value_misses)
                if cached_value_hits + cached_value_misses
                else None
            ),
            "verified_cached_policy_step_eval_count": cached_policy_steps,
            "verified_cached_cache_entry_count_max": cached_entry_count_max,
            "verified_parallelism_used_max": parallelism_used_max,
            "verified_candidate_eval_wall_ms": candidate_eval_wall_ms,
            "verified_proposer_decision_count": proposer_decisions,
            "verified_proposer_non_rule_candidate_count": proposer_non_rule,
            "verified_proposer_kept_candidate_count": proposer_kept,
            "verified_proposer_feature_evaluation_count": proposer_feature_evals,
            "verified_proposer_keep_rate": (
                proposer_kept / proposer_non_rule if proposer_non_rule else None
            ),
        }
    return {
        "schema_version": "return_q_closed_loop_eval_v0",
        "config": {
            "episodes": args.episodes,
            "seed_start": args.seed_start,
            "seed_step": args.seed_step,
            "ascension": args.ascension,
            "class": args.player_class,
            "final_act": args.final_act,
            "max_steps": args.max_steps,
            "gamma": args.gamma,
            "candidate_scope": args.candidate_scope,
            "top_k": args.top_k,
            "fallback_count": args.fallback_count,
            "selective_horizon_decisions": args.selective_horizon_decisions,
            "oracle_margin": args.oracle_margin,
            "oracle_continuation_policy": args.oracle_continuation_policy,
            "verified_evaluation_mode": args.verified_evaluation_mode,
            "verified_value_cache_scope": args.verified_value_cache_scope,
            "verified_value_cache_max_entries": args.verified_value_cache_max_entries,
            "verified_parallelism": args.verified_parallelism,
            "verified_exact_root_dedup": args.verified_exact_root_dedup,
            "verified_proposer_model": str(args.verified_proposer_model) if args.verified_proposer_model else None,
            "verified_proposer_top_k": args.verified_proposer_top_k,
            "verified_proposer_threshold": args.verified_proposer_threshold,
            "verified_proposer_feature_horizons": args.verified_proposer_feature_horizons,
            "verified_prefilter_horizon_decisions": args.verified_prefilter_horizon_decisions,
            "verified_prefilter_margin": args.verified_prefilter_margin,
            "verified_prefilter_top_k": args.verified_prefilter_top_k,
            "model": str(args.model) if args.model else None,
        },
        "policy_summary": policy_summary,
        "episodes": results,
    }


def counts(values: Any) -> dict[str, int]:
    out: dict[str, int] = {}
    for value in values:
        key = str(value)
        out[key] = out.get(key, 0) + 1
    return out


def parse_int_list(value: str) -> list[int]:
    return [int(item) for item in value.split(",") if item.strip()]


def mean(values: Any) -> float:
    values = list(values)
    return sum(values) / len(values) if values else 0.0


def stderr(values: list[float]) -> float | None:
    if len(values) < 2:
        return None
    avg = mean(values)
    variance = sum((value - avg) ** 2 for value in values) / (len(values) - 1)
    return (variance ** 0.5) / (len(values) ** 0.5)


def sum_counter(items: Any) -> Counter[str]:
    out: Counter[str] = Counter()
    for item in items:
        if isinstance(item, dict):
            out.update({str(key): int(value) for key, value in item.items()})
    return out


if __name__ == "__main__":
    main()
