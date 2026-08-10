//! Exact single-episode environments; no model projection or policy objective.

pub use sts_oracle_run_control::eval::run_control::*;

#[path = "../../../../src/eval/run_control/combat_learning_env.rs"]
mod combat_learning_env;
#[path = "../../../../src/eval/run_control/combat_learning_root_artifact.rs"]
mod combat_learning_root_artifact;
#[path = "../../../../src/eval/run_control/learning_env.rs"]
mod learning_env;

pub use combat_learning_env::{
    CombatLearningBoundaryV1, CombatLearningEnvCheckpointV1, CombatLearningEnvV1,
    CombatLearningEpisodeIdentityV1, CombatLearningResourceSnapshotV1, CombatLearningRootContextV1,
    CombatLearningRootIdentityV1, CombatLearningRootV1, CombatLearningStepV1,
    CombatLearningTerminalOutcomeV1,
};
pub use combat_learning_root_artifact::{
    CombatLearningRootArtifactV1, CombatLearningRootBatchArtifactV1,
    COMBAT_LEARNING_ROOT_ARTIFACT_FORMAT_VERSION, COMBAT_LEARNING_ROOT_ARTIFACT_MAGIC,
};
#[doc(hidden)]
pub use learning_env::LearningPreparedActionV1;
pub use learning_env::{
    LearningActionV1, LearningBoundaryKindV1, LearningBoundaryV1, LearningCombatBoundaryV1,
    LearningEnvV1, LearningObservationCompletenessV1, LearningPublicRunContextV1, LearningStepV1,
    LearningStrategicBoundaryV1, LearningStrategicContextKindV1, LearningTerminalOutcomeV1,
};
