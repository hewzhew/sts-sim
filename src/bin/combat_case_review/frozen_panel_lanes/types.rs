use serde::Serialize;
use sts_simulator::ai::combat_search_v2::CombatSearchActionPriorPluginId;

use super::super::focus::CombatReviewFocus;
use super::super::key_card_lifecycle::KeyCardLifecycleReport;
use super::super::search_types::SearchReview;

#[derive(Serialize)]
pub(crate) struct FrozenPanelLaneReview {
    pub(super) schema: &'static str,
    pub(super) contract: &'static str,
    pub(super) lanes: Vec<FrozenPanelLaneResult>,
}

#[derive(Serialize)]
pub(super) struct FrozenPanelLaneResult {
    pub(super) lane: &'static str,
    pub(super) search_config_summary: FrozenPanelLaneConfigSummary,
    pub(super) review: SearchReview,
    pub(super) focus: Option<CombatReviewFocus>,
    pub(super) key_card_lifecycle: Option<KeyCardLifecycleReport>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(super) struct FrozenPanelLaneConfigSummary {
    pub(super) max_nodes: usize,
    pub(super) wall_ms: u64,
    pub(super) turn_plan_policy: &'static str,
    pub(super) potion_policy: &'static str,
    pub(super) max_potions_used: u32,
    pub(super) rollout_policy: &'static str,
    pub(super) child_rollout_policy: &'static str,
    pub(super) setup_bias_policy: &'static str,
    pub(super) phase_guard_policy: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FrozenPanelLaneSpec {
    pub(crate) lane: &'static str,
    pub(crate) action_prior_plugin: CombatSearchActionPriorPluginId,
}
