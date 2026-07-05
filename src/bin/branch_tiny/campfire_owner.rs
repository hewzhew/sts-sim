use sts_simulator::ai::strategy::campfire_upgrade_quality::{
    rank_campfire_upgrades, should_rest_before_smith, CampfireUpgradeTier,
};
use sts_simulator::content::cards::{get_card_definition, is_starter_basic, CardId, CardType};
use sts_simulator::eval::run_control::{DecisionSurface, RunControlSession};
use sts_simulator::state::core::{CampfireChoice, ClientInput, EngineState};

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
    let has_rest = options.contains(&CampfireChoice::Rest);
    let has_smith = options
        .iter()
        .any(|choice| matches!(choice, CampfireChoice::Smith(_)));

    if has_rest
        && (!has_smith
            || should_rest_before_smith(session.run_state.current_hp, session.run_state.max_hp))
    {
        return Ok(CampfireChoice::Rest);
    }
    if let Some(choice) = best_campfire_toke(session, surface, options) {
        return Ok(choice);
    }
    if has_smith {
        let ranked = rank_campfire_upgrades(&session.run_state.master_deck);
        if let Some(best) = ranked
            .iter()
            .find(|target| target.tier >= CampfireUpgradeTier::Low)
            .or_else(|| ranked.first())
        {
            return Ok(CampfireChoice::Smith(best.deck_index));
        }
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
