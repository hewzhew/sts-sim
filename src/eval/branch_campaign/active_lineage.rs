use std::collections::BTreeMap;

use super::lineage::{
    campaign_boss_relic_lineage_counts_for_pool_v1, campaign_boss_relic_lineage_counts_v1,
    campaign_branch_boss_relic_lineage_key_v1, campaign_branch_first_lineage_key_v1,
    campaign_branch_path_lineage_key_v1,
};
use super::model::{BranchCampaignBranchStatusV1, BranchCampaignBranchV1};
use super::selection_key::{
    compare_campaign_branches_for_active_v1, compare_campaign_branches_for_promotion_v1,
};

const LINEAGE_DIVERSITY_MAX_RANK_LAG: i32 = 1_000;

pub(super) fn rebalance_active_lineage_diversity_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    target_unique_lineages: usize,
) -> usize {
    if target_unique_lineages == 0 || active.len() < 2 || frozen.is_empty() {
        return 0;
    }
    let mut swaps = rebalance_active_boss_relic_lineage_v1(active, frozen, target_unique_lineages);
    swaps = swaps.saturating_add(rebalance_active_lineage_spread_v1(
        active,
        frozen,
        target_unique_lineages,
        campaign_branch_first_lineage_key_v1,
    ));
    swaps = swaps.saturating_add(rebalance_active_unique_lineage_v1(
        active,
        frozen,
        target_unique_lineages,
        campaign_branch_path_lineage_key_v1,
    ));
    swaps
}

pub(super) fn refill_active_boss_relic_axes_from_frozen_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    max_active_per_axis: usize,
) -> usize {
    if max_active_per_axis == 0 || frozen.is_empty() {
        return 0;
    }

    let pool_lineages = campaign_boss_relic_lineage_counts_for_pool_v1(active, frozen);
    let mut promoted_count = 0usize;
    for lineage in pool_lineages.keys() {
        loop {
            let active_count = active
                .iter()
                .filter(|branch| {
                    campaign_branch_boss_relic_lineage_key_v1(branch).as_ref() == Some(lineage)
                })
                .count();
            if active_count >= max_active_per_axis {
                break;
            }
            let Some((frozen_index, _)) = frozen
                .iter()
                .enumerate()
                .filter(|(_, branch)| {
                    campaign_branch_boss_relic_lineage_key_v1(branch).as_ref() == Some(lineage)
                })
                .min_by(|(_, left), (_, right)| {
                    compare_campaign_branches_for_active_v1(left, right)
                })
            else {
                break;
            };
            let mut promoted = frozen.remove(frozen_index);
            promoted.status = BranchCampaignBranchStatusV1::Active;
            active.push(promoted);
            promoted_count = promoted_count.saturating_add(1);
        }
    }
    active.sort_by(compare_campaign_branches_for_active_v1);
    frozen.sort_by(compare_campaign_branches_for_promotion_v1);
    promoted_count
}

fn rebalance_active_boss_relic_lineage_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    target_lineage_slots: usize,
) -> usize {
    if target_lineage_slots == 0 || active.len() < 2 || frozen.is_empty() {
        return 0;
    }
    let available_lineage_count = campaign_boss_relic_lineage_counts_for_pool_v1(active, frozen)
        .len()
        .min(target_lineage_slots)
        .min(active.len());
    if available_lineage_count < 2 {
        return 0;
    }
    let max_per_lineage = active.len().div_ceil(available_lineage_count);
    let mut swaps = 0usize;
    loop {
        let active_lineages = campaign_boss_relic_lineage_counts_v1(active);
        let Some((overrepresented_key, overrepresented_count)) =
            active_lineages
                .iter()
                .max_by(|(left_key, left_count), (right_key, right_count)| {
                    left_count
                        .cmp(right_count)
                        .then_with(|| right_key.cmp(left_key))
                })
        else {
            break;
        };
        if *overrepresented_count <= max_per_lineage {
            break;
        }
        let Some((frozen_index, _)) = frozen
            .iter()
            .enumerate()
            .filter(|(_, branch)| {
                let Some(key) = campaign_branch_boss_relic_lineage_key_v1(branch) else {
                    return false;
                };
                key != *overrepresented_key
                    && active_lineages.get(&key).copied().unwrap_or(0) < max_per_lineage
            })
            .min_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
        else {
            break;
        };
        let Some((replace_index, _)) = active
            .iter()
            .enumerate()
            .filter(|(_, branch)| {
                campaign_branch_boss_relic_lineage_key_v1(branch).as_ref()
                    == Some(overrepresented_key)
            })
            .max_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
        else {
            break;
        };
        let mut promoted = frozen.remove(frozen_index);
        promoted.status = BranchCampaignBranchStatusV1::Active;
        let mut demoted = std::mem::replace(&mut active[replace_index], promoted);
        demoted.status = BranchCampaignBranchStatusV1::Frozen;
        frozen.push(demoted);
        active.sort_by(compare_campaign_branches_for_active_v1);
        frozen.sort_by(compare_campaign_branches_for_promotion_v1);
        swaps = swaps.saturating_add(1);
    }
    swaps
}

fn rebalance_active_unique_lineage_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    target_unique_lineages: usize,
    key_fn: fn(&BranchCampaignBranchV1) -> String,
) -> usize {
    let target_unique_lineages = target_unique_lineages.min(active.len());
    let mut swaps = 0usize;
    while campaign_active_lineage_count_v1(active, key_fn) < target_unique_lineages {
        let active_lineages = campaign_active_lineage_counts_v1(active, key_fn);
        let Some((frozen_index, _)) = frozen
            .iter()
            .enumerate()
            .filter(|(_, branch)| !active_lineages.contains_key(&key_fn(branch)))
            .min_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
        else {
            break;
        };
        let Some((replace_index, _)) = active
            .iter()
            .enumerate()
            .filter(|(_, branch)| active_lineages.get(&key_fn(branch)).copied().unwrap_or(0) > 1)
            .max_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
        else {
            break;
        };
        if !campaign_lineage_diversity_rank_close_enough_v1(
            &frozen[frozen_index],
            &active[replace_index],
        ) {
            break;
        }
        let mut promoted = frozen.remove(frozen_index);
        promoted.status = BranchCampaignBranchStatusV1::Active;
        let mut demoted = std::mem::replace(&mut active[replace_index], promoted);
        demoted.status = BranchCampaignBranchStatusV1::Frozen;
        frozen.push(demoted);
        active.sort_by(compare_campaign_branches_for_active_v1);
        frozen.sort_by(compare_campaign_branches_for_promotion_v1);
        swaps = swaps.saturating_add(1);
    }
    swaps
}

fn rebalance_active_lineage_spread_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    target_lineage_slots: usize,
    key_fn: fn(&BranchCampaignBranchV1) -> String,
) -> usize {
    if target_lineage_slots == 0 || active.len() < 2 || frozen.is_empty() {
        return 0;
    }
    let available_lineage_count =
        campaign_branch_lineage_counts_for_pool_v1(active, frozen, key_fn)
            .len()
            .min(target_lineage_slots)
            .min(active.len());
    if available_lineage_count < 2 {
        return 0;
    }
    let max_per_lineage = active.len().div_ceil(available_lineage_count);
    let mut swaps = 0usize;
    loop {
        let active_lineages = campaign_active_lineage_counts_v1(active, key_fn);
        let Some((overrepresented_key, overrepresented_count)) =
            active_lineages
                .iter()
                .max_by(|(left_key, left_count), (right_key, right_count)| {
                    left_count
                        .cmp(right_count)
                        .then_with(|| right_key.cmp(left_key))
                })
        else {
            break;
        };
        if *overrepresented_count <= max_per_lineage {
            break;
        }
        let Some((frozen_index, _)) = frozen
            .iter()
            .enumerate()
            .filter(|(_, branch)| {
                let key = key_fn(branch);
                key != *overrepresented_key
                    && active_lineages.get(&key).copied().unwrap_or(0) < max_per_lineage
            })
            .min_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
        else {
            break;
        };
        let Some((replace_index, _)) = active
            .iter()
            .enumerate()
            .filter(|(_, branch)| key_fn(branch) == *overrepresented_key)
            .max_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
        else {
            break;
        };
        if !campaign_lineage_diversity_rank_close_enough_v1(
            &frozen[frozen_index],
            &active[replace_index],
        ) {
            break;
        }
        let mut promoted = frozen.remove(frozen_index);
        promoted.status = BranchCampaignBranchStatusV1::Active;
        let mut demoted = std::mem::replace(&mut active[replace_index], promoted);
        demoted.status = BranchCampaignBranchStatusV1::Frozen;
        frozen.push(demoted);
        active.sort_by(compare_campaign_branches_for_active_v1);
        frozen.sort_by(compare_campaign_branches_for_promotion_v1);
        swaps = swaps.saturating_add(1);
    }
    swaps
}

fn campaign_lineage_diversity_rank_close_enough_v1(
    candidate: &BranchCampaignBranchV1,
    replaced: &BranchCampaignBranchV1,
) -> bool {
    candidate
        .rank_key
        .saturating_add(LINEAGE_DIVERSITY_MAX_RANK_LAG)
        >= replaced.rank_key
}

fn campaign_active_lineage_count_v1(
    active: &[BranchCampaignBranchV1],
    key_fn: fn(&BranchCampaignBranchV1) -> String,
) -> usize {
    campaign_active_lineage_counts_v1(active, key_fn).len()
}

fn campaign_active_lineage_counts_v1(
    active: &[BranchCampaignBranchV1],
    key_fn: fn(&BranchCampaignBranchV1) -> String,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for branch in active {
        *counts.entry(key_fn(branch)).or_insert(0) += 1;
    }
    counts
}

fn campaign_branch_lineage_counts_for_pool_v1(
    active: &[BranchCampaignBranchV1],
    frozen: &[BranchCampaignBranchV1],
    key_fn: fn(&BranchCampaignBranchV1) -> String,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for branch in active.iter().chain(frozen.iter()) {
        *counts.entry(key_fn(branch)).or_insert(0) += 1;
    }
    counts
}
