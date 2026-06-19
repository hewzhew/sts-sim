use std::fs;
use std::path::PathBuf;
use std::process::Command;

use sts_simulator::eval::branch_outcome_dataset_v1::{
    analyze_branch_outcome_records_v1, extract_branch_outcome_records_v1,
    parse_branch_outcome_records_jsonl_v1, render_branch_outcome_dataset_analysis_v1,
    serialize_branch_outcome_records_jsonl_v1, summarize_branch_outcome_records_v1,
    BranchOutcomeRecordV1,
};
use sts_simulator::eval::learning_dataset_v1::{
    analyze_learning_decision_outcome_samples_v1, decision_outcome_samples_from_branch_outcomes_v1,
    learning_records_from_branch_outcomes_v1, parse_learning_decision_outcome_samples_jsonl_v1,
    probe_learning_readiness_v1, render_learning_decision_outcome_analysis_v1,
    render_learning_readiness_probe_v1, serialize_learning_branch_samples_jsonl_v1,
    serialize_learning_decision_outcome_samples_jsonl_v1, LearningBranchSampleV1,
    LearningDatasetExportContextV1, LearningDecisionOutcomeSampleV1,
};

use super::{read_campaign_checkpoint_v1, read_campaign_report_v1, Args};

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
