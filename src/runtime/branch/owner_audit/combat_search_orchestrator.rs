use sts_simulator::ai::potion_continuation_context_v1::{
    potion_run_continuation_context_v1, PotionRunContinuationContextV1,
};
use sts_simulator::eval::run_control::{
    combat_search_trace_summaries, CombatSearchTraceSummary, OraclePotionRescueKindV1,
    RunControlCombatSearchRejection, RunControlHpLossLimit, RunControlSession,
    RunControlTraceAnnotationV1, RunProgressOutcome, RunProgressStepV1,
};

use super::accepted_high_loss_diagnostic::{accepted_high_loss_diagnostic, capture_active_combat};
use super::combat_search_report::{
    combat_search_session_report, CombatSearchQuantumReport, CombatSearchSessionReport,
    CombatSearchSessionReportInput,
};
use super::combat_search_session_output::CombatSearchSessionOutput;
use super::combat_search_session_plan::{
    canonical_combat_search_session_plan, potion_conserving_primary_search_session_plan,
    potion_conserving_refinement_search_session_plan, CombatSearchSessionPlan,
};
use super::combat_search_session_result::{combat_search_result, CombatSearchSessionResult};
use super::combat_search_survival::owner_audit_hp_loss_limit;
use super::combat_search_trace_actions::complete_search_action_keys;
use super::{boundary_router, Args, BranchStatus};

pub(super) fn run_combat_search_session_step(
    session: &mut RunControlSession,
    args: Args,
) -> Result<CombatSearchSessionResult, String> {
    let canonical_plan = canonical_combat_search_session_plan(session, args);
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
        return Ok(combat_search_result(
            status,
            Some(report),
            CombatSearchSessionOutput::default(),
        ));
    }

    let active_combat = session
        .active_combat
        .as_ref()
        .ok_or_else(|| "combat search session has no active combat".to_string())?;
    let potion_continuation_context =
        potion_run_continuation_context_v1(&session.run_state, &active_combat.combat_state);
    let combat_capture = capture_active_combat(session)?;
    let owner_hp_loss_limit = match owner_audit_hp_loss_limit(session) {
        RunControlHpLossLimit::Limit(limit) => Some(limit),
        RunControlHpLossLimit::Unlimited => None,
    };
    let primary_plan = potion_conserving_primary_search_session_plan(session, args);
    let staged = primary_plan.is_some();
    let mut plan = primary_plan.unwrap_or(canonical_plan);
    let mut outcome = match session.apply_combat_search(plan.search.clone()) {
        Ok(outcome) => outcome,
        Err(error) => {
            let status = BranchStatus::AdvanceFailed(error);
            let report = session_report(
                &plan,
                status.clone(),
                Vec::new(),
                None,
                false,
                "search_error",
            );
            return Ok(combat_search_result(
                status,
                Some(report),
                CombatSearchSessionOutput::default(),
            ));
        }
    };
    let mut prior_search_summaries = Vec::new();
    if staged && committed_progress_steps(&outcome).is_empty() {
        let rescue_kind = if outcome_has_verified_win(&outcome) {
            OraclePotionRescueKindV1::ImproveVerifiedWin
        } else {
            OraclePotionRescueKindV1::FindAnyWin
        };
        if let Some(refinement) =
            potion_conserving_refinement_search_session_plan(session, args, rescue_kind)
        {
            let primary_facts =
                candidate_facts(session, &outcome.trace_annotations, owner_hp_loss_limit);
            let primary_decision = session_decision(false, primary_facts.as_ref());
            prior_search_summaries.extend(combat_search_summaries(
                &outcome,
                &plan,
                primary_facts.as_ref(),
                false,
                primary_decision,
                &potion_continuation_context,
            ));
            plan = refinement;
            outcome = match session.apply_combat_search(plan.search.clone()) {
                Ok(outcome) => outcome,
                Err(error) => {
                    let status = BranchStatus::AdvanceFailed(error);
                    let report = session_report(
                        &plan,
                        status.clone(),
                        Vec::new(),
                        None,
                        false,
                        "search_error",
                    );
                    return Ok(combat_search_result(
                        status,
                        Some(report),
                        CombatSearchSessionOutput {
                            combat_search: prior_search_summaries,
                            ..CombatSearchSessionOutput::default()
                        },
                    ));
                }
            };
        }
    }
    let status = search_status(session, &outcome);
    let action_keys = complete_search_action_keys(&outcome.trace_annotations);
    let applied_steps = committed_progress_steps(&outcome);
    let applied = !applied_steps.is_empty();
    let facts = candidate_facts(session, &outcome.trace_annotations, owner_hp_loss_limit);
    let decision = session_decision(applied, facts.as_ref());

    let mut output = CombatSearchSessionOutput::default();
    output.progress_steps = applied_steps;
    output.combat_search = prior_search_summaries;
    output.combat_search.extend(combat_search_summaries(
        &outcome,
        &plan,
        facts.as_ref(),
        applied,
        decision,
        &potion_continuation_context,
    ));
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
    Ok(combat_search_result(status, report, output))
}

fn outcome_has_verified_win(outcome: &RunProgressOutcome) -> bool {
    matches!(
        outcome.combat_search_rejection,
        Some(RunControlCombatSearchRejection::HpLossLimitExceeded)
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
        combat_search_trace_summaries(annotations).find_map(|summary| summary.best_win)?;
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

fn combat_search_summaries(
    outcome: &RunProgressOutcome,
    plan: &CombatSearchSessionPlan,
    facts: Option<&SearchCandidateFacts>,
    applied: bool,
    decision: &'static str,
    potion_continuation_context: &PotionRunContinuationContextV1,
) -> Vec<CombatSearchTraceSummary> {
    let mut summaries =
        combat_search_trace_summaries(&outcome.trace_annotations).collect::<Vec<_>>();
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
        summary.portfolio_candidate_tier = facts.map(|facts| facts.tier.as_str().to_string());
        summary.portfolio_selected = Some(applied);
        summary.portfolio_decision = Some(decision.to_string());
        summary.potion_continuation_context = Some(potion_continuation_context.clone());
    }
    summaries
}

fn session_report(
    plan: &CombatSearchSessionPlan,
    status: BranchStatus,
    action_keys: Vec<String>,
    facts: Option<&SearchCandidateFacts>,
    applied: bool,
    decision: &'static str,
) -> CombatSearchSessionReport {
    combat_search_session_report(CombatSearchSessionReportInput {
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
            .map(|quantum| CombatSearchQuantumReport {
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
    use sts_simulator::eval::run_control::RunProgressOutcome;
    use sts_simulator::eval::run_control::{RunControlConfig, RunControlSession};
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
            checkpoint_before_combat_portfolio: false,
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
    fn only_a_clean_line_rejected_by_quality_counts_as_a_verified_win() {
        let mut outcome = RunProgressOutcome::progress("quality miss");
        outcome.combat_search_rejection =
            Some(RunControlCombatSearchRejection::HpLossLimitExceeded);
        assert!(outcome_has_verified_win(&outcome));

        outcome.combat_search_rejection =
            Some(RunControlCombatSearchRejection::DirtyWinningCandidateRejected);
        assert!(!outcome_has_verified_win(&outcome));
        outcome.combat_search_rejection =
            Some(RunControlCombatSearchRejection::NoCompleteWinningCandidate);
        assert!(!outcome_has_verified_win(&outcome));
    }

    #[test]
    fn owner_audit_accepts_a_quality_no_potion_win_before_opening_rescue() {
        let mut session = hallway_session(6, vec![Some(Potion::new(PotionId::BlockPotion, 10))]);
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

        let result =
            run_combat_search_session_step(&mut session, args()).expect("owner combat search");

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
        assert!(result.combat_search.iter().any(|summary| {
            summary.lane.as_deref() == Some("no_potion_primary")
                && summary.profile_max_potions_used == Some(0)
                && summary.profile_allowed_potion_slots == Some(0)
        }));
        let continuation = result
            .combat_search
            .iter()
            .find_map(|summary| summary.potion_continuation_context.as_ref())
            .expect("owner search should preserve pre-search potion context");
        assert_eq!(continuation.capture_boundary, "before_combat_search");
        assert_eq!(continuation.inventory.slot_capacity, 1);
        assert_eq!(continuation.inventory.occupied_slots, 1);
        assert!(continuation.inventory.inventory_full);
    }

    #[test]
    fn owner_audit_uses_exact_single_potion_rescue_only_after_no_potion_failure() {
        let mut session = hallway_session(20, vec![Some(Potion::new(PotionId::FirePotion, 11))]);

        let result =
            run_combat_search_session_step(&mut session, args()).expect("owner potion rescue");

        assert!(
            session.active_combat.is_none(),
            "the exact Fire Potion rescue should resolve combat"
        );
        assert!(session
            .run_state
            .potions
            .first()
            .is_none_or(Option::is_none));
        assert!(result.combat_search.iter().any(|summary| {
            summary.lane.as_deref() == Some("no_potion_primary")
                && summary.profile_max_potions_used == Some(0)
                && summary.profile_allowed_potion_slots == Some(0)
                && summary.portfolio_selected == Some(false)
        }));
        assert!(result.combat_search.iter().any(|summary| {
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
    fn owner_audit_verified_win_opens_strength_but_preserves_power_potion() {
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

        let result =
            run_combat_search_session_step(&mut session, args()).expect("owner quality rescue");

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
        assert!(result.combat_search.iter().any(|summary| {
            summary.lane.as_deref() == Some("no_potion_primary")
                && summary.profile_max_potions_used == Some(0)
                && summary.profile_allowed_potion_slots == Some(0)
                && summary.portfolio_selected == Some(false)
        }));
        assert!(result.combat_search.iter().any(|summary| {
            summary.lane.as_deref() == Some("improve_verified_win")
                && summary.profile_max_potions_used == Some(1)
                && summary.profile_allowed_potion_slots == Some(1)
                && summary
                    .best_win
                    .as_ref()
                    .is_some_and(|win| win.potions_used == 1 && win.final_hp == 40)
        }));
    }
}
