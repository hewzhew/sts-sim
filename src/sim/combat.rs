use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::engine::core::{
    is_smoke_escape_stable_boundary, tick_engine_with_profiler, CombatEnginePhaseProfiler,
    CombatEngineProfilePhaseV1, NoopCombatEnginePhaseProfiler, COMBAT_ENGINE_PHASE_PROFILE_COUNT,
};
use crate::runtime::combat::{CardZones, CombatRuntimeHints, CombatState, EntityState};
use crate::sim::combat_action::CombatActionChoice;
use crate::sim::combat_action_surface::CombatLegalActionSurfaceV2;
use crate::state::core::{ClientInput, EngineState, RunResult};
use crate::state::DomainCardSnapshot;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CombatPosition {
    pub engine: EngineState,
    pub combat: CombatState,
}

impl CombatPosition {
    pub fn new(engine: EngineState, combat: CombatState) -> Self {
        Self { engine, combat }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatTerminal {
    Win,
    Loss,
    Unresolved,
}

#[derive(Clone, Copy, Debug)]
pub struct CombatStepLimits {
    pub max_engine_steps: usize,
    pub deadline: Option<Instant>,
}

#[derive(Clone, Debug)]
pub struct CombatStepResult {
    pub position: CombatPosition,
    pub terminal: CombatTerminal,
    pub alive: bool,
    pub truncated: bool,
    pub timed_out: bool,
    pub engine_steps: usize,
}

#[derive(Clone, Debug)]
pub struct CombatObservedStepResultV1 {
    pub step: CombatStepResult,
    pub drawn_cards: Vec<DomainCardSnapshot>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CombatStepPerformanceTimingV1 {
    pub engine_clone_elapsed_ns: u64,
    pub combat_clone_elapsed_ns: u64,
    pub combat_meta_clone_elapsed_ns: u64,
    pub combat_turn_clone_elapsed_ns: u64,
    pub combat_zones_clone_elapsed_ns: u64,
    pub combat_entities_clone_elapsed_ns: u64,
    pub combat_zone_component_elapsed_ns: [u64; 6],
    pub combat_entity_component_elapsed_ns: [u64; 4],
    pub combat_runtime_component_elapsed_ns: [u64; 6],
    pub combat_engine_clone_elapsed_ns: u64,
    pub combat_rng_clone_elapsed_ns: u64,
    pub combat_runtime_clone_elapsed_ns: u64,
    pub execution_elapsed_ns: u64,
    pub engine_phase_elapsed_ns: [u64; COMBAT_ENGINE_PHASE_PROFILE_COUNT],
    pub engine_phase_occurrences: [usize; COMBAT_ENGINE_PHASE_PROFILE_COUNT],
}

#[derive(Default)]
struct CombatEnginePhaseTimingProfilerV1 {
    elapsed_ns: [u64; COMBAT_ENGINE_PHASE_PROFILE_COUNT],
    occurrences: [usize; COMBAT_ENGINE_PHASE_PROFILE_COUNT],
}

impl CombatEnginePhaseProfiler for CombatEnginePhaseTimingProfilerV1 {
    type Marker = Instant;

    fn begin(&mut self, _phase: CombatEngineProfilePhaseV1) -> Self::Marker {
        Instant::now()
    }

    fn end(&mut self, phase: CombatEngineProfilePhaseV1, marker: Self::Marker) {
        let index = phase.index();
        self.elapsed_ns[index] =
            self.elapsed_ns[index].saturating_add(elapsed_nanos_saturated(marker));
        self.occurrences[index] = self.occurrences[index].saturating_add(1);
    }
}

pub trait CombatStepper {
    /// Returns the finite, explicitly materialized part of this boundary.
    ///
    /// For the engine stepper this deliberately excludes combinatorial
    /// Hand/Grid/Scry payload families.  Callers that need the complete input
    /// language must inspect `legal_action_surface` and schedule structured
    /// families under their own budget.
    fn atomic_actions(&self, position: &CombatPosition) -> Vec<ClientInput>;

    /// Describes the complete legal-input domain without forcing structured
    /// selection families into an eager `Vec`.
    fn legal_action_surface(&self, position: &CombatPosition) -> CombatLegalActionSurfaceV2 {
        CombatLegalActionSurfaceV2 {
            atomic_actions: self.atomic_actions(position),
            selection_families: Vec::new(),
            indexed_choice: None,
        }
    }

    /// Opts into search-owned factorization of the engine's canonical
    /// Hand/Grid/Scry pending-choice surface. Custom steppers retain full
    /// ownership of their action model unless they explicitly opt in.
    fn supports_canonical_pending_choice_actions(&self) -> bool {
        false
    }

    /// Membership is deliberately separate from candidate generation.
    /// Custom steppers default to a finite explicit domain; the engine
    /// override validates structured selection payloads symbolically.
    fn is_legal_action(&self, position: &CombatPosition, input: &ClientInput) -> bool {
        self.atomic_actions(position).contains(input)
    }

    fn choice_for_legal_input(
        &self,
        position: &CombatPosition,
        input: &ClientInput,
    ) -> Option<CombatActionChoice> {
        self.is_legal_action(position, input)
            .then(|| CombatActionChoice::from_input(&position.combat, input.clone()))
    }

    fn atomic_action_choices(&self, position: &CombatPosition) -> Vec<CombatActionChoice> {
        self.atomic_actions(position)
            .into_iter()
            .map(|input| CombatActionChoice::from_input(&position.combat, input))
            .collect()
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        limits: CombatStepLimits,
    ) -> CombatStepResult;

    fn terminal(&self, position: &CombatPosition) -> CombatTerminal;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EngineCombatStepper;

impl CombatStepper for EngineCombatStepper {
    fn atomic_actions(&self, position: &CombatPosition) -> Vec<ClientInput> {
        self.legal_action_surface(position).atomic_actions
    }

    fn legal_action_surface(&self, position: &CombatPosition) -> CombatLegalActionSurfaceV2 {
        crate::sim::combat_action_surface::combat_legal_action_surface_v2(
            &position.engine,
            &position.combat,
        )
    }

    fn supports_canonical_pending_choice_actions(&self) -> bool {
        true
    }

    fn is_legal_action(&self, position: &CombatPosition, input: &ClientInput) -> bool {
        crate::sim::combat_legal_actions::is_legal_move(&position.engine, &position.combat, input)
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        limits: CombatStepLimits,
    ) -> CombatStepResult {
        apply_combat_input_to_stable(position, input, limits)
    }

    fn terminal(&self, position: &CombatPosition) -> CombatTerminal {
        combat_terminal(&position.engine, &position.combat)
    }
}

pub fn combat_terminal(engine: &EngineState, combat: &CombatState) -> CombatTerminal {
    if matches!(engine, EngineState::GameOver(RunResult::Defeat))
        || combat.entities.player.current_hp <= 0
    {
        CombatTerminal::Loss
    } else if matches!(engine, EngineState::GameOver(RunResult::Victory))
        || combat.are_monsters_basically_dead_java()
        || post_combat_engine_state(engine)
    {
        CombatTerminal::Win
    } else {
        CombatTerminal::Unresolved
    }
}

pub fn apply_combat_input_to_stable(
    position: &CombatPosition,
    input: ClientInput,
    limits: CombatStepLimits,
) -> CombatStepResult {
    apply_combat_input_to_stable_inner(position, input, limits, false).step
}

/// Profiles one transition without putting clocks or sampling branches on the
/// production `EngineCombatStepper` path.
pub fn apply_combat_input_to_stable_profiled_v1(
    position: &CombatPosition,
    input: ClientInput,
    limits: CombatStepLimits,
) -> (CombatStepResult, CombatStepPerformanceTimingV1) {
    let engine_clone_started = Instant::now();
    let engine = position.engine.clone();
    let engine_clone_elapsed_ns = elapsed_nanos_saturated(engine_clone_started);

    let (meta, combat_meta_clone_elapsed_ns) = clone_with_timing(&position.combat.meta);
    let (turn, combat_turn_clone_elapsed_ns) = clone_with_timing(&position.combat.turn);
    let (draw_pile, draw_pile_clone_elapsed_ns) =
        clone_with_timing(&position.combat.zones.draw_pile);
    let (hand, hand_clone_elapsed_ns) = clone_with_timing(&position.combat.zones.hand);
    let (discard_pile, discard_pile_clone_elapsed_ns) =
        clone_with_timing(&position.combat.zones.discard_pile);
    let (exhaust_pile, exhaust_pile_clone_elapsed_ns) =
        clone_with_timing(&position.combat.zones.exhaust_pile);
    let (limbo, limbo_clone_elapsed_ns) = clone_with_timing(&position.combat.zones.limbo);
    let (queued_cards, queued_cards_clone_elapsed_ns) =
        clone_with_timing(&position.combat.zones.queued_cards);
    let combat_zone_component_elapsed_ns = [
        draw_pile_clone_elapsed_ns,
        hand_clone_elapsed_ns,
        discard_pile_clone_elapsed_ns,
        exhaust_pile_clone_elapsed_ns,
        limbo_clone_elapsed_ns,
        queued_cards_clone_elapsed_ns,
    ];
    let combat_zones_clone_elapsed_ns = combat_zone_component_elapsed_ns
        .iter()
        .copied()
        .fold(0u64, u64::saturating_add);
    let zones = CardZones {
        draw_pile,
        hand,
        discard_pile,
        exhaust_pile,
        limbo,
        queued_cards,
        card_uuid_counter: position.combat.zones.card_uuid_counter,
    };

    let (player, player_clone_elapsed_ns) = clone_with_timing(&position.combat.entities.player);
    let (monsters, monsters_clone_elapsed_ns) =
        clone_with_timing(&position.combat.entities.monsters);
    let (potions, potions_clone_elapsed_ns) = clone_with_timing(&position.combat.entities.potions);
    let (power_db, power_db_clone_elapsed_ns) =
        clone_with_timing(&position.combat.entities.power_db);
    let combat_entity_component_elapsed_ns = [
        player_clone_elapsed_ns,
        monsters_clone_elapsed_ns,
        potions_clone_elapsed_ns,
        power_db_clone_elapsed_ns,
    ];
    let combat_entities_clone_elapsed_ns = combat_entity_component_elapsed_ns
        .iter()
        .copied()
        .fold(0u64, u64::saturating_add);
    let entities = EntityState {
        player,
        monsters,
        potions,
        power_db,
    };
    let (combat_engine, combat_engine_clone_elapsed_ns) =
        clone_with_timing(&position.combat.engine);
    let (rng, combat_rng_clone_elapsed_ns) = clone_with_timing(&position.combat.rng);
    let (colorless_combat_pool, runtime_colorless_pool_clone_elapsed_ns) =
        clone_with_timing(&position.combat.runtime.colorless_combat_pool);
    let (emitted_events, runtime_emitted_events_clone_elapsed_ns) =
        clone_with_timing(&position.combat.runtime.emitted_events);
    let (engine_diagnostics, runtime_engine_diagnostics_clone_elapsed_ns) =
        clone_with_timing(&position.combat.runtime.engine_diagnostics);
    let (pending_rewards, runtime_pending_rewards_clone_elapsed_ns) =
        clone_with_timing(&position.combat.runtime.pending_rewards);
    let (last_drawn_cards, runtime_last_drawn_cards_clone_elapsed_ns) =
        clone_with_timing(&position.combat.runtime.last_drawn_cards);
    let (monster_protocol, runtime_monster_protocol_clone_elapsed_ns) =
        clone_with_timing(&position.combat.runtime.monster_protocol);
    let combat_runtime_component_elapsed_ns = [
        runtime_colorless_pool_clone_elapsed_ns,
        runtime_emitted_events_clone_elapsed_ns,
        runtime_engine_diagnostics_clone_elapsed_ns,
        runtime_pending_rewards_clone_elapsed_ns,
        runtime_last_drawn_cards_clone_elapsed_ns,
        runtime_monster_protocol_clone_elapsed_ns,
    ];
    let combat_runtime_clone_elapsed_ns = combat_runtime_component_elapsed_ns
        .iter()
        .copied()
        .fold(0u64, u64::saturating_add);
    let runtime = CombatRuntimeHints {
        using_card: position.combat.runtime.using_card,
        colorless_combat_pool,
        emitted_events,
        engine_diagnostics,
        pending_rewards,
        power_instance_counter: position.combat.runtime.power_instance_counter,
        last_drawn_cards,
        monster_protocol,
        combat_mugged: position.combat.runtime.combat_mugged,
        combat_smoked: position.combat.runtime.combat_smoked,
    };
    let combat_clone_elapsed_ns = combat_meta_clone_elapsed_ns
        .saturating_add(combat_turn_clone_elapsed_ns)
        .saturating_add(combat_zones_clone_elapsed_ns)
        .saturating_add(combat_entities_clone_elapsed_ns)
        .saturating_add(combat_engine_clone_elapsed_ns)
        .saturating_add(combat_rng_clone_elapsed_ns)
        .saturating_add(combat_runtime_clone_elapsed_ns);
    let combat = CombatState {
        meta,
        turn,
        zones,
        entities,
        engine: combat_engine,
        rng,
        runtime,
    };

    let execution_started = Instant::now();
    let mut phase_profiler = CombatEnginePhaseTimingProfilerV1::default();
    let step = apply_combat_input_to_stable_owned_inner_with_profiler(
        engine,
        combat,
        input,
        limits,
        false,
        &mut phase_profiler,
    )
    .step;
    let execution_elapsed_ns = elapsed_nanos_saturated(execution_started);

    (
        step,
        CombatStepPerformanceTimingV1 {
            engine_clone_elapsed_ns,
            combat_clone_elapsed_ns,
            combat_meta_clone_elapsed_ns,
            combat_turn_clone_elapsed_ns,
            combat_zones_clone_elapsed_ns,
            combat_entities_clone_elapsed_ns,
            combat_zone_component_elapsed_ns,
            combat_entity_component_elapsed_ns,
            combat_runtime_component_elapsed_ns,
            combat_engine_clone_elapsed_ns,
            combat_rng_clone_elapsed_ns,
            combat_runtime_clone_elapsed_ns,
            execution_elapsed_ns,
            engine_phase_elapsed_ns: phase_profiler.elapsed_ns,
            engine_phase_occurrences: phase_profiler.occurrences,
        },
    )
}

pub fn apply_combat_input_to_stable_observed_v1(
    position: &CombatPosition,
    input: ClientInput,
    limits: CombatStepLimits,
) -> CombatObservedStepResultV1 {
    let result = apply_combat_input_to_stable_inner(position, input, limits, true);
    CombatObservedStepResultV1 {
        step: result.step,
        drawn_cards: result
            .drawn_cards
            .expect("observed combat steps must return draw evidence"),
    }
}

struct CombatStepResultInternal {
    step: CombatStepResult,
    drawn_cards: Option<Vec<DomainCardSnapshot>>,
}

fn apply_combat_input_to_stable_inner(
    position: &CombatPosition,
    input: ClientInput,
    limits: CombatStepLimits,
    observe_draws: bool,
) -> CombatStepResultInternal {
    apply_combat_input_to_stable_owned_inner(
        position.engine.clone(),
        position.combat.clone(),
        input,
        limits,
        observe_draws,
    )
}

fn apply_combat_input_to_stable_owned_inner(
    engine: EngineState,
    combat: CombatState,
    input: ClientInput,
    limits: CombatStepLimits,
    observe_draws: bool,
) -> CombatStepResultInternal {
    let mut profiler = NoopCombatEnginePhaseProfiler;
    apply_combat_input_to_stable_owned_inner_with_profiler(
        engine,
        combat,
        input,
        limits,
        observe_draws,
        &mut profiler,
    )
}

fn apply_combat_input_to_stable_owned_inner_with_profiler<P: CombatEnginePhaseProfiler>(
    mut engine: EngineState,
    mut combat: CombatState,
    input: ClientInput,
    limits: CombatStepLimits,
    observe_draws: bool,
    profiler: &mut P,
) -> CombatStepResultInternal {
    combat.clear_card_draw_observation_events();

    if limits.deadline.is_some_and(|limit| Instant::now() >= limit) {
        return step_result(engine, combat, true, true, true, 0, observe_draws);
    }

    let mut steps = 1usize;
    let mut alive = tick_engine_with_profiler(&mut engine, &mut combat, Some(input), profiler);
    if !alive {
        mark_defeat_if_needed(&mut engine, &combat);
        return step_result(engine, combat, false, false, false, steps, observe_draws);
    }
    normalize_player_turn_processing(&mut engine, &combat);

    loop {
        if stable_boundary(&engine, &combat) {
            alive = !matches!(engine, EngineState::GameOver(_));
            return step_result(engine, combat, alive, false, false, steps, observe_draws);
        }
        if steps >= limits.max_engine_steps.max(1) {
            return step_result(engine, combat, true, true, false, steps, observe_draws);
        }
        if limits.deadline.is_some_and(|limit| Instant::now() >= limit) {
            return step_result(engine, combat, true, true, true, steps, observe_draws);
        }

        alive = tick_engine_with_profiler(&mut engine, &mut combat, None, profiler);
        steps = steps.saturating_add(1);
        if !alive {
            mark_defeat_if_needed(&mut engine, &combat);
            return step_result(engine, combat, false, false, false, steps, observe_draws);
        }
        normalize_player_turn_processing(&mut engine, &combat);
    }
}

fn elapsed_nanos_saturated(started: Instant) -> u64 {
    started.elapsed().as_nanos().min(u64::MAX as u128) as u64
}

fn clone_with_timing<T: Clone>(value: &T) -> (T, u64) {
    let started = Instant::now();
    let cloned = value.clone();
    (cloned, elapsed_nanos_saturated(started))
}

pub fn stable_boundary(engine: &EngineState, combat: &CombatState) -> bool {
    match engine {
        EngineState::CombatPlayerTurn
        | EngineState::PendingChoice(_)
        | EngineState::GameOver(_) => true,
        EngineState::CombatProcessing if is_smoke_escape_stable_boundary(engine, combat) => true,
        EngineState::CombatProcessing => false,
        EngineState::CombatStart(_) => false,
        EngineState::RewardScreen(_)
        | EngineState::RewardOverlay { .. }
        | EngineState::TreasureRoom(_)
        | EngineState::Campfire
        | EngineState::Shop(_)
        | EngineState::MapNavigation
        | EngineState::MapOverlay { .. }
        | EngineState::EventRoom
        | EngineState::RunPendingChoice(_)
        | EngineState::BossRelicSelect(_) => true,
    }
}

fn step_result(
    engine: EngineState,
    mut combat: CombatState,
    alive: bool,
    truncated: bool,
    timed_out: bool,
    engine_steps: usize,
    observe_draws: bool,
) -> CombatStepResultInternal {
    let drawn_cards = if observe_draws {
        Some(combat.take_card_draw_observation_events_v1())
    } else {
        combat.clear_card_draw_observation_events();
        None
    };
    let terminal = combat_terminal(&engine, &combat);
    combat.clear_transition_observations();
    CombatStepResultInternal {
        step: CombatStepResult {
            position: CombatPosition { engine, combat },
            terminal,
            alive,
            truncated,
            timed_out,
            engine_steps,
        },
        drawn_cards,
    }
}

fn normalize_player_turn_processing(engine: &mut EngineState, combat: &CombatState) {
    if *engine == EngineState::CombatPlayerTurn
        && (combat.has_pending_actions() || !combat.zones.queued_cards.is_empty())
    {
        *engine = EngineState::CombatProcessing;
    }
}

fn mark_defeat_if_needed(engine: &mut EngineState, combat: &CombatState) {
    if combat_terminal(engine, combat) == CombatTerminal::Unresolved {
        *engine = EngineState::GameOver(RunResult::Defeat);
    }
}

fn post_combat_engine_state(engine: &EngineState) -> bool {
    matches!(
        engine,
        EngineState::RewardScreen(_)
            | EngineState::TreasureRoom(_)
            | EngineState::Campfire
            | EngineState::Shop(_)
            | EngineState::MapNavigation
            | EngineState::EventRoom
            | EngineState::RunPendingChoice(_)
            | EngineState::BossRelicSelect(_)
    )
}

#[cfg(test)]
mod tests {
    use super::{combat_terminal, stable_boundary, CombatTerminal};
    use crate::content::cards::CardId;
    use crate::content::monsters::factory::EncounterId;
    use crate::runtime::combat::CombatCard;
    use crate::sim::combat_start::build_natural_combat_start;
    use crate::state::core::{ClientInput, CombatStartRequest, EngineState, PostCombatReturn};
    use crate::state::map::node::RoomType;
    use crate::state::rewards::RewardState;
    use crate::state::run::RunState;
    use crate::state::{
        DomainCardSnapshot, DomainEvent, DomainEventSource, EngineDiagnostic,
        EngineDiagnosticClass, EngineDiagnosticSeverity,
    };
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn observed_step_returns_cards_drawn_during_that_action() {
        let mut combat = blank_test_combat();
        combat.entities.monsters = vec![test_monster(crate::content::monsters::EnemyId::JawWorm)];
        combat.zones.hand = vec![CombatCard::new(CardId::BattleTrance, 1)];
        combat.zones.draw_pile = (vec![
            CombatCard::new(CardId::Defend, 20),
            CombatCard::new(CardId::Strike, 21),
            CombatCard::new(CardId::Bash, 22),
        ])
        .into();
        combat.emit_event(DomainEvent::CardDrawn {
            card: DomainCardSnapshot {
                id: CardId::AscendersBane,
                upgrades: 0,
                uuid: 99,
            },
            source: DomainEventSource::CombatDraw,
        });
        let position = super::CombatPosition::new(EngineState::CombatPlayerTurn, combat);

        let observed = super::apply_combat_input_to_stable_observed_v1(
            &position,
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            super::CombatStepLimits {
                max_engine_steps: 20,
                deadline: None,
            },
        );

        assert_eq!(
            observed.drawn_cards,
            vec![
                DomainCardSnapshot {
                    id: CardId::Defend,
                    upgrades: 0,
                    uuid: 20,
                },
                DomainCardSnapshot {
                    id: CardId::Strike,
                    upgrades: 0,
                    uuid: 21,
                },
                DomainCardSnapshot {
                    id: CardId::Bash,
                    upgrades: 0,
                    uuid: 22,
                },
            ]
        );
    }

    #[test]
    fn speculative_step_clears_all_transition_observations() {
        let mut combat = blank_test_combat();
        combat.entities.monsters = vec![test_monster(crate::content::monsters::EnemyId::JawWorm)];
        combat.zones.hand = vec![CombatCard::new(CardId::BattleTrance, 1)];
        combat.zones.draw_pile = (vec![CombatCard::new(CardId::Defend, 20)]).into();
        combat.emit_event(DomainEvent::CardsExhausted {
            cards: Vec::new(),
            source: DomainEventSource::DeckMutation,
        });
        combat.emit_diagnostic(EngineDiagnostic {
            severity: EngineDiagnosticSeverity::Warning,
            class: EngineDiagnosticClass::Suspicious,
            message: "observation only".to_string(),
        });
        let position = super::CombatPosition::new(EngineState::CombatPlayerTurn, combat);

        let step = super::apply_combat_input_to_stable(
            &position,
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            super::CombatStepLimits {
                max_engine_steps: 20,
                deadline: None,
            },
        );

        assert!(step.position.combat.runtime.emitted_events.is_empty());
        assert!(step.position.combat.runtime.engine_diagnostics.is_empty());
    }

    #[test]
    fn observed_step_with_expired_deadline_returns_no_draws() {
        let mut combat = blank_test_combat();
        combat.emit_event(DomainEvent::CardDrawn {
            card: DomainCardSnapshot {
                id: CardId::Strike,
                upgrades: 0,
                uuid: 7,
            },
            source: DomainEventSource::CombatDraw,
        });
        let position = super::CombatPosition::new(EngineState::CombatPlayerTurn, combat);

        let observed = super::apply_combat_input_to_stable_observed_v1(
            &position,
            ClientInput::EndTurn,
            super::CombatStepLimits {
                max_engine_steps: 20,
                deadline: Some(std::time::Instant::now() - std::time::Duration::from_secs(1)),
            },
        );

        assert!(observed.drawn_cards.is_empty());
        assert!(observed.step.timed_out);
        assert_eq!(observed.step.engine_steps, 0);
    }

    #[test]
    fn profiled_step_is_semantically_identical_to_the_production_step() {
        let mut combat = blank_test_combat();
        combat.entities.monsters = vec![test_monster(crate::content::monsters::EnemyId::JawWorm)];
        combat.zones.hand = vec![CombatCard::new(CardId::BattleTrance, 1)];
        combat.zones.draw_pile = (vec![
            CombatCard::new(CardId::Defend, 20),
            CombatCard::new(CardId::Strike, 21),
            CombatCard::new(CardId::Bash, 22),
        ])
        .into();
        let position = super::CombatPosition::new(EngineState::CombatPlayerTurn, combat);
        let limits = super::CombatStepLimits {
            max_engine_steps: 20,
            deadline: None,
        };
        let input = ClientInput::PlayCard {
            card_index: 0,
            target: None,
        };

        let expected = super::apply_combat_input_to_stable(&position, input.clone(), limits);
        let (actual, timing) =
            super::apply_combat_input_to_stable_profiled_v1(&position, input, limits);

        assert_eq!(actual.position, expected.position);
        assert_eq!(actual.terminal, expected.terminal);
        assert_eq!(actual.alive, expected.alive);
        assert_eq!(actual.truncated, expected.truncated);
        assert_eq!(actual.timed_out, expected.timed_out);
        assert_eq!(actual.engine_steps, expected.engine_steps);
        assert!(timing.combat_clone_elapsed_ns > 0);
        assert!(timing.execution_elapsed_ns > 0);
    }

    #[test]
    fn combat_start_request_is_not_a_stable_search_boundary() {
        let mut run = RunState::new(1, 0, false, "Ironclad");
        let (_engine, combat) =
            build_natural_combat_start(&mut run, EncounterId::JawWorm, RoomType::MonsterRoom)
                .expect("combat should initialize");
        let event_engine = EngineState::CombatStart(CombatStartRequest::event(
            EncounterId::JawWorm,
            RewardState::new(),
            true,
            false,
            false,
            PostCombatReturn::MapNavigation,
        ));

        assert!(!stable_boundary(&event_engine, &combat));
        assert_eq!(
            combat_terminal(&event_engine, &combat),
            CombatTerminal::Unresolved
        );
    }

    #[test]
    fn combat_terminal_requires_java_death_flags_not_only_zero_hp() {
        let mut run = RunState::new(1, 0, false, "Ironclad");
        let (_engine, mut combat) =
            build_natural_combat_start(&mut run, EncounterId::JawWorm, RoomType::MonsterRoom)
                .expect("combat should initialize");
        let monster = combat
            .entities
            .monsters
            .first_mut()
            .expect("jaw worm should exist");
        monster.current_hp = 0;
        monster.is_dying = false;
        monster.is_escaped = false;
        monster.half_dead = false;

        assert_eq!(
            combat_terminal(&EngineState::CombatPlayerTurn, &combat),
            CombatTerminal::Unresolved,
            "Java victory settlement waits for isDying/isEscaping flags; zero HP alone is not a terminal combat state"
        );
    }
}
