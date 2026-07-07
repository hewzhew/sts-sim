use super::{Args, Branch};
pub(super) use sts_simulator::runtime::branch::{
    ArtifactKind, ArtifactRef, ArtifactWriteSummary, BranchSummary, CombatSearchTelemetrySummary,
    CombatSearchTimingSummary, FrontierExhausted, FrontierSummary, RealStop, RunSliceRequestKind,
    RunSliceResult, RunStop, SoftPause,
};

pub(super) trait RunSliceResultBranchExt {
    fn with_selected_branch(self, branch: &Branch) -> Self;
}

impl RunSliceResultBranchExt for RunSliceResult {
    fn with_selected_branch(self, branch: &Branch) -> Self {
        self.with_selected_branch_summary(branch_summary(branch))
    }
}

pub(super) fn frontier_summary_from_branches<'a>(
    branches: impl IntoIterator<Item = &'a Branch>,
) -> FrontierSummary {
    FrontierSummary::from_statuses(branches.into_iter().map(|branch| &branch.status))
}

pub(super) fn combat_search_telemetry_from_branches<'a>(
    branches: impl IntoIterator<Item = &'a Branch>,
) -> CombatSearchTelemetrySummary {
    let mut summary = CombatSearchTelemetrySummary::default();
    for branch in branches {
        summary.merge(combat_search_telemetry_from_branch(branch));
    }
    summary
}

#[allow(clippy::too_many_arguments)]
pub(super) fn objective_satisfied_result(
    args: Args,
    request_kind: RunSliceRequestKind,
    generation_start: usize,
    generation: usize,
    next_branch_id: usize,
    branch: &Branch,
    artifacts: ArtifactWriteSummary,
    remaining_ms: Option<u64>,
    elapsed_ms: u64,
) -> RunSliceResult {
    RunSliceResult::new(
        args,
        request_kind,
        generation_start,
        generation,
        next_branch_id,
        RunStop::Real(RealStop::ObjectiveSatisfied {
            generation,
            reason: "objective_satisfied".to_string(),
        }),
        FrontierSummary::from_statuses(std::iter::once(&branch.status)),
        remaining_ms,
        elapsed_ms,
    )
    .with_artifacts(artifacts)
    .with_combat_search_telemetry(combat_search_telemetry_from_branch(branch))
    .with_primary_search_outcome(
        super::primary_search_outcome::primary_search_outcome_from_branch(branch),
    )
    .with_selected_branch(branch)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn slice_result_from_summary(
    args: Args,
    request_kind: RunSliceRequestKind,
    generation_start: usize,
    generation_end: usize,
    next_branch_id: usize,
    stop: RunStop,
    frontier: FrontierSummary,
    selected_branch: Option<&Branch>,
    artifacts: ArtifactWriteSummary,
    remaining_ms: Option<u64>,
    elapsed_ms: u64,
) -> RunSliceResult {
    let mut result = RunSliceResult::new(
        args,
        request_kind,
        generation_start,
        generation_end,
        next_branch_id,
        stop,
        frontier,
        remaining_ms,
        elapsed_ms,
    );
    if let Some(branch) = selected_branch {
        result = result.with_selected_branch(branch);
        result = result.with_primary_search_outcome(
            super::primary_search_outcome::primary_search_outcome_from_branch(branch),
        );
    }
    result.with_artifacts(artifacts)
}

fn branch_summary(branch: &Branch) -> BranchSummary {
    let run = &branch.session.run_state;
    BranchSummary::new(
        branch.id,
        branch.parent_id,
        &branch.status,
        run.act_num,
        run.floor_num,
        run.current_hp,
        run.max_hp,
        run.gold,
        run.master_deck.len(),
    )
}

fn combat_search_telemetry_from_branch(branch: &Branch) -> CombatSearchTelemetrySummary {
    let mut summary = CombatSearchTelemetrySummary::default();
    for attempt in &branch.combat_search {
        summary.record_attempt_with_timing(
            combat_search_telemetry_source(attempt),
            attempt.complete_win_found,
            attempt.terminal_wins,
            attempt.nodes_expanded,
            attempt.total_us,
            CombatSearchTimingSummary {
                rollout_us: attempt.rollout_us,
                expansion_us: attempt.expansion_us,
                engine_step_us: attempt.engine_step_us,
                pre_expand_us: attempt.pre_expand_us,
                frontier_pop_us: attempt.frontier_pop_us,
                child_bookkeeping_us: attempt.child_bookkeeping_us,
                turn_plan_seed_us: attempt.turn_plan_seed_us,
                shadow_audit_us: attempt.shadow_audit_us,
                root_turn_plan_diag_us: attempt.root_turn_plan_diag_us,
                unattributed_us: attempt.unattributed_us,
            },
        );
    }
    summary
}

fn combat_search_telemetry_source(
    attempt: &sts_simulator::eval::run_control::CombatSearchTraceSummary,
) -> String {
    match attempt.lane.as_ref() {
        Some(lane) if lane != &attempt.source => format!("{lane}/{}", attempt.source),
        Some(lane) => lane.clone(),
        None => attempt.source.clone(),
    }
}
