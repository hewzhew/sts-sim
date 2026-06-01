use std::path::{Path, PathBuf};

use clap::Parser;
use sts_simulator::eval::artifact_doctor::{
    audit_artifacts, render_artifact_audit_summary, save_artifact_audit_report,
};

#[derive(Parser, Debug)]
#[command(about = "Read-only audit for combat benchmark artifacts")]
struct Args {
    #[arg(long, default_value = "tools/artifacts/benchmarks")]
    root: PathBuf,

    #[arg(long)]
    output: Option<PathBuf>,

    #[arg(long)]
    json: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let report = audit_artifacts(&args.root);
    if let Some(path) = args.output.as_ref() {
        save_artifact_audit_report(path, &report)?;
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", render_artifact_audit_summary(&report));
        if let Some(path) = args.output.as_ref() {
            println!("  report={}", normalize_output_path(path));
        }
    }

    Ok(())
}

fn normalize_output_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}
