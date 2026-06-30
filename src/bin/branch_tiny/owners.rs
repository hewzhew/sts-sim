use sts_simulator::ai::strategy::boss_relic_admission::BossRelicAdmission;
use sts_simulator::ai::strategy::campfire_upgrade_quality::{
    rank_campfire_upgrades, should_rest_before_smith, CampfireUpgradeTier,
};
use sts_simulator::ai::strategy::decision_pipeline::{CandidateEvaluation, CleanupTarget};
use sts_simulator::ai::strategy::reward_admission::RewardAdmission;
use sts_simulator::content::cards::{get_card_definition, is_starter_basic, CardId, CardType};
use sts_simulator::eval::run_control::{
    DecisionCandidateKey, DecisionSurface, RunControlCommand, RunControlSession,
};
use sts_simulator::state::core::{CampfireChoice, ClientInput, EngineState};

use super::reward_shop_boss_owner::{
    boss_relic_owner_choices, card_reward_owner_choices, shop_tiny_owner_choices,
};
use super::Owner;

pub(super) type DecisionKey = DecisionCandidateKey;

pub(super) enum OwnerDecision {
    Candidates(Vec<OwnerChoice>),
    Routine(OwnerRoutine),
    Gap(String),
}

pub(super) enum OwnerRoutine {
    Command(RunControlCommand),
    RewardTinyAutomation,
    AdvanceEmptyCampfire,
}

#[derive(Clone)]
pub(super) struct OwnerChoice {
    pub(super) key: Option<DecisionKey>,
    pub(super) action: RunControlCommand,
    pub(super) label: String,
    pub(super) annotation: ChoiceAnnotation,
    pub(super) expansion: OwnerChoiceExpansion,
}

#[derive(Clone)]
pub(super) enum ChoiceAnnotation {
    None,
    Candidate(OwnerCandidateDecision),
    BossRelic(BossRelicAdmission),
}

#[derive(Clone)]
pub(super) struct OwnerCandidateDecision {
    pub(super) evaluation: CandidateEvaluation,
    pub(super) admission: Option<RewardAdmission>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OwnerChoiceExpansion {
    AutoAllowed,
    InspectOnly(&'static str),
}

impl OwnerChoice {
    pub(super) fn auto_expand_allowed(&self) -> bool {
        matches!(self.expansion, OwnerChoiceExpansion::AutoAllowed)
    }

    pub(super) fn inspect_only_reason(&self) -> Option<&'static str> {
        match self.expansion {
            OwnerChoiceExpansion::InspectOnly(reason) => Some(reason),
            OwnerChoiceExpansion::AutoAllowed => None,
        }
    }
}

impl ChoiceAnnotation {
    pub(super) fn admission(&self) -> Option<&RewardAdmission> {
        match self {
            ChoiceAnnotation::Candidate(decision) => decision.admission.as_ref(),
            _ => None,
        }
    }

    pub(super) fn evaluation(&self) -> Option<&CandidateEvaluation> {
        match self {
            ChoiceAnnotation::Candidate(decision) => Some(&decision.evaluation),
            _ => None,
        }
    }

    pub(super) fn candidate(&self) -> Option<&OwnerCandidateDecision> {
        match self {
            ChoiceAnnotation::Candidate(decision) => Some(decision),
            _ => None,
        }
    }

    pub(super) fn boss_relic(&self) -> Option<&BossRelicAdmission> {
        match self {
            ChoiceAnnotation::BossRelic(admission) => Some(admission),
            _ => None,
        }
    }
}

pub(super) fn owner_decision(
    session: &RunControlSession,
    owner: Owner,
    surface: &DecisionSurface,
) -> OwnerDecision {
    match owner {
        Owner::NeowStart => OwnerDecision::Candidates(executable_choices(surface)),
        Owner::CardReward => OwnerDecision::Candidates(card_reward_owner_choices(session, surface)),
        Owner::BossRelic => OwnerDecision::Candidates(boss_relic_owner_choices(session, surface)),
        Owner::ShopTiny => OwnerDecision::Candidates(shop_tiny_owner_choices(session, surface)),
        Owner::Event(_) => event_owner_decision(session, surface),
        Owner::RewardTiny => reward_tiny_owner_decision(surface),
        Owner::Campfire => campfire_owner_decision(session, surface),
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
            if option.ui.disabled || !selector.matches(&option.semantics) {
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

pub(super) fn cleanup_target_label(target: CleanupTarget) -> &'static str {
    match target {
        CleanupTarget::Curse => "curse",
        CleanupTarget::Status => "status",
        CleanupTarget::StarterStrike => "starter-attack",
        CleanupTarget::StarterDefend => "starter-skill",
        CleanupTarget::OtherStarter => "starter",
        CleanupTarget::Other => "other",
    }
}
