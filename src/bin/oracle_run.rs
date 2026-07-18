use std::path::PathBuf;

use clap::Parser;
use sts_simulator::eval::combat_case::save_combat_case;
use sts_simulator::runtime::branch::{run_oracle_run, OracleRunBudget, OracleRunConfig};

#[derive(Debug, Parser)]
#[command(
    name = "oracle_run",
    about = "Explore bounded exact run branches until an Act-3-boss victory witness is found"
)]
struct Cli {
    #[arg(long)]
    seed: u64,

    #[arg(long, default_value_t = 0)]
    ascension: u8,

    #[arg(long, default_value_t = 2_048)]
    max_work_items: usize,

    #[arg(long)]
    wall_ms: Option<u64>,

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

    #[arg(long, default_value_t = 50_000)]
    combat_quantum_nodes: usize,

    #[arg(long, default_value_t = 1_000)]
    combat_quantum_ms: u64,

    /// Save the first exact unresolved combat as a standalone combat case.
    #[arg(long)]
    combat_case_out: Option<PathBuf>,
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let report = run_oracle_run(OracleRunConfig {
        seed: cli.seed,
        ascension: cli.ascension,
        budget: OracleRunBudget {
            max_work_items: cli.max_work_items,
            wall_ms: cli.wall_ms,
            hallway_nodes: cli.hallway_nodes,
            hallway_ms: cli.hallway_ms,
            elite_nodes: cli.elite_nodes,
            elite_ms: cli.elite_ms,
            boss_nodes: cli.boss_nodes,
            boss_ms: cli.boss_ms,
            combat_quantum_nodes: cli.combat_quantum_nodes,
            combat_quantum_ms: cli.combat_quantum_ms,
        },
    })?;
    if let Some(path) = cli.combat_case_out.as_ref() {
        let case = report
            .first_unresolved_combat_case
            .as_ref()
            .ok_or_else(|| {
                "oracle run did not encounter an unresolved combat to export".to_string()
            })?;
        save_combat_case(path, case)?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("failed to serialize oracle report: {error}"))?
    );
    Ok(())
}
