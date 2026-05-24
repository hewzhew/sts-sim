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

mod action_effects;
mod action_equivalence;
mod action_ordering;
mod baseline;
mod card_identity;
mod card_pile_value;
mod diagnostics;
mod enemy_phase_value;
mod expansion;
mod frontier;
mod outcome_score;
mod potions;
mod report;
mod rollout;
mod search;
mod target_fanout;
mod transition;
mod turn_branching;
mod turn_local_dominance;
mod turn_prefix;
mod turn_sequence;
mod types;
mod value;

use action_equivalence::{
    compress_equivalent_actions, ActionEquivalenceDiagnosticsCollector, ActionEquivalenceSummary,
};
use action_ordering::{
    order_indexed_action_choices, ActionOrderingDiagnosticsCollector, ActionOrderingSummary,
    IndexedActionChoice,
};
use card_identity::{
    summarize_card_identity, CardIdentityDiagnosticsCollector, CardIdentitySummary,
};
use diagnostics::{SearchDiagnosticsCollector, SearchDiagnosticsFinish, FRONTIER_SAMPLE_LIMIT};
use expansion::{
    summarize_action_expansion, ActionExpansionDiagnosticsCollector, ActionExpansionSummary,
};
use frontier::{
    is_resource_covered, push_frontier, remember_best_complete, remember_best_frontier,
    ResourceVector, SearchNode,
};
use outcome_score::CombatOutcomeScore;
use report::{summarize_state, trajectory_report};
use rollout::{RolloutCache, RolloutNodeEstimate};
use target_fanout::{
    summarize_target_fanout, TargetFanoutDiagnosticsCollector, TargetFanoutSummary,
};
use transition::{filtered_legal_actions, is_use_potion_input, terminal_label};
use turn_branching::{
    classify_turn_branch_transition, TurnBranchTransition, TurnBranchingDiagnosticsCollector,
    TurnBranchingStateObservation,
};
use turn_local_dominance::{
    TurnLocalDominanceDiagnosticsCollector, TurnLocalDominanceStateObservation,
};
use turn_prefix::{
    advance_turn_prefix, summarize_turn_prefix, TurnPrefixDiagnosticsCollector, TurnPrefixState,
    TurnPrefixSummary,
};
use turn_sequence::{
    summarize_turn_sequence, TurnSequenceDiagnosticsCollector, TurnSequenceSummary,
};
use value::{
    combat_search_frontier_value_report, living_enemy_count, survival_margin, terminal_rank,
    total_living_enemy_hp, visible_incoming_damage, COMBAT_SEARCH_FRONTIER_VALUE_POLICY,
};

pub use baseline::{
    compare_outcome_metrics, compare_trajectory_reports, CombatSearchV2OutcomeMetrics,
    WHOLE_COMBAT_OUTCOME_CRITERIA,
};
pub use report::trajectory_from_state;
pub use search::{run_combat_search_v2, run_combat_search_v2_with_stepper};
pub use types::*;

pub(crate) fn filter_combat_search_legal_actions(
    choices: Vec<CombatActionChoice>,
    potion_policy: CombatSearchV2PotionPolicy,
    combat: &CombatState,
) -> Vec<CombatActionChoice> {
    transition::filtered_legal_actions(choices, potion_policy, combat)
}
