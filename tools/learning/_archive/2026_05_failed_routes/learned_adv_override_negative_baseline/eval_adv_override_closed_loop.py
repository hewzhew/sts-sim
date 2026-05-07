#!/usr/bin/env python3
"""Closed-loop evaluation for rule-backed learned advantage override."""
from __future__ import annotations

import argparse
import json
import random
from collections import defaultdict
from pathlib import Path
from typing import Any

import torch
import torch.nn as nn

from return_q_common import (
    FullRunDriver,
    adv_override_features,
    legal_candidate_indices,
    write_json,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--episodes", type=int, default=20)
    parser.add_argument("--seed-start", type=int, default=10000)
    parser.add_argument("--seed-step", type=int, default=1)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=500)
    parser.add_argument("--gamma", type=float, default=0.99)
    parser.add_argument("--candidate-scope", default="controlled_v1", choices=["all", "controlled_v0", "controlled_v1"])
    parser.add_argument("--threshold", type=float, default=0.8)
    parser.add_argument("--audit-horizon-decisions", type=int, default=4)
    parser.add_argument("--policies", default="rule_baseline_v0,learned_adv_shielded")
    return parser.parse_args()


class SparseOverrideMlp(nn.Module):
    def __init__(self, feature_dim: int, hidden_dim: int) -> None:
        super().__init__()
        self.embedding = nn.EmbeddingBag(
            feature_dim,
            hidden_dim,
            mode="sum",
            include_last_offset=True,
        )
        self.net = nn.Sequential(
            nn.LayerNorm(hidden_dim),
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
            nn.Dropout(0.05),
            nn.Linear(hidden_dim, hidden_dim // 2),
            nn.ReLU(),
            nn.Linear(hidden_dim // 2, 1),
        )

    def forward(self, indices: Any, offsets: Any, weights: Any) -> Any:
        embedded = self.embedding(indices, offsets, per_sample_weights=weights)
        return self.net(embedded).squeeze(-1)


def main() -> None:
    args = parse_args()
    model = load_model(args.model)
    policies = [item.strip() for item in args.policies.split(",") if item.strip()]
    results = []
    driver = FullRunDriver(args.binary)
    rng = random.Random(args.seed_start)
    try:
        for policy in policies:
            for episode_index in range(args.episodes):
                seed = args.seed_start + episode_index * args.seed_step
                results.append(run_episode(args, driver, rng, seed, policy, model))
    finally:
        driver.close()
    summary = summarize(args, results)
    write_json(args.out, summary)
    print(json.dumps(summary, indent=2, sort_keys=True))


def load_model(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        payload = json.load(handle)
    if payload.get("model_type") != "adv_override_torch_embedding_mlp":
        raise SystemExit(f"unsupported model_type {payload.get('model_type')}")
    state_path = Path(str(payload["state_dict_path"]))
    if not state_path.is_absolute():
        state_path = path.resolve().parents[2] / state_path
    config = payload.get("config") or {}
    torch_model = SparseOverrideMlp(
        int(config.get("feature_dim") or 32768),
        int(config.get("hidden_dim") or 96),
    )
    torch_model.load_state_dict(torch.load(state_path, map_location="cpu"))
    torch_model.eval()
    payload["_torch_model"] = torch_model
    return payload


def run_episode(
    args: argparse.Namespace,
    driver: FullRunDriver,
    rng: random.Random,
    seed: int,
    policy: str,
    model: dict[str, Any],
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
        return episode_error(policy, seed, str(err), "reset_error")

    done = bool(response.get("done"))
    total_reward = float(response.get("reward") or 0.0)
    steps = 0
    last_info = response.get("info") or {}
    crash = None
    override_stats = OverrideStats()

    while not done and steps < args.max_steps:
        try:
            if policy == "rule_baseline_v0":
                response = driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
            elif policy == "random":
                legal = legal_candidate_indices(response, "all")
                if legal:
                    response = driver.request({"cmd": "step", "action_index": rng.choice(legal)})
                else:
                    response = driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
            elif policy == "learned_adv_shielded":
                response = step_learned_adv_shielded(args, driver, response, model, override_stats)
            else:
                raise RuntimeError(f"unknown policy {policy}")
        except Exception as err:
            crash = str(err)
            break
        total_reward += float(response.get("reward") or 0.0)
        info = response.get("info") or {}
        if info:
            last_info = info
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
    }
    result.update(override_stats.as_episode_dict())
    return result


def step_learned_adv_shielded(
    args: argparse.Namespace,
    driver: FullRunDriver,
    response: dict[str, Any],
    model: dict[str, Any],
    stats: "OverrideStats",
) -> dict[str, Any]:
    if not is_combat_response(response):
        return driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
    payload = response.get("payload") or {}
    observation = payload.get("observation") or {}
    candidates = payload.get("action_candidates") or []
    scoped = legal_candidate_indices(response, args.candidate_scope)
    if not scoped:
        return driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
    rule_index = preview_rule_index(driver)
    if rule_index is None:
        return driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
    if rule_index >= len(candidates):
        return driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})
    if rule_index not in scoped:
        return driver.request({"cmd": "step_policy", "policy": "rule_baseline_v0"})

    stats.decision_count += 1
    rule_candidate = candidates[rule_index]
    scored = []
    for index in scoped:
        if index == rule_index:
            continue
        row = {
            "observation": observation,
            "candidate": candidates[index],
            "rule_candidate": rule_candidate,
            "candidate_delta_vs_start": {},
            "rule_delta_vs_start": {},
            "delta_vs_rule_features": {},
            "decision_kind": observation.get("decision_type", "unknown"),
        }
        scored.append((predict_probability(model, row), index))
    if not scored:
        return driver.request({"cmd": "step", "action_index": rule_index})
    best_prob, best_index = max(scored, key=lambda item: item[0])
    if best_prob < args.threshold:
        return driver.request({"cmd": "step", "action_index": rule_index})

    audit = audit_override(args, driver, best_index, rule_index)
    stats.record_override(best_prob, audit)
    return driver.request({"cmd": "step", "action_index": best_index})


def predict_probability(model: dict[str, Any], row: dict[str, Any]) -> float:
    torch_model = model["_torch_model"]
    sparse = adv_override_features(row, str(model["feature_set"]))
    if sparse:
        indices = torch.tensor(list(sparse.keys()), dtype=torch.long)
        weights = torch.tensor(list(sparse.values()), dtype=torch.float32)
    else:
        indices = torch.tensor([0], dtype=torch.long)
        weights = torch.tensor([0.0], dtype=torch.float32)
    offsets = torch.tensor([0, len(indices)], dtype=torch.long)
    with torch.no_grad():
        logit = torch_model(indices, offsets, weights).item()
    return float(torch.sigmoid(torch.tensor(logit)).item())


def audit_override(
    args: argparse.Namespace,
    driver: FullRunDriver,
    best_index: int,
    rule_index: int,
) -> dict[str, Any]:
    by_index = evaluate_action_indices(
        args,
        driver,
        sorted({best_index, rule_index}),
    )
    best_eval = by_index.get(best_index) or {}
    rule_eval = by_index.get(rule_index) or {}
    best_return = float(best_eval.get("discounted_return") or 0.0)
    rule_return = float(rule_eval.get("discounted_return") or 0.0)
    return {
        "ok": bool(best_eval and rule_eval),
        "best_index": best_index,
        "rule_index": rule_index,
        "best_return": best_return,
        "rule_return": rule_return,
        "adv": best_return - rule_return,
    }


def evaluate_action_indices(
    args: argparse.Namespace,
    driver: FullRunDriver,
    action_indices: list[int],
) -> dict[int, dict[str, Any]]:
    payload = driver.request(
        {
            "cmd": "evaluate_candidates",
            "action_indices": action_indices,
            "continuation_policy": "rule_baseline_v0",
            "horizon_decisions": args.audit_horizon_decisions,
            "gamma": args.gamma,
            "include_state": False,
            "include_next_state": False,
            "include_continuation_trace": False,
            "check_live_env_unchanged": False,
        }
    ).get("payload") or {}
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


def is_combat_response(response: dict[str, Any]) -> bool:
    observation = ((response.get("payload") or {}).get("observation") or {})
    return str(observation.get("decision_type") or "").startswith("combat")


class OverrideStats:
    def __init__(self) -> None:
        self.decision_count = 0
        self.override_count = 0
        self.harmful_override_count = 0
        self.override_adv_sum = 0.0
        self.override_prob_sum = 0.0
        self.audit_count = 0

    def record_override(self, probability: float, audit: dict[str, Any]) -> None:
        self.override_count += 1
        self.override_prob_sum += probability
        if audit.get("ok"):
            adv = float(audit.get("adv") or 0.0)
            self.audit_count += 1
            self.override_adv_sum += adv
            if adv < 0.0:
                self.harmful_override_count += 1

    def as_episode_dict(self) -> dict[str, Any]:
        return {
            "decision_count": self.decision_count,
            "override_count": self.override_count,
            "override_rate": self.override_count / self.decision_count if self.decision_count else 0.0,
            "audited_override_count": self.audit_count,
            "accepted_override_real_adv": self.override_adv_sum / self.audit_count if self.audit_count else None,
            "harmful_override_count": self.harmful_override_count,
            "harmful_override_rate": self.harmful_override_count / self.audit_count if self.audit_count else None,
            "average_override_probability": self.override_prob_sum / self.override_count if self.override_count else None,
        }


def episode_error(policy: str, seed: int, error: str, terminal_reason: str) -> dict[str, Any]:
    return {
        "policy": policy,
        "seed": seed,
        "steps": 0,
        "done": True,
        "crash": error,
        "result": "crash",
        "terminal_reason": terminal_reason,
        "combat_win_count": 0,
        "total_reward": 0.0,
        "decision_count": 0,
        "override_count": 0,
        "override_rate": 0.0,
        "audited_override_count": 0,
        "accepted_override_real_adv": None,
        "harmful_override_count": 0,
        "harmful_override_rate": None,
    }


def summarize(args: argparse.Namespace, results: list[dict[str, Any]]) -> dict[str, Any]:
    by_policy: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for result in results:
        by_policy[str(result["policy"])].append(result)
    policy_summary = {}
    for policy, rows in by_policy.items():
        audited = sum(int(row.get("audited_override_count") or 0) for row in rows)
        adv_sum = sum(
            float(row.get("accepted_override_real_adv") or 0.0)
            * int(row.get("audited_override_count") or 0)
            for row in rows
        )
        harmful = sum(int(row.get("harmful_override_count") or 0) for row in rows)
        decisions = sum(int(row.get("decision_count") or 0) for row in rows)
        overrides = sum(int(row.get("override_count") or 0) for row in rows)
        rewards = [float(row.get("total_reward") or 0.0) for row in rows]
        policy_summary[policy] = {
            "episodes": len(rows),
            "crash_count": sum(1 for row in rows if row.get("crash")),
            "result_counts": counts(row.get("result") for row in rows),
            "average_total_reward": mean(rewards),
            "reward_stderr": stderr(rewards),
            "average_combat_win_count": mean(float(row.get("combat_win_count") or 0.0) for row in rows),
            "average_steps": mean(float(row.get("steps") or 0.0) for row in rows),
            "decision_count": decisions,
            "override_count": overrides,
            "override_rate": overrides / decisions if decisions else 0.0,
            "audited_override_count": audited,
            "accepted_override_real_adv": adv_sum / audited if audited else None,
            "harmful_override_count": harmful,
            "harmful_override_rate": harmful / audited if audited else None,
        }
    return {
        "schema_version": "adv_override_closed_loop_eval_v1",
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
            "threshold": args.threshold,
            "audit_horizon_decisions": args.audit_horizon_decisions,
            "model": str(args.model),
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


def mean(values: Any) -> float:
    values = list(values)
    return sum(values) / len(values) if values else 0.0


def stderr(values: list[float]) -> float | None:
    if len(values) < 2:
        return None
    avg = mean(values)
    variance = sum((value - avg) ** 2 for value in values) / (len(values) - 1)
    return (variance ** 0.5) / (len(values) ** 0.5)


if __name__ == "__main__":
    main()
