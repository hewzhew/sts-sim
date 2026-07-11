use sts_simulator::ai::boss_matchup::boss_matchup_acquisition_pressure_v0;
use sts_simulator::ai::combat_search_v2::{
    derive_combat_deficit_evidence, replay_combat_search_witness_line_v0,
};
use sts_simulator::eval::combat_case::CombatCase;

#[path = "review_pipeline/ladder.rs"]
mod ladder;

use super::awakened_one_evidence::{
    awakened_one_failure_evidence, awakened_one_path_audit_v0, static_boss_matchup_audit_v0,
};
use super::boss_pressure_lens::boss_pressure_lens;
use super::boss_setup_lane::run_boss_setup_lane;
use super::case_payload::{
    assemble_combat_case_review, CombatCaseReview, CombatCaseReviewArtifacts,
};
use super::champ_phase::champ_phase_audit;
use super::classification::classify_gap_review;
use super::collector_tactic_lanes::run_collector_tactic_lanes;
use super::counterfactual_hp::run_counterfactual_hp_probe;
use super::focus::{focus_witness_line, review_focus, witness_prior_rerun};
use super::forced_potion_opening::run_forced_potion_opening_lanes;
use super::frozen_panel_lanes::run_frozen_panel_lanes;
use super::key_card_counterfactual::run_key_card_counterfactual_probe;
use super::key_card_decision_microscope::run_key_card_decision_microscope_probe;
use super::line_lab::run_line_lab;
use super::options::ReviewOptions;
use super::quality_lanes::run_quality_lanes;
use super::root_action_role_duel::run_root_action_role_duel_probe;
use ladder::run_review_ladder;

pub(super) fn build_review(
    case_path: String,
    options: ReviewOptions,
    case: CombatCase,
) -> CombatCaseReview {
    let ladder_run = run_review_ladder(&options, &case);
    let ladder = ladder_run.reviews;
    let review_focus = review_focus(&ladder);
    let classification =
        classify_gap_review(case.failed_search.as_ref(), &ladder, review_focus.as_ref());
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
    let line_lab = run_line_lab(&options, &case, ladder_run.line_lab_parent.as_ref());
    let combat_deficit_evidence = line_lab.as_ref().map(derive_combat_deficit_evidence);
    let boss_pressure_lens = boss_pressure_lens(&case, &ladder, line_lab.as_ref());
    let boss_setup_lane = run_boss_setup_lane(&options, &case);
    let frozen_panel_lanes = run_frozen_panel_lanes(&options, &case);
    let forced_potion_opening_lanes = run_forced_potion_opening_lanes(&options, &case);
    let key_card_counterfactual = run_key_card_counterfactual_probe(&options, &case);
    let key_card_decision_microscope = run_key_card_decision_microscope_probe(&options, &case);
    let root_action_role_duel = run_root_action_role_duel_probe(&options, &case);
    let collector_tactic_lanes = run_collector_tactic_lanes(&options, &case);
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
    let static_boss_matchup_audit_v0 = static_boss_matchup_audit_v0(&case);
    let boss_matchup_acquisition_pressure_v0 =
        boss_matchup_acquisition_pressure_v0(&case.position.combat);
    let awakened_one_path_audit_v0 = awakened_one_path_audit_v0(&case);
    let awakened_one_failure_evidence =
        awakened_one_failure_evidence(&case, counterfactual_hp_probe.as_ref());
    let champ_phase_audit = review_focus
        .as_ref()
        .and_then(|focus| champ_phase_audit(&case.position, focus));
    assemble_combat_case_review(
        case_path,
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
            static_boss_matchup_audit_v0,
            boss_matchup_acquisition_pressure_v0,
            awakened_one_failure_evidence,
            awakened_one_path_audit_v0,
            boss_pressure_lens,
            boss_setup_lane,
            frozen_panel_lanes,
            forced_potion_opening_lanes,
            key_card_counterfactual,
            key_card_decision_microscope,
            root_action_role_duel,
            collector_tactic_lanes,
            champ_phase_audit,
        },
    )
}
