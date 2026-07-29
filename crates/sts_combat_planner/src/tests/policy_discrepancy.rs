use super::*;

#[test]
fn policy_discrepancy_search_follows_a_good_policy_to_terminal_truth() {
    let mut search = PolicyDiscrepancySession::with_policy(
        root(),
        PolicyDiscrepancyConfig {
            max_engine_steps_per_transition: 4,
            max_greedy_actions_per_dive: 8,
            ..PolicyDiscrepancyConfig::default()
        },
        Arc::new(PreferPlayPolicy),
    );

    let report = search.advance(
        &TinyTurnStepper::lethal(),
        PolicyDiscrepancyQuantum {
            additional_applied_transitions: 128,
            additional_engine_steps: 512,
            deadline: None,
        },
    );

    assert_eq!(report.status, PolicyDiscrepancyStatus::WitnessFound);
    assert_eq!(report.after.policy_dives, 1);
    assert_eq!(report.witness.unwrap().actions.len(), 1);
}

#[test]
fn policy_discrepancy_trajectory_audit_uses_runtime_policy_probabilities() {
    let mut search = PolicyDiscrepancySession::with_policy(
        root(),
        PolicyDiscrepancyConfig {
            max_engine_steps_per_transition: 4,
            ..PolicyDiscrepancyConfig::default()
        },
        Arc::new(PreferEndTurnPolicy),
    );

    let audit = search
        .audit_trajectory(&TinyTurnStepper::lethal(), &[PLAY])
        .expect("known legal winning trajectory should be auditable");

    assert_eq!(audit.source_action_count, 1);
    assert_eq!(audit.non_greedy_action_count, 1);
    assert_eq!(audit.terminal, CombatTerminal::Win);
    assert!(audit.total_weighted_discrepancy > 0.0);
    assert_eq!(audit.deviations[0].demonstrated_input, PLAY);
    assert_eq!(audit.deviations[0].greedy_input, ClientInput::EndTurn);
}

#[test]
fn policy_discrepancy_search_repairs_one_wrong_policy_action() {
    let mut search = PolicyDiscrepancySession::with_policy(
        root(),
        PolicyDiscrepancyConfig {
            max_engine_steps_per_transition: 4,
            max_greedy_actions_per_dive: 4,
            ..PolicyDiscrepancyConfig::default()
        },
        Arc::new(PreferEndTurnPolicy),
    );

    let report = search.advance(
        &TinyTurnStepper::lethal(),
        PolicyDiscrepancyQuantum {
            additional_applied_transitions: 128,
            additional_engine_steps: 512,
            deadline: None,
        },
    );

    assert_eq!(report.status, PolicyDiscrepancyStatus::WitnessFound);
    let witness = report
        .witness
        .expect("one discrepancy should repair policy");
    assert_eq!(
        witness.actions,
        exact_actions(&TinyTurnStepper::lethal(), &root(), [PLAY])
    );
    assert!(report.after.queued_discrepancies > 0);
}

#[test]
fn policy_discrepancy_search_split_quanta_match_one_shot_after_reroot() {
    let stepper = TinyTurnStepper::lethal_after_current_turn();
    let boundary = stepper.apply_to_stable(
        root().position(),
        ClientInput::EndTurn,
        CombatStepLimits {
            max_engine_steps: 4,
            deadline: None,
        },
    );
    let mut search = PolicyDiscrepancySession::with_policy(
        CombatDecisionRoot::new(boundary.position).expect("exact player-turn boundary"),
        PolicyDiscrepancyConfig {
            max_engine_steps_per_transition: 4,
            max_greedy_actions_per_dive: 4,
            ..PolicyDiscrepancyConfig::default()
        },
        Arc::new(PreferEndTurnPolicy),
    );

    let mut last = None;
    for _ in 0..256 {
        let report = search.advance(
            &stepper,
            PolicyDiscrepancyQuantum {
                additional_applied_transitions: 8,
                additional_engine_steps: 32,
                deadline: None,
            },
        );
        if report.status == PolicyDiscrepancyStatus::WitnessFound {
            return;
        }
        last = Some(report);
    }

    let report = last.expect("at least one quantum");
    panic!(
        "split search failed: status={:?}, counters={:?}, frontier={}",
        report.status, report.after, report.frontier_entries
    );
}

#[test]
fn policy_discrepancy_search_resumes_the_same_deviation_after_budget_exhaustion() {
    let mut search = PolicyDiscrepancySession::with_policy(
        root(),
        PolicyDiscrepancyConfig {
            max_engine_steps_per_transition: 4,
            ..PolicyDiscrepancyConfig::default()
        },
        Arc::new(PreferPlayPolicy),
    );

    let interrupted = search.advance(
        &TinyTurnStepper::lethal(),
        PolicyDiscrepancyQuantum {
            additional_applied_transitions: 0,
            additional_engine_steps: 0,
            deadline: None,
        },
    );
    assert_eq!(
        interrupted.status,
        PolicyDiscrepancyStatus::Partial(PolicyDiscrepancyInterruption::AppliedTransitionBudget)
    );

    let resumed = search.advance(
        &TinyTurnStepper::lethal(),
        PolicyDiscrepancyQuantum {
            additional_applied_transitions: 1,
            additional_engine_steps: 4,
            deadline: None,
        },
    );
    assert_eq!(resumed.status, PolicyDiscrepancyStatus::WitnessFound);
    assert_eq!(resumed.after.applied_action_transitions, 1);
}

#[test]
fn policy_discrepancy_search_resumes_a_greedy_dive_after_its_depth_quantum() {
    let mut search = PolicyDiscrepancySession::with_policy(
        root(),
        PolicyDiscrepancyConfig {
            max_engine_steps_per_transition: 4,
            max_greedy_actions_per_dive: 1,
            ..PolicyDiscrepancyConfig::default()
        },
        Arc::new(PreferSelection22Policy),
    );

    let report = search.advance(
        &TinyTurnStepper::with_winning_single_selection(),
        PolicyDiscrepancyQuantum {
            additional_applied_transitions: 8,
            additional_engine_steps: 32,
            deadline: None,
        },
    );

    assert_eq!(report.status, PolicyDiscrepancyStatus::WitnessFound);
    assert_eq!(report.after.greedy_depth_limit_hits, 2);
    assert_eq!(report.witness.unwrap().actions.len(), 2);
}

#[test]
fn policy_discrepancy_search_uses_policy_order_for_single_card_selection() {
    let mut search = PolicyDiscrepancySession::with_policy(
        root(),
        PolicyDiscrepancyConfig {
            max_engine_steps_per_transition: 4,
            ..PolicyDiscrepancyConfig::default()
        },
        Arc::new(PreferSelection22Policy),
    );

    let report = search.advance(
        &TinyTurnStepper::with_winning_single_selection(),
        PolicyDiscrepancyQuantum {
            additional_applied_transitions: 16,
            additional_engine_steps: 64,
            deadline: None,
        },
    );

    assert_eq!(report.status, PolicyDiscrepancyStatus::WitnessFound);
    assert_eq!(report.witness.unwrap().actions.len(), 2);
    assert_eq!(report.after.structured_inputs_materialized, 2);
}

#[test]
fn policy_discrepancy_search_keeps_turn_macros_lazy_when_policy_already_wins() {
    let mut search = PolicyDiscrepancySession::with_policy(
        root(),
        PolicyDiscrepancyConfig {
            max_engine_steps_per_transition: 4,
            turn_macro: Some(PolicyDiscrepancyTurnMacroConfig {
                max_applied_transitions: 8,
                proposals_per_view: 2,
                ..PolicyDiscrepancyTurnMacroConfig::default()
            }),
            ..PolicyDiscrepancyConfig::default()
        },
        Arc::new(PreferPlayPolicy),
    );

    let report = search.advance(
        &TinyTurnStepper::lethal(),
        PolicyDiscrepancyQuantum {
            additional_applied_transitions: 16,
            additional_engine_steps: 64,
            deadline: None,
        },
    );

    assert_eq!(report.status, PolicyDiscrepancyStatus::WitnessFound);
    assert_eq!(report.after.applied_action_transitions, 1);
    assert_eq!(report.after.turn_macro_generations, 0);
}

#[test]
fn policy_discrepancy_search_can_use_a_complete_turn_macro_as_one_deviation() {
    let mut search = PolicyDiscrepancySession::with_policy(
        root(),
        PolicyDiscrepancyConfig {
            max_engine_steps_per_transition: 4,
            max_greedy_actions_per_dive: 4,
            turn_macro: Some(PolicyDiscrepancyTurnMacroConfig {
                max_applied_transitions: 8,
                proposals_per_view: 2,
                ..PolicyDiscrepancyTurnMacroConfig::default()
            }),
            ..PolicyDiscrepancyConfig::default()
        },
        Arc::new(PreferEndTurnPolicy),
    );

    let report = search.advance(
        &TinyTurnStepper::lethal(),
        PolicyDiscrepancyQuantum {
            additional_applied_transitions: 32,
            additional_engine_steps: 128,
            deadline: None,
        },
    );

    assert_eq!(report.status, PolicyDiscrepancyStatus::WitnessFound);
    assert_eq!(report.after.turn_macro_generations, 1);
    assert!(report.after.turn_macro_options_enqueued > 0);
    assert_eq!(report.witness.unwrap().actions.len(), 1);
}

#[test]
fn policy_discrepancy_turn_macro_waits_for_its_full_budget_across_quanta() {
    let mut search = PolicyDiscrepancySession::with_policy(
        root(),
        PolicyDiscrepancyConfig {
            max_engine_steps_per_transition: 4,
            max_greedy_actions_per_dive: 4,
            turn_macro: Some(PolicyDiscrepancyTurnMacroConfig {
                max_applied_transitions: 8,
                proposals_per_view: 1,
                ..PolicyDiscrepancyTurnMacroConfig::default()
            }),
            ..PolicyDiscrepancyConfig::default()
        },
        Arc::new(PreferEndTurnPolicy),
    );

    let mut last = None;
    for _ in 0..128 {
        let report = search.advance(
            &TinyTurnStepper::lethal(),
            PolicyDiscrepancyQuantum {
                additional_applied_transitions: 1,
                additional_engine_steps: 4,
                deadline: None,
            },
        );
        if report.status == PolicyDiscrepancyStatus::WitnessFound {
            assert!(report.after.turn_macro_generations > 0);
            return;
        }
        last = Some(report);
    }

    let report = last.expect("at least one quantum");
    panic!(
        "a split budget never completed the exact turn macro: status={:?}, counters={:?}, frontier={}",
        report.status, report.after, report.frontier_entries
    );
}
