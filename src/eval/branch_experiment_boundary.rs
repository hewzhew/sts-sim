use std::collections::{BTreeMap, BTreeSet};

use crate::ai::card_reward_policy_v1::{
    card_reward_semantic_profile_v1, CardRewardSemanticProfileV1, CardRewardSemanticRoleV1,
};
use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::eval::branch_experiment::{
    BranchExperimentChoiceCardV1, BranchExperimentRewardOptionPortfolioEntryV1,
    BranchExperimentRewardOptionPortfolioV1,
};
use crate::eval::branch_experiment_trajectory::summarize_branch_trajectory_v1;
use crate::eval::run_control::{build_decision_surface, RunControlSession};
use crate::runtime::combat::CombatCard;
use crate::state::core::{CampfireChoice, ClientInput, EngineState};
use crate::state::rewards::{RewardCard, RewardItem};

mod run_selection;

use run_selection::{run_selection_branch_options, RunSelectionBranchOption};

const MAX_EVENT_OPTIONS_PER_BRANCH: usize = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BranchBoundaryIdV1 {
    CardReward,
    Campfire,
    BossRelic,
    RunSelection,
    Event,
}

impl BranchBoundaryIdV1 {
    pub(crate) fn empty_portfolio_reason(self) -> &'static str {
        match self {
            BranchBoundaryIdV1::CardReward => "card reward option portfolio is empty",
            BranchBoundaryIdV1::Campfire => "campfire option portfolio is empty",
            BranchBoundaryIdV1::BossRelic => "boss relic option portfolio is empty",
            BranchBoundaryIdV1::RunSelection => "run selection option portfolio is empty",
            BranchBoundaryIdV1::Event => "event option portfolio is empty",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct BranchBoundaryConfigV1 {
    pub(crate) max_reward_options_per_branch: Option<usize>,
    pub(crate) max_campfire_options_per_branch: Option<usize>,
}

#[derive(Clone, Debug)]
pub(crate) struct BranchBoundarySelectionV1 {
    pub(crate) id: BranchBoundaryIdV1,
    pub(crate) options: Vec<BranchBoundaryOptionV1>,
    pub(crate) reward_option_portfolio: Option<BranchExperimentRewardOptionPortfolioV1>,
}

#[derive(Clone, Debug)]
pub(crate) struct BranchBoundaryOptionV1 {
    pub(crate) kind: &'static str,
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) card: Option<CardId>,
    pub(crate) upgrades: Option<u8>,
    pub(crate) selected_cards: Vec<BranchExperimentChoiceCardV1>,
    pub(crate) effect_kind: String,
    pub(crate) effect_key: String,
    pub(crate) effect_label: String,
    pub(crate) representative_count: usize,
    pub(crate) suppressed_count: usize,
    pub(crate) success_reason: &'static str,
}

#[derive(Clone, Debug)]
pub(crate) struct CardRewardBranchOption {
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) card: CardId,
    pub(crate) upgrades: u8,
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

#[derive(Clone, Debug)]
pub(crate) struct CampfireBranchOption {
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) card: Option<CardId>,
    pub(crate) upgrades: Option<u8>,
    semantic_class: String,
    equivalence_key: String,
    representative_count: usize,
    suppressed_count: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct CampfireBranchOptionSelection {
    pub(crate) options: Vec<CampfireBranchOption>,
}

#[derive(Clone, Debug)]
struct BossRelicBranchOption {
    label: String,
    command: String,
}

#[derive(Clone, Debug)]
struct EventBranchOption {
    label: String,
    command: String,
}

pub(crate) fn current_branch_boundary(
    session: &RunControlSession,
    config: BranchBoundaryConfigV1,
    reward_portfolio_context: Option<CardRewardPortfolioContext>,
) -> Option<BranchBoundarySelectionV1> {
    if let Some(options) = card_reward_branch_options(session) {
        let selected = select_card_reward_branch_options(
            options,
            config.max_reward_options_per_branch,
            reward_portfolio_context,
        );
        return Some(BranchBoundarySelectionV1 {
            id: BranchBoundaryIdV1::CardReward,
            options: selected
                .options
                .into_iter()
                .map(BranchBoundaryOptionV1::from_card_reward)
                .collect(),
            reward_option_portfolio: selected.portfolio,
        });
    }

    if let Some(options) = campfire_branch_options(session) {
        let selected =
            select_campfire_branch_options(options, config.max_campfire_options_per_branch);
        return Some(BranchBoundarySelectionV1 {
            id: BranchBoundaryIdV1::Campfire,
            options: selected
                .options
                .into_iter()
                .map(BranchBoundaryOptionV1::from_campfire)
                .collect(),
            reward_option_portfolio: None,
        });
    }

    if let Some(options) = boss_relic_branch_options(session) {
        return Some(BranchBoundarySelectionV1 {
            id: BranchBoundaryIdV1::BossRelic,
            options: options
                .into_iter()
                .map(BranchBoundaryOptionV1::from_boss_relic)
                .collect(),
            reward_option_portfolio: None,
        });
    }

    if let Some(options) = run_selection_branch_options(session) {
        return Some(BranchBoundarySelectionV1 {
            id: BranchBoundaryIdV1::RunSelection,
            options: options
                .into_iter()
                .map(BranchBoundaryOptionV1::from_run_selection)
                .collect(),
            reward_option_portfolio: None,
        });
    }

    if let Some(options) = event_branch_options(session) {
        return Some(BranchBoundarySelectionV1 {
            id: BranchBoundaryIdV1::Event,
            options: options
                .into_iter()
                .map(BranchBoundaryOptionV1::from_event)
                .collect(),
            reward_option_portfolio: None,
        });
    }

    None
}

pub(crate) fn branch_boundary_available(session: &RunControlSession) -> bool {
    card_reward_branch_options(session).is_some()
        || campfire_branch_options(session).is_some()
        || boss_relic_branch_options(session).is_some()
        || run_selection_branch_options(session).is_some()
        || event_branch_options(session).is_some()
}

impl BranchBoundaryOptionV1 {
    fn from_card_reward(option: CardRewardBranchOption) -> Self {
        Self {
            kind: "card_reward",
            effect_label: option.label.clone(),
            label: option.label,
            command: option.command,
            card: Some(option.card),
            upgrades: Some(option.upgrades),
            selected_cards: selected_card_vec(Some(option.card), Some(option.upgrades)),
            effect_kind: "add_card".to_string(),
            effect_key: format!("card_reward:add_card:{:?}:{}", option.card, option.upgrades),
            representative_count: 1,
            suppressed_count: 0,
            success_reason: "card reward branch applied",
        }
    }

    fn from_campfire(option: CampfireBranchOption) -> Self {
        let effect_kind = campfire_effect_kind(&option);
        let effect_key = format!("campfire:{effect_kind}:{}", option.command);
        Self {
            kind: "campfire",
            effect_label: option.label.clone(),
            label: option.label,
            command: option.command,
            card: option.card,
            upgrades: option.upgrades,
            selected_cards: selected_card_vec(option.card, option.upgrades),
            effect_kind,
            effect_key,
            representative_count: option.representative_count,
            suppressed_count: option.suppressed_count,
            success_reason: "campfire branch applied",
        }
    }

    fn from_boss_relic(option: BossRelicBranchOption) -> Self {
        let effect_key = format!("boss_relic:choose:{}", option.command);
        Self {
            kind: "boss_relic",
            effect_label: option.label.clone(),
            label: option.label,
            command: option.command,
            card: None,
            upgrades: None,
            selected_cards: Vec::new(),
            effect_kind: "boss_relic".to_string(),
            effect_key,
            representative_count: 1,
            suppressed_count: 0,
            success_reason: "boss relic branch applied",
        }
    }

    fn from_run_selection(option: RunSelectionBranchOption) -> Self {
        Self {
            kind: "run_selection",
            label: option.label,
            command: option.command,
            card: option.card,
            upgrades: option.upgrades,
            selected_cards: option.selected_cards,
            effect_kind: option.effect_kind,
            effect_key: option.effect_key,
            effect_label: option.effect_label,
            representative_count: option.representative_count,
            suppressed_count: option.suppressed_count,
            success_reason: "run selection branch applied",
        }
    }

    fn from_event(option: EventBranchOption) -> Self {
        let effect_key = format!("event:choose:{}", option.command);
        Self {
            kind: "event",
            effect_label: option.label.clone(),
            label: option.label,
            command: option.command,
            card: None,
            upgrades: None,
            selected_cards: Vec::new(),
            effect_kind: "event_choice".to_string(),
            effect_key,
            representative_count: 1,
            suppressed_count: 0,
            success_reason: "event branch applied",
        }
    }
}

fn campfire_effect_kind(option: &CampfireBranchOption) -> String {
    if option.command.starts_with("smith ") || option.command.starts_with("smith-") {
        "upgrade_card".to_string()
    } else if option.command.starts_with("toke ") || option.command.starts_with("toke-") {
        "remove_card".to_string()
    } else if option.command == "rest" {
        "campfire_rest".to_string()
    } else {
        "campfire_action".to_string()
    }
}

fn selected_card_vec(
    card: Option<CardId>,
    upgrades: Option<u8>,
) -> Vec<BranchExperimentChoiceCardV1> {
    match card {
        Some(card) => vec![BranchExperimentChoiceCardV1 {
            card,
            upgrades: upgrades.unwrap_or_default(),
        }],
        None => Vec::new(),
    }
}

pub(crate) fn card_reward_branch_options(
    session: &RunControlSession,
) -> Option<Vec<CardRewardBranchOption>> {
    let cards = active_or_visible_reward_cards(session)?;
    let options = cards
        .iter()
        .enumerate()
        .map(|(idx, card)| CardRewardBranchOption {
            label: format_reward_card_label(card),
            command: format!("rp {idx}"),
            card: card.id,
            upgrades: card.upgrades,
        })
        .collect::<Vec<_>>();
    if options.is_empty() {
        return None;
    }
    Some(options)
}

pub(crate) fn select_card_reward_branch_options(
    options: Vec<CardRewardBranchOption>,
    max_reward_options_per_branch: Option<usize>,
    portfolio_context: Option<CardRewardPortfolioContext>,
) -> CardRewardBranchOptionSelection {
    let Some(limit) = max_reward_options_per_branch else {
        return CardRewardBranchOptionSelection {
            options,
            portfolio: None,
        };
    };
    select_card_reward_branch_options_with_limit(options, limit, portfolio_context)
}

fn select_card_reward_branch_options_with_limit(
    options: Vec<CardRewardBranchOption>,
    limit: usize,
    portfolio_context: Option<CardRewardPortfolioContext>,
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
            (index, priority, class_key)
        })
        .collect::<Vec<_>>();
    annotated.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));

    let mut selected = Vec::new();
    let mut selected_classes = BTreeSet::new();
    for (index, _, class_key) in &annotated {
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

pub(crate) fn campfire_branch_options(
    session: &RunControlSession,
) -> Option<Vec<CampfireBranchOption>> {
    if !matches!(session.engine_state, EngineState::Campfire) {
        return None;
    }
    let surface = build_decision_surface(session);
    let options = surface
        .view
        .candidates
        .iter()
        .filter_map(|candidate| {
            let input = candidate.action.executable_input()?;
            let ClientInput::CampfireOption(choice) = input else {
                return None;
            };
            let (card, upgrades, semantic_class, equivalence_key) =
                campfire_option_metadata(session, choice);
            Some(CampfireBranchOption {
                label: candidate.label.clone(),
                command: candidate.action.command_hint(),
                card,
                upgrades,
                semantic_class,
                equivalence_key,
                representative_count: 1,
                suppressed_count: 0,
            })
        })
        .collect::<Vec<_>>();
    let options = compressed_campfire_branch_options(options);
    (!options.is_empty()).then_some(options)
}

fn compressed_campfire_branch_options(
    options: Vec<CampfireBranchOption>,
) -> Vec<CampfireBranchOption> {
    let mut groups = Vec::<CampfireBranchOption>::new();
    let mut counts = Vec::<usize>::new();
    for option in options {
        let key = campfire_option_equivalence_key(&option);
        if let Some((index, _)) = groups
            .iter()
            .enumerate()
            .find(|(_, representative)| campfire_option_equivalence_key(representative) == key)
        {
            counts[index] += 1;
        } else {
            groups.push(option);
            counts.push(1);
        }
    }
    groups
        .into_iter()
        .zip(counts)
        .map(|(mut option, count)| {
            option.representative_count = count;
            option.suppressed_count = count.saturating_sub(1);
            option
        })
        .collect()
}

fn campfire_option_equivalence_key(option: &CampfireBranchOption) -> String {
    option.equivalence_key.clone()
}

fn campfire_option_metadata(
    session: &RunControlSession,
    choice: CampfireChoice,
) -> (Option<CardId>, Option<u8>, String, String) {
    match choice {
        CampfireChoice::Rest => (None, None, "rest".to_string(), "rest".to_string()),
        CampfireChoice::Smith(idx) => {
            let Some(card) = session.run_state.master_deck.get(idx) else {
                return (
                    None,
                    None,
                    "smith:unknown".to_string(),
                    "smith:unknown".to_string(),
                );
            };
            let upgraded = card.upgrades.saturating_add(1);
            let profile = card_reward_semantic_profile_v1(&RewardCard::new(card.id, upgraded));
            let (_, class_key) = reward_option_semantic_class(&profile);
            (
                Some(card.id),
                Some(card.upgrades),
                format!("smith:{class_key}"),
                format!("smith:{}", card_stat_identity_key(card)),
            )
        }
        CampfireChoice::Dig => (None, None, "dig".to_string(), "dig".to_string()),
        CampfireChoice::Lift => (None, None, "lift".to_string(), "lift".to_string()),
        CampfireChoice::Toke(idx) => {
            let card = session.run_state.master_deck.get(idx);
            (
                card.map(|card| card.id),
                card.map(|card| card.upgrades),
                "toke".to_string(),
                card.map(|card| format!("toke:{}", card_stat_identity_key(card)))
                    .unwrap_or_else(|| "toke:unknown".to_string()),
            )
        }
        CampfireChoice::Recall => (None, None, "recall".to_string(), "recall".to_string()),
    }
}

pub(crate) fn select_campfire_branch_options(
    options: Vec<CampfireBranchOption>,
    max_campfire_options_per_branch: Option<usize>,
) -> CampfireBranchOptionSelection {
    let Some(limit) = max_campfire_options_per_branch else {
        return CampfireBranchOptionSelection { options };
    };
    select_campfire_branch_options_with_limit(options, limit)
}

fn select_campfire_branch_options_with_limit(
    options: Vec<CampfireBranchOption>,
    limit: usize,
) -> CampfireBranchOptionSelection {
    let capped_limit = limit.min(options.len());
    if options.len() <= capped_limit {
        return CampfireBranchOptionSelection { options };
    }

    let mut annotated = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            (
                index,
                campfire_option_priority(option),
                option.semantic_class.clone(),
            )
        })
        .collect::<Vec<_>>();
    annotated.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));

    let mut selected = Vec::new();
    let mut selected_classes = BTreeSet::new();
    for (index, _, class_key) in &annotated {
        if selected.len() >= capped_limit {
            break;
        }
        if selected_classes.insert(class_key.clone()) {
            selected.push(*index);
        }
    }
    for index in 0..options.len() {
        if selected.len() >= capped_limit {
            break;
        }
        if !selected.contains(&index) {
            selected.push(index);
        }
    }

    selected.sort_unstable();
    let selected_indices = selected.iter().copied().collect::<BTreeSet<_>>();
    let options = options
        .into_iter()
        .enumerate()
        .filter_map(|(index, option)| selected_indices.contains(&index).then_some(option))
        .collect();
    CampfireBranchOptionSelection { options }
}

fn boss_relic_branch_options(session: &RunControlSession) -> Option<Vec<BossRelicBranchOption>> {
    let EngineState::BossRelicSelect(choice) = &session.engine_state else {
        return None;
    };
    let options = choice
        .relics
        .iter()
        .enumerate()
        .map(|(idx, relic)| BossRelicBranchOption {
            label: boss_relic_label(*relic),
            command: format!("relic {idx}"),
        })
        .collect::<Vec<_>>();
    (!options.is_empty()).then_some(options)
}

fn boss_relic_label(relic: RelicId) -> String {
    format!("{relic:?}")
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

fn event_branch_options(session: &RunControlSession) -> Option<Vec<EventBranchOption>> {
    if !matches!(session.engine_state, EngineState::EventRoom) {
        return None;
    }
    let event_options = crate::engine::event_handler::get_event_options(&session.run_state);
    let surface = build_decision_surface(session);
    let mut branch_options = Vec::new();

    for candidate in &surface.view.candidates {
        let Some(ClientInput::EventChoice(index)) = candidate.action.executable_input() else {
            continue;
        };
        let Some(event_option) = event_options.get(index) else {
            continue;
        };
        if event_option.ui.disabled {
            return None;
        }
        branch_options.push(EventBranchOption {
            label: candidate.label.clone(),
            command: candidate.action.command_hint(),
        });
    }

    if branch_options.is_empty() || branch_options.len() > MAX_EVENT_OPTIONS_PER_BRANCH {
        return None;
    }
    Some(branch_options)
}

fn campfire_option_priority(option: &CampfireBranchOption) -> usize {
    match option.semantic_class.as_str() {
        "rest" => 0,
        class if class.starts_with("smith:") => 1,
        "dig" | "lift" | "toke" => 2,
        "recall" => 3,
        _ => 4,
    }
}

fn reward_option_portfolio_report(
    depth: usize,
    frontier_key: String,
    boundary_title: String,
    max_reward_options_per_branch: usize,
    options: &[CardRewardBranchOption],
    annotated: &[(usize, usize, String)],
    selected_indices: &BTreeSet<usize>,
) -> BranchExperimentRewardOptionPortfolioV1 {
    let class_by_index = annotated
        .iter()
        .map(|(index, _, class_key)| (*index, class_key.clone()))
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

fn reward_option_semantic_class(profile: &CardRewardSemanticProfileV1) -> (usize, String) {
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

fn format_reward_card_label(card: &RewardCard) -> String {
    let name = crate::content::cards::get_card_definition(card.id).name;
    match card.upgrades {
        0 => name.to_string(),
        1 => format!("{name}+"),
        upgrades => format!("{name}+{upgrades}"),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::content::cards::CardId;
    use crate::content::relics::RelicId;
    use crate::eval::run_control::{RunControlConfig, RunControlSession};
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{RunPendingChoiceReason, RunPendingChoiceState};
    use crate::state::events::{EventId, EventState};
    use crate::state::rewards::{BossRelicChoiceState, RewardState};

    #[test]
    fn card_reward_option_portfolio_keeps_semantic_variety() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
            RewardCard::new(CardId::ShrugItOff, 0),
        ]);
        session.engine_state = EngineState::RewardScreen(reward);

        let options = card_reward_branch_options(&session).expect("card reward options");
        let selected = select_card_reward_branch_options_with_limit(options, 2, None).options;
        let picked_labels = selected
            .iter()
            .map(|option| option.label.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(selected.len(), 2);
        assert!(
            picked_labels.contains("Shrug It Off"),
            "non-transition defense/draw candidate should not be crowded out"
        );
        assert_eq!(
            picked_labels
                .iter()
                .filter(|label| **label == "Twin Strike" || **label == "Cleave")
                .count(),
            1,
            "pure transition options should be represented, not exhaustively expanded"
        );
    }

    #[test]
    fn current_boundary_wraps_card_reward_options() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::ShrugItOff, 0),
        ]);
        session.engine_state = EngineState::RewardScreen(reward);

        let boundary = current_branch_boundary(
            &session,
            BranchBoundaryConfigV1 {
                max_reward_options_per_branch: Some(1),
                max_campfire_options_per_branch: None,
            },
            Some(CardRewardPortfolioContext {
                depth: 0,
                frontier_key: "frontier".to_string(),
                boundary_title: "Card Reward".to_string(),
            }),
        )
        .expect("card reward boundary");

        assert_eq!(boundary.id, BranchBoundaryIdV1::CardReward);
        assert_eq!(boundary.options.len(), 1);
        assert_eq!(boundary.options[0].kind, "card_reward");
        assert!(boundary.reward_option_portfolio.is_some());
    }

    #[test]
    fn current_boundary_wraps_campfire_options() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::Campfire;

        let boundary = current_branch_boundary(
            &session,
            BranchBoundaryConfigV1 {
                max_reward_options_per_branch: None,
                max_campfire_options_per_branch: Some(2),
            },
            None,
        )
        .expect("campfire boundary");

        assert_eq!(boundary.id, BranchBoundaryIdV1::Campfire);
        assert_eq!(boundary.options.len(), 2);
        assert!(boundary
            .options
            .iter()
            .all(|option| option.kind == "campfire"));
    }

    #[test]
    fn current_boundary_compresses_duplicate_campfire_smith_options() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::Campfire;

        let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
            .expect("campfire boundary");

        assert_eq!(boundary.id, BranchBoundaryIdV1::Campfire);
        assert_eq!(
            boundary
                .options
                .iter()
                .filter(|option| option.effect_kind == "upgrade_card")
                .map(|option| {
                    (
                        option.command.as_str(),
                        option.effect_label.as_str(),
                        option.card,
                        option.upgrades,
                        option.representative_count,
                        option.suppressed_count,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                (
                    "smith 0",
                    "Smith Strike",
                    Some(CardId::Strike),
                    Some(0),
                    5,
                    4
                ),
                (
                    "smith 5",
                    "Smith Defend",
                    Some(CardId::Defend),
                    Some(0),
                    4,
                    3
                ),
                ("smith 9", "Smith Bash", Some(CardId::Bash), Some(0), 1, 0),
            ]
        );
    }

    #[test]
    fn current_boundary_keeps_distinct_campfire_smith_card_state_separate() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut first = CombatCard::new(CardId::RitualDagger, 10);
        first.misc_value = 17;
        let mut second = CombatCard::new(CardId::RitualDagger, 11);
        second.misc_value = 23;
        session.run_state.master_deck = vec![first, second];
        session.engine_state = EngineState::Campfire;

        let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
            .expect("campfire boundary");

        assert_eq!(boundary.id, BranchBoundaryIdV1::Campfire);
        assert_eq!(
            boundary
                .options
                .iter()
                .filter(|option| option.effect_kind == "upgrade_card")
                .map(|option| {
                    (
                        option.command.as_str(),
                        option.effect_label.as_str(),
                        option.representative_count,
                        option.suppressed_count,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("smith 0", "Smith Ritual Dagger", 1, 0),
                ("smith 1", "Smith Ritual Dagger", 1, 0),
            ]
        );
    }

    #[test]
    fn current_boundary_wraps_boss_relic_options() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::BossRelicSelect(BossRelicChoiceState::new(vec![
            RelicId::BlackStar,
            RelicId::EmptyCage,
            RelicId::TinyHouse,
        ]));

        let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
            .expect("boss relic boundary");

        assert_eq!(boundary.id, BranchBoundaryIdV1::BossRelic);
        assert_eq!(
            boundary
                .options
                .iter()
                .map(|option| (option.kind, option.command.as_str(), option.card))
                .collect::<Vec<_>>(),
            vec![
                ("boss_relic", "relic 0", None),
                ("boss_relic", "relic 1", None),
                ("boss_relic", "relic 2", None),
            ]
        );
    }

    #[test]
    fn current_boundary_wraps_low_fanout_event_options() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = Some(EventState::new(EventId::BigFish));
        session.engine_state = EngineState::EventRoom;

        let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
            .expect("event boundary");

        assert_eq!(boundary.id, BranchBoundaryIdV1::Event);
        assert_eq!(
            boundary
                .options
                .iter()
                .map(|option| (option.kind, option.command.as_str(), option.card))
                .collect::<Vec<_>>(),
            vec![
                ("event", "event 0", None),
                ("event", "event 1", None),
                ("event", "event 2", None),
            ]
        );
    }

    #[test]
    fn current_boundary_allows_event_options_that_open_single_card_selection() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = Some(EventState::new(EventId::UpgradeShrine));
        session.engine_state = EngineState::EventRoom;

        let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
            .expect("event boundary");

        assert_eq!(boundary.id, BranchBoundaryIdV1::Event);
        assert_eq!(
            boundary
                .options
                .iter()
                .map(|option| (option.kind, option.command.as_str()))
                .collect::<Vec<_>>(),
            vec![("event", "event 0"), ("event", "event 1")]
        );
    }

    #[test]
    fn current_boundary_wraps_single_card_run_selection_options() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
            min_choices: 1,
            max_choices: 1,
            reason: RunPendingChoiceReason::Purge,
            return_state: Box::new(EngineState::EventRoom),
        });

        let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
            .expect("run selection boundary");

        assert_eq!(boundary.id, BranchBoundaryIdV1::RunSelection);
        assert_eq!(
            boundary
                .options
                .iter()
                .map(|option| (
                    option.kind,
                    option.command.as_str(),
                    option.card,
                    option.upgrades,
                    option.effect_key.as_str(),
                    option.effect_label.as_str(),
                    option.representative_count,
                    option.suppressed_count,
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "run_selection",
                    "select 0",
                    Some(CardId::Strike),
                    Some(0),
                    "run_selection:remove_card:Strike:0",
                    "remove Strike",
                    5,
                    4,
                ),
                (
                    "run_selection",
                    "select 5",
                    Some(CardId::Defend),
                    Some(0),
                    "run_selection:remove_card:Defend:0",
                    "remove Defend",
                    4,
                    3,
                ),
                (
                    "run_selection",
                    "select 9",
                    Some(CardId::Bash),
                    Some(0),
                    "run_selection:remove_card:Bash:0",
                    "remove Bash",
                    1,
                    0,
                ),
            ]
        );
    }

    #[test]
    fn current_boundary_keeps_distinct_run_selection_card_state_separate() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut first = CombatCard::new(CardId::RitualDagger, 10);
        first.misc_value = 17;
        let mut second = CombatCard::new(CardId::RitualDagger, 11);
        second.misc_value = 23;
        session.run_state.master_deck = vec![first, second];
        session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
            min_choices: 1,
            max_choices: 1,
            reason: RunPendingChoiceReason::Purge,
            return_state: Box::new(EngineState::EventRoom),
        });

        let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
            .expect("run selection boundary");

        assert_eq!(boundary.id, BranchBoundaryIdV1::RunSelection);
        assert_eq!(
            boundary
                .options
                .iter()
                .map(|option| {
                    (
                        option.command.as_str(),
                        option.effect_label.as_str(),
                        option.representative_count,
                        option.suppressed_count,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("select 0", "remove Ritual Dagger", 1, 0),
                ("select 1", "remove Ritual Dagger", 1, 0),
            ]
        );
    }

    #[test]
    fn current_boundary_wraps_small_multi_card_run_selection_options() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.master_deck.truncate(3);
        session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
            min_choices: 2,
            max_choices: 2,
            reason: RunPendingChoiceReason::Transform,
            return_state: Box::new(EngineState::EventRoom),
        });

        let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
            .expect("small multi-card run selection boundary");

        assert_eq!(boundary.id, BranchBoundaryIdV1::RunSelection);
        assert_eq!(
            boundary
                .options
                .iter()
                .map(|option| {
                    (
                        option.command.as_str(),
                        option.card,
                        option.selected_cards.len(),
                        option.representative_count,
                        option.suppressed_count,
                    )
                })
                .collect::<Vec<_>>(),
            vec![("select 0 1", None, 2, 3, 2)]
        );
    }

    #[test]
    fn current_boundary_compresses_multi_card_run_selection_by_effect_key() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
            min_choices: 2,
            max_choices: 2,
            reason: RunPendingChoiceReason::Transform,
            return_state: Box::new(EngineState::EventRoom),
        });

        let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
            .expect("compressed multi-card run selection boundary");

        assert_eq!(boundary.id, BranchBoundaryIdV1::RunSelection);
        assert_eq!(
            boundary
                .options
                .iter()
                .map(|option| {
                    (
                        option.command.as_str(),
                        option.effect_label.as_str(),
                        option.representative_count,
                        option.suppressed_count,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("select 0 1", "transform Strike x2", 10, 9),
                ("select 0 5", "transform Strike, Defend", 20, 19),
                ("select 0 9", "transform Strike, Bash", 5, 4),
                ("select 5 6", "transform Defend x2", 6, 5),
                ("select 5 9", "transform Defend, Bash", 4, 3),
            ]
        );
    }

    #[test]
    fn current_boundary_still_rejects_high_fanout_distinct_multi_card_run_selection_options() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.master_deck = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Defend, 11),
            CombatCard::new(CardId::Bash, 12),
            CombatCard::new(CardId::TwinStrike, 13),
            CombatCard::new(CardId::PommelStrike, 14),
            CombatCard::new(CardId::Shockwave, 15),
        ];
        session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
            min_choices: 2,
            max_choices: 2,
            reason: RunPendingChoiceReason::Transform,
            return_state: Box::new(EngineState::EventRoom),
        });

        assert!(
            current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None).is_none(),
            "multi-card run selection should still stop when semantic combinations exceed the branch cap"
        );
    }

    #[test]
    fn campfire_branch_option_portfolio_keeps_rest_and_smith_classes() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::Campfire;

        let options = campfire_branch_options(&session).expect("campfire options");
        let selected = select_campfire_branch_options_with_limit(options, 2).options;

        assert!(
            selected.iter().any(|option| option.command == "rest"),
            "rest should remain represented when campfire branching is capped"
        );
        assert!(
            selected
                .iter()
                .any(|option| option.command.starts_with("smith ")),
            "at least one smith option should remain represented when campfire branching is capped"
        );
    }

    #[test]
    fn reward_option_semantic_class_distinguishes_stabilizer_roles() {
        let shockwave = card_reward_semantic_profile_v1(&RewardCard::new(CardId::Shockwave, 0));
        let armaments = card_reward_semantic_profile_v1(&RewardCard::new(CardId::Armaments, 0));

        let (_, shockwave_class) = reward_option_semantic_class(&shockwave);
        let (_, armaments_class) = reward_option_semantic_class(&armaments);

        assert_ne!(
            shockwave_class, armaments_class,
            "control/debuff stabilizers and plain block/upgrade stabilizers should not collapse into one option class"
        );
    }
}
