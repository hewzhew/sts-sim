use std::cmp::Ordering;
use std::time::Instant;

use crate::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper};

use super::phase_profile::combat_search_phase_profile;
use super::transition::terminal_label;
use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RolloutActionProbeScore {
    terminal_rank: i32,
    final_hp: i32,
    survival_margin: i32,
    living_enemy_progress: i32,
    phase_adjusted_enemy_progress: i32,
    split_debt_stability: i32,
    mechanics_stability: i32,
    pending_choice_fanout: i32,
    ordered_preference: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
struct RolloutPhaseProbeScore {
    split_debt_stability: i32,
    mechanics_stability: i32,
    pending_choice_fanout: i32,
}

impl Ord for RolloutActionProbeScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.terminal_rank
            .cmp(&other.terminal_rank)
            .then_with(|| self.final_hp.cmp(&other.final_hp))
            .then_with(|| self.survival_margin.cmp(&other.survival_margin))
            .then_with(|| self.living_enemy_progress.cmp(&other.living_enemy_progress))
            .then_with(|| {
                self.phase_adjusted_enemy_progress
                    .cmp(&other.phase_adjusted_enemy_progress)
            })
            .then_with(|| self.mechanics_stability.cmp(&other.mechanics_stability))
            .then_with(|| self.pending_choice_fanout.cmp(&other.pending_choice_fanout))
            .then_with(|| self.ordered_preference.cmp(&other.ordered_preference))
    }
}

impl PartialOrd for RolloutActionProbeScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) fn choose_by_one_step_probe(
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
    ordered: &[IndexedActionChoice],
    allow_nonterminal_upgrade: bool,
) -> Option<(IndexedActionChoice, &'static str)> {
    let fallback_score = probe_action_score(node, stepper, config, deadline, ordered.first()?, 0)?;
    let mut best: Option<(RolloutActionProbeScore, IndexedActionChoice, &'static str)> = None;
    for (ordered_index, choice) in ordered
        .iter()
        .take(super::rollout_policy::CONSERVATIVE_ROLLOUT_PROBE_ACTION_LIMIT)
        .enumerate()
    {
        let Some(score) =
            probe_action_score(node, stepper, config, deadline, choice, ordered_index)
        else {
            continue;
        };
        let Some(reason) = probe_upgrade_reason(score, fallback_score, allow_nonterminal_upgrade)
        else {
            continue;
        };
        let replace = best
            .as_ref()
            .map(|(best_score, _, _)| score > *best_score)
            .unwrap_or(true);
        if replace {
            best = Some((score, choice.clone(), reason));
        }
    }
    let (_, best_choice, reason) = best?;
    Some((best_choice, reason))
}

fn probe_action_score(
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
    choice: &IndexedActionChoice,
    ordered_index: usize,
) -> Option<RolloutActionProbeScore> {
    if deadline.is_some_and(|limit| Instant::now() >= limit) {
        return None;
    }
    let position = CombatPosition::new(node.engine.clone(), node.combat.clone());
    let step = stepper.apply_to_stable(
        &position,
        choice.choice.input.clone(),
        CombatStepLimits {
            max_engine_steps: config.max_engine_steps_per_action,
            deadline,
        },
    );
    if step.truncated || step.timed_out {
        return None;
    }
    let phase_profile = combat_search_phase_profile(&step.position.engine, &step.position.combat);
    let terminal = terminal_label(&step.position.engine, &step.position.combat);
    let mechanics_pressure = (phase_profile
        .enemy_mechanics
        .guardian_mode_shift_pending_count
        .saturating_add(phase_profile.enemy_mechanics.lagavulin_waking_count)
        .saturating_add(phase_profile.enemy_mechanics.sentry_dazed_pressure_count)
        .saturating_add(
            phase_profile
                .enemy_mechanics
                .hexaghost_opening_pressure_count,
        ) as i32)
        .saturating_add(
            phase_profile
                .enemy_mechanics
                .gremlin_nob_anger_amount_total
                .max(0),
        );
    Some(RolloutActionProbeScore {
        terminal_rank: terminal_rank(terminal),
        final_hp: step.position.combat.entities.player.current_hp,
        survival_margin: phase_profile.pressure.survival_margin,
        living_enemy_progress: -(living_enemy_count(&step.position.combat) as i32),
        phase_adjusted_enemy_progress: -phase_profile
            .enemy_phase
            .phase_adjusted_living_enemy_effort,
        split_debt_stability: -phase_profile.enemy_phase.split_debt_hp,
        mechanics_stability: -mechanics_pressure,
        pending_choice_fanout: -(phase_profile.pending_choice.estimated_action_fanout as i32),
        ordered_preference: -(ordered_index as i32),
    })
}

fn probe_upgrade_reason(
    candidate: RolloutActionProbeScore,
    fallback: RolloutActionProbeScore,
    allow_nonterminal_upgrade: bool,
) -> Option<&'static str> {
    if candidate.terminal_rank > fallback.terminal_rank {
        return Some(super::rollout_policy::ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_PROBE);
    }
    if candidate.terminal_rank < fallback.terminal_rank {
        return None;
    }
    if !allow_nonterminal_upgrade {
        return None;
    }
    if candidate.final_hp < fallback.final_hp
        || candidate.survival_margin < fallback.survival_margin
    {
        return None;
    }
    if candidate.final_hp > fallback.final_hp
        || candidate.survival_margin > fallback.survival_margin
    {
        return Some(
            super::rollout_policy::ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_SURVIVAL_VALUE,
        );
    }
    if candidate.phase_score() > fallback.phase_score() {
        Some(super::rollout_policy::ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_PHASE_VALUE)
    } else {
        None
    }
}

impl RolloutActionProbeScore {
    fn phase_score(self) -> RolloutPhaseProbeScore {
        RolloutPhaseProbeScore {
            split_debt_stability: self.split_debt_stability,
            mechanics_stability: self.mechanics_stability,
            pending_choice_fanout: self.pending_choice_fanout,
        }
    }
}

#[cfg(test)]
#[path = "rollout_probe_tests.rs"]
mod tests;
