#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChoiceScreenState {
    pub active_screen: Option<ChoiceScreenKind>,
    pub card_reward: Option<CardRewardScreenState>,
    pub grid_select: Option<GridSelectState>,
    pub hand_select: Option<HandSelectState>,
    pub generated_choice: Option<GeneratedChoiceState>,
    pub ordered_choice: Option<OrderedChoiceState>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChoiceScreenKind {
    CardReward,
    GridSelect,
    HandSelect,
    GeneratedChoice,
    OrderedChoice,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardRewardScreenState {
    pub reward_card_refs: Vec<CardRef>,
    pub discovery_card_ref: Option<CardRef>,
    pub codex_card_ref: Option<CardRef>,
    pub touch_card_ref: Option<CardRef>,
    pub reward_item_ref: Option<RewardItemRef>,
    pub has_taken_all: bool,
    pub card_only: bool,
    pub draft: bool,
    pub discovery: bool,
    pub choose_one: bool,
    pub codex: bool,
    pub skippable: bool,
    pub header: String,
    pub draft_count: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridSelectState {
    pub target_group_zone_ref: Option<ZoneRef>,
    pub selected_card_refs: Vec<CardRef>,
    pub hovered_card_ref: Option<CardRef>,
    pub num_cards: i32,
    pub card_select_amount: i32,
    pub can_cancel: bool,
    pub for_upgrade: bool,
    pub for_transform: bool,
    pub for_purge: bool,
    pub confirm_screen_up: bool,
    pub is_just_for_confirming: bool,
    pub any_number: bool,
    pub for_clarity: bool,
    pub cancel_was_on: bool,
    pub cancel_text: Option<String>,
    pub tip_msg: String,
    pub last_tip: String,
    pub prev_deck_size: i32,
    pub upgrade_preview_card_ref: Option<CardRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandSelectState {
    pub num_cards_to_select: i32,
    pub selected_card_refs: Vec<CardRef>,
    pub hovered_card_ref: Option<CardRef>,
    pub upgrade_preview_card_ref: Option<CardRef>,
    pub selection_reason: String,
    pub were_cards_retrieved: bool,
    pub can_pick_zero: bool,
    pub up_to: bool,
    pub any_number: bool,
    pub for_transform: bool,
    pub for_upgrade: bool,
    pub num_selected: i32,
    pub message: String,
    pub hand_zone_ref: Option<ZoneRef>,
    pub wait_then_close_if_mechanical: bool,
    pub wait_to_close_timer_bits: F32Bits,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedChoiceState {
    pub cause: String,
    pub candidate_card_refs: Vec<CardRef>,
    pub selected_card_refs: Vec<CardRef>,
    pub can_skip: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderedChoiceState {
    pub cause: String,
    pub candidate_card_refs: Vec<CardRef>,
    pub selected_in_order: Vec<CardRef>,
    pub can_cancel: bool,
}
