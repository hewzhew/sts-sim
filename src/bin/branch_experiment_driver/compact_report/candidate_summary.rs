use std::collections::{BTreeMap, BTreeSet};

use sts_simulator::eval::branch_experiment::{
    BranchExperimentBranchReportV1, BranchExperimentReportV1,
};
use sts_simulator::eval::branch_experiment_retention::BranchRetentionSlotV1;

use super::ChoiceFocus;

#[derive(Clone, Debug)]
struct FocusedCandidateSummary {
    label: String,
    branch_count: usize,
    prefix_contexts: BTreeMap<String, usize>,
    deepest_act: u8,
    deepest_floor: i32,
    min_hp: i32,
    max_hp: i32,
    retention_lanes: BTreeMap<BranchRetentionSlotV1, usize>,
    package_states: BTreeMap<String, usize>,
}

pub(crate) fn focused_candidate_summary_lines(
    report: &BranchExperimentReportV1,
    choice_focus: &ChoiceFocus,
) -> Vec<String> {
    if !choice_focus.is_targeted() {
        return Vec::new();
    }

    let mut summaries = BTreeMap::<String, FocusedCandidateSummary>::new();
    for branch in &report.branches {
        add_branch_summary(branch, choice_focus, &mut summaries);
    }

    let mut summaries = summaries.into_values().collect::<Vec<_>>();
    summaries.sort_by(|left, right| {
        right
            .branch_count
            .cmp(&left.branch_count)
            .then_with(|| right.prefix_contexts.len().cmp(&left.prefix_contexts.len()))
            .then_with(|| {
                (right.deepest_act, right.deepest_floor)
                    .cmp(&(left.deepest_act, left.deepest_floor))
            })
            .then_with(|| right.max_hp.cmp(&left.max_hp))
            .then_with(|| left.label.cmp(&right.label))
    });

    summaries
        .iter()
        .flat_map(render_candidate_summary)
        .collect()
}

fn add_branch_summary(
    branch: &BranchExperimentBranchReportV1,
    choice_focus: &ChoiceFocus,
    summaries: &mut BTreeMap<String, FocusedCandidateSummary>,
) {
    let Some(choice) = choice_focus.focused_choice(branch) else {
        return;
    };
    let label = super::choice_display_label(choice);
    let prefix_context = choice_focus
        .prefix_context_label(branch)
        .unwrap_or_else(|| "-".to_string());
    let entry = summaries
        .entry(label.clone())
        .or_insert_with(|| FocusedCandidateSummary {
            label,
            branch_count: 0,
            prefix_contexts: BTreeMap::new(),
            deepest_act: branch.summary.act,
            deepest_floor: branch.summary.floor,
            min_hp: branch.summary.hp,
            max_hp: branch.summary.hp,
            retention_lanes: BTreeMap::new(),
            package_states: BTreeMap::new(),
        });
    entry.branch_count += 1;
    *entry.prefix_contexts.entry(prefix_context).or_default() += 1;
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
    for state in super::branch_package_state_tags(branch) {
        *entry.package_states.entry(state).or_default() += 1;
    }
}

fn render_candidate_summary(summary: &FocusedCandidateSummary) -> Vec<String> {
    vec![
        format!(
            "  {} | branches={} prefix_contexts={} hp={} | lanes=[{}] | packages=[{}]",
            summary.label,
            summary.branch_count,
            visible_prefix_context_count(&summary.prefix_contexts),
            super::render_hp_range(summary.min_hp, summary.max_hp),
            super::render_retention_slot_counts(&summary.retention_lanes),
            super::render_package_state_counts(&summary.package_states),
        ),
        format!(
            "    contexts=[{}]",
            render_prefix_contexts(&summary.prefix_contexts)
        ),
    ]
}

fn visible_prefix_context_count(contexts: &BTreeMap<String, usize>) -> usize {
    contexts
        .keys()
        .filter(|context| context.as_str() != "-")
        .count()
}

fn render_prefix_contexts(contexts: &BTreeMap<String, usize>) -> String {
    let rendered = contexts
        .iter()
        .filter(|(context, _)| context.as_str() != "-")
        .map(|(context, count)| {
            if *count > 1 {
                format!("{context} x{count}")
            } else {
                context.clone()
            }
        })
        .collect::<BTreeSet<_>>();
    if rendered.is_empty() {
        "-".to_string()
    } else {
        rendered.into_iter().collect::<Vec<_>>().join(", ")
    }
}
