use std::fs;
use std::path::PathBuf;
use std::process::Command;

use sts_simulator::eval::branch_campaign::{
    render_branch_campaign_compact_with_detail_v1,
    run_branch_campaign_from_report_with_checkpoint_v1, BranchCampaignBranchStatusV1,
    BranchCampaignBranchV1, BranchCampaignReportDetailV1, BranchCampaignReportV1,
};
use sts_simulator::eval::branch_outcome_dataset_v1::{
    analyze_branch_outcome_records_v1, extract_branch_outcome_records_v1,
    parse_branch_outcome_records_jsonl_v1, render_branch_outcome_dataset_analysis_v1,
    serialize_branch_outcome_records_jsonl_v1, summarize_branch_outcome_records_v1,
    BranchOutcomeRecordV1,
};
use sts_simulator::eval::learning_dataset_v1::{
    analyze_learning_decision_outcome_samples_v1, decision_outcome_samples_from_branch_outcomes_v1,
    learning_records_from_branch_outcomes_v1, parse_learning_decision_outcome_samples_jsonl_v1,
    plan_targeted_continuations_v1, probe_learning_readiness_v1,
    render_learning_decision_outcome_analysis_v1, render_learning_readiness_probe_v1,
    render_targeted_continuation_plan_v1, serialize_learning_branch_samples_jsonl_v1,
    serialize_learning_decision_outcome_samples_jsonl_v1, targeted_continuation_execution_plan_v1,
    LearningBranchSampleV1, LearningDatasetExportContextV1, LearningDecisionOutcomeSampleV1,
    TargetedContinuationExecutionPlanV1,
};
use sts_simulator::eval::run_control::canonical_player_class;

use super::{
    campaign_config_from_args, read_campaign_checkpoint_v1, read_campaign_report_v1,
    write_campaign_checkpoint_v1, write_campaign_report_v1, Args,
};

pub(super) fn run_branch_outcome_dataset_analysis(args: &Args) -> Result<(), String> {
    let path = args
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

pub(super) fn run_decision_outcome_dataset_analysis(args: &Args) -> Result<(), String> {
    let path = args
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

pub(super) fn run_learning_readiness_probe(args: &Args) -> Result<(), String> {
    let path = args
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

pub(super) fn run_targeted_continuation_plan(args: &Args) -> Result<(), String> {
    let path = args
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

pub(super) fn run_targeted_continuation_execution(args: &Args) -> Result<(), String> {
    let samples_path = args
        .execute_targeted_continuation
        .as_ref()
        .ok_or_else(|| "--execute-targeted-continuation requires a path".to_string())?;
    let report_path = args
        .resume
        .as_ref()
        .ok_or_else(|| "--execute-targeted-continuation requires --resume PATH".to_string())?;
    let checkpoint_path = args.resume_checkpoint.as_ref().ok_or_else(|| {
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
        args.targeted_continuation_limit,
        args.targeted_continuation_candidates_per_target,
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
    let mut config = campaign_config_from_args(args)?;
    config.seed = source_report.seed;
    config.ascension_level = source_report.run_domain.ascension_level;
    config.player_class = canonical_player_class(&source_report.run_domain.player_class)?;
    config.prefix_commands.clear();

    let result = run_branch_campaign_from_report_with_checkpoint_v1(
        &config,
        &continuation_report,
        Some(&source_checkpoint),
    )?;
    if let Some(path) = args.out.as_ref() {
        write_campaign_report_v1(path, &result.report)?;
    }
    if let Some(path) = args.checkpoint_out.as_ref() {
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
            args.branch_examples,
            BranchCampaignReportDetailV1::from(args.report_detail),
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

pub(super) fn run_branch_outcome_dataset_export(args: &Args) -> Result<(), String> {
    let path = args
        .export_outcome_dataset
        .as_ref()
        .ok_or_else(|| "--export-outcome-dataset requires a path".to_string())?;
    let report_path = args
        .inspect_report
        .as_ref()
        .ok_or_else(|| "--export-outcome-dataset requires --inspect-report PATH".to_string())?;
    let report = read_campaign_report_v1(report_path)?;
    let checkpoint = args
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

pub(super) fn run_learning_dataset_export(args: &Args) -> Result<(), String> {
    let path = args
        .export_learning_dataset
        .as_ref()
        .ok_or_else(|| "--export-learning-dataset requires a path".to_string())?;
    let report_path = args
        .inspect_report
        .as_ref()
        .ok_or_else(|| "--export-learning-dataset requires --inspect-report PATH when used without running a campaign".to_string())?;
    let report = read_campaign_report_v1(report_path)?;
    let checkpoint = args
        .inspect_checkpoint
        .as_ref()
        .map(read_campaign_checkpoint_v1)
        .transpose()?;
    let outcome_records = extract_branch_outcome_records_v1(&report, checkpoint.as_ref())?;
    let samples = learning_records_from_branch_outcomes_v1(
        &outcome_records,
        learning_dataset_export_context_v1(Some(report_path), args.inspect_checkpoint.as_ref()),
    );
    write_learning_dataset_jsonl_v1(path, &samples)?;
    println!(
        "LearningBranchSampleV1 records={} output={}",
        samples.len(),
        path.display()
    );
    Ok(())
}

pub(super) fn run_decision_outcome_dataset_export(args: &Args) -> Result<(), String> {
    let path = args
        .export_decision_outcome_dataset
        .as_ref()
        .ok_or_else(|| "--export-decision-outcome-dataset requires a path".to_string())?;
    let report_path = args
        .inspect_report
        .as_ref()
        .ok_or_else(|| "--export-decision-outcome-dataset requires --inspect-report PATH when used without running a campaign".to_string())?;
    let report = read_campaign_report_v1(report_path)?;
    let checkpoint = args
        .inspect_checkpoint
        .as_ref()
        .map(read_campaign_checkpoint_v1)
        .transpose()?;
    let outcome_records = extract_branch_outcome_records_v1(&report, checkpoint.as_ref())?;
    let samples = decision_outcome_samples_from_branch_outcomes_v1(
        &outcome_records,
        learning_dataset_export_context_v1(Some(report_path), args.inspect_checkpoint.as_ref()),
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
