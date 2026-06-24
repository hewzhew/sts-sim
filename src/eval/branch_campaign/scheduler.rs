use std::collections::{BTreeMap, BTreeSet};

use super::discard_trace::{record_campaign_discard_v1, record_campaign_duplicate_merge_v1};
use super::lineage::campaign_branch_boss_relic_lineage_key_v1;
use super::model::{
    BranchCampaignBranchStatusV1, BranchCampaignBranchV1, BranchCampaignSelectionV1,
};
use super::selection_key::compare_campaign_branches_for_active_v1;
use super::{branch_progress_key, campaign_branch_quality_key_v1, BranchCampaignConfigV1};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SchedulerLaneV1 {
    BossRelicAxis,
    CoverageGapTarget,
    DecisionCandidateAxis,
    ProgressProbe,
    SurvivalProbe,
    General,
}

pub(super) fn schedule_campaign_workset_for_config_v1(
    candidates: Vec<BranchCampaignBranchV1>,
    parked: Vec<BranchCampaignBranchV1>,
    config: &BranchCampaignConfigV1,
) -> BranchCampaignSelectionV1 {
    schedule_campaign_workset_for_limits_v1(
        candidates,
        parked,
        config.max_active,
        config.max_frozen,
    )
}

pub(super) fn reschedule_campaign_existing_workset_v1(
    scheduled: Vec<BranchCampaignBranchV1>,
    parked: Vec<BranchCampaignBranchV1>,
    config: &BranchCampaignConfigV1,
) -> BranchCampaignSelectionV1 {
    schedule_campaign_workset_for_limits_v1(scheduled, parked, config.max_active, config.max_frozen)
}

pub fn select_campaign_branches_v1(
    branches: Vec<BranchCampaignBranchV1>,
    max_active: usize,
    max_frozen: usize,
) -> BranchCampaignSelectionV1 {
    schedule_campaign_workset_for_limits_v1(branches, Vec::new(), max_active, max_frozen)
}

fn schedule_campaign_workset_for_limits_v1(
    candidates: Vec<BranchCampaignBranchV1>,
    parked: Vec<BranchCampaignBranchV1>,
    max_scheduled: usize,
    max_parked: usize,
) -> BranchCampaignSelectionV1 {
    let mut selection = BranchCampaignSelectionV1::default();
    let mut pool = Vec::new();
    for branch in candidates.into_iter().chain(parked) {
        match branch.status {
            BranchCampaignBranchStatusV1::TerminalVictory => selection.victories.push(branch),
            BranchCampaignBranchStatusV1::TerminalDefeat => selection.dead.push(branch),
            BranchCampaignBranchStatusV1::Abandoned => selection.abandoned.push(branch),
            BranchCampaignBranchStatusV1::Stuck => selection.stuck.push(branch),
            BranchCampaignBranchStatusV1::Scheduled | BranchCampaignBranchStatusV1::Parked => {
                pool.push(branch)
            }
        }
    }

    let mut pool = dedup_campaign_scheduler_pool_v1(pool, &mut selection);
    let mut scheduled = Vec::new();

    take_best_per_group_v1(
        &mut pool,
        &mut scheduled,
        max_scheduled,
        SchedulerLaneV1::BossRelicAxis,
        branch_boss_relic_axis_key_v1,
    );
    take_best_per_group_v1(
        &mut pool,
        &mut scheduled,
        max_scheduled,
        SchedulerLaneV1::CoverageGapTarget,
        branch_coverage_gap_target_key_v1,
    );
    take_best_per_group_v1(
        &mut pool,
        &mut scheduled,
        max_scheduled,
        SchedulerLaneV1::DecisionCandidateAxis,
        branch_decision_candidate_axis_key_v1,
    );
    take_best_single_v1(
        &mut pool,
        &mut scheduled,
        max_scheduled,
        SchedulerLaneV1::ProgressProbe,
        |left, right| branch_progress_probe_key_v1(left).cmp(&branch_progress_probe_key_v1(right)),
    );
    take_best_single_v1(
        &mut pool,
        &mut scheduled,
        max_scheduled,
        SchedulerLaneV1::SurvivalProbe,
        |left, right| branch_hp_percent_v1(left).cmp(&branch_hp_percent_v1(right)),
    );
    take_best_general_v1(&mut pool, &mut scheduled, max_scheduled);

    for mut branch in scheduled {
        mark_scheduler_lane_v1(&mut branch, BranchCampaignBranchStatusV1::Scheduled);
        selection.scheduled.push(branch);
    }
    for mut branch in pool {
        if selection.parked.len() < max_parked {
            mark_scheduler_lane_v1(&mut branch, BranchCampaignBranchStatusV1::Parked);
            selection.parked.push(branch);
        } else {
            record_campaign_discard_v1(
                &branch,
                &mut selection.discarded_count,
                &mut selection.discarded_examples,
                &mut selection.discarded_branches,
                "scheduler_parked_capacity",
            );
        }
    }
    selection
}

fn dedup_campaign_scheduler_pool_v1(
    branches: Vec<BranchCampaignBranchV1>,
    selection: &mut BranchCampaignSelectionV1,
) -> Vec<BranchCampaignBranchV1> {
    let mut by_quality = BTreeMap::<String, BranchCampaignBranchV1>::new();
    for branch in branches {
        let key = scheduler_branch_quality_key_v1(&branch);
        match by_quality.remove(&key) {
            Some(existing) => {
                let (kept, discarded) =
                    if compare_campaign_branches_for_active_v1(&branch, &existing).is_lt() {
                        (branch, existing)
                    } else {
                        (existing, branch)
                    };
                record_campaign_duplicate_merge_v1(
                    &discarded,
                    &mut selection.discarded_count,
                    &mut selection.discarded_examples,
                    &mut selection.discarded_branches,
                );
                by_quality.insert(key, kept);
            }
            None => {
                by_quality.insert(key, branch);
            }
        }
    }
    by_quality.into_values().collect()
}

fn take_best_per_group_v1(
    pool: &mut Vec<BranchCampaignBranchV1>,
    scheduled: &mut Vec<BranchCampaignBranchV1>,
    max_scheduled: usize,
    lane: SchedulerLaneV1,
    key_fn: fn(&BranchCampaignBranchV1) -> Option<String>,
) {
    if scheduled.len() >= max_scheduled || pool.is_empty() {
        return;
    }
    sort_scheduler_pool_v1(pool);
    let mut seen = BTreeSet::<String>::new();
    let mut index = 0usize;
    while index < pool.len() && scheduled.len() < max_scheduled {
        let Some(key) = key_fn(&pool[index]) else {
            index = index.saturating_add(1);
            continue;
        };
        if !seen.insert(key) {
            index = index.saturating_add(1);
            continue;
        }
        let mut branch = pool.remove(index);
        mark_scheduler_lane_label_v1(&mut branch, lane);
        scheduled.push(branch);
    }
}

fn take_best_single_v1(
    pool: &mut Vec<BranchCampaignBranchV1>,
    scheduled: &mut Vec<BranchCampaignBranchV1>,
    max_scheduled: usize,
    lane: SchedulerLaneV1,
    cmp: fn(&BranchCampaignBranchV1, &BranchCampaignBranchV1) -> std::cmp::Ordering,
) {
    if scheduled.len() >= max_scheduled || pool.is_empty() {
        return;
    }
    let Some((index, _)) = pool.iter().enumerate().max_by(|(_, left), (_, right)| {
        cmp(left, right).then_with(|| compare_campaign_branches_for_active_v1(right, left))
    }) else {
        return;
    };
    let mut branch = pool.remove(index);
    mark_scheduler_lane_label_v1(&mut branch, lane);
    scheduled.push(branch);
}

fn take_best_general_v1(
    pool: &mut Vec<BranchCampaignBranchV1>,
    scheduled: &mut Vec<BranchCampaignBranchV1>,
    max_scheduled: usize,
) {
    sort_scheduler_pool_v1(pool);
    while scheduled.len() < max_scheduled && !pool.is_empty() {
        let mut branch = pool.remove(0);
        mark_scheduler_lane_label_v1(&mut branch, SchedulerLaneV1::General);
        scheduled.push(branch);
    }
}

fn sort_scheduler_pool_v1(pool: &mut [BranchCampaignBranchV1]) {
    pool.sort_by(compare_campaign_branches_for_active_v1);
}

fn branch_boss_relic_axis_key_v1(branch: &BranchCampaignBranchV1) -> Option<String> {
    campaign_branch_boss_relic_lineage_key_v1(branch)
}

fn branch_coverage_gap_target_key_v1(branch: &BranchCampaignBranchV1) -> Option<String> {
    let origin = branch
        .continuation_origin
        .as_ref()
        .filter(|origin| origin.kind == "coverage_gap")?;
    Some(format!(
        "{}|{}|{}|{}|{}",
        origin.source_event_id,
        origin.decision_id,
        origin.candidate_id,
        origin.target_origin_source,
        origin.milestone
    ))
}

fn branch_decision_candidate_axis_key_v1(branch: &BranchCampaignBranchV1) -> Option<String> {
    branch.decision_candidate_axis.clone()
}

fn branch_progress_probe_key_v1(branch: &BranchCampaignBranchV1) -> (u8, i32, i32) {
    let (act, floor, _) = branch_progress_key(branch);
    (act, floor, branch.rank_key)
}

fn scheduler_branch_quality_key_v1(branch: &BranchCampaignBranchV1) -> String {
    let base = campaign_branch_quality_key_v1(branch);
    match branch_coverage_gap_target_key_v1(branch) {
        Some(target) => format!("{base}|coverage_target={target}"),
        None => base,
    }
}

fn branch_hp_percent_v1(branch: &BranchCampaignBranchV1) -> i32 {
    let Some(summary) = branch.summary.as_ref() else {
        return 0;
    };
    if summary.max_hp <= 0 {
        return 0;
    }
    summary.hp.max(0).saturating_mul(100) / summary.max_hp
}

fn mark_scheduler_lane_v1(
    branch: &mut BranchCampaignBranchV1,
    status: BranchCampaignBranchStatusV1,
) {
    branch.status = status;
}

fn mark_scheduler_lane_label_v1(branch: &mut BranchCampaignBranchV1, lane: SchedulerLaneV1) {
    let label = match lane {
        SchedulerLaneV1::BossRelicAxis => "scheduler:boss_relic_axis",
        SchedulerLaneV1::CoverageGapTarget => "scheduler:coverage_gap_target",
        SchedulerLaneV1::DecisionCandidateAxis => "scheduler:decision_candidate_axis",
        SchedulerLaneV1::ProgressProbe => "scheduler:progress_probe",
        SchedulerLaneV1::SurvivalProbe => "scheduler:survival_probe",
        SchedulerLaneV1::General => "scheduler:general",
    };
    if branch.stop_reason.trim().is_empty() {
        branch.stop_reason = label.to_string();
    } else if !branch.stop_reason.contains(label) {
        branch.stop_reason = format!("{} | {label}", branch.stop_reason);
    }
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
