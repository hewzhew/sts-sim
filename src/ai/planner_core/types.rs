use serde::{Deserialize, Serialize};

use crate::content::cards::CardId;
use crate::content::monsters::factory::EncounterId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;
use crate::state::events::{EventActionKind, EventId};
use crate::state::map::node::RoomType;
use crate::state::selection::SelectionScope;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerDecisionSite {
    Map,
    CardReward,
    Neow,
    Event,
    Shop,
    Campfire,
    BossRelic,
    Reward,
    RunChoice,
    Treasure,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerRunGoal {
    ActThreeVictory,
    HeartVictory,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerPlayerClass {
    Ironclad,
    Silent,
    Defect,
    Watcher,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerMechanicsManifest {
    pub mechanics_id: String,
    pub mechanics_version: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerObservation {
    pub schema_name: String,
    pub schema_version: u32,
    pub observation_id: String,
    pub mechanics: PlannerMechanicsManifest,
    pub run_goal: PlannerRunGoal,
    pub decision_site: PlannerDecisionSite,
    pub run: PlannerRunScalars,
    pub cards: Vec<PlannerCardObservation>,
    pub relics: Vec<PlannerRelicObservation>,
    pub potions: Vec<PlannerPotionSlotObservation>,
    pub public_map: PlannerPublicMap,
    pub context: PlannerDecisionContext,
    pub public_history: PlannerPublicHistory,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerRunScalars {
    pub player_class: PlannerPlayerClass,
    pub ascension_level: u8,
    pub act: u8,
    pub floor: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub keys: [bool; 3],
    pub potion_capacity: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerCardObservation {
    pub card_uuid: u32,
    pub card: CardId,
    pub upgrades: u8,
    pub misc_value: i32,
    pub base_damage_override: Option<i32>,
    pub base_block_override: Option<i32>,
    pub cost_modifier: i8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerRelicObservation {
    pub relic: RelicId,
    pub counter: i32,
    pub used_up: bool,
    pub amount: i32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerPotionSlotObservation {
    pub slot: usize,
    pub potion: Option<PlannerPotionObservation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerPotionObservation {
    pub potion: PotionId,
    pub potion_uuid: u32,
    pub can_use: bool,
    pub can_discard: bool,
    pub requires_target: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerPublicMap {
    pub current_x: i32,
    pub current_y: i32,
    pub boss: Option<EncounterId>,
    pub nodes: Vec<PlannerPublicMapNode>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerPublicMapNode {
    pub x: i32,
    pub y: i32,
    pub room: Option<RoomType>,
    pub has_emerald_key: bool,
    pub edges: Vec<PlannerPublicMapEdge>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerPublicMapEdge {
    pub destination_x: i32,
    pub destination_y: i32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlannerDecisionContext {
    Map {
        overlay: bool,
    },
    CardReward {
        reward_item_index: Option<usize>,
    },
    Event {
        event: EventId,
        screen: usize,
    },
    Shop {
        purge_cost: i32,
        purge_available: bool,
    },
    Reward,
    Campfire,
    BossRelic,
    RunChoice,
    Treasure,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerPublicHistory {
    pub shop_purge_count: i32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LegalCandidateSet {
    pub schema_name: String,
    pub schema_version: u32,
    pub candidate_set_id: String,
    pub decision_id: String,
    pub observation_id: String,
    pub site: PlannerDecisionSite,
    pub candidates: Vec<LegalCandidate>,
    pub completeness: CandidateSetCompleteness,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LegalCandidate {
    pub candidate_id: String,
    pub action: PlannerAction,
    pub mechanics: PlannerMechanicsManifest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CandidateSetCompleteness {
    Complete {
        basis: CandidateCompletenessBasis,
    },
    Incomplete {
        basis: CandidateCompletenessBasis,
        gaps: Vec<CandidateRepresentationGap>,
    },
}

impl CandidateSetCompleteness {
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete { .. })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateCompletenessBasis {
    RunControlBoundaryEnumerator,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CandidateRepresentationGap {
    ParameterizedActionFamily,
    UnsupportedBoundaryAction,
    DuplicateTypedIdentity,
    NoRepresentedLegalCandidate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlannerAction {
    ChooseRouteNode {
        x: i32,
        y: i32,
        flight: bool,
    },
    ChooseEventOption {
        event: EventId,
        screen: usize,
        option_index: usize,
        action: EventActionKind,
    },
    TakeCard {
        reward_item_index: Option<usize>,
        option_index: usize,
        card: CardId,
        upgrades: u8,
    },
    OpenCardReward {
        reward_item_index: usize,
    },
    SingingBowl {
        reward_item_index: Option<usize>,
    },
    SkipCardReward {
        reward_item_index: usize,
    },
    ClaimReward {
        reward_item_index: usize,
        reward: PlannerRewardDescriptor,
    },
    Rest,
    Smith {
        card_uuid: u32,
        card: CardId,
        upgrades: u8,
    },
    Dig,
    Lift,
    Toke {
        card_uuid: u32,
        card: CardId,
        upgrades: u8,
    },
    Recall,
    BuyCard {
        shop_slot: usize,
        card: CardId,
        upgrades: u8,
        price: i32,
    },
    BuyRelic {
        shop_slot: usize,
        relic: RelicId,
        price: i32,
    },
    BuyPotion {
        shop_slot: usize,
        potion: PotionId,
        price: i32,
    },
    RemoveCard {
        card_uuid: u32,
        card: CardId,
        upgrades: u8,
        price: i32,
    },
    OpenPendingRewards,
    LeaveShop,
    TakeBossRelic {
        option_index: usize,
        relic: RelicId,
    },
    SkipBossRelic,
    SubmitRunSelection {
        scope: SelectionScope,
        selected_card_uuids: Vec<u32>,
    },
    OpenChest,
    Proceed {
        site: PlannerDecisionSite,
    },
    Cancel {
        site: PlannerDecisionSite,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlannerRewardDescriptor {
    Gold { amount: i32 },
    StolenGold { amount: i32 },
    CardReward { cards: Vec<PlannerOfferedCard> },
    Relic { relic: RelicId },
    Potion { potion: PotionId },
    EmeraldKey,
    SapphireKey,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerOfferedCard {
    pub card: CardId,
    pub upgrades: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerBehaviorEvent {
    pub schema_name: String,
    pub schema_version: u32,
    pub behavior: PlannerBehaviorDecisionRecord,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerBehaviorDecisionRecord {
    pub behavior_id: String,
    pub trajectory_id: String,
    pub run_id: String,
    pub seed_group_id: String,
    pub decision_id: String,
    pub observation_id: String,
    pub legal_candidate_set_id: String,
    pub selected_candidate_id: String,
    pub behavior_policy: BehaviorPolicyManifest,
    pub selection_probability: SelectionProbability,
    pub continuation_policy: ContinuationPolicyManifest,
    pub mechanics: PlannerMechanicsManifest,
    pub provenance: BehaviorLabelProvenance,
    pub sequence: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorPolicyManifest {
    pub source: BehaviorPolicySource,
    pub schema_version: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorPolicySource {
    RunControlCommandStream,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SelectionProbability {
    KnownDeterministic,
    KnownStochastic { numerator: u64, denominator: u64 },
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuationPolicyManifest {
    pub source: ContinuationPolicySource,
    pub config_fingerprint: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuationPolicySource {
    ObservedRunControl,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BehaviorLabelProvenance {
    pub role: PlannerLabelRole,
    pub source: BehaviorObservationSource,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerLabelRole {
    BehaviorPolicyNotTeacher,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorObservationSource {
    LiveCommand,
    ReplayVerified,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerOutcomeAttachment {
    pub attachment_id: String,
    pub behavior_id: String,
    pub decision_id: String,
    pub label_kind: PlannerOutcomeLabelKind,
    pub horizon: PlannerOutcomeHorizon,
    pub before: PlannerOutcomeSnapshot,
    pub after: PlannerOutcomeSnapshot,
    pub continuation_policy: ContinuationPolicyManifest,
    pub mechanics: PlannerMechanicsManifest,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerOutcomeLabelKind {
    RealizedBehaviorRun,
    ExactScenarioReplay,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerOutcomeHorizon {
    AfterOneFloor,
    RunTerminal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerOutcomeSnapshot {
    pub act: u8,
    pub floor: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_size: usize,
    pub relic_count: usize,
    pub potion_count: usize,
    pub terminal: Option<PlannerTerminalKind>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerTerminalKind {
    Victory,
    Defeat,
}
