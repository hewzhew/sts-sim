#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlightState {
    pub blight_instances: BTreeMap<BlightRef, BlightInstance>,
    pub blight_order: Vec<BlightRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlightInstance {
    pub blight_ref: BlightRef,
    pub blight_id: String,
    pub counter: i32,
    pub concrete_payload: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PotionBeltState {
    pub slots: Vec<PotionSlotState>,
    pub potions: BTreeMap<PotionRef, PotionInstance>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PotionSlotState {
    pub slot_index: i32,
    pub potion_ref: Option<PotionRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PotionInstance {
    pub potion_ref: PotionRef,
    pub potion_id: String,
    pub name_id: String,
    pub description_id: String,
    pub slot: i32,
    pub potency: i32,
    pub effect: PotionEffectKind,
    pub color: PotionColorKind,
    pub rarity: PotionRarity,
    pub size: PotionSize,
    pub can_use: bool,
    pub target_required: bool,
    pub is_obtained: bool,
    pub discarded: bool,
    pub is_thrown: bool,
    pub concrete_payload: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PotionEffectKind {
    None,
    Known { source_name: String },
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PotionColorKind {
    Known { source_name: String },
    CustomRgb { r: u8, g: u8, b: u8, a: u8 },
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PotionRarity {
    Common,
    Uncommon,
    Rare,
    Placeholder,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PotionSize {
    Tiny,
    Small,
    Medium,
    Heart,
    Bottle,
    Sphere,
    Snecko,
    Fairy,
    Unknown { source_name: String },
}
