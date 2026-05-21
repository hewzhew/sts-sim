"""Action validation, routine-action handling, and search-action selection."""

from __future__ import annotations

from typing import Any

from sts_agent.utils.llm_utils import find_candidate, short_action_label


def validate_action_id(action_id: Any, candidates: list[dict[str, Any]]) -> tuple[int, bool, str]:
    legal_ids = [int(candidate["id"]) for candidate in candidates]
    try:
        parsed = int(action_id)
    except (TypeError, ValueError):
        return legal_ids[0], False, f"non-integer action_id {action_id!r}; fell back to {legal_ids[0]}"
    if parsed in legal_ids:
        return parsed, True, "legal"
    return legal_ids[0], False, f"illegal action_id {parsed}; fell back to {legal_ids[0]}"

def action_id_for_key(candidates: list[dict[str, Any]], action_key: Any) -> int | None:
    if not isinstance(action_key, str) or not action_key:
        return None
    for candidate in candidates:
        if candidate.get("action_key") == action_key:
            try:
                return int(candidate["id"])
            except (KeyError, TypeError, ValueError):
                return None
    return None

def routine_action_id(
    *,
    candidates: list[dict[str, Any]],
    decision_type: str | None,
    public_payload: dict[str, Any],
) -> tuple[int | None, str | None]:
    if len(candidates) == 1:
        return int(candidates[0]["id"]), "single_legal_action"

    def first_key(prefix: str) -> dict[str, Any] | None:
        return next(
            (
                candidate
                for candidate in candidates
                if str(candidate.get("action_key") or "").startswith(prefix)
            ),
            None,
        )

    if decision_type == "reward":
        claim = first_key("reward/claim/")
        if claim is not None:
            return int(claim["id"]), "routine_claim_visible_reward"
        proceed = first_key("proceed")
        if proceed is not None:
            return int(proceed["id"]), "routine_reward_proceed_after_claims"

    if decision_type == "map":
        map_candidates = [
            candidate
            for candidate in candidates
            if str(candidate.get("action_key") or "").startswith("map/select")
        ]
        if len(map_candidates) == 1:
            return int(map_candidates[0]["id"]), "routine_single_map_route"

    if decision_type in {"treasure", "event"}:
        structural = [
            candidate
            for candidate in candidates
            if not str(candidate.get("action_key") or "").startswith(("potion/", "discard_potion/"))
        ]
        if len(structural) == 1:
            return int(structural[0]["id"]), f"routine_single_{decision_type}_choice"

    if decision_type == "campfire":
        current_hp = public_payload.get("current_hp")
        max_hp = public_payload.get("max_hp")
        rest = next(
            (
                candidate
                for candidate in candidates
                if candidate.get("action_key") == "campfire/rest"
            ),
            None,
        )
        if (
            rest is not None
            and isinstance(current_hp, int)
            and isinstance(max_hp, int)
            and max_hp > 0
            and current_hp * 100 <= max_hp * 50
        ):
            return int(rest["id"]), "routine_low_hp_campfire_rest"

    return None, None

def first_structural_action_id(candidates: list[dict[str, Any]]) -> int:
    for candidate in candidates:
        action_key = str(candidate.get("action_key") or "")
        if action_key.startswith(("potion/", "discard_potion/")):
            continue
        return int(candidate["id"])
    return int(candidates[0]["id"])

def is_routine_mechanical_single_action(
    candidate: dict[str, Any] | None,
    public_payload: dict[str, Any],
) -> bool:
    if not isinstance(candidate, dict):
        return False
    action_key = str(candidate.get("action_key") or "")
    descriptor = candidate.get("action_descriptor") or (candidate.get("payload") or {}).get("semantic_descriptor") or {}
    label = str(descriptor.get("label") or short_action_label(candidate) or "").strip().lower()
    if label in {"[proceed]", "proceed", "leave.", "leave", "continue"}:
        return True
    if action_key == "reward/proceed":
        screen = public_payload.get("screen") if isinstance(public_payload.get("screen"), dict) else {}
        return int((screen or {}).get("reward_claimable_item_count") or 0) == 0
    return False

def plan_by_name(probe: dict[str, Any], plan_name: str) -> dict[str, Any] | None:
    for plan in probe.get("plans") or []:
        if plan.get("plan_name") == plan_name:
            return plan
    return None

def plan_score(plan: dict[str, Any] | None) -> dict[str, Any]:
    if not isinstance(plan, dict):
        return {}
    score = plan.get("best_score") or {}
    return score if isinstance(score, dict) else {}

def first_action_key_for_plan(
    probe: dict[str, Any],
    plan_name: str,
    *,
    require_score_signal: bool = False,
) -> str | None:
    plan = plan_by_name(probe, plan_name)
    if not plan:
        return None
    score = plan_score(plan)
    if plan_name == "Lethal" and int(score.get("lethal_score") or 0) <= 0:
        return None
    if require_score_signal and not any(
        int(score.get(key) or 0) > 0
        for key in ["block_score", "hp_loss_score", "damage_score", "enemy_death_score"]
    ):
        return None
    keys = plan.get("best_action_keys") or []
    if keys:
        return keys[0]
    return None

def choose_search_action(
    *,
    probe: dict[str, Any],
    candidates: list[dict[str, Any]],
) -> tuple[int, dict[str, Any]]:
    state_summary = probe.get("state_summary") or {}
    player_hp = int(state_summary.get("player_hp") or 0)
    unblocked = int(state_summary.get("unblocked_incoming_damage") or 0)

    attempts: list[tuple[str, str | None]] = []
    attempts.append(("Lethal", first_action_key_for_plan(probe, "Lethal")))
    if player_hp > 0 and unblocked >= player_hp:
        attempts.append(
            (
                "FullBlock",
                first_action_key_for_plan(
                    probe,
                    "FullBlock",
                    require_score_signal=True,
                ),
            )
        )
    if unblocked > 0:
        attempts.append(
            (
                "BlockEnoughThenDamage",
                first_action_key_for_plan(
                    probe,
                    "BlockEnoughThenDamage",
                    require_score_signal=True,
                ),
            )
        )
    attempts.append(("MaxDamage", first_action_key_for_plan(probe, "MaxDamage")))

    for plan_name, action_key in attempts:
        action_id = action_id_for_key(candidates, action_key)
        if action_id is not None:
            return action_id, search_evidence(
                probe=probe,
                selected_plan=plan_name,
                selected_action_key=action_key,
                fallback=False,
                fallback_reason=None,
            )

    affordances = probe.get("first_action_affordances") or []
    ranked_affordances = sorted(
        affordances,
        key=lambda item: (
            item.get("best_plan_rank")
            if item.get("best_plan_rank") is not None
            else 1_000_000,
            -(int(item.get("sequence_count") or 0)),
            str(item.get("action_key") or ""),
        ),
    )
    for affordance in ranked_affordances:
        action_key = affordance.get("action_key")
        action_id = action_id_for_key(candidates, action_key)
        if action_id is not None:
            return action_id, search_evidence(
                probe=probe,
                selected_plan="FirstActionAffordance",
                selected_action_key=action_key,
                fallback=False,
                fallback_reason=None,
            )

    fallback_id = int(candidates[0]["id"])
    return fallback_id, search_evidence(
        probe=probe,
        selected_plan="FallbackFirstLegal",
        selected_action_key=candidates[0].get("action_key"),
        fallback=True,
        fallback_reason="no probe action key matched current legal candidates",
    )

def compact_probe_plan(plan: dict[str, Any] | None) -> dict[str, Any] | None:
    if not isinstance(plan, dict):
        return None
    return {
        "plan_name": plan.get("plan_name"),
        "best_action_keys": plan.get("best_action_keys"),
        "best_actions": plan.get("best_actions"),
        "best_score": plan.get("best_score"),
        "candidate_sequence_count": plan.get("candidate_sequence_count"),
        "explanation": plan.get("explanation"),
    }

def search_evidence(
    *,
    probe: dict[str, Any],
    selected_plan: str,
    selected_action_key: str | None,
    fallback: bool,
    fallback_reason: str | None,
) -> dict[str, Any]:
    state_summary = probe.get("state_summary") or {}
    plan_names = [
        "Lethal",
        "FullBlock",
        "BlockEnoughThenDamage",
        "MaxDamage",
    ]
    return {
        "controller_role": "search_controller_behavior",
        "information_boundary": "engine_search",
        "label_role": "not_a_label",
        "trainable_as_action_label": False,
        "search_selected_plan": selected_plan,
        "search_selected_action_key": selected_action_key,
        "search_fallback": fallback,
        "search_fallback_reason": fallback_reason,
        "search_probe_schema_version": probe.get("schema_version"),
        "search_truth_warnings": probe.get("truth_warnings") or [],
        "probability_model": "not_implemented_v0",
        "worldline_model": "current_turn_sequence_search_only",
        "search_state_summary": {
            "player_hp": state_summary.get("player_hp"),
            "player_block": state_summary.get("player_block"),
            "energy": state_summary.get("energy"),
            "visible_incoming_damage": state_summary.get("visible_incoming_damage"),
            "unblocked_incoming_damage": state_summary.get("unblocked_incoming_damage"),
            "total_monster_hp": state_summary.get("total_monster_hp"),
            "alive_monster_count": state_summary.get("alive_monster_count"),
            "turn_count": state_summary.get("turn_count"),
        },
        "search_considered_plans": [
            compact_probe_plan(plan_by_name(probe, name)) for name in plan_names
        ],
        "search_probe_limits": probe.get("probe_limits"),
    }

def guardrail_action_id(
    *,
    parsed: dict[str, Any],
    candidates: list[dict[str, Any]],
    public_payload: dict[str, Any],
) -> tuple[Any, dict[str, Any] | None]:
    decision_type = public_payload.get("decision_type")
    if decision_type == "campfire":
        current_hp = public_payload.get("current_hp")
        max_hp = public_payload.get("max_hp")
        rest = next(
            (
                candidate
                for candidate in candidates
                if candidate.get("action_key") == "campfire/rest"
            ),
            None,
        )
        if (
            rest is not None
            and isinstance(current_hp, int)
            and isinstance(max_hp, int)
            and max_hp > 0
            and current_hp * 100 <= max_hp * 50
        ):
            return rest.get("id"), {
                "guardrail": "campfire_low_hp_rest",
                "reason": "hp <= 50% and rest is legal",
            }
    screen = public_payload.get("screen") or {}
    selected_id = parsed.get("action_id")
    selected = find_candidate(candidates, int(selected_id)) if str(selected_id).isdigit() else None
    if (
        decision_type == "reward"
        and isinstance(screen, dict)
        and int(screen.get("reward_claimable_item_count") or 0) > 0
        and selected is not None
        and selected.get("action_key") == "proceed"
    ):
        replacement = next(
            (
                candidate
                for candidate in candidates
                if str(candidate.get("action_key") or "").startswith("reward/claim/")
            ),
            None,
        )
        if replacement is not None:
            return replacement.get("id"), {
                "guardrail": "claim_visible_reward_before_proceed",
                "reason": "reward screen still has claimable items",
            }
    return parsed.get("action_id"), None
