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
fn combat_line_adjudication_has_one_production_owner() {
    let selector = std::fs::read_to_string("src/eval/run_control/combat_line_selector.rs")
        .expect("read combat line selector");
    let lane_runner =
        std::fs::read_to_string("src/runtime/branch/owner_audit/combat_search_lane_runner.rs")
            .expect("read combat search lane runner");
    let owner_audit = std::fs::read_to_string("src/runtime/branch/owner_audit.rs")
        .expect("read owner audit module");

    assert!(!selector.contains("CombatLineAcceptancePolicy::default()"));
    assert!(!lane_runner.contains("reject_dirty_win_status"));
    assert!(!lane_runner.contains("master_deck_curse_count"));
    assert!(!owner_audit.contains("combat_search_dirty_win.rs"));
}
