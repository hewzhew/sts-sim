use serde::Serialize;

use super::combat_line_executor::drawn_cards_from_action_result;
use super::combat_line_trace::{
    combat_automation_opportunity_state_v1, combat_automation_step_state_v1,
};
use super::oracle_run_explorer::run_session_fingerprint_v1;
use super::{
    RunCombatResolutionBoundaryV1, RunControlConfig, RunControlSession, RunDecisionBoundaryV1,
    RunProgressJournalV1, RunProgressStepV1,
};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExactRunProgressReplayReportV1 {
    pub seed: u64,
    pub ascension: u8,
    pub journal_entries: usize,
    pub decisions: usize,
    pub forced_transitions: usize,
    pub combat_resolutions: usize,
    pub combat_actions: usize,
    pub final_fingerprint: String,
    pub act: u8,
    pub floor: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub engine_state: String,
}

/// Re-executes a committed run journal from the canonical initial state and
/// verifies every recorded decision/combat boundary plus the final normalized
/// session fingerprint. This is deliberately independent of owner policy and
/// search: a saved witness is accepted only when its exact recorded actions
/// still produce the saved terminal state.
pub fn exact_replay_run_progress_journal_v1(
    seed: u64,
    ascension: u8,
    journal: &RunProgressJournalV1,
    expected_final: &RunControlSession,
) -> Result<ExactRunProgressReplayReportV1, String> {
    let mut session = RunControlSession::new(RunControlConfig {
        seed,
        ascension_level: ascension,
        final_act: false,
        player_class: expected_final.run_state.player_class,
        reward_automation: expected_final.reward_automation.clone(),
        ..RunControlConfig::default()
    });
    let mut decisions = 0usize;
    let mut forced_transitions = 0usize;
    let mut combat_resolutions = 0usize;
    let mut combat_actions = 0usize;

    for (entry_index, entry) in journal.entries().iter().enumerate() {
        match entry {
            RunProgressStepV1::Decision(record) => {
                let actual_before = RunDecisionBoundaryV1::capture(&session);
                if !decision_boundaries_match(&actual_before, &record.before) {
                    return Err(format!(
                        "journal entry {entry_index} decision before-boundary mismatch: expected {:?}, got {:?}",
                        record.before,
                        actual_before,
                    ));
                }
                session
                    .apply_decision_action(record.action.clone())
                    .map_err(|error| {
                        format!("journal entry {entry_index} decision replay failed: {error}")
                    })?;
                let actual_after = RunDecisionBoundaryV1::capture(&session);
                if !decision_boundaries_match(&actual_after, &record.after) {
                    return Err(format!(
                        "journal entry {entry_index} decision after-boundary mismatch: expected {:?}, got {:?}",
                        record.after,
                        actual_after,
                    ));
                }
                decisions = decisions.saturating_add(1);
            }
            RunProgressStepV1::ForcedTransition(record) => {
                let actual_before = RunDecisionBoundaryV1::capture(&session);
                if !decision_boundaries_match(&actual_before, &record.before) {
                    return Err(format!(
                        "journal entry {entry_index} forced-transition before-boundary mismatch"
                    ));
                }
                session
                    .apply_forced_transition(record.kind)
                    .map_err(|error| {
                        format!(
                            "journal entry {entry_index} forced-transition replay failed: {error}"
                        )
                    })?;
                let actual_after = RunDecisionBoundaryV1::capture(&session);
                if !decision_boundaries_match(&actual_after, &record.after) {
                    return Err(format!(
                        "journal entry {entry_index} forced-transition after-boundary mismatch"
                    ));
                }
                forced_transitions = forced_transitions.saturating_add(1);
            }
            RunProgressStepV1::CombatResolution(record) => {
                let actual_before = RunCombatResolutionBoundaryV1::capture(&session);
                if !combat_boundaries_match(&actual_before, &record.before) {
                    return Err(format!(
                        "journal entry {entry_index} combat before-boundary mismatch: expected '{} @ {}', got '{} @ {}'",
                        record.before.title,
                        record.before.location,
                        actual_before.title,
                        actual_before.location,
                    ));
                }
                session.mark_current_combat_search_resolved();
                for (action_index, action) in record.trajectory.actions.iter().enumerate() {
                    let opportunity = combat_automation_opportunity_state_v1(&session);
                    if opportunity != action.opportunity_before {
                        return Err(format!(
                            "journal entry {entry_index} combat action {action_index} opportunity mismatch"
                        ));
                    }
                    let outcome = session
                        .apply_combat_resolution_input(action.input.clone())
                        .map_err(|error| {
                            format!(
                                "journal entry {entry_index} combat action {action_index} replay failed: {error}"
                            )
                        })?;
                    let drawn_cards =
                        drawn_cards_from_action_result(outcome.action_result.as_ref());
                    if drawn_cards != action.drawn_cards {
                        return Err(format!(
                            "journal entry {entry_index} combat action {action_index} drawn-card mismatch"
                        ));
                    }
                    let combat_after = combat_automation_step_state_v1(&session);
                    if combat_after != action.combat_after {
                        return Err(format!(
                            "journal entry {entry_index} combat action {action_index} successor mismatch"
                        ));
                    }
                    combat_actions = combat_actions.saturating_add(1);
                }
                let actual_after = RunCombatResolutionBoundaryV1::capture(&session);
                if !combat_boundaries_match(&actual_after, &record.after) {
                    return Err(format!(
                        "journal entry {entry_index} combat after-boundary mismatch: expected {:?}, got {:?}",
                        record.after,
                        actual_after,
                    ));
                }
                combat_resolutions = combat_resolutions.saturating_add(1);
            }
            RunProgressStepV1::Stop(_) => {
                return Err(format!(
                    "journal entry {entry_index} contains a non-committed Stop record"
                ));
            }
        }
    }

    let final_fingerprint = run_session_fingerprint_v1(&session);
    let expected_fingerprint = run_session_fingerprint_v1(expected_final);
    if final_fingerprint != expected_fingerprint {
        return Err(format!(
            "journal replay final fingerprint mismatch: expected {expected_fingerprint}, got {final_fingerprint}"
        ));
    }

    Ok(ExactRunProgressReplayReportV1 {
        seed,
        ascension,
        journal_entries: journal.len(),
        decisions,
        forced_transitions,
        combat_resolutions,
        combat_actions,
        final_fingerprint,
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        current_hp: session.run_state.current_hp,
        max_hp: session.run_state.max_hp,
        engine_state: format!("{:?}", session.engine_state),
    })
}

fn combat_boundaries_match(
    actual: &RunCombatResolutionBoundaryV1,
    expected: &RunCombatResolutionBoundaryV1,
) -> bool {
    // combat_sequence is diagnostic bookkeeping and is deliberately removed
    // by the canonical run-session fingerprint. Historical search/import
    // paths can observe the same exact combat with a different counter while
    // preserving every game-semantic state transition.
    actual.decision_step == expected.decision_step
        && actual.title == expected.title
        && actual.location == expected.location
        && actual.active_combat == expected.active_combat
}

fn decision_boundaries_match(
    actual: &RunDecisionBoundaryV1,
    expected: &RunDecisionBoundaryV1,
) -> bool {
    // The visible candidate surface may grow when a newer build exposes an
    // additional legal action (for example explicit potion discard). The
    // journal owns the action that was actually committed; replay validates
    // that action against the current exact state. Requiring every unrelated
    // visible candidate to remain byte-identical would reject valid witnesses
    // for a presentation/schema change rather than a game-state divergence.
    actual.decision_step == expected.decision_step
        && actual.title == expected.title
        && actual.location == expected.location
}
