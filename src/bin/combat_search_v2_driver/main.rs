use std::fs;
use std::path::{Path, PathBuf};

use clap::{ArgGroup, Parser, ValueEnum};
use sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy;
use sts_simulator::eval::combat_capture::load_combat_capture_v1;
use sts_simulator::eval::combat_search_v2::{
    load_combat_search_v2_benchmark, load_combat_search_v2_snapshot, load_combat_search_v2_start,
    run_combat_search_v2_benchmark, run_combat_search_v2_loaded_start, CombatSearchV2RunOptions,
};
use sts_simulator::eval::fingerprint::StateFingerprintV1;

#[derive(Parser, Debug)]
#[command(
    about = "Combat Search V2 whole-combat runner over exact combat inputs",
    group(
        ArgGroup::new("input")
            .required(true)
            .multiple(false)
            .args(["start_spec", "combat_snapshot", "benchmark_spec"])
    )
)]
struct Args {
    #[arg(long)]
    start_spec: Option<PathBuf>,

    #[arg(long)]
    combat_snapshot: Option<PathBuf>,

    #[arg(long)]
    benchmark_spec: Option<PathBuf>,

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
    validate_only: bool,

    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CliPotionPolicy {
    Never,
    All,
}

impl From<CliPotionPolicy> for CombatSearchV2PotionPolicy {
    fn from(value: CliPotionPolicy) -> Self {
        match value {
            CliPotionPolicy::Never => CombatSearchV2PotionPolicy::Never,
            CliPotionPolicy::All => CombatSearchV2PotionPolicy::All,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.validate_only {
        let payload = validate_input_payload(&args)?;
        write_or_print(args.output.as_ref(), &payload)?;
        return Ok(());
    }

    let options = CombatSearchV2RunOptions {
        max_nodes: args.max_nodes,
        max_actions_per_line: args.max_actions_per_line,
        max_engine_steps_per_action: args.max_engine_steps_per_action,
        wall_ms: args.wall_ms,
        potion_policy: args.potion_policy.map(Into::into),
    };
    let payload = if let Some(path) = args.benchmark_spec.as_ref() {
        let loaded = load_combat_search_v2_benchmark(path)?;
        let run = run_combat_search_v2_benchmark(&loaded, options);
        serde_json::to_string_pretty(&run)?
    } else {
        let loaded = if let Some(path) = args.combat_snapshot.as_ref() {
            load_combat_search_v2_snapshot(path)?
        } else {
            let path = args
                .start_spec
                .as_ref()
                .expect("clap requires exactly one input");
            load_combat_search_v2_start(path)?
        };
        let run = run_combat_search_v2_loaded_start(&loaded, options);
        serde_json::to_string_pretty(&run.search_report)?
    };
    write_or_print(args.output.as_ref(), &payload)?;
    Ok(())
}

fn validate_input_payload(args: &Args) -> Result<String, Box<dyn std::error::Error>> {
    let payload = if let Some(path) = args.combat_snapshot.as_ref() {
        let capture = load_combat_capture_v1(path)?;
        serde_json::json!({
            "schema_name": "CombatSearchV2InputValidationReport",
            "schema_version": 1,
            "status": "valid",
            "input_kind": "combat_snapshot",
            "input_path": path.display().to_string(),
            "trust_level": capture.trust_level,
            "provenance": capture.provenance,
            "fingerprints": capture.fingerprints.as_ref().map(compact_fingerprint_report),
            "legal_action_count": capture.legal_actions.as_ref().map(|actions| actions.count),
            "summary": capture.summary,
        })
    } else if let Some(path) = args.benchmark_spec.as_ref() {
        let benchmark = load_combat_search_v2_benchmark(path)?;
        let cases = benchmark
            .cases
            .iter()
            .map(|case| {
                serde_json::json!({
                    "id": case.id,
                    "input_kind": case.input.kind,
                    "input_path": case.input.path.display().to_string(),
                    "trust_level": case.start.artifact_trust_level,
                    "fingerprints": case.start.fingerprints.as_ref().map(compact_fingerprint_report),
                    "expected_fingerprints": case.expected_fingerprints.clone(),
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!({
            "schema_name": "CombatSearchV2InputValidationReport",
            "schema_version": 1,
            "status": "valid",
            "input_kind": "benchmark_spec",
            "input_path": path.display().to_string(),
            "benchmark_name": benchmark.name,
            "min_trust_level": benchmark.min_trust_level,
            "case_count": cases.len(),
            "cases": cases,
        })
    } else {
        let path = args
            .start_spec
            .as_ref()
            .expect("clap requires exactly one input");
        let start = load_combat_search_v2_start(path)?;
        serde_json::json!({
            "schema_name": "CombatSearchV2InputValidationReport",
            "schema_version": 1,
            "status": "valid",
            "input_kind": "start_spec",
            "input_path": path.display().to_string(),
            "label": start.label,
            "artifact_trust_level": start.artifact_trust_level,
            "fingerprints": start.fingerprints.as_ref().map(compact_fingerprint_report),
        })
    };
    Ok(serde_json::to_string_pretty(&payload)?)
}

fn compact_fingerprint_report(fingerprints: &StateFingerprintV1) -> serde_json::Value {
    serde_json::json!({
        "boundary": fingerprints.boundary,
        "public_observation_hash": fingerprints.public_observation_hash,
        "legal_candidate_set_hash": fingerprints.legal_candidate_set_hash,
        "legal_candidate_order_hash": fingerprints.legal_candidate_order_hash,
        "exact_state_hash": fingerprints.exact_state_hash,
        "stable_outcome_hash": fingerprints.stable_outcome_hash,
        "rng_boundary": {
            "status": fingerprints.rng_boundary.status,
            "stream_count": fingerprints.rng_boundary.stream_count,
            "digest": fingerprints.rng_boundary.digest,
        }
    })
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
