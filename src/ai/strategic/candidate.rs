use crate::content::cards::CardId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategicDecisionSite {
    CardReward,
    Shop,
    Event,
    Campfire,
    Route,
    BranchRetention,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateAction {
    TakeCard {
        index: usize,
        card: CardId,
    },
    SkipCardReward,
    BuyCard {
        shop_index: usize,
        card: CardId,
        gold: i32,
    },
    BuyRelic {
        shop_index: usize,
        relic: RelicId,
        gold: i32,
    },
    BuyPotion {
        shop_index: usize,
        potion: PotionId,
        gold: i32,
    },
    RemoveCard {
        deck_index: usize,
        card: CardId,
        gold: Option<i32>,
    },
    SmithCard {
        deck_index: usize,
        card: CardId,
    },
    Rest,
    LeaveShop,
    EventOption {
        index: usize,
        label: String,
    },
    RouteChoice {
        command: String,
    },
    Unknown {
        id: String,
        label: String,
    },
}

impl CandidateAction {
    pub fn candidate_id(&self) -> String {
        match self {
            Self::TakeCard { index, card } => format!("card_reward:{index}:{card:?}"),
            Self::SkipCardReward => "card_reward:skip".to_string(),
            Self::BuyCard {
                shop_index, card, ..
            } => format!("shop:buy_card:{shop_index}:{card:?}"),
            Self::BuyRelic {
                shop_index, relic, ..
            } => format!("shop:buy_relic:{shop_index}:{relic:?}"),
            Self::BuyPotion {
                shop_index, potion, ..
            } => format!("shop:buy_potion:{shop_index}:{potion:?}"),
            Self::RemoveCard {
                deck_index, card, ..
            } => format!("shop:remove:{deck_index}:{card:?}"),
            Self::SmithCard { deck_index, card } => format!("campfire:smith:{deck_index}:{card:?}"),
            Self::Rest => "campfire:rest".to_string(),
            Self::LeaveShop => "shop:leave".to_string(),
            Self::EventOption { index, label } => format!("event:{index}:{label}"),
            Self::RouteChoice { command } => format!("route:{command}"),
            Self::Unknown { id, .. } => id.clone(),
        }
    }
}
