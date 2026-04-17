use std::path::{Path, PathBuf};

use clap::Parser;

use sts_simulator::bot::combat::DecisionAuditConfig;
use sts_simulator::bot::harness::{build_ledger_record, validate_case};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, num_args = 1.., required = true)]
    pack_dir: Vec<PathBuf>,
    #[arg(long)]
    jsonl_out: PathBuf,
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
    if let Some(parent) = args.jsonl_out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut writer = std::io::BufWriter::new(std::fs::File::create(&args.jsonl_out)?);
    let config = DecisionAuditConfig {
        decision_depth: args.decision_depth,
        top_k: args.top_k,
        branch_cap: args.branch_cap,
    };

    let mut total = 0usize;
    let mut passed = 0usize;
    for pack_dir in &args.pack_dir {
        for case_path in list_case_paths(pack_dir)? {
            let result = validate_case(&case_path, config)?;
            let record = build_ledger_record(&case_path, &result)?;
            serde_json::to_writer(&mut writer, &record)?;
            use std::io::Write;
            writer.write_all(b"\n")?;
            total += 1;
            if record.pass {
                passed += 1;
            }
            if !args.quiet {
                println!(
                    "{} {} expected={:?} actual={:?} pass={}",
                    pack_dir.display(),
                    result.case_name,
                    result.expected_verdict,
                    result.actual_verdict,
                    record.pass
                );
            }
        }
    }
    if !args.quiet {
        println!("wrote {} records to {}", total, args.jsonl_out.display());
        println!("pass_rate={}/{}", passed, total);
    }
    Ok(())
}

fn list_case_paths(pack_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut cases = std::fs::read_dir(pack_dir)
        .map_err(|err| format!("failed to read pack dir {}: {err}", pack_dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("state_case_") && name.ends_with(".json"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    cases.sort();
    if cases.is_empty() {
        return Err(format!(
            "pack dir {} contains no state_case_*.json files",
            pack_dir.display()
        ));
    }
    Ok(cases)
}
