use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::bot::combat::legal_moves_for_audit;
use crate::bot::combat::monster_belief::{
    build_combat_belief_state, CombatBeliefState, MonsterBeliefCertainty,
};
use crate::bot::combat::pressure::StatePressureFeatures;
use crate::content::cards::{
    can_play_card, effective_target, exhausts_when_played, get_card_definition, is_ethereal,
    CardTarget, CardType,
};
use crate::content::powers::{get_power_definition, is_debuff};
use crate::core::EntityId;
use crate::engine::core::tick_until_stable_turn;
use crate::engine::targeting;
use crate::runtime::combat::{CombatCard, CombatPhase, CombatState};
use crate::state::core::{ClientInput, EngineState, PendingChoice, PileType};

use crate::testing::fixtures::combat_case::{lower_case, CombatCase};
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
    pub fn from_case(case: &CombatCase, seed_hint_override: Option<u64>) -> Result<Self, String> {
        let initial = lower_case(case)?;
        let encounter_name = initial
            .combat
            .entities
            .monsters
            .first()
            .and_then(|monster| crate::content::monsters::EnemyId::from_id(monster.monster_type))
            .map(|enemy| enemy.get_name().to_string());
        Ok(Self {
            name: case.id.clone(),
            player_class: initial
                .player_class
                .unwrap_or_else(|| initial.combat.meta.player_class.to_string()),
            ascension_level: initial
                .ascension_level
                .map(i32::from)
                .unwrap_or(initial.combat.meta.ascension_level as i32),
            seed_hint: seed_hint_override.or(initial.seed_hint).unwrap_or(1),
            encounter_name,
            run_rule_context_summary: None,
            initial_engine_state: initial.engine_state,
            initial_combat: initial.combat,
        })
    }

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
    pub card_type: String,
    pub target_mode: String,
    pub cost_for_turn: i32,
    pub upgraded: bool,
    pub playable: bool,
    pub exhausts_when_played: bool,
    pub ethereal: bool,
    pub retain: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatObservationPower {
    pub id: String,
    pub name: String,
    pub amount: i32,
    pub extra_data: i32,
    pub just_applied: bool,
    pub is_debuff: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatObservationPotion {
    pub slot: usize,
    pub uuid: u32,
    pub potion_id: String,
    pub name: String,
    pub target_mode: String,
    pub usable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatObservationIntentPayload {
    pub kind: String,
    pub move_id: u8,
    pub damage_per_hit: i32,
    pub hits: i32,
    pub total_damage: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatObservationPendingChoiceOption {
    pub option_index: usize,
    pub label: String,
    pub card_id: Option<String>,
    pub card_uuid: Option<u32>,
    pub selection_uuids: Vec<u32>,
    pub source_pile: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatObservationPendingChoice {
    pub kind: String,
    pub min_select: u8,
    pub max_select: u8,
    pub can_cancel: bool,
    pub reason: Option<String>,
    pub source_pile: Option<String>,
    pub options: Vec<CombatObservationPendingChoiceOption>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatObservationTurnPrefix {
    pub cards_played_this_turn: u8,
    pub attacks_played_this_turn: u8,
    pub skills_played_this_turn: u8,
    pub powers_played_this_turn: u8,
    pub energy_spent_this_turn: i32,
    pub damage_dealt_this_turn: i32,
    pub damage_taken_this_turn: i32,
    pub last_action_family: Option<String>,
    pub last_card_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatObservationMonster {
    pub slot: usize,
    pub entity_id: usize,
    pub monster_id: String,
    pub name: String,
    pub current_hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub alive: bool,
    pub targetable: bool,
    pub visible_intent: String,
    pub intent_payload: CombatObservationIntentPayload,
    pub belief_certainty: String,
    pub belief_expected_incoming: f32,
    pub belief_max_incoming: i32,
    pub powers: Vec<CombatObservationPower>,
    pub mechanic_state: Value,
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
    pub contract_version: String,
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
    pub player_powers: Vec<CombatObservationPower>,
    pub hand: Vec<CombatObservationCard>,
    pub potions: Vec<CombatObservationPotion>,
    pub draw_count: usize,
    pub discard_count: usize,
    pub exhaust_count: usize,
    pub monsters: Vec<CombatObservationMonster>,
    pub turn_prefix: CombatObservationTurnPrefix,
    pub belief: CombatBeliefObservation,
    pub pressure: CombatPressureObservation,
    pub pending_choice_kind: Option<String>,
    pub pending_choice: Option<CombatObservationPendingChoice>,
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

#[derive(Clone, Debug, Default, PartialEq)]
struct CombatTurnPrefixTracker {
    pub skills_played_this_turn: u8,
    pub powers_played_this_turn: u8,
    pub energy_spent_this_turn: i32,
    pub damage_dealt_this_turn: i32,
    pub damage_taken_this_turn: i32,
    pub last_action_family: Option<String>,
    pub last_card_id: Option<String>,
}

pub struct CombatEnv {
    spec: CombatEnvSpec,
    engine_state: EngineState,
    combat: CombatState,
    episode_steps: usize,
    turn_prefix_tracker: CombatTurnPrefixTracker,
}

impl CombatEnv {
    pub fn new(spec: CombatEnvSpec) -> Self {
        Self {
            engine_state: spec.initial_engine_state.clone(),
            combat: spec.initial_combat.clone(),
            spec,
            episode_steps: 0,
            turn_prefix_tracker: CombatTurnPrefixTracker::default(),
        }
    }

    pub fn reset(&mut self, spec: Option<CombatEnvSpec>) -> CombatObservation {
        if let Some(spec) = spec {
            self.spec = spec;
        }
        self.engine_state = self.spec.initial_engine_state.clone();
        self.combat = self.spec.initial_combat.clone();
        self.episode_steps = 0;
        self.turn_prefix_tracker = CombatTurnPrefixTracker::default();
        self.observation()
    }

    pub fn observation(&self) -> CombatObservation {
        build_observation(
            &self.spec,
            &self.engine_state,
            &self.combat,
            &self.turn_prefix_tracker,
        )
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
        update_turn_prefix_tracker(
            &mut self.turn_prefix_tracker,
            &action,
            &before_combat,
            &self.combat,
        );
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
    turn_prefix_tracker: &CombatTurnPrefixTracker,
) -> CombatObservation {
    let belief = build_combat_belief_state(combat);
    let pressure = StatePressureFeatures::from_combat(combat);
    CombatObservation {
        contract_version: "combat_rl_v0".to_string(),
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
        player_powers: build_power_observations(combat, 0),
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
                card_type: card_type_name(get_card_definition(card.id).card_type),
                target_mode: card_target_mode_name(effective_target(card)),
                cost_for_turn: card
                    .cost_for_turn
                    .map(i32::from)
                    .unwrap_or(get_card_definition(card.id).cost as i32),
                upgraded: card.upgrades > 0,
                playable: can_play_card(card, combat).is_ok(),
                exhausts_when_played: exhausts_when_played(card),
                ethereal: is_ethereal(card),
                retain: effective_retain(card),
            })
            .collect(),
        potions: build_potion_observations(engine_state, combat),
        draw_count: combat.zones.draw_pile.len(),
        discard_count: combat.zones.discard_pile.len(),
        exhaust_count: combat.zones.exhaust_pile.len(),
        monsters: build_monster_observations(combat, &belief),
        turn_prefix: CombatObservationTurnPrefix {
            cards_played_this_turn: combat.turn.counters.cards_played_this_turn,
            attacks_played_this_turn: combat.turn.counters.attacks_played_this_turn,
            skills_played_this_turn: turn_prefix_tracker.skills_played_this_turn,
            powers_played_this_turn: turn_prefix_tracker.powers_played_this_turn,
            energy_spent_this_turn: turn_prefix_tracker.energy_spent_this_turn,
            damage_dealt_this_turn: turn_prefix_tracker.damage_dealt_this_turn,
            damage_taken_this_turn: turn_prefix_tracker.damage_taken_this_turn,
            last_action_family: turn_prefix_tracker.last_action_family.clone(),
            last_card_id: turn_prefix_tracker.last_card_id.clone(),
        },
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
        pending_choice: build_pending_choice_observation(engine_state, combat),
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
        .enumerate()
        .map(|(slot, monster)| {
            let belief_state = belief
                .monsters
                .iter()
                .find(|entry| entry.entity_id == monster.id);
            let enemy_id = crate::content::monsters::EnemyId::from_id(monster.monster_type);
            let turn_plan = crate::content::monsters::resolve_monster_turn_plan(combat, monster);
            let summary_spec = turn_plan.summary_spec();
            CombatObservationMonster {
                slot,
                entity_id: monster.id,
                monster_id: enemy_id
                    .map(|enemy| format!("{enemy:?}"))
                    .unwrap_or_else(|| "Unknown".to_string()),
                name: enemy_id
                    .map(|enemy| enemy.get_name().to_string())
                    .unwrap_or_else(|| "Unknown".to_string()),
                current_hp: monster.current_hp,
                max_hp: monster.max_hp,
                block: monster.block,
                alive: monster.current_hp > 0 && !monster.is_dying && !monster.half_dead,
                targetable: !monster.is_dying && !monster.is_escaped && !monster.half_dead,
                visible_intent: format!("{summary_spec:?}"),
                intent_payload: build_intent_payload(monster, &summary_spec),
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
                powers: build_power_observations(combat, monster.id),
                mechanic_state: build_monster_mechanic_state(combat, monster),
            }
        })
        .collect()
}

fn build_power_observations(
    combat: &CombatState,
    entity_id: EntityId,
) -> Vec<CombatObservationPower> {
    crate::content::powers::store::powers_snapshot_for(combat, entity_id)
        .into_iter()
        .map(|power| CombatObservationPower {
            id: format!("{:?}", power.power_type),
            name: get_power_definition(power.power_type).name.to_string(),
            amount: power.amount,
            extra_data: power.extra_data,
            just_applied: power.just_applied,
            is_debuff: is_debuff(power.power_type, power.amount),
        })
        .collect()
}

fn build_potion_observations(
    engine_state: &EngineState,
    combat: &CombatState,
) -> Vec<CombatObservationPotion> {
    let legal_targeted = legal_moves_for_audit(engine_state, combat);
    combat
        .entities
        .potions
        .iter()
        .enumerate()
        .filter_map(|(slot, maybe_potion)| {
            let potion = maybe_potion.as_ref()?;
            let definition = crate::content::potions::get_potion_definition(potion.id);
            let usable = legal_targeted.iter().any(|legal| match legal {
                ClientInput::UsePotion {
                    potion_index,
                    target: _,
                } => *potion_index == slot,
                _ => false,
            });
            Some(CombatObservationPotion {
                slot,
                uuid: potion.uuid,
                potion_id: format!("{:?}", potion.id),
                name: definition.name.to_string(),
                target_mode: potion_target_mode_name(definition.target_required),
                usable,
            })
        })
        .collect()
}

fn build_intent_payload(
    monster: &crate::runtime::combat::MonsterEntity,
    summary_spec: &crate::semantics::combat::MonsterMoveSpec,
) -> CombatObservationIntentPayload {
    let attack = summary_spec.attack();
    let damage_per_hit = attack.map(|spec| spec.base_damage).unwrap_or(0);
    let hits = attack.map(|spec| spec.hits as i32).unwrap_or(0);
    CombatObservationIntentPayload {
        kind: match summary_spec {
            crate::semantics::combat::MonsterMoveSpec::Attack(_) => "attack",
            crate::semantics::combat::MonsterMoveSpec::AttackAddCard(_, _) => "attack_add_card",
            crate::semantics::combat::MonsterMoveSpec::AttackUpgradeCards(_, _) => {
                "attack_upgrade_cards"
            }
            crate::semantics::combat::MonsterMoveSpec::AttackBuff(_, _) => "attack_buff",
            crate::semantics::combat::MonsterMoveSpec::AttackSustain(_) => "attack_sustain",
            crate::semantics::combat::MonsterMoveSpec::AttackDebuff(_, _) => "attack_debuff",
            crate::semantics::combat::MonsterMoveSpec::AttackDefend(_, _) => "attack_defend",
            crate::semantics::combat::MonsterMoveSpec::AddCard(_) => "add_card",
            crate::semantics::combat::MonsterMoveSpec::Buff(_) => "buff",
            crate::semantics::combat::MonsterMoveSpec::Debuff(_) => "debuff",
            crate::semantics::combat::MonsterMoveSpec::StrongDebuff(_) => "strong_debuff",
            crate::semantics::combat::MonsterMoveSpec::Defend(_) => "defend",
            crate::semantics::combat::MonsterMoveSpec::DefendDebuff(_, _) => "defend_debuff",
            crate::semantics::combat::MonsterMoveSpec::DefendBuff(_, _) => "defend_buff",
            crate::semantics::combat::MonsterMoveSpec::Heal(_) => "heal",
            crate::semantics::combat::MonsterMoveSpec::Escape => "escape",
            crate::semantics::combat::MonsterMoveSpec::Magic => "magic",
            crate::semantics::combat::MonsterMoveSpec::Sleep => "sleep",
            crate::semantics::combat::MonsterMoveSpec::Stun => "stun",
            crate::semantics::combat::MonsterMoveSpec::Debug => "debug",
            crate::semantics::combat::MonsterMoveSpec::None => "none",
            crate::semantics::combat::MonsterMoveSpec::Unknown => "unknown",
        }
        .to_string(),
        move_id: monster.planned_move_id(),
        damage_per_hit,
        hits,
        total_damage: damage_per_hit.saturating_mul(hits.max(1)),
    }
}

fn build_monster_mechanic_state(
    combat: &CombatState,
    monster: &crate::runtime::combat::MonsterEntity,
) -> Value {
    let enemy_id = crate::content::monsters::EnemyId::from_id(monster.monster_type);
    let has_split_power = crate::content::powers::store::has_power(
        combat,
        monster.id,
        crate::content::powers::PowerId::Split,
    );
    let regrow_counter = crate::content::powers::store::power_amount(
        combat,
        monster.id,
        crate::content::powers::PowerId::Regrow,
    );
    let mut mechanic_state = json!({
        "planned_move_id": monster.planned_move_id(),
        "move_history": monster.move_history().iter().copied().collect::<Vec<_>>(),
    });
    if let Some(object) = mechanic_state.as_object_mut() {
        if has_split_power {
            let split_threshold = monster.max_hp / 2;
            object.insert("split_threshold".to_string(), json!(split_threshold));
            object.insert(
                "split_ready".to_string(),
                json!(monster.current_hp <= split_threshold),
            );
        }
        if regrow_counter != 0
            || matches!(enemy_id, Some(crate::content::monsters::EnemyId::Darkling))
        {
            object.insert("regrow_counter".to_string(), json!(regrow_counter));
        }
        if let Some(bite_damage) = monster.louse_bite_damage() {
            object.insert("bite_damage".to_string(), json!(bite_damage));
        }
        match enemy_id {
            Some(crate::content::monsters::EnemyId::Lagavulin) => {
                object.insert("sleeping".to_string(), json!(!monster.lagavulin.is_out));
                object.insert(
                    "idle_count".to_string(),
                    json!(monster.lagavulin.idle_count),
                );
                object.insert(
                    "debuff_turn_count".to_string(),
                    json!(monster.lagavulin.debuff_turn_count),
                );
                object.insert(
                    "wake_triggered".to_string(),
                    json!(monster.lagavulin.is_out_triggered),
                );
            }
            Some(crate::content::monsters::EnemyId::TheGuardian) => {
                object.insert(
                    "guardian_threshold".to_string(),
                    json!(monster.guardian.damage_threshold),
                );
                object.insert(
                    "guardian_damage_taken".to_string(),
                    json!(monster.guardian.damage_taken),
                );
                object.insert("guardian_open".to_string(), json!(monster.guardian.is_open));
                object.insert(
                    "close_up_triggered".to_string(),
                    json!(monster.guardian.close_up_triggered),
                );
            }
            Some(crate::content::monsters::EnemyId::Darkling) => {
                object.insert("half_dead".to_string(), json!(monster.half_dead));
                object.insert("first_move".to_string(), json!(monster.darkling.first_move));
                object.insert("nip_damage".to_string(), json!(monster.darkling.nip_dmg));
            }
            _ => {}
        }
    }
    mechanic_state
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
                .map(|index| CombatAction::SubmitDiscoverChoice { index })
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

fn build_pending_choice_observation(
    engine_state: &EngineState,
    combat: &CombatState,
) -> Option<CombatObservationPendingChoice> {
    let EngineState::PendingChoice(choice) = engine_state else {
        return None;
    };
    match choice {
        PendingChoice::DiscoverySelect(cards) => Some(CombatObservationPendingChoice {
            kind: "discovery_select".to_string(),
            min_select: 1,
            max_select: 1,
            can_cancel: false,
            reason: None,
            source_pile: None,
            options: cards
                .iter()
                .enumerate()
                .map(
                    |(option_index, card_id)| CombatObservationPendingChoiceOption {
                        option_index,
                        label: get_card_definition(*card_id).name.to_string(),
                        card_id: Some(format!("{card_id:?}")),
                        card_uuid: None,
                        selection_uuids: Vec::new(),
                        source_pile: None,
                    },
                )
                .collect(),
        }),
        PendingChoice::CardRewardSelect {
            cards,
            destination,
            can_skip,
        } => Some(CombatObservationPendingChoice {
            kind: "card_reward_select".to_string(),
            min_select: 1,
            max_select: 1,
            can_cancel: *can_skip,
            reason: Some(format!("{destination:?}")),
            source_pile: None,
            options: cards
                .iter()
                .enumerate()
                .map(
                    |(option_index, card_id)| CombatObservationPendingChoiceOption {
                        option_index,
                        label: get_card_definition(*card_id).name.to_string(),
                        card_id: Some(format!("{card_id:?}")),
                        card_uuid: None,
                        selection_uuids: Vec::new(),
                        source_pile: None,
                    },
                )
                .collect(),
        }),
        PendingChoice::StanceChoice => Some(CombatObservationPendingChoice {
            kind: "stance_choice".to_string(),
            min_select: 1,
            max_select: 1,
            can_cancel: false,
            reason: None,
            source_pile: None,
            options: vec![
                CombatObservationPendingChoiceOption {
                    option_index: 0,
                    label: "Wrath".to_string(),
                    card_id: None,
                    card_uuid: None,
                    selection_uuids: Vec::new(),
                    source_pile: None,
                },
                CombatObservationPendingChoiceOption {
                    option_index: 1,
                    label: "Calm".to_string(),
                    card_id: None,
                    card_uuid: None,
                    selection_uuids: Vec::new(),
                    source_pile: None,
                },
            ],
        }),
        PendingChoice::HandSelect {
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => Some(CombatObservationPendingChoice {
            kind: "hand_select".to_string(),
            min_select: *min_cards,
            max_select: *max_cards,
            can_cancel: *can_cancel,
            reason: Some(format!("{reason:?}")),
            source_pile: Some("Hand".to_string()),
            options: candidate_uuids
                .iter()
                .enumerate()
                .map(|(option_index, uuid)| {
                    let label = find_card_by_uuid(combat, *uuid)
                        .map(format_card)
                        .unwrap_or_else(|| format!("card#{uuid}"));
                    let card_id =
                        find_card_by_uuid(combat, *uuid).map(|card| format!("{:?}", card.id));
                    CombatObservationPendingChoiceOption {
                        option_index,
                        label,
                        card_id,
                        card_uuid: Some(*uuid),
                        selection_uuids: vec![*uuid],
                        source_pile: Some("Hand".to_string()),
                    }
                })
                .collect(),
        }),
        PendingChoice::GridSelect {
            source_pile,
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => Some(CombatObservationPendingChoice {
            kind: "grid_select".to_string(),
            min_select: *min_cards,
            max_select: *max_cards,
            can_cancel: *can_cancel,
            reason: Some(format!("{reason:?}")),
            source_pile: Some(pile_type_name(*source_pile)),
            options: candidate_uuids
                .iter()
                .enumerate()
                .map(|(option_index, uuid)| {
                    let label = find_card_by_uuid(combat, *uuid)
                        .map(format_card)
                        .unwrap_or_else(|| format!("card#{uuid}"));
                    let card_id =
                        find_card_by_uuid(combat, *uuid).map(|card| format!("{:?}", card.id));
                    CombatObservationPendingChoiceOption {
                        option_index,
                        label,
                        card_id,
                        card_uuid: Some(*uuid),
                        selection_uuids: vec![*uuid],
                        source_pile: Some(pile_type_name(*source_pile)),
                    }
                })
                .collect(),
        }),
        PendingChoice::ScrySelect { cards, card_uuids } => Some(CombatObservationPendingChoice {
            kind: "scry_select".to_string(),
            min_select: 0,
            max_select: cards.len() as u8,
            can_cancel: true,
            reason: None,
            source_pile: Some("Draw".to_string()),
            options: cards
                .iter()
                .enumerate()
                .map(
                    |(option_index, card_id)| CombatObservationPendingChoiceOption {
                        option_index,
                        label: get_card_definition(*card_id).name.to_string(),
                        card_id: Some(format!("{card_id:?}")),
                        card_uuid: card_uuids.get(option_index).copied(),
                        selection_uuids: card_uuids
                            .get(option_index)
                            .copied()
                            .into_iter()
                            .collect(),
                        source_pile: Some("Draw".to_string()),
                    },
                )
                .collect(),
        }),
    }
}

fn update_turn_prefix_tracker(
    tracker: &mut CombatTurnPrefixTracker,
    action: &CombatAction,
    before: &CombatState,
    after: &CombatState,
) {
    if after.turn.turn_count != before.turn.turn_count {
        *tracker = CombatTurnPrefixTracker::default();
        return;
    }
    tracker.damage_dealt_this_turn += (living_monster_hp(before) - living_monster_hp(after)).max(0);
    tracker.damage_taken_this_turn +=
        (before.entities.player.current_hp - after.entities.player.current_hp).max(0);
    tracker.last_action_family = Some(action_family_name(action).to_string());
    tracker.last_card_id = action_card_id(action, before);
    match action {
        CombatAction::PlayCard { card_index, .. } => {
            if let Some(card) = before.zones.hand.get(*card_index) {
                match get_card_definition(card.id).card_type {
                    CardType::Skill => {
                        tracker.skills_played_this_turn =
                            tracker.skills_played_this_turn.saturating_add(1);
                    }
                    CardType::Power => {
                        tracker.powers_played_this_turn =
                            tracker.powers_played_this_turn.saturating_add(1);
                    }
                    _ => {}
                }
                tracker.energy_spent_this_turn += energy_spent_by_card(card, before);
            }
        }
        CombatAction::UsePotion { .. }
        | CombatAction::EndTurn
        | CombatAction::SubmitDiscoverChoice { .. }
        | CombatAction::SubmitCardChoice { .. }
        | CombatAction::SubmitHandSelect { .. }
        | CombatAction::SubmitGridSelect { .. }
        | CombatAction::Proceed
        | CombatAction::Cancel
        | CombatAction::Raw { .. } => {}
    }
}

fn living_monster_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .map(|monster| monster.current_hp.max(0))
        .sum()
}

fn action_family_name(action: &CombatAction) -> &'static str {
    match action {
        CombatAction::EndTurn => "end_turn",
        CombatAction::PlayCard { .. } => "play_card",
        CombatAction::UsePotion { .. } => "use_potion",
        CombatAction::SubmitDiscoverChoice { .. } => "choice_select",
        CombatAction::SubmitCardChoice { .. } => "choice_select",
        CombatAction::SubmitHandSelect { .. } => "choice_select",
        CombatAction::SubmitGridSelect { .. } => "choice_select",
        CombatAction::Proceed => "proceed",
        CombatAction::Cancel => "cancel",
        CombatAction::Raw { .. } => "raw",
    }
}

fn action_card_id(action: &CombatAction, combat: &CombatState) -> Option<String> {
    match action {
        CombatAction::PlayCard { card_index, .. } => combat
            .zones
            .hand
            .get(*card_index)
            .map(|card| format!("{:?}", card.id)),
        _ => None,
    }
}

fn energy_spent_by_card(card: &CombatCard, combat: &CombatState) -> i32 {
    if card.free_to_play_once {
        return 0;
    }
    let cost = card.get_cost();
    if cost == -1 {
        return i32::from(combat.turn.energy);
    }
    cost.max(0) as i32
}

fn effective_retain(card: &CombatCard) -> bool {
    card.retain_override
        .unwrap_or(matches!(card.id, crate::content::cards::CardId::Miracle))
}

fn find_card_by_uuid(combat: &CombatState, uuid: u32) -> Option<&CombatCard> {
    combat
        .zones
        .hand
        .iter()
        .chain(combat.zones.draw_pile.iter())
        .chain(combat.zones.discard_pile.iter())
        .chain(combat.zones.exhaust_pile.iter())
        .chain(combat.zones.limbo.iter())
        .find(|card| card.uuid == uuid)
}

fn card_type_name(card_type: CardType) -> String {
    match card_type {
        CardType::Attack => "attack",
        CardType::Skill => "skill",
        CardType::Power => "power",
        CardType::Status => "status",
        CardType::Curse => "curse",
    }
    .to_string()
}

fn card_target_mode_name(target: CardTarget) -> String {
    match target {
        CardTarget::Enemy => "single_enemy",
        CardTarget::AllEnemy => "all_enemy",
        CardTarget::SelfTarget => "self",
        CardTarget::None => "none",
    }
    .to_string()
}

fn potion_target_mode_name(target_required: bool) -> String {
    if target_required {
        "single_enemy".to_string()
    } else {
        "none".to_string()
    }
}

fn pile_type_name(pile: PileType) -> String {
    match pile {
        PileType::Draw => "Draw",
        PileType::Discard => "Discard",
        PileType::Exhaust => "Exhaust",
        PileType::Hand => "Hand",
        PileType::Limbo => "Limbo",
        PileType::MasterDeck => "MasterDeck",
    }
    .to_string()
}

fn format_card(card: &CombatCard) -> String {
    let mut label = get_card_definition(card.id).name.to_string();
    for _ in 0..card.upgrades {
        label.push('+');
    }
    label
}
