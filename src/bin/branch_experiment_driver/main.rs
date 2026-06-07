use std::fs;
use std::path::PathBuf;

use clap::Parser;
use compact_report::{
    render_compact_report, render_compact_report_with_options, CompactReportOptions,
};
use sts_simulator::eval::branch_experiment::{
    run_branch_experiment_v1, BranchExperimentConfigV1, BranchExperimentReportV1,
};
use sts_simulator::eval::run_control::RunControlHpLossLimit;

mod compact_report;

#[derive(Debug, Parser)]
#[command(
    name = "branch_experiment_driver",
    about = "Run a small in-memory branch experiment over card reward choices"
)]
struct Args {
    #[arg(long, default_value_t = 1)]
    seed: u64,

    #[arg(long, default_value_t = 0)]
    ascension: u8,

    #[arg(long = "class", default_value = "ironclad")]
    player_class: String,

    #[arg(long)]
    final_act: bool,

    #[arg(long, default_value_t = 12)]
    max_branches: usize,

    #[arg(long)]
    max_per_frontier_group: Option<usize>,

    #[arg(long)]
    max_reward_options: Option<usize>,

    #[arg(
        long,
        default_value_t = 3,
        help = "Max campfire branch options per branch; use a larger value to inspect more smith targets"
    )]
    max_campfire_options: usize,

    #[arg(long, default_value_t = 4)]
    max_depth: usize,

    #[arg(long, default_value_t = 128)]
    auto_max_ops: usize,

    #[arg(long)]
    experiment_wall_ms: Option<u64>,

    #[arg(long)]
    search_max_nodes: Option<usize>,

    #[arg(long)]
    search_wall_ms: Option<u64>,

    #[arg(long)]
    max_hp_loss: Option<String>,

    #[arg(long = "prefix", value_name = "COMMAND")]
    prefix_commands: Vec<String>,

    #[arg(
        long = "script",
        value_name = "PATH",
        help = "Read prefix commands from a text file; blank lines and # comments are ignored"
    )]
    prefix_scripts: Vec<PathBuf>,

    #[arg(
        long,
        help = "Replay a SessionTraceV1 before starting branch exploration"
    )]
    replay_trace: Option<PathBuf>,

    #[arg(long, help = "Only replay the first N recorded trace steps")]
    replay_steps: Option<usize>,

    #[arg(long)]
    include_skip: bool,

    #[arg(long)]
    out: Option<PathBuf>,

    #[arg(
        long,
        default_value_t = 5,
        help = "Number of kept branch example lines in compact output"
    )]
    branch_examples: usize,

    #[arg(long)]
    json: bool,
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run(args) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<(), String> {
    let player_class = canonical_player_class(&args.player_class)?;
    let script_prefix_commands = load_prefix_scripts(&args.prefix_scripts)?;
    let prefix_commands = merge_prefix_commands(script_prefix_commands, args.prefix_commands);
    let report = run_branch_experiment_v1(&BranchExperimentConfigV1 {
        seed: args.seed,
        ascension_level: args.ascension,
        player_class,
        final_act: args.final_act,
        max_branches: args.max_branches,
        max_branches_per_frontier_group: args.max_per_frontier_group,
        max_reward_options_per_branch: args.max_reward_options,
        max_campfire_options_per_branch: Some(args.max_campfire_options),
        max_depth: args.max_depth,
        auto_max_operations: args.auto_max_ops,
        experiment_wall_ms: args.experiment_wall_ms,
        search_max_nodes: args.search_max_nodes,
        search_wall_ms: args.search_wall_ms.or(Some(100)),
        search_max_hp_loss: parse_hp_loss_limit(args.max_hp_loss.as_deref())?,
        include_skip: args.include_skip,
        prefix_commands,
        replay_trace_path: args.replay_trace,
        replay_trace_max_steps: args.replay_steps,
    })?;
    let compact_options = CompactReportOptions {
        kept_branch_examples: args.branch_examples,
    };
    if let Some(path) = args.out {
        let payload = serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "failed to create output directory {}: {err}",
                    parent.display()
                )
            })?;
        }
        fs::write(&path, payload)
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        println!("{}", render_report(&report, compact_options));
        println!("full JSON written: {}", path.display());
    } else if args.json {
        let payload = serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?;
        println!("{payload}");
    } else {
        println!("{}", render_report(&report, compact_options));
    }
    Ok(())
}

fn render_report(report: &BranchExperimentReportV1, options: CompactReportOptions) -> String {
    if options == CompactReportOptions::default() {
        render_compact_report(report)
    } else {
        render_compact_report_with_options(report, options)
    }
}

fn parse_hp_loss_limit(value: Option<&str>) -> Result<Option<RunControlHpLossLimit>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.eq_ignore_ascii_case("off") || value.eq_ignore_ascii_case("unlimited") {
        return Ok(Some(RunControlHpLossLimit::Unlimited));
    }
    let limit = value
        .parse::<u32>()
        .map_err(|err| format!("invalid --max-hp-loss {value}: {err}"))?;
    Ok(Some(RunControlHpLossLimit::Limit(limit)))
}

fn load_prefix_scripts(paths: &[PathBuf]) -> Result<Vec<String>, String> {
    let mut commands = Vec::new();
    for path in paths {
        let content = fs::read_to_string(path)
            .map_err(|err| format!("failed to read prefix script {}: {err}", path.display()))?;
        commands.extend(parse_prefix_script(&content));
    }
    Ok(commands)
}

fn parse_prefix_script(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect()
}

fn merge_prefix_commands(
    script_prefix_commands: Vec<String>,
    inline_prefix_commands: Vec<String>,
) -> Vec<String> {
    script_prefix_commands
        .into_iter()
        .chain(inline_prefix_commands)
        .collect()
}

fn canonical_player_class(value: &str) -> Result<&'static str, String> {
    match value.to_ascii_lowercase().as_str() {
        "ironclad" => Ok("Ironclad"),
        "silent" => Ok("Silent"),
        "defect" => Ok("Defect"),
        "watcher" => Ok("Watcher"),
        other => Err(format!(
            "unsupported class '{other}', expected ironclad|silent|defect|watcher"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unlimited_hp_loss_limit() {
        assert_eq!(
            parse_hp_loss_limit(Some("off")).expect("hp loss parses"),
            Some(RunControlHpLossLimit::Unlimited)
        );
    }

    #[test]
    fn canonicalizes_player_class() {
        assert_eq!(
            canonical_player_class("ironclad").expect("class parses"),
            "Ironclad"
        );
    }

    #[test]
    fn prefix_script_ignores_blank_lines_and_comments() {
        let commands = parse_prefix_script(
            r#"
            # Start from Neow.
            0

            2
              # Choose the visible map path.
            go 5
            "#,
        );

        assert_eq!(commands, vec!["0", "2", "go 5"]);
    }

    #[test]
    fn prefix_script_commands_precede_inline_prefix_commands() {
        let commands = merge_prefix_commands(
            vec!["0".to_string(), "2".to_string()],
            vec!["go 5".to_string()],
        );

        assert_eq!(commands, vec!["0", "2", "go 5"]);
    }
}
