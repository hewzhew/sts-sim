#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivedCombatValues {
    pub rendered_card_values: BTreeMap<CardRef, RenderedCardValues>,
    pub legal_playability_cache: BTreeMap<CardRef, PlayabilityState>,
    pub visible_intents: BTreeMap<MonsterRef, IntentState>,
    pub public_zone_summaries: BTreeMap<ZoneRef, PublicZoneSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderedCardValues {
    pub damage: i32,
    pub block: i32,
    pub magic: i32,
    pub cost_for_turn: i32,
    pub cache_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayabilityState {
    pub playable: bool,
    pub public_reason_if_unplayable: Option<String>,
    pub cache_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicZoneSummary {
    pub total_count: usize,
    pub visible_order: bool,
    pub counts_by_card_id: BTreeMap<String, usize>,
}
