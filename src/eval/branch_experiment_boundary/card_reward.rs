use std::collections::{BTreeMap, BTreeSet};

use crate::ai::card_reward_policy_v1::{
    card_reward_semantic_profile_v1, CardRewardSemanticProfileV1, CardRewardSemanticRoleV1,
};
use crate::ai::strategic::{AcquisitionVerdict, CandidateAction};
use crate::content::cards::CardId;
use crate::content::relics::RelicId;
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
    pub(crate) card: Option<CardId>,
    pub(crate) upgrades: Option<u8>,
    pub(crate) source: CardRewardBranchOptionSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CardRewardBranchOptionSource {
    PermanentReward,
    CombatGeneratedToHand,
    SkipCardReward,
    SingingBowl,
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
                CardRewardBranchOptionSource::SkipCardReward
                | CardRewardBranchOptionSource::SingingBowl => {
                    unreachable!("card reward card options cannot use non-card reward sources")
                }
            },
            card: Some(card.id),
            upgrades: Some(card.upgrades),
            source,
        })
        .collect::<Vec<_>>();
    if options.is_empty() {
        return None;
    }
    Some(options)
}

pub(crate) fn card_reward_decline_branch_options(
    session: &RunControlSession,
    include_event_reward_skip: bool,
) -> Vec<CardRewardBranchOption> {
    let mut options = Vec::new();
    if has_singing_bowl(session) && card_reward_bowl_available(session) {
        options.push(CardRewardBranchOption {
            label: "Singing Bowl | gain 2 max HP".to_string(),
            command: "bowl".to_string(),
            card: None,
            upgrades: None,
            source: CardRewardBranchOptionSource::SingingBowl,
        });
    }
    if include_event_reward_skip || !completed_event_reward_skip(session) {
        if let Some(command) = card_reward_skip_command(session) {
            options.push(CardRewardBranchOption {
                label: "Skip card reward".to_string(),
                command,
                card: None,
                upgrades: None,
                source: CardRewardBranchOptionSource::SkipCardReward,
            });
        }
    }
    options
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
    select_card_reward_branch_options_with_limit_and_strategy(
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
    select_card_reward_branch_options_with_limit_and_strategy(
        options,
        limit,
        portfolio_context,
        None,
    )
}

fn select_card_reward_branch_options_with_limit_and_strategy(
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

    let strategy_orders = reward_option_strategy_orders(session, &options);
    let mut annotated = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let (priority, class_key) = reward_option_semantic_class_for_option(option);
            let strategy = strategy_orders
                .get(&index)
                .cloned()
                .unwrap_or_else(RewardOptionStrategyOrder::missing);
            (
                index,
                strategy.order,
                strategy.score_key,
                priority,
                class_key,
                strategy.label,
            )
        })
        .collect::<Vec<_>>();
    annotated.sort_by(|left, right| {
        left.1
            .cmp(&right.1)
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| left.3.cmp(&right.3))
            .then_with(|| left.0.cmp(&right.0))
    });

    let mut selected = Vec::new();
    let mut selected_classes = BTreeSet::new();
    for (index, _, _, _, class_key, _) in &annotated {
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
    annotated: &[(usize, usize, i32, usize, String, String)],
    selected_indices: &BTreeSet<usize>,
) -> BranchExperimentRewardOptionPortfolioV1 {
    let class_by_index = annotated
        .iter()
        .map(|(index, strategy_order, _, _, class_key, strategy_label)| {
            (
                *index,
                format!("strategy={strategy_order}:{strategy_label}:{class_key}"),
            )
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

#[derive(Clone, Debug)]
struct RewardOptionStrategyOrder {
    order: usize,
    score_key: i32,
    label: String,
}

fn reward_option_strategy_orders(
    session: Option<&RunControlSession>,
    options: &[CardRewardBranchOption],
) -> BTreeMap<usize, RewardOptionStrategyOrder> {
    let Some(session) = session else {
        return options
            .iter()
            .enumerate()
            .map(|(index, _)| (index, RewardOptionStrategyOrder::unavailable()))
            .collect();
    };
    let mut option_card_indices = BTreeMap::new();
    let cards = options
        .iter()
        .enumerate()
        .filter_map(|(option_index, option)| {
            let card = option.card?;
            let card_index = option_card_indices.len();
            option_card_indices.insert(option_index, card_index);
            Some(RewardCard::new(card, option.upgrades.unwrap_or_default()))
        })
        .collect::<Vec<_>>();
    let route_trace = crate::ai::route_planner_v1::plan_route_decision_v1(
        &session.run_state,
        &session.engine_state,
        Default::default(),
    );
    let route_trace = (!route_trace.candidates.is_empty()).then_some(route_trace);
    let context = crate::ai::card_reward_policy_v1::build_card_reward_decision_context_v1(
        &session.run_state,
        cards,
        route_trace.as_ref(),
    );
    let trace = crate::ai::strategic::strategic_trace_for_card_reward(&context);
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let action = candidate_action_for_reward_option(index, option, &option_card_indices);
            let order = trace
                .compiled_for_action(&action)
                .map(|compiled| RewardOptionStrategyOrder {
                    order: compiled.verdict.retention_order(),
                    score_key: -((compiled.score * 1000.0).round() as i32),
                    label: format!("{:?}", compiled.verdict),
                })
                .unwrap_or_else(RewardOptionStrategyOrder::missing);
            (index, order)
        })
        .collect()
}

fn candidate_action_for_reward_option(
    option_index: usize,
    option: &CardRewardBranchOption,
    option_card_indices: &BTreeMap<usize, usize>,
) -> CandidateAction {
    match option.source {
        CardRewardBranchOptionSource::PermanentReward
        | CardRewardBranchOptionSource::CombatGeneratedToHand => CandidateAction::TakeCard {
            index: option_card_indices
                .get(&option_index)
                .copied()
                .unwrap_or(option_index),
            card: option
                .card
                .expect("card reward option source should carry a card"),
        },
        CardRewardBranchOptionSource::SkipCardReward => CandidateAction::SkipCardReward,
        CardRewardBranchOptionSource::SingingBowl => {
            CandidateAction::TakeSingingBowl { max_hp_gain: 2 }
        }
    }
}

impl RewardOptionStrategyOrder {
    fn unavailable() -> Self {
        Self {
            order: 0,
            score_key: 0,
            label: "strategy_unavailable".to_string(),
        }
    }

    fn missing() -> Self {
        Self {
            order: AcquisitionVerdict::Reject.retention_order(),
            score_key: 0,
            label: "missing_strategic_candidate".to_string(),
        }
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

fn reward_option_semantic_class_for_option(option: &CardRewardBranchOption) -> (usize, String) {
    match option.source {
        CardRewardBranchOptionSource::PermanentReward
        | CardRewardBranchOptionSource::CombatGeneratedToHand => {
            let profile = card_reward_semantic_profile_v1(&RewardCard::new(
                option
                    .card
                    .expect("card reward option source should carry a card"),
                option.upgrades.unwrap_or_default(),
            ));
            reward_option_semantic_class(&profile)
        }
        CardRewardBranchOptionSource::SkipCardReward => (3, "decline:skip_card_reward".to_string()),
        CardRewardBranchOptionSource::SingingBowl => (3, "decline:singing_bowl".to_string()),
    }
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

fn card_reward_skip_command(session: &RunControlSession) -> Option<String> {
    let EngineState::RewardScreen(reward) = &session.engine_state else {
        return None;
    };
    if reward.pending_card_choice.is_some() {
        return None;
    }
    let reward_index = reward
        .items
        .iter()
        .position(|item| matches!(item, RewardItem::Card { .. }))?;
    Some(format!("branch-skip-card-reward {reward_index}"))
}

fn completed_event_reward_skip(session: &RunControlSession) -> bool {
    session
        .run_state
        .event_state
        .as_ref()
        .is_some_and(|event| event.completed && !event.combat_pending)
}

fn card_reward_bowl_available(session: &RunControlSession) -> bool {
    match &session.engine_state {
        EngineState::RewardScreen(reward) => {
            reward.pending_card_choice.is_some() || reward.has_card_reward_item()
        }
        EngineState::RewardOverlay { reward_state, .. } => {
            reward_state.pending_card_choice.is_some() || reward_state.has_card_reward_item()
        }
        _ => false,
    }
}

fn has_singing_bowl(session: &RunControlSession) -> bool {
    session
        .run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::SingingBowl)
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
