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
