mod curiosity;
mod decision_audit;
mod equivalence;
mod hand_select;
mod legal_moves;
mod mcts;
mod profile;
mod root_policy;
mod root_prior;
mod root_rollout;
mod sequence_judge;
mod tactical_bonus;
mod tactical_override;

pub use decision_audit::{
    audit_fixture, audit_state, build_fixture_from_reconstructed_step, extract_preference_samples,
    load_fixture_path, render_text_report, write_fixture_path, CombatPreferenceSample,
    CombatPreferenceState, DecisionAuditConfig, DecisionAuditEngineState, DecisionAuditFixture,
    DecisionAuditReport, ScoreBreakdown, TrajectoryOutcomeKind, TrajectoryReport,
};
pub use equivalence::{SearchEquivalenceKind, SearchEquivalenceMode};
pub use mcts::{
    diagnose_root_search, diagnose_root_search_with_depth,
    diagnose_root_search_with_depth_and_mode,
    diagnose_root_search_with_depth_and_mode_and_profiling,
    diagnose_root_search_with_depth_and_mode_and_root_prior, diagnose_root_search_with_mode,
    diagnose_root_search_with_mode_and_profiling, find_best_move, find_best_move_with_mode,
    find_best_move_with_mode_and_profiling, SearchDiagnostics, SearchMoveStat,
};
pub use profile::{
    SearchNodeCounters, SearchPhaseProfile, SearchProfileBreakdown, SearchProfilingLevel,
};
pub use root_prior::{LookupRootPriorProvider, RootPriorConfig, RootPriorQueryKey};

pub(super) use curiosity::curiosity_archetype_move_bonus;
pub(crate) use equivalence::{default_equivalence_mode, reduce_search_moves};
pub(crate) use legal_moves::get_legal_moves;
pub(crate) use root_policy::{sequencing_assessment_for_input, StatePressureFeatures};
pub(crate) use tactical_override::tactical_override;
pub(super) use tactical_bonus::tactical_move_bonus;

pub fn legal_moves_for_audit(
    engine: &crate::state::EngineState,
    combat: &crate::runtime::combat::CombatState,
) -> Vec<crate::state::core::ClientInput> {
    legal_moves::get_legal_moves(engine, combat)
}

fn intent_hits(intent: &crate::runtime::combat::Intent) -> i32 {
    match intent {
        crate::runtime::combat::Intent::Attack { hits, .. }
        | crate::runtime::combat::Intent::AttackBuff { hits, .. }
        | crate::runtime::combat::Intent::AttackDebuff { hits, .. }
        | crate::runtime::combat::Intent::AttackDefend { hits, .. } => (*hits as i32).max(1),
        _ => 0,
    }
}
