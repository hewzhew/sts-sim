use std::path::PathBuf;

use crate::ai::combat_search_v2::CombatSearchV2PotionPolicy;
use crate::eval::card_reward_value_loop::{
    CardRewardOutcomeCalibrationV1, CardRewardRouteRiskCalibrationV1,
};
use crate::state::core::{ActiveCombat, EngineState};
use crate::state::run::RunState;

use super::auto_capture::AutoCombatCaptureConfig;
use super::outcome::CombatOutcomeTracker;
use super::reward_auto::RewardAutomationConfig;
use super::trace_annotation::RunControlTraceAnnotationV1;
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
    pub(super) combat_outcomes: CombatOutcomeTracker,
    pub(in crate::eval::run_control) combat_sequence: u64,
    pub(in crate::eval::run_control) auto_capture_last_combat_sequence: Option<u64>,
    last_completed_combat_sequence: Option<u64>,
    last_completed_combat_source: Option<CombatCompletionSource>,
    current_combat_source: Option<CombatCompletionSource>,
    last_capture_case: Option<LastBenchmarkCaptureCase>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::eval::run_control) struct LastBenchmarkCaptureCase {
    pub root: PathBuf,
    pub case_id: String,
    pub combat_sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::eval::run_control) enum CombatCompletionSource {
    Manual,
    SearchCombat,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunControlCommandOutcome {
    pub should_quit: bool,
    pub message: String,
    pub action_result: Option<ActionResult>,
    pub search_evidence_path: Option<PathBuf>,
    pub trace_annotations: Vec<RunControlTraceAnnotationV1>,
}

impl RunControlCommandOutcome {
    pub(in crate::eval::run_control) fn message(message: impl Into<String>) -> Self {
        Self {
            should_quit: false,
            message: message.into(),
            action_result: None,
            search_evidence_path: None,
            trace_annotations: Vec::new(),
        }
    }

    fn quit(message: impl Into<String>) -> Self {
        Self {
            should_quit: true,
            message: message.into(),
            action_result: None,
            search_evidence_path: None,
            trace_annotations: Vec::new(),
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
            search_evidence_path: None,
            trace_annotations: Vec::new(),
        }
    }

    pub(in crate::eval::run_control) fn with_trace_annotations(
        mut self,
        trace_annotations: Vec<RunControlTraceAnnotationV1>,
    ) -> Self {
        self.trace_annotations.extend(trace_annotations);
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
            auto_capture: config.auto_capture,
            search_max_nodes: config.search_max_nodes,
            search_wall_ms: config.search_wall_ms,
            search_max_hp_loss: config.search_max_hp_loss,
            search_potion_policy: config.search_potion_policy,
            search_max_potions_used: config.search_max_potions_used,
            card_reward_outcome_calibration: config.card_reward_outcome_calibration,
            card_reward_route_risk_calibration: config.card_reward_route_risk_calibration,
            combat_outcomes: CombatOutcomeTracker::default(),
            combat_sequence: 0,
            auto_capture_last_combat_sequence: None,
            last_completed_combat_sequence: None,
            last_completed_combat_source: None,
            current_combat_source: None,
            last_capture_case: None,
        }
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
