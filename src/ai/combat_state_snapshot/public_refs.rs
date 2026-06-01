#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicRefState {
    pub next_card_ref: u64,
    pub next_monster_ref: u64,
    pub next_power_ref: u64,
    pub next_relic_ref: u64,
    pub next_potion_ref: u64,
    pub tombstones: Vec<PublicRefTombstone>,
    pub visibility_ledger: Vec<VisibilityLedgerEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicRefTombstone {
    pub ref_kind: String,
    pub ref_value: u64,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibilityLedgerEntry {
    pub public_ref: String,
    pub visibility: ZoneVisibility,
    pub notes: String,
}
