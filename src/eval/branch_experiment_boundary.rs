use crate::content::cards::CardId;
use crate::eval::branch_experiment::{
    BranchExperimentChoiceCardV1, BranchExperimentRewardOptionPortfolioV1,
};
use crate::eval::run_control::RunControlSession;
use crate::runtime::combat::CombatCard;

mod boss_relic;
mod campfire;
mod card_reward;
mod event;
mod reward;
mod run_selection;
mod shop;

use boss_relic::{boss_relic_branch_options, BossRelicBranchOption};
use campfire::{campfire_branch_options, select_campfire_branch_options, CampfireBranchOption};
pub(crate) use card_reward::{
    active_or_visible_reward_cards, card_offer_labels, CardRewardPortfolioContext,
};
use card_reward::{
    card_reward_branch_options, card_reward_decline_branch_options, reward_option_semantic_class,
    select_card_reward_branch_options_for_session, CardRewardBranchOption,
};
use event::{event_branch_options, EventBranchOption};
use reward::{reward_branch_options, RewardBranchOption};
use run_selection::{run_selection_branch_options, RunSelectionBranchOption};
use shop::{shop_branch_options, ShopBranchOption};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BranchBoundaryIdV1 {
    CardReward,
    Campfire,
    BossRelic,
    RunSelection,
    Reward,
    Shop,
    Event,
}

impl BranchBoundaryIdV1 {
    pub(crate) fn empty_portfolio_reason(self) -> &'static str {
        match self {
            BranchBoundaryIdV1::CardReward => "card reward option portfolio is empty",
            BranchBoundaryIdV1::Campfire => "campfire option portfolio is empty",
            BranchBoundaryIdV1::BossRelic => "boss relic option portfolio is empty",
            BranchBoundaryIdV1::RunSelection => "run selection option portfolio is empty",
            BranchBoundaryIdV1::Reward => "reward claim option portfolio is empty",
            BranchBoundaryIdV1::Shop => "shop option portfolio is empty",
            BranchBoundaryIdV1::Event => "event option portfolio is empty",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct BranchBoundaryConfigV1 {
    pub(crate) max_reward_options_per_branch: Option<usize>,
    pub(crate) max_campfire_options_per_branch: Option<usize>,
    pub(crate) include_skip: bool,
    pub(crate) include_event_reward_skip: bool,
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

pub(crate) fn current_branch_boundary(
    session: &RunControlSession,
    config: BranchBoundaryConfigV1,
    reward_portfolio_context: Option<CardRewardPortfolioContext>,
) -> Option<BranchBoundarySelectionV1> {
    if let Some(mut options) = card_reward_branch_options(session) {
        if config.include_skip {
            options.extend(card_reward_decline_branch_options(
                session,
                config.include_event_reward_skip,
            ));
        }
        let selected = select_card_reward_branch_options_for_session(
            session,
            options,
            config.max_reward_options_per_branch,
            reward_portfolio_context,
        );
        let options = selected
            .options
            .into_iter()
            .map(BranchBoundaryOptionV1::from_card_reward)
            .collect::<Vec<_>>();
        return Some(BranchBoundarySelectionV1 {
            id: BranchBoundaryIdV1::CardReward,
            options,
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

    if let Some(options) = reward_branch_options(session) {
        return Some(BranchBoundarySelectionV1 {
            id: BranchBoundaryIdV1::Reward,
            options: options
                .into_iter()
                .map(BranchBoundaryOptionV1::from_reward)
                .collect(),
            reward_option_portfolio: None,
        });
    }

    if let Some(options) = shop_branch_options(session) {
        return Some(BranchBoundarySelectionV1 {
            id: BranchBoundaryIdV1::Shop,
            options: options
                .into_iter()
                .map(BranchBoundaryOptionV1::from_shop)
                .collect(),
            reward_option_portfolio: None,
        });
    }

    if let Some(options) = event_branch_options(session, config.max_reward_options_per_branch) {
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
        || reward_branch_options(session).is_some()
        || shop_branch_options(session).is_some()
        || event_branch_options(session, Some(4)).is_some()
}

impl BranchBoundaryOptionV1 {
    fn from_card_reward(option: CardRewardBranchOption) -> Self {
        let (kind, effect_kind, effect_key_prefix, success_reason) = match option.source {
            card_reward::CardRewardBranchOptionSource::PermanentReward => (
                "card_reward",
                "add_card",
                "card_reward:add_card",
                "card reward branch applied",
            ),
            card_reward::CardRewardBranchOptionSource::CombatGeneratedToHand => (
                "combat_card_reward",
                "combat_generated_card_to_hand",
                "combat_card_reward:to_hand",
                "combat card reward branch applied",
            ),
            card_reward::CardRewardBranchOptionSource::SkipCardReward => {
                return Self {
                    kind: "card_reward_skip",
                    effect_label: option.label.clone(),
                    label: option.label,
                    command: option.command,
                    card: None,
                    upgrades: None,
                    selected_cards: Vec::new(),
                    effect_kind: "skip_card_reward".to_string(),
                    effect_key: "card_reward:skip".to_string(),
                    representative_count: 1,
                    suppressed_count: 0,
                    success_reason: "card reward skip branch applied",
                };
            }
            card_reward::CardRewardBranchOptionSource::SingingBowl => {
                return Self {
                    kind: "card_reward_bowl",
                    effect_label: option.label.clone(),
                    label: option.label,
                    command: option.command,
                    card: None,
                    upgrades: None,
                    selected_cards: Vec::new(),
                    effect_kind: "singing_bowl".to_string(),
                    effect_key: "card_reward:singing_bowl".to_string(),
                    representative_count: 1,
                    suppressed_count: 0,
                    success_reason: "singing bowl card reward branch applied",
                };
            }
        };
        let card = option
            .card
            .expect("card reward branch option source should carry a card");
        let upgrades = option
            .upgrades
            .expect("card reward branch option source should carry upgrades");
        Self {
            kind,
            effect_label: option.label.clone(),
            label: option.label,
            command: option.command,
            card: Some(card),
            upgrades: Some(upgrades),
            selected_cards: selected_card_vec(Some(card), Some(upgrades)),
            effect_kind: effect_kind.to_string(),
            effect_key: format!("{effect_key_prefix}:{:?}:{}", card, upgrades),
            representative_count: 1,
            suppressed_count: 0,
            success_reason,
        }
    }

    fn from_campfire(option: CampfireBranchOption) -> Self {
        let effect_key = format!("campfire:{}:{}", option.effect_kind, option.command);
        Self {
            kind: "campfire",
            effect_label: option.label.clone(),
            label: option.label,
            command: option.command,
            card: option.card,
            upgrades: option.upgrades,
            selected_cards: selected_card_vec(option.card, option.upgrades),
            effect_kind: option.effect_kind,
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

    fn from_shop(option: ShopBranchOption) -> Self {
        let effect_key = format!("shop:{}:{}", option.effect_kind, option.command);
        Self {
            kind: match option.effect_kind.as_str() {
                "shop_purge" => "shop_policy_purge",
                "shop_leave" => "shop_leave",
                "shop_buy_card" => "shop_buy_card",
                "shop_buy_relic" => "shop_buy_relic",
                "shop_buy_potion" => "shop_buy_potion",
                "shop_buy_combo" => "shop_buy_combo",
                _ => "shop",
            },
            effect_label: option.effect_label,
            label: option.label,
            command: option.command,
            card: option.card,
            upgrades: None,
            selected_cards: selected_card_vec(option.card, Some(0)),
            effect_kind: option.effect_kind,
            effect_key,
            representative_count: option.representative_count,
            suppressed_count: option.suppressed_count,
            success_reason: "shop branch applied",
        }
    }

    fn from_reward(option: RewardBranchOption) -> Self {
        let success_reason = match option.kind {
            "reward_skip" => "reward skip branch applied",
            _ => "reward claim branch applied",
        };
        Self {
            kind: option.kind,
            effect_label: option.effect_label,
            label: option.label,
            command: option.command,
            card: None,
            upgrades: None,
            selected_cards: Vec::new(),
            effect_kind: option.effect_kind,
            effect_key: option.effect_key,
            representative_count: 1,
            suppressed_count: 0,
            success_reason,
        }
    }

    fn from_event(option: EventBranchOption) -> Self {
        Self {
            kind: "event",
            label: option.label,
            command: option.command,
            card: option.card,
            upgrades: option.upgrades,
            selected_cards: selected_card_vec(option.card, option.upgrades),
            effect_kind: option.effect_kind,
            effect_key: option.effect_key,
            effect_label: option.effect_label,
            representative_count: option.representative_count,
            suppressed_count: option.suppressed_count,
            success_reason: "event branch applied",
        }
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

#[cfg(test)]
mod tests;
