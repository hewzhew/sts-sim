//! Model-facing projections and batched adapters over exact learning environments.

pub use sts_oracle_learning_env::eval::run_control::*;

#[path = "../../../../src/eval/run_control/combat_learning_env_pool.rs"]
mod combat_learning_env_pool;
#[path = "../../../../src/eval/run_control/learning_env_pool.rs"]
mod learning_env_pool;
#[path = "../../../../src/eval/run_control/learning_model_input.rs"]
mod learning_model_input;
#[path = "../../../../src/eval/run_control/public_information_snapshot.rs"]
mod public_information_snapshot;

pub use combat_learning_env_pool::{
    CombatLearningEnvPoolError, CombatLearningEnvPoolModelBatchV1, CombatLearningEnvPoolSlotStepV1,
    CombatLearningEnvPoolStepV1, CombatLearningEnvPoolV1,
};
pub use learning_env_pool::{
    LearningEnvPoolError, LearningEnvPoolModelBatchV1, LearningEnvPoolSlotStepV1,
    LearningEnvPoolStepV1, LearningEnvPoolV1,
};
pub use learning_model_input::{
    CombatLearningPotionPolicyV1, LearningCombatAtomicActionV1, LearningCombatIndexedChoiceV1,
    LearningCombatModelObservationV1, LearningCombatMonsterV1, LearningCombatMonstersV1,
    LearningCombatSelectionDomainSemanticsV1, LearningCombatSelectionDomainV1,
    LearningCombatSelectionFamilyV1, LearningDenseActionMaskV1, LearningModelBatchV1,
    LearningModelCandidateSemanticsV1, LearningModelCandidateV1, LearningModelChoiceV1,
    LearningModelDecisionV1, LearningModelInputError, LearningModelObservationV1,
    LearningRunSelectionFamilyV1, LearningSelectionCandidateSemanticsV1,
    LearningSelectionCandidateV1, LearningSelectionDecisionV1, LearningSelectionDraftV1,
    LearningSelectionModelBatchV1, LearningSelectionModelRowV1, LearningSelectionStepV1,
    LearningStrategicModelObservationV1, LearningStrategicPotionSlotV1, LearningStrategicPotionV1,
};
pub use public_information_snapshot::{
    learning_public_information_snapshot_v1,
    learning_public_information_snapshot_with_potion_policy_v1,
    learning_public_selection_snapshot_v1, LEARNING_PUBLIC_CANDIDATE_SURFACE_SCHEMA_NAME,
    LEARNING_PUBLIC_CANDIDATE_SURFACE_SCHEMA_VERSION,
    LEARNING_PUBLIC_COMBAT_HISTORY_SNAPSHOT_SCHEMA_NAME,
    LEARNING_PUBLIC_COMBAT_OBSERVATION_SCHEMA_NAME,
    LEARNING_PUBLIC_COMBAT_OBSERVATION_SCHEMA_VERSION,
    LEARNING_PUBLIC_HISTORY_SNAPSHOT_SCHEMA_VERSION,
    LEARNING_PUBLIC_SELECTION_HISTORY_SNAPSHOT_SCHEMA_NAME,
    LEARNING_PUBLIC_SELECTION_OBSERVATION_SCHEMA_NAME,
    LEARNING_PUBLIC_SELECTION_OBSERVATION_SCHEMA_VERSION,
    LEARNING_PUBLIC_STRATEGIC_HISTORY_SNAPSHOT_SCHEMA_NAME,
    LEARNING_PUBLIC_STRATEGIC_OBSERVATION_SCHEMA_NAME,
    LEARNING_PUBLIC_STRATEGIC_OBSERVATION_SCHEMA_VERSION,
};

#[cfg(test)]
#[path = "../../../../src/eval/run_control/learning_env/smoke_tests.rs"]
mod learning_env_smoke_tests;
