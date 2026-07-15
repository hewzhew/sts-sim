use crate::engine::run_loop::tick_run_active_with_observer;
use crate::state::core::{ClientInput, EngineState, RunResult};

use super::{CombatCompletionSource, RunControlSession, RunProgressOutcome};
use crate::eval::run_control::auto_capture::render_auto_capture_result;
use crate::eval::run_control::render::render_run_control_state;
use crate::eval::run_control::trace_annotation::RunControlTraceAnnotationV1;
use crate::eval::run_control::transition_report::{
    action_result_changes_from_domain_events, action_result_from_transition_with_extra_changes,
    render_action_result, transition_action_for_input, RunApplyStatus, RunVisibleSnapshot,
};
use crate::eval::run_control::RunDecisionAction;
use crate::eval::run_control::{RunControlAutoStepOptions, RunControlSearchCombatOptions};

const MAX_STABLE_ADVANCE_TICKS: usize = 2_000;

impl RunControlSession {
    pub fn apply_decision_action(
        &mut self,
        action: RunDecisionAction,
    ) -> Result<RunProgressOutcome, String> {
        match action {
            RunDecisionAction::Input(input) => self.apply_input(input),
            RunDecisionAction::SkipCardReward { reward_item_index } => {
                self.apply_branch_skip_card_reward(reward_item_index)
            }
            RunDecisionAction::SingingBowlCardReward { reward_item_index } => {
                super::super::card_reward_auto::apply_singing_bowl_to_visible_card_reward_item(
                    self,
                    reward_item_index,
                )
            }
        }
    }

    pub fn apply_candidate_id(&mut self, candidate_id: &str) -> Result<RunProgressOutcome, String> {
        let surface = super::super::decision_surface::build_decision_surface(self);
        let candidate = super::super::decision_surface::resolve_surface_candidate(
            &surface,
            &self.engine_state,
            candidate_id,
        )
        .ok_or_else(|| format!("no visible candidate '{candidate_id}'"))?;
        let action = candidate.action.executable_action().ok_or_else(|| {
            format!(
                "candidate '{candidate_id}' is not directly executable: {}",
                candidate.action.summary()
            )
        })?;
        self.apply_decision_action(action)
    }

    pub fn apply_only_candidate(&mut self) -> Result<RunProgressOutcome, String> {
        let surface = super::super::decision_surface::build_decision_surface(self);
        let [candidate] = surface.view.candidates.as_slice() else {
            return Err("exactly one visible candidate is required".to_string());
        };
        let candidate_id = candidate.id.clone();
        self.apply_candidate_id(&candidate_id)
    }

    pub fn apply_progress_step(
        &mut self,
        options: RunControlAutoStepOptions,
    ) -> Result<RunProgressOutcome, String> {
        super::super::auto_step::apply_guarded_auto_step(self, options)
    }

    pub fn apply_combat_search(
        &mut self,
        options: RunControlSearchCombatOptions,
    ) -> Result<RunProgressOutcome, String> {
        super::super::combat_search::apply_search_combat(self, options)
    }

    pub fn apply_route_plan(&mut self) -> Result<RunProgressOutcome, String> {
        super::super::route_policy::apply_route_plan(self)
    }

    fn apply_branch_skip_card_reward(
        &mut self,
        reward_index: usize,
    ) -> Result<RunProgressOutcome, String> {
        let next_state = {
            let EngineState::RewardScreen(reward) = &mut self.engine_state else {
                return Err("branch-skip-card-reward is only valid on a reward screen".to_string());
            };
            crate::engine::reward_handler::skip_card_reward_item(
                &mut self.run_state,
                reward,
                reward_index,
            )?
        };

        if let Some(next_state) = next_state {
            self.engine_state = next_state;
        }
        let reward_automation = super::super::reward_auto::apply_reward_automation(self)?;
        let message = if reward_automation.is_empty() {
            format!("Branch skipped card reward at item {reward_index}")
        } else {
            format!(
                "Branch skipped card reward at item {reward_index}\n{}",
                reward_automation.render()
            )
        };
        Ok(RunProgressOutcome::message(message)
            .with_trace_annotations(reward_automation.trace_annotations))
    }

    pub(in crate::eval::run_control) fn apply_input(
        &mut self,
        input: ClientInput,
    ) -> Result<RunProgressOutcome, String> {
        self.apply_input_inner(input, true)
    }

    pub(in crate::eval::run_control) fn apply_input_without_manual_card_reward_trace(
        &mut self,
        input: ClientInput,
    ) -> Result<RunProgressOutcome, String> {
        self.apply_input_inner(input, false)
    }

    fn apply_input_inner(
        &mut self,
        input: ClientInput,
        trace_manual_card_reward_selection: bool,
    ) -> Result<RunProgressOutcome, String> {
        self.ensure_combat_started_if_needed()?;
        self.validate_input_for_current_state(&input)?;
        let manual_card_reward_annotation = if trace_manual_card_reward_selection {
            match &input {
                ClientInput::SelectCard(index) => {
                    super::super::card_reward_auto::manual_card_reward_selection_annotation(
                        self, *index,
                    )?
                }
                _ => None,
            }
        } else {
            None
        };
        self.observe_shop_visit_before_input();
        let gold_before_input = self.run_state.gold;
        let before_snapshot = RunVisibleSnapshot::capture(self);
        let action_report = transition_action_for_input(self, &input);
        self.observe_active_combat_started();
        let potion_observation = self.combat_outcomes.observe_input_before(
            self.active_combat
                .as_ref()
                .map(|active| &active.combat_state),
            &input,
        );
        let mut tick = tick_run_active_with_observer(
            &mut self.engine_state,
            &mut self.run_state,
            &mut self.active_combat,
            Some(input),
        );
        let mut finished_combat = tick.finished_combat.take();
        let mut advance_ticks = 0usize;
        while tick.keep_running && matches!(self.engine_state, EngineState::CombatProcessing) {
            if advance_ticks >= MAX_STABLE_ADVANCE_TICKS {
                return Err(format!(
                    "run-control exceeded {MAX_STABLE_ADVANCE_TICKS} engine ticks while advancing to a stable boundary"
                ));
            }
            advance_ticks += 1;
            tick = tick_run_active_with_observer(
                &mut self.engine_state,
                &mut self.run_state,
                &mut self.active_combat,
                None,
            );
            if finished_combat.is_none() {
                finished_combat = tick.finished_combat.take();
            }
        }
        self.collapse_completed_event_room();
        self.observe_shop_visit_after_input(gold_before_input);
        let after_combat = finished_combat
            .as_ref()
            .map(|finished| &finished.combat_state)
            .or_else(|| {
                self.active_combat
                    .as_ref()
                    .map(|active| &active.combat_state)
            });
        self.combat_outcomes
            .observe_input_after(potion_observation, after_combat);
        let combat_observation_changes = if let Some(finished) = finished_combat.as_mut() {
            action_result_changes_from_domain_events(finished.combat_state.take_emitted_events())
        } else if let Some(active) = self.active_combat.as_mut() {
            action_result_changes_from_domain_events(active.combat_state.take_emitted_events())
        } else {
            Vec::new()
        };
        if let Some(finished) = finished_combat.as_ref() {
            self.combat_outcomes.finish("last_combat", finished);
            self.last_completed_combat_sequence = Some(self.combat_sequence);
            self.last_completed_combat_source = Some(
                self.current_combat_source
                    .take()
                    .unwrap_or(CombatCompletionSource::Manual),
            );
        }
        self.cleanup_inactive_combat();
        self.ensure_combat_started_if_needed()?;
        let reward_automation = super::super::reward_auto::apply_reward_automation(self)?;
        self.cleanup_inactive_combat();
        self.ensure_combat_started_if_needed()?;
        self.observe_active_combat_started();
        let auto_capture = super::super::auto_capture::maybe_auto_capture_combat_start(self)?;
        let mut trace_annotations = Vec::new();
        if let Some(annotation) = manual_card_reward_annotation {
            trace_annotations.push(annotation);
        }
        trace_annotations.extend(reward_automation.trace_annotations.clone());
        self.decision_step = self.decision_step.saturating_add(1);

        let status = if tick.keep_running {
            RunApplyStatus::Running
        } else {
            match self.engine_state {
                EngineState::GameOver(RunResult::Victory) => RunApplyStatus::Victory,
                EngineState::GameOver(RunResult::Defeat) => RunApplyStatus::Defeat,
                _ => RunApplyStatus::Stopped,
            }
        };
        let after_snapshot = RunVisibleSnapshot::capture(self);
        let action_result = action_result_from_transition_with_extra_changes(
            action_report,
            &before_snapshot,
            &after_snapshot,
            status,
            combat_observation_changes,
        );
        let report = render_action_result(&action_result);
        let report = if reward_automation.is_empty() {
            report
        } else {
            format!("{}\n{report}", reward_automation.render())
        };
        let report = if let Some(auto_capture) = auto_capture.as_ref() {
            format!("{report}\n{}", render_auto_capture_result(auto_capture))
        } else {
            report
        };
        if let Some(auto_capture) = auto_capture.as_ref() {
            trace_annotations.push(RunControlTraceAnnotationV1::AutoCombatCapture {
                case_id: auto_capture.case_id.clone(),
                capture_path: auto_capture.capture_path.display().to_string(),
                benchmark_manifest_path: auto_capture.benchmark_manifest.display().to_string(),
                label_role: "diagnostic_capture_not_human_baseline".to_string(),
            });
        }
        Ok(RunProgressOutcome::action(
            format!("{report}\n{}", render_run_control_state(self)),
            action_result,
        )
        .with_trace_annotations(trace_annotations))
    }

    fn collapse_completed_event_room(&mut self) {
        if !matches!(self.engine_state, EngineState::EventRoom) {
            return;
        }
        let Some(event_state) = self.run_state.event_state.as_ref() else {
            return;
        };
        if event_state.completed && !event_state.combat_pending {
            self.run_state.event_state = None;
            self.engine_state = EngineState::MapNavigation;
        }
    }
}
