use std::collections::{BTreeMap, BTreeSet};

use crate::ai::card_admission_policy_v1::{
    evaluate_card_admission_v1, CardAdmissionSourceV1, CardAdmissionVerdictV1,
};
use crate::ai::card_reward_policy_v1::{
    card_reward_semantic_profile_v1, CardRewardSemanticProfileV1, CardRewardSemanticRoleV1,
};
use crate::content::cards::CardId;
use crate::eval::branch_experiment::{
    BranchExperimentRewardOptionPortfolioEntryV1, BranchExperimentRewardOptionPortfolioV1,
};
use crate::eval::branch_experiment_trajectory::summarize_branch_trajectory_v1;
use crate::eval::run_control::RunControlSession;
use crate::runtime::action::CardDestination;
use crate::state::core::{EngineState, PendingChoice};
use crate::state::rewards::{RewardCard, RewardItem};

#[derive(Clone, Debug)]
pub(crate) struct CardRewardBranchOption {
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) card: CardId,
    pub(crate) upgrades: u8,
    pub(crate) source: CardRewardBranchOptionSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CardRewardBranchOptionSource {
    PermanentReward,
    CombatGeneratedToHand,
}

#[derive(Clone, Debug)]
pub(crate) struct CardRewardBranchOptionSelection {
    pub(crate) options: Vec<CardRewardBranchOption>,
    pub(crate) portfolio: Option<BranchExperimentRewardOptionPortfolioV1>,
}

#[derive(Clone, Debug)]
pub(crate) struct CardRewardPortfolioContext {
    pub(crate) depth: usize,
    pub(crate) frontier_key: String,
    pub(crate) boundary_title: String,
}

pub(crate) fn card_reward_branch_options(
    session: &RunControlSession,
) -> Option<Vec<CardRewardBranchOption>> {
    let source = card_reward_option_source(session)?;
    let cards = active_or_visible_reward_cards(session)?;
    let options = cards
        .iter()
        .enumerate()
        .map(|(idx, card)| CardRewardBranchOption {
            label: format_reward_card_label(card),
            command: match source {
                CardRewardBranchOptionSource::PermanentReward => format!("rp {idx}"),
                CardRewardBranchOptionSource::CombatGeneratedToHand => format!("choose {idx}"),
            },
            card: card.id,
            upgrades: card.upgrades,
            source,
        })
        .collect::<Vec<_>>();
    if options.is_empty() {
        return None;
    }
    Some(options)
}

pub(crate) fn select_card_reward_branch_options_for_session(
    session: &RunControlSession,
    options: Vec<CardRewardBranchOption>,
    max_reward_options_per_branch: Option<usize>,
    portfolio_context: Option<CardRewardPortfolioContext>,
) -> CardRewardBranchOptionSelection {
    if options
        .iter()
        .all(|option| option.source == CardRewardBranchOptionSource::CombatGeneratedToHand)
    {
        return CardRewardBranchOptionSelection {
            options,
            portfolio: None,
        };
    }
    let Some(limit) = max_reward_options_per_branch else {
        return CardRewardBranchOptionSelection {
            options,
            portfolio: None,
        };
    };
    select_card_reward_branch_options_with_limit_and_admission(
        options,
        limit,
        portfolio_context,
        Some(session),
    )
}

#[cfg(test)]
pub(crate) fn select_card_reward_branch_options_with_limit(
    options: Vec<CardRewardBranchOption>,
    limit: usize,
    portfolio_context: Option<CardRewardPortfolioContext>,
) -> CardRewardBranchOptionSelection {
    select_card_reward_branch_options_with_limit_and_admission(
        options,
        limit,
        portfolio_context,
        None,
    )
}

fn select_card_reward_branch_options_with_limit_and_admission(
    options: Vec<CardRewardBranchOption>,
    limit: usize,
    portfolio_context: Option<CardRewardPortfolioContext>,
    session: Option<&RunControlSession>,
) -> CardRewardBranchOptionSelection {
    let capped_limit = limit.min(options.len());
    if options.len() <= capped_limit {
        return CardRewardBranchOptionSelection {
            options,
            portfolio: None,
        };
    }

    let mut annotated = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let profile =
                card_reward_semantic_profile_v1(&RewardCard::new(option.card, option.upgrades));
            let (priority, class_key) = reward_option_semantic_class(&profile);
            let admission = reward_option_admission_order(session, option);
            (index, admission, priority, class_key)
        })
        .collect::<Vec<_>>();
    annotated.sort_by(|left, right| {
        left.1
            .cmp(&right.1)
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut selected = Vec::new();
    let mut selected_classes = BTreeSet::new();
    for (index, _, _, class_key) in &annotated {
        if selected.len() >= limit {
            break;
        }
        if selected_classes.insert(class_key.clone()) {
            selected.push(*index);
        }
    }
    for index in 0..options.len() {
        if selected.len() >= limit {
            break;
        }
        if !selected.contains(&index) {
            selected.push(index);
        }
    }

    selected.sort_unstable();
    let selected_indices = selected.iter().copied().collect::<BTreeSet<_>>();
    let portfolio = portfolio_context.map(|context| {
        reward_option_portfolio_report(
            context.depth,
            context.frontier_key,
            context.boundary_title,
            limit,
            &options,
            &annotated,
            &selected_indices,
        )
    });
    let options = options
        .into_iter()
        .enumerate()
        .filter_map(|(index, option)| selected_indices.contains(&index).then_some(option))
        .collect();
    CardRewardBranchOptionSelection { options, portfolio }
}

fn reward_option_portfolio_report(
    depth: usize,
    frontier_key: String,
    boundary_title: String,
    max_reward_options_per_branch: usize,
    options: &[CardRewardBranchOption],
    annotated: &[(usize, usize, usize, String)],
    selected_indices: &BTreeSet<usize>,
) -> BranchExperimentRewardOptionPortfolioV1 {
    let class_by_index = annotated
        .iter()
        .map(|(index, admission, _, class_key)| {
            (*index, format!("admission={admission}:{class_key}"))
        })
        .collect::<BTreeMap<_, _>>();
    let mut selected_options = Vec::new();
    let mut pruned_options = Vec::new();

    for (index, option) in options.iter().enumerate() {
        let entry = BranchExperimentRewardOptionPortfolioEntryV1 {
            command: option.command.clone(),
            label: option.label.clone(),
            semantic_class: class_by_index
                .get(&index)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
        };
        if selected_indices.contains(&index) {
            selected_options.push(entry);
        } else {
            pruned_options.push(entry);
        }
    }

    BranchExperimentRewardOptionPortfolioV1 {
        depth,
        frontier_key,
        boundary_title,
        max_reward_options_per_branch,
        original_count: options.len(),
        selected_count: selected_options.len(),
        selected_options,
        pruned_options,
    }
}

fn reward_option_admission_order(
    session: Option<&RunControlSession>,
    option: &CardRewardBranchOption,
) -> usize {
    let Some(session) = session else {
        return 0;
    };
    match evaluate_card_admission_v1(
        &session.run_state,
        RewardCard::new(option.card, option.upgrades),
        CardAdmissionSourceV1::Reward,
    )
    .verdict
    {
        CardAdmissionVerdictV1::Admit => 0,
        CardAdmissionVerdictV1::AdmitIfNoCleanerAlternative => 1,
        CardAdmissionVerdictV1::Reject => 4,
    }
}

pub(super) fn reward_option_semantic_class(
    profile: &CardRewardSemanticProfileV1,
) -> (usize, String) {
    let signature = summarize_branch_trajectory_v1(std::slice::from_ref(profile));
    let setup = join_or_dash(&signature.setup_keys);
    let package = join_or_dash(&signature.package_keys);
    if !signature.setup_keys.is_empty() && !signature.package_keys.is_empty() {
        return (0, format!("closed_package:{setup}->{package}"));
    }
    if !signature.package_keys.is_empty() {
        return (1, format!("payoff:{package}"));
    }
    if !signature.setup_keys.is_empty() {
        return (2, format!("setup:{setup}"));
    }
    if signature.defense_picks > 0 || signature.draw_energy_picks > 0 {
        return (3, format!("stabilizer:{}", stabilizer_role_key(profile)));
    }
    if signature.transition_frontload_picks > 0 {
        return (4, "pure_transition_frontload".to_string());
    }
    (5, "other".to_string())
}

fn join_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values.join("+")
    }
}

fn stabilizer_role_key(profile: &CardRewardSemanticProfileV1) -> String {
    let roles = profile
        .roles
        .iter()
        .filter(|role| {
            !matches!(
                role,
                CardRewardSemanticRoleV1::FrontloadDamage
                    | CardRewardSemanticRoleV1::AoeDamage
                    | CardRewardSemanticRoleV1::PackagePayoff
            )
        })
        .map(|role| format!("{role:?}"))
        .collect::<Vec<_>>();
    if roles.is_empty() {
        "none".to_string()
    } else {
        roles.join("+")
    }
}

pub(crate) fn active_or_visible_reward_cards(
    session: &RunControlSession,
) -> Option<Vec<RewardCard>> {
    match &session.engine_state {
        EngineState::RewardScreen(reward) => reward
            .pending_card_choice
            .clone()
            .or_else(|| first_visible_card_reward(reward)),
        EngineState::RewardOverlay { reward_state, .. } => reward_state
            .pending_card_choice
            .clone()
            .or_else(|| first_visible_card_reward(reward_state)),
        EngineState::PendingChoice(PendingChoice::CardRewardSelect {
            cards,
            destination: CardDestination::Hand,
            can_skip: false,
        }) => Some(
            cards
                .iter()
                .copied()
                .map(|card| RewardCard::new(card, 0))
                .collect(),
        ),
        _ => None,
    }
}

fn card_reward_option_source(session: &RunControlSession) -> Option<CardRewardBranchOptionSource> {
    match &session.engine_state {
        EngineState::RewardScreen(_) | EngineState::RewardOverlay { .. } => {
            Some(CardRewardBranchOptionSource::PermanentReward)
        }
        EngineState::PendingChoice(PendingChoice::CardRewardSelect {
            destination: CardDestination::Hand,
            can_skip: false,
            ..
        }) => Some(CardRewardBranchOptionSource::CombatGeneratedToHand),
        _ => None,
    }
}

fn first_visible_card_reward(
    reward: &crate::state::rewards::RewardState,
) -> Option<Vec<RewardCard>> {
    reward.items.iter().find_map(|item| match item {
        RewardItem::Card { cards } => Some(cards.clone()),
        _ => None,
    })
}

pub(crate) fn card_offer_labels(cards: Vec<RewardCard>) -> Vec<String> {
    cards
        .into_iter()
        .map(|card| format_reward_card_label(&card))
        .collect()
}

pub(super) fn format_reward_card_label(card: &RewardCard) -> String {
    let name = crate::content::cards::get_card_definition(card.id).name;
    match card.upgrades {
        0 => name.to_string(),
        1 => format!("{name}+"),
        upgrades => format!("{name}+{upgrades}"),
    }
}
