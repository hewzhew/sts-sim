use crate::bot::combat_posture::CombatPostureFeatures;
use crate::combat::CombatState;
use crate::combat::PowerId;
use crate::content::cards::CardId;
use crate::state::core::ClientInput;
use crate::state::EngineState;

use super::profile::{SearchProfileCollector, SearchProfilePhase, SearchProfilingLevel};
use super::root_policy::{
    action_semantic_tags, immediate_exhaust_setup_bonus, posture_snapshot,
    sequencing_assessment_for_input, StatePressureFeatures,
};
use super::root_policy::{total_incoming_damage, TransitionPressureDelta};
use super::root_rollout::{
    advance_to_decision_point, is_terminal, project_turn_close_state, total_enemy_hp,
};
use super::{default_equivalence_mode, get_legal_moves, reduce_search_moves};

#[derive(Clone, Debug)]
pub(super) struct SequenceCandidate {
    pub input: ClientInput,
    pub after_engine: EngineState,
    pub after_combat: CombatState,
    pub passive: bool,
    pub projected_hp: i32,
    pub projected_block: i32,
    pub projected_enemy_total: i32,
    pub projected_unblocked: i32,
    pub survives: bool,
}

#[derive(Clone, Debug)]
pub(super) struct SequenceAdjustment {
    pub input: ClientInput,
    pub total_delta: f32,
    pub realized_exhaust_block: i32,
    pub realized_exhaust_draw: i32,
    pub survival_window_delta: f32,
    pub exhaust_evidence_delta: f32,
    pub frontload_delta: f32,
    pub defer_delta: f32,
    pub branch_opening_delta: f32,
    pub downside_penalty_delta: f32,
}

#[derive(Clone, Copy, Debug)]
struct LocalSequenceOutcome {
    projected_hp: i32,
    projected_block: i32,
    projected_enemy_total: i32,
    projected_unblocked: i32,
    survives: bool,
    realized_exhaust_block: i32,
    realized_exhaust_draw: i32,
}

#[derive(Clone, Copy, Debug)]
struct DeferredSetupOutcome {
    outcome: LocalSequenceOutcome,
    turns_delayed: usize,
    safe_window_unblocked: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SetupCardRef {
    id: CardId,
    uuid: u32,
}

#[derive(Clone, Copy)]
struct FollowupContext {
    pressure: StatePressureFeatures,
    posture: CombatPostureFeatures,
    feel_no_pain_live: bool,
    dark_embrace_live: bool,
    exhaust_payoff_live: i32,
}

#[derive(Clone, Copy, Debug, Default)]
struct SequenceScoreBreakdown {
    total_delta: f32,
    survival_window_delta: f32,
    exhaust_evidence_delta: f32,
    frontload_delta: f32,
    defer_delta: f32,
    branch_opening_delta: f32,
    downside_penalty_delta: f32,
}

impl SequenceScoreBreakdown {
    fn add_survival(&mut self, delta: f32) {
        self.total_delta += delta;
        self.survival_window_delta += delta;
    }

    fn add_exhaust(&mut self, delta: f32) {
        self.total_delta += delta;
        self.exhaust_evidence_delta += delta;
    }

    fn add_sequencing(&mut self, frontload: f32, defer: f32, branch: f32, downside: f32) {
        self.frontload_delta += frontload;
        self.defer_delta += defer;
        self.branch_opening_delta += branch;
        self.downside_penalty_delta += downside;
        self.total_delta += frontload + branch - defer - downside;
    }
}

pub(super) struct SequenceJudge<'a> {
    combat: &'a CombatState,
    candidates: &'a [SequenceCandidate],
    pressure: StatePressureFeatures,
    posture: CombatPostureFeatures,
    opening_turn: bool,
    high_pressure: bool,
    low_pressure: bool,
    has_active_non_end: bool,
    has_meaningful_non_end: bool,
    best_active_idx: Option<usize>,
    best_non_potion_idx: Option<usize>,
    max_followup_decisions: usize,
    max_followup_branch_width: usize,
    max_engine_steps: usize,
}

impl<'a> SequenceJudge<'a> {
    pub(super) fn new(combat: &'a CombatState, candidates: &'a [SequenceCandidate]) -> Self {
        let pressure = StatePressureFeatures::from_combat(combat);
        let posture = posture_snapshot(combat);
        let current_enemy_total = total_enemy_hp(combat);
        let has_active_non_end = candidates.iter().any(|candidate| {
            !candidate.passive && !matches!(candidate.input, ClientInput::EndTurn)
        });
        let has_meaningful_non_end = candidates.iter().any(|candidate| {
            !matches!(candidate.input, ClientInput::EndTurn)
                && (!candidate.passive
                    || candidate.projected_enemy_total < current_enemy_total
                    || candidate.projected_block > combat.entities.player.block)
        });
        let opening_turn = combat.turn.turn_count <= 1;
        let high_pressure = pressure.urgent_pressure
            || (pressure.value_unblocked > 0 && combat.entities.player.current_hp <= 30);
        let low_pressure = pressure.value_unblocked == 0
            && !combat.meta.is_elite_fight
            && combat.entities.player.current_hp > 20;
        let best_active_idx = candidates
            .iter()
            .enumerate()
            .filter(|(_, candidate)| !matches!(candidate.input, ClientInput::EndTurn))
            .max_by_key(|(_, candidate)| {
                (
                    candidate.survives,
                    -candidate.projected_unblocked,
                    candidate.projected_hp,
                    candidate.projected_block,
                    -candidate.projected_enemy_total,
                )
            })
            .map(|(idx, _)| idx);
        let best_non_potion_idx = candidates
            .iter()
            .enumerate()
            .filter(|(_, candidate)| {
                !action_semantic_tags(combat, &candidate.input).defensive_potion
            })
            .filter(|(_, candidate)| !matches!(candidate.input, ClientInput::EndTurn))
            .max_by_key(|(_, candidate)| {
                (
                    candidate.survives,
                    -candidate.projected_unblocked,
                    candidate.projected_hp,
                    candidate.projected_block,
                    -candidate.projected_enemy_total,
                )
            })
            .map(|(idx, _)| idx);

        Self {
            combat,
            candidates,
            pressure,
            posture,
            opening_turn,
            high_pressure,
            low_pressure,
            has_active_non_end,
            has_meaningful_non_end,
            best_active_idx,
            best_non_potion_idx,
            max_followup_decisions: 2,
            max_followup_branch_width: 3,
            max_engine_steps: 80,
        }
    }

    pub(super) fn adjustments(&self) -> Vec<SequenceAdjustment> {
        self.candidates
            .iter()
            .map(|candidate| {
                let tags = action_semantic_tags(self.combat, &candidate.input);
                let evidence = if tags.persistent_setup {
                    self.local_sequence_outcome(candidate)
                } else {
                    LocalSequenceOutcome {
                        projected_hp: candidate.projected_hp,
                        projected_block: candidate.projected_block,
                        projected_enemy_total: candidate.projected_enemy_total,
                        projected_unblocked: candidate.projected_unblocked,
                        survives: candidate.survives,
                        realized_exhaust_block: 0,
                        realized_exhaust_draw: 0,
                    }
                };
                let breakdown = self.sequence_breakdown(candidate, &evidence);
                SequenceAdjustment {
                    input: candidate.input.clone(),
                    total_delta: breakdown.total_delta,
                    realized_exhaust_block: evidence.realized_exhaust_block,
                    realized_exhaust_draw: evidence.realized_exhaust_draw,
                    survival_window_delta: breakdown.survival_window_delta,
                    exhaust_evidence_delta: breakdown.exhaust_evidence_delta,
                    frontload_delta: breakdown.frontload_delta,
                    defer_delta: breakdown.defer_delta,
                    branch_opening_delta: breakdown.branch_opening_delta,
                    downside_penalty_delta: breakdown.downside_penalty_delta,
                }
            })
            .collect()
    }

    fn sequence_breakdown(
        &self,
        candidate: &SequenceCandidate,
        evidence: &LocalSequenceOutcome,
    ) -> SequenceScoreBreakdown {
        let mut breakdown = SequenceScoreBreakdown::default();
        breakdown.add_survival(self.survive_now_delta(candidate));
        let (setup_survival, setup_exhaust) =
            self.setup_then_survive_components(candidate, evidence);
        breakdown.add_survival(setup_survival);
        breakdown.add_exhaust(setup_exhaust);
        let (defer_survival, defer_exhaust) =
            self.defer_setup_to_safe_window_components(candidate, evidence);
        breakdown.add_survival(defer_survival);
        breakdown.add_exhaust(defer_exhaust);
        breakdown.add_survival(self.potion_bridge_delta(candidate));
        let (frontload, defer, branch, downside) = self.sequencing_components(candidate);
        breakdown.add_sequencing(frontload, defer, branch, downside);
        breakdown
    }

    fn sequencing_components(&self, candidate: &SequenceCandidate) -> (f32, f32, f32, f32) {
        let has_safe_line = self
            .best_active()
            .is_some_and(|best| best.survives && best.projected_unblocked <= 0);
        let Some(assessment) =
            sequencing_assessment_for_input(self.combat, &candidate.input, has_safe_line)
        else {
            return (0.0, 0.0, 0.0, 0.0);
        };
        (
            assessment.frontload_bonus as f32,
            assessment.defer_bonus as f32,
            assessment.branch_value as f32,
            assessment.downside_penalty as f32,
        )
    }

    fn survive_now_delta(&self, candidate: &SequenceCandidate) -> f32 {
        let mut delta = 0.0;
        let tags = action_semantic_tags(self.combat, &candidate.input);

        if matches!(candidate.input, ClientInput::EndTurn)
            && ((self.opening_turn && self.has_meaningful_non_end)
                || (self.low_pressure && self.has_active_non_end))
        {
            delta -= 5_000_000.0;
        }

        let setup_is_current_best_survival_line = tags.persistent_setup
            && candidate.survives
            && self
                .best_active()
                .is_some_and(|best| best.input == candidate.input);

        if self.high_pressure
            && candidate.passive
            && self.has_active_non_end
            && !setup_is_current_best_survival_line
        {
            delta -= if matches!(candidate.input, ClientInput::EndTurn) {
                4_000_000.0
            } else {
                1_500_000.0
            };
        }

        if let Some(best) = self.best_active() {
            let dominated_on_survival = best.survives && !candidate.survives;
            let dominated_on_unblocked =
                best.projected_unblocked + 4 < candidate.projected_unblocked;
            let dominated_on_board = best.projected_hp >= candidate.projected_hp + 5
                || best.projected_block >= candidate.projected_block + 8
                || best.projected_enemy_total + 6 < candidate.projected_enemy_total;

            if self.pressure.lethal_pressure && dominated_on_survival {
                delta -= if matches!(candidate.input, ClientInput::EndTurn) {
                    8_000_000.0
                } else {
                    6_000_000.0
                };
            } else if self.high_pressure
                && candidate.passive
                && (dominated_on_survival || dominated_on_unblocked || dominated_on_board)
            {
                delta -= if matches!(candidate.input, ClientInput::EndTurn) {
                    5_000_000.0
                } else {
                    2_500_000.0
                };
            } else if self.low_pressure
                && matches!(candidate.input, ClientInput::EndTurn)
                && best.projected_enemy_total < candidate.projected_enemy_total
            {
                delta -= 3_000_000.0;
            }
        }

        delta
    }

    fn setup_then_survive_components(
        &self,
        candidate: &SequenceCandidate,
        outcome: &LocalSequenceOutcome,
    ) -> (f32, f32) {
        let tags = action_semantic_tags(self.combat, &candidate.input);
        if !tags.persistent_setup {
            return (0.0, 0.0);
        }

        let Some(best) = self.best_active() else {
            return (0.0, 0.0);
        };

        if !outcome.survives {
            return (0.0, 0.0);
        }
        let realized_block = outcome.realized_exhaust_block;
        let realized_draw = outcome.realized_exhaust_draw;
        let heuristic_payoff = if tags.exhaust_engine {
            immediate_exhaust_setup_bonus(&candidate.after_combat, tags.draw_core)
        } else {
            0
        };
        let evidence_payoff =
            realized_block * 260 + realized_draw * if tags.draw_core { 420 } else { 280 };
        let short_window_payoff = heuristic_payoff + evidence_payoff;

        if !best.survives {
            return (
                1_800_000.0 + heuristic_payoff as f32 * 18.0,
                evidence_payoff as f32 * 18.0,
            );
        }

        if self.high_pressure {
            if short_window_payoff > 0
                && outcome.projected_unblocked <= best.projected_unblocked + 2
                && outcome.projected_hp + 2 >= best.projected_hp
            {
                return (
                    220_000.0
                        + heuristic_payoff as f32 * 24.0
                        + self.posture.setup_payoff_density as f32 * 12_000.0,
                    evidence_payoff as f32 * 24.0,
                );
            }
            if outcome.projected_unblocked == 0
                && outcome.projected_hp + 2 >= best.projected_hp
                && outcome.projected_enemy_total <= best.projected_enemy_total + 12
            {
                return (350_000.0, 0.0);
            }
            if outcome.projected_unblocked <= best.projected_unblocked + 2
                && outcome.projected_hp + 4 >= best.projected_hp
            {
                return (120_000.0, 0.0);
            }
            return (0.0, 0.0);
        }

        if short_window_payoff > 0 {
            (
                120_000.0
                    + heuristic_payoff as f32 * 20.0
                    + self.posture.expected_fight_length_bucket as f32 * 15_000.0,
                evidence_payoff as f32 * 20.0,
            )
        } else if outcome.projected_enemy_total <= best.projected_enemy_total + 12 {
            (
                90_000.0 + self.posture.setup_payoff_density as f32 * 8_000.0,
                0.0,
            )
        } else {
            (0.0, 0.0)
        }
    }

    fn defer_setup_to_safe_window_components(
        &self,
        candidate: &SequenceCandidate,
        play_now: &LocalSequenceOutcome,
    ) -> (f32, f32) {
        let tags = action_semantic_tags(self.combat, &candidate.input);
        if !tags.persistent_setup {
            return (0.0, 0.0);
        }
        let Some(setup_card) = setup_card_ref(self.combat, &candidate.input) else {
            return (0.0, 0.0);
        };
        let play_now_window_unblocked =
            StatePressureFeatures::from_combat(&candidate.after_combat).value_unblocked;
        let Some(deferred) = self.best_deferred_setup_outcome(candidate, setup_card) else {
            return (0.0, 0.0);
        };

        if deferred_outcome_key(&deferred)
            > deferred_outcome_key(&DeferredSetupOutcome {
                outcome: *play_now,
                turns_delayed: 0,
                safe_window_unblocked: play_now_window_unblocked,
            })
        {
            let (survival_bonus, exhaust_bonus) = deferred_safe_window_bonus(
                candidate,
                play_now,
                play_now_window_unblocked,
                &deferred,
                self.pressure,
            );
            let penalty_scale = if self.pressure.lethal_pressure {
                1.0
            } else if self.high_pressure {
                0.45
            } else {
                0.10
            };
            let posture_scale = if self.pressure.urgent_pressure {
                6_000.0
            } else {
                1_200.0
            };
            return (
                -(220_000.0 * penalty_scale)
                    - survival_bonus * penalty_scale
                    - self.posture.setup_payoff_density as f32 * posture_scale,
                -(exhaust_bonus * penalty_scale),
            );
        }
        if deferred_outcome_key(&DeferredSetupOutcome {
            outcome: *play_now,
            turns_delayed: 0,
            safe_window_unblocked: play_now_window_unblocked,
        }) > deferred_outcome_key(&deferred)
        {
            return (40_000.0, 0.0);
        }

        (0.0, 0.0)
    }

    fn potion_bridge_delta(&self, candidate: &SequenceCandidate) -> f32 {
        let tags = action_semantic_tags(self.combat, &candidate.input);
        if !tags.defensive_potion {
            return 0.0;
        }

        let Some(best_non_potion) = self.best_non_potion() else {
            return 0.0;
        };

        if self.pressure.lethal_pressure && candidate.survives && !best_non_potion.survives {
            return 2_500_000.0;
        }
        if self.high_pressure
            && candidate.projected_unblocked + 6 < best_non_potion.projected_unblocked
        {
            return 650_000.0;
        }
        if candidate.survives && best_non_potion.projected_hp >= candidate.projected_hp + 8 {
            return -120_000.0;
        }

        0.0
    }

    fn local_sequence_outcome(&self, candidate: &SequenceCandidate) -> LocalSequenceOutcome {
        evaluate_followup_sequence(
            &candidate.after_engine,
            &candidate.after_combat,
            self.max_followup_decisions,
            self.max_followup_branch_width,
            self.max_engine_steps,
        )
    }

    fn best_active(&self) -> Option<&SequenceCandidate> {
        self.best_active_idx.map(|idx| &self.candidates[idx])
    }

    fn best_non_potion(&self) -> Option<&SequenceCandidate> {
        self.best_non_potion_idx.map(|idx| &self.candidates[idx])
    }

    fn best_deferred_setup_outcome(
        &self,
        setup_candidate: &SequenceCandidate,
        setup_card: SetupCardRef,
    ) -> Option<DeferredSetupOutcome> {
        let mut best = None;
        for candidate in self.candidates {
            if candidate.input == setup_candidate.input {
                continue;
            }
            let tags = action_semantic_tags(self.combat, &candidate.input);
            if tags.persistent_setup {
                continue;
            }
            if !candidate.survives {
                continue;
            }
            if candidate.projected_unblocked > setup_candidate.projected_unblocked + 2
                && self.high_pressure
            {
                continue;
            }

            if let Some(outcome) = deferred_setup_outcome_from_state(
                &candidate.after_engine,
                &candidate.after_combat,
                setup_card,
                self.max_engine_steps,
            ) {
                if best.as_ref().is_none_or(|current| {
                    deferred_outcome_key(&outcome) > deferred_outcome_key(current)
                }) {
                    best = Some(outcome);
                }
            }
        }
        best
    }
}

fn evaluate_followup_sequence(
    engine: &EngineState,
    combat: &CombatState,
    depth_left: usize,
    branch_width: usize,
    max_engine_steps: usize,
) -> LocalSequenceOutcome {
    if depth_left == 0 || is_terminal(engine, combat) {
        return evaluate_turn_close_outcome(engine, combat, max_engine_steps);
    }

    let legal_moves = get_legal_moves(engine, combat);
    if legal_moves.is_empty() {
        return evaluate_turn_close_outcome(engine, combat, max_engine_steps);
    }

    let context = FollowupContext {
        pressure: StatePressureFeatures::from_combat(combat),
        posture: posture_snapshot(combat),
        feel_no_pain_live: combat.get_power(0, PowerId::FeelNoPain) > 0,
        dark_embrace_live: combat.get_power(0, PowerId::DarkEmbrace) > 0,
        exhaust_payoff_live: immediate_exhaust_setup_bonus(combat, false),
    };
    let mut profiler = SearchProfileCollector::new(SearchProfilingLevel::Off);
    let mut ranked = reduce_search_moves(
        engine,
        combat,
        &legal_moves,
        max_engine_steps,
        default_equivalence_mode(),
        &mut profiler,
        SearchProfilePhase::Recursive,
    )
    .into_iter()
    .map(|reduced| {
        let priority =
            followup_move_priority(combat, &reduced.input, &reduced.next_combat, &context);
        let (exhaust_block, exhaust_draw) = transition_exhaust_payoff(combat, &reduced.next_combat);
        (
            reduced.input,
            reduced.next_engine,
            reduced.next_combat,
            priority,
            exhaust_block,
            exhaust_draw,
        )
    })
    .collect::<Vec<_>>();
    // Sort descending by follow-up priority so the bounded rollout keeps the strongest continuations.
    ranked.sort_by(|left, right| right.3.total_cmp(&left.3));

    let mut best = None;
    for (_, next_engine, next_combat, _, exhaust_block, exhaust_draw) in
        ranked.into_iter().take(branch_width.max(1))
    {
        let child = evaluate_followup_sequence(
            &next_engine,
            &next_combat,
            depth_left.saturating_sub(1),
            branch_width,
            max_engine_steps,
        );
        let outcome = LocalSequenceOutcome {
            realized_exhaust_block: exhaust_block + child.realized_exhaust_block,
            realized_exhaust_draw: exhaust_draw + child.realized_exhaust_draw,
            ..child
        };
        if best
            .as_ref()
            .is_none_or(|current| local_outcome_key(&outcome) > local_outcome_key(current))
        {
            best = Some(outcome);
        }
    }

    best.unwrap_or_else(|| evaluate_turn_close_outcome(engine, combat, max_engine_steps))
}

fn deferred_setup_outcome_from_state(
    engine: &EngineState,
    combat: &CombatState,
    setup_card: SetupCardRef,
    max_engine_steps: usize,
) -> Option<DeferredSetupOutcome> {
    let mut best = immediate_setup_window_outcome(engine, combat, setup_card, max_engine_steps, 0);

    if let Some(next_turn) =
        next_turn_setup_window_outcome(engine, combat, setup_card, max_engine_steps)
    {
        if best
            .as_ref()
            .is_none_or(|current| deferred_outcome_key(&next_turn) > deferred_outcome_key(current))
        {
            best = Some(next_turn);
        }
    }

    best
}

fn immediate_setup_window_outcome(
    engine: &EngineState,
    combat: &CombatState,
    setup_card: SetupCardRef,
    max_engine_steps: usize,
    turns_delayed: usize,
) -> Option<DeferredSetupOutcome> {
    if is_terminal(engine, combat) {
        return None;
    }
    let legal_moves = get_legal_moves(engine, combat);
    let mut best = None;
    let safe_window_unblocked = StatePressureFeatures::from_combat(combat).value_unblocked;

    for input in legal_moves {
        let ClientInput::PlayCard { card_index, target } = input else {
            continue;
        };
        let Some(card) = combat.zones.hand.get(card_index) else {
            continue;
        };
        if card.id != setup_card.id || card.uuid != setup_card.uuid {
            continue;
        }

        let mut next_engine = engine.clone();
        let mut next_combat = combat.clone();
        advance_to_decision_point(
            &mut next_engine,
            &mut next_combat,
            ClientInput::PlayCard { card_index, target },
            max_engine_steps,
        );
        let deferred = DeferredSetupOutcome {
            outcome: evaluate_followup_sequence(&next_engine, &next_combat, 1, 3, max_engine_steps),
            turns_delayed,
            safe_window_unblocked,
        };
        if best
            .as_ref()
            .is_none_or(|current| deferred_outcome_key(&deferred) > deferred_outcome_key(current))
        {
            best = Some(deferred);
        }
    }

    best
}

fn next_turn_setup_window_outcome(
    engine: &EngineState,
    combat: &CombatState,
    setup_card: SetupCardRef,
    max_engine_steps: usize,
) -> Option<DeferredSetupOutcome> {
    if !matches!(engine, EngineState::CombatPlayerTurn) {
        return None;
    }

    let mut next_engine = engine.clone();
    let mut next_combat = combat.clone();
    advance_to_decision_point(
        &mut next_engine,
        &mut next_combat,
        ClientInput::EndTurn,
        max_engine_steps,
    );
    if is_terminal(&next_engine, &next_combat) {
        return None;
    }

    immediate_setup_window_outcome(&next_engine, &next_combat, setup_card, max_engine_steps, 1)
}

fn evaluate_turn_close_outcome(
    engine: &EngineState,
    combat: &CombatState,
    max_engine_steps: usize,
) -> LocalSequenceOutcome {
    let (projected_engine, projected_combat) =
        project_turn_close_state(engine, combat, max_engine_steps);
    LocalSequenceOutcome {
        projected_hp: projected_combat.entities.player.current_hp,
        projected_block: projected_combat.entities.player.block,
        projected_enemy_total: total_enemy_hp(&projected_combat),
        projected_unblocked: (total_incoming_damage(&projected_combat)
            - projected_combat.entities.player.block)
            .max(0),
        survives: !matches!(
            projected_engine,
            EngineState::GameOver(crate::state::core::RunResult::Defeat)
        ) && projected_combat.entities.player.current_hp > 0,
        realized_exhaust_block: 0,
        realized_exhaust_draw: 0,
    }
}

fn followup_move_priority(
    combat: &CombatState,
    input: &ClientInput,
    next_combat: &CombatState,
    context: &FollowupContext,
) -> f32 {
    let delta = TransitionPressureDelta::between(combat, next_combat);
    let tags = action_semantic_tags(combat, input);
    let net_hand_replacement =
        next_combat.zones.hand.len() as i32 - combat.zones.hand.len() as i32 + 1;
    let mut priority = 0.0;

    priority += delta.block_gain.min(18) as f32 * 320.0;
    priority += delta.incoming_reduction.min(18) as f32 * 280.0;
    priority -= delta.after_unblocked.min(24) as f32 * 190.0;
    priority -= total_enemy_hp(next_combat) as f32 * 1.5;

    if tags.defensive_potion {
        priority += if context.pressure.lethal_pressure {
            4_000.0
        } else {
            1_200.0
        };
    }
    if tags.persistent_setup {
        let setup_signal = if tags.exhaust_engine {
            immediate_exhaust_setup_bonus(next_combat, tags.draw_core) as f32
        } else {
            0.0
        };
        priority += if context.pressure.urgent_pressure {
            -2_000.0
        } else {
            400.0
        };
        priority += setup_signal * 0.6;
        priority += context.posture.setup_payoff_density as f32 * 280.0;
        priority += context.posture.expected_fight_length_bucket as f32 * 420.0;
        priority -= context.posture.immediate_survival_pressure.min(20) as f32 * 160.0;
    }
    if tags.exhaust_trigger {
        priority += 900.0;
        priority += context.exhaust_payoff_live.min(6_000) as f32 * 0.25;
        if context.feel_no_pain_live {
            priority += delta.block_gain.min(20) as f32 * 220.0;
            if delta.block_gain > 0 {
                priority += 1_200.0;
            }
        }
        if context.dark_embrace_live {
            priority += net_hand_replacement.max(0) as f32 * 1_600.0;
        }
    }
    if tags.block_core {
        priority += delta.block_gain.min(18) as f32 * 120.0;
    }
    if tags.resource_bridge {
        priority += if context.pressure.urgent_pressure {
            800.0
        } else {
            250.0
        };
    }
    if tags.attack_like
        && delta.block_gain == 0
        && delta.incoming_reduction == 0
        && context.pressure.urgent_pressure
    {
        priority -= 1_400.0;
    }
    if matches!(input, ClientInput::EndTurn) {
        priority -= 2_500.0;
    }

    priority
}

fn setup_card_ref(combat: &CombatState, input: &ClientInput) -> Option<SetupCardRef> {
    let ClientInput::PlayCard { card_index, .. } = input else {
        return None;
    };
    combat.zones.hand.get(*card_index).map(|card| SetupCardRef {
        id: card.id,
        uuid: card.uuid,
    })
}

fn deferred_safe_window_bonus(
    candidate: &SequenceCandidate,
    play_now: &LocalSequenceOutcome,
    play_now_window_unblocked: i32,
    deferred: &DeferredSetupOutcome,
    pressure: StatePressureFeatures,
) -> (f32, f32) {
    let mut survival_bonus = 0.0;
    let mut exhaust_bonus = 0.0;
    if deferred.safe_window_unblocked + 2 < play_now_window_unblocked {
        survival_bonus +=
            (play_now_window_unblocked - deferred.safe_window_unblocked).min(12) as f32 * 18_000.0;
    }
    if deferred.outcome.projected_hp > play_now.projected_hp {
        survival_bonus +=
            (deferred.outcome.projected_hp - play_now.projected_hp).min(10) as f32 * 12_000.0;
    }
    if deferred.outcome.realized_exhaust_block > play_now.realized_exhaust_block {
        exhaust_bonus += (deferred.outcome.realized_exhaust_block - play_now.realized_exhaust_block)
            .min(16) as f32
            * 14_000.0;
    }
    if deferred.outcome.realized_exhaust_draw > play_now.realized_exhaust_draw {
        exhaust_bonus += (deferred.outcome.realized_exhaust_draw - play_now.realized_exhaust_draw)
            .min(6) as f32
            * 18_000.0;
    }
    if pressure.urgent_pressure && candidate.projected_unblocked > 0 {
        survival_bonus += 80_000.0;
    }
    if deferred.turns_delayed == 1 && deferred.safe_window_unblocked == 0 {
        survival_bonus += 40_000.0;
    }
    (survival_bonus, exhaust_bonus)
}

fn local_outcome_key(outcome: &LocalSequenceOutcome) -> (bool, i32, i32, i32, i32, i32, i32) {
    (
        outcome.survives,
        -outcome.projected_unblocked,
        outcome.projected_hp,
        outcome.projected_block,
        -outcome.projected_enemy_total,
        outcome.realized_exhaust_block,
        outcome.realized_exhaust_draw,
    )
}

fn deferred_outcome_key(
    outcome: &DeferredSetupOutcome,
) -> (bool, i32, i32, i32, i32, i32, i32, i32) {
    let local = local_outcome_key(&outcome.outcome);
    (
        local.0,
        local.1,
        local.2,
        local.3,
        local.4,
        local.5,
        local.6,
        -(outcome.turns_delayed as i32) * 10 - outcome.safe_window_unblocked,
    )
}

fn transition_exhaust_payoff(before: &CombatState, after: &CombatState) -> (i32, i32) {
    let exhausted_cards =
        (after.zones.exhaust_pile.len() as i32 - before.zones.exhaust_pile.len() as i32).max(0);
    if exhausted_cards == 0 {
        return (0, 0);
    }

    let feel_no_pain = before.get_power(0, PowerId::FeelNoPain).max(0);
    let dark_embrace = before.get_power(0, PowerId::DarkEmbrace).max(0);
    (
        exhausted_cards * feel_no_pain,
        exhausted_cards * dark_embrace,
    )
}

