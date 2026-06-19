use std::collections::BTreeMap;

use super::model::{BranchCampaignBranchStatusV1, BranchCampaignBranchV1};
use super::selection_key::{
    act_boss_floor_v1, campaign_branch_retention_key_v1, compare_campaign_branches_for_active_v1,
    compare_campaign_branches_for_promotion_v1,
};
use super::{branch_progress_key, normalized_campaign_boundary_title};

const PROGRESS_ANCHOR_MAX_RANK_LAG: i32 = 1_000;
const SURVIVAL_ANCHOR_LOW_HP_PERCENT: i32 = 25;
const SURVIVAL_ANCHOR_NEARBY_MIN_HP_GAIN: i32 = 20;
const SURVIVAL_ANCHOR_HEALTHY_SALVAGE_HP_PERCENT: i32 = 50;
const SURVIVAL_ANCHOR_HEALTHY_SALVAGE_HP_GAIN: i32 = 40;
const SURVIVAL_ANCHOR_CRITICAL_HP_PERCENT: i32 = 15;
const SURVIVAL_ANCHOR_CRITICAL_SALVAGE_HP_PERCENT: i32 = 30;
const SURVIVAL_ANCHOR_CRITICAL_SALVAGE_HP_GAIN: i32 = 25;

pub(super) fn rebalance_active_progress_anchor_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
) -> bool {
    if active.len() < 2 || frozen.is_empty() {
        return false;
    }

    let Some((frozen_index, frozen_branch)) =
        frozen.iter().enumerate().max_by(|(_, left), (_, right)| {
            branch_progress_key(left)
                .cmp(&branch_progress_key(right))
                .then_with(|| compare_campaign_branches_for_active_v1(left, right).reverse())
        })
    else {
        return false;
    };
    let frozen_progress = branch_progress_key(frozen_branch);

    let duplicate_keys = active
        .iter()
        .map(campaign_branch_local_frontier_key_v1)
        .fold(BTreeMap::<String, usize>::new(), |mut counts, key| {
            *counts.entry(key).or_default() += 1;
            counts
        });

    let Some((replace_index, _)) = active
        .iter()
        .enumerate()
        .filter(|(_, branch)| {
            duplicate_keys
                .get(&campaign_branch_local_frontier_key_v1(branch))
                .copied()
                .unwrap_or(0)
                > 1
                && campaign_progress_is_clearly_ahead_v1(
                    frozen_progress,
                    branch_progress_key(branch),
                )
                && campaign_active_swap_respects_survival_v1(frozen_branch, branch)
                && campaign_progress_anchor_rank_close_enough_v1(frozen_branch, branch)
        })
        .max_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
    else {
        return false;
    };

    let mut promoted = frozen.remove(frozen_index);
    promoted.status = BranchCampaignBranchStatusV1::Active;
    let mut demoted = std::mem::replace(&mut active[replace_index], promoted);
    demoted.status = BranchCampaignBranchStatusV1::Frozen;
    frozen.push(demoted);
    active.sort_by(compare_campaign_branches_for_active_v1);
    frozen.sort_by(compare_campaign_branches_for_promotion_v1);
    true
}

fn campaign_progress_anchor_rank_close_enough_v1(
    candidate: &BranchCampaignBranchV1,
    replaced: &BranchCampaignBranchV1,
) -> bool {
    candidate
        .rank_key
        .saturating_add(PROGRESS_ANCHOR_MAX_RANK_LAG)
        >= replaced.rank_key
}

pub(super) fn rebalance_active_survival_anchor_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
) -> bool {
    if active.is_empty() || frozen.is_empty() {
        return false;
    }

    let Some((replace_index, replace_hp)) = active
        .iter()
        .enumerate()
        .filter_map(|(idx, branch)| campaign_branch_hp_percent_v1(branch).map(|hp| (idx, hp)))
        .filter(|(_, hp)| *hp < SURVIVAL_ANCHOR_LOW_HP_PERCENT)
        .min_by_key(|(_, hp)| *hp)
    else {
        return false;
    };
    if campaign_branch_is_act_clear_transition_v1(&active[replace_index]) {
        return false;
    }

    let maybe_nearby = frozen
        .iter()
        .enumerate()
        .filter(|(_, branch)| !branch_is_rehydrated_checkpointed_combat_failure_v1(branch))
        .filter(|(_, branch)| {
            campaign_progress_is_nearby_v1(
                branch_progress_key(branch),
                branch_progress_key(&active[replace_index]),
            )
        })
        .filter_map(|(idx, branch)| {
            let hp = campaign_branch_hp_percent_v1(branch)?;
            (hp >= replace_hp.saturating_add(SURVIVAL_ANCHOR_NEARBY_MIN_HP_GAIN)
                && campaign_survival_anchor_respects_low_max_hp_risk_v1(
                    branch,
                    &active[replace_index],
                ))
            .then_some((idx, hp))
        })
        .max_by(|(left_idx, left_hp), (right_idx, right_hp)| {
            left_hp.cmp(right_hp).then_with(|| {
                campaign_branch_retention_key_v1(&frozen[*left_idx])
                    .cmp(&campaign_branch_retention_key_v1(&frozen[*right_idx]))
            })
        });

    let maybe_salvage = || {
        frozen
            .iter()
            .enumerate()
            .filter(|(_, branch)| !branch_is_rehydrated_checkpointed_combat_failure_v1(branch))
            .filter(|(_, branch)| {
                campaign_progress_is_survival_salvage_checkpoint_v1(
                    branch_progress_key(branch),
                    branch_progress_key(&active[replace_index]),
                )
            })
            .filter_map(|(idx, branch)| {
                let hp = campaign_branch_hp_percent_v1(branch)?;
                (campaign_branch_is_survival_salvage_v1(hp, replace_hp)
                    && campaign_survival_anchor_respects_low_max_hp_risk_v1(
                        branch,
                        &active[replace_index],
                    ))
                .then_some((idx, hp))
            })
            .max_by(|(left_idx, left_hp), (right_idx, right_hp)| {
                left_hp.cmp(right_hp).then_with(|| {
                    campaign_branch_retention_key_v1(&frozen[*left_idx])
                        .cmp(&campaign_branch_retention_key_v1(&frozen[*right_idx]))
                })
            })
    };

    let Some((frozen_index, _)) = maybe_nearby.or_else(maybe_salvage) else {
        return false;
    };

    let mut promoted = frozen.remove(frozen_index);
    promoted.status = BranchCampaignBranchStatusV1::Active;
    let mut demoted = std::mem::replace(&mut active[replace_index], promoted);
    demoted.status = BranchCampaignBranchStatusV1::Frozen;
    frozen.push(demoted);
    active.sort_by(compare_campaign_branches_for_active_v1);
    frozen.sort_by(compare_campaign_branches_for_promotion_v1);
    true
}

fn campaign_branch_is_survival_salvage_v1(candidate_hp: i32, replaced_hp: i32) -> bool {
    let healthy_salvage = candidate_hp >= SURVIVAL_ANCHOR_HEALTHY_SALVAGE_HP_PERCENT
        && candidate_hp >= replaced_hp.saturating_add(SURVIVAL_ANCHOR_HEALTHY_SALVAGE_HP_GAIN);
    let critical_salvage = replaced_hp < SURVIVAL_ANCHOR_CRITICAL_HP_PERCENT
        && candidate_hp >= SURVIVAL_ANCHOR_CRITICAL_SALVAGE_HP_PERCENT
        && candidate_hp >= replaced_hp.saturating_add(SURVIVAL_ANCHOR_CRITICAL_SALVAGE_HP_GAIN);
    healthy_salvage || critical_salvage
}

fn campaign_survival_anchor_respects_low_max_hp_risk_v1(
    candidate: &BranchCampaignBranchV1,
    replaced: &BranchCampaignBranchV1,
) -> bool {
    let (Some(candidate_summary), Some(replaced_summary)) =
        (candidate.summary.as_ref(), replaced.summary.as_ref())
    else {
        return true;
    };
    let candidate_max_hp = candidate_summary.max_hp.max(0);
    let replaced_max_hp = replaced_summary.max_hp.max(0);
    if candidate_max_hp == 0 || replaced_max_hp == 0 {
        return true;
    }
    if candidate_max_hp.saturating_mul(3) >= replaced_max_hp.saturating_mul(2) {
        return true;
    }
    candidate
        .rank_key
        .saturating_add(PROGRESS_ANCHOR_MAX_RANK_LAG)
        >= replaced.rank_key
}

fn campaign_branch_local_frontier_key_v1(branch: &BranchCampaignBranchV1) -> String {
    let (act, floor, _) = branch_progress_key(branch);
    format!(
        "a{act}f{floor}|{}",
        normalized_campaign_boundary_title(&branch.frontier_title)
    )
}

pub(super) fn campaign_progress_is_clearly_ahead_v1(
    left: (u8, i32, i32),
    right: (u8, i32, i32),
) -> bool {
    if left.0 > right.0 {
        return true;
    }
    left.0 == right.0 && left.1 >= right.1.saturating_add(2)
}

fn campaign_active_swap_respects_survival_v1(
    candidate: &BranchCampaignBranchV1,
    replaced: &BranchCampaignBranchV1,
) -> bool {
    if campaign_branch_is_act_clear_transition_v1(candidate)
        && !campaign_branch_is_act_clear_transition_v1(replaced)
    {
        return true;
    }

    let Some(candidate_hp_percent) = campaign_branch_hp_percent_v1(candidate) else {
        return true;
    };
    let Some(replaced_hp_percent) = campaign_branch_hp_percent_v1(replaced) else {
        return true;
    };
    let candidate_progress = branch_progress_key(candidate);
    let replaced_progress = branch_progress_key(replaced);
    if candidate_progress.0 == replaced_progress.0
        && candidate_progress.1 >= replaced_progress.1
        && candidate_progress.1.saturating_sub(replaced_progress.1) <= 8
        && candidate_hp_percent < SURVIVAL_ANCHOR_LOW_HP_PERCENT
        && campaign_branch_is_survival_salvage_v1(replaced_hp_percent, candidate_hp_percent)
    {
        return false;
    }
    if !campaign_progress_is_nearby_v1(candidate_progress, replaced_progress) {
        return true;
    }
    !(candidate_hp_percent < SURVIVAL_ANCHOR_LOW_HP_PERCENT
        && replaced_hp_percent
            >= candidate_hp_percent.saturating_add(SURVIVAL_ANCHOR_NEARBY_MIN_HP_GAIN))
}

fn campaign_branch_is_act_clear_transition_v1(branch: &BranchCampaignBranchV1) -> bool {
    let Some(summary) = branch.summary.as_ref() else {
        return false;
    };
    summary.act < 3
        && summary.floor >= act_boss_floor_v1(summary.act)
        && matches!(
            normalized_campaign_boundary_title(&branch.frontier_title).as_str(),
            "bossrelic" | "rewardscreen"
        )
}

fn campaign_progress_is_nearby_v1(left: (u8, i32, i32), right: (u8, i32, i32)) -> bool {
    left.0 == right.0 && (left.1 - right.1).abs() <= 2
}

fn campaign_progress_is_survival_salvage_checkpoint_v1(
    candidate: (u8, i32, i32),
    replaced: (u8, i32, i32),
) -> bool {
    candidate.0 == replaced.0
        && candidate.1 <= replaced.1
        && replaced.1.saturating_sub(candidate.1) <= 8
}

fn campaign_branch_hp_percent_v1(branch: &BranchCampaignBranchV1) -> Option<i32> {
    let summary = branch.summary.as_ref()?;
    if summary.max_hp <= 0 {
        return None;
    }
    Some(summary.hp.max(0).saturating_mul(100) / summary.max_hp)
}

pub(super) fn promote_frozen_to_active_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    max_active: usize,
) -> usize {
    frozen.sort_by(compare_campaign_branches_for_promotion_v1);
    let mut promoted = 0usize;
    while active.len() < max_active && !frozen.is_empty() {
        let require_primary_eligible = !active.is_empty();
        let Some(promote_index) = frozen.iter().position(|branch| {
            !branch_is_rehydrated_checkpointed_combat_failure_v1(branch)
                && (!require_primary_eligible || campaign_branch_primary_active_eligible_v1(branch))
        }) else {
            break;
        };
        let mut branch = frozen.remove(promote_index);
        branch.status = BranchCampaignBranchStatusV1::Active;
        active.push(branch);
        promoted = promoted.saturating_add(1);
    }
    promoted
}

pub(super) fn campaign_branch_primary_active_eligible_v1(branch: &BranchCampaignBranchV1) -> bool {
    branch.rank_key >= 0
}

pub(super) fn promote_rehydrated_combat_failures_to_active_on_stall_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    max_active: usize,
) -> usize {
    if max_active == 0 || !active.is_empty() {
        return 0;
    }
    frozen.sort_by(compare_campaign_branches_for_promotion_v1);
    let mut promoted = 0usize;
    while active.len() < max_active {
        let Some(promote_index) = frozen
            .iter()
            .position(branch_is_rehydrated_checkpointed_combat_failure_v1)
        else {
            break;
        };
        let mut branch = frozen.remove(promote_index);
        branch.status = BranchCampaignBranchStatusV1::Active;
        active.push(branch);
        promoted = promoted.saturating_add(1);
    }
    promoted
}

pub(super) fn rebalance_active_with_stronger_frozen_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    max_active: usize,
) -> usize {
    let mut total = 0usize;
    let max_iterations = active.len().saturating_add(frozen.len()).saturating_add(1);
    for _ in 0..max_iterations {
        let promoted = rebalance_active_with_stronger_frozen_once_v1(active, frozen, max_active);
        if promoted == 0 {
            break;
        }
        total = total.saturating_add(promoted);
    }
    total
}

fn rebalance_active_with_stronger_frozen_once_v1(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    max_active: usize,
) -> usize {
    if max_active == 0 || frozen.is_empty() {
        return 0;
    }
    if active.len() < max_active {
        return promote_frozen_to_active_v1(active, frozen, max_active);
    }
    if active.is_empty() {
        return 0;
    }

    if rebalance_active_survival_anchor_v1(active, frozen) {
        return 1;
    }

    let Some((worst_active_index, worst_active)) = active
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
    else {
        return 0;
    };
    let Some((best_frozen_index, best_frozen)) = frozen
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| compare_campaign_branches_for_active_v1(left, right))
    else {
        return 0;
    };

    if branch_is_rehydrated_checkpointed_combat_failure_v1(best_frozen)
        && active.iter().any(|branch| {
            campaign_progress_is_clearly_ahead_v1(
                branch_progress_key(branch),
                branch_progress_key(best_frozen),
            )
        })
    {
        return 0;
    }

    if active.iter().any(|branch| {
        campaign_branch_local_frontier_key_v1(branch)
            == campaign_branch_local_frontier_key_v1(best_frozen)
    }) && active.iter().any(|branch| {
        campaign_progress_is_clearly_ahead_v1(
            branch_progress_key(branch),
            branch_progress_key(best_frozen),
        )
    }) {
        return 0;
    }

    if !campaign_active_swap_respects_survival_v1(best_frozen, worst_active) {
        return 0;
    }

    if compare_campaign_branches_for_active_v1(best_frozen, worst_active)
        != std::cmp::Ordering::Less
    {
        return 0;
    }

    let mut promoted = frozen.remove(best_frozen_index);
    promoted.status = BranchCampaignBranchStatusV1::Active;
    let mut demoted = std::mem::replace(&mut active[worst_active_index], promoted);
    demoted.status = BranchCampaignBranchStatusV1::Frozen;
    frozen.push(demoted);
    active.sort_by(compare_campaign_branches_for_active_v1);
    frozen.sort_by(compare_campaign_branches_for_promotion_v1);
    1
}

pub(super) fn branch_is_rehydrated_checkpointed_combat_failure_v1(
    branch: &BranchCampaignBranchV1,
) -> bool {
    normalized_campaign_boundary_title(&branch.frontier_title).starts_with("combat")
        && branch
            .stop_reason
            .to_ascii_lowercase()
            .contains("rehydrated checkpointed")
}
