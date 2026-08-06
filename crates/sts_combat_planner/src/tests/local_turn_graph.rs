use super::*;

#[test]
fn local_turn_graph_retires_finished_generator_search_storage() {
    let mut session = LocalTurnGraphWitnessSession::with_policy(
        root(),
        LocalTurnGraphWitnessConfig {
            generator: config(),
            initial_expansion_work: 16,
            root_initial_expansion_work: 16,
            max_turn_depth: 1,
            satisfaction: OracleCombatWitnessSatisfaction::BudgetOrExhaustion,
            ..LocalTurnGraphWitnessConfig::default()
        },
        Arc::new(PreferPlayPolicy),
    );
    session.advance(
        LocalTurnGraphWitnessQuantum {
            additional_selections: 64,
            additional_generation_work: 256,
            additional_engine_steps: 1_024,
            deadline: None,
        },
        &TinyTurnStepper::plain(),
    );

    let storage = session.storage_snapshot();
    assert!(storage.finished_generators > 0);
    assert_eq!(storage.finished_generator_work_capacity, 0);
    assert_eq!(storage.finished_generator_seen_capacity, 0);
    assert_eq!(storage.finished_generator_anchor_capacity, 0);
    assert_eq!(storage.finished_generator_guide_capacity, 0);
    assert_eq!(storage.finished_generator_scheduled_round_capacity, 0);
}

#[test]
fn local_turn_graph_absolute_final_hp_satisfaction_uses_terminal_hp() {
    let decision_root = root();
    let terminal_hp = decision_root.position().combat.entities.player.current_hp;
    let config_for = |minimum| LocalTurnGraphWitnessConfig {
        generator: config(),
        initial_expansion_work: 16,
        root_initial_expansion_work: 16,
        max_turn_depth: 1,
        satisfaction: OracleCombatWitnessSatisfaction::FinalHpAtLeast(minimum),
        ..LocalTurnGraphWitnessConfig::default()
    };
    let quantum = LocalTurnGraphWitnessQuantum {
        additional_selections: 64,
        additional_generation_work: 256,
        additional_engine_steps: 1_024,
        deadline: None,
    };

    let mut reached = LocalTurnGraphWitnessSession::with_policy(
        decision_root.clone(),
        config_for(terminal_hp),
        Arc::new(PreferPlayPolicy),
    );
    let reached_report = reached.advance(quantum, &TinyTurnStepper::lethal());
    assert_eq!(
        reached_report.status,
        LocalTurnGraphWitnessStatus::WitnessFound
    );

    let mut missed = LocalTurnGraphWitnessSession::with_policy(
        decision_root,
        config_for(terminal_hp.saturating_add(1)),
        Arc::new(PreferPlayPolicy),
    );
    let missed_report = missed.advance(quantum, &TinyTurnStepper::lethal());
    assert_ne!(
        missed_report.status,
        LocalTurnGraphWitnessStatus::WitnessFound
    );
    assert_eq!(
        missed
            .witness()
            .expect("the lower-HP exact witness remains diagnostic evidence")
            .final_position
            .combat
            .entities
            .player
            .current_hp,
        terminal_hp
    );
}

#[test]
fn local_turn_graph_policy_line_defers_reserved_conversion_at_a_safe_phase_boundary() {
    let stepper = TinyTurnStepper::activating_finite_skill_conversion();
    let mut session = LocalTurnGraphWitnessSession::with_policy(
        awakened_conversion_root(),
        LocalTurnGraphWitnessConfig {
            generator: config(),
            ..LocalTurnGraphWitnessConfig::default()
        },
        Arc::new(PreferPlayPolicy),
    );

    let report = session
        .offer_plan_compatible_policy_line(1, 4, &stepper)
        .expect("plan-compatible line");

    assert_eq!(report.proposed_turns, 1);
    assert_eq!(report.deferred_actions, 1);
    assert_eq!(report.rejected_preview_transitions, 1);
    assert_eq!(stepper.call_count(&PLAY), 1);
    assert_eq!(stepper.call_count(&ClientInput::EndTurn), 1);
    let families = session.root_action_families();
    assert_eq!(families.len(), 1);
    assert_eq!(families[0].first_action, ClientInput::EndTurn);
}

#[test]
fn local_turn_graph_policy_line_does_not_end_before_an_emergency_conversion() {
    let stepper = TinyTurnStepper::activating_finite_skill_conversion();
    let mut session = LocalTurnGraphWitnessSession::with_policy(
        threatened_awakened_conversion_root(),
        LocalTurnGraphWitnessConfig {
            generator: config(),
            ..LocalTurnGraphWitnessConfig::default()
        },
        Arc::new(PreferPlayPolicy),
    );

    let report = session
        .offer_plan_compatible_policy_line(1, 1, &stepper)
        .expect("plan-compatible line");

    assert_eq!(report.deferred_actions, 0);
    assert_eq!(stepper.call_count(&PLAY), 1);
    assert_eq!(stepper.call_count(&ClientInput::EndTurn), 0);
}

#[test]
fn local_turn_graph_policy_line_does_not_defer_a_phase_committing_action() {
    let stepper = TinyTurnStepper {
        activates_finite_skill_conversion: true,
        opens_awakened_transition_window: true,
        ..TinyTurnStepper::plain()
    };
    let mut session = LocalTurnGraphWitnessSession::with_policy(
        awakened_conversion_root(),
        LocalTurnGraphWitnessConfig {
            generator: config(),
            ..LocalTurnGraphWitnessConfig::default()
        },
        Arc::new(PreferPlayPolicy),
    );

    let report = session
        .offer_plan_compatible_policy_line(1, 1, &stepper)
        .expect("plan-compatible line");

    assert_eq!(report.deferred_actions, 0);
    assert_eq!(stepper.call_count(&PLAY), 1);
    assert_eq!(stepper.call_count(&ClientInput::EndTurn), 0);
}

#[test]
fn local_turn_graph_policy_line_deploys_conversion_in_the_untaxed_window() {
    let stepper = TinyTurnStepper::activating_finite_skill_conversion();
    let mut session = LocalTurnGraphWitnessSession::with_policy(
        transition_window_conversion_root(),
        LocalTurnGraphWitnessConfig {
            generator: config(),
            ..LocalTurnGraphWitnessConfig::default()
        },
        Arc::new(PreferEndTurnPolicy),
    );

    let report = session
        .offer_plan_compatible_policy_line(1, 4, &stepper)
        .expect("plan-compatible line");

    assert_eq!(report.proposed_turns, 1);
    assert_eq!(report.deferred_actions, 0);
    let families = session.root_action_families();
    assert_eq!(families.len(), 1);
    assert_eq!(families[0].first_action, PLAY);
}

#[test]
fn local_turn_graph_policy_line_crosses_a_ranked_single_selection_transaction() {
    let stepper = TinyTurnStepper::with_winning_single_selection();
    let mut session = LocalTurnGraphWitnessSession::with_policy(
        awakened_root(),
        LocalTurnGraphWitnessConfig {
            generator: config(),
            ..LocalTurnGraphWitnessConfig::default()
        },
        Arc::new(PreferSelection22Policy),
    );

    let report = session
        .offer_plan_compatible_policy_line(1, 4, &stepper)
        .expect("plan-compatible line");

    assert!(report.reached_terminal_win);
    assert_eq!(report.chosen_action_transitions, 2);
    assert_eq!(
        stepper.call_count(&PLAY),
        2,
        "the proposed play and the authoritative witness replay each execute once"
    );
    assert!(session.witness().is_some());
}

#[test]
fn local_turn_graph_plan_annotations_are_opt_in_and_read_only() {
    let root = awakened_root();
    let parent_hash = root.exact_state_hash().to_owned();
    let successor_stepper = TinyTurnStepper::plain();
    let direct_end_turn = successor_stepper.apply_to_stable(
        root.position(),
        ClientInput::EndTurn,
        CombatStepLimits {
            max_engine_steps: 4,
            deadline: None,
        },
    );
    let successor_hash = exact_hash(&direct_end_turn.position);
    let search_config = LocalTurnGraphWitnessConfig {
        generator: config(),
        generation_quantum_work: 4,
        backed_generation_quantum_work: 4,
        initial_expansion_work: 16,
        root_initial_expansion_work: 16,
        max_turn_depth: 1,
        satisfaction: OracleCombatWitnessSatisfaction::BudgetOrExhaustion,
        ..LocalTurnGraphWitnessConfig::default()
    };
    let quantum = LocalTurnGraphWitnessQuantum {
        additional_selections: 64,
        additional_generation_work: 256,
        additional_engine_steps: 1_024,
        deadline: None,
    };

    let mut plain = LocalTurnGraphWitnessSession::with_policy(
        root.clone(),
        search_config,
        Arc::new(PreferPlayPolicy),
    );
    let plain_report = plain.advance(quantum, &TinyTurnStepper::plain());
    let plain_edge = plain
        .edge_snapshot_by_exact_hashes(&parent_hash, &successor_hash)
        .expect("the direct end-turn edge must be materialized");
    assert_eq!(plain_edge.plan_transition_annotation, None);
    assert_eq!(plain.counters().annotated_exact_edges, 0);

    let mut annotated =
        LocalTurnGraphWitnessSession::with_policy(root, search_config, Arc::new(PreferPlayPolicy));
    annotated
        .enable_plan_transition_annotations()
        .expect("annotation collection must be enabled before graph construction");
    let annotated_report = annotated.advance(quantum, &TinyTurnStepper::plain());
    let annotated_edge = annotated
        .edge_snapshot_by_exact_hashes(&parent_hash, &successor_hash)
        .expect("the annotated direct end-turn edge must be materialized");
    assert!(
        annotated_edge.plan_transition_annotation.is_some(),
        "an encounter-owned typed plan must annotate its exact edge"
    );
    assert!(annotated.counters().annotated_exact_edges > 0);
    let plan_edges = annotated.plan_transition_edge_snapshots();
    assert_eq!(plan_edges.len(), annotated.counters().annotated_exact_edges);
    assert!(plan_edges.iter().any(|edge| {
        edge.parent_exact_state_hash == parent_hash
            && edge.successor_exact_state_hash == successor_hash
            && edge.action_count == annotated_edge.actions.len()
    }));
    assert_eq!(
        annotated.enable_plan_transition_annotations(),
        Err(LocalTurnGraphPlanAnnotationEnableError::EdgesAlreadyMaterialized)
    );

    let mut plain_counters = plain.counters();
    let mut annotated_counters = annotated.counters();
    annotated_counters.annotated_exact_edges = 0;
    plain_counters.annotated_exact_edges = 0;
    assert_eq!(annotated_report.status, plain_report.status);
    assert_eq!(annotated_counters, plain_counters);
    assert_eq!(annotated_edge.actions, plain_edge.actions);
    assert_eq!(
        annotated_edge.negative_log_policy,
        plain_edge.negative_log_policy
    );
}

#[test]
fn state_service_index_attributes_plan_prefix_work_to_its_exact_boundary() {
    let root = double_thief_bridge_root();
    let root_hash = root.exact_state_hash().to_owned();
    let mut session = LocalTurnGraphWitnessSession::with_policy(
        root,
        LocalTurnGraphWitnessConfig {
            generator: TurnOptionGeneratorConfig {
                max_engine_steps_per_transition: 250,
                ..TurnOptionGeneratorConfig::default()
            },
            generation_quantum_work: 4,
            backed_generation_quantum_work: 4,
            initial_expansion_work: 16,
            root_initial_expansion_work: 16,
            max_turn_depth: 1,
            satisfaction: OracleCombatWitnessSatisfaction::BudgetOrExhaustion,
            ..LocalTurnGraphWitnessConfig::default()
        },
        Arc::new(PreferPlayPolicy),
    );

    session.advance(
        LocalTurnGraphWitnessQuantum {
            additional_selections: 64,
            additional_generation_work: 256,
            additional_engine_steps: 8_192,
            deadline: None,
        },
        &EngineCombatStepper,
    );

    let state = session
        .state_service_index()
        .into_iter()
        .find(|state| state.exact_state_hash == root_hash)
        .expect("root service attribution");
    assert!(state.plan_prefix_applicable);
    assert!(state.plan_prefix_step_count.is_some_and(|count| count > 0));
    assert_eq!(state.plan_prefix_attempts, 1);
    assert_eq!(state.plan_prefix_completed, 1);
    assert_eq!(state.plan_prefix_rejections, 0);
    assert_eq!(state.plan_prefix_successor_exact_state_hashes.len(), 1);
    assert!(state.generation_anchor_services > 0);
}

#[test]
fn local_turn_graph_plan_annotations_leave_unowned_encounters_empty() {
    let mut session = LocalTurnGraphWitnessSession::with_policy(
        root(),
        LocalTurnGraphWitnessConfig {
            generator: config(),
            initial_expansion_work: 16,
            root_initial_expansion_work: 16,
            max_turn_depth: 1,
            satisfaction: OracleCombatWitnessSatisfaction::BudgetOrExhaustion,
            ..LocalTurnGraphWitnessConfig::default()
        },
        Arc::new(PreferPlayPolicy),
    );
    session
        .enable_plan_transition_annotations()
        .expect("an empty graph can enable annotations");
    session.advance(
        LocalTurnGraphWitnessQuantum {
            additional_selections: 64,
            additional_generation_work: 256,
            additional_engine_steps: 1_024,
            deadline: None,
        },
        &TinyTurnStepper::plain(),
    );

    assert!(session.counters().exact_edges > 0);
    assert_eq!(session.counters().annotated_exact_edges, 0);
    assert!(session.plan_transition_edge_snapshots().is_empty());
}
