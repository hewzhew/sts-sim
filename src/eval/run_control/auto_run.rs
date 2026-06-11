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

    #[test]
    fn branch_experiment_auto_run_uses_match_and_keep_policy() {
        let mut session =
            RunControlSession::new(crate::eval::run_control::RunControlConfig::default());
        let mut event_state = EventState::new(EventId::MatchAndKeep);
        event_state.current_screen = 1;
        event_state.extra_data = match_and_keep_board_with_entries(&[
            (crate::content::cards::CardId::Bash, 1),
            (crate::content::cards::CardId::Strike, 0),
            (crate::content::cards::CardId::Defend, 0),
            (crate::content::cards::CardId::Clumsy, 0),
            (crate::content::cards::CardId::IronWave, 0),
            (crate::content::cards::CardId::Cleave, 0),
        ]);
        session.run_state.event_state = Some(event_state);
        session.engine_state = EngineState::EventRoom;

        let outcome = apply_branch_experiment_auto_run(
            &mut session,
            RunControlAutoStepOptions {
                max_operations: Some(2),
                ..Default::default()
            },
        )
        .expect("branch campaign auto-run should use Match and Keep event policy");

        assert!(outcome.message.contains("event policy: Match and Keep"));
        assert_eq!(
            session.run_state.master_deck.last().unwrap().id,
            crate::content::cards::CardId::IronWave
        );
    }

    fn match_and_keep_board_with_entries(
        entries: &[(crate::content::cards::CardId, u8); 6],
    ) -> Vec<i32> {
        let mut extra_data = vec![0, 0, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 0, 5, -1];
        for &(card_id, upgrades) in entries {
            extra_data.push(card_id as i32);
            extra_data.push(upgrades as i32);
        }
        extra_data.push(-1);
        extra_data.push(-1);
        extra_data
    }
}
