use sts_simulator::ai::combat_search_v2::SearchTerminalLabel;
use sts_simulator::ai::strategy::deck_strategic_deficit::{
    DeckStrategicDeficit, StrategicDeficitLevel,
};
use sts_simulator::eval::combat_case::CombatCase;

use super::super::classification::CombatGapReviewClassification;
use super::super::search_types::{SearchDiagnosticProgressFacts, SearchReview};
use super::types::{CombatStrategicSignal, CombatStrategicSite};

pub(super) fn strategic_signals(
    case: &CombatCase,
    static_deficit: &DeckStrategicDeficit,
    classification: &CombatGapReviewClassification,
    progress: Option<&SearchDiagnosticProgressFacts>,
    site: CombatStrategicSite,
    ladder: &[SearchReview],
) -> Vec<CombatStrategicSignal> {
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
    signals
}

fn push_signal(signals: &mut Vec<CombatStrategicSignal>, signal: CombatStrategicSignal) {
    if !signals.contains(&signal) {
        signals.push(signal);
    }
}
