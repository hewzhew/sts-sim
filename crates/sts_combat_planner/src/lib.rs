//! Exact, resumable planning over complete player-turn options.
//!
//! It turns the simulator's exact legal input surface into replayable options
//! ending at the next supported combat boundary. Optional policies guide work
//! order but never change legality or terminal truth. Partial action prefixes
//! remain private generator work.
//! The crate boundary also keeps planner iteration out of the core unit-test
//! harness; production integration belongs to the control layer.

mod agenda;
mod decision;
mod evidence;
mod generator;
mod outcome_dataset;
mod outcome_model;
mod policy;
mod prospect;
mod replay;
mod selection_transaction;
mod types;
mod witness_search;

pub use agenda::{
    CombatPlannerAgendaBudget, CombatPlannerAgendaConfig, CombatPlannerAgendaCounters,
    CombatPlannerAgendaInterruption, CombatPlannerAgendaQuantum, CombatPlannerAgendaReport,
    CombatPlannerAgendaSession, CombatPlannerAgendaStatus,
};
pub use decision::{
    decide_combat_option, decide_combat_option_with_outcome_model, CombatEvaluationContext,
    CombatPlannerDecision, CombatPlannerDecisionBasis, CombatPlannerDecisionDeferral,
    CombatPlannerDecisionGap, CombatPlannerDecisionResult, ProspectEvidenceGap,
};
pub use evidence::{
    BoundaryWitnessEvidence, ContinuationEvidence, ContinuationInterruption, ExactHorizonEvidence,
    ExactHorizonGenerationGapEvidence, OptionProspect, OptionProspectId,
};
pub use generator::TurnOptionGeneratorSession;
pub use outcome_dataset::{
    load_combat_outcome_model_artifact_v1, load_combat_outcome_training_batch_v1,
    save_combat_outcome_model_artifact_v1, save_combat_outcome_training_batch_v1,
    train_combat_outcome_model_artifact_v1, CombatOutcomeDatasetErrorV1,
    CombatOutcomeDatasetSplitManifestV1, CombatOutcomeModelArtifactV1,
    CombatOutcomeTrainingBatchV1, CombatOutcomeTrainingCaseV1,
    COMBAT_OUTCOME_MODEL_ARTIFACT_SCHEMA_NAME_V1, COMBAT_OUTCOME_TRAINING_BATCH_SCHEMA_NAME_V1,
};
pub use outcome_model::{
    CombatOutcomeEstimateV1, CombatOutcomeFeatureVectorV1, CombatOutcomeLabelProvenanceV1,
    CombatOutcomeModelApplicabilityV1, CombatOutcomeModelEpistemicV1, CombatOutcomeModelErrorV1,
    CombatOutcomeModelTrainingConfigV1, CombatOutcomeModelV1, CombatOutcomeProbabilityIntervalV1,
    CombatOutcomeTrainingExampleV1, COMBAT_OUTCOME_FEATURE_SCHEMA_V1,
};
pub use policy::{
    CombatActionPolicy, CombatPolicyChoice, CombatPolicyWitnessProposal, CombatStateGuideRank,
    SharedCombatActionPolicy, UniformCombatActionPolicy,
};
pub use prospect::{
    ExactCombatZoneCounts, ExactCountChange, ExactI32Change, ExactImmediateOptionProspect,
    ExactProspectError,
};
pub use replay::{
    replay_turn_option, ReplayError, ReplayFailure, ReplayLimits, VerifiedTurnOptionReplay,
};
pub use types::{
    CombatDecisionRoot, CombatDecisionRootError, CombatPlanningCounters, CombatPlanningQuantum,
    CompleteTurnOption, CompleteTurnOptionBoundary, GenerationInterruption, TurnOptionAction,
    TurnOptionGenerationDiagnostics, TurnOptionGenerationGap, TurnOptionGenerationGapKind,
    TurnOptionGenerationReport, TurnOptionGenerationStatus, TurnOptionGeneratorConfig,
};
pub use witness_search::{
    OracleCombatDeepStateSnapshot, OracleCombatOneTurnLossEvidence,
    OracleCombatOneTurnViabilityEvidence, OracleCombatWitness, OracleCombatWitnessConfig,
    OracleCombatWitnessCounters, OracleCombatWitnessInterruption,
    OracleCombatWitnessProgressSnapshot, OracleCombatWitnessQuantum,
    OracleCombatWitnessReplayError, OracleCombatWitnessReport, OracleCombatWitnessSatisfaction,
    OracleCombatWitnessSession, OracleCombatWitnessStateMembershipSnapshot,
    OracleCombatWitnessStateProgressSnapshot, OracleCombatWitnessStatus,
};

#[cfg(test)]
mod tests;
