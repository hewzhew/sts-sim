"""Combat multi-turn lab diagnostics and compact views."""

from __future__ import annotations

from typing import Any

from sts_agent.evidence.tool_result_views import (
    compact_decision_lab_result,
    compact_event_action_key,
)


def combat_tactical_brief_from_branches(
    branches: list[dict[str, Any]],
    *,
    max_rollout_steps: Any,
) -> dict[str, Any]:
    fatal: list[dict[str, Any]] = []
    critical: list[dict[str, Any]] = []
    immediate_hp_loss: list[dict[str, Any]] = []
    surviving: list[dict[str, Any]] = []
    best_terminal_hp: int | None = None
    worst_terminal_hp: int | None = None

    for branch in branches:
        if not isinstance(branch, dict):
            continue
        action = {
            "action_id": branch.get("root_action_id"),
            "action_key": compact_event_action_key(branch.get("root_action_key")),
        }
        root_delta = branch.get("root_state_delta") if isinstance(branch.get("root_state_delta"), dict) else {}
        terminal = branch.get("terminal") if isinstance(branch.get("terminal"), dict) else {}
        flags = branch.get("final_risk_flags") or []
        hp_delta = root_delta.get("hp_delta")
        terminal_hp = terminal.get("hp")
        result = str(terminal.get("result") or "").lower()
        terminal_reason = str(terminal.get("terminal_reason") or "").lower()
        is_defeat = result == "defeat" or terminal_reason == "game_over" or (
            isinstance(terminal_hp, int) and terminal_hp <= 0 and str(branch.get("stop_reason") or "").startswith("terminal")
        )
        is_critical = (
            is_defeat
            or "lethal_incoming" in flags
            or "low_hp" in flags
            or (isinstance(terminal_hp, int) and terminal_hp <= 5)
        )
        annotated = {
            **action,
            "root_hp_delta": hp_delta,
            "terminal_hp": terminal_hp,
            "terminal_result": terminal.get("result"),
            "terminal_reason": terminal.get("terminal_reason"),
            "stop_reason": branch.get("stop_reason"),
            "final_risk_flags": flags,
        }
        if isinstance(terminal_hp, int):
            best_terminal_hp = terminal_hp if best_terminal_hp is None else max(best_terminal_hp, terminal_hp)
            worst_terminal_hp = terminal_hp if worst_terminal_hp is None else min(worst_terminal_hp, terminal_hp)
        if isinstance(hp_delta, int) and hp_delta < 0:
            immediate_hp_loss.append(annotated)
        if is_defeat:
            fatal.append(annotated)
        elif is_critical:
            critical.append(annotated)
        else:
            surviving.append(annotated)

    surviving_sorted = sorted(
        surviving,
        key=lambda item: item.get("terminal_hp") if isinstance(item.get("terminal_hp"), int) else -999,
        reverse=True,
    )
    warnings: list[str] = []
    if fatal:
        warnings.append("Some sampled root actions reach defeat in the bounded combat lab.")
    if critical:
        warnings.append("Some sampled root actions leave the player at critical HP or with lethal/low-HP risk.")
    if immediate_hp_loss:
        warnings.append("Some root actions immediately lose HP; do not call them safe.")
    if branches and len(fatal) == len(branches):
        warnings.append("All sampled root actions reach defeat in the bounded lab; choose the least-bad survival line and say it is degraded.")
    elif not surviving_sorted and branches:
        warnings.append("No clearly safe sampled root action was found.")

    return {
        "schema_name": "CombatTacticalBrief",
        "schema_version": 1,
        "evidence_role": "hard_tactical_evidence_not_command",
        "worldline_model": "bounded_combat_branch_lab_v0",
        "probability_model": "not_implemented_v0",
        "max_rollout_steps": max_rollout_steps,
        "sampled_branch_count": len(branches),
        "fatal_root_actions": fatal[:4],
        "critical_root_actions": critical[:4],
        "immediate_hp_loss_actions": immediate_hp_loss[:4],
        "best_surviving_root_actions": surviving_sorted[:4],
        "best_terminal_hp": best_terminal_hp,
        "worst_terminal_hp": worst_terminal_hp,
        "all_sampled_branches_defeat": bool(branches) and len(fatal) == len(branches),
        "warnings": warnings,
        "llm_handling_rule": (
            "Treat fatal_root_actions and critical_root_actions as hard negative evidence. "
            "If a best_surviving_root_action exists, do not choose a fatal or critical action unless you explain a forced constraint."
        ),
    }

def combat_lab_sampling_exhaustiveness_check(
    branches: list[dict[str, Any]],
    candidates: list[dict[str, Any]],
) -> dict[str, Any]:
    sampled_ids: set[int] = set()
    sampled_keys: list[str] = []
    for branch in branches:
        try:
            action_id = int(branch.get("root_action_id"))
        except (TypeError, ValueError):
            continue
        sampled_ids.add(action_id)
        key = branch.get("root_action_key")
        if isinstance(key, str) and key:
            sampled_keys.append(key)

    legal: list[dict[str, Any]] = []
    for candidate in candidates:
        try:
            action_id = int(candidate.get("id"))
        except (TypeError, ValueError):
            continue
        legal.append(
            {
                "action_id": action_id,
                "action_key": str(candidate.get("action_key") or ""),
            }
        )

    unsampled = [item for item in legal if item["action_id"] not in sampled_ids]
    legal_ids = {item["action_id"] for item in legal}
    sampled_legal_count = len(sampled_ids & legal_ids) if legal_ids else len(sampled_ids)
    sampled_branch_count = len(branches)
    all_sampled_defeat = bool(branches) and all(
        ((branch.get("terminal") or {}).get("result") == "defeat")
        or ((branch.get("terminal") or {}).get("terminal_reason") == "game_over")
        for branch in branches
        if isinstance(branch, dict)
    )
    has_surviving_sampled_root = any(
        ((branch.get("terminal") or {}).get("result") not in {"defeat", "lost"})
        and ((branch.get("terminal") or {}).get("terminal_reason") != "game_over")
        for branch in branches
        if isinstance(branch, dict)
    )
    exhaustive = bool(legal) and not unsampled and sampled_legal_count == len(legal)

    warnings: list[str] = []
    if not branches:
        reliability = "no_sampled_roots"
        warnings.append("No sampled root actions were returned by combat_multi_turn_lab.")
    elif exhaustive and all_sampled_defeat:
        reliability = "all_legal_sampled_roots_defeat"
    elif all_sampled_defeat and unsampled:
        reliability = "cannot_conclude_no_survival"
        warnings.append(
            "All sampled branches defeat, but not all current legal root actions were sampled."
        )
    elif has_surviving_sampled_root:
        reliability = "has_surviving_sampled_root"
    elif unsampled:
        reliability = "partial_sample"
        warnings.append("Some legal root actions were not sampled.")
    else:
        reliability = "sampled_roots_no_clear_survival"

    end_turn_id = next(
        (
            item["action_id"]
            for item in legal
            if item["action_key"] == "combat/end_turn"
        ),
        None,
    )
    potion_sampled = [
        key for key in sampled_keys if key.startswith("combat/use_potion/")
    ]
    potion_unsampled = [
        item
        for item in unsampled
        if item["action_key"].startswith("combat/use_potion/")
    ]

    return {
        "schema_name": "CombatLabSamplingExhaustivenessCheck",
        "schema_version": 1,
        "evidence_role": "sampling_diagnostic_not_command",
        "worldline_model": "bounded_combat_branch_lab_v0",
        "probability_model": "not_implemented_v0",
        "legal_candidate_count": len(legal),
        "sampled_branch_count": sampled_branch_count,
        "sampled_legal_root_count": sampled_legal_count,
        "unsampled_legal_root_count": len(unsampled),
        "sample_limited": bool(unsampled),
        "exhaustiveness_level": "exhaustive_legal_roots" if exhaustive else "partial_sample",
        "reliability": reliability,
        "all_sampled_branches_defeat": all_sampled_defeat,
        "has_surviving_sampled_root": has_surviving_sampled_root,
        "end_turn_sampled": end_turn_id in sampled_ids if end_turn_id is not None else None,
        "sampled_action_keys": sampled_keys[:6],
        "unsampled_action_keys": [
            item["action_key"] for item in unsampled[:6]
        ],
        "potion_action_sampled_count": len(potion_sampled),
        "potion_action_unsampled_count": len(potion_unsampled),
        "warnings": warnings,
        "llm_handling_rule": (
            "Do not interpret all_sampled_branches_defeat as proof that no legal survival line exists "
            "unless exhaustiveness_level is exhaustive_legal_roots. If reliability is "
            "cannot_conclude_no_survival, treat the lab as a partial sample and avoid giving up silently."
        ),
    }

def combat_lab_delaying_action_analysis_from_branches(
    branches: list[dict[str, Any]],
    *,
    max_rollout_steps: Any,
) -> dict[str, Any]:
    scored: list[dict[str, Any]] = []
    for branch in branches:
        if not isinstance(branch, dict):
            continue
        terminal = branch.get("terminal") if isinstance(branch.get("terminal"), dict) else {}
        delta = (
            branch.get("root_state_delta")
            if isinstance(branch.get("root_state_delta"), dict)
            else {}
        )
        result = str(terminal.get("result") or "")
        terminal_reason = str(terminal.get("terminal_reason") or "")
        terminal_hp = terminal.get("hp")
        if not isinstance(terminal_hp, int):
            terminal_hp = None
        rollout_steps = branch.get("rollout_step_count")
        if not isinstance(rollout_steps, int):
            rollout_steps = 0
        root_hp_delta = delta.get("hp_delta")
        if not isinstance(root_hp_delta, int):
            root_hp_delta = 0
        monster_hp_delta = delta.get("monster_hp_delta")
        if not isinstance(monster_hp_delta, int):
            monster_hp_delta = 0
        defeat = result == "defeat" or terminal_reason == "game_over" or (
            isinstance(terminal_hp, int) and terminal_hp <= 0
        )
        win = result in {"victory", "win"} or terminal_reason in {"combat_won", "victory"}
        terminal_rank = 3 if win else 2 if not defeat else 0
        score = (
            terminal_rank,
            terminal_hp if terminal_hp is not None else -9999,
            rollout_steps,
            root_hp_delta,
            -monster_hp_delta,
        )
        scored.append(
            {
                "action_id": branch.get("root_action_id"),
                "action_key": branch.get("root_action_key"),
                "root_hp_delta": root_hp_delta,
                "root_monster_hp_delta": monster_hp_delta,
                "terminal_hp": terminal_hp,
                "terminal_result": result,
                "terminal_reason": terminal_reason,
                "rollout_step_count": rollout_steps,
                "stop_reason": branch.get("stop_reason"),
                "final_risk_flags": branch.get("final_risk_flags") or [],
                "defeat": defeat,
                "_score": score,
            }
        )

    scored.sort(key=lambda item: item["_score"], reverse=True)
    best = [{k: v for k, v in item.items() if k != "_score"} for item in scored[:4]]
    all_sampled_defeat = bool(scored) and all(item.get("defeat") for item in scored)
    warnings: list[str] = []
    if not scored:
        status = "no_sampled_roots"
        warnings.append("No sampled root actions are available for delay analysis.")
    elif all_sampled_defeat:
        status = "all_sampled_roots_defeat"
        warnings.append(
            "All sampled root actions defeat; use best_delay_actions as least-bad evidence, not as proof of survival."
        )
    else:
        status = "survival_available"
        warnings.append(
            "At least one sampled root action is non-defeat; prefer survival evidence over delay-only evidence."
        )

    return {
        "schema_name": "CombatLabDelayingActionAnalysis",
        "schema_version": 1,
        "evidence_role": "least_bad_action_evidence_not_command",
        "worldline_model": "bounded_combat_branch_lab_v0",
        "probability_model": "not_implemented_v0",
        "status": status,
        "max_rollout_steps": max_rollout_steps,
        "all_sampled_branches_defeat": all_sampled_defeat,
        "best_delay_actions": best,
        "best_delay_action_key": (best[0].get("action_key") if best else None),
        "ranking_rule": (
            "Prefer non-defeat over defeat; otherwise maximize terminal_hp, rollout_step_count, "
            "root_hp_delta, then root monster damage."
        ),
        "warnings": warnings,
        "llm_handling_rule": (
            "If every sampled branch is fatal, do not default to end_turn. Use best_delay_actions "
            "to choose the least-bad legal action, while keeping the decision marked under-evidenced."
        ),
    }

def combat_potion_option_check(
    branches: list[dict[str, Any]],
    candidates: list[dict[str, Any]],
) -> dict[str, Any]:
    sampled_ids: set[int] = set()
    for branch in branches:
        try:
            sampled_ids.add(int(branch.get("root_action_id")))
        except (TypeError, ValueError):
            continue

    potion_options: list[dict[str, Any]] = []
    for candidate in candidates:
        key = str(candidate.get("action_key") or "")
        if not key.startswith(("combat/use_potion/", "potion/use/")):
            continue
        try:
            action_id = int(candidate.get("id"))
        except (TypeError, ValueError):
            continue
        payload = candidate.get("payload") if isinstance(candidate.get("payload"), dict) else {}
        action = payload.get("action") if isinstance(payload.get("action"), dict) else {}
        potion_options.append(
            {
                "action_id": action_id,
                "action_key": key,
                "sampled_by_lab": action_id in sampled_ids,
                "public_action": action,
            }
        )

    unsampled = [item for item in potion_options if not item.get("sampled_by_lab")]
    warnings: list[str] = []
    if unsampled:
        warnings.append("Potion actions are legal but were not sampled by combat_multi_turn_lab.")

    return {
        "schema_name": "CombatPotionOptionCheck",
        "schema_version": 1,
        "evidence_role": "potion_availability_diagnostic_not_command",
        "worldline_model": "current_legal_action_inventory_v0",
        "probability_model": "not_implemented_v0",
        "legal_potion_action_count": len(potion_options),
        "sampled_potion_action_count": len(potion_options) - len(unsampled),
        "unsampled_potion_action_count": len(unsampled),
        "potion_options": potion_options[:6],
        "warnings": warnings,
        "llm_handling_rule": (
            "If legal_potion_action_count is nonzero in low-HP combat, explicitly consider whether a potion line "
            "changes survival before choosing a non-potion action or end_turn."
        ),
    }

def combat_lab_rollout_depth_adequacy_check(
    branches: list[dict[str, Any]],
    *,
    max_rollout_steps: Any,
) -> dict[str, Any]:
    ongoing_at_limit = 0
    terminal_resolved = 0
    defeat_count = 0
    victory_count = 0
    stop_reasons: dict[str, int] = {}
    shallow_defeats: list[dict[str, Any]] = []

    for branch in branches:
        if not isinstance(branch, dict):
            continue
        terminal = branch.get("terminal") if isinstance(branch.get("terminal"), dict) else {}
        result = str(terminal.get("result") or "")
        reason = str(terminal.get("terminal_reason") or "")
        stop = str(branch.get("stop_reason") or "")
        stop_reasons[stop] = stop_reasons.get(stop, 0) + 1
        terminal_hp = terminal.get("hp")
        defeat = result == "defeat" or reason == "game_over" or (
            isinstance(terminal_hp, int) and terminal_hp <= 0
        )
        victory = result in {"victory", "win"} or reason in {"combat_won", "victory"}
        if defeat:
            defeat_count += 1
            shallow_defeats.append(
                {
                    "action_key": branch.get("root_action_key"),
                    "rollout_step_count": branch.get("rollout_step_count"),
                    "terminal_hp": terminal_hp,
                    "terminal_reason": reason,
                }
            )
        if victory:
            victory_count += 1
        if result == "ongoing" and stop == "max_rollout_steps_reached":
            ongoing_at_limit += 1
        elif result and result != "ongoing":
            terminal_resolved += 1

    branch_count = len(branches)
    warnings: list[str] = []
    if branch_count == 0:
        adequacy = "no_sampled_roots"
        warnings.append("No sampled roots are available, so rollout depth cannot be assessed.")
    elif ongoing_at_limit:
        adequacy = "depth_limited_for_long_horizon"
        warnings.append("Some sampled roots were still ongoing when max_rollout_steps was reached.")
    elif terminal_resolved == branch_count:
        adequacy = "terminal_resolved_for_sampled_roots"
    else:
        adequacy = "mixed_or_unclear"
        warnings.append("Sampled root outcomes are mixed; do not treat this as full long-horizon proof.")

    if defeat_count == branch_count and branch_count > 0:
        warnings.append(
            "All sampled roots ended in defeat within the bounded rollout; this is strong for sampled roots but not for unsampled roots."
        )

    return {
        "schema_name": "CombatLabRolloutDepthAdequacyCheck",
        "schema_version": 1,
        "evidence_role": "rollout_depth_diagnostic_not_command",
        "worldline_model": "bounded_combat_branch_lab_v0",
        "probability_model": "not_implemented_v0",
        "max_rollout_steps": max_rollout_steps,
        "sampled_branch_count": branch_count,
        "ongoing_at_depth_limit_count": ongoing_at_limit,
        "terminal_resolved_count": terminal_resolved,
        "defeat_count": defeat_count,
        "victory_count": victory_count,
        "stop_reasons": stop_reasons,
        "adequacy": adequacy,
        "deeper_search_worthwhile": bool(ongoing_at_limit),
        "sampled_shallow_defeats": shallow_defeats[:4],
        "warnings": warnings,
        "llm_handling_rule": (
            "If adequacy is depth_limited_for_long_horizon, treat the lab as short-horizon evidence. "
            "If terminal_resolved_for_sampled_roots, the bounded rollout is stronger for those sampled roots, "
            "but still does not prove anything about unsampled legal roots."
        ),
    }

def combat_evidence_conflict_resolver(
    probe_result: dict[str, Any],
    lab_result: dict[str, Any],
) -> dict[str, Any]:
    plans = probe_result.get("plans") if isinstance(probe_result.get("plans"), dict) else {}
    branches = lab_result.get("branches") if isinstance(lab_result.get("branches"), list) else []
    brief = lab_result.get("hard_tactical_brief") if isinstance(lab_result.get("hard_tactical_brief"), dict) else {}
    sampling = (
        lab_result.get("sampling_exhaustiveness_check")
        if isinstance(lab_result.get("sampling_exhaustiveness_check"), dict)
        else {}
    )

    branch_by_key: dict[str, dict[str, Any]] = {}
    for branch in branches:
        if not isinstance(branch, dict):
            continue
        key = branch.get("root_action_key")
        if isinstance(key, str) and key:
            branch_by_key[key] = branch

    conflicts: list[dict[str, Any]] = []
    for plan_name in ["Lethal", "FullBlock", "BlockEnoughThenDamage", "MaxDamage"]:
        plan = plans.get(plan_name)
        if not isinstance(plan, dict):
            continue
        key = plan.get("first_action_key")
        if not isinstance(key, str) or not key:
            continue
        branch = branch_by_key.get(key)
        if branch is None:
            conflicts.append(
                {
                    "kind": "probe_plan_not_sampled_by_lab",
                    "plan": plan_name,
                    "action_key": key,
                    "severity": "medium",
                    "interpretation": "A probe plan exists, but the multi-turn lab did not sample that root action.",
                }
            )
            continue
        terminal = branch.get("terminal") if isinstance(branch.get("terminal"), dict) else {}
        result = terminal.get("result")
        reason = terminal.get("terminal_reason")
        hp = terminal.get("hp")
        defeat = result == "defeat" or reason == "game_over" or (
            isinstance(hp, int) and hp <= 0
        )
        if defeat and plan_name in {"Lethal", "FullBlock", "BlockEnoughThenDamage"}:
            conflicts.append(
                {
                    "kind": "probe_plan_fails_in_lab_rollout",
                    "plan": plan_name,
                    "action_key": key,
                    "severity": "high",
                    "terminal_result": result,
                    "terminal_hp": hp,
                    "interpretation": "A tactical probe plan looks promising, but bounded rollout marks that root as fatal.",
                }
            )

    all_sampled_defeat = bool(brief.get("all_sampled_branches_defeat"))
    if all_sampled_defeat and sampling.get("reliability") == "cannot_conclude_no_survival":
        conflicts.append(
            {
                "kind": "fatal_sample_not_exhaustive",
                "severity": "high",
                "interpretation": "The lab says all sampled branches defeat, but sampling was partial.",
                "unsampled_legal_root_count": sampling.get("unsampled_legal_root_count"),
            }
        )

    status = "conflict" if conflicts else "consistent_or_no_conflict_detected"
    return {
        "tool": "combat_evidence_conflict_resolver",
        "status": status,
        "schema_name": "CombatEvidenceConflictResolver",
        "schema_version": 1,
        "evidence_role": "tool_consistency_diagnostic_not_command",
        "worldline_model": "bounded_combat_branch_lab_v0_plus_current_turn_probe_v0",
        "probability_model": "not_implemented_v0",
        "conflict_count": len(conflicts),
        "conflicts": conflicts[:6],
        "lab_reliability": sampling.get("reliability"),
        "lab_exhaustiveness_level": sampling.get("exhaustiveness_level"),
        "warnings": [
            "This resolves tool-evidence consistency only; it is not a final action recommendation."
        ],
        "llm_handling_rule": (
            "If conflicts are present, do not blindly trust a single tool. Explain which evidence you prioritize. "
            "If fatal_sample_not_exhaustive is present, do not conclude there is no survival line."
        ),
    }

def combat_end_turn_commitment_check(
    branches: list[dict[str, Any]],
    candidates: list[dict[str, Any]],
) -> dict[str, Any]:
    legal: list[dict[str, Any]] = []
    for candidate in candidates:
        try:
            action_id = int(candidate.get("id"))
        except (TypeError, ValueError):
            continue
        legal.append(
            {
                "action_id": action_id,
                "action_key": str(candidate.get("action_key") or ""),
            }
        )

    end_turn_ids = {
        item["action_id"]
        for item in legal
        if item["action_key"] == "combat/end_turn"
    }
    non_end_legal = [
        item
        for item in legal
        if item["action_id"] not in end_turn_ids
    ]

    def summarize_branch(branch: dict[str, Any]) -> dict[str, Any]:
        terminal = branch.get("terminal") if isinstance(branch.get("terminal"), dict) else {}
        delta = (
            branch.get("root_state_delta")
            if isinstance(branch.get("root_state_delta"), dict)
            else {}
        )
        result = str(terminal.get("result") or "")
        reason = str(terminal.get("terminal_reason") or "")
        terminal_hp = terminal.get("hp")
        if not isinstance(terminal_hp, int):
            terminal_hp = None
        root_hp_delta = delta.get("hp_delta")
        if not isinstance(root_hp_delta, int):
            root_hp_delta = 0
        monster_hp_delta = delta.get("monster_hp_delta")
        if not isinstance(monster_hp_delta, int):
            monster_hp_delta = 0
        rollout_steps = branch.get("rollout_step_count")
        if not isinstance(rollout_steps, int):
            rollout_steps = 0
        defeat = result == "defeat" or reason == "game_over" or (
            isinstance(terminal_hp, int) and terminal_hp <= 0
        )
        victory = result in {"victory", "win"} or reason in {"combat_won", "victory"}
        return {
            "action_id": branch.get("root_action_id"),
            "action_key": branch.get("root_action_key"),
            "root_hp_delta": root_hp_delta,
            "root_monster_hp_delta": monster_hp_delta,
            "terminal_hp": terminal_hp,
            "terminal_result": result,
            "terminal_reason": reason,
            "rollout_step_count": rollout_steps,
            "stop_reason": branch.get("stop_reason"),
            "final_risk_flags": branch.get("final_risk_flags") or [],
            "defeat": defeat,
            "victory": victory,
        }

    def score_summary(summary: dict[str, Any]) -> tuple[int, int, int, int, int]:
        terminal_hp = summary.get("terminal_hp")
        terminal_hp_value = terminal_hp if isinstance(terminal_hp, int) else -9999
        terminal_rank = 3 if summary.get("victory") else 2 if not summary.get("defeat") else 0
        return (
            terminal_rank,
            terminal_hp_value,
            int(summary.get("rollout_step_count") or 0),
            int(summary.get("root_hp_delta") or 0),
            -int(summary.get("root_monster_hp_delta") or 0),
        )

    sampled_ids: set[int] = set()
    end_turn_branch: dict[str, Any] | None = None
    non_end_summaries: list[dict[str, Any]] = []
    for branch in branches:
        if not isinstance(branch, dict):
            continue
        try:
            action_id = int(branch.get("root_action_id"))
        except (TypeError, ValueError):
            continue
        sampled_ids.add(action_id)
        summary = summarize_branch(branch)
        if action_id in end_turn_ids:
            end_turn_branch = summary
        else:
            non_end_summaries.append(summary)

    non_end_summaries.sort(key=score_summary, reverse=True)
    best_non_end = non_end_summaries[0] if non_end_summaries else None
    unsampled_non_end = [
        item for item in non_end_legal if item["action_id"] not in sampled_ids
    ]

    warnings: list[str] = []
    if not end_turn_ids:
        commitment_risk = "no_end_turn_legal"
    elif not non_end_legal:
        commitment_risk = "no_alternative_legal_action"
    elif end_turn_branch is None:
        commitment_risk = "end_turn_not_sampled"
        warnings.append("End turn is legal but was not sampled by combat_multi_turn_lab.")
    elif best_non_end is None:
        commitment_risk = "non_end_actions_not_sampled"
        warnings.append("Non-end legal actions exist but none were sampled by combat_multi_turn_lab.")
    else:
        end_score = score_summary(end_turn_branch)
        non_end_score = score_summary(best_non_end)
        if end_turn_branch.get("defeat") and not best_non_end.get("defeat"):
            commitment_risk = "premature_end_turn_fatal"
        elif non_end_score > end_score:
            commitment_risk = "premature_end_turn_worse_than_sampled_non_end"
        elif unsampled_non_end:
            commitment_risk = "end_turn_under_evidenced_due_to_unsampled_non_end"
            warnings.append("Some non-end legal actions were not sampled before evaluating end_turn.")
        else:
            commitment_risk = "end_turn_not_worse_than_sampled_non_end"

    requires_explanation = commitment_risk in {
        "premature_end_turn_fatal",
        "premature_end_turn_worse_than_sampled_non_end",
        "end_turn_under_evidenced_due_to_unsampled_non_end",
        "non_end_actions_not_sampled",
        "end_turn_not_sampled",
    }

    return {
        "schema_name": "CombatEndTurnCommitmentCheck",
        "schema_version": 1,
        "evidence_role": "turn_commitment_diagnostic_not_command",
        "worldline_model": "bounded_combat_branch_lab_v0",
        "probability_model": "not_implemented_v0",
        "end_turn_legal": bool(end_turn_ids),
        "end_turn_sampled": end_turn_branch is not None,
        "non_end_legal_count": len(non_end_legal),
        "sampled_non_end_count": len(non_end_summaries),
        "unsampled_non_end_count": len(unsampled_non_end),
        "commitment_risk": commitment_risk,
        "requires_llm_explanation": requires_explanation,
        "end_turn_outcome": end_turn_branch,
        "best_sampled_non_end_action": best_non_end,
        "unsampled_non_end_action_keys": [
            item["action_key"] for item in unsampled_non_end[:6]
        ],
        "warnings": warnings,
        "llm_handling_rule": (
            "Choosing end_turn commits the current hand and gives control to the enemy turn. "
            "If requires_llm_explanation is true, do not choose end_turn unless you explicitly explain "
            "why no sampled or unsampled non-end legal action is preferable."
        ),
    }

def compact_combat_multi_turn_lab_result(
    result: dict[str, Any],
    candidates: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    lab = compact_decision_lab_result(result, candidates)
    branches = lab.get("branches") or []
    sampling_check = combat_lab_sampling_exhaustiveness_check(branches, candidates or [])
    delaying_analysis = combat_lab_delaying_action_analysis_from_branches(
        branches,
        max_rollout_steps=lab.get("max_rollout_steps"),
    )
    potion_check = combat_potion_option_check(branches, candidates or [])
    depth_check = combat_lab_rollout_depth_adequacy_check(
        branches,
        max_rollout_steps=lab.get("max_rollout_steps"),
    )
    end_turn_check = combat_end_turn_commitment_check(branches, candidates or [])
    return {
        "tool": "combat_multi_turn_lab",
        "status": lab.get("status"),
        "schema_version": lab.get("schema_version"),
        "hard_tactical_brief": combat_tactical_brief_from_branches(
            branches,
            max_rollout_steps=lab.get("max_rollout_steps"),
        ),
        "base_worldline_model": lab.get("worldline_model"),
        "worldline_model": "bounded_combat_branch_lab_v0",
        "probability_model": lab.get("probability_model"),
        "truth_warnings": lab.get("truth_warnings") or [],
        "max_rollout_steps": lab.get("max_rollout_steps"),
        "scope": "combat_root_action_branches",
        "sampling_exhaustiveness_check": sampling_check,
        "delaying_action_analysis": delaying_analysis,
        "potion_option_check": potion_check,
        "rollout_depth_adequacy_check": depth_check,
        "end_turn_commitment_check": end_turn_check,
        "branches": branches,
    }
