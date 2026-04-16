use crate::bot::search::StatePressureFeatures;
use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;

use super::terminal::TerminalKind;

pub(super) fn project_turn_close_state(
    engine: &EngineState,
    combat: &CombatState,
    _max_engine_steps: usize,
) -> (EngineState, CombatState) {
    let mut projected_engine = engine.clone();
    let mut projected_combat = combat.clone();

    if matches!(projected_engine, EngineState::CombatPlayerTurn) {
        let _ = crate::engine::core::tick_until_stable_turn(
            &mut projected_engine,
            &mut projected_combat,
            ClientInput::EndTurn,
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

pub(super) fn projected_unblocked(combat: &CombatState) -> i32 {
    (StatePressureFeatures::from_combat(combat).value_incoming - combat.entities.player.block).max(0)
}

pub(super) fn incoming_damage(combat: &CombatState) -> i32 {
    StatePressureFeatures::from_combat(combat).value_incoming
}

pub(super) fn display_score(
    terminal_kind: TerminalKind,
    projected_unblocked: i32,
    projected_enemy_total: i32,
    projected_hp: i32,
    projected_block: i32,
    input: &ClientInput,
) -> f32 {
    let mut score = projected_hp as f32 * 0.1 + projected_block as f32 * 0.05;
    score -= projected_unblocked as f32 * 2.0;
    score -= projected_enemy_total as f32 * 0.02;
    score += match terminal_kind {
        TerminalKind::Defeat => -20.0,
        TerminalKind::Ongoing => 0.0,
        TerminalKind::CombatCleared => 20.0,
        TerminalKind::Victory => 25.0,
    };
    if !matches!(input, ClientInput::EndTurn) {
        score += 0.1;
    }
    score
}
