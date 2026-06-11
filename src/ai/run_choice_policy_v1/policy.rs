use crate::content::cards::{get_card_definition, CardRarity, CardTag, CardType};
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
            let upgrade_priority =
                if matches!(choice.reason, RunPendingChoiceReason::Upgrade) && selectable {
                    Some(
                        crate::ai::campfire_policy_v1::campfire_smith_upgrade_priority_v1(
                            card, run_state,
                        ),
                    )
                } else {
                    None
                };
            let class = if matches!(choice.reason, RunPendingChoiceReason::Upgrade) && selectable {
                RunChoicePolicyClassV1::UpgradeTarget
            } else if is_deck_mutation_choice(&choice.reason) {
                if definition.card_type == CardType::Curse {
                    RunChoicePolicyClassV1::CursePurge
                } else if definition.tags.contains(&CardTag::StarterStrike) {
                    RunChoicePolicyClassV1::StarterStrikeMutation
                } else if definition.tags.contains(&CardTag::StarterDefend) {
                    RunChoicePolicyClassV1::StarterDefendMutation
                } else if definition.rarity == CardRarity::Basic {
                    RunChoicePolicyClassV1::BasicCardMutation
                } else {
                    RunChoicePolicyClassV1::OtherDeckMutation
                }
            } else {
                RunChoicePolicyClassV1::UnsupportedChoice
            };
            let rank = mutation_target_rank(class, card.upgrades);
            RunChoiceCandidateEvidenceV1 {
                candidate_id: candidate_id(deck_index),
                label: definition.name.to_string(),
                deck_index,
                card: card.id,
                class,
                selectable,
                evidence: vec![
                    format!("choice reason is {:?}", choice.reason),
                    format!("deck mutation rank={rank}"),
                    format!("upgrades={}", card.upgrades),
                    upgrade_priority
                        .map(|priority| format!("smith upgrade priority={priority}"))
                        .unwrap_or_else(|| "smith upgrade priority=none".to_string()),
                ],
                risks: mutation_risks(class),
                upgrade_priority,
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
    let action = if let Some(selected) = select_upgrade_target(context, config) {
        let reason = selection_reason(&context.reason, &selected);
        RunChoicePolicyActionV1::SelectDeckIndices {
            indices: selected
                .iter()
                .map(|candidate| candidate.deck_index)
                .collect(),
            labels: selected
                .iter()
                .map(|candidate| candidate.label.clone())
                .collect(),
            confidence: 0.78,
            reason,
        }
    } else if let Some(selected) = select_deck_mutation_targets(context, config) {
        let confidence = selection_confidence(&selected);
        let reason = selection_reason(&context.reason, &selected);
        RunChoicePolicyActionV1::SelectDeckIndices {
            indices: selected
                .iter()
                .map(|candidate| candidate.deck_index)
                .collect(),
            labels: selected
                .iter()
                .map(|candidate| candidate.label.clone())
                .collect(),
            confidence,
            reason,
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

fn select_upgrade_target<'a>(
    context: &'a RunChoiceDecisionContextV1,
    config: &RunChoicePolicyConfigV1,
) -> Option<Vec<&'a RunChoiceCandidateEvidenceV1>> {
    if !config.allow_clear_upgrade
        || context.reason != RunPendingChoiceReason::Upgrade
        || context.min_choices != 1
        || context.max_choices != 1
    {
        return None;
    }
    context
        .candidates
        .iter()
        .filter(|candidate| candidate.selectable)
        .filter(|candidate| candidate.class == RunChoicePolicyClassV1::UpgradeTarget)
        .filter_map(|candidate| {
            let priority = candidate.upgrade_priority?;
            (priority >= config.clear_upgrade_priority_threshold).then_some((candidate, priority))
        })
        .max_by_key(|(candidate, priority)| (*priority, std::cmp::Reverse(candidate.deck_index)))
        .map(|(candidate, _)| vec![candidate])
}

fn select_deck_mutation_targets<'a>(
    context: &'a RunChoiceDecisionContextV1,
    config: &RunChoicePolicyConfigV1,
) -> Option<Vec<&'a RunChoiceCandidateEvidenceV1>> {
    if context.min_choices == 0 || context.min_choices != context.max_choices {
        return None;
    }
    if !is_deck_mutation_choice(&context.reason) {
        return None;
    }

    let count = context.min_choices;
    let mut candidates = context
        .candidates
        .iter()
        .filter(|candidate| candidate.selectable)
        .filter(|candidate| target_allowed(candidate, &context.reason, config))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| {
        (
            mutation_target_rank(candidate.class, 0),
            candidate.deck_index,
            candidate.label.as_str(),
        )
    });

    if candidates.len() < count {
        return None;
    }

    let mut selected = Vec::new();
    if is_transform_choice(&context.reason) {
        select_one_per_low_value_class(&candidates, &mut selected, count);
    }
    fill_low_value_targets(&candidates, &mut selected, count);

    (selected.len() == count).then_some(selected)
}

fn select_one_per_low_value_class<'a>(
    candidates: &[&'a RunChoiceCandidateEvidenceV1],
    selected: &mut Vec<&'a RunChoiceCandidateEvidenceV1>,
    count: usize,
) {
    for class in [
        RunChoicePolicyClassV1::CursePurge,
        RunChoicePolicyClassV1::StarterStrikeMutation,
        RunChoicePolicyClassV1::StarterDefendMutation,
    ] {
        let Some(candidate) = candidates
            .iter()
            .copied()
            .find(|candidate| candidate.class == class)
        else {
            continue;
        };
        push_unique(selected, candidate, count);
    }
}

fn fill_low_value_targets<'a>(
    candidates: &[&'a RunChoiceCandidateEvidenceV1],
    selected: &mut Vec<&'a RunChoiceCandidateEvidenceV1>,
    count: usize,
) {
    for candidate in candidates {
        push_unique(selected, candidate, count);
    }
}

fn push_unique<'a>(
    selected: &mut Vec<&'a RunChoiceCandidateEvidenceV1>,
    candidate: &'a RunChoiceCandidateEvidenceV1,
    count: usize,
) {
    if selected.len() >= count
        || selected
            .iter()
            .any(|existing| existing.deck_index == candidate.deck_index)
    {
        return;
    }
    selected.push(candidate);
}

fn target_allowed(
    candidate: &RunChoiceCandidateEvidenceV1,
    reason: &RunPendingChoiceReason,
    config: &RunChoicePolicyConfigV1,
) -> bool {
    match candidate.class {
        RunChoicePolicyClassV1::CursePurge => {
            config.allow_curse_purge
                && matches!(
                    reason,
                    RunPendingChoiceReason::Purge
                        | RunPendingChoiceReason::PurgeNonBottled
                        | RunPendingChoiceReason::Transform
                        | RunPendingChoiceReason::TransformNonBottled
                        | RunPendingChoiceReason::TransformUpgraded
                )
        }
        RunChoicePolicyClassV1::StarterStrikeMutation
        | RunChoicePolicyClassV1::StarterDefendMutation
        | RunChoicePolicyClassV1::BasicCardMutation => {
            (is_purge_choice(reason) && config.allow_low_value_purge)
                || (is_transform_choice(reason) && config.allow_low_value_transform)
        }
        RunChoicePolicyClassV1::OtherDeckMutation | RunChoicePolicyClassV1::UnsupportedChoice => {
            false
        }
        RunChoicePolicyClassV1::UpgradeTarget => false,
    }
}

fn mutation_target_rank(class: RunChoicePolicyClassV1, upgrades: u8) -> i32 {
    let base = match class {
        RunChoicePolicyClassV1::CursePurge => 0,
        RunChoicePolicyClassV1::StarterStrikeMutation => 10,
        RunChoicePolicyClassV1::StarterDefendMutation => 20,
        RunChoicePolicyClassV1::BasicCardMutation => 35,
        RunChoicePolicyClassV1::OtherDeckMutation => 100,
        RunChoicePolicyClassV1::UpgradeTarget => 10_000,
        RunChoicePolicyClassV1::UnsupportedChoice => 10_000,
    };
    base + i32::from(upgrades) * 5
}

fn mutation_risks(class: RunChoicePolicyClassV1) -> Vec<String> {
    match class {
        RunChoicePolicyClassV1::CursePurge => Vec::new(),
        RunChoicePolicyClassV1::StarterStrikeMutation => {
            vec![
                "removing or transforming starter attacks can reduce short-term frontload"
                    .to_string(),
            ]
        }
        RunChoicePolicyClassV1::StarterDefendMutation => {
            vec![
                "removing or transforming starter blocks can reduce short-term defense".to_string(),
            ]
        }
        RunChoicePolicyClassV1::BasicCardMutation => {
            vec!["basic non-starter mutation is only used after lower-value targets".to_string()]
        }
        RunChoicePolicyClassV1::OtherDeckMutation => {
            vec!["non-basic deck mutation remains a human choice".to_string()]
        }
        RunChoicePolicyClassV1::UpgradeTarget => {
            vec!["upgrade selection requires clear smith priority".to_string()]
        }
        RunChoicePolicyClassV1::UnsupportedChoice => Vec::new(),
    }
}

fn selection_confidence(selected: &[&RunChoiceCandidateEvidenceV1]) -> f32 {
    if selected
        .iter()
        .all(|candidate| candidate.class == RunChoicePolicyClassV1::CursePurge)
    {
        0.94
    } else if selected.iter().all(|candidate| {
        matches!(
            candidate.class,
            RunChoicePolicyClassV1::StarterStrikeMutation
                | RunChoicePolicyClassV1::StarterDefendMutation
        )
    }) {
        0.82
    } else {
        0.74
    }
}

fn selection_reason(
    reason: &RunPendingChoiceReason,
    selected: &[&RunChoiceCandidateEvidenceV1],
) -> String {
    let classes = selected
        .iter()
        .map(|candidate| format!("{:?}", candidate.class))
        .collect::<Vec<_>>()
        .join(",");
    format!("low-value visible deck mutation targets for {reason:?}: {classes}")
}

fn is_purge_choice(reason: &RunPendingChoiceReason) -> bool {
    matches!(
        reason,
        RunPendingChoiceReason::Purge | RunPendingChoiceReason::PurgeNonBottled
    )
}

fn is_transform_choice(reason: &RunPendingChoiceReason) -> bool {
    matches!(
        reason,
        RunPendingChoiceReason::Transform
            | RunPendingChoiceReason::TransformNonBottled
            | RunPendingChoiceReason::TransformUpgraded
    )
}

fn is_deck_mutation_choice(reason: &RunPendingChoiceReason) -> bool {
    is_purge_choice(reason) || is_transform_choice(reason)
}

fn stop_reason(context: &RunChoiceDecisionContextV1) -> String {
    format!(
        "run choice policy stopped because no low-value visible deck mutation target satisfied min_choices={} for {:?}",
        context.min_choices, context.reason
    )
}
