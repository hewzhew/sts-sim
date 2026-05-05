use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use sts_simulator::bot::harness::PolicyKind;
use sts_simulator::bot::harness::{
    run_combat_case_lab, write_sanitized_case_for_local_lab, CombatCaseLabConfig, LabVariantMode,
};
use sts_simulator::fixtures::author_spec::CombatAuthorSpec;
use sts_simulator::fixtures::combat_case::{
    case_from_scenario_fixture, compile_combat_author_case, CombatCase,
};
use sts_simulator::fixtures::scenario::ScenarioFixture;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliPolicy {
    #[value(name = "bot")]
    Bot,
    #[value(name = "bot_contested_takeover")]
    BotContestedTakeover,
    #[value(name = "bot_no_idle_end_turn")]
    BotNoIdleEndTurn,
    #[value(name = "bot_combined")]
    BotCombined,
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
            CliPolicy::BotContestedTakeover => PolicyKind::BotContestedTakeover,
            CliPolicy::BotNoIdleEndTurn => PolicyKind::BotNoIdleEndTurn,
            CliPolicy::BotCombined => PolicyKind::BotCombined,
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
    case: Option<PathBuf>,
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
    let case = load_case(&args)?;

    std::fs::create_dir_all(&args.out_dir)?;
    let sanitized_path = args.out_dir.join("case_start.json");
    let sanitized = write_sanitized_case_for_local_lab(&case, &sanitized_path)?;

    let summary = run_combat_case_lab(&CombatCaseLabConfig {
        case: sanitized,
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

fn load_case(args: &Args) -> Result<CombatCase, Box<dyn std::error::Error>> {
    let mut supplied = 0usize;
    if args.case.is_some() {
        supplied += 1;
    }
    if args.fixture.is_some() {
        supplied += 1;
    }
    if args.author_spec.is_some() {
        supplied += 1;
    }
    if supplied != 1 {
        return Err("use exactly one of --case, --fixture, or --author-spec".into());
    }

    if let Some(path) = &args.case {
        let payload = std::fs::read_to_string(path)?;
        return Ok(serde_json::from_str(&payload)?);
    }
    if let Some(path) = &args.fixture {
        let fixture_payload = std::fs::read_to_string(path)?;
        let fixture: ScenarioFixture = serde_json::from_str(&fixture_payload)?;
        return Ok(case_from_scenario_fixture(&fixture)?);
    }
    if let Some(path) = &args.author_spec {
        let spec_payload = std::fs::read_to_string(path)?;
        let spec: CombatAuthorSpec = serde_json::from_str(&spec_payload)?;
        return Ok(compile_combat_author_case(&spec)?);
    }

    Err("one of --case, --fixture, or --author-spec is required".into())
}
