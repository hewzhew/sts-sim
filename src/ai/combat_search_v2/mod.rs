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
mod action_priority;
mod baseline;
mod card_identity;
mod card_pile_value;
mod diagnostics;
mod diagnostics_tags;
mod enemy_mechanics_profile;
mod enemy_phase_transition;
mod enemy_phase_value;
mod expansion;
mod frontier;
mod outcome_score;
mod pending_choice_ordering;
mod pending_choice_profile;
mod phase_action_ordering;
mod phase_profile;
mod potions;
mod pressure_value;
mod report;
mod rollout;
mod rollout_policy;
mod rollout_value;
mod search;
#[cfg(test)]
mod semantic_regression;
pub mod state_abstraction;
mod target_fanout;
mod transition;
mod turn_branching;
mod turn_local_dominance;
mod turn_prefix;
mod turn_sequence;
mod turn_sequence_effect;
mod types;
mod value;
mod value_facts;

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
use diagnostics_tags::diagnosis_tags;
use enemy_mechanics_profile::enemy_mechanics_profile_report;
use enemy_phase_transition::enemy_phase_transition_hint_for_input;
use expansion::{
    summarize_action_expansion, ActionExpansionDiagnosticsCollector, ActionExpansionSummary,
};
use frontier::{
    is_resource_covered, push_frontier, remember_best_complete, remember_best_frontier,
    ResourceVector, SearchNode,
};
use outcome_score::CombatOutcomeScore;
use pending_choice_ordering::{pending_choice_ordering_hint, PendingChoiceOrderingRole};
use pending_choice_profile::{
    summarize_pending_choice, PendingChoiceDiagnosticsCollector, PendingChoiceProfile,
};
use phase_profile::{combat_search_phase_profile, combat_search_phase_profile_report};
use pressure_value::visible_incoming_damage;
use report::{summarize_state, trajectory_report};
use rollout::{RolloutCache, RolloutNodeEstimate};
use rollout_policy::{choose_rollout_action, filtered_rollout_legal_actions};
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
use value::{combat_search_frontier_value_report, COMBAT_SEARCH_FRONTIER_VALUE_POLICY};
use value_facts::{living_enemy_count, terminal_rank, total_living_enemy_hp};

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
