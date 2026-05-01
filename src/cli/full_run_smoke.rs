use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::content::cards::CardId;
use crate::content::monsters::factory::{self, EncounterId};
use crate::content::relics::RelicId;
use crate::engine::run_loop::tick_run;
use crate::map::node::RoomType;
use crate::rewards::state::{RewardItem, RewardState};
use crate::runtime::action::Action;
use crate::runtime::combat::{
    CardZones, CombatMeta, CombatRng, CombatState, EngineRuntime, EntityState, TurnRuntime,
};
use crate::runtime::rng::{shuffle_with_random_long, StsRng};
use crate::state::core::{
    CampfireChoice, ClientInput, EngineState, EventCombatState, PendingChoice, PostCombatReturn,
    RunResult,
};
use crate::state::run::RunState;
use crate::state::selection::{SelectionResolution, SelectionScope, SelectionTargetRef};

pub const FULL_RUN_OBSERVATION_SCHEMA_VERSION: &str = "full_run_observation_v0";
pub const FULL_RUN_ACTION_SCHEMA_VERSION: &str = "full_run_action_candidate_set_v0";

#[derive(Clone, Debug)]
pub struct RunBatchConfig {
    pub episodes: usize,
    pub base_seed: u64,
    pub ascension: u8,
    pub final_act: bool,
    pub player_class: &'static str,
    pub max_steps: usize,
    pub trace_dir: Option<PathBuf>,
    pub determinism_check: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunBatchSummary {
    pub observation_schema_version: String,
    pub action_schema_version: String,
    pub action_mask_kind: String,
    pub policy: String,
    pub episodes_requested: usize,
    pub base_seed: u64,
    pub ascension: u8,
    pub final_act: bool,
    pub player_class: String,
    pub max_steps: usize,
    pub episodes_completed: usize,
    pub crash_count: usize,
    pub illegal_action_count: usize,
    pub deterministic_replay_pass_count: usize,
    pub average_floor: f32,
    pub median_floor: f32,
    pub average_steps: f32,
    pub steps_per_second: f32,
    pub episodes_per_hour: f32,
    pub result_counts: std::collections::BTreeMap<String, usize>,
    pub death_floor_counts: std::collections::BTreeMap<String, usize>,
    pub act_counts: std::collections::BTreeMap<String, usize>,
    pub episodes: Vec<RunEpisodeSummary>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunEpisodeSummary {
    pub episode_id: usize,
    pub seed: u64,
    pub result: String,
    pub terminal_reason: String,
    pub floor: i32,
    pub act: u8,
    pub steps: usize,
    pub forced_engine_ticks: usize,
    pub illegal_actions: usize,
    pub crash: Option<String>,
    pub deterministic_replay_pass: Option<bool>,
    pub deterministic_replay_error: Option<String>,
    pub duration_ms: u128,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_size: usize,
    pub relic_count: usize,
    pub trace_path: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunEpisodeTraceFile {
    pub observation_schema_version: String,
    pub action_schema_version: String,
    pub summary: RunEpisodeSummary,
    pub steps: Vec<RunStepTrace>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunStepTrace {
    pub step_index: usize,
    pub floor: i32,
    pub act: u8,
    pub engine_state: String,
    pub decision_type: String,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_size: usize,
    pub relic_count: usize,
    pub legal_action_count: usize,
    pub observation: RunObservationV0,
    pub action_mask: Vec<RunActionCandidate>,
    pub chosen_action_index: usize,
    pub chosen_action_id: u32,
    pub chosen_action_key: String,
    pub chosen_action: TraceClientInput,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunObservationV0 {
    pub schema_version: String,
    pub decision_type: String,
    pub engine_state: String,
    pub act: u8,
    pub floor: i32,
    pub current_room: Option<String>,
    pub current_hp: i32,
    pub max_hp: i32,
    pub hp_ratio_milli: i32,
    pub gold: i32,
    pub deck_size: usize,
    pub relic_count: usize,
    pub potion_slots: usize,
    pub filled_potion_slots: usize,
    pub combat: Option<RunCombatObservationV0>,
    pub screen: RunScreenObservationV0,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunCombatObservationV0 {
    pub player_hp: i32,
    pub player_block: i32,
    pub energy: i32,
    pub turn_count: u32,
    pub hand_count: usize,
    pub draw_count: usize,
    pub discard_count: usize,
    pub exhaust_count: usize,
    pub alive_monster_count: usize,
    pub total_monster_hp: i32,
    pub visible_incoming_damage: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunScreenObservationV0 {
    pub event_option_count: usize,
    pub reward_item_count: usize,
    pub reward_card_choice_count: usize,
    pub shop_card_count: usize,
    pub shop_relic_count: usize,
    pub shop_potion_count: usize,
    pub boss_relic_choice_count: usize,
    pub selection_target_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunActionCandidate {
    pub action_index: usize,
    pub action_id: u32,
    pub action_key: String,
    pub action: TraceClientInput,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TraceSelectionScope {
    Hand,
    Deck,
    Grid,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TraceClientInput {
    PlayCard {
        card_index: usize,
        target: Option<usize>,
    },
    UsePotion {
        potion_index: usize,
        target: Option<usize>,
    },
    DiscardPotion {
        potion_index: usize,
    },
    EndTurn,
    SubmitCardChoice {
        indices: Vec<usize>,
    },
    SubmitDiscoverChoice {
        index: usize,
    },
    SelectMapNode {
        x: usize,
    },
    FlyToNode {
        x: usize,
        y: usize,
    },
    SelectEventOption {
        index: usize,
    },
    CampfireOption {
        choice: TraceCampfireChoice,
    },
    EventChoice {
        index: usize,
    },
    SubmitScryDiscard {
        indices: Vec<usize>,
    },
    SubmitSelection {
        scope: TraceSelectionScope,
        selected_card_uuids: Vec<u32>,
    },
    SubmitHandSelect {
        card_uuids: Vec<u32>,
    },
    SubmitGridSelect {
        card_uuids: Vec<u32>,
    },
    SubmitDeckSelect {
        indices: Vec<usize>,
    },
    ClaimReward {
        index: usize,
    },
    SelectCard {
        index: usize,
    },
    BuyCard {
        index: usize,
    },
    BuyRelic {
        index: usize,
    },
    BuyPotion {
        index: usize,
    },
    PurgeCard {
        index: usize,
    },
    SubmitRelicChoice {
        index: usize,
    },
    Proceed,
    Cancel,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TraceCampfireChoice {
    Rest,
    Smith { deck_index: usize },
    Dig,
    Lift,
    Toke { deck_index: usize },
    Recall,
}

#[derive(Clone, Debug)]
struct EpisodeRun {
    summary: RunEpisodeSummary,
    trace: Vec<RunStepTrace>,
    actions: Vec<ClientInput>,
}

#[derive(Clone, Debug)]
enum EpisodePolicy {
    RandomMasked {
        rng: StsRng,
    },
    Replay {
        actions: Vec<ClientInput>,
        cursor: usize,
    },
}

struct EpisodeContext {
    engine_state: EngineState,
    run_state: RunState,
    combat_state: Option<CombatState>,
    stashed_event_combat: Option<EventCombatState>,
    forced_engine_ticks: usize,
}

pub fn run_batch(config: &RunBatchConfig) -> Result<RunBatchSummary, String> {
    if config.episodes == 0 {
        return Err("episodes must be greater than 0".to_string());
    }
    if config.max_steps == 0 {
        return Err("max_steps must be greater than 0".to_string());
    }
    if let Some(trace_dir) = &config.trace_dir {
        std::fs::create_dir_all(trace_dir).map_err(|err| {
            format!(
                "failed to create trace dir '{}': {err}",
                trace_dir.display()
            )
        })?;
    }

    let batch_start = Instant::now();
    let mut episodes = Vec::new();
    let mut crash_count = 0usize;
    let mut illegal_action_count = 0usize;
    let mut deterministic_replay_pass_count = 0usize;

    for episode_id in 0..config.episodes {
        let seed = config.base_seed.wrapping_add(episode_id as u64);
        let policy_seed = seed ^ 0x9e37_79b9_7f4a_7c15;
        let mut episode = run_episode(
            config,
            episode_id,
            seed,
            EpisodePolicy::RandomMasked {
                rng: StsRng::new(policy_seed),
            },
            true,
        );

        if config.determinism_check {
            let replay = run_episode(
                config,
                episode_id,
                seed,
                EpisodePolicy::Replay {
                    actions: episode.actions.clone(),
                    cursor: 0,
                },
                false,
            );
            let replay_error = deterministic_replay_error(&episode.summary, &replay.summary);
            let passed = replay_error.is_none();
            episode.summary.deterministic_replay_pass = Some(passed);
            episode.summary.deterministic_replay_error = replay_error;
            if passed {
                deterministic_replay_pass_count += 1;
            }
        }

        if let Some(trace_dir) = &config.trace_dir {
            let trace_path = trace_dir.join(format!("episode_{episode_id:04}_seed_{seed}.json"));
            episode.summary.trace_path = Some(trace_path.display().to_string());
            write_trace_file(&trace_path, &episode.summary, &episode.trace)?;
        }

        if episode.summary.crash.is_some() {
            crash_count += 1;
        }
        illegal_action_count += episode.summary.illegal_actions;
        episodes.push(episode.summary);
    }

    let elapsed = batch_start.elapsed().as_secs_f32().max(0.001);
    let total_steps = episodes.iter().map(|episode| episode.steps).sum::<usize>();
    let episodes_completed = episodes
        .iter()
        .filter(|episode| episode.crash.is_none())
        .count();
    let mut floors = episodes
        .iter()
        .map(|episode| episode.floor)
        .collect::<Vec<_>>();
    floors.sort_unstable();
    let average_floor = if floors.is_empty() {
        0.0
    } else {
        floors.iter().sum::<i32>() as f32 / floors.len() as f32
    };
    let median_floor = median_i32(&floors);
    let average_steps = total_steps as f32 / episodes.len().max(1) as f32;
    let result_counts = count_by(episodes.iter().map(|episode| episode.result.clone()));
    let death_floor_counts = count_by(
        episodes
            .iter()
            .filter(|episode| episode.result == "defeat")
            .map(|episode| episode.floor.to_string()),
    );
    let act_counts = count_by(episodes.iter().map(|episode| episode.act.to_string()));

    Ok(RunBatchSummary {
        observation_schema_version: FULL_RUN_OBSERVATION_SCHEMA_VERSION.to_string(),
        action_schema_version: FULL_RUN_ACTION_SCHEMA_VERSION.to_string(),
        action_mask_kind: "per_decision_candidate_set".to_string(),
        policy: "random_masked".to_string(),
        episodes_requested: config.episodes,
        base_seed: config.base_seed,
        ascension: config.ascension,
        final_act: config.final_act,
        player_class: config.player_class.to_string(),
        max_steps: config.max_steps,
        episodes_completed,
        crash_count,
        illegal_action_count,
        deterministic_replay_pass_count,
        average_floor,
        median_floor,
        average_steps,
        steps_per_second: total_steps as f32 / elapsed,
        episodes_per_hour: episodes.len() as f32 / elapsed * 3600.0,
        result_counts,
        death_floor_counts,
        act_counts,
        episodes,
    })
}

fn run_episode(
    config: &RunBatchConfig,
    episode_id: usize,
    seed: u64,
    policy: EpisodePolicy,
    capture_trace: bool,
) -> EpisodeRun {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_episode_inner(config, episode_id, seed, policy, capture_trace)
    }));
    match result {
        Ok(run) => run,
        Err(payload) => {
            let crash = if let Some(value) = payload.downcast_ref::<&str>() {
                (*value).to_string()
            } else if let Some(value) = payload.downcast_ref::<String>() {
                value.clone()
            } else {
                "panic without string payload".to_string()
            };
            EpisodeRun {
                summary: RunEpisodeSummary {
                    episode_id,
                    seed,
                    result: "crash".to_string(),
                    terminal_reason: "panic".to_string(),
                    floor: 0,
                    act: 1,
                    steps: 0,
                    forced_engine_ticks: 0,
                    illegal_actions: 0,
                    crash: Some(crash),
                    deterministic_replay_pass: None,
                    deterministic_replay_error: None,
                    duration_ms: 0,
                    hp: 0,
                    max_hp: 0,
                    gold: 0,
                    deck_size: 0,
                    relic_count: 0,
                    trace_path: None,
                },
                trace: Vec::new(),
                actions: Vec::new(),
            }
        }
    }
}

fn run_episode_inner(
    config: &RunBatchConfig,
    episode_id: usize,
    seed: u64,
    mut policy: EpisodePolicy,
    capture_trace: bool,
) -> EpisodeRun {
    let start = Instant::now();
    let mut ctx = EpisodeContext {
        engine_state: EngineState::EventRoom,
        run_state: RunState::new(
            seed,
            config.ascension,
            config.final_act,
            config.player_class,
        ),
        combat_state: None,
        stashed_event_combat: None,
        forced_engine_ticks: 0,
    };
    let mut trace = Vec::new();
    let mut actions = Vec::new();
    let mut illegal_actions = 0usize;
    let mut crash = None;
    let mut terminal_reason = "step_cap".to_string();

    for step_index in 0..config.max_steps {
        if let Err(err) = prepare_decision_point(&mut ctx, config.max_steps) {
            crash = Some(err);
            terminal_reason = "engine_error".to_string();
            break;
        }

        if matches!(ctx.engine_state, EngineState::GameOver(_)) {
            terminal_reason = "game_over".to_string();
            break;
        }

        let legal_actions = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
        if legal_actions.is_empty() {
            crash = Some(format!(
                "no legal actions at {} on floor {}",
                engine_state_label(&ctx.engine_state),
                ctx.run_state.floor_num
            ));
            terminal_reason = "no_legal_actions".to_string();
            break;
        }

        let (chosen_action_index, action) = match choose_action(&mut policy, &legal_actions) {
            Ok(action) => action,
            Err(err) => {
                illegal_actions += 1;
                crash = Some(err);
                terminal_reason = "illegal_replay_action".to_string();
                break;
            }
        };

        if capture_trace {
            let observation = build_observation(&ctx);
            let action_mask = build_action_candidates(&legal_actions, ctx.combat_state.as_ref());
            let chosen = action_mask
                .get(chosen_action_index)
                .expect("chosen action index should be in legal action mask");
            let chosen_action_id = chosen.action_id;
            let chosen_action_key = chosen.action_key.clone();
            trace.push(RunStepTrace {
                step_index,
                floor: ctx.run_state.floor_num,
                act: ctx.run_state.act_num,
                engine_state: engine_state_label(&ctx.engine_state).to_string(),
                decision_type: decision_type(&ctx.engine_state).to_string(),
                hp: ctx.run_state.current_hp,
                max_hp: ctx.run_state.max_hp,
                gold: ctx.run_state.gold,
                deck_size: ctx.run_state.master_deck.len(),
                relic_count: ctx.run_state.relics.len(),
                legal_action_count: legal_actions.len(),
                observation,
                action_mask,
                chosen_action_index,
                chosen_action_id,
                chosen_action_key,
                chosen_action: trace_input_from_client_input(&action),
            });
        }
        actions.push(action.clone());

        let keep_running = tick_run(
            &mut ctx.engine_state,
            &mut ctx.run_state,
            &mut ctx.combat_state,
            Some(action),
        );
        finish_combat_if_needed(&mut ctx);
        if !keep_running {
            terminal_reason = "engine_stopped".to_string();
            break;
        }
    }

    if crash.is_none() {
        let _ = prepare_decision_point(&mut ctx, config.max_steps);
        if matches!(ctx.engine_state, EngineState::GameOver(_)) {
            terminal_reason = "game_over".to_string();
        }
    }

    let result = match &ctx.engine_state {
        EngineState::GameOver(RunResult::Victory) => "victory",
        EngineState::GameOver(RunResult::Defeat) => "defeat",
        _ if crash.is_some() => "crash",
        _ => "step_cap",
    }
    .to_string();

    EpisodeRun {
        summary: RunEpisodeSummary {
            episode_id,
            seed,
            result,
            terminal_reason,
            floor: ctx.run_state.floor_num,
            act: ctx.run_state.act_num,
            steps: actions.len(),
            forced_engine_ticks: ctx.forced_engine_ticks,
            illegal_actions,
            crash,
            deterministic_replay_pass: None,
            deterministic_replay_error: None,
            duration_ms: start.elapsed().as_millis(),
            hp: ctx.run_state.current_hp,
            max_hp: ctx.run_state.max_hp,
            gold: ctx.run_state.gold,
            deck_size: ctx.run_state.master_deck.len(),
            relic_count: ctx.run_state.relics.len(),
            trace_path: None,
        },
        trace,
        actions,
    }
}

fn prepare_decision_point(ctx: &mut EpisodeContext, max_steps: usize) -> Result<(), String> {
    let forced_cap = max_steps.saturating_mul(10).max(1_000);
    let mut local_ticks = 0usize;
    loop {
        init_combat_if_needed(ctx)?;
        finish_combat_if_needed(ctx);

        if !matches!(ctx.engine_state, EngineState::CombatProcessing) {
            return Ok(());
        }

        let keep_running = tick_run(
            &mut ctx.engine_state,
            &mut ctx.run_state,
            &mut ctx.combat_state,
            None,
        );
        ctx.forced_engine_ticks += 1;
        local_ticks += 1;
        finish_combat_if_needed(ctx);
        if !keep_running || matches!(ctx.engine_state, EngineState::GameOver(_)) {
            return Ok(());
        }
        if local_ticks > forced_cap {
            return Err(format!(
                "forced engine ticks exceeded cap at floor {} state {}",
                ctx.run_state.floor_num,
                engine_state_label(&ctx.engine_state)
            ));
        }
    }
}

fn init_combat_if_needed(ctx: &mut EpisodeContext) -> Result<(), String> {
    if matches!(ctx.engine_state, EngineState::CombatPlayerTurn) && ctx.combat_state.is_none() {
        ctx.combat_state = Some(init_combat(&mut ctx.run_state));
        ctx.engine_state = EngineState::CombatProcessing;
        return Ok(());
    }

    if let EngineState::EventCombat(event_combat) = ctx.engine_state.clone() {
        if ctx.combat_state.is_none() {
            let encounter_id =
                encounter_key_to_id(event_combat.encounter_key).ok_or_else(|| {
                    format!("unknown event combat key '{}'", event_combat.encounter_key)
                })?;
            ctx.stashed_event_combat = Some(event_combat);
            ctx.combat_state = Some(init_event_combat(&mut ctx.run_state, encounter_id));
            ctx.engine_state = EngineState::CombatProcessing;
        }
    }

    Ok(())
}

fn finish_combat_if_needed(ctx: &mut EpisodeContext) {
    if matches!(
        ctx.engine_state,
        EngineState::CombatPlayerTurn
            | EngineState::CombatProcessing
            | EngineState::PendingChoice(_)
            | EngineState::EventCombat(_)
    ) {
        return;
    }

    if ctx.combat_state.is_none() {
        return;
    }
    ctx.combat_state = None;

    let Some(event_combat) = ctx.stashed_event_combat.take() else {
        return;
    };
    if matches!(ctx.engine_state, EngineState::GameOver(_)) {
        return;
    }
    if event_combat.reward_allowed {
        let mut rewards = event_combat.rewards;
        if !event_combat.no_cards_in_rewards {
            if let EngineState::RewardScreen(existing) = &ctx.engine_state {
                for item in &existing.items {
                    if matches!(item, RewardItem::Card { .. }) {
                        rewards.items.push(item.clone());
                    }
                }
            }
        }
        ctx.engine_state = EngineState::RewardScreen(rewards);
    } else {
        ctx.engine_state = match event_combat.post_combat_return {
            PostCombatReturn::EventRoom => EngineState::EventRoom,
            PostCombatReturn::MapNavigation => EngineState::MapNavigation,
        };
    }
}

fn choose_action(
    policy: &mut EpisodePolicy,
    legal_actions: &[ClientInput],
) -> Result<(usize, ClientInput), String> {
    match policy {
        EpisodePolicy::RandomMasked { rng } => {
            let idx = if legal_actions.len() == 1 {
                0
            } else {
                rng.random_range(0, legal_actions.len() as i32 - 1) as usize
            };
            Ok((idx, legal_actions[idx].clone()))
        }
        EpisodePolicy::Replay { actions, cursor } => {
            let action = actions
                .get(*cursor)
                .cloned()
                .ok_or_else(|| format!("replay trace exhausted at action {}", cursor))?;
            *cursor += 1;
            if let Some(index) = legal_actions
                .iter()
                .position(|legal_action| legal_action == &action)
            {
                Ok((index, action))
            } else {
                Err(format!(
                    "replay action {:?} is not legal; legal_count={}",
                    action,
                    legal_actions.len()
                ))
            }
        }
    }
}

fn legal_actions(
    engine_state: &EngineState,
    run_state: &RunState,
    combat_state: &Option<CombatState>,
) -> Vec<ClientInput> {
    match engine_state {
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_) => combat_state
            .as_ref()
            .map(|combat| crate::bot::combat::legal_moves_for_audit(engine_state, combat))
            .unwrap_or_default(),
        EngineState::MapNavigation => legal_map_actions(run_state),
        EngineState::EventRoom => crate::engine::event_handler::get_event_options(run_state)
            .into_iter()
            .enumerate()
            .filter(|(_, option)| !option.ui.disabled)
            .map(|(idx, _)| ClientInput::EventChoice(idx))
            .collect(),
        EngineState::RewardScreen(reward_state) => legal_reward_actions(run_state, reward_state),
        EngineState::BossRelicSelect(state) => {
            let mut actions = (0..state.relics.len())
                .map(ClientInput::SubmitRelicChoice)
                .collect::<Vec<_>>();
            actions.push(ClientInput::Proceed);
            actions
        }
        EngineState::Campfire => legal_campfire_actions(run_state),
        EngineState::Shop(shop) => legal_shop_actions(run_state, shop),
        EngineState::RunPendingChoice(choice) => {
            let request = choice.selection_request(run_state);
            legal_selection_actions(&request)
        }
        EngineState::CombatProcessing | EngineState::EventCombat(_) | EngineState::GameOver(_) => {
            Vec::new()
        }
    }
}

fn legal_map_actions(run_state: &RunState) -> Vec<ClientInput> {
    let next_y = if run_state.map.current_y == -1 {
        0
    } else {
        run_state.map.current_y + 1
    };
    if run_state.map.current_y == 14 {
        return vec![ClientInput::SelectMapNode(0)];
    }

    let mut actions = Vec::new();
    if next_y <= run_state.map.graph.len() as i32 {
        for x in 0..7 {
            if run_state.map.can_travel_to(x, next_y, false) {
                actions.push(ClientInput::SelectMapNode(x as usize));
            }
        }
    }
    actions
}

fn legal_reward_actions(run_state: &RunState, reward_state: &RewardState) -> Vec<ClientInput> {
    if let Some(cards) = &reward_state.pending_card_choice {
        let mut actions = (0..cards.len())
            .map(ClientInput::SelectCard)
            .collect::<Vec<_>>();
        if run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::SingingBowl)
        {
            actions.push(ClientInput::SelectCard(cards.len()));
        }
        actions.push(ClientInput::Proceed);
        return actions;
    }

    let mut actions = Vec::new();
    for (idx, item) in reward_state.items.iter().enumerate() {
        let claimable = match item {
            RewardItem::Potion { .. } => {
                run_state
                    .relics
                    .iter()
                    .any(|relic| relic.id == RelicId::Sozu)
                    || run_state.potions.iter().any(Option::is_none)
            }
            _ => true,
        };
        if claimable {
            actions.push(ClientInput::ClaimReward(idx));
        }
    }
    actions.push(ClientInput::Proceed);
    actions
}

fn legal_shop_actions(run_state: &RunState, shop: &crate::shop::ShopState) -> Vec<ClientInput> {
    let mut actions = vec![ClientInput::Proceed];
    for (idx, card) in shop.cards.iter().enumerate() {
        if card.can_buy && run_state.gold >= card.price {
            actions.push(ClientInput::BuyCard(idx));
        }
    }
    for (idx, relic) in shop.relics.iter().enumerate() {
        if relic.can_buy && run_state.gold >= relic.price {
            actions.push(ClientInput::BuyRelic(idx));
        }
    }
    let has_empty_potion_slot = run_state.potions.iter().any(Option::is_none);
    let has_sozu = run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::Sozu);
    for (idx, potion) in shop.potions.iter().enumerate() {
        if potion.can_buy && run_state.gold >= potion.price && (has_empty_potion_slot || has_sozu) {
            actions.push(ClientInput::BuyPotion(idx));
        }
    }
    if shop.purge_available && run_state.gold >= shop.purge_cost {
        for idx in 0..run_state.master_deck.len() {
            actions.push(ClientInput::PurgeCard(idx));
        }
    }
    actions
}

fn legal_campfire_actions(run_state: &RunState) -> Vec<ClientInput> {
    let available = crate::engine::campfire_handler::get_available_options(run_state);
    let mut actions = Vec::new();
    for choice in available {
        match choice {
            CampfireChoice::Smith(_) => {
                for (idx, card) in run_state.master_deck.iter().enumerate() {
                    if card.id == CardId::SearingBlow || card.upgrades == 0 {
                        actions.push(ClientInput::CampfireOption(CampfireChoice::Smith(idx)));
                    }
                }
            }
            CampfireChoice::Toke(_) => {
                for idx in 0..run_state.master_deck.len() {
                    actions.push(ClientInput::CampfireOption(CampfireChoice::Toke(idx)));
                }
            }
            other => actions.push(ClientInput::CampfireOption(other)),
        }
    }
    actions
}

fn legal_selection_actions(
    request: &crate::state::selection::SelectionRequest,
) -> Vec<ClientInput> {
    let (min, max) = selection_bounds(request);
    let targets = request.targets.clone();
    let mut actions = Vec::new();
    if request.can_cancel || min == 0 {
        actions.push(ClientInput::Cancel);
    }
    let max_actions = 128usize;
    let max_take = max.min(targets.len());
    for take in min..=max_take {
        if take == 0 {
            continue;
        }
        let mut current = Vec::new();
        push_selection_combinations(
            request.scope,
            &targets,
            take,
            0,
            &mut current,
            &mut actions,
            max_actions,
        );
        if actions.len() >= max_actions {
            break;
        }
    }
    actions
}

fn selection_bounds(request: &crate::state::selection::SelectionRequest) -> (usize, usize) {
    match request.constraint {
        crate::state::selection::SelectionConstraint::Exactly(n) => (n, n),
        crate::state::selection::SelectionConstraint::Between { min, max } => (min, max),
        crate::state::selection::SelectionConstraint::UpToAvailable => (1, request.targets.len()),
        crate::state::selection::SelectionConstraint::OptionalUpToAvailable => {
            (0, request.targets.len())
        }
    }
}

fn push_selection_combinations(
    scope: SelectionScope,
    targets: &[SelectionTargetRef],
    take: usize,
    start: usize,
    current: &mut Vec<SelectionTargetRef>,
    out: &mut Vec<ClientInput>,
    max_actions: usize,
) {
    if out.len() >= max_actions {
        return;
    }
    if current.len() == take {
        out.push(ClientInput::SubmitSelection(SelectionResolution {
            scope,
            selected: current.clone(),
        }));
        return;
    }
    for idx in start..targets.len() {
        current.push(targets[idx]);
        push_selection_combinations(scope, targets, take, idx + 1, current, out, max_actions);
        current.pop();
        if out.len() >= max_actions {
            return;
        }
    }
}

fn build_observation(ctx: &EpisodeContext) -> RunObservationV0 {
    let combat = ctx.combat_state.as_ref();
    let active_hp = combat
        .map(|combat| combat.entities.player.current_hp)
        .unwrap_or(ctx.run_state.current_hp);
    let active_max_hp = combat
        .map(|combat| combat.entities.player.max_hp)
        .unwrap_or(ctx.run_state.max_hp);

    RunObservationV0 {
        schema_version: FULL_RUN_OBSERVATION_SCHEMA_VERSION.to_string(),
        decision_type: decision_type(&ctx.engine_state).to_string(),
        engine_state: engine_state_label(&ctx.engine_state).to_string(),
        act: ctx.run_state.act_num,
        floor: ctx.run_state.floor_num,
        current_room: ctx
            .run_state
            .map
            .get_current_room_type()
            .map(|room_type| format!("{room_type:?}")),
        current_hp: active_hp,
        max_hp: active_max_hp,
        hp_ratio_milli: if active_max_hp > 0 {
            active_hp * 1000 / active_max_hp
        } else {
            0
        },
        gold: ctx.run_state.gold,
        deck_size: ctx.run_state.master_deck.len(),
        relic_count: ctx.run_state.relics.len(),
        potion_slots: ctx.run_state.potions.len(),
        filled_potion_slots: ctx
            .run_state
            .potions
            .iter()
            .filter(|slot| slot.is_some())
            .count(),
        combat: combat.map(build_combat_observation),
        screen: build_screen_observation(&ctx.engine_state, &ctx.run_state),
    }
}

fn build_combat_observation(combat: &CombatState) -> RunCombatObservationV0 {
    let alive_monsters = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .collect::<Vec<_>>();
    let visible_incoming_damage = alive_monsters
        .iter()
        .map(|monster| {
            crate::projection::combat::monster_preview_total_damage_in_combat(combat, monster)
        })
        .sum();

    RunCombatObservationV0 {
        player_hp: combat.entities.player.current_hp,
        player_block: combat.entities.player.block,
        energy: combat.turn.energy as i32,
        turn_count: combat.turn.turn_count,
        hand_count: combat.zones.hand.len(),
        draw_count: combat.zones.draw_pile.len(),
        discard_count: combat.zones.discard_pile.len(),
        exhaust_count: combat.zones.exhaust_pile.len(),
        alive_monster_count: alive_monsters.len(),
        total_monster_hp: alive_monsters
            .iter()
            .map(|monster| monster.current_hp.max(0))
            .sum(),
        visible_incoming_damage,
    }
}

fn build_screen_observation(
    engine_state: &EngineState,
    run_state: &RunState,
) -> RunScreenObservationV0 {
    match engine_state {
        EngineState::EventRoom => RunScreenObservationV0 {
            event_option_count: crate::engine::event_handler::get_event_options(run_state)
                .iter()
                .filter(|option| !option.ui.disabled)
                .count(),
            ..empty_screen_observation()
        },
        EngineState::RewardScreen(reward_state) => RunScreenObservationV0 {
            reward_item_count: reward_state.items.len(),
            reward_card_choice_count: reward_state
                .pending_card_choice
                .as_ref()
                .map(Vec::len)
                .unwrap_or(0),
            ..empty_screen_observation()
        },
        EngineState::Shop(shop) => RunScreenObservationV0 {
            shop_card_count: shop.cards.len(),
            shop_relic_count: shop.relics.len(),
            shop_potion_count: shop.potions.len(),
            ..empty_screen_observation()
        },
        EngineState::BossRelicSelect(state) => RunScreenObservationV0 {
            boss_relic_choice_count: state.relics.len(),
            ..empty_screen_observation()
        },
        EngineState::RunPendingChoice(choice) => RunScreenObservationV0 {
            selection_target_count: choice.selection_request(run_state).targets.len(),
            ..empty_screen_observation()
        },
        EngineState::PendingChoice(choice) => RunScreenObservationV0 {
            selection_target_count: choice
                .selection_request()
                .map(|request| request.targets.len())
                .unwrap_or(0),
            ..empty_screen_observation()
        },
        _ => empty_screen_observation(),
    }
}

fn empty_screen_observation() -> RunScreenObservationV0 {
    RunScreenObservationV0 {
        event_option_count: 0,
        reward_item_count: 0,
        reward_card_choice_count: 0,
        shop_card_count: 0,
        shop_relic_count: 0,
        shop_potion_count: 0,
        boss_relic_choice_count: 0,
        selection_target_count: 0,
    }
}

fn build_action_candidates(
    legal_actions: &[ClientInput],
    combat: Option<&CombatState>,
) -> Vec<RunActionCandidate> {
    legal_actions
        .iter()
        .enumerate()
        .map(|(action_index, action)| {
            let action_key = action_key_for_input(action, combat);
            RunActionCandidate {
                action_index,
                action_id: stable_action_id(&action_key),
                action_key,
                action: trace_input_from_client_input(action),
            }
        })
        .collect()
}

fn action_key_for_input(input: &ClientInput, combat: Option<&CombatState>) -> String {
    match input {
        ClientInput::PlayCard { card_index, target } => format!(
            "combat/play_card/hand:{card_index}/target:{}",
            target_label(*target, combat)
        ),
        ClientInput::UsePotion {
            potion_index,
            target,
        } => format!(
            "combat/use_potion/slot:{potion_index}/target:{}",
            target_label(*target, combat)
        ),
        ClientInput::DiscardPotion(index) => format!("combat/discard_potion/slot:{index}"),
        ClientInput::EndTurn => "combat/end_turn".to_string(),
        ClientInput::SubmitCardChoice(indices) => format!("combat/card_choice/{indices:?}"),
        ClientInput::SubmitDiscoverChoice(index) => format!("choice/discover/{index}"),
        ClientInput::SelectMapNode(x) => format!("map/select_x/{x}"),
        ClientInput::FlyToNode(x, y) => format!("map/fly/x:{x}/y:{y}"),
        ClientInput::SelectEventOption(index) => format!("event/select_option/{index}"),
        ClientInput::CampfireOption(choice) => format!("campfire/{}", campfire_choice_key(choice)),
        ClientInput::EventChoice(index) => format!("event/choice/{index}"),
        ClientInput::SubmitScryDiscard(indices) => format!("combat/scry_discard/{indices:?}"),
        ClientInput::SubmitSelection(selection) => format!(
            "selection/{}/uuids:{}",
            selection_scope_key(selection.scope),
            selection_uuid_key(&selection.selected)
        ),
        ClientInput::SubmitHandSelect(uuids) => {
            format!("combat/hand_select/uuids:{}", uuid_list_key(uuids))
        }
        ClientInput::SubmitGridSelect(uuids) => {
            format!("combat/grid_select/uuids:{}", uuid_list_key(uuids))
        }
        ClientInput::SubmitDeckSelect(indices) => format!("deck/select_indices/{indices:?}"),
        ClientInput::ClaimReward(index) => format!("reward/claim/{index}"),
        ClientInput::SelectCard(index) => format!("reward/select_card/{index}"),
        ClientInput::BuyCard(index) => format!("shop/buy_card/{index}"),
        ClientInput::BuyRelic(index) => format!("shop/buy_relic/{index}"),
        ClientInput::BuyPotion(index) => format!("shop/buy_potion/{index}"),
        ClientInput::PurgeCard(index) => format!("shop/purge_card/{index}"),
        ClientInput::SubmitRelicChoice(index) => format!("boss_relic/select/{index}"),
        ClientInput::Proceed => "proceed".to_string(),
        ClientInput::Cancel => "cancel".to_string(),
    }
}

fn target_label(target: Option<usize>, combat: Option<&CombatState>) -> String {
    match target {
        None => "none".to_string(),
        Some(entity_id) => combat
            .and_then(|combat| {
                combat
                    .entities
                    .monsters
                    .iter()
                    .position(|monster| monster.id == entity_id)
            })
            .map(|slot| format!("monster_slot:{slot}"))
            .unwrap_or_else(|| format!("entity:{entity_id}")),
    }
}

fn campfire_choice_key(choice: &CampfireChoice) -> String {
    match choice {
        CampfireChoice::Rest => "rest".to_string(),
        CampfireChoice::Smith(idx) => format!("smith/{idx}"),
        CampfireChoice::Dig => "dig".to_string(),
        CampfireChoice::Lift => "lift".to_string(),
        CampfireChoice::Toke(idx) => format!("toke/{idx}"),
        CampfireChoice::Recall => "recall".to_string(),
    }
}

fn selection_scope_key(scope: SelectionScope) -> &'static str {
    match scope {
        SelectionScope::Hand => "hand",
        SelectionScope::Deck => "deck",
        SelectionScope::Grid => "grid",
    }
}

fn selection_uuid_key(selected: &[SelectionTargetRef]) -> String {
    let uuids = selected
        .iter()
        .map(|target| match target {
            SelectionTargetRef::CardUuid(uuid) => *uuid,
        })
        .collect::<Vec<_>>();
    uuid_list_key(&uuids)
}

fn uuid_list_key(uuids: &[u32]) -> String {
    uuids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn stable_action_id(action_key: &str) -> u32 {
    let mut hash = 2_166_136_261u32;
    for byte in action_key.as_bytes() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

fn init_combat(run_state: &mut RunState) -> CombatState {
    let encounter_id = if let Some(room_type) = run_state.map.get_current_room_type() {
        match room_type {
            RoomType::MonsterRoomElite => run_state.next_elite().unwrap_or(EncounterId::JawWorm),
            RoomType::MonsterRoomBoss => run_state.next_boss().unwrap_or(EncounterId::Hexaghost),
            _ => run_state.next_encounter().unwrap_or(EncounterId::JawWorm),
        }
    } else {
        run_state.next_encounter().unwrap_or(EncounterId::JawWorm)
    };
    let mut combat = build_combat_state(run_state, encounter_id);
    if let Some(room_type) = run_state.map.get_current_room_type() {
        combat.meta.is_boss_fight = room_type == RoomType::MonsterRoomBoss;
        combat.meta.is_elite_fight = room_type == RoomType::MonsterRoomElite;
    }
    combat
}

fn init_event_combat(run_state: &mut RunState, encounter_id: EncounterId) -> CombatState {
    build_combat_state(run_state, encounter_id)
}

fn build_combat_state(run_state: &mut RunState, encounter_id: EncounterId) -> CombatState {
    let player = run_state.build_combat_player(0);
    let monsters = factory::build_encounter(
        encounter_id,
        &mut run_state.rng_pool.misc_rng,
        &mut run_state.rng_pool.monster_hp_rng,
        run_state.ascension_level,
    );

    let mut combat = CombatState {
        meta: CombatMeta {
            ascension_level: run_state.ascension_level,
            player_class: run_state.player_class,
            is_boss_fight: false,
            is_elite_fight: false,
            meta_changes: Vec::new(),
        },
        turn: TurnRuntime::fresh_player_turn(3),
        zones: CardZones {
            draw_pile: run_state.master_deck.clone(),
            hand: Vec::new(),
            discard_pile: Vec::new(),
            exhaust_pile: Vec::new(),
            limbo: Vec::new(),
            queued_cards: std::collections::VecDeque::new(),
            card_uuid_counter: 9999,
        },
        entities: EntityState {
            player,
            monsters,
            potions: run_state.potions.clone(),
            power_db: std::collections::HashMap::new(),
        },
        engine: EngineRuntime::new(),
        rng: CombatRng::new(run_state.rng_pool.clone()),
        runtime: Default::default(),
    };

    initialize_monster_intents(&mut combat);
    combat.reset_turn_energy_from_player();
    shuffle_with_random_long(&mut combat.zones.draw_pile, &mut combat.rng.shuffle_rng);
    move_innate_cards_to_front(&mut combat);
    combat.queue_action_back(Action::PreBattleTrigger);
    combat
}

fn initialize_monster_intents(combat: &mut CombatState) {
    let monsters_clone = combat.entities.monsters.clone();
    let player_powers = crate::content::powers::store::powers_snapshot_for(combat, 0);
    let monster_ids = combat
        .entities
        .monsters
        .iter()
        .map(|monster| monster.id)
        .collect::<Vec<_>>();

    for monster_id in monster_ids {
        let entity_snapshot = combat
            .entities
            .monsters
            .iter()
            .find(|monster| monster.id == monster_id)
            .cloned()
            .expect("initial monster should exist while rolling intent");
        let num = combat.rng.ai_rng.random(99);
        let outcome = crate::content::monsters::roll_monster_turn_outcome(
            &mut combat.rng.ai_rng,
            &entity_snapshot,
            combat.meta.ascension_level,
            num,
            &monsters_clone,
            &player_powers,
        );
        for action in outcome.setup_actions {
            crate::engine::action_handlers::execute_action(action, combat);
        }
        let plan = outcome.plan;
        let monster = combat
            .entities
            .monsters
            .iter_mut()
            .find(|monster| monster.id == monster_id)
            .expect("rolled monster should still exist");
        monster.set_planned_move_id(plan.move_id);
        monster.set_planned_steps(plan.steps);
        monster.set_planned_visible_spec(plan.visible_spec);
        monster.move_history_mut().push_back(plan.move_id);
        combat
            .runtime
            .monster_protocol
            .entry(monster_id)
            .or_default()
            .observation = Default::default();
    }
}

fn move_innate_cards_to_front(combat: &mut CombatState) {
    let mut innate_cards = Vec::new();
    let mut normal_cards = Vec::new();
    for card in std::mem::take(&mut combat.zones.draw_pile) {
        if crate::content::cards::is_innate_card(&card) {
            innate_cards.push(card);
        } else {
            normal_cards.push(card);
        }
    }
    innate_cards.extend(normal_cards);
    combat.zones.draw_pile = innate_cards;
}

fn encounter_key_to_id(key: &str) -> Option<EncounterId> {
    match key {
        "Colosseum Slavers" => Some(EncounterId::ColosseumSlavers),
        "Colosseum Nobs" => Some(EncounterId::ColosseumNobs),
        "3 Bandits" => Some(EncounterId::MaskedBandits),
        "Dead Adventurer" => Some(EncounterId::LagavulinEvent),
        "3 Fungi Beasts" => Some(EncounterId::TheMushroomLair),
        "2 Orb Walkers" => Some(EncounterId::TwoOrbWalkers),
        "Mind Bloom Boss" => Some(EncounterId::AwakenedOne),
        _ => None,
    }
}

fn trace_input_from_client_input(input: &ClientInput) -> TraceClientInput {
    match input {
        ClientInput::PlayCard { card_index, target } => TraceClientInput::PlayCard {
            card_index: *card_index,
            target: *target,
        },
        ClientInput::UsePotion {
            potion_index,
            target,
        } => TraceClientInput::UsePotion {
            potion_index: *potion_index,
            target: *target,
        },
        ClientInput::DiscardPotion(index) => TraceClientInput::DiscardPotion {
            potion_index: *index,
        },
        ClientInput::EndTurn => TraceClientInput::EndTurn,
        ClientInput::SubmitCardChoice(indices) => TraceClientInput::SubmitCardChoice {
            indices: indices.clone(),
        },
        ClientInput::SubmitDiscoverChoice(index) => {
            TraceClientInput::SubmitDiscoverChoice { index: *index }
        }
        ClientInput::SelectMapNode(x) => TraceClientInput::SelectMapNode { x: *x },
        ClientInput::FlyToNode(x, y) => TraceClientInput::FlyToNode { x: *x, y: *y },
        ClientInput::SelectEventOption(index) => {
            TraceClientInput::SelectEventOption { index: *index }
        }
        ClientInput::CampfireOption(choice) => TraceClientInput::CampfireOption {
            choice: trace_campfire_choice(*choice),
        },
        ClientInput::EventChoice(index) => TraceClientInput::EventChoice { index: *index },
        ClientInput::SubmitScryDiscard(indices) => TraceClientInput::SubmitScryDiscard {
            indices: indices.clone(),
        },
        ClientInput::SubmitSelection(selection) => TraceClientInput::SubmitSelection {
            scope: trace_selection_scope(selection.scope),
            selected_card_uuids: selection
                .selected
                .iter()
                .map(|target| match target {
                    SelectionTargetRef::CardUuid(uuid) => *uuid,
                })
                .collect(),
        },
        ClientInput::SubmitHandSelect(card_uuids) => TraceClientInput::SubmitHandSelect {
            card_uuids: card_uuids.clone(),
        },
        ClientInput::SubmitGridSelect(card_uuids) => TraceClientInput::SubmitGridSelect {
            card_uuids: card_uuids.clone(),
        },
        ClientInput::SubmitDeckSelect(indices) => TraceClientInput::SubmitDeckSelect {
            indices: indices.clone(),
        },
        ClientInput::ClaimReward(index) => TraceClientInput::ClaimReward { index: *index },
        ClientInput::SelectCard(index) => TraceClientInput::SelectCard { index: *index },
        ClientInput::BuyCard(index) => TraceClientInput::BuyCard { index: *index },
        ClientInput::BuyRelic(index) => TraceClientInput::BuyRelic { index: *index },
        ClientInput::BuyPotion(index) => TraceClientInput::BuyPotion { index: *index },
        ClientInput::PurgeCard(index) => TraceClientInput::PurgeCard { index: *index },
        ClientInput::SubmitRelicChoice(index) => {
            TraceClientInput::SubmitRelicChoice { index: *index }
        }
        ClientInput::Proceed => TraceClientInput::Proceed,
        ClientInput::Cancel => TraceClientInput::Cancel,
    }
}

fn trace_campfire_choice(choice: CampfireChoice) -> TraceCampfireChoice {
    match choice {
        CampfireChoice::Rest => TraceCampfireChoice::Rest,
        CampfireChoice::Smith(deck_index) => TraceCampfireChoice::Smith { deck_index },
        CampfireChoice::Dig => TraceCampfireChoice::Dig,
        CampfireChoice::Lift => TraceCampfireChoice::Lift,
        CampfireChoice::Toke(deck_index) => TraceCampfireChoice::Toke { deck_index },
        CampfireChoice::Recall => TraceCampfireChoice::Recall,
    }
}

fn trace_selection_scope(scope: SelectionScope) -> TraceSelectionScope {
    match scope {
        SelectionScope::Hand => TraceSelectionScope::Hand,
        SelectionScope::Deck => TraceSelectionScope::Deck,
        SelectionScope::Grid => TraceSelectionScope::Grid,
    }
}

fn engine_state_label(engine_state: &EngineState) -> &'static str {
    match engine_state {
        EngineState::CombatPlayerTurn => "combat_player_turn",
        EngineState::CombatProcessing => "combat_processing",
        EngineState::RewardScreen(_) => "reward_screen",
        EngineState::Campfire => "campfire",
        EngineState::Shop(_) => "shop",
        EngineState::MapNavigation => "map_navigation",
        EngineState::EventRoom => "event_room",
        EngineState::PendingChoice(_) => "pending_choice",
        EngineState::RunPendingChoice(_) => "run_pending_choice",
        EngineState::EventCombat(_) => "event_combat",
        EngineState::BossRelicSelect(_) => "boss_relic_select",
        EngineState::GameOver(_) => "game_over",
    }
}

fn decision_type(engine_state: &EngineState) -> &'static str {
    match engine_state {
        EngineState::CombatPlayerTurn => "combat",
        EngineState::PendingChoice(PendingChoice::HandSelect { .. }) => "combat_hand_select",
        EngineState::PendingChoice(PendingChoice::GridSelect { .. }) => "combat_grid_select",
        EngineState::PendingChoice(PendingChoice::DiscoverySelect(_)) => "combat_discovery",
        EngineState::PendingChoice(PendingChoice::ScrySelect { .. }) => "combat_scry",
        EngineState::PendingChoice(PendingChoice::CardRewardSelect { .. }) => "combat_card_reward",
        EngineState::PendingChoice(PendingChoice::StanceChoice) => "combat_stance",
        EngineState::RewardScreen(_) => "reward",
        EngineState::Campfire => "campfire",
        EngineState::Shop(_) => "shop",
        EngineState::MapNavigation => "map",
        EngineState::EventRoom => "event",
        EngineState::RunPendingChoice(_) => "run_deck_selection",
        EngineState::BossRelicSelect(_) => "boss_relic",
        EngineState::CombatProcessing | EngineState::EventCombat(_) | EngineState::GameOver(_) => {
            "none"
        }
    }
}

fn deterministic_replay_error(
    primary: &RunEpisodeSummary,
    replay: &RunEpisodeSummary,
) -> Option<String> {
    let mismatches = [
        ("result", primary.result.clone(), replay.result.clone()),
        (
            "terminal_reason",
            primary.terminal_reason.clone(),
            replay.terminal_reason.clone(),
        ),
        ("floor", primary.floor.to_string(), replay.floor.to_string()),
        ("act", primary.act.to_string(), replay.act.to_string()),
        ("steps", primary.steps.to_string(), replay.steps.to_string()),
        ("hp", primary.hp.to_string(), replay.hp.to_string()),
        (
            "deck_size",
            primary.deck_size.to_string(),
            replay.deck_size.to_string(),
        ),
    ]
    .into_iter()
    .filter_map(|(field, left, right)| {
        if left == right {
            None
        } else {
            Some(format!("{field}: primary={left} replay={right}"))
        }
    })
    .collect::<Vec<_>>();

    if replay.crash.is_some() && primary.crash != replay.crash {
        return Some(format!(
            "replay crashed differently: primary={:?} replay={:?}",
            primary.crash, replay.crash
        ));
    }

    if mismatches.is_empty() {
        None
    } else {
        Some(mismatches.join("; "))
    }
}

fn write_trace_file(
    path: &Path,
    summary: &RunEpisodeSummary,
    steps: &[RunStepTrace],
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create trace parent '{}': {err}",
                parent.display()
            )
        })?;
    }
    let trace = RunEpisodeTraceFile {
        observation_schema_version: FULL_RUN_OBSERVATION_SCHEMA_VERSION.to_string(),
        action_schema_version: FULL_RUN_ACTION_SCHEMA_VERSION.to_string(),
        summary: summary.clone(),
        steps: steps.to_vec(),
    };
    std::fs::write(
        path,
        serde_json::to_string_pretty(&trace)
            .map_err(|err| format!("failed to serialize trace: {err}"))?,
    )
    .map_err(|err| format!("failed to write trace '{}': {err}", path.display()))
}

fn median_i32(values: &[i32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) as f32 / 2.0
    } else {
        values[mid] as f32
    }
}

fn count_by<I>(values: I) -> std::collections::BTreeMap<String, usize>
where
    I: IntoIterator<Item = String>,
{
    let mut counts = std::collections::BTreeMap::new();
    for value in values {
        *counts.entry(value).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_key_distinguishes_parametric_choices() {
        let left = ClientInput::PlayCard {
            card_index: 0,
            target: None,
        };
        let right = ClientInput::PlayCard {
            card_index: 1,
            target: None,
        };

        let left_key = action_key_for_input(&left, None);
        let right_key = action_key_for_input(&right, None);
        assert_ne!(left_key, right_key);
        assert_ne!(stable_action_id(&left_key), stable_action_id(&right_key));
    }

    #[test]
    fn action_candidate_records_schema_visible_action_id_and_trace_input() {
        let legal_actions = vec![ClientInput::EndTurn, ClientInput::SelectMapNode(3)];
        let candidates = build_action_candidates(&legal_actions, None);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].action_index, 0);
        assert_eq!(candidates[0].action_key, "combat/end_turn");
        assert_eq!(candidates[0].action_id, stable_action_id("combat/end_turn"));
        assert!(matches!(candidates[0].action, TraceClientInput::EndTurn));
        assert_eq!(candidates[1].action_key, "map/select_x/3");
    }

    #[test]
    fn run_batch_summary_exposes_contract_versions() {
        let config = RunBatchConfig {
            episodes: 1,
            base_seed: 42,
            ascension: 0,
            final_act: false,
            player_class: "Ironclad",
            max_steps: 50,
            trace_dir: None,
            determinism_check: true,
        };

        let summary = run_batch(&config).expect("one episode smoke should run");
        assert_eq!(
            summary.observation_schema_version,
            FULL_RUN_OBSERVATION_SCHEMA_VERSION
        );
        assert_eq!(
            summary.action_schema_version,
            FULL_RUN_ACTION_SCHEMA_VERSION
        );
        assert_eq!(summary.action_mask_kind, "per_decision_candidate_set");
        assert_eq!(summary.deterministic_replay_pass_count, 1);
    }
}
