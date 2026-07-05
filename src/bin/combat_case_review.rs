use std::path::PathBuf;

use clap::Parser;
#[path = "combat_case_review/boss_pressure_lens.rs"]
mod boss_pressure_lens;
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
#[path = "combat_case_review/key_card_lifecycle.rs"]
mod key_card_lifecycle;
#[path = "combat_case_review/line_lab.rs"]
mod line_lab;
#[path = "combat_case_review/options.rs"]
mod options;
#[path = "combat_case_review/quality_lanes.rs"]
mod quality_lanes;
#[path = "combat_case_review/search_review.rs"]
mod search_review;
#[path = "combat_case_review/search_runner.rs"]
mod search_runner;
#[path = "combat_case_review/search_types.rs"]
mod search_types;
#[path = "combat_case_review/strategic_feedback.rs"]
mod strategic_feedback;

use boss_pressure_lens::boss_pressure_lens;
use case_payload::{assemble_combat_case_review, CombatCaseReview, CombatCaseReviewArtifacts};
use champ_phase::champ_phase_audit;
use classification::classify_gap_review;
use counterfactual_hp::run_counterfactual_hp_probe;
use focus::{focus_witness_line, review_focus, witness_prior_rerun};
use line_lab::run_line_lab;
use options::ReviewOptions;
use quality_lanes::run_quality_lanes;
use search_runner::run_search;
use sts_simulator::ai::combat_search_v2::{
    derive_combat_deficit_evidence, replay_combat_search_witness_line_v0,
    CombatSearchV2PotionPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::eval::combat_case::{load_combat_case, CombatCase};

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
    let review = build_review(&args, case);
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

fn build_review(args: &Args, case: CombatCase) -> CombatCaseReview {
    let options = ReviewOptions::from_args(args);
    let (ladder, line_lab_parent) = if options.ladder {
        let (fast_review, _) = run_search(
            "fast_no_potion_diagnostic",
            &case,
            options.fast_nodes,
            options.fast_ms,
            CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            CombatSearchV2PotionPolicy::Never,
            Some(0),
            &options,
        );
        let (slow_review, slow_report) = run_search(
            "slow_potion_diagnostic",
            &case,
            options.slow_nodes,
            options.slow_ms,
            CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            CombatSearchV2PotionPolicy::All,
            Some(options.diagnostic_potion_max),
            &options,
        );
        (
            vec![fast_review, slow_review],
            slow_report.best_complete_trajectory.clone(),
        )
    } else {
        (Vec::new(), None)
    };
    let review_focus = review_focus(&ladder);
    let classification = classify_gap_review(&ladder, review_focus.as_ref());
    let review_focus_replay = if options.replay_focus {
        review_focus.as_ref().map(|focus| {
            replay_combat_search_witness_line_v0(&case.position, &focus_witness_line(focus))
        })
    } else {
        None
    };
    let review_focus_prior_rerun = review_focus
        .as_ref()
        .zip(review_focus_replay.as_ref())
        .and_then(|(focus, replay)| witness_prior_rerun(&options, &case, focus, replay));
    let line_lab = run_line_lab(&options, &case, line_lab_parent.as_ref());
    let combat_deficit_evidence = line_lab.as_ref().map(derive_combat_deficit_evidence);
    let boss_pressure_lens = boss_pressure_lens(&case, &ladder, line_lab.as_ref());
    let quality_lanes = if options.quality_lanes {
        Some(run_quality_lanes(&options, &case))
    } else {
        None
    };
    let counterfactual_hp_probe = if options.counterfactual_hp_probe {
        Some(run_counterfactual_hp_probe(&options, &case))
    } else {
        None
    };
    let champ_phase_audit = review_focus
        .as_ref()
        .and_then(|focus| champ_phase_audit(&case.position, focus));
    assemble_combat_case_review(
        args.case.display().to_string(),
        case,
        CombatCaseReviewArtifacts {
            ladder,
            classification,
            review_focus,
            review_focus_replay,
            review_focus_prior_rerun,
            line_lab,
            quality_lanes,
            counterfactual_hp_probe,
            combat_deficit_evidence,
            boss_pressure_lens,
            champ_phase_audit,
        },
    )
}
