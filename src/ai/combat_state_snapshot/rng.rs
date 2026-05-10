#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatRngState {
    pub monster_rng: Option<RngStreamState>,
    pub monster_hp_rng: Option<RngStreamState>,
    pub ai_rng: Option<RngStreamState>,
    pub shuffle_rng: Option<RngStreamState>,
    pub card_random_rng: Option<RngStreamState>,
    pub card_rng: Option<RngStreamState>,
    pub misc_rng: Option<RngStreamState>,
    pub potion_rng: Option<RngStreamState>,
    pub relic_rng_if_combat_consumed: Option<RngStreamState>,
    pub treasure_rng_if_combat_consumed: Option<RngStreamState>,
    pub custom_streams: BTreeMap<String, RngStreamState>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RngStreamState {
    pub stream_id: String,
    pub xs128_state_0: u64,
    pub xs128_state_1: u64,
    pub counter: u32,
}
