#!/usr/bin/env python3
"""Dependency-free ranking baseline for combat search guidance.

Input is either:

- CombatSearchGuidanceSampleV1 JSONL produced from decision microscope reports
  by combat_search_guidance_samples.py.
- CombatActionProbeSampleV1 JSONL produced from guidance-lab reports by
  combat_guidance_lab_extract.py.
- CombatTurnPlanProbeSampleV1 JSONL produced from turn-plan guidance-lab
  reports by combat_turn_plan_guidance_lab_extract.py.

This is an offline diagnostic.  It does not train a combat policy and does not
claim the selected action is human-optimal. Targets are oracle-under-budget
labels produced by the current search/probe pipeline.
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
LEGACY_SCHEMA_NAME = "CombatSearchGuidanceSampleV1"
PROBE_SCHEMA_NAME = "CombatActionProbeSampleV1"
TURN_PLAN_SCHEMA_NAME = "CombatTurnPlanProbeSampleV1"


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
                schema_name = sample.get("schema_name")
                if schema_name == LEGACY_SCHEMA_NAME:
                    if sample.get("target_kind") != TARGET_KIND:
                        continue
                    sample["_source_jsonl"] = str(path)
                    samples.append(sample)
                elif schema_name in (PROBE_SCHEMA_NAME, TURN_PLAN_SCHEMA_NAME):
                    sample["_source_jsonl"] = str(path)
                    samples.append(sample)
                else:
                    raise SystemExit(
                        f"{path}:{line_no}: expected {LEGACY_SCHEMA_NAME} or "
                        f"{PROBE_SCHEMA_NAME} or {TURN_PLAN_SCHEMA_NAME}, got {schema_name!r}"
                    )
    return samples


def discover_turn_plan_probe_paths(roots: list[Path]) -> list[Path]:
    explicit_files: list[Path] = []
    discovered_by_key: dict[str, Path] = {}
    for root in roots:
        if root.is_file():
            explicit_files.append(root)
            continue
        if not root.exists():
            raise SystemExit(f"discover root does not exist: {root}")
        for path in root.rglob("*.turn_plan_probe*.jsonl"):
            key = turn_plan_probe_discovery_key(path)
            previous = discovered_by_key.get(key)
            if previous is None or path.stat().st_mtime > previous.stat().st_mtime:
                discovered_by_key[key] = path
    return sorted(set(explicit_files + list(discovered_by_key.values())))


def turn_plan_probe_discovery_key(path: Path) -> str:
    name = path.name
    for suffix in (".turn_plan_probe_batch.jsonl", ".turn_plan_probe.jsonl"):
        if name.endswith(suffix):
            return f"{path.parent}|{name.removesuffix(suffix)}"
    return str(path)


def is_root_context_schema(sample: dict[str, Any]) -> bool:
    return sample.get("schema_name") in (PROBE_SCHEMA_NAME, TURN_PLAN_SCHEMA_NAME)


def is_turn_plan_sample(sample: dict[str, Any]) -> bool:
    return sample.get("schema_name") == TURN_PLAN_SCHEMA_NAME


def group_key(sample: dict[str, Any]) -> str:
    if is_root_context_schema(sample):
        source = sample.get("source") or {}
        context = (sample.get("root_context") or {}).get("config") or {}
        enumeration = (sample.get("root_context") or {}).get("enumeration") or {}
        return "|".join(
            str(part)
            for part in (
                sample.get("schema_name"),
                source.get("source_file"),
                source.get("benchmark_name"),
                source.get("case_id"),
                source.get("input_kind"),
                source.get("input_path"),
                context.get("max_nodes"),
                context.get("wall_time_ms"),
                context.get("max_inner_nodes"),
                context.get("max_end_states"),
                context.get("per_bucket_limit"),
                context.get("rollout_policy"),
                context.get("frontier_policy"),
                enumeration.get("planning_policy"),
            )
        )
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
            groups[key] = sorted(group, key=sample_ordered_index)
    return groups


def is_selected(sample: dict[str, Any]) -> bool:
    if sample.get("schema_name") == PROBE_SCHEMA_NAME:
        target = sample.get("target") if isinstance(sample.get("target"), dict) else {}
        return bool(target.get("is_best_target_candidate"))
    if is_turn_plan_sample(sample):
        target = sample.get("target") if isinstance(sample.get("target"), dict) else {}
        return bool(target.get("is_best_target_plan"))
    return bool((sample.get("label") or {}).get("selected_by_best_complete"))


def candidate(sample: dict[str, Any]) -> dict[str, Any]:
    if is_turn_plan_sample(sample):
        value = sample.get("plan")
        return value if isinstance(value, dict) else {}
    value = sample.get("candidate")
    return value if isinstance(value, dict) else {}


def initial_context(sample: dict[str, Any]) -> dict[str, Any]:
    if is_root_context_schema(sample):
        context = (sample.get("root_context") or {}).get("initial_context")
        return context if isinstance(context, dict) else {}
    context = sample.get("initial_context")
    return context if isinstance(context, dict) else {}


def search_context(sample: dict[str, Any]) -> dict[str, Any]:
    if is_root_context_schema(sample):
        context = (sample.get("root_context") or {}).get("config")
        return context if isinstance(context, dict) else {}
    context = sample.get("search_context")
    return context if isinstance(context, dict) else {}


def one_step_context(sample: dict[str, Any]) -> dict[str, Any]:
    cand = candidate(sample)
    one_step = cand.get("one_step") if isinstance(cand.get("one_step"), dict) else {}
    if one_step:
        return one_step
    if sample.get("schema_name") == PROBE_SCHEMA_NAME:
        return {
            "status": cand.get("one_step_status"),
            "terminal": cand.get("one_step_terminal"),
        }
    return {}


def candidate_action_key(sample: dict[str, Any]) -> str:
    cand = candidate(sample)
    if is_turn_plan_sample(sample):
        return str(cand.get("first_action_key") or "")
    return str(cand.get("action_key") or "")


def sample_ordered_index(sample: dict[str, Any]) -> int:
    cand = candidate(sample)
    if is_turn_plan_sample(sample):
        return int_or_max(cand.get("plan_index"))
    return int_or_max(cand.get("ordered_index"))


def candidate_outcome(sample: dict[str, Any]) -> tuple[int, int, int, int]:
    """Sort key for the candidate's bounded child-search result.

    This is diagnostic target data, not an online policy feature.
    """

    target = sample.get("target") if isinstance(sample.get("target"), dict) else {}
    complete_win = bool(target.get("complete_win"))
    terminal = target.get("terminal")
    if complete_win and terminal == "win":
        tier = 3
    elif terminal == "win":
        tier = 2
    elif terminal == "unresolved":
        tier = 1
    else:
        tier = 0
    return (
        tier,
        int_or_min(target.get("final_hp")),
        -int_or_max(target.get("child_search_hp_loss")),
        -int_or_max(target.get("nodes_expanded")),
    )


def candidate_final_hp(sample: dict[str, Any]) -> int | None:
    target = sample.get("target") if isinstance(sample.get("target"), dict) else {}
    value = target.get("final_hp")
    return value if isinstance(value, int) else None


def nested_get(root: dict[str, Any], path: str) -> Any:
    current: Any = root
    for part in path.split("."):
        if not isinstance(current, dict):
            return None
        current = current.get(part)
    return current


def int_or_min(value: Any) -> int:
    return value if isinstance(value, int) else -10**9


def int_or_max(value: Any) -> int:
    return value if isinstance(value, int) else 10**9


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


def display_card_from_normalized(card: str) -> str:
    upgraded = card.endswith("+")
    base = card[:-1] if upgraded else card
    base = re.sub(r"_(R|G|B|P|C)$", "", base)
    return f"{base}+" if upgraded else base


def extract_features(sample: dict[str, Any], *, include_order_features: bool) -> dict[str, float]:
    features: dict[str, float] = defaultdict(float)
    cand = candidate(sample)
    context = initial_context(sample)
    state = context.get("state") if isinstance(context.get("state"), dict) else {}
    frontier = context.get("frontier_value") if isinstance(context.get("frontier_value"), dict) else {}
    search = search_context(sample)
    one_step = one_step_context(sample)
    action_key = candidate_action_key(sample)

    add_token(features, "bias")
    add_token(features, f"schema:{sample.get('schema_name')}")
    add_token(features, f"action_class:{cand.get('action_class')}")
    add_token(features, f"action_role:{cand.get('action_role')}")
    add_token(features, f"plan_bucket:{cand.get('bucket')}")
    add_token(features, f"plan_stop_reason:{cand.get('stop_reason')}")
    add_token(features, f"plan_outcome_class:{cand.get('outcome_class')}")
    add_token(features, f"plan_survival_bucket:{cand.get('survival_bucket')}")
    add_token(features, f"plan_progress_bucket:{cand.get('progress_bucket')}")
    add_token(features, f"rollout_policy:{search.get('rollout_policy')}")
    add_token(features, f"frontier_policy:{search.get('frontier_policy')}")
    add_token(features, f"potion_policy:{search.get('potion_policy')}")
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
        add_number(features, "ordered_index", sample_ordered_index(sample), 24.0)
        add_number(features, "original_action_id", cand.get("original_action_id"), 24.0)

    if is_turn_plan_sample(sample):
        add_number(features, "plan_action_count", cand.get("action_count"), 12.0)
        add_number(features, "plan_eval_final_hp", cand.get("eval_final_hp"), 100.0)
        add_number(features, "plan_eval_risk_margin", cand.get("eval_risk_margin"), 100.0)
        add_number(features, "plan_eval_enemy_progress", cand.get("eval_enemy_progress"), 300.0)
        action_keys = cand.get("action_keys") if isinstance(cand.get("action_keys"), list) else []
        for position, key in enumerate(action_keys[:8]):
            action = str(key)
            if action == "combat/end_turn":
                add_token(features, f"plan_action:{position}:end_turn")
                continue
            card = normalized_card_from_action_key(action)
            if card:
                add_token(features, f"plan_card:{card}")
                add_token(features, f"plan_action:{position}:card:{card}")
            target = TARGET_IN_ACTION_RE.search(action)
            if target:
                add_token(features, f"plan_action:{position}:target:{target.group(1).split(':')[0]}")

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


def sample_source_key(sample: dict[str, Any]) -> str:
    source = sample.get("source") if isinstance(sample.get("source"), dict) else {}
    value = source.get("source_file") or source.get("file") or sample.get("_source_jsonl")
    return str(value or "unknown_source")


def source_unit_to_group_keys(groups: dict[str, list[dict[str, Any]]]) -> dict[str, list[str]]:
    units: dict[str, list[str]] = defaultdict(list)
    for key, group in groups.items():
        unit = sample_source_key(group[0]) if group else "unknown_source"
        units[unit].append(key)
    return dict(units)


def split_groups(
    groups: dict[str, list[dict[str, Any]]],
    *,
    test_ratio: float,
    split_mode: str,
    split_seed: int,
) -> tuple[dict[str, list[dict[str, Any]]], dict[str, list[dict[str, Any]]], dict[str, Any]]:
    unit_to_group_keys: dict[str, list[str]] = defaultdict(list)
    use_group_split = split_mode == "group"
    if split_mode == "source":
        sources = {
            sample_source_key(group[0])
            for group in groups.values()
            if group
        }
        # A one-source dataset cannot honestly hold out a campaign/source. Fall
        # back to group split so tiny smoke files still run.
        use_group_split = len(sources) < 2
    for key, group in groups.items():
        if use_group_split or not group:
            unit = key
        else:
            unit = sample_source_key(group[0])
        unit_to_group_keys[unit].append(key)

    unit_train: set[str] = set()
    unit_test: set[str] = set()
    threshold = int(test_ratio * 10_000)
    for unit in sorted(unit_to_group_keys):
        bucket = stable_hash(f"{split_seed}:{unit}") % 10_000
        if bucket < threshold:
            unit_test.add(unit)
        else:
            unit_train.add(unit)
    if not unit_train and unit_test:
        unit = sorted(unit_test)[0]
        unit_test.remove(unit)
        unit_train.add(unit)
    if not unit_test and len(unit_train) > 1:
        unit = sorted(unit_train)[-1]
        unit_train.remove(unit)
        unit_test.add(unit)

    train = {}
    test = {}
    for unit, keys in unit_to_group_keys.items():
        target = test if unit in unit_test else train
        for key in keys:
            target[key] = groups[key]
    meta = {
        "mode": "group" if use_group_split else split_mode,
        "requested_mode": split_mode,
        "seed": split_seed,
        "train_units": len(unit_train),
        "test_units": len(unit_test),
    }
    return train, test, meta


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
    group_scores = {}
    for key, group in groups.items():
        group_scores[key] = [-sample_ordered_index(sample) for sample in group]
    return metrics_from_group_scores(groups, group_scores)


def evaluate_model(
    groups: dict[str, list[dict[str, Any]]],
    weights: dict[int, float],
    bias: float,
    *,
    dim: int,
    include_order_features: bool,
) -> dict[str, float]:
    group_scores = {}
    for key, group in groups.items():
        scores = []
        for sample in group:
            features = extract_features(sample, include_order_features=include_order_features)
            scores.append(dot(weights, hashed_features(features, dim), bias))
        group_scores[key] = scores
    return metrics_from_group_scores(groups, group_scores)


def source_cross_validated_model_metrics(
    groups: dict[str, list[dict[str, Any]]],
    *,
    dim: int,
    epochs: int,
    learning_rate: float,
    l2: float,
    seed: int,
    include_order_features: bool,
    return_scores: bool = False,
) -> tuple[dict[str, float], dict[str, Any], dict[str, list[float]]]:
    units = source_unit_to_group_keys(groups)
    if len(units) < 2:
        metrics = {
            "groups": 0.0,
            "top1": 0.0,
            "mrr": 0.0,
            "avg_rank": 0.0,
            "avg_candidates": 0.0,
            "avg_hp_gain_vs_ordered": 0.0,
            "positive_hp_gain": 0.0,
            "negative_hp_gain": 0.0,
            "target_missed": 0.0,
            "target_outcome_missed": 0.0,
            "target_outcome_match_rate": 0.0,
            "avg_hp_regret_to_target": 0.0,
        }
        return metrics, {"folds": 0, "source_units": len(units)}, {}
    out_of_fold_scores: dict[str, list[float]] = {}
    folds = 0
    for fold_index, held_out_unit in enumerate(sorted(units)):
        test_keys = set(units[held_out_unit])
        train_groups = {key: group for key, group in groups.items() if key not in test_keys}
        test_groups = {key: group for key, group in groups.items() if key in test_keys}
        train_examples = flatten_training_examples(
            train_groups,
            include_order_features=include_order_features,
        )
        if not train_examples or not test_groups:
            continue
        weights, bias = train_logistic(
            train_examples,
            dim=dim,
            epochs=epochs,
            learning_rate=learning_rate,
            l2=l2,
            seed=seed + fold_index,
        )
        for key, group in test_groups.items():
            scores = []
            for sample in group:
                features = extract_features(sample, include_order_features=include_order_features)
                scores.append(dot(weights, hashed_features(features, dim), bias))
            out_of_fold_scores[key] = scores
        folds += 1
    scores = out_of_fold_scores if return_scores else {}
    return metrics_from_group_scores(groups, out_of_fold_scores), {
        "folds": folds,
        "source_units": len(units),
    }, scores


def metrics_from_group_scores(
    groups: dict[str, list[dict[str, Any]]], group_scores: dict[str, list[float]]
) -> dict[str, float]:
    ranks = []
    hp_gains = []
    target_hp_regrets = []
    positive_gain = 0
    negative_gain = 0
    target_missed = 0
    target_outcome_missed = 0
    target_outcome_matched = 0
    for key, group in groups.items():
        scores = group_scores.get(key) or []
        if len(scores) != len(group):
            continue
        ranks.append(selected_rank(group, scores))
        top_index = max(range(len(group)), key=lambda index: scores[index])
        current_index = min(
            range(len(group)),
            key=lambda index: sample_ordered_index(group[index]),
        )
        target_index = next((index for index, sample in enumerate(group) if is_selected(sample)), None)
        top = group[top_index]
        current = group[current_index]
        top_hp = candidate_final_hp(top)
        current_hp = candidate_final_hp(current)
        target = group[target_index] if target_index is not None else None
        target_hp = candidate_final_hp(target) if target is not None else None
        if top_hp is not None and current_hp is not None:
            gain = top_hp - current_hp
            hp_gains.append(gain)
            if gain > 0:
                positive_gain += 1
            elif gain < 0:
                negative_gain += 1
        if target_hp is not None and top_hp is not None:
            target_hp_regrets.append(target_hp - top_hp)
        if target_index is not None:
            if candidate_outcome(group[target_index]) == candidate_outcome(top):
                target_outcome_matched += 1
            if top_index != target_index:
                target_missed += 1
            if top_index != target_index and candidate_outcome(group[target_index]) != candidate_outcome(top):
                target_outcome_missed += 1
    if not ranks:
        return {
            "groups": 0.0,
            "top1": 0.0,
            "mrr": 0.0,
            "avg_rank": 0.0,
            "avg_hp_gain_vs_ordered": 0.0,
            "positive_hp_gain": 0.0,
            "negative_hp_gain": 0.0,
            "target_missed": 0.0,
            "target_outcome_missed": 0.0,
            "target_outcome_match_rate": 0.0,
            "avg_hp_regret_to_target": 0.0,
        }
    return {
        "groups": float(len(ranks)),
        "top1": sum(1 for rank in ranks if rank == 1) / len(ranks),
        "mrr": sum(1.0 / rank for rank in ranks) / len(ranks),
        "avg_rank": sum(ranks) / len(ranks),
        "avg_candidates": sum(len(group) for group in groups.values()) / len(groups),
        "avg_hp_gain_vs_ordered": sum(hp_gains) / len(hp_gains) if hp_gains else 0.0,
        "positive_hp_gain": float(positive_gain),
        "negative_hp_gain": float(negative_gain),
        "target_missed": float(target_missed),
        "target_outcome_missed": float(target_outcome_missed),
        "target_outcome_match_rate": target_outcome_matched / len(ranks),
        "avg_hp_regret_to_target": (
            sum(target_hp_regrets) / len(target_hp_regrets) if target_hp_regrets else 0.0
        ),
    }


def selected_indices_for_scores(
    group: list[dict[str, Any]], scores: list[float]
) -> tuple[int, int, int | None]:
    model_index = max(range(len(group)), key=lambda index: scores[index])
    ordered_index = min(range(len(group)), key=lambda index: sample_ordered_index(group[index]))
    target_index = next((index for index, sample in enumerate(group) if is_selected(sample)), None)
    return ordered_index, model_index, target_index


def source_label(sample: dict[str, Any]) -> str:
    source = sample.get("source") if isinstance(sample.get("source"), dict) else {}
    case_id = source.get("case_id") or "-"
    benchmark = source.get("benchmark_name") or Path(sample_source_key(sample)).stem
    return f"{benchmark}:{case_id}"


def state_summary(sample: dict[str, Any]) -> str:
    context = initial_context(sample)
    state = context.get("state") if isinstance(context.get("state"), dict) else {}
    frontier = context.get("frontier_value") if isinstance(context.get("frontier_value"), dict) else {}
    return (
        f"hp={state.get('player_hp')} block={state.get('player_block')} "
        f"energy={state.get('energy')} incoming={state.get('visible_incoming_damage')} "
        f"enemies={state.get('living_enemy_count')} enemy_hp={state.get('total_enemy_hp')} "
        f"hand_dmg={nested_get(frontier, 'hand.damage')} hand_block={nested_get(frontier, 'hand.block')}"
    )


def plan_summary(sample: dict[str, Any]) -> str:
    cand = candidate(sample)
    target = sample.get("target") if isinstance(sample.get("target"), dict) else {}
    action_keys = cand.get("action_keys") if isinstance(cand.get("action_keys"), list) else []
    if not action_keys:
        key = candidate_action_key(sample)
        action_keys = [key] if key else []
    preview: list[str] = []
    for key in action_keys[:4]:
        text = str(key)
        card = normalized_card_from_action_key(text)
        if card:
            preview.append(display_card_from_normalized(card))
        elif text == "combat/end_turn":
            preview.append("end")
        else:
            preview.append(text.rsplit("/", 1)[-1])
    if len(action_keys) > 4:
        preview.append("...")
    return (
        f"idx={sample_ordered_index(sample)} hp={candidate_final_hp(sample)} "
        f"outcome={target.get('terminal')} complete={target.get('complete_win')} "
        f"seq=[{' -> '.join(preview)}]"
    )


def interesting_case_rows(
    groups: dict[str, list[dict[str, Any]]],
    group_scores: dict[str, list[float]],
    *,
    kind: str,
    limit: int,
) -> list[tuple[float, str]]:
    rows: list[tuple[float, str]] = []
    for key, group in groups.items():
        scores = group_scores.get(key) or []
        if len(scores) != len(group):
            continue
        ordered_index, model_index, target_index = selected_indices_for_scores(group, scores)
        if target_index is None:
            continue
        ordered = group[ordered_index]
        model = group[model_index]
        target = group[target_index]
        ordered_hp = candidate_final_hp(ordered)
        model_hp = candidate_final_hp(model)
        target_hp = candidate_final_hp(target)
        if ordered_hp is None or model_hp is None or target_hp is None:
            continue
        model_gain = model_hp - ordered_hp
        model_regret = target_hp - model_hp
        ordered_regret = target_hp - ordered_hp
        if kind == "worse" and model_gain >= 0:
            continue
        if kind == "better" and model_gain <= 0:
            continue
        if kind == "both-bad" and not (ordered_regret > 0 and model_regret > 0):
            continue
        sort_key = {
            "worse": -model_gain,
            "better": model_gain,
            "both-bad": max(ordered_regret, model_regret),
        }[kind]
        body = "\n".join(
            [
                f"case={source_label(group[0])} state={state_summary(group[0])}",
                f"  ordered: {plan_summary(ordered)}",
                f"  model:   {plan_summary(model)} gain_vs_ordered={model_gain:+d} regret={model_regret:+d}",
                f"  target:  {plan_summary(target)} ordered_regret={ordered_regret:+d}",
            ]
        )
        rows.append((float(sort_key), body))
    rows.sort(key=lambda item: item[0], reverse=True)
    return rows[:limit]


def print_case_rows(
    title: str,
    groups: dict[str, list[dict[str, Any]]],
    scores: dict[str, list[float]],
    *,
    kind: str,
    limit: int,
) -> None:
    rows = interesting_case_rows(groups, scores, kind=kind, limit=limit)
    print(f"  cases:{title} count={len(rows)}")
    for _score, body in rows:
        print(body)


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


def print_metrics(label: str, metrics: dict[str, float], *, report_mode: str) -> None:
    if report_mode == "compact":
        print(
            f"  {label}: groups={metrics['groups']:.0f} "
            f"outcome_match={metrics.get('target_outcome_match_rate', 0.0):.3f} "
            f"hp_regret={metrics.get('avg_hp_regret_to_target', 0.0):+.2f} "
            f"hp_gain_vs_ordered={metrics.get('avg_hp_gain_vs_ordered', 0.0):+.2f} "
            f"worse_hp={metrics.get('negative_hp_gain', 0.0):.0f}"
        )
        return
    print(
        f"  {label}: groups={metrics['groups']:.0f} top1={metrics['top1']:.3f} "
        f"mrr={metrics['mrr']:.3f} avg_rank={metrics['avg_rank']:.2f} "
        f"avg_candidates={metrics.get('avg_candidates', 0.0):.2f} "
        f"avg_hp_gain_vs_ordered={metrics.get('avg_hp_gain_vs_ordered', 0.0):+.2f} "
        f"hp_gain(+/-)={metrics.get('positive_hp_gain', 0.0):.0f}/"
        f"{metrics.get('negative_hp_gain', 0.0):.0f} "
        f"target_missed={metrics.get('target_missed', 0.0):.0f} "
        f"target_outcome_missed={metrics.get('target_outcome_missed', 0.0):.0f} "
        f"target_outcome_match={metrics.get('target_outcome_match_rate', 0.0):.3f} "
        f"avg_hp_regret_to_target={metrics.get('avg_hp_regret_to_target', 0.0):+.2f}"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "inputs",
        nargs="*",
        type=Path,
        help=(
            "CombatSearchGuidanceSampleV1, CombatActionProbeSampleV1, or "
            "CombatTurnPlanProbeSampleV1 JSONL"
        ),
    )
    parser.add_argument(
        "--discover-turn-plan-probes",
        action="append",
        nargs="+",
        type=Path,
        metavar="ROOT",
        help=(
            "Discover *.turn_plan_probe*.jsonl under ROOT. When several probes "
            "for the same suite prefix exist in one directory, the newest is used."
        ),
    )
    parser.add_argument("--dim", type=int, default=4096)
    parser.add_argument("--epochs", type=int, default=25)
    parser.add_argument("--learning-rate", type=float, default=0.05)
    parser.add_argument("--l2", type=float, default=0.0005)
    parser.add_argument("--test-ratio", type=float, default=0.3)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument(
        "--split-mode",
        choices=("source", "group", "source-cv"),
        default="source",
        help=(
            "source holds out whole source/lab files; group is the older per-root "
            "hash split; source-cv does leave-one-source-out evaluation."
        ),
    )
    parser.add_argument(
        "--split-seed",
        type=int,
        default=1,
        help="Hash seed for train/test assignment; independent from --seed training shuffle.",
    )
    parser.add_argument(
        "--include-order-features",
        action="store_true",
        help="Allow ordered_index/original_action_id/hand_index as features",
    )
    parser.add_argument("--top-features", type=int, default=12)
    parser.add_argument(
        "--report-mode",
        choices=("compact", "full"),
        default="compact",
        help="compact prints regret/outcome metrics only; full also prints top1/MRR/features.",
    )
    parser.add_argument(
        "--show-cases",
        type=int,
        default=0,
        help="For source-cv, print compact ordered/model/target case comparisons.",
    )
    parser.add_argument(
        "--case-kind",
        choices=("worse", "better", "both-bad", "all"),
        default="worse",
        help="Which source-cv case comparisons to show when --show-cases is set.",
    )
    args = parser.parse_args()

    input_paths = list(args.inputs)
    if args.discover_turn_plan_probes:
        roots = [root for group in args.discover_turn_plan_probes for root in group]
        input_paths.extend(discover_turn_plan_probe_paths(roots))
    input_paths = sorted(set(input_paths))
    if not input_paths:
        parser.error("provide JSONL inputs or --discover-turn-plan-probes ROOT")

    samples = load_samples(input_paths)
    groups = usable_groups(samples)
    target_counts = Counter()
    for group in groups.values():
        for sample in group:
            target_counts["selected" if is_selected(sample) else "not_selected"] += 1
    print("CombatSearchRankingBaseline")
    print(
        f"  input_files={len(input_paths)} samples={len(samples)} "
        f"usable_groups={len(groups)} labels={dict(target_counts)}"
    )
    print(
        "  label_role=oracle_search_guidance_ranking_not_human_policy "
        "candidate_coverage=root_legal_candidates_reported_limit"
    )
    if len(groups) < 8:
        print("  readiness=too_few_groups_for_meaningful_ml")
    else:
        print("  readiness=small_offline_ranking_probe")
    if not groups:
        return

    if args.split_mode == "source-cv":
        cv_metrics, cv_meta, cv_scores = source_cross_validated_model_metrics(
            groups,
            dim=args.dim,
            epochs=args.epochs,
            learning_rate=args.learning_rate,
            l2=args.l2,
            seed=args.seed,
            include_order_features=args.include_order_features,
            return_scores=args.show_cases > 0,
        )
        print(
            f"  split=mode:source-cv source_units:{cv_meta['source_units']} "
            f"folds:{cv_meta['folds']}"
        )
        print_metrics(
            "ordered_index_all",
            evaluate_ordered_index(groups),
            report_mode=args.report_mode,
        )
        print_metrics("logistic_source_cv", cv_metrics, report_mode=args.report_mode)
        if args.show_cases > 0:
            kinds = (
                ("worse", "model_worse_than_ordered"),
                ("better", "model_better_than_ordered"),
                ("both-bad", "ordered_bad_model_bad"),
            )
            for kind, title in kinds:
                if args.case_kind in (kind, "all"):
                    print_case_rows(
                        title,
                        groups,
                        cv_scores,
                        kind=kind,
                        limit=args.show_cases,
                    )
        train_examples = flatten_training_examples(
            groups,
            include_order_features=args.include_order_features,
        )
        if not train_examples:
            print("  logistic=skipped_not_enough_data")
            return
        weights, _bias = train_logistic(
            train_examples,
            dim=args.dim,
            epochs=args.epochs,
            learning_rate=args.learning_rate,
            l2=args.l2,
            seed=args.seed,
        )
        if args.report_mode == "full":
            print("  top_weighted_features_full_data:")
            for name, weight in feature_weight_report(
                weights,
                groups,
                dim=args.dim,
                include_order_features=args.include_order_features,
                limit=args.top_features,
            ):
                print(f"    {weight:+.4f} {name}")
        return

    train_groups, test_groups, split_meta = split_groups(
        groups,
        test_ratio=args.test_ratio,
        split_mode=args.split_mode,
        split_seed=args.split_seed,
    )
    print(
        f"  split=mode:{split_meta['mode']} requested:{split_meta['requested_mode']} "
        f"split_seed:{split_meta['seed']} train_groups:{len(train_groups)} "
        f"test_groups:{len(test_groups)} train_units:{split_meta['train_units']} "
        f"test_units:{split_meta['test_units']}"
    )
    print_metrics(
        "ordered_index_train",
        evaluate_ordered_index(train_groups),
        report_mode=args.report_mode,
    )
    print_metrics(
        "ordered_index_test",
        evaluate_ordered_index(test_groups),
        report_mode=args.report_mode,
    )

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
        report_mode=args.report_mode,
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
        report_mode=args.report_mode,
    )
    if args.report_mode == "full":
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
