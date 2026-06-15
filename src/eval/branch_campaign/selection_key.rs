use std::cmp::Ordering;

use super::{branch_progress_key, BranchCampaignBranchV1};

type BranchCampaignBossCheckpointSortKeyV1 = (u8, i32, i32, i32);
type BranchCampaignActClearSortKeyV1 = (u8, i32, i32);
type BranchCampaignActiveSortKeyV1 = (
    BranchCampaignActClearSortKeyV1,
    BranchCampaignBossCheckpointSortKeyV1,
    i32,
    i32,
    i32,
    (u8, i32, i32),
);
type BranchCampaignPromotionSortKeyV1 = ((u8, i32, i32), i32);
type BranchCampaignRetentionKeyV1 = (u8, i32, i32, i32);

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
    )
}

pub(super) fn render_campaign_branch_selection_basis_v1(branch: &BranchCampaignBranchV1) -> String {
    let lineage = if branch.lineage_decision_signal_rank_adjustment == 0 {
        String::new()
    } else {
        format!(
            " lineage_signal={}",
            format_campaign_selection_rank_key_v1(branch.lineage_decision_signal_rank_adjustment)
        )
    };
    format!(
        "sel=[retention_rank={}{}]",
        format_campaign_selection_rank_key_v1(campaign_branch_selection_rank_key_v1(branch)),
        lineage,
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
        campaign_branch_act_clear_sort_key_v1(branch),
        campaign_branch_boss_checkpoint_sort_key_v1(branch),
        campaign_branch_primary_eligible_key_v1(branch),
        campaign_branch_selection_rank_bucket_v1(branch),
        campaign_branch_selection_rank_key_v1(branch),
        branch_progress_key(branch),
    )
}

fn campaign_branch_promotion_sort_key_v1(
    branch: &BranchCampaignBranchV1,
) -> BranchCampaignPromotionSortKeyV1 {
    (
        branch_progress_key(branch),
        campaign_branch_selection_rank_key_v1(branch),
    )
}

fn campaign_branch_selection_rank_key_v1(branch: &BranchCampaignBranchV1) -> i32 {
    branch.rank_key
}

fn campaign_branch_selection_rank_bucket_v1(branch: &BranchCampaignBranchV1) -> i32 {
    campaign_branch_selection_rank_key_v1(branch).div_euclid(1_000)
}

fn campaign_branch_primary_eligible_key_v1(branch: &BranchCampaignBranchV1) -> i32 {
    i32::from(branch.rank_key >= 0)
}

fn campaign_branch_boss_checkpoint_sort_key_v1(
    branch: &BranchCampaignBranchV1,
) -> BranchCampaignBossCheckpointSortKeyV1 {
    let Some(summary) = branch.summary.as_ref() else {
        return (0, 0, 0, 0);
    };
    let frontier = normalized_frontier_title_v1(&branch.frontier_title);
    if summary.act < 3
        && summary.floor >= act_boss_floor_v1(summary.act)
        && matches!(frontier.as_str(), "bossrelic" | "rewardscreen")
    {
        return (0, 0, 0, 0);
    }
    if summary.floor < boss_readiness_checkpoint_floor_v1(summary.act) {
        return (0, 0, 0, 0);
    }
    let signature = &branch.strategic_summary;
    if signature.is_empty() {
        return (0, 0, 0, 0);
    }
    if signature.boss_readiness_milli <= 0 {
        return (0, 0, 0, 0);
    }
    // Strategic summaries are diagnostic during ordinary campaign selection.
    // Only final boss checkpoints consume them as an explicit stage gate.
    let hp_percent = if summary.max_hp > 0 {
        summary.hp.max(0).saturating_mul(100) / summary.max_hp
    } else {
        0
    };
    let hp_bucket = hp_percent.div_euclid(10);
    let debt = signature
        .cycle_debt_milli
        .saturating_add(signature.setup_debt_milli);
    (1, signature.boss_readiness_milli, hp_bucket, -debt)
}

fn campaign_branch_act_clear_sort_key_v1(
    branch: &BranchCampaignBranchV1,
) -> BranchCampaignActClearSortKeyV1 {
    let Some(summary) = branch.summary.as_ref() else {
        return (0, 0, 0);
    };
    let frontier = normalized_frontier_title_v1(&branch.frontier_title);
    let is_act_clear_transition = summary.act < 3
        && summary.floor >= act_boss_floor_v1(summary.act)
        && matches!(frontier.as_str(), "bossrelic" | "rewardscreen");
    if !is_act_clear_transition {
        return (summary.act, 0, 0);
    }
    // Once an act boss has actually been cleared, campaign selection should keep
    // processing the reward/relic transition instead of revisiting unproven
    // pre-boss checkpoints. Treat that transition as the start of the next act,
    // but do not let it outrank branches that have already advanced deeper into
    // that next act.
    (summary.act.saturating_add(1), 0, 0)
}

pub(super) fn act_boss_floor_v1(act: u8) -> i32 {
    final_boss_checkpoint_floor_v1(act).saturating_add(2)
}

fn final_boss_checkpoint_floor_v1(act: u8) -> i32 {
    match act {
        1 => 14,
        2 => 30,
        3 => 46,
        _ => i32::MAX,
    }
}

fn boss_readiness_checkpoint_floor_v1(act: u8) -> i32 {
    match act {
        1 => 10,
        2 => 24,
        3 => 40,
        _ => i32::MAX,
    }
}

fn normalized_frontier_title_v1(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}
