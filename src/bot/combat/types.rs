use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;

use super::value::CombatValue;

#[derive(Clone)]
pub(super) struct CombatCandidate {
    pub(super) input: ClientInput,
    pub(super) next_combat: CombatState,
    pub(super) frontier_engine: EngineState,
    pub(super) frontier_combat: CombatState,
    pub(super) local_plan: Vec<ClientInput>,
    pub(super) planner_nodes: u32,
    pub(super) value: CombatValue,
    pub(super) projection_truncated: bool,
    pub(super) cluster_size: usize,
    pub(super) collapsed_inputs: Vec<ClientInput>,
    pub(super) projected_hp: i32,
    pub(super) projected_block: i32,
    pub(super) projected_enemy_total: i32,
    pub(super) projected_unblocked: i32,
    pub(super) survives: bool,
    pub(super) diagnostic_score: f32,
}
