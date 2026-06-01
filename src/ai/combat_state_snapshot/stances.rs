#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StanceState {
    pub stance_ref: StanceRef,
    pub stance_id: String,
    pub name_id: String,
    pub description_id: String,
    pub particle_timer_bits: F32Bits,
    pub particle_timer2_bits: F32Bits,
    pub concrete_payload: BTreeMap<String, String>,
}
