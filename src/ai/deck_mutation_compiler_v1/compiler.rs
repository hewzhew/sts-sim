use std::collections::BTreeSet;

use crate::ai::run_choice_policy_v1::{
    build_run_choice_decision_context_v1, plan_run_choice_decision_v1,
    run_choice_duplicate_priority_v1, RunChoicePolicyActionV1, RunChoicePolicyClassV1,
    RunChoicePolicyConfigV1,
};
use crate::content::cards::{get_card_definition, CardId};
use crate::runtime::combat::CombatCard;
use crate::state::core::{RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::run::RunState;

use super::types::{
    AllowedDeckMutationConsumersV1, CompiledDeckMutationDecisionV1, DeckMutationCardSnapshotV1,
    DeckMutationCompilerModeV1, DeckMutationKindV1, DeckMutationPlanCandidateV1,
    DeckMutationPlanRoleV1, DeckMutationPlanStepV1, DeckMutationTargetClassV1,
};

const MAX_DUPLICATE_OPTIONS_PER_BRANCH: usize = 4;

#[derive(Clone, Debug)]
struct ExactTarget {
    card: DeckMutationCardSnapshotV1,
    identity_key: String,
    selectable: bool,
    upgrade_priority: Option<i32>,
    duplicate_priority: i32,
}

#[derive(Clone, Debug)]
struct TargetGroup {
    targets: Vec<ExactTarget>,
}

#[derive(Clone, Debug)]
struct GroupCountCombination {
    group_counts: Vec<usize>,
    represented_exact_count: usize,
}

pub fn compile_deck_mutation_decision_v1(
    run_state: &RunState,
    choice: &RunPendingChoiceState,
    mode: DeckMutationCompilerModeV1,
) -> CompiledDeckMutationDecisionV1 {
    let run_choice_context = build_run_choice_decision_context_v1(run_state, choice);
    let run_choice_decision =
        plan_run_choice_decision_v1(&run_choice_context, &RunChoicePolicyConfigV1::default());
    let run_choice_policy_indices = run_choice_policy_selected_indices(&run_choice_decision);
    let targets = exact_targets(run_state, choice);
    let mut candidate_plans = plan_candidates(run_state, choice, &targets, mode);

    if let Some(indices) = &run_choice_policy_indices {
        if !candidate_plans
            .iter()
            .any(|candidate| candidate.step.deck_indices == *indices)
        {
            if let Some(candidate) = run_choice_policy_plan_candidate(choice, &targets, indices) {
                candidate_plans.push(candidate);
            }
        }
    }

    let low_value_available = targets
        .iter()
        .filter(|target| target.selectable)
        .any(|target| target.card.target_class.is_low_value_mutation_target());

    for candidate in &mut candidate_plans {
        candidate.run_choice_policy_selected = run_choice_policy_indices
            .as_ref()
            .is_some_and(|indices| candidate.step.deck_indices == *indices);
        evaluate_candidate(choice, candidate, low_value_available);
    }
    candidate_plans.sort_by(compare_deck_mutation_candidates_v1);

    let selected_plan = candidate_plans
        .iter()
        .find(|candidate| candidate.allowed_consumers.execute_autopilot)
        .cloned();
    let branch_limit = match mode {
        DeckMutationCompilerModeV1::BranchTopK { max_active } => max_active,
        _ => usize::MAX,
    };
    let branch_active_plans = candidate_plans
        .iter()
        .filter(|candidate| candidate.allowed_consumers.branch_active)
        .take(branch_limit)
        .cloned()
        .collect();
    let inspect_only_plans = candidate_plans
        .iter()
        .filter(|candidate| candidate.role == DeckMutationPlanRoleV1::InspectOnly)
        .cloned()
        .collect();
    let blocked_plans = candidate_plans
        .iter()
        .filter(|candidate| candidate.role == DeckMutationPlanRoleV1::Blocked)
        .cloned()
        .collect();

    CompiledDeckMutationDecisionV1 {
        reason: choice.reason,
        min_choices: choice.min_choices,
        max_choices: choice.max_choices,
        selected_plan,
        branch_active_plans,
        inspect_only_plans,
        blocked_plans,
        candidate_plans,
        label_role: "behavior_policy_not_teacher",
    }
}

fn compare_deck_mutation_candidates_v1(
    left: &DeckMutationPlanCandidateV1,
    right: &DeckMutationPlanCandidateV1,
) -> std::cmp::Ordering {
    deck_mutation_role_rank(left.role)
        .cmp(&deck_mutation_role_rank(right.role))
        .then_with(|| right.score_hint.cmp(&left.score_hint))
        .then_with(|| left.step.command.cmp(&right.step.command))
}

fn deck_mutation_role_rank(role: DeckMutationPlanRoleV1) -> u8 {
    match role {
        DeckMutationPlanRoleV1::PolicyPreferred => 0,
        DeckMutationPlanRoleV1::SafeAlternative => 1,
        DeckMutationPlanRoleV1::RiskyExploration => 2,
        DeckMutationPlanRoleV1::InspectOnly => 3,
        DeckMutationPlanRoleV1::Blocked => 4,
    }
}

fn exact_targets(run_state: &RunState, choice: &RunPendingChoiceState) -> Vec<ExactTarget> {
    let request = choice.selection_request(run_state);
    let target_uuids = request
        .targets
        .iter()
        .map(|target| match target {
            crate::state::selection::SelectionTargetRef::CardUuid(uuid) => *uuid,
        })
        .collect::<BTreeSet<_>>();
    let run_choice_context = build_run_choice_decision_context_v1(run_state, choice);

    run_state
        .master_deck
        .iter()
        .enumerate()
        .filter(|(_, card)| target_uuids.contains(&card.uuid))
        .map(|(deck_index, card)| {
            let run_choice_candidate = run_choice_context
                .candidates
                .iter()
                .find(|candidate| candidate.deck_index == deck_index);
            ExactTarget {
                card: DeckMutationCardSnapshotV1 {
                    deck_index,
                    card: card.id,
                    upgrades: card.upgrades,
                    label: card_label(card.id, card.upgrades),
                    target_class: target_class_for_choice(
                        choice.reason,
                        run_choice_candidate.map(|candidate| candidate.class),
                    ),
                },
                identity_key: card_stat_identity_key(card),
                selectable: run_choice_candidate.is_some_and(|candidate| candidate.selectable),
                upgrade_priority: run_choice_candidate
                    .and_then(|candidate| candidate.upgrade_priority),
                duplicate_priority: run_choice_duplicate_priority_v1(card, run_state),
            }
        })
        .collect()
}

fn plan_candidates(
    run_state: &RunState,
    choice: &RunPendingChoiceState,
    targets: &[ExactTarget],
    mode: DeckMutationCompilerModeV1,
) -> Vec<DeckMutationPlanCandidateV1> {
    if choice.min_choices == 0 || choice.min_choices != choice.max_choices {
        return Vec::new();
    }

    if choice.min_choices == 1 {
        let mut candidates = compressed_single_candidates(choice, targets);
        if matches!(choice.reason, RunPendingChoiceReason::Duplicate) {
            candidates.sort_by(|left, right| {
                right
                    .score_hint
                    .cmp(&left.score_hint)
                    .then_with(|| left.step.command.cmp(&right.step.command))
            });
            let limit = match mode {
                DeckMutationCompilerModeV1::BranchTopK { max_active } => {
                    max_active.min(MAX_DUPLICATE_OPTIONS_PER_BRANCH)
                }
                _ => candidates.len(),
            };
            apply_portfolio_suppression(&mut candidates, limit);
            return candidates.into_iter().take(limit).collect();
        }
        return candidates;
    }

    compressed_multi_candidates(
        choice,
        targets,
        match mode {
            DeckMutationCompilerModeV1::BranchTopK { max_active } => max_active,
            _ => usize::MAX,
        },
    )
    .unwrap_or_else(|| {
        let run_choice_context = build_run_choice_decision_context_v1(run_state, choice);
        let run_choice_decision =
            plan_run_choice_decision_v1(&run_choice_context, &RunChoicePolicyConfigV1::default());
        run_choice_policy_selected_indices(&run_choice_decision)
            .and_then(|indices| run_choice_policy_plan_candidate(choice, targets, &indices))
            .into_iter()
            .collect()
    })
}

fn compressed_single_candidates(
    choice: &RunPendingChoiceState,
    targets: &[ExactTarget],
) -> Vec<DeckMutationPlanCandidateV1> {
    let mut groups = Vec::<TargetGroup>::new();
    for target in targets.iter().filter(|target| target.selectable).cloned() {
        if let Some(group) = groups.iter_mut().find(|group| {
            group.targets[0].identity_key == target.identity_key
                && group.targets[0].card.target_class == target.card.target_class
        }) {
            group.targets.push(target);
        } else {
            groups.push(TargetGroup {
                targets: vec![target],
            });
        }
    }
    groups
        .into_iter()
        .filter_map(|group| {
            let target = group.targets.first()?.clone();
            Some(candidate_from_targets(
                choice,
                vec![target],
                group.targets.len(),
            ))
        })
        .collect()
}

fn compressed_multi_candidates(
    choice: &RunPendingChoiceState,
    targets: &[ExactTarget],
    limit: usize,
) -> Option<Vec<DeckMutationPlanCandidateV1>> {
    let groups = target_groups(targets);
    let combinations = bounded_group_count_combinations(&groups, choice.min_choices, limit)?;
    if combinations.is_empty() {
        return None;
    }
    Some(
        combinations
            .into_iter()
            .filter_map(|combo| {
                let selected_targets = combo
                    .group_counts
                    .iter()
                    .enumerate()
                    .flat_map(|(group_idx, count)| groups[group_idx].targets.iter().take(*count))
                    .cloned()
                    .collect::<Vec<_>>();
                if selected_targets.len() == choice.min_choices {
                    Some(candidate_from_targets(
                        choice,
                        selected_targets,
                        combo.represented_exact_count,
                    ))
                } else {
                    None
                }
            })
            .collect(),
    )
}

fn target_groups(targets: &[ExactTarget]) -> Vec<TargetGroup> {
    let mut groups = Vec::<TargetGroup>::new();
    for target in targets.iter().filter(|target| target.selectable).cloned() {
        if let Some(group) = groups.iter_mut().find(|group| {
            group.targets[0].identity_key == target.identity_key
                && group.targets[0].card.target_class == target.card.target_class
        }) {
            group.targets.push(target);
        } else {
            groups.push(TargetGroup {
                targets: vec![target],
            });
        }
    }
    groups
}

fn bounded_group_count_combinations(
    groups: &[TargetGroup],
    choose: usize,
    limit: usize,
) -> Option<Vec<GroupCountCombination>> {
    if choose == 0
        || groups
            .iter()
            .map(|group| group.targets.len())
            .sum::<usize>()
            < choose
    {
        return None;
    }

    let mut combinations = Vec::new();
    let mut group_counts = vec![0; groups.len()];
    if collect_group_count_combinations(
        groups,
        choose,
        limit,
        0,
        &mut group_counts,
        &mut combinations,
    ) {
        Some(combinations)
    } else {
        None
    }
}

fn collect_group_count_combinations(
    groups: &[TargetGroup],
    remaining: usize,
    limit: usize,
    group_index: usize,
    group_counts: &mut [usize],
    combinations: &mut Vec<GroupCountCombination>,
) -> bool {
    if group_index >= groups.len() {
        if remaining == 0 {
            let represented_exact_count = group_counts
                .iter()
                .enumerate()
                .map(|(idx, count)| binomial(groups[idx].targets.len(), *count))
                .product();
            combinations.push(GroupCountCombination {
                group_counts: group_counts.to_vec(),
                represented_exact_count,
            });
        }
        return combinations.len() <= limit;
    }

    let max_count = groups[group_index].targets.len().min(remaining);
    for count in (0..=max_count).rev() {
        group_counts[group_index] = count;
        if !collect_group_count_combinations(
            groups,
            remaining - count,
            limit,
            group_index + 1,
            group_counts,
            combinations,
        ) {
            return false;
        }
    }
    group_counts[group_index] = 0;
    true
}

fn binomial(n: usize, k: usize) -> usize {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    (0..k).fold(1usize, |acc, i| acc * (n - i) / (i + 1))
}

fn run_choice_policy_plan_candidate(
    choice: &RunPendingChoiceState,
    targets: &[ExactTarget],
    indices: &[usize],
) -> Option<DeckMutationPlanCandidateV1> {
    let selected = indices
        .iter()
        .filter_map(|idx| {
            targets
                .iter()
                .find(|target| target.card.deck_index == *idx)
                .cloned()
        })
        .collect::<Vec<_>>();
    (selected.len() == indices.len()).then(|| candidate_from_targets(choice, selected, 1))
}

fn candidate_from_targets(
    choice: &RunPendingChoiceState,
    selected_targets: Vec<ExactTarget>,
    representative_count: usize,
) -> DeckMutationPlanCandidateV1 {
    let kind = deck_mutation_kind(choice.reason);
    let cards = selected_targets
        .iter()
        .map(|target| target.card.clone())
        .collect::<Vec<_>>();
    let deck_indices = cards.iter().map(|card| card.deck_index).collect::<Vec<_>>();
    let labels = cards
        .iter()
        .map(|card| card.label.as_str())
        .collect::<Vec<_>>();
    let effect_kind = effect_kind(choice.reason).to_string();
    let effect_key = if selected_targets.len() == 1 {
        format!(
            "run_selection:{}:{}",
            effect_kind, selected_targets[0].identity_key
        )
    } else {
        format!(
            "run_selection:combo:{}",
            selected_targets
                .iter()
                .map(|target| format!("{}:{}", effect_kind, target.identity_key))
                .collect::<Vec<_>>()
                .join("+")
        )
    };
    let effect_label = format!(
        "{} {}",
        effect_verb(choice.reason),
        render_repeated_labels(&labels)
    );
    let score_hint = selected_targets
        .iter()
        .map(|target| target_score_hint(choice.reason, target))
        .sum();
    DeckMutationPlanCandidateV1 {
        plan_id: format!("deck_mutation:{effect_key}"),
        step: DeckMutationPlanStepV1 {
            kind,
            deck_indices,
            cards,
            command: format_select_command(
                &selected_targets
                    .iter()
                    .map(|target| target.card.deck_index)
                    .collect::<Vec<_>>(),
            ),
            effect_kind,
            effect_key,
            effect_label: effect_label.clone(),
        },
        role: DeckMutationPlanRoleV1::InspectOnly,
        allowed_consumers: AllowedDeckMutationConsumersV1::default(),
        representative_count,
        suppressed_count: representative_count.saturating_sub(1),
        run_choice_policy_selected: false,
        score_hint,
        confidence: 0.0,
        reasons: vec![format!("candidate for {:?}", choice.reason)],
        risks: selected_targets
            .iter()
            .flat_map(|target| target_risks(target.card.target_class))
            .collect(),
    }
}

fn evaluate_candidate(
    choice: &RunPendingChoiceState,
    candidate: &mut DeckMutationPlanCandidateV1,
    low_value_available: bool,
) {
    let has_unsupported_target = candidate
        .step
        .cards
        .iter()
        .any(|card| card.target_class == DeckMutationTargetClassV1::Unsupported);
    let has_functional_target = candidate.step.cards.iter().any(|card| {
        card.target_class == DeckMutationTargetClassV1::Functional
            || card.target_class == DeckMutationTargetClassV1::Unsupported
    });
    let all_low_value = candidate
        .step
        .cards
        .iter()
        .all(|card| card.target_class.is_low_value_mutation_target());
    let mutation_choice = is_remove_choice(choice.reason) || is_transform_choice(choice.reason);

    let role = if has_unsupported_target {
        DeckMutationPlanRoleV1::Blocked
    } else if mutation_choice && has_functional_target && low_value_available {
        DeckMutationPlanRoleV1::InspectOnly
    } else if mutation_choice && has_functional_target {
        DeckMutationPlanRoleV1::RiskyExploration
    } else if candidate.run_choice_policy_selected {
        DeckMutationPlanRoleV1::PolicyPreferred
    } else if mutation_choice && all_low_value {
        DeckMutationPlanRoleV1::SafeAlternative
    } else if matches!(choice.reason, RunPendingChoiceReason::Upgrade) {
        DeckMutationPlanRoleV1::SafeAlternative
    } else {
        DeckMutationPlanRoleV1::SafeAlternative
    };

    candidate.role = role;
    candidate.confidence = match role {
        DeckMutationPlanRoleV1::PolicyPreferred => 0.82,
        DeckMutationPlanRoleV1::SafeAlternative => 0.66,
        DeckMutationPlanRoleV1::RiskyExploration => 0.35,
        DeckMutationPlanRoleV1::InspectOnly | DeckMutationPlanRoleV1::Blocked => 0.0,
    };
    candidate.allowed_consumers = allowed_consumers(choice.reason, role, candidate);
    candidate.reasons.push(format!("role={role:?}"));
    if candidate.run_choice_policy_selected {
        candidate
            .reasons
            .push("legacy run-choice policy selected this plan".to_string());
    }
    if mutation_choice && has_functional_target && low_value_available {
        candidate.reasons.push(
            "functional deck mutation target is inspect-only while low-value targets exist"
                .to_string(),
        );
    }
}

fn allowed_consumers(
    reason: RunPendingChoiceReason,
    role: DeckMutationPlanRoleV1,
    candidate: &DeckMutationPlanCandidateV1,
) -> AllowedDeckMutationConsumersV1 {
    let all_low_value = candidate
        .step
        .cards
        .iter()
        .all(|card| card.target_class.is_low_value_mutation_target());
    let execute_autopilot = match role {
        DeckMutationPlanRoleV1::PolicyPreferred => true,
        DeckMutationPlanRoleV1::SafeAlternative => is_remove_choice(reason) && all_low_value,
        DeckMutationPlanRoleV1::RiskyExploration
        | DeckMutationPlanRoleV1::InspectOnly
        | DeckMutationPlanRoleV1::Blocked => false,
    };

    match role {
        DeckMutationPlanRoleV1::PolicyPreferred => AllowedDeckMutationConsumersV1 {
            execute_autopilot,
            branch_active: true,
            branch_frozen: false,
            inspect: true,
            replay: true,
            human_prompt: false,
        },
        DeckMutationPlanRoleV1::SafeAlternative => AllowedDeckMutationConsumersV1 {
            execute_autopilot,
            branch_active: true,
            branch_frozen: false,
            inspect: true,
            replay: true,
            human_prompt: false,
        },
        DeckMutationPlanRoleV1::RiskyExploration => AllowedDeckMutationConsumersV1 {
            execute_autopilot: false,
            branch_active: true,
            branch_frozen: true,
            inspect: true,
            replay: true,
            human_prompt: true,
        },
        DeckMutationPlanRoleV1::InspectOnly => AllowedDeckMutationConsumersV1 {
            execute_autopilot: false,
            branch_active: false,
            branch_frozen: true,
            inspect: true,
            replay: true,
            human_prompt: true,
        },
        DeckMutationPlanRoleV1::Blocked => AllowedDeckMutationConsumersV1 {
            execute_autopilot: false,
            branch_active: false,
            branch_frozen: false,
            inspect: true,
            replay: false,
            human_prompt: true,
        },
    }
}

fn apply_portfolio_suppression(candidates: &mut [DeckMutationPlanCandidateV1], limit: usize) {
    let suppressed = candidates.len().saturating_sub(limit);
    if suppressed == 0 {
        return;
    }
    if let Some(first) = candidates.first_mut() {
        first.suppressed_count += suppressed;
        first.step.effect_label = format!(
            "{} | compiler portfolio cap suppressed {suppressed} candidate(s)",
            first.step.effect_label
        );
    }
}

fn target_class_from_legacy(class: RunChoicePolicyClassV1) -> DeckMutationTargetClassV1 {
    match class {
        RunChoicePolicyClassV1::CursePurge => DeckMutationTargetClassV1::Curse,
        RunChoicePolicyClassV1::StarterStrikeMutation => DeckMutationTargetClassV1::StarterStrike,
        RunChoicePolicyClassV1::StarterDefendMutation => DeckMutationTargetClassV1::StarterDefend,
        RunChoicePolicyClassV1::BasicCardMutation => DeckMutationTargetClassV1::Basic,
        RunChoicePolicyClassV1::OtherDeckMutation => DeckMutationTargetClassV1::Functional,
        RunChoicePolicyClassV1::UpgradeTarget => DeckMutationTargetClassV1::UpgradeTarget,
        RunChoicePolicyClassV1::UnsupportedChoice => DeckMutationTargetClassV1::Unsupported,
    }
}

fn target_class_for_choice(
    reason: RunPendingChoiceReason,
    run_choice_class: Option<RunChoicePolicyClassV1>,
) -> DeckMutationTargetClassV1 {
    match reason {
        RunPendingChoiceReason::Upgrade => DeckMutationTargetClassV1::UpgradeTarget,
        RunPendingChoiceReason::Duplicate
        | RunPendingChoiceReason::BottleFlame
        | RunPendingChoiceReason::BottleLightning
        | RunPendingChoiceReason::BottleTornado => DeckMutationTargetClassV1::Functional,
        RunPendingChoiceReason::Purge
        | RunPendingChoiceReason::PurgeNonBottled
        | RunPendingChoiceReason::Transform
        | RunPendingChoiceReason::TransformNonBottled
        | RunPendingChoiceReason::TransformUpgraded => run_choice_class
            .map(target_class_from_legacy)
            .unwrap_or(DeckMutationTargetClassV1::Unsupported),
    }
}

impl DeckMutationTargetClassV1 {
    fn is_low_value_mutation_target(self) -> bool {
        matches!(
            self,
            DeckMutationTargetClassV1::Curse
                | DeckMutationTargetClassV1::StarterStrike
                | DeckMutationTargetClassV1::StarterDefend
                | DeckMutationTargetClassV1::Basic
        )
    }
}

fn run_choice_policy_selected_indices(
    decision: &crate::ai::run_choice_policy_v1::RunChoiceDecisionV1,
) -> Option<Vec<usize>> {
    let RunChoicePolicyActionV1::SelectDeckIndices { indices, .. } = &decision.action else {
        return None;
    };
    Some(indices.clone())
}

fn target_score_hint(reason: RunPendingChoiceReason, target: &ExactTarget) -> i32 {
    match reason {
        RunPendingChoiceReason::Duplicate => target.duplicate_priority,
        RunPendingChoiceReason::Upgrade => target.upgrade_priority.unwrap_or_default(),
        RunPendingChoiceReason::Purge
        | RunPendingChoiceReason::PurgeNonBottled
        | RunPendingChoiceReason::Transform
        | RunPendingChoiceReason::TransformNonBottled
        | RunPendingChoiceReason::TransformUpgraded => {
            -target_class_rank(target.card.target_class) - i32::from(target.card.upgrades) * 5
        }
        RunPendingChoiceReason::BottleFlame
        | RunPendingChoiceReason::BottleLightning
        | RunPendingChoiceReason::BottleTornado => 0,
    }
}

fn target_class_rank(class: DeckMutationTargetClassV1) -> i32 {
    match class {
        DeckMutationTargetClassV1::Curse => 0,
        DeckMutationTargetClassV1::StarterStrike => 10,
        DeckMutationTargetClassV1::StarterDefend => 20,
        DeckMutationTargetClassV1::Basic => 35,
        DeckMutationTargetClassV1::Functional => 100,
        DeckMutationTargetClassV1::UpgradeTarget => 10_000,
        DeckMutationTargetClassV1::Unsupported => 10_000,
    }
}

fn target_risks(class: DeckMutationTargetClassV1) -> Vec<String> {
    match class {
        DeckMutationTargetClassV1::StarterStrike => {
            vec!["mutating starter attacks can reduce short-term frontload".to_string()]
        }
        DeckMutationTargetClassV1::StarterDefend => {
            vec!["mutating starter blocks can reduce short-term defense".to_string()]
        }
        DeckMutationTargetClassV1::Functional => {
            vec!["functional card mutation requires explicit strategy context".to_string()]
        }
        DeckMutationTargetClassV1::Unsupported => {
            vec!["unsupported target is blocked from automatic consumers".to_string()]
        }
        _ => Vec::new(),
    }
}

fn is_remove_choice(reason: RunPendingChoiceReason) -> bool {
    matches!(
        reason,
        RunPendingChoiceReason::Purge | RunPendingChoiceReason::PurgeNonBottled
    )
}

fn is_transform_choice(reason: RunPendingChoiceReason) -> bool {
    matches!(
        reason,
        RunPendingChoiceReason::Transform
            | RunPendingChoiceReason::TransformNonBottled
            | RunPendingChoiceReason::TransformUpgraded
    )
}

fn deck_mutation_kind(reason: RunPendingChoiceReason) -> DeckMutationKindV1 {
    match reason {
        RunPendingChoiceReason::Purge | RunPendingChoiceReason::PurgeNonBottled => {
            DeckMutationKindV1::Remove
        }
        RunPendingChoiceReason::Upgrade => DeckMutationKindV1::Upgrade,
        RunPendingChoiceReason::Transform
        | RunPendingChoiceReason::TransformNonBottled
        | RunPendingChoiceReason::TransformUpgraded => DeckMutationKindV1::Transform,
        RunPendingChoiceReason::Duplicate => DeckMutationKindV1::Duplicate,
        RunPendingChoiceReason::BottleFlame
        | RunPendingChoiceReason::BottleLightning
        | RunPendingChoiceReason::BottleTornado => DeckMutationKindV1::Bottle,
    }
}

fn effect_kind(reason: RunPendingChoiceReason) -> &'static str {
    match reason {
        RunPendingChoiceReason::Purge | RunPendingChoiceReason::PurgeNonBottled => "remove_card",
        RunPendingChoiceReason::Upgrade => "upgrade_card",
        RunPendingChoiceReason::Transform
        | RunPendingChoiceReason::TransformNonBottled
        | RunPendingChoiceReason::TransformUpgraded => "transform_card",
        RunPendingChoiceReason::Duplicate => "duplicate_card",
        RunPendingChoiceReason::BottleFlame
        | RunPendingChoiceReason::BottleLightning
        | RunPendingChoiceReason::BottleTornado => "bottle_card",
    }
}

fn effect_verb(reason: RunPendingChoiceReason) -> &'static str {
    match reason {
        RunPendingChoiceReason::Purge | RunPendingChoiceReason::PurgeNonBottled => "remove",
        RunPendingChoiceReason::Upgrade => "upgrade",
        RunPendingChoiceReason::Transform
        | RunPendingChoiceReason::TransformNonBottled
        | RunPendingChoiceReason::TransformUpgraded => "transform",
        RunPendingChoiceReason::Duplicate => "duplicate",
        RunPendingChoiceReason::BottleFlame
        | RunPendingChoiceReason::BottleLightning
        | RunPendingChoiceReason::BottleTornado => "bottle",
    }
}

fn card_label(card: CardId, upgrades: u8) -> String {
    let name = get_card_definition(card).name;
    match upgrades {
        0 => name.to_string(),
        1 => format!("{name}+"),
        upgrades => format!("{name}+{upgrades}"),
    }
}

fn render_repeated_labels(labels: &[&str]) -> String {
    let mut runs = Vec::<(&str, usize)>::new();
    for label in labels {
        if let Some((_, count)) = runs
            .iter_mut()
            .find(|(existing_label, _)| existing_label == label)
        {
            *count += 1;
        } else {
            runs.push((label, 1));
        }
    }
    runs.into_iter()
        .map(|(label, count)| {
            if count > 1 {
                format!("{label} x{count}")
            } else {
                label.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_select_command(indices: &[usize]) -> String {
    format!(
        "select {}",
        indices
            .iter()
            .map(|idx| idx.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    )
}

fn card_stat_identity_key(card: &CombatCard) -> String {
    let mut key = format!("{:?}:{}", card.id, card.upgrades);
    let default = CombatCard::new(card.id, 0);
    let mut extras = Vec::new();

    if card.misc_value != default.misc_value {
        extras.push(format!("misc={}", card.misc_value));
    }
    if let Some(value) = card.base_damage_override {
        extras.push(format!("base_damage={value}"));
    }
    if let Some(value) = card.base_block_override {
        extras.push(format!("base_block={value}"));
    }
    if card.cost_modifier != 0 {
        extras.push(format!("cost_modifier={}", card.cost_modifier));
    }

    if !extras.is_empty() {
        key.push(':');
        key.push_str(&extras.join(":"));
    }
    key
}
