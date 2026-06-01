#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardQueueItemState {
    pub card_ref: Option<CardRef>,
    pub monster_ref: Option<MonsterRef>,
    pub energy_on_use: i32,
    pub ignore_energy_total: bool,
    pub autoplay_card: bool,
    pub random_target: bool,
    pub is_end_turn_auto_play: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterQueueItemState {
    pub monster_ref: MonsterRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCombatState {
    pub creature: CreatureState,
    pub player_class: PlayerClass,
    pub starting_max_hp: i32,
    pub master_deck_zone_ref: ZoneRef,
    pub draw_pile_zone_ref: ZoneRef,
    pub hand_zone_ref: ZoneRef,
    pub discard_pile_zone_ref: ZoneRef,
    pub exhaust_pile_zone_ref: ZoneRef,
    pub limbo_zone_ref: ZoneRef,
    pub relic_refs: Vec<RelicRef>,
    pub blight_refs: Vec<BlightRef>,
    pub potion_slot_refs: Vec<Option<PotionRef>>,
    pub energy: EnergyState,
    pub is_ending_turn: bool,
    pub end_turn_queued: bool,
    pub master_hand_size: i32,
    pub game_hand_size: i32,
    pub master_max_orbs: i32,
    pub max_orbs: i32,
    pub orb_refs_in_order: Vec<OrbRef>,
    pub stance_ref: StanceRef,
    pub card_in_use_ref: Option<CardRef>,
    pub damaged_this_combat: i32,
    pub deprecated_cards_played_this_turn_counter: i32,
    pub custom_mods: Vec<String>,
    pub class_specific_payload: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatureState {
    pub creature_ref: CombatantRef,
    pub creature_id: String,
    pub name_id: String,
    pub is_player: bool,
    pub hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub gold: i32,
    pub display_gold: i32,
    pub powers: Vec<PowerRef>,
    pub lifecycle: CreatureLifecycle,
    pub half_dead: bool,
    pub is_bloodied: bool,
    pub last_damage_taken: i32,
    pub escape_state: EscapeState,
    pub escape_timer_bits: F32Bits,
    pub mechanically_relevant_flags: BTreeMap<String, bool>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CombatantRef {
    Player,
    Monster(MonsterRef),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreatureLifecycle {
    Alive,
    Dying,
    Dead,
    Escaping,
    Escaped,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscapeState {
    pub is_escaping: bool,
    pub escaped: bool,
    pub escape_next: bool,
    pub cannot_escape: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnergyState {
    pub turn_energy: i32,
    pub energy_master: i32,
    pub panel_total_count: i32,
}
