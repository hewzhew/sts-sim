use crate::content::cards::{get_card_definition, CardType};
use crate::state::core::{RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::run::RunState;

use super::types::{
    candidate_id, RunChoiceCandidateEvidenceV1, RunChoiceDecisionContextV1, RunChoiceDecisionV1,
    RunChoicePolicyActionV1, RunChoicePolicyClassV1, RunChoicePolicyConfigV1,
};

pub fn build_run_choice_decision_context_v1(
    run_state: &RunState,
    choice: &RunPendingChoiceState,
) -> RunChoiceDecisionContextV1 {
    let candidates = run_state
        .master_deck
        .iter()
        .enumerate()
        .map(|(deck_index, card)| {
            let selectable = crate::state::core::run_pending_choice_allows_card_for_run(
                &choice.reason,
                card,
                run_state,
            );
            let definition = get_card_definition(card.id);
            let class = if is_purge_choice(&choice.reason) {
                if definition.card_type == CardType::Curse {
                    RunChoicePolicyClassV1::CursePurge
                } else {
                    RunChoicePolicyClassV1::NonCursePurge
                }
            } else {
                RunChoicePolicyClassV1::UnsupportedChoice
            };
            RunChoiceCandidateEvidenceV1 {
                candidate_id: candidate_id(deck_index),
                label: definition.name.to_string(),
                deck_index,
                card: card.id,
                class,
                selectable,
                evidence: vec![format!("choice reason is {:?}", choice.reason)],
                risks: if class == RunChoicePolicyClassV1::NonCursePurge {
                    vec!["non-curse deck mutation remains a human choice".to_string()]
                } else {
                    Vec::new()
                },
            }
        })
        .collect();

    RunChoiceDecisionContextV1 {
        reason: choice.reason.clone(),
        min_choices: choice.min_choices,
        max_choices: choice.max_choices,
        candidates,
    }
}

pub fn plan_run_choice_decision_v1(
    context: &RunChoiceDecisionContextV1,
    config: &RunChoicePolicyConfigV1,
) -> RunChoiceDecisionV1 {
    let action = if config.allow_curse_purge && is_purge_choice(&context.reason) {
        let mut selected = context
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.selectable && candidate.class == RunChoicePolicyClassV1::CursePurge
            })
            .take(context.max_choices)
            .collect::<Vec<_>>();
        if selected.len() >= context.min_choices && !selected.is_empty() {
            selected.truncate(context.max_choices);
            RunChoicePolicyActionV1::SelectDeckIndices {
                indices: selected
                    .iter()
                    .map(|candidate| candidate.deck_index)
                    .collect(),
                labels: selected
                    .iter()
                    .map(|candidate| candidate.label.clone())
                    .collect(),
                confidence: 0.94,
                reason: "visible curse purge at run pending purge choice".to_string(),
            }
        } else {
            RunChoicePolicyActionV1::Stop {
                reason: stop_reason(context),
            }
        }
    } else {
        RunChoicePolicyActionV1::Stop {
            reason: stop_reason(context),
        }
    };

    RunChoiceDecisionV1 {
        action,
        label_role: "behavior_policy_not_teacher",
        context: context.clone(),
    }
}

fn is_purge_choice(reason: &RunPendingChoiceReason) -> bool {
    matches!(
        reason,
        RunPendingChoiceReason::Purge | RunPendingChoiceReason::PurgeNonBottled
    )
}

fn stop_reason(context: &RunChoiceDecisionContextV1) -> String {
    format!(
        "run choice policy stopped because no visible curse purge satisfied min_choices={} for {:?}",
        context.min_choices, context.reason
    )
}
