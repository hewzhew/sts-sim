use std::collections::BTreeMap;

use sts_simulator::eval::branch_experiment::{
    BranchExperimentPrunedFirstPickCountV1, BranchExperimentReportV1,
};
use sts_simulator::eval::branch_experiment_retention::BranchRetentionSlotV1;

pub(super) fn render_pruned_first_pick_count_line(
    counts: &[BranchExperimentPrunedFirstPickCountV1],
) -> Option<String> {
    if counts.is_empty() {
        return None;
    }
    let rendered = counts
        .iter()
        .take(8)
        .map(|entry| format!("{}={}", entry.first_pick, entry.count))
        .collect::<Vec<_>>()
        .join(" ");
    let suffix = if counts.len() > 8 {
        format!(" ... {} more", counts.len() - 8)
    } else {
        String::new()
    };
    Some(format!("Pruned first picks: {rendered}{suffix}"))
}

pub(super) fn render_pruned_branch_summary_line(
    report: &BranchExperimentReportV1,
) -> Option<String> {
    if report.pruned_branch_count == 0 {
        return None;
    }
    let summary = &report.pruned_branch_summary;
    Some(format!(
        "Pruned branch summary: primary=[{}] eligible=[{}] packages=[{}]",
        super::render_retention_slot_counts(&summary.primary_slot_counts),
        super::render_retention_slot_counts(&summary.eligible_slot_counts),
        super::render_package_state_counts(&summary.package_state_counts)
    ))
}

pub(super) fn render_pruned_long_horizon_coverage_note(
    report: &BranchExperimentReportV1,
) -> Option<String> {
    if report.pruned_branch_count == 0 {
        return None;
    }
    let primary = long_horizon_slot_counts(&report.pruned_branch_summary.primary_slot_counts);
    if primary.is_empty() {
        return None;
    }
    Some(format!(
        "Coverage note: pruned long-horizon branches primary=[{}] packages=[{}]; use --compare-profiles or raise --max-branches before treating missing packages as evidence",
        super::render_retention_slot_counts(&primary),
        super::render_package_state_counts(&report.pruned_branch_summary.package_state_counts)
    ))
}

fn long_horizon_slot_counts(
    counts: &BTreeMap<BranchRetentionSlotV1, usize>,
) -> BTreeMap<BranchRetentionSlotV1, usize> {
    [
        BranchRetentionSlotV1::Package,
        BranchRetentionSlotV1::EngineSetup,
        BranchRetentionSlotV1::Scaling,
    ]
    .into_iter()
    .filter_map(|slot| counts.get(&slot).copied().map(|count| (slot, count)))
    .filter(|(_, count)| *count > 0)
    .collect()
}
