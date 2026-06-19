#!/usr/bin/env python3
"""Dependency-free first-action ranking baseline for combat search guidance.

Input is CombatSearchGuidanceSampleV1 JSONL produced from decision microscope
reports by combat_search_guidance_samples.py.

This is an offline diagnostic.  It does not train a combat policy and does not
claim the selected action is human-optimal.  The label is only:
"the first action of the best complete trajectory found by current search under
this budget".
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import random
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


TARGET_KIND = "initial_decision_candidate_selected_by_best_complete"
SCHEMA_NAME = "CombatSearchGuidanceSampleV1"


def stable_hash(text: str) -> int:
    return int(hashlib.sha256(text.encode("utf-8")).hexdigest()[:16], 16)


def load_samples(paths: list[Path]) -> list[dict[str, Any]]:
    samples: list[dict[str, Any]] = []
    for path in paths:
        with path.open("r", encoding="utf-8") as handle:
            for line_no, line in enumerate(handle, start=1):
                stripped = line.strip()
                if not stripped:
                    continue
                try:
                    sample = json.loads(stripped)
                except json.JSONDecodeError as exc:
                    raise SystemExit(f"{path}:{line_no}: invalid JSONL: {exc}") from exc
                if sample.get("schema_name") != SCHEMA_NAME:
                    raise SystemExit(
                        f"{path}:{line_no}: expected {SCHEMA_NAME}, got {sample.get('schema_name')!r}"
                    )
                if sample.get("target_kind") == TARGET_KIND:
                    sample["_source_jsonl"] = str(path)
                    samples.append(sample)
    return samples


def group_key(sample: dict[str, Any]) -> str:
    source = sample.get("source") or {}
    context = sample.get("search_context") or {}
    return "|".join(
        str(part)
        for part in (
            source.get("file"),
            source.get("case_id"),
            context.get("max_nodes"),
            context.get("wall_time_ms"),
            context.get("rollout_policy"),
            context.get("frontier_policy"),
        )
    )


def grouped_samples(samples: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    groups: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for sample in samples:
        groups[group_key(sample)].append(sample)
    return dict(groups)


def usable_groups(samples: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    groups = {}
    for key, group in grouped_samples(samples).items():
        positives = sum(is_selected(sample) for sample in group)
        if positives == 1 and len(group) >= 2:
            groups[key] = sorted(group, key=lambda sample: candidate(sample).get("ordered_index", 0))
    return groups


def is_selected(sample: dict[str, Any]) -> bool:
    return bool((sample.get("label") or {}).get("selected_by_best_complete"))


def candidate(sample: dict[str, Any]) -> dict[str, Any]:
    value = sample.get("candidate")
    return value if isinstance(value, dict) else {}


def nested_get(root: dict[str, Any], path: str) -> Any:
    current: Any = root
    for part in path.split("."):
        if not isinstance(current, dict):
            return None
        current = current.get(part)
    return current


def add_token(features: dict[str, float], token: str, value: float = 1.0) -> None:
    if token and not token.endswith(":None"):
        features[token] += value


def add_number(features: dict[str, float], name: str, value: Any, scale: float) -> None:
    if isinstance(value, bool):
        value = int(value)
    if not isinstance(value, (int, float)):
        return
    numeric = float(value)
    features[f"num:{name}"] += numeric / scale
    bucket = int(math.floor(numeric / scale * 10.0))
    add_token(features, f"bin:{name}:{bucket}")


CARD_IN_ACTION_RE = re.compile(r"/card:([^/#]+?)(?:#|/)")
HAND_IN_ACTION_RE = re.compile(r"/hand:(\d+)")
TARGET_IN_ACTION_RE = re.compile(r"/target:([^/]+)")


def normalized_card_from_action_key(action_key: str) -> str | None:
    match = CARD_IN_ACTION_RE.search(action_key)
    if not match:
        return None
    card = match.group(1)
    card = re.sub(r"\+\d+$", "+", card)
    return card


def extract_features(sample: dict[str, Any], *, include_order_features: bool) -> dict[str, float]:
    features: dict[str, float] = defaultdict(float)
    cand = candidate(sample)
    context = sample.get("initial_context") if isinstance(sample.get("initial_context"), dict) else {}
    state = context.get("state") if isinstance(context.get("state"), dict) else {}
    frontier = context.get("frontier_value") if isinstance(context.get("frontier_value"), dict) else {}
    search = sample.get("search_context") if isinstance(sample.get("search_context"), dict) else {}
    one_step = cand.get("one_step") if isinstance(cand.get("one_step"), dict) else {}
    action_key = str(cand.get("action_key") or "")

    add_token(features, "bias")
    add_token(features, f"action_class:{cand.get('action_class')}")
    add_token(features, f"action_role:{cand.get('action_role')}")
    add_token(features, f"rollout_policy:{search.get('rollout_policy')}")
    add_token(features, f"frontier_policy:{search.get('frontier_policy')}")
    add_token(features, f"one_step_status:{one_step.get('status')}")
    add_token(features, f"one_step_terminal:{one_step.get('terminal')}")
    add_token(features, f"one_step_transition:{one_step.get('transition')}")

    normalized_card = normalized_card_from_action_key(action_key)
    if normalized_card:
        add_token(features, f"card:{normalized_card}")
    target_match = TARGET_IN_ACTION_RE.search(action_key)
    if target_match:
        add_token(features, f"target:{target_match.group(1).split(':')[0]}")
    hand_match = HAND_IN_ACTION_RE.search(action_key)
    if include_order_features and hand_match:
        add_number(features, "hand_index", int(hand_match.group(1)), 10.0)
    if include_order_features:
        add_number(features, "ordered_index", cand.get("ordered_index"), 24.0)
        add_number(features, "original_action_id", cand.get("original_action_id"), 24.0)

    for path, scale in (
        ("player_hp", 100.0),
        ("player_block", 80.0),
        ("energy", 6.0),
        ("visible_incoming_damage", 80.0),
        ("visible_hp_loss_if_turn_ends", 80.0),
        ("survival_margin", 100.0),
        ("living_enemy_count", 5.0),
        ("total_enemy_hp", 300.0),
        ("total_enemy_block", 150.0),
        ("phase_adjusted_enemy_effort", 400.0),
        ("split_debt_hp", 200.0),
        ("turn_branch_priority_hint", 20.0),
        ("pending_choice_estimated_action_fanout", 50.0),
        ("gremlin_nob_anger_amount_total", 30.0),
        ("guardian_mode_shift_pending_count", 5.0),
        ("lagavulin_waking_count", 5.0),
        ("sentry_dazed_pressure_count", 10.0),
        ("hexaghost_opening_pressure_count", 5.0),
    ):
        add_number(features, f"one_step_{path}", one_step.get(path), scale)

    for path, scale in (
        ("player_hp", 100.0),
        ("player_block", 80.0),
        ("energy", 6.0),
        ("living_enemy_count", 5.0),
        ("total_enemy_hp", 300.0),
        ("visible_incoming_damage", 80.0),
        ("hand_count", 12.0),
        ("draw_count", 40.0),
        ("discard_count", 40.0),
        ("exhaust_count", 40.0),
    ):
        add_number(features, f"state_{path}", state.get(path), scale)

    for path, scale in (
        ("hand.damage", 100.0),
        ("hand.block", 100.0),
        ("hand.playable_cards", 10.0),
        ("next_draw.damage", 100.0),
        ("next_draw.block", 100.0),
        ("next_draw.playable_cards", 10.0),
        ("phase_adjusted_enemy_effort", 400.0),
        ("survival_margin", 100.0),
        ("sustained_mitigation", 50.0),
        ("gremlin_nob_anger_amount_total", 30.0),
        ("guardian_mode_shift_pending_count", 5.0),
    ):
        add_number(features, f"frontier_{path}", nested_get(frontier, path), scale)

    return dict(features)


def hashed_features(features: dict[str, float], dim: int) -> dict[int, float]:
    out: dict[int, float] = defaultdict(float)
    for key, value in features.items():
        index = stable_hash(key) % dim
        sign = -1.0 if stable_hash("sign:" + key) % 2 else 1.0
        out[index] += sign * value
    return dict(out)


def dot(weights: dict[int, float], features: dict[int, float], bias: float) -> float:
    return bias + sum(weights.get(index, 0.0) * value for index, value in features.items())


def sigmoid(value: float) -> float:
    if value >= 0:
        z = math.exp(-value)
        return 1.0 / (1.0 + z)
    z = math.exp(value)
    return z / (1.0 + z)


def split_groups(
    groups: dict[str, list[dict[str, Any]]], test_ratio: float
) -> tuple[dict[str, list[dict[str, Any]]], dict[str, list[dict[str, Any]]]]:
    train = {}
    test = {}
    for key, group in groups.items():
        bucket = stable_hash(key) % 10_000
        if bucket < int(test_ratio * 10_000):
            test[key] = group
        else:
            train[key] = group
    if not train and test:
        key = sorted(test)[0]
        train[key] = test.pop(key)
    if not test and len(train) > 1:
        key = sorted(train)[-1]
        test[key] = train.pop(key)
    return train, test


def flatten_training_examples(
    groups: dict[str, list[dict[str, Any]]],
    *,
    include_order_features: bool,
) -> list[tuple[int, dict[str, float]]]:
    examples = []
    for group in groups.values():
        for sample in group:
            label = 1 if is_selected(sample) else 0
            features = extract_features(sample, include_order_features=include_order_features)
            examples.append((label, features))
    return examples


def train_logistic(
    examples: list[tuple[int, dict[str, float]]],
    *,
    dim: int,
    epochs: int,
    learning_rate: float,
    l2: float,
    seed: int,
) -> tuple[dict[int, float], float]:
    rng = random.Random(seed)
    weights: dict[int, float] = defaultdict(float)
    bias = 0.0
    hashed = [(label, hashed_features(features, dim)) for label, features in examples]
    for _ in range(epochs):
        rng.shuffle(hashed)
        for label, features in hashed:
            pred = sigmoid(dot(weights, features, bias))
            error = pred - label
            bias -= learning_rate * error
            for index, value in features.items():
                weights[index] -= learning_rate * (error * value + l2 * weights[index])
    return dict(weights), bias


def selected_rank(group: list[dict[str, Any]], scores: list[float]) -> int:
    ranked = sorted(zip(group, scores), key=lambda item: item[1], reverse=True)
    for rank, (sample, _) in enumerate(ranked, start=1):
        if is_selected(sample):
            return rank
    return len(group) + 1


def evaluate_ordered_index(groups: dict[str, list[dict[str, Any]]]) -> dict[str, float]:
    ranks = []
    for group in groups.values():
        scores = [-(candidate(sample).get("ordered_index") or 0) for sample in group]
        ranks.append(selected_rank(group, scores))
    return metrics_from_ranks(groups, ranks)


def evaluate_model(
    groups: dict[str, list[dict[str, Any]]],
    weights: dict[int, float],
    bias: float,
    *,
    dim: int,
    include_order_features: bool,
) -> dict[str, float]:
    ranks = []
    for group in groups.values():
        scores = []
        for sample in group:
            features = extract_features(sample, include_order_features=include_order_features)
            scores.append(dot(weights, hashed_features(features, dim), bias))
        ranks.append(selected_rank(group, scores))
    return metrics_from_ranks(groups, ranks)


def metrics_from_ranks(groups: dict[str, list[dict[str, Any]]], ranks: list[int]) -> dict[str, float]:
    if not ranks:
        return {"groups": 0.0, "top1": 0.0, "mrr": 0.0, "avg_rank": 0.0}
    return {
        "groups": float(len(ranks)),
        "top1": sum(1 for rank in ranks if rank == 1) / len(ranks),
        "mrr": sum(1.0 / rank for rank in ranks) / len(ranks),
        "avg_rank": sum(ranks) / len(ranks),
        "avg_candidates": sum(len(group) for group in groups.values()) / len(groups),
    }


def feature_weight_report(
    weights: dict[int, float],
    groups: dict[str, list[dict[str, Any]]],
    *,
    dim: int,
    include_order_features: bool,
    limit: int,
) -> list[tuple[str, float]]:
    bucket_to_names: dict[int, Counter[str]] = defaultdict(Counter)
    for group in groups.values():
        for sample in group:
            features = extract_features(sample, include_order_features=include_order_features)
            for name in features:
                bucket_to_names[stable_hash(name) % dim][name] += 1
    ranked = sorted(weights.items(), key=lambda item: abs(item[1]), reverse=True)
    out = []
    for bucket, weight in ranked[:limit]:
        if bucket_to_names[bucket]:
            name = bucket_to_names[bucket].most_common(1)[0][0]
        else:
            name = f"hash_bucket:{bucket}"
        out.append((name, weight))
    return out


def print_metrics(label: str, metrics: dict[str, float]) -> None:
    print(
        f"  {label}: groups={metrics['groups']:.0f} top1={metrics['top1']:.3f} "
        f"mrr={metrics['mrr']:.3f} avg_rank={metrics['avg_rank']:.2f} "
        f"avg_candidates={metrics.get('avg_candidates', 0.0):.2f}"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="+", type=Path, help="CombatSearchGuidanceSampleV1 JSONL")
    parser.add_argument("--dim", type=int, default=4096)
    parser.add_argument("--epochs", type=int, default=25)
    parser.add_argument("--learning-rate", type=float, default=0.05)
    parser.add_argument("--l2", type=float, default=0.0005)
    parser.add_argument("--test-ratio", type=float, default=0.3)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument(
        "--include-order-features",
        action="store_true",
        help="Allow ordered_index/original_action_id/hand_index as features",
    )
    parser.add_argument("--top-features", type=int, default=12)
    args = parser.parse_args()

    samples = load_samples(args.inputs)
    groups = usable_groups(samples)
    target_counts = Counter()
    for group in groups.values():
        for sample in group:
            target_counts["selected" if is_selected(sample) else "not_selected"] += 1
    print("CombatFirstActionRankingBaseline")
    print(f"  samples={len(samples)} usable_groups={len(groups)} labels={dict(target_counts)}")
    print(
        "  label_role=oracle_search_guidance_first_action_not_human_policy "
        "candidate_coverage=root_legal_candidates_reported_limit"
    )
    if len(groups) < 8:
        print("  readiness=too_few_groups_for_meaningful_ml")
    else:
        print("  readiness=small_offline_ranking_probe")
    if not groups:
        return

    train_groups, test_groups = split_groups(groups, args.test_ratio)
    print(f"  split=train_groups:{len(train_groups)} test_groups:{len(test_groups)}")
    print_metrics("ordered_index_train", evaluate_ordered_index(train_groups))
    print_metrics("ordered_index_test", evaluate_ordered_index(test_groups))

    train_examples = flatten_training_examples(
        train_groups,
        include_order_features=args.include_order_features,
    )
    if not train_examples or not test_groups:
        print("  logistic=skipped_not_enough_split_data")
        return
    weights, bias = train_logistic(
        train_examples,
        dim=args.dim,
        epochs=args.epochs,
        learning_rate=args.learning_rate,
        l2=args.l2,
        seed=args.seed,
    )
    print_metrics(
        "logistic_train",
        evaluate_model(
            train_groups,
            weights,
            bias,
            dim=args.dim,
            include_order_features=args.include_order_features,
        ),
    )
    print_metrics(
        "logistic_test",
        evaluate_model(
            test_groups,
            weights,
            bias,
            dim=args.dim,
            include_order_features=args.include_order_features,
        ),
    )
    print("  top_weighted_features:")
    for name, weight in feature_weight_report(
        weights,
        train_groups,
        dim=args.dim,
        include_order_features=args.include_order_features,
        limit=args.top_features,
    ):
        print(f"    {weight:+.4f} {name}")


if __name__ == "__main__":
    main()
