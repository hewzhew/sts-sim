#!/usr/bin/env python3
"""Dependency-free ranking baseline for combat search guidance.

Input is either:

- CombatSearchGuidanceSampleV1 JSONL produced from decision microscope reports
  by combat_search_guidance_samples.py.
- CombatActionProbeSampleV1 JSONL produced from guidance-lab reports by
  combat_guidance_lab_extract.py.
- CombatTurnPlanProbeSampleV1 JSONL produced from turn-plan guidance-lab
  reports by combat_turn_plan_guidance_lab_extract.py.
- CombatTacticalEpisodeV1 JSONL produced by combat_tactical_trace_extract.py.
  These are expanded into compatible turn-plan candidate samples at load time.

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
TACTICAL_EPISODE_SCHEMA_NAME = "CombatTacticalEpisodeV1"
EXPERIMENTAL_FEATURE_GROUPS = (
    "root-delta",
    "action-shape",
    "target-detail",
    "enemy-slot-context",
    "tactical-summary",
    "action-facts",
)
TARGET_MODES = ("selected", "equivalent-hp-outcome")
TRAINING_MODES = ("binary", "pairwise-utility", "decomposed-utility")
DECOMPOSED_OUTCOME_WEIGHT = 1.0
DECOMPOSED_HP_WEIGHT = 1.0
_CAPTURE_ENEMY_SLOT_CACHE: dict[str, list[dict[str, Any]]] = {}


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
                    sample["_source_schema_name"] = schema_name
                    sample["_source_jsonl"] = str(path)
                    samples.append(sample)
                elif schema_name in (PROBE_SCHEMA_NAME, TURN_PLAN_SCHEMA_NAME):
                    sample["_source_schema_name"] = schema_name
                    sample["_source_jsonl"] = str(path)
                    samples.append(sample)
                elif schema_name == TACTICAL_EPISODE_SCHEMA_NAME:
                    samples.extend(samples_from_tactical_episode(sample, path))
                else:
                    raise SystemExit(
                        f"{path}:{line_no}: expected {LEGACY_SCHEMA_NAME} or "
                        f"{PROBE_SCHEMA_NAME} or {TURN_PLAN_SCHEMA_NAME} or "
                        f"{TACTICAL_EPISODE_SCHEMA_NAME}, got {schema_name!r}"
                    )
    return samples


def samples_from_tactical_episode(episode: dict[str, Any], path: Path) -> list[dict[str, Any]]:
    plans = [plan for plan in episode.get("candidate_plans", []) if isinstance(plan, dict)]
    if not plans:
        return []
    best_plan_index = tactical_best_plan_index(plans)
    root = episode.get("root") if isinstance(episode.get("root"), dict) else {}
    public_view = root.get("public_view") if isinstance(root.get("public_view"), dict) else {}
    provenance = episode.get("provenance") if isinstance(episode.get("provenance"), dict) else {}
    root_context = {
        "config": provenance.get("search_config"),
        "initial_context": {
            "state": public_view.get("state"),
            "phase_profile": public_view.get("phase_profile"),
            "frontier_value": public_view.get("frontier_value"),
        },
        "enumeration": {
            "planning_policy": provenance.get("candidate_generator_id"),
            "source_schema": episode.get("schema_name"),
            "root_exact_state_hash": root.get("exact_state_hash"),
        },
        "enemy_slots": public_view.get("enemy_slots") if isinstance(public_view.get("enemy_slots"), list) else [],
    }
    source = episode.get("source") if isinstance(episode.get("source"), dict) else {}
    samples = []
    for plan in plans:
        summary = plan.get("plan_summary") if isinstance(plan.get("plan_summary"), dict) else {}
        outcome = (
            plan.get("outcome_attachment")
            if isinstance(plan.get("outcome_attachment"), dict)
            else {}
        )
        candidate_plan = tactical_plan_as_turn_plan_probe(plan, summary)
        samples.append(
            {
                "schema_name": TURN_PLAN_SCHEMA_NAME,
                "schema_version": 2,
                "label_role": "expanded_from_tactical_episode_oracle_under_budget_not_human_policy",
                "_source_schema_name": TACTICAL_EPISODE_SCHEMA_NAME,
                "_source_jsonl": str(path),
                "source": {
                    **source,
                    "source_file": str(path),
                    "tactical_episode_input_label": source.get("input_label"),
                    "tactical_episode_source_file": source.get("source_file"),
                },
                "root_context": root_context,
                "plan": candidate_plan,
                "target": {
                    "target_kind": outcome.get("target_kind"),
                    "source": outcome.get("source"),
                    "terminal": outcome.get("terminal"),
                    "complete_win": outcome.get("complete_win"),
                    "post_root_player_hp": outcome.get("post_root_player_hp"),
                    "child_search_hp_loss": outcome.get("child_search_hp_loss"),
                    "final_hp": outcome.get("final_hp"),
                    "nodes_expanded": outcome.get("nodes_expanded"),
                    "is_best_target_plan": plan.get("plan_index") == best_plan_index,
                    "limitations": outcome.get("limitations") or [],
                },
                "child_search": outcome.get("child_search"),
            }
        )
    return samples


def tactical_best_plan_index(plans: list[dict[str, Any]]) -> Any:
    best = max(
        plans,
        key=lambda plan: (
            tactical_target_sort_key(plan),
            -int_or_max(plan.get("plan_index")),
        ),
    )
    return best.get("plan_index")


def tactical_target_sort_key(plan: dict[str, Any]) -> tuple[int, int, int, int]:
    outcome = plan.get("outcome_attachment") if isinstance(plan.get("outcome_attachment"), dict) else {}
    return (
        tactical_target_tier(outcome),
        int_or_min(outcome.get("final_hp")),
        -int_or_max(outcome.get("child_search_hp_loss")),
        -int_or_max(outcome.get("nodes_expanded")),
    )


def tactical_target_tier(outcome: dict[str, Any]) -> int:
    terminal = outcome.get("terminal")
    if outcome.get("complete_win") and terminal == "win":
        return 3
    if terminal == "win":
        return 2
    if terminal == "unresolved":
        return 1
    return 0


def tactical_plan_as_turn_plan_probe(plan: dict[str, Any], summary: dict[str, Any]) -> dict[str, Any]:
    generation = plan.get("generation") if isinstance(plan.get("generation"), dict) else {}
    steps = [step for step in plan.get("steps", []) if isinstance(step, dict)]
    actions = [
        step.get("action")
        for step in steps
        if isinstance(step.get("action"), dict)
    ]
    action_keys = [str(action.get("action_key") or "") for action in actions]
    final_state = plan.get("final_state_summary") if isinstance(plan.get("final_state_summary"), dict) else {}
    return {
        "plan_index": plan.get("plan_index"),
        "bucket": generation.get("bucket"),
        "stop_reason": generation.get("stop_reason"),
        "outcome_class": generation.get("outcome_class"),
        "survival_bucket": generation.get("survival_bucket"),
        "progress_bucket": generation.get("progress_bucket"),
        "action_count": len(actions),
        "first_action_key": action_keys[0] if action_keys else None,
        "action_keys": action_keys,
        "actions": actions,
        "steps": steps,
        "end_state": final_state,
        "final_state_hash": plan.get("final_state_hash"),
        "plan_summary": summary,
        "eval_final_hp": final_state.get("player_hp"),
        "eval_risk_margin": None,
        "eval_enemy_progress": summary.get("enemy_hp_removed_to_plan_boundary"),
    }


def load_enemy_slots_from_capture_path(path_text: str) -> list[dict[str, Any]]:
    cached = _CAPTURE_ENEMY_SLOT_CACHE.get(path_text)
    if cached is not None:
        return cached
    path = Path(path_text)
    if not path.exists():
        _CAPTURE_ENEMY_SLOT_CACHE[path_text] = []
        return []
    try:
        with path.open("r", encoding="utf-8") as handle:
            payload = json.load(handle)
    except (OSError, json.JSONDecodeError):
        _CAPTURE_ENEMY_SLOT_CACHE[path_text] = []
        return []
    summary = payload.get("summary") if isinstance(payload, dict) else {}
    monsters = summary.get("monsters") if isinstance(summary, dict) else []
    if not isinstance(monsters, list):
        _CAPTURE_ENEMY_SLOT_CACHE[path_text] = []
        return []
    slots = [monster for monster in monsters if isinstance(monster, dict)]
    _CAPTURE_ENEMY_SLOT_CACHE[path_text] = slots
    return slots


def enemy_slots_from_sample(sample: dict[str, Any]) -> list[dict[str, Any]]:
    root = sample.get("root_context") if isinstance(sample.get("root_context"), dict) else {}
    slots = root.get("enemy_slots") if isinstance(root.get("enemy_slots"), list) else []
    if slots:
        return [slot for slot in slots if isinstance(slot, dict)]
    source = sample.get("source") if isinstance(sample.get("source"), dict) else {}
    input_path = source.get("input_path")
    if isinstance(input_path, str) and input_path:
        return load_enemy_slots_from_capture_path(input_path)
    return []


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


def discover_tactical_episode_paths(roots: list[Path]) -> list[Path]:
    explicit_files: list[Path] = []
    discovered_by_key: dict[str, Path] = {}
    for root in roots:
        if root.is_file():
            explicit_files.append(root)
            continue
        if not root.exists():
            raise SystemExit(f"discover root does not exist: {root}")
        for path in root.rglob("*.tactical_episode*.jsonl"):
            key = tactical_episode_discovery_key(path)
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


def tactical_episode_discovery_key(path: Path) -> str:
    name = path.name
    for suffix in (".tactical_episode_batch.jsonl", ".tactical_episode.jsonl"):
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


def candidate_terminal_signature(sample: dict[str, Any]) -> tuple[bool, Any, int | None]:
    target = sample.get("target") if isinstance(sample.get("target"), dict) else {}
    return (
        bool(target.get("complete_win")),
        target.get("terminal"),
        candidate_final_hp(sample),
    )


def candidate_utility_key(sample: dict[str, Any]) -> tuple[int, int, int]:
    """Diagnostic utility order for pairwise ranking.

    This deliberately excludes nodes_expanded: two plans with the same terminal
    and HP result should not become different training targets only because the
    bounded child search found one with fewer nodes.
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
    )


def candidate_final_hp(sample: dict[str, Any]) -> int | None:
    target = sample.get("target") if isinstance(sample.get("target"), dict) else {}
    value = target.get("final_hp")
    return value if isinstance(value, int) else None


def primary_target_index(group: list[dict[str, Any]]) -> int | None:
    return next((index for index, sample in enumerate(group) if is_selected(sample)), None)


def positive_target_indices(group: list[dict[str, Any]], target_mode: str) -> list[int]:
    selected = primary_target_index(group)
    if selected is None:
        return []
    if target_mode == "selected":
        return [selected]
    if target_mode != "equivalent-hp-outcome":
        raise ValueError(f"unknown target mode: {target_mode}")

    signature = candidate_terminal_signature(group[selected])
    complete_win, terminal, final_hp = signature
    if terminal is None or final_hp is None:
        return [selected]
    return [
        index
        for index, sample in enumerate(group)
        if candidate_terminal_signature(sample) == (complete_win, terminal, final_hp)
    ]


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


def numeric_value(value: Any) -> float | None:
    if isinstance(value, bool):
        return float(int(value))
    if isinstance(value, (int, float)):
        return float(value)
    return None


def numeric_or_zero(value: Any) -> float:
    number = numeric_value(value)
    return number if number is not None else 0.0


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


def display_action_target_suffix(action_key: str) -> str:
    target = TARGET_IN_ACTION_RE.search(action_key)
    if not target:
        return ""
    target_value = target.group(1)
    if target_value == "none":
        return ""
    if target_value.startswith("monster_slot:"):
        return f"@m{target_value.split(':', 1)[1]}"
    return f"@{target_value}"


def monster_slot_from_action_key(action_key: str) -> str | None:
    target = TARGET_IN_ACTION_RE.search(action_key)
    if not target:
        return None
    target_value = target.group(1)
    if not target_value.startswith("monster_slot:"):
        return None
    return target_value.split(":", 1)[1]


def intent_kind_from_text(value: Any) -> str:
    if not isinstance(value, str) or not value:
        return "unknown"
    return value.split("(", 1)[0]


def add_turn_plan_root_delta_features(
    features: dict[str, float],
    state: dict[str, Any],
    plan: dict[str, Any],
) -> None:
    end_state = plan.get("end_state") if isinstance(plan.get("end_state"), dict) else {}
    if not end_state:
        return

    initial_enemy_hp = numeric_value(state.get("total_enemy_hp"))
    final_enemy_hp = numeric_value(end_state.get("total_enemy_hp"))
    if initial_enemy_hp is not None and final_enemy_hp is not None:
        enemy_hp_removed = initial_enemy_hp - final_enemy_hp
        add_number(features, "plan_root_enemy_hp_removed", enemy_hp_removed, 300.0)
        if initial_enemy_hp > 0:
            add_number(
                features,
                "plan_root_enemy_progress_ratio",
                enemy_hp_removed / initial_enemy_hp,
                1.0,
            )
        add_token(
            features,
            "plan_root_enemy_hp_progress" if enemy_hp_removed > 0 else "plan_root_no_enemy_hp_progress",
        )

    initial_enemies = numeric_value(state.get("living_enemy_count"))
    final_enemies = numeric_value(end_state.get("living_enemy_count"))
    if initial_enemies is not None and final_enemies is not None:
        enemies_killed = initial_enemies - final_enemies
        add_number(features, "plan_root_enemies_killed", enemies_killed, 5.0)
        add_number(features, "plan_root_living_enemies_after", final_enemies, 5.0)
        add_token(features, "plan_root_kills_enemy" if enemies_killed > 0 else "plan_root_no_enemy_kill")


def add_turn_plan_action_shape_features(
    features: dict[str, float],
    action_keys: list[Any],
) -> None:
    play_cards = 0
    targeted_plays = 0
    no_target_plays = 0
    unique_monster_targets: set[str] = set()
    first_play_target_kind: str | None = None
    for key in action_keys:
        text = str(key)
        if text == "combat/end_turn":
            continue
        if not text.startswith("combat/play_card/"):
            continue
        play_cards += 1
        target = TARGET_IN_ACTION_RE.search(text)
        target_value = target.group(1) if target else "none"
        target_kind = target_value.split(":", 1)[0]
        if first_play_target_kind is None:
            first_play_target_kind = target_kind
        if target_kind == "monster_slot":
            targeted_plays += 1
            unique_monster_targets.add(target_value)
        elif target_kind == "none":
            no_target_plays += 1

    add_number(features, "plan_play_card_count", play_cards, 12.0)
    add_number(features, "plan_targeted_play_count", targeted_plays, 12.0)
    add_number(features, "plan_no_target_play_count", no_target_plays, 12.0)
    add_number(features, "plan_unique_monster_targets", len(unique_monster_targets), 5.0)
    if first_play_target_kind is not None:
        add_token(features, f"plan_first_play_target:{first_play_target_kind}")
    if no_target_plays:
        add_token(features, "plan_has_no_target_play")
    if targeted_plays:
        add_token(features, "plan_has_targeted_play")


def add_turn_plan_target_detail_features(
    features: dict[str, float],
    action_keys: list[Any],
) -> None:
    monster_slots: list[str] = []
    for position, key in enumerate(action_keys[:8]):
        text = str(key)
        if not text.startswith("combat/play_card/"):
            continue
        slot = monster_slot_from_action_key(text)
        if slot is None:
            continue
        monster_slots.append(slot)
        add_token(features, f"plan_target_slot:{slot}")
        add_token(features, f"plan_action:{position}:target_slot:{slot}")
        card = normalized_card_from_action_key(text)
        if card:
            add_token(features, f"plan_card_target_slot:{card}:{slot}")

    if not monster_slots:
        return
    add_token(features, f"plan_first_target_slot:{monster_slots[0]}")
    add_number(features, "plan_target_slot_count", len(monster_slots), 12.0)
    unique_slots = set(monster_slots)
    add_number(features, "plan_unique_target_slots_exact", len(unique_slots), 5.0)
    slot_switches = sum(
        1
        for previous, current in zip(monster_slots, monster_slots[1:])
        if previous != current
    )
    add_number(features, "plan_target_slot_switches", slot_switches, 8.0)
    add_token(
        features,
        "plan_focuses_one_target_slot"
        if len(unique_slots) == 1
        else "plan_spreads_target_slots",
    )


def add_turn_plan_enemy_slot_context_features(
    features: dict[str, float],
    sample: dict[str, Any],
    action_keys: list[Any],
) -> None:
    slots = enemy_slots_from_sample(sample)
    by_slot = {str(slot.get("slot")): slot for slot in slots if slot.get("slot") is not None}
    if not by_slot:
        add_token(features, "enemy_slot_context:missing")
        return

    add_number(features, "enemy_slot_context_count", len(by_slot), 5.0)
    targeted_slots: list[str] = []
    for position, key in enumerate(action_keys[:8]):
        text = str(key)
        if not text.startswith("combat/play_card/"):
            continue
        slot_key = monster_slot_from_action_key(text)
        if slot_key is None:
            continue
        enemy = by_slot.get(slot_key)
        if not enemy:
            add_token(features, "plan_targets_unknown_enemy_slot")
            continue
        targeted_slots.append(slot_key)
        enemy_id = str(enemy.get("enemy_id") or "unknown")
        intent_kind = intent_kind_from_text(enemy.get("visible_intent"))
        add_token(features, f"plan_target_enemy:{enemy_id}")
        add_token(features, f"plan_action:{position}:target_enemy:{enemy_id}")
        add_token(features, f"plan_target_intent:{intent_kind}")
        add_token(features, f"plan_action:{position}:target_intent:{intent_kind}")
        add_number(features, "plan_target_enemy_hp", enemy.get("hp"), 100.0)
        add_number(features, "plan_target_enemy_block", enemy.get("block"), 80.0)
        add_number(
            features,
            "plan_target_enemy_preview_damage_per_hit",
            enemy.get("preview_damage_per_hit"),
            40.0,
        )
        hp = numeric_value(enemy.get("hp"))
        max_hp = numeric_value(enemy.get("max_hp"))
        if hp is not None and max_hp and max_hp > 0:
            add_number(features, "plan_target_enemy_hp_ratio", hp / max_hp, 1.0)
        if "Attack" in intent_kind:
            add_token(features, "plan_targets_attacking_enemy")
        elif intent_kind != "unknown":
            add_token(features, "plan_targets_non_attacking_enemy")

    if targeted_slots:
        first_enemy = by_slot.get(targeted_slots[0])
        if first_enemy:
            add_token(features, f"plan_first_target_enemy:{first_enemy.get('enemy_id') or 'unknown'}")
            add_token(
                features,
                f"plan_first_target_intent:{intent_kind_from_text(first_enemy.get('visible_intent'))}",
            )


def add_turn_plan_tactical_summary_features(
    features: dict[str, float],
    plan: dict[str, Any],
) -> None:
    summary = plan.get("plan_summary") if isinstance(plan.get("plan_summary"), dict) else {}
    if not summary:
        add_token(features, "plan_tactical_summary:missing")
        return

    add_token(features, "plan_tactical_summary:present")
    for name, scale in (
        ("cards_played", 12.0),
        ("potion_actions", 4.0),
        ("hp_lost_to_plan_boundary", 80.0),
        ("enemy_hp_removed_to_plan_boundary", 300.0),
        ("enemy_kill_count_to_plan_boundary", 5.0),
        ("damage_hint_total", 300.0),
        ("block_hint_total", 120.0),
        ("visible_attack_mitigation_hint_total", 120.0),
        ("energy_unspent_at_plan_boundary", 6.0),
    ):
        add_number(features, f"plan_summary_{name}", summary.get(name), scale)

    if summary.get("all_enemies_dead_at_plan_boundary"):
        add_token(features, "plan_summary_terminal_win_boundary")
    if numeric_value(summary.get("hp_lost_to_plan_boundary")) == 0:
        add_token(features, "plan_summary_no_hp_loss_to_boundary")
    if numeric_value(summary.get("enemy_hp_removed_to_plan_boundary")) == 0:
        add_token(features, "plan_summary_no_enemy_hp_removed")
    targets = summary.get("unique_target_slots")
    if isinstance(targets, list):
        add_number(features, "plan_summary_unique_target_slots", len(targets), 5.0)


def add_turn_plan_action_fact_features(
    features: dict[str, float],
    plan: dict[str, Any],
) -> None:
    steps = [step for step in plan.get("steps", []) if isinstance(step, dict)]
    if not steps:
        add_token(features, "plan_action_facts:missing")
        return

    add_token(features, "plan_action_facts:present")
    kind_counts: Counter[str] = Counter()
    card_type_counts: Counter[str] = Counter()
    target_enemy_counts: Counter[str] = Counter()
    exact_delta_sums: Counter[str] = Counter()
    damage_hint_total = 0
    block_hint_total = 0
    mitigation_hint_total = 0
    reactive_bad_draw_total = 0
    cost_total = 0
    upgraded_cards = 0
    exhaust_cards = 0
    for step in steps:
        facts = step.get("action_facts") if isinstance(step.get("action_facts"), dict) else {}
        kind = str(facts.get("action_kind") or "unknown")
        kind_counts[kind] += 1
        card = facts.get("card") if isinstance(facts.get("card"), dict) else {}
        if card:
            card_type_counts[str(card.get("card_type") or "unknown")] += 1
            cost = numeric_value(card.get("cost_for_turn"))
            if cost is not None:
                cost_total += cost
            if card.get("upgraded"):
                upgraded_cards += 1
            if card.get("exhaust"):
                exhaust_cards += 1
            if card.get("ethereal"):
                add_token(features, "plan_action_facts_plays_ethereal_card")
            if card.get("innate"):
                add_token(features, "plan_action_facts_plays_innate_card")
        target = facts.get("target") if isinstance(facts.get("target"), dict) else {}
        if target.get("enemy_id"):
            target_enemy_counts[str(target.get("enemy_id"))] += 1
        immediate = facts.get("immediate") if isinstance(facts.get("immediate"), dict) else {}
        mechanics = facts.get("mechanics") if isinstance(facts.get("mechanics"), dict) else {}
        exact = (
            facts.get("exact_one_step_delta")
            if isinstance(facts.get("exact_one_step_delta"), dict)
            else {}
        )
        damage_hint_total += numeric_or_zero(immediate.get("action_payload_damage_hint"))
        block_hint_total += numeric_or_zero(immediate.get("block_hint"))
        mitigation_hint_total += numeric_or_zero(mechanics.get("visible_attack_mitigation_hint"))
        reactive_bad_draw_total += numeric_or_zero(mechanics.get("reactive_bad_draw_cards"))
        for name in (
            "player_hp_delta",
            "player_block_delta",
            "energy_delta",
            "hand_delta",
            "draw_delta",
            "discard_delta",
            "exhaust_delta",
            "total_enemy_hp_delta",
            "total_enemy_block_delta",
        ):
            exact_delta_sums[name] += numeric_or_zero(exact.get(name))
        if immediate.get("creates_pending_choice_after_one_step"):
            add_token(features, "plan_action_facts_creates_pending_choice")
        if mechanics.get("reactive_forced_turn_end"):
            add_token(features, "plan_action_facts_reactive_forced_turn_end")

    for kind, count in kind_counts.items():
        add_number(features, f"plan_action_kind_count:{kind}", count, 12.0)
        add_token(features, f"plan_action_kind_seen:{kind}")
    for card_type, count in card_type_counts.items():
        add_number(features, f"plan_card_type_count:{card_type}", count, 12.0)
    for enemy_id, count in target_enemy_counts.items():
        add_number(features, f"plan_target_enemy_count:{enemy_id}", count, 12.0)
    add_number(features, "plan_action_facts_total_cost", cost_total, 12.0)
    add_number(features, "plan_action_facts_upgraded_cards", upgraded_cards, 12.0)
    add_number(features, "plan_action_facts_exhaust_cards", exhaust_cards, 12.0)
    add_number(features, "plan_action_facts_damage_hint_total", damage_hint_total, 300.0)
    add_number(features, "plan_action_facts_block_hint_total", block_hint_total, 120.0)
    add_number(features, "plan_action_facts_mitigation_hint_total", mitigation_hint_total, 120.0)
    add_number(features, "plan_action_facts_reactive_bad_draw_total", reactive_bad_draw_total, 20.0)
    for name, value in exact_delta_sums.items():
        add_number(features, f"plan_action_facts_exact_sum:{name}", value, 300.0)


def extract_features(
    sample: dict[str, Any],
    *,
    include_order_features: bool,
    feature_groups: frozenset[str] = frozenset(),
) -> dict[str, float]:
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
        if "root-delta" in feature_groups:
            add_turn_plan_root_delta_features(features, state, cand)
        if "action-shape" in feature_groups:
            add_turn_plan_action_shape_features(features, action_keys)
        if "target-detail" in feature_groups:
            add_turn_plan_target_detail_features(features, action_keys)
        if "enemy-slot-context" in feature_groups:
            add_turn_plan_enemy_slot_context_features(features, sample, action_keys)
        if "tactical-summary" in feature_groups:
            add_turn_plan_tactical_summary_features(features, cand)
        if "action-facts" in feature_groups:
            add_turn_plan_action_fact_features(features, cand)
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
    feature_groups: frozenset[str],
    target_mode: str,
) -> list[tuple[int, dict[str, float]]]:
    examples = []
    for group in groups.values():
        positives = set(positive_target_indices(group, target_mode))
        for index, sample in enumerate(group):
            label = 1 if index in positives else 0
            features = extract_features(
                sample,
                include_order_features=include_order_features,
                feature_groups=feature_groups,
            )
            examples.append((label, features))
    return examples


def diff_features(left: dict[str, float], right: dict[str, float]) -> dict[str, float]:
    out: dict[str, float] = {}
    keys = set(left) | set(right)
    for key in keys:
        value = left.get(key, 0.0) - right.get(key, 0.0)
        if value:
            out[key] = value
    return out


def flatten_pairwise_utility_examples(
    groups: dict[str, list[dict[str, Any]]],
    *,
    include_order_features: bool,
    feature_groups: frozenset[str],
) -> list[tuple[int, dict[str, float]]]:
    examples: list[tuple[int, dict[str, float]]] = []
    for group in groups.values():
        feature_rows = [
            extract_features(
                sample,
                include_order_features=include_order_features,
                feature_groups=feature_groups,
            )
            for sample in group
        ]
        utility_rows = [candidate_utility_key(sample) for sample in group]
        for left_index in range(len(group)):
            for right_index in range(left_index + 1, len(group)):
                left_utility = utility_rows[left_index]
                right_utility = utility_rows[right_index]
                if left_utility == right_utility:
                    continue
                if left_utility > right_utility:
                    better_index, worse_index = left_index, right_index
                else:
                    better_index, worse_index = right_index, left_index
                better_minus_worse = diff_features(
                    feature_rows[better_index],
                    feature_rows[worse_index],
                )
                worse_minus_better = diff_features(
                    feature_rows[worse_index],
                    feature_rows[better_index],
                )
                examples.append((1, better_minus_worse))
                examples.append((0, worse_minus_better))
    return examples


def flatten_decomposed_utility_examples(
    groups: dict[str, list[dict[str, Any]]],
    *,
    include_order_features: bool,
    feature_groups: frozenset[str],
) -> tuple[list[tuple[int, dict[str, float]]], list[tuple[int, dict[str, float]]]]:
    outcome_examples: list[tuple[int, dict[str, float]]] = []
    hp_examples: list[tuple[int, dict[str, float]]] = []
    for group in groups.values():
        feature_rows = [
            extract_features(
                sample,
                include_order_features=include_order_features,
                feature_groups=feature_groups,
            )
            for sample in group
        ]
        utility_rows = [candidate_utility_key(sample) for sample in group]
        for left_index in range(len(group)):
            for right_index in range(left_index + 1, len(group)):
                left_utility = utility_rows[left_index]
                right_utility = utility_rows[right_index]
                if left_utility == right_utility:
                    continue
                if left_utility > right_utility:
                    better_index, worse_index = left_index, right_index
                else:
                    better_index, worse_index = right_index, left_index
                better_minus_worse = diff_features(
                    feature_rows[better_index],
                    feature_rows[worse_index],
                )
                worse_minus_better = diff_features(
                    feature_rows[worse_index],
                    feature_rows[better_index],
                )
                target_examples = (
                    outcome_examples
                    if left_utility[0] != right_utility[0]
                    else hp_examples
                )
                target_examples.append((1, better_minus_worse))
                target_examples.append((0, worse_minus_better))
    return outcome_examples, hp_examples


def training_examples_for_groups(
    groups: dict[str, list[dict[str, Any]]],
    *,
    include_order_features: bool,
    feature_groups: frozenset[str],
    target_mode: str,
    training_mode: str,
) -> list[tuple[int, dict[str, float]]]:
    if training_mode == "binary":
        return flatten_training_examples(
            groups,
            include_order_features=include_order_features,
            feature_groups=feature_groups,
            target_mode=target_mode,
        )
    if training_mode == "pairwise-utility":
        return flatten_pairwise_utility_examples(
            groups,
            include_order_features=include_order_features,
            feature_groups=feature_groups,
        )
    if training_mode == "decomposed-utility":
        raise ValueError("decomposed-utility trains separate components; use score_groups_with_training")
    raise ValueError(f"unknown training mode: {training_mode}")


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


def selected_rank(group: list[dict[str, Any]], scores: list[float], *, target_mode: str) -> int:
    positives = set(positive_target_indices(group, target_mode))
    ranked = sorted(enumerate(zip(group, scores)), key=lambda item: item[1][1], reverse=True)
    for rank, (index, _item) in enumerate(ranked, start=1):
        if index in positives:
            return rank
    return len(group) + 1


def evaluate_ordered_index(
    groups: dict[str, list[dict[str, Any]]],
    *,
    target_mode: str,
) -> dict[str, float]:
    group_scores = {}
    for key, group in groups.items():
        group_scores[key] = [-sample_ordered_index(sample) for sample in group]
    return metrics_from_group_scores(groups, group_scores, target_mode=target_mode)


def evaluate_model(
    groups: dict[str, list[dict[str, Any]]],
    weights: dict[int, float],
    bias: float,
    *,
    dim: int,
    include_order_features: bool,
    feature_groups: frozenset[str],
    target_mode: str,
) -> dict[str, float]:
    group_scores = {}
    for key, group in groups.items():
        scores = []
        for sample in group:
            features = extract_features(
                sample,
                include_order_features=include_order_features,
                feature_groups=feature_groups,
            )
            scores.append(dot(weights, hashed_features(features, dim), bias))
        group_scores[key] = scores
    return metrics_from_group_scores(groups, group_scores, target_mode=target_mode)


def score_group_with_single_model(
    group: list[dict[str, Any]],
    weights: dict[int, float],
    bias: float,
    *,
    dim: int,
    include_order_features: bool,
    feature_groups: frozenset[str],
) -> list[float]:
    scores = []
    for sample in group:
        features = extract_features(
            sample,
            include_order_features=include_order_features,
            feature_groups=feature_groups,
        )
        scores.append(dot(weights, hashed_features(features, dim), bias))
    return scores


def score_groups_with_training(
    train_groups: dict[str, list[dict[str, Any]]],
    eval_groups: dict[str, list[dict[str, Any]]],
    *,
    dim: int,
    epochs: int,
    learning_rate: float,
    l2: float,
    seed: int,
    include_order_features: bool,
    feature_groups: frozenset[str],
    target_mode: str,
    training_mode: str,
) -> tuple[dict[str, list[float]], dict[str, Any]]:
    if not train_groups or not eval_groups:
        return {}, {"training_mode": training_mode, "examples": 0}

    if training_mode == "decomposed-utility":
        outcome_examples, hp_examples = flatten_decomposed_utility_examples(
            train_groups,
            include_order_features=include_order_features,
            feature_groups=feature_groups,
        )
        if not outcome_examples and not hp_examples:
            return {}, {
                "training_mode": training_mode,
                "examples": 0,
                "outcome_examples": 0,
                "hp_examples": 0,
            }
        if outcome_examples:
            outcome_weights, outcome_bias = train_logistic(
                outcome_examples,
                dim=dim,
                epochs=epochs,
                learning_rate=learning_rate,
                l2=l2,
                seed=seed,
            )
        else:
            outcome_weights, outcome_bias = {}, 0.0
        if hp_examples:
            hp_weights, hp_bias = train_logistic(
                hp_examples,
                dim=dim,
                epochs=epochs,
                learning_rate=learning_rate,
                l2=l2,
                seed=seed + 1009,
            )
        else:
            hp_weights, hp_bias = {}, 0.0

        group_scores: dict[str, list[float]] = {}
        for key, group in eval_groups.items():
            outcome_scores = score_group_with_single_model(
                group,
                outcome_weights,
                outcome_bias,
                dim=dim,
                include_order_features=include_order_features,
                feature_groups=feature_groups,
            )
            hp_scores = score_group_with_single_model(
                group,
                hp_weights,
                hp_bias,
                dim=dim,
                include_order_features=include_order_features,
                feature_groups=feature_groups,
            )
            group_scores[key] = [
                DECOMPOSED_OUTCOME_WEIGHT * outcome + DECOMPOSED_HP_WEIGHT * hp
                for outcome, hp in zip(outcome_scores, hp_scores)
            ]
        return group_scores, {
            "training_mode": training_mode,
            "examples": len(outcome_examples) + len(hp_examples),
            "outcome_examples": len(outcome_examples),
            "hp_examples": len(hp_examples),
            "outcome_weight": DECOMPOSED_OUTCOME_WEIGHT,
            "hp_weight": DECOMPOSED_HP_WEIGHT,
        }

    train_examples = training_examples_for_groups(
        train_groups,
        include_order_features=include_order_features,
        feature_groups=feature_groups,
        target_mode=target_mode,
        training_mode=training_mode,
    )
    if not train_examples:
        return {}, {"training_mode": training_mode, "examples": 0}
    weights, bias = train_logistic(
        train_examples,
        dim=dim,
        epochs=epochs,
        learning_rate=learning_rate,
        l2=l2,
        seed=seed,
    )
    return {
        key: score_group_with_single_model(
            group,
            weights,
            bias,
            dim=dim,
            include_order_features=include_order_features,
            feature_groups=feature_groups,
        )
        for key, group in eval_groups.items()
    }, {"training_mode": training_mode, "examples": len(train_examples)}


def source_cross_validated_model_metrics(
    groups: dict[str, list[dict[str, Any]]],
    *,
    dim: int,
    epochs: int,
    learning_rate: float,
    l2: float,
    seed: int,
    include_order_features: bool,
    feature_groups: frozenset[str],
    target_mode: str,
    training_mode: str,
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
            "avg_positive_targets": 0.0,
        }
        return metrics, {"folds": 0, "source_units": len(units)}, {}
    out_of_fold_scores: dict[str, list[float]] = {}
    folds = 0
    for fold_index, held_out_unit in enumerate(sorted(units)):
        test_keys = set(units[held_out_unit])
        train_groups = {key: group for key, group in groups.items() if key not in test_keys}
        test_groups = {key: group for key, group in groups.items() if key in test_keys}
        fold_scores, meta = score_groups_with_training(
            train_groups,
            test_groups,
            dim=dim,
            epochs=epochs,
            learning_rate=learning_rate,
            l2=l2,
            seed=seed + fold_index,
            include_order_features=include_order_features,
            feature_groups=feature_groups,
            target_mode=target_mode,
            training_mode=training_mode,
        )
        if not fold_scores or not meta.get("examples"):
            continue
        out_of_fold_scores.update(fold_scores)
        folds += 1
    scores = out_of_fold_scores if return_scores else {}
    return metrics_from_group_scores(groups, out_of_fold_scores, target_mode=target_mode), {
        "folds": folds,
        "source_units": len(units),
    }, scores


def print_source_cv_feature_group_comparison(
    groups: dict[str, list[dict[str, Any]]],
    *,
    dim: int,
    epochs: int,
    learning_rate: float,
    l2: float,
    seed: int,
    include_order_features: bool,
    target_mode: str,
    training_mode: str,
    report_mode: str,
) -> None:
    print("  feature_group_compare:")
    variants: list[tuple[str, frozenset[str]]] = [("base", frozenset())]
    variants.extend((f"+{name}", frozenset({name})) for name in EXPERIMENTAL_FEATURE_GROUPS)
    variants.append(("all", frozenset(EXPERIMENTAL_FEATURE_GROUPS)))
    seen: set[frozenset[str]] = set()
    for label, feature_groups in variants:
        if feature_groups in seen:
            continue
        seen.add(feature_groups)
        metrics, _meta, _scores = source_cross_validated_model_metrics(
            groups,
            dim=dim,
            epochs=epochs,
            learning_rate=learning_rate,
            l2=l2,
            seed=seed,
            include_order_features=include_order_features,
            feature_groups=feature_groups,
            target_mode=target_mode,
            training_mode=training_mode,
        )
        print_metrics(f"feature_group:{label}", metrics, report_mode=report_mode)


def print_source_cv_target_mode_comparison(
    groups: dict[str, list[dict[str, Any]]],
    *,
    dim: int,
    epochs: int,
    learning_rate: float,
    l2: float,
    seed: int,
    include_order_features: bool,
    feature_groups: frozenset[str],
    training_mode: str,
    report_mode: str,
) -> None:
    print("  target_mode_compare:")
    for target_mode in TARGET_MODES:
        metrics, _meta, _scores = source_cross_validated_model_metrics(
            groups,
            dim=dim,
            epochs=epochs,
            learning_rate=learning_rate,
            l2=l2,
            seed=seed,
            include_order_features=include_order_features,
            feature_groups=feature_groups,
            target_mode=target_mode,
            training_mode=training_mode,
        )
        print_metrics(f"target_mode:{target_mode}", metrics, report_mode=report_mode)


def print_source_cv_training_mode_comparison(
    groups: dict[str, list[dict[str, Any]]],
    *,
    dim: int,
    epochs: int,
    learning_rate: float,
    l2: float,
    seed: int,
    include_order_features: bool,
    feature_groups: frozenset[str],
    target_mode: str,
    report_mode: str,
) -> None:
    print("  training_mode_compare:")
    for training_mode in TRAINING_MODES:
        metrics, _meta, _scores = source_cross_validated_model_metrics(
            groups,
            dim=dim,
            epochs=epochs,
            learning_rate=learning_rate,
            l2=l2,
            seed=seed,
            include_order_features=include_order_features,
            feature_groups=feature_groups,
            target_mode=target_mode,
            training_mode=training_mode,
        )
        print_metrics(f"training_mode:{training_mode}", metrics, report_mode=report_mode)


def metrics_from_group_scores(
    groups: dict[str, list[dict[str, Any]]],
    group_scores: dict[str, list[float]],
    *,
    target_mode: str,
) -> dict[str, float]:
    ranks = []
    hp_gains = []
    target_hp_regrets = []
    positive_gain = 0
    negative_gain = 0
    target_missed = 0
    target_outcome_missed = 0
    target_outcome_matched = 0
    positive_target_counts = []
    for key, group in groups.items():
        scores = group_scores.get(key) or []
        if len(scores) != len(group):
            continue
        positives = set(positive_target_indices(group, target_mode))
        if not positives:
            continue
        positive_target_counts.append(len(positives))
        ranks.append(selected_rank(group, scores, target_mode=target_mode))
        top_index = max(range(len(group)), key=lambda index: scores[index])
        current_index = min(
            range(len(group)),
            key=lambda index: sample_ordered_index(group[index]),
        )
        target_index = primary_target_index(group)
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
            if candidate_terminal_signature(group[target_index]) == candidate_terminal_signature(top):
                target_outcome_matched += 1
            if top_index not in positives:
                target_missed += 1
            if top_index not in positives and candidate_terminal_signature(group[target_index]) != candidate_terminal_signature(top):
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
            "avg_positive_targets": 0.0,
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
        "avg_positive_targets": (
            sum(positive_target_counts) / len(positive_target_counts) if positive_target_counts else 0.0
        ),
    }


def selected_indices_for_scores(
    group: list[dict[str, Any]], scores: list[float]
) -> tuple[int, int, int | None]:
    model_index = max(range(len(group)), key=lambda index: scores[index])
    ordered_index = min(range(len(group)), key=lambda index: sample_ordered_index(group[index]))
    target_index = next((index for index, sample in enumerate(group) if is_selected(sample)), None)
    return ordered_index, model_index, target_index


def utility_delta_score(left: dict[str, Any], right: dict[str, Any]) -> float:
    left_tier, left_hp, left_loss_key = candidate_utility_key(left)
    right_tier, right_hp, right_loss_key = candidate_utility_key(right)
    return (
        (left_tier - right_tier) * 10000.0
        + (left_hp - right_hp)
        + (left_loss_key - right_loss_key) * 0.01
    )


def utility_delta_summary(left: dict[str, Any], right: dict[str, Any]) -> str:
    left_tier, left_hp, left_loss_key = candidate_utility_key(left)
    right_tier, right_hp, right_loss_key = candidate_utility_key(right)
    return (
        f"tier_delta={left_tier - right_tier:+d} "
        f"hp_delta={left_hp - right_hp:+d} "
        f"loss_key_delta={left_loss_key - right_loss_key:+d}"
    )


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
            preview.append(f"{display_card_from_normalized(card)}{display_action_target_suffix(text)}")
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
    target_mode: str,
) -> list[tuple[float, str]]:
    rows: list[tuple[float, str]] = []
    for key, group in groups.items():
        scores = group_scores.get(key) or []
        if len(scores) != len(group):
            continue
        ordered_index, model_index, target_index = selected_indices_for_scores(group, scores)
        if target_index is None:
            continue
        positive_count = len(positive_target_indices(group, target_mode))
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
                f"  target_mode={target_mode} positive_targets={positive_count}",
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
    target_mode: str,
) -> None:
    rows = interesting_case_rows(groups, scores, kind=kind, limit=limit, target_mode=target_mode)
    print(f"  cases:{title} count={len(rows)}")
    for _score, body in rows:
        print(body)


def training_mode_case_rows(
    groups: dict[str, list[dict[str, Any]]],
    reference_scores: dict[str, list[float]],
    candidate_scores: dict[str, list[float]],
    *,
    kind: str,
    limit: int,
    target_mode: str,
    reference_label: str,
    candidate_label: str,
) -> list[tuple[float, str]]:
    rows: list[tuple[float, str]] = []
    for key, group in groups.items():
        ref_scores = reference_scores.get(key) or []
        cand_scores = candidate_scores.get(key) or []
        if len(ref_scores) != len(group) or len(cand_scores) != len(group):
            continue
        positives = set(positive_target_indices(group, target_mode))
        if not positives:
            continue
        ordered_index = min(range(len(group)), key=lambda index: sample_ordered_index(group[index]))
        reference_index = max(range(len(group)), key=lambda index: ref_scores[index])
        candidate_index = max(range(len(group)), key=lambda index: cand_scores[index])
        target_index = primary_target_index(group)
        if target_index is None or reference_index == candidate_index:
            continue
        reference = group[reference_index]
        candidate_pick = group[candidate_index]
        target = group[target_index]
        delta = utility_delta_score(candidate_pick, reference)
        reference_regret = utility_delta_score(target, reference)
        candidate_regret = utility_delta_score(target, candidate_pick)
        if kind == "better" and delta <= 0:
            continue
        if kind == "worse" and delta >= 0:
            continue
        if kind == "both-bad" and not (
            reference_index not in positives and candidate_index not in positives
        ):
            continue
        if kind == "disagree" and reference_index == candidate_index:
            continue
        sort_key = {
            "better": delta,
            "worse": -delta,
            "both-bad": max(reference_regret, candidate_regret),
            "disagree": abs(delta),
        }[kind]
        body = "\n".join(
            [
                f"case={source_label(group[0])} state={state_summary(group[0])}",
                f"  target_mode={target_mode} positive_targets={len(positives)}",
                f"  ordered: {plan_summary(group[ordered_index])}",
                f"  {reference_label}: {plan_summary(reference)}",
                (
                    f"  {candidate_label}: {plan_summary(candidate_pick)} "
                    f"delta_vs_{reference_label}={utility_delta_summary(candidate_pick, reference)}"
                ),
                (
                    f"  target: {plan_summary(target)} "
                    f"{reference_label}_regret={utility_delta_summary(target, reference)} "
                    f"{candidate_label}_regret={utility_delta_summary(target, candidate_pick)}"
                ),
            ]
        )
        rows.append((float(sort_key), body))
    rows.sort(key=lambda item: item[0], reverse=True)
    return rows[:limit]


def print_training_mode_case_rows(
    groups: dict[str, list[dict[str, Any]]],
    *,
    dim: int,
    epochs: int,
    learning_rate: float,
    l2: float,
    seed: int,
    include_order_features: bool,
    feature_groups: frozenset[str],
    target_mode: str,
    report_mode: str,
    reference_training_mode: str,
    candidate_training_mode: str,
    kind: str,
    limit: int,
) -> None:
    reference_metrics, _reference_meta, reference_scores = source_cross_validated_model_metrics(
        groups,
        dim=dim,
        epochs=epochs,
        learning_rate=learning_rate,
        l2=l2,
        seed=seed,
        include_order_features=include_order_features,
        feature_groups=feature_groups,
        target_mode=target_mode,
        training_mode=reference_training_mode,
        return_scores=True,
    )
    candidate_metrics, _candidate_meta, candidate_scores = source_cross_validated_model_metrics(
        groups,
        dim=dim,
        epochs=epochs,
        learning_rate=learning_rate,
        l2=l2,
        seed=seed,
        include_order_features=include_order_features,
        feature_groups=feature_groups,
        target_mode=target_mode,
        training_mode=candidate_training_mode,
        return_scores=True,
    )
    print(
        "  training_mode_case_compare:"
        f" reference={reference_training_mode} candidate={candidate_training_mode}"
    )
    print_metrics(f"reference:{reference_training_mode}", reference_metrics, report_mode=report_mode)
    print_metrics(f"candidate:{candidate_training_mode}", candidate_metrics, report_mode=report_mode)
    kinds = ("better", "worse", "both-bad") if kind == "all" else (kind,)
    for selected_kind in kinds:
        rows = training_mode_case_rows(
            groups,
            reference_scores,
            candidate_scores,
            kind=selected_kind,
            limit=limit,
            target_mode=target_mode,
            reference_label=reference_training_mode,
            candidate_label=candidate_training_mode,
        )
        print(f"  cases:training_mode_{selected_kind} count={len(rows)}")
        for _score, body in rows:
            print(body)


def feature_weight_report(
    weights: dict[int, float],
    groups: dict[str, list[dict[str, Any]]],
    *,
    dim: int,
    include_order_features: bool,
    feature_groups: frozenset[str],
    limit: int,
) -> list[tuple[str, float]]:
    bucket_to_names: dict[int, Counter[str]] = defaultdict(Counter)
    for group in groups.values():
        for sample in group:
            features = extract_features(
                sample,
                include_order_features=include_order_features,
                feature_groups=feature_groups,
            )
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
            f"worse_hp={metrics.get('negative_hp_gain', 0.0):.0f} "
            f"pos_avg={metrics.get('avg_positive_targets', 0.0):.2f}"
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
        f"avg_hp_regret_to_target={metrics.get('avg_hp_regret_to_target', 0.0):+.2f} "
        f"avg_positive_targets={metrics.get('avg_positive_targets', 0.0):.2f}"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "inputs",
        nargs="*",
        type=Path,
        help=(
            "CombatSearchGuidanceSampleV1, CombatActionProbeSampleV1, or "
            "CombatTurnPlanProbeSampleV1 JSONL, plus CombatTacticalEpisodeV1 JSONL"
            " expanded at load time"
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
    parser.add_argument(
        "--discover-tactical-episodes",
        action="append",
        nargs="+",
        type=Path,
        metavar="ROOT",
        help=(
            "Discover *.tactical_episode*.jsonl under ROOT and expand "
            "CombatTacticalEpisodeV1 records into turn-plan candidate samples."
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
    parser.add_argument(
        "--feature-groups",
        nargs="*",
        choices=EXPERIMENTAL_FEATURE_GROUPS,
        default=[],
        help="Opt-in experimental feature groups. Default keeps the committed baseline unchanged.",
    )
    parser.add_argument(
        "--target-mode",
        choices=TARGET_MODES,
        default="selected",
        help="Training/evaluation target definition. selected preserves the original single oracle label.",
    )
    parser.add_argument(
        "--training-mode",
        choices=TRAINING_MODES,
        default="binary",
        help="Training sample construction. binary preserves the current baseline.",
    )
    parser.add_argument(
        "--compare-feature-groups",
        action="store_true",
        help="For source-cv, print base/+group/all comparisons without changing the selected run.",
    )
    parser.add_argument(
        "--compare-target-modes",
        action="store_true",
        help="For source-cv, compare selected vs equivalent target definitions.",
    )
    parser.add_argument(
        "--compare-training-modes",
        action="store_true",
        help="For source-cv, compare binary labels vs pairwise diagnostic utility training.",
    )
    parser.add_argument(
        "--show-training-cases",
        type=int,
        default=0,
        help=(
            "For source-cv, print case comparisons between two training modes. "
            "Defaults compare binary against decomposed-utility."
        ),
    )
    parser.add_argument(
        "--training-case-kind",
        choices=("better", "worse", "both-bad", "disagree", "all"),
        default="all",
        help="Which training-mode disagreement cases to show.",
    )
    parser.add_argument(
        "--reference-training-mode",
        choices=TRAINING_MODES,
        default="binary",
        help="Reference mode for --show-training-cases.",
    )
    parser.add_argument(
        "--candidate-training-mode",
        choices=TRAINING_MODES,
        default="decomposed-utility",
        help="Candidate mode for --show-training-cases.",
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
    feature_groups = frozenset(args.feature_groups)
    target_mode = args.target_mode
    training_mode = args.training_mode
    if args.show_training_cases > 0 and args.split_mode != "source-cv":
        parser.error("--show-training-cases requires --split-mode source-cv")

    input_paths = list(args.inputs)
    if args.discover_turn_plan_probes:
        roots = [root for group in args.discover_turn_plan_probes for root in group]
        input_paths.extend(discover_turn_plan_probe_paths(roots))
    if args.discover_tactical_episodes:
        roots = [root for group in args.discover_tactical_episodes for root in group]
        input_paths.extend(discover_tactical_episode_paths(roots))
    input_paths = sorted(set(input_paths))
    if not input_paths:
        parser.error("provide JSONL inputs or --discover-turn-plan-probes ROOT")

    samples = load_samples(input_paths)
    groups = usable_groups(samples)
    target_counts = Counter()
    sample_schema_counts = Counter(str(sample.get("schema_name")) for sample in samples)
    source_schema_counts = Counter(str(sample.get("_source_schema_name")) for sample in samples)
    for group in groups.values():
        for sample in group:
            target_counts["selected" if is_selected(sample) else "not_selected"] += 1
    print("CombatSearchRankingBaseline")
    print(
        f"  input_files={len(input_paths)} samples={len(samples)} "
        f"usable_groups={len(groups)} labels={dict(target_counts)}"
    )
    print(f"  sample_schemas={dict(sample_schema_counts)}")
    print(f"  source_schemas={dict(source_schema_counts)}")
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
            feature_groups=feature_groups,
            target_mode=target_mode,
            training_mode=training_mode,
            return_scores=args.show_cases > 0,
        )
        print(
            f"  split=mode:source-cv source_units:{cv_meta['source_units']} "
            f"folds:{cv_meta['folds']} target_mode:{target_mode} training_mode:{training_mode}"
        )
        print_metrics(
            "ordered_index_all",
            evaluate_ordered_index(groups, target_mode=target_mode),
            report_mode=args.report_mode,
        )
        print_metrics("logistic_source_cv", cv_metrics, report_mode=args.report_mode)
        if args.compare_feature_groups:
            print_source_cv_feature_group_comparison(
                groups,
                dim=args.dim,
                epochs=args.epochs,
                learning_rate=args.learning_rate,
                l2=args.l2,
                seed=args.seed,
                include_order_features=args.include_order_features,
                target_mode=target_mode,
                training_mode=training_mode,
                report_mode=args.report_mode,
            )
        if args.compare_target_modes:
            print_source_cv_target_mode_comparison(
                groups,
                dim=args.dim,
                epochs=args.epochs,
                learning_rate=args.learning_rate,
                l2=args.l2,
                seed=args.seed,
                include_order_features=args.include_order_features,
                feature_groups=feature_groups,
                training_mode=training_mode,
                report_mode=args.report_mode,
            )
        if args.compare_training_modes:
            print_source_cv_training_mode_comparison(
                groups,
                dim=args.dim,
                epochs=args.epochs,
                learning_rate=args.learning_rate,
                l2=args.l2,
                seed=args.seed,
                include_order_features=args.include_order_features,
                feature_groups=feature_groups,
                target_mode=target_mode,
                report_mode=args.report_mode,
            )
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
                        target_mode=target_mode,
                    )
        if args.show_training_cases > 0:
            print_training_mode_case_rows(
                groups,
                dim=args.dim,
                epochs=args.epochs,
                learning_rate=args.learning_rate,
                l2=args.l2,
                seed=args.seed,
                include_order_features=args.include_order_features,
                feature_groups=feature_groups,
                target_mode=target_mode,
                report_mode=args.report_mode,
                reference_training_mode=args.reference_training_mode,
                candidate_training_mode=args.candidate_training_mode,
                kind=args.training_case_kind,
                limit=args.show_training_cases,
            )
        if training_mode == "decomposed-utility":
            outcome_examples, hp_examples = flatten_decomposed_utility_examples(
                groups,
                include_order_features=args.include_order_features,
                feature_groups=feature_groups,
            )
            print(
                "  decomposed_utility_full_data="
                f"outcome_examples:{len(outcome_examples)} hp_examples:{len(hp_examples)} "
                f"outcome_weight:{DECOMPOSED_OUTCOME_WEIGHT:.1f} "
                f"hp_weight:{DECOMPOSED_HP_WEIGHT:.1f}"
            )
            if args.report_mode == "full":
                if outcome_examples:
                    outcome_weights, _outcome_bias = train_logistic(
                        outcome_examples,
                        dim=args.dim,
                        epochs=args.epochs,
                        learning_rate=args.learning_rate,
                        l2=args.l2,
                        seed=args.seed,
                    )
                    print("  top_weighted_features_full_data:outcome_component")
                    for name, weight in feature_weight_report(
                        outcome_weights,
                        groups,
                        dim=args.dim,
                        include_order_features=args.include_order_features,
                        feature_groups=feature_groups,
                        limit=args.top_features,
                    ):
                        print(f"    {weight:+.4f} {name}")
                if hp_examples:
                    hp_weights, _hp_bias = train_logistic(
                        hp_examples,
                        dim=args.dim,
                        epochs=args.epochs,
                        learning_rate=args.learning_rate,
                        l2=args.l2,
                        seed=args.seed + 1009,
                    )
                    print("  top_weighted_features_full_data:hp_component")
                    for name, weight in feature_weight_report(
                        hp_weights,
                        groups,
                        dim=args.dim,
                        include_order_features=args.include_order_features,
                        feature_groups=feature_groups,
                        limit=args.top_features,
                    ):
                        print(f"    {weight:+.4f} {name}")
            return

        train_examples = training_examples_for_groups(
            groups,
            include_order_features=args.include_order_features,
            feature_groups=feature_groups,
            target_mode=target_mode,
            training_mode=training_mode,
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
                feature_groups=feature_groups,
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
        evaluate_ordered_index(train_groups, target_mode=target_mode),
        report_mode=args.report_mode,
    )
    print_metrics(
        "ordered_index_test",
        evaluate_ordered_index(test_groups, target_mode=target_mode),
        report_mode=args.report_mode,
    )

    train_scores, train_meta = score_groups_with_training(
        train_groups,
        train_groups,
        dim=args.dim,
        epochs=args.epochs,
        learning_rate=args.learning_rate,
        l2=args.l2,
        seed=args.seed,
        include_order_features=args.include_order_features,
        feature_groups=feature_groups,
        target_mode=target_mode,
        training_mode=training_mode,
    )
    test_scores, _test_meta = score_groups_with_training(
        train_groups,
        test_groups,
        dim=args.dim,
        epochs=args.epochs,
        learning_rate=args.learning_rate,
        l2=args.l2,
        seed=args.seed,
        include_order_features=args.include_order_features,
        feature_groups=feature_groups,
        target_mode=target_mode,
        training_mode=training_mode,
    )
    if not train_meta.get("examples") or not test_scores:
        print("  logistic=skipped_not_enough_split_data")
        return
    print_metrics(
        "logistic_train",
        metrics_from_group_scores(train_groups, train_scores, target_mode=target_mode),
        report_mode=args.report_mode,
    )
    print_metrics(
        "logistic_test",
        metrics_from_group_scores(test_groups, test_scores, target_mode=target_mode),
        report_mode=args.report_mode,
    )
    if args.report_mode == "full":
        if training_mode == "decomposed-utility":
            print(
                "  top_weighted_features=skipped_decomposed_mode "
                "use source-cv full report for component feature weights"
            )
        else:
            train_examples = training_examples_for_groups(
                train_groups,
                include_order_features=args.include_order_features,
                feature_groups=feature_groups,
                target_mode=target_mode,
                training_mode=training_mode,
            )
            weights, _bias = train_logistic(
                train_examples,
                dim=args.dim,
                epochs=args.epochs,
                learning_rate=args.learning_rate,
                l2=args.l2,
                seed=args.seed,
            )
            print("  top_weighted_features:")
            for name, weight in feature_weight_report(
                weights,
                train_groups,
                dim=args.dim,
                include_order_features=args.include_order_features,
                feature_groups=feature_groups,
                limit=args.top_features,
            ):
                print(f"    {weight:+.4f} {name}")


if __name__ == "__main__":
    main()
