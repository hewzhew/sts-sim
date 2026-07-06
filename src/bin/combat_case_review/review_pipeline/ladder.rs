use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2PotionPolicy, CombatSearchV2TrajectoryReport, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::eval::combat_case::CombatCase;

use super::super::options::ReviewOptions;
use super::super::search_runner::run_search;
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

    let (fast_review, _) = run_search(
        "fast_no_potion_diagnostic",
        case,
        options.fast_nodes,
        options.fast_ms,
        CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        CombatSearchV2PotionPolicy::Never,
        Some(0),
        options,
    );
    let (slow_review, slow_report) = run_search(
        "slow_potion_diagnostic",
        case,
        options.slow_nodes,
        options.slow_ms,
        CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        CombatSearchV2PotionPolicy::All,
        Some(options.diagnostic_potion_max),
        options,
    );

    ReviewLadderRun {
        reviews: vec![fast_review, slow_review],
        line_lab_parent: slow_report.best_complete_trajectory,
    }
}
