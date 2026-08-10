//! Model-facing projections and batched adapters over exact learning environments.

pub use sts_oracle_learning_env::eval::run_control::*;

#[path = "../../../../src/eval/run_control/combat_learning_env_pool.rs"]
mod combat_learning_env_pool;
#[path = "../../../../src/eval/run_control/learning_env_pool.rs"]
mod learning_env_pool;
#[path = "../../../../src/eval/run_control/learning_model_input.rs"]
mod learning_model_input;

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

#[cfg(test)]
#[path = "../../../../src/eval/run_control/learning_env/smoke_tests.rs"]
mod learning_env_smoke_tests;
