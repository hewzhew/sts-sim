use std::path::PathBuf;

use clap::Parser;

use super::{Cli, Command, OracleDriveBoundaryArg};

#[test]
fn drive_parses_typed_stop_boundary() {
    let cli = Cli::try_parse_from([
        "oracle_lab",
        "drive",
        "--workspace",
        "case.workspace.json",
        "--stop-at",
        "map-decision",
    ])
    .expect("typed drive boundary should parse");
    let Command::Drive { stop_at, .. } = cli.command else {
        panic!("expected drive command");
    };

    assert_eq!(stop_at, Some(OracleDriveBoundaryArg::MapDecision));
}

#[test]
fn drive_parses_optional_full_ledger_output() {
    let cli = Cli::try_parse_from([
        "oracle_lab",
        "drive",
        "--workspace",
        "case.workspace.json",
        "--output",
        "drive.json",
    ])
    .expect("drive ledger output should parse");
    let Command::Drive { output, .. } = cli.command else {
        panic!("expected drive command");
    };

    assert_eq!(output, Some(PathBuf::from("drive.json")));
}

#[test]
fn combat_case_parses_repeated_exact_potion_slots() {
    Cli::try_parse_from([
        "oracle_lab",
        "combat-case",
        "--case",
        "fight.case.json",
        "--max-potions-used",
        "2",
        "--potion-slot",
        "0",
        "--potion-slot",
        "2",
    ])
    .expect("a bounded potion combination should parse");
}

#[test]
fn compact_workspace_parses_source_node_and_fresh_output() {
    let cli = Cli::try_parse_from([
        "oracle_lab",
        "compact-workspace",
        "--workspace",
        "source.workspace.json",
        "--node",
        "42",
        "--output",
        "active.workspace.json",
    ])
    .expect("compact workspace command should parse");
    let Command::CompactWorkspace {
        workspace,
        node,
        output,
    } = cli.command
    else {
        panic!("expected compact workspace command");
    };

    assert_eq!(workspace, PathBuf::from("source.workspace.json"));
    assert_eq!(node, Some(42));
    assert_eq!(output, PathBuf::from("active.workspace.json"));
}

#[test]
fn repack_workspace_parses_source_and_fresh_output() {
    let cli = Cli::try_parse_from([
        "oracle_lab",
        "repack-workspace",
        "--workspace",
        "historical.workspace.json",
        "--output",
        "pooled.workspace.json",
    ])
    .expect("repack workspace command should parse");
    let Command::RepackWorkspace { workspace, output } = cli.command else {
        panic!("expected repack workspace command");
    };

    assert_eq!(workspace, PathBuf::from("historical.workspace.json"));
    assert_eq!(output, PathBuf::from("pooled.workspace.json"));
}

#[test]
fn route_policy_audit_parses_an_optional_exact_node() {
    let cli = Cli::try_parse_from([
        "oracle_lab",
        "route-policy-audit",
        "--workspace",
        "run.workspace.json",
        "--node",
        "186",
    ])
    .expect("route policy audit should parse");
    let Command::RoutePolicyAudit(args) = cli.command else {
        panic!("expected route policy audit command");
    };

    assert_eq!(args.workspace, PathBuf::from("run.workspace.json"));
    assert_eq!(args.node, Some(186));
}

#[test]
fn shop_policy_audit_parses_an_optional_exact_node() {
    let cli = Cli::try_parse_from([
        "oracle_lab",
        "shop-policy-audit",
        "--workspace",
        "run.workspace.json",
        "--node",
        "27",
    ])
    .expect("shop policy audit should parse");
    let Command::ShopPolicyAudit(args) = cli.command else {
        panic!("expected shop policy audit command");
    };

    assert_eq!(args.workspace, PathBuf::from("run.workspace.json"));
    assert_eq!(args.node, Some(27));
}

#[test]
fn witness_verification_defaults_to_cursor_and_accepts_an_exact_node() {
    let omitted = Cli::try_parse_from([
        "oracle_lab",
        "verify-run-witness",
        "--workspace",
        "run.workspace.json",
    ])
    .expect("cursor witness verification should parse");
    let Command::VerifyRunWitness { node, .. } = omitted.command else {
        panic!("expected witness verification command");
    };
    assert_eq!(node, None);

    let explicit = Cli::try_parse_from([
        "oracle_lab",
        "verify-run-witness",
        "--workspace",
        "run.workspace.json",
        "--node",
        "35",
    ])
    .expect("exact-node witness verification should parse");
    let Command::VerifyRunWitness { node, .. } = explicit.command else {
        panic!("expected witness verification command");
    };
    assert_eq!(node, Some(35));
}
