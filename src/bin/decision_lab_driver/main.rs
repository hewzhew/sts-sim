use std::collections::{BTreeMap, BTreeSet};

use clap::Parser;
use serde::Serialize;

use sts_simulator::eval::branch_experiment::{
    run_branch_experiment_v1, BranchExperimentBranchStatusV1, BranchExperimentConfigV1,
    BranchExperimentReportV1, BranchExperimentWallLimitPhaseV1,
};
use sts_simulator::eval::branch_experiment_retention::{
    BranchRetentionBudgetProfileV1, BranchRetentionSlotV1,
};
use sts_simulator::eval::neow_guided_prefix::{
    neow_guided_prefix_commands_v1, NeowGuidedPrefixConfigV1,
};
use sts_simulator::eval::run_control::{canonical_player_class, RunControlHpLossLimit};

#[derive(Debug, Parser)]
#[command(
    name = "decision_lab_driver",
    about = "Run autonomous noncombat decision experiments over a small seed batch"
)]
struct Args {
    #[arg(long = "seed", value_name = "SEED")]
    seeds: Vec<u64>,

    #[arg(long, value_name = "SEED")]
    seed_start: Option<u64>,

    #[arg(long, value_name = "N", default_value_t = 4)]
    count: usize,

    #[arg(long, default_value_t = 0)]
    ascension: u8,

    #[arg(long = "class", default_value = "ironclad")]
    player_class: String,

    #[arg(long, default_value_t = 24)]
    max_branches: usize,

    #[arg(long, default_value_t = 3)]
    max_depth: usize,

    #[arg(long, default_value_t = 1)]
    depth_retries: usize,

    #[arg(long, default_value_t = 1)]
    wall_retries: usize,

    #[arg(long, default_value_t = 3)]
    wall_retry_multiplier: u64,

    #[arg(long, default_value_t = 1)]
    combat_retries: usize,

    #[arg(long, default_value_t = 4)]
    combat_retry_multiplier: u64,

    #[arg(long, default_value_t = 128)]
    auto_max_ops: usize,

    #[arg(long, default_value_t = 10_000)]
    experiment_wall_ms: u64,

    #[arg(long)]
    search_max_nodes: Option<usize>,

    #[arg(long, default_value_t = 100)]
    search_wall_ms: u64,

    #[arg(long)]
    max_hp_loss: Option<String>,

    #[arg(long, default_value = "balanced")]
    retention_profile: String,

    #[arg(long, default_value_t = 6)]
    max_cases: usize,

    #[arg(long)]
    include_event_reward_skip: bool,

    #[arg(long)]
    no_neow_guidance: bool,

    #[arg(long)]
    json_lines: bool,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum DecisionLabCaseKindV1 {
    EngineeringIssue,
    NeedsCombatBudget,
    NeedsMoreBudget,
    NeedsHumanJudgment,
    NotEnoughEvidence,
    Routine,
}

impl DecisionLabCaseKindV1 {
    fn as_str(self) -> &'static str {
        match self {
            DecisionLabCaseKindV1::EngineeringIssue => "engineering_issue",
            DecisionLabCaseKindV1::NeedsCombatBudget => "needs_combat_budget",
            DecisionLabCaseKindV1::NeedsMoreBudget => "needs_more_budget",
            DecisionLabCaseKindV1::NeedsHumanJudgment => "needs_human_judgment",
            DecisionLabCaseKindV1::NotEnoughEvidence => "not_enough_evidence",
            DecisionLabCaseKindV1::Routine => "routine",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DecisionLabSignalsV1 {
    error: Option<String>,
    explored_branch_points: usize,
    depth_limit_reached: bool,
    branch_limit_hit: bool,
    wall_limit_hit: bool,
    wall_limit_phase: Option<BranchExperimentWallLimitPhaseV1>,
    frontier_group_limit_hit: bool,
    pruned_branch_count: usize,
    kept_branch_count: usize,
    frontier_group_count: usize,
    strategy_request_count: usize,
    human_strategy_request_count: usize,
}

#[derive(Clone, Debug, Serialize)]
struct DecisionLabCaseV1 {
    schema_name: &'static str,
    schema_version: u32,
    seed: u64,
    kind: DecisionLabCaseKindV1,
    decision: String,
    boundary: String,
    explored_branch_points: usize,
    kept_branches: usize,
    pruned_branches: usize,
    frontier_groups: usize,
    branch_limit_hit: bool,
    depth_limit_reached: bool,
    wall_limit_hit: bool,
    wall_limit_phase: Option<BranchExperimentWallLimitPhaseV1>,
    first_picks: Vec<String>,
    retention_lanes: Vec<String>,
    context_signals: Vec<String>,
    strategy_requests: Vec<String>,
    next_command: String,
    error: Option<String>,
}

struct DecisionLabSeedRunV1 {
    report: BranchExperimentReportV1,
    effective_config: BranchExperimentConfigV1,
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run(args) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<(), String> {
    let seeds = expand_lab_seeds(&args.seeds, args.seed_start, args.count)?;
    let mut cases = Vec::new();
    for &seed in &seeds {
        cases.push(run_seed_case(&args, seed));
    }

    if args.json_lines {
        for case in &cases {
            println!(
                "{}",
                serde_json::to_string(case).map_err(|err| err.to_string())?
            );
        }
    } else {
        print_compact_lab_report(&cases, args.max_cases);
    }
    Ok(())
}

fn run_seed_case(args: &Args, seed: u64) -> DecisionLabCaseV1 {
    match run_seed_report(args, seed) {
        Ok(run) => case_from_report(args, &run.report, &run.effective_config),
        Err(err) => case_from_error(args, seed, err),
    }
}

fn run_seed_report(args: &Args, seed: u64) -> Result<DecisionLabSeedRunV1, String> {
    let mut config = branch_config_for_seed(args, seed)?;
    let mut retries_used = 0usize;
    let mut wall_retries_used = 0usize;
    let mut combat_retries_used = 0usize;
    loop {
        let report = run_branch_experiment_v1(&config)?;
        if !should_retry_depth(&report, retries_used, args.depth_retries) {
            if should_retry_wall_budget(&report, wall_retries_used, args.wall_retries) {
                wall_retries_used = wall_retries_used.saturating_add(1);
                escalate_wall_retry_budget(&mut config, args.wall_retry_multiplier);
                continue;
            }
            if !should_retry_combat_budget(&report, combat_retries_used, args.combat_retries) {
                return Ok(DecisionLabSeedRunV1 {
                    report,
                    effective_config: config,
                });
            }
            combat_retries_used = combat_retries_used.saturating_add(1);
            escalate_combat_retry_budget(&mut config, args.combat_retry_multiplier);
            continue;
        }
        retries_used = retries_used.saturating_add(1);
        config.max_depth = config.max_depth.saturating_add(1);
    }
}

fn branch_config_for_seed(args: &Args, seed: u64) -> Result<BranchExperimentConfigV1, String> {
    let player_class = canonical_player_class(&args.player_class)?;
    let retention_budget_profile = args
        .retention_profile
        .parse::<BranchRetentionBudgetProfileV1>()?;
    let prefix_commands = neow_guided_prefix_commands(args, seed, player_class)?;
    Ok(BranchExperimentConfigV1 {
        seed,
        ascension_level: args.ascension,
        player_class,
        max_branches: args.max_branches,
        retention_budget_profile,
        max_depth: args.max_depth,
        auto_max_operations: args.auto_max_ops,
        experiment_wall_ms: Some(args.experiment_wall_ms),
        search_max_nodes: args.search_max_nodes,
        search_wall_ms: Some(args.search_wall_ms),
        search_max_hp_loss: parse_hp_loss_limit(args.max_hp_loss.as_deref())?,
        include_skip: true,
        include_event_reward_skip: args.include_event_reward_skip,
        prefix_commands,
        ..BranchExperimentConfigV1::default()
    })
}

fn neow_guided_prefix_commands(
    args: &Args,
    seed: u64,
    player_class: &'static str,
) -> Result<Vec<String>, String> {
    if args.no_neow_guidance {
        return Ok(vec!["0".to_string()]);
    }
    neow_guided_prefix_commands_v1(&NeowGuidedPrefixConfigV1 {
        seed,
        ascension_level: args.ascension,
        final_act: false,
        player_class,
        search_max_nodes: args.search_max_nodes,
        search_wall_ms: Some(args.search_wall_ms),
    })
}

fn expand_lab_seeds(
    explicit_seeds: &[u64],
    seed_start: Option<u64>,
    count: usize,
) -> Result<Vec<u64>, String> {
    if explicit_seeds.is_empty() && count == 0 {
        return Err("--count must be positive when no --seed is provided".to_string());
    }

    let mut seeds = explicit_seeds.to_vec();
    if let Some(start) = seed_start.or_else(|| explicit_seeds.is_empty().then_some(1)) {
        for offset in 0..count {
            seeds.push(start.saturating_add(offset as u64));
        }
    }
    let mut seen = BTreeSet::new();
    seeds.retain(|seed| seen.insert(*seed));
    Ok(seeds)
}

fn case_from_report(
    args: &Args,
    report: &BranchExperimentReportV1,
    effective_config: &BranchExperimentConfigV1,
) -> DecisionLabCaseV1 {
    let signals = signals_from_report(report);
    let kind = classify_lab_case(&signals);
    let branch_context_available = report.explored_branch_points > 0;
    DecisionLabCaseV1 {
        schema_name: "DecisionLabCaseV1",
        schema_version: 4,
        seed: report.seed,
        kind,
        decision: first_decision_kind(report),
        boundary: report
            .frontier_groups
            .first()
            .map(|group| group.boundary_title.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        explored_branch_points: report.explored_branch_points,
        kept_branches: report.branches.len(),
        pruned_branches: report.pruned_branch_count,
        frontier_groups: report.frontier_groups.len(),
        branch_limit_hit: report.branch_limit_hit || report.frontier_group_limit_hit,
        depth_limit_reached: signals.depth_limit_reached,
        wall_limit_hit: report.wall_limit_hit,
        wall_limit_phase: report.wall_limit_phase,
        first_picks: first_pick_labels(report, 6),
        retention_lanes: branch_context_available
            .then(|| retention_lane_counts(report))
            .unwrap_or_default(),
        context_signals: branch_context_available
            .then(|| context_signal_counts(report))
            .unwrap_or_default(),
        strategy_requests: strategy_request_counts(report),
        next_command: rerun_command_for_config(args, report.seed, effective_config),
        error: None,
    }
}

fn case_from_error(args: &Args, seed: u64, error: String) -> DecisionLabCaseV1 {
    DecisionLabCaseV1 {
        schema_name: "DecisionLabCaseV1",
        schema_version: 4,
        seed,
        kind: DecisionLabCaseKindV1::EngineeringIssue,
        decision: "error".to_string(),
        boundary: "error".to_string(),
        explored_branch_points: 0,
        kept_branches: 0,
        pruned_branches: 0,
        frontier_groups: 0,
        branch_limit_hit: false,
        depth_limit_reached: false,
        wall_limit_hit: false,
        wall_limit_phase: None,
        first_picks: Vec::new(),
        retention_lanes: Vec::new(),
        context_signals: Vec::new(),
        strategy_requests: Vec::new(),
        next_command: rerun_command(args, seed),
        error: Some(error),
    }
}

fn signals_from_report(report: &BranchExperimentReportV1) -> DecisionLabSignalsV1 {
    DecisionLabSignalsV1 {
        error: None,
        explored_branch_points: report.explored_branch_points,
        depth_limit_reached: report_depth_limit_reached(report),
        branch_limit_hit: report.branch_limit_hit,
        wall_limit_hit: report.wall_limit_hit,
        wall_limit_phase: report.wall_limit_phase,
        frontier_group_limit_hit: report.frontier_group_limit_hit,
        pruned_branch_count: report.pruned_branch_count,
        kept_branch_count: report.branches.len(),
        frontier_group_count: report.frontier_groups.len(),
        strategy_request_count: report.strategy_requests.len(),
        human_strategy_request_count: report
            .strategy_requests
            .iter()
            .filter(|request| strategy_request_needs_human_judgment(&request.kind))
            .count(),
    }
}

fn classify_lab_case(signals: &DecisionLabSignalsV1) -> DecisionLabCaseKindV1 {
    if signals.error.is_some() {
        return DecisionLabCaseKindV1::EngineeringIssue;
    }
    if signals.human_strategy_request_count > 0 {
        return DecisionLabCaseKindV1::NeedsHumanJudgment;
    }
    if signals.strategy_request_count > 0 {
        return DecisionLabCaseKindV1::NeedsCombatBudget;
    }
    if signals.branch_limit_hit || signals.wall_limit_hit || signals.frontier_group_limit_hit {
        return DecisionLabCaseKindV1::NeedsMoreBudget;
    }
    if signals.depth_limit_reached {
        return DecisionLabCaseKindV1::NeedsMoreBudget;
    }
    if signals.explored_branch_points == 0 {
        return DecisionLabCaseKindV1::NotEnoughEvidence;
    }
    if signals.pruned_branch_count > 0
        || signals.kept_branch_count > 1
        || signals.frontier_group_count > 1
    {
        return DecisionLabCaseKindV1::NeedsHumanJudgment;
    }
    DecisionLabCaseKindV1::Routine
}

fn strategy_request_needs_human_judgment(kind: &str) -> bool {
    kind != "combat_manual_or_budget"
}

fn should_retry_depth(
    report: &BranchExperimentReportV1,
    retries_used: usize,
    depth_retries: usize,
) -> bool {
    if retries_used >= depth_retries {
        return false;
    }
    let signals = signals_from_report(report);
    signals.depth_limit_reached
        && !signals.branch_limit_hit
        && !signals.wall_limit_hit
        && !signals.frontier_group_limit_hit
        && signals.strategy_request_count == 0
}

fn should_retry_combat_budget(
    report: &BranchExperimentReportV1,
    retries_used: usize,
    combat_retries: usize,
) -> bool {
    if retries_used >= combat_retries {
        return false;
    }
    let signals = signals_from_report(report);
    signals.strategy_request_count > 0
        && signals.human_strategy_request_count == 0
        && !signals.depth_limit_reached
        && !signals.branch_limit_hit
        && !signals.wall_limit_hit
        && !signals.frontier_group_limit_hit
}

fn should_retry_wall_budget(
    report: &BranchExperimentReportV1,
    retries_used: usize,
    wall_retries: usize,
) -> bool {
    if retries_used >= wall_retries {
        return false;
    }
    let signals = signals_from_report(report);
    signals.wall_limit_hit
        && signals.wall_limit_phase != Some(BranchExperimentWallLimitPhaseV1::FinalSettle)
        && !signals.depth_limit_reached
        && !signals.branch_limit_hit
        && !signals.frontier_group_limit_hit
        && signals.strategy_request_count == 0
}

fn escalate_combat_retry_budget(config: &mut BranchExperimentConfigV1, multiplier: u64) {
    let multiplier = multiplier.max(1);
    let current_wall_ms = config.search_wall_ms.unwrap_or(100);
    config.search_wall_ms = Some(current_wall_ms.saturating_mul(multiplier));

    let current_max_nodes = config.search_max_nodes.unwrap_or(20_000);
    let scaled_nodes =
        current_max_nodes.saturating_mul(usize::try_from(multiplier).unwrap_or(usize::MAX));
    config.search_max_nodes = Some(scaled_nodes);
}

fn escalate_wall_retry_budget(config: &mut BranchExperimentConfigV1, multiplier: u64) {
    let multiplier = multiplier.max(1);
    let current_wall_ms = config.experiment_wall_ms.unwrap_or(10_000);
    config.experiment_wall_ms = Some(current_wall_ms.saturating_mul(multiplier));
}

fn report_depth_limit_reached(report: &BranchExperimentReportV1) -> bool {
    report.max_depth > 0
        && report.branches.iter().any(|branch| {
            branch.status == BranchExperimentBranchStatusV1::Active
                && branch.choices.len() >= report.max_depth
        })
}

fn strategy_request_counts(report: &BranchExperimentReportV1) -> Vec<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for request in &report.strategy_requests {
        *counts.entry(request.kind.clone()).or_default() += request.branch_count;
    }
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    ranked
        .into_iter()
        .take(4)
        .map(|(kind, count)| format!("{kind}={count}"))
        .collect()
}

fn first_pick_labels(report: &BranchExperimentReportV1, max_labels: usize) -> Vec<String> {
    let mut labels = BTreeSet::new();
    for branch in &report.branches {
        if let Some(choice) = branch.choices.first() {
            labels.insert(choice.label.clone());
        }
    }
    labels.into_iter().take(max_labels).collect()
}

fn first_decision_kind(report: &BranchExperimentReportV1) -> String {
    report
        .branches
        .iter()
        .find_map(|branch| branch.choices.first().map(|choice| choice.kind.clone()))
        .unwrap_or_else(|| "unknown".to_string())
}

fn retention_lane_counts(report: &BranchExperimentReportV1) -> Vec<String> {
    let mut counts = BTreeMap::<BranchRetentionSlotV1, usize>::new();
    for branch in &report.branches {
        let lane = branch
            .retention
            .selected_by_slot
            .unwrap_or(BranchRetentionSlotV1::Diversity);
        *counts.entry(lane).or_default() += 1;
    }
    ordered_lane_counts(&counts)
}

fn context_signal_counts(report: &BranchExperimentReportV1) -> Vec<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for branch in &report.branches {
        let branch_signals = branch
            .retention
            .reasons
            .iter()
            .filter_map(|reason| reason.strip_prefix("context: "))
            .map(context_signal_key)
            .collect::<BTreeSet<_>>();
        for signal in branch_signals {
            *counts.entry(signal).or_default() += 1;
        }
    }
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    ranked
        .into_iter()
        .take(4)
        .map(|(signal, count)| format!("{signal}={count}"))
        .collect()
}

fn ordered_lane_counts(counts: &BTreeMap<BranchRetentionSlotV1, usize>) -> Vec<String> {
    RETENTION_LANE_DISPLAY_ORDER
        .iter()
        .filter_map(|lane| {
            counts
                .get(lane)
                .filter(|count| **count > 0)
                .map(|count| format!("{}={count}", retention_lane_name(*lane)))
        })
        .collect()
}

fn retention_lane_name(lane: BranchRetentionSlotV1) -> &'static str {
    match lane {
        BranchRetentionSlotV1::Package => "package",
        BranchRetentionSlotV1::EngineSetup => "engine_setup",
        BranchRetentionSlotV1::Scaling => "scaling",
        BranchRetentionSlotV1::DefenseEngine => "defense",
        BranchRetentionSlotV1::Survival => "survival",
        BranchRetentionSlotV1::Frontload => "frontload",
        BranchRetentionSlotV1::CleanDeck => "clean",
        BranchRetentionSlotV1::Diversity => "diversity",
    }
}

fn context_signal_key(reason: &str) -> String {
    match reason {
        "matches current frontload need" => "matches_current_frontload_need".to_string(),
        "matches current block or mitigation need" => "matches_current_block_need".to_string(),
        "matches current scaling need" => "matches_current_scaling_need".to_string(),
        "matches current draw/energy need" => "matches_current_draw_energy_need".to_string(),
        "matches current consistency need" => "matches_current_consistency_need".to_string(),
        "opens a setup path worth carrying forward" => "opens_setup_path".to_string(),
        "closes or supports an active package" => "supports_active_package".to_string(),
        "patches low-hp or route pressure" => "immediate_safety_patch".to_string(),
        other => other
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() {
                    ch.to_ascii_lowercase()
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .split('_')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("_"),
    }
}

const RETENTION_LANE_DISPLAY_ORDER: &[BranchRetentionSlotV1] = &[
    BranchRetentionSlotV1::Package,
    BranchRetentionSlotV1::EngineSetup,
    BranchRetentionSlotV1::Scaling,
    BranchRetentionSlotV1::DefenseEngine,
    BranchRetentionSlotV1::Survival,
    BranchRetentionSlotV1::Frontload,
    BranchRetentionSlotV1::CleanDeck,
    BranchRetentionSlotV1::Diversity,
];

fn rerun_command(args: &Args, seed: u64) -> String {
    rerun_command_with_depth(args, seed, args.max_depth)
}

fn rerun_command_with_depth(args: &Args, seed: u64, max_depth: usize) -> String {
    let config = BranchExperimentConfigV1 {
        max_depth,
        search_wall_ms: Some(args.search_wall_ms),
        search_max_nodes: args.search_max_nodes,
        ..BranchExperimentConfigV1::default()
    };
    rerun_command_for_config(args, seed, &config)
}

fn rerun_command_for_config(
    args: &Args,
    seed: u64,
    effective_config: &BranchExperimentConfigV1,
) -> String {
    let mut tokens = vec![
        "cargo".to_string(),
        "run".to_string(),
        "--quiet".to_string(),
        "--bin".to_string(),
        "branch_experiment_driver".to_string(),
        "--".to_string(),
        "--seed".to_string(),
        seed.to_string(),
        "--ascension".to_string(),
        args.ascension.to_string(),
        "--class".to_string(),
        command_arg(&args.player_class),
        "--max-depth".to_string(),
        effective_config.max_depth.to_string(),
        "--max-branches".to_string(),
        args.max_branches.to_string(),
        "--auto-max-ops".to_string(),
        args.auto_max_ops.to_string(),
        "--experiment-wall-ms".to_string(),
        effective_config
            .experiment_wall_ms
            .unwrap_or(args.experiment_wall_ms)
            .to_string(),
        "--search-wall-ms".to_string(),
        effective_config
            .search_wall_ms
            .unwrap_or(args.search_wall_ms)
            .to_string(),
        "--retention-profile".to_string(),
        command_arg(&args.retention_profile),
        "--branch-examples".to_string(),
        "8".to_string(),
    ];
    if let Some(max_nodes) = effective_config.search_max_nodes.or(args.search_max_nodes) {
        tokens.push("--search-max-nodes".to_string());
        tokens.push(max_nodes.to_string());
    }
    if let Some(max_hp_loss) = args.max_hp_loss.as_deref() {
        tokens.push("--max-hp-loss".to_string());
        tokens.push(command_arg(max_hp_loss));
    }
    if args.include_event_reward_skip {
        tokens.push("--include-event-reward-skip".to_string());
    }
    if let Ok(player_class) = canonical_player_class(&args.player_class) {
        if let Ok(prefix_commands) = neow_guided_prefix_commands(args, seed, player_class) {
            for command in prefix_commands {
                tokens.push("--prefix".to_string());
                tokens.push(command_arg(&command));
            }
        }
    }
    tokens.join(" ")
}

fn command_arg(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | '\\' | ':'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "''"))
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

fn print_compact_lab_report(cases: &[DecisionLabCaseV1], max_cases: usize) {
    println!(
        "DecisionLabV1 cases={} shown={}",
        cases.len(),
        cases.len().min(max_cases)
    );
    let counts = kind_counts(cases);
    println!(
        "Summary: {}",
        counts
            .into_iter()
            .map(|(kind, count)| format!("{}={count}", kind.as_str()))
            .collect::<Vec<_>>()
            .join(" ")
    );
    if let Some(line) = render_hot_context_signals_line(cases) {
        println!("{line}");
    }
    if let Some(line) = render_hot_budget_requests_line(cases) {
        println!("{line}");
    }
    if let Some(line) = render_hot_strategy_requests_line(cases) {
        println!("{line}");
    }
    println!();
    println!("Cases:");
    for case in prioritized_cases(cases).into_iter().take(max_cases) {
        println!("{}", render_case_line(case));
        if let Some(error) = case.error.as_ref() {
            println!("    error: {error}");
        }
    }
    if cases.len() > max_cases {
        println!(
            "  ... {} more case(s); use --max-cases N or --json-lines",
            cases.len() - max_cases
        );
    }
}

fn kind_counts(cases: &[DecisionLabCaseV1]) -> BTreeMap<DecisionLabCaseKindV1, usize> {
    let mut counts = BTreeMap::new();
    for case in cases {
        *counts.entry(case.kind).or_insert(0) += 1;
    }
    counts
}

fn prioritized_cases(cases: &[DecisionLabCaseV1]) -> Vec<&DecisionLabCaseV1> {
    let mut ordered = cases.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|case| (case_priority(case.kind), case.seed));
    ordered
}

fn case_priority(kind: DecisionLabCaseKindV1) -> u8 {
    match kind {
        DecisionLabCaseKindV1::EngineeringIssue => 0,
        DecisionLabCaseKindV1::NeedsHumanJudgment => 1,
        DecisionLabCaseKindV1::NeedsCombatBudget => 2,
        DecisionLabCaseKindV1::NeedsMoreBudget => 3,
        DecisionLabCaseKindV1::NotEnoughEvidence => 4,
        DecisionLabCaseKindV1::Routine => 5,
    }
}

fn render_case_line(case: &DecisionLabCaseV1) -> String {
    format!(
        "  seed={} kind={} decision={} frontier={} limits=[{}] branch_points={} kept={} pruned={} groups={} first_picks=[{}] lanes=[{}] ctx=[{}] requests=[{}]",
        case.seed,
        case.kind.as_str(),
        case.decision,
        case.boundary,
        case_limit_flags(case),
        case.explored_branch_points,
        case.kept_branches,
        case.pruned_branches,
        case.frontier_groups,
        if case.first_picks.is_empty() {
            "-".to_string()
        } else {
            case.first_picks.join(", ")
        },
        if case.retention_lanes.is_empty() {
            "-".to_string()
        } else {
            case.retention_lanes.join(" ")
        },
        if case.context_signals.is_empty() {
            "-".to_string()
        } else {
            case.context_signals.join(" ")
        },
        if case.strategy_requests.is_empty() {
            "-".to_string()
        } else {
            case.strategy_requests.join(" ")
        }
    )
}

fn case_limit_flags(case: &DecisionLabCaseV1) -> String {
    let mut flags = Vec::new();
    if case.depth_limit_reached {
        flags.push("depth");
    }
    if case.branch_limit_hit {
        flags.push("branch");
    }
    if case.wall_limit_hit {
        match case.wall_limit_phase {
            Some(BranchExperimentWallLimitPhaseV1::Expansion) => flags.push("wall:expansion"),
            Some(BranchExperimentWallLimitPhaseV1::FinalSettle) => flags.push("wall:settle"),
            None => flags.push("wall"),
        }
    }
    if case
        .strategy_requests
        .iter()
        .any(|request| request.starts_with("combat_manual_or_budget="))
    {
        flags.push("combat");
    }
    if flags.is_empty() {
        "-".to_string()
    } else {
        flags.join(",")
    }
}

fn render_hot_context_signals_line(cases: &[DecisionLabCaseV1]) -> Option<String> {
    let counts = aggregate_context_signals(cases);
    if counts.is_empty() {
        return None;
    }
    Some(format!("Hot context signals: {}", counts.join(" ")))
}

fn aggregate_context_signals(cases: &[DecisionLabCaseV1]) -> Vec<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for case in cases {
        if !matches!(
            case.kind,
            DecisionLabCaseKindV1::NeedsHumanJudgment
                | DecisionLabCaseKindV1::NeedsCombatBudget
                | DecisionLabCaseKindV1::NeedsMoreBudget
        ) {
            continue;
        }
        for signal in &case.context_signals {
            let Some((key, count)) = split_count_token(signal) else {
                continue;
            };
            *counts.entry(key.to_string()).or_default() += count;
        }
    }
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    ranked
        .into_iter()
        .take(5)
        .map(|(signal, count)| format!("{signal}={count}"))
        .collect()
}

fn render_hot_strategy_requests_line(cases: &[DecisionLabCaseV1]) -> Option<String> {
    render_hot_request_line(
        cases,
        "Hot strategy requests",
        strategy_request_needs_human_judgment,
    )
}

fn render_hot_budget_requests_line(cases: &[DecisionLabCaseV1]) -> Option<String> {
    render_hot_request_line(cases, "Hot budget requests", |kind| {
        !strategy_request_needs_human_judgment(kind)
    })
}

fn render_hot_request_line<F>(
    cases: &[DecisionLabCaseV1],
    title: &str,
    include_kind: F,
) -> Option<String>
where
    F: Fn(&str) -> bool,
{
    let mut counts = BTreeMap::<String, usize>::new();
    for case in cases {
        for token in &case.strategy_requests {
            let Some((key, count)) = split_count_token(token) else {
                continue;
            };
            if !include_kind(key) {
                continue;
            }
            *counts.entry(key.to_string()).or_default() += count;
        }
    }
    if counts.is_empty() {
        return None;
    }
    let mut ranked = counts.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    Some(format!(
        "{title}: {}",
        ranked
            .into_iter()
            .take(5)
            .map(|(kind, count)| format!("{kind}={count}"))
            .collect::<Vec<_>>()
            .join(" ")
    ))
}

fn split_count_token(token: &str) -> Option<(&str, usize)> {
    let (key, count) = token.rsplit_once('=')?;
    if key.is_empty() {
        return None;
    }
    let count = count.parse().ok()?;
    Some((key, count))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::eval::branch_experiment::BranchExperimentStrategyRequestV1;

    #[test]
    fn expands_explicit_seeds_before_generated_range() {
        let seeds = expand_lab_seeds(&[42, 99], Some(1000), 3).expect("seeds expand");

        assert_eq!(seeds, vec![42, 99, 1000, 1001, 1002]);
    }

    #[test]
    fn classifies_missing_branch_point_as_not_enough_evidence() {
        let kind = classify_lab_case(&DecisionLabSignalsV1 {
            error: None,
            explored_branch_points: 0,
            depth_limit_reached: false,
            branch_limit_hit: false,
            wall_limit_hit: false,
            wall_limit_phase: None,
            frontier_group_limit_hit: false,
            pruned_branch_count: 0,
            kept_branch_count: 1,
            frontier_group_count: 1,
            strategy_request_count: 0,
            human_strategy_request_count: 0,
        });

        assert_eq!(kind, DecisionLabCaseKindV1::NotEnoughEvidence);
    }

    #[test]
    fn classifies_strategy_request_without_branch_point_as_human_judgment() {
        let kind = classify_lab_case(&DecisionLabSignalsV1 {
            error: None,
            explored_branch_points: 0,
            depth_limit_reached: false,
            branch_limit_hit: false,
            wall_limit_hit: false,
            wall_limit_phase: None,
            frontier_group_limit_hit: false,
            pruned_branch_count: 0,
            kept_branch_count: 1,
            frontier_group_count: 1,
            strategy_request_count: 1,
            human_strategy_request_count: 1,
        });

        assert_eq!(kind, DecisionLabCaseKindV1::NeedsHumanJudgment);
    }

    #[test]
    fn classifies_combat_budget_request_separately() {
        let kind = classify_lab_case(&DecisionLabSignalsV1 {
            error: None,
            explored_branch_points: 0,
            depth_limit_reached: false,
            branch_limit_hit: false,
            wall_limit_hit: false,
            wall_limit_phase: None,
            frontier_group_limit_hit: false,
            pruned_branch_count: 0,
            kept_branch_count: 1,
            frontier_group_count: 1,
            strategy_request_count: 1,
            human_strategy_request_count: 0,
        });

        assert_eq!(kind, DecisionLabCaseKindV1::NeedsCombatBudget);
    }

    #[test]
    fn classifies_human_strategy_request_before_generic_budget_limits() {
        let kind = classify_lab_case(&DecisionLabSignalsV1 {
            error: None,
            explored_branch_points: 2,
            depth_limit_reached: false,
            branch_limit_hit: true,
            wall_limit_hit: true,
            wall_limit_phase: Some(BranchExperimentWallLimitPhaseV1::Expansion),
            frontier_group_limit_hit: false,
            pruned_branch_count: 10,
            kept_branch_count: 16,
            frontier_group_count: 2,
            strategy_request_count: 1,
            human_strategy_request_count: 1,
        });

        assert_eq!(kind, DecisionLabCaseKindV1::NeedsHumanJudgment);
    }

    #[test]
    fn classifies_combat_budget_request_before_generic_budget_limits() {
        let kind = classify_lab_case(&DecisionLabSignalsV1 {
            error: None,
            explored_branch_points: 2,
            depth_limit_reached: false,
            branch_limit_hit: true,
            wall_limit_hit: true,
            wall_limit_phase: Some(BranchExperimentWallLimitPhaseV1::Expansion),
            frontier_group_limit_hit: false,
            pruned_branch_count: 10,
            kept_branch_count: 16,
            frontier_group_count: 2,
            strategy_request_count: 1,
            human_strategy_request_count: 0,
        });

        assert_eq!(kind, DecisionLabCaseKindV1::NeedsCombatBudget);
    }

    #[test]
    fn retries_pure_combat_budget_reports() {
        let mut report = test_report();
        report.strategy_requests = vec![BranchExperimentStrategyRequestV1 {
            kind: "combat_manual_or_budget".to_string(),
            boundary_title: "Combat".to_string(),
            branch_count: 1,
            representative_branch_id: "root".to_string(),
            act: 1,
            floor: 1,
            stop_reasons: vec!["combat search did not find an executable complete win".to_string()],
            examples: vec!["-".to_string()],
            next_card_reward_offer: None,
            boundary_details: Vec::new(),
            suggested_action: "raise combat search budget".to_string(),
        }];

        assert!(should_retry_combat_budget(&report, 0, 1));
        assert!(!should_retry_combat_budget(&report, 1, 1));
    }

    #[test]
    fn does_not_retry_combat_when_human_strategy_request_is_present() {
        let mut report = test_report();
        report.strategy_requests = vec![BranchExperimentStrategyRequestV1 {
            kind: "card_reward_policy_gap".to_string(),
            boundary_title: "Card Reward".to_string(),
            branch_count: 1,
            representative_branch_id: "root".to_string(),
            act: 1,
            floor: 1,
            stop_reasons: vec!["Card Reward".to_string()],
            examples: vec!["Shockwave".to_string()],
            next_card_reward_offer: Some(vec!["Shockwave".to_string()]),
            boundary_details: Vec::new(),
            suggested_action: "provide card reward policy".to_string(),
        }];

        assert!(!should_retry_combat_budget(&report, 0, 1));
    }

    #[test]
    fn combat_retry_scales_search_budget() {
        let mut config = BranchExperimentConfigV1 {
            search_wall_ms: Some(50),
            search_max_nodes: Some(5_000),
            ..BranchExperimentConfigV1::default()
        };

        escalate_combat_retry_budget(&mut config, 4);

        assert_eq!(config.search_wall_ms, Some(200));
        assert_eq!(config.search_max_nodes, Some(20_000));
    }

    #[test]
    fn retries_pure_wall_limited_reports() {
        let mut report = test_report();
        report.wall_limit_hit = true;
        report.wall_limit_phase = Some(BranchExperimentWallLimitPhaseV1::Expansion);

        assert!(should_retry_wall_budget(&report, 0, 1));
        assert!(!should_retry_wall_budget(&report, 1, 1));
    }

    #[test]
    fn does_not_retry_final_settle_wall_limited_reports() {
        let mut report = test_report();
        report.wall_limit_hit = true;
        report.wall_limit_phase = Some(BranchExperimentWallLimitPhaseV1::FinalSettle);

        assert!(!should_retry_wall_budget(&report, 0, 1));
    }

    #[test]
    fn does_not_retry_wall_when_branch_limit_is_also_hit() {
        let mut report = test_report();
        report.wall_limit_hit = true;
        report.branch_limit_hit = true;

        assert!(!should_retry_wall_budget(&report, 0, 1));
    }

    #[test]
    fn wall_retry_scales_experiment_budget() {
        let mut config = BranchExperimentConfigV1 {
            experiment_wall_ms: Some(5_000),
            ..BranchExperimentConfigV1::default()
        };

        escalate_wall_retry_budget(&mut config, 3);

        assert_eq!(config.experiment_wall_ms, Some(15_000));
    }

    #[test]
    fn classifies_budget_limited_experiment_separately() {
        let kind = classify_lab_case(&DecisionLabSignalsV1 {
            error: None,
            explored_branch_points: 2,
            depth_limit_reached: false,
            branch_limit_hit: true,
            wall_limit_hit: false,
            wall_limit_phase: None,
            frontier_group_limit_hit: false,
            pruned_branch_count: 10,
            kept_branch_count: 24,
            frontier_group_count: 2,
            strategy_request_count: 0,
            human_strategy_request_count: 0,
        });

        assert_eq!(kind, DecisionLabCaseKindV1::NeedsMoreBudget);
    }

    #[test]
    fn classifies_depth_exhausted_active_frontier_as_more_budget() {
        let kind = classify_lab_case(&DecisionLabSignalsV1 {
            error: None,
            explored_branch_points: 3,
            depth_limit_reached: true,
            branch_limit_hit: false,
            wall_limit_hit: false,
            wall_limit_phase: None,
            frontier_group_limit_hit: false,
            pruned_branch_count: 0,
            kept_branch_count: 9,
            frontier_group_count: 1,
            strategy_request_count: 0,
            human_strategy_request_count: 0,
        });

        assert_eq!(kind, DecisionLabCaseKindV1::NeedsMoreBudget);
    }

    #[test]
    fn classifies_multi_branch_decision_as_human_judgment_candidate() {
        let kind = classify_lab_case(&DecisionLabSignalsV1 {
            error: None,
            explored_branch_points: 1,
            depth_limit_reached: false,
            branch_limit_hit: false,
            wall_limit_hit: false,
            wall_limit_phase: None,
            frontier_group_limit_hit: false,
            pruned_branch_count: 0,
            kept_branch_count: 3,
            frontier_group_count: 1,
            strategy_request_count: 0,
            human_strategy_request_count: 0,
        });

        assert_eq!(kind, DecisionLabCaseKindV1::NeedsHumanJudgment);
    }

    #[test]
    fn rerun_command_preserves_lab_budget_and_prefix() {
        let args = Args::try_parse_from([
            "decision_lab_driver",
            "--max-depth",
            "2",
            "--max-branches",
            "16",
            "--auto-max-ops",
            "64",
            "--experiment-wall-ms",
            "8000",
            "--search-wall-ms",
            "1000",
            "--search-max-nodes",
            "50000",
            "--retention-profile",
            "survival",
            "--include-event-reward-skip",
        ])
        .expect("args parse");

        let command = rerun_command(&args, 521);

        assert!(command.contains("--max-depth 2"));
        assert!(command.contains("--max-branches 16"));
        assert!(command.contains("--auto-max-ops 64"));
        assert!(command.contains("--experiment-wall-ms 8000"));
        assert!(command.contains("--search-wall-ms 1000"));
        assert!(command.contains("--search-max-nodes 50000"));
        assert!(command.contains("--retention-profile survival"));
        assert!(command.contains("--include-event-reward-skip"));
        assert!(command.contains("--prefix 0"));
    }

    #[test]
    fn rerun_command_uses_effective_search_budget_after_retries() {
        let args = Args::try_parse_from([
            "decision_lab_driver",
            "--search-wall-ms",
            "50",
            "--search-max-nodes",
            "5000",
        ])
        .expect("args parse");
        let effective_config = BranchExperimentConfigV1 {
            max_depth: 4,
            experiment_wall_ms: Some(15_000),
            search_wall_ms: Some(200),
            search_max_nodes: Some(20_000),
            ..BranchExperimentConfigV1::default()
        };

        let command = rerun_command_for_config(&args, 521, &effective_config);

        assert!(command.contains("--max-depth 4"));
        assert!(command.contains("--experiment-wall-ms 15000"));
        assert!(command.contains("--search-wall-ms 200"));
        assert!(command.contains("--search-max-nodes 20000"));
    }

    #[test]
    fn branch_config_uses_neow_guidance_after_intro_by_default() {
        let args = Args::try_parse_from(["decision_lab_driver"]).expect("args parse");
        let config = branch_config_for_seed(&args, 521).expect("config builds");

        assert_eq!(
            config.prefix_commands.first().map(String::as_str),
            Some("0")
        );
        assert_eq!(config.prefix_commands.len(), 2);
    }

    #[test]
    fn branch_config_can_disable_neow_guidance() {
        let args = Args::try_parse_from(["decision_lab_driver", "--no-neow-guidance"])
            .expect("args parse");
        let config = branch_config_for_seed(&args, 521).expect("config builds");

        assert_eq!(config.prefix_commands, vec!["0"]);
    }

    #[test]
    fn branch_config_applies_neow_followup_selection_when_guidance_opens_run_selection() {
        let args = Args::try_parse_from(["decision_lab_driver"]).expect("args parse");
        let config = branch_config_for_seed(&args, 527).expect("config builds");

        assert!(
            config
                .prefix_commands
                .iter()
                .any(|command| command.starts_with("select ")),
            "seed 527 previously stopped at Neow follow-up run_selection"
        );
    }

    #[test]
    fn render_case_line_includes_lane_and_context_summaries() {
        let case = DecisionLabCaseV1 {
            schema_name: "DecisionLabCaseV1",
            schema_version: 4,
            seed: 42,
            kind: DecisionLabCaseKindV1::NeedsHumanJudgment,
            decision: "card_reward".to_string(),
            boundary: "Combat".to_string(),
            explored_branch_points: 2,
            kept_branches: 4,
            pruned_branches: 1,
            frontier_groups: 1,
            branch_limit_hit: false,
            depth_limit_reached: false,
            wall_limit_hit: false,
            wall_limit_phase: None,
            first_picks: vec!["Pommel Strike".to_string()],
            retention_lanes: vec!["frontload=2".to_string(), "package=1".to_string()],
            context_signals: vec![
                "matches_current_frontload_need=2".to_string(),
                "opens_setup_path=1".to_string(),
            ],
            strategy_requests: vec!["card_reward_policy_gap=3".to_string()],
            next_command: "rerun".to_string(),
            error: None,
        };

        let rendered = render_case_line(&case);

        assert!(rendered.contains("lanes=[frontload=2 package=1]"));
        assert!(rendered.contains("ctx=[matches_current_frontload_need=2 opens_setup_path=1]"));
        assert!(rendered.contains("requests=[card_reward_policy_gap=3]"));
    }

    #[test]
    fn render_case_line_distinguishes_final_settle_wall_limit() {
        let mut case = test_case(DecisionLabCaseKindV1::NeedsMoreBudget, &[]);
        case.wall_limit_hit = true;
        case.wall_limit_phase = Some(BranchExperimentWallLimitPhaseV1::FinalSettle);

        let rendered = render_case_line(&case);

        assert!(rendered.contains("limits=[wall:settle]"));
    }

    #[test]
    fn context_signal_counts_only_uses_context_prefixed_retention_reasons() {
        let report = BranchExperimentReportV1 {
            branches: vec![
                test_branch_with_reasons(&[
                    "contains immediate combat output",
                    "context: matches current frontload need",
                ]),
                test_branch_with_reasons(&[
                    "context: matches current frontload need",
                    "context: opens a setup path worth carrying forward",
                ]),
            ],
            ..test_report()
        };

        let signals = context_signal_counts(&report);

        assert_eq!(
            signals,
            vec![
                "matches_current_frontload_need=2".to_string(),
                "opens_setup_path=1".to_string(),
            ]
        );
    }

    #[test]
    fn hot_context_signal_line_aggregates_actionable_case_kinds() {
        let cases = vec![
            test_case(
                DecisionLabCaseKindV1::NeedsHumanJudgment,
                &["matches_current_frontload_need=3", "opens_setup_path=1"],
            ),
            test_case(
                DecisionLabCaseKindV1::NeedsMoreBudget,
                &["matches_current_frontload_need=2"],
            ),
            test_case(
                DecisionLabCaseKindV1::NotEnoughEvidence,
                &["matches_current_frontload_need=99"],
            ),
        ];

        let line = render_hot_context_signals_line(&cases).expect("hot context line");

        assert_eq!(
            line,
            "Hot context signals: matches_current_frontload_need=5 opens_setup_path=1"
        );
    }

    #[test]
    fn hot_strategy_request_line_aggregates_actionable_requests() {
        let mut card_case = test_case(
            DecisionLabCaseKindV1::NeedsHumanJudgment,
            &["matches_current_frontload_need=1"],
        );
        card_case.strategy_requests = vec!["card_reward_policy_gap=4".to_string()];
        let mut event_case = test_case(DecisionLabCaseKindV1::NeedsHumanJudgment, &[]);
        event_case.strategy_requests = vec![
            "event_strategy=2".to_string(),
            "card_reward_policy_gap=1".to_string(),
        ];

        let line = render_hot_strategy_requests_line(&[card_case, event_case])
            .expect("hot strategy request line");

        assert_eq!(
            line,
            "Hot strategy requests: card_reward_policy_gap=5 event_strategy=2"
        );
    }

    #[test]
    fn hot_strategy_request_line_excludes_budget_requests() {
        let mut case = test_case(DecisionLabCaseKindV1::NeedsMoreBudget, &[]);
        case.strategy_requests = vec!["combat_manual_or_budget=7".to_string()];

        assert_eq!(render_hot_strategy_requests_line(&[case]), None);
    }

    #[test]
    fn hot_budget_request_line_aggregates_combat_budget_requests() {
        let mut case = test_case(DecisionLabCaseKindV1::NeedsMoreBudget, &[]);
        case.strategy_requests = vec!["combat_manual_or_budget=7".to_string()];

        let line = render_hot_budget_requests_line(&[case]).expect("hot budget line");

        assert_eq!(line, "Hot budget requests: combat_manual_or_budget=7");
    }

    fn test_report() -> BranchExperimentReportV1 {
        BranchExperimentReportV1 {
            schema_name: "BranchExperimentV1".to_string(),
            schema_version:
                sts_simulator::eval::branch_experiment::BRANCH_EXPERIMENT_SCHEMA_VERSION,
            label_role: "diagnostic_not_teacher_label".to_string(),
            policy_quality_claim: false,
            seed: 1,
            replay_trace_path: None,
            replay_trace_applied_steps: 0,
            replay_trace_stop: None,
            max_branches: 4,
            max_depth: 1,
            retention_profile: BranchRetentionBudgetProfileV1::Balanced,
            explored_branch_points: 1,
            branch_limit_hit: false,
            frontier_group_limit_hit: false,
            wall_limit_hit: false,
            wall_limit_phase: None,
            elapsed_wall_ms: 0,
            pruned_branch_count: 0,
            pruned_first_pick_counts: Vec::new(),
            pruned_branch_summary: Default::default(),
            reward_option_portfolios: Vec::new(),
            strategy_requests: Vec::new(),
            frontier_groups: Vec::new(),
            branches: Vec::new(),
        }
    }

    fn test_case(kind: DecisionLabCaseKindV1, context_signals: &[&str]) -> DecisionLabCaseV1 {
        DecisionLabCaseV1 {
            schema_name: "DecisionLabCaseV1",
            schema_version: 4,
            seed: 1,
            kind,
            decision: "card_reward".to_string(),
            boundary: "Combat".to_string(),
            explored_branch_points: 1,
            kept_branches: 1,
            pruned_branches: 0,
            frontier_groups: 1,
            branch_limit_hit: false,
            depth_limit_reached: false,
            wall_limit_hit: false,
            wall_limit_phase: None,
            first_picks: Vec::new(),
            retention_lanes: Vec::new(),
            context_signals: context_signals
                .iter()
                .map(|signal| (*signal).to_string())
                .collect(),
            strategy_requests: Vec::new(),
            next_command: "rerun".to_string(),
            error: None,
        }
    }

    fn test_branch_with_reasons(
        reasons: &[&str],
    ) -> sts_simulator::eval::branch_experiment::BranchExperimentBranchReportV1 {
        sts_simulator::eval::branch_experiment::BranchExperimentBranchReportV1 {
            branch_id: "b".to_string(),
            status: sts_simulator::eval::branch_experiment::BranchExperimentBranchStatusV1::Active,
            rank_key: 0,
            retention: sts_simulator::eval::branch_experiment_retention::BranchRetentionDecisionV1 {
                primary_slot: sts_simulator::eval::branch_experiment_retention::BranchRetentionSlotV1::Frontload,
                selected_by_slot: Some(sts_simulator::eval::branch_experiment_retention::BranchRetentionSlotV1::Frontload),
                slots: vec![sts_simulator::eval::branch_experiment_retention::BranchRetentionSlotV1::Frontload],
                reasons: reasons.iter().map(|reason| (*reason).to_string()).collect(),
            },
            choices: Vec::new(),
            stop_reason: "test".to_string(),
            summary: sts_simulator::eval::branch_experiment::BranchExperimentRunSummaryV1 {
                act: 1,
                floor: 1,
                hp: 80,
                max_hp: 80,
                gold: 99,
                deck_count: 10,
                relic_count: 1,
                potion_count: 0,
                formation_stage: sts_simulator::ai::noncombat_strategy_v1::StrategyDeckFormationStageV1::StarterShell,
                formation_needs: Vec::new(),
                formation_strengths: Vec::new(),
                trajectory: Default::default(),
                boundary_title: "Combat".to_string(),
            },
            frontier: sts_simulator::eval::branch_experiment::BranchExperimentFrontierV1 {
                key: "frontier".to_string(),
                act: 1,
                floor: 1,
                boundary_title: "Combat".to_string(),
                card_rng_counter: 0,
                card_blizz_randomizer: 0,
                next_card_reward_offer: None,
                lineage: sts_simulator::eval::branch_experiment::BranchExperimentLineageV1 {
                    visibility: "test".to_string(),
                    public_policy_input: false,
                    direct_pick_consumes_card_rng: false,
                    same_reward_offer_lineage_key: "test".to_string(),
                    reward_screen_context: "test".to_string(),
                    reward_count_modifiers: Vec::new(),
                    card_pool_modifiers: Vec::new(),
                    rarity_modifiers: Vec::new(),
                    preview_modifiers: Vec::new(),
                    sequence_breakers_present: Vec::new(),
                },
            },
            boundary_details: Vec::new(),
        }
    }
}
