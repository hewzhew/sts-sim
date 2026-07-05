use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Instant;

use crate::ai::combat_state_key::{
    combat_dominance_key, combat_exact_state_hash_v1, combat_exact_state_key, CombatDominanceKey,
    CombatExactStateKey,
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

// Core search loop and frontier ownership.
mod frontier;
mod outcome_score;
mod search;
mod transition;

// Action semantics: legal action facts, ordering, equivalence, and expansion shape.
mod action_effects;
mod action_equivalence;
mod action_facts;
mod action_ordering;
mod action_priority;
mod expansion;
mod target_fanout;

// Evaluation, value, and outcome comparison.
mod baseline;
mod card_pile_value;
mod enemy_phase_value;
mod pressure_value;
mod rollout_value;
mod value;
mod value_facts;

// Rollout policies and bounded rollout execution.
mod rollout;
mod rollout_cache;
mod rollout_estimate;
mod rollout_pending_choice;
mod rollout_policy;
mod rollout_profile;
mod rollout_scheduler;

// Turn-level planning and current-turn structure.
mod turn_branching;
mod turn_local_dominance;
pub(crate) mod turn_planner;
mod turn_prefix;
mod turn_sequence;
mod turn_sequence_effect;

// Combat phase and enemy mechanics facts.
mod enemy_mechanics_profile;
mod enemy_phase_transition;
mod external_payoff;
mod phase_action_ordering;
mod phase_profile;

// Pending choice and potion boundaries.
mod pending_choice_fanout;
mod pending_choice_ordering;
mod pending_choice_profile;
mod potions;

// State abstraction and exactness audits.
mod card_identity;
mod discard_order_shadow_audit;
pub mod state_abstraction;

// Reports, diagnostics, and opt-in analysis tools.
mod decision_microscope;
mod deficit_evidence;
mod diagnostics;
mod diagnostics_tags;
mod line_lab;
mod rollout_probe;
mod segment_plan;
mod trajectory_report;
mod turn_plan_probe;
mod witness_guidance;

#[cfg(test)]
mod semantic_regression;
mod types;

use action_equivalence::{
    compress_equivalent_actions, ActionEquivalenceDiagnosticsCollector, ActionEquivalenceSummary,
};
use action_facts::summarize_action_facts_from_step;
use action_ordering::{
    order_indexed_action_choices, order_indexed_action_choices_with_prior,
    ActionOrderingDiagnosticsCollector, ActionOrderingSummary, IndexedActionChoice,
};
use card_identity::{
    summarize_card_identity, CardIdentityDiagnosticsCollector, CardIdentitySummary,
};
use diagnostics::{SearchDiagnosticsCollector, SearchDiagnosticsFinish, FRONTIER_SAMPLE_LIMIT};
use diagnostics_tags::diagnosis_tags;
use enemy_phase_transition::enemy_phase_transition_hint_for_input;
use expansion::{
    summarize_action_expansion, ActionExpansionDiagnosticsCollector, ActionExpansionSummary,
};
use frontier::{
    is_resource_covered, push_frontier, remember_best_complete, remember_best_frontier,
    FrontierQueue, ResourceVector, SearchNode,
};
use outcome_score::CombatOutcomeScore;
use pending_choice_ordering::pending_choice_ordering_hint;
use pending_choice_profile::{
    summarize_pending_choice, PendingChoiceDiagnosticsCollector, PendingChoiceProfile,
};
use phase_profile::{combat_search_phase_profile, combat_search_phase_profile_report};
use pressure_value::visible_incoming_damage;
use rollout_cache::RolloutCache;
use rollout_estimate::{RolloutNodeEstimate, RolloutStopReason};
use rollout_policy::{choose_rollout_action, filtered_rollout_legal_actions};
use target_fanout::{
    summarize_target_fanout, TargetFanoutDiagnosticsCollector, TargetFanoutSummary,
};
use trajectory_report::{summarize_state, trajectory_report};
use transition::{filtered_legal_actions, is_use_potion_input, terminal_label};
use turn_branching::{
    classify_turn_branch_transition, TurnBranchTransition, TurnBranchingDiagnosticsCollector,
    TurnBranchingStateObservation,
};
use turn_local_dominance::{
    TurnLocalDominanceDiagnosticsCollector, TurnLocalDominanceStateObservation,
};
use turn_planner::{turn_plan_frontier_seed, TurnPlanDiagnosticsCollector};
use turn_prefix::{
    advance_turn_prefix, summarize_turn_prefix, TurnPrefixDiagnosticsCollector, TurnPrefixState,
    TurnPrefixSummary,
};
use turn_sequence::{
    summarize_turn_sequence, TurnSequenceDiagnosticsCollector, TurnSequenceSummary,
};
use value::{combat_search_frontier_value_report, COMBAT_SEARCH_FRONTIER_VALUE_POLICY};
use value_facts::{living_enemy_count, terminal_rank, total_living_enemy_hp};

pub use action_facts::{
    CombatSearchV2ActionAccessMechanicsFacts, CombatSearchV2ActionCardFacts,
    CombatSearchV2ActionDerivedMechanicsFacts, CombatSearchV2ActionDirectMechanicsFacts,
    CombatSearchV2ActionExactDeltaFacts, CombatSearchV2ActionFacts,
    CombatSearchV2ActionImmediateFacts, CombatSearchV2ActionMechanicsFacts,
    CombatSearchV2ActionReactiveMechanicsFacts, CombatSearchV2ActionTargetFacts,
};
pub use baseline::{
    compare_outcome_metrics, CombatSearchV2OutcomeMetrics, WHOLE_COMBAT_OUTCOME_CRITERIA,
};
pub use decision_microscope::{
    explain_combat_search_v2_initial_decision, CombatSearchV2ActionFactsReport,
    CombatSearchV2DecisionCandidateReport, CombatSearchV2DecisionContext,
    CombatSearchV2DecisionMicroscopeConfigReport, CombatSearchV2DecisionMicroscopeReport,
    CombatSearchV2DecisionOneStepReport, CombatSearchV2DecisionSelectedAction,
    CombatSearchV2DecisionTrajectorySummary,
};
pub use deficit_evidence::{
    derive_combat_deficit_evidence, CombatDeficitEvidenceFlag, CombatDeficitEvidenceObservations,
    CombatDeficitEvidenceReport,
};
pub(crate) use external_payoff::has_external_payoff_opportunity;
pub use line_lab::{
    find_combat_line_lab_turn_pool_win_v0, run_combat_line_lab_from_parent_v0,
    run_combat_line_lab_v0, CombatLineLabReport,
};
pub use search::{run_combat_search_v2, run_combat_search_v2_with_stepper};
pub use segment_plan::{plan_combat_turn_segment_v1, CombatSearchV2TurnSegmentReport};
pub use trajectory_report::trajectory_from_state;
#[cfg(test)]
pub(crate) use turn_plan_probe::CombatSearchV2TurnPlanProbeStepReport;
pub(crate) use turn_plan_probe::{
    enumerate_combat_search_v2_turn_plan_probe_candidates,
    CombatSearchV2TurnPlanProbeCandidateReport, CombatSearchV2TurnPlanProbeRootReport,
};
pub use types::*;
pub use witness_guidance::{
    compile_combat_search_witness_prior_v0, replay_combat_search_witness_line_v0,
    CombatSearchV2WitnessLine, CombatSearchV2WitnessPrior, CombatSearchV2WitnessReplay,
    CombatSearchV2WitnessReplayStep,
};

pub fn combat_search_exact_state_hash_v1(engine: &EngineState, combat: &CombatState) -> String {
    combat_exact_state_hash_v1(engine, combat)
}

pub(crate) fn combat_search_action_ordering_role_label_for_state(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
) -> &'static str {
    action_priority::priority_for_input(
        engine,
        combat,
        input,
        CombatSearchV2PhaseGuardPolicy::Default,
    )
    .role
    .label()
}

pub(crate) fn combat_search_phase_profile_report_for_state(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatSearchV2PhaseProfileReport {
    combat_search_phase_profile_report(combat_search_phase_profile(engine, combat))
}

pub(crate) fn filter_combat_search_legal_actions(
    choices: Vec<CombatActionChoice>,
    potion_policy: CombatSearchV2PotionPolicy,
    combat: &CombatState,
) -> Vec<CombatActionChoice> {
    transition::filtered_legal_actions(choices, potion_policy, combat)
}
