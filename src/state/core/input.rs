use crate::core::EntityId;
use crate::state::selection::SelectionResolution;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum ClientInput {
    PlayCard {
        card_index: usize,
        target: Option<EntityId>,
    },
    UsePotion {
        potion_index: usize,
        target: Option<EntityId>,
    },
    DiscardPotion(usize),
    EndTurn,
    SubmitCardChoice(Vec<usize>),
    SubmitDiscoverChoice(usize),
    SelectMapNode(usize),
    FlyToNode(usize, usize),
    SelectEventOption(usize),
    CampfireOption(CampfireChoice),
    EventChoice(usize),
    SubmitScryDiscard(Vec<usize>),
    SubmitSelection(SelectionResolution),
    SubmitHandSelect(Vec<u32>),
    SubmitGridSelect(Vec<u32>),
    SubmitDeckSelect(Vec<usize>),
    ClaimReward(usize),
    OpenChest,
    SelectCard(usize),
    BuyCard(usize),
    BuyRelic(usize),
    BuyPotion(usize),
    PurgeCard(usize),
    SubmitRelicChoice(usize),
    Proceed,
    Cancel,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum CampfireChoice {
    Rest,
    Smith(usize),
    Dig,
    Lift,
    Toke(usize),
    Recall,
}
