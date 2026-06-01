#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardInstanceStore {
    pub cards: BTreeMap<CardRef, CardInstance>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardInstance {
    pub card_ref: CardRef,
    pub source_uuid: Option<String>,
    pub card_id: String,
    pub name_id: String,
    pub original_name_id: String,
    pub color: CardColor,
    pub card_type: CardType,
    pub rarity: CardRarity,
    pub target: CardTarget,
    pub tags: Vec<String>,
    pub keywords: Vec<String>,
    pub price: i32,
    pub upgraded: bool,
    pub times_upgraded: i32,
    pub upgraded_cost: bool,
    pub upgraded_damage: bool,
    pub upgraded_block: bool,
    pub upgraded_magic_number: bool,
    pub misc: i32,
    pub cost: i32,
    pub cost_for_turn: i32,
    pub charge_cost: i32,
    pub is_cost_modified: bool,
    pub is_cost_modified_for_turn: bool,
    pub free_to_play_once: bool,
    pub energy_on_use: i32,
    pub ignore_energy_on_use: bool,
    pub is_used: bool,
    pub is_seen: bool,
    pub is_locked: bool,
    pub is_selected: bool,
    pub show_evoke_value: bool,
    pub show_evoke_orb_count: i32,
    pub damage_type: DamageType,
    pub damage_type_for_turn: DamageType,
    pub base_damage: i32,
    pub damage: i32,
    pub is_damage_modified: bool,
    pub base_block: i32,
    pub block: i32,
    pub is_block_modified: bool,
    pub base_magic_number: i32,
    pub magic_number: i32,
    pub is_magic_number_modified: bool,
    pub base_heal: i32,
    pub heal: i32,
    pub base_draw: i32,
    pub draw: i32,
    pub base_discard: i32,
    pub discard: i32,
    pub multi_damage: Vec<i32>,
    pub is_multi_damage: bool,
    pub exhaust: bool,
    pub ethereal: bool,
    pub retain: bool,
    pub self_retain: bool,
    pub innate: bool,
    pub return_to_hand: bool,
    pub shuffle_back_into_draw_pile: bool,
    pub exhaust_on_use_once: bool,
    pub exhaust_on_fire: bool,
    pub dont_trigger_on_use_card: bool,
    pub purge_on_use: bool,
    pub is_in_autoplay: bool,
    pub in_bottle_flame: bool,
    pub in_bottle_lightning: bool,
    pub in_bottle_tornado: bool,
    pub cant_use_message: Option<String>,
    pub generated_by: Option<String>,
    pub card_specific_payload: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardColor {
    Red,
    Green,
    Blue,
    Purple,
    Colorless,
    Curse,
    Status,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardType {
    Attack,
    Skill,
    Power,
    Status,
    Curse,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardRarity {
    Basic,
    Common,
    Uncommon,
    Rare,
    Special,
    Curse,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardTarget {
    Enemy,
    AllEnemy,
    SelfOnly,
    SelfAndEnemy,
    None,
    All,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageType {
    Normal,
    Thorns,
    HpLoss,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageInfoState {
    pub owner: Option<CombatantRef>,
    pub name: Option<String>,
    pub damage_type: DamageType,
    pub output: i32,
    pub base: i32,
    pub is_modified: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardZoneState {
    pub master_deck: CardZone,
    pub draw_pile: CardZone,
    pub hand: CardZone,
    pub discard_pile: CardZone,
    pub exhaust_pile: CardZone,
    pub limbo: CardZone,
    pub card_in_play: Option<CardRef>,
    pub temporary_generated_cards: CardZone,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardZone {
    pub zone_ref: ZoneRef,
    pub zone_kind: CardZoneKind,
    pub ordered_card_refs: Vec<CardRef>,
    pub group_type: String,
    pub hand_positioning_map: BTreeMap<i32, i32>,
    pub queued_card_refs: Vec<CardRef>,
    pub in_hand_refs: Vec<CardRef>,
    pub public_visibility_mode: ZoneVisibility,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardZoneKind {
    MasterDeck,
    DrawPile,
    Hand,
    DiscardPile,
    ExhaustPile,
    Limbo,
    CardInPlay,
    TemporaryGenerated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoneVisibility {
    OrderedVisible,
    CountVisible,
    HiddenOrder,
    Hidden,
}
