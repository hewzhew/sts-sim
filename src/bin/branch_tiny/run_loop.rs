use super::run_deadline::RunDeadline;
use super::run_slice_request::RunSliceRequest;
use super::run_slice_result::{
    FrontierExhausted, FrontierSummary, RealStop, RunSliceResult, RunStop, SoftPause,
};
use super::{branch_frontier, branch_generation, run_stop_recorder, trace, BranchStatus};

pub(super) fn run(request: RunSliceRequest) -> Result<RunSliceResult, String> {
    let RunSliceRequest {
        args,
        request_kind,
        human_output,
        trace_path,
        combat_gap_case_dir,
        frontier_checkpoint_path,
        resume_frontier,
        run_capsule,
        generation_start,
        mut frontier,
        mut next_branch_id,
        started,
    } = request;
    let mut stop_recorder = run_stop_recorder::RunStopRecorder::new(
        &frontier_checkpoint_path,
        &resume_frontier,
        run_capsule.as_ref(),
        human_output,
    );
    let mut trace = trace_path
        .as_ref()
        .map(|path| trace::TraceWriter::create(path))
        .transpose()?;
    let deadline = RunDeadline::new(started, args.wall_ms);
    let mut recent_expanded_keys = Vec::new();

    if human_output {
        print_header(args, resume_frontier.is_some());
    }
    if let Some(trace) = trace.as_mut() {
        trace.record_run(args)?;
    }
    let mut last_generation = generation_start;
    let mut stop = None;
    let mut selected_branch = None;
    for generation in generation_start..=args.generations {
        last_generation = generation;
        if deadline.should_soft_stop(args) {
            stop_recorder.save_soft_wall(args, generation, next_branch_id, &frontier, &deadline)?;
            let summary = FrontierSummary::from_branches(frontier.iter());
            stop = Some(RunStop::SoftPause(SoftPause::SliceDeadline {
                generation,
                frontier_running_count: summary.running_count,
            }));
            break;
        }
        let prepared = branch_generation::prepare_generation(
            &mut frontier,
            args,
            generation,
            deadline,
            &mut recent_expanded_keys,
        );
        if prepared.total_expanded > 0
            && deadline.would_cap_core_search(args, prepared.total_expanded)
        {
            frontier = prepared.into_frontier();
            stop_recorder.save_soft_wall(args, generation, next_branch_id, &frontier, &deadline)?;
            let summary = FrontierSummary::from_branches(frontier.iter());
            stop = Some(RunStop::SoftPause(
                SoftPause::SearchBudgetCappedBeforeGeneration {
                    generation,
                    frontier_running_count: summary.running_count,
                },
            ));
            break;
        }
        let child_args = deadline.cap_args(args, prepared.total_expanded.max(1));
        let (mut next, generation_result) = match branch_generation::advance_generation(
            prepared,
            args,
            child_args,
            generation,
            deadline,
            &mut next_branch_id,
            &mut trace,
            combat_gap_case_dir.as_ref(),
            run_capsule.as_ref(),
            human_output,
        )? {
            branch_generation::GenerationAdvance::ObjectiveCompleted(branch) => {
                let result = RunSliceResult::new(
                    args,
                    request_kind,
                    generation_start,
                    generation,
                    next_branch_id,
                    RunStop::Real(RealStop::ObjectiveSatisfied {
                        generation,
                        reason: "objective_satisfied".to_string(),
                    }),
                    FrontierSummary::from_statuses(std::iter::once(&branch.status)),
                    deadline.remaining_ms(),
                    elapsed_ms(started),
                )
                .with_selected_branch(&branch);
                return Ok(result);
            }
            branch_generation::GenerationAdvance::Advanced {
                next,
                generation_result,
            } => (next, generation_result),
        };
        branch_frontier::retain_frontier(&mut next, args.max_branches);
        if next.is_empty() {
            if let (Some(capsule), Some((result_generation, branch))) =
                (run_capsule.as_ref(), generation_result.as_ref())
            {
                capsule.save_result(args, *result_generation, branch)?;
                if human_output {
                    println!("run_capsule_result: {}", capsule.result_path().display());
                }
            }
            if let Some((result_generation, branch)) = generation_result {
                stop = Some(
                    RunStop::from_stopped_branch_status(
                        result_generation,
                        branch.id,
                        &branch.status,
                    )
                    .unwrap_or_else(|| {
                        RunStop::SoftPause(SoftPause::GenerationLimit {
                            generation: result_generation,
                            frontier_running_count: 1,
                        })
                    }),
                );
                selected_branch = Some(branch);
            } else {
                stop = Some(RunStop::FrontierExhausted(
                    FrontierExhausted::NoRunningBranches { generation },
                ));
            }
            break;
        }
        if next
            .iter()
            .any(|branch| matches!(branch.status, BranchStatus::AwaitingAuto { .. }))
        {
            frontier = next;
            let summary = FrontierSummary::from_branches(frontier.iter());
            stop = Some(RunStop::SoftPause(SoftPause::AwaitingAutoBoundary {
                generation: generation + 1,
                frontier_running_count: summary.running_count,
            }));
            stop_recorder.save_soft_wall(
                args,
                generation + 1,
                next_branch_id,
                &frontier,
                &deadline,
            )?;
            break;
        }
        frontier = next;
        if deadline.should_soft_stop(args) {
            let summary = FrontierSummary::from_branches(frontier.iter());
            stop = Some(RunStop::SoftPause(SoftPause::SliceDeadline {
                generation: generation + 1,
                frontier_running_count: summary.running_count,
            }));
            stop_recorder.save_soft_wall(
                args,
                generation + 1,
                next_branch_id,
                &frontier,
                &deadline,
            )?;
            break;
        }
    }
    if let Some(trace) = trace.as_mut() {
        trace.record_frontier_snapshot(last_generation, &frontier)?;
    }
    stop_recorder.save_recovery_if_needed(args, last_generation, next_branch_id, &frontier)?;
    let summary = FrontierSummary::from_branches(frontier.iter());
    let stop = stop.unwrap_or_else(|| {
        if summary.running_count == 0 {
            RunStop::FrontierExhausted(FrontierExhausted::NoRunningBranches {
                generation: last_generation,
            })
        } else {
            RunStop::SoftPause(SoftPause::GenerationLimit {
                generation: last_generation,
                frontier_running_count: summary.running_count,
            })
        }
    });
    let mut result = RunSliceResult::new(
        args,
        request_kind,
        generation_start,
        last_generation,
        next_branch_id,
        stop,
        summary,
        deadline.remaining_ms(),
        elapsed_ms(started),
    );
    if let Some(branch) = selected_branch.as_ref() {
        result = result.with_selected_branch(branch);
    }
    Ok(result)
}

fn print_header(args: super::Args, resume_frontier: bool) {
    println!(
        "branch_tiny seed={} ascension={} objective={:?} generations={} max_branches={} mode=owner_audit render=timeline{}",
        args.seed,
        args.ascension,
        args.objective,
        args.generations,
        args.max_branches,
        if resume_frontier { " resume=frontier" } else { "" }
    );
    println!(
        "branch cap: {}; search={}nodes/{}ms; rescue={}nodes/{}ms diagnostic; combat_portfolio={}nodes/{}ms; '>' marks expanded choices",
        args.max_branches,
        args.search_nodes,
        args.search_ms,
        args.rescue_search_nodes,
        args.rescue_search_ms,
        args.boss_search_nodes,
        args.boss_search_ms
    );
}

fn elapsed_ms(started: std::time::Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}
