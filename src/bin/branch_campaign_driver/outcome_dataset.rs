use std::fs;
use std::path::PathBuf;
use std::process::Command;

use sts_simulator::eval::branch_campaign::{
    render_branch_campaign_compact_with_detail_v1,
    run_branch_campaign_from_report_with_checkpoint_v1, BranchCampaignBranchStatusV1,
    BranchCampaignBranchV1, BranchCampaignContinuationOriginV1, BranchCampaignReportV1,
    BranchCampaignRouteContinuationOriginV1,
};
use sts_simulator::eval::branch_outcome_dataset_v1::{
    analyze_branch_outcome_records_v1, extract_branch_outcome_records_v1,
    parse_branch_outcome_records_jsonl_v1, render_branch_outcome_dataset_analysis_v1,
    serialize_branch_outcome_records_jsonl_v1, summarize_branch_outcome_records_v1,
    BranchOutcomeRecordV1,
};
use sts_simulator::eval::learning_dataset_v1::{
    analyze_continuation_effect_v1, analyze_journal_decision_candidate_coverage_v1,
    analyze_learning_decision_outcome_samples_v1, coverage_gap_continuation_execution_plan_v1,
    decision_outcome_samples_from_campaign_report_v1, learning_records_from_branch_outcomes_v1,
    parse_learning_decision_outcome_samples_jsonl_v1, plan_coverage_gap_continuations_v1,
    plan_targeted_continuations_v1, probe_learning_readiness_v1,
    render_continuation_effect_report_v1, render_coverage_gap_continuation_plan_v1,
    render_journal_decision_candidate_coverage_v1, render_learning_decision_outcome_analysis_v1,
    render_learning_readiness_probe_v1, render_targeted_continuation_plan_v1,
    serialize_learning_branch_samples_jsonl_v1,
    serialize_learning_decision_outcome_samples_jsonl_v1, targeted_continuation_execution_plan_v1,
    CoverageGapContinuationExecutionPlanV1, CoverageGapContinuationTargetV1,
    LearningBranchSampleV1, LearningDatasetExportContextV1, LearningDecisionOutcomeSampleV1,
    TargetedContinuationExecutionPlanV1,
};
use sts_simulator::eval::neow_guided_prefix::{
    neow_guided_prefix_commands_v1, NeowGuidedPrefixConfigV1,
};
use sts_simulator::eval::run_control::canonical_player_class;

use super::campaign_artifacts::{
    read_campaign_checkpoint_v1, read_campaign_report_v1, write_campaign_checkpoint_v1,
    write_campaign_report_v1,
};
use super::command_inputs::{ContinuationCommandInput, DatasetCommandInput};

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
    let checkpoint = input
        .inspect_checkpoint
        .as_ref()
        .map(read_campaign_checkpoint_v1)
        .transpose()?;
    let records = extract_branch_outcome_records_v1(&report, checkpoint.as_ref())?;
    let plan = plan_coverage_gap_continuations_v1(
        &report,
        &records,
        input.coverage_gap_limit,
        input.coverage_gap_candidates_per_decision,
    );
    println!("{}", render_coverage_gap_continuation_plan_v1(&plan));
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
    let plan = plan_coverage_gap_continuations_v1(
        &source_report,
        &records,
        input.coverage_gap_limit,
        input.coverage_gap_candidates_per_decision,
    );
    let execution = coverage_gap_continuation_execution_plan_v1(&plan, input.coverage_gap_limit);
    if execution.targets.is_empty() {
        return Err(format!(
            "coverage gap continuation selected no candidate branches (decisions={} unobserved={})",
            plan.total_decisions, plan.total_unobserved_candidates
        ));
    }

    let continuation_report =
        coverage_gap_continuation_source_report_v1(&source_report, &execution);
    let mut config = input.config.clone();
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
        "CoverageGapContinuationExecutionV1 requested={} selected={} skipped={}",
        execution.requested_target_count,
        execution.selected_branch_count,
        execution.skipped_target_count
    );
    println!("{}", render_coverage_gap_continuation_plan_v1(&plan));
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
                    target_x: route.target_x,
                    target_y: route.target_y,
                    room_type: route.room_type.clone(),
                    move_kind: route.move_kind.clone(),
                    action_kind: route.action_kind.clone(),
                    projection_source: route.projection_source.clone(),
                    projection_coverage: route.projection_coverage.clone(),
                    path_budget: route.path_budget,
                    observed_path_count: route.observed_path_count,
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
    use sts_simulator::eval::campaign_journal::{
        CampaignJournalCandidateAdmissionStatusV1, CampaignJournalCandidateAdmissionTraceV1,
        CampaignJournalCandidateDispositionV1,
    };

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
