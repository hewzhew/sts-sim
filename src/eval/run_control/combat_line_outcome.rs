use std::collections::{HashMap, HashSet};

use crate::ai::combat_search_v2::{
    run_combat_search_v2, CombatSearchV2Config, CombatSearchV2PotionPolicy, CombatSearchV2Report,
    CombatSearchV2TrajectoryReport,
};
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::sim::combat::{CombatPosition, CombatTerminal};

use super::combat_candidate_line::{replay_candidate_line, CombatCandidateLine};
use super::session::RunControlSession;
use super::transition_report::CardSnapshot;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CombatLineAcceptance {
    CleanWin,
    DirtyWin,
    NonWinning,
}

impl CombatLineAcceptance {
    pub(super) fn is_rejected(self) -> bool {
        matches!(self, Self::DirtyWin)
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct CombatLineAcceptancePolicy {
    reject_gained_curses: bool,
}

impl Default for CombatLineAcceptancePolicy {
    fn default() -> Self {
        Self {
            reject_gained_curses: true,
        }
    }
}

impl CombatLineAcceptancePolicy {
    pub(super) fn classify(self, outcome: &CombatLineOutcome) -> CombatLineAcceptance {
        if outcome.terminal != CombatTerminal::Win || outcome.final_hp <= 0 {
            return CombatLineAcceptance::NonWinning;
        }
        if self.reject_gained_curses && !outcome.gained_curses.is_empty() {
            CombatLineAcceptance::DirtyWin
        } else {
            CombatLineAcceptance::CleanWin
        }
    }
}

pub(super) struct CombatLineEvaluation {
    pub(super) line: CombatCandidateLine,
    pub(super) outcome: CombatLineOutcome,
}

pub(super) struct CombatLineAlternative {
    pub(super) line: CombatCandidateLine,
    pub(super) outcome: CombatLineOutcome,
    pub(super) report: CombatSearchV2Report,
}

pub(super) struct CombatLineOutcome {
    terminal: CombatTerminal,
    pub(super) final_hp: i32,
    pub(super) hp_loss: i32,
    potions_used: u32,
    action_count: usize,
    gold_delta: i32,
    ritual_dagger_growth: i32,
    gained_curses: Vec<CardSnapshot>,
}

impl CombatLineOutcome {
    pub(super) fn gained_curse_count(&self) -> usize {
        self.gained_curses.len()
    }
}

pub(super) fn find_clean_no_potion_alternative(
    session: &RunControlSession,
    start: &CombatPosition,
    config: &CombatSearchV2Config,
    policy: CombatLineAcceptancePolicy,
) -> Result<Option<CombatLineAlternative>, String> {
    let mut clean_config = config.clone();
    clean_config.potion_policy = CombatSearchV2PotionPolicy::All;
    clean_config.max_potions_used = Some(0);
    clean_config.min_win_candidates_before_stop = 128;
    let report = run_combat_search_v2(&start.engine, &start.combat, clean_config.clone());
    let Some(evaluation) =
        find_accepted_alternative_in_report(session, start, &clean_config, &report, policy)?
    else {
        return Ok(None);
    };
    Ok(Some(CombatLineAlternative {
        line: evaluation.line,
        outcome: evaluation.outcome,
        report,
    }))
}

pub(super) fn find_accepted_alternative_in_report(
    session: &RunControlSession,
    start: &CombatPosition,
    config: &CombatSearchV2Config,
    report: &CombatSearchV2Report,
    policy: CombatLineAcceptancePolicy,
) -> Result<Option<CombatLineEvaluation>, String> {
    let mut best_clean: Option<CombatLineEvaluation> = None;
    for trajectory in win_candidate_trajectories(report) {
        let line = CombatCandidateLine::from_search_trajectory(trajectory);
        let evaluation = evaluate_combat_candidate_line_outcome(session, start, config, line)?;
        if policy.classify(&evaluation.outcome) != CombatLineAcceptance::CleanWin {
            continue;
        }
        let replace = best_clean
            .as_ref()
            .map(|best| prefer_accepted_outcome(&evaluation.outcome, &best.outcome))
            .unwrap_or(true);
        if replace {
            best_clean = Some(evaluation);
        }
    }
    Ok(best_clean)
}

pub(super) fn evaluate_combat_candidate_line_outcome(
    session: &RunControlSession,
    start: &CombatPosition,
    config: &CombatSearchV2Config,
    line: CombatCandidateLine,
) -> Result<CombatLineEvaluation, String> {
    let replay = replay_candidate_line(start, line.source, &line.actions, config)?;
    let before_deck_uuids = master_deck_uuids(session);
    let before_deck_cards = master_deck_cards_by_uuid(session);
    let before_gold = session.run_state.gold;
    let mut trial = session.clone();
    trial.mark_current_combat_search_resolved();
    for action in &replay.line.actions {
        trial.apply_input(action.input.clone())?;
    }
    let gained_curses = trial
        .run_state
        .master_deck
        .iter()
        .filter(|card| {
            !before_deck_uuids.contains(&card.uuid)
                && get_card_definition(card.id).card_type == CardType::Curse
        })
        .map(|card| CardSnapshot {
            id: card.id,
            uuid: card.uuid,
            upgrades: card.upgrades,
        })
        .collect();
    let ritual_dagger_growth = trial
        .run_state
        .master_deck
        .iter()
        .filter(|card| card.id == CardId::RitualDagger)
        .filter_map(|card| {
            before_deck_cards
                .get(&card.uuid)
                .filter(|before| before.id == CardId::RitualDagger)
                .map(|before| (card.misc_value - before.misc_value).max(0))
        })
        .sum();
    let outcome = CombatLineOutcome {
        terminal: replay.line.terminal,
        final_hp: replay.line.final_hp,
        hp_loss: replay.line.hp_loss,
        potions_used: replay.line.potions_used,
        action_count: replay.line.actions.len(),
        gold_delta: trial.run_state.gold - before_gold,
        ritual_dagger_growth,
        gained_curses,
    };
    Ok(CombatLineEvaluation {
        line: replay.line,
        outcome,
    })
}

pub(super) fn render_combat_line_outcome_detail(outcome: &CombatLineOutcome) -> String {
    let gained_curses = outcome
        .gained_curses
        .iter()
        .map(|card| format!("{:?}#{}", card.id, card.uuid))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "terminal={:?} final_hp={} hp_loss={} potions_used={} actions={} gold_delta={} ritual_dagger_growth={} gained_curses=[{}]",
        outcome.terminal,
        outcome.final_hp,
        outcome.hp_loss,
        outcome.potions_used,
        outcome.action_count,
        outcome.gold_delta,
        outcome.ritual_dagger_growth,
        gained_curses
    )
}

fn win_candidate_trajectories(
    report: &CombatSearchV2Report,
) -> Vec<&CombatSearchV2TrajectoryReport> {
    let mut trajectories = Vec::new();
    if let Some(best) = report.best_win_trajectory.as_ref() {
        trajectories.push(best);
    }
    for candidate in &report.win_candidate_trajectories {
        if !trajectories
            .iter()
            .any(|existing| same_action_trace(existing, candidate))
        {
            trajectories.push(candidate);
        }
    }
    trajectories
}

fn prefer_accepted_outcome(left: &CombatLineOutcome, right: &CombatLineOutcome) -> bool {
    left.final_hp > right.final_hp
        || (left.final_hp == right.final_hp
            && (
                left.potions_used,
                -left.ritual_dagger_growth,
                -left.gold_delta,
                left.action_count,
            ) < (
                right.potions_used,
                -right.ritual_dagger_growth,
                -right.gold_delta,
                right.action_count,
            ))
}

fn same_action_trace(
    left: &CombatSearchV2TrajectoryReport,
    right: &CombatSearchV2TrajectoryReport,
) -> bool {
    left.actions.len() == right.actions.len()
        && left
            .actions
            .iter()
            .zip(&right.actions)
            .all(|(left, right)| left.action_key == right.action_key)
}

fn master_deck_uuids(session: &RunControlSession) -> HashSet<u32> {
    session
        .run_state
        .master_deck
        .iter()
        .map(|card| card.uuid)
        .collect()
}

struct DeckCardLineSnapshot {
    id: CardId,
    misc_value: i32,
}

fn master_deck_cards_by_uuid(session: &RunControlSession) -> HashMap<u32, DeckCardLineSnapshot> {
    session
        .run_state
        .master_deck
        .iter()
        .map(|card| {
            (
                card.uuid,
                DeckCardLineSnapshot {
                    id: card.id,
                    misc_value: card.misc_value,
                },
            )
        })
        .collect()
}
