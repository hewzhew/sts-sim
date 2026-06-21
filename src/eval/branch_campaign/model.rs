use crate::ai::strategic::BranchSignatureCompact;
use crate::eval::branch_experiment::{
    BranchExperimentBossCombatRecordV1, BranchExperimentRewardOptionPortfolioV1,
};
use crate::eval::campaign_journal::CampaignJournalV1;
use crate::eval::combat_lab_probe_v1::CombatLabProbePacketV1;
use crate::eval::event_boundary_packet_v1::EventBoundaryPacketV1;
use crate::eval::reward_boundary_packet_v1::RewardBoundaryPacketV1;
use crate::eval::run_control::RunControlSessionCheckpointV1;
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignReportV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub seed: u64,
    #[serde(default)]
    pub run_domain: BranchCampaignRunDomainV1,
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
    pub session: RunControlSessionCheckpointV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCheckpointV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub seed: u64,
    #[serde(default)]
    pub run_domain: BranchCampaignRunDomainV1,
    pub rounds_completed: usize,
    #[serde(default)]
    pub nodes: Vec<BranchCampaignCheckpointNodeV1>,
    pub sessions: Vec<BranchCampaignCheckpointSessionV1>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchCampaignRunResultV1 {
    pub report: BranchCampaignReportV1,
    pub checkpoint: BranchCampaignCheckpointV1,
}

fn is_zero_i32(value: &i32) -> bool {
    *value == 0
}
