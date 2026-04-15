use crate::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;
use std::time::Instant;

use super::profile::SearchProfileCollector;

pub(super) fn advance_to_decision_point(
    engine: &mut EngineState,
    combat: &mut CombatState,
    initial_input: ClientInput,
    max_engine_steps: usize,
) {
    let _ =
        advance_to_decision_point_profiled(engine, combat, initial_input, max_engine_steps, None);
}

pub(super) fn advance_to_decision_point_profiled(
    engine: &mut EngineState,
    combat: &mut CombatState,
    initial_input: ClientInput,
    max_engine_steps: usize,
    profiler: Option<&mut SearchProfileCollector>,
) -> usize {
    let started = Instant::now();
    let mut first = true;
    let mut safety_counter = 0;
    loop {
        safety_counter += 1;
        if safety_counter > max_engine_steps.max(1) {
            break;
        }

        let input_val = if first {
            Some(initial_input.clone())
        } else {
            None
        };
        first = false;
        let alive = crate::engine::core::with_suppressed_engine_warnings(|| {
            crate::engine::core::tick_engine(engine, combat, input_val)
        });
        if !alive
            || matches!(
                engine,
                EngineState::CombatPlayerTurn
                    | EngineState::PendingChoice(_)
                    | EngineState::GameOver(_)
            )
        {
            break;
        }
    }
    if let Some(profiler) = profiler {
        profiler.record_advance(started.elapsed().as_millis(), safety_counter);
    }
    safety_counter
}

pub(super) fn project_turn_close_state(
    engine: &EngineState,
    combat: &CombatState,
    max_engine_steps: usize,
) -> (EngineState, CombatState) {
    project_turn_close_state_profiled(engine, combat, max_engine_steps, None)
}

pub(super) fn project_turn_close_state_profiled(
    engine: &EngineState,
    combat: &CombatState,
    max_engine_steps: usize,
    profiler: Option<&mut SearchProfileCollector>,
) -> (EngineState, CombatState) {
    let mut projected_engine = engine.clone();
    let mut projected_combat = combat.clone();

    if matches!(projected_engine, EngineState::CombatPlayerTurn) {
        advance_to_decision_point_profiled(
            &mut projected_engine,
            &mut projected_combat,
            ClientInput::EndTurn,
            max_engine_steps,
            profiler,
        );
    }

    (projected_engine, projected_combat)
}

pub(super) fn total_enemy_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .map(|monster| monster.current_hp + monster.block)
        .sum()
}

pub(super) fn is_terminal(engine: &EngineState, combat: &CombatState) -> bool {
    matches!(engine, EngineState::GameOver(_))
        || combat.entities.player.current_hp <= 0
        || combat
            .entities
            .monsters
            .iter()
            .all(|monster| monster.is_dying || monster.is_escaped || monster.current_hp <= 0)
}
