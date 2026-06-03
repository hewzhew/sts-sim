use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy;
use sts_simulator::eval::run_control::{
    canonical_player_class, load_session_trace_v1, render_run_control_state,
    render_session_trace_replay_report, replay_session_trace_with_recorder,
    AutoCombatCaptureConfig, RewardAutomationConfig, RunControlConfig, RunControlSession,
    SessionTraceLineageRoleV1, SessionTraceLineageV1, SessionTraceRecorder,
    SessionTraceReplayOptions, SessionTraceV1,
};

mod bookmarks;
mod terminal;
mod trace_cli;
use bookmarks::{default_bookmark_registry_path, resolve_goto_bookmark};
use terminal::{run_repl, run_script};
use trace_cli::{file_hash, reject_same_trace_path, trace_output_path, validate_trace_args};

#[derive(Parser, Debug)]
#[command(about = "Thin simulator run/play driver with exact combat capture support")]
struct Args {
    #[arg(long)]
    seed: Option<u64>,

    #[arg(long)]
    ascension: Option<u8>,

    #[arg(long, value_enum)]
    class: Option<CliPlayerClass>,

    #[arg(long)]
    final_act: bool,

    #[arg(long, value_name = "PATH", help = "Read commands from a script file")]
    script: Option<PathBuf>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Replay an existing SessionTraceV1 without choosing an automatic output path"
    )]
    replay_trace: Option<PathBuf>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Replay a trace and record the verified prefix plus new steps into a continuation trace"
    )]
    continue_trace: Option<PathBuf>,

    #[arg(
        long,
        value_name = "NAME",
        help = "Name the auto-generated continuation trace branch; valid only with --continue-trace"
    )]
    branch: Option<String>,

    #[arg(long, help = "Only replay the first N recorded trace steps")]
    replay_steps: Option<usize>,

    #[arg(
        long,
        value_name = "NAME",
        help = "Resume from a named bookmark created with `mark <name>`"
    )]
    goto: Option<String>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Bookmark registry path; defaults to tools/artifacts/traces/bookmarks.json"
    )]
    bookmark_file: Option<PathBuf>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Record successful state-changing commands to this SessionTraceV1 path"
    )]
    trace: Option<PathBuf>,

    #[arg(long)]
    auto_capture_combat: bool,

    #[arg(long)]
    auto_capture_combat_root: Option<PathBuf>,

    #[arg(long)]
    search_max_nodes: Option<usize>,

    #[arg(long)]
    search_wall_ms: Option<u64>,

    #[arg(long)]
    search_max_hp_loss: Option<u32>,

    #[arg(long, value_parser = parse_cli_potion_policy)]
    search_potion_policy: Option<CombatSearchV2PotionPolicy>,

    #[arg(long)]
    search_max_potions_used: Option<u32>,
}

#[derive(Clone, Debug, ValueEnum)]
enum CliPlayerClass {
    Ironclad,
    Silent,
    Defect,
    Watcher,
}

impl CliPlayerClass {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Ironclad => "ironclad",
            Self::Silent => "silent",
            Self::Defect => "defect",
            Self::Watcher => "watcher",
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let bookmark_registry_path = args
        .bookmark_file
        .clone()
        .unwrap_or_else(default_bookmark_registry_path);
    let goto_plan = args
        .goto
        .as_ref()
        .map(|name| resolve_goto_bookmark(&bookmark_registry_path, name))
        .transpose()?;
    validate_goto_args(&args)?;
    validate_trace_args(
        effective_replay_trace(&args, goto_plan.as_ref()).as_ref(),
        effective_continue_trace(&args, goto_plan.as_ref()).as_ref(),
        effective_branch(&args),
    )?;
    let replay_trace_path = effective_continue_trace(&args, goto_plan.as_ref())
        .or_else(|| effective_replay_trace(&args, goto_plan.as_ref()));
    let replay_trace = replay_trace_path
        .as_ref()
        .map(|path| load_session_trace_v1(path))
        .transpose()?;
    let config = run_config_from_args(&args, replay_trace.as_ref())?;
    let replay_options = SessionTraceReplayOptions {
        max_steps: goto_plan
            .as_ref()
            .map(|plan| plan.replay_steps)
            .or(args.replay_steps),
    };
    let mut session = RunControlSession::new(config);
    let trace_path = trace_output_path(
        args.trace.as_ref(),
        effective_continue_trace(&args, goto_plan.as_ref()).as_ref(),
        effective_branch(&args),
    );
    if let (Some(source), Some(output)) = (replay_trace_path.as_ref(), trace_path.as_ref()) {
        reject_same_trace_path(source, output)?;
    }
    let lineage = match (replay_trace_path.as_ref(), trace_path.as_ref()) {
        (Some(source), Some(_)) => Some(SessionTraceLineageV1 {
            role: SessionTraceLineageRoleV1::Continuation,
            parent_trace_path: source.display().to_string(),
            parent_trace_hash: file_hash(source)?,
        }),
        _ => None,
    };
    let mut trace = trace_path
        .as_ref()
        .map(|path| SessionTraceRecorder::new_with_lineage(path.clone(), &session, lineage));

    println!("{}", render_run_control_state(&session));
    if let Some(path) = trace_path.as_ref() {
        println!("recording trace: {}", path.display());
    }
    if let (Some(path), Some(replay_trace)) = (replay_trace_path.as_ref(), replay_trace.as_ref()) {
        let replay_note = if trace_has_combat_automation(replay_trace) {
            "fast combat replay available"
        } else {
            "recorded automation may rerun combat search and take a few seconds"
        };
        println!(
            "replaying trace {} ({} recorded step(s)); {replay_note}",
            path.display(),
            replay_trace.steps.len()
        );
        let report = replay_session_trace_with_recorder(
            &mut session,
            replay_trace,
            replay_options,
            trace.as_mut(),
        );
        println!("{}", render_session_trace_replay_report(&report, &session));
    }

    if args.auto_capture_combat {
        let root = args
            .auto_capture_combat_root
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "tools/artifacts/benchmarks/seed<seed>_act<act>".to_string());
        println!("auto combat capture enabled: {root}");
    }
    if let Some(max_hp_loss) = args.search_max_hp_loss {
        println!(
            "combat search hp-loss gate enabled: default max_hp_loss={max_hp_loss}; use max_hp_loss=off on a command to disable it"
        );
    }
    if args.search_max_nodes.is_some() || args.search_wall_ms.is_some() {
        let max_nodes = args
            .search_max_nodes
            .map(|value| value.to_string())
            .unwrap_or_else(|| "default".to_string());
        let wall_ms = args
            .search_wall_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none/default".to_string());
        println!(
            "combat search budget defaults: max_nodes={max_nodes} wall_ms={wall_ms}; command-local max_nodes/wall_ms override them"
        );
    }
    if args.search_potion_policy.is_some() || args.search_max_potions_used.is_some() {
        let potion_policy = args
            .search_potion_policy
            .map(cli_potion_policy_label)
            .unwrap_or_else(|| "default".to_string());
        let max_potions = args
            .search_max_potions_used
            .map(|value| value.to_string())
            .unwrap_or_else(|| "default".to_string());
        println!(
            "combat search potion defaults: potion={potion_policy} max_potions={max_potions}; command-local potion/max_potions override them"
        );
    }

    if let Some(script) = args.script.as_ref() {
        run_script(
            &mut session,
            script.as_path(),
            &bookmark_registry_path,
            trace.as_mut(),
        )?;
    } else {
        run_repl(&mut session, &bookmark_registry_path, trace.as_mut())?;
    }
    Ok(())
}

fn effective_replay_trace(
    args: &Args,
    goto_plan: Option<&bookmarks::GotoBookmarkPlan>,
) -> Option<PathBuf> {
    let _ = goto_plan;
    args.replay_trace.clone()
}

fn effective_continue_trace(
    args: &Args,
    goto_plan: Option<&bookmarks::GotoBookmarkPlan>,
) -> Option<PathBuf> {
    goto_plan
        .map(|plan| plan.source_trace_path.clone())
        .or_else(|| args.continue_trace.clone())
}

fn effective_branch(args: &Args) -> Option<&str> {
    args.goto.as_deref().or(args.branch.as_deref())
}

fn validate_goto_args(args: &Args) -> Result<(), String> {
    if args.goto.is_none() {
        return Ok(());
    }
    if args.replay_trace.is_some()
        || args.continue_trace.is_some()
        || args.trace.is_some()
        || args.branch.is_some()
        || args.replay_steps.is_some()
    {
        return Err(
            "--goto owns trace replay and output; do not combine it with --replay-trace, --continue-trace, --trace, --branch, or --replay-steps"
                .to_string(),
        );
    }
    Ok(())
}

fn trace_has_combat_automation(trace: &SessionTraceV1) -> bool {
    trace.steps.iter().any(|step| {
        step.annotations.iter().any(|annotation| {
            matches!(
                annotation,
                sts_simulator::eval::run_control::RunControlTraceAnnotationV1::CombatAutomationTrajectory {
                    ..
                }
            )
        })
    })
}

fn run_config_from_args(
    args: &Args,
    replay_trace: Option<&SessionTraceV1>,
) -> Result<RunControlConfig, String> {
    let player_class = match args.class.as_ref() {
        Some(class) => canonical_player_class(class.as_str())?,
        None => replay_trace
            .map(|trace| canonical_player_class(&trace.run_config.player_class))
            .transpose()?
            .unwrap_or("Ironclad"),
    };
    let reward_automation = replay_trace
        .map(|trace| RewardAutomationConfig {
            claim_gold: trace.run_config.reward_automation.claim_gold,
            claim_potion_with_empty_slot: trace
                .run_config
                .reward_automation
                .claim_potion_with_empty_slot,
        })
        .unwrap_or_default();

    Ok(RunControlConfig {
        seed: args
            .seed
            .or_else(|| replay_trace.map(|trace| trace.run_config.seed))
            .unwrap_or(1),
        ascension_level: args
            .ascension
            .or_else(|| replay_trace.map(|trace| trace.run_config.ascension_level))
            .unwrap_or(0),
        final_act: if args.final_act {
            true
        } else {
            replay_trace
                .map(|trace| trace.run_config.final_act)
                .unwrap_or(false)
        },
        player_class,
        reward_automation,
        auto_capture: AutoCombatCaptureConfig {
            enabled: args.auto_capture_combat,
            root: args.auto_capture_combat_root.clone(),
        },
        search_max_nodes: args.search_max_nodes,
        search_wall_ms: args.search_wall_ms,
        search_max_hp_loss: args.search_max_hp_loss,
        search_potion_policy: args.search_potion_policy,
        search_max_potions_used: args.search_max_potions_used,
    })
}

fn parse_cli_potion_policy(value: &str) -> Result<CombatSearchV2PotionPolicy, String> {
    match value.to_ascii_lowercase().as_str() {
        "never" => Ok(CombatSearchV2PotionPolicy::Never),
        "all" | "all_legal_potion_actions" => Ok(CombatSearchV2PotionPolicy::All),
        "semantic" | "semantic_budgeted" | "semantic_budgeted_potion_actions" => {
            Ok(CombatSearchV2PotionPolicy::SemanticBudgeted)
        }
        _ => Err(format!(
            "invalid potion policy '{value}', expected never|all|semantic"
        )),
    }
}

fn cli_potion_policy_label(policy: CombatSearchV2PotionPolicy) -> String {
    match policy {
        CombatSearchV2PotionPolicy::Never => "never",
        CombatSearchV2PotionPolicy::All => "all",
        CombatSearchV2PotionPolicy::SemanticBudgeted => "semantic",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_args() -> Args {
        Args {
            seed: None,
            ascension: None,
            class: None,
            final_act: false,
            script: None,
            replay_trace: None,
            continue_trace: None,
            branch: None,
            replay_steps: None,
            goto: None,
            bookmark_file: None,
            trace: None,
            auto_capture_combat: false,
            auto_capture_combat_root: None,
            search_max_nodes: None,
            search_wall_ms: None,
            search_max_hp_loss: None,
            search_potion_policy: None,
            search_max_potions_used: None,
        }
    }

    #[test]
    fn replay_trace_config_fills_missing_cli_run_config() {
        let session = RunControlSession::new(RunControlConfig {
            seed: 590_093_712,
            ascension_level: 12,
            final_act: true,
            player_class: "Silent",
            ..RunControlConfig::default()
        });
        let trace = SessionTraceV1::new(&session);
        let args = empty_args();

        let config = run_config_from_args(&args, Some(&trace)).expect("config should build");

        assert_eq!(config.seed, 590_093_712);
        assert_eq!(config.ascension_level, 12);
        assert_eq!(config.player_class, "Silent");
        assert!(config.final_act);
    }

    #[test]
    fn explicit_cli_run_config_overrides_replay_trace_config() {
        let session = RunControlSession::new(RunControlConfig {
            seed: 590_093_712,
            ascension_level: 12,
            player_class: "Silent",
            ..RunControlConfig::default()
        });
        let trace = SessionTraceV1::new(&session);
        let mut args = empty_args();
        args.seed = Some(1);
        args.ascension = Some(2);
        args.class = Some(CliPlayerClass::Defect);

        let config = run_config_from_args(&args, Some(&trace)).expect("config should build");

        assert_eq!(config.seed, 1);
        assert_eq!(config.ascension_level, 2);
        assert_eq!(config.player_class, "Defect");
    }

    #[test]
    fn goto_rejects_explicit_trace_replay_flags() {
        let mut args = empty_args();
        args.goto = Some("before_reward".to_string());
        args.replay_steps = Some(3);

        let err = validate_goto_args(&args).expect_err("goto should own replay steps");

        assert!(err.contains("--goto owns trace replay"));
    }

    #[test]
    fn goto_plan_supplies_continue_trace_branch_and_replay_steps() {
        let mut args = empty_args();
        args.goto = Some("before_reward".to_string());
        let plan = bookmarks::GotoBookmarkPlan {
            source_trace_path: PathBuf::from("tools/artifacts/traces/seed.trace.json"),
            replay_steps: 12,
            bookmark: bookmarks::RunPlayBookmarkV1 {
                name: "before_reward".to_string(),
                trace_path: "tools/artifacts/traces/seed.trace.json".to_string(),
                replay_steps: 12,
                decision_step: 12,
                screen_title: "Reward Screen".to_string(),
                act: 1,
                floor: 2,
                hp: 80,
                max_hp: 80,
                gold: 238,
                created_at_unix_ms: 0,
            },
        };

        assert_eq!(
            effective_continue_trace(&args, Some(&plan)),
            Some(PathBuf::from("tools/artifacts/traces/seed.trace.json"))
        );
        assert_eq!(effective_branch(&args), Some("before_reward"));
        assert_eq!(Some(plan.replay_steps), Some(12));
    }
}
