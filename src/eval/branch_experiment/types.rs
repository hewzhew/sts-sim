use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ai::noncombat_strategy_v1::{
    StrategyDeckFormationNeedV1, StrategyDeckFormationStageV1, StrategyPackageIdV2,
};
use crate::ai::route_planner_v1::{
    MapDecisionPacketV1, MapRouteTargetV1, NeedVectorV1, NodeFeaturesV1,
    RouteCandidatePoolProvenanceV1, RouteEvaluationCalibrationStatusV1, RouteEvaluationSourceV1,
    RouteMapActionV1, RoutePathSummaryV1, RouteProjectionCoverageV1, RouteProjectionSourceV1,
    RouteSafetyFlagV1, RouteScoreTermsV1, RouteValueFactorsV1,
};
use crate::content::cards::CardId;
use crate::eval::branch_experiment_retention::{
    BranchRetentionBudgetProfileV1, BranchRetentionDecisionV1, BranchRetentionSlotV1,
};
use crate::eval::branch_experiment_trajectory::BranchTrajectorySignatureV1;
use crate::eval::run_control::{
    AutoCombatCaptureConfig, CombatAutomationActionV1, RunControlHpLossLimit,
    RunControlSearchCombatOptions,
};

pub const BRANCH_EXPERIMENT_SCHEMA_NAME: &str = "BranchExperimentV1";
pub const BRANCH_EXPERIMENT_SCHEMA_VERSION: u32 = 32;
pub const BRANCH_EXPERIMENT_CARD_REWARD_STRATEGIC_TRACE_SIGNAL_SOURCE_V1: &str =
    "card_reward_strategic_trace_v1";

#[derive(Clone, Debug, PartialEq)]
pub struct BranchExperimentConfigV1 {
    pub seed: u64,
    pub ascension_level: u8,
    pub player_class: &'static str,
    pub final_act: bool,
    pub max_branches: usize,
    pub max_branches_per_frontier_group: Option<usize>,
    pub retention_budget_profile: BranchRetentionBudgetProfileV1,
    pub max_reward_options_per_branch: Option<usize>,
    pub max_campfire_options_per_branch: Option<usize>,
    pub max_depth: usize,
    pub auto_max_operations: usize,
    pub experiment_wall_ms: Option<u64>,
    pub search_max_nodes: Option<usize>,
    pub search_wall_ms: Option<u64>,
    pub search_max_hp_loss: Option<RunControlHpLossLimit>,
    pub search_options: RunControlSearchCombatOptions,
    pub auto_capture: AutoCombatCaptureConfig,
    pub include_skip: bool,
    pub include_event_reward_skip: bool,
    pub auto_leave_after_shop_purchase_branch: bool,
    pub defer_branch_settle: bool,
    pub prefix_commands: Vec<String>,
    pub replay_trace_path: Option<PathBuf>,
    pub replay_trace_max_steps: Option<usize>,
}

impl Default for BranchExperimentConfigV1 {
    fn default() -> Self {
        Self {
            seed: 1,
            ascension_level: 0,
            player_class: "Ironclad",
            final_act: false,
            max_branches: 12,
            max_branches_per_frontier_group: None,
            retention_budget_profile: BranchRetentionBudgetProfileV1::Balanced,
            max_reward_options_per_branch: None,
            max_campfire_options_per_branch: Some(3),
            max_depth: 4,
            auto_max_operations: 128,
            experiment_wall_ms: None,
            search_max_nodes: None,
            search_wall_ms: Some(100),
            search_max_hp_loss: None,
            search_options: RunControlSearchCombatOptions::default(),
            auto_capture: AutoCombatCaptureConfig::default(),
            include_skip: false,
            include_event_reward_skip: false,
            auto_leave_after_shop_purchase_branch: true,
            defer_branch_settle: true,
            prefix_commands: Vec::new(),
            replay_trace_path: None,
            replay_trace_max_steps: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentReportV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub policy_quality_claim: bool,
    pub seed: u64,
    pub replay_trace_path: Option<String>,
    pub replay_trace_applied_steps: usize,
    pub replay_trace_stop: Option<String>,
    pub max_branches: usize,
    pub max_depth: usize,
    #[serde(default)]
    pub retention_profile: BranchRetentionBudgetProfileV1,
    pub explored_branch_points: usize,
    pub branch_limit_hit: bool,
    pub frontier_group_limit_hit: bool,
    pub wall_limit_hit: bool,
    #[serde(default)]
    pub wall_limit_phase: Option<BranchExperimentWallLimitPhaseV1>,
    pub elapsed_wall_ms: u64,
    pub pruned_branch_count: usize,
    pub pruned_first_pick_counts: Vec<BranchExperimentPrunedFirstPickCountV1>,
    #[serde(default)]
    pub pruned_branch_summary: BranchExperimentPrunedBranchSummaryV1,
    pub reward_option_portfolios: Vec<BranchExperimentRewardOptionPortfolioV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shop_plan_candidate_pools: Vec<BranchExperimentShopPlanCandidatePoolV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub campfire_plan_candidate_pools: Vec<BranchExperimentCampfirePlanCandidatePoolV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_candidate_pools: Vec<BranchExperimentEventCandidatePoolV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boss_relic_candidate_pools: Vec<BranchExperimentBossRelicCandidatePoolV1>,
    #[serde(default)]
    pub strategy_requests: Vec<BranchExperimentStrategyRequestV1>,
    #[serde(default)]
    pub route_decisions: Vec<BranchExperimentRouteDecisionV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub route_candidate_pools: Vec<BranchExperimentRouteCandidatePoolV1>,
    pub frontier_groups: Vec<BranchExperimentFrontierGroupV1>,
    pub branches: Vec<BranchExperimentBranchReportV1>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchExperimentWallLimitPhaseV1 {
    Expansion,
    FinalSettle,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentPrunedFirstPickCountV1 {
    pub first_pick: String,
    pub count: usize,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentPrunedBranchSummaryV1 {
    pub primary_slot_counts: BTreeMap<BranchRetentionSlotV1, usize>,
    pub eligible_slot_counts: BTreeMap<BranchRetentionSlotV1, usize>,
    pub package_state_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub choice_effect_counts: BTreeMap<String, usize>,
    #[serde(default)]
    pub lineage_flag_counts: BTreeMap<String, usize>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentRewardOptionPortfolioV1 {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub branch_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_choices: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_commands: Vec<String>,
    pub depth: usize,
    pub frontier_key: String,
    pub boundary_title: String,
    pub max_reward_options_per_branch: usize,
    pub original_count: usize,
    pub selected_count: usize,
    pub selected_options: Vec<BranchExperimentRewardOptionPortfolioEntryV1>,
    pub pruned_options: Vec<BranchExperimentRewardOptionPortfolioEntryV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentRewardOptionPortfolioEntryV1 {
    pub command: String,
    pub label: String,
    pub semantic_class: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentShopPlanCandidatePoolV1 {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub branch_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_choices: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_commands: Vec<String>,
    pub depth: usize,
    pub frontier_key: String,
    pub boundary_title: String,
    pub candidate_count: usize,
    pub branch_frontier_count: usize,
    pub rollout_head_plan_id: Option<String>,
    pub candidates: Vec<BranchExperimentShopPlanCandidateEntryV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentShopPlanCandidateEntryV1 {
    pub plan_id: String,
    pub command: String,
    pub label: String,
    pub role: String,
    pub source: String,
    pub kind: String,
    pub lane: String,
    pub projection_roles: Vec<String>,
    pub total_gold_spent: i32,
    pub legacy_priority: Option<i32>,
    pub suppressed_count: usize,
    pub verdict: String,
    pub rollout_admission: String,
    pub branch_admission: String,
    pub tier: i32,
    pub score: i32,
    pub confidence_milli: i32,
    pub component_net_rank: i32,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentCampfirePlanCandidatePoolV1 {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub branch_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_choices: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_commands: Vec<String>,
    pub depth: usize,
    pub frontier_key: String,
    pub boundary_title: String,
    pub candidate_count: usize,
    pub branch_option_count: usize,
    pub selected_plan_id: Option<String>,
    pub candidates: Vec<BranchExperimentCampfirePlanCandidateEntryV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentCampfirePlanCandidateEntryV1 {
    pub plan_id: String,
    pub command: String,
    pub label: String,
    pub role: String,
    pub effect_kind: String,
    pub strategy_tag: Option<String>,
    pub score_hint: i32,
    pub confidence_milli: i32,
    pub execute_autopilot: bool,
    pub branch_active: bool,
    pub branch_admission: String,
    pub representative_count: usize,
    pub suppressed_count: usize,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentEventCandidatePoolV1 {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub branch_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_choices: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_commands: Vec<String>,
    pub depth: usize,
    pub frontier_key: String,
    pub boundary_title: String,
    pub event_id: String,
    pub candidate_count: usize,
    pub branch_option_count: usize,
    pub candidates: Vec<BranchExperimentEventCandidateEntryV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentEventCandidateEntryV1 {
    pub candidate_id: String,
    pub command: String,
    pub label: String,
    pub event_index: Option<usize>,
    pub effect_kind: String,
    pub effect_key: String,
    pub event_policy_class: Option<String>,
    pub event_policy_tier: Option<String>,
    pub event_policy_score: Option<i32>,
    pub branch_admission: String,
    pub representative_count: usize,
    pub suppressed_count: usize,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentBossRelicCandidatePoolV1 {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub branch_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_choices: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_commands: Vec<String>,
    pub depth: usize,
    pub frontier_key: String,
    pub boundary_title: String,
    pub candidate_count: usize,
    pub branch_option_count: usize,
    pub candidates: Vec<BranchExperimentBossRelicCandidateEntryV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentBossRelicCandidateEntryV1 {
    pub candidate_id: String,
    pub command: String,
    pub label: String,
    pub relic: String,
    pub class: String,
    pub support_gate: String,
    pub added_debt: Vec<String>,
    pub compounding_tags: Vec<String>,
    pub branch_admission: String,
    pub evidence: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentStrategyRequestV1 {
    pub kind: String,
    pub boundary_title: String,
    pub branch_count: usize,
    pub representative_branch_id: String,
    pub act: u8,
    pub floor: i32,
    pub stop_reasons: Vec<String>,
    pub examples: Vec<String>,
    pub next_card_reward_offer: Option<Vec<String>>,
    #[serde(default)]
    pub boundary_details: Vec<String>,
    pub suggested_action: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentRouteDecisionV1 {
    pub branch_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_choices: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_commands: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate_rank: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_target_node: Option<MapRouteTargetV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_candidate: Option<BranchExperimentRouteCandidateEntryV1>,
    pub target: String,
    pub move_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety_flag: Option<RouteSafetyFlagV1>,
    pub safety: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_pool_provenance: Option<RouteCandidatePoolProvenanceV1>,
    pub command: String,
    pub elite_prep_bp: i32,
    pub first_elite: BranchExperimentFirstEliteEvidenceV1,
}

impl BranchExperimentRouteDecisionV1 {
    pub fn resolved_safety_flag(&self) -> RouteSafetyFlagV1 {
        self.safety_flag
            .unwrap_or_else(|| legacy_route_safety_flag_v1(&self.safety))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentRouteCandidatePoolV1 {
    pub branch_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_choices: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_commands: Vec<String>,
    pub decision_id: String,
    pub boundary_title: String,
    pub frontier_key: String,
    pub depth: usize,
    pub candidate_count: usize,
    pub selected_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_pool_provenance: Option<RouteCandidatePoolProvenanceV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_decision_packet: Option<MapDecisionPacketV1>,
    pub candidates: Vec<BranchExperimentRouteCandidateEntryV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentRouteCandidateEntryV1 {
    pub candidate_id: String,
    pub rank: usize,
    pub selected: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_node: Option<MapRouteTargetV1>,
    pub target: String,
    pub room_type: String,
    pub move_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<RouteMapActionV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety_flag: Option<RouteSafetyFlagV1>,
    pub safety: String,
    pub score: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_terms: Option<RouteScoreTermsV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_factors: Option<RouteValueFactorsV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation_source: Option<RouteEvaluationSourceV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation_calibration_status: Option<RouteEvaluationCalibrationStatusV1>,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_features: Option<NodeFeaturesV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_summary: Option<RoutePathSummaryV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs: Option<NeedVectorV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_source: Option<RouteProjectionSourceV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_coverage: Option<RouteProjectionCoverageV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_budget: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_path_count: Option<usize>,
    pub elite_prep_bp: i32,
    pub first_elite: BranchExperimentFirstEliteEvidenceV1,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cautions: Vec<String>,
}

impl BranchExperimentRouteCandidateEntryV1 {
    pub fn resolved_safety_flag(&self) -> RouteSafetyFlagV1 {
        self.safety_flag
            .unwrap_or_else(|| legacy_route_safety_flag_v1(&self.safety))
    }
}

fn legacy_route_safety_flag_v1(safety: &str) -> RouteSafetyFlagV1 {
    match safety {
        "ok" => RouteSafetyFlagV1::Ok,
        "risky" | "risky_but_allowed" => RouteSafetyFlagV1::RiskyButAllowed,
        "reject_unless_forced" | "reject" => RouteSafetyFlagV1::RejectUnlessNoAlternative,
        _ => RouteSafetyFlagV1::RiskyButAllowed,
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentFirstEliteEvidenceV1 {
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub paths_with_first_elite: usize,
    #[serde(default, skip_serializing_if = "is_false")]
    pub forced: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub optional: bool,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub min_hallway_fights_before: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub max_hallway_fights_before: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub min_unknowns_before: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub max_unknowns_before: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub min_fires_before: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub max_fires_before: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub min_shops_before: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub max_shops_before: usize,
    #[serde(default, skip_serializing_if = "is_false")]
    pub can_bail_to_rest_before: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub can_bail_to_shop_before: bool,
}

fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentBranchReportV1 {
    pub branch_id: String,
    pub status: BranchExperimentBranchStatusV1,
    pub rank_key: i32,
    pub retention: BranchRetentionDecisionV1,
    pub choices: Vec<BranchExperimentChoiceV1>,
    pub stop_reason: String,
    pub summary: BranchExperimentRunSummaryV1,
    pub frontier: BranchExperimentFrontierV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_boss_combat_record: Option<BranchExperimentBossCombatRecordV1>,
    #[serde(default)]
    pub boundary_details: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentBossCombatRecordV1 {
    pub source: String,
    pub action_count: usize,
    pub actions: Vec<CombatAutomationActionV1>,
    pub label_role: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchExperimentBranchStatusV1 {
    Active,
    TerminalVictory,
    TerminalDefeat,
    NeedsHumanBoundary,
    Failed,
    Pruned,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentChoiceV1 {
    pub depth: usize,
    pub kind: String,
    #[serde(default)]
    pub boundary_title: String,
    pub card: Option<CardId>,
    pub upgrades: Option<u8>,
    #[serde(default)]
    pub selected_cards: Vec<BranchExperimentChoiceCardV1>,
    #[serde(default)]
    pub effect_kind: String,
    #[serde(default)]
    pub effect_key: String,
    #[serde(default)]
    pub effect_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_axis: Option<String>,
    #[serde(default)]
    pub representative_count: usize,
    #[serde(default)]
    pub suppressed_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_signal: Option<BranchExperimentChoiceDecisionSignalV1>,
    pub label: String,
    pub command: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentChoiceDecisionSignalV1 {
    pub source: String,
    pub verdict: String,
    pub tier: i32,
    pub score: i32,
    pub confidence_milli: i32,
    pub component_net_rank: i32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub preferred: bool,
}

pub const BRANCH_EXPERIMENT_SHOP_COMPAT_SELECTED_PLAN_SIGNAL_SOURCE_V1: &str =
    "shop_compat_selected_plan_evaluation_v1";
pub const BRANCH_EXPERIMENT_SHOP_ALTERNATIVE_PLAN_SIGNAL_SOURCE_V1: &str =
    "shop_plan_evaluation_v1";
pub const BRANCH_EXPERIMENT_SHOP_BRANCH_FRONTIER_SIGNAL_SOURCE_V1: &str =
    "shop_branch_frontier_evaluation_v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentChoiceCardV1 {
    pub card: CardId,
    pub upgrades: u8,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentRunSummaryV1 {
    pub act: u8,
    pub floor: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_count: usize,
    pub relic_count: usize,
    pub potion_count: usize,
    pub formation_stage: StrategyDeckFormationStageV1,
    pub formation_needs: Vec<StrategyDeckFormationNeedV1>,
    pub formation_strengths: Vec<StrategyPackageIdV2>,
    pub trajectory: BranchTrajectorySignatureV1,
    pub boundary_title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentFrontierV1 {
    pub key: String,
    pub act: u8,
    pub floor: i32,
    pub boundary_title: String,
    pub card_rng_counter: u32,
    pub card_blizz_randomizer: i32,
    pub next_card_reward_offer: Option<Vec<String>>,
    pub lineage: BranchExperimentLineageV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentLineageV1 {
    pub visibility: String,
    pub public_policy_input: bool,
    pub direct_pick_consumes_card_rng: bool,
    pub same_reward_offer_lineage_key: String,
    pub reward_screen_context: String,
    pub reward_count_modifiers: Vec<String>,
    pub card_pool_modifiers: Vec<String>,
    pub rarity_modifiers: Vec<String>,
    pub preview_modifiers: Vec<String>,
    pub sequence_breakers_present: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentFrontierGroupV1 {
    pub key: String,
    pub branch_count: usize,
    pub representative_branch_id: String,
    pub boundary_title: String,
    pub next_card_reward_offer: Option<Vec<String>>,
    pub lineage_flags: Vec<String>,
}
