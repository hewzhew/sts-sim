use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::bot::card_disposition::{
    build_context as build_card_role_context, classify_hand_card_with_context,
    combat_copy_score_for_uuid, combat_exhaust_score_for_uuid, combat_fuel_score_for_uuid,
    combat_retention_score_for_uuid, HandCardRole,
};
use crate::content::cards::{CardId, CardRarity, CardType};
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
use crate::state::selection::EngineDiagnosticSeverity;
use crate::state::selection::{SelectionResolution, SelectionScope, SelectionTargetRef};


mod types;
pub use types::*;

mod trace;
mod reward;
mod batch;
mod bot;
mod actions;
mod observation;
mod features;

pub use trace::*;
pub use reward::*;
pub use batch::*;
pub use bot::*;
pub use actions::*;
pub use observation::*;
pub use features::*;


impl FullRunEnv {
    pub fn new(config: FullRunEnvConfig) -> Result<Self, String> {
        if config.max_steps == 0 {
            return Err("max_steps must be greater than 0".to_string());
        }
        let ctx = EpisodeContext {
            engine_state: EngineState::EventRoom,
            run_state: RunState::new(
                config.seed,
                config.ascension,
                config.final_act,
                config.player_class,
            ),
            combat_state: None,
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };
        let mut env = Self {
            config,
            ctx,
            steps: 0,
            done: false,
            terminal_reason: "running".to_string(),
            crash: None,
            contract_failure: None,
            no_progress_tracker: NoProgressTracker::new(),
        };
        let _ = env.prepare_state()?;
        Ok(env)
    }

    pub fn state(&mut self) -> Result<FullRunEnvState, String> {
        self.prepare_state()
    }

    pub fn step_policy(&mut self, policy: RunPolicyKind) -> Result<FullRunEnvStep, String> {
        if self.done {
            return self.step(0);
        }
        let _ = self.prepare_state()?;
        if self.done {
            return self.step(0);
        }
        let legal_actions = legal_actions(
            &self.ctx.engine_state,
            &self.ctx.run_state,
            &self.ctx.combat_state,
        );
        if legal_actions.is_empty() {
            return Err("no legal actions available for policy step".to_string());
        }
        let action_index = match policy {
            RunPolicyKind::RuleBaselineV0 => choose_rule_baseline_action(&self.ctx, &legal_actions),
            RunPolicyKind::PlanQueryV0 => choose_plan_query_action(&self.ctx, &legal_actions)
                .unwrap_or_else(|| choose_rule_baseline_action(&self.ctx, &legal_actions)),
            RunPolicyKind::RandomMasked => {
                return Err(
                    "random_masked policy step is not stateful in FullRunEnv; choose a legal index externally"
                        .to_string(),
                )
            }
        };
        self.step(action_index)
    }

    pub fn step(&mut self, action_index: usize) -> Result<FullRunEnvStep, String> {
        if self.done {
            return Ok(FullRunEnvStep {
                state: self.prepare_state()?,
                reward: 0.0,
                done: true,
                chosen_action_key: None,
                info: self.info(),
            });
        }
        if self.steps >= self.config.max_steps {
            self.done = true;
            self.terminal_reason = "step_cap".to_string();
            return Ok(FullRunEnvStep {
                state: self.prepare_state()?,
                reward: -2.0,
                done: true,
                chosen_action_key: None,
                info: self.info(),
            });
        }

        let state = self.prepare_state()?;
        if action_index >= state.action_candidates.len() {
            return Err(format!(
                "action index {action_index} out of range for {} candidates",
                state.action_candidates.len()
            ));
        }
        if !state.action_mask[action_index] {
            return Err(format!("action index {action_index} is currently illegal"));
        }

        let legal_actions = legal_actions(
            &self.ctx.engine_state,
            &self.ctx.run_state,
            &self.ctx.combat_state,
        );
        let action = legal_actions
            .get(action_index)
            .cloned()
            .ok_or_else(|| format!("action index {action_index} missing from legal actions"))?;
        let chosen_action_key = state.action_candidates[action_index].action_key.clone();
        let signature = no_progress_signature(
            &state.observation,
            &state.action_candidates,
            chosen_action_key.clone(),
        );
        if let Some(loop_info) =
            self.no_progress_tracker
                .observe(self.steps, signature, &state.observation)
        {
            let details = format!(
                "no progress loop: action {} repeated {} times from step {} to {} at {} floor {}",
                loop_info.action_key,
                loop_info.repeat_count,
                loop_info.start_step,
                loop_info.end_step,
                loop_info.decision_type,
                loop_info.floor
            );
            self.done = true;
            self.terminal_reason = "no_progress_loop".to_string();
            self.crash = Some(details.clone());
            self.contract_failure = Some(make_full_run_env_contract_failure(
                &self.config,
                self.config.seed,
                "no_progress_loop",
                "no_progress_loop",
                loop_info.floor,
                loop_info.act,
                Some(loop_info.end_step),
                Some(loop_info.action_key.clone()),
                Some(loop_info.decision_type.clone()),
                Some(loop_info.engine_state.clone()),
                details,
            ));
            return Ok(FullRunEnvStep {
                state,
                reward: -100.0,
                done: true,
                chosen_action_key: Some(chosen_action_key),
                info: self.info(),
            });
        }

        let before_score = full_run_progress_score(&self.ctx);
        let action_shaping =
            full_run_action_shaping_reward(&self.ctx, &action, self.config.reward_shaping_profile);
        let keep_running = tick_run(
            &mut self.ctx.engine_state,
            &mut self.ctx.run_state,
            &mut self.ctx.combat_state,
            Some(action),
        );
        self.steps += 1;

        if let Some(errors) = take_engine_error_diagnostics(&mut self.ctx) {
            let details = format!(
                "engine rejected legal action {chosen_action_key}: {}",
                errors.join("; ")
            );
            self.done = true;
            self.terminal_reason = "engine_rejected_action".to_string();
            self.crash = Some(details.clone());
            self.contract_failure = Some(make_full_run_env_contract_failure(
                &self.config,
                self.config.seed,
                "engine_rejected_action",
                "engine_rejected_action",
                self.ctx.run_state.floor_num,
                self.ctx.run_state.act_num,
                Some(self.steps.saturating_sub(1)),
                Some(chosen_action_key.clone()),
                Some(state.observation.decision_type.clone()),
                Some(state.observation.engine_state.clone()),
                details,
            ));
            return Ok(FullRunEnvStep {
                state: self.prepare_state()?,
                reward: -100.0,
                done: true,
                chosen_action_key: Some(chosen_action_key),
                info: self.info(),
            });
        }

        finish_combat_if_needed(&mut self.ctx);
        if !keep_running && matches!(self.ctx.engine_state, EngineState::GameOver(_)) {
            self.done = true;
            self.terminal_reason = "game_over".to_string();
        } else if !keep_running {
            self.done = true;
            self.terminal_reason = "engine_stopped".to_string();
        }

        if !self.done {
            if let Err(err) = prepare_decision_point(&mut self.ctx, self.config.max_steps) {
                self.done = true;
                self.terminal_reason = "engine_error".to_string();
                self.crash = Some(err.clone());
                self.contract_failure = Some(make_full_run_env_contract_failure(
                    &self.config,
                    self.config.seed,
                    "engine_error",
                    "engine_error",
                    self.ctx.run_state.floor_num,
                    self.ctx.run_state.act_num,
                    Some(self.steps),
                    Some(chosen_action_key.clone()),
                    Some(decision_type(&self.ctx.engine_state).to_string()),
                    Some(engine_state_label(&self.ctx.engine_state).to_string()),
                    err,
                ));
            } else if matches!(self.ctx.engine_state, EngineState::GameOver(_)) {
                self.done = true;
                self.terminal_reason = "game_over".to_string();
            }
        }

        let after_score = full_run_progress_score(&self.ctx);
        let reward = after_score - before_score + self.terminal_reward() + action_shaping;
        Ok(FullRunEnvStep {
            state: self.prepare_state()?,
            reward,
            done: self.done,
            chosen_action_key: Some(chosen_action_key),
            info: self.info(),
        })
    }

    fn prepare_state(&mut self) -> Result<FullRunEnvState, String> {
        if !self.done {
            prepare_decision_point(&mut self.ctx, self.config.max_steps)?;
            if matches!(self.ctx.engine_state, EngineState::GameOver(_)) {
                self.done = true;
                self.terminal_reason = "game_over".to_string();
            }
        }
        let observation = build_observation(&self.ctx);
        let legal_actions = if self.done {
            Vec::new()
        } else {
            legal_actions(
                &self.ctx.engine_state,
                &self.ctx.run_state,
                &self.ctx.combat_state,
            )
        };
        let action_candidates = build_action_candidates(&legal_actions, Some(&self.ctx));
        let action_mask = vec![true; action_candidates.len()];
        Ok(FullRunEnvState {
            observation_schema_version: FULL_RUN_OBSERVATION_SCHEMA_VERSION.to_string(),
            action_schema_version: FULL_RUN_ACTION_SCHEMA_VERSION.to_string(),
            action_mask_kind: "per_decision_candidate_set".to_string(),
            observation,
            legal_action_count: action_candidates.len(),
            action_candidates,
            action_mask,
        })
    }

    fn terminal_reward(&self) -> f32 {
        if !self.done {
            return 0.0;
        }
        match &self.ctx.engine_state {
            EngineState::GameOver(RunResult::Victory) => 100.0,
            EngineState::GameOver(RunResult::Defeat) => -10.0,
            _ if self.crash.is_some() => -100.0,
            _ => -2.0,
        }
    }

    pub fn info(&self) -> FullRunEnvInfo {
        FullRunEnvInfo {
            seed: self.config.seed,
            step: self.steps,
            terminal_reason: self.terminal_reason.clone(),
            result: full_run_result_label(&self.ctx, self.done, self.crash.as_ref()),
            forced_engine_ticks: self.ctx.forced_engine_ticks,
            combat_win_count: self.ctx.combat_win_count,
            crash: self.crash.clone(),
            contract_failure: self.contract_failure.clone(),
        }
    }
}



fn no_progress_signature(
    observation: &RunObservationV0,
    action_mask: &[RunActionCandidate],
    chosen_action_key: String,
) -> NoProgressSignature {
    let mut action_mask_key = String::new();
    for candidate in action_mask {
        if !action_mask_key.is_empty() {
            action_mask_key.push('|');
        }
        action_mask_key.push_str(&candidate.action_key);
    }

    NoProgressSignature {
        observation_key: observation_signature_key(observation),
        action_mask_key,
        chosen_action_key,
    }
}

fn observation_signature_key(observation: &RunObservationV0) -> String {
    let combat_key = observation
        .combat
        .as_ref()
        .map(|combat| {
            format!(
                "combat:hp={}/{};block={};energy={};turn={};hand={};draw={};discard={};exhaust={};alive={};monster_hp={};incoming={}",
                combat.player_hp,
                observation.max_hp,
                combat.player_block,
                combat.energy,
                combat.turn_count,
                combat.hand_count,
                combat.draw_count,
                combat.discard_count,
                combat.exhaust_count,
                combat.alive_monster_count,
                combat.total_monster_hp,
                combat.visible_incoming_damage
            )
        })
        .unwrap_or_else(|| "combat:none".to_string());

    format!(
        "decision={};engine={};act={};floor={};room={:?};hp={}/{};gold={};deck={};relics={};potions={}/{};screen=e{}:r{}:rc{}:sc{}:sr{}:sp{}:br{}:sel{};{}",
        observation.decision_type,
        observation.engine_state,
        observation.act,
        observation.floor,
        observation.current_room,
        observation.current_hp,
        observation.max_hp,
        observation.gold,
        observation.deck_size,
        observation.relic_count,
        observation.filled_potion_slots,
        observation.potion_slots,
        observation.screen.event_option_count,
        observation.screen.reward_item_count,
        observation.screen.reward_card_choice_count,
        observation.screen.shop_card_count,
        observation.screen.shop_relic_count,
        observation.screen.shop_potion_count,
        observation.screen.boss_relic_choice_count,
        observation.screen.selection_target_count,
        combat_key
    )
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
    fn no_progress_tracker_reports_repeated_identical_decision_signature() {
        let observation = RunObservationV0 {
            schema_version: FULL_RUN_OBSERVATION_SCHEMA_VERSION.to_string(),
            decision_type: "combat".to_string(),
            engine_state: "combat_player_turn".to_string(),
            act: 1,
            floor: 3,
            current_room: Some("MonsterRoom".to_string()),
            current_hp: 40,
            max_hp: 80,
            hp_ratio_milli: 500,
            gold: 99,
            deck_size: 10,
            relic_count: 1,
            potion_slots: 3,
            filled_potion_slots: 0,
            deck: RunDeckObservationV0::default(),
            plan_profile: DeckPlanProfileV0 {
                score_kind: "heuristic".to_string(),
                ..DeckPlanProfileV0::default()
            },
            deck_cards: Vec::new(),
            relics: Vec::new(),
            potions: Vec::new(),
            map: None,
            next_nodes: Vec::new(),
            act_boss: None,
            reward_source: None,
            combat: Some(RunCombatObservationV0 {
                player_hp: 40,
                player_block: 0,
                energy: 1,
                combat_phase: "player_turn".to_string(),
                turn_count: 2,
                hand_count: 1,
                hand_cards: Vec::new(),
                draw_count: 5,
                discard_count: 4,
                exhaust_count: 0,
                alive_monster_count: 1,
                dying_monster_count: 0,
                half_dead_monster_count: 0,
                zero_hp_monster_count: 0,
                pending_rebirth_monster_count: 0,
                total_monster_hp: 12,
                visible_incoming_damage: 6,
                pending_action_count: 0,
                queued_card_count: 0,
                limbo_count: 0,
            }),
            screen: empty_screen_observation(),
        };
        let action_mask = vec![RunActionCandidate {
            action_index: 0,
            action_id: stable_action_id("combat/play_card/card:Apparition/hand:0/target:none"),
            action_key: "combat/play_card/card:Apparition/hand:0/target:none".to_string(),
            action: TraceClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            card: None,
            plan_delta: empty_candidate_plan_delta(),
            reward_structure: empty_reward_action_structure(),
        }];
        let mut tracker = NoProgressTracker::new();

        let mut detected = None;
        for step_index in 10..(10 + NO_PROGRESS_REPEAT_LIMIT) {
            detected = tracker.observe(
                step_index,
                no_progress_signature(
                    &observation,
                    &action_mask,
                    "combat/play_card/card:Apparition/hand:0/target:none".to_string(),
                ),
                &observation,
            );
        }

        let loop_info = detected.expect("repeat limit should report no-progress loop");
        assert_eq!(loop_info.start_step, 10);
        assert_eq!(loop_info.end_step, 10 + NO_PROGRESS_REPEAT_LIMIT - 1);
        assert_eq!(loop_info.repeat_count, NO_PROGRESS_REPEAT_LIMIT);
        assert_eq!(
            loop_info.action_key,
            "combat/play_card/card:Apparition/hand:0/target:none"
        );
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
            policy: RunPolicyKind::RandomMasked,
            trace_dir: None,
            determinism_check: true,
            reward_shaping_profile: RewardShapingProfile::Baseline,
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
        assert_eq!(summary.contract_failure_count, 0);
        assert!(summary.contract_failures.is_empty());
        assert_eq!(summary.policy, "random_masked");
        assert!(summary.max_legal_action_count > 0);
        assert!(summary.decision_type_counts.values().sum::<usize>() > 0);
    }

    #[test]
    fn trace_observation_exports_visible_run_context() {
        let config = RunBatchConfig {
            episodes: 1,
            base_seed: 71200,
            ascension: 0,
            final_act: false,
            player_class: "Ironclad",
            max_steps: 80,
            policy: RunPolicyKind::RuleBaselineV0,
            trace_dir: None,
            determinism_check: false,
            reward_shaping_profile: RewardShapingProfile::Baseline,
        };
        let episode = run_episode(&config, 0, 71200, EpisodePolicy::RuleBaselineV0, true);
        assert!(episode.summary.crash.is_none());

        let first = episode
            .trace
            .first()
            .expect("trace should include Neow step");
        let observation = &first.observation;
        assert_eq!(
            observation.schema_version,
            FULL_RUN_OBSERVATION_SCHEMA_VERSION
        );
        assert_eq!(observation.deck_cards.len(), observation.deck_size);
        assert_eq!(observation.relics.len(), observation.relic_count);
        assert_eq!(observation.potions.len(), observation.potion_slots);
        assert!(observation.act_boss.is_some());
        assert!(!observation.next_nodes.is_empty());

        let map_step = episode
            .trace
            .iter()
            .find(|step| step.decision_type == "map")
            .expect("short rule-baseline run should reach map navigation");
        assert!(map_step.observation.map.is_some());

        let combat_step = episode
            .trace
            .iter()
            .find(|step| step.decision_type == "combat")
            .expect("short rule-baseline run should reach combat");
        assert!(combat_step.observation.map.is_none());
    }

    #[test]
    fn full_run_env_reset_and_step_exposes_candidate_mask() {
        let config = FullRunEnvConfig {
            seed: 42,
            ascension: 0,
            final_act: false,
            player_class: "Ironclad",
            max_steps: 50,
            reward_shaping_profile: RewardShapingProfile::Baseline,
        };
        let mut env = FullRunEnv::new(config).expect("full-run env should reset");

        let state = env.state().expect("state should be available");
        assert_eq!(
            state.observation_schema_version,
            FULL_RUN_OBSERVATION_SCHEMA_VERSION
        );
        assert_eq!(state.action_schema_version, FULL_RUN_ACTION_SCHEMA_VERSION);
        assert_eq!(state.action_mask_kind, "per_decision_candidate_set");
        assert_eq!(state.action_candidates.len(), state.action_mask.len());
        assert!(state.legal_action_count > 0);
        assert!(state.action_mask.iter().all(|legal| *legal));

        let step = env.step(0).expect("first legal action should step");
        assert_eq!(
            step.state.observation_schema_version,
            FULL_RUN_OBSERVATION_SCHEMA_VERSION
        );
        assert_eq!(step.info.seed, 42);
        assert!(step.chosen_action_key.is_some());
    }

    #[test]
    fn full_run_env_step_policy_uses_rule_baseline() {
        let config = FullRunEnvConfig {
            seed: 42,
            ascension: 0,
            final_act: false,
            player_class: "Ironclad",
            max_steps: 50,
            reward_shaping_profile: RewardShapingProfile::Baseline,
        };
        let mut env = FullRunEnv::new(config).expect("full-run env should reset");
        let step = env
            .step_policy(RunPolicyKind::RuleBaselineV0)
            .expect("rule baseline policy should choose a legal action");
        assert_eq!(step.info.seed, 42);
        assert!(step.chosen_action_key.is_some());
    }

    #[test]
    fn full_run_env_step_policy_accepts_plan_query_v0() {
        let config = FullRunEnvConfig {
            seed: 42,
            ascension: 0,
            final_act: false,
            player_class: "Ironclad",
            max_steps: 80,
            reward_shaping_profile: RewardShapingProfile::Baseline,
        };
        let mut env = FullRunEnv::new(config).expect("full-run env should reset");
        let step = env
            .step_policy(RunPolicyKind::PlanQueryV0)
            .expect("plan-query policy should choose a legal action or fall back");
        assert_eq!(step.info.seed, 42);
        assert!(step.chosen_action_key.is_some());
    }

    #[test]
    fn plan_query_v0_cashes_low_pressure_multi_enemy_damage_window() {
        use crate::semantics::combat::{
            AttackSpec, AttackStep, DamageKind, MonsterMoveSpec, MoveStep, MoveTarget,
        };

        let mut run_state = RunState::new(42, 0, false, "Ironclad");
        let mut combat = build_combat_state(&mut run_state, EncounterId::SmallSlimes);
        combat.clear_pending_actions();
        combat.zones.queued_cards.clear();
        combat.zones.limbo.clear();
        combat.turn.energy = 3;
        combat.entities.player.current_hp = 80;
        combat.entities.player.block = 0;
        combat.zones.hand = vec![
            crate::runtime::combat::CombatCard::new(CardId::Immolate, 10_001),
            crate::runtime::combat::CombatCard::new(CardId::Defend, 10_002),
            crate::runtime::combat::CombatCard::new(CardId::Defend, 10_003),
        ];
        for (index, monster) in combat.entities.monsters.iter_mut().enumerate() {
            monster.current_hp = 30;
            monster.max_hp = 30;
            monster.block = 0;
            if index == 0 {
                let attack = AttackSpec {
                    base_damage: 6,
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                };
                monster.set_planned_move_id(1);
                monster.set_planned_visible_spec(Some(MonsterMoveSpec::Attack(attack.clone())));
                monster.set_planned_steps(smallvec::smallvec![MoveStep::Attack(AttackStep {
                    target: MoveTarget::Player,
                    attack,
                })]);
            } else {
                monster.set_planned_move_id(0);
                monster.set_planned_visible_spec(Some(MonsterMoveSpec::None));
                monster.set_planned_steps(smallvec::smallvec![]);
            }
        }

        let ctx = EpisodeContext {
            engine_state: EngineState::CombatPlayerTurn,
            run_state,
            combat_state: Some(combat),
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };
        let legal = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
        let index = choose_plan_query_action(&ctx, &legal)
            .expect("plan-query should choose a damage-window action");

        assert!(
            matches!(
                legal.get(index),
                Some(ClientInput::PlayCard {
                    card_index: 0,
                    target: None
                })
            ),
            "expected Immolate first, got {:?} from {:?}",
            legal.get(index),
            legal
        );
    }

    #[test]
    fn plan_query_v0_guards_boss_ramp_pressure_before_aoe_cashout() {
        use crate::semantics::combat::{
            AttackSpec, AttackStep, DamageKind, MonsterMoveSpec, MoveStep, MoveTarget,
        };

        let mut run_state = RunState::new(45, 0, false, "Ironclad");
        let mut combat = build_combat_state(&mut run_state, EncounterId::SmallSlimes);
        combat.clear_pending_actions();
        combat.zones.queued_cards.clear();
        combat.zones.limbo.clear();
        combat.meta.is_boss_fight = true;
        combat.turn.energy = 3;
        combat.entities.player.current_hp = 18;
        combat.entities.player.max_hp = 80;
        combat.entities.player.block = 0;
        combat.zones.hand = vec![
            crate::runtime::combat::CombatCard::new(CardId::Immolate, 11_001),
            crate::runtime::combat::CombatCard::new(CardId::Defend, 11_002),
            crate::runtime::combat::CombatCard::new(CardId::Strike, 11_003),
        ];
        for monster in combat.entities.monsters.iter_mut() {
            monster.current_hp = 220;
            monster.max_hp = 220;
            monster.block = 0;
            monster.is_dying = false;
            let attack = AttackSpec {
                base_damage: 32,
                hits: 1,
                damage_kind: DamageKind::Normal,
            };
            monster.set_planned_move_id(1);
            monster.set_planned_visible_spec(Some(MonsterMoveSpec::Attack(attack.clone())));
            monster.set_planned_steps(smallvec::smallvec![MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })]);
        }

        let ctx = EpisodeContext {
            engine_state: EngineState::CombatPlayerTurn,
            run_state,
            combat_state: Some(combat),
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };
        let combat = ctx.combat_state.as_ref().unwrap();
        let incoming = visible_incoming_damage(combat);
        let unblocked = visible_unblocked_damage(combat);
        assert!(
            guarded_survival_pressure(combat, incoming, unblocked, combat.entities.player.current_hp),
            "test setup should trigger guarded pressure, incoming={incoming}, unblocked={unblocked}"
        );
        let legal = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
        let index = choose_plan_query_action(&ctx, &legal)
            .expect("plan-query should still choose a guarded survival action");

        assert!(
            matches!(
                legal.get(index),
                Some(ClientInput::PlayCard {
                    card_index: 1,
                    target: None
                })
            ),
            "expected Defend before Immolate under boss/ramp pressure, got {:?} from {:?}",
            legal.get(index),
            legal
        );
    }

    #[test]
    fn plan_query_v0_allows_boss_race_cashout_when_guard_line_still_leaks() {
        use crate::semantics::combat::{
            AttackSpec, AttackStep, DamageKind, MonsterMoveSpec, MoveStep, MoveTarget,
        };

        let mut run_state = RunState::new(46, 0, false, "Ironclad");
        let mut combat = build_combat_state(&mut run_state, EncounterId::SmallSlimes);
        combat.clear_pending_actions();
        combat.zones.queued_cards.clear();
        combat.zones.limbo.clear();
        combat.meta.is_boss_fight = true;
        combat.turn.energy = 3;
        combat.entities.player.current_hp = 80;
        combat.entities.player.max_hp = 80;
        combat.entities.player.block = 0;
        combat.zones.hand = vec![
            crate::runtime::combat::CombatCard::new(CardId::Immolate, 12_001),
            crate::runtime::combat::CombatCard::new(CardId::Defend, 12_002),
            crate::runtime::combat::CombatCard::new(CardId::Strike, 12_003),
        ];
        for monster in combat.entities.monsters.iter_mut() {
            monster.current_hp = 220;
            monster.max_hp = 220;
            monster.block = 0;
            monster.is_dying = false;
            let attack = AttackSpec {
                base_damage: 32,
                hits: 1,
                damage_kind: DamageKind::Normal,
            };
            monster.set_planned_move_id(1);
            monster.set_planned_visible_spec(Some(MonsterMoveSpec::Attack(attack.clone())));
            monster.set_planned_steps(smallvec::smallvec![MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })]);
        }

        let ctx = EpisodeContext {
            engine_state: EngineState::CombatPlayerTurn,
            run_state,
            combat_state: Some(combat),
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };
        let combat = ctx.combat_state.as_ref().unwrap();
        let incoming = visible_incoming_damage(combat);
        let unblocked = visible_unblocked_damage(combat);
        assert!(
            guarded_survival_pressure(combat, incoming, unblocked, combat.entities.player.current_hp),
            "test setup should trigger guarded pressure, incoming={incoming}, unblocked={unblocked}"
        );
        let legal = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
        let index = choose_plan_query_action(&ctx, &legal)
            .expect("plan-query should choose a guarded boss race action");

        assert!(
            matches!(
                legal.get(index),
                Some(ClientInput::PlayCard {
                    card_index: 0,
                    target: None
                })
            ),
            "expected Immolate when guard line still leaks and HP buffer is high, got {:?} from {:?}",
            legal.get(index),
            legal
        );
    }

    #[test]
    fn plan_query_v0_allows_boss_race_cashout_over_zero_damage_full_block() {
        use crate::semantics::combat::{
            AttackSpec, AttackStep, DamageKind, MonsterMoveSpec, MoveStep, MoveTarget,
        };

        let mut run_state = RunState::new(47, 0, false, "Ironclad");
        let mut combat = build_combat_state(&mut run_state, EncounterId::SmallSlimes);
        combat.clear_pending_actions();
        combat.zones.queued_cards.clear();
        combat.zones.limbo.clear();
        combat.meta.is_boss_fight = true;
        combat.turn.energy = 3;
        combat.entities.player.current_hp = 80;
        combat.entities.player.max_hp = 80;
        combat.entities.player.block = 0;
        combat.zones.hand = vec![
            crate::runtime::combat::CombatCard::new(CardId::Immolate, 13_001),
            crate::runtime::combat::CombatCard::new(CardId::Defend, 13_002),
            crate::runtime::combat::CombatCard::new(CardId::Defend, 13_003),
        ];
        for monster in combat.entities.monsters.iter_mut() {
            monster.current_hp = 220;
            monster.max_hp = 220;
            monster.block = 0;
            monster.is_dying = false;
            let attack = AttackSpec {
                base_damage: 5,
                hits: 1,
                damage_kind: DamageKind::Normal,
            };
            monster.set_planned_move_id(1);
            monster.set_planned_visible_spec(Some(MonsterMoveSpec::Attack(attack.clone())));
            monster.set_planned_steps(smallvec::smallvec![MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })]);
        }

        let ctx = EpisodeContext {
            engine_state: EngineState::CombatPlayerTurn,
            run_state,
            combat_state: Some(combat),
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };
        let combat = ctx.combat_state.as_ref().unwrap();
        let incoming = visible_incoming_damage(combat);
        let unblocked = visible_unblocked_damage(combat);
        assert!(
            guarded_survival_pressure(combat, incoming, unblocked, combat.entities.player.current_hp),
            "test setup should trigger guarded pressure, incoming={incoming}, unblocked={unblocked}"
        );
        let legal = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
        let index = choose_plan_query_action(&ctx, &legal)
            .expect("plan-query should choose a boss race cashout action");

        assert!(
            matches!(
                legal.get(index),
                Some(ClientInput::PlayCard {
                    card_index: 0,
                    target: None
                })
            ),
            "expected Immolate over zero-damage full block when HP buffer is high, got {:?} from {:?}",
            legal.get(index),
            legal
        );
    }

    #[test]
    fn plan_query_v0_opens_safe_offering_resource_window_before_spending_attacks() {
        use crate::semantics::combat::{
            AttackSpec, AttackStep, DamageKind, MonsterMoveSpec, MoveStep, MoveTarget,
        };

        let mut run_state = RunState::new(43, 0, false, "Ironclad");
        let mut combat = build_combat_state(&mut run_state, EncounterId::SmallSlimes);
        combat.clear_pending_actions();
        combat.zones.queued_cards.clear();
        combat.zones.limbo.clear();
        combat.turn.energy = 3;
        combat.entities.player.current_hp = 200;
        combat.entities.player.max_hp = 200;
        combat.entities.player.block = 0;
        combat.zones.hand = vec![
            crate::runtime::combat::CombatCard::new(CardId::Offering, 20_001),
            crate::runtime::combat::CombatCard::new(CardId::Bash, 20_002),
            crate::runtime::combat::CombatCard::new(CardId::Strike, 20_003),
        ];
        combat.zones.draw_pile = vec![
            crate::runtime::combat::CombatCard::new(CardId::Immolate, 20_004),
            crate::runtime::combat::CombatCard::new(CardId::Strike, 20_005),
            crate::runtime::combat::CombatCard::new(CardId::Defend, 20_006),
        ];
        for monster in &mut combat.entities.monsters {
            monster.current_hp = 200;
            monster.max_hp = 200;
            monster.block = 0;
            let attack = AttackSpec {
                base_damage: 1,
                hits: 1,
                damage_kind: DamageKind::Normal,
            };
            monster.set_planned_move_id(1);
            monster.set_planned_visible_spec(Some(MonsterMoveSpec::Attack(attack.clone())));
            monster.set_planned_steps(smallvec::smallvec![MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })]);
        }

        let ctx = EpisodeContext {
            engine_state: EngineState::CombatPlayerTurn,
            run_state,
            combat_state: Some(combat),
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };
        let legal = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
        let index = choose_plan_query_action(&ctx, &legal)
            .expect("plan-query should open a safe resource window");

        assert!(
            matches!(
                legal.get(index),
                Some(ClientInput::PlayCard {
                    card_index: 0,
                    target: None
                })
            ),
            "expected Offering first, got {:?} from {:?}",
            legal.get(index),
            legal
        );
    }

    #[test]
    fn plan_query_v0_follows_resource_window_with_best_cashout_plan() {
        use crate::semantics::combat::{
            AttackSpec, AttackStep, DamageKind, MonsterMoveSpec, MoveStep, MoveTarget,
        };

        let mut run_state = RunState::new(44, 0, false, "Ironclad");
        let mut combat = build_combat_state(&mut run_state, EncounterId::SmallSlimes);
        combat.clear_pending_actions();
        combat.zones.queued_cards.clear();
        combat.zones.limbo.clear();
        combat.turn.energy = 4;
        combat.turn.record_card_played(CardId::Offering);
        combat.entities.player.current_hp = 70;
        combat.entities.player.block = 0;
        combat.zones.hand = vec![
            crate::runtime::combat::CombatCard::new(CardId::Defend, 30_001),
            crate::runtime::combat::CombatCard::new(CardId::Immolate, 30_002),
            crate::runtime::combat::CombatCard::new(CardId::Defend, 30_003),
        ];
        for monster in &mut combat.entities.monsters {
            monster.current_hp = 40;
            monster.max_hp = 40;
            monster.block = 0;
            let attack = AttackSpec {
                base_damage: 1,
                hits: 1,
                damage_kind: DamageKind::Normal,
            };
            monster.set_planned_move_id(1);
            monster.set_planned_visible_spec(Some(MonsterMoveSpec::Attack(attack.clone())));
            monster.set_planned_steps(smallvec::smallvec![MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })]);
        }

        let ctx = EpisodeContext {
            engine_state: EngineState::CombatPlayerTurn,
            run_state,
            combat_state: Some(combat),
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };
        let legal = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
        let index = choose_plan_query_action(&ctx, &legal)
            .expect("resource-window follow-through should choose a payoff action");

        assert!(
            matches!(
                legal.get(index),
                Some(ClientInput::PlayCard {
                    card_index: 1,
                    target: None
                })
            ),
            "expected Immolate cashout after Offering, got {:?} from {:?}",
            legal.get(index),
            legal
        );
    }

    #[test]
    fn terminal_combat_player_turn_is_settled_before_actions_are_exposed() {
        let mut run_state = RunState::new(42, 0, false, "Ironclad");
        let mut combat = init_combat(&mut run_state);
        combat.clear_pending_actions();
        combat.zones.queued_cards.clear();
        combat.zones.limbo.clear();
        combat
            .zones
            .limbo
            .push(crate::runtime::combat::CombatCard::new(
                CardId::Strike,
                1234,
            ));
        for monster in &mut combat.entities.monsters {
            monster.current_hp = 0;
            monster.is_dying = true;
            monster.half_dead = false;
        }
        let mut ctx = EpisodeContext {
            engine_state: EngineState::CombatPlayerTurn,
            run_state,
            combat_state: Some(combat),
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };

        prepare_decision_point(&mut ctx, 100).expect("terminal combat should settle");

        assert!(matches!(ctx.engine_state, EngineState::RewardScreen(_)));
        assert!(ctx.combat_state.is_none());
        assert_eq!(ctx.combat_win_count, 1);
    }

    #[test]
    fn reward_card_choice_candidates_include_card_features_and_deck_summary() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.master_deck.clear();
        run_state.add_card_to_deck(CardId::Strike);
        run_state.add_card_to_deck(CardId::Defend);
        run_state.add_card_to_deck(CardId::Bash);
        let reward_state = RewardState {
            pending_card_choice: Some(vec![
                crate::rewards::state::RewardCard::new(CardId::PommelStrike, 0),
                crate::rewards::state::RewardCard::new(CardId::ShrugItOff, 0),
            ]),
            ..RewardState::new()
        };
        let ctx = EpisodeContext {
            engine_state: EngineState::RewardScreen(reward_state),
            run_state,
            combat_state: None,
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };
        let legal_actions = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
        let candidates = build_action_candidates(&legal_actions, Some(&ctx));
        let observation = build_observation(&ctx);

        assert_eq!(observation.decision_type, "reward_card_choice");
        assert_eq!(observation.screen.reward_card_choice_count, 2);
        assert_eq!(observation.screen.reward_phase, "card_choice");
        assert_eq!(observation.deck.attack_count, 2);
        assert_eq!(observation.deck.skill_count, 1);
        assert!(observation.deck.starter_basic_count >= 2);

        let pommel = candidates
            .iter()
            .find(|candidate| matches!(candidate.action, TraceClientInput::SelectCard { index: 0 }))
            .and_then(|candidate| candidate.card.as_ref())
            .expect("first reward card candidate should expose card features");
        assert_eq!(pommel.card_id, "PommelStrike");
        assert_eq!(pommel.card_type_id, card_type_id(CardType::Attack));
        assert!(pommel.draws_cards);
        assert!(pommel.rule_score > 0);

        let skip = candidates
            .iter()
            .find(|candidate| matches!(candidate.action, TraceClientInput::Proceed))
            .expect("card reward skip should remain available");
        assert!(skip.card.is_none());
        assert!(skip.reward_structure.skip_card_choice);

        let take_reward = full_run_action_shaping_reward(
            &ctx,
            &ClientInput::SelectCard(0),
            RewardShapingProfile::Baseline,
        );
        let skip_reward = full_run_action_shaping_reward(
            &ctx,
            &ClientInput::Proceed,
            RewardShapingProfile::Baseline,
        );
        assert!(
            take_reward > skip_reward,
            "card choice shaping should give the learner an immediate non-oracle hint"
        );
    }

    #[test]
    fn starter_deck_plan_profile_marks_basic_burden_and_missing_draw_scaling() {
        let run_state = RunState::new(1, 0, false, "Ironclad");
        let profile = build_deck_plan_profile(&run_state);

        assert_eq!(profile.score_kind, "heuristic");
        assert!(
            profile.starter_basic_burden >= 90,
            "starter deck should expose a high starter/basic burden"
        );
        assert_eq!(profile.draw_supply, 0);
        assert_eq!(profile.scaling_supply, 0);
        assert!(profile.frontload_supply > 0);
        assert!(profile.block_supply > 0);
    }

    #[test]
    fn reward_card_candidates_expose_plan_deltas_for_draw_scaling_and_frontload() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.master_deck.clear();
        run_state.add_card_to_deck(CardId::Strike);
        run_state.add_card_to_deck(CardId::Defend);
        run_state.add_card_to_deck(CardId::Bash);
        let reward_state = RewardState {
            pending_card_choice: Some(vec![
                crate::rewards::state::RewardCard::new(CardId::PommelStrike, 0),
                crate::rewards::state::RewardCard::new(CardId::ShrugItOff, 0),
                crate::rewards::state::RewardCard::new(CardId::Inflame, 0),
                crate::rewards::state::RewardCard::new(CardId::Immolate, 0),
                crate::rewards::state::RewardCard::new(CardId::Disarm, 0),
            ]),
            ..RewardState::new()
        };
        let ctx = EpisodeContext {
            engine_state: EngineState::RewardScreen(reward_state),
            run_state,
            combat_state: None,
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };
        let legal_actions = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
        let candidates = build_action_candidates(&legal_actions, Some(&ctx));

        let by_card = |name: &str| {
            candidates
                .iter()
                .find(|candidate| {
                    candidate
                        .card
                        .as_ref()
                        .is_some_and(|card| card.card_id == name)
                })
                .expect("candidate should exist")
        };
        assert!(by_card("PommelStrike").plan_delta.draw_delta > 0);
        assert!(by_card("ShrugItOff").plan_delta.block_delta > 0);
        assert!(by_card("Inflame").plan_delta.scaling_delta > 0);
        assert!(by_card("Immolate").plan_delta.aoe_delta > 0);
        assert!(by_card("Disarm").plan_delta.block_delta > 0);
    }

    #[test]
    fn true_grit_upgrade_delta_improves_exhaust_reliability() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.master_deck.clear();
        run_state.add_card_to_deck(CardId::TrueGrit);

        let unupgraded = add_card_plan_delta(CardId::TrueGrit, 0, &run_state);
        let upgraded = add_card_plan_delta(CardId::TrueGrit, 1, &run_state);
        let upgrade = upgrade_card_plan_delta(CardId::TrueGrit, 0, &run_state);

        assert!(upgraded.exhaust_delta > unupgraded.exhaust_delta);
        assert!(upgrade.exhaust_delta > 0);
        assert!(upgrade.plan_adjusted_score > 0);
    }

    #[test]
    fn plan_deficit_shaping_strongly_penalizes_skipping_high_plan_offer() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.master_deck.clear();
        run_state.add_card_to_deck(CardId::Strike);
        run_state.add_card_to_deck(CardId::Defend);
        run_state.add_card_to_deck(CardId::Bash);
        let reward_state = RewardState {
            pending_card_choice: Some(vec![
                crate::rewards::state::RewardCard::new(CardId::PommelStrike, 0),
                crate::rewards::state::RewardCard::new(CardId::Inflame, 0),
            ]),
            ..RewardState::new()
        };
        let ctx = EpisodeContext {
            engine_state: EngineState::RewardScreen(reward_state),
            run_state,
            combat_state: None,
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };

        let take_plan = full_run_action_shaping_reward(
            &ctx,
            &ClientInput::SelectCard(0),
            RewardShapingProfile::PlanDeficitV0,
        );
        let skip_plan = full_run_action_shaping_reward(
            &ctx,
            &ClientInput::Proceed,
            RewardShapingProfile::PlanDeficitV0,
        );
        let skip_baseline = full_run_action_shaping_reward(
            &ctx,
            &ClientInput::Proceed,
            RewardShapingProfile::Baseline,
        );

        assert!(take_plan > 0.35);
        assert!(skip_plan < skip_baseline);
        assert!(skip_plan <= -0.40);
    }

    #[test]
    fn reward_item_screen_shaping_discourages_skipping_unclaimed_resources() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.potions = vec![None, None, None];
        let reward_state = RewardState {
            items: vec![
                RewardItem::Gold { amount: 42 },
                RewardItem::Card {
                    cards: vec![crate::rewards::state::RewardCard::new(
                        CardId::PommelStrike,
                        0,
                    )],
                },
            ],
            ..RewardState::new()
        };
        let ctx = EpisodeContext {
            engine_state: EngineState::RewardScreen(reward_state),
            run_state,
            combat_state: None,
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };

        let claim_gold = full_run_action_shaping_reward(
            &ctx,
            &ClientInput::ClaimReward(0),
            RewardShapingProfile::Baseline,
        );
        let claim_card = full_run_action_shaping_reward(
            &ctx,
            &ClientInput::ClaimReward(1),
            RewardShapingProfile::Baseline,
        );
        let proceed = full_run_action_shaping_reward(
            &ctx,
            &ClientInput::Proceed,
            RewardShapingProfile::Baseline,
        );

        assert!(claim_gold > 0.0);
        assert!(claim_card > 0.0);
        assert!(
            proceed < 0.0,
            "skipping unclaimed reward items should carry an immediate resource-loss hint"
        );
    }

    #[test]
    fn reward_screen_structure_exposes_claim_items_and_proceed_avoidance() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.potions = vec![None, None, None];
        let reward_state = RewardState {
            items: vec![
                RewardItem::Gold { amount: 42 },
                RewardItem::Card {
                    cards: vec![crate::rewards::state::RewardCard::new(
                        CardId::PommelStrike,
                        0,
                    )],
                },
            ],
            ..RewardState::new()
        };
        let ctx = EpisodeContext {
            engine_state: EngineState::RewardScreen(reward_state),
            run_state,
            combat_state: None,
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };

        let observation = build_observation(&ctx);
        assert_eq!(observation.screen.reward_phase, "claim_items");
        assert_eq!(observation.screen.reward_item_count, 2);
        assert_eq!(observation.screen.reward_claimable_item_count, 2);
        assert_eq!(observation.screen.reward_unclaimed_card_item_count, 1);
        assert!(observation.screen.reward_free_value_score > 0);
        assert!(observation
            .screen
            .reward_items
            .iter()
            .any(|item| item.item_type == "card_reward" && item.opens_card_choice));

        let legal_actions = legal_actions(&ctx.engine_state, &ctx.run_state, &ctx.combat_state);
        let candidates = build_action_candidates(&legal_actions, Some(&ctx));
        let claim_card = candidates
            .iter()
            .find(|candidate| {
                matches!(candidate.action, TraceClientInput::ClaimReward { index: 1 })
            })
            .expect("card reward item should be claimable");
        assert!(claim_card.reward_structure.claim_opens_card_choice);
        assert_eq!(
            claim_card
                .reward_structure
                .claim_reward_item_type
                .as_deref(),
            Some("card_reward")
        );
        assert!(claim_card.reward_structure.claim_free_value_score > 0);

        let proceed = candidates
            .iter()
            .find(|candidate| matches!(candidate.action, TraceClientInput::Proceed))
            .expect("proceed should remain available");
        assert!(proceed.reward_structure.is_proceed_with_unclaimed_rewards);
        assert_eq!(proceed.reward_structure.unclaimed_reward_count, 2);
        assert_eq!(proceed.reward_structure.unclaimed_card_reward_count, 1);
        assert!(!proceed.reward_structure.proceed_is_cleanup);
    }

    #[test]
    fn plan_deficit_shaping_penalizes_skipping_reward_item_phase_more_strongly() {
        let run_state = RunState::new(1, 0, false, "Ironclad");
        let reward_state = RewardState {
            items: vec![RewardItem::Card {
                cards: vec![crate::rewards::state::RewardCard::new(
                    CardId::PommelStrike,
                    0,
                )],
            }],
            ..RewardState::new()
        };
        let ctx = EpisodeContext {
            engine_state: EngineState::RewardScreen(reward_state),
            run_state,
            combat_state: None,
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };

        let claim_plan = full_run_action_shaping_reward(
            &ctx,
            &ClientInput::ClaimReward(0),
            RewardShapingProfile::PlanDeficitV0,
        );
        let proceed_plan = full_run_action_shaping_reward(
            &ctx,
            &ClientInput::Proceed,
            RewardShapingProfile::PlanDeficitV0,
        );
        let proceed_baseline = full_run_action_shaping_reward(
            &ctx,
            &ClientInput::Proceed,
            RewardShapingProfile::Baseline,
        );

        assert!(claim_plan >= 0.40);
        assert!(proceed_plan < proceed_baseline);
        assert!(proceed_plan <= -0.40);
    }

    #[test]
    fn legal_shop_actions_keep_sozu_potion_purchase_as_executable_resource_loss() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 200;
        run_state
            .relics
            .push(crate::content::relics::RelicState::new(RelicId::Sozu));
        let mut shop = crate::shop::ShopState::new();
        shop.potions.push(crate::shop::ShopPotion {
            potion_id: crate::content::potions::PotionId::BlockPotion,
            price: 50,
            can_buy: true,
            blocked_reason: None,
        });

        let actions = legal_shop_actions(&run_state, &shop);

        assert!(actions
            .iter()
            .any(|action| matches!(action, ClientInput::Proceed)));
        assert!(
            actions
                .iter()
                .any(|action| matches!(action, ClientInput::BuyPotion(0))),
            "Sozu shop potion buys are executable: they spend gold and absorb the potion"
        );
    }

    #[test]
    fn legal_shop_actions_keep_affordable_potions_with_empty_slot_without_sozu() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 200;
        let mut shop = crate::shop::ShopState::new();
        shop.potions.push(crate::shop::ShopPotion {
            potion_id: crate::content::potions::PotionId::BlockPotion,
            price: 50,
            can_buy: true,
            blocked_reason: None,
        });

        let actions = legal_shop_actions(&run_state, &shop);

        assert!(
            actions
                .iter()
                .any(|action| matches!(action, ClientInput::BuyPotion(0))),
            "normal affordable shop potion buys should remain legal"
        );
    }

    #[test]
    fn rule_baseline_scores_sozu_potion_purchase_as_resource_waste() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.gold = 200;
        run_state
            .relics
            .push(crate::content::relics::RelicState::new(RelicId::Sozu));
        let mut shop = crate::shop::ShopState::new();
        shop.potions.push(crate::shop::ShopPotion {
            potion_id: crate::content::potions::PotionId::BlockPotion,
            price: 50,
            can_buy: true,
            blocked_reason: None,
        });

        let buy_score = score_shop_action(&run_state, &shop, &ClientInput::BuyPotion(0));
        let leave_score = score_shop_action(&run_state, &shop, &ClientInput::Proceed);

        assert!(
            buy_score < leave_score,
            "Sozu potion purchase remains executable but should be scored as resource waste"
        );
    }

    #[test]
    fn rule_baseline_policy_runs_and_reports_metrics() {
        let config = RunBatchConfig {
            episodes: 1,
            base_seed: 42,
            ascension: 0,
            final_act: false,
            player_class: "Ironclad",
            max_steps: 50,
            policy: RunPolicyKind::RuleBaselineV0,
            trace_dir: None,
            determinism_check: true,
            reward_shaping_profile: RewardShapingProfile::Baseline,
        };

        let summary = run_batch(&config).expect("one episode rule baseline smoke should run");
        assert_eq!(summary.policy, "rule_baseline_v0");
        assert_eq!(summary.episodes_completed, 1);
        assert_eq!(summary.crash_count, 0);
        assert_eq!(summary.illegal_action_count, 0);
        assert_eq!(summary.deterministic_replay_pass_count, 1);
        assert_eq!(summary.contract_failure_count, 0);
        assert!(summary.average_legal_action_count > 0.0);
    }

    #[test]
    fn rule_baseline_seed_10542_regresses_empty_upgrade_select_fizzle() {
        let config = RunBatchConfig {
            episodes: 1,
            base_seed: 10542,
            ascension: 0,
            final_act: false,
            player_class: "Ironclad",
            max_steps: 5000,
            policy: RunPolicyKind::RuleBaselineV0,
            trace_dir: None,
            determinism_check: true,
            reward_shaping_profile: RewardShapingProfile::Baseline,
        };

        let summary = run_batch(&config).expect("seed 10542 should run without contract failure");
        assert_eq!(summary.crash_count, 0);
        assert_eq!(summary.illegal_action_count, 0);
        assert_eq!(summary.no_progress_loop_count, 0);
        assert_eq!(summary.contract_failure_count, 0);
        assert_eq!(summary.deterministic_replay_pass_count, 1);
    }

    #[test]
    fn contract_failure_records_repro_seed_policy_and_action_key() {
        let config = RunBatchConfig {
            episodes: 1,
            base_seed: 6040,
            ascension: 0,
            final_act: false,
            player_class: "Ironclad",
            max_steps: 5000,
            policy: RunPolicyKind::RandomMasked,
            trace_dir: None,
            determinism_check: true,
            reward_shaping_profile: RewardShapingProfile::Baseline,
        };

        let failure = make_contract_failure(
            &config,
            0,
            6040,
            "engine_rejected_action",
            "engine_rejected_action",
            3,
            1,
            Some(17),
            Some("combat/play_card/card:Apparition/hand:0/target:none".to_string()),
            Some("combat".to_string()),
            Some("combat_player_turn".to_string()),
            "engine rejected legal action".to_string(),
        );

        assert_eq!(failure.seed, 6040);
        assert_eq!(failure.policy, "random_masked");
        assert_eq!(failure.step, Some(17));
        assert_eq!(
            failure.action_key.as_deref(),
            Some("combat/play_card/card:Apparition/hand:0/target:none")
        );
        assert!(failure.reproduce_command.contains("--episodes 1"));
        assert!(failure.reproduce_command.contains("--seed 6040"));
        assert!(failure.reproduce_command.contains("--policy random_masked"));
        assert!(failure.reproduce_command.contains("--max-steps 5000"));
    }
}
