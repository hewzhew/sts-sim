#!/usr/bin/env python3
from __future__ import annotations

import json
import re
from collections import defaultdict
from functools import lru_cache
from pathlib import Path
from typing import Any


_REPO_ROOT = Path(__file__).resolve().parents[2]

_CARD_STRUCTURE_FLAGS = (
    "strength_enabler",
    "strength_payoff",
    "multi_attack_payoff",
    "setup_piece",
    "scaling_piece",
    "engine_piece",
    "exhaust_engine",
    "exhaust_outlet",
    "draw_core",
    "block_core",
    "resource_conversion",
    "status_engine",
    "vuln_payoff",
    "block_payoff",
)

_CARD_FACT_FLAGS = (
    "draws_cards",
    "gains_energy",
    "applies_weak",
    "applies_vuln",
    "applies_frail",
    "self_damage",
    "multi_hit",
    "aoe",
    "produces_status",
    "exhausts_other_cards",
    "target_sensitive",
    "combat_heal",
    "conditional_free",
    "self_replicating",
    "random_generation",
    "cost_manipulation_sensitive",
)

_DERIVED_CARD_FLAGS = (
    "frontload_attack",
    "recursion_tool",
    "strike_payoff",
    "hybrid_block_attack",
)

_PROFILE_FIELDS = (
    "strength_enablers",
    "strength_payoffs",
    "multi_attack_payoffs",
    "frontload_attacks",
    "setup_pieces",
    "scaling_pieces",
    "engine_pieces",
    "exhaust_engines",
    "exhaust_outlets",
    "block_core",
    "block_payoffs",
    "hybrid_block_attacks",
    "draw_sources",
    "resource_conversions",
    "status_engines",
    "status_generators",
    "control_tools",
    "recursion_tools",
    "combat_heals",
    "self_damage_sources",
    "strike_payoffs",
    "vuln_payoffs",
)

REWARD_GAP_TARGET_KEYS = (
    "strength_gap_fill",
    "exhaust_gap_fill",
    "block_gap_fill",
    "status_gap_fill",
    "draw_gap_fill",
    "control_gap_fill",
    "sustain_gap_fill",
    "pressure_gap_fill",
    "strike_gap_fill",
)


def _extract_card_variants(text: str) -> list[str]:
    return re.findall(r"\b([A-Z][A-Za-z0-9_]*)\b", text)


def _normalize_card_key(text: Any) -> str:
    normalized = re.sub(r"[^a-z0-9]+", "-", str(text or "").lower()).strip("-")
    return normalized


@lru_cache(maxsize=1)
def _java_name_to_variant_map() -> dict[str, str]:
    text = (_REPO_ROOT / "src" / "content" / "cards" / "mod.rs").read_text(encoding="utf-8")
    return {
        java_name: variant
        for variant, java_name in re.findall(r'CardId::(\w+)\s*=>\s*"([^"]+)"', text)
    }


@lru_cache(maxsize=1)
def _structure_flags_by_variant() -> dict[str, set[str]]:
    text = (_REPO_ROOT / "src" / "bot" / "facts" / "card_structure.rs").read_text(encoding="utf-8")
    by_variant: dict[str, set[str]] = defaultdict(set)
    pattern = re.compile(r"^\s*([A-Za-z0-9_|\s]+?)\s*=>\s*([^\n,]+),\s*$", re.MULTILINE)
    for variants_block, expr in pattern.findall(text):
        if "_" in variants_block:
            continue
        variants = _extract_card_variants(variants_block)
        flags = {flag.lower() for flag in re.findall(r"S::([A-Z_]+)", expr)}
        for variant in variants:
            by_variant[variant].update(flags)
    return {variant: set(flags) for variant, flags in by_variant.items()}


@lru_cache(maxsize=1)
def _fact_flags_by_variant() -> dict[str, set[str]]:
    text = (_REPO_ROOT / "src" / "bot" / "facts" / "card_facts.rs").read_text(encoding="utf-8")
    by_variant: dict[str, set[str]] = defaultdict(set)
    for field in _CARD_FACT_FLAGS:
        match = re.search(
            rf"{field}:\s*matches!\(\s*card_id,\s*(.*?)\s*\),",
            text,
            flags=re.DOTALL,
        )
        if not match:
            continue
        for variant in _extract_card_variants(match.group(1)):
            by_variant[variant].add(field)
    return {variant: set(flags) for variant, flags in by_variant.items()}


@lru_cache(maxsize=1)
def _card_semantics_by_java_name() -> dict[str, dict[str, int]]:
    java_to_variant = _java_name_to_variant_map()
    structure_by_variant = _structure_flags_by_variant()
    fact_by_variant = _fact_flags_by_variant()
    semantics: dict[str, dict[str, int]] = {}
    for java_name, variant in java_to_variant.items():
        normalized_java_name = _normalize_card_key(java_name)
        structure_flags = structure_by_variant.get(variant, set())
        fact_flags = fact_by_variant.get(variant, set())
        row = {field: 0 for field in _CARD_STRUCTURE_FLAGS + _CARD_FACT_FLAGS + _DERIVED_CARD_FLAGS}
        for flag in structure_flags:
            if flag in row:
                row[flag] = 1
        for flag in fact_flags:
            if flag in row:
                row[flag] = 1
        row["control_tool"] = int(row["applies_weak"] or row["applies_vuln"] or row["applies_frail"])
        row["draw_source"] = int(row["draws_cards"] or row["draw_core"])
        row["status_generator"] = int(row["produces_status"])
        row["recursion_tool"] = int(normalized_java_name in {"headbutt"})
        row["strike_payoff"] = int(normalized_java_name in {"perfected-strike"})
        row["hybrid_block_attack"] = int(normalized_java_name in {"iron-wave"})
        row["frontload_attack"] = int(
            row["multi_attack_payoff"]
            or row["multi_hit"]
            or row["aoe"]
            or row["combat_heal"]
            or row["hybrid_block_attack"]
            or row["strike_payoff"]
            or normalized_java_name in {"clash"}
        )
        semantics[java_name] = row
        semantics[normalized_java_name] = dict(row)
    return semantics


def _deck_semantic_profile(deck_card_counts: dict[str, Any]) -> dict[str, int]:
    profile = {field: 0 for field in _PROFILE_FIELDS}
    semantics_by_name = _card_semantics_by_java_name()
    for card_name, raw_count in (deck_card_counts or {}).items():
        count = int(raw_count or 0)
        if count <= 0:
            continue
        sem = semantics_by_name.get(str(card_name))
        if not sem:
            continue
        profile["strength_enablers"] += count * int(sem["strength_enabler"])
        profile["strength_payoffs"] += count * int(sem["strength_payoff"])
        profile["multi_attack_payoffs"] += count * int(sem["multi_attack_payoff"])
        profile["frontload_attacks"] += count * int(sem["frontload_attack"])
        profile["setup_pieces"] += count * int(sem["setup_piece"])
        profile["scaling_pieces"] += count * int(sem["scaling_piece"])
        profile["engine_pieces"] += count * int(sem["engine_piece"])
        profile["exhaust_engines"] += count * int(sem["exhaust_engine"])
        profile["exhaust_outlets"] += count * int(sem["exhaust_outlet"])
        profile["block_core"] += count * int(sem["block_core"])
        profile["block_payoffs"] += count * int(sem["block_payoff"])
        profile["hybrid_block_attacks"] += count * int(sem["hybrid_block_attack"])
        profile["draw_sources"] += count * int(sem["draw_source"])
        profile["resource_conversions"] += count * int(sem["resource_conversion"])
        profile["status_engines"] += count * int(sem["status_engine"])
        profile["status_generators"] += count * int(sem["status_generator"])
        profile["control_tools"] += count * int(sem["control_tool"])
        profile["recursion_tools"] += count * int(sem["recursion_tool"])
        profile["combat_heals"] += count * int(sem["combat_heal"])
        profile["self_damage_sources"] += count * int(sem["self_damage"])
        profile["strike_payoffs"] += count * int(sem["strike_payoff"])
        profile["vuln_payoffs"] += count * int(sem["vuln_payoff"])
    return profile


def macro_candidate_card_semantics(row: dict[str, Any]) -> dict[str, int]:
    payload = row.get("option_payload") or {}
    card_name = payload.get("card_id") or payload.get("card_name")
    semantics_by_name = _card_semantics_by_java_name()
    semantics = semantics_by_name.get(str(card_name or "")) or semantics_by_name.get(_normalize_card_key(card_name))
    if semantics:
        return dict(semantics)
    return {
        field: 0
        for field in _CARD_STRUCTURE_FLAGS
        + _CARD_FACT_FLAGS
        + _DERIVED_CARD_FLAGS
        + ("control_tool", "draw_source", "status_generator")
    }


def reward_package_gap_targets(row: dict[str, Any]) -> dict[str, float]:
    base = {key: 0.0 for key in REWARD_GAP_TARGET_KEYS}
    if str(row.get("source_kind") or "") != "reward" or str(row.get("option_kind") or "") != "reward_take_card":
        return base

    state_context = _macro_state_context(row)
    deck_card_counts = state_context.get("deck_card_counts") or {}
    profile = _deck_semantic_profile(deck_card_counts)
    candidate = macro_candidate_card_semantics(row)
    attack_count = int(state_context.get("attack_count") or 0)
    skill_count = int(state_context.get("skill_count") or 0)
    missing_hp = int(state_context.get("player_missing_hp") or 0)
    hp_pct = float(state_context.get("player_hp_pct") or 0.0)
    deck_size = int(state_context.get("deck_size") or 0)
    strike_count = sum(
        int(count or 0)
        for card_name, count in deck_card_counts.items()
        if "strike" in _normalize_card_key(card_name)
    )
    pressure_signal = max(0, 4 - profile["frontload_attacks"]) + max(
        0,
        min(profile["block_core"], 8) - min(profile["frontload_attacks"] + profile["multi_attack_payoffs"], 8),
    )

    strength_need = max(
        float(candidate["strength_enabler"])
        * float(candidate["setup_piece"])
        * float(not candidate["target_sensitive"])
        * float(profile["strength_payoffs"] > profile["strength_enablers"]),
        float(candidate["strength_payoff"])
        * float(profile["strength_enablers"] > profile["strength_payoffs"])
        * float(profile["strength_enablers"] >= 1 or profile["multi_attack_payoffs"] >= 1),
    )
    exhaust_need = max(
        float(candidate["exhaust_engine"]) * float(profile["exhaust_engines"] < 1 and (profile["exhaust_outlets"] >= 1 or profile["status_generators"] >= 1)),
        float(candidate["exhaust_outlet"]) * float(profile["exhaust_outlets"] < 1 and profile["exhaust_engines"] >= 1),
    )
    block_need = max(
        float(candidate["block_core"]) * float(profile["block_core"] < 2 and profile["block_payoffs"] >= 1),
        float(candidate["block_payoff"]) * float(profile["block_payoffs"] < 1 and profile["block_core"] >= 2),
    )
    status_need = max(
        float(candidate["status_engine"]) * float(profile["status_engines"] < 1 and profile["status_generators"] >= 1),
        float(candidate["status_generator"]) * float(profile["status_generators"] < 1 and profile["status_engines"] >= 1),
    )
    draw_need = float(candidate["draw_source"]) * float(profile["draw_sources"] < (2 if deck_size < 18 else 3))
    control_need = max(
        float(candidate["control_tool"]) * float(profile["control_tools"] < 3 or attack_count < max(4, skill_count // 2)),
        float(candidate["recursion_tool"]) * float(profile["recursion_tools"] < 1),
    )
    sustain_need = float(candidate["combat_heal"]) * float(missing_hp >= 12 or hp_pct <= 0.55)
    pressure_need = float(
        candidate["frontload_attack"] or candidate["hybrid_block_attack"] or candidate["multi_attack_payoff"]
    ) * float(pressure_signal >= 2)
    strike_need = float(candidate["strike_payoff"]) * float(
        strike_count >= 4 and (attack_count <= 6 or profile["frontload_attacks"] < 4)
    )

    base["strength_gap_fill"] = float(strength_need)
    base["exhaust_gap_fill"] = float(exhaust_need)
    base["block_gap_fill"] = float(block_need)
    base["status_gap_fill"] = float(status_need)
    base["draw_gap_fill"] = float(draw_need)
    base["control_gap_fill"] = float(control_need)
    base["sustain_gap_fill"] = float(sustain_need)
    base["pressure_gap_fill"] = float(pressure_need)
    base["strike_gap_fill"] = float(strike_need)
    return base


def iter_jsonl_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            text = line.strip()
            if text:
                rows.append(json.loads(text))
    return rows


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, ensure_ascii=False)
        handle.write("\n")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False) + "\n")


def group_option_rows(rows: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    grouped: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        grouped[str(row.get("decision_id") or "")].append(row)
    for group_rows in grouped.values():
        group_rows.sort(key=lambda row: str(row.get("option_id") or ""))
    return grouped


def _macro_state_context(row: dict[str, Any]) -> dict[str, Any]:
    return dict(row.get("state_context") or row.get("state_preview") or {})


def _probe_drop_counts(row: dict[str, Any]) -> tuple[int, int]:
    probe_sanitization = row.get("probe_sanitization") or {}
    dropped_relics = 0
    dropped_potions = 0
    for meta in probe_sanitization.values():
        dropped_relics += len(meta.get("dropped_relics") or [])
        dropped_potions += len(meta.get("dropped_potions") or [])
    return dropped_relics, dropped_potions


def macro_state_feature_dict(row: dict[str, Any]) -> dict[str, Any]:
    features: dict[str, Any] = {}
    state_context = _macro_state_context(row)

    for key in (
        "floor",
        "act",
        "gold",
        "player_current_hp",
        "player_max_hp",
        "player_missing_hp",
        "player_hp_pct",
        "deck_size",
        "deck_upgraded_count",
        "deck_upgradable_count",
        "attack_count",
        "skill_count",
        "power_count",
        "curse_count",
        "status_count",
        "relic_count",
        "potion_count",
        "has_ruby_key",
        "has_emerald_key",
        "has_sapphire_key",
        "future_window_floors",
        "future_elite_count",
        "future_rest_count",
        "future_shop_count",
        "future_reward_count",
        "survival_to_boss",
        "next_act_reached",
        "future_window_end_floor",
        "future_window_end_act",
        "floors_to_boss",
        "heart_path_ready",
    ):
        value = state_context.get(key)
        if value is not None:
            features[f"state::{key}"] = value

    for relic_id in state_context.get("relic_ids") or []:
        features[f"state::has_relic::{relic_id}"] = 1
    deck_card_counts = state_context.get("deck_card_counts") or {}
    for card_id, count in sorted(deck_card_counts.items()):
        features[f"deck::count::{card_id}"] = count
    deck_profile = _deck_semantic_profile(deck_card_counts)
    for key, value in deck_profile.items():
        features[f"profile::{key}"] = value
    return features


def macro_option_only_feature_dict(row: dict[str, Any]) -> dict[str, Any]:
    features: dict[str, Any] = {}
    effect = row.get("effect") or {}
    payload = row.get("option_payload") or {}
    source_kind = str(row.get("source_kind") or "")
    option_kind = str(row.get("option_kind") or "")

    features["meta::source_kind"] = source_kind
    features["meta::screen_type"] = str(row.get("screen_type") or "")
    features["option::kind"] = option_kind
    if source_kind and option_kind:
        features[f"typed::{source_kind}::kind::{option_kind}"] = 1
    if source_kind:
        features[f"typed::{source_kind}::active"] = 1

    for key in ("deck_delta", "gold_delta", "hp_delta", "relic_delta", "potion_delta"):
        features[f"effect::{key}"] = float(effect.get(key) or 0.0)

    price = payload.get("price")
    if price is not None:
        features["option::price"] = float(price)
    upgrades = payload.get("upgrades")
    if upgrades is not None:
        features["option::upgrades"] = int(upgrades)
    for key in ("card_id", "card_name", "relic_id", "relic_name", "potion_id", "potion_name", "label"):
        value = payload.get(key)
        if value:
            features[f"option::{key}"] = str(value)
            if source_kind:
                features[f"typed::{source_kind}::{key}::{value}"] = 1

    if source_kind == "reward":
        features[f"typed::reward::is_skip"] = 1 if option_kind == "reward_skip" else 0
        features[f"typed::reward::is_take_card"] = 1 if option_kind == "reward_take_card" else 0
    elif source_kind == "shop":
        for typed_kind in ("shop_leave", "shop_buy_card", "shop_buy_potion", "shop_purge", "shop_buy_relic"):
            features[f"typed::shop::is::{typed_kind}"] = 1 if option_kind == typed_kind else 0
    elif source_kind == "campfire":
        for typed_kind in ("campfire_rest", "campfire_smith", "campfire_recall"):
            features[f"typed::campfire::is::{typed_kind}"] = 1 if option_kind == typed_kind else 0

    dropped_relics, dropped_potions = _probe_drop_counts(row)
    features["sanitization::dropped_relics"] = dropped_relics
    features["sanitization::dropped_potions"] = dropped_potions
    probe_profile = str(row.get("probe_profile") or "")
    if probe_profile:
        features[f"meta::probe_profile::{probe_profile}"] = 1
    current_policy_score = row.get("current_policy_score")
    teacher_policy_score = row.get("teacher_policy_score")
    execution_gap = row.get("execution_gap")
    if current_policy_score is not None:
        features["eval::current_policy_score"] = float(current_policy_score)
    if teacher_policy_score is not None:
        features["eval::teacher_policy_score"] = float(teacher_policy_score)
    if execution_gap is not None:
        features["eval::execution_gap"] = float(execution_gap)
    executor_disagreement_tag = str(row.get("executor_disagreement_tag") or "")
    if executor_disagreement_tag:
        features[f"eval::executor_disagreement::{executor_disagreement_tag}"] = 1

    state_context = _macro_state_context(row)
    deck_profile = _deck_semantic_profile(state_context.get("deck_card_counts") or {})
    card_name = payload.get("card_id") or payload.get("card_name")
    card_semantics = _card_semantics_by_java_name().get(str(card_name or ""))
    if card_semantics:
        semantic_prefix = "candidate_card"
        for key, value in sorted(card_semantics.items()):
            features[f"{semantic_prefix}::{key}"] = int(value)
        synergy_pairs = [
            ("strength_bridge", bool(card_semantics["strength_enabler"]), deck_profile["strength_payoffs"]),
            ("strength_bridge", bool(card_semantics["strength_payoff"]), deck_profile["strength_enablers"]),
            ("exhaust_bridge", bool(card_semantics["exhaust_engine"]), deck_profile["exhaust_outlets"] + deck_profile["status_generators"]),
            ("exhaust_bridge", bool(card_semantics["exhaust_outlet"]), deck_profile["exhaust_engines"]),
            ("block_bridge", bool(card_semantics["block_core"]), deck_profile["block_payoffs"]),
            ("block_bridge", bool(card_semantics["block_payoff"]), deck_profile["block_core"]),
            ("status_bridge", bool(card_semantics["status_engine"]), deck_profile["status_generators"]),
            ("status_bridge", bool(card_semantics["status_generator"]), deck_profile["status_engines"] + deck_profile["exhaust_outlets"]),
            ("draw_need", bool(card_semantics["draw_source"]), max(0, 3 - deck_profile["draw_sources"])),
            ("control_need", bool(card_semantics["control_tool"]), max(0, 2 - deck_profile["control_tools"])),
            ("resource_need", bool(card_semantics["resource_conversion"]), max(0, 2 - deck_profile["resource_conversions"])),
            ("healing_need", bool(card_semantics["combat_heal"]), max(0, int(state_context.get("player_missing_hp") or 0))),
            ("vuln_bridge", bool(card_semantics["vuln_payoff"]), deck_profile["control_tools"]),
        ]
        for label, condition, signal in synergy_pairs:
            if condition:
                features[f"{semantic_prefix}::synergy::{label}"] = float(signal)
        features[f"{semantic_prefix}::synergy::shell_alignment"] = float(
            (card_semantics["setup_piece"] * deck_profile["setup_pieces"])
            + (card_semantics["scaling_piece"] * deck_profile["scaling_pieces"])
            + (card_semantics["engine_piece"] * deck_profile["engine_pieces"])
        )
    return features


def macro_option_feature_dict(row: dict[str, Any]) -> dict[str, Any]:
    features = macro_state_feature_dict(row)
    features.update(macro_option_only_feature_dict(row))
    return features


def top_scoring_macro_mistakes(predictions: list[dict[str, Any]], limit: int = 20) -> list[dict[str, Any]]:
    mistakes = [row for row in predictions if not row.get("top1_match")]

    def severity(item: dict[str, Any]) -> tuple[float, int]:
        scores = item.get("scores") or []
        if len(scores) < 2:
            return (0.0, 0)
        top_score = float(scores[0].get("score") or 0.0)
        second_score = float(scores[1].get("score") or 0.0)
        positive_count = len(item.get("positive_option_ids") or [])
        return (abs(top_score - second_score), -positive_count)

    mistakes.sort(key=severity, reverse=True)
    return mistakes[:limit]
