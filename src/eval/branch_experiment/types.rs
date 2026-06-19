use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ai::noncombat_strategy_v1::{
    StrategyDeckFormationNeedV1, StrategyDeckFormationStageV1, StrategyPackageIdV2,
};
use crate::content::cards::CardId;
use crate::eval::branch_experiment_retention::{
    BranchRetentionBudgetProfileV1, BranchRetentionDecisionV1, BranchRetentionSlotV1,
};
use crate::eval::branch_experiment_trajectory::BranchTrajectorySignatureV1;
use crate::eval::run_control::{
    CombatAutomationActionV1, RunControlHpLossLimit, RunControlSearchCombatOptions,
};

pub const BRANCH_EXPERIMENT_SCHEMA_NAME: &str = "BranchExperimentV1";
pub const BRANCH_EXPERIMENT_SCHEMA_VERSION: u32 = 22;
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
    #[serde(default)]
    pub strategy_requests: Vec<BranchExperimentStrategyRequestV1>,
    #[serde(default)]
    pub route_decisions: Vec<BranchExperimentRouteDecisionV1>,
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentRewardOptionPortfolioV1 {
    pub depth: usize,
    pub frontier_key: String,
    pub boundary_title: String,
    pub max_reward_options_per_branch: usize,
    pub original_count: usize,
    pub selected_count: usize,
    pub selected_options: Vec<BranchExperimentRewardOptionPortfolioEntryV1>,
    pub pruned_options: Vec<BranchExperimentRewardOptionPortfolioEntryV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentRewardOptionPortfolioEntryV1 {
    pub command: String,
    pub label: String,
    pub semantic_class: String,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentRouteDecisionV1 {
    pub branch_id: String,
    pub target: String,
    pub move_kind: String,
    pub safety: String,
    pub command: String,
    pub elite_prep_bp: i32,
    pub first_elite: BranchExperimentFirstEliteEvidenceV1,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentFirstEliteEvidenceV1 {
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
    #[serde(default)]
    pub acquisition_thesis_rank_adjustment: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acquisition_thesis_summary: Vec<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

pub const BRANCH_EXPERIMENT_SHOP_SELECTED_PLAN_SIGNAL_SOURCE_V1: &str =
    "shop_selected_plan_evaluation_v1";
pub const BRANCH_EXPERIMENT_SHOP_ALTERNATIVE_PLAN_SIGNAL_SOURCE_V1: &str =
    "shop_plan_evaluation_v1";

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
