#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerState {
    pub power_instances: BTreeMap<PowerRef, PowerInstance>,
    pub owner_to_power_order: BTreeMap<CombatantRef, Vec<PowerRef>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerInstance {
    pub power_ref: PowerRef,
    pub power_id: String,
    pub name_id: String,
    pub description_id: String,
    pub owner_ref: CombatantRef,
    pub amount: i32,
    pub priority: i32,
    pub power_type: PowerType,
    pub is_turn_based: bool,
    pub is_post_action_power: bool,
    pub can_go_negative: bool,
    pub concrete_payload: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerType {
    Buff,
    Debuff,
    Neutral,
    Unknown { source_name: String },
}
