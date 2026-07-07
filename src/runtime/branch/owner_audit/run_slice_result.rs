use super::Branch;
pub(super) use sts_simulator::runtime::branch::{
    ArtifactWriteSummary, BranchSummary, FrontierExhausted, FrontierSummary, RealStop,
    RunSliceRequestKind, RunSliceResult, RunStop, SoftPause,
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
