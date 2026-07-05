use super::run_deadline::RunDeadline;
use super::run_startup::RunStartupContext;
use super::{branch_frontier, branch_generation, run_persistence, trace, BranchStatus};

pub(super) fn run(context: RunStartupContext) -> Result<(), String> {
    let RunStartupContext {
        args,
        trace_path,
        combat_gap_case_dir,
        frontier_checkpoint_path,
        resume_frontier,
        run_capsule,
        generation_start,
        mut frontier,
        mut next_branch_id,
        started,
    } = context;
    let mut capsule_frontier_saved = false;
    let mut trace = trace_path
        .as_ref()
        .map(|path| trace::TraceWriter::create(path))
        .transpose()?;
    let deadline = RunDeadline::new(started, args.wall_ms);
    let mut recent_expanded_keys = Vec::new();

    print_header(args, resume_frontier.is_some());
    if let Some(trace) = trace.as_mut() {
        trace.record_run(args)?;
    }
    let mut last_generation = generation_start;
    for generation in generation_start..=args.generations {
        last_generation = generation;
        if deadline.should_soft_stop(args) {
            capsule_frontier_saved |= run_persistence::save_context_wall_stop(
                &frontier_checkpoint_path,
                &resume_frontier,
                run_capsule.as_ref(),
                args,
                generation,
                next_branch_id,
                &frontier,
                &deadline,
            )?;
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
            capsule_frontier_saved |= run_persistence::save_context_wall_stop(
                &frontier_checkpoint_path,
                &resume_frontier,
                run_capsule.as_ref(),
                args,
                generation,
                next_branch_id,
                &frontier,
                &deadline,
            )?;
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
        )? {
            branch_generation::GenerationAdvance::ObjectiveCompleted => return Ok(()),
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
                println!("run_capsule_result: {}", capsule.result_path().display());
            }
            break;
        }
        if next
            .iter()
            .any(|branch| matches!(branch.status, BranchStatus::AwaitingAuto { .. }))
        {
            frontier = next;
            capsule_frontier_saved |= run_persistence::save_context_wall_stop(
                &frontier_checkpoint_path,
                &resume_frontier,
                run_capsule.as_ref(),
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
            capsule_frontier_saved |= run_persistence::save_context_wall_stop(
                &frontier_checkpoint_path,
                &resume_frontier,
                run_capsule.as_ref(),
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
    if let Some(capsule) = run_capsule.as_ref().filter(|_| !capsule_frontier_saved) {
        run_persistence::print_capsule_save(
            capsule.save_recovery(args, last_generation, next_branch_id, &frontier)?,
            capsule,
        );
    }
    Ok(())
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
