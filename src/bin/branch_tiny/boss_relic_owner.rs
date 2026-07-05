use sts_simulator::ai::strategy::boss_relic_admission::{
    assess_boss_relic_admission, skip_boss_relic_admission,
};
use sts_simulator::ai::strategy::decision_pipeline::{boss_relic_order_key, CandidateOrderKey};
use sts_simulator::eval::run_control::{DecisionCandidateKey, DecisionSurface, RunControlSession};
use sts_simulator::state::core::EngineState;

use super::candidate_ir_adapter::{boss_relic_kind, is_boss_relic_key};
use super::owner_model::{ChoiceAnnotation, OwnerChoice};
use super::owners::executable_choices_including_cancel;

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
