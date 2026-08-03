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
