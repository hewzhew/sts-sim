use std::cmp::Ordering;
use std::time::Instant;

use crate::sim::combat::{CombatPosition, CombatStepLimits, CombatStepResult, CombatStepper};

use super::super::phase_profile::combat_search_phase_profile;
use super::super::rollout_profile::RolloutPerformanceCounters;
use super::super::transition::terminal_label;
use super::super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RolloutActionProbeScore {
    pub(super) terminal_rank: i32,
    pub(super) final_hp: i32,
    pub(super) survival_margin: i32,
    pub(super) visible_hp_loss: i32,
    pub(super) living_enemy_progress: i32,
    pub(super) phase_adjusted_enemy_progress: i32,
    pub(super) split_debt_stability: i32,
    pub(super) mechanics_stability: i32,
    pub(super) action_sustained_mitigation: i32,
    pub(super) action_visible_mitigation: i32,
    pub(super) action_debuff_setup: i32,
    pub(super) action_progress_hint: i32,
    pub(super) action_reactive_safety: i32,
    pub(super) pending_choice_fanout: i32,
    pub(super) ordered_preference: i32,
    pub(super) nonterminal_upgrade_eligible: bool,
}

#[derive(Clone, Debug)]
pub(super) struct RolloutActionProbeResult {
    pub(super) score: RolloutActionProbeScore,
    pub(super) step: CombatStepResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub(super) struct RolloutTerminalProbeScore {
    pub(super) terminal_rank: i32,
    pub(super) final_hp: i32,
    pub(super) ordered_preference: i32,
}

#[derive(Clone, Debug)]
pub(super) struct RolloutTerminalProbeResult {
    pub(super) score: RolloutTerminalProbeScore,
    pub(super) step: CombatStepResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
struct RolloutPhaseProbeScore {
    split_debt_stability: i32,
    mechanics_stability: i32,
    pending_choice_fanout: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
struct RolloutActionFactsProbeScore {
    sustained_mitigation: i32,
    visible_mitigation: i32,
    debuff_setup: i32,
    progress_hint: i32,
    reactive_safety: i32,
}

impl Ord for RolloutActionProbeScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.terminal_rank
            .cmp(&other.terminal_rank)
            .then_with(|| self.final_hp.cmp(&other.final_hp))
            .then_with(|| self.survival_margin.cmp(&other.survival_margin))
            .then_with(|| other.visible_hp_loss.cmp(&self.visible_hp_loss))
            .then_with(|| self.living_enemy_progress.cmp(&other.living_enemy_progress))
            .then_with(|| {
                self.phase_adjusted_enemy_progress
                    .cmp(&other.phase_adjusted_enemy_progress)
            })
            .then_with(|| self.phase_score().cmp(&other.phase_score()))
            .then_with(|| self.action_facts_score().cmp(&other.action_facts_score()))
            .then_with(|| self.ordered_preference.cmp(&other.ordered_preference))
    }
}

impl PartialOrd for RolloutActionProbeScore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) fn probe_action(
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
    choice: &IndexedActionChoice,
    ordered_index: usize,
    performance: &mut RolloutPerformanceCounters,
) -> Option<RolloutActionProbeResult> {
    if deadline.is_some_and(|limit| Instant::now() >= limit) {
        return None;
    }
    performance.no_potion_probe_score_calls =
        performance.no_potion_probe_score_calls.saturating_add(1);
    let position = CombatPosition::new(node.engine.clone(), node.combat.clone());
    let engine_step_started = Instant::now();
    let step = stepper.apply_to_stable(
        &position,
        choice.choice.input.clone(),
        CombatStepLimits {
            max_engine_steps: config.max_engine_steps_per_action,
            deadline,
        },
    );
    performance.no_potion_probe_engine_step_elapsed_us = performance
        .no_potion_probe_engine_step_elapsed_us
        .saturating_add(engine_step_started.elapsed().as_micros());
    if step.truncated || step.timed_out {
        return None;
    }
    performance.no_potion_probe_actions_evaluated = performance
        .no_potion_probe_actions_evaluated
        .saturating_add(1);
    let phase_profile_started = Instant::now();
    let phase_profile = combat_search_phase_profile(&step.position.engine, &step.position.combat);
    performance.no_potion_probe_phase_profile_elapsed_us = performance
        .no_potion_probe_phase_profile_elapsed_us
        .saturating_add(phase_profile_started.elapsed().as_micros());
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
    let action_facts_started = Instant::now();
    let action_facts = summarize_action_facts_from_step(&node.combat, &choice.choice.input, &step);
    performance.no_potion_probe_action_facts_elapsed_us = performance
        .no_potion_probe_action_facts_elapsed_us
        .saturating_add(action_facts_started.elapsed().as_micros());
    let action_facts_score = action_facts_probe_score(&action_facts);
    let player_block = step.position.combat.entities.player.block;
    let visible_hp_loss = (phase_profile.pressure.visible_incoming_damage - player_block).max(0);
    let score = RolloutActionProbeScore {
        terminal_rank: terminal_rank(terminal),
        final_hp: step.position.combat.entities.player.current_hp,
        survival_margin: phase_profile.pressure.survival_margin,
        visible_hp_loss,
        living_enemy_progress: -(living_enemy_count(&step.position.combat) as i32),
        phase_adjusted_enemy_progress: -phase_profile
            .enemy_phase
            .phase_adjusted_living_enemy_effort,
        split_debt_stability: -phase_profile.enemy_phase.split_debt_hp,
        mechanics_stability: -mechanics_pressure,
        action_sustained_mitigation: action_facts_score.sustained_mitigation,
        action_visible_mitigation: action_facts_score.visible_mitigation,
        action_debuff_setup: action_facts_score.debuff_setup,
        action_progress_hint: action_facts_score.progress_hint,
        action_reactive_safety: action_facts_score.reactive_safety,
        pending_choice_fanout: -(phase_profile.pending_choice.estimated_action_fanout as i32),
        ordered_preference: -(ordered_index as i32),
        nonterminal_upgrade_eligible: !matches!(choice.choice.input, ClientInput::EndTurn),
    };
    Some(RolloutActionProbeResult { score, step })
}

pub(super) fn probe_terminal_action(
    node: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    deadline: Option<Instant>,
    choice: &IndexedActionChoice,
    ordered_index: usize,
    performance: &mut RolloutPerformanceCounters,
) -> Option<RolloutTerminalProbeResult> {
    if deadline.is_some_and(|limit| Instant::now() >= limit) {
        return None;
    }
    performance.no_potion_probe_score_calls =
        performance.no_potion_probe_score_calls.saturating_add(1);
    let position = CombatPosition::new(node.engine.clone(), node.combat.clone());
    let engine_step_started = Instant::now();
    let step = stepper.apply_to_stable(
        &position,
        choice.choice.input.clone(),
        CombatStepLimits {
            max_engine_steps: config.max_engine_steps_per_action,
            deadline,
        },
    );
    performance.no_potion_probe_engine_step_elapsed_us = performance
        .no_potion_probe_engine_step_elapsed_us
        .saturating_add(engine_step_started.elapsed().as_micros());
    if step.truncated || step.timed_out {
        return None;
    }
    performance.no_potion_probe_actions_evaluated = performance
        .no_potion_probe_actions_evaluated
        .saturating_add(1);
    let terminal = terminal_label(&step.position.engine, &step.position.combat);
    Some(RolloutTerminalProbeResult {
        score: RolloutTerminalProbeScore {
            terminal_rank: terminal_rank(terminal),
            final_hp: step.position.combat.entities.player.current_hp,
            ordered_preference: -(ordered_index as i32),
        },
        step,
    })
}

pub(super) fn probe_upgrade_reason(
    candidate: RolloutActionProbeScore,
    fallback: RolloutActionProbeScore,
    allow_nonterminal_upgrade: bool,
) -> Option<&'static str> {
    if candidate.terminal_rank > fallback.terminal_rank {
        return Some(
            super::super::rollout_policy::ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_PROBE,
        );
    }
    if candidate.terminal_rank < fallback.terminal_rank {
        return None;
    }
    if !allow_nonterminal_upgrade {
        return None;
    }
    if !candidate.nonterminal_upgrade_eligible {
        return None;
    }
    if candidate.final_hp < fallback.final_hp
        || candidate.survival_margin < fallback.survival_margin
    {
        return None;
    }
    if candidate.final_hp > fallback.final_hp {
        return Some(
            super::super::rollout_policy::ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_SURVIVAL_VALUE,
        );
    }
    if candidate.visible_hp_loss < fallback.visible_hp_loss {
        return Some(
            super::super::rollout_policy::ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_SURVIVAL_VALUE,
        );
    }
    if candidate.action_reactive_safety < fallback.action_reactive_safety {
        return None;
    }
    if candidate.phase_score() > fallback.phase_score() {
        Some(super::super::rollout_policy::ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_PHASE_VALUE)
    } else if candidate.action_facts_score() > fallback.action_facts_score() {
        Some(
            super::super::rollout_policy::ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_ACTION_FACTS_VALUE,
        )
    } else {
        None
    }
}

fn action_facts_probe_score(facts: &CombatSearchV2ActionFacts) -> RolloutActionFactsProbeScore {
    RolloutActionFactsProbeScore {
        sustained_mitigation: facts.mechanics.persistent_enemy_strength_down,
        visible_mitigation: facts
            .mechanics
            .temporary_enemy_strength_down
            .saturating_add(facts.mechanics.visible_attack_mitigation_hint)
            .saturating_add(facts.mechanics.enemy_weak),
        debuff_setup: facts.mechanics.enemy_vulnerable,
        progress_hint: facts
            .immediate
            .target_progress_hint
            .max(facts.immediate.all_enemy_progress_hint)
            .saturating_add(facts.mechanics.reactive_enemy_damage),
        reactive_safety: -facts
            .mechanics
            .enemy_strength_gain
            .saturating_add(facts.mechanics.visible_attack_pressure_hint)
            .saturating_add(facts.mechanics.reactive_player_hp_loss)
            .saturating_add(facts.mechanics.reactive_bad_draw_cards)
            .saturating_add(i32::from(facts.mechanics.reactive_forced_turn_end)),
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

    fn action_facts_score(self) -> RolloutActionFactsProbeScore {
        RolloutActionFactsProbeScore {
            sustained_mitigation: self.action_sustained_mitigation,
            visible_mitigation: self.action_visible_mitigation,
            debuff_setup: self.action_debuff_setup,
            progress_hint: self.action_progress_hint,
            reactive_safety: self.action_reactive_safety,
        }
    }
}
