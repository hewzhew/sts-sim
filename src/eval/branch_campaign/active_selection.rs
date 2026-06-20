use std::collections::{BTreeMap, BTreeSet};

use super::active_lineage::rebalance_active_lineage_diversity_v1;
use super::active_rebalance::{
    campaign_branch_primary_active_eligible_v1, rebalance_active_progress_anchor_v1,
    rebalance_active_survival_anchor_v1,
};
use super::branch_display::render_campaign_discard_example_v1;
use super::frozen_pool::record_campaign_duplicate_merge_v1;
use super::lineage::campaign_branch_boss_relic_lineage_key_v1;
use super::model::{
    BranchCampaignBranchStatusV1, BranchCampaignBranchV1, BranchCampaignSelectionV1,
};
use super::selection_key::compare_campaign_branches_for_active_v1;
use super::{
    campaign_branch_quality_key_v1, normalized_campaign_boundary_title, BranchCampaignConfigV1,
};

pub fn select_campaign_branches_v1(
    branches: Vec<BranchCampaignBranchV1>,
    max_active: usize,
    max_frozen: usize,
) -> BranchCampaignSelectionV1 {
    let mut active_candidates = Vec::new();
    let mut selection = BranchCampaignSelectionV1::default();
    for branch in branches {
        if campaign_stuck_branch_should_be_abandoned_for_combat_triage_v1(&branch) {
            selection.abandoned.push(branch);
            continue;
        }
        match branch.status {
            BranchCampaignBranchStatusV1::TerminalVictory => selection.victories.push(branch),
            BranchCampaignBranchStatusV1::TerminalDefeat => selection.dead.push(branch),
            BranchCampaignBranchStatusV1::Abandoned => selection.abandoned.push(branch),
            BranchCampaignBranchStatusV1::Stuck => selection.stuck.push(branch),
            BranchCampaignBranchStatusV1::Frozen | BranchCampaignBranchStatusV1::Active => {
                active_candidates.push(branch)
            }
        }
    }

    active_candidates.sort_by(compare_campaign_branches_for_active_v1);

    let mut retained_quality_keys = BTreeSet::new();
    for mut branch in active_candidates {
        let quality_key = campaign_branch_quality_key_v1(&branch);
        if !retained_quality_keys.insert(quality_key) {
            record_campaign_duplicate_merge_v1(
                &branch,
                &mut selection.discarded_count,
                &mut selection.discarded_examples,
            );
            continue;
        }

        if selection.active.len() < max_active
            && (campaign_branch_primary_active_eligible_v1(&branch) || selection.active.is_empty())
        {
            branch.status = BranchCampaignBranchStatusV1::Active;
            selection.active.push(branch);
        } else if selection.frozen.len() < max_frozen {
            branch.status = BranchCampaignBranchStatusV1::Frozen;
            selection.frozen.push(branch);
        } else {
            selection.discarded_count = selection.discarded_count.saturating_add(1);
            if selection.discarded_examples.len() < 6 {
                selection
                    .discarded_examples
                    .push(render_campaign_discard_example_v1(&branch));
            }
        }
    }
    rebalance_active_progress_anchor_v1(&mut selection.active, &mut selection.frozen);
    rebalance_active_survival_anchor_v1(&mut selection.active, &mut selection.frozen);
    selection
}

pub(super) fn select_campaign_branches_for_config_v1(
    branches: Vec<BranchCampaignBranchV1>,
    config: &BranchCampaignConfigV1,
) -> BranchCampaignSelectionV1 {
    let mut selection = if config.boss_relic_axis_isolation {
        select_campaign_branches_by_boss_relic_axis_v1(
            branches,
            config.max_active,
            config.max_frozen,
        )
    } else {
        select_campaign_branches_v1(branches, config.max_active, config.max_frozen)
    };
    if config.active_lineage_diversity_slots > 0 {
        rebalance_active_lineage_diversity_v1(
            &mut selection.active,
            &mut selection.frozen,
            config.active_lineage_diversity_slots,
        );
    }
    selection
}

fn select_campaign_branches_by_boss_relic_axis_v1(
    branches: Vec<BranchCampaignBranchV1>,
    max_active_per_axis: usize,
    max_frozen_per_axis: usize,
) -> BranchCampaignSelectionV1 {
    let mut grouped = BTreeMap::<String, Vec<BranchCampaignBranchV1>>::new();
    for branch in branches {
        let key = campaign_branch_boss_relic_lineage_key_v1(&branch)
            .unwrap_or_else(|| "__pre_boss_relic_axis__".to_string());
        grouped.entry(key).or_default().push(branch);
    }

    let mut combined = BranchCampaignSelectionV1::default();
    for (_axis, branches) in grouped {
        let selection =
            select_campaign_branches_v1(branches, max_active_per_axis, max_frozen_per_axis);
        combined.active.extend(selection.active);
        combined.frozen.extend(selection.frozen);
        combined.victories.extend(selection.victories);
        combined.dead.extend(selection.dead);
        combined.abandoned.extend(selection.abandoned);
        combined.stuck.extend(selection.stuck);
        combined.discarded_count = combined
            .discarded_count
            .saturating_add(selection.discarded_count);
        append_discarded_examples_v1(
            &mut combined.discarded_examples,
            selection.discarded_examples,
        );
    }

    combined
        .active
        .sort_by(compare_campaign_branches_for_active_v1);
    combined
        .frozen
        .sort_by(compare_campaign_branches_for_active_v1);
    combined
}

pub(super) fn campaign_stuck_branch_should_be_abandoned_for_combat_triage_v1(
    branch: &BranchCampaignBranchV1,
) -> bool {
    if branch.status != BranchCampaignBranchStatusV1::Stuck {
        return false;
    }
    if !normalized_campaign_boundary_title(&branch.frontier_title).starts_with("combat") {
        return false;
    }
    let stop = branch.stop_reason.to_ascii_lowercase();
    stop.contains("combat search")
        || stop.contains("search-combat")
        || stop.contains("hp-loss")
        || stop.contains("max_hp_loss")
        || stop.contains("high-stakes combat")
}

pub(super) fn append_discarded_examples_v1(target: &mut Vec<String>, incoming: Vec<String>) {
    for example in incoming {
        if target.len() >= 6 {
            break;
        }
        if !target.contains(&example) {
            target.push(example);
        }
    }
}
