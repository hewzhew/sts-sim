use std::path::Path;

use crate::engine::run_loop::tick_run_active_with_observer;
use crate::eval::combat_capture::{
    capture_combat_position_v1, save_combat_capture_v1, CombatCaptureV1,
};
use crate::sim::combat::CombatPosition;
use crate::sim::combat_legal_actions::get_legal_moves;
use crate::state::core::{ActiveCombat, ClientInput, EngineState, RunResult};
use crate::state::run::RunState;

use super::combat_start::ensure_combat_started_if_needed;
use super::commands::{parse_run_control_command, run_control_help, RunControlCommand};
use super::decision_case::{
    default_run_decision_case_path, save_run_decision_case_v1, RunDecisionCaseV1,
};
use super::outcome::{
    save_combat_baseline_outcome_v1, CombatBaselineOutcomeV1, CombatOutcomeTracker,
};
use super::panels::{
    render_combat_zone_panel, render_deck_panel, render_inspect_panel, render_map_panel,
    render_potions_panel, render_relics_panel, CombatZonePanel,
};
use super::registry::{add_case_to_benchmark_registry, BenchmarkCasePaths};
use super::render::{
    render_combat_actions, render_run_control_details, render_run_control_raw,
    render_run_control_state,
};

const MAX_STABLE_ADVANCE_TICKS: usize = 2_000;

#[derive(Clone, Debug)]
pub struct RunControlConfig {
    pub seed: u64,
    pub ascension_level: u8,
    pub final_act: bool,
    pub player_class: &'static str,
    pub skip_neow: bool,
}

impl Default for RunControlConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            ascension_level: 0,
            final_act: false,
            player_class: "Ironclad",
            skip_neow: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RunControlSession {
    pub engine_state: EngineState,
    pub run_state: RunState,
    pub active_combat: Option<ActiveCombat>,
    pub decision_step: u64,
    combat_outcomes: CombatOutcomeTracker,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunControlCommandOutcome {
    pub should_quit: bool,
    pub message: String,
}

impl RunControlCommandOutcome {
    fn message(message: impl Into<String>) -> Self {
        Self {
            should_quit: false,
            message: message.into(),
        }
    }

    fn quit(message: impl Into<String>) -> Self {
        Self {
            should_quit: true,
            message: message.into(),
        }
    }
}

impl RunControlSession {
    pub fn new(config: RunControlConfig) -> Self {
        let mut run_state = RunState::new(
            config.seed,
            config.ascension_level,
            config.final_act,
            config.player_class,
        );
        let engine_state = if config.skip_neow {
            run_state.event_state = None;
            EngineState::MapNavigation
        } else {
            EngineState::EventRoom
        };

        Self {
            engine_state,
            run_state,
            active_combat: None,
            decision_step: 0,
            combat_outcomes: CombatOutcomeTracker::default(),
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
                let path = path.unwrap_or_else(|| default_run_decision_case_path(self));
                let decision_case = RunDecisionCaseV1::from_session(self);
                save_run_decision_case_v1(&path, &decision_case)?;
                Ok(RunControlCommandOutcome::message(format!(
                    "saved RunDecisionCaseV1 to {} [label_role={} trainable_as_action_label={} policy_quality_claim={}]",
                    path.display(),
                    decision_case.label_role,
                    decision_case.trainable_as_action_label,
                    decision_case.policy_quality_claim
                )))
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
                let capture = self.save_current_combat_capture(&path, label)?;
                Ok(RunControlCommandOutcome::message(format!(
                    "saved CombatCaptureV1 to {} [{} hp={}, turn={}, enemies={}]",
                    path.display(),
                    capture.summary.engine_state,
                    capture.summary.player_hp,
                    capture.summary.turn_count,
                    capture.summary.monsters.len()
                )))
            }
            RunControlCommand::CaptureCase {
                root,
                case_id,
                label,
            } => {
                let paths = BenchmarkCasePaths::for_case(&root, &case_id);
                let capture = self.save_current_combat_capture(
                    &paths.capture_path,
                    label.or_else(|| Some(case_id.clone())),
                )?;
                Ok(RunControlCommandOutcome::message(format!(
                    "saved CombatCaptureV1 case {case_id} to {} [{} hp={}, turn={}, enemies={}]",
                    paths.capture_path.display(),
                    capture.summary.engine_state,
                    capture.summary.player_hp,
                    capture.summary.turn_count,
                    capture.summary.monsters.len()
                )))
            }
            RunControlCommand::SaveBaseline { path, case_id } => {
                let baseline = self.save_last_combat_baseline(
                    &path,
                    case_id.unwrap_or_else(|| inferred_case_id_from_path(&path)),
                )?;
                Ok(RunControlCommandOutcome::message(format!(
                    "saved CombatBaselineOutcomeV1 to {} [case={} terminal={:?} hp_loss={} final_hp={} turns={} potions_used={} cards_played={}]",
                    path.display(),
                    baseline.case_id,
                    baseline.terminal,
                    baseline.hp_loss,
                    baseline.final_hp,
                    baseline.turns,
                    baseline.potions_used,
                    baseline.cards_played
                )))
            }
            RunControlCommand::SaveBaselineCase { root, case_id } => {
                let paths = BenchmarkCasePaths::for_case(&root, &case_id);
                let baseline = self.save_last_combat_baseline(&paths.baseline_path, case_id)?;
                Ok(RunControlCommandOutcome::message(format!(
                    "saved CombatBaselineOutcomeV1 to {} [case={} terminal={:?} hp_loss={} final_hp={} turns={} potions_used={} cards_played={}]",
                    paths.baseline_path.display(),
                    baseline.case_id,
                    baseline.terminal,
                    baseline.hp_loss,
                    baseline.final_hp,
                    baseline.turns,
                    baseline.potions_used,
                    baseline.cards_played
                )))
            }
            RunControlCommand::RegisterBenchmarkCase { root, case_id } => {
                let paths = add_case_to_benchmark_registry(&root, &case_id)?;
                Ok(RunControlCommandOutcome::message(format!(
                    "registered benchmark case {case_id} in {} [capture={}, baseline={}]",
                    paths.benchmark_manifest.display(),
                    paths.capture_path.display(),
                    paths.baseline_path.display()
                )))
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
                let target = self.resolve_target(target_slot_or_id)?;
                self.apply_input(ClientInput::UsePotion {
                    potion_index,
                    target,
                })
            }
            RunControlCommand::Input(input) => self.apply_input(input),
        }
    }

    fn apply_default_candidate(&mut self) -> Result<RunControlCommandOutcome, String> {
        let view = crate::eval::run_control::view_model::build_run_control_view_model(self);
        if view.candidates.len() != 1 {
            return Err(
                "Enter only executes when exactly one visible action is available; choose an id"
                    .to_string(),
            );
        }
        let id = view.candidates[0].id.clone();
        self.apply_visible_candidate(&id)
    }

    fn apply_visible_candidate(&mut self, id: &str) -> Result<RunControlCommandOutcome, String> {
        let view = crate::eval::run_control::view_model::build_run_control_view_model(self);
        let command = view
            .candidates
            .iter()
            .find(|candidate| candidate.id == id)
            .map(|candidate| candidate.command.clone())
            .ok_or_else(|| format!("no visible candidate '{id}'"))?;
        if command.contains('<') {
            return Err(format!(
                "candidate '{id}' requires an explicit command: {command}"
            ));
        }
        let parsed = parse_run_control_command(&command)?;
        match parsed {
            RunControlCommand::Candidate(_) | RunControlCommand::DefaultCandidate => Err(format!(
                "candidate '{id}' resolved to another candidate instead of an executable command"
            )),
            other => self.apply_command(other),
        }
    }

    pub fn save_current_combat_capture(
        &self,
        path: &Path,
        label: Option<String>,
    ) -> Result<CombatCaptureV1, String> {
        let position = self.current_active_combat_position()?;
        let capture = capture_combat_position_v1(label, &position)?;
        save_combat_capture_v1(path, &capture)?;
        Ok(capture)
    }

    pub fn last_combat_baseline(&self) -> Option<&CombatBaselineOutcomeV1> {
        self.combat_outcomes.last()
    }

    pub fn save_last_combat_baseline(
        &self,
        path: &Path,
        case_id: String,
    ) -> Result<CombatBaselineOutcomeV1, String> {
        let mut baseline = self
            .combat_outcomes
            .last()
            .cloned()
            .ok_or_else(|| "no completed combat baseline is available".to_string())?;
        baseline.case_id = case_id;
        save_combat_baseline_outcome_v1(path, &baseline)?;
        Ok(baseline)
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

    fn apply_input(&mut self, input: ClientInput) -> Result<RunControlCommandOutcome, String> {
        self.ensure_combat_started_if_needed()?;
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
        }
        self.cleanup_inactive_combat();
        self.ensure_combat_started_if_needed()?;
        self.observe_active_combat_started();
        self.decision_step = self.decision_step.saturating_add(1);

        let status = if tick.keep_running {
            "ok".to_string()
        } else {
            match self.engine_state {
                EngineState::GameOver(RunResult::Victory) => "game_over:victory".to_string(),
                EngineState::GameOver(RunResult::Defeat) => "game_over:defeat".to_string(),
                _ => "stopped".to_string(),
            }
        };
        Ok(RunControlCommandOutcome::message(format!(
            "{status}\n{}",
            render_run_control_state(self)
        )))
    }

    fn combat_action_by_index(&self, index: usize) -> Result<ClientInput, String> {
        let position = self.current_combat_position_for_actions()?;
        let actions = get_legal_moves(&position.engine, &position.combat);
        actions
            .get(index)
            .cloned()
            .ok_or_else(|| format!("combat action index {index} out of range"))
    }

    fn resolve_target(&self, target_slot_or_id: Option<usize>) -> Result<Option<usize>, String> {
        let Some(raw) = target_slot_or_id else {
            return Ok(None);
        };
        let combat = self
            .active_combat
            .as_ref()
            .map(|active| &active.combat_state)
            .ok_or_else(|| "targeted action requires active combat".to_string())?;
        combat
            .entities
            .monsters
            .iter()
            .find(|monster| monster.slot as usize == raw)
            .or_else(|| {
                combat
                    .entities
                    .monsters
                    .iter()
                    .find(|monster| monster.id == raw)
            })
            .map(|monster| Some(monster.id))
            .ok_or_else(|| format!("no monster slot or entity id {raw}"))
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
        self.combat_outcomes.ensure_started(
            self.active_combat
                .as_ref()
                .map(|active| &active.combat_state),
        );
    }
}

fn inferred_case_id_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.trim().is_empty())
        .unwrap_or("last_combat")
        .trim_end_matches(".baseline")
        .to_string()
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
    use crate::state::map::node::{MapEdge, MapRoomNode, RoomType};
    use crate::state::map::state::MapState;
    use std::fs;
    use std::path::PathBuf;
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
        assert!(matches!(
            loaded.position.engine,
            EngineState::CombatPlayerTurn
        ));

        let _ = fs::remove_file(path);
        let _ = fs::remove_dir(dir);
    }

    #[test]
    fn run_control_capture_command_rejects_map_state() {
        let session = RunControlSession::new(RunControlConfig {
            skip_neow: true,
            ..RunControlConfig::default()
        });

        let err = session
            .save_current_combat_capture(Path::new("unused.json"), None)
            .expect_err("map state should not capture");

        assert!(err.contains("no active combat state"));
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
        let payload = fs::read_to_string(&path).expect("decision case should exist");
        assert!(payload.contains("\"schema_name\": \"sts_simulator.run_decision_case\""));
        assert!(payload.contains("\"label_role\": \"diagnostic_not_teacher_label\""));
        assert!(payload.contains("\"trainable_as_action_label\": false"));
        assert!(payload.contains("\"policy_quality_claim\": false"));

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

    fn test_session_with_first_monster_room() -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig {
            skip_neow: true,
            ..RunControlConfig::default()
        });
        let mut first = MapRoomNode::new(0, 0);
        first.class = Some(RoomType::MonsterRoom);
        first.edges.insert(MapEdge::new(0, 0, 0, 1));
        let mut second = MapRoomNode::new(0, 1);
        second.class = Some(RoomType::MonsterRoom);
        session.run_state.map = MapState::new(vec![vec![first], vec![second]]);
        session.run_state.monster_list = vec![EncounterId::JawWorm, EncounterId::Cultist];
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
