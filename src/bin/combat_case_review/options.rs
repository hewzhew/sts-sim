use sts_simulator::ai::combat_search_v2::CombatSearchV2ChildRolloutPolicy;

use super::args::Args;

pub(super) struct ReviewOptions {
    pub(super) ladder: bool,
    pub(super) fast_nodes: usize,
    pub(super) fast_ms: u64,
    pub(super) slow_nodes: usize,
    pub(super) slow_ms: u64,
    pub(super) diagnostic_potion_max: u32,
    pub(super) action_preview_limit: usize,
    pub(super) replay_focus: bool,
    pub(super) disable_rollout: bool,
    pub(super) line_lab: bool,
    pub(super) line_lab_ms: u64,
    pub(super) line_lab_cuts: usize,
    pub(super) quality_lanes: bool,
    pub(super) frozen_panel_lanes: bool,
    pub(super) boss_setup_lane: bool,
    pub(super) key_card_counterfactual: bool,
    pub(super) key_card_decision_microscope: bool,
    pub(super) root_action_role_duel: bool,
    pub(super) quality_lane_total_nodes: Option<usize>,
    pub(super) quality_lane_total_ms: Option<u64>,
    pub(super) counterfactual_hp_probe: bool,
    pub(super) counterfactual_hp_levels: String,
    immediate_child_rollout: bool,
    lazy_child_rollout: bool,
}

impl ReviewOptions {
    pub(super) fn from_args(args: &Args) -> Self {
        Self {
            ladder: args.ladder,
            fast_nodes: args.fast_nodes,
            fast_ms: args.fast_ms,
            slow_nodes: args.slow_nodes,
            slow_ms: args.slow_ms,
            diagnostic_potion_max: args.diagnostic_potion_max,
            action_preview_limit: args.action_preview_limit,
            replay_focus: args.replay_focus,
            disable_rollout: args.disable_rollout,
            line_lab: args.line_lab,
            line_lab_ms: args.line_lab_ms,
            line_lab_cuts: args.line_lab_cuts,
            quality_lanes: args.quality_lanes,
            frozen_panel_lanes: args.frozen_panel_lanes,
            boss_setup_lane: args.boss_setup_lane,
            key_card_counterfactual: args.key_card_counterfactual,
            key_card_decision_microscope: args.key_card_decision_microscope,
            root_action_role_duel: args.root_action_role_duel,
            quality_lane_total_nodes: args.quality_lane_total_nodes,
            quality_lane_total_ms: args.quality_lane_total_ms,
            counterfactual_hp_probe: args.counterfactual_hp_probe,
            counterfactual_hp_levels: args.counterfactual_hp_levels.clone(),
            immediate_child_rollout: args.immediate_child_rollout,
            lazy_child_rollout: args.lazy_child_rollout,
        }
    }

    pub(super) fn child_rollout_policy(&self) -> CombatSearchV2ChildRolloutPolicy {
        if self.immediate_child_rollout && !self.lazy_child_rollout {
            CombatSearchV2ChildRolloutPolicy::Immediate
        } else {
            CombatSearchV2ChildRolloutPolicy::LazyOnPop
        }
    }
}
