#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrbState {
    pub max_orbs: i32,
    pub orb_refs_in_order: Vec<OrbRef>,
    pub orb_instances: BTreeMap<OrbRef, OrbInstance>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrbInstance {
    pub orb_ref: OrbRef,
    pub orb_id: String,
    pub name_id: String,
    pub description_id: String,
    pub slot: i32,
    pub evoke_amount: i32,
    pub passive_amount: i32,
    pub base_evoke_amount: i32,
    pub base_passive_amount: i32,
    pub show_evoke_value: bool,
    pub channel_anim_timer_bits: F32Bits,
    pub concrete_payload: BTreeMap<String, String>,
}
