use sts_simulator::ai::combat_search_v2::CombatSearchTurnPlanPluginId;
use sts_simulator::ai::combat_search_v2::CombatSearchV2TrajectoryReport;
use sts_simulator::eval::combat_case::CombatCase;

use super::super::options::ReviewOptions;
use super::super::search_runner::{
    review_all_potions_profile, review_no_potion_profile, run_profile_search,
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
    let (fast_review, _) = run_profile_search(case, fast_profile, options.action_preview_limit);

    let slow_profile = review_all_potions_profile(
        "slow_potion_diagnostic",
        options.slow_nodes,
        options.slow_ms,
        options,
    );
    let (slow_review, slow_report) =
        run_profile_search(case, slow_profile, options.action_preview_limit);

    let mut reviews = vec![fast_review, slow_review];
    if options.turn_plan_ladder {
        let turn_plan_profile = review_no_potion_profile(
            "turn_boundary_seed_no_potion",
            options.fast_nodes,
            options.fast_ms,
            options,
        )
        .with_turn_plan_plugin(CombatSearchTurnPlanPluginId::TurnBoundaryFrontierSeed);
        let (turn_plan_review, _) =
            run_profile_search(case, turn_plan_profile, options.action_preview_limit);
        reviews.push(turn_plan_review);
    }

    ReviewLadderRun {
        reviews,
        line_lab_parent: slow_report.best_complete_trajectory,
    }
}
