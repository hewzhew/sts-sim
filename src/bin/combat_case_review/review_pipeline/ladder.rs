use sts_simulator::ai::combat_search_v2::{
    CombatSearchProfile, CombatSearchTurnPlanPluginId, CombatSearchV2Report,
    CombatSearchV2TrajectoryReport,
};
use sts_simulator::eval::combat_case::CombatCase;

use super::super::options::ReviewOptions;
use super::super::search_runner::{
    review_all_potions_profile, review_no_potion_profile, run_config_search,
};
use super::super::search_types::SearchReview;

pub(super) struct ReviewLadderRun {
    pub(super) reviews: Vec<SearchReview>,
    pub(super) line_lab_parent: Option<CombatSearchV2TrajectoryReport>,
}

pub(super) fn run_review_ladder(options: &ReviewOptions, case: &CombatCase) -> ReviewLadderRun {
    if !options.ladder {
        return ReviewLadderRun {
            reviews: Vec::new(),
            line_lab_parent: None,
        };
    }

    let fast_profile = review_no_potion_profile(
        "fast_no_potion_diagnostic",
        options.fast_nodes,
        options.fast_ms,
        options,
    );
    let (fast_review, _) = run_ladder_profile(case, fast_profile, options);

    let slow_profile = review_all_potions_profile(
        "slow_potion_diagnostic",
        options.slow_nodes,
        options.slow_ms,
        options,
    );
    let (slow_review, slow_report) = run_ladder_profile(case, slow_profile, options);

    let mut reviews = vec![fast_review, slow_review];
    if options.turn_plan_ladder {
        let turn_plan_profile = review_no_potion_profile(
            "turn_boundary_seed_no_potion",
            options.fast_nodes,
            options.fast_ms,
            options,
        )
        .with_turn_plan_plugin(CombatSearchTurnPlanPluginId::TurnBoundaryFrontierSeed);
        let (turn_plan_review, _) = run_ladder_profile(case, turn_plan_profile, options);
        reviews.push(turn_plan_review);
    }

    ReviewLadderRun {
        reviews,
        line_lab_parent: slow_report.best_complete_trajectory,
    }
}

fn run_ladder_profile(
    case: &CombatCase,
    profile: CombatSearchProfile,
    options: &ReviewOptions,
) -> (SearchReview, CombatSearchV2Report) {
    let label = profile.label;
    let mut config = profile.to_config();
    if let Some(max_actions) = options.rollout_max_actions {
        config.rollout_max_actions = max_actions;
    }
    if let Some(max_evaluations) = options.rollout_max_evaluations {
        config.rollout_max_evaluations = max_evaluations;
    }
    run_config_search(label, case, config, options.action_preview_limit)
}
