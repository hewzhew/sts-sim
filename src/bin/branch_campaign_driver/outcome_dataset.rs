use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use sts_simulator::eval::branch_campaign::{
    render_branch_campaign_compact_with_detail_v1,
    run_branch_campaign_from_report_with_checkpoint_v1, BranchCampaignBranchStatusV1,
    BranchCampaignBranchV1, BranchCampaignCheckpointV1, BranchCampaignContinuationOriginV1,
    BranchCampaignReportV1, BranchCampaignRouteContinuationOriginV1,
    BranchCampaignRouteFirstEliteContinuationOriginV1, BranchCampaignRoutePathContinuationOriginV1,
};
use sts_simulator::eval::branch_outcome_dataset_v1::{
    analyze_branch_outcome_records_v1, extract_branch_outcome_records_v1,
    parse_branch_outcome_records_jsonl_v1, render_branch_outcome_dataset_analysis_v1,
    serialize_branch_outcome_records_jsonl_v1, summarize_branch_outcome_records_v1,
    BranchOutcomeRecordV1,
};
use sts_simulator::eval::decision_path::decision_path_commands_include_decision_parent_coordinate_v1;
use sts_simulator::eval::learning_dataset_v1::{
    analyze_continuation_effect_v1, analyze_journal_decision_candidate_coverage_v1,
    analyze_learning_decision_outcome_samples_v1, coverage_gap_continuation_execution_plan_v1,
    coverage_gap_continuation_target_lane_v1, decision_outcome_samples_from_campaign_report_v1,
    learning_records_from_branch_outcomes_v1, parse_learning_decision_outcome_samples_jsonl_v1,
    plan_coverage_gap_continuations_v1, plan_targeted_continuations_v1,
    probe_learning_readiness_v1, refresh_coverage_gap_execution_bucket_summaries_v1,
    render_continuation_effect_report_v1, render_coverage_gap_continuation_plan_v1,
    render_coverage_gap_execution_plan_v1, render_journal_decision_candidate_coverage_v1,
    render_learning_decision_outcome_analysis_v1, render_learning_readiness_probe_v1,
    render_targeted_continuation_plan_v1, serialize_learning_branch_samples_jsonl_v1,
    serialize_learning_decision_outcome_samples_jsonl_v1, targeted_continuation_execution_plan_v1,
    CoverageGapContinuationExecutionPlanV1, CoverageGapContinuationPlanV1,
    CoverageGapContinuationTargetV1, LearningBranchSampleV1, LearningDatasetExportContextV1,
    LearningDecisionOutcomeSampleV1, TargetedContinuationExecutionPlanV1,
};
#[cfg(test)]
use sts_simulator::eval::learning_dataset_v1::{
    CoverageGapRouteFirstEliteOriginV1, CoverageGapRoutePathOriginV1,
    CoverageGapRouteTargetOriginV1,
};
use sts_simulator::eval::neow_guided_prefix::{
    neow_guided_prefix_commands_v1, NeowGuidedPrefixConfigV1,
};
use sts_simulator::eval::run_control::canonical_player_class;

use super::campaign_artifacts::{
    read_campaign_checkpoint_v1, read_campaign_report_v1, write_campaign_checkpoint_v1,
    write_campaign_report_v1,
};
use super::command_inputs::{
    render_round_budget_resolution_v1, ContinuationCommandInput, DatasetCommandInput,
};

pub(super) fn run_branch_outcome_dataset_analysis(
    input: &DatasetCommandInput,
) -> Result<(), String> {
    let path = input
        .analyze_outcome_dataset
        .as_ref()
        .ok_or_else(|| "--analyze-outcome-dataset requires a path".to_string())?;
    let text = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read --analyze-outcome-dataset {}: {err}",
            path.display()
        )
    })?;
    let records = parse_branch_outcome_records_jsonl_v1(&text)?;
    let analysis = analyze_branch_outcome_records_v1(&records);
    println!("{}", render_branch_outcome_dataset_analysis_v1(&analysis));
    Ok(())
}

pub(super) fn run_decision_outcome_dataset_analysis(
    input: &DatasetCommandInput,
) -> Result<(), String> {
    let path = input
        .analyze_decision_outcome_dataset
        .as_ref()
        .ok_or_else(|| "--analyze-decision-outcome-dataset requires a path".to_string())?;
    let text = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read --analyze-decision-outcome-dataset {}: {err}",
            path.display()
        )
    })?;
    let samples = parse_learning_decision_outcome_samples_jsonl_v1(&text)?;
    let analysis = analyze_learning_decision_outcome_samples_v1(&samples);
    println!(
        "{}",
        render_learning_decision_outcome_analysis_v1(&analysis)
    );
    Ok(())
}

pub(super) fn run_continuation_effect_report(
    input: &ContinuationCommandInput,
) -> Result<(), String> {
    let before_path = input
        .continuation_effect_before
        .as_ref()
        .ok_or_else(|| "--continuation-effect-before requires a path".to_string())?;
    let after_path = input
        .continuation_effect_after
        .as_ref()
        .ok_or_else(|| "--continuation-effect-after requires a path".to_string())?;
    let before_text = fs::read_to_string(before_path).map_err(|err| {
        format!(
            "failed to read --continuation-effect-before {}: {err}",
            before_path.display()
        )
    })?;
    let after_text = fs::read_to_string(after_path).map_err(|err| {
        format!(
            "failed to read --continuation-effect-after {}: {err}",
            after_path.display()
        )
    })?;
    let before_samples = parse_learning_decision_outcome_samples_jsonl_v1(&before_text)?;
    let after_samples = parse_learning_decision_outcome_samples_jsonl_v1(&after_text)?;
    let report = analyze_continuation_effect_v1(&before_samples, &after_samples);
    println!("{}", render_continuation_effect_report_v1(&report));
    Ok(())
}

pub(super) fn run_learning_readiness_probe(input: &DatasetCommandInput) -> Result<(), String> {
    let path = input
        .probe_learning_readiness
        .as_ref()
        .ok_or_else(|| "--probe-learning-readiness requires a path".to_string())?;
    let text = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read --probe-learning-readiness {}: {err}",
            path.display()
        )
    })?;
    let samples = parse_learning_decision_outcome_samples_jsonl_v1(&text)?;
    let probe = probe_learning_readiness_v1(&samples);
    println!("{}", render_learning_readiness_probe_v1(&probe));
    Ok(())
}

pub(super) fn run_targeted_continuation_plan(
    input: &ContinuationCommandInput,
) -> Result<(), String> {
    let path = input
        .plan_targeted_continuation
        .as_ref()
        .ok_or_else(|| "--plan-targeted-continuation requires a path".to_string())?;
    let text = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read --plan-targeted-continuation {}: {err}",
            path.display()
        )
    })?;
    let samples = parse_learning_decision_outcome_samples_jsonl_v1(&text)?;
    let plan = plan_targeted_continuations_v1(&samples);
    println!("{}", render_targeted_continuation_plan_v1(&plan));
    Ok(())
}

pub(super) fn run_targeted_continuation_execution(
    input: &ContinuationCommandInput,
) -> Result<(), String> {
    let samples_path = input
        .execute_targeted_continuation
        .as_ref()
        .ok_or_else(|| "--execute-targeted-continuation requires a path".to_string())?;
    let report_path = input
        .resume
        .as_ref()
        .ok_or_else(|| "--execute-targeted-continuation requires --resume PATH".to_string())?;
    let checkpoint_path = input.resume_checkpoint.as_ref().ok_or_else(|| {
        "--execute-targeted-continuation requires --resume-checkpoint PATH".to_string()
    })?;

    let source_report = read_campaign_report_v1(report_path)?;
    let source_checkpoint = read_campaign_checkpoint_v1(checkpoint_path)?;
    let text = fs::read_to_string(samples_path).map_err(|err| {
        format!(
            "failed to read --execute-targeted-continuation {}: {err}",
            samples_path.display()
        )
    })?;
    let samples = parse_learning_decision_outcome_samples_jsonl_v1(&text)?;
    let plan = plan_targeted_continuations_v1(&samples);
    let execution = targeted_continuation_execution_plan_v1(
        &plan,
        &source_report,
        input.targeted_continuation_limit,
        input.targeted_continuation_candidates_per_target,
    );
    if execution.branches.is_empty() {
        return Err(format!(
            "targeted continuation selected no executable branches (targets={} missing={} skipped={})",
            execution.requested_target_count,
            execution.missing_branch_count,
            execution.skipped_candidate_count
        ));
    }

    let continuation_report = continuation_source_report_v1(&source_report, &execution)
        .ok_or_else(|| {
            "targeted continuation selected branches but none were present in the source report"
                .to_string()
        })?;
    let mut config = input.config.clone();
    let round_budget = input
        .round_budget
        .resolve_for_source_rounds(source_report.rounds_completed)?;
    config.max_rounds = round_budget.round_budget;
    config.seed = source_report.seed;
    config.ascension_level = source_report.run_domain.ascension_level;
    config.player_class = canonical_player_class(&source_report.run_domain.player_class)?;
    config.prefix_commands.clear();

    let result = run_branch_campaign_from_report_with_checkpoint_v1(
        &config,
        &continuation_report,
        Some(&source_checkpoint),
    )?;
    if let Some(path) = input.out.as_ref() {
        write_campaign_report_v1(path, &result.report)?;
    }
    if let Some(path) = input.checkpoint_out.as_ref() {
        write_campaign_checkpoint_v1(path, &result.checkpoint)?;
    }

    println!(
        "TargetedContinuationExecutionV1 targets={} selected={} missing={} skipped={}",
        execution.requested_target_count,
        execution.selected_branch_count,
        execution.missing_branch_count,
        execution.skipped_candidate_count
    );
    println!("{}", render_round_budget_resolution_v1(round_budget));
    println!(
        "{}",
        render_branch_campaign_compact_with_detail_v1(
            &result.report,
            input.branch_examples,
            input.report_detail,
        )
    );
    Ok(())
}

pub(super) fn run_coverage_gap_continuation_plan(
    input: &DatasetCommandInput,
) -> Result<(), String> {
    if !input.plan_coverage_gap_continuation {
        return Err("--plan-coverage-gap-continuation is not enabled".to_string());
    }
    let report_path = input.inspect_report.as_ref().ok_or_else(|| {
        "--plan-coverage-gap-continuation requires --inspect-report PATH".to_string()
    })?;
    let report = read_campaign_report_v1(report_path)?;
    let checkpoint_path = input
        .inspect_checkpoint
        .as_ref()
        .or(input.resume_checkpoint.as_ref());
    let checkpoint = checkpoint_path
        .map(read_campaign_checkpoint_v1)
        .transpose()?;
    let records = extract_branch_outcome_records_v1(&report, checkpoint.as_ref())?;
    if let Some(checkpoint) = checkpoint.as_ref() {
        let (plan, replayable_preview, planning_window) =
            build_replayable_coverage_gap_execution_plan_v1(
                &report,
                &records,
                checkpoint,
                input.coverage_gap_limit,
                input.coverage_gap_candidates_per_decision,
            );
        println!(
            "Replayable preview from current checkpoint (requested={} planning_window={}):\n{}",
            input.coverage_gap_limit,
            planning_window,
            render_coverage_gap_execution_plan_v1(&replayable_preview)
        );
        println!("{}", render_coverage_gap_continuation_plan_v1(&plan));
    } else {
        let plan = plan_coverage_gap_continuations_v1(
            &report,
            &records,
            input.coverage_gap_limit,
            input.coverage_gap_candidates_per_decision,
        );
        println!("{}", render_coverage_gap_continuation_plan_v1(&plan));
    }
    Ok(())
}

pub(super) fn run_coverage_gap_continuation_execution(
    input: &ContinuationCommandInput,
) -> Result<(), String> {
    if !input.execute_coverage_gap_continuation {
        return Err("--execute-coverage-gap-continuation is not enabled".to_string());
    }
    let report_path = input
        .resume
        .as_ref()
        .ok_or_else(|| "--execute-coverage-gap-continuation requires --resume PATH".to_string())?;
    let checkpoint_path = input.resume_checkpoint.as_ref().ok_or_else(|| {
        "--execute-coverage-gap-continuation requires --resume-checkpoint PATH".to_string()
    })?;
    let source_report = read_campaign_report_v1(report_path)?;
    let source_checkpoint = read_campaign_checkpoint_v1(checkpoint_path)?;
    let records = extract_branch_outcome_records_v1(&source_report, Some(&source_checkpoint))?;
    let (plan, execution, planning_window) = build_replayable_coverage_gap_execution_plan_v1(
        &source_report,
        &records,
        &source_checkpoint,
        input.coverage_gap_limit,
        input.coverage_gap_candidates_per_decision,
    );
    if execution.targets.is_empty() {
        return Err(format!(
            "coverage gap continuation selected no replayable candidate branches (decisions={} unobserved={} requested={} skipped={}); run a campaign/checkpoint that preserves decision-parent snapshots before continuing these targets",
            plan.total_decisions,
            plan.total_unobserved_candidates,
            execution.requested_target_count,
            execution.skipped_target_count
        ));
    }

    let continuation_report =
        coverage_gap_continuation_source_report_v1(&source_report, &execution);
    let mut config = input.config.clone();
    let round_budget = input
        .round_budget
        .resolve_for_source_rounds(source_report.rounds_completed)?;
    config.max_rounds = round_budget.round_budget;
    let use_neow_guided_prefix =
        source_report.run_prelude.is_empty() && !config.prefix_commands.is_empty();
    config.seed = source_report.seed;
    config.ascension_level = source_report.run_domain.ascension_level;
    config.player_class = canonical_player_class(&source_report.run_domain.player_class)?;
    config.prefix_commands = if use_neow_guided_prefix {
        // Backward compatibility for reports written before BranchCampaignRunPreludeV1.
        coverage_gap_source_prefix_commands_v1(&config)?
    } else if source_report.run_prelude.is_empty() {
        Vec::new()
    } else {
        source_report.run_prelude.prefix_commands.clone()
    };

    let result = run_branch_campaign_from_report_with_checkpoint_v1(
        &config,
        &continuation_report,
        Some(&source_checkpoint),
    )?;
    if let Some(path) = input.out.as_ref() {
        write_campaign_report_v1(path, &result.report)?;
    }
    if let Some(path) = input.checkpoint_out.as_ref() {
        write_campaign_checkpoint_v1(path, &result.checkpoint)?;
    }

    println!(
        "CoverageGapContinuationExecutionV1 requested={} planning_window={} selected={} skipped={}",
        execution.requested_target_count,
        planning_window,
        execution.selected_branch_count,
        execution.skipped_target_count
    );
    println!("{}", render_round_budget_resolution_v1(round_budget));
    println!("{}", render_coverage_gap_execution_plan_v1(&execution));
    println!("{}", render_coverage_gap_continuation_plan_v1(&plan));
    println!(
        "{}",
        render_coverage_gap_result_audit_v1(&execution, &result.report)
    );
    println!(
        "{}",
        render_branch_campaign_compact_with_detail_v1(
            &result.report,
            input.branch_examples,
            input.report_detail,
        )
    );
    Ok(())
}

fn build_replayable_coverage_gap_execution_plan_v1(
    source_report: &BranchCampaignReportV1,
    records: &[BranchOutcomeRecordV1],
    source_checkpoint: &BranchCampaignCheckpointV1,
    requested_targets: usize,
    max_candidates_per_decision: usize,
) -> (
    CoverageGapContinuationPlanV1,
    CoverageGapContinuationExecutionPlanV1,
    usize,
) {
    let mut planning_window = requested_targets;
    let max_planning_window = coverage_gap_execution_planning_window_cap_v1(requested_targets);

    loop {
        let plan = plan_coverage_gap_continuations_v1(
            source_report,
            records,
            planning_window,
            max_candidates_per_decision,
        );
        let execution = trim_coverage_gap_execution_plan_v1(
            filter_coverage_gap_execution_plan_for_checkpoint_v1(
                coverage_gap_continuation_execution_plan_v1(&plan, planning_window),
                source_checkpoint,
            ),
            requested_targets,
        );
        let selected_enough = execution.selected_branch_count >= requested_targets;
        let exhausted_planned_targets = plan.targets.len() < planning_window;

        if requested_targets == 0
            || selected_enough
            || exhausted_planned_targets
            || planning_window >= max_planning_window
        {
            break (plan, execution, planning_window);
        }
        planning_window =
            next_coverage_gap_execution_planning_window_v1(planning_window, max_planning_window);
    }
}

fn continuation_source_report_v1(
    source_report: &BranchCampaignReportV1,
    execution: &TargetedContinuationExecutionPlanV1,
) -> Option<BranchCampaignReportV1> {
    let mut selected = Vec::new();
    for request in &execution.branches {
        if let Some(mut branch) =
            find_campaign_branch_by_id_v1(source_report, &request.representative_branch_id).cloned()
        {
            branch.status = BranchCampaignBranchStatusV1::Active;
            branch.stop_reason = format!("targeted continuation to {}", request.milestone);
            selected.push(branch);
        }
    }
    if selected.is_empty() {
        return None;
    }

    let mut report = source_report.clone();
    report.stop_reason = "targeted_continuation_seed".to_string();
    report.active = selected;
    report.frozen.clear();
    report.victories.clear();
    report.dead.clear();
    report.abandoned.clear();
    report.stuck.clear();
    report.strategy_requests.clear();
    Some(report)
}

fn coverage_gap_continuation_source_report_v1(
    source_report: &BranchCampaignReportV1,
    execution: &CoverageGapContinuationExecutionPlanV1,
) -> BranchCampaignReportV1 {
    let active = execution
        .targets
        .iter()
        .map(coverage_gap_branch_from_target_v1)
        .collect::<Vec<_>>();
    let mut report = source_report.clone();
    report.stop_reason = "coverage_gap_continuation_seed".to_string();
    report.active = active;
    report.frozen.clear();
    report.victories.clear();
    report.dead.clear();
    report.abandoned.clear();
    report.stuck.clear();
    report.strategy_requests.clear();
    report
}

fn coverage_gap_branch_from_target_v1(
    target: &CoverageGapContinuationTargetV1,
) -> BranchCampaignBranchV1 {
    let mut commands = target.parent_commands.clone();
    commands.push(target.command.clone());
    let mut choice_labels = target.parent_choices.clone();
    choice_labels.push(target.label.clone());
    BranchCampaignBranchV1 {
        branch_id: branch_id_from_commands_v1(&commands),
        commands,
        choice_labels,
        summary: None,
        strategic_summary: Default::default(),
        frontier_title: format!("Coverage Gap: {}", target.event_type),
        status: BranchCampaignBranchStatusV1::Active,
        stop_reason: format!("coverage gap continuation from {}", target.decision_id),
        continuation_origin: Some(BranchCampaignContinuationOriginV1 {
            kind: "coverage_gap".to_string(),
            source_event_id: target.event_id.clone(),
            decision_id: target.decision_id.clone(),
            event_type: target.event_type.clone(),
            parent_branch_id: target.parent_branch_id.clone(),
            parent_frontier_title: target.parent_frontier_title.clone(),
            candidate_index: target.candidate_index,
            candidate_id: target.candidate_id.clone(),
            command: target.command.clone(),
            label: target.label.clone(),
            semantic_class: target.semantic_class.clone(),
            admission: target.admission.clone(),
            disposition: target.disposition,
            target_origin_source: target.target_origin.source.clone(),
            route_origin: target.target_origin.route.as_ref().map(|route| {
                BranchCampaignRouteContinuationOriginV1 {
                    legal_candidate_count: route.legal_candidate_count,
                    emitted_candidate_count: route.emitted_candidate_count,
                    complete_legal_pool: route.complete_legal_pool,
                    ordering: route.ordering.clone(),
                    ordering_kind: route.ordering_kind,
                    target_x: route.target_x,
                    target_y: route.target_y,
                    target_node: route.target_node.clone(),
                    room_type: route.room_type.clone(),
                    move_kind: route.move_kind.clone(),
                    action_kind: route.action_kind.clone(),
                    action: route.action.clone(),
                    projection_source: route.projection_source.clone(),
                    projection_source_kind: route.projection_source_kind,
                    projection_coverage: route.projection_coverage.clone(),
                    projection_coverage_kind: route.projection_coverage_kind,
                    path_budget: route.path_budget,
                    observed_path_count: route.observed_path_count,
                    path: Some(BranchCampaignRoutePathContinuationOriginV1 {
                        path_count: route.path.path_count,
                        path_budget_exhausted: route.path.path_budget_exhausted,
                        min_early_pressure: route.path.min_early_pressure,
                        max_early_pressure: route.path.max_early_pressure,
                        min_elites: route.path.min_elites,
                        max_elites: route.path.max_elites,
                        min_shops: route.path.min_shops,
                        max_shops: route.path.max_shops,
                        min_fires: route.path.min_fires,
                        max_fires: route.path.max_fires,
                        min_unknowns: route.path.min_unknowns,
                        max_unknowns: route.path.max_unknowns,
                        min_treasures: route.path.min_treasures,
                        max_treasures: route.path.max_treasures,
                        first_shop_floor: route.path.first_shop_floor,
                        first_fire_floor: route.path.first_fire_floor,
                        min_damage_rooms_before_recovery: route
                            .path
                            .min_damage_rooms_before_recovery,
                        max_damage_rooms_before_recovery: route
                            .path
                            .max_damage_rooms_before_recovery,
                        min_unknowns_before_recovery: route.path.min_unknowns_before_recovery,
                        max_unknowns_before_recovery: route.path.max_unknowns_before_recovery,
                        paths_with_recovery_before_damage: route
                            .path
                            .paths_with_recovery_before_damage,
                    }),
                    first_elite: Some(BranchCampaignRouteFirstEliteContinuationOriginV1 {
                        paths_with_first_elite: route.first_elite.paths_with_first_elite,
                        forced: route.first_elite.forced,
                        optional: route.first_elite.optional,
                        min_hallway_fights_before: route.first_elite.min_hallway_fights_before,
                        max_hallway_fights_before: route.first_elite.max_hallway_fights_before,
                        min_unknowns_before: route.first_elite.min_unknowns_before,
                        max_unknowns_before: route.first_elite.max_unknowns_before,
                        min_fires_before: route.first_elite.min_fires_before,
                        max_fires_before: route.first_elite.max_fires_before,
                        min_shops_before: route.first_elite.min_shops_before,
                        max_shops_before: route.first_elite.max_shops_before,
                        can_bail_to_rest_before: route.first_elite.can_bail_to_rest_before,
                        can_bail_to_shop_before: route.first_elite.can_bail_to_shop_before,
                    }),
                }
            }),
            milestone: target.milestone.clone(),
        }),
        lineage_decision_signal_rank_adjustment: 0,
        rank_key: 0,
        final_boss_combat_record: None,
        combat_lab_probes: Vec::new(),
    }
}

fn filter_coverage_gap_execution_plan_for_checkpoint_v1(
    mut execution: CoverageGapContinuationExecutionPlanV1,
    checkpoint: &BranchCampaignCheckpointV1,
) -> CoverageGapContinuationExecutionPlanV1 {
    let requested = execution.requested_target_count;
    let original_selected = execution.targets.len();
    execution.targets = execution
        .targets
        .into_iter()
        .filter(|target| {
            if coverage_gap_target_requires_exact_parent_snapshot_v1(target) {
                return coverage_gap_parent_commands_have_exact_coordinate_v1(
                    &target.parent_commands,
                ) && checkpoint_has_exact_session_v1(checkpoint, &target.parent_commands);
            }
            checkpoint_can_replay_parent_commands_v1(checkpoint, &target.parent_commands)
        })
        .collect();
    execution.selected_branch_count = execution.targets.len();
    execution.skipped_target_count = execution
        .skipped_target_count
        .saturating_add(original_selected.saturating_sub(execution.targets.len()));
    execution.requested_target_count = requested;
    refresh_coverage_gap_execution_bucket_summaries_v1(&mut execution);
    execution
}

fn coverage_gap_execution_planning_window_cap_v1(requested_targets: usize) -> usize {
    requested_targets.saturating_mul(16).max(requested_targets)
}

fn next_coverage_gap_execution_planning_window_v1(
    current_window: usize,
    max_window: usize,
) -> usize {
    current_window
        .saturating_mul(2)
        .max(current_window.saturating_add(1))
        .min(max_window)
}

fn trim_coverage_gap_execution_plan_v1(
    mut execution: CoverageGapContinuationExecutionPlanV1,
    requested_targets: usize,
) -> CoverageGapContinuationExecutionPlanV1 {
    let overflow = execution.targets.len().saturating_sub(requested_targets);
    if overflow > 0 {
        execution.targets.truncate(requested_targets);
        execution.skipped_target_count = execution.skipped_target_count.saturating_add(overflow);
    }
    execution.requested_target_count = requested_targets;
    execution.selected_branch_count = execution.targets.len();
    refresh_coverage_gap_execution_bucket_summaries_v1(&mut execution);
    execution
}

fn coverage_gap_target_requires_exact_parent_snapshot_v1(
    target: &CoverageGapContinuationTargetV1,
) -> bool {
    // Coverage gap targets describe a specific decision surface, not merely a
    // command prefix. Auto-run may route and fight between recorded choices, so a
    // replayable ancestor can land on the wrong screen even when the command path
    // exists. Root decisions are safe to replay from a fresh seed; all later
    // decision surfaces need an exact parent snapshot.
    !target.parent_commands.is_empty()
}

fn coverage_gap_parent_commands_have_exact_coordinate_v1(parent_commands: &[String]) -> bool {
    decision_path_commands_include_decision_parent_coordinate_v1(parent_commands)
}

fn checkpoint_has_exact_session_v1(
    checkpoint: &BranchCampaignCheckpointV1,
    parent_commands: &[String],
) -> bool {
    checkpoint
        .sessions
        .iter()
        .any(|session| session.commands == parent_commands)
}

fn checkpoint_can_replay_parent_commands_v1(
    checkpoint: &BranchCampaignCheckpointV1,
    parent_commands: &[String],
) -> bool {
    let session_commands = checkpoint
        .sessions
        .iter()
        .map(|session| session.commands.clone())
        .collect::<BTreeSet<_>>();
    if session_commands.contains(parent_commands) {
        return true;
    }

    let nodes_by_commands = checkpoint
        .nodes
        .iter()
        .map(|node| (node.commands.clone(), node.node_id))
        .collect::<BTreeMap<_, _>>();
    let nodes_by_id = checkpoint
        .nodes
        .iter()
        .map(|node| (node.node_id, node))
        .collect::<BTreeMap<_, _>>();
    let Some(mut current_id) = nodes_by_commands.get(parent_commands).copied() else {
        return false;
    };
    while let Some(node) = nodes_by_id.get(&current_id) {
        if session_commands.contains(&node.commands) {
            return true;
        }
        let Some(parent_id) = node.parent_id else {
            return false;
        };
        current_id = parent_id;
    }
    false
}

fn render_coverage_gap_result_audit_v1(
    execution: &CoverageGapContinuationExecutionPlanV1,
    report: &BranchCampaignReportV1,
) -> String {
    let branches = coverage_gap_result_branches_v1(report);
    let mut matched = 0usize;
    let mut missing = 0usize;
    let mut final_bucket_matched = 0usize;
    let mut discarded_matched = 0usize;
    let mut outcome_counts = BTreeMap::<(String, String, String), usize>::new();
    let mut target_lines = Vec::new();

    for (index, target) in execution.targets.iter().enumerate() {
        let lane = coverage_gap_continuation_target_lane_v1(target);
        if let Some(result) = branches
            .iter()
            .find(|result| coverage_gap_result_branch_matches_target_v1(result, target))
        {
            matched = matched.saturating_add(1);
            if result.outcome == "discarded" {
                discarded_matched = discarded_matched.saturating_add(1);
            } else {
                final_bucket_matched = final_bucket_matched.saturating_add(1);
            }
            *outcome_counts
                .entry((
                    target.event_type.clone(),
                    lane.clone(),
                    result.outcome.to_string(),
                ))
                .or_default() += 1;
            target_lines.push(format!(
                "  {}. {} {} {{{}}} lane={} seeded=yes final_bucket={}{} -> frontier={} {} stop={}",
                index + 1,
                target.event_type,
                compact_coverage_gap_audit_text_v1(&target.label, 40),
                compact_coverage_gap_audit_text_v1(&target.command, 24),
                compact_coverage_gap_audit_text_v1(&lane, 72),
                result.outcome,
                render_coverage_gap_discard_reason_suffix_v1(result),
                result.frontier_title,
                render_coverage_gap_branch_progress_v1(result.summary),
                compact_coverage_gap_audit_text_v1(result.stop_reason, 92)
            ));
        } else {
            missing = missing.saturating_add(1);
            let discarded_tracking = if report.discarded_count > 0 {
                " discarded_tracking=aggregate_only"
            } else {
                ""
            };
            target_lines.push(format!(
                "  {}. missing target {} {} {{{}}} lane={} seeded=yes final_bucket=missing diagnostic=not_in_final_buckets{} parent={}",
                index + 1,
                target.event_type,
                compact_coverage_gap_audit_text_v1(&target.label, 40),
                compact_coverage_gap_audit_text_v1(&target.command, 24),
                compact_coverage_gap_audit_text_v1(&lane, 72),
                discarded_tracking,
                compact_coverage_gap_audit_text_v1(&target.parent_branch_id, 48)
            ));
        }
    }

    let mut lines = Vec::new();
    lines.push(format!(
        "CoverageGapResultAuditV1 targets={} matched={} missing={}",
        execution.targets.len(),
        matched,
        missing
    ));
    lines.push(format!(
        "Lifecycle: seeded={} final_bucket_matched={} discarded_matched={} final_bucket_missing={} report_discarded={}",
        execution.targets.len(),
        final_bucket_matched,
        discarded_matched,
        missing,
        report.discarded_count
    ));
    if !outcome_counts.is_empty() {
        lines.push("Outcomes:".to_string());
        for ((event_type, lane, outcome), count) in outcome_counts {
            lines.push(format!(
                "  {} lane={} outcome={} count={}",
                event_type,
                compact_coverage_gap_audit_text_v1(&lane, 72),
                outcome,
                count
            ));
        }
    }
    if target_lines.is_empty() {
        lines.push("Targets: none".to_string());
    } else {
        lines.push("Targets:".to_string());
        lines.extend(target_lines);
    }
    lines.join("\n")
}

struct CoverageGapResultBranchRefV1<'a> {
    outcome: &'static str,
    frontier_title: &'a str,
    stop_reason: &'a str,
    summary: Option<&'a sts_simulator::eval::branch_campaign::BranchCampaignBranchSummaryV1>,
    continuation_origin: Option<&'a BranchCampaignContinuationOriginV1>,
    discard_reason: Option<&'a str>,
}

fn coverage_gap_result_branches_v1(
    report: &BranchCampaignReportV1,
) -> Vec<CoverageGapResultBranchRefV1<'_>> {
    let mut branches = Vec::new();
    branches.extend(
        report
            .active
            .iter()
            .map(|branch| coverage_gap_result_branch_ref_from_branch_v1("active", branch)),
    );
    branches.extend(
        report
            .frozen
            .iter()
            .map(|branch| coverage_gap_result_branch_ref_from_branch_v1("frozen", branch)),
    );
    branches.extend(
        report
            .victories
            .iter()
            .map(|branch| coverage_gap_result_branch_ref_from_branch_v1("victory", branch)),
    );
    branches.extend(
        report
            .dead
            .iter()
            .map(|branch| coverage_gap_result_branch_ref_from_branch_v1("dead", branch)),
    );
    branches.extend(
        report
            .abandoned
            .iter()
            .map(|branch| coverage_gap_result_branch_ref_from_branch_v1("abandoned", branch)),
    );
    branches.extend(
        report
            .stuck
            .iter()
            .map(|branch| coverage_gap_result_branch_ref_from_branch_v1("stuck", branch)),
    );
    branches.extend(
        report
            .discarded_branches
            .iter()
            .map(|branch| CoverageGapResultBranchRefV1 {
                outcome: "discarded",
                frontier_title: &branch.frontier_title,
                stop_reason: &branch.stop_reason,
                summary: branch.summary.as_ref(),
                continuation_origin: branch.continuation_origin.as_ref(),
                discard_reason: Some(&branch.reason),
            }),
    );
    branches
}

fn coverage_gap_result_branch_ref_from_branch_v1<'a>(
    outcome: &'static str,
    branch: &'a BranchCampaignBranchV1,
) -> CoverageGapResultBranchRefV1<'a> {
    CoverageGapResultBranchRefV1 {
        outcome,
        frontier_title: &branch.frontier_title,
        stop_reason: &branch.stop_reason,
        summary: branch.summary.as_ref(),
        continuation_origin: branch.continuation_origin.as_ref(),
        discard_reason: None,
    }
}

fn coverage_gap_result_branch_matches_target_v1(
    branch: &CoverageGapResultBranchRefV1<'_>,
    target: &CoverageGapContinuationTargetV1,
) -> bool {
    let Some(origin) = branch.continuation_origin else {
        return false;
    };
    origin.kind == "coverage_gap"
        && origin.decision_id == target.decision_id
        && origin.source_event_id == target.event_id
        && origin.candidate_id == target.candidate_id
        && origin.candidate_index == target.candidate_index
        && origin.command == target.command
}

fn render_coverage_gap_branch_progress_v1(
    summary: Option<&sts_simulator::eval::branch_campaign::BranchCampaignBranchSummaryV1>,
) -> String {
    let Some(summary) = summary else {
        return "A?F? HP ?/? deck ?".to_string();
    };
    format!(
        "A{}F{} HP {}/{} deck {}",
        summary.act, summary.floor, summary.hp, summary.max_hp, summary.deck_count
    )
}

fn render_coverage_gap_discard_reason_suffix_v1(
    branch: &CoverageGapResultBranchRefV1<'_>,
) -> String {
    branch
        .discard_reason
        .map(|reason| format!(" discard_reason={reason}"))
        .unwrap_or_default()
}

fn compact_coverage_gap_audit_text_v1(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut compact = value
        .chars()
        .take(max_chars.saturating_sub(4))
        .collect::<String>();
    compact.push_str(" ...");
    compact
}

fn coverage_gap_source_prefix_commands_v1(
    config: &sts_simulator::eval::branch_campaign::BranchCampaignConfigV1,
) -> Result<Vec<String>, String> {
    neow_guided_prefix_commands_v1(&NeowGuidedPrefixConfigV1 {
        seed: config.seed,
        ascension_level: config.ascension_level,
        final_act: config.final_act,
        player_class: config.player_class,
        search_max_nodes: config.search_max_nodes,
        search_wall_ms: config.search_wall_ms,
    })
}

fn branch_id_from_commands_v1(commands: &[String]) -> String {
    if commands.is_empty() {
        "root".to_string()
    } else {
        format!("root.{}", commands.join("."))
    }
}

fn find_campaign_branch_by_id_v1<'a>(
    report: &'a BranchCampaignReportV1,
    branch_id: &str,
) -> Option<&'a BranchCampaignBranchV1> {
    report
        .active
        .iter()
        .chain(report.frozen.iter())
        .chain(report.abandoned.iter())
        .chain(report.stuck.iter())
        .find(|branch| branch.branch_id == branch_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::eval::branch_campaign::{
        BranchCampaignBranchSummaryV1, BranchCampaignDiscardedBranchV1,
    };
    use sts_simulator::eval::campaign_journal::{
        CampaignJournalCandidateAdmissionStatusV1, CampaignJournalCandidateAdmissionTraceV1,
        CampaignJournalCandidateDispositionV1,
    };

    #[test]
    fn coverage_gap_result_audit_links_targets_to_result_branches() {
        let reward_target = coverage_gap_test_target("reward", "rp 2", "Shrug It Off", 0);
        let route_target = coverage_gap_test_target("route", "go 1", "x=1 y=2 Shop", 1);
        let execution = CoverageGapContinuationExecutionPlanV1 {
            schema_name: "CoverageGapContinuationExecutionPlanV1".to_string(),
            schema_version: 3,
            label_role: "campaign_observation_not_teacher".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            requested_target_count: 2,
            selected_branch_count: 2,
            skipped_target_count: 0,
            bucket_summaries: Vec::new(),
            targets: vec![reward_target.clone(), route_target.clone()],
        };
        let mut report = BranchCampaignReportV1 {
            schema_name: "BranchCampaignV1".to_string(),
            schema_version: 1,
            seed: 1,
            run_domain: Default::default(),
            run_prelude: Default::default(),
            rounds_completed: 2,
            stop_reason: "max_rounds".to_string(),
            active: vec![coverage_gap_test_result_branch(
                &reward_target,
                BranchCampaignBranchStatusV1::Active,
                "Reward Screen",
                "advanced to reward",
                1,
                7,
                61,
                80,
            )],
            frozen: Vec::new(),
            victories: Vec::new(),
            dead: Vec::new(),
            abandoned: vec![coverage_gap_test_result_branch(
                &route_target,
                BranchCampaignBranchStatusV1::Abandoned,
                "Combat",
                "combat search did not find an executable complete win",
                1,
                6,
                44,
                80,
            )],
            stuck: Vec::new(),
            discarded_count: 0,
            discarded_examples: Vec::new(),
            discarded_branches: Vec::new(),
            strategy_requests: Vec::new(),
            route_evidence: Default::default(),
            combat_retry_ledger: Default::default(),
            strategic_signals: Default::default(),
            state_store: Default::default(),
            journal: Default::default(),
            rounds: Vec::new(),
        };

        let rendered = render_coverage_gap_result_audit_v1(&execution, &report);

        assert!(rendered.contains("CoverageGapResultAuditV1 targets=2 matched=2 missing=0"));
        assert!(rendered.contains(
            "Lifecycle: seeded=2 final_bucket_matched=2 discarded_matched=0 final_bucket_missing=0"
        ));
        assert!(rendered.contains("reward:scheduled:kept"));
        assert!(rendered.contains("seeded=yes final_bucket=active"));
        assert!(rendered.contains("frontier=Reward Screen"));
        assert!(rendered.contains("A1F7 HP 61/80"));
        assert!(rendered.contains("route:legacy:x=1 y=2 Shop"));
        assert!(rendered.contains("seeded=yes final_bucket=abandoned"));
        assert!(rendered.contains("frontier=Combat"));
        assert!(rendered.contains("combat search did not find an executable complete win"));

        report.abandoned.clear();
        report.discarded_count = 3;
        report.discarded_examples = vec!["some other branch".to_string()];
        let discarded_branch = coverage_gap_test_result_branch(
            &route_target,
            BranchCampaignBranchStatusV1::Frozen,
            "Map",
            "discarded by retention cap",
            1,
            6,
            44,
            80,
        );
        report.discarded_branches = vec![BranchCampaignDiscardedBranchV1::from_branch_v1(
            &discarded_branch,
            "selection_capacity",
        )];
        let rendered = render_coverage_gap_result_audit_v1(&execution, &report);
        assert!(rendered.contains("matched=2 missing=0"));
        assert!(rendered.contains(
            "Lifecycle: seeded=2 final_bucket_matched=1 discarded_matched=1 final_bucket_missing=0"
        ));
        assert!(rendered.contains("route x=1 y=2 Shop {go 1}"));
        assert!(rendered.contains("seeded=yes final_bucket=discarded"));
        assert!(rendered.contains("discard_reason=selection_capacity"));
    }

    #[test]
    fn coverage_gap_branch_records_structured_target_origin() {
        let target = CoverageGapContinuationTargetV1 {
            decision_id: "root:round1:reward0".to_string(),
            event_id: "root:round1:reward0:candidate_set".to_string(),
            event_type: "reward".to_string(),
            parent_branch_id: "root".to_string(),
            parent_frontier_title: "Card Reward".to_string(),
            parent_commands: vec!["rp 0".to_string()],
            parent_choices: vec!["Pommel Strike".to_string()],
            candidate_index: 2,
            candidate_id: "pruned:2:rp 2".to_string(),
            command: "rp 2".to_string(),
            label: "Shrug It Off".to_string(),
            semantic_class: "block".to_string(),
            admission: CampaignJournalCandidateAdmissionTraceV1::new(
                CampaignJournalCandidateAdmissionStatusV1::Deferred,
                "reward_portfolio",
                "pruned",
            ),
            disposition: CampaignJournalCandidateDispositionV1::Pruned,
            target_origin: Default::default(),
            milestone: "next_major_boundary".to_string(),
        };

        let branch = coverage_gap_branch_from_target_v1(&target);
        let origin = branch
            .continuation_origin
            .as_ref()
            .expect("coverage-gap branch should carry target origin");

        assert_eq!(origin.kind, "coverage_gap");
        assert_eq!(origin.decision_id, target.decision_id);
        assert_eq!(origin.source_event_id, target.event_id);
        assert_eq!(origin.event_type, target.event_type);
        assert_eq!(origin.candidate_id, target.candidate_id);
        assert_eq!(origin.candidate_index, target.candidate_index);
        assert_eq!(origin.command, target.command);
        assert_eq!(origin.label, target.label);
        assert_eq!(origin.milestone, target.milestone);
        assert_eq!(origin.admission.status, target.admission.status);
        assert_eq!(origin.disposition, target.disposition);
        assert!(origin.target_origin_source.is_empty());
        assert!(origin.route_origin.is_none());
    }

    #[test]
    fn coverage_gap_branch_preserves_route_path_and_first_elite_origin() {
        let target = CoverageGapContinuationTargetV1 {
            decision_id: "root:round1:route0".to_string(),
            event_id: "root:round1:route0:candidate_set".to_string(),
            event_type: "route".to_string(),
            parent_branch_id: "root".to_string(),
            parent_frontier_title: "Map".to_string(),
            parent_commands: Vec::new(),
            parent_choices: Vec::new(),
            candidate_index: 1,
            candidate_id: "route_move:normal_edge:x2:y3".to_string(),
            command: "go 2".to_string(),
            label: "x=2 y=3 Elite".to_string(),
            semantic_class: "route".to_string(),
            admission: CampaignJournalCandidateAdmissionTraceV1::new(
                CampaignJournalCandidateAdmissionStatusV1::Deferred,
                "route_candidate_pool",
                "deferred",
            ),
            disposition: CampaignJournalCandidateDispositionV1::Pruned,
            target_origin:
                sts_simulator::eval::learning_dataset_v1::CoverageGapContinuationTargetOriginV1 {
                    source: "route_candidate_pool".to_string(),
                    route: Some(CoverageGapRouteTargetOriginV1 {
                        legal_candidate_count: 4,
                        emitted_candidate_count: 4,
                        complete_legal_pool: true,
                        ordering: "SafetyThenScoreThenX".to_string(),
                        ordering_kind: Some(
                            sts_simulator::ai::route_planner_v1::RouteCandidateOrderingV1::SafetyThenScoreThenX,
                        ),
                        target_x: 2,
                        target_y: 3,
                        target_node: Some(sts_simulator::ai::route_planner_v1::MapRouteTargetV1 {
                            x: 2,
                            y: 3,
                            room_type: Some(sts_simulator::state::map::node::RoomType::MonsterRoomElite),
                            has_emerald_key: false,
                            move_kind: sts_simulator::ai::route_planner_v1::RouteMoveKindV1::NormalEdge,
                        }),
                        room_type: "Elite".to_string(),
                        move_kind: "NormalEdge".to_string(),
                        action_kind: "go".to_string(),
                        action: Some(sts_simulator::ai::route_planner_v1::RouteMapActionV1::Go {
                            x: 2,
                        }),
                        projection_source: "VisibleMapDfs".to_string(),
                        projection_source_kind: Some(
                            sts_simulator::ai::route_planner_v1::RouteProjectionSourceV1::VisibleMapDfs,
                        ),
                        projection_coverage: "CompleteWithinBudget".to_string(),
                        projection_coverage_kind: Some(
                            sts_simulator::ai::route_planner_v1::RouteProjectionCoverageV1::CompleteWithinBudget,
                        ),
                        path_budget: 2000,
                        observed_path_count: 17,
                        path: CoverageGapRoutePathOriginV1 {
                            path_count: 17,
                            path_budget_exhausted: false,
                            min_early_pressure: 2,
                            max_early_pressure: 5,
                            min_elites: 1,
                            max_elites: 3,
                            min_shops: 0,
                            max_shops: 2,
                            min_fires: 1,
                            max_fires: 3,
                            min_unknowns: 2,
                            max_unknowns: 6,
                            min_treasures: 1,
                            max_treasures: 1,
                            first_shop_floor: Some(5),
                            first_fire_floor: Some(6),
                            min_damage_rooms_before_recovery: 1,
                            max_damage_rooms_before_recovery: 4,
                            min_unknowns_before_recovery: 1,
                            max_unknowns_before_recovery: 2,
                            paths_with_recovery_before_damage: 3,
                        },
                        first_elite: CoverageGapRouteFirstEliteOriginV1 {
                            paths_with_first_elite: 12,
                            forced: false,
                            optional: true,
                            min_hallway_fights_before: 2,
                            max_hallway_fights_before: 4,
                            min_unknowns_before: 1,
                            max_unknowns_before: 3,
                            min_fires_before: 0,
                            max_fires_before: 1,
                            min_shops_before: 0,
                            max_shops_before: 1,
                            can_bail_to_rest_before: true,
                            can_bail_to_shop_before: true,
                        },
                    }),
                },
            milestone: "route_frontier".to_string(),
        };

        let branch = coverage_gap_branch_from_target_v1(&target);
        let route = branch
            .continuation_origin
            .as_ref()
            .and_then(|origin| origin.route_origin.as_ref())
            .expect("route coverage gap branch should preserve route origin");

        assert_eq!(route.target_x, 2);
        assert_eq!(
            route.ordering_kind,
            Some(
                sts_simulator::ai::route_planner_v1::RouteCandidateOrderingV1::SafetyThenScoreThenX
            )
        );
        assert_eq!(
            route
                .target_node
                .as_ref()
                .map(|target| (target.x, target.y, target.room_type)),
            Some((
                2,
                3,
                Some(sts_simulator::state::map::node::RoomType::MonsterRoomElite)
            ))
        );
        assert_eq!(
            route.action.as_ref(),
            Some(&sts_simulator::ai::route_planner_v1::RouteMapActionV1::Go { x: 2 })
        );
        assert_eq!(
            route.projection_source_kind,
            Some(sts_simulator::ai::route_planner_v1::RouteProjectionSourceV1::VisibleMapDfs)
        );
        assert_eq!(
            route.projection_coverage_kind,
            Some(
                sts_simulator::ai::route_planner_v1::RouteProjectionCoverageV1::CompleteWithinBudget
            )
        );
        assert_eq!(
            route.path.as_ref().expect("path should survive").path_count,
            17
        );
        assert_eq!(
            route
                .first_elite
                .as_ref()
                .expect("first elite should survive")
                .paths_with_first_elite,
            12
        );
    }

    #[test]
    fn coverage_gap_non_root_targets_require_exact_parent_snapshot() {
        let mut target = CoverageGapContinuationTargetV1 {
            decision_id: "root:round1:shop0".to_string(),
            event_id: "root:round1:shop0:candidate_set".to_string(),
            event_type: "shop".to_string(),
            parent_branch_id: "root.rp 0".to_string(),
            parent_frontier_title: "Shop".to_string(),
            parent_commands: vec!["rp 0".to_string()],
            parent_choices: vec!["Pommel Strike".to_string()],
            candidate_index: 0,
            candidate_id: "legacy:shop:purge:0".to_string(),
            command: "purge 0".to_string(),
            label: "purge Strike".to_string(),
            semantic_class: "purge".to_string(),
            admission: CampaignJournalCandidateAdmissionTraceV1::new(
                CampaignJournalCandidateAdmissionStatusV1::Scheduled,
                "shop_candidate_pool",
                "admit",
            ),
            disposition: CampaignJournalCandidateDispositionV1::Kept,
            target_origin: Default::default(),
            milestone: "resource_conversion_frontier".to_string(),
        };

        assert!(coverage_gap_target_requires_exact_parent_snapshot_v1(
            &target
        ));
        assert!(!coverage_gap_parent_commands_have_exact_coordinate_v1(
            &target.parent_commands
        ));
        target
            .parent_commands
            .push("__decision_parent:1:shop:abcd".to_string());
        assert!(coverage_gap_parent_commands_have_exact_coordinate_v1(
            &target.parent_commands
        ));
        target.parent_commands.clear();
        assert!(!coverage_gap_target_requires_exact_parent_snapshot_v1(
            &target
        ));
    }

    fn coverage_gap_test_target(
        event_type: &str,
        command: &str,
        label: &str,
        candidate_index: usize,
    ) -> CoverageGapContinuationTargetV1 {
        CoverageGapContinuationTargetV1 {
            decision_id: format!("{event_type}:decision"),
            event_id: format!("{event_type}:event"),
            event_type: event_type.to_string(),
            parent_branch_id: "root".to_string(),
            parent_frontier_title: "Map".to_string(),
            parent_commands: Vec::new(),
            parent_choices: Vec::new(),
            candidate_index,
            candidate_id: format!("{event_type}:candidate:{candidate_index}"),
            command: command.to_string(),
            label: label.to_string(),
            semantic_class: event_type.to_string(),
            admission: CampaignJournalCandidateAdmissionTraceV1::new(
                CampaignJournalCandidateAdmissionStatusV1::Scheduled,
                format!("{event_type}_candidate_pool"),
                "selected",
            ),
            disposition: CampaignJournalCandidateDispositionV1::Kept,
            target_origin: Default::default(),
            milestone: format!("{event_type}_milestone"),
        }
    }

    fn coverage_gap_test_result_branch(
        target: &CoverageGapContinuationTargetV1,
        status: BranchCampaignBranchStatusV1,
        frontier_title: &str,
        stop_reason: &str,
        act: u8,
        floor: i32,
        hp: i32,
        max_hp: i32,
    ) -> BranchCampaignBranchV1 {
        let mut branch = coverage_gap_branch_from_target_v1(target);
        branch.status = status;
        branch.frontier_title = frontier_title.to_string();
        branch.stop_reason = stop_reason.to_string();
        branch.summary = Some(BranchCampaignBranchSummaryV1 {
            act,
            floor,
            hp,
            max_hp,
            gold: 99,
            deck_count: 12,
            deck_key: String::new(),
            formation_stage: "test".to_string(),
            formation_strengths: Vec::new(),
            formation_needs: Vec::new(),
            trajectory_key: String::new(),
            boss: String::new(),
            boss_pressure: Vec::new(),
            run_debt: Vec::new(),
            event_boundary: None,
            reward_boundary: None,
        });
        branch
    }
}

pub(super) fn run_branch_outcome_dataset_export(input: &DatasetCommandInput) -> Result<(), String> {
    let path = input
        .export_outcome_dataset
        .as_ref()
        .ok_or_else(|| "--export-outcome-dataset requires a path".to_string())?;
    let report_path = input
        .inspect_report
        .as_ref()
        .ok_or_else(|| "--export-outcome-dataset requires --inspect-report PATH".to_string())?;
    let report = read_campaign_report_v1(report_path)?;
    let checkpoint = input
        .inspect_checkpoint
        .as_ref()
        .map(read_campaign_checkpoint_v1)
        .transpose()?;
    let records = extract_branch_outcome_records_v1(&report, checkpoint.as_ref())?;
    write_branch_outcome_dataset_jsonl_v1(path, &records)?;
    let summary = summarize_branch_outcome_records_v1(&records);
    println!(
        "BranchOutcomeDatasetV1 records={} checkpoint_enriched={} output={}",
        summary.total_records,
        summary.checkpoint_enriched_records,
        path.display()
    );
    if !summary.outcome_class_counts.is_empty() {
        println!(
            "outcome_classes={}",
            summary
                .outcome_class_counts
                .iter()
                .map(|entry| format!("{}:{}", entry.key, entry.count))
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    Ok(())
}

pub(super) fn write_branch_outcome_dataset_jsonl_v1(
    path: &PathBuf,
    records: &[BranchOutcomeRecordV1],
) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create --export-outcome-dataset directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let text = serialize_branch_outcome_records_jsonl_v1(records)?;
    fs::write(path, text).map_err(|err| {
        format!(
            "failed to write --export-outcome-dataset {}: {err}",
            path.display()
        )
    })
}

pub(super) fn run_learning_dataset_export(input: &DatasetCommandInput) -> Result<(), String> {
    let path = input
        .export_learning_dataset
        .as_ref()
        .ok_or_else(|| "--export-learning-dataset requires a path".to_string())?;
    let report_path = input
        .inspect_report
        .as_ref()
        .ok_or_else(|| "--export-learning-dataset requires --inspect-report PATH when used without running a campaign".to_string())?;
    let report = read_campaign_report_v1(report_path)?;
    let checkpoint = input
        .inspect_checkpoint
        .as_ref()
        .map(read_campaign_checkpoint_v1)
        .transpose()?;
    let outcome_records = extract_branch_outcome_records_v1(&report, checkpoint.as_ref())?;
    let samples = learning_records_from_branch_outcomes_v1(
        &outcome_records,
        learning_dataset_export_context_v1(Some(report_path), input.inspect_checkpoint.as_ref()),
    );
    write_learning_dataset_jsonl_v1(path, &samples)?;
    println!(
        "LearningBranchSampleV1 records={} output={}",
        samples.len(),
        path.display()
    );
    Ok(())
}

pub(super) fn run_decision_outcome_dataset_export(
    input: &DatasetCommandInput,
) -> Result<(), String> {
    let path = input
        .export_decision_outcome_dataset
        .as_ref()
        .ok_or_else(|| "--export-decision-outcome-dataset requires a path".to_string())?;
    let report_path = input
        .inspect_report
        .as_ref()
        .ok_or_else(|| "--export-decision-outcome-dataset requires --inspect-report PATH when used without running a campaign".to_string())?;
    let report = read_campaign_report_v1(report_path)?;
    let checkpoint = input
        .inspect_checkpoint
        .as_ref()
        .map(read_campaign_checkpoint_v1)
        .transpose()?;
    let outcome_records = extract_branch_outcome_records_v1(&report, checkpoint.as_ref())?;
    let samples = decision_outcome_samples_from_campaign_report_v1(
        &report,
        &outcome_records,
        learning_dataset_export_context_v1(Some(report_path), input.inspect_checkpoint.as_ref()),
    );
    write_decision_outcome_dataset_jsonl_v1(path, &samples)?;
    let observed_sibling_samples = samples
        .iter()
        .filter(|sample| sample.observed_sibling_count > 1)
        .count();
    println!(
        "LearningDecisionOutcomeSampleV1 records={} observed_sibling_records={} output={}",
        samples.len(),
        observed_sibling_samples,
        path.display()
    );
    let coverage = analyze_journal_decision_candidate_coverage_v1(&report, &outcome_records);
    println!(
        "{}",
        render_journal_decision_candidate_coverage_v1(&coverage)
    );
    Ok(())
}

pub(super) fn run_decision_candidate_coverage_inspection(
    input: &DatasetCommandInput,
) -> Result<(), String> {
    let report_path = input
        .inspect_report
        .as_ref()
        .ok_or_else(|| "--inspect-decision-coverage requires --inspect-report PATH".to_string())?;
    let report = read_campaign_report_v1(report_path)?;
    let checkpoint = input
        .inspect_checkpoint
        .as_ref()
        .map(read_campaign_checkpoint_v1)
        .transpose()?;
    let outcome_records = extract_branch_outcome_records_v1(&report, checkpoint.as_ref())?;
    let coverage = analyze_journal_decision_candidate_coverage_v1(&report, &outcome_records);
    println!(
        "{}",
        render_journal_decision_candidate_coverage_v1(&coverage)
    );
    Ok(())
}

pub(super) fn learning_dataset_export_context_v1(
    report_path: Option<&PathBuf>,
    checkpoint_path: Option<&PathBuf>,
) -> LearningDatasetExportContextV1 {
    LearningDatasetExportContextV1 {
        exporter_git_commit: current_git_commit_v1(),
        exporter_git_dirty: current_git_dirty_v1(),
        source_report_path: report_path.map(|path| path.display().to_string()),
        source_checkpoint_path: checkpoint_path.map(|path| path.display().to_string()),
    }
}

pub(super) fn write_learning_dataset_jsonl_v1(
    path: &PathBuf,
    samples: &[LearningBranchSampleV1],
) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create --export-learning-dataset directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let text = serialize_learning_branch_samples_jsonl_v1(samples)?;
    fs::write(path, text).map_err(|err| {
        format!(
            "failed to write --export-learning-dataset {}: {err}",
            path.display()
        )
    })
}

pub(super) fn write_decision_outcome_dataset_jsonl_v1(
    path: &PathBuf,
    samples: &[LearningDecisionOutcomeSampleV1],
) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create --export-decision-outcome-dataset directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let text = serialize_learning_decision_outcome_samples_jsonl_v1(samples)?;
    fs::write(path, text).map_err(|err| {
        format!(
            "failed to write --export-decision-outcome-dataset {}: {err}",
            path.display()
        )
    })
}

fn current_git_dirty_v1() -> Option<bool> {
    let output = Command::new("git")
        .args(["status", "--short"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    Some(!text.trim().is_empty())
}

fn current_git_commit_v1() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let commit = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if commit.is_empty() {
        None
    } else {
        Some(commit)
    }
}
