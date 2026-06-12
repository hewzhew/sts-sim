use clap::parser::ValueSource;
use clap::{CommandFactory, FromArgMatches, Parser, ValueEnum};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

mod inspect_summary;

use sts_simulator::eval::branch_campaign::{
    render_branch_campaign_compact_v1, render_branch_campaign_progress_event_v1,
    run_branch_campaign_from_report_with_checkpoint_and_progress_v1,
    run_branch_campaign_from_report_with_checkpoint_v1,
    run_branch_campaign_with_checkpoint_and_progress_v1, run_branch_campaign_with_checkpoint_v1,
    BranchCampaignCheckpointV1, BranchCampaignCombatRetryPolicyV1, BranchCampaignConfigV1,
    BranchCampaignReportV1,
};
use sts_simulator::eval::branch_experiment_retention::BranchRetentionBudgetProfileV1;
use sts_simulator::eval::branch_experiment_search_options::parse_branch_experiment_search_options_v1;
use sts_simulator::eval::neow_guided_prefix::{
    neow_guided_prefix_commands_v1, NeowGuidedPrefixConfigV1,
};
use sts_simulator::eval::run_control::{canonical_player_class, RunControlHpLossLimit};
use sts_simulator::eval::run_control::{
    render_run_control_details, render_run_control_state, RunControlCombatSegmentMode,
    RunControlCommand, RunControlSearchCombatOptions, RunControlSession,
};

const QUICK_PRESET_MAX_ROUNDS: usize = 2;
const QUICK_PRESET_ROUND_DEPTH: usize = 2;
const QUICK_PRESET_MAX_ACTIVE: usize = 2;
const QUICK_PRESET_MAX_FROZEN: usize = 16;
const QUICK_PRESET_MAX_BRANCHES_PER_ACTIVE: usize = 8;
const QUICK_PRESET_EXPERIMENT_WALL_MS: u64 = 5_000;
const QUICK_PRESET_SEARCH_WALL_MS: u64 = 300;
const QUICK_PRESET_SEARCH_MAX_NODES: usize = 50_000;
const QUICK_PRESET_BRANCH_EXAMPLES: usize = 3;

const FOCUSED_PRESET_MAX_ROUNDS: usize = 6;
const FOCUSED_PRESET_ROUND_DEPTH: usize = 2;
const FOCUSED_PRESET_MAX_ACTIVE: usize = 2;
const FOCUSED_PRESET_MAX_FROZEN: usize = 16;
const FOCUSED_PRESET_MAX_BRANCHES_PER_ACTIVE: usize = 8;
const FOCUSED_PRESET_EXPERIMENT_WALL_MS: u64 = 10_000;
const FOCUSED_PRESET_SEARCH_WALL_MS: u64 = 300;
const FOCUSED_PRESET_SEARCH_MAX_NODES: usize = 50_000;
const FOCUSED_PRESET_BRANCH_EXAMPLES: usize = 4;

const DEEP_PRESET_MAX_ROUNDS: usize = 10;
const DEEP_PRESET_ROUND_DEPTH: usize = 2;
const DEEP_PRESET_MAX_ACTIVE: usize = 2;
const DEEP_PRESET_MAX_FROZEN: usize = 16;
const DEEP_PRESET_MAX_BRANCHES_PER_ACTIVE: usize = 8;
const DEEP_PRESET_EXPERIMENT_WALL_MS: u64 = 30_000;
const DEEP_PRESET_SEARCH_WALL_MS: u64 = 1_000;
const DEEP_PRESET_SEARCH_MAX_NODES: usize = 200_000;
const DEEP_PRESET_BRANCH_EXAMPLES: usize = 6;

#[derive(Debug, Parser)]
#[command(
    name = "branch_campaign_driver",
    about = "Advance a small campaign of noncombat branches until victory, budget, or strategy boundary"
)]
struct Args {
    #[arg(long, value_enum)]
    preset: Option<BranchCampaignPresetV1>,

    #[arg(long, default_value_t = 1)]
    seed: u64,

    #[arg(long, default_value_t = 0)]
    ascension: u8,

    #[arg(long = "class", default_value = "ironclad")]
    player_class: String,

    #[arg(long)]
    final_act: bool,

    #[arg(long, default_value_t = 8)]
    max_rounds: usize,

    #[arg(long, default_value_t = 1)]
    round_depth: usize,

    #[arg(long, default_value_t = 8)]
    max_active: usize,

    #[arg(long, default_value_t = 32)]
    max_frozen: usize,

    #[arg(long, default_value_t = 12)]
    max_branches_per_active: usize,

    #[arg(long, default_value = "package")]
    retention_profile: String,

    #[arg(long)]
    max_reward_options: Option<usize>,

    #[arg(long)]
    all_reward_options: bool,

    #[arg(long, default_value_t = 3)]
    max_campfire_options: usize,

    #[arg(long, default_value_t = 128)]
    auto_max_ops: usize,

    #[arg(long, default_value_t = 10_000)]
    experiment_wall_ms: u64,

    #[arg(long)]
    search_max_nodes: Option<usize>,

    #[arg(long, default_value_t = 200)]
    search_wall_ms: u64,

    #[arg(long)]
    max_hp_loss: Option<String>,

    #[arg(
        long = "combat-search-option",
        value_name = "KEY=VALUE",
        help = "Additional run_control search-combat option forwarded to branch experiments"
    )]
    combat_search_options: Vec<String>,

    #[arg(long, value_enum, default_value_t = BranchCampaignCombatRetryArgV1::OnStall)]
    combat_retry: BranchCampaignCombatRetryArgV1,

    #[arg(long, default_value_t = 20)]
    min_acceptable_victory_hp_percent: u8,

    #[arg(long = "prefix", value_name = "COMMAND")]
    prefix_commands: Vec<String>,

    #[arg(long)]
    no_neow_guidance: bool,

    #[arg(long, default_value_t = 4)]
    branch_examples: usize,

    #[arg(long)]
    json: bool,

    #[arg(long, help = "Print coarse campaign progress to stderr while running")]
    progress: bool,

    #[arg(
        long,
        value_name = "PATH",
        help = "Resume from a previous BranchCampaignV1 JSON report"
    )]
    resume: Option<PathBuf>,

    #[arg(
        long = "resume-checkpoint",
        value_name = "PATH",
        help = "Resume exact branch sessions from a BranchCampaignCheckpointV1 sidecar"
    )]
    resume_checkpoint: Option<PathBuf>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Write the resulting BranchCampaignV1 JSON report"
    )]
    out: Option<PathBuf>,

    #[arg(
        long = "checkpoint-out",
        value_name = "PATH",
        help = "Write the resulting BranchCampaignCheckpointV1 exact session sidecar"
    )]
    checkpoint_out: Option<PathBuf>,

    #[arg(
        long = "inspect-checkpoint",
        value_name = "PATH",
        help = "Inspect a saved BranchCampaignCheckpointV1 session instead of running a campaign"
    )]
    inspect_checkpoint: Option<PathBuf>,

    #[arg(
        long = "inspect-report",
        value_name = "PATH",
        help = "Pair --inspect-checkpoint with a BranchCampaignV1 report for active/frozen/abandoned labels"
    )]
    inspect_report: Option<PathBuf>,

    #[arg(
        long = "inspect-summary",
        help = "Print compact deck/resource/strategy summaries for checkpoint sessions"
    )]
    inspect_summary: bool,

    #[arg(
        long = "inspect-act",
        help = "Filter inspected checkpoint sessions by act"
    )]
    inspect_act: Option<u8>,

    #[arg(
        long = "inspect-floor",
        help = "Filter inspected checkpoint sessions by floor"
    )]
    inspect_floor: Option<i32>,

    #[arg(
        long = "inspect-hp",
        help = "Filter inspected checkpoint sessions by current HP"
    )]
    inspect_hp: Option<i32>,

    #[arg(
        long = "inspect-index",
        default_value_t = 0,
        help = "Select the Nth matching checkpoint session after filters"
    )]
    inspect_index: usize,

    #[arg(
        long = "inspect-search",
        help = "Run search-combat from the selected checkpoint session and print the result"
    )]
    inspect_search: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum BranchCampaignPresetV1 {
    Quick,
    Focused,
    Deep,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum BranchCampaignCombatRetryArgV1 {
    OnStall,
    Immediate,
    Disabled,
}

fn main() {
    let args = parse_args();
    if let Err(err) = run(args) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn parse_args() -> Args {
    parse_args_from(std::env::args_os()).unwrap_or_else(|err| err.exit())
}

fn parse_args_from<I, T>(itr: I) -> Result<Args, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let matches = Args::command().try_get_matches_from(itr)?;
    let mut args = Args::from_arg_matches(&matches)?;
    apply_preset_defaults(&mut args, |name| {
        matches.value_source(name) == Some(ValueSource::CommandLine)
    });
    Ok(args)
}

fn apply_preset_defaults<F>(args: &mut Args, was_explicit: F)
where
    F: Fn(&'static str) -> bool,
{
    match args.preset {
        Some(BranchCampaignPresetV1::Quick) => apply_quick_preset_defaults(args, was_explicit),
        Some(BranchCampaignPresetV1::Focused) => apply_focused_preset_defaults(args, was_explicit),
        Some(BranchCampaignPresetV1::Deep) => apply_deep_preset_defaults(args, was_explicit),
        None => {}
    }
}

fn apply_quick_preset_defaults<F>(args: &mut Args, was_explicit: F)
where
    F: Fn(&'static str) -> bool,
{
    apply_campaign_preset_defaults(
        args,
        was_explicit,
        CampaignPresetDefaults {
            max_rounds: QUICK_PRESET_MAX_ROUNDS,
            round_depth: QUICK_PRESET_ROUND_DEPTH,
            max_active: QUICK_PRESET_MAX_ACTIVE,
            max_frozen: QUICK_PRESET_MAX_FROZEN,
            max_branches_per_active: QUICK_PRESET_MAX_BRANCHES_PER_ACTIVE,
            experiment_wall_ms: QUICK_PRESET_EXPERIMENT_WALL_MS,
            search_wall_ms: QUICK_PRESET_SEARCH_WALL_MS,
            search_max_nodes: QUICK_PRESET_SEARCH_MAX_NODES,
            branch_examples: QUICK_PRESET_BRANCH_EXAMPLES,
        },
    );
}

fn apply_focused_preset_defaults<F>(args: &mut Args, was_explicit: F)
where
    F: Fn(&'static str) -> bool,
{
    apply_campaign_preset_defaults(
        args,
        was_explicit,
        CampaignPresetDefaults {
            max_rounds: FOCUSED_PRESET_MAX_ROUNDS,
            round_depth: FOCUSED_PRESET_ROUND_DEPTH,
            max_active: FOCUSED_PRESET_MAX_ACTIVE,
            max_frozen: FOCUSED_PRESET_MAX_FROZEN,
            max_branches_per_active: FOCUSED_PRESET_MAX_BRANCHES_PER_ACTIVE,
            experiment_wall_ms: FOCUSED_PRESET_EXPERIMENT_WALL_MS,
            search_wall_ms: FOCUSED_PRESET_SEARCH_WALL_MS,
            search_max_nodes: FOCUSED_PRESET_SEARCH_MAX_NODES,
            branch_examples: FOCUSED_PRESET_BRANCH_EXAMPLES,
        },
    );
}

fn apply_deep_preset_defaults<F>(args: &mut Args, was_explicit: F)
where
    F: Fn(&'static str) -> bool,
{
    apply_campaign_preset_defaults(
        args,
        was_explicit,
        CampaignPresetDefaults {
            max_rounds: DEEP_PRESET_MAX_ROUNDS,
            round_depth: DEEP_PRESET_ROUND_DEPTH,
            max_active: DEEP_PRESET_MAX_ACTIVE,
            max_frozen: DEEP_PRESET_MAX_FROZEN,
            max_branches_per_active: DEEP_PRESET_MAX_BRANCHES_PER_ACTIVE,
            experiment_wall_ms: DEEP_PRESET_EXPERIMENT_WALL_MS,
            search_wall_ms: DEEP_PRESET_SEARCH_WALL_MS,
            search_max_nodes: DEEP_PRESET_SEARCH_MAX_NODES,
            branch_examples: DEEP_PRESET_BRANCH_EXAMPLES,
        },
    );
}

#[derive(Clone, Copy, Debug)]
struct CampaignPresetDefaults {
    max_rounds: usize,
    round_depth: usize,
    max_active: usize,
    max_frozen: usize,
    max_branches_per_active: usize,
    experiment_wall_ms: u64,
    search_wall_ms: u64,
    search_max_nodes: usize,
    branch_examples: usize,
}

fn apply_campaign_preset_defaults<F>(
    args: &mut Args,
    was_explicit: F,
    defaults: CampaignPresetDefaults,
) where
    F: Fn(&'static str) -> bool,
{
    if !was_explicit("max_rounds") {
        args.max_rounds = defaults.max_rounds;
    }
    if !was_explicit("round_depth") {
        args.round_depth = defaults.round_depth;
    }
    if !was_explicit("max_active") {
        args.max_active = defaults.max_active;
    }
    if !was_explicit("max_frozen") {
        args.max_frozen = defaults.max_frozen;
    }
    if !was_explicit("max_branches_per_active") {
        args.max_branches_per_active = defaults.max_branches_per_active;
    }
    if !was_explicit("experiment_wall_ms") {
        args.experiment_wall_ms = defaults.experiment_wall_ms;
    }
    if !was_explicit("search_wall_ms") {
        args.search_wall_ms = defaults.search_wall_ms;
    }
    if !was_explicit("search_max_nodes") {
        args.search_max_nodes = Some(defaults.search_max_nodes);
    }
    if !was_explicit("branch_examples") {
        args.branch_examples = defaults.branch_examples;
    }
}

fn run(args: Args) -> Result<(), String> {
    if args.inspect_checkpoint.is_some() {
        return run_checkpoint_inspection(&args);
    }
    let config = campaign_config_from_args(&args)?;
    if args.resume_checkpoint.is_some() && args.resume.is_none() {
        return Err("--resume-checkpoint requires --resume".to_string());
    }
    let previous = args
        .resume
        .as_ref()
        .map(read_campaign_report_v1)
        .transpose()?;
    let checkpoint = args
        .resume_checkpoint
        .as_ref()
        .map(read_campaign_checkpoint_v1)
        .transpose()?;
    let result = if args.progress && !args.json {
        let started_at = Instant::now();
        let progress = |event| {
            println!(
                "[{:>4}s] {}",
                started_at.elapsed().as_secs(),
                render_branch_campaign_progress_event_v1(&event)
            );
        };
        if let Some(previous) = previous.as_ref() {
            run_branch_campaign_from_report_with_checkpoint_and_progress_v1(
                &config,
                previous,
                checkpoint.as_ref(),
                progress,
            )?
        } else {
            run_branch_campaign_with_checkpoint_and_progress_v1(&config, progress)?
        }
    } else if let Some(previous) = previous.as_ref() {
        run_branch_campaign_from_report_with_checkpoint_v1(&config, previous, checkpoint.as_ref())?
    } else {
        run_branch_campaign_with_checkpoint_v1(&config)?
    };
    let report = result.report;
    if let Some(path) = args.out.as_ref() {
        write_campaign_report_v1(path, &report)?;
    }
    if let Some(path) = args.checkpoint_out.as_ref() {
        write_campaign_checkpoint_v1(path, &result.checkpoint)?;
    }
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?
        );
    } else {
        println!(
            "{}",
            render_branch_campaign_compact_v1(&report, args.branch_examples)
        );
    }
    Ok(())
}

fn run_checkpoint_inspection(args: &Args) -> Result<(), String> {
    let path = args
        .inspect_checkpoint
        .as_ref()
        .ok_or_else(|| "--inspect-checkpoint requires a path".to_string())?;
    let checkpoint = read_campaign_checkpoint_v1(path)?;
    let report = args
        .inspect_report
        .as_ref()
        .map(read_campaign_report_v1)
        .transpose()?;
    let mut matches = Vec::new();
    for entry in checkpoint.sessions {
        let session = entry
            .session
            .clone()
            .into_session()
            .map_err(|err| format!("failed to restore checkpoint session: {err}"))?;
        if !checkpoint_session_matches_filters(args, &session) {
            continue;
        }
        matches.push((entry.commands, session));
    }
    if matches.is_empty() {
        return Err(format!(
            "no checkpoint sessions matched filters act={:?} floor={:?} hp={:?}",
            args.inspect_act, args.inspect_floor, args.inspect_hp
        ));
    }
    if args.inspect_summary {
        println!(
            "{}",
            inspect_summary::render_checkpoint_inspect_summary_v1(
                checkpoint.seed,
                &matches,
                report.as_ref(),
                args.branch_examples,
            )
        );
        return Ok(());
    }
    if args.inspect_index >= matches.len() {
        return Err(format!(
            "--inspect-index {} is out of range for {} matching checkpoint session(s)",
            args.inspect_index,
            matches.len()
        ));
    }

    let match_count = matches.len();
    let (commands, mut session) = matches.swap_remove(args.inspect_index);
    let (hp, max_hp) = inspect_visible_player_hp(&session);
    println!(
        "Checkpoint inspection: seed={} match={}/{} act={} floor={} hp={}/{} engine={:?}",
        checkpoint.seed,
        args.inspect_index + 1,
        match_count,
        session.run_state.act_num,
        session.run_state.floor_num,
        hp,
        max_hp,
        session.engine_state
    );
    println!("commands: {}", render_inspect_command_path(&commands));
    if args.inspect_search {
        let options = inspect_search_options_from_args(args)?;
        let outcome = session.apply_command(RunControlCommand::SearchCombat(options))?;
        println!("{}", outcome.message);
    } else {
        println!("{}", render_run_control_details(&session));
        println!();
        println!("{}", render_run_control_state(&session));
    }
    Ok(())
}

fn checkpoint_session_matches_filters(args: &Args, session: &RunControlSession) -> bool {
    if args
        .inspect_act
        .is_some_and(|act| session.run_state.act_num != act)
    {
        return false;
    }
    if args
        .inspect_floor
        .is_some_and(|floor| session.run_state.floor_num != floor)
    {
        return false;
    }
    if args
        .inspect_hp
        .is_some_and(|hp| inspect_visible_player_hp(session).0 != hp)
    {
        return false;
    }
    true
}

fn inspect_visible_player_hp(session: &RunControlSession) -> (i32, i32) {
    session
        .active_combat
        .as_ref()
        .map(|active| {
            (
                active.combat_state.entities.player.current_hp,
                active.combat_state.entities.player.max_hp,
            )
        })
        .unwrap_or((session.run_state.current_hp, session.run_state.max_hp))
}

fn inspect_search_options_from_args(args: &Args) -> Result<RunControlSearchCombatOptions, String> {
    let mut options = parse_branch_experiment_search_options_v1(&args.combat_search_options)?;
    options.max_nodes = args.search_max_nodes.or(options.max_nodes);
    options.wall_ms = Some(args.search_wall_ms).or(options.wall_ms);
    options.max_hp_loss = parse_hp_loss_limit(args.max_hp_loss.as_deref())?.or(options.max_hp_loss);
    Ok(options)
}

fn render_inspect_command_path(commands: &[String]) -> String {
    const HEAD: usize = 4;
    const TAIL: usize = 6;
    if commands.is_empty() {
        return "-".to_string();
    }
    if commands.len() <= HEAD + TAIL + 1 {
        return commands.join(" -> ");
    }
    let mut parts = Vec::new();
    parts.extend(commands.iter().take(HEAD).cloned());
    parts.push(format!("... {} more ...", commands.len() - HEAD - TAIL));
    parts.extend(commands.iter().skip(commands.len() - TAIL).cloned());
    parts.join(" -> ")
}

fn read_campaign_report_v1(path: &PathBuf) -> Result<BranchCampaignReportV1, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read --resume {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse --resume {} as BranchCampaignV1: {err}",
            path.display()
        )
    })
}

fn read_campaign_checkpoint_v1(path: &PathBuf) -> Result<BranchCampaignCheckpointV1, String> {
    let text = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read --resume-checkpoint {}: {err}",
            path.display()
        )
    })?;
    serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse --resume-checkpoint {} as BranchCampaignCheckpointV1: {err}",
            path.display()
        )
    })
}

fn write_campaign_report_v1(path: &PathBuf, report: &BranchCampaignReportV1) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create --out directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(report)
        .map_err(|err| format!("failed to serialize BranchCampaignV1 report: {err}"))?;
    fs::write(path, text).map_err(|err| format!("failed to write --out {}: {err}", path.display()))
}

fn write_campaign_checkpoint_v1(
    path: &PathBuf,
    checkpoint: &BranchCampaignCheckpointV1,
) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create --checkpoint-out directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(checkpoint)
        .map_err(|err| format!("failed to serialize BranchCampaignCheckpointV1: {err}"))?;
    fs::write(path, text)
        .map_err(|err| format!("failed to write --checkpoint-out {}: {err}", path.display()))
}

fn campaign_config_from_args(args: &Args) -> Result<BranchCampaignConfigV1, String> {
    let player_class = canonical_player_class(&args.player_class)?;
    let mut prefix_commands = Vec::new();
    if !args.no_neow_guidance {
        prefix_commands.extend(neow_guided_prefix_commands_v1(&NeowGuidedPrefixConfigV1 {
            seed: args.seed,
            ascension_level: args.ascension,
            final_act: args.final_act,
            player_class,
            search_max_nodes: args.search_max_nodes,
            search_wall_ms: Some(args.search_wall_ms),
        })?);
    } else {
        prefix_commands.push("0".to_string());
    }
    prefix_commands.extend(args.prefix_commands.iter().cloned());

    let search_max_hp_loss = parse_hp_loss_limit(args.max_hp_loss.as_deref())?
        .or(Some(RunControlHpLossLimit::Unlimited));

    Ok(BranchCampaignConfigV1 {
        seed: args.seed,
        ascension_level: args.ascension,
        player_class,
        final_act: args.final_act,
        max_rounds: args.max_rounds,
        round_depth: args.round_depth,
        max_active: args.max_active,
        max_frozen: args.max_frozen,
        max_branches_per_active: args.max_branches_per_active,
        retention_budget_profile: args
            .retention_profile
            .parse::<BranchRetentionBudgetProfileV1>()?,
        max_reward_options_per_branch: if args.all_reward_options {
            None
        } else {
            Some(args.max_reward_options.unwrap_or(2))
        },
        max_campfire_options_per_branch: args.max_campfire_options,
        auto_max_operations: args.auto_max_ops,
        experiment_wall_ms: Some(args.experiment_wall_ms),
        search_max_nodes: args.search_max_nodes,
        search_wall_ms: Some(args.search_wall_ms),
        search_max_hp_loss,
        search_options: campaign_search_options_from_args(args)?,
        combat_retry_policy: match args.combat_retry {
            BranchCampaignCombatRetryArgV1::OnStall => BranchCampaignCombatRetryPolicyV1::OnStall,
            BranchCampaignCombatRetryArgV1::Immediate => {
                BranchCampaignCombatRetryPolicyV1::Immediate
            }
            BranchCampaignCombatRetryArgV1::Disabled => BranchCampaignCombatRetryPolicyV1::Disabled,
        },
        include_event_reward_skip: false,
        min_acceptable_victory_hp_percent: args.min_acceptable_victory_hp_percent,
        prefix_commands,
    })
}

fn campaign_search_options_from_args(args: &Args) -> Result<RunControlSearchCombatOptions, String> {
    let mut options = parse_branch_experiment_search_options_v1(&args.combat_search_options)?;
    if !combat_search_options_include_segment_mode(&args.combat_search_options) {
        options.segment_mode = Some(RunControlCombatSegmentMode::NonBossTurnBoundary);
    }
    Ok(options)
}

fn combat_search_options_include_segment_mode(tokens: &[String]) -> bool {
    tokens.iter().any(|token| {
        token.split_once('=').is_some_and(|(key, _)| {
            matches!(
                key.to_ascii_lowercase().as_str(),
                "segment" | "segment_mode" | "partial" | "partial_mode"
            )
        })
    })
}

fn parse_hp_loss_limit(value: Option<&str>) -> Result<Option<RunControlHpLossLimit>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    match value.to_ascii_lowercase().as_str() {
        "off" | "none" | "unlimited" | "no_limit" | "no-limit" => {
            Ok(Some(RunControlHpLossLimit::Unlimited))
        }
        _ => value
            .parse::<u32>()
            .map(RunControlHpLossLimit::Limit)
            .map(Some)
            .map_err(|err| format!("invalid --max-hp-loss `{value}`: {err}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campaign_cli_defaults_to_bounded_reward_branching() {
        let args = Args::try_parse_from(["branch_campaign_driver"]).expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_reward_options_per_branch, Some(2));
        assert_eq!(
            config.search_options.segment_mode,
            Some(RunControlCombatSegmentMode::NonBossTurnBoundary)
        );
        assert_eq!(config.max_active, 8);
        assert_eq!(config.max_frozen, 32);
        assert_eq!(config.round_depth, 1);
    }

    #[test]
    fn campaign_cli_can_disable_segment_combat_fallback() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--combat-search-option",
            "segment=off",
        ])
        .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.search_options.segment_mode, None);
    }

    #[test]
    fn campaign_cli_accepts_resume_and_out_paths() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--resume",
            "old.campaign.json",
            "--resume-checkpoint",
            "old.checkpoint.json",
            "--out",
            "new.campaign.json",
            "--checkpoint-out",
            "new.checkpoint.json",
        ])
        .expect("args parse");

        assert_eq!(args.resume, Some(PathBuf::from("old.campaign.json")));
        assert_eq!(
            args.resume_checkpoint,
            Some(PathBuf::from("old.checkpoint.json"))
        );
        assert_eq!(args.out, Some(PathBuf::from("new.campaign.json")));
        assert_eq!(
            args.checkpoint_out,
            Some(PathBuf::from("new.checkpoint.json"))
        );
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_summary_inspection_paths() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-summary",
            "--branch-examples",
            "2",
        ])
        .expect("args parse");

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert!(args.inspect_summary);
        assert_eq!(args.branch_examples, 2);
    }

    #[test]
    fn campaign_cli_can_branch_all_reward_options() {
        let args = Args::try_parse_from(["branch_campaign_driver", "--all-reward-options"])
            .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_reward_options_per_branch, None);
    }

    #[test]
    fn focused_preset_uses_deeper_fewer_active_branches() {
        let args =
            parse_args_from(["branch_campaign_driver", "--preset", "focused"]).expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_rounds, 6);
        assert_eq!(config.round_depth, 2);
        assert_eq!(config.max_active, 2);
        assert_eq!(config.max_frozen, 16);
        assert_eq!(config.max_branches_per_active, 8);
        assert_eq!(config.experiment_wall_ms, Some(10_000));
        assert_eq!(config.search_wall_ms, Some(300));
        assert_eq!(config.search_max_nodes, Some(50_000));
        assert_eq!(args.branch_examples, 4);
    }

    #[test]
    fn quick_preset_uses_short_smoke_budgets() {
        let args =
            parse_args_from(["branch_campaign_driver", "--preset", "quick"]).expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_rounds, 2);
        assert_eq!(config.round_depth, 2);
        assert_eq!(config.max_active, 2);
        assert_eq!(config.max_frozen, 16);
        assert_eq!(config.max_branches_per_active, 8);
        assert_eq!(config.experiment_wall_ms, Some(5_000));
        assert_eq!(config.search_wall_ms, Some(300));
        assert_eq!(config.search_max_nodes, Some(50_000));
        assert_eq!(
            config.search_max_hp_loss,
            Some(RunControlHpLossLimit::Unlimited)
        );
        assert_eq!(
            config.combat_retry_policy,
            BranchCampaignCombatRetryPolicyV1::OnStall
        );
        assert_eq!(args.branch_examples, 3);
    }

    #[test]
    fn deep_preset_uses_larger_budgets() {
        let args =
            parse_args_from(["branch_campaign_driver", "--preset", "deep"]).expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_rounds, 10);
        assert_eq!(config.round_depth, 2);
        assert_eq!(config.max_active, 2);
        assert_eq!(config.max_frozen, 16);
        assert_eq!(config.max_branches_per_active, 8);
        assert_eq!(config.experiment_wall_ms, Some(30_000));
        assert_eq!(config.search_wall_ms, Some(1_000));
        assert_eq!(config.search_max_nodes, Some(200_000));
        assert_eq!(
            config.search_max_hp_loss,
            Some(RunControlHpLossLimit::Unlimited)
        );
        assert_eq!(args.branch_examples, 6);
    }

    #[test]
    fn campaign_cli_keeps_explicit_hp_loss_limit() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--preset",
            "quick",
            "--max-hp-loss",
            "12",
        ])
        .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(
            config.search_max_hp_loss,
            Some(RunControlHpLossLimit::Limit(12))
        );
    }

    #[test]
    fn campaign_cli_can_enable_immediate_combat_retry_for_comparison() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--preset",
            "quick",
            "--combat-retry",
            "immediate",
        ])
        .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(
            config.combat_retry_policy,
            BranchCampaignCombatRetryPolicyV1::Immediate
        );
    }

    #[test]
    fn focused_preset_keeps_explicit_branch_overrides() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--preset",
            "focused",
            "--round-depth",
            "1",
            "--max-active",
            "4",
            "--max-frozen",
            "8",
            "--max-branches-per-active",
            "12",
        ])
        .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.round_depth, 1);
        assert_eq!(config.max_active, 4);
        assert_eq!(config.max_frozen, 8);
        assert_eq!(config.max_branches_per_active, 12);
    }
}
