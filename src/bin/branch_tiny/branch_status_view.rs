use super::BranchStatus;

pub(super) fn status_boundary_label(status: &BranchStatus) -> String {
    match status {
        BranchStatus::Running { boundary, .. }
        | BranchStatus::AwaitingAuto { boundary, .. }
        | BranchStatus::AutomationGap { boundary, .. }
        | BranchStatus::CombatGap { boundary, .. }
        | BranchStatus::OperationBudgetExhausted { boundary, .. }
        | BranchStatus::BudgetGap { boundary, .. } => boundary.clone(),
        BranchStatus::Terminal(_) => "Terminal".to_string(),
        BranchStatus::ApplyFailed(_) => "ApplyFailed".to_string(),
        BranchStatus::AdvanceFailed(_) => "AdvanceFailed".to_string(),
    }
}

pub(super) fn status_boundary(status: &BranchStatus) -> &str {
    match status {
        BranchStatus::Running { boundary, .. }
        | BranchStatus::AwaitingAuto { boundary, .. }
        | BranchStatus::AutomationGap { boundary, .. }
        | BranchStatus::CombatGap { boundary, .. }
        | BranchStatus::OperationBudgetExhausted { boundary, .. }
        | BranchStatus::BudgetGap { boundary, .. } => boundary,
        BranchStatus::Terminal(_)
        | BranchStatus::ApplyFailed(_)
        | BranchStatus::AdvanceFailed(_) => "-",
    }
}
