use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use sts_simulator::ai::combat_search_v2::CombatSearchV2PotionPolicy;
use sts_simulator::eval::card_reward_value_loop::{
    build_card_reward_runtime_calibration_pipeline_v1, extract_card_reward_value_loop_examples_v1,
    CardRewardOutcomeCalibrationPromotionConfigV1, CardRewardOutcomeCalibrationV1,
    CardRewardRouteRiskCalibrationV1, CardRewardStrategyPackageCalibrationV1,
    CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_NAME, CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_VERSION,
    CARD_REWARD_ROUTE_RISK_CALIBRATION_SCHEMA_NAME,
    CARD_REWARD_ROUTE_RISK_CALIBRATION_SCHEMA_VERSION,
    CARD_REWARD_STRATEGY_PACKAGE_CALIBRATION_SCHEMA_NAME,
    CARD_REWARD_STRATEGY_PACKAGE_CALIBRATION_SCHEMA_VERSION,
};
use sts_simulator::eval::run_control::{
    canonical_player_class, default_bookmark_registry_path, load_session_trace_v1,
    render_run_control_state, render_session_trace_replay_report,
    replay_session_trace_with_recorder, resolve_goto_bookmark, AutoCombatCaptureConfig,
    GotoBookmarkPlan, RewardAutomationConfig, RunControlConfig, RunControlSession,
    SessionTraceLineageRoleV1, SessionTraceLineageV1, SessionTraceRecorder,
    SessionTraceReplayOptions, SessionTraceV1,
};

mod terminal;
mod trace_cli;
use terminal::{run_repl, run_script};
use trace_cli::{
    default_record_trace_path, file_hash, reject_same_trace_path, trace_output_path,
    validate_trace_args,
};

#[derive(Parser, Debug)]
#[command(
    about = "Thin simulator run/play driver with exact combat capture support",
    after_long_help = "Daily examples:
  Start and auto-record:
    run_play_driver --seed 521 --ascension 0 --class ironclad --record

  Resume a bookmark created by `mark <name>`:
    run_play_driver --goto <name>

  Continue a trace with a named branch:
    run_play_driver --continue-trace tools/artifacts/traces/seed521.trace.json --branch test1

  Start with trace-derived card-reward calibration in lab mode:
    run_play_driver --seed 521 --class ironclad --card-reward-calibration-trace tools/artifacts/traces/seed521.trace.json --card-reward-calibration-profile lab
"
)]
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

    #[arg(
        long,
        help = "Record this new run to an auto-named trace under tools/artifacts/traces"
    )]
    record: bool,

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

    #[arg(
        long,
        value_name = "PATH",
        help = "Load CardRewardOutcomeCalibrationV1 and feed it into card reward value arbitration"
    )]
    card_reward_calibration: Option<PathBuf>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Load CardRewardRouteRiskCalibrationV1 and feed corrected RouteRisk estimates into card reward value arbitration"
    )]
    card_reward_route_risk_calibration: Option<PathBuf>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Load CardRewardStrategyPackageCalibrationV1 and feed corrected StrategyPackage estimates into card reward value arbitration"
    )]
    card_reward_strategy_package_calibration: Option<PathBuf>,

    #[arg(
        long = "card-reward-calibration-trace",
        value_name = "PATH",
        help = "Build a runtime CardRewardOutcomeCalibrationV1 from SessionTraceV1 trace(s) at startup"
    )]
    card_reward_calibration_traces: Vec<PathBuf>,

    #[arg(long, value_enum, default_value = "strict")]
    card_reward_calibration_profile: CliCardRewardCalibrationProfile,
}

#[derive(Clone, Debug, ValueEnum)]
enum CliPlayerClass {
    Ironclad,
    Silent,
    Defect,
    Watcher,
}

#[derive(Clone, Debug, Eq, PartialEq, ValueEnum)]
enum CliCardRewardCalibrationProfile {
    Strict,
    Lab,
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
    validate_record_args(&args)?;
    validate_card_reward_calibration_args(&args)?;
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
    let card_reward_calibration_loaded_message = card_reward_calibration_source_label(&args)
        .zip(config.card_reward_outcome_calibration.as_ref())
        .map(|(source, calibration)| card_reward_calibration_loaded_message(&source, calibration));
    let card_reward_route_risk_calibration_loaded_message =
        card_reward_route_risk_calibration_source_label(&args)
            .zip(config.card_reward_route_risk_calibration.as_ref())
            .map(|(source, calibration)| {
                card_reward_route_risk_calibration_loaded_message(&source, calibration)
            });
    let card_reward_strategy_package_calibration_loaded_message =
        card_reward_strategy_package_calibration_source_label(&args)
            .zip(config.card_reward_strategy_package_calibration.as_ref())
            .map(|(source, calibration)| {
                card_reward_strategy_package_calibration_loaded_message(&source, calibration)
            });
    let replay_options = SessionTraceReplayOptions {
        max_steps: goto_plan
            .as_ref()
            .map(|plan| plan.replay_steps)
            .or(args.replay_steps),
    };
    let mut session = RunControlSession::new(config);
    let record_trace = args.record.then(|| {
        default_record_trace_path(
            session.run_state.seed,
            session.run_state.ascension_level,
            &session.run_state.player_class,
        )
    });
    let trace_path = trace_output_path(
        args.trace.as_ref(),
        record_trace,
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
    if let Some(message) = card_reward_calibration_loaded_message.as_ref() {
        println!("{message}");
    }
    if let Some(message) = card_reward_route_risk_calibration_loaded_message.as_ref() {
        println!("{message}");
    }
    if let Some(message) = card_reward_strategy_package_calibration_loaded_message.as_ref() {
        println!("{message}");
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

fn effective_replay_trace(args: &Args, goto_plan: Option<&GotoBookmarkPlan>) -> Option<PathBuf> {
    let _ = goto_plan;
    args.replay_trace.clone()
}

fn effective_continue_trace(args: &Args, goto_plan: Option<&GotoBookmarkPlan>) -> Option<PathBuf> {
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

fn validate_record_args(args: &Args) -> Result<(), String> {
    if !args.record {
        return Ok(());
    }
    if args.trace.is_some()
        || args.replay_trace.is_some()
        || args.continue_trace.is_some()
        || args.goto.is_some()
        || args.branch.is_some()
        || args.replay_steps.is_some()
    {
        return Err(
            "--record starts a fresh auto-named trace; do not combine it with --trace, --replay-trace, --continue-trace, --goto, --branch, or --replay-steps"
                .to_string(),
        );
    }
    Ok(())
}

fn validate_card_reward_calibration_args(args: &Args) -> Result<(), String> {
    if args.card_reward_calibration.is_some() && !args.card_reward_calibration_traces.is_empty() {
        return Err(
            "--card-reward-calibration and --card-reward-calibration-trace are alternative card reward calibration sources; use only one"
                .to_string(),
        );
    }
    Ok(())
}

fn trace_has_combat_automation(trace: &SessionTraceV1) -> bool {
    trace.steps.iter().any(|step| {
        sts_simulator::eval::run_control::annotations_have_combat_automation_trajectory_v1(
            &step.annotations,
        )
    })
}

fn run_config_from_args(
    args: &Args,
    replay_trace: Option<&SessionTraceV1>,
) -> Result<RunControlConfig, String> {
    let card_reward_calibrations = card_reward_calibrations_from_args(args)?;
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
            claim_safe_relic_without_sapphire_key: trace
                .run_config
                .reward_automation
                .claim_safe_relic_without_sapphire_key,
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
        card_reward_outcome_calibration: card_reward_calibrations.outcome,
        card_reward_route_risk_calibration: card_reward_calibrations.route_risk,
        card_reward_strategy_package_calibration: card_reward_calibrations.strategy_package,
    })
}

struct CardRewardRuntimeCalibrationsFromArgs {
    outcome: Option<CardRewardOutcomeCalibrationV1>,
    route_risk: Option<CardRewardRouteRiskCalibrationV1>,
    strategy_package: Option<CardRewardStrategyPackageCalibrationV1>,
}

fn card_reward_calibrations_from_args(
    args: &Args,
) -> Result<CardRewardRuntimeCalibrationsFromArgs, String> {
    if let Some(path) = args.card_reward_calibration.as_deref() {
        let outcome = load_card_reward_outcome_calibration(path)?;
        let route_risk = args
            .card_reward_route_risk_calibration
            .as_deref()
            .map(load_card_reward_route_risk_calibration)
            .transpose()?;
        let strategy_package = args
            .card_reward_strategy_package_calibration
            .as_deref()
            .map(load_card_reward_strategy_package_calibration)
            .transpose()?;
        return Ok(CardRewardRuntimeCalibrationsFromArgs {
            outcome: Some(outcome),
            route_risk,
            strategy_package,
        });
    }
    if args.card_reward_calibration_traces.is_empty() {
        let route_risk = args
            .card_reward_route_risk_calibration
            .as_deref()
            .map(load_card_reward_route_risk_calibration)
            .transpose()?;
        let strategy_package = args
            .card_reward_strategy_package_calibration
            .as_deref()
            .map(load_card_reward_strategy_package_calibration)
            .transpose()?;
        return Ok(CardRewardRuntimeCalibrationsFromArgs {
            outcome: None,
            route_risk,
            strategy_package,
        });
    }

    let mut examples = Vec::new();
    for path in &args.card_reward_calibration_traces {
        let trace = load_session_trace_v1(path)?;
        examples.extend(extract_card_reward_value_loop_examples_v1(&trace)?);
    }
    let pipeline = build_card_reward_runtime_calibration_pipeline_v1(
        &examples,
        &card_reward_calibration_promotion_config(args),
    );
    let route_risk = if let Some(path) = args.card_reward_route_risk_calibration.as_deref() {
        load_card_reward_route_risk_calibration(path)?
    } else {
        pipeline.route_risk_calibration
    };
    let strategy_package =
        if let Some(path) = args.card_reward_strategy_package_calibration.as_deref() {
            load_card_reward_strategy_package_calibration(path)?
        } else {
            pipeline.strategy_package_calibration
        };
    Ok(CardRewardRuntimeCalibrationsFromArgs {
        outcome: Some(pipeline.promoted_calibration),
        route_risk: Some(route_risk),
        strategy_package: Some(strategy_package),
    })
}

fn card_reward_calibration_promotion_config(
    args: &Args,
) -> CardRewardOutcomeCalibrationPromotionConfigV1 {
    match args.card_reward_calibration_profile {
        CliCardRewardCalibrationProfile::Strict => {
            CardRewardOutcomeCalibrationPromotionConfigV1::default()
        }
        CliCardRewardCalibrationProfile::Lab => CardRewardOutcomeCalibrationPromotionConfigV1 {
            approve_short_horizon_autopilot_gate: true,
            min_distinct_seeds: 1,
            min_bucket_outcome_attached_count: 1,
            min_bucket_confidence: 0.2,
            max_bucket_uncertainty: 0.8,
            reject_hidden_simulator_state: true,
        },
    }
}

fn card_reward_calibration_source_label(args: &Args) -> Option<String> {
    if let Some(path) = args.card_reward_calibration.as_ref() {
        return Some(path.display().to_string());
    }
    (!args.card_reward_calibration_traces.is_empty()).then(|| {
        format!(
            "generated from {} trace(s) using {:?} profile",
            args.card_reward_calibration_traces.len(),
            args.card_reward_calibration_profile,
        )
    })
}

fn card_reward_route_risk_calibration_source_label(args: &Args) -> Option<String> {
    if let Some(path) = args.card_reward_route_risk_calibration.as_ref() {
        return Some(path.display().to_string());
    }
    (!args.card_reward_calibration_traces.is_empty()).then(|| {
        format!(
            "generated from {} trace(s) using {:?} profile",
            args.card_reward_calibration_traces.len(),
            args.card_reward_calibration_profile,
        )
    })
}

fn card_reward_strategy_package_calibration_source_label(args: &Args) -> Option<String> {
    if let Some(path) = args.card_reward_strategy_package_calibration.as_ref() {
        return Some(path.display().to_string());
    }
    (!args.card_reward_calibration_traces.is_empty()).then(|| {
        format!(
            "generated from {} trace(s) using {:?} profile",
            args.card_reward_calibration_traces.len(),
            args.card_reward_calibration_profile,
        )
    })
}

fn load_card_reward_outcome_calibration(
    path: &Path,
) -> Result<CardRewardOutcomeCalibrationV1, String> {
    let raw = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read card reward calibration {}: {err}",
            path.display()
        )
    })?;
    let calibration =
        serde_json::from_str::<CardRewardOutcomeCalibrationV1>(&raw).map_err(|err| {
            format!(
                "failed to parse CardRewardOutcomeCalibrationV1 {}: {err}",
                path.display()
            )
        })?;
    if calibration.schema_name != CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_NAME {
        return Err(format!(
            "card reward calibration {} has schema_name {}, expected {}",
            path.display(),
            calibration.schema_name,
            CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_NAME
        ));
    }
    if calibration.schema_version != CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_VERSION {
        return Err(format!(
            "card reward calibration {} has schema_version {}, expected {}",
            path.display(),
            calibration.schema_version,
            CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_VERSION
        ));
    }
    Ok(calibration)
}

fn load_card_reward_route_risk_calibration(
    path: &Path,
) -> Result<CardRewardRouteRiskCalibrationV1, String> {
    let raw = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read card reward RouteRisk calibration {}: {err}",
            path.display()
        )
    })?;
    let calibration =
        serde_json::from_str::<CardRewardRouteRiskCalibrationV1>(&raw).map_err(|err| {
            format!(
                "failed to parse CardRewardRouteRiskCalibrationV1 {}: {err}",
                path.display()
            )
        })?;
    if calibration.schema_name != CARD_REWARD_ROUTE_RISK_CALIBRATION_SCHEMA_NAME {
        return Err(format!(
            "card reward RouteRisk calibration {} has schema_name {}, expected {}",
            path.display(),
            calibration.schema_name,
            CARD_REWARD_ROUTE_RISK_CALIBRATION_SCHEMA_NAME
        ));
    }
    if calibration.schema_version != CARD_REWARD_ROUTE_RISK_CALIBRATION_SCHEMA_VERSION {
        return Err(format!(
            "card reward RouteRisk calibration {} has schema_version {}, expected {}",
            path.display(),
            calibration.schema_version,
            CARD_REWARD_ROUTE_RISK_CALIBRATION_SCHEMA_VERSION
        ));
    }
    Ok(calibration)
}

fn load_card_reward_strategy_package_calibration(
    path: &Path,
) -> Result<CardRewardStrategyPackageCalibrationV1, String> {
    let raw = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read card reward StrategyPackage calibration {}: {err}",
            path.display()
        )
    })?;
    let calibration = serde_json::from_str::<CardRewardStrategyPackageCalibrationV1>(&raw)
        .map_err(|err| {
            format!(
                "failed to parse CardRewardStrategyPackageCalibrationV1 {}: {err}",
                path.display()
            )
        })?;
    if calibration.schema_name != CARD_REWARD_STRATEGY_PACKAGE_CALIBRATION_SCHEMA_NAME {
        return Err(format!(
            "card reward StrategyPackage calibration {} has schema_name {}, expected {}",
            path.display(),
            calibration.schema_name,
            CARD_REWARD_STRATEGY_PACKAGE_CALIBRATION_SCHEMA_NAME
        ));
    }
    if calibration.schema_version != CARD_REWARD_STRATEGY_PACKAGE_CALIBRATION_SCHEMA_VERSION {
        return Err(format!(
            "card reward StrategyPackage calibration {} has schema_version {}, expected {}",
            path.display(),
            calibration.schema_version,
            CARD_REWARD_STRATEGY_PACKAGE_CALIBRATION_SCHEMA_VERSION
        ));
    }
    Ok(calibration)
}

fn card_reward_calibration_loaded_message(
    source: &str,
    calibration: &CardRewardOutcomeCalibrationV1,
) -> String {
    let bucket_count = calibration.card_id_buckets.len();
    let value_usable_count = calibration
        .card_id_buckets
        .iter()
        .filter(|bucket| bucket.usable_for_value_estimate)
        .count();
    let gate_usable_count = calibration
        .card_id_buckets
        .iter()
        .filter(|bucket| bucket.usable_for_autopilot_gate)
        .count();
    let distinct_seed_count = calibration
        .provenance
        .distinct_seed_count
        .map(|count| count.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let ruleset = calibration
        .provenance
        .ruleset_version
        .as_deref()
        .filter(|ruleset| !ruleset.trim().is_empty())
        .unwrap_or("unknown");
    let mean_played = calibration
        .global
        .mean_picked_card_played_count
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "unknown".to_string());
    let mean_drawn = calibration
        .global
        .mean_picked_card_drawn_count
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "unknown".to_string());

    format!(
        "card reward calibration loaded: {source} [buckets={bucket_count} value_usable={value_usable_count} gate_usable={gate_usable_count} distinct_seeds={distinct_seed_count} played_obs={} mean_played={mean_played} drawn_obs={} mean_drawn={mean_drawn} short_horizon_gate_approved={} ruleset={ruleset}]",
        calibration.global.picked_card_played_observation_count,
        calibration.global.picked_card_drawn_observation_count,
        calibration
            .provenance
            .short_horizon_autopilot_gate_approved,
    )
}

fn card_reward_strategy_package_calibration_loaded_message(
    source: &str,
    calibration: &CardRewardStrategyPackageCalibrationV1,
) -> String {
    format!(
        "card reward StrategyPackage calibration loaded: {source} [examples={} evaluated={} buckets={} mean_route_hp_loss={} mean_abs_error={}]",
        calibration.total_examples,
        calibration.evaluated_examples,
        calibration.buckets.len(),
        calibration
            .global
            .mean_actual_route_hp_loss
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "unknown".to_string()),
        calibration
            .global
            .mean_absolute_error
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "unknown".to_string()),
    )
}

fn card_reward_route_risk_calibration_loaded_message(
    source: &str,
    calibration: &CardRewardRouteRiskCalibrationV1,
) -> String {
    format!(
        "card reward RouteRisk calibration loaded: {source} [examples={} evaluated={} buckets={} mean_route_hp_loss={} mean_hp_before_next_elite={} mean_hp_after_next_elite={} mean_pre_elite_route_loss={} mean_elite_combat_loss={} mean_abs_error={}]",
        calibration.total_examples,
        calibration.evaluated_examples,
        calibration.buckets.len(),
        calibration
            .global
            .mean_actual_route_hp_loss
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "unknown".to_string()),
        calibration
            .global
            .mean_actual_hp_before_next_elite
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "unknown".to_string()),
        calibration
            .global
            .mean_actual_hp_after_next_elite
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "unknown".to_string()),
        calibration
            .global
            .mean_pre_next_elite_route_hp_loss
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "unknown".to_string()),
        calibration
            .global
            .mean_next_elite_combat_hp_loss
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "unknown".to_string()),
        calibration
            .global
            .mean_absolute_error
            .map(|value| format!("{value:.3}"))
            .unwrap_or_else(|| "unknown".to_string()),
    )
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
            record: false,
            auto_capture_combat: false,
            auto_capture_combat_root: None,
            search_max_nodes: None,
            search_wall_ms: None,
            search_max_hp_loss: None,
            search_potion_policy: None,
            search_max_potions_used: None,
            card_reward_calibration: None,
            card_reward_route_risk_calibration: None,
            card_reward_strategy_package_calibration: None,
            card_reward_calibration_traces: Vec::new(),
            card_reward_calibration_profile: CliCardRewardCalibrationProfile::Strict,
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
    fn card_reward_calibration_arg_loads_runtime_config() {
        let path = unique_temp_path("card_reward_calibration.json");
        let calibration = CardRewardOutcomeCalibrationV1 {
            schema_name: CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_NAME.to_string(),
            schema_version: CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_VERSION,
            label_role: "diagnostic_not_teacher_label".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            estimator_kind: "selected_outcome_card_id_prior_v1".to_string(),
            provenance: Default::default(),
            total_examples: 0,
            usable_outcome_examples: 0,
            missing_outcome_examples: 0,
            global:
                sts_simulator::eval::card_reward_value_loop::CardRewardOutcomeCalibrationGlobalV1 {
                    selected_count: 0,
                    outcome_attached_count: 0,
                    mean_next_combat_hp_loss: None,
                    picked_card_drawn_observation_count: 0,
                    mean_picked_card_drawn_count: None,
                    picked_card_played_observation_count: 0,
                    mean_picked_card_played_count: None,
                },
            card_id_buckets: Vec::new(),
        };
        fs::write(
            &path,
            serde_json::to_string_pretty(&calibration).expect("calibration should serialize"),
        )
        .expect("calibration fixture should write");
        let mut args = empty_args();
        args.card_reward_calibration = Some(path.clone());

        let config = run_config_from_args(&args, None).expect("config should load calibration");

        assert_eq!(
            config
                .card_reward_outcome_calibration
                .expect("calibration should be present"),
            calibration
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn card_reward_calibration_trace_builds_runtime_config() {
        let path = unique_temp_path("card_reward_trace.json");
        let session = RunControlSession::new(RunControlConfig::default());
        let trace = SessionTraceV1::new(&session);
        fs::write(
            &path,
            serde_json::to_string_pretty(&trace).expect("trace should serialize"),
        )
        .expect("trace fixture should write");
        let mut args = empty_args();
        args.card_reward_calibration_traces = vec![path.clone()];
        args.card_reward_calibration_profile = CliCardRewardCalibrationProfile::Lab;

        let config =
            run_config_from_args(&args, None).expect("config should build runtime calibration");

        let calibration = config
            .card_reward_outcome_calibration
            .expect("trace-derived calibration should be present");
        assert_eq!(calibration.total_examples, 0);
        assert!(calibration.provenance.short_horizon_autopilot_gate_approved);
        let route_risk = config
            .card_reward_route_risk_calibration
            .expect("trace-derived RouteRisk calibration should be present");
        assert_eq!(route_risk.total_examples, 0);
        assert_eq!(route_risk.evaluated_examples, 0);
        let strategy_package = config
            .card_reward_strategy_package_calibration
            .expect("trace-derived StrategyPackage calibration should be present");
        assert_eq!(strategy_package.total_examples, 0);
        assert_eq!(strategy_package.evaluated_examples, 0);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn card_reward_route_risk_calibration_arg_loads_runtime_config() {
        let path = unique_temp_path("card_reward_route_risk_calibration.json");
        let calibration = empty_route_risk_calibration();
        fs::write(
            &path,
            serde_json::to_string_pretty(&calibration)
                .expect("route risk calibration should serialize"),
        )
        .expect("route risk calibration fixture should write");
        let mut args = empty_args();
        args.card_reward_route_risk_calibration = Some(path.clone());

        let config =
            run_config_from_args(&args, None).expect("config should load RouteRisk calibration");

        assert!(config.card_reward_outcome_calibration.is_none());
        assert_eq!(
            config
                .card_reward_route_risk_calibration
                .expect("RouteRisk calibration should be present"),
            calibration
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn card_reward_strategy_package_calibration_arg_loads_runtime_config() {
        let path = unique_temp_path("card_reward_strategy_package_calibration.json");
        let calibration = empty_strategy_package_calibration();
        fs::write(
            &path,
            serde_json::to_string_pretty(&calibration)
                .expect("strategy package calibration should serialize"),
        )
        .expect("strategy package calibration fixture should write");
        let mut args = empty_args();
        args.card_reward_strategy_package_calibration = Some(path.clone());

        let config = run_config_from_args(&args, None)
            .expect("config should load StrategyPackage calibration");

        assert!(config.card_reward_outcome_calibration.is_none());
        assert_eq!(
            config
                .card_reward_strategy_package_calibration
                .expect("StrategyPackage calibration should be present"),
            calibration
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn card_reward_calibration_sources_are_mutually_exclusive() {
        let mut args = empty_args();
        args.card_reward_calibration = Some(PathBuf::from("calibration.json"));
        args.card_reward_calibration_traces = vec![PathBuf::from("trace.json")];

        let err = validate_card_reward_calibration_args(&args)
            .expect_err("only one card reward calibration source should be allowed");

        assert!(err.contains("--card-reward-calibration"));
        assert!(err.contains("--card-reward-calibration-trace"));
    }

    #[test]
    fn lab_card_reward_calibration_profile_uses_explicit_permissive_dev_gates() {
        let mut args = empty_args();
        args.card_reward_calibration_profile = CliCardRewardCalibrationProfile::Lab;

        let config = card_reward_calibration_promotion_config(&args);

        assert!(config.approve_short_horizon_autopilot_gate);
        assert_eq!(config.min_distinct_seeds, 1);
        assert_eq!(config.min_bucket_outcome_attached_count, 1);
        assert_eq!(config.min_bucket_confidence, 0.2);
        assert_eq!(config.max_bucket_uncertainty, 0.8);
        assert!(config.reject_hidden_simulator_state);
    }

    #[test]
    fn card_reward_calibration_loaded_message_summarizes_runtime_eligibility() {
        let mut calibration = CardRewardOutcomeCalibrationV1 {
            schema_name: CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_NAME.to_string(),
            schema_version: CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_VERSION,
            label_role: "diagnostic_not_teacher_label".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            estimator_kind: "selected_outcome_card_id_prior_v1".to_string(),
            provenance: Default::default(),
            total_examples: 3,
            usable_outcome_examples: 3,
            missing_outcome_examples: 0,
            global:
                sts_simulator::eval::card_reward_value_loop::CardRewardOutcomeCalibrationGlobalV1 {
                    selected_count: 3,
                    outcome_attached_count: 3,
                    mean_next_combat_hp_loss: Some(5.0),
                    picked_card_drawn_observation_count: 1,
                    mean_picked_card_drawn_count: Some(1.0),
                    picked_card_played_observation_count: 2,
                    mean_picked_card_played_count: Some(1.5),
                },
            card_id_buckets: Vec::new(),
        };
        calibration.provenance.distinct_seed_count = Some(2);
        calibration.provenance.short_horizon_autopilot_gate_approved = true;
        calibration.card_id_buckets = vec![
            sts_simulator::eval::card_reward_value_loop::CardRewardOutcomeCalibrationBucketV1 {
                bucket_key: "card_id:TwinStrike".to_string(),
                card_id: "TwinStrike".to_string(),
                selected_count: 2,
                outcome_attached_count: 2,
                missing_outcome_count: 0,
                mean_next_combat_hp_loss: Some(4.0),
                hp_loss_bucket_counts: Vec::new(),
                upgraded_count: 0,
                removed_count: 0,
                picked_card_drawn_observation_count: 0,
                mean_picked_card_drawn_count: None,
                picked_card_played_observation_count: 0,
                mean_picked_card_played_count: None,
                confidence: 0.8,
                uncertainty: 0.2,
                usable_for_value_estimate: true,
                usable_for_autopilot_gate: true,
            },
            sts_simulator::eval::card_reward_value_loop::CardRewardOutcomeCalibrationBucketV1 {
                bucket_key: "card_id:Cleave".to_string(),
                card_id: "Cleave".to_string(),
                selected_count: 1,
                outcome_attached_count: 1,
                missing_outcome_count: 0,
                mean_next_combat_hp_loss: Some(7.0),
                hp_loss_bucket_counts: Vec::new(),
                upgraded_count: 0,
                removed_count: 0,
                picked_card_drawn_observation_count: 0,
                mean_picked_card_drawn_count: None,
                picked_card_played_observation_count: 0,
                mean_picked_card_played_count: None,
                confidence: 0.2,
                uncertainty: 0.8,
                usable_for_value_estimate: true,
                usable_for_autopilot_gate: false,
            },
        ];

        let message = card_reward_calibration_loaded_message(
            "tools/artifacts/card_reward.promoted.json",
            &calibration,
        );

        assert!(message.contains("card reward calibration loaded:"));
        assert!(message.contains("buckets=2"));
        assert!(message.contains("value_usable=2"));
        assert!(message.contains("gate_usable=1"));
        assert!(message.contains("distinct_seeds=2"));
        assert!(message.contains("short_horizon_gate_approved=true"));
        assert!(message.contains("played_obs=2"));
        assert!(message.contains("mean_played=1.500"));
        assert!(message.contains("drawn_obs=1"));
        assert!(message.contains("mean_drawn=1.000"));
    }

    #[test]
    fn card_reward_route_risk_calibration_loaded_message_summarizes_estimator_coverage() {
        let mut calibration = empty_route_risk_calibration();
        calibration.total_examples = 5;
        calibration.evaluated_examples = 3;
        calibration.buckets = vec![
            sts_simulator::eval::card_reward_value_loop::CardRewardRouteRiskCalibrationBucketV1 {
                bucket_key: "route_risk_delta:positive".to_string(),
                evaluated_count: 3,
                mean_actual_route_hp_loss: Some(4.0),
                mean_actual_next_combat_hp_loss: Some(4.0),
                mean_actual_hp_before_next_elite: None,
                mean_actual_hp_after_next_elite: None,
                mean_pre_next_elite_route_hp_loss: None,
                mean_next_elite_combat_hp_loss: None,
                mean_predicted_route_risk_delta: Some(0.5),
                mean_actual_survival_delta: Some(0.2),
                mean_signed_error: Some(0.3),
                mean_absolute_error: Some(0.3),
                confidence: 0.5,
                uncertainty: 0.5,
                usable_for_value_estimate: true,
                usable_for_autopilot_gate: false,
            },
        ];
        calibration.global.mean_actual_route_hp_loss = Some(4.0);
        calibration.global.mean_absolute_error = Some(0.3);

        let message = card_reward_route_risk_calibration_loaded_message(
            "tools/artifacts/card_reward.route_risk.json",
            &calibration,
        );

        assert!(message.contains("card reward RouteRisk calibration loaded:"));
        assert!(message.contains("examples=5"));
        assert!(message.contains("evaluated=3"));
        assert!(message.contains("buckets=1"));
        assert!(message.contains("mean_route_hp_loss=4.000"));
        assert!(message.contains("mean_hp_before_next_elite=unknown"));
        assert!(message.contains("mean_pre_elite_route_loss=unknown"));
        assert!(message.contains("mean_abs_error=0.300"));
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
    fn record_rejects_explicit_trace_and_replay_flags() {
        let mut args = empty_args();
        args.record = true;
        args.trace = Some(PathBuf::from("manual.trace.json"));

        let err = validate_record_args(&args).expect_err("record should own trace output path");

        assert!(err.contains("--record starts a fresh auto-named trace"));
    }

    #[test]
    fn goto_plan_supplies_continue_trace_branch_and_replay_steps() {
        let mut args = empty_args();
        args.goto = Some("before_reward".to_string());
        let plan = GotoBookmarkPlan {
            source_trace_path: PathBuf::from("tools/artifacts/traces/seed.trace.json"),
            replay_steps: 12,
            bookmark: sts_simulator::eval::run_control::RunPlayBookmarkV1 {
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

    fn unique_temp_path(file_name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "run_play_driver_test_{}_{}_{}",
            std::process::id(),
            nanos,
            file_name
        ))
    }

    fn empty_route_risk_calibration() -> CardRewardRouteRiskCalibrationV1 {
        CardRewardRouteRiskCalibrationV1 {
            schema_name: CARD_REWARD_ROUTE_RISK_CALIBRATION_SCHEMA_NAME.to_string(),
            schema_version: CARD_REWARD_ROUTE_RISK_CALIBRATION_SCHEMA_VERSION,
            label_role: "diagnostic_not_teacher_label".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            estimator_kind: "route_risk_selected_candidate_route_window_v1".to_string(),
            total_examples: 0,
            evaluated_examples: 0,
            missing_public_packet_examples: 0,
            missing_outcome_examples: 0,
            missing_selected_route_risk_estimate_examples: 0,
            global: sts_simulator::eval::card_reward_value_loop::CardRewardRouteRiskCalibrationGlobalV1 {
                evaluated_count: 0,
                mean_actual_route_hp_loss: None,
                mean_actual_next_combat_hp_loss: None,
                mean_actual_hp_before_next_elite: None,
                mean_actual_hp_after_next_elite: None,
                mean_pre_next_elite_route_hp_loss: None,
                mean_next_elite_combat_hp_loss: None,
                mean_predicted_route_risk_delta: None,
                mean_actual_survival_delta: None,
                mean_signed_error: None,
                mean_absolute_error: None,
            },
            buckets: Vec::new(),
        }
    }

    fn empty_strategy_package_calibration() -> CardRewardStrategyPackageCalibrationV1 {
        CardRewardStrategyPackageCalibrationV1 {
            schema_name: CARD_REWARD_STRATEGY_PACKAGE_CALIBRATION_SCHEMA_NAME.to_string(),
            schema_version: CARD_REWARD_STRATEGY_PACKAGE_CALIBRATION_SCHEMA_VERSION,
            label_role: "diagnostic_not_teacher_label".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            estimator_kind: "strategy_package_selected_candidate_alignment_v1".to_string(),
            total_examples: 0,
            evaluated_examples: 0,
            missing_public_packet_examples: 0,
            missing_outcome_examples: 0,
            missing_selected_strategy_package_estimate_examples: 0,
            global:
                sts_simulator::eval::card_reward_value_loop::CardRewardStrategyPackageCalibrationGlobalV1 {
                    evaluated_count: 0,
                    mean_actual_route_hp_loss: None,
                    mean_actual_next_combat_hp_loss: None,
                    mean_predicted_strategy_package_delta: None,
                    mean_actual_survival_delta: None,
                    mean_signed_error: None,
                    mean_absolute_error: None,
                },
            buckets: Vec::new(),
        }
    }
}
