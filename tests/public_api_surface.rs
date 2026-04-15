use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn manifest_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn source_lines(rel_path: &str) -> Vec<String> {
    fs::read_to_string(manifest_root().join(rel_path))
        .unwrap_or_else(|err| panic!("read_to_string failed for {rel_path}: {err}"))
        .lines()
        .map(|line| line.trim().to_string())
        .collect()
}

fn collect_prefixed_lines(rel_path: &str, prefix: &str) -> BTreeSet<String> {
    source_lines(rel_path)
        .into_iter()
        .filter(|line| line.starts_with(prefix))
        .collect()
}

#[test]
fn lib_root_public_surface_matches_expected_whitelist() {
    let public_mods = collect_prefixed_lines("src/lib.rs", "pub mod ");
    let public_uses = collect_prefixed_lines("src/lib.rs", "pub use ");

    let expected_mods = BTreeSet::from([
        "pub mod bot;".to_string(),
        "pub mod cli;".to_string(),
        "pub mod content;".to_string(),
        "pub mod diff;".to_string(),
        "pub mod engine;".to_string(),
        "pub mod map;".to_string(),
        "pub mod runtime;".to_string(),
        "pub mod state;".to_string(),
    ]);
    let expected_uses = BTreeSet::from([
        "pub use core::EntityId;".to_string(),
        "pub use testing::fixtures;".to_string(),
        "pub use utils::SimulationWatchdog;".to_string(),
    ]);

    assert_eq!(public_mods, expected_mods, "unexpected root pub mod surface");
    assert_eq!(public_uses, expected_uses, "unexpected root pub use surface");
}

#[test]
fn bot_public_surface_matches_expected_whitelist() {
    let public_mods = collect_prefixed_lines("src/bot/mod.rs", "pub mod ");
    let public_uses = collect_prefixed_lines("src/bot/mod.rs", "pub use ");

    let expected_mods = BTreeSet::from([
        "pub mod harness;".to_string(),
        "pub mod search;".to_string(),
    ]);
    let expected_uses = BTreeSet::from([
        "pub use agent::Agent;".to_string(),
        "pub use combat_heuristic::{".to_string(),
        "pub use coverage::{".to_string(),
        "pub use deck_delta_eval::{compare_pick_vs_skip, DeltaScore};".to_string(),
        "pub use evaluator::{evaluate_state, CardEvaluator, DeckProfile};".to_string(),
        "pub use event_policy::{".to_string(),
        "pub use reward_heuristics::{".to_string(),
        "pub use strategy_families::{branch_family_for_card, BranchFamily};".to_string(),
    ]);

    assert_eq!(public_mods, expected_mods, "unexpected bot pub mod surface");
    assert_eq!(public_uses, expected_uses, "unexpected bot pub use surface");
}

#[test]
fn runtime_public_surface_matches_expected_whitelist() {
    let public_mods = collect_prefixed_lines("src/runtime/mod.rs", "pub mod ");
    let expected_mods = BTreeSet::from([
        "pub mod action;".to_string(),
        "pub mod combat;".to_string(),
        "pub mod rng;".to_string(),
    ]);

    assert_eq!(public_mods, expected_mods, "unexpected runtime pub mod surface");
}

#[test]
fn cli_public_surface_matches_expected_whitelist() {
    let public_mods = collect_prefixed_lines("src/cli/mod.rs", "pub mod ");
    let expected_mods = BTreeSet::from([
        "pub mod coverage_tools;".to_string(),
        "pub mod live_comm;".to_string(),
        "pub mod live_comm_admin;".to_string(),
    ]);

    assert_eq!(public_mods, expected_mods, "unexpected cli pub mod surface");
}

#[test]
fn cli_coverage_tools_public_surface_matches_expected_whitelist() {
    let header = source_lines("src/cli/coverage_tools/mod.rs");
    let expected_header = vec![
        "mod io;".to_string(),
        "mod report;".to_string(),
        "".to_string(),
        "pub use io::{default_replay_inputs, load_live_comm_records, replay_records_from_path};"
            .to_string(),
        "pub use report::{write_coverage_outputs, InteractionCoverageReport};".to_string(),
    ];
    assert_eq!(
        header, expected_header,
        "unexpected cli::coverage_tools module surface"
    );
}

#[test]
fn bot_coverage_signatures_surface_matches_expected_whitelist() {
    let header = source_lines("src/bot/coverage_signatures.rs");
    let expected_prefix = vec![
        "use std::collections::BTreeSet;".to_string(),
        "".to_string(),
        "use serde::{Deserialize, Serialize};".to_string(),
    ];
    assert_eq!(
        header[..3].to_vec(),
        expected_prefix,
        "unexpected bot::coverage_signatures module prefix"
    );

    let required_exports = [
        "pub struct InteractionSignature {",
        "pub struct ObservedInteractionRecord {",
        "pub fn signature_from_transition_with_archetypes(",
        "pub fn command_string(input: &ClientInput) -> String {",
    ];
    for export in required_exports {
        assert!(
            header.iter().any(|line| line == export),
            "missing bot::coverage_signatures export: {export}"
        );
    }
}

#[test]
fn bot_harness_public_surface_matches_expected_whitelist() {
    let public_mods = collect_prefixed_lines("src/bot/harness/mod.rs", "pub mod ");
    let public_uses = collect_prefixed_lines("src/bot/harness/mod.rs", "pub use ");

    let expected_mods = BTreeSet::from([
        "pub mod combat_env;".to_string(),
        "pub mod combat_lab;".to_string(),
    ]);
    let expected_uses = BTreeSet::from([
        "pub use boss_validation::{build_ledger_record, validate_case};".to_string(),
        "pub use combat_policy::PolicyKind;".to_string(),
    ]);

    assert_eq!(
        public_mods, expected_mods,
        "unexpected bot::harness pub mod surface"
    );
    assert_eq!(
        public_uses, expected_uses,
        "unexpected bot::harness pub use surface"
    );
}
