#![allow(unused_imports)]

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterGroupState {
    pub group_ref: String,
    pub monsters_in_slot_order: Vec<MonsterRef>,
    pub monsters: BTreeMap<MonsterRef, MonsterState>,
    pub hovered_monster_ref_if_mechanical: Option<MonsterRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterState {
    pub creature: CreatureState,
    pub monster_ref: MonsterRef,
    pub monster_id: String,
    pub enemy_type: EnemyType,
    pub slot: i32,
    pub death_timer_bits: F32Bits,
    pub tint_fade_out_called: bool,
    pub move_set: BTreeMap<i8, String>,
    pub max_hp_roll_state: Option<RngStreamState>,
    pub damage_entries: Vec<DamageInfoState>,
    pub move_state: MonsterMoveState,
    pub intent_state: IntentState,
    pub escape_next: bool,
    pub escaped: bool,
    pub cannot_escape: bool,
    pub half_dead: bool,
    pub monster_specific_payload: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnemyType {
    Normal,
    Elite,
    Boss,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonsterMoveState {
    pub next_move: i8,
    pub move_byte: i8,
    pub move_name_id: Option<String>,
    pub move_history: Vec<i8>,
    pub enemy_move_info: Option<EnemyMoveInfoState>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnemyMoveInfoState {
    pub next_move: i8,
    pub intent: IntentKind,
    pub base_damage: i32,
    pub multiplier: i32,
    pub is_multi_damage: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentState {
    pub visibility: IntentVisibility,
    pub intent_kind: Option<IntentKind>,
    pub tip_intent_kind: Option<IntentKind>,
    pub base_damage: Option<i32>,
    pub displayed_damage: Option<i32>,
    pub damage_per_hit: Option<i32>,
    pub hit_count: Option<i32>,
    pub is_multi_damage: bool,
    pub block_amount: Option<i32>,
    pub debuffs: Vec<String>,
    pub status_cards: Vec<String>,
    pub summon_or_escape_flags: Vec<String>,
    pub target_scope: TargetScope,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentVisibility {
    Visible,
    UnknownToPlayer,
    MissingVisibleBridgeBug,
    OracleOnly,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentKind {
    Attack,
    AttackBuff,
    AttackDebuff,
    AttackDefend,
    Buff,
    Debuff,
    StrongDebuff,
    Defend,
    DefendDebuff,
    DefendBuff,
    Escape,
    Magic,
    Sleep,
    Stun,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetScope {
    Player,
    SelfOnly,
    AllEnemies,
    RandomEnemy,
    None,
    Unknown { source_name: String },
}
