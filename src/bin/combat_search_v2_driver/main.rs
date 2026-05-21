use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy;
use sts_simulator::eval::combat_search_v2::{
    load_combat_search_v2_start, run_combat_search_v2_loaded_start, CombatSearchV2RunOptions,
};

#[derive(Parser, Debug)]
#[command(about = "Combat Search V2 whole-combat runner over a start-spec")]
struct Args {
    #[arg(long)]
    start_spec: PathBuf,

    #[arg(long)]
    max_nodes: Option<usize>,

    #[arg(long)]
    max_actions_per_line: Option<usize>,

    #[arg(long)]
    max_engine_steps_per_action: Option<usize>,

    #[arg(long)]
    wall_ms: Option<u64>,

    #[arg(long, value_enum)]
    potion_policy: Option<CliPotionPolicy>,

    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliPotionPolicy {
    Never,
    LethalOnly,
    All,
}

impl From<CliPotionPolicy> for CombatSearchV2PotionPolicy {
    fn from(value: CliPotionPolicy) -> Self {
        match value {
            CliPotionPolicy::Never => CombatSearchV2PotionPolicy::Never,
            CliPotionPolicy::LethalOnly => CombatSearchV2PotionPolicy::LethalOnly,
            CliPotionPolicy::All => CombatSearchV2PotionPolicy::All,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let loaded = load_combat_search_v2_start(&args.start_spec)?;
    let run = run_combat_search_v2_loaded_start(
        &loaded,
        CombatSearchV2RunOptions {
            max_nodes: args.max_nodes,
            max_actions_per_line: args.max_actions_per_line,
            max_engine_steps_per_action: args.max_engine_steps_per_action,
            wall_ms: args.wall_ms,
            potion_policy: args.potion_policy.map(Into::into),
        },
    );
    let payload = serde_json::to_string_pretty(&run.search_report)?;
    write_or_print(args.output.as_ref(), &payload)?;
    Ok(())
}

fn write_or_print(path: Option<&PathBuf>, payload: &str) -> Result<(), std::io::Error> {
    if let Some(path) = path {
        ensure_parent_dir(path)?;
        fs::write(path, payload)
    } else {
        println!("{payload}");
        Ok(())
    }
}

fn ensure_parent_dir(path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
