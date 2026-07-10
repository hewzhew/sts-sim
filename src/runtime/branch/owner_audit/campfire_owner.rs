use sts_simulator::ai::campfire_policy_v1::{
    build_campfire_decision_context_v1, plan_campfire_decision_v1, CampfirePolicyActionV1,
    CampfirePolicyConfigV1,
};
use sts_simulator::content::cards::{get_card_definition, is_starter_basic, CardId, CardType};
use sts_simulator::eval::run_control::{DecisionSurface, RunControlSession};
use sts_simulator::state::core::{CampfireChoice, ClientInput, EngineState};
use sts_simulator::state::run::RunState;

use super::owner_commands::visible_input_decision;
use super::owner_model::{OwnerDecision, OwnerRoutine};

pub(super) fn campfire_owner_decision(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> OwnerDecision {
    if !matches!(session.engine_state, EngineState::Campfire) {
        return OwnerDecision::Gap("Campfire owner requires Campfire engine state".to_string());
    }
    let options =
        sts_simulator::engine::campfire_handler::get_available_options(&session.run_state);
    if options.is_empty() {
        return OwnerDecision::Routine(OwnerRoutine::AdvanceEmptyCampfire);
    }
    match choose_campfire_owner_action(session, surface, &options) {
        Ok(choice) => visible_input_decision(surface, ClientInput::CampfireOption(choice)),
        Err(err) => OwnerDecision::Gap(err),
    }
}

fn choose_campfire_owner_action(
    session: &RunControlSession,
    surface: &DecisionSurface,
    options: &[CampfireChoice],
) -> Result<CampfireChoice, String> {
    if let Some(choice) = strategic_rest_or_smith_choice(&session.run_state, options) {
        return Ok(choice);
    }
    if let Some(choice) = best_campfire_toke(session, surface, options) {
        return Ok(choice);
    }
    for fallback in [
        CampfireChoice::Dig,
        CampfireChoice::Lift,
        CampfireChoice::Recall,
        CampfireChoice::Rest,
    ] {
        if options.contains(&fallback) {
            return Ok(fallback);
        }
    }
    Err("Campfire owner found no policy action".to_string())
}

fn strategic_rest_or_smith_choice(
    run_state: &RunState,
    options: &[CampfireChoice],
) -> Option<CampfireChoice> {
    let context = build_campfire_decision_context_v1(run_state, options.to_vec());
    let decision = plan_campfire_decision_v1(&context, &CampfirePolicyConfigV1::default());
    match decision.action {
        CampfirePolicyActionV1::Rest { .. } => Some(CampfireChoice::Rest),
        CampfirePolicyActionV1::Smith { deck_index, .. } => Some(CampfireChoice::Smith(deck_index)),
        CampfirePolicyActionV1::Stop { .. } => None,
    }
}

fn best_campfire_toke(
    session: &RunControlSession,
    surface: &DecisionSurface,
    options: &[CampfireChoice],
) -> Option<CampfireChoice> {
    if !options
        .iter()
        .any(|choice| matches!(choice, CampfireChoice::Toke(_)))
    {
        return None;
    }
    surface
        .visible_executable_inputs
        .iter()
        .filter_map(|input| {
            let ClientInput::CampfireOption(CampfireChoice::Toke(index)) = input else {
                return None;
            };
            session
                .run_state
                .master_deck
                .get(*index)
                .map(|card| (*index, card.id))
        })
        .min_by_key(|(_, card)| campfire_toke_rank(*card))
        .map(|(index, _)| CampfireChoice::Toke(index))
}

fn campfire_toke_rank(card: CardId) -> u8 {
    let definition = get_card_definition(card);
    match definition.card_type {
        CardType::Curse => 0,
        CardType::Status => 1,
        _ if is_starter_basic(card) => 2,
        _ => 9,
    }
}

#[cfg(test)]
mod tests {
    use super::super::owner_model::{OwnerDecision, OwnerRoutine};
    use super::campfire_owner_decision;
    use sts_simulator::ai::campfire_policy_v1::{
        build_campfire_decision_context_v1, plan_campfire_decision_v1, CampfirePolicyActionV1,
        CampfirePolicyConfigV1,
    };
    use sts_simulator::content::cards::CardId;
    use sts_simulator::content::relics::{RelicId, RelicState};
    use sts_simulator::eval::run_control::{
        build_decision_surface, RunControlCommand, RunControlConfig, RunControlSession,
    };
    use sts_simulator::runtime::combat::CombatCard;
    use sts_simulator::state::core::{CampfireChoice, ClientInput, EngineState};

    #[test]
    fn owner_rest_or_smith_choice_matches_strategic_policy() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::Campfire;
        session.run_state.current_hp = session.run_state.max_hp;
        session.run_state.master_deck = vec![
            CombatCard::new(CardId::TrueGrit, 1),
            CombatCard::new(CardId::FiendFire, 2),
        ];
        let options =
            sts_simulator::engine::campfire_handler::get_available_options(&session.run_state);
        let context = build_campfire_decision_context_v1(&session.run_state, options);
        let expected = match plan_campfire_decision_v1(&context, &CampfirePolicyConfigV1::default())
            .action
        {
            CampfirePolicyActionV1::Rest { .. } => CampfireChoice::Rest,
            CampfirePolicyActionV1::Smith { deck_index, .. } => CampfireChoice::Smith(deck_index),
            CampfirePolicyActionV1::Stop { reason } => {
                panic!("test requires an executable strategic action: {reason}")
            }
        };

        assert_eq!(owner_choice(&session), expected);
    }

    #[test]
    fn policy_stop_preserves_visible_owner_fallback() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::Campfire;
        session.run_state.current_hp = session.run_state.max_hp;
        session.run_state.relics.clear();
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::FusionHammer));
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::Shovel));

        assert_eq!(owner_choice(&session), CampfireChoice::Dig);
    }

    fn owner_choice(session: &RunControlSession) -> CampfireChoice {
        let surface = build_decision_surface(session);
        match campfire_owner_decision(session, &surface) {
            OwnerDecision::Routine(OwnerRoutine::Command(RunControlCommand::Input(
                ClientInput::CampfireOption(choice),
            ))) => choice,
            _ => panic!("expected visible campfire input"),
        }
    }
}
