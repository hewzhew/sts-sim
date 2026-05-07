#!/usr/bin/env python3
"""Audit whether candidate outcome packs support state-conditioned training.

This is a gate, not a dataset builder. It turns each pack into within-state
pairwise candidate comparisons, trains tiny dependency-free linear baselines,
and refuses to call the data useful unless the full state+candidate model beats
candidate-only, card/action-only, and state-only ablations under grouped splits.
"""
from __future__ import annotations

import argparse
import glob
import hashlib
import json
import math
import random
import re
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
REPORT_VERSION = "candidate_outcome_pack_trainability_audit_v0"
FEATURE_SETS = [
    "card_action_only",
    "candidate_only",
    "state_only",
    "full_state_plus_candidate",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run ablation training gates over combat candidate outcome packs."
    )
    parser.add_argument(
        "--input",
        nargs="+",
        required=True,
        help="Pack JSON files or glob patterns.",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=REPO_ROOT
        / "tools"
        / "artifacts"
        / "candidate_outcome_pack"
        / "trainability_audit.json",
    )
    parser.add_argument("--allow-truncated", action="store_true")
    parser.add_argument("--pairwise-only", action="store_true")
    parser.add_argument("--epochs", type=int, default=12)
    parser.add_argument("--learning-rate", type=float, default=0.08)
    parser.add_argument("--l2", type=float, default=0.000001)
    parser.add_argument("--hash-dim", type=int, default=32768)
    parser.add_argument("--seed", type=int, default=13)
    parser.add_argument("--min-test-pairs", type=int, default=100)
    parser.add_argument("--min-groups", type=int, default=80)
    parser.add_argument("--improvement-margin", type=float, default=0.02)
    return parser.parse_args()


def resolve(path: Path) -> Path:
    return path if path.is_absolute() else REPO_ROOT / path


def expand_inputs(patterns: list[str]) -> list[Path]:
    paths: list[Path] = []
    for pattern in patterns:
        expanded = glob.glob(str(resolve(Path(pattern)))) if has_glob(pattern) else [str(resolve(Path(pattern)))]
        for item in expanded:
            path = Path(item)
            if path.is_file():
                paths.append(path)
    return sorted(set(paths))


def has_glob(value: str) -> bool:
    return any(ch in value for ch in "*?[]")


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def is_candidate_pack(payload: dict[str, Any]) -> bool:
    return isinstance(payload.get("candidates"), list) and bool(payload.get("split_group_key"))


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, sort_keys=True)
        handle.write("\n")


def stable_group_split(group_key: str) -> str:
    digest = hashlib.sha256(group_key.encode("utf-8")).digest()
    bucket = int.from_bytes(digest[:8], "big") % 100
    if bucket < 80:
        return "train"
    if bucket < 90:
        return "valid"
    return "test"


def as_int(value: Any, default: int = 0) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def as_bool(value: Any) -> bool:
    return bool(value)


def candidate_utility(candidate: dict[str, Any]) -> tuple[int, ...]:
    """Explicit lexicographic protocol for current-turn outcome comparison.

    This is not declared to be the final game utility. It is only a transparent
    way to create auditable same-state pairwise labels from engine outcomes.
    """
    aggregate = candidate.get("outcome_aggregate") or {}
    return (
        1 if as_bool(aggregate.get("any_combat_cleared")) else 0,
        -1 if as_bool(aggregate.get("any_player_dead")) else 0,
        -as_int(aggregate.get("min_hp_lost")),
        as_int(aggregate.get("max_enemy_hp_reduction")),
        -as_int(aggregate.get("min_projected_unblocked_damage")),
        as_int(aggregate.get("max_final_block")),
        -as_int(aggregate.get("min_spent_potions")),
        -as_int(aggregate.get("min_exhausted_cards")),
    )


def build_pairwise_examples(
    packs: list[dict[str, Any]], allow_truncated: bool, pairwise_only: bool
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    examples: list[dict[str, Any]] = []
    stats: dict[str, Any] = {
        "packs": len(packs),
        "groups": 0,
        "candidate_count": 0,
        "included_candidates": 0,
        "skipped_ineligible_candidates": 0,
        "skipped_truncated_candidates": 0,
        "skipped_tie_pairs": 0,
        "pairwise_label_count": 0,
        "used_bounded_pairwise_labels": 0,
        "skipped_packs_without_pairwise_labels": 0,
        "split_groups": Counter(),
    }
    groups_seen: set[str] = set()

    for pack in packs:
        group_key = str(pack.get("split_group_key") or pack.get("source_trace") or "")
        groups_seen.add(group_key)
        split = stable_group_split(group_key)
        stats["split_groups"][split] += 1
        raw_candidates = pack.get("candidates") or []
        stats["candidate_count"] += len(raw_candidates)
        candidates_by_index = {
            int(candidate.get("candidate_index", idx)): candidate
            for idx, candidate in enumerate(raw_candidates)
        }
        pairwise_labels = pack.get("pairwise_labels") or []
        if pairwise_labels:
            stats["pairwise_label_count"] += len(pairwise_labels)
            for pair_label in pairwise_labels:
                preferred_index = as_int(pair_label.get("preferred_candidate_index"), -1)
                rejected_index = as_int(pair_label.get("rejected_candidate_index"), -1)
                preferred = candidates_by_index.get(preferred_index)
                rejected = candidates_by_index.get(rejected_index)
                if preferred is None or rejected is None:
                    continue
                objective = str(pair_label.get("objective") or "unknown")
                label_source = str(pair_label.get("label_source") or "pairwise_labels")
                append_symmetric_pair_examples(
                    examples,
                    group_key,
                    split,
                    pack,
                    preferred,
                    rejected,
                    objective,
                    label_source,
                )
                stats["used_bounded_pairwise_labels"] += 2
            stats["included_candidates"] += len(candidates_by_index)
            continue
        if pairwise_only:
            stats["skipped_packs_without_pairwise_labels"] += 1
            stats["included_candidates"] += len(candidates_by_index)
            continue

        candidates = []
        for candidate in raw_candidates:
            exact_turn = candidate.get("exact_turn") or {}
            oracle_quality = candidate.get("oracle_quality") or {}
            eligible = bool(oracle_quality.get("eligible_for_training", not exact_turn.get("truncated")))
            if not eligible and not allow_truncated:
                stats["skipped_ineligible_candidates"] += 1
                if exact_turn.get("truncated"):
                    stats["skipped_truncated_candidates"] += 1
                continue
            if exact_turn.get("truncated") and not allow_truncated:
                stats["skipped_truncated_candidates"] += 1
                continue
            candidates.append(candidate)
        stats["included_candidates"] += len(candidates)

        for left_index in range(len(candidates)):
            for right_index in range(left_index + 1, len(candidates)):
                left = candidates[left_index]
                right = candidates[right_index]
                left_utility = candidate_utility(left)
                right_utility = candidate_utility(right)
                if left_utility == right_utility:
                    stats["skipped_tie_pairs"] += 1
                    continue
                preferred = left if left_utility > right_utility else right
                rejected = right if left_utility > right_utility else left
                append_symmetric_pair_examples(
                    examples,
                    group_key,
                    split,
                    pack,
                    preferred,
                    rejected,
                    "lexicographic_exact_outcome",
                    "fallback_exact_outcome_utility",
                )

    stats["groups"] = len(groups_seen)
    stats["split_groups"] = dict(stats["split_groups"])
    stats["pair_count"] = len(examples)
    stats["pair_count_by_split"] = dict(Counter(example["split"] for example in examples))
    return examples, stats


def append_symmetric_pair_examples(
    examples: list[dict[str, Any]],
    group_key: str,
    split: str,
    pack: dict[str, Any],
    preferred: dict[str, Any],
    rejected: dict[str, Any],
    objective: str,
    label_source: str,
) -> None:
    examples.append(
        {
            "group_key": group_key,
            "split": split,
            "pack": pack,
            "left": preferred,
            "right": rejected,
            "label": 1,
            "objective": objective,
            "label_source": label_source,
        }
    )
    examples.append(
        {
            "group_key": group_key,
            "split": split,
            "pack": pack,
            "left": rejected,
            "right": preferred,
            "label": 0,
            "objective": objective,
            "label_source": label_source,
        }
    )


def action_key(candidate: dict[str, Any]) -> str:
    return str((candidate.get("candidate") or {}).get("action_key") or "")


def action_tokens(candidate: dict[str, Any], coarse: bool) -> list[str]:
    key = action_key(candidate)
    tokens: list[str] = []
    if key.startswith("combat/end_turn"):
        tokens.append("action:end_turn")
    elif key.startswith("combat/use_potion"):
        tokens.append("action:use_potion")
        if not coarse:
            tokens.extend(prefixed_key_parts(key, "potion_detail"))
    elif key.startswith("combat/play_card"):
        tokens.append("action:play_card")
        card = extract_segment(key, "card")
        target = extract_segment(key, "target")
        if card:
            tokens.append(f"card:{card}")
        if target and not coarse:
            tokens.append(f"target:{target}")
        if not coarse:
            tokens.extend(prefixed_key_parts(key, "action_key"))
    else:
        tokens.append(f"action:{key.split('/')[0] if key else 'unknown'}")
        if not coarse:
            tokens.extend(prefixed_key_parts(key, "action_key"))
    return sorted(set(tokens))


def extract_segment(key: str, name: str) -> str:
    match = re.search(rf"(?:^|/){re.escape(name)}:([^/]+)", key)
    return match.group(1) if match else ""


def prefixed_key_parts(key: str, prefix: str) -> list[str]:
    parts = [part for part in re.split(r"[^A-Za-z0-9_+.-]+", key) if part]
    return [f"{prefix}:{part}" for part in parts]


def state_tokens(pack: dict[str, Any]) -> list[str]:
    tokens: list[str] = []
    observation = pack.get("observation") or {}
    combat = observation.get("combat") or {}
    start = pack.get("start_outcome") or {}
    deck = observation.get("deck") or {}

    add_bucket(tokens, "act", as_int(observation.get("act")), 1)
    add_bucket(tokens, "floor", as_int(observation.get("floor")), 5)
    add_bucket(tokens, "hp", as_int(observation.get("current_hp")), 10)
    add_bucket(tokens, "hp_ratio", as_int(observation.get("hp_ratio_milli")), 100)
    tokens.append(f"room:{observation.get('current_room', 'unknown')}")

    add_bucket(tokens, "combat_energy", as_int(combat.get("energy")), 1)
    add_bucket(tokens, "combat_turn", as_int(combat.get("turn_count")), 1)
    add_bucket(tokens, "hand_count", as_int(combat.get("hand_count")), 1)
    add_bucket(tokens, "player_block", as_int(combat.get("player_block")), 5)

    add_bucket(tokens, "incoming", as_int(start.get("visible_incoming_damage")), 5)
    add_bucket(tokens, "projected_unblocked", as_int(start.get("projected_unblocked_damage")), 5)
    add_bucket(tokens, "monster_hp", as_int(start.get("total_monster_hp")), 10)
    add_bucket(tokens, "living_monsters", as_int(start.get("living_monster_count")), 1)

    for key in [
        "attack_count",
        "skill_count",
        "power_count",
        "damage_card_count",
        "block_card_count",
        "draw_card_count",
        "scaling_card_count",
        "exhaust_card_count",
        "starter_basic_count",
    ]:
        add_bucket(tokens, f"deck_{key}", as_int(deck.get(key)), 2)

    for hand_card in combat.get("hand_cards") or []:
        card_id = hand_card.get("card_id")
        if card_id:
            tokens.append(f"hand_card:{card_id}")
        if hand_card.get("playable"):
            tokens.append("hand_has_playable")

    for power in start.get("player_powers") or []:
        power_id = power.get("power_id")
        if power_id:
            add_bucket(tokens, f"player_power_{power_id}", as_int(power.get("amount")), 1)

    for monster in start.get("monsters") or []:
        monster_id = monster.get("monster_id")
        if monster_id:
            tokens.append(f"monster:{monster_id}")
        add_bucket(tokens, "monster_incoming_each", as_int(monster.get("visible_incoming_damage")), 5)
        for power in monster.get("powers") or []:
            power_id = power.get("power_id")
            if power_id:
                add_bucket(tokens, f"monster_power_{power_id}", as_int(power.get("amount")), 1)

    return sorted(set(tokens))


def add_bucket(tokens: list[str], name: str, value: int, width: int) -> None:
    width = max(width, 1)
    tokens.append(f"{name}_bucket:{math.floor(value / width)}")


def candidate_tokens_for_feature_set(
    pack: dict[str, Any], candidate: dict[str, Any], feature_set: str
) -> Counter[str]:
    features: Counter[str] = Counter()
    if feature_set == "card_action_only":
        for token in action_tokens(candidate, coarse=True):
            features[token] += 1.0
    elif feature_set == "candidate_only":
        for token in action_tokens(candidate, coarse=False):
            features[token] += 1.0
    elif feature_set == "state_only":
        for token in state_tokens(pack):
            features[f"state:{token}"] += 1.0
    elif feature_set == "full_state_plus_candidate":
        state = state_tokens(pack)
        action = action_tokens(candidate, coarse=False)
        for token in state:
            features[f"state:{token}"] += 1.0
        for token in action:
            features[f"candidate:{token}"] += 1.0
        for state_token in state[:96]:
            for action_token in action[:32]:
                features[f"cross:{state_token}|{action_token}"] += 1.0
    else:
        raise ValueError(f"unknown feature set {feature_set}")
    return features


def pair_features(example: dict[str, Any], feature_set: str) -> dict[int, float]:
    left = candidate_tokens_for_feature_set(example["pack"], example["left"], feature_set)
    right = candidate_tokens_for_feature_set(example["pack"], example["right"], feature_set)
    sparse: defaultdict[int, float] = defaultdict(float)
    for token, value in left.items():
        sparse[hash_feature(token)] += value
    for token, value in right.items():
        sparse[hash_feature(token)] -= value
    return {idx: value for idx, value in sparse.items() if value}


def hash_feature(token: str, dim: int = 32768) -> int:
    digest = hashlib.blake2b(token.encode("utf-8"), digest_size=8).digest()
    return int.from_bytes(digest, "big") % dim


def vectorize_examples(
    examples: list[dict[str, Any]], feature_set: str, dim: int
) -> list[tuple[dict[int, float], int]]:
    rows = []
    for example in examples:
        sparse: defaultdict[int, float] = defaultdict(float)
        left = candidate_tokens_for_feature_set(example["pack"], example["left"], feature_set)
        right = candidate_tokens_for_feature_set(example["pack"], example["right"], feature_set)
        for token, value in left.items():
            sparse[hash_feature(token, dim)] += value
        for token, value in right.items():
            sparse[hash_feature(token, dim)] -= value
        sparse[hash_feature(f"objective:{example.get('objective', 'unknown')}", dim)] += 1.0
        sparse[hash_feature(f"label_source:{example.get('label_source', 'unknown')}", dim)] += 1.0
        rows.append(({idx: value for idx, value in sparse.items() if value}, int(example["label"])))
    return rows


def train_logistic(
    train_rows: list[tuple[dict[int, float], int]],
    eval_rows: dict[str, list[tuple[dict[int, float], int]]],
    dim: int,
    epochs: int,
    learning_rate: float,
    l2: float,
    seed: int,
) -> dict[str, Any]:
    rng = random.Random(seed)
    weights = [0.0] * dim
    bias = 0.0
    rows = list(train_rows)
    for _ in range(max(epochs, 0)):
        rng.shuffle(rows)
        for sparse, label in rows:
            prediction = sigmoid(bias + dot(weights, sparse))
            error = prediction - label
            bias -= learning_rate * error
            for idx, value in sparse.items():
                weights[idx] -= learning_rate * (error * value + l2 * weights[idx])

    metrics = {
        split: evaluate_rows(rows_for_split, weights, bias)
        for split, rows_for_split in eval_rows.items()
    }
    return {
        "nonzero_weights": sum(1 for weight in weights if abs(weight) > 1e-12),
        "bias": bias,
        "metrics": metrics,
    }


def dot(weights: list[float], sparse: dict[int, float]) -> float:
    return sum(weights[idx] * value for idx, value in sparse.items())


def sigmoid(value: float) -> float:
    if value < -40:
        return 0.0
    if value > 40:
        return 1.0
    return 1.0 / (1.0 + math.exp(-value))


def evaluate_rows(rows: list[tuple[dict[int, float], int]], weights: list[float], bias: float) -> dict[str, Any]:
    if not rows:
        return {"count": 0, "accuracy": None, "log_loss": None, "positive_rate": None}
    correct = 0
    loss = 0.0
    positives = 0
    for sparse, label in rows:
        prediction = min(max(sigmoid(bias + dot(weights, sparse)), 1e-9), 1 - 1e-9)
        predicted_label = 1 if prediction >= 0.5 else 0
        correct += 1 if predicted_label == label else 0
        positives += label
        loss += -(label * math.log(prediction) + (1 - label) * math.log(1 - prediction))
    return {
        "count": len(rows),
        "accuracy": correct / len(rows),
        "log_loss": loss / len(rows),
        "positive_rate": positives / len(rows),
    }


def majority_metrics(rows: list[tuple[dict[int, float], int]]) -> dict[str, Any]:
    if not rows:
        return {"count": 0, "accuracy": None, "log_loss": None, "positive_rate": None}
    positives = sum(label for _, label in rows)
    majority = max(positives, len(rows) - positives)
    positive_rate = positives / len(rows)
    prediction = min(max(positive_rate, 1e-9), 1 - 1e-9)
    loss = 0.0
    for _, label in rows:
        loss += -(label * math.log(prediction) + (1 - label) * math.log(1 - prediction))
    return {
        "count": len(rows),
        "accuracy": majority / len(rows),
        "log_loss": loss / len(rows),
        "positive_rate": positive_rate,
    }


def gate_report(
    stats: dict[str, Any],
    metrics_by_feature_set: dict[str, Any],
    majority_test: dict[str, Any],
    min_groups: int,
    min_test_pairs: int,
    improvement_margin: float,
) -> dict[str, Any]:
    failures = []
    if stats["groups"] < min_groups:
        failures.append(f"group_count {stats['groups']} < min_groups {min_groups}")
    test_pairs = stats.get("pair_count_by_split", {}).get("test", 0)
    if test_pairs < min_test_pairs:
        failures.append(f"test_pair_count {test_pairs} < min_test_pairs {min_test_pairs}")

    full_test = (
        metrics_by_feature_set.get("full_state_plus_candidate", {})
        .get("metrics", {})
        .get("test", {})
    )
    full_accuracy = full_test.get("accuracy")
    full_log_loss = full_test.get("log_loss")
    majority_accuracy = majority_test.get("accuracy")
    majority_log_loss = majority_test.get("log_loss")
    if full_accuracy is None or majority_accuracy is None:
        failures.append("missing full-state or majority heldout metric")
    elif full_accuracy < majority_accuracy + improvement_margin:
        failures.append(
            "full_state_plus_candidate does not beat majority by required margin"
        )
    if full_log_loss is None or majority_log_loss is None:
        failures.append("missing full-state or majority heldout log_loss")
    elif full_log_loss > majority_log_loss:
        failures.append("full_state_plus_candidate log_loss is worse than majority")

    for baseline in ["candidate_only", "card_action_only", "state_only"]:
        baseline_test = metrics_by_feature_set.get(baseline, {}).get("metrics", {}).get("test", {})
        baseline_accuracy = baseline_test.get("accuracy")
        baseline_log_loss = baseline_test.get("log_loss")
        if full_accuracy is None or baseline_accuracy is None:
            failures.append(f"missing {baseline} heldout metric")
        elif full_accuracy < baseline_accuracy + improvement_margin:
            failures.append(
                f"full_state_plus_candidate does not beat {baseline} by required margin"
            )
        if full_log_loss is None or baseline_log_loss is None:
            failures.append(f"missing {baseline} heldout log_loss")
        elif full_log_loss > baseline_log_loss:
            failures.append(f"full_state_plus_candidate log_loss is worse than {baseline}")

    return {
        "offline_trainability_gate_passed": not failures,
        "failures": failures,
        "acceptance_requires_closed_loop": True,
        "accepted_for_main_training": False,
        "closed_loop_gate": "not_evaluated_by_this_script",
    }


def feedback_report(
    stats: dict[str, Any],
    metrics_by_feature_set: dict[str, Any],
    gate: dict[str, Any],
    improvement_margin: float,
) -> list[str]:
    feedback = []
    candidate_count = stats.get("candidate_count", 0) or 0
    skipped_ineligible = stats.get("skipped_ineligible_candidates", 0) or 0
    if candidate_count and skipped_ineligible / candidate_count >= 0.40:
        feedback.append(
            "oracle_not_ready_high_ineligible_rate: do not train; inspect truncation reasons and search budgets"
        )

    pair_count = stats.get("pair_count", 0) or 0
    skipped_ties = stats.get("skipped_tie_pairs", 0) or 0
    if pair_count and skipped_ties > pair_count:
        feedback.append(
            "many_pairwise_ties: current utility protocol may not separate candidates enough"
        )

    full_test = (
        metrics_by_feature_set.get("full_state_plus_candidate", {})
        .get("metrics", {})
        .get("test", {})
    )
    full_accuracy = full_test.get("accuracy")
    for baseline in ["candidate_only", "card_action_only", "state_only"]:
        baseline_accuracy = (
            metrics_by_feature_set.get(baseline, {})
            .get("metrics", {})
            .get("test", {})
            .get("accuracy")
        )
        if full_accuracy is None or baseline_accuracy is None:
            continue
        if full_accuracy < baseline_accuracy + improvement_margin:
            feedback.append(
                f"state_conditioning_not_proven_against_{baseline}: do not promote dataset"
            )

    if gate.get("offline_trainability_gate_passed"):
        feedback.append(
            "offline_gate_passed_only: run closed-loop engine selection before main training promotion"
        )
    return feedback


def main() -> None:
    args = parse_args()
    input_paths = expand_inputs(args.input)
    raw_inputs = [(path, read_json(path)) for path in input_paths]
    packs = [payload for _, payload in raw_inputs if is_candidate_pack(payload)]
    skipped_non_pack_inputs = [
        str(path) for path, payload in raw_inputs if not is_candidate_pack(payload)
    ]
    examples, stats = build_pairwise_examples(packs, args.allow_truncated, args.pairwise_only)
    examples_by_split = {
        split: [example for example in examples if example["split"] == split]
        for split in ["train", "valid", "test"]
    }

    metrics_by_feature_set: dict[str, Any] = {}
    vectorized_by_feature: dict[str, dict[str, list[tuple[dict[int, float], int]]]] = {}
    for feature_set in FEATURE_SETS:
        vectorized_by_feature[feature_set] = {
            split: vectorize_examples(split_examples, feature_set, args.hash_dim)
            for split, split_examples in examples_by_split.items()
        }
        if vectorized_by_feature[feature_set]["train"]:
            metrics_by_feature_set[feature_set] = train_logistic(
                vectorized_by_feature[feature_set]["train"],
                vectorized_by_feature[feature_set],
                args.hash_dim,
                args.epochs,
                args.learning_rate,
                args.l2,
                args.seed,
            )
        else:
            metrics_by_feature_set[feature_set] = {
                "nonzero_weights": 0,
                "bias": 0.0,
                "metrics": {
                    split: evaluate_rows(rows, [0.0] * args.hash_dim, 0.0)
                    for split, rows in vectorized_by_feature[feature_set].items()
                },
            }

    majority = {
        split: majority_metrics(vectorized_by_feature["candidate_only"][split])
        for split in ["train", "valid", "test"]
    }
    gate = gate_report(
        stats,
        metrics_by_feature_set,
        majority["test"],
        args.min_groups,
        args.min_test_pairs,
        args.improvement_margin,
    )
    feedback = feedback_report(
        stats,
        metrics_by_feature_set,
        gate,
        args.improvement_margin,
    )
    report = {
        "report_version": REPORT_VERSION,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "input_files": [str(path) for path in input_paths],
        "skipped_non_pack_inputs": skipped_non_pack_inputs,
        "config": {
            "allow_truncated": args.allow_truncated,
            "pairwise_only": args.pairwise_only,
            "epochs": args.epochs,
            "learning_rate": args.learning_rate,
            "l2": args.l2,
            "hash_dim": args.hash_dim,
            "seed": args.seed,
            "min_test_pairs": args.min_test_pairs,
            "min_groups": args.min_groups,
            "improvement_margin": args.improvement_margin,
        },
        "stats": stats,
        "majority": majority,
        "metrics_by_feature_set": metrics_by_feature_set,
        "gate": gate,
        "feedback": feedback,
    }
    write_json(resolve(args.out), report)
    print(
        json.dumps(
            {
                "report_version": REPORT_VERSION,
                "out": str(resolve(args.out)),
                "pairs": stats["pair_count"],
                "pair_count_by_split": stats["pair_count_by_split"],
                "offline_trainability_gate_passed": gate["offline_trainability_gate_passed"],
                "accepted_for_main_training": gate["accepted_for_main_training"],
                "failures": gate["failures"],
                "feedback": feedback,
            },
            indent=2,
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
