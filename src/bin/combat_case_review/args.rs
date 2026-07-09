use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
pub(super) struct Args {
    #[arg(long)]
    pub(super) case: PathBuf,
    #[arg(long)]
    pub(super) ladder: bool,
    #[arg(long, default_value_t = 200_000)]
    pub(super) fast_nodes: usize,
    #[arg(long, default_value_t = 2_000)]
    pub(super) fast_ms: u64,
    #[arg(long, default_value_t = 800_000)]
    pub(super) slow_nodes: usize,
    #[arg(long, default_value_t = 8_000)]
    pub(super) slow_ms: u64,
    #[arg(long, default_value_t = 3)]
    pub(super) diagnostic_potion_max: u32,
    #[arg(long)]
    pub(super) write_review: Option<PathBuf>,
    #[arg(long)]
    pub(super) compact: bool,
    #[arg(long, default_value_t = 12)]
    pub(super) action_preview_limit: usize,
    #[arg(long)]
    pub(super) replay_focus: bool,
    #[arg(long)]
    pub(super) immediate_child_rollout: bool,
    #[arg(long, hide = true)]
    pub(super) lazy_child_rollout: bool,
    #[arg(long)]
    pub(super) disable_rollout: bool,
    #[arg(long)]
    pub(super) turn_plan_ladder: bool,
    #[arg(long)]
    pub(super) rollout_max_actions: Option<usize>,
    #[arg(long)]
    pub(super) rollout_max_evaluations: Option<usize>,
    #[arg(long)]
    pub(super) line_lab: bool,
    #[arg(long, default_value_t = 30_000)]
    pub(super) line_lab_ms: u64,
    #[arg(long, default_value_t = 8)]
    pub(super) line_lab_cuts: usize,
    #[arg(long)]
    pub(super) quality_lanes: bool,
    #[arg(long)]
    pub(super) frozen_panel_lanes: bool,
    #[arg(long)]
    pub(super) forced_potion_opening_lanes: bool,
    #[arg(long)]
    pub(super) boss_setup_lane: bool,
    #[arg(long)]
    pub(super) key_card_counterfactual: bool,
    #[arg(long)]
    pub(super) key_card_decision_microscope: bool,
    #[arg(long)]
    pub(super) root_action_role_duel: bool,
    #[arg(long)]
    pub(super) quality_lane_total_nodes: Option<usize>,
    #[arg(long)]
    pub(super) quality_lane_total_ms: Option<u64>,
    #[arg(long)]
    pub(super) counterfactual_hp_probe: bool,
    #[arg(long, default_value = "real,half,full")]
    pub(super) counterfactual_hp_levels: String,
}
