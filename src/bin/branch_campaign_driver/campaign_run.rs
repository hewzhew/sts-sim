use std::time::Instant;

use sts_simulator::eval::branch_campaign::{
    render_branch_campaign_compact_with_detail_v1, render_branch_campaign_progress_event_v1,
    render_branch_campaign_progress_event_with_detail_v1,
    run_branch_campaign_ancestor_replay_self_check_v1,
    run_branch_campaign_from_report_with_checkpoint_and_progress_v1,
    run_branch_campaign_from_report_with_checkpoint_v1,
    run_branch_campaign_with_checkpoint_and_progress_v1, run_branch_campaign_with_checkpoint_v1,
    BranchCampaignProgressDetailV1, BranchCampaignReportDetailV1,
};
use sts_simulator::eval::branch_outcome_dataset_v1::{
    extract_branch_outcome_records_v1, summarize_branch_outcome_records_v1,
};

use super::outcome_dataset::{
    learning_dataset_export_context_v1, write_branch_outcome_dataset_jsonl_v1,
    write_decision_outcome_dataset_jsonl_v1, write_learning_dataset_jsonl_v1,
};
use super::{
    campaign_config_from_args, read_campaign_checkpoint_v1, read_campaign_report_v1,
    write_campaign_checkpoint_v1, write_campaign_report_v1, Args,
};

pub(super) fn run_ancestor_replay_self_check() -> Result<(), String> {
    let summary = run_branch_campaign_ancestor_replay_self_check_v1()?;
    println!(
        "AncestorReplaySelfCheckV1 exact={} ancestor={} miss={} suffix_sum={} suffix_max={} sessions={} nodes={} pruned={} anchors={}",
        summary.replay_exact_hits,
        summary.replay_ancestor_hits,
        summary.replay_misses,
        summary.replay_suffix_commands_sum,
        summary.replay_suffix_commands_max,
        summary.sessions,
        summary.nodes,
        summary.sessions_pruned,
        summary.anchor_sessions_kept
    );
    Ok(())
}

pub(super) fn run_campaign_command(args: &Args) -> Result<(), String> {
    let config = campaign_config_from_args(args)?;
    if args.resume_checkpoint.is_some() && args.resume.is_none() {
        return Err("--resume-checkpoint requires --resume".to_string());
    }
    let previous = args
        .resume
        .as_ref()
        .map(read_campaign_report_v1)
        .transpose()?;
    let checkpoint = args
        .resume_checkpoint
        .as_ref()
        .map(read_campaign_checkpoint_v1)
        .transpose()?;
    let result = if args.progress && !args.json {
        let started_at = Instant::now();
        let progress_detail = BranchCampaignProgressDetailV1::from(args.progress_detail);
        let progress = |event| {
            let rendered = match progress_detail {
                BranchCampaignProgressDetailV1::Summary => {
                    render_branch_campaign_progress_event_with_detail_v1(&event, progress_detail)
                }
                BranchCampaignProgressDetailV1::Verbose => {
                    Some(render_branch_campaign_progress_event_v1(&event))
                }
            };
            if let Some(line) = rendered {
                println!("[{:>4}s] {line}", started_at.elapsed().as_secs());
            }
        };
        if let Some(previous) = previous.as_ref() {
            run_branch_campaign_from_report_with_checkpoint_and_progress_v1(
                &config,
                previous,
                checkpoint.as_ref(),
                progress,
            )?
        } else {
            run_branch_campaign_with_checkpoint_and_progress_v1(&config, progress)?
        }
    } else if let Some(previous) = previous.as_ref() {
        run_branch_campaign_from_report_with_checkpoint_v1(&config, previous, checkpoint.as_ref())?
    } else {
        run_branch_campaign_with_checkpoint_v1(&config)?
    };
    let report = result.report;
    if !args.json {
        eprintln!(
            "run-domain: ascension=A{} label={} class={}",
            report.run_domain.ascension_level,
            report.run_domain.label,
            report.run_domain.player_class
        );
    }
    if let Some(path) = args.out.as_ref() {
        write_campaign_report_v1(path, &report)?;
    }
    if let Some(path) = args.checkpoint_out.as_ref() {
        write_campaign_checkpoint_v1(path, &result.checkpoint)?;
    }
    if let Some(path) = args.export_outcome_dataset.as_ref() {
        let records = extract_branch_outcome_records_v1(&report, Some(&result.checkpoint))?;
        write_branch_outcome_dataset_jsonl_v1(path, &records)?;
        let summary = summarize_branch_outcome_records_v1(&records);
        eprintln!(
            "wrote {} BranchOutcomeRecordV1 row(s) to {} (checkpoint_enriched={})",
            summary.total_records,
            path.display(),
            summary.checkpoint_enriched_records
        );
    }
    if let Some(path) = args.export_learning_dataset.as_ref() {
        let records = extract_branch_outcome_records_v1(&report, Some(&result.checkpoint))?;
        let samples =
            sts_simulator::eval::learning_dataset_v1::learning_records_from_branch_outcomes_v1(
                &records,
                learning_dataset_export_context_v1(args.out.as_ref(), args.checkpoint_out.as_ref()),
            );
        write_learning_dataset_jsonl_v1(path, &samples)?;
        eprintln!(
            "wrote {} LearningBranchSampleV1 row(s) to {}",
            samples.len(),
            path.display()
        );
    }
    if let Some(path) = args.export_decision_outcome_dataset.as_ref() {
        let records = extract_branch_outcome_records_v1(&report, Some(&result.checkpoint))?;
        let samples = sts_simulator::eval::learning_dataset_v1::decision_outcome_samples_from_branch_outcomes_v1(
            &records,
            learning_dataset_export_context_v1(args.out.as_ref(), args.checkpoint_out.as_ref()),
        );
        write_decision_outcome_dataset_jsonl_v1(path, &samples)?;
        let observed_sibling_samples = samples
            .iter()
            .filter(|sample| sample.observed_sibling_count > 1)
            .count();
        eprintln!(
            "wrote {} LearningDecisionOutcomeSampleV1 row(s) to {} (observed_sibling_records={})",
            samples.len(),
            path.display(),
            observed_sibling_samples
        );
    }
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?
        );
    } else {
        println!(
            "{}",
            render_branch_campaign_compact_with_detail_v1(
                &report,
                args.branch_examples,
                BranchCampaignReportDetailV1::from(args.report_detail)
            )
        );
    }
    Ok(())
}
