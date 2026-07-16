use serde::{Deserialize, Serialize};

use super::super::{
    CombatPublicObservationV1, HiddenInformationReasonV1, ObservationEvidenceKindV1,
};

pub const COMBAT_POLICY_OBSERVATION_SCHEMA_NAME: &str = "CombatPolicyObservationV1";
pub const COMBAT_POLICY_OBSERVATION_SCHEMA_VERSION: u32 = 1;

/// Public combat facts that may affect policy choice.
///
/// This is intentionally separate from `CombatPublicObservationV1`. The older
/// schema remains a compatibility boundary for fingerprints and evidence,
/// while this schema can grow under an explicit policy-version contract.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPolicyObservationV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub compatibility_public: CombatPublicObservationV1,
    pub encounter: CombatPolicyEncounterV1,
    pub turn: CombatPolicyTurnV1,
    pub player_runtime: CombatPolicyPlayerRuntimeV1,
    pub zones: CombatPolicyZonesV1,
    pub monster_runtime: Vec<CombatPolicyMonsterRuntimeV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPolicyEncounterV1 {
    pub is_boss_fight: bool,
    pub is_elite_fight: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPolicyTurnV1 {
    pub turn_count: u32,
    pub phase: CombatPolicyPhaseV1,
    pub turn_start_draw_modifier: i32,
    pub counters: CombatPolicyTurnCountersV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatPolicyPhaseV1 {
    PlayerTurn,
    MonsterTurn,
    TurnTransition,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPolicyTurnCountersV1 {
    pub cards_played_this_turn: u8,
    pub attacks_played_this_turn: u8,
    pub cards_discarded_this_turn: u16,
    pub card_ids_played_this_turn: Vec<String>,
    pub card_ids_played_this_combat: Vec<String>,
    pub orbs_channeled_this_turn: Vec<CombatPolicyOrbIdV1>,
    pub orbs_channeled_this_combat: Vec<CombatPolicyOrbIdV1>,
    pub mantra_gained_this_combat: i32,
    pub times_damaged_this_combat: u8,
    pub discovery_cost_for_turn: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPolicyPlayerRuntimeV1 {
    pub gold: i32,
    pub gold_delta_this_combat: i32,
    pub facing_left: bool,
    pub energy_master: u8,
    pub stance: CombatPolicyStanceV1,
    pub max_orbs: u8,
    pub orbs: Vec<CombatPolicyOrbV1>,
    pub relics: Vec<CombatPolicyRelicV1>,
    pub powers: Vec<CombatPolicyPowerV1>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatPolicyStanceV1 {
    Neutral,
    Wrath,
    Calm,
    Divinity,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatPolicyOrbIdV1 {
    Empty,
    Lightning,
    Dark,
    Frost,
    Plasma,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPolicyOrbV1 {
    pub orb_id: CombatPolicyOrbIdV1,
    pub base_passive_amount: i32,
    pub base_evoke_amount: i32,
    pub passive_amount: i32,
    pub evoke_amount: i32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPolicyRelicV1 {
    pub relic_id: String,
    pub counter: i32,
    pub used_up: bool,
    pub amount: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPolicyPowerV1 {
    pub power_id: String,
    pub amount: i32,
    pub extra_data: i32,
    /// Public timing fact derived from whether the power was applied this round.
    pub fresh_this_round: bool,
    pub payload: CombatPolicyPowerPayloadV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatPolicyPowerPayloadV1 {
    None,
    Card { card: CombatPolicyCardV1 },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPolicyZonesV1 {
    pub hand: Vec<CombatPolicyCardV1>,
    pub draw: CombatPolicyCardPileV1,
    pub discard: CombatPolicyCardPileV1,
    pub exhaust: CombatPolicyCardPileV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPolicyCardPileV1 {
    pub count: usize,
    pub evidence: ObservationEvidenceKindV1,
    pub hidden_reason: Option<HiddenInformationReasonV1>,
    pub cards: Vec<CombatPolicyCardV1>,
}

/// All mechanically relevant, player-visible card state except exact UUID.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPolicyCardV1 {
    pub card_id: String,
    pub upgrades: u8,
    pub misc_value: i32,
    pub base_damage_override: Option<i32>,
    pub base_block_override: Option<i32>,
    pub cost_modifier: i8,
    pub combat_cost: i32,
    pub cost_for_turn: i32,
    pub base_damage_mut: i32,
    pub base_block_mut: i32,
    pub base_magic_num_mut: i32,
    pub multi_damage: Vec<i32>,
    pub exhaust_override: Option<bool>,
    pub retain_override: Option<bool>,
    pub free_to_play_once: bool,
    pub energy_on_use: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPolicyMonsterRuntimeV1 {
    pub monster_slot: u8,
    pub powers: Vec<CombatPolicyPowerV1>,
}
