#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DungeonCombatContext {
    pub dungeon_name: String,
    pub level_num: String,
    pub player_class: PlayerClass,
    pub floor_num: i32,
    pub act_num: i32,
    pub ascension_level: i32,
    pub is_ascension_mode: bool,
    pub curr_map_node_ref: Option<String>,
    pub dungeon_id: String,
    pub boss_key: Option<String>,
    pub screen_state: ScreenState,
    pub combat_relevant_global_flags: BTreeMap<String, bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerClass {
    Ironclad,
    Silent,
    Defect,
    Watcher,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenState {
    None,
    Combat,
    GridSelect,
    HandSelect,
    Other { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomCombatState {
    pub room_kind: RoomKind,
    pub phase: RoomPhase,
    pub map_symbol: Option<String>,
    pub monster_group_ref: String,
    pub is_battle_over: bool,
    pub cannot_lose: bool,
    pub elite_trigger: bool,
    pub blizzard_potion_mod: i32,
    pub mugged: bool,
    pub smoked: bool,
    pub combat_event: bool,
    pub reward_allowed: bool,
    pub reward_time: bool,
    pub skip_monster_turn: bool,
    pub base_rare_card_chance: i32,
    pub base_uncommon_card_chance: i32,
    pub rare_card_chance: i32,
    pub uncommon_card_chance: i32,
    pub combat_end_timer_state: TimerState,
    pub reward_pop_out_timer_bits: F32Bits,
    pub wait_timer_bits: F32Bits,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatContentPoolState {
    pub src_colorless_card_pool: Vec<String>,
    pub src_curse_card_pool: Vec<String>,
    pub src_common_card_pool: Vec<String>,
    pub src_uncommon_card_pool: Vec<String>,
    pub src_rare_card_pool: Vec<String>,
    pub colorless_card_pool: Vec<String>,
    pub curse_card_pool: Vec<String>,
    pub common_card_pool: Vec<String>,
    pub uncommon_card_pool: Vec<String>,
    pub rare_card_pool: Vec<String>,
    pub common_relic_pool: Vec<String>,
    pub uncommon_relic_pool: Vec<String>,
    pub rare_relic_pool: Vec<String>,
    pub shop_relic_pool: Vec<String>,
    pub boss_relic_pool: Vec<String>,
    pub monster_list: Vec<String>,
    pub elite_monster_list: Vec<String>,
    pub boss_list: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalCombatTempState {
    pub transformed_card_ref: Option<CardRef>,
    pub loading_post_combat: bool,
    pub is_victory: bool,
    pub turn_phase_effect_active: bool,
    pub colorless_rare_chance_bits: F32Bits,
    pub card_blizz_start_offset: i32,
    pub card_blizz_randomizer: i32,
    pub card_blizz_growth: i32,
    pub card_blizz_max_offset: i32,
    pub boss_count: i32,
    pub relics_to_remove_on_start: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomKind {
    Monster,
    Elite,
    Boss,
    EventCombat,
    Unknown { source_name: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomPhase {
    Combat,
    Complete,
    Event,
    Incomplete,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerState {
    pub ticks_or_millis: i64,
    pub source_field: String,
    pub mechanical: bool,
}
