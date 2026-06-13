use std::collections::BTreeSet;

use crate::content::cards::{get_card_definition, CardId, CardRarity, CardTag, CardType};
use crate::eval::branch_experiment::BranchExperimentChoiceCardV1;
use crate::eval::run_control::RunControlSession;
use crate::runtime::combat::CombatCard;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::rewards::RewardCard;

pub(super) const MAX_RUN_SELECTION_OPTIONS_PER_BRANCH: usize = 12;
const MAX_DUPLICATE_OPTIONS_PER_BRANCH: usize = 4;

#[derive(Clone, Debug)]
pub(super) struct RunSelectionBranchOption {
    pub(super) label: String,
    pub(super) command: String,
    pub(super) card: Option<CardId>,
    pub(super) upgrades: Option<u8>,
    pub(super) selected_cards: Vec<BranchExperimentChoiceCardV1>,
    pub(super) effect_kind: String,
    pub(super) effect_key: String,
    pub(super) effect_label: String,
    pub(super) representative_count: usize,
    pub(super) suppressed_count: usize,
}

#[derive(Clone, Debug)]
struct RunSelectionDeckCardOption {
    deck_idx: usize,
    label: String,
    selected_card: BranchExperimentChoiceCardV1,
    effect_kind: String,
    effect_key: String,
    effect_label: String,
}

#[derive(Clone, Debug)]
struct RunSelectionDeckCardGroup {
    options: Vec<RunSelectionDeckCardOption>,
}

#[derive(Clone, Debug)]
struct RunSelectionGroupCountCombination {
    group_counts: Vec<usize>,
    represented_exact_count: usize,
}

pub(super) fn run_selection_branch_options(
    session: &RunControlSession,
) -> Option<Vec<RunSelectionBranchOption>> {
    let EngineState::RunPendingChoice(choice) = &session.engine_state else {
        return None;
    };
    if choice.min_choices == 0 || choice.min_choices != choice.max_choices {
        return None;
    }

    let deck_options = run_selection_deck_card_options(session, choice);
    if choice.min_choices == 1 {
        let options = filter_single_run_selection_options_for_branching(
            compressed_single_run_selection_options(deck_options),
            choice.reason,
        );
        if options.is_empty() || options.len() > MAX_RUN_SELECTION_OPTIONS_PER_BRANCH {
            if choice.reason == RunPendingChoiceReason::Duplicate {
                let duplicate_options = select_duplicate_branch_options(session, options);
                if !duplicate_options.is_empty() {
                    return Some(duplicate_options);
                }
            }
            return policy_run_selection_branch_option(session, choice).map(|option| vec![option]);
        }
        return Some(options);
    }

    compressed_multi_run_selection_options(
        deck_options,
        choice.min_choices,
        MAX_RUN_SELECTION_OPTIONS_PER_BRANCH,
    )
    .or_else(|| policy_run_selection_branch_option(session, choice).map(|option| vec![option]))
}

fn filter_single_run_selection_options_for_branching(
    options: Vec<RunSelectionBranchOption>,
    reason: RunPendingChoiceReason,
) -> Vec<RunSelectionBranchOption> {
    if !matches!(
        reason,
        RunPendingChoiceReason::Purge | RunPendingChoiceReason::PurgeNonBottled
    ) {
        return options;
    }

    let low_value_options = options
        .iter()
        .filter(|option| option.card.is_some_and(single_purge_option_is_low_value))
        .cloned()
        .collect::<Vec<_>>();
    if low_value_options.is_empty() {
        options
    } else {
        low_value_options
    }
}

fn single_purge_option_is_low_value(card: CardId) -> bool {
    let definition = get_card_definition(card);
    definition.card_type == CardType::Curse
        || definition.tags.contains(&CardTag::StarterStrike)
        || definition.tags.contains(&CardTag::StarterDefend)
        || definition.rarity == CardRarity::Basic
}

fn run_selection_deck_card_options(
    session: &RunControlSession,
    choice: &RunPendingChoiceState,
) -> Vec<RunSelectionDeckCardOption> {
    let request = choice.selection_request(&session.run_state);
    let target_uuids = request
        .targets
        .iter()
        .map(|target| match target {
            crate::state::selection::SelectionTargetRef::CardUuid(uuid) => *uuid,
        })
        .collect::<BTreeSet<_>>();
    session
        .run_state
        .master_deck
        .iter()
        .enumerate()
        .filter(|(_, card)| target_uuids.contains(&card.uuid))
        .map(|(deck_idx, card)| RunSelectionDeckCardOption {
            deck_idx,
            label: super::format_reward_card_label(&RewardCard::new(card.id, card.upgrades)),
            selected_card: BranchExperimentChoiceCardV1 {
                card: card.id,
                upgrades: card.upgrades,
            },
            effect_kind: run_selection_effect_kind(choice.reason).to_string(),
            effect_key: run_selection_effect_key(choice.reason, card),
            effect_label: run_selection_effect_label(
                choice.reason,
                &super::format_reward_card_label(&RewardCard::new(card.id, card.upgrades)),
            ),
        })
        .collect()
}

fn compressed_single_run_selection_options(
    deck_options: Vec<RunSelectionDeckCardOption>,
) -> Vec<RunSelectionBranchOption> {
    let mut groups = Vec::<(RunSelectionDeckCardOption, usize)>::new();
    for option in deck_options {
        if let Some((_, count)) = groups
            .iter_mut()
            .find(|(representative, _)| representative.effect_key == option.effect_key)
        {
            *count += 1;
        } else {
            groups.push((option, 1));
        }
    }
    groups
        .into_iter()
        .map(|(option, count)| run_selection_branch_option_from_single(option, count))
        .collect()
}

fn compressed_multi_run_selection_options(
    deck_options: Vec<RunSelectionDeckCardOption>,
    choose: usize,
    limit: usize,
) -> Option<Vec<RunSelectionBranchOption>> {
    if choose == 0 || deck_options.len() < choose {
        return None;
    }

    let groups = run_selection_effect_groups(deck_options);
    let combinations = bounded_group_count_combinations(&groups, choose, limit)?;
    if combinations.is_empty() {
        return None;
    }

    Some(
        combinations
            .into_iter()
            .map(|combo| run_selection_branch_option_from_group_counts(&groups, &combo))
            .collect(),
    )
}

fn run_selection_effect_groups(
    deck_options: Vec<RunSelectionDeckCardOption>,
) -> Vec<RunSelectionDeckCardGroup> {
    let mut groups = Vec::<RunSelectionDeckCardGroup>::new();
    for option in deck_options {
        if let Some(group) = groups
            .iter_mut()
            .find(|group| group.options[0].effect_key == option.effect_key)
        {
            group.options.push(option);
        } else {
            groups.push(RunSelectionDeckCardGroup {
                options: vec![option],
            });
        }
    }
    groups
}

fn bounded_group_count_combinations(
    groups: &[RunSelectionDeckCardGroup],
    choose: usize,
    limit: usize,
) -> Option<Vec<RunSelectionGroupCountCombination>> {
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
    groups: &[RunSelectionDeckCardGroup],
    remaining: usize,
    limit: usize,
    group_index: usize,
    group_counts: &mut [usize],
    combinations: &mut Vec<RunSelectionGroupCountCombination>,
) -> bool {
    if group_index >= groups.len() {
        if remaining == 0 {
            let represented_exact_count = group_counts
                .iter()
                .enumerate()
                .map(|(idx, count)| binomial(groups[idx].options.len(), *count))
                .product();
            combinations.push(RunSelectionGroupCountCombination {
                group_counts: group_counts.to_vec(),
                represented_exact_count,
            });
        }
        return combinations.len() <= limit;
    }

    let max_count = groups[group_index].options.len().min(remaining);
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

fn run_selection_branch_option_from_single(
    option: RunSelectionDeckCardOption,
    representative_count: usize,
) -> RunSelectionBranchOption {
    let selected_card = option.selected_card.clone();
    RunSelectionBranchOption {
        label: option.label,
        command: format_select_command(&[option.deck_idx]),
        card: Some(selected_card.card),
        upgrades: Some(selected_card.upgrades),
        selected_cards: vec![selected_card],
        effect_kind: option.effect_kind,
        effect_key: option.effect_key,
        effect_label: option.effect_label,
        representative_count,
        suppressed_count: representative_count.saturating_sub(1),
    }
}

fn run_selection_branch_option_from_group_counts(
    groups: &[RunSelectionDeckCardGroup],
    combo: &RunSelectionGroupCountCombination,
) -> RunSelectionBranchOption {
    let selected_options = combo
        .group_counts
        .iter()
        .enumerate()
        .flat_map(|(group_idx, count)| groups[group_idx].options.iter().take(*count))
        .collect::<Vec<_>>();
    let selected_cards = selected_options
        .iter()
        .map(|option| option.selected_card.clone())
        .collect::<Vec<_>>();
    let (card, upgrades) = match selected_cards.as_slice() {
        [selected] => (Some(selected.card), Some(selected.upgrades)),
        _ => (None, None),
    };
    let effect_kind = run_selection_combo_effect_kind(&selected_options);
    RunSelectionBranchOption {
        label: selected_options
            .iter()
            .map(|option| option.label.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        command: format_select_command(
            &selected_options
                .iter()
                .map(|option| option.deck_idx)
                .collect::<Vec<_>>(),
        ),
        card,
        upgrades,
        selected_cards,
        effect_key: run_selection_combo_effect_key(&selected_options),
        effect_label: run_selection_combo_effect_label(&selected_options),
        effect_kind,
        representative_count: combo.represented_exact_count,
        suppressed_count: combo.represented_exact_count.saturating_sub(1),
    }
}

fn select_duplicate_branch_options(
    session: &RunControlSession,
    mut options: Vec<RunSelectionBranchOption>,
) -> Vec<RunSelectionBranchOption> {
    options.sort_by(|left, right| {
        duplicate_option_priority(right, session)
            .cmp(&duplicate_option_priority(left, session))
            .then_with(|| left.command.cmp(&right.command))
    });
    let suppressed_count = options
        .len()
        .saturating_sub(MAX_DUPLICATE_OPTIONS_PER_BRANCH);
    options
        .into_iter()
        .take(MAX_DUPLICATE_OPTIONS_PER_BRANCH)
        .enumerate()
        .map(|(index, mut option)| {
            if index == 0 && suppressed_count > 0 {
                option.suppressed_count += suppressed_count;
                option.effect_label = format!(
                    "{} | duplicate portfolio cap suppressed {suppressed_count} candidate(s)",
                    option.effect_label
                );
            }
            option
        })
        .collect()
}

fn duplicate_option_priority(
    option: &RunSelectionBranchOption,
    session: &RunControlSession,
) -> i32 {
    let Some(card_id) = option.card else {
        return -10_000;
    };
    session
        .run_state
        .master_deck
        .iter()
        .find(|card| card.id == card_id && Some(card.upgrades) == option.upgrades)
        .map(|card| {
            crate::ai::run_choice_policy_v1::run_choice_duplicate_priority_v1(
                card,
                &session.run_state,
            )
        })
        .unwrap_or(-10_000)
}

fn policy_run_selection_branch_option(
    session: &RunControlSession,
    choice: &RunPendingChoiceState,
) -> Option<RunSelectionBranchOption> {
    let context = crate::ai::run_choice_policy_v1::build_run_choice_decision_context_v1(
        &session.run_state,
        choice,
    );
    let decision = crate::ai::run_choice_policy_v1::plan_run_choice_decision_v1(
        &context,
        &crate::ai::run_choice_policy_v1::RunChoicePolicyConfigV1::default(),
    );
    let crate::ai::run_choice_policy_v1::RunChoicePolicyActionV1::SelectDeckIndices {
        indices,
        labels,
        confidence,
        reason,
    } = decision.action
    else {
        return None;
    };
    if indices.is_empty()
        || indices.len() < choice.min_choices
        || indices.len() > choice.max_choices
    {
        return None;
    }
    let selected_cards = indices
        .iter()
        .filter_map(|idx| session.run_state.master_deck.get(*idx))
        .map(|card| BranchExperimentChoiceCardV1 {
            card: card.id,
            upgrades: card.upgrades,
        })
        .collect::<Vec<_>>();
    if selected_cards.len() != indices.len() {
        return None;
    }

    let effect_kind = run_selection_effect_kind(choice.reason).to_string();
    let effect_label = format!(
        "{} {} | confidence={confidence:.2} | {reason}",
        run_selection_effect_verb(choice.reason),
        render_repeated_option_labels(&labels.iter().map(String::as_str).collect::<Vec<_>>())
    );
    Some(RunSelectionBranchOption {
        label: effect_label.clone(),
        command: format_select_command(&indices),
        card: selected_cards.first().map(|card| card.card),
        upgrades: selected_cards.first().map(|card| card.upgrades),
        selected_cards,
        effect_kind,
        effect_key: format!(
            "run_selection:policy:{}:{}",
            run_selection_effect_kind(choice.reason),
            indices
                .iter()
                .map(|idx| idx.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ),
        effect_label,
        representative_count: 1,
        suppressed_count: 0,
    })
}

fn run_selection_effect_kind(reason: RunPendingChoiceReason) -> &'static str {
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

fn run_selection_effect_verb(reason: RunPendingChoiceReason) -> &'static str {
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

fn run_selection_effect_key(reason: RunPendingChoiceReason, card: &CombatCard) -> String {
    format!(
        "run_selection:{}:{}",
        run_selection_effect_kind(reason),
        super::card_stat_identity_key(card)
    )
}

fn run_selection_effect_label(reason: RunPendingChoiceReason, card_label: &str) -> String {
    format!("{} {card_label}", run_selection_effect_verb(reason))
}

fn run_selection_combo_effect_kind(options: &[&RunSelectionDeckCardOption]) -> String {
    let mut kinds = options
        .iter()
        .map(|option| option.effect_kind.as_str())
        .collect::<BTreeSet<_>>();
    if kinds.len() == 1 {
        kinds.pop_first().unwrap_or("run_selection").to_string()
    } else {
        "run_selection".to_string()
    }
}

fn run_selection_combo_effect_key(options: &[&RunSelectionDeckCardOption]) -> String {
    format!(
        "run_selection:combo:{}",
        options
            .iter()
            .map(|option| option.effect_key.as_str())
            .collect::<Vec<_>>()
            .join("+")
    )
}

fn run_selection_combo_effect_label(options: &[&RunSelectionDeckCardOption]) -> String {
    let verb = options
        .first()
        .and_then(|first| first.effect_label.split_once(' ').map(|(verb, _)| verb))
        .unwrap_or("select");
    format!(
        "{} {}",
        verb,
        render_repeated_option_labels(
            &options
                .iter()
                .map(|option| option.label.as_str())
                .collect::<Vec<_>>()
        )
    )
}

fn render_repeated_option_labels(labels: &[&str]) -> String {
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
