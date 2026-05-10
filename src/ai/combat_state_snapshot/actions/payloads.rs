#![allow(unused_imports)]

use super::super::*;
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddCardToDeckActionState {
    pub card_to_obtain: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPoisonOnRandomMonsterActionState {
    pub starting_duration_bits: F32Bits,
    pub power_to_apply: Option<PowerRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPowerActionState {
    pub power_to_apply: PowerRef,
    pub starting_duration_bits: F32Bits,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyPowerToRandomEnemyActionState {
    pub power_to_apply: PowerRef,
    pub is_fast: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackDamageRandomEnemyActionState {
    pub card_ref: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BetterDiscardPileToHandActionState {
    pub number_of_cards: i32,
    pub optional: bool,
    pub new_cost: i32,
    pub set_cost: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BetterDrawPileToHandActionState {
    pub number_of_cards: i32,
    pub optional: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BurnIncreaseActionState {
    pub got_burned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChooseOneColorlessActionState {
    pub retrieve_card: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalDrawActionState {
    pub restricted_type: CardType,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodexActionState {
    pub retrieve_card: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageActionState {
    pub gold_amount: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageAllEnemiesActionState {
    pub damage: Vec<i32>,
    pub base_damage: i32,
    pub first_frame: bool,
    pub utilize_base_damage: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageRandomEnemyActionState {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscardToHandActionState {
    pub card_ref: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrawCardActionState {
    pub shuffle_check: bool,
    pub clear_draw_history: bool,
    pub follow_up_action: Option<Box<ActionState>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrawPileToHandActionState {
    pub type_to_check: CardType,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscardActionState {
    pub is_random: bool,
    pub end_turn: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscardSpecificCardActionState {
    pub target_card: CardRef,
    pub group_zone_ref: Option<ZoneRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryActionState {
    pub retrieve_card: bool,
    pub return_colorless: bool,
    pub card_type: Option<CardType>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmptyDeckShuffleActionState {
    pub shuffled: bool,
    pub vfx_done: bool,
    pub count: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExhaustActionState {
    pub is_random: bool,
    pub any_number: bool,
    pub can_pick_zero: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExhaustToHandActionState {
    pub card_ref: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExhaustSpecificCardActionState {
    pub target_card: CardRef,
    pub group_zone_ref: ZoneRef,
    pub starting_duration_bits: F32Bits,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForeignInfluenceActionState {
    pub retrieve_card: bool,
    pub upgraded: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GainEnergyActionState {
    pub energy_gain: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeTempCardInDiscardActionState {
    pub card_to_make: CardRef,
    pub num_cards: i32,
    pub same_uuid: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeTempCardInDiscardAndDeckActionState {
    pub card_to_make: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeTempCardInDrawPileActionState {
    pub card_to_make: CardRef,
    pub random_spot: bool,
    pub to_bottom: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeTempCardInHandActionState {
    pub card_to_make: CardRef,
    pub same_uuid: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifyBlockActionState {
    pub target_uuid: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewQueueCardActionState {
    pub card_ref: Option<CardRef>,
    pub random_target: bool,
    pub immediate_card: bool,
    pub autoplay_card: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObtainPotionActionState {
    pub potion_ref: PotionRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayTopCardActionState {
    pub exhaust_cards: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutOnBottomOfDeckActionState {
    pub is_random: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutOnDeckActionState {
    pub is_random: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PummelDamageActionState {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueCardActionState {
    pub card_ref: Option<CardRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReApplyPowersActionState {
    pub card_ref: CardRef,
    pub monster_ref: MonsterRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReduceCostActionState {
    pub target_uuid: Option<String>,
    pub card_ref: Option<CardRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReduceCostForTurnActionState {
    pub target_card: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReducePowerActionState {
    pub power_id: Option<String>,
    pub power_ref: Option<PowerRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveSpecificPowerActionState {
    pub power_id: Option<String>,
    pub power_ref: Option<PowerRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResetFlagsActionState {
    pub card_ref: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviveMonsterActionState {}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollMoveActionState {
    pub monster_ref: MonsterRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScryActionState {
    pub starting_duration_bits: F32Bits,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetMoveActionState {
    pub monster_ref: MonsterRef,
    pub next_move: i8,
    pub next_intent: IntentKind,
    pub next_damage: i32,
    pub next_name: Option<String>,
    pub multiplier: i32,
    pub is_multiplier: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetDontTriggerActionState {
    pub card_ref: CardRef,
    pub trigger: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowCardActionState {
    pub card_ref: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowCardAndPoofActionState {
    pub card_ref: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnMonsterActionState {
    pub used: bool,
    pub monster_ref: MonsterRef,
    pub minion: bool,
    pub target_slot: i32,
    pub use_smart_positioning: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuicideActionState {
    pub monster_ref: MonsterRef,
    pub relic_trigger: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransformCardInHandActionState {
    pub replacement_card: CardRef,
    pub hand_index: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnlimboActionState {
    pub card_ref: CardRef,
    pub exhaust: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCardDescriptionActionState {
    pub target_card: CardRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UseCardActionState {
    pub target_card: CardRef,
    pub card_target: Option<CombatantRef>,
    pub exhaust_card: bool,
    pub return_to_hand: bool,
    pub rebound_card: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedActionPayload {
    pub source_class: String,
    pub source_fields: BTreeMap<String, String>,
    pub abort_reason: UnsupportedActionAbortReason,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnsupportedActionAbortReason {
    UnmodeledActionSubclass,
    UnmodeledSourceField { field_name: String },
    OpaqueEngineState { field_name: String },
    Unknown { source_name: String },
}
