use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;

use super::pressure::StatePressureFeatures;
use super::terminal::{survives, terminal_outcome, TerminalKind, TerminalOutcome};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NonTerminalValue {
    pub(super) survives: bool,
    pub(super) projected_unblocked: i32,
    pub(super) projected_enemy_total: i32,
    pub(super) projected_hp: i32,
    pub(super) projected_block: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CombatValue {
    Terminal(TerminalOutcome),
    NonTerminal(NonTerminalValue),
}

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
    (StatePressureFeatures::from_combat(combat).value_incoming - combat.entities.player.block)
        .max(0)
}

pub(super) fn incoming_damage(combat: &CombatState) -> i32 {
    StatePressureFeatures::from_combat(combat).value_incoming
}

pub(super) fn non_terminal_value(
    survives: bool,
    projected_unblocked: i32,
    projected_enemy_total: i32,
    projected_hp: i32,
    projected_block: i32,
) -> NonTerminalValue {
    NonTerminalValue {
        survives,
        projected_unblocked,
        projected_enemy_total,
        projected_hp,
        projected_block,
    }
}

pub(super) fn projected_frontier(
    engine: &EngineState,
    combat: &CombatState,
    max_engine_steps: usize,
) -> (EngineState, CombatState, CombatValue) {
    let (projected_engine, projected_combat) =
        project_turn_close_state(engine, combat, max_engine_steps);
    let value = if let Some(outcome) = terminal_outcome(&projected_engine, &projected_combat) {
        CombatValue::Terminal(outcome)
    } else {
        CombatValue::NonTerminal(non_terminal_value(
            survives(
                super::terminal::terminal_kind(&projected_engine, &projected_combat),
                projected_combat.entities.player.current_hp,
            ),
            projected_unblocked(&projected_combat),
            total_enemy_hp(&projected_combat),
            projected_combat.entities.player.current_hp,
            projected_combat.entities.player.block,
        ))
    };
    (projected_engine, projected_combat, value)
}

pub(super) fn compare_values(left: &CombatValue, right: &CombatValue) -> std::cmp::Ordering {
    value_bucket(right)
        .cmp(&value_bucket(left))
        .then_with(|| match (left, right) {
            (CombatValue::Terminal(left), CombatValue::Terminal(right)) => right
                .kind
                .cmp(&left.kind)
                .then_with(|| right.final_hp.cmp(&left.final_hp))
                .then_with(|| right.final_block.cmp(&left.final_block)),
            (CombatValue::NonTerminal(left), CombatValue::NonTerminal(right)) => right
                .survives
                .cmp(&left.survives)
                .then_with(|| left.projected_unblocked.cmp(&right.projected_unblocked))
                .then_with(|| left.projected_enemy_total.cmp(&right.projected_enemy_total))
                .then_with(|| right.projected_hp.cmp(&left.projected_hp))
                .then_with(|| right.projected_block.cmp(&left.projected_block)),
            _ => std::cmp::Ordering::Equal,
        })
}

pub(super) fn diagnostic_score(value: CombatValue, input: &ClientInput) -> f32 {
    match value {
        CombatValue::Terminal(outcome) => {
            let mut score = outcome.final_hp as f32 * 0.1 + outcome.final_block as f32 * 0.05;
            score += match outcome.kind {
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
        CombatValue::NonTerminal(value) => {
            let mut score = value.projected_hp as f32 * 0.1 + value.projected_block as f32 * 0.05;
            score -= value.projected_unblocked as f32 * 2.0;
            score -= value.projected_enemy_total as f32 * 0.02;
            if value.survives {
                score += 10.0;
            }
            if !matches!(input, ClientInput::EndTurn) {
                score += 0.1;
            }
            score
        }
    }
}

fn value_bucket(value: &CombatValue) -> i32 {
    match value {
        CombatValue::Terminal(TerminalOutcome {
            kind: TerminalKind::Victory,
            ..
        }) => 3,
        CombatValue::Terminal(TerminalOutcome {
            kind: TerminalKind::CombatCleared,
            ..
        }) => 2,
        CombatValue::NonTerminal(_) => 1,
        CombatValue::Terminal(TerminalOutcome {
            kind: TerminalKind::Defeat,
            ..
        }) => 0,
        CombatValue::Terminal(TerminalOutcome {
            kind: TerminalKind::Ongoing,
            ..
        }) => 1,
    }
}
