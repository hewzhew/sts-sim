use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy;
use sts_simulator::eval::combat_search_v2::{
    load_combat_search_v2_start, load_complete_baseline_from_case_path,
    run_combat_search_v2_benchmark_manifest, run_combat_search_v2_loaded_start,
    CombatSearchV2RunOptions, CombatSearchV2StartSource,
};

#[derive(Parser, Debug)]
#[command(about = "Combat Search V2 whole-combat runner")]
struct Args {
    #[arg(long, conflicts_with_all = ["case", "start_spec", "baseline_case"])]
    manifest: Option<PathBuf>,

    #[arg(long, conflicts_with_all = ["manifest", "start_spec"])]
    case: Option<PathBuf>,

    #[arg(long, conflicts_with_all = ["manifest", "case"])]
    start_spec: Option<PathBuf>,

    #[arg(long, conflicts_with = "manifest")]
    baseline_case: Option<PathBuf>,

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
    jsonl: Option<PathBuf>,

    #[arg(long)]
    require_baseline: bool,

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
    let options = run_options_from_args(&args);
    if let Some(manifest) = args.manifest.as_ref() {
        run_manifest_mode(&args, manifest, options)?;
    } else {
        run_single_mode(&args, options)?;
    }
    Ok(())
}

fn run_options_from_args(args: &Args) -> CombatSearchV2RunOptions {
    CombatSearchV2RunOptions {
        max_nodes: args.max_nodes,
        max_actions_per_line: args.max_actions_per_line,
        max_engine_steps_per_action: args.max_engine_steps_per_action,
        wall_ms: args.wall_ms,
        potion_policy: args.potion_policy.map(Into::into),
    }
}

fn run_single_mode(
    args: &Args,
    options: CombatSearchV2RunOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = start_source_from_args(args)?;
    let loaded = load_combat_search_v2_start(&source)?;
    let baseline_override = if let Some(path) = args.baseline_case.as_ref() {
        load_complete_baseline_from_case_path(path)?
    } else {
        None
    };
    if args.require_baseline && baseline_override.is_none() && loaded.case_baseline.is_none() {
        return Err(
            "a complete baseline is required but no complete case program was available"
                .to_string()
                .into(),
        );
    }

    let run = run_combat_search_v2_loaded_start(&loaded, baseline_override, options);
    let payload = serde_json::to_string_pretty(&run.to_legacy_output_value()?)?;
    write_or_print(args.output.as_ref(), &payload)?;
    Ok(())
}

fn run_manifest_mode(
    args: &Args,
    manifest_path: &Path,
    options: CombatSearchV2RunOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from("target/combat_search_v2_bench_report.json"));
    let jsonl_path = args
        .jsonl
        .clone()
        .unwrap_or_else(|| output_path.with_extension("jsonl"));
    ensure_parent_dir(&output_path)?;
    ensure_parent_dir(&jsonl_path)?;

    let jsonl_file = fs::File::create(&jsonl_path)?;
    let mut jsonl = BufWriter::new(jsonl_file);
    let summary = run_combat_search_v2_benchmark_manifest(
        manifest_path,
        options,
        args.require_baseline,
        |detail| {
            serde_json::to_writer(&mut jsonl, detail).map_err(|err| err.to_string())?;
            jsonl.write_all(b"\n").map_err(|err| err.to_string())?;
            Ok::<(), String>(())
        },
    )?;
    jsonl.flush()?;

    let payload = serde_json::to_string_pretty(&summary)?;
    fs::write(&output_path, &payload)?;
    println!("{payload}");
    Ok(())
}

fn start_source_from_args(args: &Args) -> Result<CombatSearchV2StartSource, String> {
    match (&args.case, &args.start_spec) {
        (Some(path), None) => Ok(CombatSearchV2StartSource::Case(path.clone())),
        (None, Some(path)) => Ok(CombatSearchV2StartSource::StartSpec(path.clone())),
        _ => Err("provide exactly one of --case or --start-spec".to_string()),
    }
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
