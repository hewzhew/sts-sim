use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use sts_simulator::testing::combat_lab::{
    run_combat_lab, write_sanitized_fixture_for_local_lab, CombatLabConfig, LabPolicyMode,
    LabVariantMode,
};
use sts_simulator::testing::scenario::ScenarioFixture;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliPolicy {
    #[value(name = "bot")]
    Bot,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliVariantMode {
    #[value(name = "exact")]
    Exact,
    #[value(name = "reshuffle_draw")]
    ReshuffleDraw,
}

impl From<CliPolicy> for LabPolicyMode {
    fn from(value: CliPolicy) -> Self {
        match value {
            CliPolicy::Bot => LabPolicyMode::Bot,
        }
    }
}

impl From<CliVariantMode> for LabVariantMode {
    fn from(value: CliVariantMode) -> Self {
        match value {
            CliVariantMode::Exact => LabVariantMode::Exact,
            CliVariantMode::ReshuffleDraw => LabVariantMode::ReshuffleDraw,
        }
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long)]
    fixture: PathBuf,
    #[arg(long, default_value_t = 1)]
    episodes: usize,
    #[arg(long, value_enum, default_value_t = CliPolicy::Bot)]
    policy: CliPolicy,
    #[arg(long, default_value_t = 6)]
    depth: u32,
    #[arg(long, value_enum, default_value_t = CliVariantMode::Exact)]
    variant_mode: CliVariantMode,
    #[arg(long, default_value_t = 1)]
    base_seed: u64,
    #[arg(long)]
    out_dir: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let fixture_payload = std::fs::read_to_string(&args.fixture)?;
    let fixture: ScenarioFixture = serde_json::from_str(&fixture_payload)?;

    std::fs::create_dir_all(&args.out_dir)?;
    let sanitized_path = args.out_dir.join("fixture_start.json");
    let sanitized = write_sanitized_fixture_for_local_lab(&fixture, &sanitized_path)?;

    let summary = run_combat_lab(&CombatLabConfig {
        fixture: sanitized,
        episodes: args.episodes,
        policy: args.policy.into(),
        depth: args.depth,
        variant_mode: args.variant_mode.into(),
        base_seed: args.base_seed,
        out_dir: args.out_dir.clone(),
    })?;

    println!(
        "combat_lab wrote {} episode(s) to {} | wins={} win_rate={:.3} best_win={:?} best_attempt={:?}",
        summary.total_episodes,
        args.out_dir.display(),
        summary.wins,
        summary.win_rate,
        summary.best_win_episode_id,
        summary.best_attempt_episode_id
    );
    Ok(())
}
