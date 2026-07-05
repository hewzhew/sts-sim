use sts_simulator::ai::strategy::campfire_upgrade_quality::{
    rank_campfire_upgrades, should_rest_before_smith, CampfireUpgradeTier,
};
use sts_simulator::content::cards::{get_card_definition, is_starter_basic, CardId, CardType};
use sts_simulator::eval::run_control::{
    DecisionCandidateKey, DecisionSurface, RunControlCommand, RunControlSession,
};
use sts_simulator::state::core::{CampfireChoice, ClientInput, EngineState};

use super::boss_relic_owner::boss_relic_owner_choices;
use super::neow_owner::neow_owner_decision;
use super::owner_model::{
    ChoiceAnnotation, OwnerChoice, OwnerChoiceExpansion, OwnerDecision, OwnerRoutine,
};
use super::reward_shop_boss_owner::{card_reward_owner_choices, shop_tiny_owner_choices};
use super::run_choice_owner::run_choice_owner_decision;
use super::Owner;

pub(super) fn owner_decision(
    session: &RunControlSession,
    owner: Owner,
    surface: &DecisionSurface,
) -> OwnerDecision {
    match owner {
        Owner::NeowStart => neow_owner_decision(session, surface),
        Owner::CardReward => OwnerDecision::Candidates(card_reward_owner_choices(session, surface)),
        Owner::BossRelic => OwnerDecision::Candidates(boss_relic_owner_choices(session, surface)),
        Owner::ShopTiny => OwnerDecision::Candidates(shop_tiny_owner_choices(session, surface)),
        Owner::Event(_) => event_owner_decision(session, surface),
        Owner::RewardTiny => reward_tiny_owner_decision(surface),
        Owner::Campfire => campfire_owner_decision(session, surface),
        Owner::RunChoice => run_choice_owner_decision(session),
    }
}

pub(super) fn executable_choices(surface: &DecisionSurface) -> Vec<OwnerChoice> {
    executable_choices_with_cancel(surface, false)
}

pub(super) fn executable_choices_including_cancel(surface: &DecisionSurface) -> Vec<OwnerChoice> {
    executable_choices_with_cancel(surface, true)
}

fn event_owner_decision(session: &RunControlSession, surface: &DecisionSurface) -> OwnerDecision {
    match sts_simulator::content::events::owner_policy::event_owner_policy_action(
        &session.engine_state,
        &session.run_state,
    ) {
        Ok(sts_simulator::content::events::owner_policy::EventOwnerAction::ChooseOption(
            selector,
        )) => visible_event_option_decision(session, surface, &selector),
        Ok(sts_simulator::content::events::owner_policy::EventOwnerAction::SubmitSelection(
            resolution,
        )) => visible_input_decision(surface, ClientInput::SubmitSelection(resolution)),
        Err(err) => OwnerDecision::Gap(format!("{err:?}")),
    }
}

fn visible_event_option_decision(
    session: &RunControlSession,
    surface: &DecisionSurface,
    selector: &sts_simulator::content::events::owner_policy::EventOwnerOptionSelector,
) -> OwnerDecision {
    let Some(event) = session.run_state.event_state.as_ref() else {
        return OwnerDecision::Gap("event owner requires event_state".to_string());
    };
    let options = sts_simulator::engine::event_handler::get_event_options(&session.run_state);
    let matches = surface
        .view
        .candidates
        .iter()
        .filter_map(|candidate| {
            let Some(DecisionCandidateKey::EventOption {
                event_id,
                screen,
                option_index,
                ..
            }) = candidate.key
            else {
                return None;
            };
            if event_id != event.id || screen != event.current_screen {
                return None;
            }
            let option = options.get(option_index)?;
            if option.ui.disabled || !selector.matches(option_index, &option.semantics) {
                return None;
            }
            candidate.action.executable_command()
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [command] => OwnerDecision::Routine(OwnerRoutine::Command(command.clone())),
        [] => OwnerDecision::Gap(format!("event selector {selector:?} has no visible option")),
        _ => OwnerDecision::Gap(format!(
            "event selector {selector:?} matched {} visible options",
            matches.len()
        )),
    }
}

fn reward_tiny_owner_decision(surface: &DecisionSurface) -> OwnerDecision {
    if let Some(command) = surface
        .view
        .candidates
        .iter()
        .find(|candidate| {
            matches!(
                candidate.key,
                Some(DecisionCandidateKey::CardRewardOpen { .. })
            )
        })
        .and_then(|candidate| candidate.action.executable_command())
    {
        return OwnerDecision::Routine(OwnerRoutine::Command(command));
    }
    OwnerDecision::Routine(OwnerRoutine::RewardTinyAutomation)
}

fn campfire_owner_decision(
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

fn visible_input_decision(surface: &DecisionSurface, input: ClientInput) -> OwnerDecision {
    if surface
        .visible_executable_inputs
        .iter()
        .any(|visible| visible == &input)
    {
        OwnerDecision::Routine(OwnerRoutine::Command(RunControlCommand::Input(input)))
    } else {
        OwnerDecision::Gap(format!("routine input {input:?} is not visible"))
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

fn executable_choices_with_cancel(
    surface: &DecisionSurface,
    include_cancel: bool,
) -> Vec<OwnerChoice> {
    surface
        .view
        .candidates
        .iter()
        .filter_map(|candidate| {
            let action = candidate.action.executable_command()?;
            if !include_owner_choice_command(&action, include_cancel) {
                return None;
            }
            Some(OwnerChoice {
                key: candidate.key.clone(),
                action,
                label: candidate.label.clone(),
                annotation: ChoiceAnnotation::None,
                expansion: OwnerChoiceExpansion::AutoAllowed,
            })
        })
        .collect()
}

fn include_owner_choice_command(command: &RunControlCommand, include_cancel: bool) -> bool {
    include_cancel || !is_navigation_only_command(command)
}

fn is_navigation_only_command(command: &RunControlCommand) -> bool {
    matches!(command, RunControlCommand::Input(ClientInput::Cancel))
}
