use crate::ai::route_planner_v1::{
    MapRouteTargetV1, RouteCandidateOrderingV1, RouteMapActionV1, RouteProjectionCoverageV1,
    RouteProjectionSourceV1,
};
use crate::ai::strategic::BranchSignatureCompact;
use crate::content::monsters::factory::EncounterId;
use crate::content::potions::Potion;
use crate::content::relics::{RelicId, RelicState};
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
use crate::runtime::rng::{RngPool, StsRng};
use crate::state::core::ActiveCombat;
use crate::state::events::generator::EventGenerator;
use crate::state::map::node::Map;
use crate::state::map::state::MapState;
use crate::state::run::RunStateScheduleCheckpointV1;
use crate::state::selection::DomainEvent;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
    #[serde(default, skip_serializing_if = "String::is_empty")]
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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
    #[serde(
        default,
        skip_serializing_if = "branch_campaign_combat_performance_summary_is_default_v1"
    )]
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

#[derive(Clone, Debug, PartialEq)]
pub struct BranchCampaignCheckpointNodeV1 {
    pub node_id: usize,
    pub parent_id: Option<usize>,
    pub commands: Vec<String>,
    pub added_commands: Vec<String>,
}

impl Serialize for BranchCampaignCheckpointNodeV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (
            self.node_id,
            self.parent_id,
            &self.commands,
            &self.added_commands,
        )
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BranchCampaignCheckpointNodeV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Compact(usize, Option<usize>, Vec<String>, Vec<String>),
            Map {
                node_id: usize,
                #[serde(default)]
                parent_id: Option<usize>,
                #[serde(default)]
                commands: Vec<String>,
                #[serde(default)]
                added_commands: Vec<String>,
            },
        }

        match Wire::deserialize(deserializer)? {
            Wire::Compact(node_id, parent_id, commands, added_commands) => Ok(Self {
                node_id,
                parent_id,
                commands,
                added_commands,
            }),
            Wire::Map {
                node_id,
                parent_id,
                commands,
                added_commands,
            } => Ok(Self {
                node_id,
                parent_id,
                commands,
                added_commands,
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchCampaignCheckpointSessionV1 {
    pub node_id: Option<usize>,
    pub commands: Vec<String>,
    pub run_state_map_id: Option<String>,
    pub run_state_master_deck_id: Option<String>,
    pub run_state_relics_id: Option<String>,
    pub run_state_potions_id: Option<String>,
    pub run_state_schedule_id: Option<String>,
    pub run_state_emitted_events_id: Option<String>,
    pub active_combat_id: Option<String>,
    pub session: RunControlSessionCheckpointV1,
}

impl Serialize for BranchCampaignCheckpointSessionV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (
            self.node_id,
            &self.commands,
            &self.run_state_map_id,
            &self.run_state_master_deck_id,
            &self.run_state_relics_id,
            &self.run_state_potions_id,
            &self.run_state_schedule_id,
            &self.run_state_emitted_events_id,
            &self.active_combat_id,
            &self.session,
        )
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BranchCampaignCheckpointSessionV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Compact(
                Option<usize>,
                Vec<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                RunControlSessionCheckpointV1,
            ),
            Map {
                #[serde(default)]
                node_id: Option<usize>,
                #[serde(default)]
                commands: Vec<String>,
                #[serde(default)]
                run_state_map_id: Option<String>,
                #[serde(default)]
                run_state_master_deck_id: Option<String>,
                #[serde(default)]
                run_state_relics_id: Option<String>,
                #[serde(default)]
                run_state_potions_id: Option<String>,
                #[serde(default)]
                run_state_schedule_id: Option<String>,
                #[serde(default)]
                run_state_emitted_events_id: Option<String>,
                #[serde(default)]
                active_combat_id: Option<String>,
                session: RunControlSessionCheckpointV1,
            },
        }

        match Wire::deserialize(deserializer)? {
            Wire::Compact(
                node_id,
                commands,
                run_state_map_id,
                run_state_master_deck_id,
                run_state_relics_id,
                run_state_potions_id,
                run_state_schedule_id,
                run_state_emitted_events_id,
                active_combat_id,
                session,
            ) => Ok(Self {
                node_id,
                commands,
                run_state_map_id,
                run_state_master_deck_id,
                run_state_relics_id,
                run_state_potions_id,
                run_state_schedule_id,
                run_state_emitted_events_id,
                active_combat_id,
                session,
            }),
            Wire::Map {
                node_id,
                commands,
                run_state_map_id,
                run_state_master_deck_id,
                run_state_relics_id,
                run_state_potions_id,
                run_state_schedule_id,
                run_state_emitted_events_id,
                active_combat_id,
                session,
            } => Ok(Self {
                node_id,
                commands,
                run_state_map_id,
                run_state_master_deck_id,
                run_state_relics_id,
                run_state_potions_id,
                run_state_schedule_id,
                run_state_emitted_events_id,
                active_combat_id,
                session,
            }),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointRunStateMapRecordV1 {
    pub map_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_state_map_graph_id: Option<String>,
    pub map: MapState,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointRunStateMapGraphRecordV1 {
    pub graph_id: String,
    pub graph: Map,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointRunStateMasterDeckRecordV1 {
    pub deck_id: String,
    pub master_deck: Vec<CombatCard>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointRunStateRelicsRecordV1 {
    pub relics_id: String,
    pub relics: Vec<RelicState>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointRunStatePotionsRecordV1 {
    pub potions_id: String,
    pub potions: Vec<Option<Potion>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointRunStateScheduleRecordV1 {
    pub schedule_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<RunStateScheduleCheckpointV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule_refs: Option<BranchCampaignCheckpointRunStateScheduleRefsV1>,
}

impl BranchCampaignCheckpointRunStateScheduleRecordV1 {
    pub fn resolved_schedule_v1(
        &self,
        components: &BranchCampaignCheckpointScheduleComponentsV1,
    ) -> Result<RunStateScheduleCheckpointV1, String> {
        if let Some(schedule) = self.schedule.clone() {
            return Ok(schedule);
        }
        let refs = self
            .schedule_refs
            .as_ref()
            .ok_or_else(|| format!("schedule {} has no schedule body or refs", self.schedule_id))?;
        refs.resolved_schedule_v1(components)
            .map_err(|err| format!("schedule {}: {err}", self.schedule_id))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointRunStateScheduleRefsV1 {
    #[serde(with = "compact_rng_pool_checkpoint_wire_v1")]
    pub rng_pool: RngPool,
    pub neow_rng_id: String,
    pub event_generator_id: String,
    pub common_relic_pool_id: String,
    pub uncommon_relic_pool_id: String,
    pub rare_relic_pool_id: String,
    pub shop_relic_pool_id: String,
    pub boss_relic_pool_id: String,
    pub monster_list_id: String,
    pub elite_monster_list_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boss_key: Option<EncounterId>,
    pub boss_list_id: String,
}

mod compact_rng_pool_checkpoint_wire_v1 {
    use crate::runtime::rng::{RngPool, StsRng};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Clone, Debug, Deserialize, Serialize)]
    struct StsRngTuple(u64, u64, u32);

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RngPoolWire {
        Compact(Vec<StsRngTuple>),
        Legacy(RngPool),
    }

    pub fn serialize<S>(rng_pool: &RngPool, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        vec![
            tuple_from_rng(&rng_pool.monster_rng),
            tuple_from_rng(&rng_pool.event_rng),
            tuple_from_rng(&rng_pool.merchant_rng),
            tuple_from_rng(&rng_pool.card_rng),
            tuple_from_rng(&rng_pool.treasure_rng),
            tuple_from_rng(&rng_pool.relic_rng),
            tuple_from_rng(&rng_pool.potion_rng),
            tuple_from_rng(&rng_pool.monster_hp_rng),
            tuple_from_rng(&rng_pool.ai_rng),
            tuple_from_rng(&rng_pool.shuffle_rng),
            tuple_from_rng(&rng_pool.card_random_rng),
            tuple_from_rng(&rng_pool.misc_rng),
            tuple_from_rng(&rng_pool.math_rng),
        ]
        .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<RngPool, D::Error>
    where
        D: Deserializer<'de>,
    {
        match RngPoolWire::deserialize(deserializer)? {
            RngPoolWire::Legacy(rng_pool) => Ok(rng_pool),
            RngPoolWire::Compact(entries) => {
                if entries.len() != 13 {
                    return Err(serde::de::Error::custom(format!(
                        "compact rng pool must contain 13 streams, found {}",
                        entries.len()
                    )));
                }
                let mut entries = entries.into_iter().map(rng_from_tuple);
                Ok(RngPool {
                    monster_rng: entries.next().expect("validated stream count"),
                    event_rng: entries.next().expect("validated stream count"),
                    merchant_rng: entries.next().expect("validated stream count"),
                    card_rng: entries.next().expect("validated stream count"),
                    treasure_rng: entries.next().expect("validated stream count"),
                    relic_rng: entries.next().expect("validated stream count"),
                    potion_rng: entries.next().expect("validated stream count"),
                    monster_hp_rng: entries.next().expect("validated stream count"),
                    ai_rng: entries.next().expect("validated stream count"),
                    shuffle_rng: entries.next().expect("validated stream count"),
                    card_random_rng: entries.next().expect("validated stream count"),
                    misc_rng: entries.next().expect("validated stream count"),
                    math_rng: entries.next().expect("validated stream count"),
                })
            }
        }
    }

    fn tuple_from_rng(rng: &StsRng) -> StsRngTuple {
        StsRngTuple(rng.seed0, rng.seed1, rng.counter)
    }

    fn rng_from_tuple(tuple: StsRngTuple) -> StsRng {
        StsRng {
            seed0: tuple.0,
            seed1: tuple.1,
            counter: tuple.2,
        }
    }
}

impl BranchCampaignCheckpointRunStateScheduleRefsV1 {
    fn resolved_schedule_v1(
        &self,
        components: &BranchCampaignCheckpointScheduleComponentsV1,
    ) -> Result<RunStateScheduleCheckpointV1, String> {
        Ok(RunStateScheduleCheckpointV1 {
            rng_pool: self.rng_pool.clone(),
            neow_rng: component_value_v1(&components.neow_rngs, &self.neow_rng_id)?,
            event_generator: component_value_v1(
                &components.event_generators,
                &self.event_generator_id,
            )?,
            common_relic_pool: component_value_v1(
                &components.common_relic_pools,
                &self.common_relic_pool_id,
            )?,
            uncommon_relic_pool: component_value_v1(
                &components.uncommon_relic_pools,
                &self.uncommon_relic_pool_id,
            )?,
            rare_relic_pool: component_value_v1(
                &components.rare_relic_pools,
                &self.rare_relic_pool_id,
            )?,
            shop_relic_pool: component_value_v1(
                &components.shop_relic_pools,
                &self.shop_relic_pool_id,
            )?,
            boss_relic_pool: component_value_v1(
                &components.boss_relic_pools,
                &self.boss_relic_pool_id,
            )?,
            monster_list: component_value_v1(&components.monster_lists, &self.monster_list_id)?,
            elite_monster_list: component_value_v1(
                &components.elite_monster_lists,
                &self.elite_monster_list_id,
            )?,
            boss_key: self.boss_key,
            boss_list: component_value_v1(&components.boss_lists, &self.boss_list_id)?,
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointScheduleComponentsV1 {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub neow_rngs: Vec<BranchCampaignCheckpointComponentRecordV1<StsRng>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_generators: Vec<BranchCampaignCheckpointComponentRecordV1<EventGenerator>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub common_relic_pools: Vec<BranchCampaignCheckpointComponentRecordV1<Vec<RelicId>>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uncommon_relic_pools: Vec<BranchCampaignCheckpointComponentRecordV1<Vec<RelicId>>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rare_relic_pools: Vec<BranchCampaignCheckpointComponentRecordV1<Vec<RelicId>>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shop_relic_pools: Vec<BranchCampaignCheckpointComponentRecordV1<Vec<RelicId>>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boss_relic_pools: Vec<BranchCampaignCheckpointComponentRecordV1<Vec<RelicId>>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub monster_lists: Vec<BranchCampaignCheckpointComponentRecordV1<Vec<EncounterId>>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elite_monster_lists: Vec<BranchCampaignCheckpointComponentRecordV1<Vec<EncounterId>>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boss_lists: Vec<BranchCampaignCheckpointComponentRecordV1<Vec<EncounterId>>>,
}

impl BranchCampaignCheckpointScheduleComponentsV1 {
    pub fn is_empty(&self) -> bool {
        self.neow_rngs.is_empty()
            && self.event_generators.is_empty()
            && self.common_relic_pools.is_empty()
            && self.uncommon_relic_pools.is_empty()
            && self.rare_relic_pools.is_empty()
            && self.shop_relic_pools.is_empty()
            && self.boss_relic_pools.is_empty()
            && self.monster_lists.is_empty()
            && self.elite_monster_lists.is_empty()
            && self.boss_lists.is_empty()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointComponentRecordV1<T> {
    pub component_id: String,
    pub value: T,
}

fn component_value_v1<T: Clone>(
    records: &[BranchCampaignCheckpointComponentRecordV1<T>],
    component_id: &str,
) -> Result<T, String> {
    records
        .iter()
        .find(|record| record.component_id == component_id)
        .map(|record| record.value.clone())
        .ok_or_else(|| format!("missing schedule component {component_id}"))
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointRunStateEmittedEventsRecordV1 {
    pub emitted_events_id: String,
    pub emitted_events: Vec<DomainEvent>,
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
pub struct BranchCampaignCheckpointActiveCombatRecordV1 {
    pub active_combat_id: String,
    pub active_combat: ActiveCombat,
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
    pub decision_parent_anchor_node_ids: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_state_map_graphs: Vec<BranchCampaignCheckpointRunStateMapGraphRecordV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_state_maps: Vec<BranchCampaignCheckpointRunStateMapRecordV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_state_master_decks: Vec<BranchCampaignCheckpointRunStateMasterDeckRecordV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_state_relics: Vec<BranchCampaignCheckpointRunStateRelicsRecordV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_state_potions: Vec<BranchCampaignCheckpointRunStatePotionsRecordV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_state_schedules: Vec<BranchCampaignCheckpointRunStateScheduleRecordV1>,
    #[serde(
        default,
        skip_serializing_if = "BranchCampaignCheckpointScheduleComponentsV1::is_empty"
    )]
    pub run_state_schedule_components: BranchCampaignCheckpointScheduleComponentsV1,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_state_emitted_events: Vec<BranchCampaignCheckpointRunStateEmittedEventsRecordV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub combat_automation_trajectories: Vec<BranchCampaignCheckpointCombatTrajectoryRecordV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_combats: Vec<BranchCampaignCheckpointActiveCombatRecordV1>,
    pub sessions: Vec<BranchCampaignCheckpointSessionV1>,
}

impl BranchCampaignCheckpointV1 {
    pub fn checkpoint_node_commands_v1(&self, node_id: usize) -> Result<Vec<String>, String> {
        let records = self.resolved_checkpoint_nodes_v1()?;
        records
            .into_iter()
            .find(|node| node.node_id == node_id)
            .map(|node| node.commands)
            .ok_or_else(|| format!("missing checkpoint node {node_id}"))
    }

    pub fn session_commands_v1(
        &self,
        entry: &BranchCampaignCheckpointSessionV1,
    ) -> Result<Vec<String>, String> {
        if !entry.commands.is_empty() {
            return Ok(entry.commands.clone());
        }
        let node_id = entry
            .node_id
            .ok_or_else(|| "checkpoint session has no commands or node_id".to_string())?;
        self.checkpoint_node_commands_v1(node_id)
    }

    pub fn resolved_decision_parent_anchor_commands_v1(&self) -> Result<Vec<Vec<String>>, String> {
        let mut commands = self.decision_parent_anchor_commands.clone();
        for node_id in &self.decision_parent_anchor_node_ids {
            commands.push(self.checkpoint_node_commands_v1(*node_id)?);
        }
        commands.sort();
        commands.dedup();
        Ok(commands)
    }

    pub fn resolved_checkpoint_nodes_v1(
        &self,
    ) -> Result<Vec<BranchCampaignCheckpointNodeV1>, String> {
        let mut records = self.nodes.clone();
        records.sort_by_key(|node| node.node_id);
        let mut resolved = Vec::with_capacity(records.len());
        for (expected_id, node) in records.into_iter().enumerate() {
            if node.node_id != expected_id {
                return Err(format!(
                    "campaign checkpoint node ids must be contiguous: expected {}, found {}",
                    expected_id, node.node_id
                ));
            }
            if let Some(parent_id) = node.parent_id {
                if parent_id > node.node_id {
                    return Err(format!(
                        "campaign checkpoint node {} has invalid parent {}",
                        node.node_id, parent_id
                    ));
                }
            }
            let commands = if !node.commands.is_empty() {
                node.commands.clone()
            } else if let Some(parent_id) = node.parent_id {
                let mut commands = resolved
                    .get(parent_id)
                    .map(|parent: &BranchCampaignCheckpointNodeV1| parent.commands.clone())
                    .ok_or_else(|| {
                        format!(
                            "campaign checkpoint node {} references missing parent {}",
                            node.node_id, parent_id
                        )
                    })?;
                commands.extend(node.added_commands.clone());
                commands
            } else {
                node.added_commands.clone()
            };
            resolved.push(BranchCampaignCheckpointNodeV1 { commands, ..node });
        }
        Ok(resolved)
    }

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
            let mut map = record.map.clone();
            if let Some(graph_id) = record.run_state_map_graph_id.as_deref() {
                let graph = self
                    .run_state_map_graphs
                    .iter()
                    .find(|record| record.graph_id == graph_id)
                    .ok_or_else(|| format!("missing checkpoint run_state map graph {graph_id}"))?;
                map.graph = graph.graph.clone();
            }
            session.restore_run_state_map_from_external_ref(map);
        }
        if let Some(deck_id) = entry.run_state_master_deck_id.as_deref() {
            let record = self
                .run_state_master_decks
                .iter()
                .find(|record| record.deck_id == deck_id)
                .ok_or_else(|| format!("missing checkpoint run_state master deck {deck_id}"))?;
            session.restore_run_state_master_deck_from_external_ref(record.master_deck.clone());
        }
        if let Some(relics_id) = entry.run_state_relics_id.as_deref() {
            let record = self
                .run_state_relics
                .iter()
                .find(|record| record.relics_id == relics_id)
                .ok_or_else(|| format!("missing checkpoint run_state relics {relics_id}"))?;
            session.restore_run_state_relics_from_external_ref(record.relics.clone());
        }
        if let Some(potions_id) = entry.run_state_potions_id.as_deref() {
            let record = self
                .run_state_potions
                .iter()
                .find(|record| record.potions_id == potions_id)
                .ok_or_else(|| format!("missing checkpoint run_state potions {potions_id}"))?;
            session.restore_run_state_potions_from_external_ref(record.potions.clone());
        }
        if let Some(schedule_id) = entry.run_state_schedule_id.as_deref() {
            let record = self
                .run_state_schedules
                .iter()
                .find(|record| record.schedule_id == schedule_id)
                .ok_or_else(|| format!("missing checkpoint run_state schedule {schedule_id}"))?;
            session.restore_run_state_schedule_from_external_ref(
                record.resolved_schedule_v1(&self.run_state_schedule_components)?,
            );
        }
        if let Some(emitted_events_id) = entry.run_state_emitted_events_id.as_deref() {
            let record = self
                .run_state_emitted_events
                .iter()
                .find(|record| record.emitted_events_id == emitted_events_id)
                .ok_or_else(|| {
                    format!("missing checkpoint run_state emitted events {emitted_events_id}")
                })?;
            session
                .restore_run_state_emitted_events_from_external_ref(record.emitted_events.clone());
        }
        if let Some(active_combat_id) = entry.active_combat_id.as_deref() {
            let record = self
                .active_combats
                .iter()
                .find(|record| record.active_combat_id == active_combat_id)
                .ok_or_else(|| format!("missing checkpoint active combat {active_combat_id}"))?;
            session.restore_active_combat_from_external_ref(record.active_combat.clone());
        }
        if session.last_combat_automation_trajectory_record().is_some() {
            return Ok(session);
        }
        let entry_commands = self.session_commands_v1(entry)?;
        let Some(record) = self.combat_automation_trajectories.iter().find(|record| {
            record
                .commands
                .iter()
                .any(|commands| commands == &entry_commands)
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

fn branch_campaign_combat_performance_summary_is_default_v1(
    value: &BranchCampaignCombatPerformanceSummaryV1,
) -> bool {
    value == &BranchCampaignCombatPerformanceSummaryV1::default()
}
