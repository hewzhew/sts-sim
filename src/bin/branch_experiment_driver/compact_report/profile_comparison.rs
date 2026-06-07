use std::collections::{BTreeMap, BTreeSet};

use sts_simulator::eval::branch_experiment::{
    BranchExperimentBranchReportV1, BranchExperimentReportV1,
};
use sts_simulator::eval::branch_experiment_retention::BranchRetentionSlotV1;

pub(crate) fn render_profile_comparison(reports: &[BranchExperimentReportV1]) -> String {
    let mut lines = Vec::new();
    lines.push("Profile comparison:".to_string());
    if let Some(warning) = render_branch_point_warning(reports) {
        lines.push(warning);
    }
    if let Some(note) = render_retention_inactive_note(reports) {
        lines.push(note);
    }
    for report in reports {
        lines.push(format!(
            "  {} branch_points={} kept={} pruned={}{} lanes=[{}] deepest=A{}F{} hp={}{}",
            report.retention_profile,
            report.explored_branch_points,
            report.branches.len(),
            report.pruned_branch_count,
            render_profile_delta_suffix(report, reports.first()),
            render_report_lane_counts(report),
            deepest_act(report),
            deepest_floor(report),
            render_report_hp_range(report),
            render_pruned_long_horizon_suffix(report)
        ));
    }
    let unique_sections = render_profile_unique_branch_sections(reports);
    if !unique_sections.is_empty() {
        lines.push("".to_string());
        lines.extend(unique_sections);
    }
    lines.join("\n")
}

fn render_branch_point_warning(reports: &[BranchExperimentReportV1]) -> Option<String> {
    if reports.is_empty() {
        return None;
    }
    if reports
        .iter()
        .all(|report| report.explored_branch_points == 0)
    {
        return Some(
            "Warning: no compared profile reached a branch point; provide a prefix/trace that reaches a branchable decision or increase search automation budget"
                .to_string(),
        );
    }

    let first = reports.first()?.explored_branch_points;
    if reports
        .iter()
        .all(|report| report.explored_branch_points == first)
    {
        return None;
    }
    let branch_points = reports
        .iter()
        .map(|report| {
            format!(
                "{}={}",
                report.retention_profile, report.explored_branch_points
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    Some(format!(
        "Warning: compared profiles reached different branch-point counts after the shared start; later frontier depth differs, so compare unique branches as exploratory evidence. branch_points=[{branch_points}]"
    ))
}

fn render_retention_inactive_note(reports: &[BranchExperimentReportV1]) -> Option<String> {
    let baseline = reports.first()?;
    if reports.len() < 2 || reports.iter().any(|report| report.pruned_branch_count > 0) {
        return None;
    }
    let baseline_keys = report_branch_keys(baseline)
        .into_iter()
        .collect::<BTreeSet<_>>();
    if reports.iter().skip(1).all(|report| {
        report_branch_keys(report)
            .into_iter()
            .collect::<BTreeSet<_>>()
            == baseline_keys
    }) {
        Some(
            "Note: retention budget did not bind; profile differences cannot change the kept branch set in this run"
                .to_string(),
        )
    } else {
        None
    }
}

fn render_profile_delta_suffix(
    report: &BranchExperimentReportV1,
    baseline: Option<&BranchExperimentReportV1>,
) -> String {
    let Some(baseline) = baseline else {
        return String::new();
    };
    if report.retention_profile == baseline.retention_profile {
        return String::new();
    }

    let baseline_keys = report_branch_keys(baseline)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let report_keys = report_branch_keys(report)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let shared = report_keys.intersection(&baseline_keys).count();
    let unique = report_keys.difference(&baseline_keys).count();
    let missing = baseline_keys.difference(&report_keys).count();
    format!(
        " vs_{}=shared:{shared} unique:{unique} missing:{missing}",
        baseline.retention_profile
    )
}

fn render_pruned_long_horizon_suffix(report: &BranchExperimentReportV1) -> String {
    let primary =
        super::pruned::long_horizon_slot_counts(&report.pruned_branch_summary.primary_slot_counts);
    if report.pruned_branch_count == 0 || primary.is_empty() {
        return String::new();
    }
    format!(
        " pruned_long=[{}]",
        super::render_retention_slot_counts(&primary)
    )
}

fn render_report_lane_counts(report: &BranchExperimentReportV1) -> String {
    let lanes = report
        .branches
        .iter()
        .map(|branch| branch.retention.selected_by_slot)
        .collect::<Vec<_>>();
    super::render_retention_lane_count_payload(&lanes).unwrap_or_else(|| "-".to_string())
}

fn deepest_act(report: &BranchExperimentReportV1) -> u8 {
    report
        .branches
        .iter()
        .map(|branch| branch.summary.act)
        .max()
        .unwrap_or_default()
}

fn deepest_floor(report: &BranchExperimentReportV1) -> i32 {
    report
        .branches
        .iter()
        .map(|branch| branch.summary.floor)
        .max()
        .unwrap_or_default()
}

fn render_report_hp_range(report: &BranchExperimentReportV1) -> String {
    let Some(first) = report.branches.first() else {
        return "-".to_string();
    };
    let (min_hp, max_hp) = report.branches.iter().fold(
        (first.summary.hp, first.summary.hp),
        |(min_hp, max_hp), branch| (min_hp.min(branch.summary.hp), max_hp.max(branch.summary.hp)),
    );
    super::render_hp_range(min_hp, max_hp)
}

fn render_profile_unique_branch_sections(reports: &[BranchExperimentReportV1]) -> Vec<String> {
    let mut lines = Vec::new();
    for report in reports {
        let other_paths = reports
            .iter()
            .filter(|other| other.retention_profile != report.retention_profile)
            .flat_map(report_branch_keys)
            .collect::<BTreeSet<_>>();
        let unique_branches = report
            .branches
            .iter()
            .filter(|branch| !other_paths.contains(&branch_comparison_key(branch)))
            .take(4)
            .collect::<Vec<_>>();
        if unique_branches.is_empty() {
            continue;
        }
        lines.push(render_unique_branch_section_header(
            report.retention_profile.to_string(),
            &unique_branches,
        ));
        for branch in unique_branches {
            lines.push(format!("  - {}", render_comparison_branch_line(branch)));
        }
    }
    lines
}

fn render_unique_branch_section_header(
    profile_name: String,
    unique_branches: &[&BranchExperimentBranchReportV1],
) -> String {
    let mut lane_counts = BTreeMap::<BranchRetentionSlotV1, usize>::new();
    let mut package_counts = BTreeMap::<String, usize>::new();
    for branch in unique_branches {
        *lane_counts
            .entry(super::retention_lane(branch))
            .or_default() += 1;
        for package_state in super::branch_package_state_tags(branch) {
            *package_counts.entry(package_state).or_default() += 1;
        }
    }
    format!(
        "Only in {} ({} branch(es), lanes=[{}], packages=[{}]):",
        profile_name,
        unique_branches.len(),
        super::render_retention_slot_counts(&lane_counts),
        super::render_package_state_counts(&package_counts)
    )
}

fn report_branch_keys(report: &BranchExperimentReportV1) -> Vec<String> {
    report.branches.iter().map(branch_comparison_key).collect()
}

fn branch_comparison_key(branch: &BranchExperimentBranchReportV1) -> String {
    format!(
        "{}|A{}F{}|hp={}/{}|boundary={}",
        super::render_choice_path(branch),
        branch.summary.act,
        branch.summary.floor,
        branch.summary.hp,
        branch.summary.max_hp,
        branch.summary.boundary_title,
    )
}

fn render_comparison_branch_line(branch: &BranchExperimentBranchReportV1) -> String {
    format!(
        "{} | A{}F{} HP {}/{} | {} | lane={} | {}",
        super::render_choice_path(branch),
        branch.summary.act,
        branch.summary.floor,
        branch.summary.hp,
        branch.summary.max_hp,
        branch.summary.boundary_title,
        super::retention_slot_name(super::retention_lane(branch)),
        super::render_trajectory_summary(branch)
    )
}
