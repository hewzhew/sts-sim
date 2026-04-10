use clap::Parser;
use std::io::Write;
use sts_simulator::ml::reward_samples::{
    expand_reward_sample_to_choice_rows, load_raw_response_lookup,
    reward_choice_row_is_disagreement, reward_sample_from_audit_line,
    reward_sample_is_disagreement,
};

#[derive(Parser, Debug)]
struct Args {
    #[arg(
        long,
        default_value = "D:\\rust\\sts_simulator\\live_comm_reward_audit.jsonl"
    )]
    audit: String,
    #[arg(long, default_value = "D:\\rust\\sts_simulator\\live_comm_raw.jsonl")]
    raw: String,
    #[arg(
        long,
        default_value = "D:\\rust\\sts_simulator\\data\\reward_samples.jsonl"
    )]
    out: String,
    #[arg(
        long,
        default_value = "D:\\rust\\sts_simulator\\data\\reward_choice_rows.jsonl"
    )]
    choice_out: String,
    #[arg(
        long,
        default_value = "D:\\rust\\sts_simulator\\data\\reward_disagreements.jsonl"
    )]
    disagreement_out: String,
    #[arg(
        long,
        default_value = "D:\\rust\\sts_simulator\\data\\reward_disagreement_choice_rows.jsonl"
    )]
    disagreement_choice_out: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let raw_lookup = load_raw_response_lookup(&args.raw)?;
    let audit_content = std::fs::read_to_string(&args.audit)?;

    let out_path = std::path::Path::new(&args.out);
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = std::fs::File::create(out_path)?;
    let choice_out_path = std::path::Path::new(&args.choice_out);
    if let Some(parent) = choice_out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut choice_out = std::fs::File::create(choice_out_path)?;
    let disagreement_out_path = std::path::Path::new(&args.disagreement_out);
    if let Some(parent) = disagreement_out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut disagreement_out = std::fs::File::create(disagreement_out_path)?;
    let disagreement_choice_out_path = std::path::Path::new(&args.disagreement_choice_out);
    if let Some(parent) = disagreement_choice_out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut disagreement_choice_out = std::fs::File::create(disagreement_choice_out_path)?;

    let mut exported = 0usize;
    let mut exported_choice_rows = 0usize;
    let mut exported_disagreements = 0usize;
    let mut exported_disagreement_choice_rows = 0usize;
    let mut skipped = 0usize;

    for line in audit_content.lines().filter(|line| !line.trim().is_empty()) {
        match reward_sample_from_audit_line(line, &raw_lookup)? {
            Some(sample) => {
                writeln!(out, "{}", serde_json::to_string(&sample)?)?;
                if reward_sample_is_disagreement(&sample) {
                    writeln!(disagreement_out, "{}", serde_json::to_string(&sample)?)?;
                    exported_disagreements += 1;
                }
                for row in expand_reward_sample_to_choice_rows(&sample) {
                    writeln!(choice_out, "{}", serde_json::to_string(&row)?)?;
                    exported_choice_rows += 1;
                    if reward_choice_row_is_disagreement(&row) {
                        writeln!(disagreement_choice_out, "{}", serde_json::to_string(&row)?)?;
                        exported_disagreement_choice_rows += 1;
                    }
                }
                exported += 1;
            }
            None => skipped += 1,
        }
    }

    println!(
        "exported {exported} reward samples to {}, {exported_choice_rows} choice rows to {}, {exported_disagreements} disagreement samples to {}, and {exported_disagreement_choice_rows} disagreement choice rows to {} (skipped {skipped})",
        out_path.display(),
        choice_out_path.display(),
        disagreement_out_path.display(),
        disagreement_choice_out_path.display()
    );
    Ok(())
}
