use std::cmp::Ordering;

use super::strategic_signals::{
    campaign_branch_strategic_sort_key_v1, BranchCampaignStrategicSortKeyV1,
};
use super::{branch_progress_key, branch_resource_conversion_key_v1, BranchCampaignBranchV1};

type BranchCampaignBossCheckpointSortKeyV1 = (u8, i32, i32, i32);
type BranchCampaignActiveSortKeyV1 = (
    BranchCampaignBossCheckpointSortKeyV1,
    i32,
    (u8, i32, i32),
    i32,
    BranchCampaignStrategicSortKeyV1,
);
type BranchCampaignPromotionSortKeyV1 =
    ((u8, i32, i32), i32, i32, BranchCampaignStrategicSortKeyV1);
type BranchCampaignRetentionKeyV1 = (u8, i32, i32, i32, i32, BranchCampaignStrategicSortKeyV1);

pub(super) fn compare_campaign_branches_for_active_v1(
    left: &BranchCampaignBranchV1,
    right: &BranchCampaignBranchV1,
) -> Ordering {
    campaign_branch_active_sort_key_v1(right)
        .cmp(&campaign_branch_active_sort_key_v1(left))
        .then_with(|| left.branch_id.cmp(&right.branch_id))
}

pub(super) fn compare_campaign_branches_for_promotion_v1(
    left: &BranchCampaignBranchV1,
    right: &BranchCampaignBranchV1,
) -> Ordering {
    campaign_branch_promotion_sort_key_v1(right)
        .cmp(&campaign_branch_promotion_sort_key_v1(left))
        .then_with(|| left.branch_id.cmp(&right.branch_id))
}

pub(super) fn campaign_branch_retention_key_v1(
    branch: &BranchCampaignBranchV1,
) -> BranchCampaignRetentionKeyV1 {
    let (act, floor, hp) = branch_progress_key(branch);
    (
        act,
        floor,
        hp,
        campaign_branch_selection_rank_key_v1(branch),
        branch_resource_conversion_key_v1(branch),
        campaign_branch_strategic_sort_key_v1(branch),
    )
}

pub(super) fn render_campaign_branch_selection_basis_v1(branch: &BranchCampaignBranchV1) -> String {
    format!(
        "sel=[rank={} res={}]",
        format_campaign_selection_rank_key_v1(campaign_branch_selection_rank_key_v1(branch)),
        branch_resource_conversion_key_v1(branch)
    )
}

fn format_campaign_selection_rank_key_v1(rank_key: i32) -> String {
    let abs = rank_key.abs();
    if abs < 1_000 {
        return rank_key.to_string();
    }
    let sign = if rank_key < 0 { "-" } else { "" };
    let tenths = (abs + 50) / 100;
    format!("{sign}{}.{}k", tenths / 10, tenths % 10)
}

fn campaign_branch_active_sort_key_v1(
    branch: &BranchCampaignBranchV1,
) -> BranchCampaignActiveSortKeyV1 {
    (
        campaign_branch_boss_checkpoint_sort_key_v1(branch),
        campaign_branch_selection_rank_key_v1(branch),
        branch_progress_key(branch),
        branch_resource_conversion_key_v1(branch),
        campaign_branch_strategic_sort_key_v1(branch),
    )
}

fn campaign_branch_promotion_sort_key_v1(
    branch: &BranchCampaignBranchV1,
) -> BranchCampaignPromotionSortKeyV1 {
    (
        branch_progress_key(branch),
        campaign_branch_selection_rank_key_v1(branch),
        branch_resource_conversion_key_v1(branch),
        campaign_branch_strategic_sort_key_v1(branch),
    )
}

fn campaign_branch_selection_rank_key_v1(branch: &BranchCampaignBranchV1) -> i32 {
    branch
        .rank_key
        .saturating_add(branch_resource_conversion_key_v1(branch).saturating_mul(10))
        .saturating_add(campaign_branch_strategic_rank_adjustment_v1(branch))
}

fn campaign_branch_strategic_rank_adjustment_v1(branch: &BranchCampaignBranchV1) -> i32 {
    let signature = &branch.strategic_summary;
    if signature.is_empty() {
        return 0;
    }
    signature
        .boss_readiness_milli
        .saturating_add(signature.engine_score_milli)
        .saturating_add(signature.package_coherence_milli)
        .saturating_add(signature.economy_conversion_milli / 2)
        .saturating_sub(signature.cycle_debt_milli)
        .saturating_sub(signature.setup_debt_milli)
}

fn campaign_branch_boss_checkpoint_sort_key_v1(
    branch: &BranchCampaignBranchV1,
) -> BranchCampaignBossCheckpointSortKeyV1 {
    let Some(summary) = branch.summary.as_ref() else {
        return (0, 0, 0, 0);
    };
    if summary.floor < final_boss_checkpoint_floor_v1(summary.act) {
        return (0, 0, 0, 0);
    }
    let signature = &branch.strategic_summary;
    if signature.is_empty() {
        return (0, 0, 0, 0);
    }
    let hp_percent = if summary.max_hp > 0 {
        summary.hp.max(0).saturating_mul(100) / summary.max_hp
    } else {
        0
    };
    let debt = signature
        .cycle_debt_milli
        .saturating_add(signature.setup_debt_milli);
    (1, signature.boss_readiness_milli, hp_percent, -debt)
}

fn final_boss_checkpoint_floor_v1(act: u8) -> i32 {
    match act {
        1 => 14,
        2 => 30,
        3 => 46,
        _ => i32::MAX,
    }
}
