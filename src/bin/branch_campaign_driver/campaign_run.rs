use std::time::Instant;

use sts_simulator::eval::branch_campaign::{
    render_branch_campaign_compact_with_detail_v1, render_branch_campaign_progress_event_v1,
    render_branch_campaign_progress_event_with_detail_v1,
    run_branch_campaign_ancestor_replay_self_check_v1,
    run_branch_campaign_from_report_with_checkpoint_and_progress_v1,
    run_branch_campaign_from_report_with_checkpoint_v1,
    run_branch_campaign_with_checkpoint_and_progress_v1, run_branch_campaign_with_checkpoint_v1,
    BranchCampaignCheckpointV1, BranchCampaignProgressDetailV1, BranchCampaignReportV1,
    BranchCampaignRunResultV1,
};
use sts_simulator::eval::branch_outcome_dataset_v1::{
    extract_branch_outcome_records_v1, summarize_branch_outcome_records_v1,
};

use super::campaign_artifacts::{
    read_campaign_checkpoint_v1, read_campaign_report_v1, write_campaign_checkpoint_v1,
    write_campaign_report_v1,
};
use super::campaign_milestones::{
    campaign_milestone_status_v1, render_campaign_milestone_status_v1,
    resolve_campaign_milestone_target_v1,
};
use super::command_inputs::{
    render_round_budget_resolution_v1, CampaignMilestoneStopV1, RoundBudgetModeV1,
    RoundBudgetResolutionV1, RunCommandInput,
};
use super::outcome_dataset::{
    learning_dataset_export_context_v1, write_branch_outcome_dataset_jsonl_v1,
    write_decision_outcome_dataset_jsonl_v1, write_learning_dataset_jsonl_v1,
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

pub(super) fn run_continue_campaign_command(input: &RunCommandInput) -> Result<(), String> {
    if input.resume.is_none() {
        return Err("campaign continue requires --resume".to_string());
    }
    run_campaign_command(input)
}

pub(super) fn run_campaign_command(input: &RunCommandInput) -> Result<(), String> {
    if input.resume_checkpoint.is_some() && input.resume.is_none() {
        return Err("--resume-checkpoint requires --resume".to_string());
    }
    let mut previous = input
        .resume
        .as_ref()
        .map(read_campaign_report_v1)
        .transpose()?;
    let mut checkpoint = input
        .resume_checkpoint
        .as_ref()
        .map(read_campaign_checkpoint_v1)
        .transpose()?;
    let source_rounds = previous
        .as_ref()
        .map_or(0, |report| report.rounds_completed);
    let mut round_budget = input
        .round_budget
        .resolve_for_source_rounds(source_rounds)?;
    if input.milestone.enabled() {
        round_budget = RoundBudgetResolutionV1 {
            mode: RoundBudgetModeV1::UntilMilestone,
            source_rounds,
            round_budget: input.milestone.step_rounds.min(input.milestone.max_rounds),
            target_total_rounds: source_rounds
                .saturating_add(input.milestone.step_rounds.min(input.milestone.max_rounds)),
        };
    }
    let started_at = Instant::now();
    let mut result = run_campaign_iteration_v1(
        input,
        previous.as_ref(),
        checkpoint.as_ref(),
        round_budget.round_budget,
        started_at,
    )?;
    if let Some(target) = input.milestone.target {
        let concrete_target = resolve_campaign_milestone_target_v1(target, &result.report);
        let stop = input.milestone.stop.resolve_for_run();
        let mut spent_rounds = round_budget.round_budget;
        while spent_rounds < input.milestone.max_rounds {
            let status = campaign_milestone_status_v1(&result.report, concrete_target);
            if !input.json {
                println!(
                    "{}",
                    render_campaign_milestone_status_v1(
                        concrete_target,
                        stop,
                        status,
                        spent_rounds,
                        input.milestone.max_rounds
                    )
                );
            }
            if status.reached && stop == CampaignMilestoneStopV1::FirstHit {
                break;
            }
            let step_rounds = input
                .milestone
                .step_rounds
                .min(input.milestone.max_rounds - spent_rounds);
            if step_rounds == 0 {
                break;
            }
            previous = Some(result.report);
            checkpoint = Some(result.checkpoint);
            result = run_campaign_iteration_v1(
                input,
                previous.as_ref(),
                checkpoint.as_ref(),
                step_rounds,
                started_at,
            )?;
            spent_rounds += step_rounds;
        }
        let status = campaign_milestone_status_v1(&result.report, concrete_target);
        if !input.json {
            println!(
                "{}",
                render_campaign_milestone_status_v1(
                    concrete_target,
                    stop,
                    status,
                    spent_rounds,
                    input.milestone.max_rounds
                )
            );
        }
    }
    let report = result.report;
    let checkpoint = result.checkpoint;
    if !input.json {
        println!("{}", render_round_budget_resolution_v1(round_budget));
        eprintln!(
            "run-domain: ascension=A{} label={} class={}",
            report.run_domain.ascension_level,
            report.run_domain.label,
            report.run_domain.player_class
        );
    }
    if let Some(path) = input.out.as_ref() {
        write_campaign_report_v1(path, &report)?;
    }
    if let Some(path) = input.checkpoint_out.as_ref() {
        write_campaign_checkpoint_v1(path, &checkpoint)?;
    }
    if let Some(path) = input.export_outcome_dataset.as_ref() {
        let records = extract_branch_outcome_records_v1(&report, Some(&checkpoint))?;
        write_branch_outcome_dataset_jsonl_v1(path, &records)?;
        let summary = summarize_branch_outcome_records_v1(&records);
        eprintln!(
            "wrote {} BranchOutcomeRecordV1 row(s) to {} (checkpoint_enriched={})",
            summary.total_records,
            path.display(),
            summary.checkpoint_enriched_records
        );
    }
    if let Some(path) = input.export_learning_dataset.as_ref() {
        let records = extract_branch_outcome_records_v1(&report, Some(&checkpoint))?;
        let samples =
            sts_simulator::eval::learning_dataset_v1::learning_records_from_branch_outcomes_v1(
                &records,
                learning_dataset_export_context_v1(
                    input.out.as_ref(),
                    input.checkpoint_out.as_ref(),
                ),
            );
        write_learning_dataset_jsonl_v1(path, &samples)?;
        eprintln!(
            "wrote {} LearningBranchSampleV1 row(s) to {}",
            samples.len(),
            path.display()
        );
    }
    if let Some(path) = input.export_decision_outcome_dataset.as_ref() {
        let records = extract_branch_outcome_records_v1(&report, Some(&checkpoint))?;
        let samples = sts_simulator::eval::learning_dataset_v1::decision_outcome_samples_from_campaign_report_v1(
            &report,
            &records,
            learning_dataset_export_context_v1(input.out.as_ref(), input.checkpoint_out.as_ref()),
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
        let coverage =
            sts_simulator::eval::learning_dataset_v1::analyze_journal_decision_candidate_coverage_v1(
                &report,
                &records,
            );
        eprintln!(
            "{}",
            sts_simulator::eval::learning_dataset_v1::render_journal_decision_candidate_coverage_v1(
                &coverage
            )
        );
    }
    if input.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?
        );
    } else {
        println!(
            "{}",
            render_branch_campaign_compact_with_detail_v1(
                &report,
                input.branch_examples,
                input.report_detail
            )
        );
    }
    Ok(())
}

fn run_campaign_iteration_v1(
    input: &RunCommandInput,
    previous: Option<&BranchCampaignReportV1>,
    checkpoint: Option<&BranchCampaignCheckpointV1>,
    round_budget: usize,
    started_at: Instant,
) -> Result<BranchCampaignRunResultV1, String> {
    let mut config = input.config.clone();
    config.max_rounds = round_budget;
    if input.progress && !input.json {
        let progress_detail = input.progress_detail;
        let progress = |event| print_campaign_progress_event_v1(started_at, progress_detail, event);
        if let Some(previous) = previous {
            run_branch_campaign_from_report_with_checkpoint_and_progress_v1(
                &config, previous, checkpoint, progress,
            )
        } else {
            run_branch_campaign_with_checkpoint_and_progress_v1(&config, progress)
        }
    } else if let Some(previous) = previous {
        run_branch_campaign_from_report_with_checkpoint_v1(&config, previous, checkpoint)
    } else {
        run_branch_campaign_with_checkpoint_v1(&config)
    }
}

fn print_campaign_progress_event_v1(
    started_at: Instant,
    progress_detail: BranchCampaignProgressDetailV1,
    event: sts_simulator::eval::branch_campaign::BranchCampaignProgressEventV1,
) {
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
}
