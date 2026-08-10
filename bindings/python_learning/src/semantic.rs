//! Versioned sparse semantic tensors for the standalone Python bridge.
//!
//! The simulator continues to own typed observations and actions. This module
//! is the only owner of their numeric learning vocabulary. Runtime identities
//! such as card UUIDs are used to resolve graph edges and are never emitted as
//! categorical or scalar features.

mod combat;

use std::collections::BTreeMap;
use std::fmt;

use sts_oracle_eval::ai::planner_core::{
    PlannerAction, PlannerDecisionContext, PlannerDecisionSite, PlannerPlayerClass,
    PlannerRewardDescriptor, PlannerRunGoal,
};
use sts_oracle_eval::content::cards::CardId;
use sts_oracle_eval::content::monsters::{factory::EncounterId, EnemyId};
use sts_oracle_eval::content::potions::PotionId;
use sts_oracle_eval::content::powers::PowerId;
use sts_oracle_eval::content::relics::RelicId;
use sts_oracle_learning::eval::run_control::{
    LearningModelCandidateSemanticsV1, LearningModelDecisionV1, LearningModelObservationV1,
    LearningRunSelectionFamilyV1, LearningSelectionCandidateSemanticsV1,
    LearningSelectionDecisionV1, LearningSelectionDraftV1, LearningStrategicContextKindV1,
    LearningStrategicModelObservationV1,
};
use sts_oracle_eval::sim::combat_action_surface::CombatSelectionDistinctByV2;
use sts_oracle_eval::state::events::{EventActionKind, EventId};
use sts_oracle_eval::state::map::node::RoomType;
use sts_oracle_eval::state::selection::{SelectionReason, SelectionScope};

pub const SEMANTIC_SCHEMA_VERSION: u32 = 5;
pub const NO_CANDIDATE_TOKEN: u64 = u64::MAX;
pub const CARD_ID_VOCABULARY_SIZE: u64 = 371;
pub const RELIC_ID_VOCABULARY_SIZE: u64 = 182;
pub const POTION_ID_VOCABULARY_SIZE: u64 = 42;
pub const ENCOUNTER_ID_VOCABULARY_SIZE: u64 = 65;
pub const EVENT_ID_VOCABULARY_SIZE: u64 = 53;
pub const ENEMY_ID_VOCABULARY_SIZE: u64 = 65;
pub const POWER_ID_VOCABULARY_SIZE: u64 = 135;

const RUN_SELECTION_DECK_CARD_UUID_INPUT_ENCODING: i64 = 3;

// Domain identities use their fieldless enum ordinals inside schema v5. These
// compile-time size sentinels catch vocabulary extension; any intentional
// insertion or reordering also requires an explicit schema review and bump.
const _: () = {
    assert!(CardId::Strike as i64 == 0);
    assert!(CardId::Wish as u64 + 1 == CARD_ID_VOCABULARY_SIZE);
    assert!(RelicId::Abacus as i64 == 0);
    assert!(RelicId::WristBlade as u64 + 1 == RELIC_ID_VOCABULARY_SIZE);
    assert!(PotionId::FirePotion as i64 == 0);
    assert!(PotionId::EssenceOfDarkness as u64 + 1 == POTION_ID_VOCABULARY_SIZE);
    assert!(EncounterId::BlueSlaver as i64 == 0);
    assert!(EncounterId::TheHeart as u64 + 1 == ENCOUNTER_ID_VOCABULARY_SIZE);
    assert!(EventId::BigFish as i64 == 0);
    assert!(EventId::Neow as u64 + 1 == EVENT_ID_VOCABULARY_SIZE);
    assert!(EnemyId::JawWorm as i64 == 0);
    assert!(EnemyId::CorruptHeart as u64 + 1 == ENEMY_ID_VOCABULARY_SIZE);
    assert!(PowerId::Strength as i64 == 0);
    assert!(PowerId::Study as u64 + 1 == POWER_ID_VOCABULARY_SIZE);
    assert!(EventActionKind::Unknown as i64 == 0);
    assert!(EventActionKind::Special as i64 == 9);
    assert!(RoomType::EventRoom as i64 == 0);
    assert!(RoomType::TrueVictoryRoom as i64 == 7);
    assert!(SelectionScope::Hand as i64 == 0);
    assert!(SelectionScope::Grid as i64 == 2);
    assert!(PlannerRunGoal::ActThreeVictory as i64 == 0);
    assert!(PlannerRunGoal::HeartVictory as i64 == 1);
    assert!(PlannerDecisionSite::Map as i64 == 0);
    assert!(PlannerDecisionSite::Treasure as i64 == 9);
    assert!(PlannerPlayerClass::Ironclad as i64 == 0);
    assert!(PlannerPlayerClass::Watcher as i64 == 3);
};

macro_rules! numeric_schema_enum {
    ($(#[$meta:meta])* $visibility:vis enum $name:ident: $repr:ty {
        $($variant:ident = $value:expr),+ $(,)?
    }) => {
        $(#[$meta])*
        #[repr($repr)]
        $visibility enum $name {
            $($variant = $value),+
        }

        impl $name {
            pub const SCHEMA: &'static [(&'static str, i64)] = &[
                $((stringify!($variant), Self::$variant as i64)),+
            ];
        }
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SemanticCompleteness {
    NotEncoded = 0,
    Complete = 1,
}

impl SemanticCompleteness {
    pub const SCHEMA: &'static [(&'static str, i64)] = &[
        ("NotEncoded", Self::NotEncoded as i64),
        ("Complete", Self::Complete as i64),
    ];
}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum TokenKind: u16 {
    Observation = 1,
    Run = 2,
    Context = 3,
    History = 4,
    Card = 5,
    Relic = 6,
    PotionSlot = 7,
    Map = 8,
    MapNode = 9,
    Candidate = 10,
    OfferedCard = 11,
    CombatEncounter = 12,
    CombatTurn = 13,
    CombatTurnCounters = 14,
    CombatPlayer = 15,
    CombatOrb = 16,
    CombatPower = 17,
    CombatMonster = 18,
    CombatIntent = 19,
    CombatMoveHistory = 20,
    CombatPublicCounter = 21,
    CombatCardZone = 22,
    CombatCard = 23,
    CombatHiddenReason = 24,
    CombatCounterItem = 25,
    CombatMove = 26,
    CombatDamageProjection = 27,
    SelectionDomain = 28,
    SelectionState = 29,
    SelectionChosen = 30,
    CombatActionPayload = 31,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum CategoricalField: u16 {
    RunGoal = 1,
    DecisionSite = 2,
    PlayerClass = 3,
    RubyKey = 4,
    EmeraldKey = 5,
    SapphireKey = 6,
    CardId = 7,
    RelicId = 8,
    RelicUsedUp = 9,
    PotionOccupied = 10,
    PotionId = 11,
    PotionCanUse = 12,
    PotionCanDiscard = 13,
    PotionRequiresTarget = 14,
    BossEncounterId = 15,
    MapRoom = 16,
    MapNodeHasEmeraldKey = 17,
    ContextKind = 18,
    ContextOverlay = 19,
    ContextEventId = 20,
    ContextPurgeAvailable = 21,
    ActionKind = 22,
    ActionFlight = 23,
    ActionEventId = 24,
    ActionEventKind = 25,
    ActionCardId = 26,
    ActionRelicId = 27,
    ActionPotionId = 28,
    ActionSelectionScope = 29,
    ActionSite = 30,
    RewardKind = 31,
    CombatPhase = 32,
    CombatIsBoss = 33,
    CombatIsElite = 34,
    HiddenReason = 35,
    PlayerFacingLeft = 36,
    StanceId = 37,
    OrbId = 38,
    PowerId = 39,
    PowerJustApplied = 40,
    EnemyIdentityKind = 41,
    EnemyId = 42,
    MonsterAlive = 43,
    MonsterEscaped = 44,
    MonsterDying = 45,
    MonsterHalfDead = 46,
    EvidenceKind = 47,
    IntentKind = 48,
    PublicCounterKind = 49,
    CardZoneKind = 50,
    CardExhaustOverride = 51,
    CardRetainOverride = 52,
    CardFreeToPlay = 53,
    CombatActionKind = 54,
    IndexedChoiceInputEncoding = 55,
    IndexedChoiceReasonKind = 56,
    IndexedChoiceColorless = 57,
    IndexedChoiceCardType = 58,
    IndexedChoiceDestination = 59,
    IndexedChoiceUpgraded = 60,
    IndexedChoiceCandidateKind = 61,
    SelectionInputEncoding = 62,
    SelectionReasonKind = 63,
    SelectionSourcePile = 64,
    SelectionPayloadDistinctBy = 65,
    SelectionCandidateKind = 66,
    SelectionDomainKind = 67,
    SelectionDomainEligible = 68,
    CounterItemKind = 69,
    SelectionReasonFlag = 70,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum ScalarField: u16 {
    AscensionLevel = 1,
    Act = 2,
    Floor = 3,
    CurrentHp = 4,
    MaxHp = 5,
    Gold = 6,
    PotionCapacity = 7,
    CardUpgrades = 8,
    CardMiscValue = 9,
    CardBaseDamageOverride = 10,
    CardBaseBlockOverride = 11,
    CardCostModifier = 12,
    RelicCounter = 13,
    RelicAmount = 14,
    PotionSlot = 15,
    MapCurrentX = 16,
    MapCurrentY = 17,
    MapNodeX = 18,
    MapNodeY = 19,
    ContextRewardItemIndex = 20,
    ContextEventScreen = 21,
    ContextPurgeCost = 22,
    ShopPurgeCount = 23,
    ActionX = 24,
    ActionY = 25,
    ActionScreen = 26,
    ActionOptionIndex = 27,
    ActionRewardItemIndex = 28,
    ActionUpgrades = 29,
    ActionShopSlot = 30,
    ActionPrice = 31,
    RewardAmount = 32,
    CombatTurnCount = 33,
    CombatEnergy = 34,
    TurnStartDrawModifier = 35,
    CardsPlayedThisTurn = 36,
    AttacksPlayedThisTurn = 37,
    CardsDiscardedThisTurn = 38,
    MantraGainedThisCombat = 39,
    TimesDamagedThisCombat = 40,
    DiscoveryCostForTurn = 41,
    CombatPlayerHp = 42,
    CombatPlayerMaxHp = 43,
    CombatPlayerBlock = 44,
    CombatPlayerGold = 45,
    GoldDeltaThisCombat = 46,
    EnergyMaster = 47,
    MaxOrbs = 48,
    CollectionPosition = 49,
    OrbBasePassive = 50,
    OrbBaseEvoke = 51,
    OrbPassive = 52,
    OrbEvoke = 53,
    PowerAmount = 54,
    PowerExtraData = 55,
    MonsterSlot = 56,
    MonsterHp = 57,
    MonsterMaxHp = 58,
    MonsterBlock = 59,
    UnmappedMonsterType = 60,
    IntentPreviewDamagePerHit = 61,
    IntentDamage = 62,
    IntentHits = 63,
    MoveId = 64,
    PublicCounterValue = 65,
    CardCostForTurn = 66,
    CardEffectiveCost = 67,
    CardCurrentDamage = 68,
    CardCurrentBlock = 69,
    CardCurrentMagicNumber = 70,
    CardEnergyOnUse = 71,
    DamageProjectionValue = 72,
    ActionIndex = 73,
    ActionSecondaryIndex = 74,
    PayloadPosition = 75,
    IndexedChoiceAmount = 76,
    SelectionRawDomainCount = 77,
    SelectionEligibleDomainCount = 78,
    SelectionMaxDistinctCount = 79,
    SelectionDeclaredMin = 80,
    SelectionDeclaredMax = 81,
    SelectionEffectiveMax = 82,
    SelectionDomainAddress = 83,
    SelectionChosenPosition = 84,
    SelectionReasonAmount = 85,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum RelationKind: u16 {
    ObservationHasRun = 1,
    ObservationHasContext = 2,
    ObservationHasHistory = 3,
    ObservationHasCard = 4,
    ObservationHasRelic = 5,
    ObservationHasPotionSlot = 6,
    ObservationHasMap = 7,
    ObservationHasCandidate = 8,
    MapHasNode = 9,
    MapPathTo = 10,
    CandidateTargets = 11,
    CandidateHasPayload = 12,
    ObservationHasEncounter = 13,
    ObservationHasTurn = 14,
    ObservationHasPlayer = 15,
    ObservationHasHiddenReason = 16,
    ObservationHasCardZone = 17,
    ObservationHasMonster = 18,
    PlayerHasRelic = 19,
    PlayerHasOrb = 20,
    EntityHasPower = 21,
    PowerHasPayloadCard = 22,
    MonsterHasIntent = 23,
    MonsterHasMoveHistory = 24,
    MonsterHasPublicCounter = 25,
    ZoneHasCard = 26,
    TurnHasCounters = 27,
    CountersHasItem = 28,
    HistoryHasMove = 29,
    CardHasDamageProjection = 30,
    DamageTargetsMonster = 31,
    CandidateHasSelectionDomain = 32,
    ObservationHasSelectionState = 33,
    SelectionHasChosen = 34,
    ChosenTargetsDomain = 35,
    SelectionHasDomain = 36,
    SelectionDomainTargets = 37,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum ContextKind: i64 {
    Map = LearningStrategicContextKindV1::Map as i64,
    CardReward = LearningStrategicContextKindV1::CardReward as i64,
    Event = LearningStrategicContextKindV1::Event as i64,
    Shop = LearningStrategicContextKindV1::Shop as i64,
    Reward = LearningStrategicContextKindV1::Reward as i64,
    Campfire = LearningStrategicContextKindV1::Campfire as i64,
    BossRelic = LearningStrategicContextKindV1::BossRelic as i64,
    RunChoice = LearningStrategicContextKindV1::RunChoice as i64,
    Treasure = LearningStrategicContextKindV1::Treasure as i64,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum ActionKind: i64 {
    ChooseRouteNode = 1,
    ChooseEventOption = 2,
    TakeCard = 3,
    OpenCardReward = 4,
    SingingBowl = 5,
    SkipCardReward = 6,
    ClaimReward = 7,
    Rest = 8,
    Smith = 9,
    Dig = 10,
    Lift = 11,
    Toke = 12,
    Recall = 13,
    BuyCard = 14,
    BuyRelic = 15,
    BuyPotion = 16,
    RemoveCard = 17,
    OpenPendingRewards = 18,
    LeaveShop = 19,
    TakeBossRelic = 20,
    SkipBossRelic = 21,
    SubmitRunSelection = 22,
    OpenChest = 23,
    Proceed = 24,
    Cancel = 25,
    UseRunPotion = 26,
    DiscardRunPotion = 27,
    BeginRunCardSelection = 28,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum RewardKind: i64 {
    Gold = 1,
    StolenGold = 2,
    CardReward = 3,
    Relic = 4,
    Potion = 5,
    EmeraldKey = 6,
    SapphireKey = 7,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum CombatActionKind: i64 {
    PlayCard = 1,
    UsePotion = 2,
    DiscardPotion = 3,
    EndTurn = 4,
    SubmitIndexedChoice = 5,
    Proceed = 6,
    Cancel = 7,
    BeginSelection = 8,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum IntentKind: i64 {
    Attack = 1,
    AttackBuff = 2,
    AttackDebuff = 3,
    AttackDefend = 4,
    Buff = 5,
    Debuff = 6,
    StrongDebuff = 7,
    Debug = 8,
    Defend = 9,
    DefendDebuff = 10,
    DefendBuff = 11,
    Escape = 12,
    Magic = 13,
    None = 14,
    Sleep = 15,
    Stun = 16,
    Unknown = 17,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum EnemyIdentityKind: i64 {
    Known = 1,
    Unmapped = 2,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum PublicCounterKind: i64 {
    HexaghostActiveOrbs = 1,
    StolenGold = 2,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum CardZoneKind: i64 {
    MasterDeck = 1,
    Hand = 2,
    Draw = 3,
    Discard = 4,
    Exhaust = 5,
    Limbo = 6,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum IndexedChoiceReasonKind: i64 {
    Discovery = 1,
    CardReward = 2,
    ForeignInfluence = 3,
    ChooseOne = 4,
    Stance = 5,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum IndexedChoiceCandidateKind: i64 {
    Card = 1,
    Stance = 2,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum SelectionReasonKind: i64 {
    HandExhaust = 1,
    HandDiscard = 2,
    HandRetain = 3,
    HandPutOnDrawPile = 4,
    HandPutToBottomOfDraw = 5,
    HandSetup = 6,
    HandCopy = 7,
    HandNightmare = 8,
    HandUpgrade = 9,
    HandGamblingChip = 10,
    HandRecycle = 11,
    GridMoveToDrawPile = 12,
    GridExhume = 13,
    GridDrawPileToHand = 14,
    GridSkillFromDeckToHand = 15,
    GridAttackFromDeckToHand = 16,
    GridDiscardToHand = 17,
    GridDiscardToHandNoCostChange = 18,
    GridDiscardToHandRetain = 19,
    GridOmniscience = 20,
    ScryDiscard = 21,
    RunUpgrade = 22,
    RunPurge = 23,
    RunTransform = 24,
    RunTransformUpgraded = 25,
    RunDuplicate = 26,
    RunBottleFlame = 27,
    RunBottleLightning = 28,
    RunBottleTornado = 29,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum SelectionCandidateKind: i64 {
    Submit = 1,
    Append = 2,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum SelectionDomainKind: i64 {
    Card = 1,
    Scry = 2,
}}

numeric_schema_enum! {
#[derive(Clone, Copy, Debug)]
pub enum CounterItemKind: i64 {
    CardPlayedThisTurn = 1,
    CardPlayedThisCombat = 2,
    OrbChanneledThisTurn = 3,
    OrbChanneledThisCombat = 4,
}}

pub const CATEGORICAL_VOCABULARY_SIZES: &[(u16, u64)] = &[
    (CategoricalField::RunGoal as u16, 2),
    (CategoricalField::DecisionSite as u16, 10),
    (CategoricalField::PlayerClass as u16, 4),
    (CategoricalField::RubyKey as u16, 2),
    (CategoricalField::EmeraldKey as u16, 2),
    (CategoricalField::SapphireKey as u16, 2),
    (CategoricalField::CardId as u16, CARD_ID_VOCABULARY_SIZE),
    (CategoricalField::RelicId as u16, RELIC_ID_VOCABULARY_SIZE),
    (CategoricalField::RelicUsedUp as u16, 2),
    (CategoricalField::PotionOccupied as u16, 2),
    (CategoricalField::PotionId as u16, POTION_ID_VOCABULARY_SIZE),
    (CategoricalField::PotionCanUse as u16, 2),
    (CategoricalField::PotionCanDiscard as u16, 2),
    (CategoricalField::PotionRequiresTarget as u16, 2),
    (
        CategoricalField::BossEncounterId as u16,
        ENCOUNTER_ID_VOCABULARY_SIZE,
    ),
    (CategoricalField::MapRoom as u16, 8),
    (CategoricalField::MapNodeHasEmeraldKey as u16, 2),
    (CategoricalField::ContextKind as u16, 10),
    (CategoricalField::ContextOverlay as u16, 2),
    (
        CategoricalField::ContextEventId as u16,
        EVENT_ID_VOCABULARY_SIZE,
    ),
    (CategoricalField::ContextPurgeAvailable as u16, 2),
    (CategoricalField::ActionKind as u16, 29),
    (CategoricalField::ActionFlight as u16, 2),
    (
        CategoricalField::ActionEventId as u16,
        EVENT_ID_VOCABULARY_SIZE,
    ),
    (CategoricalField::ActionEventKind as u16, 10),
    (
        CategoricalField::ActionCardId as u16,
        CARD_ID_VOCABULARY_SIZE,
    ),
    (
        CategoricalField::ActionRelicId as u16,
        RELIC_ID_VOCABULARY_SIZE,
    ),
    (
        CategoricalField::ActionPotionId as u16,
        POTION_ID_VOCABULARY_SIZE,
    ),
    (CategoricalField::ActionSelectionScope as u16, 3),
    (CategoricalField::ActionSite as u16, 10),
    (CategoricalField::RewardKind as u16, 8),
    (CategoricalField::CombatPhase as u16, 3),
    (CategoricalField::CombatIsBoss as u16, 2),
    (CategoricalField::CombatIsElite as u16, 2),
    (CategoricalField::HiddenReason as u16, 3),
    (CategoricalField::PlayerFacingLeft as u16, 2),
    (CategoricalField::StanceId as u16, 4),
    (CategoricalField::OrbId as u16, 5),
    (CategoricalField::PowerId as u16, POWER_ID_VOCABULARY_SIZE),
    (CategoricalField::PowerJustApplied as u16, 2),
    (CategoricalField::EnemyIdentityKind as u16, 3),
    (CategoricalField::EnemyId as u16, ENEMY_ID_VOCABULARY_SIZE),
    (CategoricalField::MonsterAlive as u16, 2),
    (CategoricalField::MonsterEscaped as u16, 2),
    (CategoricalField::MonsterDying as u16, 2),
    (CategoricalField::MonsterHalfDead as u16, 2),
    (CategoricalField::EvidenceKind as u16, 4),
    (CategoricalField::IntentKind as u16, 18),
    (CategoricalField::PublicCounterKind as u16, 3),
    (CategoricalField::CardZoneKind as u16, 7),
    (CategoricalField::CardExhaustOverride as u16, 2),
    (CategoricalField::CardRetainOverride as u16, 2),
    (CategoricalField::CardFreeToPlay as u16, 2),
    (CategoricalField::CombatActionKind as u16, 9),
    (CategoricalField::IndexedChoiceInputEncoding as u16, 1),
    (CategoricalField::IndexedChoiceReasonKind as u16, 6),
    (CategoricalField::IndexedChoiceColorless as u16, 2),
    (CategoricalField::IndexedChoiceCardType as u16, 5),
    (CategoricalField::IndexedChoiceDestination as u16, 2),
    (CategoricalField::IndexedChoiceUpgraded as u16, 2),
    (CategoricalField::IndexedChoiceCandidateKind as u16, 3),
    (CategoricalField::SelectionInputEncoding as u16, 4),
    (CategoricalField::SelectionReasonKind as u16, 30),
    (CategoricalField::SelectionSourcePile as u16, 6),
    (CategoricalField::SelectionPayloadDistinctBy as u16, 2),
    (CategoricalField::SelectionCandidateKind as u16, 3),
    (CategoricalField::SelectionDomainKind as u16, 3),
    (CategoricalField::SelectionDomainEligible as u16, 2),
    (CategoricalField::CounterItemKind as u16, 5),
    (CategoricalField::SelectionReasonFlag as u16, 2),
];

#[derive(Clone, Debug, Default)]
pub struct CategoricalTable {
    pub token_indices: Vec<u64>,
    pub fields: Vec<u16>,
    pub values: Vec<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct ScalarTable {
    pub token_indices: Vec<u64>,
    pub fields: Vec<u16>,
    pub values: Vec<f32>,
}

#[derive(Clone, Debug, Default)]
pub struct RelationTable {
    pub source_token_indices: Vec<u64>,
    pub relations: Vec<u16>,
    pub target_token_indices: Vec<u64>,
}

#[derive(Clone, Debug)]
pub struct SemanticBatch {
    pub completeness: Vec<u8>,
    pub token_row_splits: Vec<u64>,
    pub token_kinds: Vec<u16>,
    pub categorical: CategoricalTable,
    pub scalar: ScalarTable,
    pub relation: RelationTable,
    pub candidate_token_indices: Vec<u64>,
}

struct StrategicTokenIndex {
    root: u64,
    cards: BTreeMap<u32, u64>,
    potions: BTreeMap<u32, (u64, usize, PotionId)>,
    map_nodes: BTreeMap<(i32, i32), u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticEncodingError {
    DuplicateCardIdentity(u32),
    MissingCardTarget(u32),
    DuplicatePotionRelationKey(u32),
    MissingPotionRelationTarget(u32),
    PotionTargetMismatch {
        relation_key: u32,
        slot: usize,
        potion: PotionId,
    },
    DuplicateMapNode { x: i32, y: i32 },
    MissingMapTarget { x: i32, y: i32 },
    NonStrategicCandidateInStrategicRow,
    CandidateAlignmentMismatch { expected: usize, actual: usize },
    MissingCombatHandCard(usize),
    MissingCombatPotionSlot(usize),
    MissingCombatMonsterTarget(usize),
    MissingDamageProjectionMonster(usize),
    UnsupportedCombatAtomicInput,
    UnsupportedRunSelectionReason(SelectionReason),
    MissingSelectionDomain(usize),
    SelectionObservationMismatch,
    IndexOverflow,
}

impl fmt::Display for SemanticEncodingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SemanticEncodingError {}

#[derive(Default)]
pub struct SemanticBatchBuilder {
    completeness: Vec<u8>,
    token_row_splits: Vec<u64>,
    token_kinds: Vec<u16>,
    categorical: CategoricalTable,
    scalar: ScalarTable,
    relation: RelationTable,
    candidate_token_indices: Vec<u64>,
}

impl SemanticBatchBuilder {
    pub fn new() -> Self {
        Self {
            token_row_splits: vec![0],
            ..Self::default()
        }
    }

    pub fn push_decision(
        &mut self,
        decision: &LearningModelDecisionV1<'_>,
    ) -> Result<(), SemanticEncodingError> {
        match decision.observation {
            LearningModelObservationV1::Strategic(observation) => {
                self.completeness.push(SemanticCompleteness::Complete as u8);
                let before = self.candidate_token_indices.len();
                self.encode_strategic(observation, decision)?;
                let actual = self.candidate_token_indices.len() - before;
                if actual != decision.candidates.len() {
                    return Err(SemanticEncodingError::CandidateAlignmentMismatch {
                        expected: decision.candidates.len(),
                        actual,
                    });
                }
            }
            LearningModelObservationV1::Combat(observation) => {
                self.completeness.push(SemanticCompleteness::Complete as u8);
                let before = self.candidate_token_indices.len();
                self.encode_combat_root(observation, decision)?;
                let actual = self.candidate_token_indices.len() - before;
                if actual != decision.candidates.len() {
                    return Err(SemanticEncodingError::CandidateAlignmentMismatch {
                        expected: decision.candidates.len(),
                        actual,
                    });
                }
            }
        }
        self.finish_row()
    }

    pub fn push_selection(
        &mut self,
        observation: LearningModelObservationV1<'_>,
        draft: &LearningSelectionDraftV1,
    ) -> Result<(), SemanticEncodingError> {
        self.completeness.push(SemanticCompleteness::Complete as u8);
        let decision = draft.decision();
        let before = self.candidate_token_indices.len();
        match (observation, draft.combat_family(), draft.run_family()) {
            (LearningModelObservationV1::Combat(observation), Some(_), None) => {
                self.encode_combat_selection(observation, draft, &decision)?;
            }
            (LearningModelObservationV1::Strategic(observation), None, Some(family)) => {
                self.encode_strategic_selection(observation, draft, family, &decision)?;
            }
            _ => return Err(SemanticEncodingError::SelectionObservationMismatch),
        }
        let actual = self.candidate_token_indices.len() - before;
        if actual != decision.candidates.len() {
            return Err(SemanticEncodingError::CandidateAlignmentMismatch {
                expected: decision.candidates.len(),
                actual,
            });
        }
        self.finish_row()
    }

    pub fn finish(self) -> SemanticBatch {
        SemanticBatch {
            completeness: self.completeness,
            token_row_splits: self.token_row_splits,
            token_kinds: self.token_kinds,
            categorical: self.categorical,
            scalar: self.scalar,
            relation: self.relation,
            candidate_token_indices: self.candidate_token_indices,
        }
    }

    fn finish_row(&mut self) -> Result<(), SemanticEncodingError> {
        self.token_row_splits.push(self.token_count()?);
        Ok(())
    }

    fn encode_strategic(
        &mut self,
        observation: LearningStrategicModelObservationV1<'_>,
        decision: &LearningModelDecisionV1<'_>,
    ) -> Result<(), SemanticEncodingError> {
        let tokens = self.encode_strategic_observation(observation)?;
        for candidate in &decision.candidates {
            let LearningModelCandidateSemanticsV1::Strategic { action } = candidate.semantics
            else {
                return Err(SemanticEncodingError::NonStrategicCandidateInStrategicRow);
            };
            let token = self.add_token(TokenKind::Candidate)?;
            self.edge(
                tokens.root,
                RelationKind::ObservationHasCandidate,
                token,
            );
            self.encode_action(
                token,
                action,
                &tokens.cards,
                &tokens.potions,
                &tokens.map_nodes,
            )?;
            self.candidate_token_indices.push(token);
        }
        Ok(())
    }

    fn encode_strategic_observation(
        &mut self,
        observation: LearningStrategicModelObservationV1<'_>,
    ) -> Result<StrategicTokenIndex, SemanticEncodingError> {
        let root = self.add_token(TokenKind::Observation)?;
        self.category(root, CategoricalField::RunGoal, observation.run_goal as i64);
        self.category(
            root,
            CategoricalField::DecisionSite,
            observation.decision_site as i64,
        );

        let run = self.add_token(TokenKind::Run)?;
        self.edge(root, RelationKind::ObservationHasRun, run);
        self.category(
            run,
            CategoricalField::PlayerClass,
            observation.run.player_class as i64,
        );
        self.category(
            run,
            CategoricalField::RubyKey,
            bool_value(observation.run.keys[0]),
        );
        self.category(
            run,
            CategoricalField::EmeraldKey,
            bool_value(observation.run.keys[1]),
        );
        self.category(
            run,
            CategoricalField::SapphireKey,
            bool_value(observation.run.keys[2]),
        );
        self.scalar(
            run,
            ScalarField::AscensionLevel,
            observation.run.ascension_level,
        );
        self.scalar(run, ScalarField::Act, observation.run.act);
        self.scalar(run, ScalarField::Floor, observation.run.floor);
        self.scalar(run, ScalarField::CurrentHp, observation.run.current_hp);
        self.scalar(run, ScalarField::MaxHp, observation.run.max_hp);
        self.scalar(run, ScalarField::Gold, observation.run.gold);
        self.scalar(
            run,
            ScalarField::PotionCapacity,
            observation.run.potion_capacity,
        );

        let mut cards = observation.cards.iter().collect::<Vec<_>>();
        cards.sort_by_key(|card| {
            (
                card.card as i64,
                card.upgrades,
                card.misc_value,
                card.base_damage_override,
                card.base_block_override,
                card.cost_modifier,
            )
        });
        let mut card_tokens = BTreeMap::new();
        for card in cards {
            let token = self.add_token(TokenKind::Card)?;
            if card_tokens.insert(card.card_uuid, token).is_some() {
                return Err(SemanticEncodingError::DuplicateCardIdentity(card.card_uuid));
            }
            self.edge(root, RelationKind::ObservationHasCard, token);
            self.category(token, CategoricalField::CardId, card.card as i64);
            self.scalar(token, ScalarField::CardUpgrades, card.upgrades);
            self.scalar(token, ScalarField::CardMiscValue, card.misc_value);
            if let Some(value) = card.base_damage_override {
                self.scalar(token, ScalarField::CardBaseDamageOverride, value);
            }
            if let Some(value) = card.base_block_override {
                self.scalar(token, ScalarField::CardBaseBlockOverride, value);
            }
            self.scalar(token, ScalarField::CardCostModifier, card.cost_modifier);
        }

        let mut relics = observation.relics.iter().collect::<Vec<_>>();
        relics.sort_by_key(|relic| {
            (
                relic.relic as i64,
                relic.counter,
                relic.used_up,
                relic.amount,
            )
        });
        for relic in relics {
            let token = self.add_token(TokenKind::Relic)?;
            self.edge(root, RelationKind::ObservationHasRelic, token);
            self.category(token, CategoricalField::RelicId, relic.relic as i64);
            self.category(
                token,
                CategoricalField::RelicUsedUp,
                bool_value(relic.used_up),
            );
            self.scalar(token, ScalarField::RelicCounter, relic.counter);
            self.scalar(token, ScalarField::RelicAmount, relic.amount);
        }

        let mut potions = observation.potion_slots().collect::<Vec<_>>();
        potions.sort_by_key(|potion_slot| potion_slot.slot());
        let mut potion_tokens = BTreeMap::new();
        for potion_slot in potions {
            let token = self.add_token(TokenKind::PotionSlot)?;
            self.edge(root, RelationKind::ObservationHasPotionSlot, token);
            self.scalar(token, ScalarField::PotionSlot, potion_slot.slot());
            self.category(
                token,
                CategoricalField::PotionOccupied,
                bool_value(potion_slot.potion().is_some()),
            );
            if let Some(potion) = potion_slot.potion() {
                let relation_key = potion.relation_key();
                if potion_tokens
                    .insert(relation_key, (token, potion_slot.slot(), potion.potion()))
                    .is_some()
                {
                    return Err(SemanticEncodingError::DuplicatePotionRelationKey(
                        relation_key,
                    ));
                }
                self.category(token, CategoricalField::PotionId, potion.potion() as i64);
                self.category(
                    token,
                    CategoricalField::PotionCanUse,
                    bool_value(potion.can_use()),
                );
                self.category(
                    token,
                    CategoricalField::PotionCanDiscard,
                    bool_value(potion.can_discard()),
                );
                self.category(
                    token,
                    CategoricalField::PotionRequiresTarget,
                    bool_value(potion.requires_target()),
                );
            }
        }

        let map = self.add_token(TokenKind::Map)?;
        self.edge(root, RelationKind::ObservationHasMap, map);
        self.scalar(
            map,
            ScalarField::MapCurrentX,
            observation.public_map.current_x,
        );
        self.scalar(
            map,
            ScalarField::MapCurrentY,
            observation.public_map.current_y,
        );
        if let Some(boss) = observation.public_map.boss {
            self.category(map, CategoricalField::BossEncounterId, boss as i64);
        }
        let mut nodes = observation.public_map.nodes.iter().collect::<Vec<_>>();
        nodes.sort_by_key(|node| (node.y, node.x));
        let mut map_tokens = BTreeMap::new();
        for node in &nodes {
            let token = self.add_token(TokenKind::MapNode)?;
            if map_tokens.insert((node.x, node.y), token).is_some() {
                return Err(SemanticEncodingError::DuplicateMapNode {
                    x: node.x,
                    y: node.y,
                });
            }
            self.edge(map, RelationKind::MapHasNode, token);
            self.scalar(token, ScalarField::MapNodeX, node.x);
            self.scalar(token, ScalarField::MapNodeY, node.y);
            if let Some(room) = node.room {
                self.category(token, CategoricalField::MapRoom, room as i64);
            }
            self.category(
                token,
                CategoricalField::MapNodeHasEmeraldKey,
                bool_value(node.has_emerald_key),
            );
        }
        for node in nodes {
            let source = map_tokens[&(node.x, node.y)];
            for edge in &node.edges {
                let target = map_tokens
                    .get(&(edge.destination_x, edge.destination_y))
                    .copied()
                    .ok_or(SemanticEncodingError::MissingMapTarget {
                        x: edge.destination_x,
                        y: edge.destination_y,
                    })?;
                self.edge(source, RelationKind::MapPathTo, target);
            }
        }

        let context = self.add_token(TokenKind::Context)?;
        self.edge(root, RelationKind::ObservationHasContext, context);
        self.encode_context(context, observation.context);

        let history = self.add_token(TokenKind::History)?;
        self.edge(root, RelationKind::ObservationHasHistory, history);
        self.scalar(
            history,
            ScalarField::ShopPurgeCount,
            observation.public_history.shop_purge_count,
        );

        Ok(StrategicTokenIndex {
            root,
            cards: card_tokens,
            potions: potion_tokens,
            map_nodes: map_tokens,
        })
    }

    fn encode_strategic_selection(
        &mut self,
        observation: LearningStrategicModelObservationV1<'_>,
        draft: &LearningSelectionDraftV1,
        family: LearningRunSelectionFamilyV1<'_>,
        decision: &LearningSelectionDecisionV1,
    ) -> Result<(), SemanticEncodingError> {
        let tokens = self.encode_strategic_observation(observation)?;
        let selection = self.add_token(TokenKind::SelectionState)?;
        self.edge(
            tokens.root,
            RelationKind::ObservationHasSelectionState,
            selection,
        );
        let card_uuids = (0..family.domain_count())
            .map(|index| {
                family
                    .domain_card_uuid(index)
                    .ok_or(SemanticEncodingError::MissingSelectionDomain(index))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let domains = self.encode_run_selection_family(
            selection,
            RelationKind::SelectionHasDomain,
            family.scope(),
            family.reason(),
            family.declared_min(),
            family.declared_max(),
            &card_uuids,
            &tokens.cards,
        )?;
        for (position, domain_index) in draft.selected_domain_indices().iter().copied().enumerate()
        {
            let target = domains
                .get(domain_index)
                .copied()
                .ok_or(SemanticEncodingError::MissingSelectionDomain(domain_index))?;
            let chosen = self.add_token(TokenKind::SelectionChosen)?;
            self.scalar(chosen, ScalarField::SelectionChosenPosition, position);
            self.edge(selection, RelationKind::SelectionHasChosen, chosen);
            self.edge(chosen, RelationKind::ChosenTargetsDomain, target);
        }
        for candidate in &decision.candidates {
            let token = self.add_token(TokenKind::Candidate)?;
            self.edge(
                tokens.root,
                RelationKind::ObservationHasCandidate,
                token,
            );
            match candidate.semantics {
                LearningSelectionCandidateSemanticsV1::Submit => self.category(
                    token,
                    CategoricalField::SelectionCandidateKind,
                    SelectionCandidateKind::Submit as i64,
                ),
                LearningSelectionCandidateSemanticsV1::Append { domain_index } => {
                    self.category(
                        token,
                        CategoricalField::SelectionCandidateKind,
                        SelectionCandidateKind::Append as i64,
                    );
                    let target = domains
                        .get(domain_index)
                        .copied()
                        .ok_or(SemanticEncodingError::MissingSelectionDomain(domain_index))?;
                    self.edge(token, RelationKind::CandidateTargets, target);
                }
            }
            self.candidate_token_indices.push(token);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn encode_run_selection_family(
        &mut self,
        owner: u64,
        domain_relation: RelationKind,
        scope: SelectionScope,
        reason: SelectionReason,
        declared_min: usize,
        declared_max: usize,
        card_uuids: &[u32],
        card_tokens: &BTreeMap<u32, u64>,
    ) -> Result<Vec<u64>, SemanticEncodingError> {
        self.category(
            owner,
            CategoricalField::SelectionInputEncoding,
            RUN_SELECTION_DECK_CARD_UUID_INPUT_ENCODING,
        );
        self.category(
            owner,
            CategoricalField::ActionSelectionScope,
            scope as i64,
        );
        self.encode_run_selection_reason(owner, reason)?;
        self.category(
            owner,
            CategoricalField::SelectionPayloadDistinctBy,
            CombatSelectionDistinctByV2::CardUuid as i64,
        );
        self.scalar(
            owner,
            ScalarField::SelectionRawDomainCount,
            card_uuids.len(),
        );
        self.scalar(
            owner,
            ScalarField::SelectionEligibleDomainCount,
            card_uuids.len(),
        );
        self.scalar(
            owner,
            ScalarField::SelectionMaxDistinctCount,
            card_uuids.len(),
        );
        self.scalar(owner, ScalarField::SelectionDeclaredMin, declared_min);
        self.scalar(owner, ScalarField::SelectionDeclaredMax, declared_max);
        self.scalar(
            owner,
            ScalarField::SelectionEffectiveMax,
            declared_max.min(card_uuids.len()),
        );

        let mut domains = Vec::with_capacity(card_uuids.len());
        for (domain_index, card_uuid) in card_uuids.iter().copied().enumerate() {
            let card = card_tokens
                .get(&card_uuid)
                .copied()
                .ok_or(SemanticEncodingError::MissingCardTarget(card_uuid))?;
            let domain = self.add_token(TokenKind::SelectionDomain)?;
            self.edge(owner, domain_relation, domain);
            self.edge(domain, RelationKind::SelectionDomainTargets, card);
            self.category(
                domain,
                CategoricalField::SelectionDomainKind,
                SelectionDomainKind::Card as i64,
            );
            self.category(
                domain,
                CategoricalField::SelectionDomainEligible,
                bool_value(true),
            );
            self.scalar(
                domain,
                ScalarField::SelectionDomainAddress,
                domain_index,
            );
            domains.push(domain);
        }
        Ok(domains)
    }

    fn encode_run_selection_reason(
        &mut self,
        token: u64,
        reason: SelectionReason,
    ) -> Result<(), SemanticEncodingError> {
        let kind = match reason {
            SelectionReason::Upgrade => SelectionReasonKind::RunUpgrade,
            SelectionReason::Purge => SelectionReasonKind::RunPurge,
            SelectionReason::Transform => SelectionReasonKind::RunTransform,
            SelectionReason::TransformUpgraded => SelectionReasonKind::RunTransformUpgraded,
            SelectionReason::Duplicate => SelectionReasonKind::RunDuplicate,
            SelectionReason::BottleFlame => SelectionReasonKind::RunBottleFlame,
            SelectionReason::BottleLightning => SelectionReasonKind::RunBottleLightning,
            SelectionReason::BottleTornado => SelectionReasonKind::RunBottleTornado,
            _ => return Err(SemanticEncodingError::UnsupportedRunSelectionReason(reason)),
        };
        self.category(
            token,
            CategoricalField::SelectionReasonKind,
            kind as i64,
        );
        Ok(())
    }

    fn encode_context(&mut self, token: u64, context: &PlannerDecisionContext) {
        match context {
            PlannerDecisionContext::Map { overlay } => {
                self.context_kind(token, ContextKind::Map);
                self.category(
                    token,
                    CategoricalField::ContextOverlay,
                    bool_value(*overlay),
                );
            }
            PlannerDecisionContext::CardReward { reward_item_index } => {
                self.context_kind(token, ContextKind::CardReward);
                if let Some(index) = reward_item_index {
                    self.scalar(token, ScalarField::ContextRewardItemIndex, *index);
                }
            }
            PlannerDecisionContext::Event { event, screen } => {
                self.context_kind(token, ContextKind::Event);
                self.category(token, CategoricalField::ContextEventId, *event as i64);
                self.scalar(token, ScalarField::ContextEventScreen, *screen);
            }
            PlannerDecisionContext::Shop {
                purge_cost,
                purge_available,
            } => {
                self.context_kind(token, ContextKind::Shop);
                self.scalar(token, ScalarField::ContextPurgeCost, *purge_cost);
                self.category(
                    token,
                    CategoricalField::ContextPurgeAvailable,
                    bool_value(*purge_available),
                );
            }
            PlannerDecisionContext::Reward => self.context_kind(token, ContextKind::Reward),
            PlannerDecisionContext::Campfire => self.context_kind(token, ContextKind::Campfire),
            PlannerDecisionContext::BossRelic => self.context_kind(token, ContextKind::BossRelic),
            PlannerDecisionContext::RunChoice => self.context_kind(token, ContextKind::RunChoice),
            PlannerDecisionContext::Treasure => self.context_kind(token, ContextKind::Treasure),
        }
    }

    fn encode_action(
        &mut self,
        token: u64,
        action: &PlannerAction,
        card_tokens: &BTreeMap<u32, u64>,
        potion_tokens: &BTreeMap<u32, (u64, usize, PotionId)>,
        map_tokens: &BTreeMap<(i32, i32), u64>,
    ) -> Result<(), SemanticEncodingError> {
        match action {
            PlannerAction::ChooseRouteNode { x, y, flight } => {
                self.action_kind(token, ActionKind::ChooseRouteNode);
                self.scalar(token, ScalarField::ActionX, *x);
                self.scalar(token, ScalarField::ActionY, *y);
                self.category(token, CategoricalField::ActionFlight, bool_value(*flight));
                let target = map_tokens
                    .get(&(*x, *y))
                    .copied()
                    .ok_or(SemanticEncodingError::MissingMapTarget { x: *x, y: *y })?;
                self.edge(token, RelationKind::CandidateTargets, target);
            }
            PlannerAction::ChooseEventOption {
                event,
                screen,
                option_index,
                action,
            } => {
                self.action_kind(token, ActionKind::ChooseEventOption);
                self.category(token, CategoricalField::ActionEventId, *event as i64);
                self.category(token, CategoricalField::ActionEventKind, *action as i64);
                self.scalar(token, ScalarField::ActionScreen, *screen);
                self.scalar(token, ScalarField::ActionOptionIndex, *option_index);
            }
            PlannerAction::TakeCard {
                reward_item_index,
                option_index,
                card,
                upgrades,
            } => {
                self.action_kind(token, ActionKind::TakeCard);
                self.optional_reward_index(token, *reward_item_index);
                self.scalar(token, ScalarField::ActionOptionIndex, *option_index);
                self.action_card(token, *card, *upgrades);
            }
            PlannerAction::OpenCardReward { reward_item_index } => {
                self.action_kind(token, ActionKind::OpenCardReward);
                self.scalar(
                    token,
                    ScalarField::ActionRewardItemIndex,
                    *reward_item_index,
                );
            }
            PlannerAction::SingingBowl { reward_item_index } => {
                self.action_kind(token, ActionKind::SingingBowl);
                self.optional_reward_index(token, *reward_item_index);
            }
            PlannerAction::SkipCardReward { reward_item_index } => {
                self.action_kind(token, ActionKind::SkipCardReward);
                self.scalar(
                    token,
                    ScalarField::ActionRewardItemIndex,
                    *reward_item_index,
                );
            }
            PlannerAction::ClaimReward {
                reward_item_index,
                reward,
            } => {
                self.action_kind(token, ActionKind::ClaimReward);
                self.scalar(
                    token,
                    ScalarField::ActionRewardItemIndex,
                    *reward_item_index,
                );
                self.encode_reward(token, reward)?;
            }
            PlannerAction::Rest => self.action_kind(token, ActionKind::Rest),
            PlannerAction::Smith {
                card_uuid,
                card,
                upgrades,
            } => {
                self.action_kind(token, ActionKind::Smith);
                self.action_card(token, *card, *upgrades);
                self.link_card_target(token, *card_uuid, card_tokens)?;
            }
            PlannerAction::Dig => self.action_kind(token, ActionKind::Dig),
            PlannerAction::Lift => self.action_kind(token, ActionKind::Lift),
            PlannerAction::Toke {
                card_uuid,
                card,
                upgrades,
            } => {
                self.action_kind(token, ActionKind::Toke);
                self.action_card(token, *card, *upgrades);
                self.link_card_target(token, *card_uuid, card_tokens)?;
            }
            PlannerAction::Recall => self.action_kind(token, ActionKind::Recall),
            PlannerAction::BuyCard {
                shop_slot,
                card,
                upgrades,
                price,
            } => {
                self.action_kind(token, ActionKind::BuyCard);
                self.scalar(token, ScalarField::ActionShopSlot, *shop_slot);
                self.scalar(token, ScalarField::ActionPrice, *price);
                self.action_card(token, *card, *upgrades);
            }
            PlannerAction::BuyRelic {
                shop_slot,
                relic,
                price,
            } => {
                self.action_kind(token, ActionKind::BuyRelic);
                self.scalar(token, ScalarField::ActionShopSlot, *shop_slot);
                self.scalar(token, ScalarField::ActionPrice, *price);
                self.category(token, CategoricalField::ActionRelicId, *relic as i64);
            }
            PlannerAction::BuyPotion {
                shop_slot,
                potion,
                price,
            } => {
                self.action_kind(token, ActionKind::BuyPotion);
                self.scalar(token, ScalarField::ActionShopSlot, *shop_slot);
                self.scalar(token, ScalarField::ActionPrice, *price);
                self.category(token, CategoricalField::ActionPotionId, *potion as i64);
            }
            PlannerAction::UseRunPotion {
                slot,
                potion,
                potion_uuid: relation_key,
            } => {
                self.action_kind(token, ActionKind::UseRunPotion);
                self.category(token, CategoricalField::ActionPotionId, *potion as i64);
                self.link_potion_target(
                    token,
                    *slot,
                    *potion,
                    *relation_key,
                    potion_tokens,
                )?;
            }
            PlannerAction::DiscardRunPotion {
                slot,
                potion,
                potion_uuid: relation_key,
            } => {
                self.action_kind(token, ActionKind::DiscardRunPotion);
                self.category(token, CategoricalField::ActionPotionId, *potion as i64);
                self.link_potion_target(
                    token,
                    *slot,
                    *potion,
                    *relation_key,
                    potion_tokens,
                )?;
            }
            PlannerAction::RemoveCard {
                card_uuid,
                card,
                upgrades,
                price,
            } => {
                self.action_kind(token, ActionKind::RemoveCard);
                self.scalar(token, ScalarField::ActionPrice, *price);
                self.action_card(token, *card, *upgrades);
                self.link_card_target(token, *card_uuid, card_tokens)?;
            }
            PlannerAction::OpenPendingRewards => {
                self.action_kind(token, ActionKind::OpenPendingRewards)
            }
            PlannerAction::LeaveShop => self.action_kind(token, ActionKind::LeaveShop),
            PlannerAction::TakeBossRelic {
                option_index,
                relic,
            } => {
                self.action_kind(token, ActionKind::TakeBossRelic);
                self.scalar(token, ScalarField::ActionOptionIndex, *option_index);
                self.category(token, CategoricalField::ActionRelicId, *relic as i64);
            }
            PlannerAction::SkipBossRelic => self.action_kind(token, ActionKind::SkipBossRelic),
            PlannerAction::BeginRunCardSelection {
                scope,
                reason,
                min_choices,
                max_choices,
                selectable_card_uuids,
            } => {
                self.action_kind(token, ActionKind::BeginRunCardSelection);
                self.encode_run_selection_family(
                    token,
                    RelationKind::CandidateHasSelectionDomain,
                    *scope,
                    *reason,
                    *min_choices,
                    *max_choices,
                    selectable_card_uuids,
                    card_tokens,
                )?;
            }
            PlannerAction::SubmitRunSelection {
                scope,
                selected_card_uuids,
            } => {
                self.action_kind(token, ActionKind::SubmitRunSelection);
                self.category(token, CategoricalField::ActionSelectionScope, *scope as i64);
                for uuid in selected_card_uuids {
                    self.link_card_target(token, *uuid, card_tokens)?;
                }
            }
            PlannerAction::OpenChest => self.action_kind(token, ActionKind::OpenChest),
            PlannerAction::Proceed { site } => {
                self.action_kind(token, ActionKind::Proceed);
                self.category(token, CategoricalField::ActionSite, *site as i64);
            }
            PlannerAction::Cancel { site } => {
                self.action_kind(token, ActionKind::Cancel);
                self.category(token, CategoricalField::ActionSite, *site as i64);
            }
        }
        Ok(())
    }

    fn encode_reward(
        &mut self,
        candidate: u64,
        reward: &PlannerRewardDescriptor,
    ) -> Result<(), SemanticEncodingError> {
        match reward {
            PlannerRewardDescriptor::Gold { amount } => {
                self.reward_kind(candidate, RewardKind::Gold);
                self.scalar(candidate, ScalarField::RewardAmount, *amount);
            }
            PlannerRewardDescriptor::StolenGold { amount } => {
                self.reward_kind(candidate, RewardKind::StolenGold);
                self.scalar(candidate, ScalarField::RewardAmount, *amount);
            }
            PlannerRewardDescriptor::CardReward { cards } => {
                self.reward_kind(candidate, RewardKind::CardReward);
                for card in cards {
                    let token = self.add_token(TokenKind::OfferedCard)?;
                    self.edge(candidate, RelationKind::CandidateHasPayload, token);
                    self.category(token, CategoricalField::CardId, card.card as i64);
                    self.scalar(token, ScalarField::CardUpgrades, card.upgrades);
                }
            }
            PlannerRewardDescriptor::Relic { relic } => {
                self.reward_kind(candidate, RewardKind::Relic);
                self.category(candidate, CategoricalField::ActionRelicId, *relic as i64);
            }
            PlannerRewardDescriptor::Potion { potion } => {
                self.reward_kind(candidate, RewardKind::Potion);
                self.category(candidate, CategoricalField::ActionPotionId, *potion as i64);
            }
            PlannerRewardDescriptor::EmeraldKey => {
                self.reward_kind(candidate, RewardKind::EmeraldKey)
            }
            PlannerRewardDescriptor::SapphireKey => {
                self.reward_kind(candidate, RewardKind::SapphireKey)
            }
        }
        Ok(())
    }

    fn link_card_target(
        &mut self,
        candidate: u64,
        uuid: u32,
        card_tokens: &BTreeMap<u32, u64>,
    ) -> Result<(), SemanticEncodingError> {
        let target = card_tokens
            .get(&uuid)
            .copied()
            .ok_or(SemanticEncodingError::MissingCardTarget(uuid))?;
        self.edge(candidate, RelationKind::CandidateTargets, target);
        Ok(())
    }

    fn link_potion_target(
        &mut self,
        candidate: u64,
        slot: usize,
        potion: PotionId,
        relation_key: u32,
        potion_tokens: &BTreeMap<u32, (u64, usize, PotionId)>,
    ) -> Result<(), SemanticEncodingError> {
        let (target, observed_slot, observed_potion) = potion_tokens
            .get(&relation_key)
            .copied()
            .ok_or(SemanticEncodingError::MissingPotionRelationTarget(
                relation_key,
            ))?;
        if observed_slot != slot || observed_potion != potion {
            return Err(SemanticEncodingError::PotionTargetMismatch {
                relation_key,
                slot,
                potion,
            });
        }
        self.edge(candidate, RelationKind::CandidateTargets, target);
        Ok(())
    }

    fn optional_reward_index(&mut self, token: u64, index: Option<usize>) {
        if let Some(index) = index {
            self.scalar(token, ScalarField::ActionRewardItemIndex, index);
        }
    }

    fn action_card(&mut self, token: u64, card: CardId, upgrades: u8) {
        self.category(token, CategoricalField::ActionCardId, card as i64);
        self.scalar(token, ScalarField::ActionUpgrades, upgrades);
    }

    fn context_kind(&mut self, token: u64, kind: ContextKind) {
        self.category(token, CategoricalField::ContextKind, kind as i64);
    }

    fn action_kind(&mut self, token: u64, kind: ActionKind) {
        self.category(token, CategoricalField::ActionKind, kind as i64);
    }

    fn reward_kind(&mut self, token: u64, kind: RewardKind) {
        self.category(token, CategoricalField::RewardKind, kind as i64);
    }

    fn add_token(&mut self, kind: TokenKind) -> Result<u64, SemanticEncodingError> {
        let index = self.token_count()?;
        self.token_kinds.push(kind as u16);
        Ok(index)
    }

    fn token_count(&self) -> Result<u64, SemanticEncodingError> {
        u64::try_from(self.token_kinds.len()).map_err(|_| SemanticEncodingError::IndexOverflow)
    }

    fn category(&mut self, token: u64, field: CategoricalField, value: i64) {
        self.categorical.token_indices.push(token);
        self.categorical.fields.push(field as u16);
        self.categorical.values.push(value);
    }

    fn scalar(&mut self, token: u64, field: ScalarField, value: impl IntoF32) {
        self.scalar.token_indices.push(token);
        self.scalar.fields.push(field as u16);
        self.scalar.values.push(value.into_f32());
    }

    fn edge(&mut self, source: u64, relation: RelationKind, target: u64) {
        self.relation.source_token_indices.push(source);
        self.relation.relations.push(relation as u16);
        self.relation.target_token_indices.push(target);
    }
}

trait IntoF32 {
    fn into_f32(self) -> f32;
}

macro_rules! impl_into_f32 {
    ($($type:ty),+ $(,)?) => {
        $(
            impl IntoF32 for $type {
                fn into_f32(self) -> f32 {
                    self as f32
                }
            }
        )+
    };
}

impl_into_f32!(u8, u16, u32, u64, i8, i32, usize);

fn bool_value(value: bool) -> i64 {
    i64::from(value)
}

#[cfg(test)]
mod tests {
    use sts_oracle_eval::content::cards::CardId;
    use sts_oracle_learning::eval::run_control::{
        LearningEnvV1, LearningModelChoiceV1, LearningModelDecisionV1, LearningSelectionStepV1,
        RunControlConfig, RunControlSession,
    };
    use sts_oracle_eval::runtime::combat::CombatCard;
    use sts_oracle_eval::state::core::{
        EngineState, RunPendingChoiceReason, RunPendingChoiceState,
    };
    use sts_oracle_eval::state::map::node::{MapRoomNode, RoomType};
    use sts_oracle_eval::state::selection::{DomainEventSource, SelectionReason};

    use super::*;

    #[test]
    fn run_selection_root_and_prefix_share_one_complete_strategic_encoding() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.master_deck = vec![
            CombatCard::new(CardId::Strike, 11),
            CombatCard::new(CardId::Defend, 22),
            CombatCard::new(CardId::Bash, 33),
        ];
        session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
            min_choices: 2,
            max_choices: 2,
            reason: RunPendingChoiceReason::Transform,
            source: DomainEventSource::Selection(SelectionReason::Transform),
            return_state: Box::new(EngineState::MapNavigation),
        });
        let boundary = LearningEnvV1::from_session(session)
            .observe()
            .expect("run selection boundary");
        let decision =
            LearningModelDecisionV1::from_boundary(&boundary).expect("root model decision");

        let mut root_builder = SemanticBatchBuilder::new();
        root_builder
            .push_decision(&decision)
            .expect("encode selection root");
        let root = root_builder.finish();
        assert!(root
            .categorical
            .values
            .contains(&(ActionKind::BeginRunCardSelection as i64)));
        assert_eq!(
            root.token_kinds
                .iter()
                .filter(|kind| **kind == TokenKind::SelectionDomain as u16)
                .count(),
            3
        );

        let LearningModelChoiceV1::DecodeSelection(mut draft) =
            decision.choose(0).expect("begin selection")
        else {
            panic!("run root must begin a symbolic decoder");
        };
        assert!(matches!(
            draft.choose(0).expect("append first card"),
            LearningSelectionStepV1::Continue
        ));
        let mut prefix_builder = SemanticBatchBuilder::new();
        prefix_builder
            .push_selection(decision.observation, &draft)
            .expect("encode selection prefix");
        let prefix = prefix_builder.finish();

        assert!(prefix
            .token_kinds
            .contains(&(TokenKind::SelectionState as u16)));
        assert!(prefix
            .token_kinds
            .contains(&(TokenKind::SelectionChosen as u16)));
        assert_eq!(prefix.candidate_token_indices.len(), 2);
        assert_eq!(
            prefix
                .relation
                .relations
                .iter()
                .filter(|relation| **relation == RelationKind::SelectionDomainTargets as u16)
                .count(),
            3
        );
    }

    #[test]
    fn synthetic_boss_route_has_a_semantic_map_target() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::MapNavigation;
        session.run_state.map.graph = vec![Vec::new(); 14];
        let mut final_campfire = MapRoomNode::new(0, 14);
        final_campfire.class = Some(RoomType::RestRoom);
        session.run_state.map.graph.push(vec![final_campfire]);
        session.run_state.map.current_x = 0;
        session.run_state.map.current_y = 14;
        let boundary = LearningEnvV1::from_session(session)
            .observe()
            .expect("boss route boundary");
        let decision =
            LearningModelDecisionV1::from_boundary(&boundary).expect("boss route decision");

        let mut builder = SemanticBatchBuilder::new();
        builder
            .push_decision(&decision)
            .expect("encode synthetic boss route");
        let batch = builder.finish();

        assert_eq!(batch.candidate_token_indices.len(), 1);
        assert!(batch
            .token_kinds
            .contains(&(TokenKind::MapNode as u16)));
        assert!(batch
            .relation
            .relations
            .contains(&(RelationKind::CandidateTargets as u16)));
    }
}
