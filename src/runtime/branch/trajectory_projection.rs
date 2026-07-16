use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ai::planner_core::{
    PlannerMechanicsManifest, PlannerOutcomeSnapshot, SelectionProbability,
};
use crate::eval::run_control::{RunCombatResolutionKindV1, RunDecisionSelectionSourceV1};

use super::{RunTrajectoryHeadV1, RunTrajectorySegmentDispositionV1, RunTrajectorySegmentV1};

pub const RUN_TRAJECTORY_RECONSTRUCTION_SCHEMA_NAME: &str = "RunTrajectoryReconstruction";
pub const RUN_TRAJECTORY_RECONSTRUCTION_SCHEMA_VERSION: u32 = 1;
pub const RUN_TRAJECTORY_BEHAVIOR_PROJECTION_SCHEMA_NAME: &str = "RunTrajectoryBehaviorProjection";
pub const RUN_TRAJECTORY_BEHAVIOR_PROJECTION_SCHEMA_VERSION: u32 = 1;
pub const RUN_TRAJECTORY_OUTCOME_PROJECTION_SCHEMA_NAME: &str = "RunTrajectoryOutcomeProjection";
pub const RUN_TRAJECTORY_OUTCOME_PROJECTION_SCHEMA_VERSION: u32 = 1;
pub const RUN_TRAJECTORY_PROJECTION_INDEX_SCHEMA_NAME: &str = "RunTrajectoryProjectionIndex";
pub const RUN_TRAJECTORY_PROJECTION_INDEX_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunTrajectoryReconstructionV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub run_id: String,
    pub head: RunTrajectoryHeadV1,
    pub segments: Vec<RunTrajectorySegmentV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunTrajectoryBehaviorProjectionV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub run_id: String,
    pub head: RunTrajectoryHeadV1,
    pub events: Vec<RunTrajectoryBehaviorEventV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunTrajectoryBehaviorEventV1 {
    pub behavior_id: String,
    pub segment_id: String,
    pub occurrence_id: String,
    pub segment_depth: u64,
    pub journal_ordinal: usize,
    pub sequence: u64,
    pub decision_step: u64,
    pub decision_id: String,
    pub observation_id: String,
    pub legal_candidate_set_id: String,
    pub run_candidate_id: String,
    pub planner_candidate_id: String,
    pub selection_source: RunDecisionSelectionSourceV1,
    pub selection_probability: SelectionProbability,
    pub mechanics: PlannerMechanicsManifest,
    pub label_role: RunTrajectoryBehaviorLabelRoleV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunTrajectoryBehaviorLabelRoleV1 {
    ObservedBehaviorNotTeacher,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunTrajectoryOutcomeProjectionV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub run_id: String,
    pub head: RunTrajectoryHeadV1,
    pub attachments: Vec<RunTrajectoryOutcomeAttachmentV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunTrajectoryProjectionIndexV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub run_id: String,
    pub entries: Vec<RunTrajectoryProjectionIndexEntryV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunTrajectoryProjectionIndexEntryV1 {
    pub branch_id: u64,
    pub head: RunTrajectoryHeadV1,
    pub reconstruction_path: PathBuf,
    pub behavior_path: PathBuf,
    pub outcome_path: PathBuf,
    pub behavior_event_count: usize,
    pub outcome_attachment_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunTrajectoryOutcomeAttachmentV1 {
    pub attachment_id: String,
    pub behavior_id: String,
    pub horizon: RunTrajectoryOutcomeHorizonV1,
    pub before: PlannerOutcomeSnapshot,
    pub result: RunTrajectoryOutcomeResultV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunTrajectoryOutcomeHorizonV1 {
    ImmediateCommittedSuccessor,
    NextCombatResolution,
    ActTerminal,
    RunTerminal,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RunTrajectoryOutcomeResultV1 {
    Observed { fact: RunTrajectoryOutcomeFactV1 },
    Censored { reason: RunTrajectoryCensorReasonV1 },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RunTrajectoryOutcomeFactV1 {
    Observation {
        segment_id: String,
        occurrence_id: String,
        observation_id: String,
        snapshot: PlannerOutcomeSnapshot,
    },
    CombatResolution {
        segment_id: String,
        journal_ordinal: usize,
        resolution_kind: RunCombatResolutionKindV1,
    },
    Terminal {
        segment_id: String,
        outcome: RunTrajectoryTerminalOutcomeV1,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunTrajectoryTerminalOutcomeV1 {
    Victory,
    Defeat,
}

impl RunTrajectoryTerminalOutcomeV1 {
    pub fn from_disposition(disposition: RunTrajectorySegmentDispositionV1) -> Option<Self> {
        match disposition {
            RunTrajectorySegmentDispositionV1::TerminalVictory => Some(Self::Victory),
            RunTrajectorySegmentDispositionV1::TerminalDefeat => Some(Self::Defeat),
            RunTrajectorySegmentDispositionV1::Resumable
            | RunTrajectorySegmentDispositionV1::Stopped => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunTrajectoryCensorReasonV1 {
    TrajectoryHeadResumable,
    TrajectoryStoppedBeforeHorizon,
}
