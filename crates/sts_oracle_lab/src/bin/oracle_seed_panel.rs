use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use clap::Args;
use serde::Serialize;
use serde_json::{json, Value};
use sts_simulator::runtime::branch::{
    load_oracle_analysis_workspace_v1, run_oracle_analysis_to_stop_v1,
    save_oracle_analysis_workspace_v1, OracleAnalysisWorkspaceV1, OracleAutonomousRunConfigV1,
    OracleRunBudget, OracleRunConfig,
};

#[derive(Clone, Debug, Args)]
pub struct OracleSeedPanelArgs {
    /// First exact game seed in the consecutive panel.
    #[arg(long)]
    seed_start: u64,
    /// Number of consecutive seeds in the resumable panel.
    #[arg(
        long,
        default_value_t = 10,
        value_parser = clap::value_parser!(u16).range(1..=1000)
    )]
    count: u16,
    #[arg(long, default_value_t = 0)]
    ascension: u8,
    /// Durable reports, exact witnesses, and resumable failures live here.
    #[arg(long)]
    output_dir: PathBuf,
    /// Total wall allowance for one seed. A stopped seed remains resumable.
    #[arg(long, default_value_t = 30_000)]
    run_wall_ms: u64,
    /// Total wall allowance for this invocation. Zero disables the cap.
    ///
    /// A capped invocation exits successfully after publishing its partial
    /// summary. Re-running the same command skips durable results and resumes
    /// only interrupted seeds.
    #[arg(long, default_value_t = 600_000)]
    invocation_wall_ms: u64,
    #[arg(long, default_value_t = 250_000)]
    hallway_nodes: usize,
    #[arg(long, default_value_t = 5_000)]
    hallway_ms: u64,
    #[arg(long, default_value_t = 750_000)]
    elite_nodes: usize,
    #[arg(long, default_value_t = 15_000)]
    elite_ms: u64,
    #[arg(long, default_value_t = 2_000_000)]
    boss_nodes: usize,
    #[arg(long, default_value_t = 30_000)]
    boss_ms: u64,
    #[arg(long, default_value_t = 100_000)]
    max_quanta: usize,
    #[arg(long, default_value_t = 4_096)]
    quantum_nodes: usize,
    #[arg(long, default_value_t = 100)]
    quantum_ms: u64,
    #[arg(long, default_value_t = 256)]
    max_boundaries: usize,
    /// Ignore prior reports and resumable workspaces for the selected seeds.
    #[arg(long)]
    force: bool,
    /// Re-enter deterministic stopped/error states instead of preserving their first report.
    #[arg(long)]
    retry_stopped: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct PanelSeedSummaryV1 {
    seed: u64,
    status: String,
    reason: Option<String>,
    resumed: bool,
    /// Time spent inside the autonomous run loop.
    elapsed_ms: u64,
    /// End-to-end time for this seed, including workspace load and persistence.
    total_elapsed_ms: u64,
    /// Non-run overhead, principally workspace load and durable persistence.
    persistence_elapsed_ms: u64,
    act: Option<u64>,
    floor: Option<i64>,
    current_hp: Option<i64>,
    max_hp: Option<i64>,
    combat_count: Option<u64>,
    owner_decisions: Option<u64>,
    report: PathBuf,
    continuation: Option<PathBuf>,
    workspace: Option<PathBuf>,
    error: Option<String>,
}

pub fn run(args: OracleSeedPanelArgs) -> Result<Value, String> {
    validate_args(&args)?;
    let reports_dir = args.output_dir.join("reports");
    let witnesses_dir = args.output_dir.join("witnesses");
    let incomplete_dir = args.output_dir.join("incomplete");
    fs::create_dir_all(&reports_dir)
        .map_err(|error| format!("failed to create {}: {error}", reports_dir.display()))?;
    fs::create_dir_all(&witnesses_dir)
        .map_err(|error| format!("failed to create {}: {error}", witnesses_dir.display()))?;
    fs::create_dir_all(&incomplete_dir)
        .map_err(|error| format!("failed to create {}: {error}", incomplete_dir.display()))?;

    let source = source_identity();
    let panel_started = Instant::now();
    let mut seeds = Vec::with_capacity(usize::from(args.count));
    for offset in 0..u64::from(args.count) {
        if invocation_wall_budget_reached(&args, panel_started) {
            break;
        }
        let seed = args
            .seed_start
            .checked_add(offset)
            .ok_or_else(|| "seed panel range overflowed u64".to_string())?;
        let report_path = reports_dir.join(format!("seed-{seed}.report.json"));
        let continuation_path = witnesses_dir.join(format!("seed-{seed}.continuation.json"));
        let workspace_path = incomplete_dir.join(format!("seed-{seed}.workspace.json"));

        if !args.force {
            if let Some(summary) = reusable_summary(
                seed,
                &report_path,
                &continuation_path,
                &workspace_path,
                args.retry_stopped,
            )? {
                seeds.push(summary);
                write_panel_summary(&args, &source, &seeds, panel_started, &args.output_dir)?;
                continue;
            }
        }

        let seed_total_started = Instant::now();
        let resumed = !args.force && workspace_path.is_file();
        let mut workspace = if resumed {
            let workspace = load_oracle_analysis_workspace_v1(&workspace_path)?;
            if workspace.seed != seed || workspace.ascension != args.ascension {
                return Err(format!(
                    "resumable workspace {} belongs to seed {} A{}, expected seed {seed} A{}",
                    workspace_path.display(),
                    workspace.seed,
                    workspace.ascension,
                    args.ascension
                ));
            }
            workspace
        } else {
            OracleAnalysisWorkspaceV1::new_with_combat_guidance(
                OracleRunConfig {
                    seed,
                    ascension: args.ascension,
                    budget: run_budget(&args),
                },
                None,
            )?
        };

        let seed_started = Instant::now();
        let run_result = run_oracle_analysis_to_stop_v1(
            &mut workspace,
            &OracleAutonomousRunConfigV1 {
                hallway_wall_ms: args.hallway_ms,
                elite_wall_ms: args.elite_ms,
                boss_wall_ms: args.boss_ms,
                max_quanta: args.max_quanta,
                quantum_nodes: args.quantum_nodes,
                quantum_ms: args.quantum_ms,
                max_boundaries: args.max_boundaries,
                run_wall_ms: Some(current_seed_wall_ms(&args, panel_started)),
                export_continuation: Some(continuation_path.clone()),
            },
        );
        let elapsed_ms = elapsed_millis(seed_started);

        let mut summary = match run_result {
            Ok(report) => {
                write_json(&report_path, &report)?;
                let status = report
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let victory = status == "victory_verified";
                if victory {
                    if workspace_path.is_file() {
                        fs::remove_file(&workspace_path).map_err(|error| {
                            format!(
                                "failed to remove completed workspace {}: {error}",
                                workspace_path.display()
                            )
                        })?;
                    }
                } else {
                    save_oracle_analysis_workspace_v1(&workspace_path, &workspace)?;
                }
                summary_from_report(
                    seed,
                    &report,
                    resumed,
                    elapsed_ms,
                    report_path,
                    victory.then_some(continuation_path),
                    (!victory).then_some(workspace_path),
                )
            }
            Err(error) => {
                save_oracle_analysis_workspace_v1(&workspace_path, &workspace)?;
                let report = json!({
                    "schema_name": "OracleSeedPanelErrorV1",
                    "schema_version": 1,
                    "seed": seed,
                    "ascension": args.ascension,
                    "status": "error",
                    "error": error,
                });
                write_json(&report_path, &report)?;
                PanelSeedSummaryV1 {
                    seed,
                    status: "error".to_string(),
                    reason: None,
                    resumed,
                    elapsed_ms,
                    total_elapsed_ms: elapsed_ms,
                    persistence_elapsed_ms: 0,
                    act: None,
                    floor: None,
                    current_hp: None,
                    max_hp: None,
                    combat_count: None,
                    owner_decisions: None,
                    report: report_path,
                    continuation: None,
                    workspace: Some(workspace_path),
                    error: report
                        .get("error")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                }
            }
        };
        summary.total_elapsed_ms = elapsed_millis(seed_total_started);
        summary.persistence_elapsed_ms =
            summary.total_elapsed_ms.saturating_sub(summary.elapsed_ms);
        eprintln!(
            "seed {}: {} at A{}F{} in {} ms run / {} ms total",
            summary.seed,
            summary.status,
            summary.act.unwrap_or(0),
            summary.floor.unwrap_or(0),
            summary.elapsed_ms,
            summary.total_elapsed_ms,
        );
        seeds.push(summary);
        write_panel_summary(&args, &source, &seeds, panel_started, &args.output_dir)?;
    }

    let summary = panel_summary(&args, &source, &seeds, panel_started)?;
    write_json(&args.output_dir.join("panel.summary.json"), &summary)?;
    Ok(summary)
}

fn validate_args(args: &OracleSeedPanelArgs) -> Result<(), String> {
    if args.run_wall_ms == 0
        || args.hallway_nodes == 0
        || args.hallway_ms == 0
        || args.elite_nodes == 0
        || args.elite_ms == 0
        || args.boss_nodes == 0
        || args.boss_ms == 0
        || args.max_quanta == 0
        || args.quantum_nodes == 0
        || args.quantum_ms == 0
        || args.max_boundaries == 0
    {
        return Err("seed panel budgets and limits must be positive".to_string());
    }
    Ok(())
}

fn run_budget(args: &OracleSeedPanelArgs) -> OracleRunBudget {
    OracleRunBudget {
        hallway_nodes: args.hallway_nodes,
        hallway_ms: args.hallway_ms,
        elite_nodes: args.elite_nodes,
        elite_ms: args.elite_ms,
        boss_nodes: args.boss_nodes,
        boss_ms: args.boss_ms,
        ..OracleRunBudget::default()
    }
}

fn reusable_summary(
    seed: u64,
    report_path: &Path,
    continuation_path: &Path,
    workspace_path: &Path,
    retry_stopped: bool,
) -> Result<Option<PanelSeedSummaryV1>, String> {
    if !report_path.is_file() {
        return Ok(None);
    }
    let report = read_json(report_path)?;
    let status = report.get("status").and_then(Value::as_str);
    let victory = status == Some("victory_verified") && continuation_path.is_file();
    if victory {
        return Ok(Some(summary_from_report(
            seed,
            &report,
            false,
            0,
            report_path.to_path_buf(),
            Some(continuation_path.to_path_buf()),
            None,
        )));
    }
    if !workspace_path.is_file() {
        return Ok(None);
    }
    let reason = report.get("reason").and_then(Value::as_str);
    let interrupted = matches!(reason, Some("run_wall_budget" | "boundary_limit"));
    if retry_stopped || interrupted {
        return Ok(None);
    }
    Ok(Some(summary_from_report(
        seed,
        &report,
        false,
        0,
        report_path.to_path_buf(),
        None,
        Some(workspace_path.to_path_buf()),
    )))
}

fn summary_from_report(
    seed: u64,
    report: &Value,
    resumed: bool,
    elapsed_ms: u64,
    report_path: PathBuf,
    continuation: Option<PathBuf>,
    workspace: Option<PathBuf>,
) -> PanelSeedSummaryV1 {
    let final_node = report.get("final");
    PanelSeedSummaryV1 {
        seed,
        status: report
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        reason: report
            .get("reason")
            .and_then(Value::as_str)
            .map(str::to_string),
        resumed,
        elapsed_ms,
        total_elapsed_ms: elapsed_ms,
        persistence_elapsed_ms: 0,
        act: final_node
            .and_then(|value| value.get("act"))
            .and_then(Value::as_u64),
        floor: final_node
            .and_then(|value| value.get("floor"))
            .and_then(Value::as_i64),
        current_hp: final_node
            .and_then(|value| value.get("hp"))
            .and_then(Value::as_i64),
        max_hp: final_node
            .and_then(|value| value.get("max_hp"))
            .and_then(Value::as_i64),
        combat_count: report.get("combat_count").and_then(Value::as_u64),
        owner_decisions: report.get("owner_decisions").and_then(Value::as_u64),
        report: report_path,
        continuation,
        workspace,
        error: report
            .get("error")
            .and_then(Value::as_str)
            .map(str::to_string),
    }
}

fn panel_summary(
    args: &OracleSeedPanelArgs,
    source: &Value,
    seeds: &[PanelSeedSummaryV1],
    started: Instant,
) -> Result<Value, String> {
    let victories = seeds
        .iter()
        .filter(|seed| seed.status == "victory_verified")
        .count();
    let stopped = seeds.iter().filter(|seed| seed.status == "stopped").count();
    let errors = seeds.iter().filter(|seed| seed.status == "error").count();
    let requested = usize::from(args.count);
    let complete = seeds.len() == requested;
    Ok(json!({
        "schema_name": "OracleSeedPanelReportV1",
        "schema_version": 1,
        "status": if complete { "complete" } else { "interrupted" },
        "reason": if complete {
            Value::Null
        } else {
            Value::String("invocation_wall_budget".to_string())
        },
        "seed_start": args.seed_start,
        "count": args.count,
        "ascension": args.ascension,
        "source": source,
        "budgets": {
            "run_wall_ms": args.run_wall_ms,
            "invocation_wall_ms": args.invocation_wall_ms,
            "hallway_nodes": args.hallway_nodes,
            "hallway_ms": args.hallway_ms,
            "elite_nodes": args.elite_nodes,
            "elite_ms": args.elite_ms,
            "boss_nodes": args.boss_nodes,
            "boss_ms": args.boss_ms,
            "max_quanta": args.max_quanta,
            "quantum_nodes": args.quantum_nodes,
            "quantum_ms": args.quantum_ms,
            "max_boundaries": args.max_boundaries,
        },
        "elapsed_ms": elapsed_millis(started),
        "completed": seeds.len(),
        "remaining": requested.saturating_sub(seeds.len()),
        "victories": victories,
        "stopped": stopped,
        "errors": errors,
        "seeds": seeds,
    }))
}

fn invocation_wall_budget_reached(args: &OracleSeedPanelArgs, started: Instant) -> bool {
    args.invocation_wall_ms != 0 && elapsed_millis(started) >= args.invocation_wall_ms
}

fn current_seed_wall_ms(args: &OracleSeedPanelArgs, started: Instant) -> u64 {
    if args.invocation_wall_ms == 0 {
        return args.run_wall_ms;
    }
    args.run_wall_ms.min(
        args.invocation_wall_ms
            .saturating_sub(elapsed_millis(started))
            .max(1),
    )
}

fn write_panel_summary(
    args: &OracleSeedPanelArgs,
    source: &Value,
    seeds: &[PanelSeedSummaryV1],
    started: Instant,
    output_dir: &Path,
) -> Result<(), String> {
    write_json(
        &output_dir.join("panel.summary.json"),
        &panel_summary(args, source, seeds, started)?,
    )
}

fn source_identity() -> Value {
    let commit = git_output(&["rev-parse", "HEAD"]);
    let dirty = git_output(&["status", "--porcelain"])
        .map(|status| !status.trim().is_empty())
        .unwrap_or(true);
    json!({
        "commit": commit,
        "dirty": dirty,
        "binary": std::env::current_exe().ok(),
        "started_unix_ms": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    })
}

fn git_output(arguments: &[&str]) -> Option<String> {
    let output = Command::new("git").args(arguments).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn read_json(path: &Path) -> Result<Value, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("failed to serialize {}: {error}", path.display()))?;
    fs::write(&temporary, bytes)
        .map_err(|error| format!("failed to write {}: {error}", temporary.display()))?;
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("failed to replace {}: {error}", path.display()))?;
    }
    fs::rename(&temporary, path).map_err(|error| {
        format!(
            "failed to publish {} as {}: {error}",
            temporary.display(),
            path.display()
        )
    })
}

fn elapsed_millis(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use clap::Parser;

    use super::{current_seed_wall_ms, invocation_wall_budget_reached, OracleSeedPanelArgs};

    #[derive(Debug, Parser)]
    struct TestCli {
        #[command(flatten)]
        panel: OracleSeedPanelArgs,
    }

    fn parse_defaults() -> OracleSeedPanelArgs {
        TestCli::try_parse_from([
            "seed-panel-test",
            "--seed-start",
            "20260713006",
            "--output-dir",
            "panel-output",
        ])
        .expect("safe panel defaults parse")
        .panel
    }

    #[test]
    fn daily_panel_defaults_are_small_and_bounded() {
        let args = parse_defaults();

        assert_eq!(args.count, 10);
        assert_eq!(args.run_wall_ms, 30_000);
        assert_eq!(args.invocation_wall_ms, 600_000);
        assert_eq!(args.output_dir, PathBuf::from("panel-output"));
    }

    #[test]
    fn invocation_budget_caps_the_current_seed_allowance() {
        let mut args = parse_defaults();
        args.invocation_wall_ms = 60_000;
        let started = Instant::now() - Duration::from_secs(31);

        assert!(!invocation_wall_budget_reached(&args, started));
        assert!(current_seed_wall_ms(&args, started) <= 29_000);
        assert!(current_seed_wall_ms(&args, started) < args.run_wall_ms);
    }

    #[test]
    fn zero_invocation_budget_disables_the_cap() {
        let mut args = parse_defaults();
        args.invocation_wall_ms = 0;
        let started = Instant::now() - Duration::from_secs(60);

        assert!(!invocation_wall_budget_reached(&args, started));
        assert_eq!(current_seed_wall_ms(&args, started), args.run_wall_ms);
    }
}
