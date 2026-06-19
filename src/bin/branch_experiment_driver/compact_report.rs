use std::collections::{BTreeMap, BTreeSet};

use sts_simulator::ai::strategic::{compact_branch_signature, BranchSignature};
use sts_simulator::eval::branch_experiment::{
    branch_experiment_choice_effect_key_v1, BranchExperimentBranchReportV1,
    BranchExperimentBranchStatusV1, BranchExperimentChoiceV1, BranchExperimentReportV1,
    BranchExperimentRewardOptionPortfolioEntryV1, BranchExperimentRewardOptionPortfolioV1,
    BranchExperimentStrategyRequestV1, BranchExperimentWallLimitPhaseV1,
};
use sts_simulator::eval::branch_experiment_retention::BranchRetentionSlotV1;

mod candidate_summary;
mod first_pick;
mod focus;
mod profile_comparison;
mod pruned;

use candidate_summary::focused_candidate_summary_lines;
#[cfg(test)]
use first_pick::first_pick_outcome_summary_lines;
pub(super) use first_pick::{
    branch_package_state_tags, first_pick_outcome_summary_lines_with_focus,
};
use focus::ChoiceFocus;
pub(super) use profile_comparison::render_profile_comparison;
use pruned::{
    render_pruned_branch_summary_line, render_pruned_first_pick_count_line,
    render_pruned_long_horizon_coverage_note, render_pruned_next_experiment_line,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CompactReportOptions {
    pub kept_branch_examples: usize,
    pub focus_boundary: Option<String>,
}

impl Default for CompactReportOptions {
    fn default() -> Self {
        Self {
            kept_branch_examples: 5,
            focus_boundary: None,
        }
    }
}

pub(super) fn render_compact_report(report: &BranchExperimentReportV1) -> String {
    render_compact_report_with_options(report, CompactReportOptions::default())
}

pub(super) fn render_compact_report_with_options(
    report: &BranchExperimentReportV1,
    options: CompactReportOptions,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "BranchExperimentV1 seed={} depth={} max_branches={} branch_points={} kept={} pruned={} groups={} elapsed={}ms",
        report.seed,
        report.max_depth,
        report.max_branches,
        report.explored_branch_points,
        report.branches.len(),
        report.pruned_branch_count,
        report.frontier_groups.len(),
        report.elapsed_wall_ms
    ));
    let choice_focus = options
        .focus_boundary
        .as_deref()
        .map(|boundary| ChoiceFocus::for_target_boundary(report, boundary))
        .unwrap_or_else(|| ChoiceFocus::for_report(report));
    if report.branch_limit_hit {
        lines.push("branch limit hit: increase --max-branches or reduce --max-depth".to_string());
    }
    if report.frontier_group_limit_hit {
        lines.push(
            "frontier group cap hit: increase --max-per-frontier-group to keep more variants per same frontier"
                .to_string(),
        );
    }
    if report.wall_limit_hit {
        lines.push(render_wall_limit_line(report));
    }
    if let Some(path) = report.replay_trace_path.as_ref() {
        lines.push(format!(
            "replay trace: {} applied_steps={} stop={}",
            path,
            report.replay_trace_applied_steps,
            report
                .replay_trace_stop
                .as_deref()
                .unwrap_or("not_recorded")
        ));
    }
    lines.push(format!("retention_profile={}", report.retention_profile));
    let retention_lanes = report
        .branches
        .iter()
        .filter(|branch| choice_focus.branch_in_scope(branch))
        .map(|branch| branch.retention.selected_by_slot)
        .collect::<Vec<_>>();
    if let Some(line) = render_retention_lane_count_line(&retention_lanes) {
        lines.push(line);
    }
    if let Some(line) = render_choice_effect_count_line(report, &choice_focus) {
        lines.push(line);
    }
    if let Some(line) = render_kept_lineage_flag_count_line(report) {
        lines.push(line);
    }
    if let Some(line) = render_kept_long_horizon_coverage_line(report, &choice_focus) {
        lines.push(line);
    }
    if let Some(line) = render_pruned_first_pick_count_line(&report.pruned_first_pick_counts) {
        lines.push(line);
    }
    if let Some(line) = render_pruned_branch_summary_line(report) {
        lines.push(line);
    }
    if let Some(line) = render_pruned_long_horizon_coverage_note(report) {
        lines.push(line);
    }
    if let Some(line) = render_pruned_next_experiment_line(report) {
        lines.push(line);
    }
    if let Some(line) = choice_focus.notice_line() {
        lines.push(line);
    }
    if !report.strategy_requests.is_empty() {
        lines.push("".to_string());
        lines.push("Strategy requests:".to_string());
        for request in report.strategy_requests.iter().take(6) {
            lines.push(render_strategy_request_line(request));
        }
        if report.strategy_requests.len() > 6 {
            lines.push(format!(
                "  ... {} more strategy request(s); use --json or --out for full detail",
                report.strategy_requests.len() - 6
            ));
        }
    }
    let first_pick_outcomes = first_pick_outcome_summary_lines_with_focus(report, &choice_focus);
    if !first_pick_outcomes.is_empty() {
        lines.push("".to_string());
        if choice_focus.has_skipped_prefix() {
            lines.push("Focused-pick outcomes:".to_string());
        } else {
            lines.push("First-pick outcomes:".to_string());
        }
        for line in first_pick_outcomes.iter().take(8) {
            lines.push(line.clone());
        }
        if first_pick_outcomes.len() > 8 {
            lines.push(format!(
                "  ... {} more first-pick outcome group(s); use --json or --out for full detail",
                first_pick_outcomes.len() - 8
            ));
        }
    }
    let candidate_summaries = focused_candidate_summary_lines(report, &choice_focus);
    if !candidate_summaries.is_empty() {
        lines.push("".to_string());
        lines.push("Focused candidate summary:".to_string());
        for line in candidate_summaries.iter().take(12) {
            lines.push(line.clone());
        }
        if candidate_summaries.len() > 12 {
            lines.push(format!(
                "  ... {} more focused candidate summary line(s); use --json or --out for full detail",
                candidate_summaries.len() - 12
            ));
        }
    }
    if report.explored_branch_points == 0 {
        let boundary = report
            .frontier_groups
            .first()
            .map(|group| group.boundary_title.as_str())
            .unwrap_or("unknown boundary");
        lines.push(format!(
            "no branch point reached before {boundary}; provide --prefix commands or start from a trace/goto point that reaches the decision you want to explore"
        ));
    }
    if !report.reward_option_portfolios.is_empty() {
        lines.push("".to_string());
        lines.push("Reward option portfolios:".to_string());
        let summarized = summarize_reward_option_portfolios(&report.reward_option_portfolios);
        for summary in summarized.iter().take(4) {
            lines.push(format!(
                "  {} depth {}: kept {}/{} kept=[{}] pruned=[{}]{}",
                summary.boundary_title,
                summary.depth,
                summary.selected_count,
                summary.original_count,
                summary.kept,
                summary.pruned,
                if summary.count > 1 {
                    format!(" x{}", summary.count)
                } else {
                    String::new()
                },
            ));
        }
        if summarized.len() > 4 {
            lines.push(format!(
                "  ... {} more reward option portfolio event(s); use --json or --out for full detail",
                summarized.len() - 4
            ));
        }
    }
    lines.push("".to_string());
    lines.push("Frontier groups:".to_string());
    let mut rendered_group_count = 0usize;
    for group in &report.frontier_groups {
        let scoped_group_count = report
            .branches
            .iter()
            .filter(|branch| {
                branch.frontier.key == group.key && choice_focus.branch_in_scope(branch)
            })
            .count();
        if scoped_group_count == 0 {
            continue;
        }
        if rendered_group_count >= 8 {
            break;
        }
        rendered_group_count += 1;
        let offer = group
            .next_card_reward_offer
            .as_ref()
            .map(|cards| cards.join(", "))
            .unwrap_or_else(|| "-".to_string());
        let example = report
            .branches
            .iter()
            .find(|branch| branch.frontier.key == group.key && choice_focus.branch_in_scope(branch))
            .map(|branch| render_choice_path_with_focus(branch, &choice_focus))
            .unwrap_or_else(|| "-".to_string());
        let first_picks = render_group_first_picks(report, &group.key, &choice_focus);
        let lineage = render_lineage_flags(&group.lineage_flags);
        lines.push(format!(
            "  {:>2} branch(es) | {} | first_picks=[{}] | example: {} | next_reward=[{}]{}",
            scoped_group_count, group.boundary_title, first_picks, example, offer, lineage
        ));
    }
    if report.frontier_groups.len() > 8 {
        lines.push(format!(
            "  ... {} more group(s); use --json or --out for full detail",
            report.frontier_groups.len() - 8
        ));
    }
    if options.kept_branch_examples > 0 {
        lines.push("".to_string());
        lines.push("Kept branch examples:".to_string());
        for branch in ordered_branch_examples(report, &choice_focus)
            .into_iter()
            .take(options.kept_branch_examples)
        {
            lines.push(render_branch_line(branch, &choice_focus));
        }
        let scoped_branch_count = report
            .branches
            .iter()
            .filter(|branch| choice_focus.branch_in_scope(branch))
            .count();
        if scoped_branch_count > options.kept_branch_examples {
            lines.push(format!(
                "  ... {} more branch(es); use --branch-examples N, --json, or --out for full detail",
                scoped_branch_count - options.kept_branch_examples
            ));
        }
    }
    lines.join("\n")
}

fn render_wall_limit_line(report: &BranchExperimentReportV1) -> String {
    match report.wall_limit_phase {
        Some(BranchExperimentWallLimitPhaseV1::Expansion) => {
            "experiment wall-clock limit hit during expansion: increase --experiment-wall-ms for more exploration".to_string()
        }
        Some(BranchExperimentWallLimitPhaseV1::FinalSettle) => {
            "experiment wall-clock limit hit during final settle: retained branches may not all be advanced to their next frontier".to_string()
        }
        None => {
            "experiment wall-clock limit hit: increase --experiment-wall-ms for more exploration"
                .to_string()
        }
    }
}

fn render_strategy_request_line(request: &BranchExperimentStrategyRequestV1) -> String {
    let offer = request
        .next_card_reward_offer
        .as_ref()
        .map(|cards| cards.join(", "))
        .unwrap_or_else(|| "-".to_string());
    let examples = if request.examples.is_empty() {
        "-".to_string()
    } else {
        request.examples.join(" | ")
    };
    let reasons = if request.stop_reasons.is_empty() {
        "-".to_string()
    } else {
        request.stop_reasons.join(" | ")
    };
    let details = if request.boundary_details.is_empty() {
        "-".to_string()
    } else {
        request
            .boundary_details
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join(" | ")
    };
    format!(
        "  {} | {} branch(es) | A{}F{} {} | next=[{}] | ask: {} | examples=[{}] | details=[{}] | reason=[{}]",
        request.kind,
        request.branch_count,
        request.act,
        request.floor,
        request.boundary_title,
        offer,
        request.suggested_action,
        examples,
        details,
        reasons
    )
}

#[derive(Clone, Debug)]
struct RewardOptionPortfolioSummary {
    count: usize,
    depth: usize,
    boundary_title: String,
    original_count: usize,
    selected_count: usize,
    kept: String,
    pruned: String,
}

fn summarize_reward_option_portfolios(
    portfolios: &[BranchExperimentRewardOptionPortfolioV1],
) -> Vec<RewardOptionPortfolioSummary> {
    let mut groups = BTreeMap::<String, RewardOptionPortfolioSummary>::new();
    for portfolio in portfolios {
        let kept = render_reward_option_entries(&portfolio.selected_options);
        let pruned = render_reward_option_entries(&portfolio.pruned_options);
        let key = format!(
            "{}|{}|{}|{}|{}|{}",
            portfolio.boundary_title,
            portfolio.depth,
            portfolio.original_count,
            portfolio.selected_count,
            kept,
            pruned
        );
        groups
            .entry(key)
            .and_modify(|summary| summary.count += 1)
            .or_insert_with(|| RewardOptionPortfolioSummary {
                count: 1,
                depth: portfolio.depth,
                boundary_title: portfolio.boundary_title.clone(),
                original_count: portfolio.original_count,
                selected_count: portfolio.selected_count,
                kept,
                pruned,
            });
    }
    groups.into_values().collect()
}

fn render_reward_option_entries(
    entries: &[BranchExperimentRewardOptionPortfolioEntryV1],
) -> String {
    if entries.is_empty() {
        return "-".to_string();
    }
    entries
        .iter()
        .map(|entry| format!("{}:{}", entry.label, entry.semantic_class))
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_hp_range(min_hp: i32, max_hp: i32) -> String {
    if min_hp == max_hp {
        min_hp.to_string()
    } else {
        format!("{min_hp}-{max_hp}")
    }
}

fn render_package_state_counts(counts: &BTreeMap<String, usize>) -> String {
    if counts.is_empty() {
        return "-".to_string();
    }
    render_string_count_map(counts)
}

fn render_choice_effect_counts(counts: &BTreeMap<String, usize>) -> String {
    if counts.is_empty() {
        return "-".to_string();
    }
    CHOICE_EFFECT_DISPLAY_ORDER
        .iter()
        .filter_map(|effect| {
            counts
                .get(*effect)
                .filter(|count| **count > 0)
                .map(|count| format!("{effect}={count}"))
        })
        .chain(
            counts
                .iter()
                .filter(|(effect, _)| !CHOICE_EFFECT_DISPLAY_ORDER.contains(&effect.as_str()))
                .map(|(effect, count)| format!("{effect}={count}")),
        )
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_retention_slot_counts(counts: &BTreeMap<BranchRetentionSlotV1, usize>) -> String {
    RETENTION_SLOT_DISPLAY_ORDER
        .iter()
        .filter_map(|slot| {
            counts
                .get(slot)
                .filter(|count| **count > 0)
                .map(|count| format!("{}={count}", retention_slot_name(*slot)))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_choice_effect_count_line(
    report: &BranchExperimentReportV1,
    choice_focus: &ChoiceFocus,
) -> Option<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for branch in &report.branches {
        if !choice_focus.branch_in_scope(branch) {
            continue;
        }
        let branch_effects = choice_effect_keys(branch, choice_focus);
        for effect in branch_effects {
            *counts.entry(effect).or_default() += 1;
        }
    }
    if counts.is_empty() {
        return None;
    }
    Some(format!(
        "Kept choice effects: {}",
        render_choice_effect_counts(&counts)
    ))
}

fn choice_effect_keys(
    branch: &BranchExperimentBranchReportV1,
    choice_focus: &ChoiceFocus,
) -> BTreeSet<String> {
    if choice_focus.is_targeted() {
        return choice_focus
            .focused_choice(branch)
            .map(|choice| branch_experiment_choice_effect_key_v1(&choice.effect_kind))
            .into_iter()
            .collect();
    }
    branch
        .choices
        .iter()
        .map(|choice| branch_experiment_choice_effect_key_v1(&choice.effect_kind))
        .collect()
}

fn render_kept_lineage_flag_count_line(report: &BranchExperimentReportV1) -> Option<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for branch in &report.branches {
        for flag in &branch.frontier.lineage.sequence_breakers_present {
            if !flag.is_empty() {
                *counts.entry(flag.clone()).or_default() += 1;
            }
        }
    }
    if counts.is_empty() {
        return None;
    }
    Some(format!(
        "Kept lineage flags: {}",
        render_string_count_map(&counts)
    ))
}

fn render_string_count_map(counts: &BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(label, count)| format!("{label}={count}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn ordered_branch_examples<'a>(
    report: &'a BranchExperimentReportV1,
    choice_focus: &ChoiceFocus,
) -> Vec<&'a BranchExperimentBranchReportV1> {
    let mut ordered = Vec::new();
    let mut used_indices = BTreeSet::new();
    let mut covered_frontier_and_first_pick = BTreeSet::new();

    for (index, branch) in report.branches.iter().enumerate() {
        if !choice_focus.branch_in_scope(branch) {
            continue;
        }
        let key = branch_example_diversity_key(branch, choice_focus);
        if covered_frontier_and_first_pick.insert(key) {
            ordered.push(branch);
            used_indices.insert(index);
        }
    }

    for (index, branch) in report.branches.iter().enumerate() {
        if !choice_focus.branch_in_scope(branch) {
            continue;
        }
        if used_indices.insert(index) {
            ordered.push(branch);
        }
    }
    ordered
}

fn branch_example_diversity_key(
    branch: &BranchExperimentBranchReportV1,
    choice_focus: &ChoiceFocus,
) -> String {
    let first_pick = choice_focus
        .focused_choice(branch)
        .map(choice_display_label)
        .unwrap_or_else(|| "-".to_string());
    format!("{}|{first_pick}", branch.frontier.key)
}

fn render_branch_line(
    branch: &BranchExperimentBranchReportV1,
    choice_focus: &ChoiceFocus,
) -> String {
    let choices = render_choice_path_with_focus(branch, choice_focus);
    let next_reward = branch
        .frontier
        .next_card_reward_offer
        .as_ref()
        .map(|cards| cards.join(", "))
        .unwrap_or_else(|| "-".to_string());
    let status = branch_status_suffix(branch.status);
    let lane = retention_slot_name(retention_lane(branch));
    let retention = render_retention_slots(&branch.retention.slots);
    let formation = render_formation_summary(branch);
    let trajectory = render_trajectory_summary(branch);
    let context = render_retention_context_suffix(branch);
    let strategic = render_strategic_signature_suffix(&branch.retention.strategic_signature);
    format!(
        "  A{}F{} HP {}/{} gold {} | {}{} | {} | {}{} | lane={} keep=[{}]{} | choices: {} | next_reward=[{}]",
        branch.summary.act,
        branch.summary.floor,
        branch.summary.hp,
        branch.summary.max_hp,
        branch.summary.gold,
        branch.summary.boundary_title,
        status,
        formation,
        trajectory,
        context,
        lane,
        retention,
        strategic,
        choices,
        next_reward
    )
}

fn render_strategic_signature_suffix(signature: &BranchSignature) -> String {
    let summary = compact_branch_signature(signature);
    if summary.is_empty() {
        return String::new();
    }
    format!(" | strat=[{summary}]")
}

fn render_retention_context_suffix(branch: &BranchExperimentBranchReportV1) -> String {
    let reasons = branch
        .retention
        .reasons
        .iter()
        .filter_map(|reason| reason.strip_prefix("context: "))
        .take(3)
        .collect::<Vec<_>>();
    if reasons.is_empty() {
        String::new()
    } else {
        format!(" | ctx=[{}]", reasons.join("; "))
    }
}

fn render_formation_summary(branch: &BranchExperimentBranchReportV1) -> String {
    let strengths = branch
        .summary
        .formation_strengths
        .iter()
        .map(|strength| format!("{strength:?}"))
        .collect::<Vec<_>>();
    let needs = branch
        .summary
        .formation_needs
        .iter()
        .map(|need| format!("{need:?}"))
        .collect::<Vec<_>>();
    let strengths = if strengths.is_empty() {
        "-".to_string()
    } else {
        strengths.join("+")
    };
    let needs = if needs.is_empty() {
        "-".to_string()
    } else {
        needs.join("+")
    };
    format!(
        "formation={:?} strengths={} needs={}",
        branch.summary.formation_stage, strengths, needs
    )
}

fn render_trajectory_summary(branch: &BranchExperimentBranchReportV1) -> String {
    let trajectory = &branch.summary.trajectory;
    let setups = if trajectory.setup_keys.is_empty() {
        "-".to_string()
    } else {
        trajectory.setup_keys.join("+")
    };
    let packages = if trajectory.package_keys.is_empty() {
        "-".to_string()
    } else {
        trajectory.package_keys.join("+")
    };
    format!(
        "traj=setup:{} pkg:{} trans:{} eng:{}/{} def:{} de:{}",
        setups,
        packages,
        trajectory.transition_frontload_picks,
        trajectory.engine_generator_picks,
        trajectory.engine_payoff_picks,
        trajectory.defense_picks,
        trajectory.draw_energy_picks
    )
}

fn render_choice_path(branch: &BranchExperimentBranchReportV1) -> String {
    render_choice_path_with_focus(branch, &ChoiceFocus::default())
}

fn render_choice_path_with_focus(
    branch: &BranchExperimentBranchReportV1,
    choice_focus: &ChoiceFocus,
) -> String {
    if branch.choices.is_empty() {
        return "-".to_string();
    }
    choice_focus.render_choice_path(branch)
}

fn render_group_first_picks(
    report: &BranchExperimentReportV1,
    group_key: &str,
    choice_focus: &ChoiceFocus,
) -> String {
    let mut picks = Vec::<String>::new();
    for branch in &report.branches {
        if branch.frontier.key != group_key {
            continue;
        }
        let Some(choice) = choice_focus.focused_choice(branch) else {
            continue;
        };
        let label = choice_display_label(choice);
        if !picks.contains(&label) {
            picks.push(label);
        }
        if picks.len() >= 5 {
            break;
        }
    }
    if picks.is_empty() {
        "-".to_string()
    } else {
        picks.join(", ")
    }
}

fn choice_display_label(choice: &BranchExperimentChoiceV1) -> String {
    let base = if choice.effect_label.is_empty() {
        choice.label.clone()
    } else {
        choice.effect_label.clone()
    };
    if choice.representative_count > 1 {
        format!("{base} (covers {})", choice.representative_count)
    } else {
        base
    }
}

fn branch_status_suffix(status: BranchExperimentBranchStatusV1) -> &'static str {
    match status {
        BranchExperimentBranchStatusV1::NeedsHumanBoundary => "",
        BranchExperimentBranchStatusV1::Active => " [active]",
        BranchExperimentBranchStatusV1::TerminalVictory => " [victory]",
        BranchExperimentBranchStatusV1::TerminalDefeat => " [defeat]",
        BranchExperimentBranchStatusV1::Failed => " [failed]",
        BranchExperimentBranchStatusV1::Pruned => " [pruned]",
    }
}

fn render_lineage_flags(flags: &[String]) -> String {
    if flags.is_empty() {
        return String::new();
    }
    format!(" | lineage_flags=[{}]", flags.join(", "))
}

fn render_retention_slots(slots: &[BranchRetentionSlotV1]) -> String {
    slots
        .iter()
        .map(|slot| retention_slot_name(*slot))
        .collect::<Vec<_>>()
        .join(",")
}

fn retention_lane(branch: &BranchExperimentBranchReportV1) -> BranchRetentionSlotV1 {
    branch
        .retention
        .selected_by_slot
        .unwrap_or(BranchRetentionSlotV1::Diversity)
}

fn render_retention_lane_count_line(slots: &[Option<BranchRetentionSlotV1>]) -> Option<String> {
    render_retention_lane_count_payload(slots).map(|payload| format!("Retention lanes: {payload}"))
}

fn render_kept_long_horizon_coverage_line(
    report: &BranchExperimentReportV1,
    choice_focus: &ChoiceFocus,
) -> Option<String> {
    let mut counts = BTreeMap::<BranchRetentionSlotV1, usize>::new();
    let mut first_picks = BTreeSet::<String>::new();
    for branch in &report.branches {
        if !choice_focus.branch_in_scope(branch) {
            continue;
        }
        let slot = retention_lane(branch);
        if !is_long_horizon_slot(slot) {
            continue;
        }
        *counts.entry(slot).or_default() += 1;
        if let Some(choice) = choice_focus.focused_choice(branch) {
            first_picks.insert(choice_display_label(choice));
        }
    }
    if counts.is_empty() {
        return None;
    }
    let first_picks = if first_picks.is_empty() {
        "-".to_string()
    } else {
        first_picks.into_iter().collect::<Vec<_>>().join(", ")
    };
    Some(format!(
        "Long-horizon coverage: kept primary=[{}] first_picks=[{}]",
        render_retention_slot_counts(&counts),
        first_picks
    ))
}

fn is_long_horizon_slot(slot: BranchRetentionSlotV1) -> bool {
    matches!(
        slot,
        BranchRetentionSlotV1::Package
            | BranchRetentionSlotV1::EngineSetup
            | BranchRetentionSlotV1::Scaling
    )
}

fn render_retention_lane_count_payload(slots: &[Option<BranchRetentionSlotV1>]) -> Option<String> {
    let mut counts = BTreeMap::<BranchRetentionSlotV1, usize>::new();
    for slot in slots {
        *counts
            .entry(slot.unwrap_or(BranchRetentionSlotV1::Diversity))
            .or_default() += 1;
    }
    if counts.is_empty() {
        return None;
    }
    let rendered = RETENTION_SLOT_DISPLAY_ORDER
        .iter()
        .filter_map(|slot| {
            counts
                .get(slot)
                .filter(|count| **count > 0)
                .map(|count| format!("{}={count}", retention_slot_name(*slot)))
        })
        .collect::<Vec<_>>();
    Some(rendered.join(" "))
}

const RETENTION_SLOT_DISPLAY_ORDER: [BranchRetentionSlotV1; 8] = [
    BranchRetentionSlotV1::Package,
    BranchRetentionSlotV1::EngineSetup,
    BranchRetentionSlotV1::Scaling,
    BranchRetentionSlotV1::DefenseEngine,
    BranchRetentionSlotV1::Survival,
    BranchRetentionSlotV1::Frontload,
    BranchRetentionSlotV1::CleanDeck,
    BranchRetentionSlotV1::Diversity,
];

const CHOICE_EFFECT_DISPLAY_ORDER: [&str; 14] = [
    "take_card",
    "skip_reward",
    "singing_bowl",
    "remove_card",
    "transform_card",
    "upgrade_card",
    "duplicate_card",
    "bottle_card",
    "rest",
    "dig",
    "lift",
    "recall",
    "boss_relic",
    "event_choice",
];

fn retention_slot_name(slot: BranchRetentionSlotV1) -> &'static str {
    match slot {
        BranchRetentionSlotV1::Package => "package",
        BranchRetentionSlotV1::EngineSetup => "engine_setup",
        BranchRetentionSlotV1::Scaling => "scaling",
        BranchRetentionSlotV1::DefenseEngine => "defense",
        BranchRetentionSlotV1::Survival => "survival",
        BranchRetentionSlotV1::Frontload => "frontload",
        BranchRetentionSlotV1::CleanDeck => "clean",
        BranchRetentionSlotV1::Diversity => "diversity",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::noncombat_strategy_v1::StrategyDeckFormationStageV1;
    use sts_simulator::content::cards::CardId;
    use sts_simulator::eval::branch_experiment::{
        BranchExperimentChoiceV1, BranchExperimentFrontierGroupV1, BranchExperimentFrontierV1,
        BranchExperimentLineageV1, BranchExperimentPrunedFirstPickCountV1,
        BranchExperimentRunSummaryV1, BRANCH_EXPERIMENT_SCHEMA_VERSION,
    };
    use sts_simulator::eval::branch_experiment_retention::{
        BranchRetentionBudgetProfileV1, BranchRetentionDecisionV1,
    };

    #[test]
    fn compact_report_respects_kept_branch_example_limit() {
        let report = BranchExperimentReportV1 {
            branches: vec![
                branch_report(
                    "b0",
                    "Shockwave",
                    1,
                    3,
                    70,
                    BranchRetentionSlotV1::Package,
                    "Combat",
                ),
                branch_report(
                    "b1",
                    "Armaments",
                    1,
                    3,
                    71,
                    BranchRetentionSlotV1::DefenseEngine,
                    "Combat",
                ),
                branch_report(
                    "b2",
                    "Sever Soul",
                    1,
                    3,
                    72,
                    BranchRetentionSlotV1::EngineSetup,
                    "Combat",
                ),
            ],
            ..empty_report()
        };

        let rendered = render_compact_report_with_options(
            &report,
            CompactReportOptions {
                kept_branch_examples: 2,
                focus_boundary: None,
            },
        );

        assert_eq!(rendered.matches(" | choices: ").count(), 2);
        assert!(rendered.contains("... 1 more branch(es); use --branch-examples N"));
    }

    #[test]
    fn profile_comparison_renders_summary_and_unique_branch_examples() {
        let mut balanced = empty_report();
        balanced.retention_profile = BranchRetentionBudgetProfileV1::Balanced;
        balanced.pruned_branch_count = 3;
        balanced.pruned_branch_summary.primary_slot_counts = BTreeMap::from([
            (BranchRetentionSlotV1::EngineSetup, 1),
            (BranchRetentionSlotV1::Frontload, 2),
        ]);
        balanced.branches = vec![
            branch_report(
                "b0",
                "Shockwave",
                1,
                4,
                70,
                BranchRetentionSlotV1::Package,
                "Combat",
            ),
            branch_report(
                "b1",
                "Armaments",
                1,
                5,
                74,
                BranchRetentionSlotV1::Survival,
                "Campfire",
            ),
        ];
        let mut package = empty_report();
        package.retention_profile = BranchRetentionBudgetProfileV1::Package;
        package.pruned_branch_count = 1;
        let mut sever_soul = branch_report(
            "p1",
            "Sever Soul",
            1,
            6,
            65,
            BranchRetentionSlotV1::EngineSetup,
            "Campfire",
        );
        sever_soul
            .summary
            .trajectory
            .setup_keys
            .push("exhaust_engine".to_string());
        package.branches = vec![
            branch_report(
                "p0",
                "Shockwave",
                1,
                4,
                70,
                BranchRetentionSlotV1::Package,
                "Combat",
            ),
            sever_soul,
        ];

        let rendered = render_profile_comparison(&[balanced, package]);

        assert!(rendered.contains("Profile comparison:"));
        assert!(rendered.contains(
            "balanced branch_points=1 kept=2 pruned=3 lanes=[package=1 survival=1] deepest=A1F5 hp=70-74 pruned_long=[engine_setup=1]"
        ));
        assert!(rendered.contains(
            "package branch_points=1 kept=2 pruned=1 vs_balanced=shared:1 unique:1 missing:1 lanes=[package=1 engine_setup=1] deepest=A1F6 hp=65-70"
        ));
        assert!(
            rendered.contains("Only in balanced (1 branch(es), lanes=[survival=1], packages=[-]):")
        );
        assert!(rendered.contains("Armaments"));
        assert!(rendered.contains("lane=survival"));
        assert!(rendered.contains("traj=setup:- pkg:-"));
        assert!(rendered.contains(
            "Only in package (1 branch(es), lanes=[engine_setup=1], packages=[open:exhaust_engine=1]):"
        ));
        assert!(rendered.contains("Sever Soul"));
        assert!(rendered.contains("lane=engine_setup"));
    }

    #[test]
    fn profile_comparison_distinguishes_same_choices_with_different_outcomes() {
        let mut balanced = empty_report();
        balanced.retention_profile = BranchRetentionBudgetProfileV1::Balanced;
        balanced.branches = vec![branch_report(
            "b0",
            "Shockwave",
            1,
            4,
            70,
            BranchRetentionSlotV1::Package,
            "Combat",
        )];

        let mut package = empty_report();
        package.retention_profile = BranchRetentionBudgetProfileV1::Package;
        package.branches = vec![branch_report(
            "p0",
            "Shockwave",
            1,
            4,
            50,
            BranchRetentionSlotV1::Package,
            "Combat",
        )];

        let rendered = render_profile_comparison(&[balanced, package]);

        assert!(rendered.contains("vs_balanced=shared:0 unique:1 missing:1"));
        assert!(rendered.contains("Only in package"));
        assert!(rendered.contains("HP 50/80"));
    }

    #[test]
    fn profile_comparison_warns_when_branch_point_counts_differ() {
        let mut balanced = empty_report();
        balanced.retention_profile = BranchRetentionBudgetProfileV1::Balanced;
        balanced.explored_branch_points = 4;
        balanced.branches = vec![branch_report(
            "b0",
            "Shockwave",
            1,
            6,
            70,
            BranchRetentionSlotV1::Package,
            "Campfire",
        )];

        let mut exploration = empty_report();
        exploration.retention_profile = BranchRetentionBudgetProfileV1::Exploration;
        exploration.explored_branch_points = 0;
        exploration.branches = vec![branch_report(
            "e0",
            "-",
            1,
            1,
            56,
            BranchRetentionSlotV1::Diversity,
            "Combat",
        )];

        let rendered = render_profile_comparison(&[balanced, exploration]);

        assert!(rendered.contains(
            "Warning: compared profiles reached different branch-point counts after the shared start; later frontier depth differs, so compare unique branches as exploratory evidence"
        ));
        assert!(rendered.contains("branch_points=[balanced=4 exploration=0]"));
    }

    #[test]
    fn profile_comparison_warns_when_no_profile_reaches_branch_points() {
        let mut balanced = empty_report();
        balanced.retention_profile = BranchRetentionBudgetProfileV1::Balanced;
        balanced.explored_branch_points = 0;
        balanced.branches = vec![branch_report(
            "b0",
            "-",
            1,
            1,
            56,
            BranchRetentionSlotV1::Diversity,
            "Combat",
        )];

        let mut package = empty_report();
        package.retention_profile = BranchRetentionBudgetProfileV1::Package;
        package.explored_branch_points = 0;
        package.branches = vec![branch_report(
            "p0",
            "-",
            1,
            1,
            56,
            BranchRetentionSlotV1::Diversity,
            "Combat",
        )];

        let rendered = render_profile_comparison(&[balanced, package]);

        assert!(rendered.contains(
            "Warning: no compared profile reached a branch point; provide a prefix/trace that reaches a branchable decision or increase search automation budget"
        ));
    }

    #[test]
    fn profile_comparison_notes_when_retention_budget_does_not_bind() {
        let mut balanced = empty_report();
        balanced.retention_profile = BranchRetentionBudgetProfileV1::Balanced;
        balanced.explored_branch_points = 3;
        balanced.branches = vec![branch_report(
            "b0",
            "Shockwave",
            1,
            3,
            80,
            BranchRetentionSlotV1::Package,
            "Combat",
        )];

        let mut survival = balanced.clone();
        survival.retention_profile = BranchRetentionBudgetProfileV1::Survival;

        let rendered = render_profile_comparison(&[balanced, survival]);

        assert!(rendered.contains(
            "Note: retention budget did not bind; profile differences cannot change the kept branch set in this run"
        ));
    }

    #[test]
    fn compact_report_renders_engine_setup_retention_slot() {
        let rendered = render_retention_slots(&[
            BranchRetentionSlotV1::EngineSetup,
            BranchRetentionSlotV1::Diversity,
        ]);

        assert_eq!(rendered, "engine_setup,diversity");
    }

    #[test]
    fn retention_lane_count_line_summarizes_actual_portfolio_budget() {
        let rendered = render_retention_lane_count_line(&[
            Some(BranchRetentionSlotV1::Package),
            Some(BranchRetentionSlotV1::Survival),
            Some(BranchRetentionSlotV1::Frontload),
            Some(BranchRetentionSlotV1::Survival),
            None,
        ])
        .expect("slot count line");

        assert_eq!(
            rendered,
            "Retention lanes: package=1 survival=2 frontload=1 diversity=1"
        );
    }

    #[test]
    fn compact_report_renders_context_retention_reasons_without_all_internal_reasons() {
        let mut branch = branch_report(
            "b0",
            "Pommel Strike",
            1,
            3,
            70,
            BranchRetentionSlotV1::Frontload,
            "Combat",
        );
        branch.retention.reasons = vec![
            "contains immediate combat output".to_string(),
            "context: matches current frontload need".to_string(),
        ];
        let report = BranchExperimentReportV1 {
            branches: vec![branch],
            ..empty_report()
        };

        let rendered = render_compact_report(&report);

        assert!(rendered.contains("ctx=[matches current frontload need]"));
        assert!(
            !rendered.contains("contains immediate combat output"),
            "compact output should not dump every internal retention reason"
        );
    }

    #[test]
    fn first_pick_outcome_summary_groups_kept_branch_results() {
        let report = BranchExperimentReportV1 {
            branches: vec![
                branch_report(
                    "b0",
                    "Shockwave",
                    1,
                    4,
                    70,
                    BranchRetentionSlotV1::Package,
                    "Combat",
                ),
                branch_report(
                    "b1",
                    "Shockwave",
                    1,
                    6,
                    60,
                    BranchRetentionSlotV1::EngineSetup,
                    "Campfire",
                ),
                branch_report(
                    "b2",
                    "Armaments",
                    1,
                    5,
                    76,
                    BranchRetentionSlotV1::DefenseEngine,
                    "Combat",
                ),
            ],
            ..empty_report()
        };

        let lines = first_pick_outcome_summary_lines(&report);

        assert_eq!(
            lines[0],
            "  Shockwave | branches=2 deepest=A1F6 hp=60-70 | lanes=[package=1 engine_setup=1] | packages=[-] | frontiers=[Campfire=1 Combat=1]"
        );
        assert_eq!(
            lines[1],
            "  Armaments | branches=1 deepest=A1F5 hp=76 | lanes=[defense=1] | packages=[-] | frontiers=[Combat=1]"
        );
    }

    #[test]
    fn compact_report_focuses_choices_after_shared_prefix() {
        let mut searing = branch_report(
            "b0",
            "Searing Blow",
            1,
            2,
            80,
            BranchRetentionSlotV1::Package,
            "Card Reward",
        );
        prepend_choice(&mut searing, "Proceed");
        let mut shrug = branch_report(
            "b1",
            "Shrug It Off",
            1,
            2,
            80,
            BranchRetentionSlotV1::EngineSetup,
            "Card Reward",
        );
        prepend_choice(&mut shrug, "Proceed");
        let report = BranchExperimentReportV1 {
            branches: vec![searing, shrug],
            frontier_groups: vec![BranchExperimentFrontierGroupV1 {
                key: "A1F2:Card Reward".to_string(),
                boundary_title: "Card Reward".to_string(),
                branch_count: 2,
                representative_branch_id: "b0".to_string(),
                next_card_reward_offer: None,
                lineage_flags: Vec::new(),
            }],
            ..empty_report()
        };

        let rendered = render_compact_report_with_options(
            &report,
            CompactReportOptions {
                kept_branch_examples: 2,
                focus_boundary: None,
            },
        );

        assert!(rendered.contains("Decision focus: skipped shared prefix [Proceed]"));
        assert!(rendered.contains("Focused-pick outcomes:"));
        assert!(rendered.contains("first_picks=[Searing Blow, Shrug It Off]"));
        assert!(rendered.contains("choices: Searing Blow"));
        assert!(rendered.contains("choices: Shrug It Off"));
        assert!(!rendered.contains("First-pick outcomes:"));
        assert!(!rendered.contains("choices: Proceed -> Searing Blow"));
    }

    #[test]
    fn compact_report_can_focus_a_target_boundary() {
        let mut searing = branch_report(
            "b0",
            "Searing Blow",
            1,
            2,
            80,
            BranchRetentionSlotV1::Package,
            "Card Reward",
        );
        prepend_choice(&mut searing, "Obtain a random rare card.");
        prepend_choice(&mut searing, "Proceed");
        let mut shrug = branch_report(
            "b1",
            "Shrug It Off",
            1,
            2,
            80,
            BranchRetentionSlotV1::EngineSetup,
            "Card Reward",
        );
        prepend_choice(&mut shrug, "Enemies in your next three combats have 1 HP.");
        prepend_choice(&mut shrug, "Proceed");
        let mut combat = branch_report(
            "b2",
            "Obtain a random Boss Relic. Lose your starter Relic.",
            1,
            1,
            80,
            BranchRetentionSlotV1::Diversity,
            "Combat",
        );
        combat.choices[0].kind = "event".to_string();
        combat.choices[0].effect_kind = "event_choice".to_string();
        let report = BranchExperimentReportV1 {
            branches: vec![searing, shrug, combat],
            frontier_groups: vec![
                BranchExperimentFrontierGroupV1 {
                    key: "A1F2:Card Reward".to_string(),
                    boundary_title: "Card Reward".to_string(),
                    branch_count: 2,
                    representative_branch_id: "b0".to_string(),
                    next_card_reward_offer: None,
                    lineage_flags: Vec::new(),
                },
                BranchExperimentFrontierGroupV1 {
                    key: "A1F1:Combat".to_string(),
                    boundary_title: "Combat".to_string(),
                    branch_count: 1,
                    representative_branch_id: "b2".to_string(),
                    next_card_reward_offer: None,
                    lineage_flags: Vec::new(),
                },
            ],
            ..empty_report()
        };

        let rendered = render_compact_report_with_options(
            &report,
            CompactReportOptions {
                kept_branch_examples: 5,
                focus_boundary: Some("Card Reward".to_string()),
            },
        );

        assert!(rendered.contains(
            "Decision focus: target boundary [Card Reward] matched 2/3 branch(es); skipped shared prefix [Proceed]"
        ));
        assert!(rendered.contains("Kept choice effects: take_card=2"));
        assert!(rendered.contains("Focused-pick outcomes:"));
        assert!(rendered.contains("Focused candidate summary:"));
        assert!(rendered.contains(
            "  Searing Blow | branches=1 prefix_contexts=1 hp=80 | lanes=[package=1] | packages=[-]"
        ));
        assert!(rendered.contains("    contexts=[Obtain a random rare card.]"));
        assert!(rendered.contains(
            "  Shrug It Off | branches=1 prefix_contexts=1 hp=80 | lanes=[engine_setup=1] | packages=[-]"
        ));
        assert!(rendered.contains("    contexts=[Enemies in your next three combats have 1 HP.]"));
        assert!(rendered.contains("  Searing Blow | branches=1"));
        assert!(rendered.contains("  Shrug It Off | branches=1"));
        assert!(rendered.contains("  2 branch(es) | Card Reward"));
        assert!(!rendered.contains("  1 branch(es) | Combat"));
        assert!(rendered.contains("choices: Searing Blow"));
        assert!(rendered.contains("choices: Shrug It Off"));
        assert!(!rendered.contains("choices: Proceed -> Obtain a random rare card."));
    }

    #[test]
    fn first_pick_outcome_summary_includes_generic_package_state() {
        let report = BranchExperimentReportV1 {
            branches: vec![
                branch_report_with_packages(
                    "b0",
                    "Body Slam",
                    &["block_engine", "exhaust_engine"],
                    &["block_engine", "upgrade_sink"],
                ),
                branch_report_with_packages("b1", "Body Slam", &["exhaust_engine"], &[]),
            ],
            ..empty_report()
        };

        let lines = first_pick_outcome_summary_lines(&report);

        assert_eq!(
            lines[0],
            "  Body Slam | branches=2 deepest=A1F5 hp=70 | lanes=[package=2] | packages=[closed:block_engine=1 open:exhaust_engine=2 payoff_only:upgrade_sink=1] | frontiers=[Combat=2]"
        );
    }

    #[test]
    fn compact_report_renders_pruned_first_pick_counts() {
        let report = BranchExperimentReportV1 {
            pruned_branch_count: 3,
            pruned_first_pick_counts: vec![
                BranchExperimentPrunedFirstPickCountV1 {
                    first_pick: "Armaments".to_string(),
                    count: 7,
                },
                BranchExperimentPrunedFirstPickCountV1 {
                    first_pick: "Shockwave".to_string(),
                    count: 3,
                },
            ],
            pruned_branch_summary:
                sts_simulator::eval::branch_experiment::BranchExperimentPrunedBranchSummaryV1 {
                    primary_slot_counts: BTreeMap::from([
                        (BranchRetentionSlotV1::EngineSetup, 1),
                        (BranchRetentionSlotV1::Frontload, 2),
                    ]),
                    eligible_slot_counts: BTreeMap::from([
                        (BranchRetentionSlotV1::EngineSetup, 1),
                        (BranchRetentionSlotV1::Frontload, 2),
                        (BranchRetentionSlotV1::Diversity, 3),
                    ]),
                    package_state_counts: BTreeMap::from([("open:exhaust_engine".to_string(), 1)]),
                    choice_effect_counts: BTreeMap::from([
                        ("take_card".to_string(), 2),
                        ("skip_reward".to_string(), 1),
                    ]),
                    lineage_flag_counts: BTreeMap::from([(
                        "question_card_reward_count_plus_1".to_string(),
                        2,
                    )]),
                },
            ..empty_report()
        };

        let rendered = render_compact_report(&report);

        assert!(rendered.contains("Pruned first picks: Armaments=7 Shockwave=3"));
        assert!(rendered.contains(
            "Pruned branch summary: primary=[engine_setup=1 frontload=2] eligible=[engine_setup=1 frontload=2 diversity=3] effects=[take_card=2 skip_reward=1] packages=[open:exhaust_engine=1] lineage=[question_card_reward_count_plus_1=2]"
        ));
        assert!(rendered.contains(
            "Coverage note: pruned long-horizon branches primary=[engine_setup=1] packages=[open:exhaust_engine=1]; use --compare-profiles or raise --max-branches before treating missing packages as evidence"
        ));
        assert!(rendered.contains(
            "Next experiment: add --compare-profiles --retention-profile balanced,package,survival, or retry with --max-branches 8"
        ));
    }

    #[test]
    fn compact_report_renders_strategy_requests() {
        let report = BranchExperimentReportV1 {
            strategy_requests: vec![BranchExperimentStrategyRequestV1 {
                kind: "shop_strategy".to_string(),
                boundary_title: "Shop".to_string(),
                branch_count: 3,
                representative_branch_id: "root.rp 0".to_string(),
                act: 1,
                floor: 7,
                stop_reasons: vec!["shop action requires human choice".to_string()],
                examples: vec!["Anger".to_string(), "Rampage".to_string()],
                next_card_reward_offer: None,
                boundary_details: vec![
                    "Shop: cards=1 relics=0 potions=0 purge_cost=75 purge_available=true"
                        .to_string(),
                    "card-0 | Pommel Strike | 50 gold | buy card 0".to_string(),
                ],
                suggested_action: "provide buy/remove/leave priorities for this shop state"
                    .to_string(),
            }],
            ..empty_report()
        };

        let rendered = render_compact_report(&report);

        assert!(rendered.contains("Strategy requests:"));
        assert!(rendered.contains("shop_strategy | 3 branch(es) | A1F7 Shop | next=[-]"));
        assert!(rendered.contains("ask: provide buy/remove/leave priorities for this shop state"));
        assert!(rendered.contains("examples=[Anger | Rampage]"));
        assert!(rendered.contains(
            "details=[Shop: cards=1 relics=0 potions=0 purge_cost=75 purge_available=true | card-0 | Pommel Strike | 50 gold | buy card 0]"
        ));
    }

    #[test]
    fn compact_report_distinguishes_final_settle_wall_limit() {
        let report = BranchExperimentReportV1 {
            wall_limit_hit: true,
            wall_limit_phase: Some(BranchExperimentWallLimitPhaseV1::FinalSettle),
            ..empty_report()
        };

        let rendered = render_compact_report(&report);

        assert!(rendered.contains("experiment wall-clock limit hit during final settle"));
        assert!(!rendered.contains("for more exploration"));
    }

    fn empty_report() -> BranchExperimentReportV1 {
        BranchExperimentReportV1 {
            schema_name: "BranchExperimentV1".to_string(),
            schema_version: BRANCH_EXPERIMENT_SCHEMA_VERSION,
            label_role: "diagnostic_not_teacher_label".to_string(),
            policy_quality_claim: false,
            seed: 1,
            replay_trace_path: None,
            replay_trace_applied_steps: 0,
            replay_trace_stop: None,
            max_branches: 4,
            max_depth: 1,
            retention_profile: BranchRetentionBudgetProfileV1::Balanced,
            explored_branch_points: 1,
            branch_limit_hit: false,
            frontier_group_limit_hit: false,
            wall_limit_hit: false,
            wall_limit_phase: None,
            elapsed_wall_ms: 0,
            pruned_branch_count: 0,
            pruned_first_pick_counts: Vec::new(),
            pruned_branch_summary: Default::default(),
            reward_option_portfolios: Vec::new(),
            strategy_requests: Vec::new(),
            route_decisions: Vec::new(),
            frontier_groups: Vec::new(),
            branches: Vec::new(),
        }
    }

    fn branch_report(
        branch_id: &str,
        first_pick: &str,
        act: u8,
        floor: i32,
        hp: i32,
        primary_slot: BranchRetentionSlotV1,
        boundary_title: &str,
    ) -> BranchExperimentBranchReportV1 {
        BranchExperimentBranchReportV1 {
            branch_id: branch_id.to_string(),
            status: BranchExperimentBranchStatusV1::Active,
            rank_key: 0,
            retention: BranchRetentionDecisionV1 {
                primary_slot,
                selected_by_slot: Some(primary_slot),
                slots: vec![primary_slot],
                reasons: Vec::new(),
                strategic_signature: Default::default(),
                coverage_selection: Default::default(),
                rank_adjustment: Default::default(),
            },
            choices: vec![BranchExperimentChoiceV1 {
                depth: 0,
                kind: "card_reward".to_string(),
                boundary_title: "Card Reward".to_string(),
                card: Some(CardId::Strike),
                upgrades: Some(0),
                selected_cards: Vec::new(),
                effect_kind: "add_card".to_string(),
                effect_key: "card_reward:add_card:Strike:0".to_string(),
                effect_label: first_pick.to_string(),
                representative_count: 1,
                suppressed_count: 0,
                decision_signal: None,
                label: first_pick.to_string(),
                command: "rp 0".to_string(),
            }],
            stop_reason: boundary_title.to_string(),
            summary: BranchExperimentRunSummaryV1 {
                act,
                floor,
                hp,
                max_hp: 80,
                gold: 120,
                deck_count: 12,
                relic_count: 1,
                potion_count: 0,
                formation_stage: StrategyDeckFormationStageV1::PlanSeeded,
                formation_needs: Vec::new(),
                formation_strengths: Vec::new(),
                trajectory: Default::default(),
                boundary_title: boundary_title.to_string(),
            },
            frontier: BranchExperimentFrontierV1 {
                key: format!("A{act}F{floor}:{boundary_title}"),
                act,
                floor,
                boundary_title: boundary_title.to_string(),
                card_rng_counter: 0,
                card_blizz_randomizer: 0,
                next_card_reward_offer: None,
                lineage: BranchExperimentLineageV1 {
                    visibility: "test".to_string(),
                    public_policy_input: false,
                    direct_pick_consumes_card_rng: false,
                    same_reward_offer_lineage_key: "test".to_string(),
                    reward_screen_context: "test".to_string(),
                    reward_count_modifiers: Vec::new(),
                    card_pool_modifiers: Vec::new(),
                    rarity_modifiers: Vec::new(),
                    preview_modifiers: Vec::new(),
                    sequence_breakers_present: Vec::new(),
                },
            },
            final_boss_combat_record: None,
            boundary_details: Vec::new(),
        }
    }

    fn prepend_choice(branch: &mut BranchExperimentBranchReportV1, label: &str) {
        branch.choices.insert(
            0,
            BranchExperimentChoiceV1 {
                depth: 0,
                kind: "event".to_string(),
                boundary_title: "Event".to_string(),
                card: None,
                upgrades: None,
                selected_cards: Vec::new(),
                effect_kind: "event_choice".to_string(),
                effect_key: "event_choice".to_string(),
                effect_label: label.to_string(),
                representative_count: 1,
                suppressed_count: 0,
                decision_signal: None,
                label: label.to_string(),
                command: "event 0".to_string(),
            },
        );
    }

    fn branch_report_with_packages(
        branch_id: &str,
        first_pick: &str,
        setup_keys: &[&str],
        package_keys: &[&str],
    ) -> BranchExperimentBranchReportV1 {
        let mut branch = branch_report(
            branch_id,
            first_pick,
            1,
            5,
            70,
            BranchRetentionSlotV1::Package,
            "Combat",
        );
        branch.summary.trajectory.setup_keys =
            setup_keys.iter().map(|key| key.to_string()).collect();
        branch.summary.trajectory.package_keys =
            package_keys.iter().map(|key| key.to_string()).collect();
        branch
    }
}
