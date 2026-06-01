use std::cmp::Ordering;

use super::super::{RolloutNodeEstimate, SearchTerminalLabel};

// CombatEval is an estimate-only ordering surface. It must not become an
// authoritative terminal-outcome boundary; exact terminal trajectories and exact
// state keys remain authoritative.
const CRITICAL_SURVIVAL_MARGIN_MAX: i32 = 6;
const STABILIZING_SURVIVAL_MARGIN_MAX: i32 = 15;
const LIKELY_NEXT_TURN_LETHAL_EFFORT_MAX: i32 = 20;
const RACE_FAVORED_EFFORT_MAX: i32 = 60;
const ATTRITION_FAVORED_EFFORT_MAX: i32 = 120;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub(in crate::ai::combat_search_v2) enum CombatEvalEvidenceKind {
    None,
    UnresolvedEstimate,
    SimulatedTerminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub(in crate::ai::combat_search_v2) enum CombatEvalOutcomeClass {
    Loss,
    Unresolved,
    Win,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub(in crate::ai::combat_search_v2) enum CombatEvalSurvivalBucket {
    DeadOrForcedLoss,
    LethalVisible,
    Critical,
    Stabilizing,
    Stable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub(in crate::ai::combat_search_v2) enum CombatEvalProgressBucket {
    Regression,
    Stalled,
    AttritionFavored,
    RaceFavored,
    LethalNextTurnLikely,
    LethalNow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::ai::combat_search_v2) struct CombatEvalV2 {
    pub(super) evidence: CombatEvalEvidenceKind,
    pub(super) outcome: CombatEvalOutcomeClass,
    pub(super) survival: CombatEvalSurvivalBucket,
    pub(super) progress: CombatEvalProgressBucket,
    pub(super) risk_margin: i32,
    pub(super) final_hp: i32,
    pub(super) enemy_progress: i32,
    pub(super) phase_stability: i32,
    pub(super) resource_conservation: i32,
    pub(super) faster_turns: i32,
    pub(super) fewer_cards_played: i32,
}

impl Default for CombatEvalV2 {
    fn default() -> Self {
        Self {
            evidence: CombatEvalEvidenceKind::None,
            outcome: CombatEvalOutcomeClass::Unresolved,
            survival: CombatEvalSurvivalBucket::DeadOrForcedLoss,
            progress: CombatEvalProgressBucket::Stalled,
            risk_margin: 0,
            final_hp: 0,
            enemy_progress: 0,
            phase_stability: 0,
            resource_conservation: 0,
            faster_turns: 0,
            fewer_cards_played: 0,
        }
    }
}

pub(in crate::ai::combat_search_v2) fn combat_eval_from_rollout_estimate(
    estimate: RolloutNodeEstimate,
) -> CombatEvalV2 {
    CombatEvalV2 {
        evidence: rollout_evidence(estimate),
        outcome: outcome_class(estimate.terminal),
        survival: survival_bucket(estimate),
        progress: progress_bucket(estimate),
        risk_margin: estimate.survival_margin,
        final_hp: estimate.final_hp,
        enemy_progress: -estimate.phase_adjusted_enemy_effort,
        phase_stability: phase_stability(estimate),
        resource_conservation: -((estimate.potions_used + estimate.potions_discarded) as i32),
        faster_turns: -(estimate.turns as i32),
        fewer_cards_played: -(estimate.cards_played as i32),
    }
}

impl Ord for CombatEvalV2 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.outcome
            .cmp(&other.outcome)
            .then_with(|| self.evidence.cmp(&other.evidence))
            .then_with(|| match self.outcome {
                CombatEvalOutcomeClass::Win | CombatEvalOutcomeClass::Loss => {
                    self.compare_terminal(other)
                }
                CombatEvalOutcomeClass::Unresolved => self.compare_unresolved(other),
            })
            .then_with(|| self.resource_conservation.cmp(&other.resource_conservation))
            .then_with(|| self.faster_turns.cmp(&other.faster_turns))
            .then_with(|| self.fewer_cards_played.cmp(&other.fewer_cards_played))
    }
}

impl PartialOrd for CombatEvalV2 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl CombatEvalV2 {
    pub(in crate::ai::combat_search_v2) fn outcome_class(self) -> CombatEvalOutcomeClass {
        self.outcome
    }

    pub(in crate::ai::combat_search_v2) fn survival_bucket(self) -> CombatEvalSurvivalBucket {
        self.survival
    }

    pub(in crate::ai::combat_search_v2) fn progress_bucket(self) -> CombatEvalProgressBucket {
        self.progress
    }

    pub(in crate::ai::combat_search_v2) fn risk_margin(self) -> i32 {
        self.risk_margin
    }

    pub(in crate::ai::combat_search_v2) fn final_hp(self) -> i32 {
        self.final_hp
    }

    pub(in crate::ai::combat_search_v2) fn enemy_progress(self) -> i32 {
        self.enemy_progress
    }

    fn compare_terminal(self, other: &Self) -> Ordering {
        self.final_hp
            .cmp(&other.final_hp)
            .then_with(|| self.risk_margin.cmp(&other.risk_margin))
            .then_with(|| self.enemy_progress.cmp(&other.enemy_progress))
            .then_with(|| self.phase_stability.cmp(&other.phase_stability))
    }

    fn compare_unresolved(self, other: &Self) -> Ordering {
        if self.survival.is_danger() || other.survival.is_danger() {
            // In danger states, avoid ranking a flashy race line above a line
            // that first fixes visible survival.
            self.survival
                .cmp(&other.survival)
                .then_with(|| self.risk_margin.cmp(&other.risk_margin))
                .then_with(|| self.progress.cmp(&other.progress))
                .then_with(|| self.enemy_progress.cmp(&other.enemy_progress))
                .then_with(|| self.phase_stability.cmp(&other.phase_stability))
                .then_with(|| self.final_hp.cmp(&other.final_hp))
        } else {
            // In stable states, enemy/phase progress outranks small HP
            // differences so high-HP stalled states do not dominate the queue.
            self.progress
                .cmp(&other.progress)
                .then_with(|| self.enemy_progress.cmp(&other.enemy_progress))
                .then_with(|| self.phase_stability.cmp(&other.phase_stability))
                .then_with(|| self.survival.cmp(&other.survival))
                .then_with(|| self.risk_margin.cmp(&other.risk_margin))
                .then_with(|| self.final_hp.cmp(&other.final_hp))
        }
    }
}

impl CombatEvalOutcomeClass {
    pub(in crate::ai::combat_search_v2) fn label(self) -> &'static str {
        match self {
            Self::Loss => "loss",
            Self::Unresolved => "unresolved",
            Self::Win => "win",
        }
    }
}

impl CombatEvalSurvivalBucket {
    pub(in crate::ai::combat_search_v2) fn label(self) -> &'static str {
        match self {
            Self::DeadOrForcedLoss => "dead_or_forced_loss",
            Self::LethalVisible => "lethal_visible",
            Self::Critical => "critical",
            Self::Stabilizing => "stabilizing",
            Self::Stable => "stable",
        }
    }
}

impl CombatEvalProgressBucket {
    pub(in crate::ai::combat_search_v2) fn label(self) -> &'static str {
        match self {
            Self::Regression => "regression",
            Self::Stalled => "stalled",
            Self::AttritionFavored => "attrition_favored",
            Self::RaceFavored => "race_favored",
            Self::LethalNextTurnLikely => "lethal_next_turn_likely",
            Self::LethalNow => "lethal_now",
        }
    }
}

impl CombatEvalSurvivalBucket {
    fn is_danger(self) -> bool {
        matches!(
            self,
            CombatEvalSurvivalBucket::DeadOrForcedLoss
                | CombatEvalSurvivalBucket::LethalVisible
                | CombatEvalSurvivalBucket::Critical
        )
    }
}

fn rollout_evidence(estimate: RolloutNodeEstimate) -> CombatEvalEvidenceKind {
    if !estimate.evaluated {
        CombatEvalEvidenceKind::None
    } else if estimate.terminal == SearchTerminalLabel::Unresolved {
        CombatEvalEvidenceKind::UnresolvedEstimate
    } else {
        CombatEvalEvidenceKind::SimulatedTerminal
    }
}

fn outcome_class(terminal: SearchTerminalLabel) -> CombatEvalOutcomeClass {
    match terminal {
        SearchTerminalLabel::Win => CombatEvalOutcomeClass::Win,
        SearchTerminalLabel::Unresolved => CombatEvalOutcomeClass::Unresolved,
        SearchTerminalLabel::Loss => CombatEvalOutcomeClass::Loss,
    }
}

fn survival_bucket(estimate: RolloutNodeEstimate) -> CombatEvalSurvivalBucket {
    if estimate.terminal == SearchTerminalLabel::Loss || estimate.final_hp <= 0 {
        CombatEvalSurvivalBucket::DeadOrForcedLoss
    } else if estimate.survival_margin < 0 {
        CombatEvalSurvivalBucket::LethalVisible
    } else if estimate.survival_margin <= CRITICAL_SURVIVAL_MARGIN_MAX {
        CombatEvalSurvivalBucket::Critical
    } else if estimate.survival_margin <= STABILIZING_SURVIVAL_MARGIN_MAX {
        CombatEvalSurvivalBucket::Stabilizing
    } else {
        CombatEvalSurvivalBucket::Stable
    }
}

fn progress_bucket(estimate: RolloutNodeEstimate) -> CombatEvalProgressBucket {
    if estimate.terminal == SearchTerminalLabel::Win {
        return CombatEvalProgressBucket::LethalNow;
    }
    if estimate.terminal == SearchTerminalLabel::Loss {
        return CombatEvalProgressBucket::Regression;
    }

    let effort = estimate.phase_adjusted_enemy_effort.max(0);
    if effort <= LIKELY_NEXT_TURN_LETHAL_EFFORT_MAX {
        CombatEvalProgressBucket::LethalNextTurnLikely
    } else if effort <= RACE_FAVORED_EFFORT_MAX {
        CombatEvalProgressBucket::RaceFavored
    } else if effort <= ATTRITION_FAVORED_EFFORT_MAX {
        CombatEvalProgressBucket::AttritionFavored
    } else {
        CombatEvalProgressBucket::Stalled
    }
}

fn phase_stability(estimate: RolloutNodeEstimate) -> i32 {
    let pressure = ((estimate.special_enemy_phase_count
        + estimate.guardian_mode_shift_pending_count
        + estimate.lagavulin_waking_count
        + estimate.sentry_dazed_pressure_count
        + estimate.hexaghost_opening_pressure_count
        + usize::from(estimate.high_fanout_pending_choice)) as i32)
        .saturating_add(estimate.gremlin_nob_anger_amount_total.max(0))
        .saturating_add(estimate.pending_choice_estimated_action_fanout as i32);
    -pressure
}
