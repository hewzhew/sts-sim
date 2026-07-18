use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Instant;

use crate::ai::combat_state_key::{
    combat_dominance_diagnostic_parts_v1, combat_dominance_key, combat_exact_state_hash_v1,
    combat_exact_state_key, CombatDominanceKey, CombatExactStateKey,
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
mod action_resource_timing;
mod attack_retaliation;
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
mod rollout_action_selector;
mod rollout_cache;
mod rollout_estimate;
mod rollout_pending_choice;
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
mod timed_enemy_threat;

// Pending choice and potion boundaries.
pub(crate) mod pending_choice_action_prefix;
mod pending_choice_fanout;
mod pending_choice_ordering;
mod pending_choice_profile;
mod plugins;
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
mod turn_plan_probe_report;
mod turn_plan_rescue;
mod turn_pool_rescue;
mod witness_guidance;

#[cfg(test)]
mod semantic_regression;
mod types;

use action_equivalence::{
    compress_equivalent_actions, ActionEquivalenceDiagnosticsCollector, ActionEquivalenceSummary,
};
use action_facts::summarize_action_facts_from_step;
use action_ordering::{
    order_indexed_action_choices, order_indexed_action_choices_with_plugins,
    ActionOrderingDiagnosticsCollector, ActionOrderingSummary, IndexedActionChoice,
};
use card_identity::{
    summarize_card_identity, CardIdentityDiagnosticsCollector, CardIdentitySummary,
};
use diagnostics::{SearchDiagnosticsCollector, SearchDiagnosticsFinish, FRONTIER_SAMPLE_LIMIT};
use diagnostics_tags::diagnosis_tags;
use expansion::{
    summarize_action_expansion, ActionExpansionDiagnosticsCollector, ActionExpansionSummary,
};
use frontier::{is_resource_covered, ResourceVector, SearchNode};
use outcome_score::CombatOutcomeScore;
use pending_choice_action_prefix::{
    PendingChoiceActionFamily, PendingChoiceActionPrefix, PendingChoiceActionWork,
};
use pending_choice_ordering::pending_choice_ordering_hint;
use pending_choice_profile::{
    summarize_pending_choice, PendingChoiceDiagnosticsCollector, PendingChoiceProfile,
};
use phase_profile::{combat_search_phase_profile, combat_search_phase_profile_report};
use pressure_value::visible_incoming_damage;
use rollout_action_selector::{choose_rollout_action, filtered_rollout_legal_actions};
use rollout_cache::RolloutCache;

use rollout_estimate::{RolloutNodeEstimate, RolloutStopReason};
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
use turn_planner::{
    build_turn_boundary_portfolio, turn_plan_frontier_seed, TurnPlanDiagnosticsCollector,
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

pub use action_facts::{
    CombatSearchV2ActionAccessMechanicsFacts, CombatSearchV2ActionCardFacts,
    CombatSearchV2ActionDerivedMechanicsFacts, CombatSearchV2ActionDirectMechanicsFacts,
    CombatSearchV2ActionExactDeltaFacts, CombatSearchV2ActionFacts,
    CombatSearchV2ActionImmediateFacts, CombatSearchV2ActionMechanicsFacts,
    CombatSearchV2ActionReactiveMechanicsFacts, CombatSearchV2ActionResourceTimingFacts,
    CombatSearchV2ActionTargetFacts, CombatSearchV2AttackRetaliationTargetFacts,
    CombatSearchV2TimedEnemyThreatTargetFacts,
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
    run_combat_line_lab_from_parent_v0, run_combat_line_lab_v0, CombatLineLabReport,
};
pub use plugins::{
    CombatSearchAcceptancePlugin, CombatSearchAcceptancePluginId,
    CombatSearchActionOrderingPlugins, CombatSearchActionPriorPlugin,
    CombatSearchActionPriorPluginId, CombatSearchArtifactPlugin, CombatSearchArtifactPluginId,
    CombatSearchAttemptPolicy, CombatSearchBudgetSpec, CombatSearchChildRolloutPlugin,
    CombatSearchChildRolloutPluginId, CombatSearchEngineProfile, CombatSearchExpansionPlugin,
    CombatSearchExpansionPluginId, CombatSearchNodeEvaluatorPlugin,
    CombatSearchNodeEvaluatorPluginId, CombatSearchPhaseGuardPlugin,
    CombatSearchPhaseGuardPluginId, CombatSearchPluginStack, CombatSearchPotionPlugin,
    CombatSearchProfile, CombatSearchRolloutPlugin, CombatSearchRolloutPluginId,
    CombatSearchTurnPlanPlugin, CombatSearchTurnPlanPluginId,
};
pub use search::{
    run_combat_search_v2, run_combat_search_v2_with_stepper, CombatSearchV2DecisionSnapshot,
    CombatSearchV2Session, CombatSearchV2WorkQuantum,
};
pub use segment_plan::{plan_combat_turn_segment_v1, CombatSearchV2TurnSegmentReport};
pub use trajectory_report::trajectory_from_state;
pub(crate) use turn_plan_probe::enumerate_combat_search_v2_turn_plan_probe_candidates;
#[cfg(test)]
pub(crate) use turn_plan_probe_report::CombatSearchV2TurnPlanProbeStepReport;
pub(crate) use turn_plan_probe_report::{
    CombatSearchV2TurnPlanProbeCandidateReport, CombatSearchV2TurnPlanProbeRootReport,
};
pub use turn_plan_rescue::{find_combat_turn_plan_rescue_win_v0, CombatTurnPlanRescueWin};
pub use turn_pool_rescue::{
    find_combat_turn_pool_rescue_win_v0, run_combat_turn_pool_opening_report_v0,
    CombatTurnPoolOpeningLineReport, CombatTurnPoolOpeningReport, CombatTurnPoolRescueWin,
};
pub use types::*;
pub use witness_guidance::{
    compile_combat_search_witness_prior_v0, replay_combat_search_witness_line_v0,
    replay_combat_search_witness_line_v1, CombatSearchV2WitnessLine, CombatSearchV2WitnessPrior,
    CombatSearchV2WitnessReplay, CombatSearchV2WitnessReplayStep,
    CombatSearchV2WitnessReplayStepV1, CombatSearchV2WitnessReplayV1,
};

pub fn combat_search_exact_state_hash_v1(engine: &EngineState, combat: &CombatState) -> String {
    combat_exact_state_hash_v1(engine, combat)
}

pub(crate) fn combat_search_action_ordering_role_label_for_state(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
) -> &'static str {
    combat_search_action_ordering_role_label_for_state_with_plugins(
        engine,
        combat,
        input,
        CombatSearchActionOrderingPlugins::default(),
    )
}

pub(crate) fn combat_search_action_ordering_role_label_for_state_with_plugins(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
    plugins: CombatSearchActionOrderingPlugins<'_>,
) -> &'static str {
    action_priority::priority_for_input_with_plugins(engine, combat, input, plugins)
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

#[cfg(test)]
mod witness_v1_reexport_tests {
    #[test]
    fn witness_replay_v1_api_is_reexported() {
        let _: fn(
            &crate::sim::combat::CombatPosition,
            &super::CombatSearchV2WitnessLine,
            usize,
        ) -> Result<super::CombatSearchV2WitnessReplayV1, String> =
            super::replay_combat_search_witness_line_v1;
    }
}
