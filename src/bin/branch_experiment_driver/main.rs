use std::fs;
use std::path::PathBuf;

use clap::Parser;
use compact_report::{
    render_compact_report, render_compact_report_with_options, render_profile_comparison,
    CompactReportOptions,
};
use sts_simulator::eval::branch_experiment::{
    run_branch_experiment_profiles_from_shared_start_v1, run_branch_experiment_v1,
    BranchExperimentConfigV1, BranchExperimentReportV1,
};
use sts_simulator::eval::branch_experiment_retention::BranchRetentionBudgetProfileV1;
use sts_simulator::eval::branch_experiment_search_options::parse_branch_experiment_search_options_v1;
use sts_simulator::eval::neow_guided_prefix::{
    neow_guided_prefix_commands_v1, NeowGuidedPrefixConfigV1,
};
use sts_simulator::eval::run_control::{
    default_bookmark_registry_path, resolve_goto_bookmark, AutoCombatCaptureConfig,
    GotoBookmarkPlan, RunControlHpLossLimit,
};

mod compact_report;

#[derive(Debug, Parser)]
#[command(
    name = "branch_experiment_driver",
    about = "Run a small in-memory branch experiment over card reward choices",
    after_long_help = "Examples:
  Start from a bookmark created in run_play_driver with `mark before_reward`:
    branch_experiment_driver --goto before_reward --max-depth 3 --max-branches 24

  Start from an explicit trace prefix:
    branch_experiment_driver --replay-trace tools/artifacts/traces/seed521.trace.json --replay-steps 12

  Compare retention profiles from the same start state:
    branch_experiment_driver --goto before_reward --compare-profiles"
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

    #[arg(
        long,
        default_value = "balanced",
        help = "Branch retention budget profile: balanced, exploration, survival, package, or a comma-separated list with --compare-profiles"
    )]
    retention_profile: String,

    #[arg(
        long,
        help = "Run multiple retention profiles and render a compact side-by-side comparison"
    )]
    compare_profiles: bool,

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

    #[arg(
        long = "combat-search-option",
        value_name = "KEY=VALUE",
        help = "Additional run_control search-combat option, e.g. rollout=turn_beam, beam=4, turn_plan=diagnostic_only, frontier=single_queue"
    )]
    combat_search_options: Vec<String>,

    #[arg(long = "prefix", value_name = "COMMAND")]
    prefix_commands: Vec<String>,

    #[arg(
        long,
        help = "Prepend the shared Neow guidance prefix before script/inline prefix commands"
    )]
    auto_neow_guidance: bool,

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

    #[arg(
        long,
        help = "Start from a named bookmark created in run_play_driver with `mark <name>`"
    )]
    goto: Option<String>,

    #[arg(
        long,
        help = "Bookmark registry path; defaults to tools/artifacts/traces/bookmarks.json"
    )]
    bookmark_file: Option<PathBuf>,

    #[arg(
        long,
        help = "Include card reward skip/Singing Bowl alternatives; this is the default, kept for explicitness"
    )]
    include_skip: bool,

    #[arg(
        long,
        help = "Do not branch card reward skip/Singing Bowl alternatives"
    )]
    exclude_skip: bool,

    #[arg(
        long,
        help = "Also branch skip for completed event card rewards such as Neow/Sensory Stone rewards; off by default to avoid treating already-committed event rewards like ordinary free skips"
    )]
    include_event_reward_skip: bool,

    #[arg(
        long,
        help = "Allow repeated shop purchase branching within the same shop visit; default closes the shop after one purchase branch to avoid buy-combination explosion"
    )]
    allow_shop_multi_buy_branches: bool,

    #[arg(
        long,
        help = "Settle every child branch immediately after a choice; default defers settle until after retention so pruned branches do not spend combat budget"
    )]
    eager_branch_settle: bool,

    #[arg(long)]
    out: Option<PathBuf>,

    #[arg(
        long,
        default_value_t = 5,
        help = "Number of kept branch example lines in compact output"
    )]
    branch_examples: usize,

    #[arg(
        long,
        help = "Focus compact output on branches ending at this boundary title, e.g. \"Card Reward\""
    )]
    focus_boundary: Option<String>,

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
    validate_goto_args(&args)?;
    let bookmark_registry_path = args
        .bookmark_file
        .clone()
        .unwrap_or_else(default_bookmark_registry_path);
    let goto_plan = args
        .goto
        .as_ref()
        .map(|name| resolve_goto_bookmark(&bookmark_registry_path, name))
        .transpose()?;
    let prefix_commands = effective_prefix_commands(&args, player_class)?;
    let profiles = parse_retention_profiles(&args.retention_profile, args.compare_profiles)?;
    if args.compare_profiles {
        if args.focus_boundary.is_some() {
            return Err("--focus-boundary cannot be combined with --compare-profiles".to_string());
        }
        if args.json || args.out.is_some() {
            return Err("--compare-profiles cannot be combined with --json or --out".to_string());
        }
        let configs = profiles
            .into_iter()
            .map(|profile| {
                branch_experiment_config(
                    &args,
                    player_class,
                    prefix_commands.clone(),
                    profile,
                    goto_plan.as_ref(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let reports = run_branch_experiment_profiles_from_shared_start_v1(&configs)?;
        println!("{}", render_profile_comparison(&reports));
        return Ok(());
    }
    let profile = profiles
        .first()
        .copied()
        .unwrap_or(BranchRetentionBudgetProfileV1::Balanced);
    let report = run_branch_experiment_v1(&branch_experiment_config(
        &args,
        player_class,
        prefix_commands,
        profile,
        goto_plan.as_ref(),
    )?)?;
    let compact_options = CompactReportOptions {
        kept_branch_examples: args.branch_examples,
        focus_boundary: args.focus_boundary.clone(),
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

fn branch_experiment_config(
    args: &Args,
    player_class: &'static str,
    prefix_commands: Vec<String>,
    retention_budget_profile: BranchRetentionBudgetProfileV1,
    goto_plan: Option<&GotoBookmarkPlan>,
) -> Result<BranchExperimentConfigV1, String> {
    if args.include_skip && args.exclude_skip {
        return Err("--include-skip and --exclude-skip cannot be combined".to_string());
    }
    Ok(BranchExperimentConfigV1 {
        seed: args.seed,
        ascension_level: args.ascension,
        player_class,
        final_act: args.final_act,
        max_branches: args.max_branches,
        max_branches_per_frontier_group: args.max_per_frontier_group,
        retention_budget_profile,
        max_reward_options_per_branch: args.max_reward_options,
        max_campfire_options_per_branch: Some(args.max_campfire_options),
        max_depth: args.max_depth,
        auto_max_operations: args.auto_max_ops,
        experiment_wall_ms: args.experiment_wall_ms,
        search_max_nodes: args.search_max_nodes,
        search_wall_ms: args.search_wall_ms.or(Some(100)),
        search_max_hp_loss: parse_hp_loss_limit(args.max_hp_loss.as_deref())?,
        search_options: parse_branch_experiment_search_options_v1(&args.combat_search_options)?,
        auto_capture: AutoCombatCaptureConfig::default(),
        include_skip: args.include_skip || !args.exclude_skip,
        include_event_reward_skip: args.include_event_reward_skip,
        auto_leave_after_shop_purchase_branch: !args.allow_shop_multi_buy_branches,
        defer_branch_settle: !args.eager_branch_settle,
        prefix_commands,
        replay_trace_path: effective_replay_trace(args, goto_plan),
        replay_trace_max_steps: effective_replay_steps(args, goto_plan),
    })
}

fn effective_replay_trace(args: &Args, goto_plan: Option<&GotoBookmarkPlan>) -> Option<PathBuf> {
    goto_plan
        .map(|plan| plan.source_trace_path.clone())
        .or_else(|| args.replay_trace.clone())
}

fn effective_replay_steps(args: &Args, goto_plan: Option<&GotoBookmarkPlan>) -> Option<usize> {
    goto_plan
        .map(|plan| plan.replay_steps)
        .or(args.replay_steps)
}

fn validate_goto_args(args: &Args) -> Result<(), String> {
    if args.auto_neow_guidance && (args.replay_trace.is_some() || args.replay_steps.is_some()) {
        return Err(
            "--auto-neow-guidance cannot be combined with trace replay options".to_string(),
        );
    }
    if args.goto.is_none() {
        return Ok(());
    }
    if args.replay_trace.is_some() || args.replay_steps.is_some() {
        return Err(
            "--goto owns trace replay; do not combine it with --replay-trace or --replay-steps"
                .to_string(),
        );
    }
    if args.auto_neow_guidance {
        return Err("--auto-neow-guidance cannot be combined with --goto".to_string());
    }
    Ok(())
}

fn effective_prefix_commands(
    args: &Args,
    player_class: &'static str,
) -> Result<Vec<String>, String> {
    let auto_prefix = if args.auto_neow_guidance {
        neow_guided_prefix_commands_v1(&NeowGuidedPrefixConfigV1 {
            seed: args.seed,
            ascension_level: args.ascension,
            final_act: args.final_act,
            player_class,
            search_max_nodes: args.search_max_nodes,
            search_wall_ms: args.search_wall_ms.or(Some(100)),
        })?
    } else {
        Vec::new()
    };
    let script_prefix_commands = load_prefix_scripts(&args.prefix_scripts)?;
    Ok(merge_prefix_commands(
        merge_prefix_commands(auto_prefix, script_prefix_commands),
        args.prefix_commands.clone(),
    ))
}

fn parse_retention_profiles(
    value: &str,
    compare_profiles: bool,
) -> Result<Vec<BranchRetentionBudgetProfileV1>, String> {
    if compare_profiles && value.trim().eq_ignore_ascii_case("balanced") {
        return Ok(vec![
            BranchRetentionBudgetProfileV1::Balanced,
            BranchRetentionBudgetProfileV1::Exploration,
            BranchRetentionBudgetProfileV1::Survival,
            BranchRetentionBudgetProfileV1::Package,
        ]);
    }
    let profiles = value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::parse)
        .collect::<Result<Vec<_>, _>>()?;
    if profiles.is_empty() {
        return Err("--retention-profile must name at least one profile".to_string());
    }
    if !compare_profiles && profiles.len() > 1 {
        return Err("multiple --retention-profile values require --compare-profiles".to_string());
    }
    Ok(profiles)
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
    use sts_simulator::eval::run_control::{GotoBookmarkPlan, RunPlayBookmarkV1};

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

    #[test]
    fn auto_neow_guidance_prefix_precedes_inline_prefix_commands() {
        let args = Args::try_parse_from([
            "branch_experiment_driver",
            "--seed",
            "521",
            "--auto-neow-guidance",
            "--prefix",
            "go 5",
        ])
        .expect("args parse");

        let commands = effective_prefix_commands(&args, "Ironclad").expect("prefix builds");

        assert_eq!(commands.first().map(String::as_str), Some("0"));
        assert!(commands.len() >= 3);
        assert_eq!(commands.last().map(String::as_str), Some("go 5"));
    }

    #[test]
    fn auto_neow_guidance_rejects_trace_replay_start() {
        let args = Args::try_parse_from([
            "branch_experiment_driver",
            "--auto-neow-guidance",
            "--replay-trace",
            "trace.json",
        ])
        .expect("args parse");

        let err = validate_goto_args(&args).expect_err("auto neow conflicts with replay");

        assert!(err.contains("--auto-neow-guidance cannot be combined"));
    }

    #[test]
    fn cli_includes_reward_skip_branches_by_default() {
        let args = Args::try_parse_from(["branch_experiment_driver"]).expect("args parse");

        let config = branch_experiment_config(
            &args,
            "Ironclad",
            Vec::new(),
            BranchRetentionBudgetProfileV1::Balanced,
            None,
        )
        .expect("default config builds");

        assert!(config.include_skip);
    }

    #[test]
    fn cli_can_exclude_reward_skip_branches() {
        let args = Args::try_parse_from(["branch_experiment_driver", "--exclude-skip"])
            .expect("args parse");

        let config = branch_experiment_config(
            &args,
            "Ironclad",
            Vec::new(),
            BranchRetentionBudgetProfileV1::Balanced,
            None,
        )
        .expect("exclude-skip config builds");

        assert!(!config.include_skip);
    }

    #[test]
    fn cli_only_includes_completed_event_reward_skip_when_requested() {
        let default_args =
            Args::try_parse_from(["branch_experiment_driver"]).expect("default args parse");
        let default_config = branch_experiment_config(
            &default_args,
            "Ironclad",
            Vec::new(),
            BranchRetentionBudgetProfileV1::Balanced,
            None,
        )
        .expect("default config builds");

        let opted_in_args =
            Args::try_parse_from(["branch_experiment_driver", "--include-event-reward-skip"])
                .expect("opt-in args parse");
        let opted_in_config = branch_experiment_config(
            &opted_in_args,
            "Ironclad",
            Vec::new(),
            BranchRetentionBudgetProfileV1::Balanced,
            None,
        )
        .expect("opt-in config builds");

        assert!(!default_config.include_event_reward_skip);
        assert!(opted_in_config.include_event_reward_skip);
    }

    #[test]
    fn cli_defers_branch_settle_by_default() {
        let args = Args::try_parse_from(["branch_experiment_driver"]).expect("args parse");
        let config = branch_experiment_config(
            &args,
            "Ironclad",
            Vec::new(),
            BranchRetentionBudgetProfileV1::Balanced,
            None,
        )
        .expect("default config builds");

        assert!(config.defer_branch_settle);
    }

    #[test]
    fn cli_can_request_eager_branch_settle_for_old_budget_order() {
        let args = Args::try_parse_from(["branch_experiment_driver", "--eager-branch-settle"])
            .expect("args parse");
        let config = branch_experiment_config(
            &args,
            "Ironclad",
            Vec::new(),
            BranchRetentionBudgetProfileV1::Balanced,
            None,
        )
        .expect("config builds");

        assert!(!config.defer_branch_settle);
    }

    #[test]
    fn cli_passes_combat_search_option_overrides_to_branch_config() {
        let args = Args::try_parse_from([
            "branch_experiment_driver",
            "--combat-search-option",
            "rollout=turn_beam",
            "--combat-search-option",
            "beam=4",
        ])
        .expect("args parse");
        let config = branch_experiment_config(
            &args,
            "Ironclad",
            Vec::new(),
            BranchRetentionBudgetProfileV1::Balanced,
            None,
        )
        .expect("config builds");

        assert_eq!(
            config.search_options.rollout_policy,
            Some(
                sts_simulator::ai::combat_search_v2::CombatSearchV2RolloutPolicy::TurnBeamNoPotion
            )
        );
        assert_eq!(config.search_options.rollout_beam_width, Some(4));
    }

    #[test]
    fn cli_rejects_conflicting_reward_skip_flags() {
        let args = Args::try_parse_from([
            "branch_experiment_driver",
            "--include-skip",
            "--exclude-skip",
        ])
        .expect("args parse");

        let err = branch_experiment_config(
            &args,
            "Ironclad",
            Vec::new(),
            BranchRetentionBudgetProfileV1::Balanced,
            None,
        )
        .expect_err("conflicting skip flags should be rejected");

        assert!(err.contains("--include-skip and --exclude-skip cannot be combined"));
    }

    #[test]
    fn goto_plan_supplies_replay_trace_and_steps() {
        let args = Args::try_parse_from(["branch_experiment_driver", "--goto", "before_reward"])
            .expect("args parse");
        let plan = goto_plan("before_reward", "tools/artifacts/traces/seed.trace.json", 7);

        let config = branch_experiment_config(
            &args,
            "Ironclad",
            Vec::new(),
            BranchRetentionBudgetProfileV1::Balanced,
            Some(&plan),
        )
        .expect("goto config builds");

        assert_eq!(
            config.replay_trace_path,
            Some(PathBuf::from("tools/artifacts/traces/seed.trace.json"))
        );
        assert_eq!(config.replay_trace_max_steps, Some(7));
    }

    #[test]
    fn goto_rejects_explicit_trace_replay_flags() {
        let mut args = Args::try_parse_from([
            "branch_experiment_driver",
            "--goto",
            "before_reward",
            "--replay-trace",
            "trace.json",
        ])
        .expect("args parse");

        let err = validate_goto_args(&args).expect_err("goto should own replay trace");
        assert!(err.contains("--goto owns trace replay"));

        args.replay_trace = None;
        args.replay_steps = Some(3);
        let err = validate_goto_args(&args).expect_err("goto should own replay steps");
        assert!(err.contains("--goto owns trace replay"));
    }

    fn goto_plan(name: &str, trace_path: &str, replay_steps: usize) -> GotoBookmarkPlan {
        GotoBookmarkPlan {
            source_trace_path: PathBuf::from(trace_path),
            replay_steps,
            bookmark: RunPlayBookmarkV1 {
                name: name.to_string(),
                trace_path: trace_path.to_string(),
                replay_steps,
                decision_step: 7,
                screen_title: "Card Reward".to_string(),
                act: 1,
                floor: 3,
                hp: 70,
                max_hp: 80,
                gold: 120,
                created_at_unix_ms: 1,
            },
        }
    }
}
