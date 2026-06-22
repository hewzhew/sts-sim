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
        frontier_title: frontier_title.to_string(),
        stop_reason: stop_reason.to_string(),
        choice_labels: choice_labels.to_vec(),
    }
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
}
