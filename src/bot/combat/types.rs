use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;

use super::terminal::TerminalKind;

#[derive(Clone)]
pub(super) struct CombatCandidate {
    pub(super) input: ClientInput,
    pub(super) next_combat: CombatState,
    pub(super) terminal_kind: TerminalKind,
    pub(super) projected_hp: i32,
    pub(super) projected_block: i32,
    pub(super) projected_enemy_total: i32,
    pub(super) projected_unblocked: i32,
    pub(super) survives: bool,
    pub(super) display_score: f32,
}

