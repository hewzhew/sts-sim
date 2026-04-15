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
        "pub mod action;".to_string(),
        "pub mod bot;".to_string(),
        "pub mod cli;".to_string(),
        "pub mod combat;".to_string(),
        "pub mod content;".to_string(),
        "pub mod diff;".to_string(),
        "pub mod engine;".to_string(),
        "pub mod interaction_coverage;".to_string(),
        "pub mod map;".to_string(),
        "pub mod rng;".to_string(),
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
        "pub mod agent;".to_string(),
        "pub mod combat_heuristic;".to_string(),
        "pub mod coverage;".to_string(),
        "pub mod deck_delta_eval;".to_string(),
        "pub mod evaluator;".to_string(),
        "pub mod event_policy;".to_string(),
        "pub mod harness;".to_string(),
        "pub mod reward_heuristics;".to_string(),
        "pub mod search;".to_string(),
    ]);
    let expected_uses = BTreeSet::from([
        "pub use strategy_families::{branch_family_for_card, BranchFamily};".to_string(),
    ]);

    assert_eq!(public_mods, expected_mods, "unexpected bot pub mod surface");
    assert_eq!(public_uses, expected_uses, "unexpected bot pub use surface");
}
