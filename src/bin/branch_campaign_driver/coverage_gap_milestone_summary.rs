use std::cmp::Ordering;
use std::collections::BTreeMap;

use sts_simulator::eval::branch_campaign::{
    BranchCampaignBranchSummaryV1, BranchCampaignBranchV1, BranchCampaignDiscardedBranchV1,
    BranchCampaignReportV1,
};
use sts_simulator::eval::learning_dataset_v1::CoverageGapContinuationFilterV1;

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
    pub(super) branch_id: String,
    pub(super) commands: Vec<String>,
    pub(super) target_key: String,
    pub(super) event_type: String,
    pub(super) label: String,
    pub(super) command: String,
    pub(super) act: u8,
    pub(super) floor: i32,
    pub(super) hp: i32,
    pub(super) max_hp: i32,
    pub(super) deck_count: usize,
    pub(super) deck_key: String,
    pub(super) boss: String,
    pub(super) boss_pressure: Vec<String>,
    pub(super) run_debt: Vec<String>,
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

#[derive(Default)]
struct CoverageGapMilestoneTargetGroupSummaryV1<'a> {
    rows: usize,
    reached: bool,
    active: usize,
    frozen: usize,
    stuck: usize,
    abandoned: usize,
    dead: usize,
    victories: usize,
    discarded: usize,
    other: usize,
    first: Option<&'a CoverageGapMilestoneBranchRowV1>,
    furthest: Option<&'a CoverageGapMilestoneBranchRowV1>,
    stop_reason: Option<&'a str>,
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
        render_coverage_gap_milestone_summary_with_filter_and_group_index_v1(
            &report,
            target,
            &input.coverage_gap_filter,
            input.filters.index,
        )
    );
    Ok(())
}

pub(super) fn render_coverage_gap_milestone_summary_with_filter_and_group_index_v1(
    report: &BranchCampaignReportV1,
    target: CoverageGapMilestoneTargetV1,
    filter: &CoverageGapContinuationFilterV1,
    group_index: Option<usize>,
) -> String {
    let rows = coverage_gap_milestone_rows_from_report_v1(report);
    render_coverage_gap_milestone_summary_from_rows_with_filter_and_group_index_v1(
        &rows,
        target,
        filter,
        group_index,
    )
}

#[cfg(test)]
pub(super) fn render_coverage_gap_milestone_summary_from_rows_with_filter_v1(
    rows: &[CoverageGapMilestoneBranchRowV1],
    target: CoverageGapMilestoneTargetV1,
    filter: &CoverageGapContinuationFilterV1,
) -> String {
    render_coverage_gap_milestone_summary_from_rows_with_filter_and_group_index_v1(
        rows, target, filter, None,
    )
}

pub(super) fn render_coverage_gap_milestone_summary_from_rows_with_filter_and_group_index_v1(
    rows: &[CoverageGapMilestoneBranchRowV1],
    target: CoverageGapMilestoneTargetV1,
    filter: &CoverageGapContinuationFilterV1,
    group_index: Option<usize>,
) -> String {
    let filtered_rows = rows
        .iter()
        .filter(|row| coverage_gap_milestone_row_matches_filter_v1(row, filter))
        .cloned()
        .collect::<Vec<_>>();
    let rows = filtered_rows.as_slice();
    let reached = rows.iter().filter(|row| target.is_reached_by(row)).count();
    let (target_groups, reached_target_groups) = target_group_counts_v1(rows, target);
    let mut lines = Vec::new();
    lines.push(format!(
        "CoverageGapMilestoneSummaryV1 target={} total={} reached={} not_reached={}",
        target.as_str(),
        rows.len(),
        reached,
        rows.len().saturating_sub(reached)
    ));
    lines.push(format!(
        "Target groups: total={} reached={} not_reached={}",
        target_groups,
        reached_target_groups,
        target_groups.saturating_sub(reached_target_groups)
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
        for (lane, counts) in &by_lane_progress {
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
        let mut extended_lanes = by_lane_progress
            .iter()
            .filter_map(|(lane, counts)| {
                let extended = count_for_key_v1(counts, "extended");
                if extended == 0 {
                    return None;
                }
                Some((
                    *lane,
                    extended,
                    count_for_key_v1(counts, "discarded"),
                    count_for_key_v1(counts, "target_only"),
                ))
            })
            .collect::<Vec<_>>();
        extended_lanes.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| left.0.cmp(right.0))
        });
        if !extended_lanes.is_empty() {
            lines.push("Extended lanes:".to_string());
            for (lane, extended, discarded, target_only) in extended_lanes.into_iter().take(8) {
                lines.push(format!(
                    "  {lane} extended={extended} discarded={discarded} target_only={target_only}"
                ));
            }
        }
        let mut discarded_heavy_lanes = by_lane_progress
            .iter()
            .filter_map(|(lane, counts)| {
                let discarded = count_for_key_v1(counts, "discarded");
                if discarded == 0 {
                    return None;
                }
                Some((
                    *lane,
                    discarded,
                    count_for_key_v1(counts, "extended"),
                    count_for_key_v1(counts, "target_only"),
                ))
            })
            .collect::<Vec<_>>();
        discarded_heavy_lanes.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| left.0.cmp(right.0))
        });
        if !discarded_heavy_lanes.is_empty() {
            lines.push("Discarded-heavy lanes:".to_string());
            for (lane, discarded, extended, target_only) in
                discarded_heavy_lanes.into_iter().take(8)
            {
                lines.push(format!(
                    "  {lane} discarded={discarded} extended={extended} target_only={target_only}"
                ));
            }
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

    let target_group_audit = target_group_audit_summaries_v1(rows, target);
    if !target_group_audit.is_empty() {
        let focused_target_group_audit = if let Some(index) = group_index {
            lines.push(format!(
                "Target group focus: index={} total={}",
                index,
                target_group_audit.len()
            ));
            target_group_audit
                .get(index)
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            target_group_audit.iter().take(12).collect::<Vec<_>>()
        };
        lines.push("Target group audit:".to_string());
        for summary in &focused_target_group_audit {
            lines.push(format_target_group_audit_line_v1(summary));
        }
        lines.push("Target group details:".to_string());
        for summary in focused_target_group_audit.into_iter().take(6) {
            lines.extend(format_target_group_detail_lines_v1(summary));
        }
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

fn coverage_gap_milestone_row_matches_filter_v1(
    row: &CoverageGapMilestoneBranchRowV1,
    filter: &CoverageGapContinuationFilterV1,
) -> bool {
    if let Some(bucket) = filter.bucket.as_deref().filter(|value| !value.is_empty()) {
        let bucket = normalize_filter_text_v1(bucket);
        let event_type = normalize_filter_text_v1(&row.event_type);
        if event_type != bucket && !event_type.contains(&bucket) {
            return false;
        }
    }
    if let Some(event_id) = filter.event_id.as_deref().filter(|value| !value.is_empty()) {
        let event_id = normalize_filter_text_v1(event_id);
        let fields = [
            row.target_key.as_str(),
            row.event_type.as_str(),
            row.label.as_str(),
            row.command.as_str(),
            row.frontier_title.as_str(),
            row.stop_reason.as_str(),
        ];
        if !fields
            .iter()
            .any(|field| normalize_filter_text_v1(field).contains(&event_id))
            && !row
                .choice_labels
                .iter()
                .any(|field| normalize_filter_text_v1(field).contains(&event_id))
        {
            return false;
        }
    }
    if let Some(lane) = filter.lane.as_deref().filter(|value| !value.is_empty()) {
        let lane = normalize_filter_text_v1(lane);
        let row_lane = normalize_filter_text_v1(&row.target_lane);
        if !row_lane.contains(&lane) {
            return false;
        }
    }
    if let Some(origin_source) = filter
        .origin_source
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        let origin_source = normalize_filter_text_v1(origin_source);
        let row_origin = normalize_filter_text_v1(&row.target_origin_source);
        if row_origin != origin_source && !row_origin.contains(&origin_source) {
            return false;
        }
    }
    if let Some(progress) = filter.progress.as_deref().filter(|value| !value.is_empty()) {
        let progress = normalize_filter_text_v1(progress);
        let row_progress = normalize_filter_text_v1(&row.target_progress);
        if row_progress != progress && !row_progress.contains(&progress) {
            return false;
        }
    }
    true
}

fn normalize_filter_text_v1(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
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
            &branch.branch_id,
            &branch.commands,
            target_key_from_origin_v1(origin),
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
            &branch.branch_id,
            &[],
            target_key_from_origin_v1(origin),
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
    branch_id: &str,
    commands: &[String],
    target_key: String,
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
        branch_id: branch_id.to_string(),
        commands: commands.to_vec(),
        target_key,
        event_type: event_type.to_string(),
        label: label.to_string(),
        command: command.to_string(),
        act: summary.map_or(0, |summary| summary.act),
        floor: summary.map_or(0, |summary| summary.floor),
        hp: summary.map_or(0, |summary| summary.hp),
        max_hp: summary.map_or(0, |summary| summary.max_hp),
        deck_count: summary.map_or(0, |summary| summary.deck_count),
        deck_key: summary
            .map(|summary| summary.deck_key.clone())
            .unwrap_or_default(),
        boss: summary
            .map(|summary| summary.boss.clone())
            .unwrap_or_default(),
        boss_pressure: summary
            .map(|summary| summary.boss_pressure.clone())
            .unwrap_or_default(),
        run_debt: summary
            .map(|summary| summary.run_debt.clone())
            .unwrap_or_default(),
        target_origin_source: target_origin_source.to_string(),
        target_lane,
        target_progress,
        frontier_title: frontier_title.to_string(),
        stop_reason: stop_reason.to_string(),
        choice_labels: choice_labels.to_vec(),
    }
}

fn target_key_from_origin_v1(
    origin: &sts_simulator::eval::branch_campaign::BranchCampaignContinuationOriginV1,
) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        origin.event_type,
        origin.decision_id,
        origin.candidate_id,
        origin.candidate_index,
        origin.command
    )
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
    let first_elite = route
        .first_elite
        .as_ref()
        .map(route_first_elite_lane_v1)
        .unwrap_or("no_first_elite");
    format!(
        "route:{}:{}:{}:{}:{}",
        route.action_kind, route.room_type, route.projection_coverage, first_elite, path
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

fn route_first_elite_lane_v1(
    first_elite: &sts_simulator::eval::branch_campaign::BranchCampaignRouteFirstEliteContinuationOriginV1,
) -> &'static str {
    if first_elite.forced {
        "forced_elite"
    } else if first_elite.optional {
        "optional_elite"
    } else if first_elite.paths_with_first_elite > 0 {
        "elite_access"
    } else {
        "no_first_elite"
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

fn target_group_counts_v1(
    rows: &[CoverageGapMilestoneBranchRowV1],
    target: CoverageGapMilestoneTargetV1,
) -> (usize, usize) {
    let mut groups = BTreeMap::<&str, bool>::new();
    for row in rows {
        let reached = target.is_reached_by(row);
        groups
            .entry(row.target_key.as_str())
            .and_modify(|existing| *existing |= reached)
            .or_insert(reached);
    }
    let reached = groups.values().filter(|is_reached| **is_reached).count();
    (groups.len(), reached)
}

fn target_group_audit_summaries_v1<'a>(
    rows: &'a [CoverageGapMilestoneBranchRowV1],
    target: CoverageGapMilestoneTargetV1,
) -> Vec<CoverageGapMilestoneTargetGroupSummaryV1<'a>> {
    let mut groups = BTreeMap::<&str, CoverageGapMilestoneTargetGroupSummaryV1<'a>>::new();
    for row in rows {
        let summary = groups.entry(row.target_key.as_str()).or_default();
        summary.rows += 1;
        summary.reached |= target.is_reached_by(row);
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
        if summary.first.is_none() {
            summary.first = Some(row);
        }
        if summary
            .furthest
            .is_none_or(|current| compare_milestone_rows(row, current) == Ordering::Greater)
        {
            summary.furthest = Some(row);
            summary.stop_reason = (!row.stop_reason.is_empty()).then_some(row.stop_reason.as_str());
        }
    }

    let mut summaries = groups.into_values().collect::<Vec<_>>();
    summaries.sort_by(|left, right| compare_target_group_audit_summary_v1(right, left));
    summaries
}

fn compare_target_group_audit_summary_v1(
    left: &CoverageGapMilestoneTargetGroupSummaryV1<'_>,
    right: &CoverageGapMilestoneTargetGroupSummaryV1<'_>,
) -> Ordering {
    left.reached
        .cmp(&right.reached)
        .then_with(|| {
            compare_milestone_rows(
                left.furthest
                    .expect("target group should have a furthest row"),
                right
                    .furthest
                    .expect("target group should have a furthest row"),
            )
        })
        .then_with(|| left.rows.cmp(&right.rows))
}

fn format_target_group_audit_line_v1(
    summary: &CoverageGapMilestoneTargetGroupSummaryV1<'_>,
) -> String {
    let first = summary.first.expect("target group should have a first row");
    let furthest = summary
        .furthest
        .expect("target group should have a furthest row");
    let reached = if summary.reached { "yes" } else { "no" };
    let lane = if first.target_lane.is_empty() {
        String::new()
    } else {
        format!(" lane={}", first.target_lane)
    };
    let stop = summary
        .stop_reason
        .map(|reason| format!(" stop={reason}"))
        .unwrap_or_default();
    format!(
        "  {} | {} {{{}}} | reached={} rows={} active={} frozen={} abandoned={} stuck={} dead={} victories={} discarded={} other={} furthest=A{}F{}{}{}",
        first.event_type,
        first.label,
        first.command,
        reached,
        summary.rows,
        summary.active,
        summary.frozen,
        summary.abandoned,
        summary.stuck,
        summary.dead,
        summary.victories,
        summary.discarded,
        summary.other,
        furthest.act,
        furthest.floor,
        lane,
        stop
    )
}

fn format_target_group_detail_lines_v1(
    summary: &CoverageGapMilestoneTargetGroupSummaryV1<'_>,
) -> Vec<String> {
    let first = summary.first.expect("target group should have a first row");
    let furthest = summary
        .furthest
        .expect("target group should have a furthest row");
    let mut lines = Vec::new();
    lines.push(format!(
        "  {} | {} {{{}}}",
        first.event_type, first.label, first.command
    ));
    lines.push(format!(
        "    furthest: {}",
        format_milestone_row_v1(furthest)
    ));
    if !furthest.target_progress.is_empty() {
        lines.push(format!("    target_progress: {}", furthest.target_progress));
    }
    if !furthest.branch_id.is_empty() {
        lines.push(format!("    branch_id: {}", furthest.branch_id));
    }
    if !furthest.commands.is_empty() {
        lines.push(format!(
            "    commands: {}",
            render_recent_command_path_v1(&furthest.commands)
        ));
    }
    if !furthest.deck_key.is_empty() {
        lines.push(format!("    deck_key: {}", furthest.deck_key));
    }
    if !furthest.boss.is_empty() {
        lines.push(format!("    boss: {}", furthest.boss));
    }
    if !furthest.boss_pressure.is_empty() {
        lines.push(format!(
            "    boss_pressure: {}",
            furthest.boss_pressure.join(", ")
        ));
    }
    if !furthest.run_debt.is_empty() {
        lines.push(format!("    run_debt: {}", furthest.run_debt.join(", ")));
    }
    if !furthest.target_origin_source.is_empty() {
        lines.push(format!(
            "    target_origin_source: {}",
            furthest.target_origin_source
        ));
    }
    if !furthest.target_lane.is_empty() {
        lines.push(format!("    lane: {}", furthest.target_lane));
    }
    if !furthest.choice_labels.is_empty() {
        lines.push(format!(
            "    choices: {}",
            render_recent_choice_path_v1(&furthest.choice_labels)
        ));
    }
    lines
}

fn render_recent_choice_path_v1(choice_labels: &[String]) -> String {
    const MAX_CHOICES: usize = 8;
    if choice_labels.len() <= MAX_CHOICES {
        return choice_labels.join(" -> ");
    }
    let skipped = choice_labels.len() - MAX_CHOICES;
    let suffix = choice_labels[skipped..].join(" -> ");
    format!("... {skipped} earlier -> {suffix}")
}

fn render_recent_command_path_v1(commands: &[String]) -> String {
    const MAX_COMMANDS: usize = 10;
    if commands.len() <= MAX_COMMANDS {
        return commands.join(" -> ");
    }
    let skipped = commands.len() - MAX_COMMANDS;
    let suffix = commands[skipped..].join(" -> ");
    format!("... {skipped} earlier -> {suffix}")
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
            branch_id: format!("branch:{event_type}:{label}"),
            commands: vec![format!("choose {label}")],
            target_key: format!("{event_type}:{label}"),
            event_type: event_type.to_string(),
            label: label.to_string(),
            command: format!("choose {label}"),
            act,
            floor,
            hp: 80,
            max_hp: 80,
            deck_count: 12,
            deck_key: String::new(),
            boss: String::new(),
            boss_pressure: Vec::new(),
            run_debt: Vec::new(),
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

        let text = render_coverage_gap_milestone_summary_from_rows_with_filter_v1(
            &rows,
            CoverageGapMilestoneTargetV1::Act2Start,
            &CoverageGapContinuationFilterV1::default(),
        );

        assert!(text.contains("CoverageGapMilestoneSummaryV1 target=Act2Start total=4 reached=2"));
        assert!(text.contains("boss_relic total=1 reached=1 active=1 frozen=0"));
        assert!(text.contains("shop total=2 reached=1 active=1 frozen=1"));
        assert!(text.contains("route total=1 reached=0 active=0 frozen=1"));
        assert!(text.contains("Reached target examples:"));
        assert!(text.contains("A2F19 HP 80/80 deck 12 | active | boss_relic | RunicPyramid"));
    }

    #[test]
    fn milestone_summary_reports_target_groups_separately_from_branch_rows() {
        let mut reached_a = row("active", "boss_relic", 2, 18, "RunicPyramid");
        reached_a.target_key = "boss_relic:runic_pyramid".to_string();
        let mut reached_b = row("active", "boss_relic", 2, 18, "RunicPyramid");
        reached_b.target_key = "boss_relic:runic_pyramid".to_string();
        let unreached = row("frozen", "reward", 1, 8, "True Grit");

        let text = render_coverage_gap_milestone_summary_from_rows_with_filter_v1(
            &[reached_a, reached_b, unreached],
            CoverageGapMilestoneTargetV1::Act2Start,
            &CoverageGapContinuationFilterV1::default(),
        );

        assert!(text.contains("CoverageGapMilestoneSummaryV1 target=Act2Start total=3 reached=2"));
        assert!(text.contains("Target groups: total=2 reached=1 not_reached=1"));
    }

    #[test]
    fn milestone_summary_reports_target_group_audit() {
        let mut reached_a = row("active", "boss_relic", 2, 18, "RunicPyramid");
        reached_a.target_key = "boss_relic:runic_pyramid".to_string();
        reached_a.target_lane = "boss_relic:scheduled:kept:relic:RunicPyramid".to_string();
        let mut reached_b = row("frozen", "boss_relic", 2, 19, "RunicPyramid");
        reached_b.target_key = "boss_relic:runic_pyramid".to_string();
        reached_b.target_lane = "boss_relic:scheduled:kept:relic:RunicPyramid".to_string();
        reached_b.frontier_title = "Reward Screen".to_string();
        let mut abandoned = row("abandoned", "route", 1, 16, "x=6 y=12 Rest");
        abandoned.target_key = "route:rest".to_string();
        abandoned.target_lane = "route:go:RestRoom:CompleteWithinBudget:no_first_elite".to_string();
        abandoned.stop_reason = "combat search did not find an executable complete win".to_string();

        let text = render_coverage_gap_milestone_summary_from_rows_with_filter_v1(
            &[reached_a, reached_b, abandoned],
            CoverageGapMilestoneTargetV1::Act2Start,
            &CoverageGapContinuationFilterV1::default(),
        );

        assert!(text.contains("Target group audit:"));
        assert!(text.contains(
            "boss_relic | RunicPyramid {choose RunicPyramid} | reached=yes rows=2 active=1 frozen=1"
        ));
        assert!(text.contains(
            "route | x=6 y=12 Rest {choose x=6 y=12 Rest} | reached=no rows=1 active=0 frozen=0 abandoned=1"
        ));
        assert!(text.contains("stop=combat search did not find an executable complete win"));
    }

    #[test]
    fn milestone_summary_includes_target_group_drilldown_details() {
        let mut abandoned = row("abandoned", "route", 1, 16, "x=6 y=12 Rest");
        abandoned.target_key = "route:rest".to_string();
        abandoned.target_lane = "route:go:RestRoom:CompleteWithinBudget:no_first_elite".to_string();
        abandoned.frontier_title = "Combat".to_string();
        abandoned.stop_reason = "combat search did not find an executable complete win".to_string();
        abandoned.branch_id = "branch-route-rest".to_string();
        abandoned.commands = vec![
            "rp 0".to_string(),
            "smith 1".to_string(),
            "go 6".to_string(),
            "rp 1".to_string(),
        ];
        abandoned.choice_labels = vec![
            "Skip card reward".to_string(),
            "Smith Bash".to_string(),
            "x=6 y=12 Rest".to_string(),
            "Battle Trance".to_string(),
        ];
        let mut frozen = row("frozen", "route", 1, 10, "x=6 y=12 Rest");
        frozen.target_key = "route:rest".to_string();
        frozen.target_lane = abandoned.target_lane.clone();

        let text = render_coverage_gap_milestone_summary_from_rows_with_filter_v1(
            &[abandoned, frozen],
            CoverageGapMilestoneTargetV1::Act2Start,
            &CoverageGapContinuationFilterV1::default(),
        );

        assert!(text.contains("Target group details:"));
        assert!(text.contains("route | x=6 y=12 Rest {choose x=6 y=12 Rest}"));
        assert!(text.contains("furthest: A1F16 HP 80/80 deck 12 | abandoned | route"));
        assert!(text.contains("branch_id: branch-route-rest"));
        assert!(text.contains("commands: rp 0 -> smith 1 -> go 6 -> rp 1"));
        assert!(text
            .contains("choices: Skip card reward -> Smith Bash -> x=6 y=12 Rest -> Battle Trance"));
    }

    #[test]
    fn milestone_summary_drilldown_shows_deck_key_when_available() {
        let summary = BranchCampaignBranchSummaryV1 {
            act: 1,
            floor: 16,
            hp: 72,
            max_hp: 80,
            gold: 120,
            deck_count: 13,
            deck_key: "Bash+1x1;Feed+0x1;Strike+0x4".to_string(),
            formation_stage: "Transitional".to_string(),
            formation_strengths: Vec::new(),
            formation_needs: Vec::new(),
            trajectory_key: String::new(),
            boss: "Hexaghost".to_string(),
            boss_pressure: Vec::new(),
            run_debt: Vec::new(),
            event_boundary: None,
            reward_boundary: None,
        };
        let mut route = row_from_parts_v1(
            "abandoned",
            "branch-route-rest",
            &["smith 1".to_string(), "go 6".to_string()],
            "route:rest".to_string(),
            "route",
            "x=6 y=12 Rest",
            "go 6",
            "map_decision_packet",
            "route:go:RestRoom:CompleteWithinBudget:no_first_elite".to_string(),
            "extended".to_string(),
            Some(&summary),
            "Combat",
            "combat search did not find an executable complete win",
            &["Smith Bash".to_string(), "Battle Trance".to_string()],
        );
        route.target_key = "route:rest".to_string();

        let text = render_coverage_gap_milestone_summary_from_rows_with_filter_v1(
            &[route],
            CoverageGapMilestoneTargetV1::Act2Start,
            &CoverageGapContinuationFilterV1::default(),
        );

        assert!(text.contains("deck_key: Bash+1x1;Feed+0x1;Strike+0x4"));
    }

    #[test]
    fn milestone_summary_drilldown_shows_boss_pressure_and_debt_when_available() {
        let summary = BranchCampaignBranchSummaryV1 {
            act: 2,
            floor: 33,
            hp: 41,
            max_hp: 90,
            gold: 250,
            deck_count: 24,
            deck_key: String::new(),
            formation_stage: "PlanCommitted".to_string(),
            formation_strengths: Vec::new(),
            formation_needs: Vec::new(),
            trajectory_key: String::new(),
            boss: "Automaton".to_string(),
            boss_pressure: vec![
                "missing:block50_or_kill_before_beam".to_string(),
                "pressure:stasis_key_card_access".to_string(),
            ],
            run_debt: vec!["CoffeeDripper=rest_lock".to_string()],
            event_boundary: None,
            reward_boundary: None,
        };
        let route = row_from_parts_v1(
            "abandoned",
            "branch-route-automaton",
            &["go 4".to_string()],
            "route:automaton".to_string(),
            "route",
            "x=4 y=10 Shop",
            "go 4",
            "map_decision_packet",
            "route:go:ShopRoom:CompleteWithinBudget".to_string(),
            "extended".to_string(),
            Some(&summary),
            "Combat",
            "combat search did not find an executable complete win",
            &["Buy Flame Barrier".to_string()],
        );

        let text = render_coverage_gap_milestone_summary_from_rows_with_filter_v1(
            &[route],
            CoverageGapMilestoneTargetV1::Act2Start,
            &CoverageGapContinuationFilterV1::default(),
        );

        assert!(text.contains("boss: Automaton"));
        assert!(text.contains(
            "boss_pressure: missing:block50_or_kill_before_beam, pressure:stasis_key_card_access"
        ));
        assert!(text.contains("run_debt: CoffeeDripper=rest_lock"));
    }

    #[test]
    fn milestone_summary_can_focus_one_target_group_by_index() {
        let mut furthest = row("abandoned", "route", 1, 16, "x=6 y=12 Rest");
        furthest.target_key = "route:rest".to_string();
        let mut earlier = row("active", "route", 1, 10, "x=1 y=5 Rest");
        earlier.target_key = "route:early_rest".to_string();

        let text = render_coverage_gap_milestone_summary_from_rows_with_filter_and_group_index_v1(
            &[furthest, earlier],
            CoverageGapMilestoneTargetV1::Act2Start,
            &CoverageGapContinuationFilterV1::default(),
            Some(1),
        );

        assert!(text.contains("Target group focus: index=1 total=2"));
        let target_group_section = text
            .split_once("Target group audit:")
            .expect("summary should contain target group audit")
            .1
            .split_once("Reached target examples:")
            .expect("summary should contain reached examples after target groups")
            .0;
        assert!(target_group_section.contains("route | x=1 y=5 Rest {choose x=1 y=5 Rest}"));
        assert!(!target_group_section.contains("route | x=6 y=12 Rest {choose x=6 y=12 Rest}"));
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

        let text = render_coverage_gap_milestone_summary_from_rows_with_filter_v1(
            &[extended, target_only, discarded],
            CoverageGapMilestoneTargetV1::Act2Start,
            &CoverageGapContinuationFilterV1::default(),
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
        assert!(text.contains("Extended lanes:"));
        assert!(text.contains("shop:scheduled:kept:test extended=1 discarded=0"));
        assert!(text.contains("Discarded-heavy lanes:"));
        assert!(text.contains(
            "route:go:MonsterRoom:CompleteWithinBudget:optional_elite discarded=1 extended=0"
        ));
    }

    #[test]
    fn milestone_summary_can_filter_by_coverage_gap_target_metadata() {
        let mut route_missing = row("active", "route", 1, 8, "x=1 Monster");
        route_missing.target_origin_source = "map_decision_packet".to_string();
        route_missing.target_progress = "missing".to_string();
        route_missing.target_lane =
            "route:go:MonsterRoom:CompleteWithinBudget:no_first_elite".to_string();
        let mut route_target_only = row("frozen", "route", 1, 8, "x=2 Shop");
        route_target_only.target_origin_source = "map_decision_packet".to_string();
        route_target_only.target_progress = "target_only".to_string();
        let mut event_missing = row("active", "event", 1, 8, "Mushrooms Eat");
        event_missing.target_origin_source = "event_boundary_packet".to_string();
        event_missing.target_progress = "missing".to_string();

        let text = render_coverage_gap_milestone_summary_from_rows_with_filter_v1(
            &[route_missing, route_target_only, event_missing],
            CoverageGapMilestoneTargetV1::Act2Start,
            &sts_simulator::eval::learning_dataset_v1::CoverageGapContinuationFilterV1 {
                origin_source: Some("map_decision_packet".to_string()),
                progress: Some("missing".to_string()),
                ..Default::default()
            },
        );

        assert!(text.contains("CoverageGapMilestoneSummaryV1 target=Act2Start total=1"));
        assert!(text.contains("route total=1"));
        assert!(!text.contains("event total=1"));
        assert!(text.contains("Target progress: extended=0 target_only=0 discarded=0 missing=1"));
        assert!(text.contains("Target origin sources: map_decision_packet=1"));
    }

    #[test]
    fn milestone_summary_prefers_route_origin_lane_over_generic_target_lane() {
        use sts_simulator::eval::branch_campaign::{
            BranchCampaignContinuationOriginV1, BranchCampaignContinuationTargetLaneV1,
            BranchCampaignRouteContinuationOriginV1,
            BranchCampaignRouteFirstEliteContinuationOriginV1,
            BranchCampaignRoutePathContinuationOriginV1,
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
                first_elite: Some(BranchCampaignRouteFirstEliteContinuationOriginV1 {
                    paths_with_first_elite: 2,
                    forced: false,
                    optional: true,
                    min_hallway_fights_before: 2,
                    max_hallway_fights_before: 3,
                    min_unknowns_before: 0,
                    max_unknowns_before: 1,
                    min_fires_before: 0,
                    max_fires_before: 1,
                    min_shops_before: 0,
                    max_shops_before: 1,
                    can_bail_to_rest_before: true,
                    can_bail_to_shop_before: true,
                }),
            }),
            milestone: "route_frontier".to_string(),
        };

        let lane = render_origin_target_lane_v1(&origin);

        assert!(lane.contains("route:go:MonsterRoom:CompleteWithinBudget"));
        assert!(lane.contains("optional_elite"));
        assert!(lane.contains("early_fire"));
        assert!(!lane.ends_with("room:Monster"));
    }
}
