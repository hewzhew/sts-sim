use std::path::{Path, PathBuf};

use crate::content::cards::java_id;
use crate::content::monsters::factory::EncounterId;
use crate::engine::run_loop::tick_run;
use crate::eval::combat_capture::{
    capture_combat_position_v1, save_combat_capture_v1, CombatCaptureV1,
};
use crate::fixtures::combat_start_spec::{build_natural_start_state, encounter_id_from_spec};
use crate::runtime::combat::{CombatState, Intent};
use crate::sim::combat::{combat_terminal, stable_boundary, CombatPosition};
use crate::sim::combat_action::combat_action_key;
use crate::sim::combat_legal_actions::get_legal_moves;
use crate::state::core::{CampfireChoice, ClientInput, EngineState, RunResult};
use crate::state::map::node::RoomType;
use crate::state::run::RunState;

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
pub enum RunPlayCommand {
    Noop,
    Help,
    Quit,
    State,
    Actions,
    Capture {
        path: PathBuf,
        label: Option<String>,
    },
    ActionIndex(usize),
    PlayCard {
        card_index: usize,
        target_slot_or_id: Option<usize>,
    },
    UsePotion {
        potion_index: usize,
        target_slot_or_id: Option<usize>,
    },
    Input(ClientInput),
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

    fn current_active_combat_position(&self) -> Result<CombatPosition, String> {
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

    fn current_combat_position_for_actions(&self) -> Result<CombatPosition, String> {
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
        match &self.engine_state {
            EngineState::CombatPlayerTurn
            | EngineState::CombatProcessing
            | EngineState::PendingChoice(_)
                if self.combat_state.is_none() =>
            {
                let room_type =
                    self.run_state.map.get_current_room_type().ok_or_else(|| {
                        "combat engine state without current map room".to_string()
                    })?;
                let encounter = encounter_for_current_room(&mut self.run_state, room_type)?;
                let (engine, combat) =
                    build_natural_start_state(&mut self.run_state, encounter, room_type)?;
                self.engine_state = engine;
                self.combat_state = Some(combat);
            }
            EngineState::EventCombat(event_combat) if self.combat_state.is_none() => {
                let encounter = event_encounter_id(&event_combat.encounter_key)?;
                let room_type = if event_combat.elite_trigger {
                    RoomType::MonsterRoomElite
                } else {
                    RoomType::MonsterRoom
                };
                let (_engine, combat) =
                    build_natural_start_state(&mut self.run_state, encounter, room_type)?;
                self.combat_state = Some(combat);
            }
            _ => {}
        }
        Ok(())
    }
}

pub fn parse_run_play_command(line: &str) -> Result<RunPlayCommand, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(RunPlayCommand::Noop);
    }

    let mut parts = trimmed.split_whitespace();
    let Some(command) = parts.next() else {
        return Ok(RunPlayCommand::Noop);
    };
    let rest = parts.collect::<Vec<_>>();

    match command.to_ascii_lowercase().as_str() {
        "?" | "help" => Ok(RunPlayCommand::Help),
        "q" | "quit" | "exit" => Ok(RunPlayCommand::Quit),
        "state" => Ok(RunPlayCommand::State),
        "actions" | "legal" => Ok(RunPlayCommand::Actions),
        "capture" | "save-capture" => parse_capture_command(&rest),
        "action" => Ok(RunPlayCommand::ActionIndex(parse_usize_arg(
            rest.first(),
            "action index",
        )?)),
        "play" => Ok(RunPlayCommand::PlayCard {
            card_index: parse_usize_arg(rest.first(), "hand card index")?,
            target_slot_or_id: parse_optional_usize_arg(rest.get(1), "target slot")?,
        }),
        "end" => Ok(RunPlayCommand::Input(ClientInput::EndTurn)),
        "potion" => Ok(RunPlayCommand::UsePotion {
            potion_index: parse_usize_arg(rest.first(), "potion slot")?,
            target_slot_or_id: parse_optional_usize_arg(rest.get(1), "target slot")?,
        }),
        "discard-potion" => Ok(RunPlayCommand::Input(ClientInput::DiscardPotion(
            parse_usize_arg(rest.first(), "potion slot")?,
        ))),
        "go" => Ok(RunPlayCommand::Input(ClientInput::SelectMapNode(
            parse_usize_arg(rest.first(), "map x")?,
        ))),
        "fly" => Ok(RunPlayCommand::Input(ClientInput::FlyToNode(
            parse_usize_arg(rest.first(), "map x")?,
            parse_usize_arg(rest.get(1), "map y")?,
        ))),
        "event" | "option" => Ok(RunPlayCommand::Input(ClientInput::EventChoice(
            parse_usize_arg(rest.first(), "event option index")?,
        ))),
        "claim" => Ok(RunPlayCommand::Input(ClientInput::ClaimReward(
            parse_usize_arg(rest.first(), "reward index")?,
        ))),
        "pick" | "card" | "select-card" => Ok(RunPlayCommand::Input(ClientInput::SelectCard(
            parse_usize_arg(rest.first(), "card option index")?,
        ))),
        "select" => Ok(RunPlayCommand::Input(ClientInput::SubmitDeckSelect(
            parse_usize_list(&rest, "deck index")?,
        ))),
        "hand-select" => Ok(RunPlayCommand::Input(ClientInput::SubmitHandSelect(
            parse_u32_list(&rest, "hand card uuid")?,
        ))),
        "grid-select" => Ok(RunPlayCommand::Input(ClientInput::SubmitGridSelect(
            parse_u32_list(&rest, "grid card uuid")?,
        ))),
        "choose" => Ok(RunPlayCommand::Input(ClientInput::SubmitDiscoverChoice(
            parse_usize_arg(rest.first(), "choice index")?,
        ))),
        "proceed" => Ok(RunPlayCommand::Input(ClientInput::Proceed)),
        "cancel" => Ok(RunPlayCommand::Input(ClientInput::Cancel)),
        "open" | "chest" => Ok(RunPlayCommand::Input(ClientInput::OpenChest)),
        "rest" => Ok(RunPlayCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Rest,
        ))),
        "smith" => Ok(RunPlayCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Smith(parse_usize_arg(rest.first(), "deck index")?),
        ))),
        "dig" => Ok(RunPlayCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Dig,
        ))),
        "lift" => Ok(RunPlayCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Lift,
        ))),
        "recall" => Ok(RunPlayCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Recall,
        ))),
        "toke" => Ok(RunPlayCommand::Input(ClientInput::CampfireOption(
            CampfireChoice::Toke(parse_usize_arg(rest.first(), "deck index")?),
        ))),
        "buy" => parse_buy_command(&rest),
        "purge" => Ok(RunPlayCommand::Input(ClientInput::PurgeCard(
            parse_usize_arg(rest.first(), "deck index")?,
        ))),
        "relic" => Ok(RunPlayCommand::Input(ClientInput::SubmitRelicChoice(
            parse_usize_arg(rest.first(), "boss relic index")?,
        ))),
        other => Err(format!("unknown run/play command '{other}'")),
    }
}

pub fn render_run_play_state(session: &RunPlaySession) -> String {
    let mut out = String::new();
    push_line(
        &mut out,
        format!(
            "engine={:?} seed={} act={} floor={} hp={}/{} gold={} deck={} relics={} potions={}",
            session.engine_state,
            session.run_state.seed,
            session.run_state.act_num,
            session.run_state.floor_num,
            session.run_state.current_hp,
            session.run_state.max_hp,
            session.run_state.gold,
            session.run_state.master_deck.len(),
            session.run_state.relics.len(),
            session
                .run_state
                .potions
                .iter()
                .filter(|slot| slot.is_some())
                .count()
        ),
    );

    match &session.engine_state {
        EngineState::MapNavigation => render_map_state(session, &mut out),
        EngineState::EventRoom => render_event_state(session, &mut out),
        EngineState::RewardScreen(reward) => render_reward_state(reward, &mut out),
        EngineState::TreasureRoom(chest) => {
            push_line(&mut out, format!("treasure={chest:?} command=open"));
        }
        EngineState::Campfire => {
            push_line(
                &mut out,
                "campfire commands=rest|smith <deck_idx>|dig|lift|toke <deck_idx>|recall",
            );
        }
        EngineState::Shop(shop) => {
            push_line(
                &mut out,
                format!(
                    "shop cards={} relics={} potions={} purge_cost={} purge_available={}",
                    shop.cards.len(),
                    shop.relics.len(),
                    shop.potions.len(),
                    shop.purge_cost,
                    shop.purge_available
                ),
            );
        }
        EngineState::RunPendingChoice(choice) => {
            push_line(
                &mut out,
                format!(
                    "run_choice reason={:?} min={} max={} command=select <deck_idx...>",
                    choice.reason, choice.min_choices, choice.max_choices
                ),
            );
            render_master_deck(&session.run_state, &mut out);
        }
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_)
        | EngineState::EventCombat(_) => render_combat_state(session, &mut out),
        EngineState::BossRelicSelect(choice) => {
            for (idx, relic) in choice.relics.iter().enumerate() {
                push_line(&mut out, format!("boss_relic[{idx}]={relic:?}"));
            }
        }
        EngineState::GameOver(result) => push_line(&mut out, format!("game_over={result:?}")),
    }

    out
}

pub fn render_combat_actions(session: &RunPlaySession) -> Result<String, String> {
    let position = session.current_combat_position_for_actions()?;
    let actions = get_legal_moves(&position.engine, &position.combat);
    if actions.is_empty() {
        return Ok("no legal combat actions".to_string());
    }

    let mut out = String::new();
    for (idx, action) in actions.iter().enumerate() {
        push_line(
            &mut out,
            format!(
                "action[{idx}] {} {:?}",
                combat_action_key(&position.combat, action),
                action
            ),
        );
    }
    Ok(out)
}

pub fn run_play_help() -> &'static str {
    "commands: state, actions, action <idx>, capture <path> [label], play <hand_idx> [target_slot], end, potion <slot> [target_slot], discard-potion <slot>, go <x>, fly <x> <y>, event <idx>, claim <idx>, pick <idx>, select <deck_idx...>, hand-select <uuid...>, grid-select <uuid...>, open, rest, smith <deck_idx>, buy card|relic|potion <idx>, purge <deck_idx>, relic <idx>, proceed, cancel, quit"
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

fn encounter_for_current_room(
    run_state: &mut RunState,
    room_type: RoomType,
) -> Result<EncounterId, String> {
    match room_type {
        RoomType::MonsterRoom => run_state
            .peek_next_encounter()
            .ok_or_else(|| "normal encounter queue is empty".to_string()),
        RoomType::MonsterRoomElite => run_state
            .peek_next_elite()
            .ok_or_else(|| "elite encounter queue is empty".to_string()),
        RoomType::MonsterRoomBoss => run_state
            .next_boss()
            .ok_or_else(|| "boss encounter queue is empty".to_string()),
        other => Err(format!("room type {other:?} is not combat")),
    }
}

fn event_encounter_id(encounter_key: &str) -> Result<EncounterId, String> {
    match encounter_key {
        "Lagavulin Event" => Ok(EncounterId::LagavulinEvent),
        "The Mushroom Lair" => Ok(EncounterId::TheMushroomLair),
        "Masked Bandits" => Ok(EncounterId::MaskedBandits),
        "Colosseum Slavers" => Ok(EncounterId::ColosseumSlavers),
        "Colosseum Nobs" => Ok(EncounterId::ColosseumNobs),
        "2 Orb Walkers" => Ok(EncounterId::TwoOrbWalkers),
        other => encounter_id_from_spec(other),
    }
}

fn parse_capture_command(rest: &[&str]) -> Result<RunPlayCommand, String> {
    let path = rest
        .first()
        .ok_or_else(|| "capture requires an output path".to_string())?;
    let label = (!rest[1.min(rest.len())..].is_empty()).then(|| rest[1..].join(" "));
    Ok(RunPlayCommand::Capture {
        path: PathBuf::from(path),
        label,
    })
}

fn parse_buy_command(rest: &[&str]) -> Result<RunPlayCommand, String> {
    let kind = rest
        .first()
        .ok_or_else(|| "buy requires card|relic|potion".to_string())?
        .to_ascii_lowercase();
    let index = parse_usize_arg(rest.get(1), "shop index")?;
    match kind.as_str() {
        "card" => Ok(RunPlayCommand::Input(ClientInput::BuyCard(index))),
        "relic" => Ok(RunPlayCommand::Input(ClientInput::BuyRelic(index))),
        "potion" => Ok(RunPlayCommand::Input(ClientInput::BuyPotion(index))),
        _ => Err("buy requires card|relic|potion".to_string()),
    }
}

fn parse_usize_arg(value: Option<&&str>, name: &str) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("missing {name}"))?
        .parse::<usize>()
        .map_err(|_| format!("invalid {name} '{}'", value.unwrap()))
}

fn parse_optional_usize_arg(value: Option<&&str>, name: &str) -> Result<Option<usize>, String> {
    value
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| format!("invalid {name} '{value}'"))
        })
        .transpose()
}

fn parse_usize_list(values: &[&str], name: &str) -> Result<Vec<usize>, String> {
    values
        .iter()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| format!("invalid {name} '{value}'"))
        })
        .collect()
}

fn parse_u32_list(values: &[&str], name: &str) -> Result<Vec<u32>, String> {
    values
        .iter()
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| format!("invalid {name} '{value}'"))
        })
        .collect()
}

fn render_map_state(session: &RunPlaySession, out: &mut String) {
    let target_y = if session.run_state.map.current_y == -1 {
        0
    } else {
        session.run_state.map.current_y + 1
    };
    if target_y == 15 {
        push_line(out, "map target: go 0 -> boss");
        return;
    }
    if target_y < 0 || target_y as usize >= session.run_state.map.graph.len() {
        push_line(out, "map has no next row");
        return;
    }
    for node in &session.run_state.map.graph[target_y as usize] {
        if session.run_state.map.can_travel_to(node.x, node.y, false) {
            push_line(
                out,
                format!("map target: go {} -> y={} {:?}", node.x, node.y, node.class),
            );
        }
    }
}

fn render_event_state(session: &RunPlaySession, out: &mut String) {
    let Some(event) = session.run_state.event_state.as_ref() else {
        push_line(out, "event state missing");
        return;
    };
    push_line(
        out,
        format!(
            "event={:?} screen={} completed={}",
            event.id, event.current_screen, event.completed
        ),
    );
    for (idx, option) in crate::engine::event_handler::get_event_options(&session.run_state)
        .iter()
        .enumerate()
    {
        push_line(
            out,
            format!(
                "event[{idx}] disabled={} label={}",
                option.ui.disabled, option.ui.text
            ),
        );
    }
}

fn render_reward_state(reward: &crate::state::rewards::RewardState, out: &mut String) {
    if let Some(cards) = reward.pending_card_choice.as_ref() {
        for (idx, card) in cards.iter().enumerate() {
            push_line(out, format!("card[{idx}] {:?}+{}", card.id, card.upgrades));
        }
        return;
    }
    for (idx, item) in reward.items.iter().enumerate() {
        push_line(out, format!("reward[{idx}] {item:?}"));
    }
}

fn render_master_deck(run_state: &RunState, out: &mut String) {
    for (idx, card) in run_state.master_deck.iter().enumerate() {
        push_line(
            out,
            format!(
                "deck[{idx}] {}#{}+{}",
                java_id(card.id),
                card.uuid,
                card.upgrades
            ),
        );
    }
}

fn render_combat_state(session: &RunPlaySession, out: &mut String) {
    let Some(combat) = session.combat_state.as_ref() else {
        push_line(out, "combat state missing");
        return;
    };
    let capture_state = session.current_active_combat_position().ok();
    let stable = capture_state
        .as_ref()
        .is_some_and(|position| stable_boundary(&position.engine, &position.combat));
    let terminal = capture_state
        .as_ref()
        .map(|position| combat_terminal(&position.engine, &position.combat));
    push_line(
        out,
        format!(
            "combat stable_capture={} terminal={:?} hp={}/{} block={} energy={} turn={}",
            stable,
            terminal,
            combat.entities.player.current_hp,
            combat.entities.player.max_hp,
            combat.entities.player.block,
            combat.turn.energy,
            combat.turn.turn_count
        ),
    );
    for (idx, card) in combat.zones.hand.iter().enumerate() {
        push_line(
            out,
            format!(
                "hand[{idx}] {}#{}+{} cost={}",
                java_id(card.id),
                card.uuid,
                card.upgrades,
                card.cost_for_turn_java()
            ),
        );
    }
    for monster in &combat.entities.monsters {
        let observation = combat
            .runtime
            .monster_protocol
            .get(&monster.id)
            .map(|protocol| &protocol.observation);
        let turn_plan = monster.turn_plan();
        let intent = observation
            .filter(|obs| obs.visible_intent != Intent::Unknown)
            .map(|obs| format!("{:?}", obs.visible_intent))
            .unwrap_or_else(|| format!("{:?}", turn_plan.summary_spec()));
        let damage = observation
            .filter(|obs| obs.preview_damage_per_hit > 0)
            .map(|obs| obs.preview_damage_per_hit)
            .or_else(|| turn_plan.attack().map(|attack| attack.base_damage))
            .unwrap_or(0);
        push_line(
            out,
            format!(
                "monster[slot={}] id={} type={} hp={}/{} block={} alive={} intent={} dmg={}",
                monster.slot,
                monster.id,
                monster.monster_type,
                monster.current_hp,
                monster.max_hp,
                monster.block,
                monster.is_alive_for_action(),
                intent,
                damage,
            ),
        );
    }
}

fn push_line(out: &mut String, line: impl AsRef<str>) {
    out.push_str(line.as_ref());
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::map::node::{MapEdge, MapRoomNode};
    use crate::state::map::state::MapState;
    use std::fs;
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

    #[test]
    fn run_play_parser_accepts_capture_label() {
        let parsed = parse_run_play_command("capture captures/jaw.json jaw worm start")
            .expect("capture command should parse");

        assert_eq!(
            parsed,
            RunPlayCommand::Capture {
                path: PathBuf::from("captures/jaw.json"),
                label: Some("jaw worm start".to_string()),
            }
        );
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
