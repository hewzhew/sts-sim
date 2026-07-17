use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{
    CombatOutcomeModelErrorV1, CombatOutcomeModelTrainingConfigV1, CombatOutcomeModelV1,
    CombatOutcomeTrainingExampleV1,
};

pub const COMBAT_OUTCOME_TRAINING_BATCH_SCHEMA_NAME_V1: &str = "CombatOutcomeTrainingBatchV1";
pub const COMBAT_OUTCOME_MODEL_ARTIFACT_SCHEMA_NAME_V1: &str = "CombatOutcomeModelArtifactV1";
const SCHEMA_VERSION_V1: u32 = 1;

/// Every observation from one realized combat stays in one case. Cases with the
/// same split group (normally one run root and its counterfactuals) are never
/// divided between fitting and calibration.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatOutcomeTrainingCaseV1 {
    pub case_id: String,
    pub split_group_id: String,
    pub examples: Vec<CombatOutcomeTrainingExampleV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatOutcomeTrainingBatchV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub continuation_policy_manifest: String,
    pub cases: Vec<CombatOutcomeTrainingCaseV1>,
}

impl CombatOutcomeTrainingBatchV1 {
    pub fn new(
        continuation_policy_manifest: impl Into<String>,
        cases: Vec<CombatOutcomeTrainingCaseV1>,
    ) -> Result<Self, CombatOutcomeDatasetErrorV1> {
        let batch = Self {
            schema_name: COMBAT_OUTCOME_TRAINING_BATCH_SCHEMA_NAME_V1.to_string(),
            schema_version: SCHEMA_VERSION_V1,
            continuation_policy_manifest: continuation_policy_manifest.into(),
            cases,
        };
        batch.validate()?;
        Ok(batch)
    }

    pub fn validate(&self) -> Result<(), CombatOutcomeDatasetErrorV1> {
        if self.schema_name != COMBAT_OUTCOME_TRAINING_BATCH_SCHEMA_NAME_V1
            || self.schema_version != SCHEMA_VERSION_V1
        {
            return Err(CombatOutcomeDatasetErrorV1::UnsupportedTrainingBatchSchema);
        }
        if self.continuation_policy_manifest.trim().is_empty() {
            return Err(CombatOutcomeDatasetErrorV1::EmptyContinuationPolicyManifest);
        }
        if self.cases.is_empty() {
            return Err(CombatOutcomeDatasetErrorV1::EmptyTrainingBatch);
        }
        let mut case_ids = BTreeSet::new();
        for case in &self.cases {
            if case.case_id.trim().is_empty() || case.split_group_id.trim().is_empty() {
                return Err(CombatOutcomeDatasetErrorV1::EmptyCaseIdentity);
            }
            if !case_ids.insert(case.case_id.as_str()) {
                return Err(CombatOutcomeDatasetErrorV1::DuplicateCaseId(
                    case.case_id.clone(),
                ));
            }
            if case.examples.is_empty() {
                return Err(CombatOutcomeDatasetErrorV1::EmptyCase(case.case_id.clone()));
            }
            let victory = case.examples[0].victory;
            for example in &case.examples {
                if example.victory != victory {
                    return Err(CombatOutcomeDatasetErrorV1::MixedTerminalLabels(
                        case.case_id.clone(),
                    ));
                }
                if example.continuation_policy_manifest != self.continuation_policy_manifest {
                    return Err(CombatOutcomeDatasetErrorV1::ContinuationPolicyMismatch);
                }
                if example.features.0.iter().any(|value| !value.is_finite())
                    || !example.terminal_hp_fraction.is_finite()
                    || !(0.0..=1.0).contains(&example.terminal_hp_fraction)
                {
                    return Err(CombatOutcomeDatasetErrorV1::NonFiniteExample(
                        case.case_id.clone(),
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatOutcomeDatasetSplitManifestV1 {
    pub algorithm: String,
    pub calibration_group_modulus: u64,
    pub calibration_group_remainder: u64,
    pub training_split_group_ids: Vec<String>,
    pub calibration_split_group_ids: Vec<String>,
    pub training_case_ids: Vec<String>,
    pub calibration_case_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatOutcomeModelArtifactV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub feature_schema_id: String,
    pub continuation_policy_manifest: String,
    pub split: CombatOutcomeDatasetSplitManifestV1,
    pub training_config: CombatOutcomeModelTrainingConfigV1,
    pub model: CombatOutcomeModelV1,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CombatOutcomeDatasetErrorV1 {
    Io(String),
    Json(String),
    UnsupportedTrainingBatchSchema,
    UnsupportedModelArtifactSchema,
    EmptyContinuationPolicyManifest,
    EmptyTrainingBatch,
    EmptyCaseIdentity,
    EmptyCase(String),
    DuplicateCaseId(String),
    MixedTerminalLabels(String),
    ContinuationPolicyMismatch,
    NonFiniteExample(String),
    InvalidSplitConfiguration,
    EmptyTrainingPartition,
    EmptyCalibrationPartition,
    Model(CombatOutcomeModelErrorV1),
}

pub fn train_combat_outcome_model_artifact_v1(
    batches: &[CombatOutcomeTrainingBatchV1],
    model_id: impl Into<String>,
    config: CombatOutcomeModelTrainingConfigV1,
    calibration_group_modulus: u64,
    calibration_group_remainder: u64,
) -> Result<CombatOutcomeModelArtifactV1, CombatOutcomeDatasetErrorV1> {
    if calibration_group_modulus < 2 || calibration_group_remainder >= calibration_group_modulus {
        return Err(CombatOutcomeDatasetErrorV1::InvalidSplitConfiguration);
    }
    let Some(first) = batches.first() else {
        return Err(CombatOutcomeDatasetErrorV1::EmptyTrainingBatch);
    };
    let policy = first.continuation_policy_manifest.clone();
    let mut cases = Vec::new();
    let mut case_ids = BTreeSet::new();
    for batch in batches {
        batch.validate()?;
        if batch.continuation_policy_manifest != policy {
            return Err(CombatOutcomeDatasetErrorV1::ContinuationPolicyMismatch);
        }
        for case in &batch.cases {
            if !case_ids.insert(case.case_id.clone()) {
                return Err(CombatOutcomeDatasetErrorV1::DuplicateCaseId(
                    case.case_id.clone(),
                ));
            }
            cases.push(case);
        }
    }

    let mut groups = BTreeMap::<&str, bool>::new();
    for case in &cases {
        groups.entry(&case.split_group_id).or_insert_with(|| {
            stable_hash(&case.split_group_id) % calibration_group_modulus
                == calibration_group_remainder
        });
    }

    let mut training = Vec::new();
    let mut calibration = Vec::new();
    let mut training_case_ids = Vec::new();
    let mut calibration_case_ids = Vec::new();
    for case in cases {
        if groups[case.split_group_id.as_str()] {
            calibration.extend(case.examples.iter().cloned());
            calibration_case_ids.push(case.case_id.clone());
        } else {
            training.extend(case.examples.iter().cloned());
            training_case_ids.push(case.case_id.clone());
        }
    }
    if training.is_empty() {
        return Err(CombatOutcomeDatasetErrorV1::EmptyTrainingPartition);
    }
    if calibration.is_empty() {
        return Err(CombatOutcomeDatasetErrorV1::EmptyCalibrationPartition);
    }

    let mut training_split_group_ids = groups
        .iter()
        .filter_map(|(group, calibration)| (!*calibration).then_some((*group).to_string()))
        .collect::<Vec<_>>();
    let mut calibration_split_group_ids = groups
        .iter()
        .filter_map(|(group, calibration)| calibration.then_some((*group).to_string()))
        .collect::<Vec<_>>();
    training_split_group_ids.sort();
    calibration_split_group_ids.sort();
    training_case_ids.sort();
    calibration_case_ids.sort();

    let model = CombatOutcomeModelV1::fit(model_id, &policy, &training, &calibration, config)
        .map_err(CombatOutcomeDatasetErrorV1::Model)?;
    Ok(CombatOutcomeModelArtifactV1 {
        schema_name: COMBAT_OUTCOME_MODEL_ARTIFACT_SCHEMA_NAME_V1.to_string(),
        schema_version: SCHEMA_VERSION_V1,
        feature_schema_id: crate::COMBAT_OUTCOME_FEATURE_SCHEMA_V1.to_string(),
        continuation_policy_manifest: policy,
        split: CombatOutcomeDatasetSplitManifestV1 {
            algorithm: "fnv1a64(split_group_id) modulo v1".to_string(),
            calibration_group_modulus,
            calibration_group_remainder,
            training_split_group_ids,
            calibration_split_group_ids,
            training_case_ids,
            calibration_case_ids,
        },
        training_config: config,
        model,
    })
}

pub fn load_combat_outcome_training_batch_v1(
    path: &Path,
) -> Result<CombatOutcomeTrainingBatchV1, CombatOutcomeDatasetErrorV1> {
    let payload = fs::read_to_string(path)
        .map_err(|error| CombatOutcomeDatasetErrorV1::Io(error.to_string()))?;
    let batch = serde_json::from_str::<CombatOutcomeTrainingBatchV1>(&payload)
        .map_err(|error| CombatOutcomeDatasetErrorV1::Json(error.to_string()))?;
    batch.validate()?;
    Ok(batch)
}

pub fn save_combat_outcome_training_batch_v1(
    path: &Path,
    batch: &CombatOutcomeTrainingBatchV1,
) -> Result<(), CombatOutcomeDatasetErrorV1> {
    batch.validate()?;
    write_pretty_json(path, batch)
}

pub fn load_combat_outcome_model_artifact_v1(
    path: &Path,
) -> Result<CombatOutcomeModelArtifactV1, CombatOutcomeDatasetErrorV1> {
    let payload = fs::read_to_string(path)
        .map_err(|error| CombatOutcomeDatasetErrorV1::Io(error.to_string()))?;
    let artifact = serde_json::from_str::<CombatOutcomeModelArtifactV1>(&payload)
        .map_err(|error| CombatOutcomeDatasetErrorV1::Json(error.to_string()))?;
    if artifact.schema_name != COMBAT_OUTCOME_MODEL_ARTIFACT_SCHEMA_NAME_V1
        || artifact.schema_version != SCHEMA_VERSION_V1
        || artifact.feature_schema_id != crate::COMBAT_OUTCOME_FEATURE_SCHEMA_V1
    {
        return Err(CombatOutcomeDatasetErrorV1::UnsupportedModelArtifactSchema);
    }
    Ok(artifact)
}

pub fn save_combat_outcome_model_artifact_v1(
    path: &Path,
    artifact: &CombatOutcomeModelArtifactV1,
) -> Result<(), CombatOutcomeDatasetErrorV1> {
    if artifact.schema_name != COMBAT_OUTCOME_MODEL_ARTIFACT_SCHEMA_NAME_V1
        || artifact.schema_version != SCHEMA_VERSION_V1
        || artifact.feature_schema_id != crate::COMBAT_OUTCOME_FEATURE_SCHEMA_V1
    {
        return Err(CombatOutcomeDatasetErrorV1::UnsupportedModelArtifactSchema);
    }
    write_pretty_json(path, artifact)
}

fn write_pretty_json(
    path: &Path,
    value: &impl Serialize,
) -> Result<(), CombatOutcomeDatasetErrorV1> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| CombatOutcomeDatasetErrorV1::Io(error.to_string()))?;
    }
    let payload = serde_json::to_string_pretty(value)
        .map_err(|error| CombatOutcomeDatasetErrorV1::Json(error.to_string()))?;
    fs::write(path, payload).map_err(|error| CombatOutcomeDatasetErrorV1::Io(error.to_string()))
}

fn stable_hash(value: &str) -> u64 {
    value
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CombatOutcomeFeatureVectorV1, CombatOutcomeLabelProvenanceV1};

    fn case(id: &str, group: &str, victory: bool, count: usize) -> CombatOutcomeTrainingCaseV1 {
        CombatOutcomeTrainingCaseV1 {
            case_id: id.to_string(),
            split_group_id: group.to_string(),
            examples: (0..count)
                .map(|index| CombatOutcomeTrainingExampleV1 {
                    features: CombatOutcomeFeatureVectorV1([
                        if victory { 0.8 } else { 0.1 },
                        0.0,
                        0.6,
                        index as f64 / 100.0,
                        0.2,
                        if victory { 0.1 } else { 0.8 },
                        0.0,
                        0.5,
                        0.5,
                        0.0,
                        0.0,
                        0.0,
                    ]),
                    victory,
                    terminal_hp_fraction: if victory { 0.6 } else { 0.0 },
                    provenance: CombatOutcomeLabelProvenanceV1::RealizedBehaviorCombat,
                    continuation_policy_manifest: "policy-v1".to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn split_group_is_never_leaked_across_fit_and_calibration() {
        let cases = (0..40)
            .map(|index| {
                case(
                    &format!("case-{index}"),
                    &format!("run-{index}"),
                    index % 2 == 0,
                    4,
                )
            })
            .collect();
        let batch = CombatOutcomeTrainingBatchV1::new("policy-v1", cases).unwrap();
        let artifact = (0..5)
            .find_map(|remainder| {
                train_combat_outcome_model_artifact_v1(
                    std::slice::from_ref(&batch),
                    "model-v1",
                    CombatOutcomeModelTrainingConfigV1::default(),
                    5,
                    remainder,
                )
                .ok()
            })
            .expect("one deterministic remainder must contain both outcome classes");
        let training = artifact
            .split
            .training_split_group_ids
            .iter()
            .collect::<BTreeSet<_>>();
        assert!(artifact
            .split
            .calibration_split_group_ids
            .iter()
            .all(|group| !training.contains(group)));
        let encoded = serde_json::to_string(&artifact).expect("artifact serializes");
        let decoded: CombatOutcomeModelArtifactV1 =
            serde_json::from_str(&encoded).expect("artifact deserializes");
        assert_eq!(decoded.schema_name, artifact.schema_name);
        assert_eq!(decoded.feature_schema_id, artifact.feature_schema_id);
        assert_eq!(decoded.continuation_policy_manifest, artifact.continuation_policy_manifest);
        assert_eq!(decoded.split, artifact.split);
    }

    #[test]
    fn malformed_case_cannot_mix_terminal_labels() {
        let mut invalid = case("case", "run", true, 2);
        invalid.examples[1].victory = false;
        assert_eq!(
            CombatOutcomeTrainingBatchV1::new("policy-v1", vec![invalid]),
            Err(CombatOutcomeDatasetErrorV1::MixedTerminalLabels(
                "case".to_string()
            ))
        );
    }
}
