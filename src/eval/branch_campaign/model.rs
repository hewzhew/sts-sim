use crate::ai::route_planner_v1::{
    MapRouteTargetV1, RouteCandidateOrderingV1, RouteMapActionV1, RouteProjectionCoverageV1,
    RouteProjectionSourceV1,
};
use crate::ai::strategic::BranchSignatureCompact;
use crate::eval::branch_experiment::{
    BranchExperimentBossCombatRecordV1, BranchExperimentRewardOptionPortfolioV1,
};
use crate::eval::campaign_journal::{
    CampaignJournalCandidateAdmissionStatusV1, CampaignJournalCandidateAdmissionTraceV1,
    CampaignJournalCandidateDispositionV1, CampaignJournalV1,
};
use crate::eval::combat_lab_probe_v1::CombatLabProbePacketV1;
use crate::eval::event_boundary_packet_v1::EventBoundaryPacketV1;
use crate::eval::reward_boundary_packet_v1::RewardBoundaryPacketV1;
use crate::eval::run_control::{CombatAutomationTrajectoryRecordV1, RunControlSessionCheckpointV1};
use crate::runtime::combat::CombatCard;
use crate::state::map::state::MapState;
use crate::state::run::RunStateScheduleCheckpointV1;
use serde::{Deserialize, Serialize};

use super::performance::BranchCampaignCombatPerformanceSummaryV1;
use super::retry::BranchCampaignCombatRetryLedgerV1;
use super::run_domain::BranchCampaignRunDomainV1;
use super::strategic_signals::BranchCampaignStrategicSignalsV1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchCampaignBranchStatusV1 {
    Active,
    Frozen,
    TerminalVictory,
    TerminalDefeat,
    Abandoned,
    Stuck,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignBranchSummaryV1 {
    pub act: u8,
    pub floor: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_count: usize,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub deck_key: String,
    pub formation_stage: String,
    pub formation_strengths: Vec<String>,
    pub formation_needs: Vec<String>,
    #[serde(default)]
    pub trajectory_key: String,
    #[serde(default)]
    pub boss: String,
    #[serde(default)]
    pub boss_pressure: Vec<String>,
    #[serde(default)]
    pub run_debt: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_boundary: Option<EventBoundaryPacketV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reward_boundary: Option<RewardBoundaryPacketV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignBranchV1 {
    pub branch_id: String,
    pub commands: Vec<String>,
    pub choice_labels: Vec<String>,
    pub summary: Option<BranchCampaignBranchSummaryV1>,
    #[serde(default, skip_serializing_if = "BranchSignatureCompact::is_empty")]
    pub strategic_summary: BranchSignatureCompact,
    pub frontier_title: String,
    pub status: BranchCampaignBranchStatusV1,
    #[serde(default)]
    pub stop_reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation_origin: Option<BranchCampaignContinuationOriginV1>,
    /// Deprecated compatibility field. Older campaign checkpoints may contain
    /// this, but campaign selection no longer carries local decision signals
    /// across rounds.
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub lineage_decision_signal_rank_adjustment: i32,
    pub rank_key: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_boss_combat_record: Option<BranchExperimentBossCombatRecordV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub combat_lab_probes: Vec<CombatLabProbePacketV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignDiscardedBranchV1 {
    pub reason: String,
    pub branch_id: String,
    pub choice_labels: Vec<String>,
    pub frontier_title: String,
    #[serde(default)]
    pub stop_reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<BranchCampaignBranchSummaryV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation_origin: Option<BranchCampaignContinuationOriginV1>,
}

impl BranchCampaignDiscardedBranchV1 {
    pub fn from_branch_v1(branch: &BranchCampaignBranchV1, reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
            branch_id: branch.branch_id.clone(),
            choice_labels: branch.choice_labels.clone(),
            frontier_title: branch.frontier_title.clone(),
            stop_reason: branch.stop_reason.clone(),
            summary: branch.summary.clone(),
            continuation_origin: branch.continuation_origin.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignContinuationOriginV1 {
    pub kind: String,
    pub source_event_id: String,
    pub decision_id: String,
    pub event_type: String,
    pub parent_branch_id: String,
    pub parent_frontier_title: String,
    pub candidate_index: usize,
    pub candidate_id: String,
    pub command: String,
    pub label: String,
    pub semantic_class: String,
    #[serde(default)]
    pub admission: CampaignJournalCandidateAdmissionTraceV1,
    pub disposition: CampaignJournalCandidateDispositionV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_lane: Option<BranchCampaignContinuationTargetLaneV1>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub target_origin_source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_origin: Option<BranchCampaignRouteContinuationOriginV1>,
    pub milestone: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignContinuationTargetLaneV1 {
    pub bucket: String,
    pub admission_status: CampaignJournalCandidateAdmissionStatusV1,
    pub disposition: CampaignJournalCandidateDispositionV1,
    pub semantic_lane: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shop_action_kind: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignRouteContinuationOriginV1 {
    pub legal_candidate_count: usize,
    pub emitted_candidate_count: usize,
    pub complete_legal_pool: bool,
    pub ordering: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordering_kind: Option<RouteCandidateOrderingV1>,
    pub target_x: i32,
    pub target_y: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_node: Option<MapRouteTargetV1>,
    pub room_type: String,
    pub move_kind: String,
    pub action_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<RouteMapActionV1>,
    pub projection_source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_source_kind: Option<RouteProjectionSourceV1>,
    pub projection_coverage: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_coverage_kind: Option<RouteProjectionCoverageV1>,
    pub path_budget: usize,
    pub observed_path_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<BranchCampaignRoutePathContinuationOriginV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_elite: Option<BranchCampaignRouteFirstEliteContinuationOriginV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignRoutePathContinuationOriginV1 {
    pub path_count: usize,
    pub path_budget_exhausted: bool,
    pub min_early_pressure: usize,
    pub max_early_pressure: usize,
    pub min_elites: usize,
    pub max_elites: usize,
    pub min_shops: usize,
    pub max_shops: usize,
    pub min_fires: usize,
    pub max_fires: usize,
    pub min_unknowns: usize,
    pub max_unknowns: usize,
    pub min_treasures: usize,
    pub max_treasures: usize,
    pub first_shop_floor: Option<i32>,
    pub first_fire_floor: Option<i32>,
    pub min_damage_rooms_before_recovery: usize,
    pub max_damage_rooms_before_recovery: usize,
    pub min_unknowns_before_recovery: usize,
    pub max_unknowns_before_recovery: usize,
    pub paths_with_recovery_before_damage: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignRouteFirstEliteContinuationOriginV1 {
    pub paths_with_first_elite: usize,
    pub forced: bool,
    pub optional: bool,
    pub min_hallway_fights_before: usize,
    pub max_hallway_fights_before: usize,
    pub min_unknowns_before: usize,
    pub max_unknowns_before: usize,
    pub min_fires_before: usize,
    pub max_fires_before: usize,
    pub min_shops_before: usize,
    pub max_shops_before: usize,
    pub can_bail_to_rest_before: bool,
    pub can_bail_to_shop_before: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignSelectionV1 {
    pub active: Vec<BranchCampaignBranchV1>,
    pub frozen: Vec<BranchCampaignBranchV1>,
    pub victories: Vec<BranchCampaignBranchV1>,
    pub dead: Vec<BranchCampaignBranchV1>,
    pub abandoned: Vec<BranchCampaignBranchV1>,
    pub stuck: Vec<BranchCampaignBranchV1>,
    pub discarded_count: usize,
    #[serde(default)]
    pub discarded_examples: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discarded_branches: Vec<BranchCampaignDiscardedBranchV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignStrategyRequestV1 {
    pub kind: String,
    pub boundary_title: String,
    pub branch_count: usize,
    #[serde(default)]
    pub act: u8,
    #[serde(default)]
    pub floor: i32,
    #[serde(default)]
    pub stop_reasons: Vec<String>,
    pub examples: Vec<String>,
    #[serde(default)]
    pub next_card_reward_offer: Option<Vec<String>>,
    #[serde(default)]
    pub boundary_details: Vec<String>,
    pub suggested_action: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignRoundSummaryV1 {
    pub round: usize,
    pub started_active: usize,
    pub produced_branches: usize,
    pub active_after: usize,
    pub frozen_added: usize,
    pub dead_added: usize,
    pub abandoned_added: usize,
    pub victories_added: usize,
    pub stuck_added: usize,
    pub discarded_added: usize,
    pub explored_branch_points: usize,
    pub wall_limit_hit: bool,
    pub branch_limit_hit: bool,
    pub combat_budget_retries: usize,
    #[serde(default)]
    pub elapsed_wall_ms: u64,
    #[serde(default)]
    pub parent_elapsed_wall_ms_sum: u64,
    #[serde(default)]
    pub parent_elapsed_wall_ms_max: u64,
    #[serde(default)]
    pub combat_retry_elapsed_wall_ms_sum: u64,
    #[serde(default)]
    pub combat_retry_elapsed_wall_ms_max: u64,
    #[serde(default)]
    pub combat_performance: BranchCampaignCombatPerformanceSummaryV1,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decision_observations: Vec<BranchCampaignDecisionObservationV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignDecisionObservationV1 {
    pub round: usize,
    pub parent_index: usize,
    pub parent_branch_id: String,
    #[serde(default)]
    pub parent_frontier_title: String,
    #[serde(default)]
    pub parent_act: u8,
    #[serde(default)]
    pub parent_floor: i32,
    #[serde(default)]
    pub parent_choices: Vec<String>,
    #[serde(default)]
    pub parent_commands: Vec<String>,
    #[serde(default)]
    pub combat_budget_retry_used: bool,
    pub portfolio: BranchExperimentRewardOptionPortfolioV1,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignRouteEvidenceSummaryV1 {
    pub decisions: usize,
    #[serde(default)]
    pub candidate_pools: usize,
    #[serde(default)]
    pub candidate_pool_candidates: usize,
    #[serde(default)]
    pub candidate_pool_ok: usize,
    #[serde(default)]
    pub candidate_pool_risky: usize,
    #[serde(default)]
    pub candidate_pool_rejected: usize,
    #[serde(default)]
    pub complete_candidate_pools: usize,
    pub first_elite_forced: usize,
    pub first_elite_optional: usize,
    pub first_elite_none: usize,
    pub rest_bailout: usize,
    pub shop_bailout: usize,
    pub underprepared_first_elite: usize,
    pub avg_elite_prep_bp: i32,
    pub examples: Vec<BranchCampaignRouteEvidenceExampleV1>,
    #[serde(default)]
    pub underprepared_examples: Vec<BranchCampaignRouteEvidenceExampleV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignRouteEvidenceExampleV1 {
    pub target: String,
    pub first_elite: String,
    pub elite_prep_bp: i32,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignRunPreludeV1 {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub replay_root_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub branch_command_coordinate: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prefix_commands: Vec<String>,
}

impl BranchCampaignRunPreludeV1 {
    pub fn is_empty(&self) -> bool {
        self.replay_root_id.is_empty()
            && self.branch_command_coordinate.is_empty()
            && self.prefix_commands.is_empty()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignReportV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub seed: u64,
    #[serde(default)]
    pub run_domain: BranchCampaignRunDomainV1,
    #[serde(default, skip_serializing_if = "BranchCampaignRunPreludeV1::is_empty")]
    pub run_prelude: BranchCampaignRunPreludeV1,
    pub rounds_completed: usize,
    pub stop_reason: String,
    pub active: Vec<BranchCampaignBranchV1>,
    pub frozen: Vec<BranchCampaignBranchV1>,
    pub victories: Vec<BranchCampaignBranchV1>,
    pub dead: Vec<BranchCampaignBranchV1>,
    pub abandoned: Vec<BranchCampaignBranchV1>,
    pub stuck: Vec<BranchCampaignBranchV1>,
    pub discarded_count: usize,
    #[serde(default)]
    pub discarded_examples: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discarded_branches: Vec<BranchCampaignDiscardedBranchV1>,
    pub strategy_requests: Vec<BranchCampaignStrategyRequestV1>,
    #[serde(default)]
    pub route_evidence: BranchCampaignRouteEvidenceSummaryV1,
    #[serde(
        default,
        skip_serializing_if = "BranchCampaignCombatRetryLedgerV1::is_empty"
    )]
    pub combat_retry_ledger: BranchCampaignCombatRetryLedgerV1,
    #[serde(
        default,
        skip_serializing_if = "BranchCampaignStrategicSignalsV1::is_empty"
    )]
    pub strategic_signals: BranchCampaignStrategicSignalsV1,
    #[serde(
        default,
        skip_serializing_if = "BranchCampaignStateStoreSummaryV1::is_empty"
    )]
    pub state_store: BranchCampaignStateStoreSummaryV1,
    #[serde(default, skip_serializing_if = "CampaignJournalV1::is_empty")]
    pub journal: CampaignJournalV1,
    pub rounds: Vec<BranchCampaignRoundSummaryV1>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignStateStoreSummaryV1 {
    pub sessions: usize,
    pub nodes: usize,
    pub linked_nodes: usize,
    pub lookup_hits: usize,
    pub lookup_misses: usize,
    #[serde(default)]
    pub replay_exact_hits: usize,
    #[serde(default)]
    pub replay_ancestor_hits: usize,
    #[serde(default)]
    pub replay_misses: usize,
    #[serde(default)]
    pub replay_suffix_commands_sum: usize,
    #[serde(default)]
    pub replay_suffix_commands_max: usize,
    #[serde(default)]
    pub sessions_pruned: usize,
    #[serde(default)]
    pub anchor_sessions_kept: usize,
    #[serde(default)]
    pub decision_coordinate_nodes: usize,
    #[serde(default)]
    pub route_decision_coordinate_nodes: usize,
    #[serde(default)]
    pub decision_coordinate_sessions: usize,
    #[serde(default)]
    pub route_decision_coordinate_sessions: usize,
    pub inserts: usize,
    pub retains: usize,
}

impl BranchCampaignStateStoreSummaryV1 {
    pub fn is_empty(&self) -> bool {
        self.sessions == 0
            && self.nodes == 0
            && self.linked_nodes == 0
            && self.lookup_hits == 0
            && self.lookup_misses == 0
            && self.replay_exact_hits == 0
            && self.replay_ancestor_hits == 0
            && self.replay_misses == 0
            && self.replay_suffix_commands_sum == 0
            && self.replay_suffix_commands_max == 0
            && self.sessions_pruned == 0
            && self.anchor_sessions_kept == 0
            && self.decision_coordinate_nodes == 0
            && self.route_decision_coordinate_nodes == 0
            && self.decision_coordinate_sessions == 0
            && self.route_decision_coordinate_sessions == 0
            && self.inserts == 0
            && self.retains == 0
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointNodeV1 {
    pub node_id: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<usize>,
    pub commands: Vec<String>,
    pub added_commands: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointSessionV1 {
    pub commands: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_state_map_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_state_master_deck_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_state_schedule_id: Option<String>,
    pub session: RunControlSessionCheckpointV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointRunStateMapRecordV1 {
    pub map_id: String,
    pub map: MapState,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointRunStateMasterDeckRecordV1 {
    pub deck_id: String,
    pub master_deck: Vec<CombatCard>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointRunStateScheduleRecordV1 {
    pub schedule_id: String,
    pub schedule: RunStateScheduleCheckpointV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointCombatTrajectoryRecordV1 {
    pub trajectory_id: String,
    pub commands: Vec<Vec<String>>,
    pub trajectory: CombatAutomationTrajectoryRecordV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub seed: u64,
    #[serde(default)]
    pub run_domain: BranchCampaignRunDomainV1,
    #[serde(default, skip_serializing_if = "BranchCampaignRunPreludeV1::is_empty")]
    pub run_prelude: BranchCampaignRunPreludeV1,
    pub rounds_completed: usize,
    #[serde(default)]
    pub nodes: Vec<BranchCampaignCheckpointNodeV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decision_parent_anchor_commands: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_state_maps: Vec<BranchCampaignCheckpointRunStateMapRecordV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_state_master_decks: Vec<BranchCampaignCheckpointRunStateMasterDeckRecordV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_state_schedules: Vec<BranchCampaignCheckpointRunStateScheduleRecordV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub combat_automation_trajectories: Vec<BranchCampaignCheckpointCombatTrajectoryRecordV1>,
    pub sessions: Vec<BranchCampaignCheckpointSessionV1>,
}

impl BranchCampaignCheckpointV1 {
    pub fn hydrated_session_checkpoint_v1(
        &self,
        entry: &BranchCampaignCheckpointSessionV1,
    ) -> Result<RunControlSessionCheckpointV1, String> {
        let mut session = entry.session.clone();
        if let Some(map_id) = entry.run_state_map_id.as_deref() {
            let record = self
                .run_state_maps
                .iter()
                .find(|record| record.map_id == map_id)
                .ok_or_else(|| format!("missing checkpoint run_state map {map_id}"))?;
            session.restore_run_state_map_from_external_ref(record.map.clone());
        }
        if let Some(deck_id) = entry.run_state_master_deck_id.as_deref() {
            let record = self
                .run_state_master_decks
                .iter()
                .find(|record| record.deck_id == deck_id)
                .ok_or_else(|| format!("missing checkpoint run_state master deck {deck_id}"))?;
            session.restore_run_state_master_deck_from_external_ref(record.master_deck.clone());
        }
        if let Some(schedule_id) = entry.run_state_schedule_id.as_deref() {
            let record = self
                .run_state_schedules
                .iter()
                .find(|record| record.schedule_id == schedule_id)
                .ok_or_else(|| format!("missing checkpoint run_state schedule {schedule_id}"))?;
            session.restore_run_state_schedule_from_external_ref(record.schedule.clone());
        }
        if session.last_combat_automation_trajectory_record().is_some() {
            return Ok(session);
        }
        let Some(record) = self.combat_automation_trajectories.iter().find(|record| {
            record
                .commands
                .iter()
                .any(|commands| commands == &entry.commands)
        }) else {
            return Ok(session);
        };
        session.restore_last_combat_automation_trajectory_record(record.trajectory.clone());
        Ok(session)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchCampaignRunResultV1 {
    pub report: BranchCampaignReportV1,
    pub checkpoint: BranchCampaignCheckpointV1,
}

fn is_zero_i32(value: &i32) -> bool {
    *value == 0
}
