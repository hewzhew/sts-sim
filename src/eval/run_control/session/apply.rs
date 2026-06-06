use crate::engine::run_loop::tick_run_active_with_observer;
use crate::state::core::{ClientInput, EngineState, RunResult};

use super::{CombatCompletionSource, RunControlCommandOutcome, RunControlSession};
use crate::eval::run_control::auto_capture::render_auto_capture_result;
use crate::eval::run_control::commands::{run_control_help, RunControlCommand};
use crate::eval::run_control::panels::{
    render_combat_zone_panel, render_deck_panel, render_full_map_panel, render_inspect_panel,
    render_map_panel, render_potions_panel, render_relics_panel, render_route_summary_panel,
    CombatZonePanel,
};
use crate::eval::run_control::render::{
    render_combat_actions, render_run_control_details, render_run_control_raw,
    render_run_control_state,
};
use crate::eval::run_control::reward_auto::set_reward_automation;
use crate::eval::run_control::trace_annotation::RunControlTraceAnnotationV1;
use crate::eval::run_control::transition_report::{
    action_result_from_transition, render_action_result, transition_action_for_input,
    RunApplyStatus, RunVisibleSnapshot,
};

const MAX_STABLE_ADVANCE_TICKS: usize = 2_000;

impl RunControlSession {
    pub fn apply_command(
        &mut self,
        command: RunControlCommand,
    ) -> Result<RunControlCommandOutcome, String> {
        self.ensure_combat_started_if_needed()?;

        match command {
            RunControlCommand::Noop => Ok(RunControlCommandOutcome::message("")),
            RunControlCommand::DefaultCandidate => self.apply_default_candidate(),
            RunControlCommand::Candidate(id) => self.apply_visible_candidate(&id),
            RunControlCommand::Help => Ok(RunControlCommandOutcome::message(run_control_help())),
            RunControlCommand::Quit => Ok(RunControlCommandOutcome::quit("quit")),
            RunControlCommand::Main => Ok(RunControlCommandOutcome::message(
                render_run_control_state(self),
            )),
            RunControlCommand::Deck => {
                Ok(RunControlCommandOutcome::message(render_deck_panel(self)))
            }
            RunControlCommand::Map => Ok(RunControlCommandOutcome::message(render_map_panel(self))),
            RunControlCommand::MapFull => Ok(RunControlCommandOutcome::message(
                render_full_map_panel(self),
            )),
            RunControlCommand::MapSummary => Ok(RunControlCommandOutcome::message(
                render_route_summary_panel(self),
            )),
            RunControlCommand::BoundaryRecord => Ok(RunControlCommandOutcome::message(
                super::super::noncombat_boundary::render_current_noncombat_boundary_record(self),
            )),
            RunControlCommand::RouteSuggest => Ok(RunControlCommandOutcome::message(
                super::super::route_policy::render_route_suggestion(self),
            )),
            RunControlCommand::RouteGo => super::super::route_policy::apply_route_go(self),
            RunControlCommand::Relics => {
                Ok(RunControlCommandOutcome::message(render_relics_panel(self)))
            }
            RunControlCommand::Potions => Ok(RunControlCommandOutcome::message(
                render_potions_panel(self),
            )),
            RunControlCommand::Draw => Ok(RunControlCommandOutcome::message(
                render_combat_zone_panel(self, CombatZonePanel::Draw),
            )),
            RunControlCommand::Discard => Ok(RunControlCommandOutcome::message(
                render_combat_zone_panel(self, CombatZonePanel::Discard),
            )),
            RunControlCommand::Exhaust => Ok(RunControlCommandOutcome::message(
                render_combat_zone_panel(self, CombatZonePanel::Exhaust),
            )),
            RunControlCommand::Inspect(id) => Ok(RunControlCommandOutcome::message(
                render_inspect_panel(self, &id),
            )),
            RunControlCommand::SaveDecisionCase { path } => {
                super::super::artifact_commands::apply_save_decision_case(self, path)
            }
            RunControlCommand::Details => Ok(RunControlCommandOutcome::message(
                render_run_control_details(self),
            )),
            RunControlCommand::Raw => Ok(RunControlCommandOutcome::message(
                render_run_control_raw(self),
            )),
            RunControlCommand::Actions => Ok(RunControlCommandOutcome::message(
                render_combat_actions(self)?,
            )),
            RunControlCommand::Capture { path, label } => {
                super::super::artifact_commands::apply_capture(self, path, label)
            }
            RunControlCommand::CaptureCase {
                root,
                case_id,
                label,
            } => super::super::artifact_commands::apply_capture_case(self, root, case_id, label),
            RunControlCommand::CaptureCaseDefault { case_id, label } => {
                super::super::artifact_commands::apply_default_capture_case(self, case_id, label)
            }
            RunControlCommand::SaveBaseline { path, case_id } => {
                super::super::artifact_commands::apply_save_baseline(self, path, case_id)
            }
            RunControlCommand::SaveBaselineCase { root, case_id } => {
                super::super::artifact_commands::apply_save_baseline_case(self, root, case_id)
            }
            RunControlCommand::SaveBaselineForLastCaptureCase => {
                super::super::artifact_commands::apply_save_baseline_for_last_capture_case(self)
            }
            RunControlCommand::RegisterBenchmarkCase { root, case_id } => {
                super::super::artifact_commands::apply_register_benchmark_case(root, case_id)
            }
            RunControlCommand::SearchDefaults(command) => {
                super::super::search_defaults::apply_search_defaults(self, command)
            }
            RunControlCommand::SearchCombat(options) => {
                super::super::combat_search::apply_search_combat(self, options)
            }
            RunControlCommand::AutoStep(options) => {
                super::super::auto_step::apply_guarded_auto_step(self, options)
            }
            RunControlCommand::AutoRun(options) => {
                super::super::auto_run::apply_auto_run(self, options)
            }
            RunControlCommand::RewardAutomationStatus => Ok(RunControlCommandOutcome::message(
                self.reward_automation.summary(),
            )),
            RunControlCommand::SetRewardAutomation { target, enabled } => {
                set_reward_automation(&mut self.reward_automation, target, enabled);
                Ok(RunControlCommandOutcome::message(
                    self.reward_automation.summary(),
                ))
            }
            RunControlCommand::RecordedCardRewardPick(index) => {
                super::super::card_reward_auto::apply_recorded_card_reward_pick(self, index)
            }
            RunControlCommand::CardIndex(index) => {
                if matches!(self.engine_state, EngineState::Shop(_)) {
                    self.apply_input(ClientInput::BuyCard(index))
                } else if matches!(
                    self.engine_state,
                    EngineState::RewardScreen(_) | EngineState::RewardOverlay { .. }
                ) {
                    self.apply_input(ClientInput::SelectCard(index))
                } else {
                    Err("card <idx> is only valid in shop or card reward screens".to_string())
                }
            }
            RunControlCommand::RelicIndex(index) => {
                if matches!(self.engine_state, EngineState::Shop(_)) {
                    self.apply_input(ClientInput::BuyRelic(index))
                } else if matches!(self.engine_state, EngineState::BossRelicSelect(_)) {
                    self.apply_input(ClientInput::SubmitRelicChoice(index))
                } else {
                    Err("relic <idx> is only valid in shop or boss relic screens".to_string())
                }
            }
            RunControlCommand::SelectionIndices(indices) => {
                let input =
                    super::super::selection_surface::resolve_selection_indices(self, indices)?;
                self.apply_input(input)
            }
            RunControlCommand::ActionIndex(index) => {
                let input = self.combat_action_by_index(index)?;
                self.apply_input(input)
            }
            RunControlCommand::PlayCard {
                card_index,
                target_slot_or_id,
            } => {
                let target = self.resolve_target(target_slot_or_id)?;
                self.apply_input(ClientInput::PlayCard { card_index, target })
            }
            RunControlCommand::UsePotion {
                potion_index,
                target_slot_or_id,
            } => {
                if matches!(self.engine_state, EngineState::Shop(_)) && target_slot_or_id.is_none()
                {
                    self.apply_input(ClientInput::BuyPotion(potion_index))
                } else {
                    let target = self.resolve_target(target_slot_or_id)?;
                    self.apply_input(ClientInput::UsePotion {
                        potion_index,
                        target,
                    })
                }
            }
            RunControlCommand::Input(input) => self.apply_input(input),
        }
    }

    fn apply_default_candidate(&mut self) -> Result<RunControlCommandOutcome, String> {
        let surface = super::super::decision_surface::build_decision_surface(self);
        if surface.view.candidates.len() != 1 {
            return Err(
                "Enter only executes when exactly one visible action is available; choose an id"
                    .to_string(),
            );
        }
        let id = surface.view.candidates[0].id.clone();
        self.apply_visible_candidate(&id)
    }

    fn apply_visible_candidate(&mut self, id: &str) -> Result<RunControlCommandOutcome, String> {
        let surface = super::super::decision_surface::build_decision_surface(self);
        let candidate = super::super::decision_surface::resolve_surface_candidate(
            &surface,
            &self.engine_state,
            id,
        )
        .ok_or_else(|| format!("no visible candidate '{id}'"))?;
        match candidate.action.executable_input() {
            Some(input) => self.apply_input(input),
            None => Err(format!(
                "candidate '{id}' is not directly executable: {}",
                candidate.action.command_hint()
            )),
        }
    }

    pub(in crate::eval::run_control) fn apply_input(
        &mut self,
        input: ClientInput,
    ) -> Result<RunControlCommandOutcome, String> {
        self.apply_input_inner(input, true)
    }

    pub(in crate::eval::run_control) fn apply_input_without_manual_card_reward_trace(
        &mut self,
        input: ClientInput,
    ) -> Result<RunControlCommandOutcome, String> {
        self.apply_input_inner(input, false)
    }

    fn apply_input_inner(
        &mut self,
        input: ClientInput,
        trace_manual_card_reward_selection: bool,
    ) -> Result<RunControlCommandOutcome, String> {
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
        let action_result =
            action_result_from_transition(action_report, &before_snapshot, &after_snapshot, status);
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
        Ok(RunControlCommandOutcome::action(
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
