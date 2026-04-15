use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use sts_simulator::testing::fixtures::author_spec::{compile_combat_author_spec, CombatAuthorSpec};
use sts_simulator::testing::fixtures::scenario::ScenarioFixture;
use sts_simulator::bot::harness::combat_lab::{
    run_combat_lab, write_sanitized_fixture_for_local_lab, CombatLabConfig, LabVariantMode,
};
use sts_simulator::bot::harness::combat_policy::PolicyKind;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliPolicy {
    #[value(name = "bot")]
    Bot,
    #[value(name = "heuristic")]
    Heuristic,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliVariantMode {
    #[value(name = "exact")]
    Exact,
    #[value(name = "reshuffle_draw")]
    ReshuffleDraw,
}

impl From<CliPolicy> for PolicyKind {
    fn from(value: CliPolicy) -> Self {
        match value {
            CliPolicy::Bot => PolicyKind::Bot,
            CliPolicy::Heuristic => PolicyKind::Heuristic,
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
    fixture: Option<PathBuf>,
    #[arg(long)]
    author_spec: Option<PathBuf>,
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
    let fixture = load_fixture(&args)?;

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
        "combat_lab policy={:?} wrote {} episode(s) to {} | wins={} win_rate={:.3} avg_hp={:.2} avg_dmg_taken={:.2} avg_potions={:.2} bad_actions={} best_win={:?} best_attempt={:?}",
        summary.policy,
        summary.total_episodes,
        args.out_dir.display(),
        summary.wins,
        summary.win_rate,
        summary.average_final_hp,
        summary.metrics.average_damage_taken_per_episode,
        summary.metrics.average_potion_uses_per_episode,
        summary.metrics.bad_action_count,
        summary.best_win_episode_id,
        summary.best_attempt_episode_id
    );
    Ok(())
}

fn load_fixture(args: &Args) -> Result<ScenarioFixture, Box<dyn std::error::Error>> {
    match (&args.fixture, &args.author_spec) {
        (Some(_), Some(_)) => Err("use either --fixture or --author-spec, not both".into()),
        (None, None) => Err("one of --fixture or --author-spec is required".into()),
        (Some(path), None) => {
            let fixture_payload = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&fixture_payload)?)
        }
        (None, Some(path)) => {
            let spec_payload = std::fs::read_to_string(path)?;
            let spec: CombatAuthorSpec = serde_json::from_str(&spec_payload)?;
            Ok(compile_combat_author_spec(&spec)?)
        }
    }
}
