use serde::{Deserialize, Serialize};

use crate::bot::monster_belief::{
    build_combat_belief_state, CombatBeliefState, MonsterBeliefCertainty,
};
use crate::bot::search::{legal_moves_for_audit, StatePressureFeatures};
use crate::combat::{CombatCard, CombatPhase, CombatState};
use crate::content::cards::{can_play_card, effective_target, get_card_definition};
use crate::core::EntityId;
use crate::engine::core::tick_until_stable_turn;
use crate::engine::targeting;
use crate::state::core::{ClientInput, EngineState, PendingChoice};

use crate::testing::fixtures::combat_start_spec::CombatStartSpec;
use crate::testing::fixtures::scenario::{initialize_fixture_state, ScenarioFixture};
use crate::testing::harness::hexaghost_value::{
    analyze_hexaghost_persistent_attack_script, project_hexaghost_future_script,
    HexaghostFutureScriptSummary, HexaghostPersistentAttackScriptValue,
};

#[derive(Clone, Debug)]
pub struct CombatEnvSpec {
    pub name: String,
    pub player_class: String,
    pub ascension_level: i32,
    pub seed_hint: u64,
    pub encounter_name: Option<String>,
    pub run_rule_context_summary: Option<String>,
    pub initial_engine_state: EngineState,
    pub initial_combat: CombatState,
}

impl CombatEnvSpec {
    pub fn from_fixture(fixture: &ScenarioFixture, seed_hint: u64) -> Self {
        let initial = initialize_fixture_state(fixture);
        let encounter_name = initial
            .combat
            .entities
            .monsters
            .first()
            .and_then(|monster| crate::content::monsters::EnemyId::from_id(monster.monster_type))
            .map(|enemy| enemy.get_name().to_string());
        Self {
            name: fixture.name.clone(),
            player_class: initial.combat.meta.player_class.to_string(),
            ascension_level: initial.combat.meta.ascension_level as i32,
            seed_hint,
            encounter_name,
            run_rule_context_summary: None,
            initial_engine_state: initial.engine_state,
            initial_combat: initial.combat,
        }
    }

    pub fn from_combat(
        name: impl Into<String>,
        seed_hint: u64,
        initial_engine_state: EngineState,
        initial_combat: CombatState,
    ) -> Self {
        let encounter_name = initial_combat
            .entities
            .monsters
            .first()
            .and_then(|monster| crate::content::monsters::EnemyId::from_id(monster.monster_type))
            .map(|enemy| enemy.get_name().to_string());
        Self {
            name: name.into(),
            player_class: initial_combat.meta.player_class.to_string(),
            ascension_level: initial_combat.meta.ascension_level as i32,
            seed_hint,
            encounter_name,
            run_rule_context_summary: None,
            initial_engine_state,
            initial_combat,
        }
    }

    pub fn from_start_spec(start_spec: &CombatStartSpec) -> Result<Self, String> {
        Self::from_start_spec_with_seed(start_spec, start_spec.seed)
    }

    pub fn from_start_spec_with_seed(
        start_spec: &CombatStartSpec,
        seed: u64,
    ) -> Result<Self, String> {
        let (engine_state, combat) =
            crate::testing::fixtures::combat_start_spec::compile_combat_start_spec_with_seed(
                start_spec, seed,
            )?;
        let mut spec = Self::from_combat(start_spec.name.clone(), seed, engine_state, combat);
        spec.run_rule_context_summary = Some(format!(
            "natural_start encounter={} room_type={} seed={}",
            start_spec.encounter_id, start_spec.room_type, seed
        ));
        Ok(spec)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CombatAction {
    EndTurn,
    PlayCard {
        card_index: usize,
        target: Option<EntityId>,
    },
    UsePotion {
        potion_index: usize,
        target: Option<EntityId>,
    },
    SubmitDiscoverChoice {
        index: usize,
    },
    SubmitCardChoice {
        indices: Vec<usize>,
    },
    SubmitHandSelect {
        uuids: Vec<u32>,
    },
    SubmitGridSelect {
        uuids: Vec<u32>,
    },
    Proceed,
    Cancel,
    Raw {
        input: ClientInput,
    },
}

impl CombatAction {
    pub fn to_client_input(&self) -> ClientInput {
        match self {
            Self::EndTurn => ClientInput::EndTurn,
            Self::PlayCard { card_index, target } => ClientInput::PlayCard {
                card_index: *card_index,
                target: *target,
            },
            Self::UsePotion {
                potion_index,
                target,
            } => ClientInput::UsePotion {
                potion_index: *potion_index,
                target: *target,
            },
            Self::SubmitDiscoverChoice { index } => ClientInput::SubmitDiscoverChoice(*index),
            Self::SubmitCardChoice { indices } => ClientInput::SubmitCardChoice(indices.clone()),
            Self::SubmitHandSelect { uuids } => ClientInput::SubmitHandSelect(uuids.clone()),
            Self::SubmitGridSelect { uuids } => ClientInput::SubmitGridSelect(uuids.clone()),
            Self::Proceed => ClientInput::Proceed,
            Self::Cancel => ClientInput::Cancel,
            Self::Raw { input } => input.clone(),
        }
    }

    pub fn label(&self, combat: &CombatState) -> String {
        match self {
            Self::EndTurn => "EndTurn".to_string(),
            Self::PlayCard { card_index, target } => {
                let card = combat
                    .zones
                    .hand
                    .get(*card_index)
                    .map(format_card)
                    .unwrap_or_else(|| format!("hand[{card_index}]"));
                match target {
                    Some(target) => format!("Play #{} {} @{}", card_index + 1, card, target),
                    None => format!("Play #{} {}", card_index + 1, card),
                }
            }
            Self::UsePotion {
                potion_index,
                target,
            } => match target {
                Some(target) => format!("UsePotion#{} @{}", potion_index, target),
                None => format!("UsePotion#{}", potion_index),
            },
            Self::SubmitDiscoverChoice { index } => format!("DiscoverChoice#{index}"),
            Self::SubmitCardChoice { indices } => format!("CardChoice{:?}", indices),
            Self::SubmitHandSelect { uuids } => format!("HandSelect{:?}", uuids),
            Self::SubmitGridSelect { uuids } => format!("GridSelect{:?}", uuids),
            Self::Proceed => "Proceed".to_string(),
            Self::Cancel => "Cancel".to_string(),
            Self::Raw { input } => format!("{input:?}"),
        }
    }
}

impl From<ClientInput> for CombatAction {
    fn from(value: ClientInput) -> Self {
        match value {
            ClientInput::EndTurn => Self::EndTurn,
            ClientInput::PlayCard { card_index, target } => Self::PlayCard { card_index, target },
            ClientInput::UsePotion {
                potion_index,
                target,
            } => Self::UsePotion {
                potion_index,
                target,
            },
            ClientInput::SubmitDiscoverChoice(index) => Self::SubmitDiscoverChoice { index },
            ClientInput::SubmitCardChoice(indices) => Self::SubmitCardChoice { indices },
            ClientInput::SubmitHandSelect(uuids) => Self::SubmitHandSelect { uuids },
            ClientInput::SubmitGridSelect(uuids) => Self::SubmitGridSelect { uuids },
            ClientInput::Proceed => Self::Proceed,
            ClientInput::Cancel => Self::Cancel,
            other => Self::Raw { input: other },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionMask {
    pub candidate_actions: Vec<CombatAction>,
    pub legal: Vec<bool>,
}

impl ActionMask {
    pub fn legal_actions(&self) -> Vec<CombatAction> {
        self.candidate_actions
            .iter()
            .zip(&self.legal)
            .filter(|(_, legal)| **legal)
            .map(|(action, _)| action.clone())
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatObservationCard {
    pub index: usize,
    pub uuid: u32,
    pub card_id: String,
    pub name: String,
    pub cost_for_turn: i32,
    pub upgraded: bool,
    pub playable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatObservationMonster {
    pub entity_id: usize,
    pub name: String,
    pub current_hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub visible_intent: String,
    pub belief_certainty: String,
    pub belief_expected_incoming: f32,
    pub belief_max_incoming: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatBeliefObservation {
    pub hidden_intent_active: bool,
    pub expected_incoming_damage: f32,
    pub max_incoming_damage: i32,
    pub attack_probability: f32,
    pub lethal_probability: f32,
    pub urgent_probability: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatPressureObservation {
    pub visible_incoming: i32,
    pub visible_unblocked: i32,
    pub belief_expected_incoming: i32,
    pub belief_expected_unblocked: i32,
    pub belief_max_incoming: i32,
    pub belief_max_unblocked: i32,
    pub value_incoming: i32,
    pub value_unblocked: i32,
    pub survival_guard_incoming: i32,
    pub survival_guard_unblocked: i32,
    pub lethal_pressure: bool,
    pub urgent_pressure: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatObservation {
    pub env_name: String,
    pub player_class: String,
    pub ascension_level: i32,
    pub encounter_name: Option<String>,
    pub run_rule_context_summary: Option<String>,
    pub turn_count: u32,
    pub phase: String,
    pub engine_state: String,
    pub player_hp: i32,
    pub player_max_hp: i32,
    pub player_block: i32,
    pub energy: u8,
    pub hand: Vec<CombatObservationCard>,
    pub draw_count: usize,
    pub discard_count: usize,
    pub exhaust_count: usize,
    pub monsters: Vec<CombatObservationMonster>,
    pub belief: CombatBeliefObservation,
    pub pressure: CombatPressureObservation,
    pub pending_choice_kind: Option<String>,
    pub hexaghost_future_script: Option<HexaghostFutureScriptSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CombatEpisodeOutcome {
    Victory,
    Defeat,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CombatRewardBreakdown {
    pub enemy_hp_delta: f32,
    pub player_hp_delta: f32,
    pub incoming_relief: f32,
    pub next_enemy_window_hp_loss_baseline: f32,
    pub next_enemy_window_hp_loss_after_action: f32,
    pub next_enemy_window_relief: f32,
    pub persistent_attack_script_relief: f32,
    pub persistent_multihit_attack_script_relief: f32,
    pub persistent_attack_windows_affected: f32,
    pub persistent_inferno_damage_prevented: f32,
    pub kill_bonus: f32,
    pub stabilize_bonus: f32,
    pub idle_penalty: f32,
    pub hexaghost_persistent_attack_script: Option<HexaghostPersistentAttackScriptValue>,
    pub total: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombatStepInfo {
    pub chosen_action: CombatAction,
    pub chosen_action_label: String,
    pub reward: f32,
    pub reward_breakdown: CombatRewardBreakdown,
    pub done: bool,
    pub outcome: Option<CombatEpisodeOutcome>,
    pub observation: CombatObservation,
    pub legal_actions_after: usize,
}

pub struct CombatEnv {
    spec: CombatEnvSpec,
    engine_state: EngineState,
    combat: CombatState,
    episode_steps: usize,
}

impl CombatEnv {
    pub fn new(spec: CombatEnvSpec) -> Self {
        Self {
            engine_state: spec.initial_engine_state.clone(),
            combat: spec.initial_combat.clone(),
            spec,
            episode_steps: 0,
        }
    }

    pub fn reset(&mut self, spec: Option<CombatEnvSpec>) -> CombatObservation {
        if let Some(spec) = spec {
            self.spec = spec;
        }
        self.engine_state = self.spec.initial_engine_state.clone();
        self.combat = self.spec.initial_combat.clone();
        self.episode_steps = 0;
        self.observation()
    }

    pub fn observation(&self) -> CombatObservation {
        build_observation(&self.spec, &self.engine_state, &self.combat)
    }

    pub fn action_mask(&self) -> ActionMask {
        build_action_mask(&self.engine_state, &self.combat)
    }

    pub fn legal_actions(&self) -> Vec<CombatAction> {
        self.action_mask().legal_actions()
    }

    pub fn current_engine_state(&self) -> &EngineState {
        &self.engine_state
    }

    pub fn current_combat(&self) -> &CombatState {
        &self.combat
    }

    pub fn step(&mut self, action: CombatAction) -> Result<CombatStepInfo, String> {
        let mask = self.action_mask();
        let desired_input = action.to_client_input();
        if !mask
            .candidate_actions
            .iter()
            .zip(&mask.legal)
            .any(|(candidate, legal)| *legal && candidate.to_client_input() == desired_input)
        {
            return Err(format!("illegal combat action: {:?}", desired_input));
        }

        let before_engine_state = self.engine_state.clone();
        let before_combat = self.combat.clone();
        let alive = tick_until_stable_turn(&mut self.engine_state, &mut self.combat, desired_input);
        self.episode_steps += 1;

        let outcome = if !alive || self.combat.entities.player.current_hp <= 0 {
            Some(CombatEpisodeOutcome::Defeat)
        } else if living_monster_count(&self.combat) == 0 {
            Some(CombatEpisodeOutcome::Victory)
        } else {
            None
        };
        let done = outcome.is_some();
        let reward_breakdown = reward_breakdown(
            &before_engine_state,
            &before_combat,
            &self.engine_state,
            &self.combat,
            &action,
            done,
        );
        let observation = self.observation();
        let legal_actions_after = self.legal_actions().len();

        Ok(CombatStepInfo {
            chosen_action_label: action.label(&before_combat),
            chosen_action: action,
            reward: reward_breakdown.total,
            reward_breakdown,
            done,
            outcome,
            observation,
            legal_actions_after,
        })
    }
}

fn build_observation(
    spec: &CombatEnvSpec,
    engine_state: &EngineState,
    combat: &CombatState,
) -> CombatObservation {
    let belief = build_combat_belief_state(combat);
    let pressure = StatePressureFeatures::from_combat(combat);
    CombatObservation {
        env_name: spec.name.clone(),
        player_class: spec.player_class.clone(),
        ascension_level: spec.ascension_level,
        encounter_name: spec.encounter_name.clone(),
        run_rule_context_summary: spec.run_rule_context_summary.clone(),
        turn_count: combat.turn.turn_count,
        phase: combat_phase_name(combat.turn.current_phase),
        engine_state: format!("{engine_state:?}"),
        player_hp: combat.entities.player.current_hp,
        player_max_hp: combat.entities.player.max_hp,
        player_block: combat.entities.player.block,
        energy: combat.turn.energy,
        hand: combat
            .zones
            .hand
            .iter()
            .enumerate()
            .map(|(index, card)| CombatObservationCard {
                index,
                uuid: card.uuid,
                card_id: format!("{:?}", card.id),
                name: format_card(card),
                cost_for_turn: card
                    .cost_for_turn
                    .map(i32::from)
                    .unwrap_or(get_card_definition(card.id).cost as i32),
                upgraded: card.upgrades > 0,
                playable: can_play_card(card, combat).is_ok(),
            })
            .collect(),
        draw_count: combat.zones.draw_pile.len(),
        discard_count: combat.zones.discard_pile.len(),
        exhaust_count: combat.zones.exhaust_pile.len(),
        monsters: build_monster_observations(combat, &belief),
        belief: CombatBeliefObservation {
            hidden_intent_active: belief.hidden_intent_active,
            expected_incoming_damage: belief.expected_incoming_damage,
            max_incoming_damage: belief.max_incoming_damage,
            attack_probability: belief.attack_probability,
            lethal_probability: belief.lethal_probability,
            urgent_probability: belief.urgent_probability,
        },
        pressure: CombatPressureObservation {
            visible_incoming: pressure.visible_incoming,
            visible_unblocked: pressure.visible_unblocked,
            belief_expected_incoming: pressure.belief_expected_incoming,
            belief_expected_unblocked: pressure.belief_expected_unblocked,
            belief_max_incoming: pressure.belief_max_incoming,
            belief_max_unblocked: pressure.belief_max_unblocked,
            value_incoming: pressure.value_incoming,
            value_unblocked: pressure.value_unblocked,
            survival_guard_incoming: pressure.survival_guard_incoming,
            survival_guard_unblocked: pressure.survival_guard_unblocked,
            lethal_pressure: pressure.lethal_pressure,
            urgent_pressure: pressure.urgent_pressure,
        },
        pending_choice_kind: pending_choice_kind(engine_state),
        hexaghost_future_script: project_hexaghost_future_script(engine_state, combat),
    }
}

fn build_monster_observations(
    combat: &CombatState,
    belief: &CombatBeliefState,
) -> Vec<CombatObservationMonster> {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .map(|monster| {
            let belief_state = belief
                .monsters
                .iter()
                .find(|entry| entry.entity_id == monster.id);
            CombatObservationMonster {
                entity_id: monster.id,
                name: crate::content::monsters::EnemyId::from_id(monster.monster_type)
                    .map(|enemy| enemy.get_name().to_string())
                    .unwrap_or_else(|| "Unknown".to_string()),
                current_hp: monster.current_hp,
                max_hp: monster.max_hp,
                block: monster.block,
                visible_intent: format!("{:?}", monster.current_intent),
                belief_certainty: belief_state
                    .map(|state| match state.certainty {
                        MonsterBeliefCertainty::Exact => "exact",
                        MonsterBeliefCertainty::Distribution => "distribution",
                        MonsterBeliefCertainty::Unknown => "unknown",
                    })
                    .unwrap_or("unknown")
                    .to_string(),
                belief_expected_incoming: belief_state
                    .map(|state| state.expected_incoming_damage)
                    .unwrap_or(0.0),
                belief_max_incoming: belief_state
                    .map(|state| state.max_incoming_damage)
                    .unwrap_or(0),
            }
        })
        .collect()
}

fn build_action_mask(engine_state: &EngineState, combat: &CombatState) -> ActionMask {
    let legal_inputs = legal_moves_for_audit(engine_state, combat);
    let candidate_actions = enumerate_candidate_actions(engine_state, combat);
    let legal = candidate_actions
        .iter()
        .map(|candidate| {
            let input = candidate.to_client_input();
            legal_inputs.iter().any(|legal| *legal == input)
        })
        .collect();
    ActionMask {
        candidate_actions,
        legal,
    }
}

fn enumerate_candidate_actions(
    engine_state: &EngineState,
    combat: &CombatState,
) -> Vec<CombatAction> {
    match engine_state {
        EngineState::CombatPlayerTurn => {
            let mut actions = Vec::new();
            actions.push(CombatAction::EndTurn);

            for (potion_index, maybe_potion) in combat.entities.potions.iter().enumerate() {
                if maybe_potion.is_none() {
                    continue;
                }
                let legal_targeted = legal_moves_for_audit(engine_state, combat)
                    .into_iter()
                    .filter_map(|input| match input {
                        ClientInput::UsePotion {
                            potion_index: idx,
                            target,
                        } if idx == potion_index => Some(target),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if legal_targeted.is_empty() {
                    actions.push(CombatAction::UsePotion {
                        potion_index,
                        target: None,
                    });
                } else {
                    for target in dedup_targets(legal_targeted) {
                        actions.push(CombatAction::UsePotion {
                            potion_index,
                            target,
                        });
                    }
                }
            }

            for (card_index, card) in combat.zones.hand.iter().enumerate() {
                let target_type = effective_target(card);
                if let Some(validation) = targeting::validation_for_card_target(target_type) {
                    let targets = targeting::candidate_targets(combat, validation);
                    for target in targets {
                        actions.push(CombatAction::PlayCard {
                            card_index,
                            target: Some(target),
                        });
                    }
                } else {
                    actions.push(CombatAction::PlayCard {
                        card_index,
                        target: None,
                    });
                }
            }
            dedup_actions(actions)
        }
        EngineState::PendingChoice(choice) => {
            enumerate_pending_choice_actions(choice, engine_state, combat)
        }
        _ => dedup_actions(
            legal_moves_for_audit(engine_state, combat)
                .into_iter()
                .map(CombatAction::from)
                .collect(),
        ),
    }
}

fn enumerate_pending_choice_actions(
    choice: &PendingChoice,
    engine_state: &EngineState,
    combat: &CombatState,
) -> Vec<CombatAction> {
    match choice {
        PendingChoice::DiscoverySelect(cards) => (0..cards.len())
            .map(|index| CombatAction::SubmitDiscoverChoice { index })
            .collect(),
        PendingChoice::CardRewardSelect {
            cards, can_skip, ..
        } => {
            let mut actions = (0..cards.len())
                .map(|index| CombatAction::SubmitCardChoice {
                    indices: vec![index],
                })
                .collect::<Vec<_>>();
            if *can_skip {
                actions.push(CombatAction::Cancel);
            }
            actions
        }
        PendingChoice::StanceChoice => vec![
            CombatAction::SubmitDiscoverChoice { index: 0 },
            CombatAction::SubmitDiscoverChoice { index: 1 },
        ],
        PendingChoice::HandSelect { .. } | PendingChoice::GridSelect { .. } => dedup_actions(
            legal_moves_for_audit(engine_state, combat)
                .into_iter()
                .map(CombatAction::from)
                .collect(),
        ),
        _ => dedup_actions(
            legal_moves_for_audit(engine_state, combat)
                .into_iter()
                .map(CombatAction::from)
                .collect(),
        ),
    }
}

fn dedup_actions(actions: Vec<CombatAction>) -> Vec<CombatAction> {
    let mut deduped = Vec::new();
    for action in actions {
        if !deduped.contains(&action) {
            deduped.push(action);
        }
    }
    deduped
}

fn dedup_targets(targets: Vec<Option<EntityId>>) -> Vec<Option<EntityId>> {
    let mut deduped = Vec::new();
    for target in targets {
        if !deduped.contains(&target) {
            deduped.push(target);
        }
    }
    deduped
}

fn reward_breakdown(
    before_engine_state: &EngineState,
    before: &CombatState,
    after_engine_state: &EngineState,
    after: &CombatState,
    action: &CombatAction,
    done: bool,
) -> CombatRewardBreakdown {
    let before_enemy_hp = total_enemy_hp(before);
    let after_enemy_hp = total_enemy_hp(after);
    let enemy_hp_delta = (before_enemy_hp - after_enemy_hp).max(0) as f32;
    let player_hp_delta =
        (after.entities.player.current_hp - before.entities.player.current_hp).min(0) as f32;

    let before_belief = build_combat_belief_state(before);
    let after_belief = build_combat_belief_state(after);
    let before_unblocked = (before_belief.expected_incoming_damage.round() as i32
        - before.entities.player.block)
        .max(0);
    let after_unblocked =
        (after_belief.expected_incoming_damage.round() as i32 - after.entities.player.block).max(0);
    let incoming_relief = (before_unblocked - after_unblocked).max(0) as f32 * 0.35;
    let next_enemy_window_hp_loss_baseline =
        simulate_next_enemy_window_hp_loss(before_engine_state, before);
    let next_enemy_window_hp_loss_after_action = if matches!(action, CombatAction::EndTurn) {
        (before.entities.player.current_hp - after.entities.player.current_hp).max(0) as f32
    } else if done {
        0.0
    } else {
        simulate_next_enemy_window_hp_loss(after_engine_state, after)
    };
    let next_enemy_window_relief =
        (next_enemy_window_hp_loss_baseline - next_enemy_window_hp_loss_after_action).max(0.0);
    let persistent_attack_script = if matches!(action, CombatAction::EndTurn)
        || !matches!(before_engine_state, EngineState::CombatPlayerTurn)
        || !matches!(after_engine_state, EngineState::CombatPlayerTurn)
        || before.turn.turn_count != after.turn.turn_count
    {
        None
    } else {
        analyze_hexaghost_persistent_attack_script(
            before_engine_state,
            before,
            after_engine_state,
            after,
        )
    };
    let persistent_attack_script_relief = persistent_attack_script
        .as_ref()
        .map(|value| value.future_raw_damage_prevented_total as f32)
        .unwrap_or(0.0);
    let persistent_multihit_attack_script_relief = persistent_attack_script
        .as_ref()
        .map(|value| value.future_multihit_damage_prevented_total as f32)
        .unwrap_or(0.0);
    let persistent_attack_windows_affected = persistent_attack_script
        .as_ref()
        .map(|value| value.future_attack_windows_affected as f32)
        .unwrap_or(0.0);
    let persistent_inferno_damage_prevented = persistent_attack_script
        .as_ref()
        .map(|value| value.future_inferno_damage_prevented as f32)
        .unwrap_or(0.0);
    let kill_bonus = ((living_monster_count(before) as i32 - living_monster_count(after) as i32)
        .max(0)
        * 2) as f32;
    let stabilize_bonus = if before_unblocked > 0 && after_unblocked == 0 {
        1.5
    } else {
        0.0
    };
    let idle_penalty = if !done
        && enemy_hp_delta <= 0.0
        && incoming_relief <= 0.0
        && after.entities.player.block <= before.entities.player.block
        && !matches!(action, CombatAction::EndTurn)
    {
        -0.75
    } else {
        0.0
    };
    let total = enemy_hp_delta
        + player_hp_delta * 1.5
        + incoming_relief
        + kill_bonus
        + stabilize_bonus
        + idle_penalty;

    CombatRewardBreakdown {
        enemy_hp_delta,
        player_hp_delta,
        incoming_relief,
        next_enemy_window_hp_loss_baseline,
        next_enemy_window_hp_loss_after_action,
        next_enemy_window_relief,
        persistent_attack_script_relief,
        persistent_multihit_attack_script_relief,
        persistent_attack_windows_affected,
        persistent_inferno_damage_prevented,
        kill_bonus,
        stabilize_bonus,
        idle_penalty,
        hexaghost_persistent_attack_script: persistent_attack_script,
        total,
    }
}

fn simulate_next_enemy_window_hp_loss(engine_state: &EngineState, combat: &CombatState) -> f32 {
    if !matches!(engine_state, EngineState::CombatPlayerTurn) {
        return 0.0;
    }
    let mut engine_clone = engine_state.clone();
    let mut combat_clone = combat.clone();
    let hp_before = combat_clone.entities.player.current_hp;
    let _alive = tick_until_stable_turn(&mut engine_clone, &mut combat_clone, ClientInput::EndTurn);
    (hp_before - combat_clone.entities.player.current_hp).max(0) as f32
}

fn total_enemy_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .map(|monster| monster.current_hp.max(0))
        .sum()
}

fn living_monster_count(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_dying && !monster.is_escaped && !monster.half_dead && monster.current_hp > 0
        })
        .count()
}

fn combat_phase_name(phase: CombatPhase) -> String {
    format!("{phase:?}")
}

fn pending_choice_kind(engine_state: &EngineState) -> Option<String> {
    match engine_state {
        EngineState::PendingChoice(choice) => Some(
            match choice {
                PendingChoice::GridSelect { .. } => "grid_select",
                PendingChoice::HandSelect { .. } => "hand_select",
                PendingChoice::DiscoverySelect(_) => "discovery_select",
                PendingChoice::ScrySelect { .. } => "scry_select",
                PendingChoice::CardRewardSelect { .. } => "card_reward_select",
                PendingChoice::StanceChoice => "stance_choice",
            }
            .to_string(),
        ),
        _ => None,
    }
}

fn format_card(card: &CombatCard) -> String {
    let mut label = get_card_definition(card.id).name.to_string();
    for _ in 0..card.upgrades {
        label.push('+');
    }
    label
}

#[cfg(test)]
mod tests {
    use super::{CombatAction, CombatEnv, CombatEnvSpec};
    use crate::combat::{CombatCard, Intent};
    use crate::content::cards::CardId;
    use crate::state::core::ClientInput;
    use crate::state::EngineState;
    use crate::testing::fixtures::author_spec::AuthorCardSpec;
    use crate::testing::fixtures::combat_start_spec::CombatStartSpec;
    use crate::testing::support::test_support::{combat_with_hand_and_intent, CombatTestExt};

    fn hexaghost_start_spec() -> CombatStartSpec {
        CombatStartSpec {
            name: "hexaghost_start_spec_test".to_string(),
            player_class: "Ironclad".to_string(),
            ascension_level: 0,
            encounter_id: "Hexaghost".to_string(),
            room_type: "MonsterRoomBoss".to_string(),
            seed: 777,
            player_current_hp: 36,
            player_max_hp: 80,
            relics: vec![],
            potions: vec![],
            master_deck: vec![
                AuthorCardSpec::Simple("Disarm".to_string()),
                AuthorCardSpec::Simple("Bash".to_string()),
                AuthorCardSpec::Simple("Defend_R".to_string()),
                AuthorCardSpec::Simple("Defend_R".to_string()),
                AuthorCardSpec::Simple("Shrug It Off".to_string()),
            ],
        }
    }

    #[test]
    fn action_mask_hides_unplayable_cards() {
        let combat = combat_with_hand_and_intent(
            vec![
                CombatCard::new(CardId::Strike, 1),
                CombatCard::new(CardId::Defend, 2),
            ],
            Intent::Buff,
        )
        .with_energy(0);
        let env = CombatEnv::new(CombatEnvSpec::from_combat(
            "mask_unplayable",
            7,
            EngineState::CombatPlayerTurn,
            combat,
        ));
        let mask = env.action_mask();
        let end_turn = mask
            .candidate_actions
            .iter()
            .position(|action| matches!(action, CombatAction::EndTurn))
            .expect("end turn candidate");
        let strike = mask
            .candidate_actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    CombatAction::PlayCard {
                        card_index: 0,
                        target: Some(1)
                    }
                )
            })
            .expect("strike candidate");
        assert!(mask.legal[end_turn]);
        assert!(!mask.legal[strike]);
    }

    #[test]
    fn reset_is_deterministic_for_same_spec() {
        let combat = combat_with_hand_and_intent(
            vec![CombatCard::new(CardId::Strike, 1)],
            Intent::Attack { damage: 6, hits: 1 },
        );
        let spec =
            CombatEnvSpec::from_combat("deterministic", 11, EngineState::CombatPlayerTurn, combat);
        let mut env = CombatEnv::new(spec.clone());
        let obs_a = env.reset(None);
        let obs_b = env.reset(Some(spec));
        assert_eq!(obs_a, obs_b);
    }

    #[test]
    fn start_spec_reset_reaches_playable_player_turn() {
        let start_spec = hexaghost_start_spec();
        let spec = CombatEnvSpec::from_start_spec(&start_spec).expect("start spec");
        let env = CombatEnv::new(spec);
        let obs = env.observation();
        let mask = env.action_mask();
        assert_eq!(obs.engine_state, "CombatPlayerTurn");
        assert!(!obs.hand.is_empty());
        assert!(mask.legal.iter().any(|legal| *legal));
    }

    #[test]
    fn hexaghost_projection_from_natural_start_reaches_inferno() {
        let spec = CombatEnvSpec::from_start_spec(&hexaghost_start_spec()).expect("start spec");
        let env = CombatEnv::new(spec);
        let obs = env.observation();
        let script = obs
            .hexaghost_future_script
            .expect("hexaghost future script summary");
        assert!(script
            .windows
            .iter()
            .any(|window| window.move_kind == "Inferno"));
    }

    #[test]
    fn disarm_step_exports_positive_hexaghost_persistent_attack_script_relief() {
        let spec = CombatEnvSpec::from_start_spec(&hexaghost_start_spec()).expect("start spec");
        let mut env = CombatEnv::new(spec);
        let mask = env.action_mask();
        let disarm_index = mask
            .candidate_actions
            .iter()
            .position(|action| action.label(env.current_combat()).contains("Disarm"))
            .expect("disarm action index");
        let step = env
            .step(mask.candidate_actions[disarm_index].clone())
            .expect("step disarm");
        assert!(
            step.reward_breakdown.persistent_attack_script_relief > 0.0,
            "{:?}",
            step.reward_breakdown
        );
        let persistent = step
            .reward_breakdown
            .hexaghost_persistent_attack_script
            .expect("persistent script summary");
        assert!(persistent.future_inferno_damage_prevented >= 0);
    }

    #[test]
    fn step_replays_flex_then_strike_cleanly() {
        let combat = combat_with_hand_and_intent(
            vec![
                CombatCard::new(CardId::Flex, 1),
                CombatCard::new(CardId::Strike, 2),
            ],
            Intent::Buff,
        )
        .with_energy(2);
        let mut env = CombatEnv::new(CombatEnvSpec::from_combat(
            "flex_then_strike",
            19,
            EngineState::CombatPlayerTurn,
            combat,
        ));
        let first = env
            .step(CombatAction::PlayCard {
                card_index: 0,
                target: None,
            })
            .expect("play flex");
        assert!(!first.done);
        let second = env
            .step(CombatAction::PlayCard {
                card_index: 0,
                target: Some(1),
            })
            .expect("play strike");
        assert!(!second.done);
        assert!(
            second.reward_breakdown.enemy_hp_delta >= 6.0,
            "{:?}",
            second
        );
    }

    #[test]
    fn hidden_intent_observation_surfaces_belief() {
        let mut combat =
            combat_with_hand_and_intent(vec![CombatCard::new(CardId::Strike, 1)], Intent::Unknown);
        let monster = combat.entities.monsters.first_mut().expect("monster");
        monster.move_history.push_back(3);
        let env = CombatEnv::new(CombatEnvSpec::from_combat(
            "hidden_intent",
            23,
            EngineState::CombatPlayerTurn,
            combat,
        ));
        let obs = env.observation();
        assert!(obs.belief.hidden_intent_active);
        assert_eq!(obs.monsters.len(), 1);
    }

    #[test]
    fn illegal_step_is_rejected() {
        let combat =
            combat_with_hand_and_intent(vec![CombatCard::new(CardId::Strike, 1)], Intent::Buff)
                .with_energy(0);
        let mut env = CombatEnv::new(CombatEnvSpec::from_combat(
            "illegal",
            31,
            EngineState::CombatPlayerTurn,
            combat,
        ));
        let err = env
            .step(CombatAction::PlayCard {
                card_index: 0,
                target: Some(1),
            })
            .expect_err("illegal play should fail");
        assert!(err.contains("illegal combat action"));
    }

    #[test]
    fn action_round_trip_preserves_client_input() {
        let input = ClientInput::PlayCard {
            card_index: 2,
            target: Some(7),
        };
        let action = CombatAction::from(input.clone());
        assert_eq!(action.to_client_input(), input);
    }

    #[test]
    fn headbutt_single_discard_candidate_auto_moves_card_to_draw_pile() {
        let combat =
            combat_with_hand_and_intent(vec![CombatCard::new(CardId::Headbutt, 1)], Intent::Buff)
                .with_energy(1)
                .with_discard_cards(vec![CombatCard::new(CardId::Strike, 200)]);
        let mut env = CombatEnv::new(CombatEnvSpec::from_combat(
            "headbutt_single_candidate",
            41,
            EngineState::CombatPlayerTurn,
            combat,
        ));

        let step = env
            .step(CombatAction::PlayCard {
                card_index: 0,
                target: Some(1),
            })
            .expect("play headbutt");

        assert_eq!(step.observation.pending_choice_kind, None);
        assert!(
            env.current_combat()
                .zones
                .discard_pile
                .iter()
                .all(|card| card.uuid != 200),
            "{:?}",
            env.current_combat().zones.discard_pile
        );
        assert!(
            env.current_combat()
                .zones
                .draw_pile
                .iter()
                .any(|card| card.uuid == 200),
            "{:?}",
            env.current_combat().zones.draw_pile
        );
    }

    #[test]
    fn headbutt_multiple_discard_candidates_enters_grid_select_and_resolves() {
        let first = CombatCard::new(CardId::Strike, 200);
        let second = CombatCard::new(CardId::Defend, 201);
        let combat =
            combat_with_hand_and_intent(vec![CombatCard::new(CardId::Headbutt, 1)], Intent::Buff)
                .with_energy(1)
                .with_discard_cards(vec![first.clone(), second.clone()]);
        let mut env = CombatEnv::new(CombatEnvSpec::from_combat(
            "headbutt_multiple_candidates",
            43,
            EngineState::CombatPlayerTurn,
            combat,
        ));

        let headbutt = env
            .step(CombatAction::PlayCard {
                card_index: 0,
                target: Some(1),
            })
            .expect("play headbutt");
        assert_eq!(
            headbutt.observation.pending_choice_kind.as_deref(),
            Some("grid_select")
        );

        let resolve = env
            .step(CombatAction::SubmitGridSelect {
                uuids: vec![second.uuid],
            })
            .expect("resolve grid select");
        assert_eq!(resolve.observation.pending_choice_kind, None);
        assert!(env
            .current_combat()
            .zones
            .discard_pile
            .iter()
            .any(|card| card.uuid == first.uuid));
        assert!(env
            .current_combat()
            .zones
            .discard_pile
            .iter()
            .all(|card| card.uuid != second.uuid));
        assert!(env
            .current_combat()
            .zones
            .draw_pile
            .iter()
            .any(|card| card.uuid == second.uuid));
    }
}
