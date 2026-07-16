use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::sim::combat::CombatPosition;
use crate::state::core::ClientInput;

use super::super::CombatPolicyObservationV1;

pub const COMBAT_POLICY_INFORMATION_SET_SCHEMA_NAME: &str = "CombatPolicyInformationSetV1";
pub const COMBAT_POLICY_INFORMATION_SET_SCHEMA_VERSION: u32 = 1;
pub const COMBAT_POLICY_ROOT_HISTORY_ID: &str = "combat_policy_history:root";

#[derive(Clone, Debug)]
pub struct CombatScenarioParticleV1 {
    pub(super) scenario_id: String,
    pub(super) public_history_id: String,
    pub(super) position: CombatPosition,
}

impl CombatScenarioParticleV1 {
    pub fn root(scenario_id: impl Into<String>, position: CombatPosition) -> Self {
        Self {
            scenario_id: scenario_id.into(),
            public_history_id: COMBAT_POLICY_ROOT_HISTORY_ID.to_string(),
            position,
        }
    }

    pub(super) fn from_public_history(
        scenario_id: impl Into<String>,
        public_history_id: impl Into<String>,
        position: CombatPosition,
    ) -> Self {
        Self {
            scenario_id: scenario_id.into(),
            public_history_id: public_history_id.into(),
            position,
        }
    }

    pub fn scenario_id(&self) -> &str {
        &self.scenario_id
    }

    pub fn public_history_id(&self) -> &str {
        &self.public_history_id
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPolicyInformationSetKeyV1 {
    pub public_history_id: String,
    pub public_observation_hash: String,
    pub public_candidate_set_hash: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPolicyObservationEnvelopeV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub engine_state: String,
    pub turn_count: u32,
    pub observation: CombatPolicyObservationV1,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicTargetV1 {
    pub monster_slot: u8,
    pub enemy_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatPublicActionV1 {
    PlayCard {
        hand_index: usize,
        card_id: String,
        upgrades: u8,
        cost_for_turn: i32,
        target: Option<CombatPublicTargetV1>,
    },
    UsePotion {
        potion_slot: usize,
        potion_id: String,
        target: Option<CombatPublicTargetV1>,
    },
    DiscardPotion {
        potion_slot: usize,
        potion_id: String,
    },
    EndTurn,
    Proceed,
    Cancel,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatPolicyObservationGroupV1 {
    pub key: CombatPolicyInformationSetKeyV1,
    pub observation: CombatPolicyObservationEnvelopeV1,
    pub candidates: Vec<CombatPublicActionV1>,
    pub scenario_count: usize,
}

#[derive(Clone)]
pub struct CombatScenarioDecisionBindingV1 {
    pub(super) action: CombatPublicActionV1,
    pub(super) exact_inputs: Vec<(String, ClientInput)>,
}

impl CombatScenarioDecisionBindingV1 {
    pub fn action(&self) -> &CombatPublicActionV1 {
        &self.action
    }

    pub fn scenario_count(&self) -> usize {
        self.exact_inputs.len()
    }

    #[cfg(test)]
    pub(super) fn exact_inputs(&self) -> &[(String, ClientInput)] {
        &self.exact_inputs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatScenarioPolicyErrorV1 {
    EmptyScenarioSet,
    DuplicateScenarioId {
        scenario_id: String,
    },
    UnsupportedBoundary {
        scenario_id: String,
        engine_state: String,
    },
    NonQuiescentBoundary {
        scenario_id: String,
        pending_work: Vec<String>,
    },
    UnsupportedSuccessorBoundary {
        scenario_id: String,
        engine_state: String,
    },
    StepTruncated {
        scenario_id: String,
        engine_steps: usize,
        timed_out: bool,
    },
    UnsupportedAction {
        scenario_id: String,
        input: String,
    },
    InvalidLegalAction {
        scenario_id: String,
        input: String,
        reason: String,
    },
    AmbiguousPublicAction {
        scenario_id: String,
        action: String,
    },
    InformationSetHashCollision {
        key: CombatPolicyInformationSetKeyV1,
    },
    ActionUnavailable {
        information_set: CombatPolicyInformationSetKeyV1,
        action: String,
    },
    MissingExactBinding {
        action: String,
    },
}

impl fmt::Display for CombatScenarioPolicyErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyScenarioSet => write!(formatter, "combat scenario set is empty"),
            Self::DuplicateScenarioId { scenario_id } => {
                write!(formatter, "duplicate combat scenario id '{scenario_id}'")
            }
            Self::UnsupportedBoundary {
                scenario_id,
                engine_state,
            } => write!(
                formatter,
                "combat scenario '{scenario_id}' has unsupported policy boundary {engine_state}"
            ),
            Self::NonQuiescentBoundary {
                scenario_id,
                pending_work,
            } => write!(
                formatter,
                "combat scenario '{scenario_id}' is not at a quiescent policy boundary: {}",
                pending_work.join(", ")
            ),
            Self::UnsupportedSuccessorBoundary {
                scenario_id,
                engine_state,
            } => write!(
                formatter,
                "combat scenario '{scenario_id}' stepped to unsupported policy boundary {engine_state}"
            ),
            Self::StepTruncated {
                scenario_id,
                engine_steps,
                timed_out,
            } => write!(
                formatter,
                "combat scenario '{scenario_id}' did not reach a stable policy boundary after {engine_steps} engine steps (timed_out={timed_out})"
            ),
            Self::UnsupportedAction { scenario_id, input } => write!(
                formatter,
                "combat scenario '{scenario_id}' exposes unsupported public action {input}"
            ),
            Self::InvalidLegalAction {
                scenario_id,
                input,
                reason,
            } => write!(
                formatter,
                "combat scenario '{scenario_id}' has invalid legal action {input}: {reason}"
            ),
            Self::AmbiguousPublicAction {
                scenario_id,
                action,
            } => write!(
                formatter,
                "combat scenario '{scenario_id}' maps multiple exact actions to public action {action}"
            ),
            Self::InformationSetHashCollision { key } => write!(
                formatter,
                "combat information-set hash collision for history '{}' observation '{}' candidates '{}'",
                key.public_history_id,
                key.public_observation_hash,
                key.public_candidate_set_hash
            ),
            Self::ActionUnavailable {
                information_set,
                action,
            } => write!(
                formatter,
                "public action {action} is unavailable in information set '{}'",
                information_set.public_observation_hash
            ),
            Self::MissingExactBinding { action } => write!(
                formatter,
                "combat information set is missing an exact binding for public action {action}"
            ),
        }
    }
}

impl Error for CombatScenarioPolicyErrorV1 {}

pub(super) type ExactActionMap = BTreeMap<CombatPublicActionV1, ClientInput>;
