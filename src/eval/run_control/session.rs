use std::path::PathBuf;

use serde::de::{DeserializeOwned, Error as DeError};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::ai::combat_search_v2::CombatSearchV2PotionPolicy;
use crate::content::relics::RelicId;
use crate::eval::card_reward_value_loop::{
    CardRewardOutcomeCalibrationV1, CardRewardRouteRiskCalibrationV1,
    CardRewardStrategyPackageCalibrationV1,
};
use crate::runtime::combat::CombatCard;
use crate::state::core::{ActiveCombat, EngineState};
use crate::state::map::state::MapState;
use crate::state::run::{RunState, RunStateCheckpointV1, RunStateScheduleCheckpointV1};
use crate::state::selection::DomainEvent;

use super::auto_capture::AutoCombatCaptureConfig;
use super::outcome::CombatOutcomeTracker;
use super::reward_auto::RewardAutomationConfig;
use super::trace_annotation::{CombatAutomationTrajectoryRecordV1, RunControlTraceAnnotationV1};
use super::transition_report::ActionResult;

mod apply;
mod combat;

#[derive(Clone, Debug)]
pub struct RunControlConfig {
    pub seed: u64,
    pub ascension_level: u8,
    pub final_act: bool,
    pub player_class: &'static str,
    pub reward_automation: RewardAutomationConfig,
    pub auto_capture: AutoCombatCaptureConfig,
    pub search_max_nodes: Option<usize>,
    pub search_wall_ms: Option<u64>,
    pub search_max_hp_loss: Option<u32>,
    pub search_potion_policy: Option<CombatSearchV2PotionPolicy>,
    pub search_max_potions_used: Option<u32>,
    pub card_reward_outcome_calibration: Option<CardRewardOutcomeCalibrationV1>,
    pub card_reward_route_risk_calibration: Option<CardRewardRouteRiskCalibrationV1>,
    pub card_reward_strategy_package_calibration: Option<CardRewardStrategyPackageCalibrationV1>,
}

impl Default for RunControlConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            ascension_level: 0,
            final_act: false,
            player_class: "Ironclad",
            reward_automation: RewardAutomationConfig::default(),
            auto_capture: AutoCombatCaptureConfig::default(),
            search_max_nodes: None,
            search_wall_ms: None,
            search_max_hp_loss: None,
            search_potion_policy: None,
            search_max_potions_used: None,
            card_reward_outcome_calibration: None,
            card_reward_route_risk_calibration: None,
            card_reward_strategy_package_calibration: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RunControlSession {
    pub engine_state: EngineState,
    pub run_state: RunState,
    pub active_combat: Option<ActiveCombat>,
    pub decision_step: u64,
    pub reward_automation: RewardAutomationConfig,
    pub shop_visit_context: Option<ShopVisitContextV1>,
    pub(in crate::eval::run_control) auto_capture: AutoCombatCaptureConfig,
    pub(in crate::eval::run_control) search_max_nodes: Option<usize>,
    pub(in crate::eval::run_control) search_wall_ms: Option<u64>,
    pub(in crate::eval::run_control) search_max_hp_loss: Option<u32>,
    pub(in crate::eval::run_control) search_potion_policy: Option<CombatSearchV2PotionPolicy>,
    pub(in crate::eval::run_control) search_max_potions_used: Option<u32>,
    pub(in crate::eval::run_control) card_reward_outcome_calibration:
        Option<CardRewardOutcomeCalibrationV1>,
    pub(in crate::eval::run_control) card_reward_route_risk_calibration:
        Option<CardRewardRouteRiskCalibrationV1>,
    pub(in crate::eval::run_control) card_reward_strategy_package_calibration:
        Option<CardRewardStrategyPackageCalibrationV1>,
    pub(super) combat_outcomes: CombatOutcomeTracker,
    pub(in crate::eval::run_control) combat_sequence: u64,
    pub(in crate::eval::run_control) auto_capture_last_combat_sequence: Option<u64>,
    last_completed_combat_sequence: Option<u64>,
    last_completed_combat_source: Option<CombatCompletionSource>,
    current_combat_source: Option<CombatCompletionSource>,
    last_combat_automation_sequence: Option<u64>,
    last_combat_automation_trajectory: Option<CombatAutomationTrajectoryRecordV1>,
    last_capture_case: Option<LastBenchmarkCaptureCase>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ShopVisitContextV1 {
    #[serde(default)]
    pub entry_act: u8,
    #[serde(default)]
    pub entry_floor: i32,
    pub entry_gold: i32,
    pub maw_bank_live_at_entry: bool,
    pub spent_gold_in_visit: bool,
}

impl ShopVisitContextV1 {
    fn from_run_state(run_state: &RunState) -> Self {
        Self {
            entry_act: run_state.act_num,
            entry_floor: run_state.floor_num,
            entry_gold: run_state.gold,
            maw_bank_live_at_entry: run_state
                .relics
                .iter()
                .any(|relic| relic.id == RelicId::MawBank && !relic.used_up),
            spent_gold_in_visit: false,
        }
    }

    fn matches_run_state(self, run_state: &RunState) -> bool {
        self.entry_act == run_state.act_num && self.entry_floor == run_state.floor_num
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub(in crate::eval::run_control) struct LastBenchmarkCaptureCase {
    pub root: PathBuf,
    pub case_id: String,
    pub combat_sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub(in crate::eval::run_control) enum CombatCompletionSource {
    Manual,
    SearchCombat,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunControlSessionCheckpointV1 {
    engine_state: EngineState,
    run_state: RunStateCheckpointV1,
    active_combat: Option<ActiveCombat>,
    decision_step: u64,
    reward_automation: RewardAutomationConfig,
    shop_visit_context: Option<ShopVisitContextV1>,
    auto_capture: AutoCombatCaptureConfig,
    search_max_nodes: Option<usize>,
    search_wall_ms: Option<u64>,
    search_max_hp_loss: Option<u32>,
    search_potion_policy: Option<CombatSearchV2PotionPolicy>,
    search_max_potions_used: Option<u32>,
    card_reward_outcome_calibration: Option<CardRewardOutcomeCalibrationV1>,
    card_reward_route_risk_calibration: Option<CardRewardRouteRiskCalibrationV1>,
    card_reward_strategy_package_calibration: Option<CardRewardStrategyPackageCalibrationV1>,
    combat_outcomes: CombatOutcomeTracker,
    combat_sequence: u64,
    auto_capture_last_combat_sequence: Option<u64>,
    last_completed_combat_sequence: Option<u64>,
    last_completed_combat_source: Option<CombatCompletionSource>,
    current_combat_source: Option<CombatCompletionSource>,
    last_combat_automation_sequence: Option<u64>,
    last_combat_automation_trajectory: Option<CombatAutomationTrajectoryRecordV1>,
    last_capture_case: Option<LastBenchmarkCaptureCase>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
struct RunControlSessionCheckpointExtrasV1 {
    #[serde(skip_serializing_if = "reward_automation_config_is_default")]
    reward_automation: RewardAutomationConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    shop_visit_context: Option<ShopVisitContextV1>,
    #[serde(skip_serializing_if = "auto_capture_config_is_default")]
    auto_capture: AutoCombatCaptureConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    card_reward_outcome_calibration: Option<CardRewardOutcomeCalibrationV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    card_reward_route_risk_calibration: Option<CardRewardRouteRiskCalibrationV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    card_reward_strategy_package_calibration: Option<CardRewardStrategyPackageCalibrationV1>,
    #[serde(skip_serializing_if = "combat_outcome_tracker_is_default")]
    combat_outcomes: CombatOutcomeTracker,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_capture_last_combat_sequence: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_completed_combat_sequence: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_completed_combat_source: Option<CombatCompletionSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_combat_source: Option<CombatCompletionSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_combat_automation_sequence: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_combat_automation_trajectory: Option<CombatAutomationTrajectoryRecordV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_capture_case: Option<LastBenchmarkCaptureCase>,
}

impl RunControlSessionCheckpointExtrasV1 {
    fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

impl Serialize for RunControlSessionCheckpointV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let extras = RunControlSessionCheckpointExtrasV1 {
            reward_automation: self.reward_automation.clone(),
            shop_visit_context: self.shop_visit_context,
            auto_capture: self.auto_capture.clone(),
            card_reward_outcome_calibration: self.card_reward_outcome_calibration.clone(),
            card_reward_route_risk_calibration: self.card_reward_route_risk_calibration.clone(),
            card_reward_strategy_package_calibration: self
                .card_reward_strategy_package_calibration
                .clone(),
            combat_outcomes: self.combat_outcomes.clone(),
            auto_capture_last_combat_sequence: self.auto_capture_last_combat_sequence,
            last_completed_combat_sequence: self.last_completed_combat_sequence,
            last_completed_combat_source: self.last_completed_combat_source,
            current_combat_source: self.current_combat_source,
            last_combat_automation_sequence: self.last_combat_automation_sequence,
            last_combat_automation_trajectory: self.last_combat_automation_trajectory.clone(),
            last_capture_case: self.last_capture_case.clone(),
        };
        let extras = (!extras.is_empty()).then_some(extras);
        (
            &self.engine_state,
            &self.run_state,
            &self.active_combat,
            self.decision_step,
            &self.search_max_nodes,
            &self.search_wall_ms,
            &self.search_max_hp_loss,
            &self.search_potion_policy,
            &self.search_max_potions_used,
            self.combat_sequence,
            extras,
        )
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RunControlSessionCheckpointV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        if let Some(items) = value.as_array() {
            return compact_checkpoint_from_values(items).map_err(D::Error::custom);
        }
        serde_json::from_value::<RunControlSessionCheckpointLegacyV1>(value)
            .map(RunControlSessionCheckpointLegacyV1::into_checkpoint)
            .map_err(D::Error::custom)
    }
}

fn compact_checkpoint_from_values(
    items: &[serde_json::Value],
) -> Result<RunControlSessionCheckpointV1, String> {
    if items.len() != 11 {
        return Err(format!(
            "compact run-control checkpoint expected 11 fields, got {}",
            items.len()
        ));
    }
    let extras = compact_value::<Option<RunControlSessionCheckpointExtrasV1>>(items, 10, "extras")?
        .unwrap_or_default();
    Ok(RunControlSessionCheckpointV1 {
        engine_state: compact_value(items, 0, "engine_state")?,
        run_state: compact_value(items, 1, "run_state")?,
        active_combat: compact_value(items, 2, "active_combat")?,
        decision_step: compact_value(items, 3, "decision_step")?,
        reward_automation: extras.reward_automation,
        shop_visit_context: extras.shop_visit_context,
        auto_capture: extras.auto_capture,
        search_max_nodes: compact_value(items, 4, "search_max_nodes")?,
        search_wall_ms: compact_value(items, 5, "search_wall_ms")?,
        search_max_hp_loss: compact_value(items, 6, "search_max_hp_loss")?,
        search_potion_policy: compact_value(items, 7, "search_potion_policy")?,
        search_max_potions_used: compact_value(items, 8, "search_max_potions_used")?,
        card_reward_outcome_calibration: extras.card_reward_outcome_calibration,
        card_reward_route_risk_calibration: extras.card_reward_route_risk_calibration,
        card_reward_strategy_package_calibration: extras.card_reward_strategy_package_calibration,
        combat_outcomes: extras.combat_outcomes,
        combat_sequence: compact_value(items, 9, "combat_sequence")?,
        auto_capture_last_combat_sequence: extras.auto_capture_last_combat_sequence,
        last_completed_combat_sequence: extras.last_completed_combat_sequence,
        last_completed_combat_source: extras.last_completed_combat_source,
        current_combat_source: extras.current_combat_source,
        last_combat_automation_sequence: extras.last_combat_automation_sequence,
        last_combat_automation_trajectory: extras.last_combat_automation_trajectory,
        last_capture_case: extras.last_capture_case,
    })
}

fn compact_value<T: DeserializeOwned>(
    items: &[serde_json::Value],
    index: usize,
    label: &str,
) -> Result<T, String> {
    serde_json::from_value(items[index].clone())
        .map_err(|err| format!("invalid compact run-control checkpoint {label}: {err}"))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RunControlSessionCheckpointLegacyV1 {
    engine_state: EngineState,
    run_state: RunStateCheckpointV1,
    #[serde(default)]
    active_combat: Option<ActiveCombat>,
    decision_step: u64,
    #[serde(default)]
    reward_automation: RewardAutomationConfig,
    #[serde(default)]
    shop_visit_context: Option<ShopVisitContextV1>,
    #[serde(default)]
    auto_capture: AutoCombatCaptureConfig,
    #[serde(default)]
    search_max_nodes: Option<usize>,
    #[serde(default)]
    search_wall_ms: Option<u64>,
    #[serde(default)]
    search_max_hp_loss: Option<u32>,
    #[serde(default)]
    search_potion_policy: Option<CombatSearchV2PotionPolicy>,
    #[serde(default)]
    search_max_potions_used: Option<u32>,
    #[serde(default)]
    card_reward_outcome_calibration: Option<CardRewardOutcomeCalibrationV1>,
    #[serde(default)]
    card_reward_route_risk_calibration: Option<CardRewardRouteRiskCalibrationV1>,
    #[serde(default)]
    card_reward_strategy_package_calibration: Option<CardRewardStrategyPackageCalibrationV1>,
    #[serde(default)]
    combat_outcomes: CombatOutcomeTracker,
    combat_sequence: u64,
    #[serde(default)]
    auto_capture_last_combat_sequence: Option<u64>,
    #[serde(default)]
    last_completed_combat_sequence: Option<u64>,
    #[serde(default)]
    last_completed_combat_source: Option<CombatCompletionSource>,
    #[serde(default)]
    current_combat_source: Option<CombatCompletionSource>,
    #[serde(default)]
    last_combat_automation_sequence: Option<u64>,
    #[serde(default)]
    last_combat_automation_trajectory: Option<CombatAutomationTrajectoryRecordV1>,
    #[serde(default)]
    last_capture_case: Option<LastBenchmarkCaptureCase>,
}

impl RunControlSessionCheckpointLegacyV1 {
    fn into_checkpoint(self) -> RunControlSessionCheckpointV1 {
        RunControlSessionCheckpointV1 {
            engine_state: self.engine_state,
            run_state: self.run_state,
            active_combat: self.active_combat,
            decision_step: self.decision_step,
            reward_automation: self.reward_automation,
            shop_visit_context: self.shop_visit_context,
            auto_capture: self.auto_capture,
            search_max_nodes: self.search_max_nodes,
            search_wall_ms: self.search_wall_ms,
            search_max_hp_loss: self.search_max_hp_loss,
            search_potion_policy: self.search_potion_policy,
            search_max_potions_used: self.search_max_potions_used,
            card_reward_outcome_calibration: self.card_reward_outcome_calibration,
            card_reward_route_risk_calibration: self.card_reward_route_risk_calibration,
            card_reward_strategy_package_calibration: self.card_reward_strategy_package_calibration,
            combat_outcomes: self.combat_outcomes,
            combat_sequence: self.combat_sequence,
            auto_capture_last_combat_sequence: self.auto_capture_last_combat_sequence,
            last_completed_combat_sequence: self.last_completed_combat_sequence,
            last_completed_combat_source: self.last_completed_combat_source,
            current_combat_source: self.current_combat_source,
            last_combat_automation_sequence: self.last_combat_automation_sequence,
            last_combat_automation_trajectory: self.last_combat_automation_trajectory,
            last_capture_case: self.last_capture_case,
        }
    }
}

fn reward_automation_config_is_default(value: &RewardAutomationConfig) -> bool {
    value == &RewardAutomationConfig::default()
}

fn auto_capture_config_is_default(value: &AutoCombatCaptureConfig) -> bool {
    value == &AutoCombatCaptureConfig::default()
}

fn combat_outcome_tracker_is_default(value: &CombatOutcomeTracker) -> bool {
    value == &CombatOutcomeTracker::default()
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunControlDecisionParentSnapshotV1 {
    pub source: String,
    pub command: String,
    pub snapshot: RunControlSessionCheckpointV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunControlCommandOutcome {
    pub should_quit: bool,
    pub message: String,
    pub action_result: Option<ActionResult>,
    pub combat_search_rejection: Option<RunControlCombatSearchRejection>,
    pub auto_stop: Option<RunControlAutoStopV1>,
    pub auto_applied_steps: Vec<RunControlAutoAppliedStepV1>,
    pub trace_annotations: Vec<RunControlTraceAnnotationV1>,
    pub decision_parent_snapshots: Vec<RunControlDecisionParentSnapshotV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunControlCombatSearchRejection {
    InvalidCardIdentity,
    NoCompleteWinningCandidate,
    DirtyWinningCandidateRejected,
    HpLossLimitExceeded,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunControlAutoAppliedStepV1 {
    pub kind: RunControlAutoAppliedKindV1,
    pub label: String,
    pub action_result: Option<ActionResult>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunControlAutoAppliedKindV1 {
    RewardAutomation,
    CombatSearch,
    RoutePlanner,
    RewardOverlay,
    NoncombatPolicy,
    RoutineCandidate,
    AutoCapture,
    OwnerRoutine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunControlAutoStopV1 {
    pub kind: RunControlAutoStopKind,
    pub reason: String,
    pub applied_operations: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunControlAutoStopKind {
    RepeatedBoundary,
    HpLossGateRequired,
    CombatSearchNoCompleteWin,
    RoutePlannerNoMutation,
    RoutePlannerDeclined,
    NoncombatPolicyStop,
    BranchExperimentBoundary,
    AutoCandidateNotExecutable,
    HumanBoundary,
    OperationBudgetExhausted,
}

impl RunControlCommandOutcome {
    pub(in crate::eval::run_control) fn message(message: impl Into<String>) -> Self {
        Self {
            should_quit: false,
            message: message.into(),
            action_result: None,
            combat_search_rejection: None,
            auto_stop: None,
            auto_applied_steps: Vec::new(),
            trace_annotations: Vec::new(),
            decision_parent_snapshots: Vec::new(),
        }
    }

    fn quit(message: impl Into<String>) -> Self {
        Self {
            should_quit: true,
            message: message.into(),
            action_result: None,
            combat_search_rejection: None,
            auto_stop: None,
            auto_applied_steps: Vec::new(),
            trace_annotations: Vec::new(),
            decision_parent_snapshots: Vec::new(),
        }
    }

    pub(in crate::eval::run_control) fn action(
        message: impl Into<String>,
        action_result: ActionResult,
    ) -> Self {
        Self {
            should_quit: false,
            message: message.into(),
            action_result: Some(action_result),
            combat_search_rejection: None,
            auto_stop: None,
            auto_applied_steps: Vec::new(),
            trace_annotations: Vec::new(),
            decision_parent_snapshots: Vec::new(),
        }
    }

    pub(in crate::eval::run_control) fn with_auto_applied_steps(
        mut self,
        auto_applied_steps: Vec<RunControlAutoAppliedStepV1>,
    ) -> Self {
        self.auto_applied_steps.extend(auto_applied_steps);
        self
    }

    pub(in crate::eval::run_control) fn with_trace_annotations(
        mut self,
        trace_annotations: Vec<RunControlTraceAnnotationV1>,
    ) -> Self {
        self.trace_annotations.extend(trace_annotations);
        self
    }

    pub(in crate::eval::run_control) fn with_decision_parent_snapshots(
        mut self,
        snapshots: Vec<RunControlDecisionParentSnapshotV1>,
    ) -> Self {
        self.decision_parent_snapshots.extend(snapshots);
        self
    }

    pub(in crate::eval::run_control) fn with_auto_stop(
        mut self,
        auto_stop: RunControlAutoStopV1,
    ) -> Self {
        self.auto_stop = Some(auto_stop);
        self
    }

    pub(in crate::eval::run_control) fn with_combat_search_rejection(
        mut self,
        rejection: RunControlCombatSearchRejection,
    ) -> Self {
        self.combat_search_rejection = Some(rejection);
        self
    }
}

impl RunControlSession {
    pub fn new(config: RunControlConfig) -> Self {
        let run_state = RunState::new(
            config.seed,
            config.ascension_level,
            config.final_act,
            config.player_class,
        );
        let engine_state = EngineState::EventRoom;

        Self {
            engine_state,
            run_state,
            active_combat: None,
            decision_step: 0,
            reward_automation: config.reward_automation,
            shop_visit_context: None,
            auto_capture: config.auto_capture,
            search_max_nodes: config.search_max_nodes,
            search_wall_ms: config.search_wall_ms,
            search_max_hp_loss: config.search_max_hp_loss,
            search_potion_policy: config.search_potion_policy,
            search_max_potions_used: config.search_max_potions_used,
            card_reward_outcome_calibration: config.card_reward_outcome_calibration,
            card_reward_route_risk_calibration: config.card_reward_route_risk_calibration,
            card_reward_strategy_package_calibration: config
                .card_reward_strategy_package_calibration,
            combat_outcomes: CombatOutcomeTracker::default(),
            combat_sequence: 0,
            auto_capture_last_combat_sequence: None,
            last_completed_combat_sequence: None,
            last_completed_combat_source: None,
            current_combat_source: None,
            last_combat_automation_sequence: None,
            last_combat_automation_trajectory: None,
            last_capture_case: None,
        }
    }

    pub fn set_auto_capture_config(&mut self, auto_capture: AutoCombatCaptureConfig) {
        self.auto_capture = auto_capture;
        self.auto_capture_last_combat_sequence = None;
    }

    pub fn shop_visit_context(&self) -> Option<ShopVisitContextV1> {
        self.shop_visit_context
            .filter(|context| context.matches_run_state(&self.run_state))
    }

    pub(in crate::eval::run_control) fn observe_shop_visit_before_input(&mut self) {
        if engine_state_is_inside_shop_visit(&self.engine_state) {
            if !self
                .shop_visit_context
                .is_some_and(|context| context.matches_run_state(&self.run_state))
            {
                self.shop_visit_context = Some(ShopVisitContextV1::from_run_state(&self.run_state));
            }
        } else {
            self.shop_visit_context = None;
        }
    }

    pub(in crate::eval::run_control) fn observe_shop_visit_after_input(
        &mut self,
        gold_before: i32,
    ) {
        if engine_state_is_inside_shop_visit(&self.engine_state) {
            if !self
                .shop_visit_context
                .is_some_and(|context| context.matches_run_state(&self.run_state))
            {
                self.shop_visit_context = Some(ShopVisitContextV1::from_run_state(&self.run_state));
            }
            if self.run_state.gold < gold_before {
                if let Some(context) = self.shop_visit_context.as_mut() {
                    context.spent_gold_in_visit = true;
                }
            }
        } else {
            self.shop_visit_context = None;
        }
    }
}

fn engine_state_is_inside_shop_visit(state: &EngineState) -> bool {
    match state {
        EngineState::Shop(_) => true,
        EngineState::RewardOverlay { return_state, .. }
        | EngineState::MapOverlay { return_state } => {
            engine_state_is_inside_shop_visit(return_state)
        }
        EngineState::RunPendingChoice(choice) => {
            engine_state_is_inside_shop_visit(&choice.return_state)
        }
        _ => false,
    }
}

impl RunControlSessionCheckpointV1 {
    pub fn from_session(session: &RunControlSession) -> Self {
        Self {
            engine_state: session.engine_state.clone(),
            run_state: RunStateCheckpointV1::from_run_state(&session.run_state),
            active_combat: session.active_combat.clone(),
            decision_step: session.decision_step,
            reward_automation: session.reward_automation.clone(),
            shop_visit_context: session.shop_visit_context,
            auto_capture: session.auto_capture.clone(),
            search_max_nodes: session.search_max_nodes,
            search_wall_ms: session.search_wall_ms,
            search_max_hp_loss: session.search_max_hp_loss,
            search_potion_policy: session.search_potion_policy,
            search_max_potions_used: session.search_max_potions_used,
            card_reward_outcome_calibration: session.card_reward_outcome_calibration.clone(),
            card_reward_route_risk_calibration: session.card_reward_route_risk_calibration.clone(),
            card_reward_strategy_package_calibration: session
                .card_reward_strategy_package_calibration
                .clone(),
            combat_outcomes: session.combat_outcomes.clone(),
            combat_sequence: session.combat_sequence,
            auto_capture_last_combat_sequence: session.auto_capture_last_combat_sequence,
            last_completed_combat_sequence: session.last_completed_combat_sequence,
            last_completed_combat_source: session.last_completed_combat_source,
            current_combat_source: session.current_combat_source,
            last_combat_automation_sequence: session.last_combat_automation_sequence,
            last_combat_automation_trajectory: session.last_combat_automation_trajectory.clone(),
            last_capture_case: session.last_capture_case.clone(),
        }
    }

    pub fn last_combat_automation_trajectory_record(
        &self,
    ) -> Option<&CombatAutomationTrajectoryRecordV1> {
        self.last_combat_automation_trajectory.as_ref()
    }

    pub fn take_last_combat_automation_trajectory_record(
        &mut self,
    ) -> Option<CombatAutomationTrajectoryRecordV1> {
        self.last_combat_automation_trajectory.take()
    }

    pub fn restore_last_combat_automation_trajectory_record(
        &mut self,
        record: CombatAutomationTrajectoryRecordV1,
    ) {
        self.last_combat_automation_trajectory = Some(record);
    }

    pub fn take_run_state_map_for_external_ref(&mut self) -> MapState {
        std::mem::take(&mut self.run_state.map)
    }

    pub fn restore_run_state_map_from_external_ref(&mut self, map: MapState) {
        self.run_state.map = map;
    }

    pub fn take_run_state_master_deck_for_external_ref(&mut self) -> Vec<CombatCard> {
        std::mem::take(&mut self.run_state.master_deck)
    }

    pub fn restore_run_state_master_deck_from_external_ref(
        &mut self,
        master_deck: Vec<CombatCard>,
    ) {
        self.run_state.master_deck = master_deck;
    }

    pub fn take_run_state_relics_for_external_ref(
        &mut self,
    ) -> Vec<crate::content::relics::RelicState> {
        std::mem::take(&mut self.run_state.relics)
    }

    pub fn restore_run_state_relics_from_external_ref(
        &mut self,
        relics: Vec<crate::content::relics::RelicState>,
    ) {
        self.run_state.relics = relics;
    }

    pub fn take_run_state_potions_for_external_ref(
        &mut self,
    ) -> Vec<Option<crate::content::potions::Potion>> {
        std::mem::take(&mut self.run_state.potions)
    }

    pub fn restore_run_state_potions_from_external_ref(
        &mut self,
        potions: Vec<Option<crate::content::potions::Potion>>,
    ) {
        self.run_state.potions = potions;
    }

    pub fn take_run_state_schedule_for_external_ref(&mut self) -> RunStateScheduleCheckpointV1 {
        self.run_state.take_schedule_for_external_ref()
    }

    pub fn restore_run_state_schedule_from_external_ref(
        &mut self,
        schedule: RunStateScheduleCheckpointV1,
    ) {
        self.run_state.restore_schedule_from_external_ref(schedule);
    }

    pub fn take_run_state_emitted_events_for_external_ref(&mut self) -> Vec<DomainEvent> {
        self.run_state.take_emitted_events_for_external_ref()
    }

    pub fn restore_run_state_emitted_events_from_external_ref(
        &mut self,
        emitted_events: Vec<DomainEvent>,
    ) {
        self.run_state
            .restore_emitted_events_from_external_ref(emitted_events);
    }

    pub fn clear_combat_diagnostics_for_external_checkpoint(&mut self) {
        self.combat_outcomes = CombatOutcomeTracker::default();
        self.last_completed_combat_sequence = None;
        self.last_completed_combat_source = None;
        self.current_combat_source = None;
        self.last_combat_automation_sequence = None;
        self.last_combat_automation_trajectory = None;
        self.last_capture_case = None;
    }

    pub fn take_active_combat_for_external_ref(&mut self) -> Option<ActiveCombat> {
        self.active_combat.take()
    }

    pub fn restore_active_combat_from_external_ref(&mut self, active_combat: ActiveCombat) {
        self.active_combat = Some(active_combat);
    }

    pub fn into_session(self) -> Result<RunControlSession, String> {
        Ok(RunControlSession {
            engine_state: self.engine_state,
            run_state: self.run_state.into_run_state()?,
            active_combat: self.active_combat,
            decision_step: self.decision_step,
            reward_automation: self.reward_automation,
            shop_visit_context: self.shop_visit_context,
            auto_capture: self.auto_capture,
            search_max_nodes: self.search_max_nodes,
            search_wall_ms: self.search_wall_ms,
            search_max_hp_loss: self.search_max_hp_loss,
            search_potion_policy: self.search_potion_policy,
            search_max_potions_used: self.search_max_potions_used,
            card_reward_outcome_calibration: self.card_reward_outcome_calibration,
            card_reward_route_risk_calibration: self.card_reward_route_risk_calibration,
            card_reward_strategy_package_calibration: self.card_reward_strategy_package_calibration,
            combat_outcomes: self.combat_outcomes,
            combat_sequence: self.combat_sequence,
            auto_capture_last_combat_sequence: self.auto_capture_last_combat_sequence,
            last_completed_combat_sequence: self.last_completed_combat_sequence,
            last_completed_combat_source: self.last_completed_combat_source,
            current_combat_source: self.current_combat_source,
            last_combat_automation_sequence: self.last_combat_automation_sequence,
            last_combat_automation_trajectory: self.last_combat_automation_trajectory,
            last_capture_case: self.last_capture_case,
        })
    }
}

pub fn canonical_player_class(raw: &str) -> Result<&'static str, String> {
    match raw.to_ascii_lowercase().as_str() {
        "ironclad" | "red" => Ok("Ironclad"),
        "silent" | "green" => Ok("Silent"),
        "defect" | "blue" => Ok("Defect"),
        "watcher" | "purple" => Ok("Watcher"),
        _ => Err(format!("unsupported player class '{raw}'")),
    }
}

#[cfg(test)]
mod tests;
