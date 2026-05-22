use std::path::Path;

use crate::engine::run_loop::tick_run;
use crate::eval::combat_capture::{
    capture_combat_position_v1, save_combat_capture_v1, CombatCaptureV1,
};
use crate::runtime::combat::CombatState;
use crate::sim::combat::CombatPosition;
use crate::sim::combat_legal_actions::get_legal_moves;
use crate::state::core::{ClientInput, EngineState, RunResult};
use crate::state::run::RunState;

use super::combat_start::ensure_combat_started_if_needed;
use super::commands::{run_play_help, RunPlayCommand};
use super::render::{render_combat_actions, render_run_play_state};

#[derive(Clone, Debug)]
pub struct RunPlayConfig {
    pub seed: u64,
    pub ascension_level: u8,
    pub final_act: bool,
    pub player_class: &'static str,
    pub skip_neow: bool,
}

impl Default for RunPlayConfig {
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
pub struct RunPlaySession {
    pub engine_state: EngineState,
    pub run_state: RunState,
    pub combat_state: Option<CombatState>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunPlayCommandOutcome {
    pub should_quit: bool,
    pub message: String,
}

impl RunPlayCommandOutcome {
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

impl RunPlaySession {
    pub fn new(config: RunPlayConfig) -> Self {
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
            combat_state: None,
        }
    }

    pub fn apply_command(
        &mut self,
        command: RunPlayCommand,
    ) -> Result<RunPlayCommandOutcome, String> {
        self.ensure_combat_started_if_needed()?;

        match command {
            RunPlayCommand::Noop => Ok(RunPlayCommandOutcome::message("")),
            RunPlayCommand::Help => Ok(RunPlayCommandOutcome::message(run_play_help())),
            RunPlayCommand::Quit => Ok(RunPlayCommandOutcome::quit("quit")),
            RunPlayCommand::State => {
                Ok(RunPlayCommandOutcome::message(render_run_play_state(self)))
            }
            RunPlayCommand::Actions => {
                Ok(RunPlayCommandOutcome::message(render_combat_actions(self)?))
            }
            RunPlayCommand::Capture { path, label } => {
                let capture = self.save_current_combat_capture(&path, label)?;
                Ok(RunPlayCommandOutcome::message(format!(
                    "saved CombatCaptureV1 to {} [{} hp={}, turn={}, enemies={}]",
                    path.display(),
                    capture.summary.engine_state,
                    capture.summary.player_hp,
                    capture.summary.turn_count,
                    capture.summary.monsters.len()
                )))
            }
            RunPlayCommand::ActionIndex(index) => {
                let input = self.combat_action_by_index(index)?;
                self.apply_input(input)
            }
            RunPlayCommand::PlayCard {
                card_index,
                target_slot_or_id,
            } => {
                let target = self.resolve_target(target_slot_or_id)?;
                self.apply_input(ClientInput::PlayCard { card_index, target })
            }
            RunPlayCommand::UsePotion {
                potion_index,
                target_slot_or_id,
            } => {
                let target = self.resolve_target(target_slot_or_id)?;
                self.apply_input(ClientInput::UsePotion {
                    potion_index,
                    target,
                })
            }
            RunPlayCommand::Input(input) => self.apply_input(input),
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

    pub(crate) fn current_active_combat_position(&self) -> Result<CombatPosition, String> {
        let combat = self
            .combat_state
            .as_ref()
            .ok_or_else(|| "no active combat state to capture".to_string())?;
        match self.engine_state {
            EngineState::CombatPlayerTurn | EngineState::PendingChoice(_) => {
                Ok(CombatPosition::new(self.engine_state.clone(), combat.clone()))
            }
            EngineState::EventCombat(_) => Err(
                "event combat capture is not supported yet; EventCombat currently wraps combat outside the search engine state"
                    .to_string(),
            ),
            _ => Err(format!(
                "cannot capture combat from engine state {:?}",
                self.engine_state
            )),
        }
    }

    pub(crate) fn current_combat_position_for_actions(&self) -> Result<CombatPosition, String> {
        let combat = self
            .combat_state
            .as_ref()
            .ok_or_else(|| "no active combat state".to_string())?;
        let engine = match &self.engine_state {
            EngineState::CombatPlayerTurn
            | EngineState::CombatProcessing
            | EngineState::PendingChoice(_) => self.engine_state.clone(),
            EngineState::EventCombat(_) => EngineState::CombatPlayerTurn,
            other => {
                return Err(format!(
                    "engine state {other:?} is not an active combat input state"
                ))
            }
        };
        Ok(CombatPosition::new(engine, combat.clone()))
    }

    fn apply_input(&mut self, input: ClientInput) -> Result<RunPlayCommandOutcome, String> {
        let keep_running = tick_run(
            &mut self.engine_state,
            &mut self.run_state,
            &mut self.combat_state,
            Some(input),
        );
        self.cleanup_inactive_combat();
        self.ensure_combat_started_if_needed()?;

        let status = if keep_running {
            "ok".to_string()
        } else {
            match self.engine_state {
                EngineState::GameOver(RunResult::Victory) => "game_over:victory".to_string(),
                EngineState::GameOver(RunResult::Defeat) => "game_over:defeat".to_string(),
                _ => "stopped".to_string(),
            }
        };
        Ok(RunPlayCommandOutcome::message(format!(
            "{status}\n{}",
            render_run_play_state(self)
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
            .combat_state
            .as_ref()
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
                | EngineState::EventCombat(_)
        ) {
            self.combat_state = None;
        }
    }

    fn ensure_combat_started_if_needed(&mut self) -> Result<(), String> {
        ensure_combat_started_if_needed(
            &mut self.engine_state,
            &mut self.run_state,
            &mut self.combat_state,
        )
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
    use crate::state::map::node::{MapEdge, MapRoomNode, RoomType};
    use crate::state::map::state::MapState;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn run_play_capture_command_saves_active_combat_position() {
        let mut session = test_session_with_first_monster_room();
        session
            .apply_command(RunPlayCommand::Input(ClientInput::SelectMapNode(0)))
            .expect("map input should enter combat");
        assert!(matches!(
            session.engine_state,
            EngineState::CombatPlayerTurn
        ));

        let dir = unique_temp_dir("run_play_capture");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let path = dir.join("capture.json");
        let outcome = session
            .apply_command(RunPlayCommand::Capture {
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
    fn run_play_capture_command_rejects_map_state() {
        let session = RunPlaySession::new(RunPlayConfig {
            skip_neow: true,
            ..RunPlayConfig::default()
        });

        let err = session
            .save_current_combat_capture(Path::new("unused.json"), None)
            .expect_err("map state should not capture");

        assert!(err.contains("no active combat state"));
    }

    fn test_session_with_first_monster_room() -> RunPlaySession {
        let mut session = RunPlaySession::new(RunPlayConfig {
            skip_neow: true,
            ..RunPlayConfig::default()
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
