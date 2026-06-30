use sts_simulator::ai::strategy::boss_relic_admission::{
    assess_boss_relic_admission, skip_boss_relic_admission,
};
use sts_simulator::ai::strategy::decision_pipeline::{
    boss_relic_order_key, evaluate_decision_candidate, CandidateOrderKey, DecisionCandidateKind,
    DecisionPipelineContext,
};
use sts_simulator::ai::strategy::deck_plan::DeckPlanSnapshot;
use sts_simulator::ai::strategy::reward_admission::{
    assess_reward_admission_from_master_deck, reward_admission_order_key_v1, skip_reward_admission,
    RewardAdmission, RewardAdmissionOrderKeyV1,
};
use sts_simulator::eval::run_control::{DecisionCandidateKey, DecisionSurface, RunControlSession};
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::core::EngineState;

use super::candidate_ir_adapter::{
    boss_relic_kind, card_reward_kind, is_boss_relic_key, is_card_reward_key, shop_tiny_kind,
};
use super::expansion_policy::{expansion_from_evaluation, shop_tiny_choice_expansion};
use super::owner_model::{
    ChoiceAnnotation, OwnerCandidateDecision, OwnerChoice, OwnerChoiceExpansion,
};
use super::owners::{executable_choices, executable_choices_including_cancel};

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

pub(super) fn boss_relic_owner_choices(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> Vec<OwnerChoice> {
    let EngineState::BossRelicSelect(_) = &session.engine_state else {
        return Vec::new();
    };
    let mut choices = executable_choices_including_cancel(surface)
        .into_iter()
        .filter(|choice| is_boss_relic_key(&choice.key))
        .map(|mut choice| {
            choice.annotation = boss_relic_annotation_for_choice(session, &choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    choices.sort_by_key(|(index, choice)| (boss_relic_choice_rank(choice), *index));
    choices.into_iter().map(|(_, choice)| choice).collect()
}

pub(super) fn shop_tiny_owner_choices(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> Vec<OwnerChoice> {
    let context = shop_tiny_context(session);
    let deck = &session.run_state.master_deck;
    let mut choices = executable_choices(surface)
        .into_iter()
        .map(|mut choice| {
            choice.annotation = shop_tiny_candidate_for_choice(context, deck, &choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    let mut auto_purge_targets = Vec::new();
    for (_, choice) in choices.iter_mut() {
        choice.expansion = shop_tiny_choice_expansion(&choice.annotation, &mut auto_purge_targets);
    }
    choices.sort_by_key(|(index, choice)| (shop_tiny_choice_rank(choice), *index));
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn shop_tiny_context(session: &RunControlSession) -> DecisionPipelineContext {
    DecisionPipelineContext::shop(
        DeckPlanSnapshot::from_run_state(&session.run_state),
        session.run_state.gold,
    )
}

fn shop_tiny_candidate_for_choice(
    context: DecisionPipelineContext,
    deck: &[CombatCard],
    choice: &OwnerChoice,
) -> ChoiceAnnotation {
    let kind = shop_tiny_kind(&choice.key);
    candidate_annotation(context, kind, shop_card_admission(deck, kind))
}

fn shop_card_admission(
    deck: &[CombatCard],
    kind: DecisionCandidateKind,
) -> Option<RewardAdmission> {
    if let DecisionCandidateKind::ShopBuyCard { card, upgrades, .. } = kind {
        Some(assess_reward_admission_from_master_deck(
            deck, card, upgrades,
        ))
    } else {
        None
    }
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

fn candidate_annotation(
    context: DecisionPipelineContext,
    kind: DecisionCandidateKind,
    admission: Option<RewardAdmission>,
) -> ChoiceAnnotation {
    let evaluation = evaluate_decision_candidate(context, kind, admission.as_ref());
    ChoiceAnnotation::Candidate(OwnerCandidateDecision {
        admission,
        evaluation,
    })
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

fn boss_relic_choice_rank(choice: &OwnerChoice) -> (u8, CandidateOrderKey) {
    let kind = boss_relic_kind(&choice.key);
    match choice.key {
        Some(DecisionCandidateKey::BossRelicPick { .. }) => (
            0,
            boss_relic_order_key(kind, choice.annotation.boss_relic()),
        ),
        Some(DecisionCandidateKey::BossRelicSkip) => (
            1,
            boss_relic_order_key(kind, choice.annotation.boss_relic()),
        ),
        _ => (2, CandidateOrderKey::fallback()),
    }
}

fn shop_tiny_choice_rank(choice: &OwnerChoice) -> (u8, CandidateOrderKey) {
    match &choice.annotation {
        ChoiceAnnotation::Candidate(decision) => decision.evaluation.auto_order_key(false),
        _ => (u8::MAX, CandidateOrderKey::fallback()),
    }
}
