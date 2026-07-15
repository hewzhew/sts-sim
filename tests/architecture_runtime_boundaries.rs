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
        ] {
            assert!(
                !source.contains(forbidden),
                "owner-audit source '{}' must execute typed RunDecisionAction values, not `{forbidden}`",
                path.display()
            );
        }
    }

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
