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
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};

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

    #[test]
    fn branch_experiment_auto_run_consumes_terminal_single_event_leave() {
        let mut session =
            RunControlSession::new(crate::eval::run_control::RunControlConfig::default());
        session.run_state.event_state = Some(EventState {
            id: EventId::Beggar,
            current_screen: 2,
            internal_state: 0,
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        });
        session.engine_state = EngineState::EventRoom;

        let outcome = apply_branch_experiment_auto_run(
            &mut session,
            RunControlAutoStepOptions {
                max_operations: Some(1),
                ..Default::default()
            },
        )
        .expect("branch experiment auto-run should consume the terminal event leave");

        assert!(
            matches!(session.engine_state, EngineState::MapNavigation),
            "state={:?}\nmessage={}",
            session.engine_state,
            outcome.message
        );
        assert!(outcome.message.contains("routine: Leave"));
    }
}
