use sts_simulator::ai::strategy::boss_relic_admission::{
    assess_boss_relic_admission, boss_relic_admission_order_rank, skip_boss_relic_admission,
    BossRelicAdmission,
};
use sts_simulator::ai::strategy::reward_admission::{
    assess_reward_admission, reward_admission_order_key_v1, skip_reward_admission, RewardAdmission,
    RewardAdmissionOrderKeyV1,
};
use sts_simulator::eval::run_control::{
    DecisionCandidateKey, DecisionSurface, RunControlCommand, RunControlSession,
};
use sts_simulator::state::core::{ClientInput, EngineState};

use super::Owner;

pub(super) type DecisionKey = DecisionCandidateKey;

#[derive(Clone)]
pub(super) struct OwnerChoice {
    pub(super) key: Option<DecisionKey>,
    pub(super) action: RunControlCommand,
    pub(super) label: String,
    pub(super) annotation: ChoiceAnnotation,
}

#[derive(Clone)]
pub(super) enum ChoiceAnnotation {
    None,
    Reward(RewardAdmission),
    BossRelic(BossRelicAdmission),
}

impl ChoiceAnnotation {
    fn reward(&self) -> Option<&RewardAdmission> {
        match self {
            ChoiceAnnotation::Reward(admission) => Some(admission),
            _ => None,
        }
    }

    fn boss_relic(&self) -> Option<&BossRelicAdmission> {
        match self {
            ChoiceAnnotation::BossRelic(admission) => Some(admission),
            _ => None,
        }
    }
}

pub(super) fn owner_choices(
    session: &RunControlSession,
    owner: Owner,
    surface: &DecisionSurface,
) -> Vec<OwnerChoice> {
    match owner {
        Owner::NeowStart => executable_choices(surface),
        Owner::CardReward => card_reward_owner_choices(session, surface),
        Owner::BossRelic => boss_relic_owner_choices(session, surface),
        Owner::Event(_) | Owner::RewardTiny | Owner::ShopTiny => Vec::new(),
    }
}

pub(super) fn executable_choices(surface: &DecisionSurface) -> Vec<OwnerChoice> {
    executable_choices_with_cancel(surface, false)
}

pub(super) fn executable_choices_including_cancel(surface: &DecisionSurface) -> Vec<OwnerChoice> {
    executable_choices_with_cancel(surface, true)
}

fn card_reward_owner_choices(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> Vec<OwnerChoice> {
    let deck = session
        .run_state
        .master_deck
        .iter()
        .map(|card| card.id)
        .collect::<Vec<_>>();
    let mut choices = executable_choices(surface)
        .into_iter()
        .filter(is_card_reward_choice)
        .map(|mut choice| {
            choice.annotation = reward_annotation_for_choice(&deck, &choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    choices.sort_by_key(|(index, choice)| (card_reward_choice_rank(choice), *index));
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn boss_relic_owner_choices(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> Vec<OwnerChoice> {
    let EngineState::BossRelicSelect(_) = &session.engine_state else {
        return Vec::new();
    };
    let mut choices = executable_choices_including_cancel(surface)
        .into_iter()
        .filter(is_boss_relic_choice)
        .map(|mut choice| {
            choice.annotation = boss_relic_annotation_for_choice(session, &choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    choices.sort_by_key(|(index, choice)| (boss_relic_choice_rank(choice), *index));
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn boss_relic_annotation_for_choice(
    session: &RunControlSession,
    choice: &OwnerChoice,
) -> ChoiceAnnotation {
    match choice.key {
        Some(DecisionCandidateKey::BossRelicPick { relic, .. }) => {
            ChoiceAnnotation::BossRelic(assess_boss_relic_admission(&session.run_state, relic))
        }
        Some(DecisionCandidateKey::BossRelicSkip) => {
            ChoiceAnnotation::BossRelic(skip_boss_relic_admission())
        }
        _ => ChoiceAnnotation::None,
    }
}

fn reward_annotation_for_choice(
    deck: &[sts_simulator::content::cards::CardId],
    choice: &OwnerChoice,
) -> ChoiceAnnotation {
    match choice.key {
        Some(DecisionCandidateKey::CardRewardPick { card, .. }) => {
            ChoiceAnnotation::Reward(assess_reward_admission(deck, card))
        }
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => {
            ChoiceAnnotation::Reward(skip_reward_admission())
        }
        Some(DecisionCandidateKey::CardRewardOpen { .. })
        | Some(DecisionCandidateKey::CardRewardSingingBowl { .. })
        | None => ChoiceAnnotation::None,
        _ => ChoiceAnnotation::None,
    }
}

fn card_reward_choice_rank(choice: &OwnerChoice) -> (u8, RewardAdmissionOrderKeyV1) {
    match &choice.key {
        Some(DecisionCandidateKey::CardRewardOpen { .. }) => {
            (0, RewardAdmissionOrderKeyV1::empty_or_deferred())
        }
        Some(DecisionCandidateKey::CardRewardPick { .. }) => (
            1,
            choice
                .annotation
                .reward()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::empty_or_deferred),
        ),
        Some(DecisionCandidateKey::CardRewardSingingBowl { .. }) => {
            (1, RewardAdmissionOrderKeyV1::unscored_optional_reward())
        }
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => (
            1,
            choice
                .annotation
                .reward()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::static_skip_boundary),
        ),
        _ => (2, RewardAdmissionOrderKeyV1::empty_or_deferred()),
    }
}

fn boss_relic_choice_rank(choice: &OwnerChoice) -> (u8, u8) {
    let skip_order = boss_relic_admission_order_rank(&skip_boss_relic_admission());
    match choice.key {
        Some(DecisionCandidateKey::BossRelicPick { .. }) => (
            0,
            choice
                .annotation
                .boss_relic()
                .map(boss_relic_admission_order_rank)
                .unwrap_or(skip_order),
        ),
        Some(DecisionCandidateKey::BossRelicSkip) => (1, skip_order),
        _ => (2, skip_order),
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

fn is_card_reward_choice(choice: &OwnerChoice) -> bool {
    matches!(
        choice.key,
        Some(
            DecisionCandidateKey::CardRewardOpen { .. }
                | DecisionCandidateKey::CardRewardPick { .. }
                | DecisionCandidateKey::CardRewardSingingBowl { .. }
                | DecisionCandidateKey::CardRewardSkip { .. }
        )
    )
}

fn is_boss_relic_choice(choice: &OwnerChoice) -> bool {
    matches!(
        choice.key,
        Some(DecisionCandidateKey::BossRelicPick { .. } | DecisionCandidateKey::BossRelicSkip)
    )
}
