use sts_combat_planner::{
    decide_combat_option, replay_turn_option, CombatDecisionRoot, CombatEvaluationContext,
    CombatPlannerAgendaConfig, CombatPlannerAgendaQuantum, CombatPlannerAgendaSession,
    CombatPlannerDecision, CombatPlannerDecisionBasis, CombatPlannerDecisionResult,
    CombatPlanningQuantum, CompleteTurnOptionBoundary, OptionProspectId, ReplayLimits,
    TurnOptionGenerationStatus, TurnOptionGeneratorConfig, TurnOptionGeneratorSession,
};

use crate::ai::combat_state_key::combat_exact_state_hash_v1;
use crate::sim::combat::EngineCombatStepper;

use super::session::RunControlSession;
use super::transition_report::ActionResultChange;

fn plan_one_bounded_option(session: &RunControlSession) -> Result<CombatPlannerDecision, String> {
    let position = session.current_active_combat_position()?;
    let root = CombatDecisionRoot::new(position).map_err(|error| format!("{error:?}"))?;
    let mut agenda = CombatPlannerAgendaSession::new(root, CombatPlannerAgendaConfig::default());
    let report = agenda.advance(
        &EngineCombatStepper,
        CombatPlannerAgendaQuantum::deterministic(128, 256, 2_048),
    );
    match decide_combat_option(&agenda) {
        CombatPlannerDecisionResult::Selected(decision) => Ok(decision),
        CombatPlannerDecisionResult::Deferred(deferral) => Err(format!(
            "planner deferred after {:?}: {:?}",
            report.status, deferral.gaps
        )),
    }
}

fn apply_on_clone(
    session: &RunControlSession,
    decision: &CombatPlannerDecision,
) -> Result<RunControlSession, String> {
    let start = session.current_active_combat_position()?;
    let root = CombatDecisionRoot::new(start).map_err(|error| format!("{error:?}"))?;
    if root.exact_state_hash() != decision.root_exact_state_hash {
        return Err("planner decision root no longer matches run-control".to_string());
    }
    replay_turn_option(
        &root,
        &decision.selected_option,
        &EngineCombatStepper,
        ReplayLimits::deterministic(decision.selected_option.engine_steps()),
    )
    .map_err(|error| format!("planner option replay failed: {error:?}"))?;

    let mut trial = session.clone();
    let mut combat_ended = false;
    for (index, action) in decision.selected_option.actions().iter().enumerate() {
        let outcome = trial.apply_combat_resolution_input(action.input.clone())?;
        combat_ended |= outcome.action_result.as_ref().is_some_and(|result| {
            result
                .changes
                .iter()
                .any(|change| matches!(change, ActionResultChange::CombatEnded))
        });
        match trial.current_active_combat_position() {
            Ok(position) => {
                let actual = combat_exact_state_hash_v1(&position.engine, &position.combat);
                if actual != action.expected_successor_hash {
                    return Err(format!("live successor mismatch at action {index}"));
                }
            }
            Err(_) if index + 1 == decision.selected_option.actions().len() => {}
            Err(error) => return Err(format!("combat vanished before option end: {error}")),
        }
    }

    match decision.selected_option.boundary() {
        CompleteTurnOptionBoundary::NextPlayerTurn => {
            let final_position = trial.current_active_combat_position()?;
            let actual = combat_exact_state_hash_v1(&final_position.engine, &final_position.combat);
            if actual != decision.selected_option.exact_successor_hash() {
                return Err("live final successor does not match planner option".to_string());
            }
        }
        CompleteTurnOptionBoundary::TerminalWin
            if combat_ended && trial.active_combat.is_none() => {}
        boundary => {
            return Err(format!(
                "unsupported committed planner boundary: {boundary:?}"
            ))
        }
    }
    Ok(trial)
}

fn generated_decision(
    session: &RunControlSession,
    boundary: CompleteTurnOptionBoundary,
    basis: CombatPlannerDecisionBasis,
) -> Result<CombatPlannerDecision, String> {
    let root = CombatDecisionRoot::new(session.current_active_combat_position()?)
        .map_err(|error| format!("{error:?}"))?;
    let mut generator =
        TurnOptionGeneratorSession::new(root.clone(), TurnOptionGeneratorConfig::default());
    let report = generator.advance(
        &EngineCombatStepper,
        CombatPlanningQuantum::deterministic(100_000, 1_000_000),
    );
    if !matches!(
        report.status,
        TurnOptionGenerationStatus::Complete | TurnOptionGenerationStatus::PartialWithMechanicsGaps
    ) {
        return Err(format!("option generation incomplete: {:?}", report.status));
    }
    let option = generator
        .completed_options()
        .iter()
        .find(|option| option.boundary() == boundary)
        .cloned()
        .ok_or_else(|| format!("no generated option for {boundary:?}"))?;
    Ok(CombatPlannerDecision {
        root_exact_state_hash: root.exact_state_hash().to_owned(),
        evaluation_context: CombatEvaluationContext::ORACLE_EXACT_ONE_TURN,
        selected_prospect_id: OptionProspectId(0),
        selected_option: option,
        nondominated_alternatives: Vec::new(),
        unresolved_gaps: Vec::new(),
        basis,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{ActiveCombat, CombatContext, EngineState, RoomCombatContext};
    use crate::state::map::node::RoomType;

    fn session_with_active_combat(
        mut combat: crate::runtime::combat::CombatState,
    ) -> RunControlSession {
        let mut monster = crate::test_support::planned_monster(EnemyId::JawWorm, 1);
        monster.move_state.planned_visible_spec =
            Some(crate::runtime::monster_move::MonsterMoveSpec::Unknown);
        combat.entities.monsters = vec![monster];
        let mut session = RunControlSession::new(super::super::RunControlConfig::default());
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
    fn real_next_turn_option_replays_and_applies_only_on_a_clone() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.turn.energy = 0;
        combat.zones.hand.clear();
        combat.zones.draw_pile.clear();
        combat.zones.discard_pile.clear();
        combat.turn.mark_skip_monster_turn_pending();
        let session = session_with_active_combat(combat);
        let before = session.current_active_combat_position().unwrap();

        let decision = plan_one_bounded_option(&session).unwrap();
        assert_eq!(
            decision.selected_option.boundary(),
            CompleteTurnOptionBoundary::NextPlayerTurn
        );
        let trial = apply_on_clone(&session, &decision).unwrap();

        assert_eq!(session.current_active_combat_position().unwrap(), before);
        assert_eq!(
            combat_exact_state_hash_v1(
                &trial.current_active_combat_position().unwrap().engine,
                &trial.current_active_combat_position().unwrap().combat,
            ),
            decision.selected_option.exact_successor_hash()
        );
    }

    #[test]
    fn real_terminal_win_maps_to_run_control_combat_end() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.turn.energy = 1;
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 11)];
        combat.zones.draw_pile.clear();
        combat.zones.discard_pile.clear();
        combat.turn.mark_skip_monster_turn_pending();
        let mut session = session_with_active_combat(combat);
        let monster = &mut session
            .active_combat
            .as_mut()
            .unwrap()
            .combat_state
            .entities
            .monsters[0];
        monster.current_hp = 1;
        monster.max_hp = 1;

        let decision = generated_decision(
            &session,
            CompleteTurnOptionBoundary::TerminalWin,
            CombatPlannerDecisionBasis::VerifiedTerminalWin,
        )
        .unwrap();
        assert_eq!(
            decision.selected_option.boundary(),
            CompleteTurnOptionBoundary::TerminalWin
        );
        let trial = apply_on_clone(&session, &decision).unwrap();

        assert!(session.active_combat.is_some());
        assert!(trial.active_combat.is_none());
    }
}
