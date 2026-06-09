use super::commands::{RunControlAutoStepOptions, RunControlRouteAutomationMode};
use super::session::{RunControlCommandOutcome, RunControlSession};
use super::view_model::build_run_control_view_model;

const DEFAULT_AUTO_RUN_MAX_OPERATIONS: usize = 128;

pub(in crate::eval::run_control) fn apply_auto_run(
    session: &mut RunControlSession,
    options: RunControlAutoStepOptions,
) -> Result<RunControlCommandOutcome, String> {
    apply_auto_run_with_noncombat_mode(
        session,
        options,
        super::auto_step::NonCombatAutoMode::FullPlanner,
    )
}

pub(crate) fn apply_branch_experiment_auto_run(
    session: &mut RunControlSession,
    options: RunControlAutoStepOptions,
) -> Result<RunControlCommandOutcome, String> {
    apply_auto_run_with_noncombat_mode(
        session,
        options,
        super::auto_step::NonCombatAutoMode::BranchExperimentBoundary,
    )
}

fn apply_auto_run_with_noncombat_mode(
    session: &mut RunControlSession,
    mut options: RunControlAutoStepOptions,
    noncombat_mode: super::auto_step::NonCombatAutoMode,
) -> Result<RunControlCommandOutcome, String> {
    options.route = RunControlRouteAutomationMode::Planner;
    let max_operations = options
        .max_operations
        .unwrap_or(DEFAULT_AUTO_RUN_MAX_OPERATIONS);
    options.max_operations = Some(max_operations);

    let mut outcome =
        super::auto_step::apply_guarded_auto_step_with_mode(session, options, noncombat_mode)?;
    let title = build_run_control_view_model(session).header.title;
    let applied_operations = count_applied_operations(&outcome.message);
    outcome.message = format!(
        "Auto-run stopped: {title}\nroute=planner max_operations={max_operations} applied_operations={applied_operations}\n{}",
        outcome.message
    );
    Ok(outcome)
}

fn count_applied_operations(message: &str) -> usize {
    let mut in_applied = false;
    let mut count = 0usize;
    for line in message.lines() {
        if line == "Applied:" {
            in_applied = true;
            continue;
        }
        if line.starts_with("Reason: ") {
            break;
        }
        if in_applied && line.starts_with("  - ") {
            count = count.saturating_add(1);
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_applied_operations_ignores_none() {
        assert_eq!(
            count_applied_operations("Applied:\n  none\nReason: map route requires human choice"),
            0
        );
    }

    #[test]
    fn count_applied_operations_counts_bullets_before_reason() {
        assert_eq!(
            count_applied_operations(
                "Applied:\n  - route planner\n  - combat search\nReason: done\n  - detail"
            ),
            2
        );
    }
}
