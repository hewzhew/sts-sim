use serde::{Deserialize, Serialize};
use sts_core::sim::combat::CombatPosition;

use crate::types::TurnOptionAction;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OracleCombatWitnessSatisfaction {
    #[default]
    FirstWitness,
    HpLossAtMost(u32),
    BudgetOrExhaustion,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleCombatWitnessReplayError {
    IllegalInput { action_index: usize },
    TransitionStepLimit { action_index: usize },
    SuccessorMismatch { action_index: usize },
    FinalStateIsNotWin,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OracleCombatWitness {
    pub actions: Vec<TurnOptionAction>,
    pub final_position: CombatPosition,
    pub negative_log_policy: f64,
    pub replay_engine_steps: usize,
    #[serde(default)]
    pub discovery_source: OracleCombatWitnessDiscoverySource,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleCombatWitnessDiscoverySource {
    /// Older serialized witnesses predate discovery provenance. They remain
    /// exact replay evidence but cannot prove which search capability found
    /// their action sequence.
    #[default]
    LegacyUnattributed,
    PlannerSearch,
    PolicyDiscrepancySearch,
    PolicyProposal,
    RestoredExactActions,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct OracleCombatWitnessProgressSnapshot {
    pub retained_states: usize,
    pub queued_anchor_entries: usize,
    pub queued_guided_entries: Vec<usize>,
    pub guide_queues: Vec<OracleCombatGuideQueueSnapshot>,
    pub max_player_turn: u32,
    pub max_path_atomic_depth: usize,
    pub max_completed_turn_options_at_state: usize,
    pub generation_gap_count: usize,
    pub pending_witness_replay: bool,
    pub root_state: Option<OracleCombatWitnessStateProgressSnapshot>,
    pub deepest_survival_state: Option<OracleCombatDeepStateSnapshot>,
    pub deepest_progress_state: Option<OracleCombatDeepStateSnapshot>,
    /// Exact public action prefix that reaches `deepest_survival_state`.
    /// Diagnostic only; it has no authority over queue ordering.
    pub deepest_survival_actions: Vec<TurnOptionAction>,
    /// Exact public action prefix that reaches `deepest_progress_state`.
    /// Diagnostic only; it has no authority over queue ordering.
    pub deepest_progress_actions: Vec<TurnOptionAction>,
    /// For each of the most recent retained player turns, the state with the
    /// highest player HP (then least remaining enemy HP). This is diagnostic:
    /// it exposes whether deeper search is advancing only along a dying line
    /// without assigning that envelope any search authority.
    pub recent_turn_survival_envelope: Vec<OracleCombatDeepStateSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OracleCombatGuideQueueSnapshot {
    pub lane_id: u32,
    pub entries: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OracleCombatGuideRankSnapshot {
    pub lane_id: u32,
    pub states_ahead: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OracleCombatDeepStateSnapshot {
    pub player_turn: u32,
    pub player_hp: i32,
    pub player_block: i32,
    pub alive_enemy_count: usize,
    pub enemy_total_hp: i32,
    pub hand_size: usize,
    pub draw_pile_size: usize,
    pub discard_pile_size: usize,
    pub exhaust_pile_size: usize,
    pub path_atomic_depth: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OracleCombatWitnessStateProgressSnapshot {
    pub exact_state_hash: String,
    pub path_atomic_depth: usize,
    pub path_negative_log_policy: f64,
    pub generator_work: usize,
    pub generator_engine_steps: usize,
    pub completed_turn_options: usize,
    pub retained_generator_work_items: usize,
    pub synced_options: usize,
    pub anchor_states_ahead: Option<usize>,
    pub guided_states_ahead: Option<Vec<usize>>,
    pub guided_lane_ranks: Option<Vec<OracleCombatGuideRankSnapshot>>,
}
