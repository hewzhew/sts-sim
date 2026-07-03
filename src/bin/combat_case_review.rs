use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{
    compile_combat_search_witness_prior_v0, derive_combat_deficit_evidence,
    replay_combat_search_witness_line_v0, run_combat_line_lab_from_parent_v0,
    run_combat_line_lab_v0, run_combat_search_v2, CombatDeficitEvidenceReport, CombatLineLabReport,
    CombatSearchV2ActionPreview, CombatSearchV2ChildRolloutPolicy, CombatSearchV2Config,
    CombatSearchV2PotionPolicy, CombatSearchV2Report, CombatSearchV2RolloutPolicy,
    CombatSearchV2TurnPlanPolicy, CombatSearchV2WitnessLine, CombatSearchV2WitnessReplay,
    SearchTerminalLabel,
};
use sts_simulator::ai::strategy::deck_strategic_deficit::{
    assess_deck_strategic_deficit, DeckStrategicDeficit, StrategicDeficitLevel,
};
use sts_simulator::ai::strategy::run_strategic_facts::RunStrategicFacts;
use sts_simulator::content::cards::{get_card_definition, is_starter_basic, CardType};
use sts_simulator::content::relics::energy_master_delta;
use sts_simulator::eval::combat_case::{
    card_summary, load_combat_case, CombatCase, CombatCaseCardSummary, CombatCasePathStep,
};
use sts_simulator::sim::combat::CombatTerminal;
use sts_simulator::state::core::ClientInput;

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
    #[arg(long, default_value_t = 12)]
    action_preview_limit: usize,
    #[arg(long)]
    replay_focus: bool,
    #[arg(long)]
    immediate_child_rollout: bool,
    #[arg(long, hide = true)]
    lazy_child_rollout: bool,
    #[arg(long)]
    disable_rollout: bool,
    #[arg(long)]
    line_lab: bool,
    #[arg(long, default_value_t = 30_000)]
    line_lab_ms: u64,
    #[arg(long, default_value_t = 8)]
    line_lab_cuts: usize,
    #[arg(long)]
    quality_lanes: bool,
    #[arg(long)]
    quality_lane_total_nodes: Option<usize>,
    #[arg(long)]
    quality_lane_total_ms: Option<u64>,
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
    static_strategic_deficit: DeckStrategicDeficit,
    relics: Vec<String>,
    potions: Vec<Option<String>>,
    path_tail: Vec<CombatCasePathStep>,
    saved_search: Option<sts_simulator::eval::run_control::CombatSearchTraceSummary>,
    ladder: Vec<SearchReview>,
    classification: CombatGapReviewClassification,
    review_focus: Option<CombatReviewFocus>,
    review_focus_replay: Option<CombatSearchV2WitnessReplay>,
    review_focus_prior_rerun: Option<CombatReviewFocusPriorRerun>,
    line_lab: Option<CombatLineLabReport>,
    quality_lanes: Option<CombatQualityLaneReview>,
    combat_deficit_evidence: Option<CombatDeficitEvidenceReport>,
    combat_strategic_feedback: Option<CombatStrategicFeedbackReport>,
}

#[derive(Serialize)]
struct SearchReview {
    label: &'static str,
    nodes: usize,
    wall_ms: u64,
    rollout_policy: &'static str,
    turn_plan_policy: &'static str,
    child_rollout_policy: &'static str,
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
    facts: SearchReviewFacts,
}

#[derive(Serialize)]
struct CombatQualityLaneReview {
    schema: &'static str,
    contract: &'static str,
    total_nodes: usize,
    total_wall_ms: u64,
    per_lane_nodes: usize,
    per_lane_wall_ms: u64,
    selected_lane: Option<&'static str>,
    selected_reason: &'static str,
    lanes: Vec<CombatQualityLaneResult>,
}

#[derive(Serialize)]
struct CombatQualityLaneResult {
    lane: &'static str,
    intent: &'static str,
    review: SearchReview,
    quality: Option<CombatLineQuality>,
}

#[derive(Clone, Serialize)]
struct CombatLineQuality {
    terminal: SearchTerminalLabel,
    hp_loss: i32,
    final_hp: i32,
    persistent_run_value: i32,
    persistent_adjusted_hp: i32,
    potions_used: u32,
    turns: u32,
    cards_played: u32,
    action_count: usize,
}

#[derive(Serialize)]
struct CombatGapReviewClassification {
    kind: &'static str,
    reason: &'static str,
    basis_review: Option<&'static str>,
}

#[derive(Serialize)]
struct CombatReviewFocus {
    selected_review: &'static str,
    reason: &'static str,
    progress: SearchDiagnosticProgressFacts,
}

#[derive(Serialize)]
struct CombatStrategicFeedbackReport {
    schema: &'static str,
    site: CombatStrategicSite,
    signals: Vec<CombatStrategicSignal>,
    observations: CombatStrategicFeedbackObservations,
}

#[derive(Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CombatStrategicSite {
    ActBoss,
    EliteLike,
    HallwayOrUnknown,
}

#[derive(Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CombatStrategicSignal {
    SearchExecutionGap,
    ActBossNoWinAfterReview,
    Act2BossNoWinAfterReview,
    LowHpAtCombatStart,
    LowHpReachedAct3Elite,
    ScalingMissingForBoss,
    ScalingThinUnderBossPressure,
    FrontloadSurplusButBossNoWin,
    StaticBlockAdequateButFatalLoss,
    StaticAoeAdequateButMultiEnemyNoWin,
    StaticScalingAdequateButNoWin,
}

#[derive(Serialize)]
struct CombatStrategicFeedbackObservations {
    review_kind: &'static str,
    focus_source: Option<&'static str>,
    focus_terminal: Option<SearchTerminalLabel>,
    focus_estimated: Option<bool>,
    focus_final_hp: Option<i32>,
    focus_hp_loss: Option<i32>,
    focus_living_enemy_count: Option<usize>,
    focus_total_enemy_hp: Option<i32>,
    enemy_count: usize,
    hp_ratio_pct: i32,
    static_frontload: sts_simulator::ai::strategy::deck_strategic_deficit::StrategicDeficitLevel,
    static_aoe: sts_simulator::ai::strategy::deck_strategic_deficit::StrategicDeficitLevel,
    static_block: sts_simulator::ai::strategy::deck_strategic_deficit::StrategicDeficitLevel,
    static_scaling: sts_simulator::ai::strategy::deck_strategic_deficit::StrategicDeficitLevel,
    static_burden: sts_simulator::ai::strategy::deck_strategic_deficit::StrategicBurdenLevel,
}

#[derive(Serialize)]
struct CombatReviewFocusPriorRerun {
    selected_review: &'static str,
    witness_replayed_actions: usize,
    witness_action_count: Option<usize>,
    witness_terminal: CombatTerminal,
    prior_states: usize,
    duplicate_prior_hints: usize,
    rerun: SearchReview,
}

#[derive(Serialize)]
struct SearchPerformanceReview {
    total_us: u128,
    rollout_us: u128,
    rollout_calls: u64,
    root_rollout_calls: u64,
    child_rollout_calls: u64,
    deferred_child_rollout_calls: u64,
    turn_plan_seed_rollout_calls: u64,
    rollout_evaluations: u64,
    rollout_budget_skips: u64,
    rollout_max_evaluation_budget_skips: u64,
    rollout_deadline_budget_skips: u64,
    deferred_child_rollout_admitted_signal: u64,
    deferred_child_rollout_admitted_periodic: u64,
    deferred_child_rollout_skipped_low_signal: u64,
    deferred_child_rollout_skipped_budget_share: u64,
    turn_plan_seed_us: u128,
    engine_step_us: u128,
    frontier_pop_us: u128,
    expansion_us: u128,
    child_bookkeeping_us: u128,
    rollout_profile: SearchRolloutPerformanceReview,
}

#[derive(Serialize)]
struct SearchRolloutPerformanceReview {
    cache_queries: u64,
    cache_hits: u64,
    cache_misses: u64,
    cache_lookup_us: u128,
    policy_dispatch_us: u128,
    no_potion_iterations: u64,
    no_potion_phase_profile_us: u128,
    no_potion_legal_actions_us: u128,
    no_potion_choose_action_us: u128,
    no_potion_choose_ordering_us: u128,
    no_potion_probe_us: u128,
    no_potion_probe_score_calls: u64,
    no_potion_probe_actions_evaluated: u64,
    no_potion_probe_step_reuses: u64,
    no_potion_probe_engine_step_us: u128,
    no_potion_probe_phase_profile_us: u128,
    no_potion_probe_action_facts_us: u128,
    no_potion_engine_step_us: u128,
    no_potion_child_build_us: u128,
}

#[derive(Serialize)]
struct SearchReviewFacts {
    diagnostic_progress: Option<SearchDiagnosticProgressFacts>,
}

#[derive(Clone, Serialize)]
struct SearchDiagnosticProgressFacts {
    source: &'static str,
    terminal: SearchTerminalLabel,
    estimated: bool,
    final_hp: i32,
    hp_loss: i32,
    turns: u32,
    potions_used: u32,
    cards_played: u32,
    living_enemy_count: usize,
    total_enemy_hp: i32,
    visible_incoming_damage: Option<i32>,
    action_count: Option<usize>,
    exact_prefix_action_count: Option<usize>,
    action_key_preview: Vec<String>,
    input_preview: Vec<ClientInput>,
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
    let (ladder, line_lab_parent) = if args.ladder {
        let (fast_review, _) = run_search(
            "fast_no_potion_diagnostic",
            &case,
            args.fast_nodes,
            args.fast_ms,
            CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            CombatSearchV2PotionPolicy::Never,
            Some(0),
            args.action_preview_limit,
            review_child_rollout_policy(args),
            args.disable_rollout,
        );
        let (slow_review, slow_report) = run_search(
            "slow_potion_diagnostic",
            &case,
            args.slow_nodes,
            args.slow_ms,
            CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            CombatSearchV2PotionPolicy::All,
            Some(args.diagnostic_potion_max),
            args.action_preview_limit,
            review_child_rollout_policy(args),
            args.disable_rollout,
        );
        (
            vec![fast_review, slow_review],
            slow_report.best_complete_trajectory.clone(),
        )
    } else {
        (Vec::new(), None)
    };
    let review_focus = review_focus(&ladder);
    let classification = classify_gap_review(&ladder, review_focus.as_ref());
    let review_focus_replay = if args.replay_focus {
        review_focus.as_ref().map(|focus| {
            replay_combat_search_witness_line_v0(&case.position, &focus_witness_line(focus))
        })
    } else {
        None
    };
    let review_focus_prior_rerun = review_focus
        .as_ref()
        .zip(review_focus_replay.as_ref())
        .and_then(|(focus, replay)| witness_prior_rerun(args, &case, focus, replay));
    let line_lab = if args.line_lab {
        let config = line_lab_search_config(args);
        Some(match line_lab_parent.as_ref() {
            Some(parent) => run_combat_line_lab_from_parent_v0(
                &case.position,
                parent,
                config,
                args.line_lab_ms,
                args.line_lab_cuts,
            ),
            None => {
                run_combat_line_lab_v0(&case.position, config, args.line_lab_ms, args.line_lab_cuts)
            }
        })
    } else {
        None
    };
    let combat_deficit_evidence = line_lab.as_ref().map(derive_combat_deficit_evidence);
    let quality_lanes = if args.quality_lanes {
        Some(run_quality_lanes(args, &case))
    } else {
        None
    };
    let static_strategic_deficit = assess_deck_strategic_deficit(
        &case.position.combat.meta.master_deck_snapshot,
        strategic_facts_from_case(&case),
    );
    let combat_strategic_feedback = combat_strategic_feedback(
        &case,
        &static_strategic_deficit,
        &classification,
        review_focus.as_ref(),
        &ladder,
    );
    CombatCaseReview {
        schema: "combat_case_review",
        case_path: args.case.display().to_string(),
        static_strategic_deficit,
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
        review_focus,
        review_focus_replay,
        review_focus_prior_rerun,
        line_lab,
        quality_lanes,
        combat_deficit_evidence,
        combat_strategic_feedback,
    }
}

fn strategic_facts_from_case(case: &CombatCase) -> RunStrategicFacts {
    let deck = &case.position.combat.meta.master_deck_snapshot;
    RunStrategicFacts {
        entering_act: case.run.act,
        starter_basic_count: deck.iter().filter(|card| is_starter_basic(card.id)).count(),
        curse_count: deck
            .iter()
            .filter(|card| get_card_definition(card.id).card_type == CardType::Curse)
            .count(),
        has_energy_relic: case
            .position
            .combat
            .entities
            .player
            .relics
            .iter()
            .any(|relic| energy_master_delta(relic.id) > 0),
    }
}

fn review_child_rollout_policy(args: &Args) -> CombatSearchV2ChildRolloutPolicy {
    if args.immediate_child_rollout && !args.lazy_child_rollout {
        CombatSearchV2ChildRolloutPolicy::Immediate
    } else {
        CombatSearchV2ChildRolloutPolicy::LazyOnPop
    }
}

fn line_lab_search_config(args: &Args) -> CombatSearchV2Config {
    CombatSearchV2Config {
        max_nodes: args.slow_nodes,
        wall_time: Some(Duration::from_millis(args.line_lab_ms)),
        turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
        child_rollout_policy: review_child_rollout_policy(args),
        potion_policy: CombatSearchV2PotionPolicy::All,
        max_potions_used: Some(args.diagnostic_potion_max),
        rollout_policy: if args.disable_rollout {
            CombatSearchV2RolloutPolicy::Disabled
        } else {
            CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
        },
        ..Default::default()
    }
}

fn classify_gap_review(
    ladder: &[SearchReview],
    focus: Option<&CombatReviewFocus>,
) -> CombatGapReviewClassification {
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
    if let Some(focus) = focus {
        if is_exact_near_miss_loss(&focus.progress) {
            return classification(
                "NearMissNoWinAfterReview",
                "exact_loss_reached_single_enemy_with_low_remaining_hp",
                Some(focus.selected_review),
            );
        }
    }
    classification(
        "StillNoWinAfterReview",
        "no_win_after_review_budget",
        Some(review.label),
    )
}

fn is_exact_near_miss_loss(progress: &SearchDiagnosticProgressFacts) -> bool {
    progress.source == "best_complete"
        && progress.terminal == SearchTerminalLabel::Loss
        && !progress.estimated
        && progress.living_enemy_count == 1
        && progress.total_enemy_hp <= 10
}

fn classification(
    kind: &'static str,
    reason: &'static str,
    basis_review: Option<&'static str>,
) -> CombatGapReviewClassification {
    CombatGapReviewClassification {
        kind,
        reason,
        basis_review,
    }
}

fn combat_strategic_feedback(
    case: &CombatCase,
    static_deficit: &DeckStrategicDeficit,
    classification: &CombatGapReviewClassification,
    focus: Option<&CombatReviewFocus>,
    ladder: &[SearchReview],
) -> Option<CombatStrategicFeedbackReport> {
    if ladder.is_empty() {
        return None;
    }

    let site = combat_site(&case.combat.enemies);
    let progress = focus.map(|focus| &focus.progress);
    let no_exact_win = !ladder.iter().any(|review| review.complete_win);
    let no_win_after_review = matches!(
        classification.kind,
        "StillNoWinAfterReview" | "NearMissNoWinAfterReview" | "SearchStarvedByRollout"
    );
    let exact_loss = progress.is_some_and(|progress| {
        progress.source == "best_complete" && progress.terminal == SearchTerminalLabel::Loss
    });
    let rollout_win = progress.is_some_and(|progress| {
        progress.source == "rollout_frontier" && progress.terminal == SearchTerminalLabel::Win
    });
    let low_hp_start = case.run.hp * 100 <= case.run.max_hp * 20;

    let mut signals = Vec::new();
    if no_exact_win && rollout_win {
        push_signal(&mut signals, CombatStrategicSignal::SearchExecutionGap);
    }
    if no_win_after_review && site == CombatStrategicSite::ActBoss {
        push_signal(&mut signals, CombatStrategicSignal::ActBossNoWinAfterReview);
        if case.run.act == 2 {
            push_signal(
                &mut signals,
                CombatStrategicSignal::Act2BossNoWinAfterReview,
            );
        }
    }
    if no_win_after_review && low_hp_start {
        push_signal(&mut signals, CombatStrategicSignal::LowHpAtCombatStart);
    }
    if no_win_after_review
        && case.run.act >= 3
        && site == CombatStrategicSite::EliteLike
        && low_hp_start
    {
        push_signal(&mut signals, CombatStrategicSignal::LowHpReachedAct3Elite);
    }
    if no_win_after_review && site == CombatStrategicSite::ActBoss {
        match static_deficit.boss_scaling_plan {
            StrategicDeficitLevel::Missing => {
                push_signal(&mut signals, CombatStrategicSignal::ScalingMissingForBoss);
            }
            StrategicDeficitLevel::Thin => {
                push_signal(
                    &mut signals,
                    CombatStrategicSignal::ScalingThinUnderBossPressure,
                );
            }
            StrategicDeficitLevel::Adequate | StrategicDeficitLevel::Surplus => {
                push_signal(
                    &mut signals,
                    CombatStrategicSignal::StaticScalingAdequateButNoWin,
                );
            }
        }
        if static_deficit.frontload_damage == StrategicDeficitLevel::Surplus {
            push_signal(
                &mut signals,
                CombatStrategicSignal::FrontloadSurplusButBossNoWin,
            );
        }
    }
    if no_win_after_review
        && exact_loss
        && !low_hp_start
        && matches!(
            static_deficit.block_or_mitigation,
            StrategicDeficitLevel::Adequate | StrategicDeficitLevel::Surplus
        )
    {
        push_signal(
            &mut signals,
            CombatStrategicSignal::StaticBlockAdequateButFatalLoss,
        );
    }
    if no_win_after_review
        && exact_loss
        && case.combat.enemies.len() > 1
        && matches!(
            static_deficit.aoe_or_minion_control,
            StrategicDeficitLevel::Adequate | StrategicDeficitLevel::Surplus
        )
    {
        push_signal(
            &mut signals,
            CombatStrategicSignal::StaticAoeAdequateButMultiEnemyNoWin,
        );
    }

    Some(CombatStrategicFeedbackReport {
        schema: "combat_strategic_feedback_v0",
        site,
        signals,
        observations: CombatStrategicFeedbackObservations {
            review_kind: classification.kind,
            focus_source: progress.map(|progress| progress.source),
            focus_terminal: progress.map(|progress| progress.terminal),
            focus_estimated: progress.map(|progress| progress.estimated),
            focus_final_hp: progress.map(|progress| progress.final_hp),
            focus_hp_loss: progress.map(|progress| progress.hp_loss),
            focus_living_enemy_count: progress.map(|progress| progress.living_enemy_count),
            focus_total_enemy_hp: progress.map(|progress| progress.total_enemy_hp),
            enemy_count: case.combat.enemies.len(),
            hp_ratio_pct: if case.run.max_hp > 0 {
                case.run.hp * 100 / case.run.max_hp
            } else {
                0
            },
            static_frontload: static_deficit.frontload_damage,
            static_aoe: static_deficit.aoe_or_minion_control,
            static_block: static_deficit.block_or_mitigation,
            static_scaling: static_deficit.boss_scaling_plan,
            static_burden: static_deficit.deck_burden,
        },
    })
}

fn combat_site(enemies: &[String]) -> CombatStrategicSite {
    if enemies.iter().any(|enemy| {
        matches!(
            enemy.as_str(),
            "TheGuardian"
                | "Hexaghost"
                | "SlimeBoss"
                | "BronzeAutomaton"
                | "Champ"
                | "TheCollector"
                | "AwakenedOne"
                | "TimeEater"
                | "Donu"
                | "Deca"
        )
    }) {
        CombatStrategicSite::ActBoss
    } else if enemies.iter().any(|enemy| {
        matches!(
            enemy.as_str(),
            "GremlinNob"
                | "Lagavulin"
                | "Sentry"
                | "GremlinLeader"
                | "BookOfStabbing"
                | "Taskmaster"
                | "Nemesis"
                | "GiantHead"
                | "Reptomancer"
        )
    }) {
        CombatStrategicSite::EliteLike
    } else {
        CombatStrategicSite::HallwayOrUnknown
    }
}

fn push_signal(signals: &mut Vec<CombatStrategicSignal>, signal: CombatStrategicSignal) {
    if !signals.contains(&signal) {
        signals.push(signal);
    }
}

fn review_focus(ladder: &[SearchReview]) -> Option<CombatReviewFocus> {
    let mut selected: Option<(&SearchReview, &SearchDiagnosticProgressFacts)> = None;
    for review in ladder {
        let Some(progress) = review.facts.diagnostic_progress.as_ref() else {
            continue;
        };
        if selected
            .map(|(_, current)| progress_is_better_focus(progress, current))
            .unwrap_or(true)
        {
            selected = Some((review, progress));
        }
    }
    selected.map(|(review, progress)| CombatReviewFocus {
        selected_review: review.label,
        reason: focus_reason(progress),
        progress: progress.clone(),
    })
}

fn progress_is_better_focus(
    candidate: &SearchDiagnosticProgressFacts,
    current: &SearchDiagnosticProgressFacts,
) -> bool {
    match (
        candidate.terminal == SearchTerminalLabel::Win,
        current.terminal == SearchTerminalLabel::Win,
    ) {
        (true, false) => return true,
        (false, true) => return false,
        (true, true) => {
            return (candidate.final_hp, -(candidate.potions_used as i32))
                > (current.final_hp, -(current.potions_used as i32));
        }
        (false, false) => {}
    }

    (
        -candidate.total_enemy_hp,
        -(candidate.living_enemy_count as i32),
        candidate.turns as i32,
        candidate.final_hp,
        -(candidate.potions_used as i32),
    ) > (
        -current.total_enemy_hp,
        -(current.living_enemy_count as i32),
        current.turns as i32,
        current.final_hp,
        -(current.potions_used as i32),
    )
}

fn focus_reason(progress: &SearchDiagnosticProgressFacts) -> &'static str {
    if progress.terminal == SearchTerminalLabel::Win {
        "complete_win_available"
    } else {
        "closest_failure_progress_by_enemy_hp"
    }
}

fn focus_witness_line(focus: &CombatReviewFocus) -> CombatSearchV2WitnessLine {
    CombatSearchV2WitnessLine {
        source: focus.progress.source,
        terminal: focus.progress.terminal,
        final_hp: focus.progress.final_hp,
        total_enemy_hp: focus.progress.total_enemy_hp,
        action_count: focus.progress.action_count,
        actions: focus
            .progress
            .action_key_preview
            .iter()
            .cloned()
            .zip(focus.progress.input_preview.iter().cloned())
            .map(|(action_key, input)| CombatSearchV2ActionPreview { action_key, input })
            .collect(),
    }
}

fn witness_prior_rerun(
    args: &Args,
    case: &CombatCase,
    focus: &CombatReviewFocus,
    replay: &CombatSearchV2WitnessReplay,
) -> Option<CombatReviewFocusPriorRerun> {
    if focus.progress.source != "rollout_frontier"
        || !matches!(replay.terminal, CombatTerminal::Win)
    {
        return None;
    }
    let witness_prior =
        compile_combat_search_witness_prior_v0(&case.position, &focus_witness_line(focus));
    if witness_prior.prior.is_empty() {
        return None;
    }
    let prior_states = witness_prior.prior_states;
    let duplicate_prior_hints = witness_prior.duplicate_prior_hints;
    let rollout_policy = if args.disable_rollout {
        CombatSearchV2RolloutPolicy::Disabled
    } else {
        CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
    };
    let report = run_combat_search_v2(
        &case.position.engine,
        &case.position.combat,
        CombatSearchV2Config {
            max_nodes: args.fast_nodes,
            wall_time: Some(Duration::from_millis(args.fast_ms)),
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            potion_policy: CombatSearchV2PotionPolicy::Never,
            max_potions_used: Some(0),
            rollout_policy,
            child_rollout_policy: review_child_rollout_policy(args),
            root_action_prior: Some(witness_prior.prior),
            ..CombatSearchV2Config::default()
        },
    );
    Some(CombatReviewFocusPriorRerun {
        selected_review: focus.selected_review,
        witness_replayed_actions: replay.replayed_actions,
        witness_action_count: focus.progress.action_count,
        witness_terminal: replay.terminal,
        prior_states,
        duplicate_prior_hints,
        rerun: search_review(
            "focus_witness_prior_rerun",
            args.fast_nodes,
            args.fast_ms,
            CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            CombatSearchV2PotionPolicy::Never,
            Some(0),
            &report,
            args.action_preview_limit,
            rollout_policy.label(),
        ),
    })
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
    action_preview_limit: usize,
    child_rollout_policy: CombatSearchV2ChildRolloutPolicy,
    disable_rollout: bool,
) -> (SearchReview, CombatSearchV2Report) {
    let rollout_policy = if disable_rollout {
        CombatSearchV2RolloutPolicy::Disabled
    } else {
        CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
    };
    run_configured_search(
        label,
        case,
        CombatSearchV2Config {
            max_nodes: nodes,
            wall_time: Some(Duration::from_millis(wall_ms)),
            turn_plan_policy,
            potion_policy,
            max_potions_used,
            rollout_policy,
            child_rollout_policy,
            ..CombatSearchV2Config::default()
        },
        action_preview_limit,
    )
}

fn run_configured_search(
    label: &'static str,
    case: &CombatCase,
    config: CombatSearchV2Config,
    action_preview_limit: usize,
) -> (SearchReview, CombatSearchV2Report) {
    let nodes = config.max_nodes;
    let wall_ms = config
        .wall_time
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default();
    let turn_plan_policy = config.turn_plan_policy;
    let potion_policy = config.potion_policy;
    let max_potions_used = config.max_potions_used;
    let rollout_policy = config.rollout_policy.label();
    let report = run_combat_search_v2(&case.position.engine, &case.position.combat, config);
    let review = search_review(
        label,
        nodes,
        wall_ms,
        turn_plan_policy,
        potion_policy,
        max_potions_used,
        &report,
        action_preview_limit,
        rollout_policy,
    );
    (review, report)
}

fn run_quality_lanes(args: &Args, case: &CombatCase) -> CombatQualityLaneReview {
    const LANE_COUNT: usize = 4;
    let total_nodes = args
        .quality_lane_total_nodes
        .unwrap_or(args.slow_nodes)
        .max(1);
    let total_wall_ms = args.quality_lane_total_ms.unwrap_or(args.slow_ms).max(1);
    let per_lane_nodes = (total_nodes / LANE_COUNT).max(1);
    let per_lane_wall_ms = (total_wall_ms / LANE_COUNT as u64).max(1);
    let mut lanes = Vec::new();
    for lane in quality_lane_specs() {
        let (review, report) = run_configured_search(
            lane.label,
            case,
            lane.config(per_lane_nodes, per_lane_wall_ms),
            args.action_preview_limit,
        );
        let quality = combat_line_quality(&report);
        lanes.push(CombatQualityLaneResult {
            lane: lane.label,
            intent: lane.intent,
            review,
            quality,
        });
    }
    let selected_lane = lanes
        .iter()
        .enumerate()
        .filter_map(|(index, lane)| lane.quality.as_ref().map(|quality| (index, quality)))
        .max_by(|(_, left), (_, right)| compare_quality(left, right))
        .map(|(index, _)| lanes[index].lane);

    CombatQualityLaneReview {
        schema: "combat_quality_lane_review_v0",
        contract: "case_level_experiment_only_same_total_budget_split_across_lanes_no_runner_policy_change",
        total_nodes,
        total_wall_ms,
        per_lane_nodes,
        per_lane_wall_ms,
        selected_lane,
        selected_reason: if selected_lane.is_some() {
            "best_complete_win_by_persistent_adjusted_hp_then_potion_conservation"
        } else {
            "no_lane_found_complete_win"
        },
        lanes,
    }
}

#[derive(Clone, Copy)]
struct QualityLaneSpec {
    label: &'static str,
    intent: &'static str,
    frontier_policy: sts_simulator::ai::combat_search_v2::CombatSearchV2FrontierPolicy,
    turn_plan_policy: CombatSearchV2TurnPlanPolicy,
    rollout_policy: CombatSearchV2RolloutPolicy,
}

impl QualityLaneSpec {
    fn config(self, max_nodes: usize, wall_ms: u64) -> CombatSearchV2Config {
        CombatSearchV2Config {
            max_nodes,
            wall_time: Some(Duration::from_millis(wall_ms)),
            stop_on_win_hp_loss_at_most: Some(0),
            min_win_candidates_before_stop: 4,
            potion_policy: CombatSearchV2PotionPolicy::Never,
            max_potions_used: Some(0),
            rollout_policy: self.rollout_policy,
            child_rollout_policy: CombatSearchV2ChildRolloutPolicy::LazyOnPop,
            turn_plan_policy: self.turn_plan_policy,
            frontier_policy: self.frontier_policy,
            ..CombatSearchV2Config::default()
        }
    }
}

fn quality_lane_specs() -> [QualityLaneSpec; 4] {
    use sts_simulator::ai::combat_search_v2::CombatSearchV2FrontierPolicy;
    [
        QualityLaneSpec {
            label: "quality_balanced_rr",
            intent: "baseline round-robin frontier with adaptive rollout",
            frontier_policy: CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            rollout_policy: CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
        },
        QualityLaneSpec {
            label: "quality_strict_best_first",
            intent: "single frontier queue to let current priority fully decide",
            frontier_policy: CombatSearchV2FrontierPolicy::SingleQueue,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            rollout_policy: CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
        },
        QualityLaneSpec {
            label: "quality_exact_no_rollout",
            intent: "spend budget on exact expansion instead of rollout estimates",
            frontier_policy: CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
        },
        QualityLaneSpec {
            label: "quality_root_turn_seed",
            intent: "seed exact current-turn end states before atomic expansion",
            frontier_policy: CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::RootFrontierSeed,
            rollout_policy: CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion,
        },
    ]
}

fn combat_line_quality(report: &CombatSearchV2Report) -> Option<CombatLineQuality> {
    let trajectory = report.best_win_trajectory.as_ref()?;
    Some(CombatLineQuality {
        terminal: trajectory.terminal,
        hp_loss: trajectory.hp_loss,
        final_hp: trajectory.final_hp,
        persistent_run_value: trajectory.persistent_run_value,
        persistent_adjusted_hp: trajectory
            .final_hp
            .saturating_add(trajectory.persistent_run_value),
        potions_used: trajectory.potions_used,
        turns: trajectory.turns,
        cards_played: trajectory.cards_played,
        action_count: trajectory.actions.len(),
    })
}

fn compare_quality(left: &CombatLineQuality, right: &CombatLineQuality) -> std::cmp::Ordering {
    (
        left.persistent_adjusted_hp,
        left.final_hp,
        left.persistent_run_value,
        -(left.potions_used as i32),
        -(left.turns as i32),
        -(left.cards_played as i32),
        -(left.action_count as i32),
    )
        .cmp(&(
            right.persistent_adjusted_hp,
            right.final_hp,
            right.persistent_run_value,
            -(right.potions_used as i32),
            -(right.turns as i32),
            -(right.cards_played as i32),
            -(right.action_count as i32),
        ))
}

fn search_review(
    label: &'static str,
    nodes: usize,
    wall_ms: u64,
    turn_plan_policy: CombatSearchV2TurnPlanPolicy,
    potion_policy: CombatSearchV2PotionPolicy,
    max_potions_used: Option<u32>,
    report: &CombatSearchV2Report,
    action_preview_limit: usize,
    rollout_policy: &'static str,
) -> SearchReview {
    let best = report.best_win_trajectory.as_ref();
    SearchReview {
        label,
        nodes,
        wall_ms,
        rollout_policy,
        turn_plan_policy: turn_plan_policy.label(),
        child_rollout_policy: report.search_policy.child_rollout_policy,
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
            rollout_calls: report.performance.rollout_estimate_calls,
            root_rollout_calls: report.performance.root_rollout_estimate_calls,
            child_rollout_calls: report.performance.child_rollout_estimate_calls,
            deferred_child_rollout_calls: report.performance.deferred_child_rollout_estimate_calls,
            turn_plan_seed_rollout_calls: report.performance.turn_plan_seed_rollout_estimate_calls,
            rollout_evaluations: report.rollout.evaluations,
            rollout_budget_skips: report.rollout.budget_skips,
            rollout_max_evaluation_budget_skips: report.rollout.max_evaluation_budget_skips,
            rollout_deadline_budget_skips: report.rollout.deadline_budget_skips,
            deferred_child_rollout_admitted_signal: report
                .performance
                .deferred_child_rollout_admitted_signal,
            deferred_child_rollout_admitted_periodic: report
                .performance
                .deferred_child_rollout_admitted_periodic,
            deferred_child_rollout_skipped_low_signal: report
                .performance
                .deferred_child_rollout_skipped_low_signal,
            deferred_child_rollout_skipped_budget_share: report
                .performance
                .deferred_child_rollout_skipped_budget_share,
            turn_plan_seed_us: report.performance.turn_plan_frontier_seed_elapsed_us,
            engine_step_us: report.performance.engine_step_elapsed_us,
            frontier_pop_us: report.performance.frontier_pop_elapsed_us,
            expansion_us: report.performance.expansion_elapsed_us,
            child_bookkeeping_us: report.performance.child_bookkeeping_elapsed_us,
            rollout_profile: SearchRolloutPerformanceReview {
                cache_queries: report.rollout.cache_queries,
                cache_hits: report.rollout.cache_hits,
                cache_misses: report.rollout.cache_misses,
                cache_lookup_us: report.rollout.performance.cache_lookup_us,
                policy_dispatch_us: report.rollout.performance.policy_dispatch_us,
                no_potion_iterations: report.rollout.performance.no_potion_iterations,
                no_potion_phase_profile_us: report.rollout.performance.no_potion_phase_profile_us,
                no_potion_legal_actions_us: report.rollout.performance.no_potion_legal_actions_us,
                no_potion_choose_action_us: report.rollout.performance.no_potion_choose_action_us,
                no_potion_choose_ordering_us: report
                    .rollout
                    .performance
                    .no_potion_choose_ordering_us,
                no_potion_probe_us: report.rollout.performance.no_potion_probe_us,
                no_potion_probe_score_calls: report.rollout.performance.no_potion_probe_score_calls,
                no_potion_probe_actions_evaluated: report
                    .rollout
                    .performance
                    .no_potion_probe_actions_evaluated,
                no_potion_probe_step_reuses: report.rollout.performance.no_potion_probe_step_reuses,
                no_potion_probe_engine_step_us: report
                    .rollout
                    .performance
                    .no_potion_probe_engine_step_us,
                no_potion_probe_phase_profile_us: report
                    .rollout
                    .performance
                    .no_potion_probe_phase_profile_us,
                no_potion_probe_action_facts_us: report
                    .rollout
                    .performance
                    .no_potion_probe_action_facts_us,
                no_potion_engine_step_us: report.rollout.performance.no_potion_engine_step_us,
                no_potion_child_build_us: report.rollout.performance.no_potion_child_build_us,
            },
        },
        facts: SearchReviewFacts {
            diagnostic_progress: diagnostic_progress_facts(report, action_preview_limit),
        },
    }
}

fn diagnostic_progress_facts(
    report: &CombatSearchV2Report,
    action_preview_limit: usize,
) -> Option<SearchDiagnosticProgressFacts> {
    if let Some(trajectory) = report.best_complete_trajectory.as_ref() {
        return Some(SearchDiagnosticProgressFacts {
            source: "best_complete",
            terminal: trajectory.terminal,
            estimated: trajectory.estimated,
            final_hp: trajectory.final_hp,
            hp_loss: trajectory.hp_loss,
            turns: trajectory.turns,
            potions_used: trajectory.potions_used,
            cards_played: trajectory.cards_played,
            living_enemy_count: trajectory.final_state.living_enemy_count,
            total_enemy_hp: trajectory.final_state.total_enemy_hp,
            visible_incoming_damage: Some(trajectory.final_state.visible_incoming_damage),
            action_count: Some(trajectory.actions.len()),
            exact_prefix_action_count: Some(trajectory.actions.len()),
            action_key_preview: trajectory
                .actions
                .iter()
                .take(action_preview_limit)
                .map(|action| action.action_key.clone())
                .collect(),
            input_preview: trajectory
                .actions
                .iter()
                .take(action_preview_limit)
                .map(|action| action.input.clone())
                .collect(),
        });
    }
    report
        .rollout
        .best_frontier_estimate
        .as_ref()
        .map(|rollout| {
            let frontier = report.best_frontier_trajectory.as_ref();
            let exact_prefix_actions = frontier
                .map(|trajectory| trajectory.actions.as_slice())
                .unwrap_or(&[]);
            let exact_prefix_action_count = Some(exact_prefix_actions.len());
            SearchDiagnosticProgressFacts {
                source: "rollout_frontier",
                terminal: rollout.terminal,
                estimated: rollout.estimated,
                final_hp: rollout.final_hp,
                hp_loss: rollout.hp_loss,
                turns: rollout.turns,
                potions_used: rollout.potions_used,
                cards_played: rollout.cards_played,
                living_enemy_count: rollout.living_enemy_count,
                total_enemy_hp: rollout.total_enemy_hp,
                visible_incoming_damage: frontier
                    .map(|trajectory| trajectory.final_state.visible_incoming_damage),
                action_count: Some(
                    rollout
                        .actions_simulated
                        .saturating_add(exact_prefix_actions.len()),
                ),
                exact_prefix_action_count,
                action_key_preview: rollout
                    .action_preview
                    .iter()
                    .take(action_preview_limit)
                    .map(|action| action.action_key.clone())
                    .collect(),
                input_preview: rollout
                    .action_preview
                    .iter()
                    .take(action_preview_limit)
                    .map(|action| action.input.clone())
                    .collect(),
            }
        })
}

fn potion_policy_label(policy: CombatSearchV2PotionPolicy) -> &'static str {
    match policy {
        CombatSearchV2PotionPolicy::Never => "never",
        CombatSearchV2PotionPolicy::All => "all",
        CombatSearchV2PotionPolicy::SemanticBudgeted => "semantic",
    }
}
