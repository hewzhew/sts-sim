use std::cmp::Ordering;
use std::collections::BTreeMap;

use sts_simulator::eval::branch_campaign::{
    BranchCampaignBranchSummaryV1, BranchCampaignBranchV1, BranchCampaignDiscardedBranchV1,
    BranchCampaignReportV1,
};

use super::campaign_artifacts::read_campaign_report_v1;
use super::command_inputs::InspectCommandInput;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CoverageGapMilestoneTargetV1 {
    Act1Boss,
    Act2Start,
}

impl CoverageGapMilestoneTargetV1 {
    pub(super) fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().replace('-', "_").as_str() {
            "act1boss" | "act_1_boss" | "act1_boss" => Ok(Self::Act1Boss),
            "act2start" | "act_2_start" | "act2_start" => Ok(Self::Act2Start),
            _ => Err(format!(
                "invalid coverage gap milestone target `{value}`; expected Act1Boss or Act2Start"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Act1Boss => "Act1Boss",
            Self::Act2Start => "Act2Start",
        }
    }

    fn is_reached_by(self, row: &CoverageGapMilestoneBranchRowV1) -> bool {
        match self {
            Self::Act1Boss => row.act > 1 || (row.act == 1 && row.floor >= 16),
            Self::Act2Start => row.act >= 2,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CoverageGapMilestoneBranchRowV1 {
    pub(super) bucket: String,
    pub(super) event_type: String,
    pub(super) label: String,
    pub(super) command: String,
    pub(super) act: u8,
    pub(super) floor: i32,
    pub(super) hp: i32,
    pub(super) max_hp: i32,
    pub(super) deck_count: usize,
    pub(super) target_origin_source: String,
    pub(super) target_lane: String,
    pub(super) target_progress: String,
    pub(super) frontier_title: String,
    pub(super) stop_reason: String,
    pub(super) choice_labels: Vec<String>,
}

#[derive(Default)]
struct CoverageGapMilestoneOriginSummaryV1<'a> {
    total: usize,
    reached: usize,
    active: usize,
    frozen: usize,
    stuck: usize,
    abandoned: usize,
    dead: usize,
    victories: usize,
    discarded: usize,
    other: usize,
    furthest: Option<&'a CoverageGapMilestoneBranchRowV1>,
}

pub(super) fn run_coverage_gap_milestone_summary_inspection(
    input: &InspectCommandInput,
) -> Result<(), String> {
    let report_path = input.report_path.as_ref().ok_or_else(|| {
        "--inspect-coverage-gap-milestone-summary requires --inspect-report PATH".to_string()
    })?;
    let target = CoverageGapMilestoneTargetV1::parse(&input.coverage_gap_milestone_target)?;
    let report = read_campaign_report_v1(report_path)?;
    println!(
        "{}",
        render_coverage_gap_milestone_summary_v1(&report, target)
    );
    Ok(())
}

pub(super) fn render_coverage_gap_milestone_summary_v1(
    report: &BranchCampaignReportV1,
    target: CoverageGapMilestoneTargetV1,
) -> String {
    let rows = coverage_gap_milestone_rows_from_report_v1(report);
    render_coverage_gap_milestone_summary_from_rows_v1(&rows, target)
}

pub(super) fn render_coverage_gap_milestone_summary_from_rows_v1(
    rows: &[CoverageGapMilestoneBranchRowV1],
    target: CoverageGapMilestoneTargetV1,
) -> String {
    let reached = rows.iter().filter(|row| target.is_reached_by(row)).count();
    let mut lines = Vec::new();
    lines.push(format!(
        "CoverageGapMilestoneSummaryV1 target={} total={} reached={} not_reached={}",
        target.as_str(),
        rows.len(),
        reached,
        rows.len().saturating_sub(reached)
    ));
    if rows.is_empty() {
        return lines.join("\n");
    }

    let mut by_origin: BTreeMap<&str, CoverageGapMilestoneOriginSummaryV1<'_>> = BTreeMap::new();
    for row in rows {
        let summary = by_origin.entry(&row.event_type).or_default();
        summary.total += 1;
        if target.is_reached_by(row) {
            summary.reached += 1;
        }
        match row.bucket.as_str() {
            "active" => summary.active += 1,
            "frozen" => summary.frozen += 1,
            "stuck" => summary.stuck += 1,
            "abandoned" => summary.abandoned += 1,
            "dead" => summary.dead += 1,
            "victories" => summary.victories += 1,
            "discarded" => summary.discarded += 1,
            _ => summary.other += 1,
        }
        if summary
            .furthest
            .is_none_or(|current| compare_milestone_rows(row, current) == Ordering::Greater)
        {
            summary.furthest = Some(row);
        }
    }

    lines.push("By origin:".to_string());
    for (event_type, summary) in by_origin {
        let furthest = summary
            .furthest
            .map(format_milestone_row_v1)
            .unwrap_or_else(|| "-".to_string());
        lines.push(format!(
            "  {event_type} total={} reached={} active={} frozen={} stuck={} abandoned={} dead={} victories={} discarded={} other={} furthest={}",
            summary.total,
            summary.reached,
            summary.active,
            summary.frozen,
            summary.stuck,
            summary.abandoned,
            summary.dead,
            summary.victories,
            summary.discarded,
            summary.other,
            furthest
        ));
    }

    let progress_counts = target_progress_counts_v1(rows.iter());
    lines.push(format!(
        "Target progress: extended={} target_only={} discarded={} missing={} incomplete={} unknown={}",
        count_for_key_v1(&progress_counts, "extended"),
        count_for_key_v1(&progress_counts, "target_only"),
        count_for_key_v1(&progress_counts, "discarded"),
        count_for_key_v1(&progress_counts, "missing"),
        count_for_key_v1(&progress_counts, "incomplete"),
        count_for_key_v1(&progress_counts, "unknown")
    ));
    let mut by_origin_progress = BTreeMap::<&str, BTreeMap<&str, usize>>::new();
    for row in rows {
        *by_origin_progress
            .entry(&row.event_type)
            .or_default()
            .entry(&row.target_progress)
            .or_default() += 1;
    }
    lines.push("Target progress by origin:".to_string());
    for (event_type, counts) in by_origin_progress {
        lines.push(format!(
            "  {event_type} extended={} target_only={} discarded={} missing={} incomplete={} unknown={}",
            count_for_key_v1(&counts, "extended"),
            count_for_key_v1(&counts, "target_only"),
            count_for_key_v1(&counts, "discarded"),
            count_for_key_v1(&counts, "missing"),
            count_for_key_v1(&counts, "incomplete"),
            count_for_key_v1(&counts, "unknown")
        ));
    }

    let mut by_lane_progress = BTreeMap::<&str, BTreeMap<&str, usize>>::new();
    for row in rows {
        if row.target_lane.is_empty() {
            continue;
        }
        *by_lane_progress
            .entry(&row.target_lane)
            .or_default()
            .entry(&row.target_progress)
            .or_default() += 1;
    }
    if !by_lane_progress.is_empty() {
        lines.push("Target progress by lane:".to_string());
        for (lane, counts) in by_lane_progress {
            lines.push(format!(
                "  {lane} extended={} target_only={} discarded={} missing={} incomplete={} unknown={}",
                count_for_key_v1(&counts, "extended"),
                count_for_key_v1(&counts, "target_only"),
                count_for_key_v1(&counts, "discarded"),
                count_for_key_v1(&counts, "missing"),
                count_for_key_v1(&counts, "incomplete"),
                count_for_key_v1(&counts, "unknown")
            ));
        }
    }

    let source_counts = target_origin_source_counts_v1(rows.iter());
    if !source_counts.is_empty() {
        lines.push(format!(
            "Target origin sources: {}",
            source_counts
                .iter()
                .map(|(source, count)| format!("{source}={count}"))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }

    lines.push("Reached target examples:".to_string());
    let mut reached_rows: Vec<_> = rows
        .iter()
        .filter(|row| target.is_reached_by(row))
        .collect();
    reached_rows.sort_by(|left, right| compare_milestone_rows(right, left));
    if reached_rows.is_empty() {
        lines.push("  none".to_string());
    } else {
        for row in reached_rows.into_iter().take(8) {
            lines.push(format!("  {}", format_milestone_row_v1(row)));
        }
    }

    lines
        .into_iter()
        .filter(|line| !line.trim_end().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn coverage_gap_milestone_rows_from_report_v1(
    report: &BranchCampaignReportV1,
) -> Vec<CoverageGapMilestoneBranchRowV1> {
    let mut rows = Vec::new();
    collect_branch_rows_v1("active", &report.active, &mut rows);
    collect_branch_rows_v1("frozen", &report.frozen, &mut rows);
    collect_branch_rows_v1("victories", &report.victories, &mut rows);
    collect_branch_rows_v1("dead", &report.dead, &mut rows);
    collect_branch_rows_v1("abandoned", &report.abandoned, &mut rows);
    collect_branch_rows_v1("stuck", &report.stuck, &mut rows);
    collect_discarded_rows_v1(&report.discarded_branches, &mut rows);
    rows
}

fn collect_branch_rows_v1(
    bucket: &str,
    branches: &[BranchCampaignBranchV1],
    rows: &mut Vec<CoverageGapMilestoneBranchRowV1>,
) {
    for branch in branches {
        let Some(origin) = branch
            .continuation_origin
            .as_ref()
            .filter(|origin| origin.kind == "coverage_gap")
        else {
            continue;
        };
        rows.push(row_from_parts_v1(
            bucket,
            &origin.event_type,
            &origin.label,
            &origin.command,
            &origin.target_origin_source,
            render_origin_target_lane_v1(origin),
            target_progress_for_branch_v1(&branch.commands, &origin.command),
            branch.summary.as_ref(),
            &branch.frontier_title,
            &branch.stop_reason,
            &branch.choice_labels,
        ));
    }
}

fn collect_discarded_rows_v1(
    branches: &[BranchCampaignDiscardedBranchV1],
    rows: &mut Vec<CoverageGapMilestoneBranchRowV1>,
) {
    for branch in branches {
        let Some(origin) = branch
            .continuation_origin
            .as_ref()
            .filter(|origin| origin.kind == "coverage_gap")
        else {
            continue;
        };
        rows.push(row_from_parts_v1(
            "discarded",
            &origin.event_type,
            &origin.label,
            &origin.command,
            &origin.target_origin_source,
            render_origin_target_lane_v1(origin),
            "discarded".to_string(),
            branch.summary.as_ref(),
            &branch.frontier_title,
            &branch.stop_reason,
            &branch.choice_labels,
        ));
    }
}

fn row_from_parts_v1(
    bucket: &str,
    event_type: &str,
    label: &str,
    command: &str,
    target_origin_source: &str,
    target_lane: String,
    target_progress: String,
    summary: Option<&BranchCampaignBranchSummaryV1>,
    frontier_title: &str,
    stop_reason: &str,
    choice_labels: &[String],
) -> CoverageGapMilestoneBranchRowV1 {
    CoverageGapMilestoneBranchRowV1 {
        bucket: bucket.to_string(),
        event_type: event_type.to_string(),
        label: label.to_string(),
        command: command.to_string(),
        act: summary.map_or(0, |summary| summary.act),
        floor: summary.map_or(0, |summary| summary.floor),
        hp: summary.map_or(0, |summary| summary.hp),
        max_hp: summary.map_or(0, |summary| summary.max_hp),
        deck_count: summary.map_or(0, |summary| summary.deck_count),
        target_origin_source: target_origin_source.to_string(),
        target_lane,
        target_progress,
        frontier_title: frontier_title.to_string(),
        stop_reason: stop_reason.to_string(),
        choice_labels: choice_labels.to_vec(),
    }
}

fn render_origin_target_lane_v1(
    origin: &sts_simulator::eval::branch_campaign::BranchCampaignContinuationOriginV1,
) -> String {
    if origin.event_type == "route" {
        if let Some(route) = origin.route_origin.as_ref() {
            return render_route_origin_target_lane_v1(route);
        }
    }
    render_target_lane_v1(origin.target_lane.as_ref())
}

fn render_route_origin_target_lane_v1(
    route: &sts_simulator::eval::branch_campaign::BranchCampaignRouteContinuationOriginV1,
) -> String {
    let path = route
        .path
        .as_ref()
        .map(render_route_path_lane_v1)
        .unwrap_or_else(|| "unknown_path".to_string());
    format!(
        "route:{}:{}:{}:{}",
        route.action_kind, route.room_type, route.projection_coverage, path
    )
}

fn render_route_path_lane_v1(
    path: &sts_simulator::eval::branch_campaign::BranchCampaignRoutePathContinuationOriginV1,
) -> String {
    format!(
        "{}:{}:{}",
        route_shop_timing_lane_v1(path.first_shop_floor),
        route_fire_timing_lane_v1(path.first_fire_floor),
        route_pre_recovery_pressure_lane_v1(path.max_damage_rooms_before_recovery)
    )
}

fn route_shop_timing_lane_v1(first_shop_floor: Option<i32>) -> &'static str {
    match first_shop_floor {
        Some(floor) if floor <= 5 => "early_shop",
        Some(_) => "late_shop",
        None => "no_shop",
    }
}

fn route_fire_timing_lane_v1(first_fire_floor: Option<i32>) -> &'static str {
    match first_fire_floor {
        Some(floor) if floor <= 6 => "early_fire",
        Some(_) => "late_fire",
        None => "no_fire",
    }
}

fn route_pre_recovery_pressure_lane_v1(max_damage_rooms_before_recovery: usize) -> &'static str {
    if max_damage_rooms_before_recovery <= 1 {
        "low_pre_recovery_damage"
    } else if max_damage_rooms_before_recovery <= 3 {
        "medium_pre_recovery_damage"
    } else {
        "high_pre_recovery_damage"
    }
}

fn render_target_lane_v1(
    lane: Option<&sts_simulator::eval::branch_campaign::BranchCampaignContinuationTargetLaneV1>,
) -> String {
    let Some(lane) = lane else {
        return String::new();
    };
    format!(
        "{}:{}:{}:{}",
        lane.bucket,
        render_admission_status_v1(lane.admission_status),
        render_disposition_v1(lane.disposition),
        lane.semantic_lane
    )
}

fn render_admission_status_v1(
    status: sts_simulator::eval::campaign_journal::CampaignJournalCandidateAdmissionStatusV1,
) -> &'static str {
    match status {
        sts_simulator::eval::campaign_journal::CampaignJournalCandidateAdmissionStatusV1::Unknown => {
            "unknown"
        }
        sts_simulator::eval::campaign_journal::CampaignJournalCandidateAdmissionStatusV1::Scheduled => {
            "scheduled"
        }
        sts_simulator::eval::campaign_journal::CampaignJournalCandidateAdmissionStatusV1::Deferred => {
            "deferred"
        }
        sts_simulator::eval::campaign_journal::CampaignJournalCandidateAdmissionStatusV1::Rejected => {
            "rejected"
        }
    }
}

fn render_disposition_v1(
    disposition: sts_simulator::eval::campaign_journal::CampaignJournalCandidateDispositionV1,
) -> &'static str {
    match disposition {
        sts_simulator::eval::campaign_journal::CampaignJournalCandidateDispositionV1::Kept => {
            "kept"
        }
        sts_simulator::eval::campaign_journal::CampaignJournalCandidateDispositionV1::Pruned => {
            "pruned"
        }
    }
}

fn target_progress_for_branch_v1(commands: &[String], target_command: &str) -> String {
    let Some(index) = commands
        .iter()
        .position(|command| command == target_command)
    else {
        return "unknown".to_string();
    };
    if index + 1 < commands.len() {
        "extended".to_string()
    } else {
        "target_only".to_string()
    }
}

fn target_progress_counts_v1<'a>(
    rows: impl Iterator<Item = &'a CoverageGapMilestoneBranchRowV1>,
) -> BTreeMap<&'a str, usize> {
    let mut counts = BTreeMap::new();
    for row in rows {
        *counts.entry(row.target_progress.as_str()).or_default() += 1;
    }
    counts
}

fn target_origin_source_counts_v1<'a>(
    rows: impl Iterator<Item = &'a CoverageGapMilestoneBranchRowV1>,
) -> BTreeMap<&'a str, usize> {
    let mut counts = BTreeMap::new();
    for row in rows {
        if row.target_origin_source.is_empty() {
            continue;
        }
        *counts.entry(row.target_origin_source.as_str()).or_default() += 1;
    }
    counts
}

fn count_for_key_v1(counts: &BTreeMap<&str, usize>, key: &str) -> usize {
    counts.get(key).copied().unwrap_or(0)
}

fn compare_milestone_rows(
    left: &CoverageGapMilestoneBranchRowV1,
    right: &CoverageGapMilestoneBranchRowV1,
) -> Ordering {
    (
        left.act,
        left.floor,
        left.hp,
        -(left.deck_count as i32),
        &left.event_type,
        &left.label,
    )
        .cmp(&(
            right.act,
            right.floor,
            right.hp,
            -(right.deck_count as i32),
            &right.event_type,
            &right.label,
        ))
}

fn format_milestone_row_v1(row: &CoverageGapMilestoneBranchRowV1) -> String {
    let stop = if row.stop_reason.is_empty() {
        String::new()
    } else {
        format!(" stop={}", row.stop_reason)
    };
    format!(
        "A{}F{} HP {}/{} deck {} | {} | {} | {} {{{}}} | frontier={}{}",
        row.act,
        row.floor,
        row.hp,
        row.max_hp,
        row.deck_count,
        row.bucket,
        row.event_type,
        row.label,
        row.command,
        row.frontier_title,
        stop
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(
        bucket: &str,
        event_type: &str,
        act: u8,
        floor: i32,
        label: &str,
    ) -> CoverageGapMilestoneBranchRowV1 {
        CoverageGapMilestoneBranchRowV1 {
            bucket: bucket.to_string(),
            event_type: event_type.to_string(),
            label: label.to_string(),
            command: format!("choose {label}"),
            act,
            floor,
            hp: 80,
            max_hp: 80,
            deck_count: 12,
            target_origin_source: "journal_coverage_gap".to_string(),
            target_lane: format!("{event_type}:scheduled:kept:test"),
            target_progress: "extended".to_string(),
            frontier_title: "Reward Screen".to_string(),
            stop_reason: String::new(),
            choice_labels: vec![label.to_string()],
        }
    }

    #[test]
    fn milestone_summary_groups_coverage_gap_origins_and_reports_reached_target() {
        let rows = vec![
            row("active", "boss_relic", 2, 19, "RunicPyramid"),
            row("frozen", "route", 1, 6, "x=1 Elite"),
            row("active", "shop", 2, 17, "Purge Strike"),
            row("frozen", "shop", 1, 8, "Buy Reaper"),
        ];

        let text = render_coverage_gap_milestone_summary_from_rows_v1(
            &rows,
            CoverageGapMilestoneTargetV1::Act2Start,
        );

        assert!(text.contains("CoverageGapMilestoneSummaryV1 target=Act2Start total=4 reached=2"));
        assert!(text.contains("boss_relic total=1 reached=1 active=1 frozen=0"));
        assert!(text.contains("shop total=2 reached=1 active=1 frozen=1"));
        assert!(text.contains("route total=1 reached=0 active=0 frozen=1"));
        assert!(text.contains("Reached target examples:"));
        assert!(text.contains("A2F19 HP 80/80 deck 12 | active | boss_relic | RunicPyramid"));
    }

    #[test]
    fn milestone_summary_reports_target_progress_counts() {
        let mut extended = row("active", "shop", 2, 17, "Buy Reaper");
        extended.target_progress = "extended".to_string();
        extended.target_origin_source = "shop_plan_frontier".to_string();
        let mut target_only = row("frozen", "reward", 1, 5, "Shockwave");
        target_only.target_progress = "target_only".to_string();
        target_only.target_lane = "reward:scheduled:kept:role:scaling".to_string();
        let mut discarded = row("discarded", "route", 1, 6, "x=1 Elite");
        discarded.target_progress = "discarded".to_string();
        discarded.target_origin_source = "map_decision_packet".to_string();
        discarded.target_lane =
            "route:go:MonsterRoom:CompleteWithinBudget:optional_elite".to_string();

        let text = render_coverage_gap_milestone_summary_from_rows_v1(
            &[extended, target_only, discarded],
            CoverageGapMilestoneTargetV1::Act2Start,
        );

        assert!(text.contains("Target progress: extended=1 target_only=1 discarded=1"));
        assert!(text.contains("shop extended=1 target_only=0 discarded=0"));
        assert!(text.contains("reward extended=0 target_only=1 discarded=0"));
        assert!(text.contains("route extended=0 target_only=0 discarded=1"));
        assert!(text.contains("Target origin sources:"));
        assert!(text.contains("map_decision_packet=1"));
        assert!(text.contains("shop_plan_frontier=1"));
        assert!(text.contains("Target progress by lane:"));
        assert!(text.contains("shop:scheduled:kept:test extended=1 target_only=0 discarded=0"));
        assert!(text
            .contains("reward:scheduled:kept:role:scaling extended=0 target_only=1 discarded=0"));
    }

    #[test]
    fn milestone_summary_prefers_route_origin_lane_over_generic_target_lane() {
        use sts_simulator::eval::branch_campaign::{
            BranchCampaignContinuationOriginV1, BranchCampaignContinuationTargetLaneV1,
            BranchCampaignRouteContinuationOriginV1, BranchCampaignRoutePathContinuationOriginV1,
        };
        use sts_simulator::eval::campaign_journal::{
            CampaignJournalCandidateAdmissionStatusV1, CampaignJournalCandidateDispositionV1,
        };

        let origin = BranchCampaignContinuationOriginV1 {
            kind: "coverage_gap".to_string(),
            source_event_id: "route:event".to_string(),
            decision_id: "route:decision".to_string(),
            event_type: "route".to_string(),
            parent_branch_id: "root".to_string(),
            parent_frontier_title: "Map".to_string(),
            candidate_index: 0,
            candidate_id: "route:0".to_string(),
            command: "go 0".to_string(),
            label: "x=0 y=2 Monster".to_string(),
            semantic_class: "room:Monster".to_string(),
            admission: Default::default(),
            disposition: CampaignJournalCandidateDispositionV1::Kept,
            target_lane: Some(BranchCampaignContinuationTargetLaneV1 {
                bucket: "route".to_string(),
                admission_status: CampaignJournalCandidateAdmissionStatusV1::Scheduled,
                disposition: CampaignJournalCandidateDispositionV1::Kept,
                semantic_lane: "room:Monster".to_string(),
                shop_action_kind: None,
            }),
            target_origin_source: "map_decision_packet".to_string(),
            route_origin: Some(BranchCampaignRouteContinuationOriginV1 {
                legal_candidate_count: 4,
                emitted_candidate_count: 4,
                complete_legal_pool: true,
                ordering: "planner".to_string(),
                ordering_kind: None,
                target_x: 0,
                target_y: 2,
                target_node: None,
                room_type: "MonsterRoom".to_string(),
                move_kind: "normal_edge".to_string(),
                action_kind: "go".to_string(),
                action: None,
                projection_source: "Complete".to_string(),
                projection_source_kind: None,
                projection_coverage: "CompleteWithinBudget".to_string(),
                projection_coverage_kind: None,
                path_budget: 16,
                observed_path_count: 4,
                path: Some(BranchCampaignRoutePathContinuationOriginV1 {
                    path_count: 4,
                    path_budget_exhausted: false,
                    min_early_pressure: 1,
                    max_early_pressure: 3,
                    min_elites: 0,
                    max_elites: 1,
                    min_shops: 0,
                    max_shops: 1,
                    min_fires: 0,
                    max_fires: 1,
                    min_unknowns: 0,
                    max_unknowns: 2,
                    min_treasures: 0,
                    max_treasures: 1,
                    first_shop_floor: Some(8),
                    first_fire_floor: Some(5),
                    min_damage_rooms_before_recovery: 1,
                    max_damage_rooms_before_recovery: 3,
                    min_unknowns_before_recovery: 0,
                    max_unknowns_before_recovery: 1,
                    paths_with_recovery_before_damage: 1,
                }),
                first_elite: None,
            }),
            milestone: "route_frontier".to_string(),
        };

        let lane = render_origin_target_lane_v1(&origin);

        assert!(lane.contains("route:go:MonsterRoom:CompleteWithinBudget"));
        assert!(lane.contains("early_fire"));
        assert!(!lane.ends_with("room:Monster"));
    }
}
