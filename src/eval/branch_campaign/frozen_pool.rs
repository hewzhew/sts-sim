use std::collections::BTreeMap;

use super::branch_display::render_campaign_discard_example_v1;
use super::campaign_branch_quality_key_v1;
use super::lineage::{
    campaign_boss_relic_lineage_counts_v1, campaign_branch_boss_relic_lineage_key_v1,
};
use super::model::BranchCampaignBranchV1;
use super::selection_key::campaign_branch_retention_key_v1;

pub(super) fn append_limited_frozen_v1(
    frozen: &mut Vec<BranchCampaignBranchV1>,
    new_frozen: Vec<BranchCampaignBranchV1>,
    max_frozen: usize,
    discarded_count: &mut usize,
    discarded_examples: &mut Vec<String>,
) -> usize {
    let mut added = 0usize;
    for branch in new_frozen {
        if let Some(existing_index) = frozen.iter().position(|existing| {
            campaign_branch_quality_key_v1(existing) == campaign_branch_quality_key_v1(&branch)
        }) {
            if campaign_branch_retention_key_v1(&branch)
                > campaign_branch_retention_key_v1(&frozen[existing_index])
            {
                let displaced = std::mem::replace(&mut frozen[existing_index], branch);
                record_campaign_duplicate_merge_v1(&displaced, discarded_count, discarded_examples);
                added = added.saturating_add(1);
            } else {
                record_campaign_duplicate_merge_v1(&branch, discarded_count, discarded_examples);
            }
            continue;
        }

        if frozen.len() < max_frozen {
            frozen.push(branch);
            added = added.saturating_add(1);
            continue;
        }

        let Some(worst_index) = frozen_replacement_index_v1(frozen, &branch) else {
            record_campaign_discard_v1(&branch, discarded_count, discarded_examples);
            continue;
        };
        let worst_branch = &frozen[worst_index];
        if campaign_branch_retention_key_v1(&branch)
            > campaign_branch_retention_key_v1(worst_branch)
            || branch_introduces_new_boss_relic_lineage_v1(frozen, &branch)
        {
            let displaced = std::mem::replace(&mut frozen[worst_index], branch);
            record_campaign_discard_v1(&displaced, discarded_count, discarded_examples);
            added = added.saturating_add(1);
        } else {
            record_campaign_discard_v1(&branch, discarded_count, discarded_examples);
        }
    }
    added
}

fn frozen_replacement_index_v1(
    frozen: &[BranchCampaignBranchV1],
    incoming: &BranchCampaignBranchV1,
) -> Option<usize> {
    let lineage_counts = campaign_boss_relic_lineage_counts_v1(frozen);
    if branch_introduces_new_boss_relic_lineage_v1(frozen, incoming) {
        if let Some((index, _)) = frozen
            .iter()
            .enumerate()
            .filter(|(_, branch)| {
                branch_is_replaceable_without_losing_boss_relic_lineage_v1(branch, &lineage_counts)
            })
            .min_by(|(_, left), (_, right)| {
                campaign_branch_retention_key_v1(left).cmp(&campaign_branch_retention_key_v1(right))
            })
        {
            return Some(index);
        }
        return None;
    }

    frozen
        .iter()
        .enumerate()
        .filter(|(_, branch)| {
            branch_is_replaceable_without_losing_boss_relic_lineage_v1(branch, &lineage_counts)
        })
        .min_by(|(_, left), (_, right)| {
            campaign_branch_retention_key_v1(left).cmp(&campaign_branch_retention_key_v1(right))
        })
        .map(|(index, _)| index)
}

fn branch_is_replaceable_without_losing_boss_relic_lineage_v1(
    branch: &BranchCampaignBranchV1,
    lineage_counts: &BTreeMap<String, usize>,
) -> bool {
    let Some(lineage) = campaign_branch_boss_relic_lineage_key_v1(branch) else {
        return true;
    };
    lineage_counts.get(&lineage).copied().unwrap_or_default() > 1
}

fn branch_introduces_new_boss_relic_lineage_v1(
    frozen: &[BranchCampaignBranchV1],
    incoming: &BranchCampaignBranchV1,
) -> bool {
    let Some(lineage) = campaign_branch_boss_relic_lineage_key_v1(incoming) else {
        return false;
    };
    !frozen
        .iter()
        .any(|branch| campaign_branch_boss_relic_lineage_key_v1(branch).as_ref() == Some(&lineage))
}

fn record_campaign_discard_v1(
    branch: &BranchCampaignBranchV1,
    discarded_count: &mut usize,
    discarded_examples: &mut Vec<String>,
) {
    *discarded_count = discarded_count.saturating_add(1);
    if discarded_examples.len() < 6 {
        discarded_examples.push(render_campaign_discard_example_v1(branch));
    }
}

pub(super) fn record_campaign_duplicate_merge_v1(
    branch: &BranchCampaignBranchV1,
    discarded_count: &mut usize,
    discarded_examples: &mut Vec<String>,
) {
    *discarded_count = discarded_count.saturating_add(1);
    if discarded_examples.len() < 6 {
        discarded_examples.push(format!(
            "merged duplicate: {}",
            render_campaign_discard_example_v1(branch)
        ));
    }
}
