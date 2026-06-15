use super::{
    branch_is_rehydrated_checkpointed_combat_failure_v1, branch_progress_key,
    campaign_active_swap_respects_survival_v1, campaign_branch_local_frontier_key_v1,
    campaign_progress_is_clearly_ahead_v1, compare_campaign_branches_for_active_v1,
    compare_campaign_branches_for_promotion_v1, BranchCampaignBranchStatusV1,
    BranchCampaignBranchV1,
};

const THAW_MAX_ACT_FLOOR_LAG: i32 = 3;
const THAW_MAX_RANK_LAG: i32 = 1_500;
const THAW_MIN_STRUCTURAL_SIGNAL: i32 = 1_800;
const THAW_MIN_SIGNAL_MARGIN: i32 = 700;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct BranchCampaignThawResultV0 {
    pub promoted: usize,
}

pub(crate) fn thaw_promising_frozen_v0(
    active: &mut Vec<BranchCampaignBranchV1>,
    frozen: &mut Vec<BranchCampaignBranchV1>,
    max_active: usize,
) -> BranchCampaignThawResultV0 {
    if max_active == 0 || active.len() < max_active || active.is_empty() || frozen.is_empty() {
        return BranchCampaignThawResultV0::default();
    }

    let Some((active_index, frozen_index)) = best_thaw_swap_v0(active, frozen) else {
        return BranchCampaignThawResultV0::default();
    };

    let mut promoted = frozen.remove(frozen_index);
    promoted.status = BranchCampaignBranchStatusV1::Active;
    let mut demoted = std::mem::replace(&mut active[active_index], promoted);
    demoted.status = BranchCampaignBranchStatusV1::Frozen;
    frozen.push(demoted);

    active.sort_by(compare_campaign_branches_for_active_v1);
    frozen.sort_by(compare_campaign_branches_for_promotion_v1);
    BranchCampaignThawResultV0 { promoted: 1 }
}

fn best_thaw_swap_v0(
    active: &[BranchCampaignBranchV1],
    frozen: &[BranchCampaignBranchV1],
) -> Option<(usize, usize)> {
    let mut best_swap: Option<(usize, usize, i32)> = None;
    for (frozen_index, candidate) in frozen.iter().enumerate() {
        if branch_is_rehydrated_checkpointed_combat_failure_v1(candidate) {
            continue;
        }
        for (active_index, replaced) in active.iter().enumerate() {
            let Some(margin) = thaw_swap_margin_v0(candidate, replaced, active) else {
                continue;
            };
            if best_swap
                .as_ref()
                .map(|(_, _, best_margin)| margin > *best_margin)
                .unwrap_or(true)
            {
                best_swap = Some((active_index, frozen_index, margin));
            }
        }
    }
    best_swap.map(|(active_index, frozen_index, _)| (active_index, frozen_index))
}

fn thaw_swap_margin_v0(
    candidate: &BranchCampaignBranchV1,
    replaced: &BranchCampaignBranchV1,
    active: &[BranchCampaignBranchV1],
) -> Option<i32> {
    if !candidate_is_near_active_frontier_v0(candidate, active) {
        return None;
    }
    if !candidate_rank_is_close_enough_v0(candidate, replaced) {
        return None;
    }
    if active.iter().any(|branch| {
        campaign_branch_local_frontier_key_v1(branch)
            == campaign_branch_local_frontier_key_v1(candidate)
            && campaign_progress_is_clearly_ahead_v1(
                branch_progress_key(branch),
                branch_progress_key(candidate),
            )
    }) {
        return None;
    }
    if !campaign_active_swap_respects_survival_v1(candidate, replaced) {
        return None;
    }

    let candidate_signal = structural_thaw_signal_v0(candidate)?;
    let replaced_signal = structural_thaw_signal_v0(replaced).unwrap_or_default();
    if candidate_signal < THAW_MIN_STRUCTURAL_SIGNAL {
        return None;
    }
    let signal_margin = candidate_signal.saturating_sub(replaced_signal);
    if signal_margin < THAW_MIN_SIGNAL_MARGIN {
        return None;
    }
    Some(signal_margin.saturating_add(candidate.rank_key.saturating_sub(replaced.rank_key) / 10))
}

fn candidate_is_near_active_frontier_v0(
    candidate: &BranchCampaignBranchV1,
    active: &[BranchCampaignBranchV1],
) -> bool {
    let (candidate_act, candidate_floor, _) = branch_progress_key(candidate);
    let Some((front_act, front_floor, _)) = active
        .iter()
        .map(branch_progress_key)
        .max_by(|left, right| left.cmp(right))
    else {
        return false;
    };
    if candidate_act < front_act {
        return false;
    }
    if candidate_act > front_act {
        return true;
    }
    candidate_floor.saturating_add(THAW_MAX_ACT_FLOOR_LAG) >= front_floor
}

fn candidate_rank_is_close_enough_v0(
    candidate: &BranchCampaignBranchV1,
    replaced: &BranchCampaignBranchV1,
) -> bool {
    candidate.rank_key.saturating_add(THAW_MAX_RANK_LAG) >= replaced.rank_key
}

fn structural_thaw_signal_v0(branch: &BranchCampaignBranchV1) -> Option<i32> {
    let signature = branch.strategic_summary;
    if signature.is_empty() {
        return None;
    }
    let debt = signature
        .cycle_debt_milli
        .saturating_add(signature.setup_debt_milli);
    Some(
        signature
            .boss_readiness_milli
            .saturating_mul(2)
            .saturating_add(signature.clean_score_milli)
            .saturating_add(signature.package_coherence_milli)
            .saturating_add(signature.engine_score_milli / 2)
            .saturating_sub(debt),
    )
}
