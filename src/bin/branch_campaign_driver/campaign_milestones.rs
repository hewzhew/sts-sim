use sts_simulator::eval::branch_campaign::{BranchCampaignBranchV1, BranchCampaignReportV1};

use super::command_inputs::{CampaignMilestoneStopV1, CampaignMilestoneTargetV1};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CampaignMilestoneStatusV1 {
    pub(super) reached: bool,
    pub(super) furthest_act: u8,
    pub(super) furthest_floor: i32,
    pub(super) hit_count: usize,
}

pub(super) fn resolve_campaign_milestone_target_v1(
    target: CampaignMilestoneTargetV1,
    report: &BranchCampaignReportV1,
) -> CampaignMilestoneTargetV1 {
    if target != CampaignMilestoneTargetV1::CurrentActBoss {
        return target;
    }
    let status = campaign_milestone_status_v1(report, CampaignMilestoneTargetV1::Act3Boss);
    match status.furthest_act {
        0 | 1 => CampaignMilestoneTargetV1::Act1Boss,
        2 => CampaignMilestoneTargetV1::Act2Boss,
        _ => CampaignMilestoneTargetV1::Act3Boss,
    }
}

pub(super) fn campaign_milestone_status_v1(
    report: &BranchCampaignReportV1,
    target: CampaignMilestoneTargetV1,
) -> CampaignMilestoneStatusV1 {
    let mut status = CampaignMilestoneStatusV1 {
        reached: false,
        furthest_act: 0,
        furthest_floor: 0,
        hit_count: 0,
    };
    for branch in report
        .active
        .iter()
        .chain(report.frozen.iter())
        .chain(report.stuck.iter())
        .chain(report.victories.iter())
        .chain(report.dead.iter())
        .chain(report.abandoned.iter())
    {
        update_milestone_status_from_branch_v1(&mut status, target, branch);
    }
    status.reached = status.hit_count > 0;
    status
}

pub(super) fn render_campaign_milestone_status_v1(
    target: CampaignMilestoneTargetV1,
    stop: CampaignMilestoneStopV1,
    status: CampaignMilestoneStatusV1,
    spent_rounds: usize,
    max_rounds: usize,
) -> String {
    format!(
        "MilestoneStatusV1 target={} stop={} reached={} hits={} furthest=A{}F{} spent_rounds={} cap={}",
        target.as_str(),
        stop.as_str(),
        status.reached,
        status.hit_count,
        status.furthest_act,
        status.furthest_floor,
        spent_rounds,
        max_rounds
    )
}

fn update_milestone_status_from_branch_v1(
    status: &mut CampaignMilestoneStatusV1,
    target: CampaignMilestoneTargetV1,
    branch: &BranchCampaignBranchV1,
) {
    let Some(summary) = branch.summary.as_ref() else {
        return;
    };
    if summary.act > status.furthest_act
        || (summary.act == status.furthest_act && summary.floor > status.furthest_floor)
    {
        status.furthest_act = summary.act;
        status.furthest_floor = summary.floor;
    }
    if campaign_milestone_hit_v1(summary.act, summary.floor, target) {
        status.hit_count += 1;
    }
}

fn campaign_milestone_hit_v1(act: u8, floor: i32, target: CampaignMilestoneTargetV1) -> bool {
    match target {
        CampaignMilestoneTargetV1::Act1Boss => act > 1 || (act == 1 && floor >= 16),
        CampaignMilestoneTargetV1::Act2Start => act >= 2,
        CampaignMilestoneTargetV1::Act2Boss => act > 2 || (act == 2 && floor >= 32),
        CampaignMilestoneTargetV1::Act3Boss => act > 3 || (act == 3 && floor >= 48),
        CampaignMilestoneTargetV1::CurrentActBoss => false,
    }
}
