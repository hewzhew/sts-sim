fn collect_rust_sources(root: &std::path::Path, paths: &mut Vec<std::path::PathBuf>) {
    if root.is_file() {
        if root.extension().is_some_and(|extension| extension == "rs") {
            paths.push(root.to_path_buf());
        }
        return;
    }

    for entry in std::fs::read_dir(root).expect("read Rust source directory") {
        let path = entry.expect("read Rust source entry").path();
        if path.is_dir() {
            collect_rust_sources(&path, paths);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            paths.push(path);
        }
    }
}

#[test]
fn runtime_branch_does_not_path_import_branch_tiny_bin_modules() {
    let owner_audit = std::fs::read_to_string("src/runtime/branch/owner_audit.rs")
        .expect("read owner_audit runtime module");

    assert!(
        !owner_audit.contains("../../bin/branch_tiny"),
        "runtime owner_audit must own its implementation modules instead of path-importing bin files"
    );
}

#[test]
fn retired_repl_and_multi_operation_auto_run_do_not_return() {
    for retired in [
        "src/bin/run_play_driver/main.rs",
        "src/eval/run_play.rs",
        "src/eval/run_control/bookmarks.rs",
        "src/eval/run_control/auto_run.rs",
        "src/eval/neow_guided_prefix.rs",
        "src/eval/run_control/commands.rs",
        "src/eval/run_control/commands/help.rs",
        "src/eval/run_control/commands/options.rs",
        "src/eval/run_control/commands/parse.rs",
        "src/eval/run_control/commands/tests.rs",
        "src/eval/run_control/artifact_commands.rs",
        "src/eval/run_control/search_defaults.rs",
        "src/eval/run_control/trace_replay.rs",
        "src/eval/run_control/session_trace_outcome.rs",
        "src/eval/run_control/panels/map.rs",
    ] {
        assert!(
            !std::path::Path::new(retired).exists(),
            "retired human-command surface must stay deleted: {retired}"
        );
    }

    let mut sources = Vec::new();
    collect_rust_sources(std::path::Path::new("src/eval/run_control"), &mut sources);
    collect_rust_sources(std::path::Path::new("src/runtime/branch"), &mut sources);
    for path in sources {
        if path.ends_with("commands/tests.rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read run execution source");
        for forbidden in [
            "RunControlCommand::AutoRun",
            "apply_owner_audit_auto_run",
            "max_operations",
        ] {
            assert!(
                !source.contains(forbidden),
                "run execution source '{}' must not restore retired `{forbidden}`",
                path.display()
            );
        }
    }
}

#[test]
fn run_control_has_no_legacy_command_parser_recorder_or_replay_executor() {
    let mut sources = Vec::new();
    collect_rust_sources(std::path::Path::new("src/eval/run_control"), &mut sources);

    for path in sources {
        let source = std::fs::read_to_string(&path).expect("read run-control source");
        for forbidden in [
            "RunControlCommand",
            "parse_run_control_command",
            ".apply_command(",
            "SessionTraceRecorder",
            "replay_session_trace",
        ] {
            assert!(
                !source.contains(forbidden),
                "run-control source '{}' must not restore retired `{forbidden}`",
                path.display()
            );
        }
    }

    let trace_reader = std::fs::read_to_string("src/eval/run_control/session_trace.rs")
        .expect("read historical trace schema reader");
    assert!(trace_reader.contains("load_session_trace_v1"));
    assert!(trace_reader.contains("raw_command_line"));
    assert!(
        !trace_reader.contains("apply_decision_action"),
        "historical trace compatibility is read/export only, never an execution path"
    );
}

#[test]
fn live_planner_boundary_capture_uses_only_public_typed_state() {
    let source = std::fs::read_to_string("src/eval/run_control/planner_boundary_capture.rs")
        .expect("read live planner boundary capture");
    let production = source
        .split("#[cfg(test)]")
        .next()
        .expect("production capture source");

    for forbidden in [
        "raw_command",
        "RunControlCommandStream",
        "rng_pool",
        "neow_rng",
        "monster_list",
        "elite_monster_list",
        "boss_list",
        "event_generator",
    ] {
        assert!(
            !production.contains(forbidden),
            "live planner boundary capture must not read hidden or retired `{forbidden}`"
        );
    }
    for required in [
        "capture_planner_boundary_v1",
        "PlannerBoundaryCaptureSegmentV1",
        "CandidateCompletenessBasis::RunControlBoundaryEnumerator",
        "SelectionNotRepresented",
        "ProgressBudgetExhausted",
        "WallDeadlineReached",
    ] {
        assert!(
            production.contains(required),
            "live planner boundary capture must retain typed contract `{required}`"
        );
    }
}

#[test]
fn fingerprint_and_rendering_do_not_materialize_combinatorial_legal_actions() {
    for path in [
        "src/eval/fingerprint.rs",
        "src/eval/combat_capture.rs",
        "src/eval/run_control/render.rs",
    ] {
        let source = std::fs::read_to_string(path).expect("read bounded diagnostic source");
        let production = source.split("#[cfg(test)]").next().unwrap_or(&source);
        for forbidden in [
            "get_legal_moves",
            "engine_local_moves",
            "legal_moves_for_audit",
            "canonical_pending_choice_inputs",
        ] {
            assert!(
                !production.contains(forbidden),
                "{path} must not materialize a combinatorial action surface through `{forbidden}`"
            );
        }
    }

    let surface = std::fs::read_to_string("src/sim/combat_action_surface.rs")
        .expect("read symbolic action-surface owner");
    let production = surface.split("#[cfg(test)]").next().unwrap_or(&surface);
    for forbidden in [
        "get_legal_moves",
        "legal_moves_for_audit",
        "canonical_pending_choice_inputs",
        "extend_scry_moves",
    ] {
        assert!(
            !production.contains(forbidden),
            "symbolic action-surface owner must not materialize choices through `{forbidden}`"
        );
    }

    let candidates = std::fs::read_to_string("src/eval/run_control/view_model/candidates.rs")
        .expect("read run-control candidate projector");
    let pending_choice_projector = candidates
        .split("fn pending_choice_candidates")
        .nth(1)
        .and_then(|tail| tail.split("fn selection_surface_note").next())
        .expect("locate pending-choice candidate projector");
    assert!(pending_choice_projector.contains("combat_legal_action_surface_v2"));
    for forbidden in [
        "get_legal_moves",
        "engine_local_moves",
        "legal_moves_for_audit",
        "canonical_pending_choice_inputs",
        "extend_scry_moves",
    ] {
        assert!(
            !pending_choice_projector.contains(forbidden),
            "pending-choice rendering must not fall back through `{forbidden}`"
        );
    }
}

#[test]
fn engine_action_domain_keeps_structured_selections_out_of_atomic_vectors() {
    let actions = std::fs::read_to_string("src/sim/combat_legal_actions.rs")
        .expect("read engine atomic-action owner");
    let production = actions.split("#[cfg(test)]").next().unwrap_or(&actions);
    for retired in [
        "extend_hand_select_moves",
        "extend_grid_select_moves",
        "extend_scry_moves",
        "collect_scry_index_combinations",
        "selection_pool_cap",
        "selection_generation_max",
        "generate_ranked_combinations",
        "collect_ranked_combinations",
        "get_legal_moves",
        "legal_moves_for_audit",
    ] {
        assert!(
            !production.contains(retired),
            "engine atomic-action owner must not restore eager helper `{retired}`"
        );
    }
    assert!(production.contains("engine_atomic_actions"));
    assert!(production.contains("combat_legal_action_surface_v2"));
    assert!(production.contains("pending_choice_input_is_legal"));

    let stepper = std::fs::read_to_string("src/sim/combat.rs").expect("read combat stepper");
    let production = stepper.split("#[cfg(test)]").next().unwrap_or(&stepper);
    assert!(production.contains("fn atomic_actions"));
    assert!(production.contains("fn legal_action_surface"));
    assert!(
        !production.contains("fn legal_actions"),
        "CombatStepper must not describe an atomic-only Vec as the complete legal action set"
    );
}

#[test]
fn run_control_combat_membership_delegates_to_the_simulator_owner() {
    let selection = std::fs::read_to_string("src/eval/run_control/selection_surface.rs")
        .expect("read run-control selection surface");
    let production = selection.split("#[cfg(test)]").next().unwrap_or(&selection);
    assert!(production.contains("pending_choice_input_is_legal"));
    for duplicate in [
        "uuid_selection_is_allowed",
        "validate_indices_in_range",
        "reject_duplicate_indices",
        "hand_contains_all",
        "grid_source_contains_all",
        "pile_contains_all",
    ] {
        assert!(
            !production.contains(duplicate),
            "run control must not restore duplicate combat membership helper `{duplicate}`"
        );
    }

    let input_gate = std::fs::read_to_string("src/eval/run_control/input_gate.rs")
        .expect("read run-control input gate");
    let production = input_gate
        .split("#[cfg(test)]")
        .next()
        .unwrap_or(&input_gate);
    assert!(production.contains("is_legal_move"));
    assert!(!production.contains("get_legal_moves"));
}

#[test]
fn visible_input_candidates_execute_as_atomic_decision_transactions() {
    let transaction = std::fs::read_to_string("src/eval/run_control/decision_transaction.rs")
        .expect("read decision transaction contract");
    for required in [
        "selected candidate is absent from the before boundary",
        "selected candidate action disagrees with the executed action",
        "decision transaction did not advance exactly one decision step",
    ] {
        assert!(
            transaction.contains(required),
            "decision transaction must fail closed on `{required}`"
        );
    }

    let executor = std::fs::read_to_string("src/eval/run_control/session/apply.rs")
        .expect("read run decision executor");
    assert!(executor.contains("let before = RunDecisionBoundaryV1::capture(self);"));
    assert!(
        executor.contains("self.execute_decision_action_inner(action.clone(), candidate_label)?")
    );
    assert!(executor.contains("let after = RunDecisionBoundaryV1::capture(self);"));
    assert!(executor.contains("transaction.project_progress_outcome(self)"));
    assert!(executor.contains("execute_custom_decision_atomically"));
    assert!(executor.contains("execute_singing_bowl_card_reward_inner"));
    assert!(!executor.contains("transaction v1 currently supports ordinary input candidates only"));

    let card_reward_executor = std::fs::read_to_string("src/eval/run_control/card_reward_auto.rs")
        .expect("read card reward executor");
    assert!(!card_reward_executor.contains("apply_singing_bowl_to_visible_card_reward_item"));

    let progress_outcome = std::fs::read_to_string("src/eval/run_control/session.rs")
        .expect("read progress outcome contract");
    assert!(progress_outcome.contains("pub progress_steps: Vec<RunProgressStepV1>"));
    for retired_parallel_field in [
        "pub auto_stop:",
        "pub decision_transactions:",
        "pub forced_transitions:",
        "pub combat_resolutions:",
    ] {
        assert!(
            !progress_outcome.contains(retired_parallel_field),
            "RunProgressOutcome must not restore parallel semantic field `{retired_parallel_field}`"
        );
    }

    let progress_step = std::fs::read_to_string("src/eval/run_control/progress_step.rs")
        .expect("read typed progress-step contract");
    for required_variant in [
        "Decision(RunDecisionTransactionV1)",
        "ForcedTransition(RunForcedTransitionV1)",
        "CombatResolution(RunCombatResolutionV1)",
        "Stop(RunControlAutoStopV1)",
    ] {
        assert!(progress_step.contains(required_variant));
    }
    assert!(progress_outcome.contains("Stop must be the final progress step"));

    let auto_step = std::fs::read_to_string("src/eval/run_control/auto_step.rs")
        .expect("read atomic auto-step executor");
    assert!(auto_step.contains("execute_routine_candidate_transaction"));
    assert!(auto_step.contains(".extend(outcome.progress_steps.iter().cloned())"));
    assert!(auto_step.contains(".with_progress_steps(applied.progress_steps)"));
    assert!(!auto_step.contains("RunControlAutoStopKind::ProgressApplied"));

    let bounded_driver = std::fs::read_to_string("src/eval/run_control/bounded_run_driver.rs")
        .expect("read bounded run driver");
    assert!(bounded_driver.contains("max_progress_steps"));
    assert!(bounded_driver.contains("WallDeadlineReached"));
    assert!(bounded_driver.contains("ProgressBudgetExhausted"));
    assert!(bounded_driver.contains("CombatBoundary"));
    assert!(bounded_driver.contains("session.apply_progress_step(options.clone())"));

    let route_executor = std::fs::read_to_string("src/eval/run_control/route_policy/apply.rs")
        .expect("read route policy executor");
    assert!(route_executor.contains("public_route_candidate_id(session, &input)"));
    assert!(route_executor.contains("execute_route_candidate_transaction"));
    assert!(!route_executor.contains("session.apply_input(input)"));

    let map_candidates = std::fs::read_to_string("src/eval/run_control/view_model/candidates.rs")
        .expect("read public candidate enumeration");
    assert!(map_candidates.contains("ClientInput::FlyToNode"));
    let input_gate = std::fs::read_to_string("src/eval/run_control/input_gate.rs")
        .expect("read input legality gate");
    assert!(!input_gate.contains("fn map_flight_is_allowed"));
}

#[test]
fn owner_audit_executes_typed_actions_without_the_legacy_command_kernel() {
    let mut sources = Vec::new();
    collect_rust_sources(
        std::path::Path::new("src/runtime/branch/owner_audit"),
        &mut sources,
    );
    sources.push(std::path::PathBuf::from(
        "src/runtime/branch/owner_audit.rs",
    ));

    for path in sources {
        let source = std::fs::read_to_string(&path).expect("read owner-audit source");
        for forbidden in [
            "RunControlCommand::",
            "RunControlCommand,",
            "RunControlCommand;",
            ".apply_command(",
            ".executable_command(",
            "OwnerRoutine::Command",
            "OwnerRoutine::Action",
            "OwnerRoutine::RewardTinyAutomation",
        ] {
            assert!(
                !source.contains(forbidden),
                "owner-audit source '{}' must execute typed RunDecisionAction values, not `{forbidden}`",
                path.display()
            );
        }
    }

    for path in [
        "src/runtime/branch/owner_audit/owner_choice_expander.rs",
        "src/runtime/branch/owner_audit/owner_routines.rs",
    ] {
        let source = std::fs::read_to_string(path).expect("read owner execution source");
        assert!(source.contains("apply_owner_candidate"));
        assert!(
            !source.contains("apply_decision_action"),
            "owner execution source '{path}' must preserve a public candidate id"
        );
    }

    let owner_model = std::fs::read_to_string("src/runtime/branch/owner_audit/owner_model.rs")
        .expect("read owner choice contract");
    assert!(owner_model.contains("pub(super) candidate_id: String"));

    let owner_executor = std::fs::read_to_string("src/eval/run_control/session/apply.rs")
        .expect("read owner transaction executor");
    assert!(owner_executor.contains("RunDecisionSelectionSourceV1::OwnerPolicy"));
    assert!(owner_executor.contains("DecisionCandidateKey::SelectionSubmit"));

    let owner_routines =
        std::fs::read_to_string("src/runtime/branch/owner_audit/owner_routines.rs")
            .expect("read owner routine executor");
    assert!(owner_routines.contains("apply_forced_transition"));
    assert!(!owner_routines.contains("tick_run_active_with_observer"));
    assert!(owner_routines.contains("apply_reward_policy_step"));
    assert!(!owner_routines.contains("apply_reward_tiny_automation"));
    assert!(!owner_model.contains("AdvanceEmptyCampfire"));

    let reward_policy_step = std::fs::read_to_string("src/eval/run_control/reward_auto.rs")
        .expect("read reward policy step executor");
    assert!(reward_policy_step.contains("execute_reward_candidate_transaction"));
    assert!(!reward_policy_step.contains("tick_run_active_with_observer"));
    assert!(!reward_policy_step.contains("MAX_AUTO_REWARD_CLAIMS"));

    let forced_transition = std::fs::read_to_string("src/eval/run_control/forced_transition.rs")
        .expect("read forced transition contract");
    assert!(forced_transition.contains("RunForcedTransitionKindV1"));
    assert!(forced_transition.contains("before.candidates.is_empty()"));

    let candidate_model = std::fs::read_to_string("src/eval/run_control/view_model/mod.rs")
        .expect("read decision candidate model");
    for forbidden in [
        "CandidateAction::Command",
        "ManualCommand",
        "executable_command",
        "command_hint",
    ] {
        assert!(
            !candidate_model.contains(forbidden),
            "decision candidates must not expose the legacy command kernel through `{forbidden}`"
        );
    }

    let decision_surface = std::fs::read_to_string("src/eval/run_control/decision_surface.rs")
        .expect("read decision surface");
    for forbidden in [
        "command_hint",
        "inspectable_panels",
        "candidate_section_title",
    ] {
        assert!(
            !decision_surface.contains(forbidden),
            "machine decision surface must not carry retired REPL field `{forbidden}`"
        );
    }
}

#[test]
fn shop_execution_stays_single_step_and_boss_preview_bundles_stay_retired() {
    let retired_executor = "src/runtime/branch/owner_audit/shop_boss_preview_bundle_expansion.rs";
    assert!(
        !std::path::Path::new(retired_executor).exists(),
        "retired multi-purchase shop executor must stay deleted"
    );

    let mut sources = Vec::new();
    collect_rust_sources(std::path::Path::new("src"), &mut sources);
    for path in sources {
        let source = std::fs::read_to_string(&path).expect("read production Rust source");
        for forbidden in [
            "ShopBossPreviewBundle",
            "shop_boss_preview_bundles",
            "shop_boss_preview_bundle_limit",
            "shop_boss_preview_target_floor",
            "--shop-boss-preview-bundles",
            "--shop-boss-preview-target-floor",
            "expand_shop_boss_preview_bundle_children",
        ] {
            assert!(
                !source.contains(forbidden),
                "production source '{}' must not restore retired shop bundle contract `{forbidden}`",
                path.display()
            );
        }
    }

    let shop_owner = std::fs::read_to_string("src/runtime/branch/owner_audit/shop_tiny_owner.rs")
        .expect("read production shop owner");
    let production = shop_owner
        .split("#[cfg(test)]")
        .next()
        .unwrap_or(&shop_owner);
    assert!(production.contains("compiled_rollout_plan(&compiled)?.steps.first().cloned()"));
    assert!(production.contains("shop_plan_step_matches_choice"));
    assert!(
        !production.contains("for step in"),
        "shop owner may execute only the freshly compiled plan head, never a stored step sequence"
    );
}

#[test]
fn committed_combat_execution_is_atomic_and_separate_from_run_decisions() {
    let executor = std::fs::read_to_string("src/eval/run_control/combat_line_executor.rs")
        .expect("read combat line executor");
    assert!(executor.contains("RunCombatResolutionV1::new"));
    assert!(executor.contains("apply_combat_resolution_input"));
    assert!(executor.contains("let mut trial = session.clone()"));
    assert!(!executor.contains("session.apply_input("));

    let contract = std::fs::read_to_string("src/eval/run_control/combat_resolution.rs")
        .expect("read combat resolution contract");
    assert!(contract.contains("RunCombatResolutionKindV1"));
    assert!(contract.contains("after.decision_step != before.decision_step"));
    assert!(contract.contains("ActionResultChange::CombatEnded"));

    let progress = std::fs::read_to_string("src/eval/run_control/auto_step.rs")
        .expect("read atomic progress step projection");
    assert!(progress.contains("outcome.progress_steps.iter().cloned()"));
    assert!(progress.contains("with_progress_steps"));

    let lane_runner =
        std::fs::read_to_string("src/runtime/branch/owner_audit/combat_search_lane_runner.rs")
            .expect("read owner-audit combat lane runner");
    assert!(lane_runner.contains("trial.apply_combat_search(options.search)"));
    assert!(!lane_runner.contains("apply_progress_step"));
    assert!(!lane_runner.contains("apply_owner_audit_progress_step"));

    let owner_runner = std::fs::read_to_string("src/runtime/branch/owner_audit/runner.rs")
        .expect("read owner-audit bounded runner integration");
    assert!(owner_runner.contains("BoundedRunDriver::new"));
    assert!(owner_runner.contains(".drive_with(session"));
    assert!(owner_runner.contains("RunProgressJournalV1"));
    for retired_loop_owner in [
        "loop {",
        "auto_ops_used",
        "policy_steps",
        "should_continue_operation_budget_chunk",
    ] {
        assert!(
            !owner_runner.contains(retired_loop_owner),
            "owner-audit runner must not restore parallel repetition owner `{retired_loop_owner}`"
        );
    }

    let owner_orchestrator =
        std::fs::read_to_string("src/runtime/branch/owner_audit/owner_orchestrator.rs")
            .expect("read owner routine orchestrator");
    assert!(!owner_orchestrator.contains("OWNER_ROUTINE_STEP_LIMIT"));
    assert!(!owner_orchestrator.contains("owner routine step budget exhausted"));

    let portfolio_result =
        std::fs::read_to_string("src/runtime/branch/owner_audit/combat_search_portfolio_result.rs")
            .expect("read combat portfolio result");
    assert!(!portfolio_result.contains("should_continue_operation_budget_chunk"));
    assert!(!portfolio_result.contains("applied_operations"));
    let portfolio_output =
        std::fs::read_to_string("src/runtime/branch/owner_audit/combat_search_portfolio_output.rs")
            .expect("read combat portfolio output");
    assert!(!portfolio_output.contains("applied_operations"));

    let journal = std::fs::read_to_string("src/eval/run_control/progress_journal.rs")
        .expect("read typed progress journal");
    assert!(journal.contains("RUN_PROGRESS_JOURNAL_SCHEMA_VERSION"));
    assert!(journal.contains("run progress journal cannot contain stop records"));

    let driver = std::fs::read_to_string("src/eval/run_control/bounded_run_driver.rs")
        .expect("read bounded run driver");
    assert!(driver.contains("let applied_progress_steps = journal.len()"));
    assert!(!driver.contains("let mut applied_progress_steps"));

    for path in [
        "src/runtime/branch/owner_audit/runner.rs",
        "src/runtime/branch/owner_audit/branch_model.rs",
        "src/runtime/branch/owner_audit/owner_orchestrator.rs",
        "src/runtime/branch/owner_audit/combat_search_portfolio_output.rs",
        "src/runtime/branch/owner_audit/combat_search_portfolio_result.rs",
        "src/runtime/branch/owner_audit/render.rs",
        "src/runtime/branch/owner_audit/trace_format.rs",
        "src/runtime/branch/owner_audit/run_capsule_format.rs",
    ] {
        let source = std::fs::read_to_string(path).expect("read owner-audit journal consumer");
        assert!(
            !source.contains("auto_steps") && !source.contains("RunControlAutoAppliedStepV1"),
            "owner-audit progress must not be flattened back into legacy auto summaries in {path}"
        );
    }

    let trace = std::fs::read_to_string("src/runtime/branch/owner_audit/trace_format.rs")
        .expect("read owner-audit trace schema");
    assert!(trace.contains("branch_tiny_trace_v4"));
    assert!(trace.contains("trajectory_head"));
    let capsule = std::fs::read_to_string("src/runtime/branch/owner_audit/run_capsule_format.rs")
        .expect("read owner-audit capsule schema");
    assert!(capsule.contains("branch_tiny_run_result_v4"));
    assert!(capsule.contains("trajectory_head"));
    assert!(capsule.contains("trajectory_projection_index"));
}

#[test]
fn run_capsule_delegates_filesystem_writes_to_capsule_artifact_store() {
    let owner_audit = std::fs::read_to_string("src/runtime/branch/owner_audit.rs")
        .expect("read owner_audit runtime module");
    let run_capsule = std::fs::read_to_string("src/runtime/branch/owner_audit/run_capsule.rs")
        .expect("read run_capsule runtime module");

    assert!(
        owner_audit.contains("owner_audit/capsule_artifact_store.rs"),
        "owner_audit runtime should register the capsule artifact store adapter"
    );
    for forbidden in [
        "run_capsule_format",
        "run_capsule_io",
        "frontier_checkpoint",
        "combat_gap_case",
        "write_json",
        "remove_if_exists",
        "read_terminal_entries",
    ] {
        assert!(
            !run_capsule.contains(forbidden),
            "run_capsule should delegate `{forbidden}` details to capsule_artifact_store"
        );
    }
}

#[test]
fn run_persistence_only_handles_recovery_persistence() {
    let run_persistence =
        std::fs::read_to_string("src/runtime/branch/owner_audit/run_persistence.rs")
            .expect("read run_persistence runtime module");

    assert!(
        !run_persistence.contains("finalize_objective_result"),
        "objective completion should be owned by branch observation, not recovery persistence"
    );
    assert!(
        !run_persistence.contains("branch_status_view"),
        "run_persistence should not format branch status labels"
    );
}

#[test]
fn panel_scheduler_does_not_know_capsule_file_names() {
    let panel =
        std::fs::read_to_string("src/runtime/branch/panel.rs").expect("read panel scheduler");
    let panel = panel.split("#[cfg(test)]").next().unwrap_or(&panel);

    for forbidden in [
        "manifest.json",
        "frontier.json",
        "result.json",
        "terminal.json",
        "summary.json",
        "capsule_ledger.jsonl",
    ] {
        assert!(
            !panel.contains(forbidden),
            "panel scheduler should obtain `{forbidden}` facts from BranchArtifactStore"
        );
    }
}

#[test]
fn run_loop_delegates_slice_result_construction() {
    let run_loop = std::fs::read_to_string("src/runtime/branch/owner_audit/run_loop.rs")
        .expect("read run_loop");

    assert!(
        !run_loop.contains("RunSliceResult::new"),
        "run_loop should delegate RunSliceResult construction to run_slice_result helpers"
    );
}

#[test]
fn run_loop_delegates_capsule_result_persistence() {
    let run_loop = std::fs::read_to_string("src/runtime/branch/owner_audit/run_loop.rs")
        .expect("read run_loop");

    assert!(
        !run_loop.contains("capsule.save_result"),
        "run_loop should delegate capsule result persistence"
    );
    assert!(
        !run_loop.contains("run_capsule_result:"),
        "run_loop should not directly format capsule result output"
    );
}

#[test]
fn build_script_only_watches_consumed_inputs() {
    let build_script = std::fs::read_to_string("build.rs").expect("read root build script");

    for required in [
        "cargo:rerun-if-changed=build.rs",
        "cargo:rerun-if-changed=tools/compiled_protocol_schema.json",
    ] {
        assert!(
            build_script.contains(required),
            "build script must keep the consumed input watcher `{required}`"
        );
    }

    for obsolete in [
        "emit_git_rerun_watchers",
        "Command::new(\"git\")",
        "packed-refs",
        "refs/heads",
    ] {
        assert!(
            !build_script.contains(obsolete),
            "build script must not retain obsolete Git invalidation `{obsolete}`"
        );
    }
}

#[test]
fn windows_test_linking_uses_the_bundled_lld_without_machine_specific_paths() {
    let cargo_config =
        std::fs::read_to_string(".cargo/config.toml").expect("read repository Cargo config");

    assert!(
        cargo_config.contains("[target.x86_64-pc-windows-msvc]")
            && cargo_config.contains("linker = \"rust-lld\""),
        "Windows MSVC builds should use rustup's bundled LLD"
    );
    for forbidden in ["C:\\", "Users\\", "17239"] {
        assert!(
            !cargo_config.contains(forbidden),
            "Cargo linker configuration must not contain machine-specific path fragment '{forbidden}'"
        );
    }
}
#[test]
fn combat_line_adjudication_has_one_production_owner() {
    let selector = std::fs::read_to_string("src/eval/run_control/combat_line_selector.rs")
        .expect("read combat line selector");
    let lane_runner =
        std::fs::read_to_string("src/runtime/branch/owner_audit/combat_search_lane_runner.rs")
            .expect("read combat search lane runner");
    let owner_audit = std::fs::read_to_string("src/runtime/branch/owner_audit.rs")
        .expect("read owner audit module");
    let review_adapter = [
        "src/bin/combat_case_review/adjudication_probe.rs",
        "src/bin/combat_case_review/review_pipeline.rs",
        "src/bin/combat_case_review/search_types.rs",
    ]
    .into_iter()
    .map(|path| std::fs::read_to_string(path).expect("read combat review adapter"))
    .collect::<Vec<_>>()
    .join("\n");

    assert!(!selector.contains("CombatLineAcceptancePolicy::default()"));
    assert!(!lane_runner.contains("reject_dirty_win_status"));
    assert!(!lane_runner.contains("master_deck_curse_count"));
    assert!(!owner_audit.contains("combat_search_dirty_win.rs"));
    for forbidden in [
        "meta_changes",
        "CardType::Curse",
        "master_deck_curse_count",
        "WrithingMass",
        "Parasite",
        "planned_move_id",
        "run_combat_search_v2",
    ] {
        assert!(
            !review_adapter.contains(forbidden),
            "combat_case_review adapters must not own `{forbidden}` semantics"
        );
    }
}

#[test]
fn live_decision_layers_do_not_depend_on_offline_laboratories() {
    let mut sources = Vec::new();
    for root in [
        "src/eval/run_control",
        "src/runtime/branch/owner_audit",
        "src/ai/campfire_policy_v1",
        "src/ai/route_planner_v1",
        "src/ai/strategy/acquisition.rs",
    ] {
        collect_rust_sources(std::path::Path::new(root), &mut sources);
    }

    for path in sources {
        let source = std::fs::read_to_string(&path).expect("read live decision-layer source");
        for forbidden in [
            "combat_lab_v1",
            "campfire_survival_scenarios",
            "campfire_threat_panel",
        ] {
            assert!(
                !source.contains(forbidden),
                "live decision layer '{}' must not import or read offline laboratory `{forbidden}`",
                path.display()
            );
        }
    }
}

#[test]
fn public_scenario_policy_bank_does_not_depend_on_legacy_search_or_rollout() {
    let mut sources = Vec::new();
    for root in [
        "src/ai/combat_policy_v1/scenario",
        "src/eval/combat_lab_v1/policy_bank",
    ] {
        collect_rust_sources(std::path::Path::new(root), &mut sources);
    }

    for path in sources {
        let source = std::fs::read_to_string(&path).expect("read public scenario policy source");
        for forbidden in ["run_combat_search_v2", "CombatSearchV2", "rollout"] {
            assert!(
                !source.contains(forbidden),
                "public scenario policy source '{}' must not depend on legacy search detail '{forbidden}'",
                path.display()
            );
        }
    }
}

#[test]
fn turn_option_widening_schedule_sees_only_public_policy_state() {
    let source = std::fs::read_to_string("src/ai/combat_policy_v1/turn_option_schedule.rs")
        .expect("read public turn-option widening schedule");

    for forbidden in [
        "CombatScenarioGroupV1",
        "CombatScenarioParticleV1",
        "CombatScenarioStepResultV1",
        "CombatPosition",
        "ClientInput",
        "scenario_id",
        "bind_action",
        "exact_inputs",
        "step_combat_scenario_group_v1",
    ] {
        assert!(
            !source.contains(forbidden),
            "turn-option widening schedule must not depend on exact transition detail `{forbidden}`"
        );
    }
}

#[test]
fn turn_option_observable_effect_uses_only_public_candidate_evidence() {
    let source = std::fs::read_to_string("src/ai/combat_policy_v1/turn_option_effect.rs")
        .expect("read public turn-option observable effect");

    for forbidden in [
        "CombatScenarioGroupV1",
        "CombatScenarioParticleV1",
        "CombatScenarioStepResultV1",
        "CombatScenarioStepViewV1",
        "CombatPosition",
        "CombatStepResult",
        "ClientInput",
        "scenario_id",
        "bind_action",
        "exact_inputs",
        "step_combat_scenario_group_v1",
        "terminal_outcomes",
        "retained_step",
        "worlds",
        "public_history_id",
        "candidate.action",
        "engine_steps",
        "Deserialize",
        "crate::runtime",
        "crate::sim",
    ] {
        assert!(
            !source.contains(forbidden),
            "observable-effect evidence must not depend on unchecked input or exact transition detail `{forbidden}`"
        );
    }
}

#[test]
fn durable_upgrade_consumers_do_not_depend_on_rest_vs_smith() {
    for path in [
        "src/ai/random_upgrade_opportunity_v1.rs",
        "src/ai/shop_policy_v1/policy.rs",
    ] {
        let source = std::fs::read_to_string(path).expect("read durable upgrade consumer");
        assert!(
            !source.contains("rest_vs_smith.best_smith_debt_paid"),
            "durable upgrade consumer '{path}' must read the plan-level Smith debt fact"
        );
    }
}

#[test]
fn deck_mutation_compiler_does_not_depend_on_campfire_policy_configuration() {
    let source = std::fs::read_to_string("src/ai/deck_mutation_compiler_v1/compiler.rs")
        .expect("read deck mutation compiler");
    for forbidden in ["campfire_policy_v1", "clear_core_smith_priority_threshold"] {
        assert!(
            !source.contains(forbidden),
            "deck mutation compiler must not depend on Campfire policy detail '{forbidden}'"
        );
    }
}

#[test]
fn campfire_growth_facts_are_built_once_without_policy_scores() {
    let source = std::fs::read_to_string("src/eval/campfire_evaluation/growth.rs")
        .expect("read Campfire growth evaluator");
    for forbidden in [
        "upgrade_candidate_for_card_uuid_v1",
        "score_hint",
        "DeckMutationPlanRoleV1",
        "AllowedDeckMutationConsumersV1",
    ] {
        assert!(
            !source.contains(forbidden),
            "Campfire growth must not contain per-candidate replanning or policy detail '{forbidden}'"
        );
    }
    assert_eq!(
        source.matches("plan_upgrades_v1(").count(),
        1,
        "Campfire growth must build upgrade facts once"
    );
    assert_eq!(
        source.matches("deck_removal_target_snapshots_v1(").count(),
        1,
        "Campfire growth must build removal facts once"
    );

    let batch = std::fs::read_to_string("src/eval/campfire_evaluation.rs")
        .expect("read Campfire evaluation batch");
    let build = batch
        .find("let growth_facts = build_growth_facts(root);")
        .expect("growth facts must be constructed by the batch");
    let loop_start = batch
        .find("for candidate in legal_campfire_candidates(root)")
        .expect("Campfire batch must enumerate legal candidates");
    assert!(
        build < loop_start,
        "Campfire growth facts must be built before candidate iteration"
    );
}

#[test]
fn planner_core_is_clean_room_representation_not_a_strategy_owner() {
    let source = ["src/ai/planner_core/mod.rs", "src/ai/planner_core/types.rs"]
        .into_iter()
        .map(|path| std::fs::read_to_string(path).expect("read planner core source"))
        .collect::<Vec<_>>()
        .join("\n");

    for forbidden in [
        "crate::eval",
        "noncombat_strategy_v1",
        "campfire_policy_v1",
        "pressure",
        "prospect",
        "ValueEstimateV1",
    ] {
        assert!(
            !source.contains(forbidden),
            "planner core must not import incumbent strategy vocabulary `{forbidden}`"
        );
    }
}

#[test]
fn planner_capture_uses_candidate_enumeration_without_incumbent_explanations() {
    let source = std::fs::read_to_string("src/eval/run_control/planner_capture.rs")
        .expect("read planner capture adapter");
    let production = source.split("#[cfg(test)]").next().unwrap_or(&source);
    for forbidden in [
        "build_run_control_view_model",
        "noncombat_strategy_v1",
        "pressure",
        "prospect",
    ] {
        assert!(
            !production.contains(forbidden),
            "planner capture adapter must not depend on incumbent explanation `{forbidden}`"
        );
    }
}
