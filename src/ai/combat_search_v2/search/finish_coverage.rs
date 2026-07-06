use super::super::*;

pub(super) fn coverage_status_for_finished_search(
    stats: &CombatSearchV2Stats,
    exhaustive: bool,
    accepted_complete_candidate: bool,
) -> SearchCoverageStatus {
    if accepted_complete_candidate {
        SearchCoverageStatus::AcceptedCompleteCandidate
    } else if stats.deadline_hit {
        SearchCoverageStatus::TimeBudgetLimited
    } else if stats.node_budget_hit {
        SearchCoverageStatus::NodeBudgetLimited
    } else if exhaustive {
        SearchCoverageStatus::Exhaustive
    } else {
        SearchCoverageStatus::FrontierOpen
    }
}

pub(super) fn coverage_status_reason(coverage_status: SearchCoverageStatus) -> String {
    match coverage_status {
        SearchCoverageStatus::Exhaustive => {
            "frontier exhausted under the current exact-state search configuration".to_string()
        }
        SearchCoverageStatus::AcceptedCompleteCandidate => {
            "stopped after finding a complete winning candidate within the configured hp-loss acceptance threshold".to_string()
        }
        SearchCoverageStatus::NodeBudgetLimited => {
            "node budget limit reached with frontier still open".to_string()
        }
        SearchCoverageStatus::TimeBudgetLimited => {
            "wall-clock deadline reached with frontier still open".to_string()
        }
        SearchCoverageStatus::FrontierOpen => {
            "frontier remains open under current safety limits".to_string()
        }
    }
}
