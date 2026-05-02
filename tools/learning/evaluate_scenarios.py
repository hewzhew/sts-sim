#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import random
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import numpy as np
from sb3_contrib import MaskablePPO

from card_semantics import card_semantics, normalize_card_name
from combat_rl_common import REPO_ROOT, write_json
from full_run_candidate_policy import FullRunCandidateScorerPolicy  # noqa: F401
from full_run_env import MAX_ACTIONS, FullRunGymEnv


DEFAULT_SUITE = REPO_ROOT / "tools" / "learning" / "scenarios" / "scenario_bench_v0.json"
DEFAULT_ARTIFACT_DIR = REPO_ROOT / "tools" / "artifacts" / "scenario_bench"

CARD_TYPE_IDS = {
    "attack": 1,
    "skill": 2,
    "power": 3,
    "status": 4,
    "curse": 5,
}

RARITY_IDS = {
    "basic": 1,
    "common": 2,
    "uncommon": 3,
    "rare": 4,
    "special": 5,
    "curse": 6,
}

RUST_CARD_IDS = {
    "Anger": "Anger",
    "Armaments": "Armaments",
    "Barricade": "Barricade",
    "Bash": "Bash",
    "Battle Trance": "BattleTrance",
    "Clash": "Clash",
    "Cleave": "Cleave",
    "Corruption": "Corruption",
    "Dark Embrace": "DarkEmbrace",
    "Defend": "Defend",
    "Demon Form": "DemonForm",
    "Disarm": "Disarm",
    "Entrench": "Entrench",
    "Feel No Pain": "FeelNoPain",
    "Fire Breathing": "FireBreathing",
    "Flame Barrier": "FlameBarrier",
    "Flex": "Flex",
    "Ghostly Armor": "GhostlyArmor",
    "Heavy Blade": "HeavyBlade",
    "Hemokinesis": "Hemokinesis",
    "Immolate": "Immolate",
    "Impervious": "Impervious",
    "Inflame": "Inflame",
    "Metallicize": "Metallicize",
    "Pommel Strike": "PommelStrike",
    "Power Through": "PowerThrough",
    "Perfected Strike": "PerfectedStrike",
    "Second Wind": "SecondWind",
    "Shockwave": "Shockwave",
    "Shrug It Off": "ShrugItOff",
    "Spot Weakness": "SpotWeakness",
    "Strike": "Strike",
    "Sword Boomerang": "SwordBoomerang",
    "Thunderclap": "ThunderClap",
    "True Grit": "TrueGrit",
    "Twin Strike": "TwinStrike",
    "Uppercut": "Uppercut",
    "Warcry": "Warcry",
    "Wild Strike": "WildStrike",
}

RARE_CARDS = {
    "Barricade",
    "Corruption",
    "Demon Form",
    "Immolate",
    "Impervious",
    "Offering",
}

UNCOMMON_CARDS = {
    "Battle Trance",
    "Dark Embrace",
    "Disarm",
    "Entrench",
    "Feel No Pain",
    "Fire Breathing",
    "Flame Barrier",
    "Ghostly Armor",
    "Inflame",
    "Metallicize",
    "Power Through",
    "Second Wind",
    "Shockwave",
    "Spot Weakness",
    "Uppercut",
}

RELIC_SCORES = {
    "Black Star": 70,
    "Busted Crown": -25,
    "Coffee Dripper": -25,
    "Ectoplasm": 5,
    "Mark of Pain": 10,
    "Runic Dome": -25,
    "Runic Pyramid": 75,
    "Snecko Eye": 70,
    "Sozu": -10,
    "Tiny House": 20,
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Evaluate policies on hand-authored full-run decision scenarios."
    )
    parser.add_argument("--suite", type=Path, default=DEFAULT_SUITE)
    parser.add_argument("--policies", default="rule_baseline_v0,model")
    parser.add_argument("--model", type=Path)
    parser.add_argument("--model-name", default="ppo_model")
    parser.add_argument("--seed", type=int, default=70000)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--human-review-out", type=Path)
    return parser.parse_args()


def parse_policy_list(text: str) -> list[str]:
    policies = [part.strip() for part in str(text or "").split(",") if part.strip()]
    if not policies:
        raise SystemExit("expected at least one policy")
    allowed = {"random_masked", "rule_baseline_v0", "model"}
    unknown = [policy for policy in policies if policy not in allowed]
    if unknown:
        raise SystemExit(f"unknown policy names: {unknown}; allowed={sorted(allowed)}")
    return policies


def load_suite(path: Path) -> dict[str, Any]:
    if not path.exists():
        raise SystemExit(f"missing scenario suite: {path}")
    with path.open("r", encoding="utf-8") as handle:
        data = json.load(handle)
    scenarios = data.get("scenarios") or []
    if not isinstance(scenarios, list) or not scenarios:
        raise SystemExit(f"scenario suite has no scenarios: {path}")
    return data


def main() -> int:
    args = parse_args()
    policies = parse_policy_list(args.policies)
    suite = load_suite(args.suite)
    out_path = args.out or (DEFAULT_ARTIFACT_DIR / "scenario_report_v0.json")
    review_path = args.human_review_out or (DEFAULT_ARTIFACT_DIR / "human_review_v0.json")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    review_path.parent.mkdir(parents=True, exist_ok=True)

    model = None
    if "model" in policies:
        if args.model is None:
            raise SystemExit("--model is required when policies includes 'model'")
        model = MaskablePPO.load(str(args.model))

    rng = random.Random(args.seed)
    scenario_reports = []
    for scenario in suite.get("scenarios") or []:
        scenario_reports.append(evaluate_scenario(scenario, policies, model, args.model_name, rng))

    report = {
        "report_version": "scenario_bench_report_v0",
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "purpose": "offline scenario capability checks; not runtime policy rules and not rollout proof",
        "config": {
            "suite": str(args.suite),
            "suite_id": suite.get("suite_id"),
            "suite_version": suite.get("version"),
            "seed": args.seed,
            "policies": policies,
            "model": str(args.model) if args.model else None,
            "model_name": args.model_name if "model" in policies else None,
        },
        "summary": summarize_scenarios(scenario_reports),
        "scenarios": scenario_reports,
    }
    write_json(out_path, report)
    write_json(review_path, build_human_review(report))
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0


def evaluate_scenario(
    scenario: dict[str, Any],
    policies: list[str],
    model: MaskablePPO | None,
    model_name: str,
    rng: random.Random,
) -> dict[str, Any]:
    candidates = materialize_candidates(scenario)
    policy_reports = []
    for policy in policies:
        if policy == "random_masked":
            scores = score_random(candidates, rng)
            policy_name = policy
            source = "stable_random_scenario_order"
        elif policy == "rule_baseline_v0":
            scores = score_rule_baseline(scenario, candidates)
            policy_name = policy
            source = "python_scenario_approximation_of_rule_baseline_v0"
        elif policy == "model":
            assert model is not None
            scores = score_model(scenario, candidates, model)
            policy_name = model_name
            source = "synthetic_full_run_observation_model_logits"
        else:
            raise AssertionError(policy)
        ranking = build_ranking(candidates, scores)
        policy_reports.append(
            {
                "policy": policy_name,
                "policy_kind": policy,
                "source": source,
                "top_choice": ranking[0] if ranking else None,
                "ranking": ranking,
                "label_eval": evaluate_label(scenario.get("human_label") or {}, ranking),
            }
        )
    return {
        "id": scenario.get("id"),
        "category": scenario.get("category"),
        "decision_type": scenario.get("decision_type"),
        "context_note": scenario.get("context_note"),
        "observation": scenario.get("observation") or {},
        "deck": scenario.get("deck") or {},
        "relics": scenario.get("relics") or [],
        "potions": scenario.get("potions") or [],
        "feature_limitations": feature_limitations(scenario),
        "human_label": scenario.get("human_label") or {},
        "candidates": review_candidates(candidates),
        "policies": policy_reports,
    }


def materialize_candidates(scenario: dict[str, Any]) -> list[dict[str, Any]]:
    decision_type = str(scenario.get("decision_type") or "")
    out = []
    for index, raw in enumerate(scenario.get("candidates") or []):
        candidate = dict(raw)
        candidate_id = str(candidate.get("id") or f"candidate_{index}")
        action_type, action, action_key = action_for_candidate(decision_type, index, candidate)
        card = build_card_feature(candidate.get("card")) if candidate.get("card") else None
        action_id = stable_action_id(action_key)
        out.append(
            {
                "id": candidate_id,
                "label": str(candidate.get("label") or candidate_id),
                "action_index": index,
                "action_id": action_id,
                "action_key": action_key,
                "action": action,
                "card": card,
                "scenario_meta": {
                    key: value
                    for key, value in candidate.items()
                    if key not in {"id", "label", "action", "card"}
                },
                "scenario_rule_score": float(candidate.get("rule_score", candidate.get("score", 0.0))),
                "action_type": action_type,
            }
        )
    return out


def action_for_candidate(
    decision_type: str,
    index: int,
    candidate: dict[str, Any],
) -> tuple[str, dict[str, Any], str]:
    kind = str(candidate.get("kind") or "")
    if decision_type == "reward_card_choice":
        if kind == "skip" or candidate.get("id") == "skip":
            return "proceed", {"type": "proceed"}, "proceed"
        return "select_card", {"type": "select_card", "index": index}, f"reward/select_card/{index}"
    if decision_type == "campfire":
        if kind == "rest":
            return "campfire_option", {"type": "campfire_option", "choice": "rest"}, "campfire/rest"
        if kind == "toke":
            deck_index = int(candidate.get("deck_index", index))
            return (
                "campfire_option",
                {"type": "campfire_option", "choice": "toke", "index": deck_index},
                f"campfire/toke/{deck_index}",
            )
        if kind == "recall":
            return "campfire_option", {"type": "campfire_option", "choice": "recall"}, "campfire/recall"
        deck_index = int(candidate.get("deck_index", index))
        return (
            "campfire_option",
            {"type": "campfire_option", "choice": "smith", "index": deck_index},
            f"campfire/smith/{deck_index}",
        )
    if decision_type == "boss_relic":
        if kind == "skip":
            return "proceed", {"type": "proceed"}, "proceed"
        return "submit_relic_choice", {"type": "submit_relic_choice", "index": index}, f"boss_relic/select/{index}"
    if decision_type == "map":
        x = int(candidate.get("x", index))
        return "select_map_node", {"type": "select_map_node", "x": x}, f"map/select_x/{x}"
    return str(candidate.get("action_type") or "unknown"), dict(candidate.get("action") or {}), str(
        candidate.get("action_key") or f"scenario/{index}"
    )


def build_card_feature(raw: dict[str, Any]) -> dict[str, Any]:
    name = normalize_card_name(str(raw.get("name") or raw.get("card_id") or ""))
    semantics = card_semantics(name)
    rust_id = str(raw.get("rust_id") or RUST_CARD_IDS.get(name) or name.replace(" ", ""))
    rarity = str(raw.get("rarity") or default_rarity(name)).lower()
    upgrades = int(raw.get("upgrades") or 0)
    base_damage = int(raw.get("base_damage", semantics.get("base_damage") or 0))
    base_block = int(raw.get("base_block", semantics.get("base_block") or 0))
    base_magic = int(
        raw.get(
            "base_magic",
            max(
                float(semantics.get("draw_count") or 0),
                abs(float(semantics.get("apply_strength") or 0)),
                float(semantics.get("apply_weak") or 0),
                float(semantics.get("apply_vulnerable") or 0),
            ),
        )
    )
    card_type = str(raw.get("card_type") or semantics.get("card_type") or "unknown")
    rule_score = float(raw.get("rule_score", 0.0))
    return {
        "name": name,
        "card_id": rust_id,
        "card_id_hash": stable_action_id(f"card:{rust_id}"),
        "card_type_id": CARD_TYPE_IDS.get(card_type, 0),
        "rarity_id": RARITY_IDS.get(rarity, 0),
        "cost": int(raw.get("cost", semantics.get("base_cost") or 0)),
        "upgrades": upgrades,
        "base_damage": base_damage,
        "base_block": base_block,
        "base_magic": base_magic,
        "upgraded_damage": int(raw.get("upgraded_damage", base_damage + upgrades * 3)),
        "upgraded_block": int(raw.get("upgraded_block", base_block + upgrades * 3)),
        "upgraded_magic": int(raw.get("upgraded_magic", base_magic + upgrades)),
        "exhaust": bool(raw.get("exhaust", semantics.get("exhausts") or False)),
        "ethereal": bool(raw.get("ethereal", semantics.get("ethereal") or False)),
        "innate": bool(raw.get("innate", False)),
        "aoe": bool(raw.get("aoe", name in {"Cleave", "Immolate", "Reaper"})),
        "multi_damage": bool(raw.get("multi_damage", semantics.get("multi_hit") or False)),
        "starter_basic": bool(raw.get("starter_basic", name in {"Strike", "Defend", "Bash"})),
        "draws_cards": bool(raw.get("draws_cards", float(semantics.get("draw_count") or 0) > 0)),
        "gains_energy": bool(raw.get("gains_energy", name in {"Offering", "Bloodletting"})),
        "applies_weak": bool(raw.get("applies_weak", float(semantics.get("apply_weak") or 0) > 0)),
        "applies_vulnerable": bool(
            raw.get("applies_vulnerable", float(semantics.get("apply_vulnerable") or 0) > 0)
        ),
        "scaling_piece": bool(
            raw.get(
                "scaling_piece",
                name
                in {
                    "Barricade",
                    "Corruption",
                    "Dark Embrace",
                    "Demon Form",
                    "Entrench",
                    "Feel No Pain",
                    "Inflame",
                    "Limit Break",
                    "Metallicize",
                    "Spot Weakness",
                },
            )
        ),
        "deck_copies": int(raw.get("deck_copies") or 0),
        "rule_score": rule_score,
    }


def default_rarity(name: str) -> str:
    if name in RARE_CARDS:
        return "rare"
    if name in UNCOMMON_CARDS:
        return "uncommon"
    if name in {"Strike", "Defend", "Bash"}:
        return "basic"
    return "common"


def score_random(candidates: list[dict[str, Any]], rng: random.Random) -> dict[str, float]:
    values = list(range(len(candidates)))
    rng.shuffle(values)
    return {candidate["id"]: float(value) for candidate, value in zip(candidates, values)}


def score_rule_baseline(scenario: dict[str, Any], candidates: list[dict[str, Any]]) -> dict[str, float]:
    decision_type = str(scenario.get("decision_type") or "")
    observation = scenario.get("observation") or {}
    hp = float(observation.get("current_hp") or 0)
    max_hp = float(observation.get("max_hp") or 1)
    hp_ratio = hp / max(max_hp, 1.0)
    scores = {}
    for candidate in candidates:
        meta = candidate.get("scenario_meta") or {}
        card = candidate.get("card") or {}
        action_type = str(candidate.get("action_type") or "")
        score = float(candidate.get("scenario_rule_score") or 0.0)
        if decision_type == "reward_card_choice":
            score = 5.0 if action_type == "proceed" else float(card.get("rule_score") or score)
        elif decision_type == "campfire":
            choice = str((candidate.get("action") or {}).get("choice") or "")
            if choice == "rest":
                score = 160.0 if hp_ratio < 0.45 else 90.0 if hp_ratio < 0.70 else 10.0
            elif choice == "smith":
                score = float(meta.get("upgrade_score", score or 35.0))
            elif choice == "toke":
                score = 60.0 + float(meta.get("remove_score", score or 0.0))
            elif choice == "recall":
                score = -20.0
        elif decision_type == "boss_relic":
            relic = str(meta.get("relic") or candidate.get("label") or "")
            score = 80.0 + float(meta.get("relic_score", RELIC_SCORES.get(relic, 20)))
        elif decision_type == "map":
            room = str(meta.get("room") or "")
            if room == "elite":
                score = 70.0 if hp_ratio >= 0.70 else -20.0
            elif room == "rest":
                score = 90.0 if hp_ratio < 0.70 else 45.0
            elif room == "treasure":
                score = 80.0
            elif room == "shop":
                score = 75.0 if float(observation.get("gold") or 0) >= 150 else 25.0
            elif room == "event":
                score = 55.0
            elif room == "monster":
                score = 50.0
            score += float(meta.get("path_bonus", 0.0))
        scores[candidate["id"]] = score
    return scores


def score_model(
    scenario: dict[str, Any],
    candidates: list[dict[str, Any]],
    model: MaskablePPO,
) -> dict[str, float]:
    response = synthetic_response(scenario, candidates)
    encoder = object.__new__(FullRunGymEnv)
    obs = FullRunGymEnv._encode_observation(encoder, response)
    mask = np.zeros(MAX_ACTIONS, dtype=bool)
    mask[: len(candidates)] = True
    obs_tensor, _ = model.policy.obs_to_tensor(obs)
    dist = model.policy.get_distribution(obs_tensor, action_masks=mask.reshape(1, -1))
    logits = dist.distribution.logits.detach().cpu().numpy()[0]
    probs = dist.distribution.probs.detach().cpu().numpy()[0]
    return {
        candidate["id"]: float(logits[index]) + float(probs[index]) * 1e-3
        for index, candidate in enumerate(candidates)
    }


def synthetic_response(scenario: dict[str, Any], candidates: list[dict[str, Any]]) -> dict[str, Any]:
    observation = dict(scenario.get("observation") or {})
    decision_type = str(scenario.get("decision_type") or "none")
    observation.setdefault("decision_type", decision_type)
    observation.setdefault("engine_state", decision_type)
    observation.setdefault("act", 1)
    observation.setdefault("floor", 1)
    observation.setdefault("current_hp", 80)
    observation.setdefault("max_hp", 80)
    observation.setdefault(
        "hp_ratio_milli",
        int(1000 * float(observation.get("current_hp") or 0) / max(float(observation.get("max_hp") or 1), 1.0)),
    )
    observation.setdefault("gold", 0)
    observation.setdefault("relic_count", len(scenario.get("relics") or []))
    observation.setdefault("potion_slots", 3)
    observation.setdefault("filled_potion_slots", len(scenario.get("potions") or []))
    observation.setdefault("deck", scenario.get("deck") or {})
    observation.setdefault("deck_size", int((scenario.get("deck") or {}).get("deck_size") or 0))
    observation.setdefault("screen", screen_for_scenario(scenario, candidates))
    return {
        "payload": {
            "observation": observation,
            "legal_action_count": len(candidates),
            "action_mask": [True for _ in candidates],
            "action_candidates": [
                {
                    "action_index": int(candidate["action_index"]),
                    "action_id": int(candidate["action_id"]),
                    "action_key": candidate["action_key"],
                    "action": candidate["action"],
                    "card": candidate.get("card"),
                }
                for candidate in candidates
            ],
        },
        "info": {"combat_win_count": int(scenario.get("combat_win_count") or 0)},
        "reward": 0,
        "done": False,
    }


def screen_for_scenario(scenario: dict[str, Any], candidates: list[dict[str, Any]]) -> dict[str, Any]:
    decision_type = str(scenario.get("decision_type") or "")
    if decision_type == "reward_card_choice":
        return {"reward_card_choice_count": sum(1 for candidate in candidates if candidate.get("card"))}
    if decision_type == "shop":
        return {"shop_card_count": sum(1 for candidate in candidates if candidate.get("card"))}
    return {}


def build_ranking(candidates: list[dict[str, Any]], scores: dict[str, float]) -> list[dict[str, Any]]:
    ranked = sorted(
        candidates,
        key=lambda candidate: (
            float(scores.get(candidate["id"], float("-inf"))),
            -int(candidate["action_index"]),
        ),
        reverse=True,
    )
    return [
        {
            "rank": rank,
            "candidate_id": candidate["id"],
            "label": candidate["label"],
            "score": float(scores.get(candidate["id"], 0.0)),
            "action_key": candidate["action_key"],
        }
        for rank, candidate in enumerate(ranked, start=1)
    ]


def evaluate_label(label: dict[str, Any], ranking: list[dict[str, Any]]) -> dict[str, Any]:
    mode = str(label.get("mode") or "unlabeled")
    top = str((ranking[0] if ranking else {}).get("candidate_id") or "")
    preferred = set(str(item) for item in label.get("preferred") or [])
    acceptable = set(str(item) for item in label.get("acceptable") or [])
    bad = set(str(item) for item in label.get("bad") or [])
    if mode in {"no_signal", "abstain", "needs_rollout", "unlabeled"}:
        return {"mode": mode, "status": "not_scored", "top_choice": top}
    if top in preferred:
        return {"mode": mode, "status": "pass", "top_choice": top}
    if top in acceptable:
        return {"mode": mode, "status": "acceptable", "top_choice": top}
    if top in bad:
        return {"mode": mode, "status": "fail_bad_choice", "top_choice": top}
    return {"mode": mode, "status": "miss", "top_choice": top}


def review_candidates(candidates: list[dict[str, Any]]) -> list[dict[str, Any]]:
    rows = []
    for candidate in candidates:
        rows.append(
            {
                "id": candidate["id"],
                "label": candidate["label"],
                "action_key": candidate["action_key"],
                "card": (candidate.get("card") or {}).get("name"),
                "rule_score": (candidate.get("card") or {}).get("rule_score", candidate.get("scenario_rule_score")),
                "scenario_meta": candidate.get("scenario_meta") or {},
            }
        )
    return rows


def feature_limitations(scenario: dict[str, Any]) -> list[str]:
    decision_type = str(scenario.get("decision_type") or "")
    limitations = []
    observation = scenario.get("observation") or {}
    if decision_type in {"boss_relic", "map"}:
        limitations.append("model_obs_v0_only_encodes_action_key_for_this_candidate_type")
    if decision_type == "reward_card_choice" and observation.get("post_boss_reward") is True:
        non_rare = []
        for candidate in scenario.get("candidates") or []:
            if str(candidate.get("kind") or "") == "skip":
                continue
            card = candidate.get("card") or {}
            if str(card.get("rarity") or "").lower() != "rare":
                non_rare.append(str(candidate.get("id") or candidate.get("label") or "unknown"))
        if non_rare:
            limitations.append("invalid_post_boss_reward_contains_non_rare_card")
    if str((scenario.get("human_label") or {}).get("mode") or "") == "no_signal":
        limitations.append("human_label_marks_no_local_training_signal")
    return limitations


def summarize_scenarios(scenarios: list[dict[str, Any]]) -> dict[str, Any]:
    by_policy: dict[str, Counter[str]] = defaultdict(Counter)
    by_policy_category: dict[str, dict[str, Counter[str]]] = defaultdict(lambda: defaultdict(Counter))
    by_policy_tag: dict[str, dict[str, Counter[str]]] = defaultdict(lambda: defaultdict(Counter))
    by_policy_evidence: dict[str, dict[str, Counter[str]]] = defaultdict(lambda: defaultdict(Counter))
    label_modes = Counter()
    evidence_modes = Counter()
    rationale_tags = Counter()
    categories = Counter()
    for scenario in scenarios:
        category = str(scenario.get("category") or "unknown")
        label = scenario.get("human_label") or {}
        evidence = str(label.get("evidence") or "unlabeled")
        tags = [str(tag) for tag in label.get("rationale_tags") or []] or ["untagged"]
        categories[category] += 1
        label_modes[str(label.get("mode") or "unlabeled")] += 1
        evidence_modes[evidence] += 1
        for tag in tags:
            rationale_tags[tag] += 1
        for policy in scenario.get("policies") or []:
            name = str(policy.get("policy") or "unknown")
            status = str((policy.get("label_eval") or {}).get("status") or "unknown")
            by_policy[name][status] += 1
            by_policy_category[name][category][status] += 1
            by_policy_evidence[name][evidence][status] += 1
            for tag in tags:
                by_policy_tag[name][tag][status] += 1
    return {
        "scenario_count": len(scenarios),
        "category_counts": dict(categories),
        "label_mode_counts": dict(label_modes),
        "evidence_counts": dict(evidence_modes),
        "rationale_tag_counts": dict(rationale_tags),
        "policy_label_status_counts": {policy: dict(counts) for policy, counts in by_policy.items()},
        "policy_category_status_counts": {
            policy: {category: dict(counts) for category, counts in categories.items()}
            for policy, categories in by_policy_category.items()
        },
        "policy_evidence_status_counts": {
            policy: {evidence: dict(counts) for evidence, counts in evidence_rows.items()}
            for policy, evidence_rows in by_policy_evidence.items()
        },
        "policy_rationale_tag_status_counts": {
            policy: {tag: dict(counts) for tag, counts in tag_rows.items()}
            for policy, tag_rows in by_policy_tag.items()
        },
    }


def build_human_review(report: dict[str, Any]) -> dict[str, Any]:
    return {
        "instructions": [
            "Edit human_label only; candidate ids must match the candidates list.",
            "Use mode=no_signal when the local decision is not meaningfully rescuable or preferences are arbitrary.",
            "Use mode=needs_rollout when human intuition is weak and paired rollouts should label the case later.",
            "Use rationale_tags for structured reasons; reason text is for humans and should not be treated as a direct training label.",
            "Use evidence=human_prior until a rollout/regression result backs the label.",
            "Do not treat model/rule rankings as labels; they are shown only to make review faster.",
        ],
        "source_report_version": report.get("report_version"),
        "generated_at_utc": report.get("generated_at_utc"),
        "scenarios": [
            {
                "id": scenario.get("id"),
                "category": scenario.get("category"),
                "decision_type": scenario.get("decision_type"),
                "context_note": scenario.get("context_note"),
                "observation": scenario.get("observation") or {},
                "deck": scenario.get("deck") or {},
                "relics": scenario.get("relics") or [],
                "potions": scenario.get("potions") or [],
                "feature_limitations": scenario.get("feature_limitations") or [],
                "candidates": scenario.get("candidates"),
                "human_label": scenario.get("human_label"),
                "policy_top_choices": {
                    policy.get("policy"): (policy.get("top_choice") or {}).get("candidate_id")
                    for policy in scenario.get("policies") or []
                },
                "policy_rankings": {
                    policy.get("policy"): policy.get("ranking") for policy in scenario.get("policies") or []
                },
            }
            for scenario in report.get("scenarios") or []
        ],
    }


def stable_action_id(action_key: str) -> int:
    value = 2_166_136_261
    for byte in str(action_key).encode("utf-8"):
        value ^= byte
        value = (value * 16_777_619) & 0xFFFFFFFF
    return value


if __name__ == "__main__":
    raise SystemExit(main())
