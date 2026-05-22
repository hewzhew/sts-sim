use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::time::Instant;

use crate::ai::combat_state_key::{
    combat_dominance_key, combat_exact_state_key, CombatDominanceKey, CombatExactStateKey,
};
use crate::content::monsters::EnemyId;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::sim::combat::{
    combat_terminal, CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal,
    EngineCombatStepper,
};
use crate::sim::combat_action::CombatActionChoice;
use crate::sim::combat_projection::monster_preview_total_damage_in_combat;
use crate::state::core::{ClientInput, EngineState};

mod baseline;
mod frontier;
mod report;
mod search;
mod transition;
mod types;
mod value;

use frontier::{
    is_resource_covered, push_frontier, remember_best_complete, remember_best_frontier,
    ResourceVector, SearchNode,
};
use report::{summarize_state, trajectory_report};
use transition::{filtered_legal_actions, terminal_label};
use value::{
    living_enemy_count, survival_margin, terminal_rank, total_living_enemy_hp,
    visible_incoming_damage,
};

pub use baseline::compare_trajectory_reports;
pub use report::trajectory_from_state;
pub use search::{run_combat_search_v2, run_combat_search_v2_with_stepper};
pub use types::*;
