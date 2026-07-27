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

fn contains_rust_identifier(source: &str, identifier: &str) -> bool {
    source
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .any(|token| token == identifier)
}

#[test]
fn java_card_queue_has_one_executable_rust_owner() {
    let state = std::fs::read_to_string("src/runtime/combat/state.rs")
        .expect("read combat state ownership model");
    let methods = std::fs::read_to_string("src/runtime/combat/combat_methods.rs")
        .expect("read combat queue operations");
    let boundary = std::fs::read_to_string("src/ai/combat_policy_v1/scenario/boundary.rs")
        .expect("read stable-boundary predicate");
    let exact_runtime_key =
        std::fs::read_to_string("src/ai/combat_state_key/types/combat/runtime.rs")
            .expect("read exact runtime key");

    assert!(
        state.contains("pub queued_cards: VecDeque<QueuedCardPlay>"),
        "CardZones must own the executable equivalent of Java GameActionManager.cardQueue"
    );
    assert!(
        !state.contains("pub card_queue:"),
        "CombatRuntimeHints must not restore a second, non-executable card queue"
    );
    for (path, source) in [
        ("src/runtime/combat/combat_methods.rs", methods.as_str()),
        (
            "src/ai/combat_policy_v1/scenario/boundary.rs",
            boundary.as_str(),
        ),
    ] {
        assert!(
            !source.contains("runtime.card_queue"),
            "{path} must use the executable CardZones queue, not a ghost runtime hint"
        );
    }
    assert!(
        !exact_runtime_key.contains("card_queue")
            && !exact_runtime_key.contains("CombatLegacyEmptyCardQueueKey"),
        "exact identity V2 must not retain a historical empty card-queue placeholder"
    );
}

#[test]
fn durable_exact_identity_is_not_a_debug_or_layout_compatibility_view() {
    let identity = std::fs::read_to_string("src/ai/combat_state_key/identity.rs")
        .expect("read exact identity implementation");
    let queue = std::fs::read_to_string("src/ai/combat_state_key/combat/queue.rs")
        .expect("read exact action-queue projection");
    let cards = std::fs::read_to_string("src/ai/combat_state_key/types/combat/cards.rs")
        .expect("read exact card-zone key");
    let monster = std::fs::read_to_string("src/ai/combat_state_key/types/combat/monster.rs")
        .expect("read exact monster key");
    let player = std::fs::read_to_string("src/ai/combat_state_key/types/combat/player.rs")
        .expect("read exact player key");

    assert!(
        identity.contains("EXACT_IDENTITY_DOMAIN_V2")
            && identity.contains("serde_json::to_writer"),
        "durable exact identity must use a versioned streaming semantic encoding"
    );
    assert!(
        !identity.contains("format!(") && !identity.contains("std::hash"),
        "durable exact identity must not use Debug text or process-local Hash"
    );
    assert!(
        !queue.contains("discriminant(action)") && !queue.contains("format!(\"{action:?}\")"),
        "queued action identity must use its canonical serde payload"
    );
    assert!(
        !cards.contains("impl std::fmt::Debug for CombatZonesKey"),
        "packed zones must not re-create the historical owned-zone Debug shape"
    );
    assert!(
        !monster.contains("CombatMonsterTurnPlanView") && !monster.contains("turn_plan_view"),
        "monster identity must not hash a turn plan derived from move_state twice"
    );
    assert!(
        !player.contains("impl std::fmt::Debug for CombatRelicBusesKey"),
        "packed relic buses must not re-create 26 historical Debug fields"
    );
}

#[test]
fn oracle_lab_frontend_stays_split_into_bounded_command_modules() {
    const FRONTEND_LIMIT: u64 = 128 * 1024;
    const COMMAND_MODULE_LIMIT: u64 = 40 * 1024;

    let frontend = std::path::Path::new("crates/sts_oracle_lab/src/bin/oracle_lab.rs");
    let frontend_bytes = std::fs::metadata(frontend)
        .expect("read oracle_lab frontend metadata")
        .len();
    assert!(
        frontend_bytes <= FRONTEND_LIMIT,
        "oracle_lab.rs grew to {frontend_bytes} bytes; move cohesive command families into modules before extending the frontend"
    );

    for module in [
        "atomic_policy_searches.rs",
        "combat_case_atomic_turn_portfolio.rs",
        "combat_case_fold_solved_suffix.rs",
        "combat_case_layered.rs",
        "combat_case_layered_window_race.rs",
        "combat_case_legacy_global.rs",
        "combat_case_local_graph.rs",
        "combat_plan_diagnostics.rs",
        "depth_beam_audits.rs",
        "oracle_seed_panel.rs",
        "policy_discrepancy_search.rs",
        "turn_audits.rs",
        "turn_membership_audit.rs",
        "v2_capability_audit.rs",
    ] {
        let path = std::path::Path::new("crates/sts_oracle_lab/src/bin").join(module);
        let bytes = std::fs::metadata(&path)
            .unwrap_or_else(|error| panic!("read {} metadata: {error}", path.display()))
            .len();
        assert!(
            bytes <= COMMAND_MODULE_LIMIT,
            "{} grew to {bytes} bytes; split its independent responsibilities instead of creating another command monolith",
            path.display()
        );
    }

    for (module, limit) in [("canonical_launch.rs", 12 * 1024), ("workspace_view.rs", 12 * 1024)] {
        let path = std::path::Path::new("crates/sts_oracle_lab/src/bin").join(module);
        let bytes = std::fs::metadata(&path)
            .unwrap_or_else(|error| panic!("read {} metadata: {error}", path.display()))
            .len();
        assert!(
            bytes <= limit,
            "{} grew to {bytes} bytes; keep this laboratory boundary focused instead of moving unrelated host logic into it",
            path.display()
        );
    }
}

#[test]
fn oracle_lab_names_its_runtime_dependency_directly() {
    let manifest = std::fs::read_to_string("crates/sts_oracle_lab/Cargo.toml")
        .expect("read oracle laboratory manifest");
    assert!(
        manifest.contains("sts_oracle_runtime = { path = \"../sts_oracle_runtime\" }"),
        "oracle laboratory must name its runtime dependency directly"
    );
    assert!(
        !manifest.contains("sts_simulator = { package = \"sts_oracle_runtime\""),
        "oracle laboratory must not disguise sts_oracle_runtime behind the retired sts_simulator package name"
    );

    let mut sources = Vec::new();
    collect_rust_sources(
        std::path::Path::new("crates/sts_oracle_lab/src"),
        &mut sources,
    );
    for path in sources {
        let source = std::fs::read_to_string(&path).expect("read oracle laboratory source");
        assert!(
            !source.contains("sts_simulator::"),
            "oracle laboratory source '{}' must import sts_oracle_runtime directly",
            path.display()
        );
    }
}

#[test]
fn autonomous_run_loop_lives_in_runtime_not_the_thin_client() {
    let client = std::fs::read_to_string("crates/oracle_lab_client/src/main.rs")
        .expect("read thin oracle client");
    let runtime = std::fs::read_to_string("src/runtime/branch/oracle_autonomous_run.rs")
        .expect("read autonomous run owner");
    assert!(
        !client.contains("fn run_live_to_stop"),
        "the thin oracle client must not regain the autonomous run loop"
    );
    assert!(
        runtime.contains("pub fn run_oracle_analysis_to_stop_v1"),
        "the runtime must retain one reusable autonomous run owner"
    );
}

#[test]
fn local_turn_graph_keeps_distinct_responsibilities_in_bounded_modules() {
    const ROOT: &str = "crates/sts_combat_planner/src/local_turn_graph_search.rs";
    const MODULES: [(&str, u64); 6] = [
        (
            "crates/sts_combat_planner/src/local_turn_graph_search/diagnostics.rs",
            24 * 1024,
        ),
        (
            "crates/sts_combat_planner/src/local_turn_graph_search/scheduling.rs",
            32 * 1024,
        ),
        (
            "crates/sts_combat_planner/src/local_turn_graph_search/policy_line.rs",
            32 * 1024,
        ),
        (
            "crates/sts_combat_planner/src/local_turn_graph_search/reporting.rs",
            16 * 1024,
        ),
        (
            "crates/sts_combat_planner/src/local_turn_graph_search/tests.rs",
            16 * 1024,
        ),
        (
            "crates/sts_combat_planner/src/local_turn_graph_search/session.rs",
            16 * 1024,
        ),
    ];

    let source = std::fs::read_to_string(ROOT).expect("read local-turn graph search source");
    let root_bytes = source.len() as u64;
    assert!(
        root_bytes <= 64 * 1024,
        "local_turn_graph_search.rs grew to {root_bytes} bytes; keep the root focused on graph state and orchestration"
    );
    for module in [
        "diagnostics",
        "scheduling",
        "policy_line",
        "reporting",
        "session",
    ] {
        assert!(
            source.contains(&format!("mod {module};")),
            "the local-turn graph root must retain the {module} responsibility boundary"
        );
    }
    assert!(
        source.contains("#[cfg(test)]\nmod tests;"),
        "local-turn graph tests must remain in their dedicated child module"
    );
    assert!(
        !source.contains("pub struct LocalTurnGraphWitnessReport"),
        "public reports belong in reporting.rs, not the orchestration root"
    );
    assert!(
        !source.contains("pub fn offer_plan_compatible_policy_line_with_suffix_probes"),
        "policy-line materialization belongs in policy_line.rs, not the orchestration root"
    );
    assert!(
        !source.contains("pub fn progress_snapshot"),
        "read-only graph inspection belongs in diagnostics.rs, not the orchestration root"
    );
    assert!(
        !source.contains("pub fn with_policy_and_lookahead"),
        "session construction and witness ingress belong in session.rs, not the orchestration root"
    );
    assert!(
        !source.contains("#[test]"),
        "inline tests must not regrow inside the orchestration root"
    );

    for (path, limit) in MODULES {
        let bytes = std::fs::metadata(path)
            .unwrap_or_else(|error| panic!("read {path} metadata: {error}"))
            .len();
        assert!(
            bytes <= limit,
            "{path} grew to {bytes} bytes (limit {limit}); split its independent responsibilities before extending it"
        );
    }
}

#[test]
fn resident_oracle_state_stays_outside_cargo_target() {
    let client = std::fs::read_to_string("crates/oracle_lab_client/src/main.rs")
        .expect("read oracle_lab client source");
    let service =
        std::fs::read_to_string("crates/sts_oracle_lab/src/bin/oracle_lab_service.rs")
            .expect("read oracle_lab service source");

    for (label, source) in [("client", client.as_str()), ("service", service.as_str())] {
        assert!(
            !source.contains(r#".join("target").join("oracle-lab")"#),
            "{label} must not place resident state below Cargo target"
        );
        assert!(
            source.contains(r#".join(".oracle-lab")"#),
            "{label} must resolve resident state through the ignored .oracle-lab root"
        );
    }
}

#[test]
fn generated_oracle_cases_stay_outside_cargo_target() {
    let mut sources = Vec::new();
    collect_rust_sources(std::path::Path::new("src"), &mut sources);
    collect_rust_sources(std::path::Path::new("crates"), &mut sources);

    for path in sources {
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        assert!(
            !source.contains("target/oracle-cases")
                && !source.contains(r"target\oracle-cases"),
            "{} must not make Cargo's target directory the durable home for generated oracle cases; use the ignored .oracle-lab root",
            path.display()
        );
    }
}

#[test]
fn exact_combat_contract_stays_outside_the_run_explorer_compilation_unit() {
    let contract_manifest =
        std::fs::read_to_string("crates/sts_combat_contract/Cargo.toml")
            .expect("read combat contract manifest");
    let knowledge_manifest =
        std::fs::read_to_string("crates/sts_combat_knowledge/Cargo.toml")
            .expect("read combat knowledge manifest");
    let runtime_policy =
        std::fs::read_to_string("src/eval/run_control/oracle_combat_policy.rs")
            .expect("read runtime combat policy adapter");

    for (label, manifest) in [
        ("combat contract", contract_manifest.as_str()),
        ("combat knowledge", knowledge_manifest.as_str()),
    ] {
        assert!(
            !manifest.contains("sts_oracle_runtime"),
            "{label} must not pull routes, shops, continuations, or the run explorer into exact-combat iteration"
        );
    }
    assert!(
        runtime_policy.contains("pub use sts_combat_knowledge"),
        "run control and the lightweight contract must share one tactical-knowledge implementation"
    );
    assert!(
        !runtime_policy.contains("impl CombatActionPolicy for ExistingCombatKnowledgePolicy"),
        "run control must not grow a second copy of the shared tactical policy"
    );
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
fn retired_eval_bucket_frontier_does_not_return() {
    let mut sources = Vec::new();
    collect_rust_sources(std::path::Path::new("src"), &mut sources);

    for path in sources {
        let source = std::fs::read_to_string(&path).expect("read Rust source");
        for forbidden in [
            "RoundRobinEvalBuckets",
            "FrontierLanes",
            "CombatSearchV2FrontierPolicy",
            "CombatSearchFrontierPluginId",
            "compare_frontier",
            "frontier_policy",
        ] {
            assert!(
                !source.contains(forbidden),
                "source '{}' must not restore retired eval-bucket scheduling surface `{forbidden}`",
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
fn exact_combat_planner_core_does_not_import_legacy_policy_owners() {
    let mut sources = Vec::new();
    collect_rust_sources(
        std::path::Path::new("crates/sts_combat_planner/src"),
        &mut sources,
    );

    for path in sources {
        let source = std::fs::read_to_string(&path).expect("read exact combat planner source");
        for forbidden in [
            "combat_search_v2",
            "SearchNode",
            "TurnPlanBucket",
            "CombatEvalV2",
            "run_control",
        ] {
            assert!(
                !contains_rust_identifier(&source, forbidden),
                "new combat planner source '{}' must not import legacy policy owner `{forbidden}`",
                path.display()
            );
        }
    }

    let manifest = std::fs::read_to_string("crates/sts_combat_planner/Cargo.toml")
        .expect("read exact combat planner manifest");
    assert!(
        !manifest.contains("sts_combat_legacy"),
        "new combat planner must not depend on the legacy combat package"
    );
}

#[test]
fn typed_combat_strategy_facts_do_not_depend_on_or_leak_into_search_control() {
    let strategy_manifest = std::fs::read_to_string("crates/sts_combat_strategy/Cargo.toml")
        .expect("read typed combat strategy manifest");
    for forbidden_dependency in [
        "sts_combat_planner",
        "sts_combat_legacy",
        "sts_oracle_runtime",
        "sts_oracle_tools",
    ] {
        assert!(
            !strategy_manifest.contains(forbidden_dependency),
            "typed combat strategy facts must not depend on search/control crate `{forbidden_dependency}`"
        );
    }

    let mut strategy_sources = Vec::new();
    collect_rust_sources(
        std::path::Path::new("crates/sts_combat_strategy/src"),
        &mut strategy_sources,
    );
    for path in strategy_sources {
        let source = std::fs::read_to_string(&path).expect("read typed combat strategy source");
        for forbidden_owner in [
            "CombatActionPolicy",
            "CombatStateGuideRank",
            "LocalTurnGraphWitnessSession",
            "OracleCombatWitnessSatisfaction",
            "RunControlSession",
        ] {
            assert!(
                !contains_rust_identifier(&source, forbidden_owner),
                "typed combat strategy source '{}' must not become search/control owner `{forbidden_owner}`",
                path.display()
            );
        }
    }

    let mut planner_sources = Vec::new();
    collect_rust_sources(
        std::path::Path::new("crates/sts_combat_planner/src"),
        &mut planner_sources,
    );
    for path in planner_sources {
        if path.file_name().is_some_and(|name| name == "tests.rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read exact combat planner source");
        let production = source.split("#[cfg(test)]").next().unwrap_or(&source);
        for encounter_detail in [
            "AwakenedOne",
            "AwakenedOnePhaseControl",
            "CombatPlanStageV1",
            "CombatPlanMilestoneV1",
            "CombatPlanObligationV1",
            "Curiosity",
        ] {
            assert!(
                !contains_rust_identifier(production, encounter_detail),
                "exact combat planner source '{}' must carry generic annotations without interpreting encounter detail `{encounter_detail}`",
                path.display()
            );
        }
    }
}

#[test]
fn oracle_tools_are_a_library_free_command_host_over_the_runtime() {
    let manifest = std::fs::read_to_string("crates/sts_oracle_tools/Cargo.toml")
        .expect("read oracle tools manifest");
    assert!(
        manifest.contains("name = \"sts_oracle_tools\"")
            && manifest.contains("sts_oracle_runtime ="),
        "oracle tools must identify the command host and depend directly on the runtime"
    );
    for forbidden in [
        "[lib]",
        "build =",
        "sts_core =",
        "sts_combat_legacy =",
        "sts_combat_planner =",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "oracle tools must not restore command-host ownership through `{forbidden}`"
        );
    }
    for retired_facade in [
        "crates/sts_oracle_tools/src/lib.rs",
        "crates/sts_oracle_tools/src/ai.rs",
        "crates/sts_oracle_tools/src/runtime.rs",
        "crates/sts_oracle_tools/build.rs",
    ] {
        assert!(
            !std::path::Path::new(retired_facade).exists(),
            "retired oracle-tools facade file '{retired_facade}' must not return"
        );
    }

    let mut command_sources = Vec::new();
    collect_rust_sources(std::path::Path::new("src/bin"), &mut command_sources);
    for path in command_sources {
        let source = std::fs::read_to_string(&path).expect("read oracle command source");
        assert!(
            !source.contains("sts_simulator::"),
            "oracle command source '{}' must name sts_oracle_runtime directly",
            path.display()
        );
    }
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

    let retired_card_reward_executor = "src/eval/run_control/card_reward_auto.rs";
    assert!(
        !std::path::Path::new(retired_card_reward_executor).exists(),
        "retired card-reward auto-policy stack must stay deleted"
    );
    let card_reward_policy =
        std::fs::read_to_string("src/eval/run_control/card_reward_policy_prior.rs")
            .expect("read exact card reward policy");
    let card_reward_policy = card_reward_policy
        .split("#[cfg(test)]")
        .next()
        .unwrap_or(&card_reward_policy);
    assert!(card_reward_policy.contains("DecisionCandidateKey::CardRewardSingingBowl"));
    assert!(card_reward_policy.contains("positive_ranked_run_policy_prior_v1"));
    for source in [executor.as_str(), card_reward_policy] {
        assert!(!source.contains("apply_singing_bowl_to_visible_card_reward_item"));
    }

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

    let retired_route_executor = "src/eval/run_control/route_policy/apply.rs";
    assert!(
        !std::path::Path::new(retired_route_executor).exists(),
        "retired route executor must stay deleted"
    );
    let route_executor = std::fs::read_to_string("src/eval/run_control/route_policy/mod.rs")
        .expect("read route policy executor");
    assert!(route_executor.contains("exact_route_policy_decision_v1(session, &legal)?"));
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
    assert!(production.contains("let mut choices = executable_choices(surface);"));
    assert!(production.contains("exact_shop_policy_prior_v1(session, &legal)?"));
    assert!(production.contains("choice.expansion = OwnerChoiceExpansion::AutoAllowed;"));
    for forbidden_execution in [
        "compiled_rollout_plan",
        "shop_plan_step_matches_choice",
        "apply_decision_action(",
        "apply_input(",
        "for step in",
    ] {
        assert!(
            !production.contains(forbidden_execution),
            "shop owner must rank the current exact surface without executing `{forbidden_execution}`"
        );
    }

    let shop_policy = std::fs::read_to_string("src/eval/run_control/shop_policy_prior.rs")
        .expect("read exact shop policy");
    let shop_policy = shop_policy
        .split("#[cfg(test)]")
        .next()
        .unwrap_or(&shop_policy);
    assert!(shop_policy.contains("validate_same_candidate_surface(&exact, legal)?"));
    assert!(shop_policy.contains("exact.actions.len() != legal.len()"));
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

    let search_orchestrator =
        std::fs::read_to_string("src/runtime/branch/owner_audit/combat_search_orchestrator.rs")
            .expect("read owner-audit combat search orchestrator");
    assert_eq!(
        search_orchestrator.matches(".apply_combat_search(").count(),
        1,
        "owner audit must invoke exactly one canonical combat search session"
    );
    assert!(!search_orchestrator.contains("run_lane_attempt"));
    assert!(!search_orchestrator.contains("arbitrate_post_primary"));
    assert!(!search_orchestrator.contains("let root = session.clone()"));
    for retired_lane_owner in [
        "src/runtime/branch/owner_audit/combat_search_lane_runner.rs",
        "src/runtime/branch/owner_audit/combat_search_lane_options.rs",
        "src/runtime/branch/owner_audit/combat_search_lanes.rs",
        "src/runtime/branch/owner_audit/combat_search_portfolio_plan.rs",
        "src/eval/run_control/combat_line_repair.rs",
    ] {
        assert!(
            !std::path::Path::new(retired_lane_owner).exists(),
            "retired multi-root combat search owner must stay deleted: {retired_lane_owner}"
        );
    }

    let line_selector =
        std::fs::read_to_string("src/eval/run_control/combat_line_selector.rs")
            .expect("read combat line selector");
    for forbidden_search_owner in [
        "combat_line_repair",
        "combat_search_v2_with_stepper",
        "CombatSearchV2Session::new",
    ] {
        assert!(
            !line_selector.contains(forbidden_search_owner),
            "candidate selection must not restore hidden combat search owner `{forbidden_search_owner}`"
        );
    }

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

    let search_result =
        std::fs::read_to_string("src/runtime/branch/owner_audit/combat_search_session_result.rs")
            .expect("read combat search session result");
    assert!(!search_result.contains("should_continue_operation_budget_chunk"));
    assert!(!search_result.contains("applied_operations"));
    let search_output =
        std::fs::read_to_string("src/runtime/branch/owner_audit/combat_search_session_output.rs")
            .expect("read combat search session output");
    assert!(!search_output.contains("applied_operations"));

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
        "src/runtime/branch/owner_audit/combat_search_session_output.rs",
        "src/runtime/branch/owner_audit/combat_search_session_result.rs",
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
    let orchestrator =
        std::fs::read_to_string("src/runtime/branch/owner_audit/combat_search_orchestrator.rs")
            .expect("read combat search orchestrator");
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
    assert!(!selector.contains("find_clean_no_potion_alternative"));
    assert!(!selector.contains("run_combat_search_v2"));
    assert!(!orchestrator.contains("reject_dirty_win_status"));
    assert!(!orchestrator.contains("master_deck_curse_count"));
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
fn durable_upgrade_consumers_use_plan_level_or_exact_state_facts() {
    for path in [
        "src/ai/random_upgrade_opportunity_v1.rs",
        "src/eval/run_control/shop_policy_prior.rs",
    ] {
        let source = std::fs::read_to_string(path).expect("read durable upgrade consumer");
        assert!(
            !source.contains("rest_vs_smith.best_smith_debt_paid"),
            "durable upgrade consumer '{path}' must read the plan-level Smith debt fact"
        );
    }

    let random_upgrade =
        std::fs::read_to_string("src/ai/random_upgrade_opportunity_v1.rs")
            .expect("read random upgrade evaluator");
    assert!(random_upgrade.contains("upgrade_plan.best_smith_debt_paid"));

    let shop_policy = std::fs::read_to_string("src/eval/run_control/shop_policy_prior.rs")
        .expect("read exact shop policy");
    assert!(shop_policy.contains("combat_upgrade_coverage.strongest_scope()"));
    assert!(
        !std::path::Path::new("src/ai/shop_policy_v1/policy.rs").exists(),
        "retired score-based shop policy must stay deleted"
    );
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
