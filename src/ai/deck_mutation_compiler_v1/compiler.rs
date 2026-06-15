use std::collections::BTreeSet;

use crate::ai::card_reward_policy_v1::{card_reward_semantic_profile_v1, CardRewardSemanticRoleV1};
use crate::content::cards::{get_card_definition, CardId, CardRarity, CardTag, CardType};
use crate::runtime::combat::CombatCard;
use crate::state::core::{RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

use super::types::{
    AllowedDeckMutationConsumersV1, CompiledDeckMutationDecisionV1, DeckMutationCardSnapshotV1,
    DeckMutationCompilerModeV1, DeckMutationKindV1, DeckMutationOpeningHandDebtTierV1,
    DeckMutationOpeningHandProfileV1, DeckMutationPlanCandidateV1, DeckMutationPlanRoleV1,
    DeckMutationPlanStepV1, DeckMutationTargetClassV1, DeckMutationTargetLossTierV1,
    DeckMutationTargetLossV1,
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
    let targets = exact_targets(run_state, choice);
    let mut candidate_plans = plan_candidates(choice, &targets, mode);

    let low_value_available = targets
        .iter()
        .filter(|target| target.selectable)
        .any(|target| target.card.target_class.is_low_value_mutation_target());

    for candidate in &mut candidate_plans {
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

pub fn compile_direct_deck_mutation_plan_candidate_v1(
    run_state: &RunState,
    reason: RunPendingChoiceReason,
    deck_index: usize,
    command: String,
    effect_kind: String,
    effect_key: String,
    effect_label: String,
    low_value_available: bool,
) -> Option<DeckMutationPlanCandidateV1> {
    let target = exact_target_for_deck_index(run_state, reason, deck_index, true, None)?;
    let mut candidate = candidate_from_targets_for_reason(reason, vec![target], 1);
    candidate.plan_id = format!("deck_mutation:{effect_key}");
    candidate.step.command = command;
    candidate.step.effect_kind = effect_kind;
    candidate.step.effect_key = effect_key;
    candidate.step.effect_label = effect_label;
    candidate
        .reasons
        .push("direct event deck mutation option".to_string());
    evaluate_candidate_for_reason(reason, &mut candidate, low_value_available);
    Some(candidate)
}

pub fn deck_mutation_target_class_for_card_v1(
    reason: RunPendingChoiceReason,
    card: &CombatCard,
) -> DeckMutationTargetClassV1 {
    target_class_for_card_mutation(reason, card)
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
    run_state
        .master_deck
        .iter()
        .enumerate()
        .filter(|(_, card)| target_uuids.contains(&card.uuid))
        .filter_map(|(deck_index, _card)| {
            exact_target_for_deck_index(
                run_state,
                choice.reason,
                deck_index,
                run_state.master_deck.get(deck_index).is_some_and(|card| {
                    crate::state::core::run_pending_choice_allows_card_for_run(
                        &choice.reason,
                        card,
                        run_state,
                    )
                }),
                run_state.master_deck.get(deck_index).and_then(|card| {
                    matches!(choice.reason, RunPendingChoiceReason::Upgrade).then(|| {
                        crate::ai::campfire_policy_v1::campfire_smith_upgrade_priority_v1(
                            card, run_state,
                        )
                    })
                }),
            )
        })
        .collect()
}

fn exact_target_for_deck_index(
    run_state: &RunState,
    reason: RunPendingChoiceReason,
    deck_index: usize,
    selectable: bool,
    upgrade_priority: Option<i32>,
) -> Option<ExactTarget> {
    let card = run_state.master_deck.get(deck_index)?;
    let target_class = target_class_for_card_mutation(reason, card);
    Some(ExactTarget {
        card: DeckMutationCardSnapshotV1 {
            deck_index,
            card: card.id,
            upgrades: card.upgrades,
            label: card_label(card.id, card.upgrades),
            target_class,
            target_loss: target_loss_for_card_mutation(run_state, reason, card, target_class),
            opening_hand: opening_hand_profile_for_card_mutation(run_state, reason, card),
        },
        identity_key: card_stat_identity_key(card),
        selectable,
        upgrade_priority,
        duplicate_priority: run_choice_duplicate_priority_v1(card, run_state),
    })
}

fn plan_candidates(
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
        greedy_multi_candidate(choice, targets)
            .into_iter()
            .collect()
    })
}

fn greedy_multi_candidate(
    choice: &RunPendingChoiceState,
    targets: &[ExactTarget],
) -> Option<DeckMutationPlanCandidateV1> {
    let mut selectable = targets
        .iter()
        .filter(|target| target.selectable)
        .cloned()
        .collect::<Vec<_>>();
    if selectable.len() < choice.min_choices {
        return None;
    }
    selectable.sort_by(|left, right| {
        target_score_hint(choice.reason, right)
            .cmp(&target_score_hint(choice.reason, left))
            .then_with(|| left.card.deck_index.cmp(&right.card.deck_index))
    });
    let selected = selectable
        .into_iter()
        .take(choice.min_choices)
        .collect::<Vec<_>>();
    let mut candidate = candidate_from_targets(choice, selected, 1);
    candidate
        .reasons
        .push("bounded compiler fallback selected a greedy representative".to_string());
    Some(candidate)
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

fn candidate_from_targets(
    choice: &RunPendingChoiceState,
    selected_targets: Vec<ExactTarget>,
    representative_count: usize,
) -> DeckMutationPlanCandidateV1 {
    candidate_from_targets_for_reason(choice.reason, selected_targets, representative_count)
}

fn candidate_from_targets_for_reason(
    reason: RunPendingChoiceReason,
    selected_targets: Vec<ExactTarget>,
    representative_count: usize,
) -> DeckMutationPlanCandidateV1 {
    let kind = deck_mutation_kind(reason);
    let cards = selected_targets
        .iter()
        .map(|target| target.card.clone())
        .collect::<Vec<_>>();
    let deck_indices = cards.iter().map(|card| card.deck_index).collect::<Vec<_>>();
    let labels = cards
        .iter()
        .map(|card| card.label.as_str())
        .collect::<Vec<_>>();
    let effect_kind = effect_kind(reason).to_string();
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
        effect_verb(reason),
        render_repeated_labels(&labels)
    );
    let score_hint = selected_targets
        .iter()
        .map(|target| target_score_hint(reason, target))
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
        score_hint,
        confidence: 0.0,
        reasons: selected_targets
            .iter()
            .flat_map(|target| target_loss_reasons(&target.card))
            .chain(std::iter::once(format!("candidate for {:?}", reason)))
            .collect(),
        risks: selected_targets
            .iter()
            .flat_map(|target| target_risks(&target.card))
            .collect(),
    }
}

fn evaluate_candidate(
    choice: &RunPendingChoiceState,
    candidate: &mut DeckMutationPlanCandidateV1,
    low_value_available: bool,
) {
    evaluate_candidate_for_reason(choice.reason, candidate, low_value_available);
}

fn evaluate_candidate_for_reason(
    reason: RunPendingChoiceReason,
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
    let mutation_choice = is_remove_choice(reason) || is_transform_choice(reason);
    let bottle_choice = is_bottle_choice(reason);
    let max_opening_debt = candidate
        .step
        .cards
        .iter()
        .map(|card| card.opening_hand.debt_tier)
        .max()
        .unwrap_or_default();

    let role = if has_unsupported_target {
        DeckMutationPlanRoleV1::Blocked
    } else if bottle_choice && max_opening_debt >= DeckMutationOpeningHandDebtTierV1::Situational {
        DeckMutationPlanRoleV1::InspectOnly
    } else if mutation_choice && has_functional_target && low_value_available {
        DeckMutationPlanRoleV1::InspectOnly
    } else if mutation_choice && has_functional_target {
        DeckMutationPlanRoleV1::RiskyExploration
    } else if matches!(reason, RunPendingChoiceReason::Upgrade)
        && candidate.score_hint >= clear_upgrade_priority_threshold()
    {
        DeckMutationPlanRoleV1::PolicyPreferred
    } else if mutation_choice && all_low_value {
        DeckMutationPlanRoleV1::SafeAlternative
    } else if matches!(reason, RunPendingChoiceReason::Upgrade) {
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
    candidate.allowed_consumers = allowed_consumers(reason, role, candidate);
    candidate.reasons.push(format!("role={role:?}"));
    if mutation_choice && has_functional_target && low_value_available {
        candidate.reasons.push(
            "functional deck mutation target is inspect-only while low-value targets exist"
                .to_string(),
        );
    }
    if bottle_choice && max_opening_debt >= DeckMutationOpeningHandDebtTierV1::Situational {
        candidate.reasons.push(format!(
            "bottle target has {:?} opening-hand debt",
            max_opening_debt
        ));
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
        DeckMutationPlanRoleV1::SafeAlternative => {
            (is_remove_choice(reason) || is_transform_choice(reason)) && all_low_value
        }
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

fn clear_upgrade_priority_threshold() -> i32 {
    crate::ai::campfire_policy_v1::CampfirePolicyConfigV1::default()
        .clear_core_smith_priority_threshold
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

fn target_class_for_card_mutation(
    reason: RunPendingChoiceReason,
    card: &CombatCard,
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
        | RunPendingChoiceReason::TransformUpgraded => {
            target_class_for_card_mutation_candidate(card)
        }
    }
}

fn target_class_for_card_mutation_candidate(card: &CombatCard) -> DeckMutationTargetClassV1 {
    let definition = get_card_definition(card.id);
    if definition.card_type == CardType::Curse {
        DeckMutationTargetClassV1::Curse
    } else if definition.tags.contains(&CardTag::StarterStrike) {
        DeckMutationTargetClassV1::StarterStrike
    } else if definition.tags.contains(&CardTag::StarterDefend) {
        DeckMutationTargetClassV1::StarterDefend
    } else if definition.rarity == CardRarity::Basic {
        DeckMutationTargetClassV1::Basic
    } else {
        DeckMutationTargetClassV1::Functional
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

fn target_score_hint(reason: RunPendingChoiceReason, target: &ExactTarget) -> i32 {
    match reason {
        RunPendingChoiceReason::Duplicate => target.duplicate_priority,
        RunPendingChoiceReason::Upgrade => target.upgrade_priority.unwrap_or_default(),
        RunPendingChoiceReason::Purge
        | RunPendingChoiceReason::PurgeNonBottled
        | RunPendingChoiceReason::Transform
        | RunPendingChoiceReason::TransformNonBottled
        | RunPendingChoiceReason::TransformUpgraded => {
            -target_class_rank(target.card.target_class)
                - target_loss_rank(target.card.target_loss.tier)
                - i32::from(target.card.upgrades) * 5
        }
        RunPendingChoiceReason::BottleFlame
        | RunPendingChoiceReason::BottleLightning
        | RunPendingChoiceReason::BottleTornado => 0,
    }
}

fn run_choice_duplicate_priority_v1(card: &CombatCard, run_state: &RunState) -> i32 {
    let def = get_card_definition(card.id);
    if matches!(def.card_type, CardType::Curse | CardType::Status) {
        return -10_000;
    }

    let profile = card_reward_semantic_profile_v1(&RewardCard::new(card.id, card.upgrades));
    let mut priority = 100;

    for role in profile.roles {
        priority += match role {
            CardRewardSemanticRoleV1::CardDraw => 180,
            CardRewardSemanticRoleV1::EnergySource => 170,
            CardRewardSemanticRoleV1::EnemyStrengthDown
            | CardRewardSemanticRoleV1::Weak
            | CardRewardSemanticRoleV1::Vulnerable => 160,
            CardRewardSemanticRoleV1::ScalingSource => 150,
            CardRewardSemanticRoleV1::TemporaryStrengthBurst => 70,
            CardRewardSemanticRoleV1::Block
            | CardRewardSemanticRoleV1::BlockRetention
            | CardRewardSemanticRoleV1::BlockMultiplier => 130,
            CardRewardSemanticRoleV1::ExhaustGenerator => 120,
            CardRewardSemanticRoleV1::PackagePayoff
            | CardRewardSemanticRoleV1::ExhaustPayoff
            | CardRewardSemanticRoleV1::StatusPayoff
            | CardRewardSemanticRoleV1::BlockPayoff
            | CardRewardSemanticRoleV1::StrengthPayoff
            | CardRewardSemanticRoleV1::StrikePayoff
            | CardRewardSemanticRoleV1::UpgradePayoff
            | CardRewardSemanticRoleV1::SelfDamagePayoff => 90,
            CardRewardSemanticRoleV1::FrontloadDamage => 80,
            CardRewardSemanticRoleV1::AoeDamage => 60,
            CardRewardSemanticRoleV1::RandomOutput => -50,
            CardRewardSemanticRoleV1::ConditionalPlayability => -80,
            CardRewardSemanticRoleV1::UnsupportedMechanics => -120,
            CardRewardSemanticRoleV1::StatusGenerator => -40,
        };
    }

    priority += high_impact_duplicate_bonus(card.id);
    if card.upgrades > 0 {
        priority += 60;
    }
    if supports_existing_deck_package(card.id, run_state) {
        priority += 100;
    }
    if def.tags.contains(&CardTag::StarterStrike) || def.tags.contains(&CardTag::StarterDefend) {
        priority -= 500;
    }
    if def.rarity == CardRarity::Basic {
        priority -= 300;
    }

    priority
}

fn high_impact_duplicate_bonus(card: CardId) -> i32 {
    match card {
        CardId::Offering | CardId::Corruption => 520,
        CardId::Shockwave | CardId::Disarm => 480,
        CardId::Impervious | CardId::DemonForm | CardId::Feed | CardId::Reaper => 400,
        CardId::FiendFire | CardId::DarkEmbrace | CardId::FeelNoPain => 360,
        CardId::BattleTrance | CardId::BurningPact | CardId::PowerThrough => 300,
        CardId::FlameBarrier | CardId::Entrench | CardId::Barricade | CardId::BodySlam => 250,
        CardId::Uppercut | CardId::ShrugItOff | CardId::PommelStrike | CardId::TrueGrit => 220,
        CardId::Armaments | CardId::SpotWeakness | CardId::Inflame => 180,
        _ => 0,
    }
}

fn supports_existing_deck_package(card: CardId, run_state: &RunState) -> bool {
    match card {
        CardId::BodySlam | CardId::Entrench | CardId::Barricade => deck_has_any(
            run_state,
            &[CardId::BodySlam, CardId::Entrench, CardId::Barricade],
        ),
        CardId::HeavyBlade | CardId::LimitBreak => {
            let startup = crate::ai::deck_startup_profile_v1::deck_startup_profile_v1(run_state);
            startup.persistent_strength_source_count > 0
                || startup.convertible_strength_source_count > 0
        }
        CardId::FeelNoPain | CardId::DarkEmbrace | CardId::Corruption => deck_has_any(
            run_state,
            &[
                CardId::FeelNoPain,
                CardId::DarkEmbrace,
                CardId::Corruption,
                CardId::SecondWind,
                CardId::FiendFire,
                CardId::TrueGrit,
            ],
        ),
        CardId::Evolve | CardId::FireBreathing => deck_has_any(
            run_state,
            &[
                CardId::Evolve,
                CardId::FireBreathing,
                CardId::PowerThrough,
                CardId::WildStrike,
                CardId::RecklessCharge,
            ],
        ),
        _ => false,
    }
}

fn deck_has_any(run_state: &RunState, cards: &[CardId]) -> bool {
    run_state
        .master_deck
        .iter()
        .any(|card| cards.contains(&card.id))
}

fn target_loss_rank(tier: DeckMutationTargetLossTierV1) -> i32 {
    match tier {
        DeckMutationTargetLossTierV1::LowValue => 0,
        DeckMutationTargetLossTierV1::RedundantFunctional => 25,
        DeckMutationTargetLossTierV1::Functional => 60,
        DeckMutationTargetLossTierV1::CoreFunctional => 120,
        DeckMutationTargetLossTierV1::Unsupported => 10_000,
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

fn target_loss_for_card_mutation(
    run_state: &RunState,
    reason: RunPendingChoiceReason,
    card: &CombatCard,
    target_class: DeckMutationTargetClassV1,
) -> DeckMutationTargetLossV1 {
    let same_card_count = run_state
        .master_deck
        .iter()
        .filter(|deck_card| deck_card.id == card.id)
        .count();
    let mut loss = DeckMutationTargetLossV1 {
        same_card_count,
        ..DeckMutationTargetLossV1::default()
    };

    if !(is_remove_choice(reason) || is_transform_choice(reason)) {
        loss.tier = DeckMutationTargetLossTierV1::LowValue;
        return loss;
    }

    if target_class == DeckMutationTargetClassV1::Unsupported {
        loss.tier = DeckMutationTargetLossTierV1::Unsupported;
        loss.signals.push("unsupported_target".to_string());
        return loss;
    }

    if target_class.is_low_value_mutation_target() {
        loss.tier = DeckMutationTargetLossTierV1::LowValue;
        loss.signals.push(format!("target_class={target_class:?}"));
        return loss;
    }

    let semantic = card_reward_semantic_profile_v1(&RewardCard::new(card.id, card.upgrades));
    let mut has_core_signal = false;
    for role in &semantic.roles {
        if let Some(signal) = target_loss_signal_for_role(*role) {
            loss.signals.push(signal.to_string());
            if target_loss_role_is_core(*role) {
                has_core_signal = true;
            }
        }
    }
    if card.upgrades > 0 {
        loss.signals.push("upgraded_target".to_string());
    }
    if same_card_count > 1 {
        loss.signals
            .push(format!("same_card_count={same_card_count}"));
    }

    loss.tier = if same_card_count > 1 && !has_core_signal {
        DeckMutationTargetLossTierV1::RedundantFunctional
    } else if same_card_count > 1 {
        DeckMutationTargetLossTierV1::Functional
    } else if has_core_signal {
        DeckMutationTargetLossTierV1::CoreFunctional
    } else {
        DeckMutationTargetLossTierV1::Functional
    };
    loss
}

fn opening_hand_profile_for_card_mutation(
    run_state: &RunState,
    reason: RunPendingChoiceReason,
    card: &CombatCard,
) -> DeckMutationOpeningHandProfileV1 {
    if !is_bottle_choice(reason) {
        return DeckMutationOpeningHandProfileV1::default();
    }

    let semantic = card_reward_semantic_profile_v1(&RewardCard::new(card.id, card.upgrades));
    let has = |role| semantic.roles.contains(&role);
    let mut profile = DeckMutationOpeningHandProfileV1 {
        debt_tier: DeckMutationOpeningHandDebtTierV1::Mild,
        signals: vec!["bottle_opening_hand_mutation".to_string()],
    };

    let definition = get_card_definition(card.id);
    match reason {
        RunPendingChoiceReason::BottleLightning => {
            if has(CardRewardSemanticRoleV1::CardDraw)
                || has(CardRewardSemanticRoleV1::EnergySource)
                || has(CardRewardSemanticRoleV1::Weak)
                || has(CardRewardSemanticRoleV1::EnemyStrengthDown)
            {
                profile.debt_tier = DeckMutationOpeningHandDebtTierV1::None;
                profile
                    .signals
                    .push("skill_bottle_has_immediate_access_role".to_string());
            } else if has(CardRewardSemanticRoleV1::TemporaryStrengthBurst)
                || has(CardRewardSemanticRoleV1::ConditionalPlayability)
                || has(CardRewardSemanticRoleV1::RandomOutput)
            {
                profile.debt_tier = DeckMutationOpeningHandDebtTierV1::Situational;
                profile
                    .signals
                    .push("skill_bottle_needs_turn_specific_context".to_string());
            }
        }
        RunPendingChoiceReason::BottleTornado => {
            let context_dependent_power = has(CardRewardSemanticRoleV1::StatusPayoff)
                || has(CardRewardSemanticRoleV1::PackagePayoff)
                || has(CardRewardSemanticRoleV1::ConditionalPlayability)
                || has(CardRewardSemanticRoleV1::RandomOutput);
            let setup_power = has(CardRewardSemanticRoleV1::ScalingSource)
                || has(CardRewardSemanticRoleV1::ExhaustGenerator)
                || has(CardRewardSemanticRoleV1::ExhaustPayoff)
                || has(CardRewardSemanticRoleV1::BlockRetention);
            let awakened_one_boss = run_state.boss_key
                == Some(crate::content::monsters::factory::EncounterId::AwakenedOne);

            if awakened_one_boss && context_dependent_power {
                profile.debt_tier = DeckMutationOpeningHandDebtTierV1::High;
                profile.signals.push(
                    "context_dependent_power_bottle_conflicts_with_awakened_one_pressure"
                        .to_string(),
                );
            } else if context_dependent_power {
                profile.debt_tier = DeckMutationOpeningHandDebtTierV1::Situational;
                profile
                    .signals
                    .push("power_bottle_is_package_or_context_dependent".to_string());
            } else if setup_power {
                profile.debt_tier = DeckMutationOpeningHandDebtTierV1::Mild;
                profile
                    .signals
                    .push("power_bottle_has_setup_role".to_string());
            } else if awakened_one_boss {
                profile.debt_tier = DeckMutationOpeningHandDebtTierV1::Situational;
                profile
                    .signals
                    .push("power_bottle_has_awakened_one_pressure".to_string());
            }
        }
        RunPendingChoiceReason::BottleFlame => {
            if has(CardRewardSemanticRoleV1::FrontloadDamage)
                || has(CardRewardSemanticRoleV1::AoeDamage)
                || has(CardRewardSemanticRoleV1::Vulnerable)
            {
                profile.debt_tier = DeckMutationOpeningHandDebtTierV1::None;
                profile
                    .signals
                    .push("attack_bottle_has_immediate_damage_role".to_string());
            } else if has(CardRewardSemanticRoleV1::StrengthPayoff)
                || has(CardRewardSemanticRoleV1::BlockPayoff)
                || has(CardRewardSemanticRoleV1::ConditionalPlayability)
            {
                profile.debt_tier = DeckMutationOpeningHandDebtTierV1::Situational;
                profile
                    .signals
                    .push("attack_bottle_payoff_needs_support".to_string());
            }
        }
        _ => {}
    }

    if definition.cost >= 2 && profile.debt_tier < DeckMutationOpeningHandDebtTierV1::High {
        profile
            .signals
            .push("bottle_target_has_high_opening_energy_cost".to_string());
        if profile.debt_tier < DeckMutationOpeningHandDebtTierV1::Mild {
            profile.debt_tier = DeckMutationOpeningHandDebtTierV1::Mild;
        }
    }

    profile
}

fn target_loss_signal_for_role(role: CardRewardSemanticRoleV1) -> Option<&'static str> {
    match role {
        CardRewardSemanticRoleV1::FrontloadDamage => Some("frontload_damage"),
        CardRewardSemanticRoleV1::AoeDamage => Some("aoe_damage"),
        CardRewardSemanticRoleV1::Block => Some("block"),
        CardRewardSemanticRoleV1::BlockRetention => Some("block_retention"),
        CardRewardSemanticRoleV1::BlockMultiplier => Some("block_multiplier"),
        CardRewardSemanticRoleV1::CardDraw => Some("card_draw"),
        CardRewardSemanticRoleV1::EnergySource => Some("energy_source"),
        CardRewardSemanticRoleV1::Vulnerable => Some("vulnerable"),
        CardRewardSemanticRoleV1::Weak => Some("weak"),
        CardRewardSemanticRoleV1::EnemyStrengthDown => Some("enemy_strength_down"),
        CardRewardSemanticRoleV1::ScalingSource => Some("scaling_source"),
        CardRewardSemanticRoleV1::TemporaryStrengthBurst => Some("temporary_strength_burst"),
        CardRewardSemanticRoleV1::StrengthPayoff => Some("strength_payoff"),
        CardRewardSemanticRoleV1::BlockPayoff => Some("block_payoff"),
        CardRewardSemanticRoleV1::StrikePayoff => Some("strike_payoff"),
        CardRewardSemanticRoleV1::UpgradePayoff => Some("upgrade_payoff"),
        CardRewardSemanticRoleV1::ExhaustGenerator => Some("exhaust_generator"),
        CardRewardSemanticRoleV1::ExhaustPayoff => Some("exhaust_payoff"),
        CardRewardSemanticRoleV1::StatusGenerator => Some("status_generator"),
        CardRewardSemanticRoleV1::StatusPayoff => Some("status_payoff"),
        CardRewardSemanticRoleV1::SelfDamagePayoff => Some("self_damage_payoff"),
        CardRewardSemanticRoleV1::PackagePayoff => None,
        CardRewardSemanticRoleV1::RandomOutput => Some("random_output"),
        CardRewardSemanticRoleV1::ConditionalPlayability => Some("conditional_playability"),
        CardRewardSemanticRoleV1::UnsupportedMechanics => Some("unsupported_mechanics"),
    }
}

fn target_loss_role_is_core(role: CardRewardSemanticRoleV1) -> bool {
    matches!(
        role,
        CardRewardSemanticRoleV1::BlockRetention
            | CardRewardSemanticRoleV1::BlockMultiplier
            | CardRewardSemanticRoleV1::CardDraw
            | CardRewardSemanticRoleV1::EnergySource
            | CardRewardSemanticRoleV1::Vulnerable
            | CardRewardSemanticRoleV1::Weak
            | CardRewardSemanticRoleV1::EnemyStrengthDown
            | CardRewardSemanticRoleV1::ScalingSource
            | CardRewardSemanticRoleV1::BlockPayoff
            | CardRewardSemanticRoleV1::ExhaustGenerator
            | CardRewardSemanticRoleV1::ExhaustPayoff
            | CardRewardSemanticRoleV1::StatusPayoff
            | CardRewardSemanticRoleV1::SelfDamagePayoff
    )
}

fn target_loss_reasons(card: &DeckMutationCardSnapshotV1) -> Vec<String> {
    let mut reasons = vec![format!(
        "target_loss={:?} same_card_count={}",
        card.target_loss.tier, card.target_loss.same_card_count
    )];
    if !card.target_loss.signals.is_empty() {
        reasons.push(format!(
            "target_loss_signals={}",
            card.target_loss.signals.join(",")
        ));
    }
    if card.opening_hand.debt_tier != DeckMutationOpeningHandDebtTierV1::None {
        reasons.push(format!(
            "opening_hand_debt={:?}",
            card.opening_hand.debt_tier
        ));
    }
    if !card.opening_hand.signals.is_empty() {
        reasons.push(format!(
            "opening_hand_signals={}",
            card.opening_hand.signals.join(",")
        ));
    }
    reasons
}

fn target_risks(card: &DeckMutationCardSnapshotV1) -> Vec<String> {
    let mut risks = match card.target_class {
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
    };
    match card.target_loss.tier {
        DeckMutationTargetLossTierV1::CoreFunctional => {
            risks.push("target carries singleton core deck function".to_string());
        }
        DeckMutationTargetLossTierV1::Functional => {
            risks.push("target carries replaceable but real deck function".to_string());
        }
        DeckMutationTargetLossTierV1::Unsupported => {
            risks.push("target loss profile is unsupported".to_string());
        }
        DeckMutationTargetLossTierV1::LowValue
        | DeckMutationTargetLossTierV1::RedundantFunctional => {}
    }
    match card.opening_hand.debt_tier {
        DeckMutationOpeningHandDebtTierV1::High => {
            risks.push("bottle target creates high opening-hand debt".to_string());
        }
        DeckMutationOpeningHandDebtTierV1::Situational => {
            risks.push("bottle target is context-dependent in the opening hand".to_string());
        }
        DeckMutationOpeningHandDebtTierV1::Mild | DeckMutationOpeningHandDebtTierV1::None => {}
    }
    risks
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

fn is_bottle_choice(reason: RunPendingChoiceReason) -> bool {
    matches!(
        reason,
        RunPendingChoiceReason::BottleFlame
            | RunPendingChoiceReason::BottleLightning
            | RunPendingChoiceReason::BottleTornado
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
