use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ai::combat_policy_v1::{
    CombatPolicyInformationSetKeyV1, CombatPolicyObservationGroupV1, CombatPublicActionV1,
};

pub const COMBAT_LAB_POLICY_BANK_REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatLabPolicyInformationScopeV1 {
    PublicHistoryScenarioPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLabPolicyBankLimitsV1 {
    pub max_information_set_decisions: usize,
    pub max_actions_per_scenario: usize,
    pub max_engine_steps_per_action: usize,
}

pub struct CombatLabPublicPolicyDecisionV1<'a> {
    pub decision_index: usize,
    pub depth: usize,
    pub information_set: &'a CombatPolicyObservationGroupV1,
}

pub trait CombatLabPublicPolicyV1 {
    fn choose_action(
        &mut self,
        decision: CombatLabPublicPolicyDecisionV1<'_>,
    ) -> Result<CombatPublicActionV1, CombatLabPolicyDecisionGapV1>;
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatLabPolicyDecisionGapV1 {
    NoAcceptableAction,
    UnsupportedInformationSet,
    ExternalStop,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatLabPolicyUnresolvedReasonV1 {
    PolicyGap { gap: CombatLabPolicyDecisionGapV1 },
    ActionLimit,
    DecisionBudget,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatLabPolicyScenarioResolutionV1 {
    Win,
    Loss,
    Unresolved {
        reason: CombatLabPolicyUnresolvedReasonV1,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLabPolicyScenarioOutcomeV1 {
    pub sample_index: u64,
    pub shuffle_seed: u64,
    pub resolution: CombatLabPolicyScenarioResolutionV1,
    pub start_hp: i32,
    pub final_observed_hp: i32,
    pub observed_hp_loss: i32,
    pub turn_count: u32,
    pub actions: usize,
    pub cards_played: u32,
    pub potions_used: u32,
    pub public_action_history: Vec<CombatPublicActionV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLabPolicyNumericSummaryV1 {
    pub count: usize,
    pub mean: Option<f64>,
    pub median: Option<f64>,
    pub p90_nearest_rank: Option<i32>,
    pub max: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLabPolicyBankSummaryV1 {
    pub scenario_count: usize,
    pub wins: usize,
    pub losses: usize,
    pub unresolved: usize,
    pub resolution_rate: Option<f64>,
    pub win_rate_all_scenarios: Option<f64>,
    pub win_rate_resolved: Option<f64>,
    pub terminal_hp_loss: CombatLabPolicyNumericSummaryV1,
    pub win_hp_loss: CombatLabPolicyNumericSummaryV1,
    pub loss_hp_loss: CombatLabPolicyNumericSummaryV1,
    pub observed_actions: CombatLabPolicyNumericSummaryV1,
    pub observed_turns: CombatLabPolicyNumericSummaryV1,
    pub observed_potions_used: CombatLabPolicyNumericSummaryV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLabPolicyGapRecordV1 {
    pub information_set: CombatPolicyInformationSetKeyV1,
    pub depth: usize,
    pub scenario_count: usize,
    pub reason: CombatLabPolicyUnresolvedReasonV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLabPolicyBankReportV1 {
    pub schema_version: u32,
    pub information_scope: CombatLabPolicyInformationScopeV1,
    pub scenario_count: usize,
    pub information_set_decisions: usize,
    pub engine_steps: usize,
    pub max_frontier_information_sets: usize,
    pub gaps: Vec<CombatLabPolicyGapRecordV1>,
    pub outcomes: Vec<CombatLabPolicyScenarioOutcomeV1>,
    pub summary: CombatLabPolicyBankSummaryV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatLabPolicyBankErrorV1 {
    EmptyScenarioBank,
    InvalidLimit {
        field: &'static str,
    },
    DuplicateSampleIndex {
        sample_index: u64,
    },
    ScenarioBoundary {
        message: String,
    },
    PolicyReturnedUnavailableAction {
        information_set: CombatPolicyInformationSetKeyV1,
        action: String,
    },
    MissingScenarioAccumulator {
        scenario_id: String,
    },
    DuplicateScenarioResolution {
        scenario_id: String,
    },
    IncompleteScenario {
        scenario_id: String,
    },
}

impl fmt::Display for CombatLabPolicyBankErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyScenarioBank => write!(formatter, "combat policy scenario bank is empty"),
            Self::InvalidLimit { field } => {
                write!(
                    formatter,
                    "combat policy scenario bank limit '{field}' must be nonzero"
                )
            }
            Self::DuplicateSampleIndex { sample_index } => write!(
                formatter,
                "combat policy scenario bank repeats sample index {sample_index}"
            ),
            Self::ScenarioBoundary { message } => {
                write!(
                    formatter,
                    "combat policy scenario boundary failed: {message}"
                )
            }
            Self::PolicyReturnedUnavailableAction {
                information_set,
                action,
            } => write!(
                formatter,
                "public policy returned unavailable action {action} for information set '{}'",
                information_set.public_observation_hash
            ),
            Self::MissingScenarioAccumulator { scenario_id } => write!(
                formatter,
                "combat policy scenario '{scenario_id}' has no outcome accumulator"
            ),
            Self::DuplicateScenarioResolution { scenario_id } => write!(
                formatter,
                "combat policy scenario '{scenario_id}' resolved more than once"
            ),
            Self::IncompleteScenario { scenario_id } => write!(
                formatter,
                "combat policy scenario '{scenario_id}' left the execution loop without a terminal or typed unresolved result"
            ),
        }
    }
}

impl Error for CombatLabPolicyBankErrorV1 {}
