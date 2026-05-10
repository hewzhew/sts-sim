#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatLifecycleState {
    pub combat_started: bool,
    pub pre_battle_actions_applied: bool,
    pub monster_pre_battle_actions_applied: bool,
    pub player_start_combat_hooks_applied: bool,
    pub turn_start_hooks_applied_for_turn: Option<i32>,
    pub combat_end_hooks_applied: bool,
    pub terminal_reached: bool,
    pub reward_generation_started: bool,
    pub reward_screen_reached: bool,
}
