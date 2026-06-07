use std::collections::{BTreeMap, BTreeSet};

use crate::ai::card_reward_policy_v1::{
    card_reward_semantic_profile_v1, CardRewardSemanticProfileV1, CardRewardSemanticRoleV1,
};
use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::eval::branch_experiment::{
    BranchExperimentRewardOptionPortfolioEntryV1, BranchExperimentRewardOptionPortfolioV1,
};
use crate::eval::branch_experiment_trajectory::summarize_branch_trajectory_v1;
use crate::eval::run_control::{build_decision_surface, RunControlSession};
use crate::state::core::{CampfireChoice, ClientInput, EngineState};
use crate::state::rewards::{RewardCard, RewardItem};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BranchBoundaryIdV1 {
    CardReward,
    Campfire,
    BossRelic,
}

impl BranchBoundaryIdV1 {
    pub(crate) fn empty_portfolio_reason(self) -> &'static str {
        match self {
            BranchBoundaryIdV1::CardReward => "card reward option portfolio is empty",
            BranchBoundaryIdV1::Campfire => "campfire option portfolio is empty",
            BranchBoundaryIdV1::BossRelic => "boss relic option portfolio is empty",
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

    None
}

pub(crate) fn branch_boundary_available(session: &RunControlSession) -> bool {
    card_reward_branch_options(session).is_some()
        || campfire_branch_options(session).is_some()
        || boss_relic_branch_options(session).is_some()
}

impl BranchBoundaryOptionV1 {
    fn from_card_reward(option: CardRewardBranchOption) -> Self {
        Self {
            kind: "card_reward",
            label: option.label,
            command: option.command,
            card: Some(option.card),
            upgrades: Some(option.upgrades),
            success_reason: "card reward branch applied",
        }
    }

    fn from_campfire(option: CampfireBranchOption) -> Self {
        Self {
            kind: "campfire",
            label: option.label,
            command: option.command,
            card: option.card,
            upgrades: option.upgrades,
            success_reason: "campfire branch applied",
        }
    }

    fn from_boss_relic(option: BossRelicBranchOption) -> Self {
        Self {
            kind: "boss_relic",
            label: option.label,
            command: option.command,
            card: None,
            upgrades: None,
            success_reason: "boss relic branch applied",
        }
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
            let (card, upgrades, semantic_class) = campfire_option_metadata(session, choice);
            Some(CampfireBranchOption {
                label: candidate.label.clone(),
                command: candidate.action.command_hint(),
                card,
                upgrades,
                semantic_class,
            })
        })
        .collect::<Vec<_>>();
    (!options.is_empty()).then_some(options)
}

fn campfire_option_metadata(
    session: &RunControlSession,
    choice: CampfireChoice,
) -> (Option<CardId>, Option<u8>, String) {
    match choice {
        CampfireChoice::Rest => (None, None, "rest".to_string()),
        CampfireChoice::Smith(idx) => {
            let Some(card) = session.run_state.master_deck.get(idx) else {
                return (None, None, "smith:unknown".to_string());
            };
            let upgraded = card.upgrades.saturating_add(1);
            let profile = card_reward_semantic_profile_v1(&RewardCard::new(card.id, upgraded));
            let (_, class_key) = reward_option_semantic_class(&profile);
            (
                Some(card.id),
                Some(card.upgrades),
                format!("smith:{class_key}"),
            )
        }
        CampfireChoice::Dig => (None, None, "dig".to_string()),
        CampfireChoice::Lift => (None, None, "lift".to_string()),
        CampfireChoice::Toke(idx) => {
            let card = session.run_state.master_deck.get(idx);
            (
                card.map(|card| card.id),
                card.map(|card| card.upgrades),
                "toke".to_string(),
            )
        }
        CampfireChoice::Recall => (None, None, "recall".to_string()),
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
