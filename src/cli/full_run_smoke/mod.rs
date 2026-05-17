use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Instant;

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

mod actions;
mod batch;
mod bot;
mod decision_env;
mod features;
mod observation;
mod public_observation;
mod reward;
mod trace;

pub use actions::*;
pub use batch::*;
pub use bot::*;
pub use features::*;
pub use observation::*;
pub use public_observation::*;
pub use reward::*;
pub use trace::*;

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

    pub fn current_combat_decision_context_parts(
        &mut self,
    ) -> Result<Option<(EngineState, CombatState, Vec<ClientInput>)>, String> {
        let _ = self.prepare_state()?;
        if self.done {
            return Ok(None);
        }
        if !matches!(
            self.ctx.engine_state,
            EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
        ) {
            return Ok(None);
        }
        let Some(combat) = self.ctx.combat_state.clone() else {
            return Ok(None);
        };
        let legal_actions = legal_actions(
            &self.ctx.engine_state,
            &self.ctx.run_state,
            &self.ctx.combat_state,
        );
        Ok(Some((self.ctx.engine_state.clone(), combat, legal_actions)))
    }

    pub fn cache_bucket_hint(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.config.seed.hash(&mut hasher);
        self.config.ascension.hash(&mut hasher);
        self.config.final_act.hash(&mut hasher);
        self.config.player_class.hash(&mut hasher);
        self.config.max_steps.hash(&mut hasher);
        self.steps.hash(&mut hasher);
        self.done.hash(&mut hasher);
        self.terminal_reason.hash(&mut hasher);
        self.crash.is_some().hash(&mut hasher);
        self.contract_failure.is_some().hash(&mut hasher);
        self.no_progress_tracker.repeat_count.hash(&mut hasher);
        self.no_progress_tracker.start_step.hash(&mut hasher);
        if let Some(last) = &self.no_progress_tracker.last {
            last.observation_key.hash(&mut hasher);
            last.action_mask_key.hash(&mut hasher);
            last.chosen_action_key.hash(&mut hasher);
        }

        engine_state_label(&self.ctx.engine_state).hash(&mut hasher);
        format!("{:?}", self.ctx.engine_state).hash(&mut hasher);
        self.ctx.forced_engine_ticks.hash(&mut hasher);
        self.ctx.combat_win_count.hash(&mut hasher);

        let run = &self.ctx.run_state;
        run.seed.hash(&mut hasher);
        run.ascension_level.hash(&mut hasher);
        run.act_num.hash(&mut hasher);
        run.floor_num.hash(&mut hasher);
        run.current_hp.hash(&mut hasher);
        run.max_hp.hash(&mut hasher);
        run.gold.hash(&mut hasher);
        run.shop_purge_count.hash(&mut hasher);
        run.relics.len().hash(&mut hasher);
        run.potions.len().hash(&mut hasher);
        run.master_deck.len().hash(&mut hasher);
        run.reward_state.is_some().hash(&mut hasher);
        run.shop_state.is_some().hash(&mut hasher);
        run.event_state.is_some().hash(&mut hasher);
        run.room_mugged.hash(&mut hasher);
        run.room_smoked.hash(&mut hasher);
        run.pending_boss_reward.hash(&mut hasher);
        run.pending_boss_act_transition.hash(&mut hasher);

        if let Some(combat) = &self.ctx.combat_state {
            combat.meta.ascension_level.hash(&mut hasher);
            combat.meta.player_class.hash(&mut hasher);
            combat.meta.is_boss_fight.hash(&mut hasher);
            combat.meta.is_elite_fight.hash(&mut hasher);
            combat.turn.turn_count.hash(&mut hasher);
            format!("{:?}", combat.turn.current_phase).hash(&mut hasher);
            combat.turn.energy.hash(&mut hasher);
            combat.turn.turn_start_draw_modifier.hash(&mut hasher);
            combat
                .turn
                .counters
                .cards_played_this_turn
                .hash(&mut hasher);
            combat
                .turn
                .counters
                .attacks_played_this_turn
                .hash(&mut hasher);
            combat
                .turn
                .counters
                .times_damaged_this_combat
                .hash(&mut hasher);
            combat
                .turn
                .counters
                .early_end_turn_pending
                .hash(&mut hasher);
            combat.turn.counters.victory_triggered.hash(&mut hasher);

            hash_card_zone_hint(&combat.zones.draw_pile, &mut hasher);
            hash_card_zone_hint(&combat.zones.hand, &mut hasher);
            hash_card_zone_hint(&combat.zones.discard_pile, &mut hasher);
            hash_card_zone_hint(&combat.zones.exhaust_pile, &mut hasher);
            hash_card_zone_hint(&combat.zones.limbo, &mut hasher);
            combat.zones.queued_cards.len().hash(&mut hasher);
            combat.zones.card_uuid_counter.hash(&mut hasher);
            combat.engine.action_queue.len().hash(&mut hasher);

            let player = &combat.entities.player;
            player.current_hp.hash(&mut hasher);
            player.max_hp.hash(&mut hasher);
            player.block.hash(&mut hasher);
            player.gold_delta_this_combat.hash(&mut hasher);
            player.gold.hash(&mut hasher);
            player.energy_master.hash(&mut hasher);
            player.stance.as_str().hash(&mut hasher);
            player.orbs.len().hash(&mut hasher);
            player.relics.len().hash(&mut hasher);

            for monster in &combat.entities.monsters {
                monster.id.hash(&mut hasher);
                monster.monster_type.hash(&mut hasher);
                monster.current_hp.hash(&mut hasher);
                monster.max_hp.hash(&mut hasher);
                monster.block.hash(&mut hasher);
                monster.slot.hash(&mut hasher);
                monster.is_dying.hash(&mut hasher);
                monster.is_escaped.hash(&mut hasher);
                monster.half_dead.hash(&mut hasher);
                monster.planned_move_id().hash(&mut hasher);
            }
            combat.entities.power_db.len().hash(&mut hasher);
            for (entity_id, powers) in &combat.entities.power_db {
                entity_id.hash(&mut hasher);
                powers.len().hash(&mut hasher);
                for power in powers {
                    format!("{:?}", power.power_type).hash(&mut hasher);
                    power.instance_id.hash(&mut hasher);
                    power.amount.hash(&mut hasher);
                    power.extra_data.hash(&mut hasher);
                    power.just_applied.hash(&mut hasher);
                }
            }
            combat.runtime.using_card.hash(&mut hasher);
            combat.runtime.card_queue.len().hash(&mut hasher);
            combat.runtime.colorless_combat_pool.len().hash(&mut hasher);
            combat.runtime.pending_rewards.len().hash(&mut hasher);
            combat.runtime.combat_mugged.hash(&mut hasher);
            combat.runtime.combat_smoked.hash(&mut hasher);
        } else {
            0u8.hash(&mut hasher);
        }
        hasher.finish()
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
        let reward = after_score - before_score + self.terminal_reward();
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
            floor: self.ctx.run_state.floor_num,
            act: self.ctx.run_state.act_num,
            hp: self
                .ctx
                .combat_state
                .as_ref()
                .map(|combat| combat.entities.player.current_hp)
                .unwrap_or(self.ctx.run_state.current_hp),
            max_hp: self
                .ctx
                .combat_state
                .as_ref()
                .map(|combat| combat.entities.player.max_hp)
                .unwrap_or(self.ctx.run_state.max_hp),
            gold: self.ctx.run_state.gold,
            deck_size: self.ctx.run_state.master_deck.len(),
            relic_count: self.ctx.run_state.relics.len(),
            terminal_reason: self.terminal_reason.clone(),
            result: full_run_result_label(&self.ctx, self.done, self.crash.as_ref()),
            forced_engine_ticks: self.ctx.forced_engine_ticks,
            combat_win_count: self.ctx.combat_win_count,
            crash: self.crash.clone(),
            contract_failure: self.contract_failure.clone(),
        }
    }
}

fn hash_card_zone_hint(cards: &[crate::runtime::combat::CombatCard], hasher: &mut DefaultHasher) {
    cards.len().hash(hasher);
    for card in cards {
        format!("{:?}", card.id).hash(hasher);
        card.uuid.hash(hasher);
        card.upgrades.hash(hasher);
        card.misc_value.hash(hasher);
        card.base_damage_override.hash(hasher);
        card.cost_modifier.hash(hasher);
        card.cost_for_turn.hash(hasher);
        card.base_damage_mut.hash(hasher);
        card.base_block_mut.hash(hasher);
        card.base_magic_num_mut.hash(hasher);
        card.exhaust_override.hash(hasher);
        card.retain_override.hash(hasher);
        card.free_to_play_once.hash(hasher);
        card.energy_on_use.hash(hasher);
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

fn init_event_combat(
    run_state: &mut RunState,
    encounter_id: EncounterId,
    elite_trigger: bool,
) -> CombatState {
    let mut combat = build_combat_state(run_state, encounter_id);
    combat.meta.is_elite_fight = elite_trigger;
    combat
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
            master_deck_snapshot: run_state.master_deck.clone(),
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
    combat.apply_java_initialize_deck_order_after_shuffle();
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

fn encounter_key_to_id(key: &str) -> Option<EncounterId> {
    match key {
        "Colosseum Slavers" => Some(EncounterId::ColosseumSlavers),
        "Colosseum Nobs" => Some(EncounterId::ColosseumNobs),
        "Masked Bandits" | "3 Bandits" => Some(EncounterId::MaskedBandits),
        "Dead Adventurer" | "Lagavulin Event" => Some(EncounterId::LagavulinEvent),
        "3 Sentries" => Some(EncounterId::ThreeSentries),
        "Gremlin Nob" => Some(EncounterId::GremlinNob),
        "The Mushroom Lair" | "3 Fungi Beasts" => Some(EncounterId::TheMushroomLair),
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
            plan_profile: DeckPlanProfileV0::default(),
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
                pending_choice_kind: None,
                pending_choice: None,
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
            plan_delta: Some(empty_candidate_plan_delta()),
            reward_structure: Some(empty_reward_action_structure()),
            dominated: false,
            dominated_by_index: None,
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
            action_selector: RunActionSelectorKind::RandomMasked,
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
        assert_eq!(summary.contract_failure_count, 0);
        assert!(summary.contract_failures.is_empty());
        assert_eq!(summary.action_selector, "random_masked");
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
            action_selector: RunActionSelectorKind::RandomMasked,
            trace_dir: None,
            determinism_check: false,
        };
        let episode = run_episode(
            &config,
            0,
            71200,
            EpisodeActionSelector::RandomMasked {
                rng: StsRng::new(71200 ^ 0x9e37_79b9_7f4a_7c15),
            },
            true,
        );
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
    }

    #[test]
    fn full_run_env_reset_and_step_exposes_candidate_mask() {
        let config = FullRunEnvConfig {
            seed: 42,
            ascension: 0,
            final_act: false,
            player_class: "Ironclad",
            max_steps: 50,
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

        let skip = candidates
            .iter()
            .find(|candidate| matches!(candidate.action, TraceClientInput::Proceed))
            .expect("card reward skip should remain available");
        assert!(skip.card.is_none());
        assert!(
            skip.reward_structure
                .as_ref()
                .expect("skip reward structure")
                .skip_card_choice
        );
    }

    #[test]
    fn starter_deck_plan_profile_marks_basic_burden_and_missing_draw_scaling() {
        let run_state = RunState::new(1, 0, false, "Ironclad");
        let profile = build_deck_plan_profile(&run_state);

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
        assert!(
            by_card("PommelStrike")
                .plan_delta
                .as_ref()
                .expect("PommelStrike plan delta")
                .draw_delta
                > 0
        );
        assert!(
            by_card("ShrugItOff")
                .plan_delta
                .as_ref()
                .expect("ShrugItOff plan delta")
                .block_delta
                > 0
        );
        assert!(
            by_card("Inflame")
                .plan_delta
                .as_ref()
                .expect("Inflame plan delta")
                .scaling_delta
                > 0
        );
        assert!(
            by_card("Immolate")
                .plan_delta
                .as_ref()
                .expect("Immolate plan delta")
                .aoe_delta
                > 0
        );
        assert!(
            by_card("Disarm")
                .plan_delta
                .as_ref()
                .expect("Disarm plan delta")
                .block_delta
                > 0
        );
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
        let claim_card_structure = claim_card
            .reward_structure
            .as_ref()
            .expect("claim card reward structure");
        assert!(claim_card_structure.claim_opens_card_choice);
        assert_eq!(
            claim_card_structure.claim_reward_item_type.as_deref(),
            Some("card_reward")
        );

        let proceed = candidates
            .iter()
            .find(|candidate| matches!(candidate.action, TraceClientInput::Proceed))
            .expect("proceed should remain available");
        let proceed_structure = proceed
            .reward_structure
            .as_ref()
            .expect("proceed reward structure");
        assert!(proceed_structure.is_proceed_with_unclaimed_rewards);
        assert_eq!(proceed_structure.unclaimed_reward_count, 2);
        assert_eq!(proceed_structure.unclaimed_card_reward_count, 1);
        assert!(!proceed_structure.proceed_is_cleanup);
    }

    #[test]
    fn legal_shop_actions_block_sozu_potion_purchase_like_java_store_potion() {
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
                .all(|action| !matches!(action, ClientInput::BuyPotion(0))),
            "Java StorePotion.purchasePotion returns immediately under Sozu without spending gold or removing the offer"
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
    fn contract_failure_records_repro_seed_selector_and_action_key() {
        let config = RunBatchConfig {
            episodes: 1,
            base_seed: 6040,
            ascension: 0,
            final_act: false,
            player_class: "Ironclad",
            max_steps: 5000,
            action_selector: RunActionSelectorKind::RandomMasked,
            trace_dir: None,
            determinism_check: true,
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
        assert_eq!(failure.action_selector, "random_masked");
        assert_eq!(failure.step, Some(17));
        assert_eq!(
            failure.action_key.as_deref(),
            Some("combat/play_card/card:Apparition/hand:0/target:none")
        );
        assert!(failure.reproduce_command.contains("--episodes 1"));
        assert!(failure.reproduce_command.contains("--seed 6040"));
        assert!(failure
            .reproduce_command
            .contains("--action-selector random_masked"));
        assert!(failure.reproduce_command.contains("--max-steps 5000"));
    }
}
