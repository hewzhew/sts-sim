use std::path::PathBuf;

use clap::Parser;

use sts_simulator::bot::combat::{audit_state, render_text_report, DecisionAuditConfig};
use sts_simulator::fixtures::author_spec::{compile_combat_author_spec, CombatAuthorSpec};
use sts_simulator::fixtures::scenario::initialize_fixture_state;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    author_spec: PathBuf,
    #[arg(long)]
    json_out: Option<PathBuf>,
    #[arg(long, default_value_t = 4)]
    decision_depth: usize,
    #[arg(long, default_value_t = 3)]
    top_k: usize,
    #[arg(long, default_value_t = 6)]
    branch_cap: usize,
    #[arg(long)]
    quiet: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let spec_payload = std::fs::read_to_string(&args.author_spec)?;
    let spec: CombatAuthorSpec = serde_json::from_str(&spec_payload)?;
    let fixture = compile_combat_author_spec(&spec)?;
    let initial = initialize_fixture_state(&fixture);
    let report = audit_state(
        fixture.name.clone(),
        Some(args.author_spec.display().to_string()),
        initial.frame_id,
        initial.response_id,
        initial.engine_state,
        initial.combat,
        None,
        DecisionAuditConfig {
            decision_depth: args.decision_depth,
            top_k: args.top_k,
            branch_cap: args.branch_cap,
        },
    )?;

    if let Some(path) = args.json_out {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let payload = serde_json::to_string_pretty(&report)?;
        std::fs::write(&path, payload)?;
    }

    if !args.quiet {
        println!("{}", render_text_report(&report));
    }

    Ok(())
}
