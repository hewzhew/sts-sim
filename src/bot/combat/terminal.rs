use crate::runtime::combat::CombatState;
use crate::state::core::RunResult;
use crate::state::EngineState;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum TerminalKind {
    Defeat,
    Ongoing,
    CombatCleared,
    Victory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TerminalOutcome {
    pub(super) kind: TerminalKind,
    pub(super) final_hp: i32,
    pub(super) final_block: i32,
}

pub(super) fn terminal_kind(engine: &EngineState, combat: &CombatState) -> TerminalKind {
    if matches!(engine, EngineState::GameOver(RunResult::Defeat))
        || combat.entities.player.current_hp <= 0
    {
        TerminalKind::Defeat
    } else if matches!(engine, EngineState::GameOver(RunResult::Victory)) {
        TerminalKind::Victory
    } else if combat_cleared(combat) {
        TerminalKind::CombatCleared
    } else {
        TerminalKind::Ongoing
    }
}

pub(super) fn terminal_outcome(
    engine: &EngineState,
    combat: &CombatState,
) -> Option<TerminalOutcome> {
    let kind = terminal_kind(engine, combat);
    (kind != TerminalKind::Ongoing).then_some(TerminalOutcome {
        kind,
        final_hp: combat.entities.player.current_hp,
        final_block: combat.entities.player.block,
    })
}

pub(super) fn combat_cleared(combat: &CombatState) -> bool {
    combat
        .entities
        .monsters
        .iter()
        .all(|monster| monster.is_dying || monster.is_escaped || monster.current_hp <= 0)
}

pub(super) fn survives(kind: TerminalKind, projected_hp: i32) -> bool {
    kind != TerminalKind::Defeat && projected_hp > 0
}
