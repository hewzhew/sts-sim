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
use sts_simulator::state::core::EngineState;

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

    #[arg(
        long = "inspect-shop-evidence",
        help = "Print current-code shop candidate evidence and strategic deltas for the selected checkpoint session"
    )]
    inspect_shop_evidence: bool,

    #[arg(
        long = "inspect-deck-mutation",
        help = "Print current-code DeckMutationCompiler plan groups for the selected checkpoint session"
    )]
    inspect_deck_mutation: bool,

    #[arg(
        long = "inspect-route-evidence",
        help = "Print current-code route planner candidate evidence for the selected map checkpoint session"
    )]
    inspect_route_evidence: bool,
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
    if args.inspect_shop_evidence {
        println!("{}", render_checkpoint_shop_evidence_v1(&session)?);
    } else if args.inspect_deck_mutation {
        println!("{}", render_checkpoint_deck_mutation_v1(&session)?);
    } else if args.inspect_route_evidence {
        println!("{}", render_checkpoint_route_evidence_v1(&session)?);
    } else if args.inspect_search {
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

fn render_checkpoint_route_evidence_v1(session: &RunControlSession) -> Result<String, String> {
    if !session.engine_state.is_map_surface() {
        return Err(format!(
            "--inspect-route-evidence requires MapNavigation/MapOverlay engine state, got {:?}",
            session.engine_state
        ));
    }
    let trace = sts_simulator::ai::route_planner_v1::plan_route_decision_v1(
        &session.run_state,
        &session.engine_state,
        sts_simulator::ai::route_planner_v1::RoutePlannerConfigV1::default(),
    );
    Ok(sts_simulator::ai::route_planner_v1::render_route_decision_trace_v1(&trace))
}

fn render_checkpoint_deck_mutation_v1(session: &RunControlSession) -> Result<String, String> {
    let EngineState::RunPendingChoice(choice) = &session.engine_state else {
        return Err(format!(
            "--inspect-deck-mutation requires RunPendingChoice engine state, got {:?}",
            session.engine_state
        ));
    };
    let decision = sts_simulator::ai::deck_mutation_compiler_v1::compile_deck_mutation_decision_v1(
        &session.run_state,
        choice,
        sts_simulator::ai::deck_mutation_compiler_v1::DeckMutationCompilerModeV1::Inspect,
    );
    Ok(
        sts_simulator::ai::deck_mutation_compiler_v1::render_compiled_deck_mutation_decision_v1(
            &decision,
        ),
    )
}

fn render_checkpoint_shop_evidence_v1(session: &RunControlSession) -> Result<String, String> {
    let EngineState::Shop(shop) = &session.engine_state else {
        return Err(format!(
            "--inspect-shop-evidence requires Shop engine state, got {:?}",
            session.engine_state
        ));
    };
    let context =
        sts_simulator::ai::shop_policy_v1::build_shop_decision_context_v1(&session.run_state, shop);
    let compiled = sts_simulator::ai::shop_policy_v1::compile_shop_decision_v1(
        &context,
        &sts_simulator::ai::shop_policy_v1::ShopPolicyConfigV1::default(),
        sts_simulator::ai::shop_policy_v1::ShopCompileModeV1::BranchTopK { max_plans: 6 },
    );
    let trace = &compiled.strategic_trace;
    let mut lines = Vec::new();
    lines.push(format!(
        "Shop compiled decision: act={} floor={} hp={}/{} gold={} boss={:?}",
        session.run_state.act_num,
        session.run_state.floor_num,
        session.run_state.current_hp,
        session.run_state.max_hp,
        session.run_state.gold,
        session.run_state.boss_key
    ));
    lines.push(format!(
        "context: conversion_pressure={} affordable_purchase_exists={} candidates={}",
        context.conversion_pressure,
        context.affordable_purchase_exists,
        context.candidates.len()
    ));
    lines.push(format!(
        "selected_plan: {}",
        render_shop_plan_with_evaluation_v1(&compiled.selected_plan, &compiled.candidate_plans)
    ));
    lines.push(render_shop_plan_candidate_summary_v1(
        &compiled.candidate_plans,
    ));
    if compiled.alternatives.is_empty() {
        lines.push("alternative_plans: -".to_string());
    } else {
        lines.push(format!(
            "alternative_plans: {}",
            compiled.alternatives.len()
        ));
        for (idx, plan) in compiled.alternatives.iter().enumerate() {
            lines.push(format!(
                "  {idx}. {}",
                render_shop_plan_with_evaluation_v1(plan, &compiled.candidate_plans)
            ));
        }
    }
    lines.push("candidate evidence:".to_string());
    for candidate in &context.candidates {
        let action_id = inspect_shop_candidate_action_id(candidate);
        let compiled = trace
            .compiled
            .iter()
            .find(|decision| decision.action.candidate_id() == action_id);
        let delta = trace
            .candidate_deltas
            .iter()
            .find(|delta| delta.action.candidate_id() == action_id);
        lines.push(format!(
            "- {} | id={} | class={:?} gate={:?} legacy_priority={} verdict={} score={}",
            candidate.label,
            action_id,
            candidate.class,
            candidate.support_gate,
            candidate
                .purchase_priority
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string()),
            compiled
                .map(|decision| format!("{:?}", decision.verdict))
                .unwrap_or_else(|| "-".to_string()),
            compiled
                .map(|decision| format!("{:.2}", decision.score))
                .unwrap_or_else(|| "-".to_string()),
        ));
        lines.push(format!(
            "    evidence: {}",
            render_short_list(&candidate.evidence)
        ));
        lines.push(format!(
            "    risks: {}",
            render_short_list(&candidate.risks)
        ));
        if let Some(delta) = delta {
            lines.push(format!(
                "    delta: role={:?} hint={:?} positive=[{}] negative=[{}]",
                delta.role,
                delta.verdict_hint,
                render_ledger_deltas(&delta.positive),
                render_ledger_deltas(&delta.negative)
            ));
        }
    }
    if let Some(action) = trace.would_choose.as_ref() {
        lines.push(format!("trace_would_choose: {}", action.candidate_id()));
    } else {
        lines.push("trace_would_choose: -".to_string());
    }
    Ok(lines.join("\n"))
}

fn render_shop_plan_candidate_summary_v1(
    candidates: &[sts_simulator::ai::shop_policy_v1::ShopPlanCandidateV1],
) -> String {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for candidate in candidates {
        *counts.entry(format!("{:?}", candidate.role)).or_insert(0) += 1;
    }
    let counts = counts
        .into_iter()
        .map(|(role, count)| format!("{role}={count}"))
        .collect::<Vec<_>>()
        .join(", ");
    let examples = candidates
        .iter()
        .take(4)
        .map(|candidate| {
            format!(
                "{:?}:{:?}:tier{}:score{}:{}",
                candidate.role,
                candidate.evaluation.verdict,
                candidate.evaluation.tier,
                candidate.evaluation.score,
                candidate.plan.plan_id
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "candidate_plans: {} [{}] examples=[{}]",
        candidates.len(),
        counts,
        if examples.is_empty() { "-" } else { &examples }
    )
}

fn render_shop_plan_with_evaluation_v1(
    plan: &sts_simulator::ai::shop_policy_v1::ShopPlanV1,
    candidates: &[sts_simulator::ai::shop_policy_v1::ShopPlanCandidateV1],
) -> String {
    let evaluation = candidates
        .iter()
        .find(|candidate| candidate.plan.plan_id == plan.plan_id)
        .map(|candidate| render_shop_plan_evaluation_v1(&candidate.evaluation))
        .unwrap_or_else(|| "evaluation=-".to_string());
    format!("{} | {}", render_shop_plan_v1(plan), evaluation)
}

fn render_shop_plan_evaluation_v1(
    evaluation: &sts_simulator::ai::shop_policy_v1::ShopPlanEvaluationV1,
) -> String {
    let legacy_priority = evaluation
        .legacy_priority
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    format!(
        "evaluation={:?} tier={} score={} confidence={:.2} legacy_priority={} component_score=net:{:.1}/pos:{:.1}/neg:{:.1}/conf:{:.2} components=[{}] reasons=[{}]",
        evaluation.verdict,
        evaluation.tier,
        evaluation.score,
        evaluation.confidence,
        legacy_priority,
        evaluation.component_score.net,
        evaluation.component_score.positive,
        evaluation.component_score.negative,
        evaluation.component_score.confidence,
        render_shop_plan_components_v1(&evaluation.components),
        render_short_list(&evaluation.reasons)
    )
}

fn render_shop_plan_components_v1(
    components: &[sts_simulator::ai::shop_policy_v1::ShopPlanComponentV1],
) -> String {
    if components.is_empty() {
        return "-".to_string();
    }
    components
        .iter()
        .map(|component| {
            format!(
                "{:?}:{:.1}:{}",
                component.kind, component.amount, component.reason
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn render_shop_plan_v1(plan: &sts_simulator::ai::shop_policy_v1::ShopPlanV1) -> String {
    let steps = if plan.steps.is_empty() {
        "-".to_string()
    } else {
        plan.steps
            .iter()
            .map(render_shop_plan_step_v1)
            .collect::<Vec<_>>()
            .join(" then ")
    };
    let priority = plan
        .legacy_priority
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string());
    format!(
        "{} | kind={:?} source={:?} cost={} legacy_priority={} candidates=[{}] steps=[{}] reason={}",
        plan.label,
        plan.kind,
        plan.source,
        plan.total_gold_spent,
        priority,
        plan.candidate_ids.join(","),
        steps,
        plan.reason
    )
}

fn render_shop_plan_step_v1(step: &sts_simulator::ai::shop_policy_v1::ShopPlanStepV1) -> String {
    match *step {
        sts_simulator::ai::shop_policy_v1::ShopPlanStepV1::BuyCard { index, card, cost } => {
            format!("buy card {index} {:?} {cost}g", card)
        }
        sts_simulator::ai::shop_policy_v1::ShopPlanStepV1::BuyRelic { index, relic, cost } => {
            format!("buy relic {index} {relic:?} {cost}g")
        }
        sts_simulator::ai::shop_policy_v1::ShopPlanStepV1::BuyPotion {
            index,
            potion,
            cost,
        } => format!("buy potion {index} {potion:?} {cost}g"),
        sts_simulator::ai::shop_policy_v1::ShopPlanStepV1::RemoveCard {
            deck_index,
            card,
            cost,
        } => format!("purge deck {deck_index} {card:?} {cost}g"),
        sts_simulator::ai::shop_policy_v1::ShopPlanStepV1::LeaveShop => "leave shop".to_string(),
    }
}

fn inspect_shop_candidate_action_id(
    candidate: &sts_simulator::ai::shop_policy_v1::ShopCandidateEvidenceV1,
) -> String {
    use sts_simulator::ai::shop_policy_v1::{ShopPolicyClassV1, ShopPurchaseTargetV1};
    use sts_simulator::ai::strategic::CandidateAction;

    match candidate.purchase_target {
        Some(ShopPurchaseTargetV1::Card { index, card }) => CandidateAction::BuyCard {
            shop_index: index,
            card,
            gold: 0,
        }
        .candidate_id(),
        Some(ShopPurchaseTargetV1::Relic { index, relic }) => CandidateAction::BuyRelic {
            shop_index: index,
            relic,
            gold: 0,
        }
        .candidate_id(),
        Some(ShopPurchaseTargetV1::Potion { index, potion }) => CandidateAction::BuyPotion {
            shop_index: index,
            potion,
            gold: 0,
        }
        .candidate_id(),
        None if candidate.class == ShopPolicyClassV1::Leave => {
            CandidateAction::LeaveShop.candidate_id()
        }
        None => candidate
            .deck_index
            .zip(candidate.card)
            .map(|(deck_index, card)| CandidateAction::RemoveCard {
                deck_index,
                card,
                gold: None,
            })
            .map(|action| action.candidate_id())
            .unwrap_or_else(|| candidate.candidate_id.clone()),
    }
}

fn render_short_list(items: &[String]) -> String {
    if items.is_empty() {
        "-".to_string()
    } else {
        items.join(", ")
    }
}

fn render_ledger_deltas(items: &[sts_simulator::ai::strategic::LedgerDelta]) -> String {
    if items.is_empty() {
        return "-".to_string();
    }
    items
        .iter()
        .map(|delta| format!("{:?}:{:.2}:{}", delta.kind, delta.amount, delta.reason))
        .collect::<Vec<_>>()
        .join("; ")
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
    options.wall_ms = options.wall_ms.or(Some(args.search_wall_ms));
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
    fn campaign_cli_accepts_checkpoint_shop_evidence_inspection() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-act",
            "2",
            "--inspect-floor",
            "18",
            "--inspect-shop-evidence",
        ])
        .expect("args parse");

        assert_eq!(args.inspect_act, Some(2));
        assert_eq!(args.inspect_floor, Some(18));
        assert!(args.inspect_shop_evidence);
    }

    #[test]
    fn inspect_search_keeps_combat_search_wall_ms_option() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-search",
            "--combat-search-option",
            "wall_ms=5000",
        ])
        .expect("args parse");

        let options = inspect_search_options_from_args(&args).expect("options parse");

        assert_eq!(options.wall_ms, Some(5_000));
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_deck_mutation_inspection() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-deck-mutation",
        ]);

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert!(args.inspect_deck_mutation);
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_route_evidence_inspection() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-route-evidence",
        ]);

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert!(args.inspect_route_evidence);
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
