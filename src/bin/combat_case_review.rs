use std::path::PathBuf;

use clap::Parser;
#[path = "combat_case_review/boss_pressure_lens.rs"]
mod boss_pressure_lens;
#[path = "combat_case_review/boss_setup_lane.rs"]
mod boss_setup_lane;
#[path = "combat_case_review/case_payload.rs"]
mod case_payload;
#[path = "combat_case_review/champ_phase.rs"]
mod champ_phase;
#[path = "combat_case_review/classification.rs"]
mod classification;
#[path = "combat_case_review/counterfactual_hp.rs"]
mod counterfactual_hp;
#[path = "combat_case_review/focus.rs"]
mod focus;
#[path = "combat_case_review/frozen_panel_lanes.rs"]
mod frozen_panel_lanes;
#[path = "combat_case_review/key_card_counterfactual.rs"]
mod key_card_counterfactual;
#[path = "combat_case_review/key_card_decision_microscope.rs"]
mod key_card_decision_microscope;
#[path = "combat_case_review/key_card_lifecycle.rs"]
mod key_card_lifecycle;
#[path = "combat_case_review/line_lab.rs"]
mod line_lab;
#[path = "combat_case_review/options.rs"]
mod options;
#[path = "combat_case_review/quality_lanes.rs"]
mod quality_lanes;
#[path = "combat_case_review/review_pipeline.rs"]
mod review_pipeline;
#[path = "combat_case_review/root_action_role_duel.rs"]
mod root_action_role_duel;
#[path = "combat_case_review/search_review.rs"]
mod search_review;
#[path = "combat_case_review/search_runner.rs"]
mod search_runner;
#[path = "combat_case_review/search_types.rs"]
mod search_types;
#[path = "combat_case_review/strategic_feedback.rs"]
mod strategic_feedback;

use options::ReviewOptions;
use review_pipeline::build_review;
use sts_simulator::eval::combat_case::load_combat_case;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    case: PathBuf,
    #[arg(long)]
    ladder: bool,
    #[arg(long, default_value_t = 200_000)]
    fast_nodes: usize,
    #[arg(long, default_value_t = 2_000)]
    fast_ms: u64,
    #[arg(long, default_value_t = 800_000)]
    slow_nodes: usize,
    #[arg(long, default_value_t = 8_000)]
    slow_ms: u64,
    #[arg(long, default_value_t = 3)]
    diagnostic_potion_max: u32,
    #[arg(long)]
    write_review: Option<PathBuf>,
    #[arg(long)]
    compact: bool,
    #[arg(long, default_value_t = 12)]
    action_preview_limit: usize,
    #[arg(long)]
    replay_focus: bool,
    #[arg(long)]
    immediate_child_rollout: bool,
    #[arg(long, hide = true)]
    lazy_child_rollout: bool,
    #[arg(long)]
    disable_rollout: bool,
    #[arg(long)]
    line_lab: bool,
    #[arg(long, default_value_t = 30_000)]
    line_lab_ms: u64,
    #[arg(long, default_value_t = 8)]
    line_lab_cuts: usize,
    #[arg(long)]
    quality_lanes: bool,
    #[arg(long)]
    frozen_panel_lanes: bool,
    #[arg(long)]
    boss_setup_lane: bool,
    #[arg(long)]
    key_card_counterfactual: bool,
    #[arg(long)]
    key_card_decision_microscope: bool,
    #[arg(long)]
    root_action_role_duel: bool,
    #[arg(long)]
    quality_lane_total_nodes: Option<usize>,
    #[arg(long)]
    quality_lane_total_ms: Option<u64>,
    #[arg(long)]
    counterfactual_hp_probe: bool,
    #[arg(long, default_value = "real,half,full")]
    counterfactual_hp_levels: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let case = load_combat_case(&args.case)?;
    let options = ReviewOptions::from_args(&args);
    let review = build_review(args.case.display().to_string(), options, case);
    let payload = if args.compact {
        serde_json::to_string(&review)?
    } else {
        serde_json::to_string_pretty(&review)?
    };
    if let Some(path) = args.write_review.as_ref() {
        std::fs::write(path, payload)?;
        println!("{}", path.display());
    } else {
        println!("{payload}");
    }
    Ok(())
}
