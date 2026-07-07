use sts_simulator::ai::strategy::decision_pipeline::{
    CandidateOrderKey, DecisionCandidateKind, DecisionPipelineContext,
};
use sts_simulator::ai::strategy::deck_plan::DeckPlanSnapshot;
use sts_simulator::ai::strategy::reward_admission::{
    assess_reward_admission_from_master_deck, reward_admission_order_key_v1, skip_reward_admission,
    RewardAdmissionOrderKeyV1,
};
use sts_simulator::eval::run_control::{DecisionCandidateKey, DecisionSurface, RunControlSession};

use super::candidate_ir_adapter::{card_reward_kind, is_card_reward_key};
use super::expansion_policy::expansion_from_evaluation;
use super::owner_candidate_eval::candidate_annotation;
use super::owner_commands::executable_choices;
use super::owner_model::{ChoiceAnnotation, OwnerChoice, OwnerChoiceExpansion};

pub(super) fn card_reward_owner_choices(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> Vec<OwnerChoice> {
    let deck_plan = DeckPlanSnapshot::from_run_state(&session.run_state);
    let context = DecisionPipelineContext::reward(deck_plan);
    let mut choices = executable_choices(surface)
        .into_iter()
        .filter(|choice| is_card_reward_key(&choice.key))
        .map(|mut choice| {
            choice.annotation = reward_annotation_for_choice(session, &choice, context);
            choice.expansion = card_reward_choice_expansion(&choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    let has_mainline_take = choices
        .iter()
        .any(|(_, choice)| is_mainline_card_reward_take(choice));
    choices.sort_by_key(|(index, choice)| {
        (card_reward_choice_rank(choice, has_mainline_take), *index)
    });
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn reward_annotation_for_choice(
    session: &RunControlSession,
    choice: &OwnerChoice,
    context: DecisionPipelineContext,
) -> ChoiceAnnotation {
    match card_reward_kind(&choice.key) {
        Some(DecisionCandidateKind::CardRewardPick { card, upgrades }) => {
            let deck = &session.run_state.master_deck;
            candidate_annotation(
                context,
                DecisionCandidateKind::CardRewardPick { card, upgrades },
                Some(assess_reward_admission_from_master_deck(
                    deck, card, upgrades,
                )),
            )
        }
        Some(DecisionCandidateKind::CardRewardSkip) => candidate_annotation(
            context,
            DecisionCandidateKind::CardRewardSkip,
            Some(skip_reward_admission()),
        ),
        _ => ChoiceAnnotation::None,
    }
}

fn card_reward_choice_expansion(choice: &OwnerChoice) -> OwnerChoiceExpansion {
    expansion_from_evaluation(choice.annotation.evaluation())
}

fn card_reward_choice_rank(
    choice: &OwnerChoice,
    has_mainline_take: bool,
) -> (u8, CandidateOrderKey, RewardAdmissionOrderKeyV1) {
    match &choice.key {
        Some(DecisionCandidateKey::CardRewardOpen { .. }) => (
            0,
            CandidateOrderKey::fallback(),
            RewardAdmissionOrderKeyV1::empty_or_deferred(),
        ),
        Some(DecisionCandidateKey::CardRewardPick { .. }) => (
            1,
            choice
                .annotation
                .evaluation()
                .map(|evaluation| evaluation.order_key(has_mainline_take))
                .unwrap_or_else(CandidateOrderKey::fallback),
            choice
                .annotation
                .admission()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::empty_or_deferred),
        ),
        Some(DecisionCandidateKey::CardRewardSingingBowl { .. }) => (
            1,
            CandidateOrderKey::optional_skip(has_mainline_take),
            RewardAdmissionOrderKeyV1::unscored_optional_reward(),
        ),
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => (
            1,
            choice
                .annotation
                .evaluation()
                .map(|evaluation| evaluation.order_key(has_mainline_take))
                .unwrap_or_else(|| CandidateOrderKey::optional_skip(has_mainline_take)),
            choice
                .annotation
                .admission()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::static_skip_boundary),
        ),
        _ => (
            2,
            CandidateOrderKey::fallback(),
            RewardAdmissionOrderKeyV1::empty_or_deferred(),
        ),
    }
}

fn is_mainline_card_reward_take(choice: &OwnerChoice) -> bool {
    matches!(
        choice.key,
        Some(DecisionCandidateKey::CardRewardPick { .. })
    ) && choice
        .annotation
        .evaluation()
        .is_some_and(|evaluation| evaluation.is_mainline())
}
