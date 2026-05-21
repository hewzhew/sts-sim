use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::time::Instant;

use crate::content::cards;
use crate::content::monsters::EnemyId;
use crate::runtime::combat::{CombatCard, CombatState, MonsterEntity, Power};
use crate::sim::combat::{
    combat_terminal, CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal,
    EngineCombatStepper,
};
use crate::sim::combat_projection::monster_preview_total_damage_in_combat;
use crate::state::core::{ClientInput, EngineState, PendingChoice};

mod baseline;
mod frontier;
mod report;
mod search;
mod state_key;
mod transition;
mod types;
mod value;

use frontier::{
    is_dominated, push_frontier, remember_best_complete, remember_best_frontier, ResourceVector,
    SearchNode,
};
use report::{action_key, summarize_state, target_label, trajectory_report};
use state_key::dominance_bucket_key;
use transition::{filtered_legal_moves, terminal_label};
use value::{
    living_enemy_count, survival_margin, terminal_rank, total_living_enemy_hp,
    visible_incoming_damage,
};

pub use baseline::compare_trajectory_reports;
pub use report::trajectory_from_state;
pub use search::{run_combat_search_v2, run_combat_search_v2_with_stepper};
pub use types::*;
