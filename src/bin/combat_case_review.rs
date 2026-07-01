use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{
    run_combat_search_v2, CombatSearchV2Config, CombatSearchV2PotionPolicy, CombatSearchV2Report,
    CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::eval::combat_case::{
    card_summary, load_combat_case, CombatCase, CombatCaseCardSummary, CombatCasePathStep,
};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    case: PathBuf,
    #[arg(long)]
    ladder: bool,
    #[arg(long, default_value_t = 200_000)]
    fast_nodes: usize,
    #[arg(long, default_value_t = 2_000)]
    fast_ms: u64,
    #[arg(long, default_value_t = 800_000)]
    slow_nodes: usize,
    #[arg(long, default_value_t = 8_000)]
    slow_ms: u64,
    #[arg(long, default_value_t = 3)]
    diagnostic_potion_max: u32,
    #[arg(long)]
    write_review: Option<PathBuf>,
    #[arg(long)]
    compact: bool,
}

#[derive(Serialize)]
struct CombatCaseReview {
    schema: &'static str,
    case_path: String,
    source: sts_simulator::eval::combat_case::CombatCaseSource,
    gap: sts_simulator::eval::combat_case::CombatCaseGap,
    run: sts_simulator::eval::combat_case::CombatCaseRunSummary,
    combat: sts_simulator::eval::combat_case::CombatCaseCombatSummary,
    deck: Vec<CombatCaseCardSummary>,
    relics: Vec<String>,
    potions: Vec<Option<String>>,
    path_tail: Vec<CombatCasePathStep>,
    saved_search: Option<sts_simulator::eval::run_control::CombatSearchTraceSummary>,
    ladder: Vec<SearchReview>,
    classification: CombatGapReviewClassification,
}

#[derive(Serialize)]
struct SearchReview {
    label: &'static str,
    nodes: usize,
    wall_ms: u64,
    turn_plan_policy: &'static str,
    potion_policy: &'static str,
    max_potions_used: Option<u32>,
    complete_win: bool,
    hp_loss: Option<i32>,
    final_hp: Option<i32>,
    turns: Option<u32>,
    potions_used: Option<u32>,
    nodes_expanded: u64,
    nodes_generated: u64,
    nodes_to_first_win: Option<u64>,
    terminal_wins: u64,
    elapsed_ms: u128,
    deadline_hit: bool,
    node_budget_hit: bool,
    performance: SearchPerformanceReview,
}

#[derive(Serialize)]
struct CombatGapReviewClassification {
    kind: &'static str,
    reason: &'static str,
    selected_review: Option<&'static str>,
}

#[derive(Serialize)]
struct SearchPerformanceReview {
    total_us: u128,
    rollout_us: u128,
    turn_plan_seed_us: u128,
    engine_step_us: u128,
    frontier_pop_us: u128,
    expansion_us: u128,
    child_bookkeeping_us: u128,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let case = load_combat_case(&args.case)?;
    let review = build_review(&args, case);
    let payload = if args.compact {
        serde_json::to_string(&review)?
    } else {
        serde_json::to_string_pretty(&review)?
    };
    if let Some(path) = args.write_review.as_ref() {
        std::fs::write(path, payload)?;
        println!("{}", path.display());
    } else {
        println!("{payload}");
    }
    Ok(())
}

fn build_review(args: &Args, case: CombatCase) -> CombatCaseReview {
    let ladder = if args.ladder {
        vec![
            run_search(
                "fast_no_potion_diagnostic",
                &case,
                args.fast_nodes,
                args.fast_ms,
                CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
                CombatSearchV2PotionPolicy::Never,
                Some(0),
            ),
            run_search(
                "slow_potion_diagnostic",
                &case,
                args.slow_nodes,
                args.slow_ms,
                CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
                CombatSearchV2PotionPolicy::All,
                Some(args.diagnostic_potion_max),
            ),
        ]
    } else {
        Vec::new()
    };
    let classification = classify_gap_review(&ladder);
    CombatCaseReview {
        schema: "combat_case_review",
        case_path: args.case.display().to_string(),
        deck: case
            .position
            .combat
            .meta
            .master_deck_snapshot
            .iter()
            .map(card_summary)
            .collect(),
        relics: case
            .position
            .combat
            .entities
            .player
            .relics
            .iter()
            .map(|relic| format!("{:?}", relic.id))
            .collect(),
        potions: case
            .position
            .combat
            .entities
            .potions
            .iter()
            .map(|potion| potion.as_ref().map(|potion| format!("{:?}", potion.id)))
            .collect(),
        path_tail: case
            .path
            .iter()
            .skip(case.path.len().saturating_sub(12))
            .cloned()
            .collect(),
        saved_search: case.failed_search.clone(),
        source: case.source,
        gap: case.gap,
        run: case.run,
        combat: case.combat,
        ladder,
        classification,
    }
}

fn classify_gap_review(ladder: &[SearchReview]) -> CombatGapReviewClassification {
    if ladder.is_empty() {
        return classification("NotReviewed", "ladder_not_requested", None);
    }
    if let Some(review) = ladder.iter().find(|review| review.complete_win) {
        return if review.potions_used.unwrap_or(0) > 0 {
            classification(
                "PotionRescueWon",
                "win_found_using_potions",
                Some(review.label),
            )
        } else {
            classification(
                "SearchMissWonWithReview",
                "win_found_with_review_budget",
                Some(review.label),
            )
        };
    }
    let review = ladder
        .last()
        .expect("non-empty ladder was checked before classification");
    if search_starved_by_rollout(review) {
        return classification(
            "SearchStarvedByRollout",
            "rollout_pct_high_and_nodes_low",
            Some(review.label),
        );
    }
    if review.deadline_hit && review.nodes_expanded < 1_000 {
        return classification(
            "TimeoutNoConclusion",
            "deadline_hit_with_too_few_exact_nodes",
            Some(review.label),
        );
    }
    classification(
        "StillNoWinAfterReview",
        "no_win_after_review_budget",
        Some(review.label),
    )
}

fn classification(
    kind: &'static str,
    reason: &'static str,
    selected_review: Option<&'static str>,
) -> CombatGapReviewClassification {
    CombatGapReviewClassification {
        kind,
        reason,
        selected_review,
    }
}

fn search_starved_by_rollout(review: &SearchReview) -> bool {
    review.nodes_expanded < 500 && rollout_pct(review) >= 75.0
}

fn rollout_pct(review: &SearchReview) -> f64 {
    if review.performance.total_us == 0 {
        return 0.0;
    }
    100.0 * review.performance.rollout_us as f64 / review.performance.total_us as f64
}

fn run_search(
    label: &'static str,
    case: &CombatCase,
    nodes: usize,
    wall_ms: u64,
    turn_plan_policy: CombatSearchV2TurnPlanPolicy,
    potion_policy: CombatSearchV2PotionPolicy,
    max_potions_used: Option<u32>,
) -> SearchReview {
    let report = run_combat_search_v2(
        &case.position.engine,
        &case.position.combat,
        CombatSearchV2Config {
            max_nodes: nodes,
            wall_time: Some(Duration::from_millis(wall_ms)),
            turn_plan_policy,
            potion_policy,
            max_potions_used,
            ..CombatSearchV2Config::default()
        },
    );
    search_review(
        label,
        nodes,
        wall_ms,
        turn_plan_policy,
        potion_policy,
        max_potions_used,
        &report,
    )
}

fn search_review(
    label: &'static str,
    nodes: usize,
    wall_ms: u64,
    turn_plan_policy: CombatSearchV2TurnPlanPolicy,
    potion_policy: CombatSearchV2PotionPolicy,
    max_potions_used: Option<u32>,
    report: &CombatSearchV2Report,
) -> SearchReview {
    let best = report.best_win_trajectory.as_ref();
    SearchReview {
        label,
        nodes,
        wall_ms,
        turn_plan_policy: turn_plan_policy.label(),
        potion_policy: potion_policy_label(potion_policy),
        max_potions_used,
        complete_win: best.is_some(),
        hp_loss: best.map(|trajectory| trajectory.hp_loss),
        final_hp: best.map(|trajectory| trajectory.final_hp),
        turns: best.map(|trajectory| trajectory.turns),
        potions_used: best.map(|trajectory| trajectory.potions_used),
        nodes_expanded: report.stats.nodes_expanded,
        nodes_generated: report.stats.nodes_generated,
        nodes_to_first_win: report.stats.nodes_to_first_win,
        terminal_wins: report.stats.terminal_wins,
        elapsed_ms: report.stats.elapsed_ms,
        deadline_hit: report.stats.deadline_hit,
        node_budget_hit: report.stats.node_budget_hit,
        performance: SearchPerformanceReview {
            total_us: report.performance.total_elapsed_us,
            rollout_us: report.performance.rollout_estimate_elapsed_us,
            turn_plan_seed_us: report.performance.turn_plan_frontier_seed_elapsed_us,
            engine_step_us: report.performance.engine_step_elapsed_us,
            frontier_pop_us: report.performance.frontier_pop_elapsed_us,
            expansion_us: report.performance.expansion_elapsed_us,
            child_bookkeeping_us: report.performance.child_bookkeeping_elapsed_us,
        },
    }
}

fn potion_policy_label(policy: CombatSearchV2PotionPolicy) -> &'static str {
    match policy {
        CombatSearchV2PotionPolicy::Never => "never",
        CombatSearchV2PotionPolicy::All => "all",
        CombatSearchV2PotionPolicy::SemanticBudgeted => "semantic",
    }
}
