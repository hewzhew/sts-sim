#![allow(unused_imports)]

use super::super::*;
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionManagerState {
    pub action_static_state: ActionStaticState,
    pub phase: ActionManagerPhase,
    pub has_control: bool,
    pub turn_has_ended: bool,
    pub using_card: bool,
    pub monster_attacks_queued: bool,
    pub current_action: Option<ActionRef>,
    pub previous_action: Option<ActionRef>,
    pub turn_start_current_action: Option<ActionRef>,
    pub next_combat_actions: Vec<ActionState>,
    pub actions: Vec<ActionState>,
    pub pre_turn_actions: Vec<ActionState>,
    pub card_queue: Vec<CardQueueItemState>,
    pub monster_queue: Vec<MonsterQueueItemState>,
    pub cards_played_this_turn: Vec<CardRef>,
    pub cards_played_this_combat: Vec<CardRef>,
    pub orbs_channeled_this_turn: Vec<OrbRef>,
    pub orbs_channeled_this_combat: Vec<OrbRef>,
    pub unique_stances_this_combat: BTreeMap<String, i32>,
    pub mantra_gained: i32,
    pub last_card_ref: Option<CardRef>,
    pub total_discarded_this_turn: i32,
    pub damage_received_this_turn: i32,
    pub damage_received_this_combat: i32,
    pub hp_loss_this_combat: i32,
    pub player_hp_last_turn: i32,
    pub energy_gained_this_combat: i32,
    pub turn_index: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionStaticState {
    pub draw_card_action_drawn_cards: Vec<CardRef>,
    pub discard_action_num_discarded: i32,
    pub exhaust_action_num_exhausted: i32,
    pub nightmare_action_num_discarded: i32,
    pub put_on_deck_action_num_placed: i32,
    pub put_on_bottom_of_deck_action_num_placed: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionManagerPhase {
    WaitingOnUser,
    ExecutingActions,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionState {
    pub action_ref: ActionRef,
    pub action_class: String,
    pub action_type: ActionType,
    pub damage_type: Option<DamageType>,
    pub duration_bits: F32Bits,
    pub start_duration_bits: F32Bits,
    pub is_done: bool,
    pub source: Option<CombatantRef>,
    pub target: Option<CombatantRef>,
    pub amount: Option<i32>,
    pub damage_info: Option<DamageInfoState>,
    pub card_ref: Option<CardRef>,
    pub power_ref: Option<PowerRef>,
    pub relic_ref: Option<RelicRef>,
    pub potion_ref: Option<PotionRef>,
    pub action_payload: Option<ActionPayload>,
    pub unsupported_subclass_payload: Option<UnsupportedActionPayload>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    Block,
    Power,
    CardManipulation,
    Damage,
    Debuff,
    Discard,
    Draw,
    Exhaust,
    Heal,
    Energy,
    Text,
    Use,
    ClearCardQueue,
    Dialog,
    Special,
    Wait,
    Shuffle,
    ReducePower,
    Unknown { source_name: String },
}
