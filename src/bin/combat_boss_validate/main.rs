use std::io::Write;
use std::path::PathBuf;

use clap::Parser;

use sts_simulator::bot::search::DecisionAuditConfig;
use sts_simulator::bot::harness::boss_validation::{build_ledger_record, validate_case};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    case: PathBuf,
    #[arg(long, default_value_t = 4)]
    decision_depth: usize,
    #[arg(long, default_value_t = 3)]
    top_k: usize,
    #[arg(long, default_value_t = 6)]
    branch_cap: usize,
    #[arg(long)]
    json_out: Option<PathBuf>,
    #[arg(long)]
    jsonl_out: Option<PathBuf>,
    #[arg(long)]
    quiet: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let result = validate_case(
        &args.case,
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
        std::fs::write(&path, serde_json::to_string_pretty(&result)?)?;
    }

    if let Some(path) = args.jsonl_out {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let record = build_ledger_record(&args.case, &result)?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        serde_json::to_writer(&mut file, &record)?;
        file.write_all(b"\n")?;
    }

    if !args.quiet {
        println!("case: {}", result.case_name);
        println!("expected: {:?}", result.expected_verdict);
        println!("actual:   {:?}", result.actual_verdict);
        if !result.rationale_tags.is_empty() {
            println!("rationale: {:?}", result.rationale_tags);
        }
        for candidate in &result.candidates {
            println!(
                "- {} outcome={:?} rank={} score={} tags={:?}",
                candidate.name,
                candidate.outcome,
                candidate.outcome_rank,
                candidate.score,
                candidate.rationale_tags
            );
            if !candidate.top_actions.is_empty() {
                println!("  top_actions={}", candidate.top_actions.join(" -> "));
            }
        }
    }

    Ok(())
}
