use std::fs;
use std::path::PathBuf;

use clap::Parser;
use serde::Serialize;
use std::collections::BTreeMap;

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

    #[arg(long = "prefix-command", value_name = "COMMAND")]
    prefix_commands: Vec<String>,

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct StopReasonSummaryV1 {
    stop_reason: String,
    count: usize,
    seeds: Vec<u64>,
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
    let mut rows = Vec::new();
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
            if let Some(error) = row.error.as_ref() {
                println!("  error: {error}");
            }
        }
        rows.push(row);
    }
    if !args.json_lines && rows.len() > 1 {
        print_stop_summary(&rows);
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

    apply_prefix_inputs(&mut session, args)?;

    let outcome = session.apply_command(RunControlCommand::AutoRun(
        sts_simulator::eval::run_control::RunControlAutoStepOptions {
            search: RunControlSearchCombatOptions::default(),
            max_operations: Some(args.max_operations),
            route: Default::default(),
            allow_route_reject_unless_forced: false,
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

fn apply_prefix_inputs(session: &mut RunControlSession, args: &Args) -> Result<(), String> {
    if let Some(script) = args.prefix_script.as_ref() {
        apply_prefix_script(session, script)?;
    }
    for (index, command_line) in args.prefix_commands.iter().enumerate() {
        apply_prefix_command(
            session,
            command_line,
            &format!("inline --prefix-command #{}", index + 1),
        )?;
    }
    Ok(())
}

fn apply_prefix_script(session: &mut RunControlSession, path: &PathBuf) -> Result<(), String> {
    let payload = fs::read_to_string(path).map_err(|err| err.to_string())?;
    for (line_index, line) in payload.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        apply_prefix_command(
            session,
            trimmed,
            &format!("{}:{}", path.display(), line_index + 1),
        )?;
    }
    Ok(())
}

fn apply_prefix_command(
    session: &mut RunControlSession,
    command_line: &str,
    label: &str,
) -> Result<(), String> {
    let command =
        parse_run_control_command(command_line).map_err(|err| format!("{label}: {err}"))?;
    let outcome = session
        .apply_command(command)
        .map_err(|err| format!("{label}: {err}"))?;
    if outcome.should_quit {
        return Err(format!(
            "{label}: prefix command requested quit before auto-run"
        ));
    }
    Ok(())
}

#[cfg(test)]
fn prefix_input_labels(args: &Args) -> Vec<String> {
    let mut labels = Vec::new();
    if let Some(script) = args.prefix_script.as_ref() {
        labels.push(format!("script {}", script.display()));
    }
    labels.extend(
        args.prefix_commands
            .iter()
            .enumerate()
            .map(|(index, _)| format!("inline --prefix-command #{}", index + 1)),
    );
    labels
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

fn summarize_stop_reasons(rows: &[AutoRunBatchRowV1]) -> Vec<StopReasonSummaryV1> {
    let mut grouped: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    for row in rows {
        grouped
            .entry(row.stop_reason.clone())
            .or_default()
            .push(row.seed);
    }
    let mut summaries = grouped
        .into_iter()
        .map(|(stop_reason, seeds)| StopReasonSummaryV1 {
            stop_reason,
            count: seeds.len(),
            seeds,
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.stop_reason.cmp(&right.stop_reason))
    });
    summaries
}

fn print_stop_summary(rows: &[AutoRunBatchRowV1]) {
    println!();
    println!("stop reason summary:");
    for summary in summarize_stop_reasons(rows) {
        let seed_preview = summary
            .seeds
            .iter()
            .take(5)
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let suffix = if summary.seeds.len() > 5 {
            format!(",... +{}", summary.seeds.len() - 5)
        } else {
            String::new()
        };
        println!(
            "  {} | count={} | seeds={}{}",
            summary.stop_reason, summary.count, seed_preview, suffix
        );
    }
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
            prefix_commands: vec![],
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

    #[test]
    fn prefix_inputs_apply_script_before_inline_commands() {
        let args = Args {
            seeds: vec![521],
            seed_start: None,
            count: 1,
            ascension: 0,
            player_class: "ironclad".to_string(),
            prefix_script: Some(PathBuf::from("opening.txt")),
            prefix_commands: vec!["0".to_string(), "ar".to_string()],
            search_max_nodes: None,
            search_wall_ms: None,
            max_operations: 128,
            json_lines: false,
        };

        assert_eq!(
            prefix_input_labels(&args),
            vec![
                "script opening.txt".to_string(),
                "inline --prefix-command #1".to_string(),
                "inline --prefix-command #2".to_string()
            ]
        );
    }

    #[test]
    fn stop_summary_groups_rows_by_reason() {
        let rows = vec![
            AutoRunBatchRowV1 {
                schema_name: "AutoRunBatchRowV1",
                schema_version: 1,
                seed: 1,
                status: "ok".to_string(),
                applied_operations: 2,
                stop_reason: "card reward requires human choice".to_string(),
                screen_title: "Reward Screen".to_string(),
                act: 1,
                floor: 1,
                hp: 80,
                max_hp: 80,
                gold: 99,
                error: None,
            },
            AutoRunBatchRowV1 {
                schema_name: "AutoRunBatchRowV1",
                schema_version: 1,
                seed: 2,
                status: "ok".to_string(),
                applied_operations: 0,
                stop_reason: "card reward requires human choice".to_string(),
                screen_title: "Reward Screen".to_string(),
                act: 1,
                floor: 0,
                hp: 80,
                max_hp: 80,
                gold: 99,
                error: None,
            },
            AutoRunBatchRowV1 {
                schema_name: "AutoRunBatchRowV1",
                schema_version: 1,
                seed: 3,
                status: "error".to_string(),
                applied_operations: 0,
                stop_reason: "prefix failed".to_string(),
                screen_title: "Neow Intro".to_string(),
                act: 1,
                floor: 0,
                hp: 80,
                max_hp: 80,
                gold: 99,
                error: Some("bad command".to_string()),
            },
        ];

        assert_eq!(
            summarize_stop_reasons(&rows),
            vec![
                StopReasonSummaryV1 {
                    stop_reason: "card reward requires human choice".to_string(),
                    count: 2,
                    seeds: vec![1, 2],
                },
                StopReasonSummaryV1 {
                    stop_reason: "prefix failed".to_string(),
                    count: 1,
                    seeds: vec![3],
                },
            ]
        );
    }
}
