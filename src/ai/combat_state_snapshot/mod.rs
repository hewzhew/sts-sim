//! AI-facing combat state snapshot schema.
//!
//! These types are an audit boundary, not a legacy engine wrapper. They are
//! shaped from `docs/AI_COMBAT_STATE_SCHEMA.md`: Java source defines combat
//! semantics, Rust owns the deterministic simulator representation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourcePath(pub String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SchemaPath(pub String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RustPath(pub String);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CardRef(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MonsterRef(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PowerRef(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RelicRef(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BlightRef(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PotionRef(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OrbRef(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StanceRef(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ZoneRef(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ActionRef(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct F32Bits(pub u32);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatStateSnapshot {
    pub source_manifest: SourceManifest,
    pub snapshot_origin: CombatSnapshotOrigin,
    pub dungeon_context: DungeonCombatContext,
    pub room_state: RoomCombatState,
    pub content_pools: CombatContentPoolState,
    pub global_temp: GlobalCombatTempState,
    pub action_manager: ActionManagerState,
    pub player: PlayerCombatState,
    pub monster_group: MonsterGroupState,
    pub card_store: CardInstanceStore,
    pub card_zones: CardZoneState,
    pub powers: PowerState,
    pub relics: RelicState,
    pub blights: BlightState,
    pub potions: PotionBeltState,
    pub orbs: OrbState,
    pub stance: StanceState,
    pub choice_screens: ChoiceScreenState,
    pub rng: CombatRngState,
    pub lifecycle: CombatLifecycleState,
    pub public_refs: PublicRefState,
    pub derived_values: DerivedCombatValues,
    pub source_coverage: Vec<SourceCoverageEntry>,
    pub migration_ledger: Vec<MigrationLedgerEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceManifest {
    pub cardcrawl_root: SourcePath,
    pub game_version: String,
    pub decompile_manifest_hash: String,
    pub source_files: Vec<SourceFileManifestEntry>,
    pub simulator_commit: String,
    pub schema_hash: String,
    pub content_manifest_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFileManifestEntry {
    pub source_path: SourcePath,
    pub sha256: String,
    pub byte_len: u64,
    pub line_count: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatSnapshotOrigin {
    AuthoredProbe { name: String },
    ReplayExtracted { replay_id: String },
    BridgeExtracted { capture_id: String },
}

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionManagerState {
    pub action_static_state: ActionStaticState,
    pub phase: ActionManagerPhase,
    pub has_control: bool,
    pub turn_has_ended: bool,
    pub using_card: bool,
    pub monster_attacks_queued: bool,
    pub current_action: Option<ActionRef>,
    pub previous_action: Option<ActionRef>,
    pub turn_start_current_action: Option<ActionRef>,
    pub next_combat_actions: Vec<ActionState>,
    pub actions: Vec<ActionState>,
    pub pre_turn_actions: Vec<ActionState>,
    pub card_queue: Vec<CardQueueItemState>,
    pub monster_queue: Vec<MonsterQueueItemState>,
    pub cards_played_this_turn: Vec<CardRef>,
    pub cards_played_this_combat: Vec<CardRef>,
    pub orbs_channeled_this_turn: Vec<OrbRef>,
    pub orbs_channeled_this_combat: Vec<OrbRef>,
    pub unique_stances_this_combat: BTreeMap<String, i32>,
    pub mantra_gained: i32,
    pub last_card_ref: Option<CardRef>,
    pub total_discarded_this_turn: i32,
    pub damage_received_this_turn: i32,
    pub damage_received_this_combat: i32,
    pub hp_loss_this_combat: i32,
    pub player_hp_last_turn: i32,
    pub energy_gained_this_combat: i32,
    pub turn_index: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionStaticState {
    pub draw_card_action_drawn_cards: Vec<CardRef>,
    pub discard_action_num_discarded: i32,
    pub exhaust_action_num_exhausted: i32,
    pub nightmare_action_num_discarded: i32,
    pub put_on_deck_action_num_placed: i32,
    pub put_on_bottom_of_deck_action_num_placed: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionManagerPhase {
    WaitingOnUser,
    ExecutingActions,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionState {
    pub action_ref: ActionRef,
    pub action_class: String,
    pub action_type: ActionType,
    pub attack_effect: AttackEffect,
    pub damage_type: Option<DamageType>,
    pub duration_bits: F32Bits,
    pub start_duration_bits: F32Bits,
    pub is_done: bool,
    pub source: Option<CombatantRef>,
    pub target: Option<CombatantRef>,
    pub amount: Option<i32>,
    pub damage_info: Option<DamageInfoState>,
    pub card_ref: Option<CardRef>,
    pub power_ref: Option<PowerRef>,
    pub relic_ref: Option<RelicRef>,
    pub potion_ref: Option<PotionRef>,
    pub action_payload: Option<ActionPayload>,
    pub unsupported_subclass_payload: Option<UnsupportedActionPayload>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    Block,
    Power,
    CardManipulation,
    Damage,
    Debuff,
    Discard,
    Draw,
    Exhaust,
    Heal,
    Energy,
    Text,
    Use,
    ClearCardQueue,
    Dialog,
    Special,
    Wait,
    Shuffle,
    ReducePower,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackEffect {
    BluntLight,
    BluntHeavy,
    SlashDiagonal,
    Smash,
    SlashHeavy,
    SlashHorizontal,
    SlashVertical,
    None,
    Fire,
    Poison,
    Shield,
    Lightning,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionPayload {
    ApplyPoisonOnRandomMonster(ApplyPoisonOnRandomMonsterActionState),
    ApplyPower(ApplyPowerActionState),
    ApplyPowerToRandomEnemy(ApplyPowerToRandomEnemyActionState),
    AttackDamageRandomEnemy(AttackDamageRandomEnemyActionState),
    BetterDiscardPileToHand(BetterDiscardPileToHandActionState),
    BetterDrawPileToHand(BetterDrawPileToHandActionState),
    Damage(DamageActionState),
    DamageAllEnemies(DamageAllEnemiesActionState),
    DamageRandomEnemy(DamageRandomEnemyActionState),
    DrawCard(DrawCardActionState),
    Discard(DiscardActionState),
    DiscardSpecificCard(DiscardSpecificCardActionState),
    EmptyDeckShuffle(EmptyDeckShuffleActionState),
    Exhaust(ExhaustActionState),
    ExhaustSpecificCard(ExhaustSpecificCardActionState),
    GainEnergy(GainEnergyActionState),
    MakeTempCardInDiscard(MakeTempCardInDiscardActionState),
    MakeTempCardInDiscardAndDeck(MakeTempCardInDiscardAndDeckActionState),
    MakeTempCardInDrawPile(MakeTempCardInDrawPileActionState),
    MakeTempCardInHand(MakeTempCardInHandActionState),
    ModifyBlock(ModifyBlockActionState),
    NewQueueCard(NewQueueCardActionState),
    PlayTopCard(PlayTopCardActionState),
    PutOnBottomOfDeck(PutOnBottomOfDeckActionState),
    PutOnDeck(PutOnDeckActionState),
    PummelDamage(PummelDamageActionState),
    QueueCard(QueueCardActionState),
    ReduceCost(ReduceCostActionState),
    ReduceCostForTurn(ReduceCostForTurnActionState),
    ReducePower(ReducePowerActionState),
    RemoveSpecificPower(RemoveSpecificPowerActionState),
    ReviveMonster(ReviveMonsterActionState),
    RollMove(RollMoveActionState),
    SetMove(SetMoveActionState),
    SetDontTrigger(SetDontTriggerActionState),
    Sfx(SfxActionState),
    SpawnMonster(SpawnMonsterActionState),
    Suicide(SuicideActionState),
    TextAboveCreature(TextAboveCreatureActionState),
    TransformCardInHand(TransformCardInHandActionState),
    Unlimbo(UnlimboActionState),
    UseCard(UseCardActionState),
}

pub const TYPED_ACTION_PAYLOAD_SOURCE_CLASSES: &[&str] = &[
    "ApplyPoisonOnRandomMonsterAction",
    "ApplyPowerAction",
    "ApplyPowerToRandomEnemyAction",
    "AttackDamageRandomEnemyAction",
    "BetterDiscardPileToHandAction",
    "BetterDrawPileToHandAction",
    "DamageAction",
    "DamageAllEnemiesAction",
    "DamageRandomEnemyAction",
    "DiscardAction",
    "DiscardSpecificCardAction",
    "DrawCardAction",
    "EmptyDeckShuffleAction",
    "ExhaustAction",
    "ExhaustSpecificCardAction",
    "GainEnergyAction",
    "MakeTempCardInDiscardAction",
    "MakeTempCardInDiscardAndDeckAction",
    "MakeTempCardInDrawPileAction",
    "MakeTempCardInHandAction",
    "ModifyBlockAction",
    "NewQueueCardAction",
    "PlayTopCardAction",
    "PutOnBottomOfDeckAction",
    "PutOnDeckAction",
    "PummelDamageAction",
    "QueueCardAction",
    "ReduceCostAction",
    "ReduceCostForTurnAction",
    "ReducePowerAction",
    "RemoveSpecificPowerAction",
    "ReviveMonsterAction",
    "RollMoveAction",
    "SetMoveAction",
    "SetDontTriggerAction",
    "SFXAction",
    "SpawnMonsterAction",
    "SuicideAction",
    "TextAboveCreatureAction",
    "TransformCardInHandAction",
    "UnlimboAction",
    "UseCardAction",
];

impl ActionPayload {
    pub fn java_source_class(&self) -> &'static str {
        match self {
            ActionPayload::ApplyPoisonOnRandomMonster(_) => "ApplyPoisonOnRandomMonsterAction",
            ActionPayload::ApplyPower(_) => "ApplyPowerAction",
            ActionPayload::ApplyPowerToRandomEnemy(_) => "ApplyPowerToRandomEnemyAction",
            ActionPayload::AttackDamageRandomEnemy(_) => "AttackDamageRandomEnemyAction",
            ActionPayload::BetterDiscardPileToHand(_) => "BetterDiscardPileToHandAction",
            ActionPayload::BetterDrawPileToHand(_) => "BetterDrawPileToHandAction",
            ActionPayload::Damage(_) => "DamageAction",
            ActionPayload::DamageAllEnemies(_) => "DamageAllEnemiesAction",
            ActionPayload::DamageRandomEnemy(_) => "DamageRandomEnemyAction",
            ActionPayload::DrawCard(_) => "DrawCardAction",
            ActionPayload::Discard(_) => "DiscardAction",
            ActionPayload::DiscardSpecificCard(_) => "DiscardSpecificCardAction",
            ActionPayload::EmptyDeckShuffle(_) => "EmptyDeckShuffleAction",
            ActionPayload::Exhaust(_) => "ExhaustAction",
            ActionPayload::ExhaustSpecificCard(_) => "ExhaustSpecificCardAction",
            ActionPayload::GainEnergy(_) => "GainEnergyAction",
            ActionPayload::MakeTempCardInDiscard(_) => "MakeTempCardInDiscardAction",
            ActionPayload::MakeTempCardInDiscardAndDeck(_) => "MakeTempCardInDiscardAndDeckAction",
            ActionPayload::MakeTempCardInDrawPile(_) => "MakeTempCardInDrawPileAction",
            ActionPayload::MakeTempCardInHand(_) => "MakeTempCardInHandAction",
            ActionPayload::ModifyBlock(_) => "ModifyBlockAction",
            ActionPayload::NewQueueCard(_) => "NewQueueCardAction",
            ActionPayload::PlayTopCard(_) => "PlayTopCardAction",
            ActionPayload::PutOnBottomOfDeck(_) => "PutOnBottomOfDeckAction",
            ActionPayload::PutOnDeck(_) => "PutOnDeckAction",
            ActionPayload::PummelDamage(_) => "PummelDamageAction",
            ActionPayload::QueueCard(_) => "QueueCardAction",
            ActionPayload::ReduceCost(_) => "ReduceCostAction",
            ActionPayload::ReduceCostForTurn(_) => "ReduceCostForTurnAction",
            ActionPayload::ReducePower(_) => "ReducePowerAction",
            ActionPayload::RemoveSpecificPower(_) => "RemoveSpecificPowerAction",
            ActionPayload::ReviveMonster(_) => "ReviveMonsterAction",
            ActionPayload::RollMove(_) => "RollMoveAction",
            ActionPayload::SetMove(_) => "SetMoveAction",
            ActionPayload::SetDontTrigger(_) => "SetDontTriggerAction",
            ActionPayload::Sfx(_) => "SFXAction",
            ActionPayload::SpawnMonster(_) => "SpawnMonsterAction",
            ActionPayload::Suicide(_) => "SuicideAction",
            ActionPayload::TextAboveCreature(_) => "TextAboveCreatureAction",
            ActionPayload::TransformCardInHand(_) => "TransformCardInHandAction",
            ActionPayload::Unlimbo(_) => "UnlimboAction",
            ActionPayload::UseCard(_) => "UseCardAction",
        }
    }
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
    pub effect: AttackEffect,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackDamageRandomEnemyActionState {
    pub card_ref: CardRef,
    pub effect: AttackEffect,
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
pub struct DamageActionState {
    pub gold_amount: i32,
    pub skip_wait: bool,
    pub mute_sfx: bool,
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
pub struct DrawCardActionState {
    pub shuffle_check: bool,
    pub clear_draw_history: bool,
    pub follow_up_action: Option<Box<ActionState>>,
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
pub struct ExhaustSpecificCardActionState {
    pub target_card: CardRef,
    pub group_zone_ref: ZoneRef,
    pub starting_duration_bits: F32Bits,
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
    pub auto_position: bool,
    pub to_bottom: bool,
    pub x_bits: F32Bits,
    pub y_bits: F32Bits,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakeTempCardInHandActionState {
    pub card_to_make: CardRef,
    pub is_other_card_in_center: bool,
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
pub struct ReviveMonsterActionState {
    pub healing_effect: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollMoveActionState {
    pub monster_ref: MonsterRef,
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
pub struct SfxActionState {
    pub key: String,
    pub pitch_var_bits: F32Bits,
    pub adjust: bool,
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
pub struct TextAboveCreatureActionState {
    pub used: bool,
    pub message: Option<String>,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardInstanceStore {
    pub cards: BTreeMap<CardRef, CardInstance>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardInstance {
    pub card_ref: CardRef,
    pub source_uuid: Option<String>,
    pub card_id: String,
    pub name_id: String,
    pub original_name_id: String,
    pub color: CardColor,
    pub card_type: CardType,
    pub rarity: CardRarity,
    pub target: CardTarget,
    pub tags: Vec<String>,
    pub keywords: Vec<String>,
    pub price: i32,
    pub upgraded: bool,
    pub times_upgraded: i32,
    pub upgraded_cost: bool,
    pub upgraded_damage: bool,
    pub upgraded_block: bool,
    pub upgraded_magic_number: bool,
    pub misc: i32,
    pub cost: i32,
    pub cost_for_turn: i32,
    pub charge_cost: i32,
    pub is_cost_modified: bool,
    pub is_cost_modified_for_turn: bool,
    pub free_to_play_once: bool,
    pub energy_on_use: i32,
    pub ignore_energy_on_use: bool,
    pub is_used: bool,
    pub is_seen: bool,
    pub is_locked: bool,
    pub is_selected: bool,
    pub show_evoke_value: bool,
    pub show_evoke_orb_count: i32,
    pub damage_type: DamageType,
    pub damage_type_for_turn: DamageType,
    pub base_damage: i32,
    pub damage: i32,
    pub is_damage_modified: bool,
    pub base_block: i32,
    pub block: i32,
    pub is_block_modified: bool,
    pub base_magic_number: i32,
    pub magic_number: i32,
    pub is_magic_number_modified: bool,
    pub base_heal: i32,
    pub heal: i32,
    pub base_draw: i32,
    pub draw: i32,
    pub base_discard: i32,
    pub discard: i32,
    pub multi_damage: Vec<i32>,
    pub is_multi_damage: bool,
    pub exhaust: bool,
    pub ethereal: bool,
    pub retain: bool,
    pub self_retain: bool,
    pub innate: bool,
    pub return_to_hand: bool,
    pub shuffle_back_into_draw_pile: bool,
    pub exhaust_on_use_once: bool,
    pub exhaust_on_fire: bool,
    pub dont_trigger_on_use_card: bool,
    pub purge_on_use: bool,
    pub is_in_autoplay: bool,
    pub in_bottle_flame: bool,
    pub in_bottle_lightning: bool,
    pub in_bottle_tornado: bool,
    pub cant_use_message: Option<String>,
    pub generated_by: Option<String>,
    pub card_specific_payload: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardColor {
    Red,
    Green,
    Blue,
    Purple,
    Colorless,
    Curse,
    Status,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardType {
    Attack,
    Skill,
    Power,
    Status,
    Curse,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardRarity {
    Basic,
    Common,
    Uncommon,
    Rare,
    Special,
    Curse,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardTarget {
    Enemy,
    AllEnemy,
    SelfOnly,
    SelfAndEnemy,
    None,
    All,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageType {
    Normal,
    Thorns,
    HpLoss,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageInfoState {
    pub owner: Option<CombatantRef>,
    pub name: Option<String>,
    pub damage_type: DamageType,
    pub output: i32,
    pub base: i32,
    pub is_modified: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardZoneState {
    pub master_deck: CardZone,
    pub draw_pile: CardZone,
    pub hand: CardZone,
    pub discard_pile: CardZone,
    pub exhaust_pile: CardZone,
    pub limbo: CardZone,
    pub card_in_play: Option<CardRef>,
    pub temporary_generated_cards: CardZone,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardZone {
    pub zone_ref: ZoneRef,
    pub zone_kind: CardZoneKind,
    pub ordered_card_refs: Vec<CardRef>,
    pub group_type: String,
    pub hand_positioning_map: BTreeMap<i32, i32>,
    pub queued_card_refs: Vec<CardRef>,
    pub in_hand_refs: Vec<CardRef>,
    pub public_visibility_mode: ZoneVisibility,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CardZoneKind {
    MasterDeck,
    DrawPile,
    Hand,
    DiscardPile,
    ExhaustPile,
    Limbo,
    CardInPlay,
    TemporaryGenerated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoneVisibility {
    OrderedVisible,
    CountVisible,
    HiddenOrder,
    Hidden,
}

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerState {
    pub power_instances: BTreeMap<PowerRef, PowerInstance>,
    pub owner_to_power_order: BTreeMap<CombatantRef, Vec<PowerRef>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerInstance {
    pub power_ref: PowerRef,
    pub power_id: String,
    pub name_id: String,
    pub description_id: String,
    pub owner_ref: CombatantRef,
    pub amount: i32,
    pub priority: i32,
    pub power_type: PowerType,
    pub is_turn_based: bool,
    pub is_post_action_power: bool,
    pub can_go_negative: bool,
    pub concrete_payload: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerType {
    Buff,
    Debuff,
    Neutral,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelicState {
    pub relic_instances: BTreeMap<RelicRef, RelicInstance>,
    pub relic_order: Vec<RelicRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelicInstance {
    pub relic_ref: RelicRef,
    pub relic_id: String,
    pub name_id: String,
    pub description_id: String,
    pub cost: i32,
    pub counter: i32,
    pub tier: RelicTier,
    pub used_up: bool,
    pub grayscale: bool,
    pub energy_based: bool,
    pub is_seen: bool,
    pub is_done: bool,
    pub is_animating: bool,
    pub is_obtained: bool,
    pub discarded: bool,
    pub concrete_payload: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelicTier {
    Starter,
    Common,
    Uncommon,
    Rare,
    Shop,
    Boss,
    Special,
    Deprecated,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlightState {
    pub blight_instances: BTreeMap<BlightRef, BlightInstance>,
    pub blight_order: Vec<BlightRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlightInstance {
    pub blight_ref: BlightRef,
    pub blight_id: String,
    pub counter: i32,
    pub concrete_payload: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PotionBeltState {
    pub slots: Vec<PotionSlotState>,
    pub potions: BTreeMap<PotionRef, PotionInstance>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PotionSlotState {
    pub slot_index: i32,
    pub potion_ref: Option<PotionRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PotionInstance {
    pub potion_ref: PotionRef,
    pub potion_id: String,
    pub name_id: String,
    pub description_id: String,
    pub slot: i32,
    pub potency: i32,
    pub effect: PotionEffectKind,
    pub color: PotionColorKind,
    pub rarity: PotionRarity,
    pub size: PotionSize,
    pub can_use: bool,
    pub target_required: bool,
    pub is_obtained: bool,
    pub discarded: bool,
    pub is_thrown: bool,
    pub concrete_payload: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PotionEffectKind {
    None,
    Known { source_name: String },
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PotionColorKind {
    Known { source_name: String },
    CustomRgb { r: u8, g: u8, b: u8, a: u8 },
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PotionRarity {
    Common,
    Uncommon,
    Rare,
    Placeholder,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PotionSize {
    Tiny,
    Small,
    Medium,
    Heart,
    Bottle,
    Sphere,
    Snecko,
    Fairy,
    Unknown { source_name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrbState {
    pub max_orbs: i32,
    pub orb_refs_in_order: Vec<OrbRef>,
    pub orb_instances: BTreeMap<OrbRef, OrbInstance>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrbInstance {
    pub orb_ref: OrbRef,
    pub orb_id: String,
    pub name_id: String,
    pub description_id: String,
    pub slot: i32,
    pub evoke_amount: i32,
    pub passive_amount: i32,
    pub base_evoke_amount: i32,
    pub base_passive_amount: i32,
    pub show_evoke_value: bool,
    pub channel_anim_timer_bits: F32Bits,
    pub concrete_payload: BTreeMap<String, String>,
}

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChoiceScreenState {
    pub active_screen: Option<ChoiceScreenKind>,
    pub grid_select: Option<GridSelectState>,
    pub hand_select: Option<HandSelectState>,
    pub generated_choice: Option<GeneratedChoiceState>,
    pub ordered_choice: Option<OrderedChoiceState>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChoiceScreenKind {
    GridSelect,
    HandSelect,
    GeneratedChoice,
    OrderedChoice,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridSelectState {
    pub target_group_zone_ref: Option<ZoneRef>,
    pub selected_card_refs: Vec<CardRef>,
    pub hovered_card_ref: Option<CardRef>,
    pub num_cards: i32,
    pub card_select_amount: i32,
    pub can_cancel: bool,
    pub for_upgrade: bool,
    pub for_transform: bool,
    pub for_purge: bool,
    pub confirm_screen_up: bool,
    pub is_just_for_confirming: bool,
    pub any_number: bool,
    pub for_clarity: bool,
    pub cancel_was_on: bool,
    pub cancel_text: Option<String>,
    pub tip_msg: String,
    pub last_tip: String,
    pub prev_deck_size: i32,
    pub upgrade_preview_card_ref: Option<CardRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandSelectState {
    pub num_cards_to_select: i32,
    pub selected_card_refs: Vec<CardRef>,
    pub hovered_card_ref: Option<CardRef>,
    pub upgrade_preview_card_ref: Option<CardRef>,
    pub selection_reason: String,
    pub were_cards_retrieved: bool,
    pub can_pick_zero: bool,
    pub up_to: bool,
    pub any_number: bool,
    pub for_transform: bool,
    pub for_upgrade: bool,
    pub num_selected: i32,
    pub message: String,
    pub hand_zone_ref: Option<ZoneRef>,
    pub wait_then_close_if_mechanical: bool,
    pub wait_to_close_timer_bits: F32Bits,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedChoiceState {
    pub cause: String,
    pub candidate_card_refs: Vec<CardRef>,
    pub selected_card_refs: Vec<CardRef>,
    pub can_skip: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderedChoiceState {
    pub cause: String,
    pub candidate_card_refs: Vec<CardRef>,
    pub selected_in_order: Vec<CardRef>,
    pub can_cancel: bool,
}

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceCoverageEntry {
    pub source_file: SourcePath,
    pub source_class: String,
    pub source_member: String,
    pub mechanic_role: String,
    pub classification: CoverageClassification,
    pub schema_path: Option<SchemaPath>,
    pub public_visibility: PublicVisibility,
    pub replay_required: bool,
    pub rust_owner_module: Option<RustPath>,
    pub rust_status: RustMigrationStatus,
    pub migration_decision: Option<String>,
    pub acceptance_check: Option<String>,
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoverageClassification {
    Modeled,
    Derived,
    RenderOnly,
    RunLevelMaterialized,
    NonCombat,
    UnsupportedAbort,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublicVisibility {
    Public,
    Privileged,
    DebugOnly,
    Hidden,
    NotApplicable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationLedgerEntry {
    pub java_source: SourcePath,
    pub java_methods: Vec<String>,
    pub java_fields: Vec<String>,
    pub java_semantic_behavior: String,
    pub rust_module: RustPath,
    pub rust_type: String,
    pub migration_kind: MigrationKind,
    pub preserved_features: Vec<String>,
    pub intentional_structural_changes: Vec<String>,
    pub semantic_equivalence_tests: Vec<String>,
    pub unsupported_cases: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationKind {
    DirectModel,
    DerivedModel,
    StructuralRedesign,
    UnsupportedAbort,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RustMigrationStatus {
    Keep,
    Rewrite,
    Delete,
    AdapterOnly,
    Unknown,
}
