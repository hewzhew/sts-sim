use serde::Serialize;
use sts_simulator::ai::combat_search_v2::SearchTerminalLabel;
use sts_simulator::ai::strategy::deck_strategic_deficit::{
    DeckStrategicDeficit, StrategicDeficitLevel,
};
use sts_simulator::eval::combat_case::CombatCase;

use super::classification::CombatGapReviewClassification;
use super::focus::CombatReviewFocus;
use super::search_types::SearchReview;

#[derive(Serialize)]
pub(super) struct CombatStrategicFeedbackReport {
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

pub(super) fn combat_strategic_feedback(
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
