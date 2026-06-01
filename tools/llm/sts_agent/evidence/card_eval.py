"""Card and route semantic evaluators used by planner tools.

These helpers provide heuristic evidence for tool requests and prompt summaries.
They are not teacher labels and should not be treated as final policy values.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[4]


CARD_FACTS_PATH = REPO_ROOT / "tools" / "data" / "card_facts_v1.json"

_CARD_FACTS_BY_NAME_CACHE: dict[str, dict[str, Any]] | None = None


def candidate_tool_descriptor(candidate: dict[str, Any] | None) -> dict[str, Any]:
    if not isinstance(candidate, dict):
        return {}
    action_key = str(candidate.get("action_key") or "")
    card_names = candidate_card_names(candidate)
    room_tags = collect_room_tags(candidate)
    payload = candidate_payload(candidate)
    reward_structure = payload.get("reward_structure") if isinstance(payload, dict) else None
    return {
        "action_id": candidate.get("id"),
        "action_key": action_key,
        "kind": action_key.split("/", 1)[0] if action_key else None,
        "card_names": card_names[:3],
        "room_tags": room_tags[:3],
        "reward_structure": reward_structure,
        "plan_delta": payload.get("plan_delta") if isinstance(payload, dict) else None,
        "dominated": payload.get("dominated") if isinstance(payload, dict) else None,
    }

CARD_KNOWLEDGE: dict[str, dict[str, Any]] = {
    "Strike": {"roles": ["attack", "frontload"], "tags": ["basic"]},
    "Defend": {"roles": ["block"], "tags": ["basic"]},
    "Bash": {"roles": ["attack", "vulnerable", "frontload"], "tags": ["starter"]},
    "Anger": {"roles": ["attack", "frontload"], "tags": ["zero_cost"]},
    "Armaments": {"roles": ["block", "upgrade"], "tags": ["skill"]},
    "Battle Trance": {"roles": ["draw"], "tags": ["skill"]},
    "Bloodletting": {"roles": ["energy"], "tags": ["hp_cost"]},
    "Carnage": {"roles": ["attack", "frontload"], "tags": ["ethereal"]},
    "Clash": {"roles": ["attack"], "tags": ["conditional"]},
    "Cleave": {"roles": ["attack", "aoe", "frontload"], "tags": []},
    "Clothesline": {"roles": ["attack", "weak", "frontload"], "tags": []},
    "Disarm": {"roles": ["mitigation"], "tags": ["boss_value"]},
    "Dropkick": {"roles": ["attack", "draw", "energy"], "tags": ["vulnerable_synergy"]},
    "Dual Wield": {"roles": ["copy"], "tags": ["setup"]},
    "Entrench": {"roles": ["block_scaling"], "tags": ["setup"]},
    "Evolve": {"roles": ["draw"], "tags": ["status_counter"]},
    "Feed": {"roles": ["attack", "max_hp"], "tags": ["meta_scaling"]},
    "Feel No Pain": {"roles": ["block_scaling", "scaling"], "tags": ["exhaust_synergy"]},
    "Fire Breathing": {"roles": ["aoe", "scaling"], "tags": ["status_counter"]},
    "Flame Barrier": {"roles": ["block", "mitigation"], "tags": []},
    "Flex": {"roles": ["damage_amp"], "tags": ["temporary"]},
    "Heavy Blade": {"roles": ["attack", "scaling_payoff"], "tags": ["strength_synergy"]},
    "Hemokinesis": {"roles": ["attack", "frontload"], "tags": ["hp_cost"]},
    "Impervious": {"roles": ["block"], "tags": ["big_block"]},
    "Inflame": {"roles": ["scaling", "damage_amp"], "tags": ["power"]},
    "Intimidate": {"roles": ["weak", "aoe_mitigation"], "tags": ["zero_cost"]},
    "Metallicize": {"roles": ["block_scaling", "scaling"], "tags": ["power"]},
    "Pommel Strike": {"roles": ["attack", "draw", "frontload"], "tags": []},
    "Power Through": {"roles": ["block"], "tags": ["status_cost"]},
    "Pummel": {"roles": ["attack", "frontload"], "tags": ["strength_synergy"]},
    "Rage": {"roles": ["block_scaling"], "tags": ["attack_synergy"]},
    "Rampage": {"roles": ["attack", "scaling"], "tags": ["slow"]},
    "Reaper": {"roles": ["attack", "heal"], "tags": ["strength_synergy"]},
    "Second Wind": {"roles": ["block", "exhaust"], "tags": ["exhaust_synergy"]},
    "Seeing Red": {"roles": ["energy"], "tags": ["exhaust"]},
    "Sentinel": {"roles": ["block", "energy"], "tags": ["exhaust_synergy"]},
    "Sever Soul": {"roles": ["attack", "exhaust"], "tags": []},
    "Shockwave": {"roles": ["weak", "vulnerable", "scaling"], "tags": ["powerful_debuff"]},
    "Shrug It Off": {"roles": ["block", "draw"], "tags": []},
    "Spot Weakness": {"roles": ["scaling", "damage_amp"], "tags": ["enemy_intent_dependent"]},
    "Sword Boomerang": {"roles": ["attack"], "tags": ["strength_synergy", "random_target"]},
    "Thunderclap": {"roles": ["attack", "aoe", "vulnerable"], "tags": []},
    "True Grit": {"roles": ["block", "exhaust"], "tags": ["upgrade_important"]},
    "Twin Strike": {"roles": ["attack", "frontload"], "tags": ["strength_synergy"]},
    "Uppercut": {"roles": ["attack", "weak", "vulnerable", "frontload"], "tags": []},
    "Warcry": {"roles": ["draw", "deck_control"], "tags": ["zero_cost"]},
    "Whirlwind": {"roles": ["attack", "aoe", "energy_sink"], "tags": ["strength_synergy"]},
    "Wound": {"roles": [], "tags": ["status", "bad_draw"]},
    "Burn": {"roles": [], "tags": ["status", "damage_status"]},
    "Ascenders Bane": {"roles": [], "tags": ["curse", "bad_draw"]},
}

ROOM_TAGS = {
    "MonsterRoom": "monster",
    "Monster": "monster",
    "EliteRoom": "elite",
    "Elite": "elite",
    "RestRoom": "campfire",
    "Campfire": "campfire",
    "ShopRoom": "shop",
    "Shop": "shop",
    "TreasureRoom": "treasure",
    "Treasure": "treasure",
    "EventRoom": "event",
    "Event": "event",
    "BossRoom": "boss",
    "Boss": "boss",
}

def normalized_key(text: Any) -> str:
    return "".join(ch.lower() for ch in str(text or "") if ch.isalnum())

def load_card_facts_by_name() -> dict[str, dict[str, Any]]:
    global _CARD_FACTS_BY_NAME_CACHE
    if _CARD_FACTS_BY_NAME_CACHE is not None:
        return _CARD_FACTS_BY_NAME_CACHE
    facts: dict[str, dict[str, Any]] = {}
    try:
        raw = json.loads(CARD_FACTS_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        _CARD_FACTS_BY_NAME_CACHE = facts
        return facts
    cards = raw.get("cards")
    if not isinstance(cards, list):
        _CARD_FACTS_BY_NAME_CACHE = facts
        return facts
    for card in cards:
        if not isinstance(card, dict):
            continue
        name = str(card.get("name") or card.get("card_id") or "").strip()
        if not name:
            continue
        info = {
            "roles": list(card.get("derived_roles") or []),
            "tags": list(card.get("derived_tags") or []),
            "card_id": card.get("card_id"),
            "java_id": card.get("java_id"),
            "cost": card.get("cost"),
            "rarity": card.get("rarity"),
            "card_type": card.get("card_type"),
            "source": "card_facts_v1",
        }
        for alias in [name, card.get("card_id"), card.get("java_id")]:
            if isinstance(alias, str) and alias:
                facts[alias] = info
    _CARD_FACTS_BY_NAME_CACHE = facts
    return facts

def card_fact_for_name(card_name: str | None) -> dict[str, Any] | None:
    if not card_name:
        return None
    normalized = normalized_key(card_name)
    for alias, info in load_card_facts_by_name().items():
        if normalized_key(alias) == normalized:
            return info
    return None

def curated_card_info(card_name: str | None) -> dict[str, Any] | None:
    if not card_name:
        return None
    normalized = normalized_key(card_name)
    for known, info in CARD_KNOWLEDGE.items():
        if normalized_key(known) == normalized:
            return info
    return None

def merged_card_info(card_name: str | None) -> dict[str, Any]:
    fact = card_fact_for_name(card_name)
    curated = curated_card_info(card_name)
    roles = set()
    tags = set()
    sources: list[str] = []
    metadata: dict[str, Any] = {}
    if fact:
        roles.update(fact.get("roles") or [])
        tags.update(fact.get("tags") or [])
        metadata.update(fact)
        sources.append("card_facts_v1")
    if curated:
        roles.update(curated.get("roles") or [])
        tags.update(curated.get("tags") or [])
        sources.append("python_curated_overlay")
    return {
        **metadata,
        "roles": sorted(roles),
        "tags": sorted(tags),
        "sources": sources,
        "known": bool(fact or curated),
    }

def known_card_names() -> list[str]:
    names = set(CARD_KNOWLEDGE)
    for alias, info in load_card_facts_by_name().items():
        if info.get("source") == "card_facts_v1":
            names.add(str(alias))
    return sorted(names)

def candidate_payload(candidate: dict[str, Any] | None) -> dict[str, Any]:
    if not isinstance(candidate, dict):
        return {}
    payload = candidate.get("payload")
    return payload if isinstance(payload, dict) else {}

def card_name_from_id(card_id: Any) -> str | None:
    if not isinstance(card_id, str) or not card_id:
        return None
    normalized = normalized_key(card_id)
    for known in known_card_names():
        if normalized_key(known) == normalized:
            fact = card_fact_for_name(known)
            if fact and isinstance(fact.get("card_id"), str) and normalized_key(fact.get("card_id")) == normalized:
                return str(known)
            if known in CARD_KNOWLEDGE:
                return known
            return str(known)
    spaced = re.sub(r"(?<!^)([A-Z])", r" \1", card_id).strip()
    return spaced or card_id

def candidate_card_names(candidate: dict[str, Any] | None) -> list[str]:
    payload = candidate_payload(candidate)
    names: list[str] = []
    card = payload.get("card")
    if isinstance(card, dict):
        name = card_name_from_id(card.get("card_id"))
        if name:
            names.append(name)
    for name in collect_card_names(candidate, unique=True):
        if name not in names:
            names.append(name)
    return names

def card_feature_roles(card: dict[str, Any] | None) -> tuple[list[str], list[str]]:
    if not isinstance(card, dict):
        return [], []
    roles: list[str] = []
    tags: list[str] = []
    if int(card.get("base_damage") or 0) > 0:
        roles.extend(["attack", "frontload"])
    if int(card.get("base_block") or 0) > 0:
        roles.append("block")
    if bool(card.get("draws_cards")):
        roles.append("draw")
    if bool(card.get("gains_energy")):
        roles.append("energy")
    if bool(card.get("applies_weak")):
        roles.append("weak")
    if bool(card.get("applies_vulnerable")):
        roles.append("vulnerable")
    if bool(card.get("scaling_piece")):
        roles.append("scaling")
    if bool(card.get("aoe")) or bool(card.get("multi_damage")):
        roles.append("aoe")
    if bool(card.get("exhaust")):
        roles.append("exhaust")
    if bool(card.get("starter_basic")):
        tags.append("basic")
    if bool(card.get("ethereal")):
        tags.append("ethereal")
    if bool(card.get("innate")):
        tags.append("innate")
    return sorted(set(roles)), sorted(set(tags))

def card_fit_for_candidate(candidate: dict[str, Any] | None, needs: list[str]) -> dict[str, Any]:
    payload = candidate_payload(candidate)
    card = payload.get("card")
    card_name = None
    if isinstance(card, dict):
        card_name = card_name_from_id(card.get("card_id"))
    if card_name is None:
        names = candidate_card_names(candidate)
        card_name = names[0] if names else None
    fit = card_fit(card_name, needs)
    feature_roles, feature_tags = card_feature_roles(card if isinstance(card, dict) else None)
    if feature_roles or feature_tags:
        roles = sorted(set((fit.get("roles") or []) + feature_roles))
        tags = sorted(set((fit.get("tags") or []) + feature_tags))
        feature_fit = card_fit(card_name, needs)
        feature_fit["roles"] = roles
        feature_fit["tags"] = tags
        feature_score = feature_fit.get("score") or 0
        if "needs_block_density" in needs and "block" in roles:
            feature_score = max(feature_score, 3)
        if "needs_draw" in needs and "draw" in roles:
            feature_score = max(feature_score, 3)
        if "needs_scaling" in needs and "scaling" in roles:
            feature_score = max(feature_score, 3)
        if "needs_aoe" in needs and "aoe" in roles:
            feature_score = max(feature_score, 2)
        if "needs_frontload" in needs and "attack" in roles:
            feature_score = max(feature_score, 2)
        feature_fit["score"] = feature_score
        feature_fit["fit"] = "high" if feature_score >= 5 else ("medium" if feature_score >= 2 else "low")
        return feature_fit
    return fit

def collect_strings(value: Any, *, limit: int = 160, depth: int = 0) -> list[str]:
    if limit <= 0 or depth > 5:
        return []
    if isinstance(value, str):
        return [value]
    if isinstance(value, (int, float, bool)) or value is None:
        return []
    strings: list[str] = []
    if isinstance(value, dict):
        for item in value.values():
            if len(strings) >= limit:
                break
            strings.extend(collect_strings(item, limit=limit - len(strings), depth=depth + 1))
    elif isinstance(value, list):
        for item in value[:80]:
            if len(strings) >= limit:
                break
            strings.extend(collect_strings(item, limit=limit - len(strings), depth=depth + 1))
    return strings[:limit]

def collect_card_names(value: Any, *, unique: bool) -> list[str]:
    names: list[str] = []
    known = [(normalized_key(name), name) for name in known_card_names()]
    for text in collect_strings(value):
        normalized = normalized_key(text)
        if not normalized:
            continue
        for key, name in known:
            if key and key in normalized:
                names.append(name)
    if unique:
        deduped: list[str] = []
        for name in names:
            if name not in deduped:
                deduped.append(name)
        return deduped
    return names

def collect_room_tags(value: Any) -> list[str]:
    tags: list[str] = []
    for text in collect_strings(value, limit=80):
        normalized = normalized_key(text)
        for key, tag in ROOM_TAGS.items():
            if normalized_key(key) in normalized and tag not in tags:
                tags.append(tag)
    return tags

def card_fit(card_name: str | None, needs: list[str]) -> dict[str, Any]:
    if not card_name:
        return {
            "card": None,
            "fit": "unknown",
            "score": 0,
            "roles": [],
            "tags": [],
            "reasons": ["card name not visible to semantic evaluator"],
            "risks": [],
        }
    info = merged_card_info(card_name)
    if not info.get("known"):
        info = {"roles": [], "tags": ["unknown_card"], "known": False}
    roles = list(info.get("roles") or [])
    tags = list(info.get("tags") or [])
    score = 0
    reasons: list[str] = []
    risks: list[str] = []
    need_set = set(needs)
    role_set = set(roles)
    tag_set = set(tags)
    if "needs_block_density" in need_set and (role_set & {"block", "block_scaling", "mitigation"}):
        score += 3
        reasons.append("matches block/mitigation need")
    if "needs_draw" in need_set and "draw" in role_set:
        score += 3
        reasons.append("matches draw need")
    if "needs_scaling" in need_set and (role_set & {"scaling", "block_scaling", "damage_amp"}):
        score += 3
        reasons.append("matches scaling need")
    if "needs_aoe" in need_set and (role_set & {"aoe", "aoe_mitigation"}):
        score += 2
        reasons.append("matches aoe need")
    if "needs_frontload" in need_set and (role_set & {"attack", "frontload"}):
        score += 2
        reasons.append("matches frontload damage need")
    if "boss_survival_risk" in need_set and (role_set & {"weak", "mitigation", "block", "scaling"}):
        score += 2
        reasons.append("helps boss survival profile")
    if tag_set & {"hp_cost", "status_cost", "slow", "conditional", "random_target"}:
        risks.extend(sorted(tag_set & {"hp_cost", "status_cost", "slow", "conditional", "random_target"}))
    if "basic" in tag_set:
        score -= 1
        risks.append("basic card low marginal value")
    fit = "high" if score >= 5 else ("medium" if score >= 2 else ("low" if info.get("known") else "unknown"))
    return {
        "card": card_name,
        "fit": fit,
        "score": score,
        "roles": roles,
        "tags": tags,
        "reasons": reasons,
        "risks": risks,
    }

def build_deck_need_eval(public_payload: dict[str, Any]) -> dict[str, Any]:
    card_names = []
    role_counts: dict[str, int] = {}
    tag_counts: dict[str, int] = {}
    deck_cards = public_payload.get("deck_cards")
    if isinstance(deck_cards, list) and deck_cards:
        for deck_card in deck_cards:
            if not isinstance(deck_card, dict):
                continue
            name = card_name_from_id(deck_card.get("card_id"))
            if name:
                copies = int(deck_card.get("count") or deck_card.get("copies") or 1)
                card_names.extend([name] * max(1, copies))
            roles, tags = card_feature_roles(deck_card)
            copies = int(deck_card.get("count") or deck_card.get("copies") or 1)
            for role in roles:
                role_counts[role] = role_counts.get(role, 0) + max(1, copies)
            for tag in tags:
                tag_counts[tag] = tag_counts.get(tag, 0) + max(1, copies)
    if not card_names:
        card_names = collect_card_names(public_payload, unique=False)
    deck_size = len(card_names)
    for name in card_names:
        info = merged_card_info(name)
        for role in info.get("roles") or []:
            role_counts[role] = role_counts.get(role, 0) + 1
        for tag in info.get("tags") or []:
            tag_counts[tag] = tag_counts.get(tag, 0) + 1
    needs: list[str] = []
    floor = public_payload.get("floor")
    current_hp = public_payload.get("current_hp")
    max_hp = public_payload.get("max_hp")
    block_like = sum(role_counts.get(role, 0) for role in ["block", "block_scaling", "mitigation", "weak"])
    attacks = sum(role_counts.get(role, 0) for role in ["attack", "frontload"])
    if deck_size and block_like * 4 < deck_size:
        needs.append("needs_block_density")
    if role_counts.get("draw", 0) < 2:
        needs.append("needs_draw")
    if isinstance(floor, int) and floor >= 6 and role_counts.get("scaling", 0) + role_counts.get("damage_amp", 0) < 2:
        needs.append("needs_scaling")
    if isinstance(floor, int) and floor >= 5 and role_counts.get("aoe", 0) < 1:
        needs.append("needs_aoe")
    if attacks < max(4, deck_size // 4):
        needs.append("needs_frontload")
    if (
        isinstance(current_hp, int)
        and isinstance(max_hp, int)
        and max_hp > 0
        and current_hp * 100 <= max_hp * 55
    ):
        needs.append("boss_survival_risk")
    if not needs:
        needs.append("no_clear_static_gap")
    return {
        "tool": "deck_need_eval",
        "status": "ok",
        "information_boundary": "public_observation_only",
        "deck_size_estimate": deck_size,
        "observed_cards": sorted(set(card_names))[:30],
        "role_counts": role_counts,
        "tag_counts": tag_counts,
        "needs": needs,
        "knowledge_source": "card_facts_v1" if load_card_facts_by_name() else "python_curated_overlay_only",
        "caveat": "keyword heuristic over public observation; not a strategy label",
    }

def build_reward_card_eval(public_payload: dict[str, Any], candidates: list[dict[str, Any]]) -> dict[str, Any]:
    deck_eval = build_deck_need_eval(public_payload)
    needs = deck_eval.get("needs") or []
    evaluated = []
    for candidate in candidates:
        action_key = str(candidate.get("action_key") or "")
        if not (
            "reward" in action_key
            or "card" in action_key
            or "select" in action_key
            or action_key == "proceed"
        ):
            continue
        evaluated.append(
            {
                "candidate": candidate_tool_descriptor(candidate),
                "card_fit": card_fit_for_candidate(candidate, needs),
            }
        )
    return {
        "tool": "reward_card_eval",
        "status": "ok",
        "information_boundary": "public_observation_only",
        "deck_needs": needs,
        "candidates": evaluated[:8],
        "caveat": "heuristic card-role lookup; not a teacher label",
    }

def build_map_route_eval(public_payload: dict[str, Any], candidates: list[dict[str, Any]]) -> dict[str, Any]:
    current_hp = public_payload.get("current_hp")
    max_hp = public_payload.get("max_hp")
    nodes_by_x: dict[int, dict[str, Any]] = {}
    for node in public_payload.get("next_nodes") or []:
        if not isinstance(node, dict):
            continue
        try:
            nodes_by_x[int(node.get("x"))] = node
        except (TypeError, ValueError):
            continue
    routes = []
    for candidate in candidates:
        action_key = str(candidate.get("action_key") or "")
        if "map/select" not in action_key:
            continue
        tags = collect_room_tags(candidate)
        matched_node = None
        match = re.search(r"map/select_x/(-?\d+)", action_key)
        if match:
            matched_node = nodes_by_x.get(int(match.group(1)))
        if isinstance(matched_node, dict):
            room_type = matched_node.get("room_type")
            if isinstance(room_type, str):
                for key, tag in ROOM_TAGS.items():
                    if normalized_key(key) in normalized_key(room_type) and tag not in tags:
                        tags.append(tag)
        risk_score = 0
        reasons: list[str] = []
        if "boss" in tags:
            risk_score += 5
            reasons.append("boss node")
        if "elite" in tags:
            risk_score += 4
            reasons.append("elite node")
        if "monster" in tags:
            risk_score += 2
            reasons.append("combat node")
        if "event" in tags:
            risk_score += 1
            reasons.append("unknown event variance")
        if "campfire" in tags:
            risk_score -= 2
            reasons.append("campfire recovery/upgrade option")
        if "shop" in tags:
            risk_score -= 1
            reasons.append("shop agency")
        if (
            isinstance(current_hp, int)
            and isinstance(max_hp, int)
            and max_hp > 0
            and current_hp * 100 <= max_hp * 45
            and {"elite", "monster", "boss"} & set(tags)
        ):
            risk_score += 2
            reasons.append("low hp increases combat route risk")
        routes.append(
            {
                "candidate": candidate_tool_descriptor(candidate),
                "node": matched_node,
                "room_tags": tags,
                "risk_score": risk_score,
                "risk_band": "high" if risk_score >= 5 else ("medium" if risk_score >= 2 else "low"),
                "reasons": reasons,
            }
        )
    return {
        "tool": "map_route_eval",
        "status": "ok",
        "information_boundary": "public_observation_only",
        "routes": routes,
        "caveat": "route tags are parsed from public candidate metadata when available",
    }

def build_shop_purchase_eval(public_payload: dict[str, Any], candidates: list[dict[str, Any]]) -> dict[str, Any]:
    deck_eval = build_deck_need_eval(public_payload)
    needs = deck_eval.get("needs") or []
    purchases = []
    for candidate in candidates:
        action_key = str(candidate.get("action_key") or "")
        if not any(token in action_key for token in ["shop", "buy", "purchase", "purge", "proceed"]):
            continue
        purchases.append(
            {
                "candidate": candidate_tool_descriptor(candidate),
                "card_fit": card_fit_for_candidate(candidate, needs),
                "notes": ["purge may improve deck quality"] if "purge" in action_key else [],
            }
        )
    return {
        "tool": "shop_purchase_eval",
        "status": "ok",
        "information_boundary": "public_observation_only",
        "deck_needs": needs,
        "purchases": purchases[:10],
        "caveat": "does not yet model long-horizon shop opportunity cost",
    }

def build_campfire_eval(
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    raw: dict[str, Any],
) -> dict[str, Any]:
    deck_eval = build_deck_need_eval(public_payload)
    needs = deck_eval.get("needs") or []
    current_hp = raw.get("current_hp")
    max_hp = raw.get("max_hp")
    hp_ratio = None
    if isinstance(current_hp, int) and isinstance(max_hp, int) and max_hp > 0:
        hp_ratio = current_hp / max_hp
    smith_targets = []
    candidates_by_id = {}
    for candidate in candidates:
        try:
            candidates_by_id[int(candidate.get("id"))] = candidate
        except (TypeError, ValueError):
            continue
    for raw_candidate in raw.get("smith_candidates") or []:
        candidate = raw_candidate
        try:
            candidate = candidates_by_id.get(int(raw_candidate.get("action_id")), raw_candidate)
        except (AttributeError, TypeError, ValueError):
            candidate = raw_candidate
        smith_targets.append(
            {
                "candidate": candidate_tool_descriptor(candidate),
                "card_fit": card_fit_for_candidate(candidate, needs),
            }
        )
    smith_targets.sort(key=lambda item: item.get("card_fit", {}).get("score", 0), reverse=True)
    rest_priority = "high" if hp_ratio is not None and hp_ratio <= 0.55 else ("medium" if hp_ratio is not None and hp_ratio <= 0.75 else "low")
    recommendation = "rest" if rest_priority == "high" and raw.get("rest_legal") else "smith_if_high_value_target"
    return {
        "tool": "campfire_eval",
        "status": "ok",
        "information_boundary": "public_observation_only",
        "current_hp": current_hp,
        "max_hp": max_hp,
        "hp_ratio": round(hp_ratio, 3) if hp_ratio is not None else None,
        "rest_legal": raw.get("rest_legal"),
        "rest_priority": rest_priority,
        "deck_needs": needs,
        "best_smith_targets": smith_targets[:5],
        "recommendation": recommendation,
        "caveat": "heuristic rest/smith triage; not a long-horizon guarantee",
    }
