use std::path::PathBuf;

use clap::Parser;

use sts_simulator::diff::replay::live_comm_replay::{
    derive_combat_replay_view, full_run_command_kind_counts, inspect_combat_replay_step,
    load_live_session_replay_path, verify_combat_replay_view, write_live_session_replay_to_path,
};

#[derive(Parser, Debug)]
struct Args {
    path: PathBuf,
    #[arg(long)]
    combat_only: bool,
    #[arg(long)]
    full_run_report: bool,
    #[arg(long)]
    first_fail: bool,
    #[arg(long)]
    all_fails: bool,
    #[arg(long)]
    emit_structured: Option<PathBuf>,
    #[arg(long)]
    inspect_step: Option<usize>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let replay = load_live_session_replay_path(&args.path)
        .map_err(|err| format!("failed to load replay '{}': {err}", args.path.display()))?;

    if let Some(path) = &args.emit_structured {
        write_live_session_replay_to_path(&replay, path)?;
        println!("wrote structured replay: {}", path.display());
    }

    if args.full_run_report || !args.combat_only {
        println!(
            "session replay: frames={} steps={} source={}",
            replay.total_frames,
            replay.steps.len(),
            replay.source_path.as_deref().unwrap_or("<unknown>")
        );
        for (kind, count) in full_run_command_kind_counts(&replay) {
            println!("  kind={kind} count={count}");
        }
    }

    let combat_view = derive_combat_replay_view(&replay);

    if let Some(step_index) = args.inspect_step {
        let inspection = inspect_combat_replay_step(&combat_view, step_index)?;
        println!(
            "inspect step_index={} command_id={} response_id={:?} frame_id={:?} command={}",
            inspection.step_index,
            inspection.command_id,
            inspection.response_id,
            inspection.state_frame_id,
            inspection.command_text
        );
        println!(
            "rust_after={}",
            serde_json::to_string_pretty(&inspection.rust_after)?
        );
        println!(
            "java_after={}",
            serde_json::to_string_pretty(&inspection.java_after)?
        );
        if inspection.diffs.is_empty() {
            println!("diffs=[]");
            return Ok(());
        }
        println!("diffs={}", serde_json::to_string_pretty(&inspection.diffs)?);
        return Err("combat inspection found divergence(s)".into());
    }

    let first_fail_only = args.first_fail || !args.all_fails;
    let report = verify_combat_replay_view(&combat_view, first_fail_only)?;

    println!(
        "combat replay: total={} executable={} skipped_noncombat={} unsupported={} insufficient_context={} failures={}",
        report.total_steps,
        report.executable_steps,
        report.skipped_noncombat_steps,
        report.unsupported_steps,
        report.insufficient_context_steps,
        report.failures.len()
    );

    if report.failures.is_empty() {
        println!("combat verification: OK");
        return Ok(());
    }

    if args.all_fails {
        for failure in &report.failures {
            print_failure(failure);
        }
    } else {
        print_failure(&report.failures[0]);
    }

    Err("combat verification found divergence(s)".into())
}

fn print_failure(
    failure: &sts_simulator::diff::replay::live_comm_replay::CombatVerificationFailure,
) {
    println!(
        "FAIL step_index={} command_id={} response_id={:?} frame_id={:?} command={}",
        failure.step_index,
        failure.command_id,
        failure.response_id,
        failure.state_frame_id,
        failure.command_text
    );
    for diff in &failure.diffs {
        println!(
            "  [{}] {} : Rust={} Java={}",
            diff.category, diff.field, diff.rust_val, diff.java_val
        );
    }
}
