use std::path::Path;

use crate::engine::run_loop::tick_run_active_with_observer;
use crate::eval::combat_capture::{
    capture_combat_position_from_run_v1, save_combat_capture_v1, CombatCaptureV1,
};
use crate::sim::combat::CombatPosition;
use crate::sim::combat_legal_actions::get_legal_moves;
use crate::state::core::{ActiveCombat, ClientInput, EngineState, RunResult};
use crate::state::run::RunState;

use super::combat_start::ensure_combat_started_if_needed;
use super::commands::{run_control_help, RunControlCommand};
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
use super::reward_auto::{set_reward_automation, RewardAutomationConfig};
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
}

impl Default for RunControlConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            ascension_level: 0,
            final_act: false,
            player_class: "Ironclad",
            reward_automation: RewardAutomationConfig::default(),
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
    combat_outcomes: CombatOutcomeTracker,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunControlCommandOutcome {
    pub should_quit: bool,
    pub message: String,
    pub action_result: Option<ActionResult>,
}

impl RunControlCommandOutcome {
    pub(in crate::eval::run_control) fn message(message: impl Into<String>) -> Self {
        Self {
            should_quit: false,
            message: message.into(),
            action_result: None,
        }
    }

    fn quit(message: impl Into<String>) -> Self {
        Self {
            should_quit: true,
            message: message.into(),
            action_result: None,
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
        }
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
                let paths = add_case_to_benchmark_registry(&root, &case_id)?;
                Ok(RunControlCommandOutcome::message(format!(
                    "saved CombatCaptureV1 case {case_id} to {} and registered {} [{} hp={}, turn={}, enemies={} trust={:?}]",
                    paths.capture_path.display(),
                    paths.benchmark_manifest.display(),
                    capture.summary.engine_state,
                    capture.summary.player_hp,
                    capture.summary.turn_count,
                    capture.summary.monsters.len(),
                    capture.trust_level
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
                let baseline =
                    self.save_last_combat_baseline(&paths.baseline_path, case_id.clone())?;
                let registry_note = if paths.capture_path.exists() {
                    let paths = add_case_to_benchmark_registry(&root, &case_id)?;
                    format!(" and registered {}", paths.benchmark_manifest.display())
                } else {
                    " [benchmark not registered: matching capture is missing]".to_string()
                };
                Ok(RunControlCommandOutcome::message(format!(
                    "saved CombatBaselineOutcomeV1 to {}{} [case={} terminal={:?} hp_loss={} final_hp={} turns={} potions_used={} cards_played={}]",
                    paths.baseline_path.display(),
                    registry_note,
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
                let baseline_status = if paths.baseline_path.exists() {
                    paths.baseline_path.display().to_string()
                } else {
                    "none".to_string()
                };
                Ok(RunControlCommandOutcome::message(format!(
                    "registered benchmark case {case_id} in {} [capture={}, baseline={}]",
                    paths.benchmark_manifest.display(),
                    paths.capture_path.display(),
                    baseline_status
                )))
            }
            RunControlCommand::SearchCombat(options) => {
                super::combat_search::apply_search_combat(self, options)
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
        let candidate = view
            .candidates
            .iter()
            .find(|candidate| candidate.id == id)
            .ok_or_else(|| format!("no visible candidate '{id}'"))?;
        match candidate.action.executable_input() {
            Some(input) => self.apply_input(input),
            None => Err(format!(
                "candidate '{id}' is not directly executable: {}",
                candidate.action.command_hint()
            )),
        }
    }

    pub fn save_current_combat_capture(
        &self,
        path: &Path,
        label: Option<String>,
    ) -> Result<CombatCaptureV1, String> {
        let position = self.current_active_combat_position()?;
        let capture = capture_combat_position_from_run_v1(label, &position, &self.run_state)?;
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
        }
        self.cleanup_inactive_combat();
        self.ensure_combat_started_if_needed()?;
        let reward_automation = super::reward_auto::apply_reward_automation(self)?;
        self.cleanup_inactive_combat();
        self.ensure_combat_started_if_needed()?;
        self.observe_active_combat_started();
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
        Ok(RunControlCommandOutcome::action(
            format!("{report}\n{}", render_run_control_state(self)),
            action_result,
        ))
    }

    fn validate_input_for_current_state(&self, input: &ClientInput) -> Result<(), String> {
        if self.visible_candidate_allows_input(input)
            || self.current_screen_allows_extra_input(input)
            || self.run_level_potion_input_is_allowed(input)
        {
            return Ok(());
        }
        Err(format!(
            "input `{}` is not valid on the current screen: {}",
            crate::eval::run_control::view_model::client_input_hint(input),
            crate::eval::run_control::view_model::build_run_control_view_model(self)
                .header
                .title
        ))
    }

    fn visible_candidate_allows_input(&self, input: &ClientInput) -> bool {
        crate::eval::run_control::view_model::build_run_control_view_model(self)
            .candidates
            .iter()
            .filter_map(|candidate| candidate.action.executable_input())
            .any(|candidate_input| &candidate_input == input)
    }

    fn current_screen_allows_extra_input(&self, input: &ClientInput) -> bool {
        match (&self.engine_state, input) {
            (
                EngineState::CombatPlayerTurn
                | EngineState::CombatProcessing
                | EngineState::PendingChoice(_),
                _,
            ) => self
                .current_combat_position_for_actions()
                .map(|position| get_legal_moves(&position.engine, &position.combat).contains(input))
                .unwrap_or(false),
            (EngineState::MapNavigation, ClientInput::FlyToNode(target_x, target_y)) => {
                self.map_flight_is_allowed(*target_x, *target_y)
            }
            (EngineState::RunPendingChoice(choice), ClientInput::SubmitDeckSelect(indices)) => {
                self.run_pending_selection_is_allowed(choice, indices)
            }
            (EngineState::RunPendingChoice(_), ClientInput::Cancel) => true,
            (EngineState::Shop(shop), ClientInput::PurgeCard(idx)) => {
                self.shop_purge_is_allowed(shop, *idx)
            }
            (EngineState::RewardScreen(reward), ClientInput::Cancel) => {
                reward.skippable || reward.pending_card_choice.is_some()
            }
            _ => false,
        }
    }

    fn map_flight_is_allowed(&self, target_x: usize, target_y: usize) -> bool {
        let has_flight = self.run_state.relics.iter().any(|relic| {
            relic.id == crate::content::relics::RelicId::WingBoots && relic.counter > 0
        });
        has_flight
            && self
                .run_state
                .map
                .can_travel_to(target_x as i32, target_y as i32, true)
    }

    fn run_pending_selection_is_allowed(
        &self,
        choice: &crate::state::core::RunPendingChoiceState,
        indices: &[usize],
    ) -> bool {
        if indices.len() < choice.min_choices || indices.len() > choice.max_choices {
            return false;
        }
        let mut seen = Vec::new();
        for &idx in indices {
            let Some(card) = self.run_state.master_deck.get(idx) else {
                return false;
            };
            if seen.contains(&idx)
                || !crate::state::core::run_pending_choice_allows_card_for_run(
                    &choice.reason,
                    card,
                    &self.run_state,
                )
            {
                return false;
            }
            seen.push(idx);
        }
        true
    }

    fn shop_purge_is_allowed(&self, shop: &crate::state::shop::ShopState, idx: usize) -> bool {
        shop.purge_available
            && self.run_state.gold >= shop.purge_cost
            && self.run_state.master_deck.get(idx).is_some_and(|card| {
                crate::state::core::master_deck_card_is_purgeable(card)
                    && !crate::state::core::master_deck_card_is_bottled(
                        card,
                        &self.run_state.relics,
                    )
            })
    }

    fn run_level_potion_input_is_allowed(&self, input: &ClientInput) -> bool {
        if !matches!(
            self.engine_state,
            EngineState::MapNavigation
                | EngineState::EventRoom
                | EngineState::RewardScreen(_)
                | EngineState::TreasureRoom(_)
                | EngineState::Campfire
                | EngineState::Shop(_)
                | EngineState::RunPendingChoice(_)
                | EngineState::BossRelicSelect(_)
        ) {
            return false;
        }
        let is_we_meet_again = self
            .run_state
            .event_state
            .as_ref()
            .is_some_and(|event| event.id == crate::state::events::EventId::WeMeetAgain);
        match input {
            ClientInput::DiscardPotion(slot) => {
                crate::content::potions::potion_can_discard_in_event(is_we_meet_again)
                    && self
                        .run_state
                        .potions
                        .get(*slot)
                        .and_then(|slot| slot.as_ref())
                        .is_some_and(|potion| potion.can_discard)
            }
            ClientInput::UsePotion {
                potion_index,
                target,
            } if target.is_none() => self
                .run_state
                .potions
                .get(*potion_index)
                .and_then(|slot| slot.as_ref())
                .is_some_and(|potion| {
                    potion.can_use
                        && crate::content::potions::potion_can_use_out_of_combat(
                            potion.id,
                            is_we_meet_again,
                        )
                }),
            _ => false,
        }
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
        assert_eq!(
            loaded.provenance.source_kind,
            crate::eval::artifact::ArtifactSourceKind::ManualRunControl
        );
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
