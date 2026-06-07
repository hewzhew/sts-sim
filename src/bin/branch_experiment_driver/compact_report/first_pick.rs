use std::collections::BTreeMap;

use sts_simulator::eval::branch_experiment::{
    BranchExperimentBranchReportV1, BranchExperimentReportV1,
};
use sts_simulator::eval::branch_experiment_retention::BranchRetentionSlotV1;

#[derive(Clone, Debug)]
struct FirstPickOutcomeSummary {
    label: String,
    branch_count: usize,
    deepest_act: u8,
    deepest_floor: i32,
    min_hp: i32,
    max_hp: i32,
    retention_lanes: BTreeMap<BranchRetentionSlotV1, usize>,
    package_states: BTreeMap<String, usize>,
    frontiers: BTreeMap<String, usize>,
}

pub(crate) fn first_pick_outcome_summary_lines(report: &BranchExperimentReportV1) -> Vec<String> {
    let mut summaries = BTreeMap::<String, FirstPickOutcomeSummary>::new();
    for branch in &report.branches {
        let Some(first_choice) = branch.choices.first() else {
            continue;
        };
        let entry = summaries
            .entry(super::choice_display_label(first_choice))
            .or_insert_with(|| FirstPickOutcomeSummary {
                label: super::choice_display_label(first_choice),
                branch_count: 0,
                deepest_act: branch.summary.act,
                deepest_floor: branch.summary.floor,
                min_hp: branch.summary.hp,
                max_hp: branch.summary.hp,
                retention_lanes: BTreeMap::new(),
                package_states: BTreeMap::new(),
                frontiers: BTreeMap::new(),
            });
        entry.branch_count += 1;
        if (branch.summary.act, branch.summary.floor) > (entry.deepest_act, entry.deepest_floor) {
            entry.deepest_act = branch.summary.act;
            entry.deepest_floor = branch.summary.floor;
        }
        entry.min_hp = entry.min_hp.min(branch.summary.hp);
        entry.max_hp = entry.max_hp.max(branch.summary.hp);
        *entry
            .retention_lanes
            .entry(super::retention_lane(branch))
            .or_default() += 1;
        for state in branch_package_state_tags(branch) {
            *entry.package_states.entry(state).or_default() += 1;
        }
        *entry
            .frontiers
            .entry(branch.summary.boundary_title.clone())
            .or_default() += 1;
    }

    let mut summaries = summaries.into_values().collect::<Vec<_>>();
    summaries.sort_by(|left, right| {
        right
            .branch_count
            .cmp(&left.branch_count)
            .then_with(|| {
                (right.deepest_act, right.deepest_floor)
                    .cmp(&(left.deepest_act, left.deepest_floor))
            })
            .then_with(|| right.max_hp.cmp(&left.max_hp))
            .then_with(|| left.label.cmp(&right.label))
    });
    summaries.iter().map(render_first_pick_outcome).collect()
}

fn render_first_pick_outcome(summary: &FirstPickOutcomeSummary) -> String {
    format!(
        "  {} | branches={} deepest=A{}F{} hp={} | lanes=[{}] | packages=[{}] | frontiers=[{}]",
        summary.label,
        summary.branch_count,
        summary.deepest_act,
        summary.deepest_floor,
        super::render_hp_range(summary.min_hp, summary.max_hp),
        super::render_retention_slot_counts(&summary.retention_lanes),
        super::render_package_state_counts(&summary.package_states),
        super::render_string_count_map(&summary.frontiers)
    )
}

pub(crate) fn branch_package_state_tags(branch: &BranchExperimentBranchReportV1) -> Vec<String> {
    let setup_keys = branch
        .summary
        .trajectory
        .setup_keys
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let package_keys = branch
        .summary
        .trajectory
        .package_keys
        .iter()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let mut tags = Vec::new();
    for key in setup_keys.intersection(&package_keys) {
        tags.push(format!("closed:{key}"));
    }
    for key in setup_keys.difference(&package_keys) {
        tags.push(format!("open:{key}"));
    }
    for key in package_keys.difference(&setup_keys) {
        tags.push(format!("payoff_only:{key}"));
    }
    tags
}
