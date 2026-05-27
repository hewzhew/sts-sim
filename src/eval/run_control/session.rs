use std::path::PathBuf;

use crate::content::potions::Potion;
use crate::engine::run_loop::tick_run_active_with_observer;
use crate::sim::combat::CombatPosition;
use crate::state::core::{ActiveCombat, ClientInput, EngineState, RunResult};
use crate::state::run::RunState;

use super::auto_capture::{render_auto_capture_result, AutoCombatCaptureConfig};
use super::combat_start::ensure_combat_started_if_needed;
use super::commands::{run_control_help, RunControlCommand};
use super::outcome::CombatOutcomeTracker;
use super::panels::{
    render_combat_zone_panel, render_deck_panel, render_full_map_panel, render_inspect_panel,
    render_map_panel, render_potions_panel, render_relics_panel, CombatZonePanel,
};
use super::render::{
    render_combat_actions, render_run_control_details, render_run_control_raw,
    render_run_control_state,
};
use super::reward_auto::{set_reward_automation, RewardAutomationConfig};
use super::trace_annotation::RunControlTraceAnnotationV1;
use super::transition_report::{
    action_result_from_transition, render_action_result, transition_action_for_input, ActionResult,
    RunApplyStatus, RunVisibleSnapshot,
};

const MAX_STABLE_ADVANCE_TICKS: usize = 2_000;

#[derive(Clone, Debug)]
pub struct RunControlConfig {
    pub seed: u64,
    pub ascension_level: u8,
    pub final_act: bool,
    pub player_class: &'static str,
    pub reward_automation: RewardAutomationConfig,
    pub auto_capture: AutoCombatCaptureConfig,
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
            combat_outcomes: CombatOutcomeTracker::default(),
            combat_sequence: 0,
            auto_capture_last_combat_sequence: None,
            last_completed_combat_sequence: None,
            last_completed_combat_source: None,
            current_combat_source: None,
            last_capture_case: None,
        }
    }

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
            RunControlCommand::RouteSuggest => Ok(RunControlCommandOutcome::message(
                super::route_policy::render_route_suggestion(self),
            )),
            RunControlCommand::RouteGo => super::route_policy::apply_route_go(self),
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
                super::artifact_commands::apply_save_decision_case(self, path)
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
                super::artifact_commands::apply_capture(self, path, label)
            }
            RunControlCommand::CaptureCase {
                root,
                case_id,
                label,
            } => super::artifact_commands::apply_capture_case(self, root, case_id, label),
            RunControlCommand::CaptureCaseDefault { case_id, label } => {
                super::artifact_commands::apply_default_capture_case(self, case_id, label)
            }
            RunControlCommand::SaveBaseline { path, case_id } => {
                super::artifact_commands::apply_save_baseline(self, path, case_id)
            }
            RunControlCommand::SaveBaselineCase { root, case_id } => {
                super::artifact_commands::apply_save_baseline_case(self, root, case_id)
            }
            RunControlCommand::SaveBaselineForLastCaptureCase => {
                super::artifact_commands::apply_save_baseline_for_last_capture_case(self)
            }
            RunControlCommand::RegisterBenchmarkCase { root, case_id } => {
                super::artifact_commands::apply_register_benchmark_case(root, case_id)
            }
            RunControlCommand::SearchCombat(options) => {
                super::combat_search::apply_search_combat(self, options)
            }
            RunControlCommand::AutoStep(options) => {
                super::auto_step::apply_guarded_auto_step(self, options)
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
            RunControlCommand::CardIndex(index) => {
                if matches!(self.engine_state, EngineState::Shop(_)) {
                    self.apply_input(ClientInput::BuyCard(index))
                } else if matches!(self.engine_state, EngineState::RewardScreen(_)) {
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
        let surface = super::decision_surface::build_decision_surface(self);
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
        let surface = super::decision_surface::build_decision_surface(self);
        let candidate =
            super::decision_surface::resolve_surface_candidate(&surface, &self.engine_state, id)
                .ok_or_else(|| format!("no visible candidate '{id}'"))?;
        match candidate.action.executable_input() {
            Some(input) => self.apply_input(input),
            None => Err(format!(
                "candidate '{id}' is not directly executable: {}",
                candidate.action.command_hint()
            )),
        }
    }

    pub(crate) fn current_active_combat_position(&self) -> Result<CombatPosition, String> {
        let combat = self
            .active_combat
            .as_ref()
            .map(|active| (&active.engine_state, &active.combat_state))
            .ok_or_else(|| "no active combat state to capture".to_string())?;
        match combat.0 {
            EngineState::CombatPlayerTurn | EngineState::PendingChoice(_) => {
                Ok(CombatPosition::new(combat.0.clone(), combat.1.clone()))
            }
            _ => Err(format!(
                "cannot capture combat from engine state {:?}",
                combat.0
            )),
        }
    }

    pub(crate) fn current_combat_position_for_actions(&self) -> Result<CombatPosition, String> {
        let active = self
            .active_combat
            .as_ref()
            .ok_or_else(|| "no active combat state".to_string())?;
        let engine = match &active.engine_state {
            EngineState::CombatPlayerTurn
            | EngineState::CombatProcessing
            | EngineState::PendingChoice(_) => active.engine_state.clone(),
            other => {
                return Err(format!(
                    "engine state {other:?} is not an active combat input state"
                ))
            }
        };
        Ok(CombatPosition::new(engine, active.combat_state.clone()))
    }

    pub(in crate::eval::run_control) fn apply_input(
        &mut self,
        input: ClientInput,
    ) -> Result<RunControlCommandOutcome, String> {
        self.ensure_combat_started_if_needed()?;
        self.validate_input_for_current_state(&input)?;
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
        let reward_automation = super::reward_auto::apply_reward_automation(self)?;
        self.cleanup_inactive_combat();
        self.ensure_combat_started_if_needed()?;
        self.observe_active_combat_started();
        let auto_capture = super::auto_capture::maybe_auto_capture_combat_start(self)?;
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
        let trace_annotations = auto_capture
            .as_ref()
            .map(|auto_capture| {
                vec![RunControlTraceAnnotationV1::AutoCombatCapture {
                    case_id: auto_capture.case_id.clone(),
                    capture_path: auto_capture.capture_path.display().to_string(),
                    benchmark_manifest_path: auto_capture.benchmark_manifest.display().to_string(),
                    label_role: "diagnostic_capture_not_human_baseline".to_string(),
                }]
            })
            .unwrap_or_default();
        Ok(RunControlCommandOutcome::action(
            format!("{report}\n{}", render_run_control_state(self)),
            action_result,
        )
        .with_trace_annotations(trace_annotations))
    }

    fn cleanup_inactive_combat(&mut self) {
        if !matches!(
            self.engine_state,
            EngineState::CombatPlayerTurn
                | EngineState::CombatProcessing
                | EngineState::PendingChoice(_)
        ) {
            self.active_combat = None;
        }
    }

    fn ensure_combat_started_if_needed(&mut self) -> Result<(), String> {
        ensure_combat_started_if_needed(
            &mut self.engine_state,
            &mut self.run_state,
            &mut self.active_combat,
        )
    }

    fn observe_active_combat_started(&mut self) {
        let started = self.combat_outcomes.ensure_started(
            self.active_combat
                .as_ref()
                .map(|active| &active.combat_state),
        );
        if started {
            self.combat_sequence = self.combat_sequence.saturating_add(1);
            self.current_combat_source = Some(CombatCompletionSource::Manual);
        }
    }

    pub(in crate::eval::run_control) fn remember_capture_case(
        &mut self,
        root: PathBuf,
        case_id: String,
    ) {
        self.observe_active_combat_started();
        self.last_capture_case = Some(LastBenchmarkCaptureCase {
            root,
            case_id,
            combat_sequence: self.combat_sequence,
        });
    }

    pub(in crate::eval::run_control) fn last_capture_case(
        &self,
    ) -> Option<&LastBenchmarkCaptureCase> {
        self.last_capture_case.as_ref()
    }

    pub(in crate::eval::run_control) fn active_capture_case(
        &self,
    ) -> Option<&LastBenchmarkCaptureCase> {
        let case = self.last_capture_case.as_ref()?;
        (self.active_combat.is_some() && case.combat_sequence == self.combat_sequence)
            .then_some(case)
    }

    pub(in crate::eval::run_control) fn last_completed_combat_matches_capture_case(&self) -> bool {
        let Some(case) = self.last_capture_case.as_ref() else {
            return false;
        };
        self.last_completed_combat_sequence == Some(case.combat_sequence)
    }

    pub(in crate::eval::run_control) fn last_completed_manual_combat_matches_capture_case(
        &self,
    ) -> bool {
        self.last_completed_combat_matches_capture_case()
            && self.last_completed_combat_source == Some(CombatCompletionSource::Manual)
    }

    pub(in crate::eval::run_control) fn last_completed_combat_source(
        &self,
    ) -> Option<CombatCompletionSource> {
        self.last_completed_combat_source
    }

    pub(in crate::eval::run_control) fn mark_current_combat_search_resolved(&mut self) {
        if self.active_combat.is_some() {
            self.current_combat_source = Some(CombatCompletionSource::SearchCombat);
        }
    }

    pub(in crate::eval::run_control) fn visible_potions(&self) -> &[Option<Potion>] {
        self.active_combat
            .as_ref()
            .map(|active| active.combat_state.entities.potions.as_slice())
            .unwrap_or(self.run_state.potions.as_slice())
    }

    pub(in crate::eval::run_control) fn visible_player_hp(&self) -> (i32, i32) {
        self.active_combat
            .as_ref()
            .map(|active| {
                (
                    active.combat_state.entities.player.current_hp,
                    active.combat_state.entities.player.max_hp,
                )
            })
            .unwrap_or((self.run_state.current_hp, self.run_state.max_hp))
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
mod tests {
    use super::*;
    use crate::content::monsters::factory::EncounterId;
    use crate::eval::run_control::decision_surface;
    use crate::eval::run_control::registry::BenchmarkCasePaths;
    use crate::eval::run_control::CombatBaselineOutcomeV1;
    use crate::state::map::node::{MapEdge, MapRoomNode, RoomType};
    use crate::state::map::state::MapState;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn run_control_capture_command_saves_active_combat_position() {
        let mut session = test_session_with_first_monster_room();
        session
            .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
            .expect("map input should enter combat");
        assert!(matches!(
            session.engine_state,
            EngineState::CombatPlayerTurn
        ));

        let dir = unique_temp_dir("run_control_capture");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let path = dir.join("capture.json");
        let outcome = session
            .apply_command(RunControlCommand::Capture {
                path: path.clone(),
                label: Some("first fight".to_string()),
            })
            .expect("capture command should save");

        assert!(outcome.message.contains("saved CombatCaptureV1"));
        let loaded = crate::eval::combat_capture::load_combat_capture_v1(&path)
            .expect("saved capture should load");
        assert_eq!(loaded.label.as_deref(), Some("first fight"));
        assert_eq!(
            loaded.provenance.source_kind,
            crate::eval::artifact::ArtifactSourceKind::ManualRunControl
        );
        assert_eq!(
            loaded.provenance.capture_method,
            "run_control_manual_capture"
        );
        assert_eq!(loaded.source.capture_method, "run_control_manual_capture");
        assert_eq!(
            loaded
                .provenance
                .run_config
                .as_ref()
                .and_then(|config| config.seed),
            Some(session.run_state.seed)
        );
        assert!(loaded.fingerprints.is_some());
        assert!(loaded.legal_actions.is_some());
        assert!(matches!(
            loaded.position.engine,
            EngineState::CombatPlayerTurn
        ));

        let _ = fs::remove_file(path);
        let _ = fs::remove_dir(dir);
    }

    #[test]
    fn run_control_capture_case_registers_benchmark_manifest() {
        let mut session = test_session_with_first_monster_room();
        session
            .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
            .expect("map input should enter combat");

        let root = unique_temp_dir("run_control_capture_case");
        let outcome = session
            .apply_command(RunControlCommand::CaptureCase {
                root: root.clone(),
                case_id: "first_fight".to_string(),
                label: Some("first fight".to_string()),
            })
            .expect("capture-case should save and register");

        assert!(outcome.message.contains("registered"));
        let paths = BenchmarkCasePaths::for_case(&root, "first_fight");
        assert!(paths.capture_path.exists());
        assert!(paths.benchmark_manifest.exists());
        let manifest = fs::read_to_string(&paths.benchmark_manifest).expect("manifest readable");
        assert!(manifest.contains("\"combat_snapshot\": \"captures/first_fight.capture.json\""));
        assert!(manifest.contains("\"expected_fingerprints\""));
        crate::eval::combat_search_v2::load_combat_search_v2_benchmark(&paths.benchmark_manifest)
            .expect("registered suite should validate through search benchmark loader");
        assert_eq!(
            session
                .last_capture_case()
                .map(|case| (case.root.clone(), case.case_id.clone())),
            Some((root.clone(), "first_fight".to_string()))
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn run_control_baseline_command_rejects_search_resolved_combat() {
        let mut session = test_session_with_first_monster_room();
        session
            .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
            .expect("map input should enter combat");

        let root = unique_temp_dir("run_control_baseline_last");
        session
            .apply_command(RunControlCommand::CaptureCase {
                root: root.clone(),
                case_id: "first_fight".to_string(),
                label: None,
            })
            .expect("capture-case should remember the case");
        session
            .apply_command(RunControlCommand::SearchCombat(
                crate::eval::run_control::RunControlSearchCombatOptions {
                    max_nodes: Some(2_000),
                    wall_ms: Some(5_000),
                    ..Default::default()
                },
            ))
            .expect("search-combat should finish the captured combat");
        assert!(session.last_completed_combat_matches_capture_case());
        assert!(!session.last_completed_manual_combat_matches_capture_case());

        let err = session
            .apply_command(RunControlCommand::SaveBaselineForLastCaptureCase)
            .expect_err("search-combat outcome should not save as human baseline");

        assert!(err.contains("resolved by search-combat"));
        let paths = BenchmarkCasePaths::for_case(&root, "first_fight");
        assert!(!paths.baseline_path.exists());
        let manifest = fs::read_to_string(&paths.benchmark_manifest).expect("manifest readable");
        assert!(!manifest.contains("\"baseline\""));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn run_control_search_combat_can_save_search_evidence_for_capture_case() {
        let mut session = test_session_with_first_monster_room();
        session
            .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
            .expect("map input should enter combat");

        let root = unique_temp_dir("run_control_search_evidence");
        session
            .apply_command(RunControlCommand::CaptureCase {
                root: root.clone(),
                case_id: "first_fight".to_string(),
                label: None,
            })
            .expect("capture-case should remember the case");
        let decision_step = session.decision_step;

        let outcome = session
            .apply_command(RunControlCommand::SearchCombat(
                crate::eval::run_control::RunControlSearchCombatOptions {
                    max_nodes: Some(2_000),
                    wall_ms: Some(5_000),
                    evidence: Some(
                        crate::eval::run_control::RunControlSearchEvidenceTarget::LastCaptureCase,
                    ),
                    ..Default::default()
                },
            ))
            .expect("search-combat should finish and save evidence");

        assert!(outcome.message.contains("Search evidence saved"));
        let evidence_path = root
            .join("search_evidence")
            .join(format!("first_fight.step{decision_step}.search.json"));
        let payload = fs::read_to_string(&evidence_path).expect("search evidence should exist");
        assert!(payload.contains("\"schema_name\": \"CombatSearchEvidenceV1\""));
        assert!(payload.contains("\"label_role\": \"search_evidence_not_human_baseline\""));
        assert!(payload.contains("\"capture_case_id\": \"first_fight\""));
        assert!(payload.contains("\"capture_path\":"));
        assert!(payload.contains("first_fight.capture.json"));
        assert!(payload.contains("\"schema_name\": \"CombatSearchV2Report\""));
        crate::eval::run_control::load_combat_search_evidence_v1(&evidence_path)
            .expect("search evidence should validate");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn run_control_capture_command_rejects_map_state() {
        let session = test_session_after_neow_at_map();

        let err = session
            .save_current_combat_capture(Path::new("unused.json"), None)
            .expect_err("map state should not capture");

        assert!(err.contains("no active combat state"));
    }

    #[test]
    fn run_control_search_combat_applies_complete_winning_trajectory() {
        let mut session = test_session_with_first_monster_room();
        session
            .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
            .expect("map input should enter combat");

        let outcome = session
            .apply_command(RunControlCommand::SearchCombat(
                crate::eval::run_control::RunControlSearchCombatOptions {
                    max_nodes: Some(2_000),
                    wall_ms: Some(5_000),
                    ..Default::default()
                },
            ))
            .expect("search-combat should resolve starter combat");

        assert!(outcome
            .message
            .contains("Search combat applied complete winning trajectory"));
        assert!(outcome
            .message
            .contains("optimality=not_claimed_budgeted_complete_win"));
        assert!(outcome.action_result.is_some());
        assert!(session.active_combat.is_none());
        assert_eq!(
            session
                .last_combat_baseline()
                .map(CombatBaselineOutcomeV1::terminal),
            Some(crate::sim::combat::CombatTerminal::Win)
        );
    }

    #[test]
    fn run_control_combat_potion_use_updates_visible_potion_slots() {
        let mut session = test_session_with_first_monster_room();
        session.run_state.potions[1] = Some(crate::content::potions::Potion::new(
            crate::content::potions::PotionId::FruitJuice,
            42,
        ));
        session
            .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
            .expect("map input should enter combat");

        let outcome = session
            .apply_command(RunControlCommand::UsePotion {
                potion_index: 1,
                target_slot_or_id: None,
            })
            .expect("fruit juice should be usable in combat");

        assert!(outcome.message.contains("Lost potion: Fruit Juice"));
        assert!(session.active_combat.as_ref().is_some_and(|active| active
            .combat_state
            .entities
            .potions[1]
            .is_none()));
        let rendered = render_run_control_state(&session);
        assert!(!rendered.contains("Fruit Juice"));
        assert!(render_run_control_details(&session).contains("potions=0"));
    }

    #[test]
    fn run_control_auto_step_advances_routine_neow_intro_only() {
        let mut session = RunControlSession::new(RunControlConfig::default());

        let outcome = session
            .apply_command(RunControlCommand::AutoStep(Default::default()))
            .expect("auto-step should advance routine intro");

        assert!(outcome.message.contains("routine: Proceed"));
        assert!(outcome
            .message
            .contains("Reason: Neow bonus requires human choice"));
        assert!(outcome.action_result.is_some());
        assert!(matches!(session.engine_state, EngineState::EventRoom));
        assert_eq!(
            session
                .run_state
                .event_state
                .as_ref()
                .map(|event| event.current_screen),
            Some(1)
        );
    }

    #[test]
    fn run_control_auto_step_stops_on_map_without_mutating_state() {
        let mut session = test_session_after_neow_at_map();

        let outcome = session
            .apply_command(RunControlCommand::AutoStep(Default::default()))
            .expect("auto-step should stop at map");

        assert!(outcome.message.contains("Applied:\n  none"));
        assert!(outcome
            .message
            .contains("Reason: map route requires human choice"));
        assert!(outcome.action_result.is_none());
        assert!(matches!(session.engine_state, EngineState::MapNavigation));
    }

    #[test]
    fn run_control_auto_step_route_planner_advances_map_then_stops_at_combat() {
        let mut session = test_session_with_first_monster_room();

        let outcome = session
            .apply_command(RunControlCommand::AutoStep(
                crate::eval::run_control::RunControlAutoStepOptions {
                    route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
                    max_operations: Some(1),
                    ..Default::default()
                },
            ))
            .expect("auto-step route planner should choose a map node");

        assert!(outcome.message.contains("route planner:"));
        assert!(outcome.message.contains("x="));
        assert!(outcome.message.contains("command=go"));
        assert!(outcome
            .message
            .contains("label_role=behavior_policy_not_teacher"));
        assert!(outcome
            .message
            .contains("Reason: operation budget exhausted at 1 automatic operations"));
        assert!(outcome.action_result.is_some());
        assert!(matches!(
            session.engine_state,
            EngineState::CombatPlayerTurn
        ));
        assert_eq!(session.run_state.map.current_y, 0);
    }

    #[test]
    fn run_control_auto_step_route_planner_reports_auto_capture() {
        let root = unique_temp_dir("run_control_auto_step_route_auto_capture");
        let mut session = test_session_with_first_monster_room();
        session.auto_capture = AutoCombatCaptureConfig {
            enabled: true,
            root: Some(root.clone()),
        };

        let outcome = session
            .apply_command(RunControlCommand::AutoStep(
                crate::eval::run_control::RunControlAutoStepOptions {
                    route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
                    max_operations: Some(1),
                    ..Default::default()
                },
            ))
            .expect("route planner should enter combat and auto-capture");

        assert!(outcome.message.contains("route planner:"));
        assert!(outcome.message.contains("auto capture:"));
        assert!(outcome.trace_annotations.iter().any(|annotation| matches!(
            annotation,
            RunControlTraceAnnotationV1::AutoCombatCapture { .. }
        )));
        assert!(outcome.trace_annotations.iter().any(|annotation| matches!(
            annotation,
            RunControlTraceAnnotationV1::RoutePlannerSelection { .. }
        )));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn run_control_auto_step_leaves_empty_shop() {
        let mut session = test_session_at_shop();
        if let EngineState::Shop(shop) = &mut session.engine_state {
            shop.cards.clear();
            shop.relics.clear();
            shop.potions.clear();
            shop.purge_available = false;
        }

        let outcome = session
            .apply_command(RunControlCommand::AutoStep(Default::default()))
            .expect("auto-step should leave a shop with no remaining executable choices");

        assert!(outcome
            .message
            .contains("routine: Leave shop (only shop exit remains)"));
        assert!(!matches!(session.engine_state, EngineState::Shop(_)));
    }

    #[test]
    fn run_control_auto_step_claims_low_risk_rewards_then_stops() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut rewards = crate::state::rewards::RewardState::new();
        rewards.items = vec![
            crate::state::rewards::RewardItem::Gold { amount: 19 },
            crate::state::rewards::RewardItem::Potion {
                potion_id: crate::content::potions::PotionId::EssenceOfSteel,
            },
            crate::state::rewards::RewardItem::Card {
                cards: vec![crate::state::rewards::RewardCard::new(
                    crate::content::cards::CardId::ShrugItOff,
                    0,
                )],
            },
        ];
        session.engine_state = EngineState::RewardScreen(rewards);

        let outcome = session
            .apply_command(RunControlCommand::AutoStep(Default::default()))
            .expect("auto-step should claim deterministic rewards");

        assert!(outcome
            .message
            .contains("routine reward: 19 gold, Essence of Steel potion"));
        assert!(outcome
            .message
            .contains("Reason: card reward requires human choice"));
        assert_eq!(session.run_state.gold, 118);
        assert_eq!(
            session.run_state.potions[0]
                .as_ref()
                .map(|potion| potion.id),
            Some(crate::content::potions::PotionId::EssenceOfSteel)
        );
        assert!(outcome.action_result.is_some());
    }

    #[test]
    fn run_control_auto_step_solves_starter_combat_and_stops_at_reward_choice() {
        let mut session = test_session_with_first_monster_room();
        session
            .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
            .expect("map input should enter combat");

        let outcome = session
            .apply_command(RunControlCommand::AutoStep(
                crate::eval::run_control::RunControlAutoStepOptions {
                    search: crate::eval::run_control::RunControlSearchCombatOptions {
                        max_nodes: Some(2_000),
                        wall_ms: Some(5_000),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .expect("auto-step should resolve starter combat");

        assert!(outcome
            .message
            .contains("combat search: search-combat applied"));
        assert!(
            outcome
                .message
                .contains("Reason: remaining reward requires human choice")
                || outcome
                    .message
                    .contains("Reason: card reward requires human choice")
        );
        assert!(outcome.action_result.is_some());
        assert!(session.active_combat.is_none());
        assert_eq!(
            session
                .last_combat_baseline()
                .map(CombatBaselineOutcomeV1::terminal),
            Some(crate::sim::combat::CombatTerminal::Win)
        );
    }

    #[test]
    fn run_control_case_command_saves_diagnostic_decision_case() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let dir = unique_temp_dir("run_control_decision_case");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let path = dir.join("decision.json");

        let outcome = session
            .apply_command(RunControlCommand::SaveDecisionCase {
                path: Some(path.clone()),
            })
            .expect("case command should save");

        assert!(outcome.message.contains("saved RunDecisionCaseV1"));
        assert!(
            outcome.action_result.is_none(),
            "non-action commands should not fabricate action results"
        );
        let payload = fs::read_to_string(&path).expect("decision case should exist");
        assert!(payload.contains("\"schema_name\": \"sts_simulator.run_decision_case\""));
        assert!(payload.contains("\"label_role\": \"diagnostic_not_teacher_label\""));
        assert!(payload.contains("\"trainable_as_action_label\": false"));
        assert!(payload.contains("\"policy_quality_claim\": false"));
        assert!(payload.contains("\"resolution\""));

        let _ = fs::remove_file(path);
        let _ = fs::remove_dir(dir);
    }

    #[test]
    fn run_control_visible_candidate_command_advances_current_screen() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let outcome = session
            .apply_command(RunControlCommand::DefaultCandidate)
            .expect("single visible Neow intro action should execute");

        assert!(outcome.message.contains("Neow Bonus"));
        let action_result = outcome
            .action_result
            .as_ref()
            .expect("state-changing commands should return a structured action result");
        assert!(action_result.changes.iter().any(|change| matches!(
            change,
            crate::eval::run_control::RunActionResultChangeV1::AdvancedTo { title }
                if title == "Neow Bonus"
        )));
        let json = serde_json::to_string(action_result)
            .expect("structured action result should be serializable");
        assert!(json.contains("advanced_to"));
        assert_eq!(session.decision_step, 1);
        assert_eq!(
            session
                .run_state
                .event_state
                .as_ref()
                .map(|event| event.current_screen),
            Some(1)
        );
    }

    #[test]
    fn run_control_rejects_proceed_alias_on_neow_intro() {
        let mut session = RunControlSession::new(RunControlConfig::default());

        let err = session
            .apply_command(RunControlCommand::Input(ClientInput::Proceed))
            .expect_err("raw proceed must not be accepted on the Neow intro event screen");

        assert!(err.contains("input `proceed` is not valid"));
        assert!(err.contains("Neow Intro"));
        assert_eq!(session.decision_step, 0);
        assert!(matches!(session.engine_state, EngineState::EventRoom));
        assert_eq!(
            session
                .run_state
                .event_state
                .as_ref()
                .map(|event| event.current_screen),
            Some(0)
        );
    }

    #[test]
    fn run_control_rejects_reward_command_on_neow_intro() {
        let mut session = RunControlSession::new(RunControlConfig::default());

        let err = session
            .apply_command(RunControlCommand::Input(ClientInput::ClaimReward(0)))
            .expect_err("reward claim must not be accepted on an event screen");

        assert!(err.contains("input `claim 0` is not valid"));
        assert!(err.contains("Neow Intro"));
        assert_eq!(session.decision_step, 0);
        assert!(matches!(session.engine_state, EngineState::EventRoom));
    }

    #[test]
    fn run_control_rejects_map_travel_before_neow_is_complete() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session
            .apply_command(RunControlCommand::DefaultCandidate)
            .expect("Neow intro should advance");

        let err = session
            .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
            .expect_err("Neow bonus should not allow first-room travel");

        assert!(err.contains("input `go 0` is not valid"));
        assert!(err.contains("Neow Bonus"));
        assert!(matches!(session.engine_state, EngineState::EventRoom));
    }

    #[test]
    fn run_control_shop_accepts_visible_candidate_ids_and_contextual_words() {
        let mut session = test_session_at_shop();

        let outcome = session
            .apply_command(RunControlCommand::Candidate("card-0".to_string()))
            .expect("visible shop card id should buy");
        assert!(outcome.message.contains("Added card: Armaments"));
        assert_eq!(session.run_state.gold, 51);

        let mut session = test_session_at_shop();
        let outcome = session
            .apply_command(RunControlCommand::CardIndex(1))
            .expect("card <idx> should buy in shop");
        assert!(outcome.message.contains("Added card: Shrug It Off"));
        assert_eq!(session.run_state.gold, 50);

        let mut session = test_session_at_shop();
        let outcome = session
            .apply_command(RunControlCommand::Candidate("1".to_string()))
            .expect("bare numeric shop id should fall back to card-<idx>");
        assert!(outcome.message.contains("Added card: Shrug It Off"));
        assert_eq!(session.run_state.gold, 50);
    }

    #[test]
    fn run_control_shop_leave_candidate_exits_shop() {
        let mut session = test_session_at_shop();

        let outcome = session
            .apply_command(RunControlCommand::Candidate("leave".to_string()))
            .expect("visible leave id should leave shop");

        assert!(outcome.message.contains("Chose: Leave shop"));
        assert!(!matches!(session.engine_state, EngineState::Shop(_)));
    }

    #[test]
    fn run_control_campfire_accepts_bare_smith_index_alias() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::Campfire;

        let outcome = session
            .apply_command(RunControlCommand::Candidate("8".to_string()))
            .expect("bare numeric campfire id should fall back to smith-<idx>");

        assert!(outcome.message.contains("Chose: Smith Defend"));
    }

    #[test]
    fn visible_candidate_alias_resolves_label_leave_and_skip() {
        use crate::eval::run_control::view_model::{CandidateAction, DecisionCandidate};

        let candidates = vec![
            DecisionCandidate {
                id: "0".to_string(),
                label: "Leave.".to_string(),
                action: CandidateAction::Input(ClientInput::EventChoice(0)),
                note: None,
                resolution: None,
            },
            DecisionCandidate {
                id: "1".to_string(),
                label: "Skip card reward".to_string(),
                action: CandidateAction::Input(ClientInput::Proceed),
                note: None,
                resolution: None,
            },
        ];

        assert_eq!(
            decision_surface::resolve_candidate_alias(
                &candidates,
                &EngineState::EventRoom,
                "leave"
            )
            .map(|candidate| candidate.id.as_str()),
            Some("0")
        );
        assert_eq!(
            decision_surface::resolve_candidate_alias(
                &candidates,
                &EngineState::RewardScreen(Default::default()),
                "skip"
            )
            .map(|candidate| candidate.id.as_str()),
            Some("1")
        );
    }

    #[test]
    fn run_control_campfire_renders_all_upgradeable_smith_targets() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::Campfire;

        let rendered = render_run_control_state(&session);

        assert!(rendered.contains("smith-9 | Smith Bash"));
        assert!(
            rendered.contains("smith-8 | Smith Defend"),
            "campfire smith candidates must not truncate after the first eight deck cards"
        );
    }

    fn test_session_with_first_monster_room() -> RunControlSession {
        let mut session = test_session_after_neow_at_map();
        let mut first = MapRoomNode::new(0, 0);
        first.class = Some(RoomType::MonsterRoom);
        first.edges.insert(MapEdge::new(0, 0, 0, 1));
        let mut second = MapRoomNode::new(0, 1);
        second.class = Some(RoomType::MonsterRoom);
        session.run_state.map = MapState::new(vec![vec![first], vec![second]]);
        session.run_state.monster_list = vec![EncounterId::JawWorm, EncounterId::Cultist];
        session
    }

    fn test_session_at_shop() -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = None;
        session.run_state.gold = 100;
        let mut shop = crate::state::shop::ShopState::new();
        shop.cards = vec![
            crate::state::shop::ShopCard {
                card_id: crate::content::cards::CardId::Armaments,
                upgrades: 0,
                price: 49,
                can_buy: true,
                blocked_reason: None,
            },
            crate::state::shop::ShopCard {
                card_id: crate::content::cards::CardId::ShrugItOff,
                upgrades: 0,
                price: 50,
                can_buy: true,
                blocked_reason: None,
            },
        ];
        session.engine_state = EngineState::Shop(shop);
        session
    }

    fn test_session_after_neow_at_map() -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = None;
        session.engine_state = EngineState::MapNavigation;
        session
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{label}_{}_{}", std::process::id(), nanos))
    }
}
