#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelicState {
    pub relic_instances: BTreeMap<RelicRef, RelicInstance>,
    pub relic_order: Vec<RelicRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelicInstance {
    pub relic_ref: RelicRef,
    pub relic_id: String,
    pub name_id: String,
    pub description_id: String,
    pub cost: i32,
    pub counter: i32,
    pub tier: RelicTier,
    pub used_up: bool,
    pub grayscale: bool,
    pub energy_based: bool,
    pub is_seen: bool,
    pub is_done: bool,
    pub is_animating: bool,
    pub is_obtained: bool,
    pub discarded: bool,
    pub concrete_payload: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelicTier {
    Starter,
    Common,
    Uncommon,
    Rare,
    Shop,
    Boss,
    Special,
    Deprecated,
    Unknown { source_name: String },
}
