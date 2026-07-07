use super::{Args, Branch};
pub(super) use sts_simulator::runtime::branch::{
    ArtifactKind, ArtifactRef, ArtifactWriteSummary, BranchSummary, FrontierExhausted,
    FrontierSummary, RealStop, RunSliceRequestKind, RunSliceResult, RunStop, SoftPause,
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
