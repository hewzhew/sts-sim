use sts_simulator::ai::potion_continuation_context_v1::{
    potion_run_continuation_context_v1, PotionRunContinuationContextV1,
};
use sts_simulator::ai::potion_continuation_pressure_v1::{
    potion_continuation_pressure_v1, PotionContinuationPressureV1,
};
use sts_simulator::eval::run_control::{
    atomic_combat_search_trace_summaries, strategic_combat_victory_reaches_full_heal_v1,
    AtomicCombatSearchAttemptV2, AtomicCombatSearchTraceSummaryV2,
    CombatSearchStrategicHpQualityFactsV1, CombatVictoryContinuationFactsV1, RunControlHpLossLimit,
    RunControlSession, RunControlTraceAnnotationV1, RunProgressOutcome, RunProgressStepV1,
};

use super::accepted_high_loss_diagnostic::{accepted_high_loss_diagnostic, capture_active_combat};
use super::atomic_combat_search_report::{
    atomic_combat_search_session_report, AtomicCombatSearchQuantumReportV2,
    AtomicCombatSearchSessionReportInputV2, AtomicCombatSearchSessionReportV2,
};
use super::atomic_combat_search_session_output::AtomicCombatSearchSessionOutputV2;
use super::atomic_combat_search_session_plan::{
    canonical_atomic_combat_search_session_plan, potion_conserving_primary_search_session_plan,
    potion_conserving_refinement_search_session_plan, AtomicCombatSearchSessionPlanV2,
    PotionRescueKind,
};
use super::atomic_combat_search_session_result::{
    atomic_combat_search_result, AtomicCombatSearchSessionResultV2,
};
use super::atomic_combat_search_survival::{
    owner_audit_hp_loss_limit, owner_audit_search_quality_loss_target,
};
use super::atomic_combat_search_trace_actions::complete_search_action_keys;
use super::{boundary_router, Args, BranchStatus};

pub(super) fn run_atomic_combat_search_session_step(
    session: &mut RunControlSession,
    args: Args,
) -> Result<AtomicCombatSearchSessionResultV2, String> {
    let canonical_plan = canonical_atomic_combat_search_session_plan(session, args);
    if canonical_plan.should_checkpoint_before_search(args) {
        let status = awaiting_auto_boundary(
            "Combat",
            "checkpoint before canonical combat search session".to_string(),
        );
        let report = session_report(
            &canonical_plan,
            status.clone(),
            Vec::new(),
            None,
            false,
            "checkpoint",
        );
        return Ok(atomic_combat_search_result(
            status,
            Some(report),
            AtomicCombatSearchSessionOutputV2::default(),
        ));
    }

    let active_combat = session
        .active_combat
        .as_ref()
        .ok_or_else(|| "combat search session has no active combat".to_string())?;
    let potion_continuation_context =
        potion_run_continuation_context_v1(&session.run_state, &active_combat.combat_state);
    let potion_continuation_pressure =
        potion_continuation_pressure_v1(&session.run_state, &potion_continuation_context);
    let combat_victory_continuation =
        CombatVictoryContinuationFactsV1::from_guaranteed_room_boss_full_heal(
            strategic_combat_victory_reaches_full_heal_v1(session),
        );
    let combat_capture = capture_active_combat(session)?;
    let owner_hp_loss_limit_fact = owner_audit_hp_loss_limit(session);
    let quality_loss_limit = owner_audit_search_quality_loss_target(session);
    let (entry_current_hp, entry_max_hp) = session.visible_player_hp();
    let strategic_hp_quality = CombatSearchStrategicHpQualityFactsV1::from_owner_limits(
        entry_current_hp,
        entry_max_hp,
        owner_hp_loss_limit_fact,
        quality_loss_limit,
    );
    let owner_hp_loss_limit = match owner_hp_loss_limit_fact {
        RunControlHpLossLimit::Limit(limit) => Some(limit),
        RunControlHpLossLimit::Unlimited => None,
    };
    let primary_plan = potion_conserving_primary_search_session_plan(session, args);
    let staged = primary_plan.is_some();
    let mut plan = primary_plan.unwrap_or(canonical_plan);
    let mut prior_search_summaries = Vec::new();
    let mut complete_staged_summaries = None;
    let outcome = if staged {
        let mut primary_attempt = match session.atomic_combat_search_attempt_v2(plan.search.clone())
        {
            Ok(attempt) => attempt,
            Err(error) => {
                return Ok(search_error_result(&plan, error, prior_search_summaries));
            }
        };
        let primary_satisfies = match quality_loss_limit {
            RunControlHpLossLimit::Limit(limit) => {
                match primary_attempt.select_verified_win_with_hp_loss_at_most(session, limit) {
                    Ok(candidate) => candidate.is_some(),
                    Err(error) => {
                        return Ok(search_error_result(&plan, error, prior_search_summaries));
                    }
                }
            }
            RunControlHpLossLimit::Unlimited => primary_attempt.verified_win().is_some(),
        };
        if primary_satisfies {
            match session.apply_atomic_combat_search_attempt_v2(primary_attempt, quality_loss_limit)
            {
                Ok(outcome) => outcome,
                Err(error) => {
                    return Ok(search_error_result(&plan, error, prior_search_summaries));
                }
            }
        } else if primary_attempt.verified_win().is_some() {
            let refinement = potion_conserving_refinement_search_session_plan(
                session,
                args,
                PotionRescueKind::ImproveVerifiedWinQualityGated,
            )
            .expect("a staged primary must retain one refinement quantum");
            let mut refinement_attempt =
                match session.atomic_combat_search_attempt_v2(refinement.search.clone()) {
                    Ok(attempt) => attempt,
                    Err(error) => {
                        let summaries = atomic_combat_search_attempt_summaries(
                            session,
                            &primary_attempt,
                            &plan,
                            owner_hp_loss_limit,
                            false,
                            "protected_incumbent_opened_quality_refinement",
                            &potion_continuation_context,
                            &potion_continuation_pressure,
                            &combat_victory_continuation,
                            &strategic_hp_quality,
                        );
                        return Ok(search_error_result(&refinement, error, summaries));
                    }
                };
            let satisfying_refinement = match quality_loss_limit {
                RunControlHpLossLimit::Limit(limit) => {
                    refinement_attempt.select_verified_win_with_hp_loss_at_most(session, limit)
                }
                RunControlHpLossLimit::Unlimited => Ok(refinement_attempt.verified_win()),
            };
            match satisfying_refinement {
                Ok(Some(_)) => {
                    prior_search_summaries.extend(atomic_combat_search_attempt_summaries(
                        session,
                        &primary_attempt,
                        &plan,
                        owner_hp_loss_limit,
                        false,
                        "protected_incumbent_replaced_by_satisfying_candidate",
                        &potion_continuation_context,
                        &potion_continuation_pressure,
                        &combat_victory_continuation,
                        &strategic_hp_quality,
                    ));
                    plan = refinement;
                    match session.apply_atomic_combat_search_attempt_v2(
                        refinement_attempt,
                        quality_loss_limit,
                    ) {
                        Ok(outcome) => outcome,
                        Err(error) => {
                            return Ok(search_error_result(&plan, error, prior_search_summaries));
                        }
                    }
                }
                Ok(None) => {
                    let mut summaries = atomic_combat_search_attempt_summaries(
                        session,
                        &primary_attempt,
                        &plan,
                        owner_hp_loss_limit,
                        true,
                        "accepted_protected_no_potion_incumbent",
                        &potion_continuation_context,
                        &potion_continuation_pressure,
                        &combat_victory_continuation,
                        &strategic_hp_quality,
                    );
                    summaries.extend(atomic_combat_search_attempt_summaries(
                        session,
                        &refinement_attempt,
                        &refinement,
                        owner_hp_loss_limit,
                        false,
                        "candidate_rejected_by_potion_quality_gate",
                        &potion_continuation_context,
                        &potion_continuation_pressure,
                        &combat_victory_continuation,
                        &strategic_hp_quality,
                    ));
                    match session.apply_atomic_combat_search_attempt_v2(
                        primary_attempt,
                        RunControlHpLossLimit::Unlimited,
                    ) {
                        Ok(outcome) => {
                            complete_staged_summaries = Some(summaries);
                            outcome
                        }
                        Err(error) => {
                            return Ok(search_error_result(&plan, error, summaries));
                        }
                    }
                }
                Err(error) => {
                    return Ok(search_error_result(
                        &refinement,
                        error,
                        prior_search_summaries,
                    ));
                }
            }
        } else {
            prior_search_summaries.extend(atomic_combat_search_attempt_summaries(
                session,
                &primary_attempt,
                &plan,
                owner_hp_loss_limit,
                false,
                "no_accepted_candidate",
                &potion_continuation_context,
                &potion_continuation_pressure,
                &combat_victory_continuation,
                &strategic_hp_quality,
            ));
            if let Some(refinement) = potion_conserving_refinement_search_session_plan(
                session,
                args,
                PotionRescueKind::FindAnyWin,
            ) {
                plan = refinement;
                let refinement_attempt =
                    match session.atomic_combat_search_attempt_v2(plan.search.clone()) {
                        Ok(attempt) => attempt,
                        Err(error) => {
                            return Ok(search_error_result(&plan, error, prior_search_summaries));
                        }
                    };
                match session.apply_atomic_combat_search_attempt_v2(
                    refinement_attempt,
                    owner_hp_loss_limit_fact,
                ) {
                    Ok(outcome) => outcome,
                    Err(error) => {
                        return Ok(search_error_result(&plan, error, prior_search_summaries));
                    }
                }
            } else {
                match session
                    .apply_atomic_combat_search_attempt_v2(primary_attempt, quality_loss_limit)
                {
                    Ok(outcome) => outcome,
                    Err(error) => {
                        return Ok(search_error_result(&plan, error, prior_search_summaries));
                    }
                }
            }
        }
    } else {
        match session.apply_atomic_combat_search_v2(plan.search.clone()) {
            Ok(outcome) => outcome,
            Err(error) => {
                return Ok(search_error_result(&plan, error, prior_search_summaries));
            }
        }
    };
    let status = search_status(session, &outcome);
    let action_keys = complete_search_action_keys(&outcome.trace_annotations);
    let applied_steps = committed_progress_steps(&outcome);
    let applied = !applied_steps.is_empty();
    let facts = candidate_facts(session, &outcome.trace_annotations, owner_hp_loss_limit);
    let decision = session_decision(applied, facts.as_ref());

    let atomic_combat_search_attempts = complete_staged_summaries.unwrap_or_else(|| {
        prior_search_summaries.extend(atomic_combat_search_summaries(
            &outcome,
            &plan,
            facts.as_ref(),
            applied,
            decision,
            &potion_continuation_context,
            &potion_continuation_pressure,
            &combat_victory_continuation,
            &strategic_hp_quality,
        ));
        prior_search_summaries
    });
    let mut output = AtomicCombatSearchSessionOutputV2 {
        progress_steps: applied_steps,
        atomic_combat_search_attempts,
        ..AtomicCombatSearchSessionOutputV2::default()
    };
    if let Some(diagnostic) = combat_capture.and_then(|capture| {
        accepted_high_loss_diagnostic(
            capture,
            plan.profile_id,
            &outcome.trace_annotations,
            applied,
            owner_hp_loss_limit,
        )
    }) {
        output.accepted_high_loss_diagnostics.push(diagnostic);
    }

    let report = (!applied).then(|| {
        session_report(
            &plan,
            status.clone(),
            action_keys,
            facts.as_ref(),
            applied,
            decision,
        )
    });
    Ok(atomic_combat_search_result(status, report, output))
}

fn search_error_result(
    plan: &AtomicCombatSearchSessionPlanV2,
    error: String,
    atomic_combat_search_attempts: Vec<AtomicCombatSearchTraceSummaryV2>,
) -> AtomicCombatSearchSessionResultV2 {
    let status = BranchStatus::AdvanceFailed(error);
    let report = session_report(
        plan,
        status.clone(),
        Vec::new(),
        None,
        false,
        "search_error",
    );
    atomic_combat_search_result(
        status,
        Some(report),
        AtomicCombatSearchSessionOutputV2 {
            atomic_combat_search_attempts,
            ..AtomicCombatSearchSessionOutputV2::default()
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn atomic_combat_search_attempt_summaries(
    session: &RunControlSession,
    attempt: &AtomicCombatSearchAttemptV2,
    plan: &AtomicCombatSearchSessionPlanV2,
    owner_hp_loss_limit: Option<u32>,
    selected: bool,
    decision: &'static str,
    potion_continuation_context: &PotionRunContinuationContextV1,
    potion_continuation_pressure: &PotionContinuationPressureV1,
    combat_victory_continuation: &CombatVictoryContinuationFactsV1,
    strategic_hp_quality: &CombatSearchStrategicHpQualityFactsV1,
) -> Vec<AtomicCombatSearchTraceSummaryV2> {
    let annotations = vec![attempt.trace_annotation(session, plan.profile_id)];
    let facts = candidate_facts(session, &annotations, owner_hp_loss_limit);
    atomic_combat_search_summaries_from_annotations(
        &annotations,
        plan,
        facts.as_ref(),
        selected,
        decision,
        potion_continuation_context,
        potion_continuation_pressure,
        combat_victory_continuation,
        strategic_hp_quality,
    )
}

#[derive(Clone, Copy)]
struct SearchCandidateFacts {
    tier: SearchCandidateTier,
    combat_final_hp: i32,
    run_hp: i32,
    potions_used: u32,
    turns: u32,
}

#[derive(Clone, Copy)]
enum SearchCandidateTier {
    RelaxedCompleteWin,
    ReserveCompliantCompleteWin,
}

impl SearchCandidateTier {
    fn as_str(self) -> &'static str {
        match self {
            Self::RelaxedCompleteWin => "relaxed_complete_win",
            Self::ReserveCompliantCompleteWin => "reserve_compliant_complete_win",
        }
    }
}

fn candidate_facts(
    session: &RunControlSession,
    annotations: &[RunControlTraceAnnotationV1],
    owner_hp_loss_limit: Option<u32>,
) -> Option<SearchCandidateFacts> {
    let best_win =
        atomic_combat_search_trace_summaries(annotations).find_map(|summary| summary.best_win)?;
    let tier = if owner_hp_loss_limit.is_some_and(|limit| best_win.hp_loss.max(0) as u32 > limit) {
        SearchCandidateTier::RelaxedCompleteWin
    } else {
        SearchCandidateTier::ReserveCompliantCompleteWin
    };
    Some(SearchCandidateFacts {
        tier,
        combat_final_hp: best_win.final_hp,
        run_hp: session.visible_player_hp().0,
        potions_used: best_win.potions_used,
        turns: best_win.turns,
    })
}

fn session_decision(applied: bool, facts: Option<&SearchCandidateFacts>) -> &'static str {
    match (applied, facts.map(|facts| facts.tier)) {
        (true, Some(SearchCandidateTier::ReserveCompliantCompleteWin)) => {
            "accepted_reserve_compliant_candidate"
        }
        (true, Some(SearchCandidateTier::RelaxedCompleteWin)) => "accepted_relaxed_candidate",
        (true, None) => "applied_direct_survival_action",
        (false, Some(_)) => "candidate_rejected_by_typed_acceptance",
        (false, None) => "no_accepted_candidate",
    }
}

fn committed_progress_steps(outcome: &RunProgressOutcome) -> Vec<RunProgressStepV1> {
    outcome
        .progress_steps
        .iter()
        .filter(|step| !matches!(step, RunProgressStepV1::Stop(_)))
        .cloned()
        .collect()
}

fn atomic_combat_search_summaries(
    outcome: &RunProgressOutcome,
    plan: &AtomicCombatSearchSessionPlanV2,
    facts: Option<&SearchCandidateFacts>,
    applied: bool,
    decision: &'static str,
    potion_continuation_context: &PotionRunContinuationContextV1,
    potion_continuation_pressure: &PotionContinuationPressureV1,
    combat_victory_continuation: &CombatVictoryContinuationFactsV1,
    strategic_hp_quality: &CombatSearchStrategicHpQualityFactsV1,
) -> Vec<AtomicCombatSearchTraceSummaryV2> {
    atomic_combat_search_summaries_from_annotations(
        &outcome.trace_annotations,
        plan,
        facts,
        applied,
        decision,
        potion_continuation_context,
        potion_continuation_pressure,
        combat_victory_continuation,
        strategic_hp_quality,
    )
}

#[allow(clippy::too_many_arguments)]
fn atomic_combat_search_summaries_from_annotations(
    annotations: &[RunControlTraceAnnotationV1],
    plan: &AtomicCombatSearchSessionPlanV2,
    facts: Option<&SearchCandidateFacts>,
    applied: bool,
    decision: &'static str,
    potion_continuation_context: &PotionRunContinuationContextV1,
    potion_continuation_pressure: &PotionContinuationPressureV1,
    combat_victory_continuation: &CombatVictoryContinuationFactsV1,
    strategic_hp_quality: &CombatSearchStrategicHpQualityFactsV1,
) -> Vec<AtomicCombatSearchTraceSummaryV2> {
    let mut summaries = atomic_combat_search_trace_summaries(annotations).collect::<Vec<_>>();
    for summary in &mut summaries {
        summary.lane = Some(plan.stage.label().to_string());
        summary.profile_id = Some(plan.profile_id.to_string());
        summary.profile_max_nodes = Some(plan.total_nodes);
        summary.profile_wall_ms = Some(plan.total_wall_ms);
        summary.profile_potion_policy = Some(potion_policy_label(plan.potion_policy).to_string());
        summary.profile_max_potions_used = plan.max_potions_used;
        summary.profile_allowed_potion_slots = plan.allowed_potion_slots;
        summary.profile_internal_no_win_rescue_enabled = Some(false);
        summary.engine_fingerprint = Some(plan.semantics_fingerprint.clone());
        summary.atomic_stage_candidate_tier = facts.map(|facts| facts.tier.as_str().to_string());
        summary.atomic_witness_selected = Some(applied);
        summary.atomic_stage_decision = Some(decision.to_string());
        summary.potion_continuation_context = Some(potion_continuation_context.clone());
        summary.potion_continuation_pressure = Some(potion_continuation_pressure.clone());
        summary.combat_victory_continuation = Some(combat_victory_continuation.clone());
        summary.strategic_hp_quality = Some(strategic_hp_quality.clone());
    }
    summaries
}

fn session_report(
    plan: &AtomicCombatSearchSessionPlanV2,
    status: BranchStatus,
    action_keys: Vec<String>,
    facts: Option<&SearchCandidateFacts>,
    applied: bool,
    decision: &'static str,
) -> AtomicCombatSearchSessionReportV2 {
    atomic_combat_search_session_report(AtomicCombatSearchSessionReportInputV2 {
        status,
        profile_id: plan.profile_id,
        max_nodes: plan.total_nodes,
        wall_ms: plan.total_wall_ms,
        potion_policy: plan.potion_policy,
        max_potions_used: plan.max_potions_used,
        allowed_potion_slots: plan.allowed_potion_slots,
        work_quanta: plan
            .search
            .work_quanta
            .iter()
            .map(|quantum| AtomicCombatSearchQuantumReportV2 {
                label: quantum.label,
                additional_nodes: quantum.additional_nodes,
                soft_wall_ms: quantum.soft_wall_ms,
            })
            .collect(),
        action_keys,
        semantics_fingerprint: plan.semantics_fingerprint.clone(),
        candidate_tier: facts.map(|facts| facts.tier.as_str().to_string()),
        applied,
        decision: decision.to_string(),
        combat_final_hp: facts.map(|facts| facts.combat_final_hp),
        run_hp: facts.map(|facts| facts.run_hp),
        potions_used: facts.map(|facts| facts.potions_used),
        turns: facts.map(|facts| facts.turns),
    })
}

fn search_status(session: &RunControlSession, outcome: &RunProgressOutcome) -> BranchStatus {
    if let Some(outcome) = boundary_router::terminal_outcome(session) {
        BranchStatus::Terminal(outcome)
    } else {
        boundary_router::classify_auto_outcome(session, outcome)
    }
}

fn potion_policy_label(
    policy: sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy,
) -> &'static str {
    match policy {
        sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy::Never => "never",
        sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy::All => "all",
        sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy::SemanticBudgeted => {
            "semantic"
        }
    }
}

fn awaiting_auto_boundary(boundary: impl Into<String>, reason: String) -> BranchStatus {
    BranchStatus::AwaitingAuto {
        boundary: boundary.into(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::branch::owner_audit::run_contract::RunObjective;
    use sts_simulator::content::monsters::EnemyId;
    use sts_simulator::content::potions::{Potion, PotionId};
    use sts_simulator::eval::run_control::{
        CombatSearchHpLossLimitV1, CombatVictoryHpCarryoverV1, RunControlConfig, RunControlSession,
        RunProgressOutcome,
    };
    use sts_simulator::state::core::{ActiveCombat, CombatContext, EngineState, RoomCombatContext};
    use sts_simulator::state::map::node::RoomType;

    fn args() -> Args {
        Args {
            seed: 1,
            ascension: 0,
            objective: RunObjective::FirstVictory,
            generations: 1,
            max_branches: 1,
            auto_ops: 1,
            search_nodes: 256,
            search_ms: 1_000,
            rescue_search_nodes: 512,
            rescue_search_ms: 1_000,
            boss_search_nodes: 512,
            boss_search_ms: 1_000,
            wall_ms: None,
            checkpoint_before_atomic_combat_search_session: false,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        }
    }

    fn hallway_session(monster_hp: i32, potions: Vec<Option<Potion>>) -> RunControlSession {
        let mut combat = crate::test_support::blank_test_combat();
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        let plan = sts_simulator::content::monsters::roll_monster_turn_plan(
            &mut combat.rng.ai_rng,
            &monster,
            combat.meta.ascension_level,
            99,
            std::slice::from_ref(&monster),
            &[],
        );
        monster.set_planned_move_id(plan.move_id);
        monster.set_planned_steps(plan.steps);
        monster.set_planned_visible_spec(plan.visible_spec);
        monster.current_hp = monster_hp;
        monster.max_hp = monster_hp;
        combat.entities.monsters = vec![monster];
        combat.entities.potions = potions;
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        session
    }

    fn guaranteed_full_heal_boss_session(
        monster_hp: i32,
        potions: Vec<Option<Potion>>,
    ) -> RunControlSession {
        let mut session = hallway_session(monster_hp, potions);
        let active = session.active_combat.as_mut().expect("active combat");
        active.combat_state.meta.is_boss_fight = true;
        active.context = CombatContext::Room(RoomCombatContext {
            room_type: RoomType::MonsterRoomBoss,
        });
        session.run_state.act_num = 1;
        session.run_state.ascension_level = 0;
        session
    }

    #[test]
    fn stop_records_are_not_misreported_as_committed_search_progress() {
        let outcome = RunProgressOutcome::progress("gap");

        assert!(committed_progress_steps(&outcome).is_empty());
    }

    #[test]
    fn candidate_tier_uses_owner_reserve_without_rejecting_relaxed_win() {
        assert_eq!(
            session_decision(
                true,
                Some(&SearchCandidateFacts {
                    tier: SearchCandidateTier::RelaxedCompleteWin,
                    combat_final_hp: 10,
                    run_hp: 10,
                    potions_used: 0,
                    turns: 5,
                })
            ),
            "accepted_relaxed_candidate"
        );
    }

    #[test]
    fn owner_audit_accepts_a_quality_no_potion_win_before_opening_rescue() {
        let mut session = hallway_session(6, vec![Some(Potion::new(PotionId::BlockPotion, 10))]);
        session.run_state.gold = 57;
        session
            .active_combat
            .as_mut()
            .unwrap()
            .combat_state
            .zones
            .hand = vec![sts_simulator::runtime::combat::CombatCard::new(
            sts_simulator::content::cards::CardId::Strike,
            1,
        )];
        session
            .active_combat
            .as_mut()
            .unwrap()
            .combat_state
            .zones
            .card_uuid_counter = 2;

        let result = run_atomic_combat_search_session_step(&mut session, args())
            .expect("owner combat search");

        assert!(
            session.active_combat.is_none(),
            "the exact no-potion win should resolve combat"
        );
        assert!(session
            .run_state
            .potions
            .first()
            .is_some_and(|potion| potion.as_ref().is_some_and(|potion| {
                potion.id == PotionId::BlockPotion && potion.uuid == 10
            })));
        assert!(result.atomic_combat_search_attempts.iter().any(|summary| {
            summary.lane.as_deref() == Some("no_potion_primary")
                && summary.profile_max_potions_used == Some(0)
                && summary.profile_allowed_potion_slots == Some(0)
        }));
        let continuation = result
            .atomic_combat_search_attempts
            .iter()
            .find_map(|summary| summary.potion_continuation_context.as_ref())
            .expect("owner search should preserve pre-search potion context");
        assert_eq!(continuation.capture_boundary, "before_atomic_combat_search");
        assert_eq!(continuation.inventory.slot_capacity, 1);
        assert_eq!(continuation.inventory.occupied_slots, 1);
        assert!(continuation.inventory.inventory_full);
        let pressure = result
            .atomic_combat_search_attempts
            .iter()
            .find_map(|summary| summary.potion_continuation_pressure.as_ref())
            .expect("owner search should preserve compact potion pressure");
        assert_eq!(pressure.capture_boundary, "before_atomic_combat_search");
        assert_eq!(pressure.inventory.slot_capacity, 1);
        assert_eq!(pressure.inventory.occupied_slots, 1);
        assert_eq!(pressure.shop.current_gold, 57);
        assert_eq!(
            pressure.recovery.current_hp_deficit,
            continuation.max_hp - continuation.current_hp
        );
        assert!(result.atomic_combat_search_attempts.iter().all(|summary| {
            summary
                .combat_victory_continuation
                .as_ref()
                .is_some_and(|facts| {
                    facts.hp_carryover
                        == CombatVictoryHpCarryoverV1::NotGuaranteedByRoomBossActTransition
                })
        }));
        assert!(result.atomic_combat_search_attempts.iter().all(|summary| {
            summary.strategic_hp_quality.as_ref().is_some_and(|facts| {
                facts.entry_current_hp == continuation.current_hp
                    && facts.entry_max_hp == continuation.max_hp
                    && matches!(
                        facts.survival_hp_loss_limit,
                        CombatSearchHpLossLimitV1::Limited { .. }
                    )
                    && matches!(
                        facts.quality_hp_loss_limit,
                        CombatSearchHpLossLimitV1::Limited { .. }
                    )
            })
        }));
    }

    #[test]
    fn owner_audit_preserves_potion_when_boss_victory_guarantees_full_heal() {
        let mut session =
            guaranteed_full_heal_boss_session(6, vec![Some(Potion::new(PotionId::FirePotion, 21))]);
        let combat = &mut session.active_combat.as_mut().unwrap().combat_state;
        combat.zones.hand = vec![sts_simulator::runtime::combat::CombatCard::new(
            sts_simulator::content::cards::CardId::Strike,
            1,
        )];
        combat.zones.card_uuid_counter = 2;

        let result = run_atomic_combat_search_session_step(&mut session, args())
            .expect("full-heal boss search");

        assert!(
            session.active_combat.is_none(),
            "the exact no-potion Boss win should resolve combat"
        );
        assert!(session.run_state.potions.first().is_some_and(|slot| {
            slot.as_ref()
                .is_some_and(|potion| potion.id == PotionId::FirePotion && potion.uuid == 21)
        }));
        assert!(result.atomic_combat_search_attempts.iter().any(|summary| {
            summary.lane.as_deref() == Some("no_potion_primary")
                && summary.profile_max_potions_used == Some(0)
                && summary.profile_allowed_potion_slots == Some(0)
                && summary.atomic_witness_selected == Some(true)
                && summary
                    .best_win
                    .as_ref()
                    .is_some_and(|win| win.potions_used == 0)
        }));
        assert!(result
            .atomic_combat_search_attempts
            .iter()
            .all(|summary| summary.lane.as_deref() != Some("find_any_win")));
        assert!(result.atomic_combat_search_attempts.iter().all(|summary| {
            summary
                .combat_victory_continuation
                .as_ref()
                .is_some_and(|facts| {
                    facts.hp_carryover
                        == CombatVictoryHpCarryoverV1::GuaranteedFullHealBeforeNextDamageBearingDecision
                })
        }));
        assert!(result.atomic_combat_search_attempts.iter().all(|summary| {
            summary.strategic_hp_quality.as_ref().is_some_and(|facts| {
                facts.survival_hp_loss_limit == CombatSearchHpLossLimitV1::Unlimited
                    && facts.quality_hp_loss_limit == CombatSearchHpLossLimitV1::Unlimited
            })
        }));
    }

    #[test]
    fn owner_audit_uses_exact_single_potion_rescue_only_after_no_potion_failure() {
        let mut session = hallway_session(20, vec![Some(Potion::new(PotionId::FirePotion, 11))]);

        let result = run_atomic_combat_search_session_step(&mut session, args())
            .expect("owner potion rescue");

        assert!(
            session.active_combat.is_none(),
            "the exact Fire Potion rescue should resolve combat"
        );
        assert!(session
            .run_state
            .potions
            .first()
            .is_none_or(Option::is_none));
        assert!(result.atomic_combat_search_attempts.iter().any(|summary| {
            summary.lane.as_deref() == Some("no_potion_primary")
                && summary.profile_max_potions_used == Some(0)
                && summary.profile_allowed_potion_slots == Some(0)
                && summary.atomic_witness_selected == Some(false)
        }));
        assert!(result.atomic_combat_search_attempts.iter().any(|summary| {
            summary.lane.as_deref() == Some("find_any_win")
                && summary.profile_max_potions_used == Some(1)
                && summary.profile_allowed_potion_slots == Some(1)
                && summary
                    .best_win
                    .as_ref()
                    .is_some_and(|win| win.potions_used == 1)
        }));
    }

    #[test]
    fn owner_audit_rejects_potion_rescue_below_the_survival_floor() {
        let mut session = hallway_session(26, vec![Some(Potion::new(PotionId::FirePotion, 11))]);
        let combat = &mut session.active_combat.as_mut().unwrap().combat_state;
        combat.entities.player.current_hp = 20;
        combat.entities.player.max_hp = 80;
        combat.zones.hand.clear();
        combat.zones.draw_pile = vec![sts_simulator::runtime::combat::CombatCard::new(
            sts_simulator::content::cards::CardId::Strike,
            1,
        )]
        .into();
        combat.zones.card_uuid_counter = 2;

        let result = run_atomic_combat_search_session_step(&mut session, args())
            .expect("owner potion rescue");

        assert!(
            session.active_combat.is_some(),
            "a win that falls below the survival floor must not resolve combat"
        );
        assert!(session
            .active_combat
            .as_ref()
            .unwrap()
            .combat_state
            .entities
            .potions
            .first()
            .is_some_and(|slot| slot
                .as_ref()
                .is_some_and(|potion| { potion.id == PotionId::FirePotion && potion.uuid == 11 })));
        assert!(result.atomic_combat_search_attempts.iter().any(|summary| {
            summary.lane.as_deref() == Some("find_any_win")
                && summary.atomic_witness_selected == Some(false)
                && summary.atomic_stage_decision.as_deref()
                    == Some("candidate_rejected_by_typed_acceptance")
                && summary
                    .best_win
                    .as_ref()
                    .is_some_and(|win| win.hp_loss > 0 && win.potions_used == 1)
        }));
    }

    #[test]
    fn owner_audit_quality_gate_opens_both_active_slots_and_selects_satisfying_strength() {
        let mut session = hallway_session(
            8,
            vec![
                Some(Potion::new(PotionId::StrengthPotion, 12)),
                Some(Potion::new(PotionId::PowerPotion, 13)),
            ],
        );
        let combat = &mut session.active_combat.as_mut().unwrap().combat_state;
        combat.entities.player.current_hp = 40;
        combat.entities.player.max_hp = 40;
        combat.zones.hand = vec![sts_simulator::runtime::combat::CombatCard::new(
            sts_simulator::content::cards::CardId::Strike,
            1,
        )];
        combat.zones.draw_pile = vec![sts_simulator::runtime::combat::CombatCard::new(
            sts_simulator::content::cards::CardId::Strike,
            2,
        )]
        .into();
        combat.zones.card_uuid_counter = 3;

        let result = run_atomic_combat_search_session_step(&mut session, args())
            .expect("owner quality rescue");

        assert!(
            session.active_combat.is_none(),
            "the bounded Strength rescue should resolve combat"
        );
        assert!(session
            .run_state
            .potions
            .first()
            .is_none_or(Option::is_none));
        assert!(session.run_state.potions.get(1).is_some_and(|potion| {
            potion
                .as_ref()
                .is_some_and(|potion| potion.id == PotionId::PowerPotion && potion.uuid == 13)
        }));
        assert!(result.atomic_combat_search_attempts.iter().any(|summary| {
            summary.lane.as_deref() == Some("no_potion_primary")
                && summary.profile_max_potions_used == Some(0)
                && summary.profile_allowed_potion_slots == Some(0)
                && summary.atomic_witness_selected == Some(false)
        }));
        assert!(result.atomic_combat_search_attempts.iter().any(|summary| {
            summary.lane.as_deref() == Some("improve_verified_win")
                && summary.profile_max_potions_used == Some(1)
                && summary.profile_allowed_potion_slots == Some(0b11)
                && summary
                    .best_win
                    .as_ref()
                    .is_some_and(|win| win.potions_used == 1 && win.final_hp == 40)
        }));
    }

    #[test]
    fn owner_audit_can_select_flexible_attack_potion_when_it_reaches_quality() {
        let mut session = hallway_session(8, vec![Some(Potion::new(PotionId::AttackPotion, 15))]);
        let combat = &mut session.active_combat.as_mut().unwrap().combat_state;
        combat.entities.player.current_hp = 40;
        combat.entities.player.max_hp = 40;
        combat.zones.hand = vec![sts_simulator::runtime::combat::CombatCard::new(
            sts_simulator::content::cards::CardId::Strike,
            1,
        )];
        combat.zones.draw_pile = vec![sts_simulator::runtime::combat::CombatCard::new(
            sts_simulator::content::cards::CardId::Strike,
            2,
        )]
        .into();
        combat.zones.card_uuid_counter = 3;

        let result = run_atomic_combat_search_session_step(&mut session, args())
            .expect("owner flexible rescue");

        assert!(
            session.active_combat.is_none(),
            "the quality-reaching Attack Potion line should resolve combat"
        );
        assert_eq!(session.visible_player_hp().0, 40);
        assert!(session
            .run_state
            .potions
            .first()
            .is_none_or(Option::is_none));
        assert!(result.atomic_combat_search_attempts.iter().any(|summary| {
            summary.lane.as_deref() == Some("improve_verified_win")
                && summary.profile_allowed_potion_slots == Some(1)
                && summary.atomic_witness_selected == Some(true)
                && summary
                    .best_win
                    .as_ref()
                    .is_some_and(|win| win.potions_used == 1 && win.final_hp == 40)
        }));
    }

    #[test]
    fn owner_audit_falls_back_to_exact_no_potion_win_when_spend_still_misses_quality() {
        let mut session = hallway_session(8, vec![Some(Potion::new(PotionId::WeakenPotion, 14))]);
        let combat = &mut session.active_combat.as_mut().unwrap().combat_state;
        combat.entities.player.current_hp = 20;
        combat.entities.player.max_hp = 20;
        combat.zones.hand = vec![sts_simulator::runtime::combat::CombatCard::new(
            sts_simulator::content::cards::CardId::Strike,
            1,
        )];
        combat.zones.draw_pile = vec![sts_simulator::runtime::combat::CombatCard::new(
            sts_simulator::content::cards::CardId::Strike,
            2,
        )]
        .into();
        combat.zones.card_uuid_counter = 3;

        let result = run_atomic_combat_search_session_step(&mut session, args())
            .expect("owner quality fallback");

        assert!(
            session.active_combat.is_none(),
            "the protected exact no-potion win should still resolve combat"
        );
        assert_eq!(session.visible_player_hp().0, 9);
        assert!(session.run_state.potions.first().is_some_and(|potion| {
            potion
                .as_ref()
                .is_some_and(|potion| potion.id == PotionId::WeakenPotion && potion.uuid == 14)
        }));
        assert!(result.atomic_combat_search_attempts.iter().any(|summary| {
            summary.lane.as_deref() == Some("no_potion_primary")
                && summary.atomic_witness_selected == Some(true)
                && summary.atomic_stage_decision.as_deref()
                    == Some("accepted_protected_no_potion_incumbent")
                && summary
                    .best_win
                    .as_ref()
                    .is_some_and(|win| win.potions_used == 0 && win.final_hp == 9)
        }));
        assert!(result.atomic_combat_search_attempts.iter().any(|summary| {
            summary.lane.as_deref() == Some("improve_verified_win")
                && summary.atomic_witness_selected == Some(false)
                && summary.atomic_stage_decision.as_deref()
                    == Some("candidate_rejected_by_potion_quality_gate")
                && summary
                    .best_win
                    .as_ref()
                    .is_some_and(|win| win.potions_used == 1 && win.final_hp > 9)
        }));
    }
}
