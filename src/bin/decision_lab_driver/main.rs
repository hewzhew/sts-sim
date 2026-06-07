use std::collections::{BTreeMap, BTreeSet};

use clap::Parser;
use serde::Serialize;

use sts_simulator::ai::neow_policy_v1::{
    choices_from_event_options_v1, neow_followup_selection_v1, neow_map_features_from_run_state_v1,
    rank_neow_choices_v1, NeowDecisionInputV1, NeowGuidanceConfigV1,
};
use sts_simulator::content::events::neow;
use sts_simulator::eval::branch_experiment::{
    run_branch_experiment_v1, BranchExperimentConfigV1, BranchExperimentReportV1,
};
use sts_simulator::eval::branch_experiment_retention::BranchRetentionBudgetProfileV1;
use sts_simulator::eval::run_control::{
    canonical_player_class, parse_run_control_command, RunControlConfig, RunControlHpLossLimit,
    RunControlSession,
};
use sts_simulator::state::events::EventId;

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
    NeedsMoreBudget,
    NeedsHumanJudgment,
    NotEnoughEvidence,
    Routine,
}

impl DecisionLabCaseKindV1 {
    fn as_str(self) -> &'static str {
        match self {
            DecisionLabCaseKindV1::EngineeringIssue => "engineering_issue",
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
    branch_limit_hit: bool,
    wall_limit_hit: bool,
    frontier_group_limit_hit: bool,
    pruned_branch_count: usize,
    kept_branch_count: usize,
    frontier_group_count: usize,
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
    wall_limit_hit: bool,
    first_picks: Vec<String>,
    next_command: String,
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
        Ok(report) => case_from_report(&report),
        Err(err) => case_from_error(seed, err),
    }
}

fn run_seed_report(args: &Args, seed: u64) -> Result<BranchExperimentReportV1, String> {
    run_branch_experiment_v1(&branch_config_for_seed(args, seed)?)
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
    let mut prefix = vec!["0".to_string()];
    if args.no_neow_guidance {
        return Ok(prefix);
    }

    let mut session = RunControlSession::new(RunControlConfig {
        seed,
        ascension_level: args.ascension,
        final_act: false,
        player_class,
        search_max_nodes: args.search_max_nodes,
        search_wall_ms: Some(args.search_wall_ms),
        ..RunControlConfig::default()
    });
    session.apply_command(parse_run_control_command("0")?)?;
    let Some(event_state) = session.run_state.event_state.as_ref() else {
        return Ok(prefix);
    };
    if event_state.id != EventId::Neow || event_state.current_screen != 1 {
        return Ok(prefix);
    }

    let options = neow::get_options(&session.run_state, event_state);
    let trace = rank_neow_choices_v1(NeowDecisionInputV1 {
        player_class: player_class.to_string(),
        map: neow_map_features_from_run_state_v1(&session.run_state),
        choices: choices_from_event_options_v1(&options),
        config: NeowGuidanceConfigV1::default(),
    });
    if let Some(selected) = trace.selected() {
        let neow_choice_command = selected.index.to_string();
        prefix.push(neow_choice_command.clone());
        session.apply_command(parse_run_control_command(&neow_choice_command)?)?;
        if is_neow_followup_selection(&session) {
            if let sts_simulator::state::core::EngineState::RunPendingChoice(choice) =
                &session.engine_state
            {
                if let Some(decision) =
                    neow_followup_selection_v1(&session.run_state, choice, player_class)
                {
                    prefix.push(decision.command);
                }
            }
        }
    }
    Ok(prefix)
}

fn is_neow_followup_selection(session: &RunControlSession) -> bool {
    session.run_state.event_state.as_ref().is_some_and(|event| {
        event.id == EventId::Neow
            && event.completed
            && matches!(
                session.engine_state,
                sts_simulator::state::core::EngineState::RunPendingChoice(_)
            )
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

fn case_from_report(report: &BranchExperimentReportV1) -> DecisionLabCaseV1 {
    let signals = signals_from_report(report);
    let kind = classify_lab_case(&signals);
    DecisionLabCaseV1 {
        schema_name: "DecisionLabCaseV1",
        schema_version: 1,
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
        wall_limit_hit: report.wall_limit_hit,
        first_picks: first_pick_labels(report, 6),
        next_command: rerun_command(report.seed),
        error: None,
    }
}

fn case_from_error(seed: u64, error: String) -> DecisionLabCaseV1 {
    DecisionLabCaseV1 {
        schema_name: "DecisionLabCaseV1",
        schema_version: 1,
        seed,
        kind: DecisionLabCaseKindV1::EngineeringIssue,
        decision: "error".to_string(),
        boundary: "error".to_string(),
        explored_branch_points: 0,
        kept_branches: 0,
        pruned_branches: 0,
        frontier_groups: 0,
        branch_limit_hit: false,
        wall_limit_hit: false,
        first_picks: Vec::new(),
        next_command: rerun_command(seed),
        error: Some(error),
    }
}

fn signals_from_report(report: &BranchExperimentReportV1) -> DecisionLabSignalsV1 {
    DecisionLabSignalsV1 {
        error: None,
        explored_branch_points: report.explored_branch_points,
        branch_limit_hit: report.branch_limit_hit,
        wall_limit_hit: report.wall_limit_hit,
        frontier_group_limit_hit: report.frontier_group_limit_hit,
        pruned_branch_count: report.pruned_branch_count,
        kept_branch_count: report.branches.len(),
        frontier_group_count: report.frontier_groups.len(),
    }
}

fn classify_lab_case(signals: &DecisionLabSignalsV1) -> DecisionLabCaseKindV1 {
    if signals.error.is_some() {
        return DecisionLabCaseKindV1::EngineeringIssue;
    }
    if signals.explored_branch_points == 0 {
        return DecisionLabCaseKindV1::NotEnoughEvidence;
    }
    if signals.branch_limit_hit || signals.wall_limit_hit || signals.frontier_group_limit_hit {
        return DecisionLabCaseKindV1::NeedsMoreBudget;
    }
    if signals.pruned_branch_count > 0
        || signals.kept_branch_count > 1
        || signals.frontier_group_count > 1
    {
        return DecisionLabCaseKindV1::NeedsHumanJudgment;
    }
    DecisionLabCaseKindV1::Routine
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

fn rerun_command(seed: u64) -> String {
    format!(
        "cargo run --quiet --bin branch_experiment_driver -- --seed {seed} --max-depth 3 --max-branches 24 --experiment-wall-ms 10000 --search-wall-ms 100 --search-max-nodes 20000 --branch-examples 8"
    )
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
        DecisionLabCaseKindV1::NeedsMoreBudget => 2,
        DecisionLabCaseKindV1::NotEnoughEvidence => 3,
        DecisionLabCaseKindV1::Routine => 4,
    }
}

fn render_case_line(case: &DecisionLabCaseV1) -> String {
    format!(
        "  seed={} kind={} decision={} frontier={} branch_points={} kept={} pruned={} groups={} first_picks=[{}]",
        case.seed,
        case.kind.as_str(),
        case.decision,
        case.boundary,
        case.explored_branch_points,
        case.kept_branches,
        case.pruned_branches,
        case.frontier_groups,
        if case.first_picks.is_empty() {
            "-".to_string()
        } else {
            case.first_picks.join(", ")
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
            branch_limit_hit: false,
            wall_limit_hit: false,
            frontier_group_limit_hit: false,
            pruned_branch_count: 0,
            kept_branch_count: 1,
            frontier_group_count: 1,
        });

        assert_eq!(kind, DecisionLabCaseKindV1::NotEnoughEvidence);
    }

    #[test]
    fn classifies_budget_limited_experiment_separately() {
        let kind = classify_lab_case(&DecisionLabSignalsV1 {
            error: None,
            explored_branch_points: 2,
            branch_limit_hit: true,
            wall_limit_hit: false,
            frontier_group_limit_hit: false,
            pruned_branch_count: 10,
            kept_branch_count: 24,
            frontier_group_count: 2,
        });

        assert_eq!(kind, DecisionLabCaseKindV1::NeedsMoreBudget);
    }

    #[test]
    fn classifies_multi_branch_decision_as_human_judgment_candidate() {
        let kind = classify_lab_case(&DecisionLabSignalsV1 {
            error: None,
            explored_branch_points: 1,
            branch_limit_hit: false,
            wall_limit_hit: false,
            frontier_group_limit_hit: false,
            pruned_branch_count: 0,
            kept_branch_count: 3,
            frontier_group_count: 1,
        });

        assert_eq!(kind, DecisionLabCaseKindV1::NeedsHumanJudgment);
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
}
