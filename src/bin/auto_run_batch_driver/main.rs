use std::fs;
use std::path::PathBuf;

use clap::Parser;
use serde::Serialize;

use sts_simulator::eval::run_control::{
    build_decision_surface, canonical_player_class, parse_run_control_command, RunControlCommand,
    RunControlConfig, RunControlSearchCombatOptions, RunControlSession,
};

#[derive(Debug, Parser)]
#[command(
    name = "auto_run_batch_driver",
    about = "Batch-smoke run_control auto-run to the next human boundary"
)]
struct Args {
    #[arg(long = "seed", value_name = "SEED")]
    seeds: Vec<u64>,

    #[arg(long, value_name = "SEED")]
    seed_start: Option<u64>,

    #[arg(long, value_name = "N", default_value_t = 1)]
    count: usize,

    #[arg(long, default_value_t = 0)]
    ascension: u8,

    #[arg(long = "class", default_value = "ironclad")]
    player_class: String,

    #[arg(long, value_name = "PATH")]
    prefix_script: Option<PathBuf>,

    #[arg(long, value_name = "N")]
    search_max_nodes: Option<usize>,

    #[arg(long, value_name = "MS")]
    search_wall_ms: Option<u64>,

    #[arg(long, value_name = "N", default_value_t = 128)]
    max_operations: usize,

    #[arg(long)]
    json_lines: bool,
}

#[derive(Debug, Serialize)]
struct AutoRunBatchRowV1 {
    schema_name: &'static str,
    schema_version: u32,
    seed: u64,
    status: String,
    applied_operations: usize,
    stop_reason: String,
    screen_title: String,
    act: u8,
    floor: i32,
    hp: i32,
    max_hp: i32,
    gold: i32,
    error: Option<String>,
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run(args) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<(), String> {
    let seeds = expand_seeds(&args)?;
    for seed in seeds {
        let row = run_seed(&args, seed);
        if args.json_lines {
            println!(
                "{}",
                serde_json::to_string(&row).map_err(|err| err.to_string())?
            );
        } else {
            println!(
                "seed={} status={} act={} floor={} hp={}/{} gold={} screen={} applied={} reason={}",
                row.seed,
                row.status,
                row.act,
                row.floor,
                row.hp,
                row.max_hp,
                row.gold,
                row.screen_title,
                row.applied_operations,
                row.stop_reason
            );
            if let Some(error) = row.error {
                println!("  error: {error}");
            }
        }
    }
    Ok(())
}

fn run_seed(args: &Args, seed: u64) -> AutoRunBatchRowV1 {
    match run_seed_inner(args, seed) {
        Ok(row) => row,
        Err(err) => {
            let class = canonical_player_class(&args.player_class).unwrap_or("Ironclad");
            let session = RunControlSession::new(RunControlConfig {
                seed,
                ascension_level: args.ascension,
                player_class: class,
                search_max_nodes: args.search_max_nodes,
                search_wall_ms: args.search_wall_ms,
                ..RunControlConfig::default()
            });
            row_from_session(
                seed,
                &session,
                "error".to_string(),
                0,
                err.clone(),
                Some(err),
            )
        }
    }
}

fn run_seed_inner(args: &Args, seed: u64) -> Result<AutoRunBatchRowV1, String> {
    let player_class = canonical_player_class(&args.player_class)?;
    let mut session = RunControlSession::new(RunControlConfig {
        seed,
        ascension_level: args.ascension,
        player_class,
        search_max_nodes: args.search_max_nodes,
        search_wall_ms: args.search_wall_ms,
        ..RunControlConfig::default()
    });

    if let Some(script) = args.prefix_script.as_ref() {
        apply_prefix_script(&mut session, script)?;
    }

    let outcome = session.apply_command(RunControlCommand::AutoRun(
        sts_simulator::eval::run_control::RunControlAutoStepOptions {
            search: RunControlSearchCombatOptions::default(),
            max_operations: Some(args.max_operations),
            route: Default::default(),
        },
    ))?;
    let reason = extract_stop_reason(&outcome.message)
        .unwrap_or_else(|| "auto-run returned without a rendered stop reason".to_string());
    let applied = extract_applied_operations(&outcome.message);
    Ok(row_from_session(
        seed,
        &session,
        "ok".to_string(),
        applied,
        reason,
        None,
    ))
}

fn apply_prefix_script(session: &mut RunControlSession, path: &PathBuf) -> Result<(), String> {
    let payload = fs::read_to_string(path).map_err(|err| err.to_string())?;
    for (line_index, line) in payload.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let command = parse_run_control_command(trimmed)
            .map_err(|err| format!("{}:{}: {err}", path.display(), line_index + 1))?;
        let outcome = session
            .apply_command(command)
            .map_err(|err| format!("{}:{}: {err}", path.display(), line_index + 1))?;
        if outcome.should_quit {
            return Err(format!(
                "{}:{}: prefix script requested quit before auto-run",
                path.display(),
                line_index + 1
            ));
        }
    }
    Ok(())
}

fn row_from_session(
    seed: u64,
    session: &RunControlSession,
    status: String,
    applied_operations: usize,
    stop_reason: String,
    error: Option<String>,
) -> AutoRunBatchRowV1 {
    let surface = build_decision_surface(session);
    AutoRunBatchRowV1 {
        schema_name: "AutoRunBatchRowV1",
        schema_version: 1,
        seed,
        status,
        applied_operations,
        stop_reason,
        screen_title: surface.view.header.title,
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        hp: session.run_state.current_hp,
        max_hp: session.run_state.max_hp,
        gold: session.run_state.gold,
        error,
    }
}

fn expand_seeds(args: &Args) -> Result<Vec<u64>, String> {
    let mut seeds = args.seeds.clone();
    if let Some(seed_start) = args.seed_start {
        if args.count == 0 {
            return Err("--count must be positive when --seed-start is used".to_string());
        }
        seeds.extend((0..args.count).map(|offset| seed_start.saturating_add(offset as u64)));
    }
    if seeds.is_empty() {
        return Err("provide at least one --seed or --seed-start".to_string());
    }
    Ok(seeds)
}

fn extract_stop_reason(message: &str) -> Option<String> {
    message
        .lines()
        .find_map(|line| line.strip_prefix("Reason: ").map(str::to_string))
}

fn extract_applied_operations(message: &str) -> usize {
    message
        .lines()
        .find_map(|line| {
            let (_before, after) = line.split_once("applied_operations=")?;
            after
                .split_whitespace()
                .next()
                .and_then(|raw| raw.parse::<usize>().ok())
        })
        .unwrap_or_else(|| count_applied_bullets(message))
}

fn count_applied_bullets(message: &str) -> usize {
    let mut in_applied = false;
    let mut count = 0usize;
    for line in message.lines() {
        if line == "Applied:" {
            in_applied = true;
            continue;
        }
        if line.starts_with("Reason: ") {
            break;
        }
        if in_applied && line.starts_with("  - ") {
            count = count.saturating_add(1);
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_seeds_supports_explicit_and_contiguous_ranges() {
        let args = Args {
            seeds: vec![521],
            seed_start: Some(100),
            count: 3,
            ascension: 0,
            player_class: "ironclad".to_string(),
            prefix_script: None,
            search_max_nodes: None,
            search_wall_ms: None,
            max_operations: 128,
            json_lines: false,
        };

        assert_eq!(expand_seeds(&args).unwrap(), vec![521, 100, 101, 102]);
    }

    #[test]
    fn extract_applied_operations_prefers_auto_run_header() {
        let message = "Auto-run stopped: Reward Screen\nroute=planner max_operations=128 applied_operations=6\nApplied:\n  - one\nReason: stop";

        assert_eq!(extract_applied_operations(message), 6);
    }

    #[test]
    fn extract_stop_reason_reads_rendered_reason_line() {
        let message = "Applied:\n  none\nReason: card reward requires human choice\nNext: pick";

        assert_eq!(
            extract_stop_reason(message),
            Some("card reward requires human choice".to_string())
        );
    }
}
