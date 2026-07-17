use serde::{Deserialize, Serialize};

use super::super::{CombatSearchV2OutcomeOrderKeyReport, SearchTerminalLabel};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2AdvanceStop {
    CandidateSatisfied,
    QuantumNodeBudget,
    QuantumWallTime,
    FrontierExhausted,
    AlreadyComplete,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2RootMaterializationStatus {
    NotStarted,
    Partial,
    Complete,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2RootClosureStatus {
    ProvenClosed,
    NotProven,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2RootClosureBlocker {
    RootActionSurfaceNotFullyMaterialized,
    OpenConcreteWork,
    OpenPendingChoiceWork,
    UnresolvedLeaf,
    PendingChoiceOrderedVariantsOmitted,
    MaxActionsPerLine,
    EngineStepLimit,
    PotionBudget,
    HierarchicalTurnBoundaryPortfolioSelection,
    ProofDispositionsUnavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatSearchV2RootActionIdentity {
    pub action_id: usize,
    pub action_key: String,
    pub action_debug: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatSearchV2RootObservedValue {
    pub terminal: SearchTerminalLabel,
    pub outcome_order_key: CombatSearchV2OutcomeOrderKeyReport,
    pub final_hp: i32,
    pub hp_loss: i32,
    pub turns: u32,
    pub potions_used: u32,
    pub potions_discarded: u32,
    pub cards_played: u32,
    pub actions_taken: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatSearchV2RootWorkEvidence {
    pub generated_concrete_nodes: u64,
    pub expanded_concrete_nodes: u64,
    pub open_concrete_states: usize,
    pub open_pending_choice_work_items: usize,
    pub best_exact_complete: Option<CombatSearchV2RootObservedValue>,
    pub best_exact_win: Option<CombatSearchV2RootObservedValue>,
    pub best_open_observed: Option<CombatSearchV2RootObservedValue>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatSearchV2RootActionEvidence {
    pub rank: Option<usize>,
    pub root_action: CombatSearchV2RootActionIdentity,
    pub work: CombatSearchV2RootWorkEvidence,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatSearchV2RootEvidenceSnapshot {
    pub ranking_policy: String,
    pub work_accounting_scope: String,
    pub materialization: CombatSearchV2RootMaterializationStatus,
    pub closure_status: CombatSearchV2RootClosureStatus,
    pub closure_blockers: Vec<CombatSearchV2RootClosureBlocker>,
    pub leader: Option<CombatSearchV2RootActionIdentity>,
    pub contenders: Vec<CombatSearchV2RootActionEvidence>,
    pub unattributed: CombatSearchV2RootWorkEvidence,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatSearchV2QuantumCounters {
    pub nodes_expanded: u64,
    pub nodes_generated: u64,
    pub pending_choice_prefixes_expanded: u64,
    pub rollout_promotion_actions_replayed: u64,
    pub frontier_work_items: usize,
    pub exact_state_keys: usize,
    pub rollout_cache_entries: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatSearchV2QuantumEvidence {
    pub quantum_index: usize,
    pub requested_additional_nodes: usize,
    pub requested_soft_wall_time_ms: Option<u128>,
    pub stop: CombatSearchV2AdvanceStop,
    pub before: CombatSearchV2QuantumCounters,
    pub after: CombatSearchV2QuantumCounters,
    pub root_evidence: CombatSearchV2RootEvidenceSnapshot,
}
