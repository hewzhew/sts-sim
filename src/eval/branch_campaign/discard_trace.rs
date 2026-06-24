use super::branch_display::render_campaign_discard_example_v1;
use super::model::{BranchCampaignBranchV1, BranchCampaignDiscardedBranchV1};

const STRUCTURED_DISCARDED_BRANCH_LIMIT: usize = 256;

pub(super) fn record_campaign_discard_v1(
    branch: &BranchCampaignBranchV1,
    discarded_count: &mut usize,
    discarded_examples: &mut Vec<String>,
    discarded_branches: &mut Vec<BranchCampaignDiscardedBranchV1>,
    reason: &str,
) {
    *discarded_count = discarded_count.saturating_add(1);
    if discarded_examples.len() < 6 {
        discarded_examples.push(render_campaign_discard_example_v1(branch));
    }
    record_campaign_discarded_branch_trace_v1(branch, discarded_branches, reason);
}

pub(super) fn record_campaign_duplicate_merge_v1(
    branch: &BranchCampaignBranchV1,
    discarded_count: &mut usize,
    discarded_examples: &mut Vec<String>,
    discarded_branches: &mut Vec<BranchCampaignDiscardedBranchV1>,
) {
    *discarded_count = discarded_count.saturating_add(1);
    if discarded_examples.len() < 6 {
        discarded_examples.push(format!(
            "merged duplicate: {}",
            render_campaign_discard_example_v1(branch)
        ));
    }
    record_campaign_discarded_branch_trace_v1(branch, discarded_branches, "duplicate_merge");
}

fn record_campaign_discarded_branch_trace_v1(
    branch: &BranchCampaignBranchV1,
    discarded_branches: &mut Vec<BranchCampaignDiscardedBranchV1>,
    reason: &str,
) {
    let is_coverage_gap = branch
        .continuation_origin
        .as_ref()
        .is_some_and(|origin| origin.kind == "coverage_gap");
    if discarded_branches.len() < STRUCTURED_DISCARDED_BRANCH_LIMIT || is_coverage_gap {
        discarded_branches.push(BranchCampaignDiscardedBranchV1::from_branch_v1(
            branch, reason,
        ));
    }
}
