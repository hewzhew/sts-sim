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

pub const FULL_RUN_OBSERVATION_SCHEMA_VERSION: &str = "full_run_observation_v5_reward_structure";
pub const FULL_RUN_ACTION_SCHEMA_VERSION: &str =
    "full_run_action_candidate_set_v3_reward_structure";
const NO_PROGRESS_REPEAT_LIMIT: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RewardShapingProfile {
    Baseline,
    PlanDeficitV0,
}

impl RewardShapingProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::PlanDeficitV0 => "plan_deficit_v0",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "baseline" => Ok(Self::Baseline),
            "plan_deficit_v0" => Ok(Self::PlanDeficitV0),
            other => Err(format!(
                "unsupported reward shaping profile '{other}'; expected baseline or plan_deficit_v0"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunPolicyKind {
    RandomMasked,
    RuleBaselineV0,
    PlanQueryV0,
}

impl RunPolicyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RandomMasked => "random_masked",
            Self::RuleBaselineV0 => "rule_baseline_v0",
            Self::PlanQueryV0 => "plan_query_v0",
        }
    }
}

#[derive(Clone, Debug)]
pub struct RunBatchConfig {
    pub episodes: usize,
    pub base_seed: u64,
    pub ascension: u8,
    pub final_act: bool,
    pub player_class: &'static str,
    pub max_steps: usize,
    pub policy: RunPolicyKind,
    pub trace_dir: Option<PathBuf>,
    pub determinism_check: bool,
    pub reward_shaping_profile: RewardShapingProfile,
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
    pub reward_shaping_profile: String,
    pub episodes_completed: usize,
    pub crash_count: usize,
    pub illegal_action_count: usize,
    pub no_progress_loop_count: usize,
    pub deterministic_replay_pass_count: usize,
    pub contract_failure_count: usize,
    pub average_floor: f32,
    pub median_floor: f32,
    pub average_steps: f32,
    pub average_total_reward: f32,
    pub average_combat_wins: f32,
    pub average_legal_action_count: f32,
    pub max_legal_action_count: usize,
    pub steps_per_second: f32,
    pub episodes_per_hour: f32,
    pub result_counts: std::collections::BTreeMap<String, usize>,
    pub death_floor_counts: std::collections::BTreeMap<String, usize>,
    pub act_counts: std::collections::BTreeMap<String, usize>,
    pub decision_type_counts: std::collections::BTreeMap<String, usize>,
    pub contract_failures: Vec<RunContractFailure>,
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
    pub no_progress_loop: Option<RunNoProgressLoop>,
    pub crash: Option<String>,
    pub deterministic_replay_pass: Option<bool>,
    pub deterministic_replay_error: Option<String>,
    pub contract_failure: Option<RunContractFailure>,
    pub duration_ms: u128,
    pub total_reward: f32,
    pub combat_win_count: usize,
    pub decision_type_counts: std::collections::BTreeMap<String, usize>,
    pub average_legal_action_count: f32,
    pub max_legal_action_count: usize,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_size: usize,
    pub relic_count: usize,
    pub trace_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RunContractFailure {
    pub kind: String,
    pub episode_id: usize,
    pub seed: u64,
    pub policy: String,
    pub step: Option<usize>,
    pub action_key: Option<String>,
    pub decision_type: Option<String>,
    pub engine_state: Option<String>,
    pub floor: i32,
    pub act: u8,
    pub terminal_reason: String,
    pub details: String,
    pub trace_path: Option<String>,
    pub reproduce_command: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RunNoProgressLoop {
    pub start_step: usize,
    pub end_step: usize,
    pub repeat_count: usize,
    pub action_key: String,
    pub decision_type: String,
    pub engine_state: String,
    pub floor: i32,
    pub act: u8,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunEpisodeTraceFile {
    pub observation_schema_version: String,
    pub action_schema_version: String,
    pub config: RunTraceConfigV0,
    pub summary: RunEpisodeSummary,
    pub steps: Vec<RunStepTrace>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunTraceConfigV0 {
    pub seed: u64,
    pub ascension: u8,
    pub final_act: bool,
    pub player_class: String,
    pub max_steps: usize,
    pub policy: String,
    pub reward_shaping_profile: String,
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
    pub deck: RunDeckObservationV0,
    pub plan_profile: DeckPlanProfileV0,
    pub deck_cards: Vec<RunDeckCardObservationV0>,
    pub relics: Vec<RunRelicObservationV0>,
    pub potions: Vec<RunPotionSlotObservationV0>,
    pub map: Option<RunMapObservationV0>,
    pub next_nodes: Vec<RunMapNodeObservationV0>,
    pub act_boss: Option<String>,
    pub reward_source: Option<String>,
    pub combat: Option<RunCombatObservationV0>,
    pub screen: RunScreenObservationV0,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunDeckCardObservationV0 {
    pub deck_index: usize,
    pub uuid: u32,
    pub card: RunCardFeatureV0,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunRelicObservationV0 {
    pub relic_id: String,
    pub counter: i32,
    pub used_up: bool,
    pub amount: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunPotionSlotObservationV0 {
    pub slot_index: usize,
    pub potion_id: Option<String>,
    pub uuid: Option<u32>,
    pub can_use: bool,
    pub can_discard: bool,
    pub requires_target: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunMapObservationV0 {
    pub current_x: i32,
    pub current_y: i32,
    pub boss_node_available: bool,
    pub has_emerald_key: bool,
    pub nodes: Vec<RunMapNodeObservationV0>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunMapNodeObservationV0 {
    pub x: i32,
    pub y: i32,
    pub room_type: Option<String>,
    pub has_emerald_key: bool,
    pub reachable_now: bool,
    pub edges: Vec<RunMapEdgeObservationV0>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunMapEdgeObservationV0 {
    pub dst_x: i32,
    pub dst_y: i32,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct RunDeckObservationV0 {
    pub attack_count: usize,
    pub skill_count: usize,
    pub power_count: usize,
    pub status_count: usize,
    pub curse_count: usize,
    pub starter_basic_count: usize,
    pub damage_card_count: usize,
    pub block_card_count: usize,
    pub draw_card_count: usize,
    pub scaling_card_count: usize,
    pub exhaust_card_count: usize,
    pub average_cost_milli: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunCombatObservationV0 {
    pub player_hp: i32,
    pub player_block: i32,
    pub energy: i32,
    pub combat_phase: String,
    pub turn_count: u32,
    pub hand_count: usize,
    pub hand_cards: Vec<RunCombatHandCardObservationV0>,
    pub draw_count: usize,
    pub discard_count: usize,
    pub exhaust_count: usize,
    pub alive_monster_count: usize,
    pub dying_monster_count: usize,
    pub half_dead_monster_count: usize,
    pub zero_hp_monster_count: usize,
    pub pending_rebirth_monster_count: usize,
    pub total_monster_hp: i32,
    pub visible_incoming_damage: i32,
    pub pending_action_count: usize,
    pub queued_card_count: usize,
    pub limbo_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunCombatHandCardObservationV0 {
    pub hand_index: usize,
    pub card_instance_id: u32,
    pub card_id: String,
    pub upgraded: bool,
    pub upgrades: u8,
    pub cost_for_turn: i8,
    pub playable: bool,
    pub base_semantics: Vec<String>,
    pub transient_tags: Vec<String>,
    pub estimated_role_scores: RunHandCardRoleScoresV0,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunHandCardRoleScoresV0 {
    pub score_kind: String,
    pub role: String,
    pub keeper: i32,
    pub fuel: i32,
    pub exhaust: i32,
    pub retention: i32,
    pub copy: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunScreenObservationV0 {
    pub event_option_count: usize,
    pub reward_item_count: usize,
    pub reward_card_choice_count: usize,
    pub reward_phase: String,
    pub reward_items: Vec<RunRewardItemObservationV0>,
    pub reward_claimable_item_count: usize,
    pub reward_unclaimed_card_item_count: usize,
    pub reward_free_value_score: i32,
    pub shop_card_count: usize,
    pub shop_relic_count: usize,
    pub shop_potion_count: usize,
    pub boss_relic_choice_count: usize,
    pub selection_target_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunRewardItemObservationV0 {
    pub item_index: usize,
    pub item_type: String,
    pub amount: i32,
    pub card_choice_count: usize,
    pub relic_id: Option<String>,
    pub potion_id: Option<String>,
    pub claimable: bool,
    pub opens_card_choice: bool,
    pub free_value_score: i32,
    pub likely_waste: bool,
    pub capacity_blocked: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunActionCandidate {
    pub action_index: usize,
    pub action_id: u32,
    pub action_key: String,
    pub action: TraceClientInput,
    pub card: Option<RunCardFeatureV0>,
    pub plan_delta: CandidatePlanDeltaV0,
    pub reward_structure: RewardActionStructureV0,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct RewardActionStructureV0 {
    pub score_kind: String,
    pub screen_phase: String,
    pub is_reward_action: bool,
    pub is_proceed_with_unclaimed_rewards: bool,
    pub unclaimed_reward_count: usize,
    pub unclaimed_card_reward_count: usize,
    pub claim_reward_item_type: Option<String>,
    pub claim_opens_card_choice: bool,
    pub claim_free_value_score: i32,
    pub claim_likely_waste: bool,
    pub claim_capacity_blocked: bool,
    pub proceed_is_cleanup: bool,
    pub skip_card_choice: bool,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct DeckPlanProfileV0 {
    pub score_kind: String,
    pub frontload_supply: i32,
    pub block_supply: i32,
    pub draw_supply: i32,
    pub scaling_supply: i32,
    pub aoe_supply: i32,
    pub exhaust_supply: i32,
    pub kill_window_supply: i32,
    pub starter_basic_burden: i32,
    pub setup_cashout_risk: i32,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct CandidatePlanDeltaV0 {
    pub score_kind: String,
    pub frontload_delta: i32,
    pub block_delta: i32,
    pub draw_delta: i32,
    pub scaling_delta: i32,
    pub aoe_delta: i32,
    pub exhaust_delta: i32,
    pub kill_window_delta: i32,
    pub starter_basic_burden_delta: i32,
    pub setup_cashout_risk_delta: i32,
    pub deck_deficit_bonus: i32,
    pub bloat_penalty: i32,
    pub duplicate_penalty: i32,
    pub plan_adjusted_score: i32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RunCardFeatureV0 {
    pub card_id: String,
    pub card_id_hash: u32,
    pub card_type_id: u8,
    pub rarity_id: u8,
    pub cost: i8,
    pub upgrades: u8,
    pub base_damage: i32,
    pub base_block: i32,
    pub base_magic: i32,
    pub upgraded_damage: i32,
    pub upgraded_block: i32,
    pub upgraded_magic: i32,
    pub exhaust: bool,
    pub ethereal: bool,
    pub innate: bool,
    pub aoe: bool,
    pub multi_damage: bool,
    pub starter_basic: bool,
    pub draws_cards: bool,
    pub gains_energy: bool,
    pub applies_weak: bool,
    pub applies_vulnerable: bool,
    pub scaling_piece: bool,
    pub deck_copies: usize,
    pub rule_score: i32,
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
    RuleBaselineV0,
    PlanQueryV0,
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
    combat_win_count: usize,
}

#[derive(Clone, Debug)]
pub struct FullRunEnvConfig {
    pub seed: u64,
    pub ascension: u8,
    pub final_act: bool,
    pub player_class: &'static str,
    pub max_steps: usize,
    pub reward_shaping_profile: RewardShapingProfile,
}

#[derive(Clone, Debug, Serialize)]
pub struct FullRunEnvState {
    pub observation_schema_version: String,
    pub action_schema_version: String,
    pub action_mask_kind: String,
    pub observation: RunObservationV0,
    pub action_candidates: Vec<RunActionCandidate>,
    pub action_mask: Vec<bool>,
    pub legal_action_count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct FullRunEnvInfo {
    pub seed: u64,
    pub step: usize,
    pub terminal_reason: String,
    pub result: String,
    pub forced_engine_ticks: usize,
    pub combat_win_count: usize,
    pub crash: Option<String>,
    pub contract_failure: Option<RunContractFailure>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FullRunEnvStep {
    pub state: FullRunEnvState,
    pub reward: f32,
    pub done: bool,
    pub chosen_action_key: Option<String>,
    pub info: FullRunEnvInfo,
}

#[derive(Clone, Debug)]
pub struct FullRunTracePlanProbeConfig {
    pub trace_file: PathBuf,
    pub step_index: usize,
    pub ascension: Option<u8>,
    pub final_act: Option<bool>,
    pub player_class: Option<String>,
    pub max_steps: Option<usize>,
    pub probe_config: crate::bot::combat::CombatTurnPlanProbeConfig,
}

pub struct FullRunEnv {
    config: FullRunEnvConfig,
    ctx: EpisodeContext,
    steps: usize,
    done: bool,
    terminal_reason: String,
    crash: Option<String>,
    contract_failure: Option<RunContractFailure>,
    no_progress_tracker: NoProgressTracker,
}

impl FullRunEnvConfig {
    pub fn batch_config(&self, policy: RunPolicyKind) -> RunBatchConfig {
        RunBatchConfig {
            episodes: 1,
            base_seed: self.seed,
            ascension: self.ascension,
            final_act: self.final_act,
            player_class: self.player_class,
            max_steps: self.max_steps,
            policy,
            trace_dir: None,
            determinism_check: false,
            reward_shaping_profile: self.reward_shaping_profile,
        }
    }
}

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

pub fn probe_combat_plan_from_trace(
    config: &FullRunTracePlanProbeConfig,
) -> Result<crate::bot::combat::CombatTurnPlanProbeReport, String> {
    let raw = std::fs::read_to_string(&config.trace_file).map_err(|err| {
        format!(
            "failed to read trace file '{}': {err}",
            config.trace_file.display()
        )
    })?;
    let trace: serde_json::Value =
        serde_json::from_str(&raw).map_err(|err| format!("failed to parse trace JSON: {err}"))?;
    let summary = trace
        .get("summary")
        .ok_or_else(|| "trace missing summary".to_string())?;
    let seed = summary
        .get("seed")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "trace summary missing seed".to_string())?;
    let trace_config = trace.get("config");
    let ascension = config.ascension.unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("ascension"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u8
    });
    let final_act = config.final_act.unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("final_act"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    });
    let player_class = config.player_class.clone().unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("player_class"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Ironclad")
            .to_string()
    });
    let max_steps = config.max_steps.unwrap_or_else(|| {
        trace_config
            .and_then(|value| value.get("max_steps"))
            .and_then(serde_json::Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or_else(|| config.step_index.saturating_add(128).max(512))
    });
    let steps = trace
        .get("steps")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "trace missing steps[]".to_string())?;
    if config.step_index >= steps.len() {
        return Err(format!(
            "step-index {} out of range for trace with {} step(s)",
            config.step_index,
            steps.len()
        ));
    }

    let mut ctx = EpisodeContext {
        engine_state: EngineState::EventRoom,
        run_state: RunState::new(
            seed,
            ascension,
            final_act,
            normalize_player_class(&player_class),
        ),
        combat_state: None,
        stashed_event_combat: None,
        forced_engine_ticks: 0,
        combat_win_count: 0,
    };

    for (step_idx, step) in steps.iter().take(config.step_index).enumerate() {
        prepare_decision_point(&mut ctx, max_steps)?;
        let action = trace_step_action(step)
            .map_err(|err| format!("failed to decode action at trace step {step_idx}: {err}"))?;
        let keep_running = tick_run(
            &mut ctx.engine_state,
            &mut ctx.run_state,
            &mut ctx.combat_state,
            Some(action),
        );
        if let Some(errors) = take_engine_error_diagnostics(&mut ctx) {
            return Err(format!(
                "replay to step {} rejected trace action: {}",
                step_idx,
                errors.join("; ")
            ));
        }
        finish_combat_if_needed(&mut ctx);
        if !keep_running {
            return Err(format!(
                "engine stopped while replaying trace before requested step {}",
                config.step_index
            ));
        }
    }

    prepare_decision_point(&mut ctx, max_steps)?;
    let Some(combat) = ctx.combat_state.as_ref() else {
        return Err(format!(
            "trace step {} replayed to non-combat state {}",
            config.step_index,
            engine_state_label(&ctx.engine_state)
        ));
    };
    if !matches!(
        ctx.engine_state,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    ) {
        return Err(format!(
            "trace step {} is not a combat turn frontier: {}",
            config.step_index,
            engine_state_label(&ctx.engine_state)
        ));
    }

    let target_trace_step = &steps[config.step_index];
    let mut report =
        crate::bot::combat::probe_turn_plans(&ctx.engine_state, combat, config.probe_config);
    report.source_trace = serde_json::json!({
        "trace_file": config.trace_file.display().to_string(),
        "step_index": config.step_index,
        "seed": seed,
        "ascension": ascension,
        "final_act": final_act,
        "player_class": player_class,
        "trace_observation_schema_version": trace.get("observation_schema_version").cloned().unwrap_or(serde_json::Value::Null),
        "trace_action_schema_version": trace.get("action_schema_version").cloned().unwrap_or(serde_json::Value::Null),
        "trace_step_decision_type": target_trace_step.get("decision_type").cloned().unwrap_or(serde_json::Value::Null),
        "trace_step_engine_state": target_trace_step.get("engine_state").cloned().unwrap_or(serde_json::Value::Null),
        "trace_step_chosen_action_key": target_trace_step.get("chosen_action_key").cloned().unwrap_or(serde_json::Value::Null),
    });
    Ok(report)
}

fn trace_step_action(step: &serde_json::Value) -> Result<ClientInput, String> {
    let value = step
        .get("chosen_action")
        .ok_or_else(|| "missing chosen_action".to_string())?
        .clone();
    let trace_input: TraceClientInput = serde_json::from_value(value)
        .map_err(|err| format!("chosen_action shape mismatch: {err}"))?;
    Ok(client_input_from_trace_input(trace_input))
}

fn client_input_from_trace_input(input: TraceClientInput) -> ClientInput {
    match input {
        TraceClientInput::PlayCard { card_index, target } => {
            ClientInput::PlayCard { card_index, target }
        }
        TraceClientInput::UsePotion {
            potion_index,
            target,
        } => ClientInput::UsePotion {
            potion_index,
            target,
        },
        TraceClientInput::DiscardPotion { potion_index } => {
            ClientInput::DiscardPotion(potion_index)
        }
        TraceClientInput::EndTurn => ClientInput::EndTurn,
        TraceClientInput::SubmitCardChoice { indices } => ClientInput::SubmitCardChoice(indices),
        TraceClientInput::SubmitDiscoverChoice { index } => {
            ClientInput::SubmitDiscoverChoice(index)
        }
        TraceClientInput::SelectMapNode { x } => ClientInput::SelectMapNode(x),
        TraceClientInput::FlyToNode { x, y } => ClientInput::FlyToNode(x, y),
        TraceClientInput::SelectEventOption { index } => ClientInput::SelectEventOption(index),
        TraceClientInput::CampfireOption { choice } => {
            ClientInput::CampfireOption(campfire_choice_from_trace(choice))
        }
        TraceClientInput::EventChoice { index } => ClientInput::EventChoice(index),
        TraceClientInput::SubmitScryDiscard { indices } => ClientInput::SubmitScryDiscard(indices),
        TraceClientInput::SubmitSelection {
            scope,
            selected_card_uuids,
        } => ClientInput::SubmitSelection(SelectionResolution {
            scope: selection_scope_from_trace(scope),
            selected: selected_card_uuids
                .into_iter()
                .map(SelectionTargetRef::CardUuid)
                .collect(),
        }),
        TraceClientInput::SubmitHandSelect { card_uuids } => {
            ClientInput::SubmitHandSelect(card_uuids)
        }
        TraceClientInput::SubmitGridSelect { card_uuids } => {
            ClientInput::SubmitGridSelect(card_uuids)
        }
        TraceClientInput::SubmitDeckSelect { indices } => ClientInput::SubmitDeckSelect(indices),
        TraceClientInput::ClaimReward { index } => ClientInput::ClaimReward(index),
        TraceClientInput::SelectCard { index } => ClientInput::SelectCard(index),
        TraceClientInput::BuyCard { index } => ClientInput::BuyCard(index),
        TraceClientInput::BuyRelic { index } => ClientInput::BuyRelic(index),
        TraceClientInput::BuyPotion { index } => ClientInput::BuyPotion(index),
        TraceClientInput::PurgeCard { index } => ClientInput::PurgeCard(index),
        TraceClientInput::SubmitRelicChoice { index } => ClientInput::SubmitRelicChoice(index),
        TraceClientInput::Proceed => ClientInput::Proceed,
        TraceClientInput::Cancel => ClientInput::Cancel,
    }
}

fn campfire_choice_from_trace(choice: TraceCampfireChoice) -> CampfireChoice {
    match choice {
        TraceCampfireChoice::Rest => CampfireChoice::Rest,
        TraceCampfireChoice::Smith { deck_index } => CampfireChoice::Smith(deck_index),
        TraceCampfireChoice::Dig => CampfireChoice::Dig,
        TraceCampfireChoice::Lift => CampfireChoice::Lift,
        TraceCampfireChoice::Toke { deck_index } => CampfireChoice::Toke(deck_index),
        TraceCampfireChoice::Recall => CampfireChoice::Recall,
    }
}

fn selection_scope_from_trace(scope: TraceSelectionScope) -> SelectionScope {
    match scope {
        TraceSelectionScope::Hand => SelectionScope::Hand,
        TraceSelectionScope::Deck => SelectionScope::Deck,
        TraceSelectionScope::Grid => SelectionScope::Grid,
    }
}

fn normalize_player_class(player_class: &str) -> &'static str {
    match player_class.to_ascii_lowercase().as_str() {
        "ironclad" | "red" => "Ironclad",
        "silent" | "green" => "Silent",
        "defect" | "blue" => "Defect",
        "watcher" | "purple" => "Watcher",
        _ => "Ironclad",
    }
}

fn full_run_progress_score(ctx: &EpisodeContext) -> f32 {
    let active_hp = ctx
        .combat_state
        .as_ref()
        .map(|combat| combat.entities.player.current_hp)
        .unwrap_or(ctx.run_state.current_hp);
    let active_max_hp = ctx
        .combat_state
        .as_ref()
        .map(|combat| combat.entities.player.max_hp)
        .unwrap_or(ctx.run_state.max_hp);
    let hp_fraction = if active_max_hp > 0 {
        active_hp.max(0) as f32 / active_max_hp as f32
    } else {
        0.0
    };
    ctx.run_state.floor_num.max(0) as f32 + ctx.combat_win_count as f32 * 2.0 + hp_fraction
}

fn full_run_action_shaping_reward(
    ctx: &EpisodeContext,
    action: &ClientInput,
    profile: RewardShapingProfile,
) -> f32 {
    let EngineState::RewardScreen(reward_state) = &ctx.engine_state else {
        return 0.0;
    };
    if let Some(cards) = &reward_state.pending_card_choice {
        return match action {
            ClientInput::SelectCard(index) => cards.get(*index).map_or(0.0, |card| match profile {
                RewardShapingProfile::Baseline => {
                    let score = rule_card_offer_score(card.id, &ctx.run_state);
                    (score as f32 / 300.0).clamp(-0.20, 0.35)
                }
                RewardShapingProfile::PlanDeficitV0 => {
                    let delta = add_card_plan_delta(card.id, card.upgrades, &ctx.run_state);
                    (delta.plan_adjusted_score as f32 / 220.0).clamp(-0.25, 0.65)
                }
            }),
            ClientInput::Proceed => match profile {
                RewardShapingProfile::Baseline => {
                    let best_score = cards
                        .iter()
                        .map(|card| rule_card_offer_score(card.id, &ctx.run_state))
                        .max()
                        .unwrap_or(0);
                    if best_score >= 70 {
                        -0.18
                    } else if best_score <= 20 {
                        0.04
                    } else {
                        -0.05
                    }
                }
                RewardShapingProfile::PlanDeficitV0 => {
                    let best_score = cards
                        .iter()
                        .map(|card| {
                            add_card_plan_delta(card.id, card.upgrades, &ctx.run_state)
                                .plan_adjusted_score
                        })
                        .max()
                        .unwrap_or(0);
                    if best_score >= 110 {
                        -0.65
                    } else if best_score >= 70 {
                        -0.40
                    } else if best_score <= 20 {
                        0.05
                    } else {
                        -0.12
                    }
                }
            },
            _ => 0.0,
        };
    }

    match action {
        ClientInput::ClaimReward(index) => reward_state
            .items
            .get(*index)
            .map(|item| match profile {
                RewardShapingProfile::Baseline => {
                    reward_item_shaping_value(&ctx.run_state, item).min(0.35)
                }
                RewardShapingProfile::PlanDeficitV0 => {
                    plan_deficit_reward_item_shaping_value(&ctx.run_state, item).min(0.55)
                }
            })
            .unwrap_or(0.0),
        ClientInput::Proceed => {
            let skipped_value = reward_state
                .items
                .iter()
                .map(|item| match profile {
                    RewardShapingProfile::Baseline => {
                        reward_item_shaping_value(&ctx.run_state, item).max(0.0)
                    }
                    RewardShapingProfile::PlanDeficitV0 => {
                        plan_deficit_reward_item_shaping_value(&ctx.run_state, item).max(0.0)
                    }
                })
                .sum::<f32>();
            match profile {
                RewardShapingProfile::Baseline => -skipped_value.min(0.60),
                RewardShapingProfile::PlanDeficitV0 => -skipped_value.min(1.00),
            }
        }
        _ => 0.0,
    }
}

fn reward_item_shaping_value(run_state: &RunState, item: &RewardItem) -> f32 {
    match item {
        RewardItem::Gold { amount } | RewardItem::StolenGold { amount } => {
            (*amount as f32 / 240.0).clamp(0.05, 0.25)
        }
        RewardItem::Relic { .. } => 0.30,
        RewardItem::Potion { .. } => {
            if run_state
                .relics
                .iter()
                .any(|relic| relic.id == RelicId::Sozu)
            {
                -0.05
            } else if run_state.potions.iter().any(Option::is_none) {
                0.08
            } else {
                0.0
            }
        }
        RewardItem::Card { .. } => 0.12,
        RewardItem::EmeraldKey | RewardItem::SapphireKey => 0.04,
    }
}

fn plan_deficit_reward_item_shaping_value(run_state: &RunState, item: &RewardItem) -> f32 {
    match item {
        RewardItem::Card { .. } => 0.42,
        RewardItem::Relic { .. } => 0.48,
        RewardItem::Gold { amount } | RewardItem::StolenGold { amount } => {
            (*amount as f32 / 180.0).clamp(0.08, 0.35)
        }
        RewardItem::Potion { .. } => {
            if reward_item_likely_waste(run_state, item) {
                -0.04
            } else if run_state.potions.iter().any(Option::is_none) {
                0.10
            } else {
                0.0
            }
        }
        RewardItem::EmeraldKey | RewardItem::SapphireKey => 0.06,
    }
}

fn reward_item_type_label(item: &RewardItem) -> &'static str {
    match item {
        RewardItem::Gold { .. } => "gold",
        RewardItem::StolenGold { .. } => "stolen_gold",
        RewardItem::Card { .. } => "card_reward",
        RewardItem::Relic { .. } => "relic",
        RewardItem::Potion { .. } => "potion",
        RewardItem::EmeraldKey => "emerald_key",
        RewardItem::SapphireKey => "sapphire_key",
    }
}

fn reward_item_amount(item: &RewardItem) -> i32 {
    match item {
        RewardItem::Gold { amount } | RewardItem::StolenGold { amount } => *amount,
        _ => 0,
    }
}

fn reward_item_claimable(run_state: &RunState, item: &RewardItem) -> bool {
    match item {
        RewardItem::Potion { .. } => {
            run_state
                .relics
                .iter()
                .any(|relic| relic.id == RelicId::Sozu)
                || run_state.potions.iter().any(Option::is_none)
        }
        _ => true,
    }
}

fn reward_item_capacity_blocked(run_state: &RunState, item: &RewardItem) -> bool {
    matches!(item, RewardItem::Potion { .. })
        && !run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::Sozu)
        && !run_state.potions.iter().any(Option::is_none)
}

fn reward_item_likely_waste(run_state: &RunState, item: &RewardItem) -> bool {
    matches!(item, RewardItem::Potion { .. })
        && run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::Sozu)
}

fn reward_item_claim_score(run_state: &RunState, item: &RewardItem) -> i32 {
    match item {
        RewardItem::Gold { amount } | RewardItem::StolenGold { amount } => 60 + amount / 8,
        RewardItem::Relic { .. } => 120,
        RewardItem::Potion { .. } => {
            if reward_item_likely_waste(run_state, item) {
                -10
            } else if run_state.potions.iter().any(Option::is_none) {
                55
            } else {
                0
            }
        }
        RewardItem::Card { .. } => 70,
        RewardItem::EmeraldKey | RewardItem::SapphireKey => 25,
    }
}

fn full_run_result_label(ctx: &EpisodeContext, done: bool, crash: Option<&String>) -> String {
    match &ctx.engine_state {
        EngineState::GameOver(RunResult::Victory) => "victory",
        EngineState::GameOver(RunResult::Defeat) => "defeat",
        _ if crash.is_some() => "crash",
        _ if done => "truncated",
        _ => "ongoing",
    }
    .to_string()
}

#[allow(clippy::too_many_arguments)]
fn make_full_run_env_contract_failure(
    config: &FullRunEnvConfig,
    seed: u64,
    kind: &str,
    terminal_reason: &str,
    floor: i32,
    act: u8,
    step: Option<usize>,
    action_key: Option<String>,
    decision_type: Option<String>,
    engine_state: Option<String>,
    details: String,
) -> RunContractFailure {
    RunContractFailure {
        kind: kind.to_string(),
        episode_id: 0,
        seed,
        policy: "external_driver".to_string(),
        step,
        action_key,
        decision_type,
        engine_state,
        floor,
        act,
        terminal_reason: terminal_reason.to_string(),
        details,
        trace_path: None,
        reproduce_command: full_run_env_reproduce_command(config, seed),
    }
}

fn full_run_env_reproduce_command(config: &FullRunEnvConfig, seed: u64) -> String {
    let mut parts = vec![
        ".venv-rl\\Scripts\\python.exe".to_string(),
        "tools\\learning\\smoke_full_run_env.py".to_string(),
        "--episodes".to_string(),
        "1".to_string(),
        "--seed".to_string(),
        seed.to_string(),
        "--ascension".to_string(),
        config.ascension.to_string(),
        "--class".to_string(),
        cli_class_arg(config.player_class).to_string(),
        "--max-steps".to_string(),
        config.max_steps.to_string(),
        "--reward-shaping-profile".to_string(),
        config.reward_shaping_profile.as_str().to_string(),
    ];
    if config.final_act {
        parts.push("--final-act".to_string());
    }
    parts.join(" ")
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NoProgressSignature {
    observation_key: String,
    action_mask_key: String,
    chosen_action_key: String,
}

#[derive(Clone, Debug)]
struct NoProgressTracker {
    last: Option<NoProgressSignature>,
    repeat_count: usize,
    start_step: usize,
}

impl NoProgressTracker {
    fn new() -> Self {
        Self {
            last: None,
            repeat_count: 0,
            start_step: 0,
        }
    }

    fn observe(
        &mut self,
        step_index: usize,
        signature: NoProgressSignature,
        observation: &RunObservationV0,
    ) -> Option<RunNoProgressLoop> {
        if self.last.as_ref() == Some(&signature) {
            self.repeat_count += 1;
        } else {
            self.last = Some(signature.clone());
            self.repeat_count = 1;
            self.start_step = step_index;
        }

        if self.repeat_count >= NO_PROGRESS_REPEAT_LIMIT {
            Some(RunNoProgressLoop {
                start_step: self.start_step,
                end_step: step_index,
                repeat_count: self.repeat_count,
                action_key: signature.chosen_action_key,
                decision_type: observation.decision_type.clone(),
                engine_state: observation.engine_state.clone(),
                floor: observation.floor,
                act: observation.act,
            })
        } else {
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn make_contract_failure(
    config: &RunBatchConfig,
    episode_id: usize,
    seed: u64,
    kind: &str,
    terminal_reason: &str,
    floor: i32,
    act: u8,
    step: Option<usize>,
    action_key: Option<String>,
    decision_type: Option<String>,
    engine_state: Option<String>,
    details: String,
) -> RunContractFailure {
    RunContractFailure {
        kind: kind.to_string(),
        episode_id,
        seed,
        policy: config.policy.as_str().to_string(),
        step,
        action_key,
        decision_type,
        engine_state,
        floor,
        act,
        terminal_reason: terminal_reason.to_string(),
        details,
        trace_path: None,
        reproduce_command: reproduce_command(config, seed),
    }
}

fn reproduce_command(config: &RunBatchConfig, seed: u64) -> String {
    let mut parts = vec![
        "cargo".to_string(),
        "run".to_string(),
        "--release".to_string(),
        "--bin".to_string(),
        "sts_dev_tool".to_string(),
        "--".to_string(),
        "run-batch".to_string(),
        "--episodes".to_string(),
        "1".to_string(),
        "--seed".to_string(),
        seed.to_string(),
        "--policy".to_string(),
        config.policy.as_str().to_string(),
        "--ascension".to_string(),
        config.ascension.to_string(),
        "--class".to_string(),
        cli_class_arg(config.player_class).to_string(),
        "--max-steps".to_string(),
        config.max_steps.to_string(),
        "--reward-shaping-profile".to_string(),
        config.reward_shaping_profile.as_str().to_string(),
        "--determinism-check".to_string(),
        "--summary-out".to_string(),
        format!(
            "tools\\artifacts\\full_run_smoke\\repro_{}_seed_{}.json",
            config.policy.as_str(),
            seed
        ),
        "--trace-dir".to_string(),
        format!(
            "tools\\artifacts\\full_run_smoke\\repro_{}_seed_{}_trace",
            config.policy.as_str(),
            seed
        ),
    ];
    if config.final_act {
        parts.push("--final-act".to_string());
    }
    parts.join(" ")
}

fn cli_class_arg(player_class: &str) -> &'static str {
    match player_class {
        "Ironclad" => "ironclad",
        "Silent" => "silent",
        "Defect" => "defect",
        "Watcher" => "watcher",
        _ => "ironclad",
    }
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
    let mut no_progress_loop_count = 0usize;
    let mut deterministic_replay_pass_count = 0usize;

    for episode_id in 0..config.episodes {
        let seed = config.base_seed.wrapping_add(episode_id as u64);
        let policy_seed = seed ^ 0x9e37_79b9_7f4a_7c15;
        let episode_policy = match config.policy {
            RunPolicyKind::RandomMasked => EpisodePolicy::RandomMasked {
                rng: StsRng::new(policy_seed),
            },
            RunPolicyKind::RuleBaselineV0 => EpisodePolicy::RuleBaselineV0,
            RunPolicyKind::PlanQueryV0 => EpisodePolicy::PlanQueryV0,
        };
        let mut episode = run_episode(config, episode_id, seed, episode_policy, true);

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
            } else if episode.summary.contract_failure.is_none() {
                let details = episode
                    .summary
                    .deterministic_replay_error
                    .clone()
                    .unwrap_or_else(|| "deterministic replay mismatch".to_string());
                episode.summary.contract_failure = Some(make_contract_failure(
                    config,
                    episode_id,
                    seed,
                    "deterministic_replay_mismatch",
                    "deterministic_replay_mismatch",
                    episode.summary.floor,
                    episode.summary.act,
                    None,
                    None,
                    None,
                    None,
                    details,
                ));
            }
        }

        if let Some(trace_dir) = &config.trace_dir {
            let trace_path = trace_dir.join(format!("episode_{episode_id:04}_seed_{seed}.json"));
            episode.summary.trace_path = Some(trace_path.display().to_string());
            if let Some(failure) = &mut episode.summary.contract_failure {
                failure.trace_path = episode.summary.trace_path.clone();
            }
            write_trace_file(&trace_path, config, &episode.summary, &episode.trace)?;
        }

        if episode.summary.crash.is_some() {
            crash_count += 1;
        }
        if episode.summary.no_progress_loop.is_some() {
            no_progress_loop_count += 1;
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
    let average_total_reward = episodes
        .iter()
        .map(|episode| episode.total_reward)
        .sum::<f32>()
        / episodes.len().max(1) as f32;
    let average_combat_wins = episodes
        .iter()
        .map(|episode| episode.combat_win_count)
        .sum::<usize>() as f32
        / episodes.len().max(1) as f32;
    let legal_action_count_sum = episodes
        .iter()
        .map(|episode| episode.average_legal_action_count * episode.steps as f32)
        .sum::<f32>();
    let average_legal_action_count = legal_action_count_sum / total_steps.max(1) as f32;
    let max_legal_action_count = episodes
        .iter()
        .map(|episode| episode.max_legal_action_count)
        .max()
        .unwrap_or(0);
    let result_counts = count_by(episodes.iter().map(|episode| episode.result.clone()));
    let death_floor_counts = count_by(
        episodes
            .iter()
            .filter(|episode| episode.result == "defeat")
            .map(|episode| episode.floor.to_string()),
    );
    let act_counts = count_by(episodes.iter().map(|episode| episode.act.to_string()));
    let mut decision_type_counts = std::collections::BTreeMap::new();
    for episode in &episodes {
        for (decision_type, count) in &episode.decision_type_counts {
            *decision_type_counts
                .entry(decision_type.clone())
                .or_insert(0) += *count;
        }
    }
    let contract_failures = episodes
        .iter()
        .filter_map(|episode| episode.contract_failure.clone())
        .collect::<Vec<_>>();
    let contract_failure_count = contract_failures.len();

    Ok(RunBatchSummary {
        observation_schema_version: FULL_RUN_OBSERVATION_SCHEMA_VERSION.to_string(),
        action_schema_version: FULL_RUN_ACTION_SCHEMA_VERSION.to_string(),
        action_mask_kind: "per_decision_candidate_set".to_string(),
        policy: config.policy.as_str().to_string(),
        episodes_requested: config.episodes,
        base_seed: config.base_seed,
        ascension: config.ascension,
        final_act: config.final_act,
        player_class: config.player_class.to_string(),
        max_steps: config.max_steps,
        reward_shaping_profile: config.reward_shaping_profile.as_str().to_string(),
        episodes_completed,
        crash_count,
        illegal_action_count,
        no_progress_loop_count,
        deterministic_replay_pass_count,
        contract_failure_count,
        average_floor,
        median_floor,
        average_steps,
        average_total_reward,
        average_combat_wins,
        average_legal_action_count,
        max_legal_action_count,
        steps_per_second: total_steps as f32 / elapsed,
        episodes_per_hour: episodes.len() as f32 / elapsed * 3600.0,
        result_counts,
        death_floor_counts,
        act_counts,
        decision_type_counts,
        contract_failures,
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
            let contract_failure = make_contract_failure(
                config,
                episode_id,
                seed,
                "panic",
                "panic",
                0,
                1,
                None,
                None,
                None,
                None,
                crash.clone(),
            );
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
                    no_progress_loop: None,
                    crash: Some(crash),
                    deterministic_replay_pass: None,
                    deterministic_replay_error: None,
                    contract_failure: Some(contract_failure),
                    duration_ms: 0,
                    total_reward: -100.0,
                    combat_win_count: 0,
                    decision_type_counts: std::collections::BTreeMap::new(),
                    average_legal_action_count: 0.0,
                    max_legal_action_count: 0,
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
        combat_win_count: 0,
    };
    let mut trace = Vec::new();
    let mut actions = Vec::new();
    let mut illegal_actions = 0usize;
    let mut no_progress_loop = None;
    let mut crash = None;
    let mut contract_failure = None;
    let mut terminal_reason = "step_cap".to_string();
    let mut decision_type_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut legal_action_count_sum = 0usize;
    let mut max_legal_action_count = 0usize;
    let mut no_progress_tracker = NoProgressTracker::new();

    for step_index in 0..config.max_steps {
        if let Err(err) = prepare_decision_point(&mut ctx, config.max_steps) {
            contract_failure = Some(make_contract_failure(
                config,
                episode_id,
                seed,
                "engine_error",
                "engine_error",
                ctx.run_state.floor_num,
                ctx.run_state.act_num,
                Some(step_index),
                None,
                Some(decision_type(&ctx.engine_state).to_string()),
                Some(engine_state_label(&ctx.engine_state).to_string()),
                err.clone(),
            ));
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
            let details = format!(
                "no legal actions at {} on floor {}",
                engine_state_label(&ctx.engine_state),
                ctx.run_state.floor_num
            );
            contract_failure = Some(make_contract_failure(
                config,
                episode_id,
                seed,
                "no_legal_actions",
                "no_legal_actions",
                ctx.run_state.floor_num,
                ctx.run_state.act_num,
                Some(step_index),
                None,
                Some(decision_type(&ctx.engine_state).to_string()),
                Some(engine_state_label(&ctx.engine_state).to_string()),
                details.clone(),
            ));
            crash = Some(details);
            terminal_reason = "no_legal_actions".to_string();
            break;
        }
        let current_decision_type = decision_type(&ctx.engine_state).to_string();
        *decision_type_counts
            .entry(current_decision_type.clone())
            .or_insert(0) += 1;
        legal_action_count_sum += legal_actions.len();
        max_legal_action_count = max_legal_action_count.max(legal_actions.len());

        let (chosen_action_index, action) = match choose_action(&mut policy, &ctx, &legal_actions) {
            Ok(action) => action,
            Err(err) => {
                illegal_actions += 1;
                contract_failure = Some(make_contract_failure(
                    config,
                    episode_id,
                    seed,
                    "illegal_replay_action",
                    "illegal_replay_action",
                    ctx.run_state.floor_num,
                    ctx.run_state.act_num,
                    Some(step_index),
                    None,
                    Some(current_decision_type.clone()),
                    Some(engine_state_label(&ctx.engine_state).to_string()),
                    err.clone(),
                ));
                crash = Some(err);
                terminal_reason = "illegal_replay_action".to_string();
                break;
            }
        };

        let observation = build_observation(&ctx);
        let action_mask = build_action_candidates(&legal_actions, Some(&ctx));
        let chosen = action_mask
            .get(chosen_action_index)
            .expect("chosen action index should be in legal action mask");
        let chosen_action_id = chosen.action_id;
        let chosen_action_key = chosen.action_key.clone();
        let signature =
            no_progress_signature(&observation, &action_mask, chosen_action_key.clone());

        if capture_trace {
            trace.push(RunStepTrace {
                step_index,
                floor: ctx.run_state.floor_num,
                act: ctx.run_state.act_num,
                engine_state: engine_state_label(&ctx.engine_state).to_string(),
                decision_type: current_decision_type.clone(),
                hp: ctx.run_state.current_hp,
                max_hp: ctx.run_state.max_hp,
                gold: ctx.run_state.gold,
                deck_size: ctx.run_state.master_deck.len(),
                relic_count: ctx.run_state.relics.len(),
                legal_action_count: legal_actions.len(),
                observation: observation.clone(),
                action_mask: action_mask.clone(),
                chosen_action_index,
                chosen_action_id,
                chosen_action_key: chosen_action_key.clone(),
                chosen_action: trace_input_from_client_input(&action),
            });
        }
        if let Some(loop_info) = no_progress_tracker.observe(step_index, signature, &observation) {
            let details = format!(
                "no progress loop: action {} repeated {} times from step {} to {} at {} floor {}",
                loop_info.action_key,
                loop_info.repeat_count,
                loop_info.start_step,
                loop_info.end_step,
                loop_info.decision_type,
                loop_info.floor
            );
            contract_failure = Some(make_contract_failure(
                config,
                episode_id,
                seed,
                "no_progress_loop",
                "no_progress_loop",
                loop_info.floor,
                loop_info.act,
                Some(loop_info.end_step),
                Some(loop_info.action_key.clone()),
                Some(loop_info.decision_type.clone()),
                Some(loop_info.engine_state.clone()),
                details.clone(),
            ));
            crash = Some(details);
            terminal_reason = "no_progress_loop".to_string();
            no_progress_loop = Some(loop_info);
            break;
        }
        actions.push(action.clone());
        let executed_action_key = action_key_for_input(&action, ctx.combat_state.as_ref());

        let keep_running = tick_run(
            &mut ctx.engine_state,
            &mut ctx.run_state,
            &mut ctx.combat_state,
            Some(action),
        );
        if let Some(errors) = take_engine_error_diagnostics(&mut ctx) {
            illegal_actions += 1;
            let details = format!(
                "engine rejected legal action {executed_action_key}: {}",
                errors.join("; ")
            );
            contract_failure = Some(make_contract_failure(
                config,
                episode_id,
                seed,
                "engine_rejected_action",
                "engine_rejected_action",
                ctx.run_state.floor_num,
                ctx.run_state.act_num,
                Some(step_index),
                Some(executed_action_key),
                Some(current_decision_type),
                Some(observation.engine_state.clone()),
                details.clone(),
            ));
            crash = Some(details);
            terminal_reason = "engine_rejected_action".to_string();
            break;
        }
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
    let average_legal_action_count = legal_action_count_sum as f32 / actions.len().max(1) as f32;
    let total_reward = episode_reward(
        &result,
        ctx.run_state.floor_num,
        ctx.combat_win_count,
        ctx.run_state.current_hp,
        ctx.run_state.max_hp,
    );

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
            no_progress_loop,
            crash,
            deterministic_replay_pass: None,
            deterministic_replay_error: None,
            contract_failure,
            duration_ms: start.elapsed().as_millis(),
            total_reward,
            combat_win_count: ctx.combat_win_count,
            decision_type_counts,
            average_legal_action_count,
            max_legal_action_count,
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
        reconcile_terminal_combat_player_turn(ctx);
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
        reconcile_terminal_combat_player_turn(ctx);
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

fn reconcile_terminal_combat_player_turn(ctx: &mut EpisodeContext) {
    if !matches!(ctx.engine_state, EngineState::CombatPlayerTurn) {
        return;
    }
    let Some(combat) = ctx.combat_state.as_ref() else {
        return;
    };
    if combat_is_waiting_for_victory_settlement(combat) {
        ctx.engine_state = EngineState::CombatProcessing;
    }
}

fn combat_is_waiting_for_victory_settlement(combat: &CombatState) -> bool {
    !combat.entities.monsters.is_empty()
        && !combat.has_pending_actions()
        && combat.zones.queued_cards.is_empty()
        && combat
            .entities
            .monsters
            .iter()
            .all(|monster| monster_is_defeated_for_victory_settlement(combat, monster))
}

fn monster_is_defeated_for_victory_settlement(
    combat: &CombatState,
    monster: &crate::runtime::combat::MonsterEntity,
) -> bool {
    if monster.is_escaped {
        return true;
    }
    if monster.half_dead {
        return false;
    }
    if monster.current_hp > 0 && !monster.is_dying {
        return false;
    }
    !crate::content::powers::store::powers_for(combat, monster.id).is_some_and(|powers| {
        powers.iter().any(|power| {
            matches!(
                power.power_type,
                crate::content::powers::PowerId::Regrow
                    | crate::content::powers::PowerId::Unawakened
            )
        })
    })
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
    let survived_combat = !matches!(ctx.engine_state, EngineState::GameOver(_));
    ctx.combat_state = None;
    if survived_combat {
        ctx.combat_win_count += 1;
    }

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

fn take_engine_error_diagnostics(ctx: &mut EpisodeContext) -> Option<Vec<String>> {
    let combat = ctx.combat_state.as_mut()?;
    let diagnostics = combat.take_engine_diagnostics();
    let errors = diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == EngineDiagnosticSeverity::Error)
        .map(|diagnostic| diagnostic.message)
        .collect::<Vec<_>>();
    if errors.is_empty() {
        None
    } else {
        Some(errors)
    }
}

fn choose_action(
    policy: &mut EpisodePolicy,
    ctx: &EpisodeContext,
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
        EpisodePolicy::RuleBaselineV0 => {
            let idx = choose_rule_baseline_action(ctx, legal_actions);
            Ok((idx, legal_actions[idx].clone()))
        }
        EpisodePolicy::PlanQueryV0 => {
            let idx = choose_plan_query_action(ctx, legal_actions)
                .unwrap_or_else(|| choose_rule_baseline_action(ctx, legal_actions));
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

fn choose_rule_baseline_action(ctx: &EpisodeContext, legal_actions: &[ClientInput]) -> usize {
    let mut best_index = 0usize;
    let mut best_score = i32::MIN;
    for (index, action) in legal_actions.iter().enumerate() {
        let score = rule_baseline_score(ctx, action);
        if score > best_score {
            best_index = index;
            best_score = score;
        }
    }
    best_index
}

fn choose_plan_query_action(ctx: &EpisodeContext, legal_actions: &[ClientInput]) -> Option<usize> {
    let combat = ctx.combat_state.as_ref()?;
    if !matches!(
        ctx.engine_state,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    ) {
        return None;
    }
    let legal_by_key = legal_actions.iter().enumerate().fold(
        BTreeMap::<String, usize>::new(),
        |mut acc, (index, action)| {
            acc.entry(action_key_for_input(action, Some(combat)))
                .or_insert(index);
            acc
        },
    );
    if legal_by_key.is_empty() {
        return None;
    }

    let report = crate::bot::combat::probe_turn_plans(
        &ctx.engine_state,
        combat,
        crate::bot::combat::CombatTurnPlanProbeConfig {
            max_depth: 4,
            max_nodes: 500,
            beam_width: 16,
            max_engine_steps_per_action: 200,
        },
    );

    if let Some(index) = mapped_query_action(&report, &legal_by_key, "CanLethal", &["feasible"]) {
        return Some(index);
    }

    let incoming = visible_incoming_damage(combat);
    let unblocked = visible_unblocked_damage(combat);
    let hp = combat.entities.player.current_hp.max(1);
    let high_pressure = unblocked > 0 && (unblocked >= 8 || unblocked * 3 >= hp);
    let low_or_moderate_pressure = !high_pressure && (unblocked <= 6 || unblocked * 5 <= hp);
    let guarded_pressure = guarded_survival_pressure(combat, incoming, unblocked, hp);
    let multi_enemy_pressure = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.current_hp > 0 && !monster.is_dying)
        .count()
        >= 2;

    if resource_window_opened_this_turn(combat) {
        if guarded_pressure {
            if let Some(index) =
                guarded_survival_query_action(combat, legal_actions, &report, &legal_by_key)
            {
                return Some(index);
            }
        }
        if let Some(index) = resource_window_follow_through_action(&report, &legal_by_key, incoming)
        {
            return Some(index);
        }
    }

    if !high_pressure || unblocked * 5 <= hp {
        if let Some(index) = resource_window_opener_action(combat, legal_actions, unblocked, hp) {
            return Some(index);
        }
    }

    if guarded_pressure {
        if let Some(index) =
            guarded_survival_query_action(combat, legal_actions, &report, &legal_by_key)
        {
            return Some(index);
        }
    }

    if high_pressure {
        for (query, statuses) in [
            ("CanFullBlockThenMaxDamage", &["feasible"][..]),
            ("CanFullBlock", &["feasible"][..]),
            ("CanFullBlockThenMaxDamage", &["partial"][..]),
            ("CanFullBlock", &["partial"][..]),
        ] {
            if let Some(index) = mapped_query_action(&report, &legal_by_key, query, statuses) {
                return Some(index);
            }
        }
    }

    if guarded_pressure {
        return None;
    }

    if incoming > 0 && low_or_moderate_pressure {
        if let Some(index) = mapped_plan_action(&report, &legal_by_key, "KillThreateningEnemy") {
            return Some(index);
        }
        if multi_enemy_pressure {
            if let Some(index) = mapped_plan_action(&report, &legal_by_key, "MaxDamage") {
                return Some(index);
            }
        }
    }

    if incoming == 0 || unblocked == 0 {
        if let Some(index) = mapped_query_action(
            &report,
            &legal_by_key,
            "CanPlaySetupAndStillBlock",
            &["feasible"],
        ) {
            return Some(index);
        }
    }

    if incoming > 0 {
        if let Some(index) = mapped_query_action(
            &report,
            &legal_by_key,
            "CanFullBlockThenMaxDamage",
            &["feasible"],
        ) {
            return Some(index);
        }
    }

    for plan_name in ["MaxDamage", "SetupPowerOrScaling"] {
        if let Some(index) = mapped_plan_action(&report, &legal_by_key, plan_name) {
            return Some(index);
        }
    }

    None
}

fn guarded_survival_pressure(combat: &CombatState, incoming: i32, unblocked: i32, hp: i32) -> bool {
    if incoming <= 0 || unblocked <= 0 {
        return false;
    }
    let total_hp = total_alive_monster_hp(combat);
    let boss_or_long_race =
        combat.meta.is_boss_fight || (alive_monster_count(combat) == 1 && total_hp >= 120);
    let severe_attack_window = incoming >= 24 || unblocked * 2 >= hp;
    boss_or_long_race && (unblocked >= 8 || severe_attack_window)
}

fn guarded_survival_query_action(
    combat: &CombatState,
    legal_actions: &[ClientInput],
    report: &crate::bot::combat::CombatTurnPlanProbeReport,
    legal_by_key: &BTreeMap<String, usize>,
) -> Option<usize> {
    for (query, statuses) in [
        ("CanFullBlockThenMaxDamage", &["feasible"][..]),
        ("CanFullBlock", &["feasible"][..]),
    ] {
        if let Some(index) = mapped_query_action(report, legal_by_key, query, statuses) {
            return Some(index);
        }
    }
    if let Some(index) = guarded_direct_block_action(combat, legal_actions) {
        return Some(index);
    }
    for (query, statuses) in [
        ("CanFullBlock", &["partial"][..]),
        ("CanFullBlockThenMaxDamage", &["partial"][..]),
    ] {
        if let Some(index) = mapped_query_action(report, legal_by_key, query, statuses) {
            return Some(index);
        }
    }
    None
}

fn guarded_direct_block_action(
    combat: &CombatState,
    legal_actions: &[ClientInput],
) -> Option<usize> {
    legal_actions
        .iter()
        .enumerate()
        .filter_map(|(index, action)| {
            let ClientInput::PlayCard { card_index, target } = action else {
                return None;
            };
            if target.is_some() {
                return None;
            }
            let card = combat.zones.hand.get(*card_index)?;
            let def = crate::content::cards::get_card_definition(card.id);
            let evaluated = crate::content::cards::evaluate_card_for_play(card, combat, *target);
            let block = evaluated
                .base_block_mut
                .max(def.base_block + card.upgrades as i32 * def.upgrade_block);
            if block <= 0 && !card_is_block_core(card.id) {
                return None;
            }
            let cost = evaluated.get_cost().max(0) as i32;
            let block_score = block.max(8);
            let utility_bonus = match card.id {
                CardId::ShrugItOff | CardId::TrueGrit | CardId::FlameBarrier => 30,
                CardId::Impervious | CardId::PowerThrough => 20,
                _ => 0,
            };
            Some((index, block_score * 100 - cost * 10 + utility_bonus))
        })
        .max_by_key(|(_, score)| *score)
        .map(|(index, _)| index)
}

fn resource_window_opened_this_turn(combat: &CombatState) -> bool {
    combat
        .turn
        .counters
        .card_ids_played_this_turn
        .iter()
        .any(|card_id| is_resource_window_card(*card_id))
}

fn is_resource_window_card(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Offering
            | CardId::Adrenaline
            | CardId::BattleTrance
            | CardId::SeeingRed
            | CardId::Bloodletting
    )
}

fn resource_window_follow_through_action(
    report: &crate::bot::combat::CombatTurnPlanProbeReport,
    legal_by_key: &BTreeMap<String, usize>,
    incoming: i32,
) -> Option<usize> {
    if let Some(plan) = report
        .plans
        .iter()
        .find(|plan| plan.plan_name == "MaxDamage")
    {
        if let Some(score) = plan.best_score.as_ref() {
            if score.enemy_death_score > 0 || score.damage_score >= 180 {
                if let Some(index) = first_mapped_action(&plan.best_action_keys, legal_by_key) {
                    return Some(index);
                }
            }
        }
    }

    let mut best: Option<(usize, i32)> = None;
    for plan_name in [
        "KillThreateningEnemy",
        "MaxDamage",
        "BlockEnoughThenDamage",
        "SetupPowerOrScaling",
    ] {
        let Some(plan) = report.plans.iter().find(|plan| plan.plan_name == plan_name) else {
            continue;
        };
        let Some(index) = first_mapped_action(&plan.best_action_keys, legal_by_key) else {
            continue;
        };
        let Some(score) = plan.best_score.as_ref() else {
            continue;
        };
        let mut adjusted = score.total_score;
        match plan_name {
            "KillThreateningEnemy" => {
                adjusted += score.enemy_death_score * 2 + score.damage_score;
            }
            "MaxDamage" => {
                adjusted += score.damage_score * 2 + score.enemy_death_score;
            }
            "BlockEnoughThenDamage" => {
                if incoming <= 0 {
                    adjusted -= 80;
                }
                adjusted += score.block_score + score.damage_score;
            }
            "SetupPowerOrScaling" => {
                adjusted += score.setup_score * 2 + score.damage_score / 2;
            }
            _ => {}
        }
        if best
            .as_ref()
            .is_none_or(|(_, best_score)| adjusted > *best_score)
        {
            best = Some((index, adjusted));
        }
    }
    best.map(|(index, _)| index)
}

fn resource_window_opener_action(
    combat: &CombatState,
    legal_actions: &[ClientInput],
    unblocked: i32,
    hp: i32,
) -> Option<usize> {
    legal_actions
        .iter()
        .enumerate()
        .filter_map(|(index, action)| {
            let ClientInput::PlayCard { card_index, target } = action else {
                return None;
            };
            if target.is_some() {
                return None;
            }
            let score = resource_window_opener_score(combat, *card_index, unblocked, hp)?;
            Some((index, score))
        })
        .max_by_key(|(_, score)| *score)
        .map(|(index, _)| index)
}

fn resource_window_opener_score(
    combat: &CombatState,
    card_index: usize,
    unblocked: i32,
    hp: i32,
) -> Option<i32> {
    let card = combat.zones.hand.get(card_index)?;
    let (base, hp_cost, extra_energy, draw_count) = match card.id {
        CardId::Adrenaline => (240, 0, 1, 2),
        CardId::Offering => (230, 6, 2, 3),
        CardId::BattleTrance => (115, 0, 0, 3),
        CardId::SeeingRed => (90, 0, 2, 0),
        CardId::Bloodletting => (80, 3, 2 + card.upgrades as i32, 0),
        _ => return None,
    };
    if resource_window_opened_this_turn(combat) {
        return None;
    }
    if hp - hp_cost <= unblocked + 6 {
        return None;
    }
    let evaluated = crate::content::cards::evaluate_card_for_play(card, combat, None);
    let cost = evaluated.get_cost().max(0) as i32;
    let energy_after = combat.turn.energy as i32 - cost + extra_energy;
    if energy_after <= 0 && draw_count > 0 {
        return None;
    }

    let immediate_payoff = resource_window_immediate_payoff_score(combat, card_index, energy_after);
    let draw_payoff = if draw_count > 0 {
        resource_window_draw_payoff_score(combat, draw_count)
    } else {
        0
    };
    let payoff = immediate_payoff + draw_payoff;
    if payoff < 45 {
        return None;
    }

    Some(base + payoff - hp_cost * 14 - cost * 8)
}

fn resource_window_immediate_payoff_score(
    combat: &CombatState,
    resource_card_index: usize,
    energy_after: i32,
) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != resource_card_index)
        .map(|(_, card)| {
            let evaluated = crate::content::cards::evaluate_card_for_play(card, combat, None);
            let cost = evaluated.get_cost().max(0) as i32;
            resource_window_card_payoff_score(card.id, cost <= energy_after)
        })
        .sum()
}

fn resource_window_draw_payoff_score(combat: &CombatState, draw_count: usize) -> i32 {
    if combat.zones.draw_pile.is_empty() && combat.zones.discard_pile.is_empty() {
        return 0;
    }
    let mut scores = combat
        .zones
        .draw_pile
        .iter()
        .chain(combat.zones.discard_pile.iter())
        .map(|card| resource_window_card_payoff_score(card.id, true))
        .filter(|score| *score > 0)
        .collect::<Vec<_>>();
    scores.sort_unstable_by(|a, b| b.cmp(a));
    scores.into_iter().take(draw_count.max(1) * 2).sum::<i32>() / 2
}

fn resource_window_card_payoff_score(card_id: CardId, currently_playable: bool) -> i32 {
    let def = crate::content::cards::get_card_definition(card_id);
    let playable_multiplier = if currently_playable { 2 } else { 1 };
    let base = match card_id {
        CardId::Immolate | CardId::Bludgeon | CardId::FiendFire | CardId::Reaper => 55,
        CardId::Bash | CardId::Uppercut | CardId::Shockwave | CardId::Disarm => 42,
        CardId::Inflame | CardId::DemonForm | CardId::FeelNoPain | CardId::DarkEmbrace => 38,
        CardId::Cleave | CardId::Whirlwind | CardId::ThunderClap | CardId::Carnage => 34,
        CardId::PommelStrike | CardId::ShrugItOff | CardId::BattleTrance => 28,
        _ => match def.card_type {
            CardType::Attack if def.base_damage > 0 => 22,
            CardType::Skill if def.base_block > 0 => 18,
            CardType::Power => 24,
            _ => 0,
        },
    };
    base * playable_multiplier
}

fn mapped_query_action(
    report: &crate::bot::combat::CombatTurnPlanProbeReport,
    legal_by_key: &BTreeMap<String, usize>,
    query_name: &str,
    allowed_statuses: &[&str],
) -> Option<usize> {
    let query = report
        .plan_queries
        .iter()
        .find(|query| query.query_name == query_name)?;
    if !allowed_statuses
        .iter()
        .any(|status| query.status.as_str() == *status)
    {
        return None;
    }
    first_mapped_action(&query.best_action_keys, legal_by_key)
}

fn mapped_plan_action(
    report: &crate::bot::combat::CombatTurnPlanProbeReport,
    legal_by_key: &BTreeMap<String, usize>,
    plan_name: &str,
) -> Option<usize> {
    let plan = report
        .plans
        .iter()
        .find(|plan| plan.plan_name == plan_name)?;
    first_mapped_action(&plan.best_action_keys, legal_by_key)
}

fn first_mapped_action(
    action_keys: &[String],
    legal_by_key: &BTreeMap<String, usize>,
) -> Option<usize> {
    action_keys
        .iter()
        .find_map(|action_key| legal_by_key.get(action_key).copied())
}

fn rule_baseline_score(ctx: &EpisodeContext, action: &ClientInput) -> i32 {
    match &ctx.engine_state {
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_) => {
            score_combat_action(ctx, action)
        }
        EngineState::RewardScreen(reward_state) => {
            score_reward_action(&ctx.run_state, reward_state, action)
        }
        EngineState::MapNavigation => score_map_action(&ctx.run_state, action),
        EngineState::EventRoom => score_event_action(action),
        EngineState::BossRelicSelect(state) => score_boss_relic_action(state, action),
        EngineState::Campfire => score_campfire_action(&ctx.run_state, action),
        EngineState::Shop(shop) => score_shop_action(&ctx.run_state, shop, action),
        EngineState::RunPendingChoice(choice) => {
            let request = choice.selection_request(&ctx.run_state);
            score_run_selection_action(&ctx.run_state, &request, action)
        }
        EngineState::CombatProcessing | EngineState::EventCombat(_) | EngineState::GameOver(_) => 0,
    }
}

fn score_combat_action(ctx: &EpisodeContext, action: &ClientInput) -> i32 {
    let Some(combat) = ctx.combat_state.as_ref() else {
        return score_noncombat_fallback(action);
    };
    match (&ctx.engine_state, action) {
        (
            EngineState::PendingChoice(PendingChoice::DiscoverySelect(cards)),
            ClientInput::SubmitDiscoverChoice(index),
        )
        | (
            EngineState::PendingChoice(PendingChoice::CardRewardSelect { cards, .. }),
            ClientInput::SubmitDiscoverChoice(index),
        ) => cards
            .get(*index)
            .map(|card_id| 100 + rule_card_offer_score(*card_id, &ctx.run_state))
            .unwrap_or(-1_000),
        (
            EngineState::PendingChoice(PendingChoice::CardRewardSelect { can_skip: true, .. }),
            ClientInput::Cancel,
        ) => 10,
        (
            EngineState::PendingChoice(PendingChoice::ScrySelect { .. }),
            ClientInput::SubmitScryDiscard(indices),
        ) => 10 + indices.len() as i32 * 8,
        (
            EngineState::PendingChoice(PendingChoice::StanceChoice),
            ClientInput::SubmitDiscoverChoice(index),
        ) => {
            let unblocked = visible_unblocked_damage(combat);
            match *index {
                1 if unblocked > 0 => 100,
                0 if unblocked == 0 => 80,
                _ => 20,
            }
        }
        (_, ClientInput::PlayCard { card_index, target }) => {
            score_play_card_action(combat, *card_index, *target)
        }
        (_, ClientInput::UsePotion { .. }) => -1_000,
        (_, ClientInput::DiscardPotion { .. }) => -50,
        (_, ClientInput::EndTurn) => {
            let playable_cards = combat
                .zones
                .hand
                .iter()
                .filter(|card| crate::content::cards::can_play_card(card, combat).is_ok())
                .count();
            if playable_cards == 0 {
                20
            } else {
                -200 - visible_unblocked_damage(combat) * 4
            }
        }
        _ => score_noncombat_fallback(action),
    }
}

fn score_play_card_action(combat: &CombatState, card_index: usize, target: Option<usize>) -> i32 {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return -1_000;
    };
    let def = crate::content::cards::get_card_definition(card.id);
    let evaluated = crate::content::cards::evaluate_card_for_play(card, combat, target);
    let incoming = visible_incoming_damage(combat);
    let unblocked = (incoming - combat.entities.player.block).max(0);
    let hp = combat.entities.player.current_hp.max(1);
    let danger = unblocked >= hp / 3 || unblocked >= 12;
    let mut score = 20 - evaluated.get_cost().max(0) as i32 * 12;

    let damage = estimated_card_damage(combat, &evaluated, target);
    let block = evaluated
        .base_block_mut
        .max(def.base_block + card.upgrades as i32 * def.upgrade_block);
    if damage > 0 {
        score += damage * if danger { 8 } else { 11 };
        if estimated_action_kills_all(combat, &evaluated, target) {
            score += 900;
        } else if target
            .and_then(|target_id| alive_monster_by_id(combat, target_id))
            .is_some_and(|monster| damage >= monster.current_hp + monster.block)
        {
            score += 180;
        }
    }
    if block > 0 {
        let useful_block = block.min(unblocked.max(0));
        score += useful_block * if danger { 18 } else { 6 };
        score += (block - useful_block).max(0) * 2;
    }

    let specific_bonus = match card.id {
        CardId::Bash | CardId::Uppercut | CardId::Shockwave => 45,
        CardId::Disarm => 70,
        CardId::Inflame | CardId::DemonForm | CardId::FeelNoPain | CardId::DarkEmbrace => 55,
        CardId::ShrugItOff | CardId::PommelStrike | CardId::BattleTrance => 35,
        CardId::Offering | CardId::Adrenaline => 80,
        CardId::Immolate | CardId::Feed | CardId::Reaper => 65,
        CardId::Flex | CardId::Bloodletting | CardId::SeeingRed | CardId::SpotWeakness => 35,
        CardId::Defend if danger => 25,
        CardId::Strike if !danger => 8,
        _ => 0,
    };

    match def.card_type {
        crate::content::cards::CardType::Power => {
            score += if danger && unblocked >= hp { -20 } else { 8 };
        }
        crate::content::cards::CardType::Skill => {
            score += 12;
        }
        crate::content::cards::CardType::Attack => {
            score += if incoming == 0 { 20 } else { 8 };
        }
        crate::content::cards::CardType::Status | crate::content::cards::CardType::Curse => {
            score -= 80;
        }
    }

    score += specific_bonus;
    if damage == 0 && block == 0 && specific_bonus <= 0 {
        score -= 350;
    }

    score
}

fn estimated_card_damage(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
    target: Option<usize>,
) -> i32 {
    let def = crate::content::cards::get_card_definition(card.id);
    if def.is_multi_damage || matches!(def.target, crate::content::cards::CardTarget::AllEnemy) {
        if !card.multi_damage.is_empty() {
            return card
                .multi_damage
                .iter()
                .take(alive_monster_count(combat))
                .copied()
                .sum();
        }
        return card.base_damage_mut.max(0) * alive_monster_count(combat) as i32;
    }

    let damage = card.base_damage_mut.max(0);
    if let Some(target_id) = target {
        if let Some(monster) = alive_monster_by_id(combat, target_id) {
            return damage.min(monster.current_hp + monster.block);
        }
    }
    damage
}

fn estimated_action_kills_all(
    combat: &CombatState,
    card: &crate::runtime::combat::CombatCard,
    target: Option<usize>,
) -> bool {
    let alive = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .collect::<Vec<_>>();
    if alive.is_empty() {
        return false;
    }
    let def = crate::content::cards::get_card_definition(card.id);
    if def.is_multi_damage || matches!(def.target, crate::content::cards::CardTarget::AllEnemy) {
        if !card.multi_damage.is_empty() {
            return alive.iter().enumerate().all(|(idx, monster)| {
                card.multi_damage.get(idx).copied().unwrap_or(0)
                    >= monster.current_hp + monster.block
            });
        }
        return alive
            .iter()
            .all(|monster| card.base_damage_mut >= monster.current_hp + monster.block);
    }
    if alive.len() == 1 {
        return target
            .and_then(|target_id| alive_monster_by_id(combat, target_id))
            .is_some_and(|monster| card.base_damage_mut >= monster.current_hp + monster.block);
    }
    false
}

fn alive_monster_by_id(
    combat: &CombatState,
    target_id: usize,
) -> Option<&crate::runtime::combat::MonsterEntity> {
    combat.entities.monsters.iter().find(|monster| {
        monster.id == target_id && !monster.is_dying && !monster.is_escaped && !monster.half_dead
    })
}

fn alive_monster_count(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .count()
}

fn total_alive_monster_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .map(|monster| monster.current_hp.max(0) + monster.block.max(0))
        .sum()
}

fn visible_unblocked_damage(combat: &CombatState) -> i32 {
    (visible_incoming_damage(combat) - combat.entities.player.block).max(0)
}

fn visible_incoming_damage(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .map(|monster| {
            crate::projection::combat::monster_preview_total_damage_in_combat(combat, monster)
        })
        .sum()
}

fn score_reward_action(
    run_state: &RunState,
    reward_state: &RewardState,
    action: &ClientInput,
) -> i32 {
    if let Some(cards) = &reward_state.pending_card_choice {
        return match action {
            ClientInput::SelectCard(index) => cards
                .get(*index)
                .map(|card| rule_card_offer_score(card.id, run_state))
                .unwrap_or(-1_000),
            ClientInput::Proceed => 5,
            _ => -100,
        };
    }

    match action {
        ClientInput::ClaimReward(index) => reward_state
            .items
            .get(*index)
            .map(|item| match item {
                RewardItem::Potion { .. } if reward_item_likely_waste(run_state, item) => -10,
                _ => reward_item_claim_score(run_state, item),
            })
            .unwrap_or(-1_000),
        ClientInput::Proceed => 0,
        _ => -100,
    }
}

fn score_map_action(run_state: &RunState, action: &ClientInput) -> i32 {
    let ClientInput::SelectMapNode(x) = action else {
        return score_noncombat_fallback(action);
    };
    let target_y = if run_state.map.current_y == -1 {
        0
    } else if run_state.map.current_y == 14 {
        15
    } else {
        run_state.map.current_y + 1
    };
    if target_y == 15 {
        return 200;
    }
    let room_type = run_state
        .map
        .graph
        .get(target_y as usize)
        .and_then(|row| row.get(*x))
        .and_then(|node| node.class);
    let hp_ratio = run_state.current_hp * 100 / run_state.max_hp.max(1);
    match room_type {
        Some(RoomType::MonsterRoomElite) if hp_ratio >= 70 => 70,
        Some(RoomType::MonsterRoomElite) => -20,
        Some(RoomType::RestRoom) if hp_ratio < 70 => 90,
        Some(RoomType::RestRoom) => 45,
        Some(RoomType::TreasureRoom) => 80,
        Some(RoomType::ShopRoom) if run_state.gold >= 150 => 75,
        Some(RoomType::ShopRoom) => 25,
        Some(RoomType::EventRoom) => 55,
        Some(RoomType::MonsterRoom) => 50,
        Some(RoomType::MonsterRoomBoss) => 200,
        Some(RoomType::TrueVictoryRoom) => 300,
        None => 0,
    }
}

fn score_event_action(action: &ClientInput) -> i32 {
    match action {
        ClientInput::EventChoice(index) => 30 - *index as i32,
        ClientInput::Proceed => 5,
        _ => score_noncombat_fallback(action),
    }
}

fn score_boss_relic_action(
    state: &crate::rewards::state::BossRelicChoiceState,
    action: &ClientInput,
) -> i32 {
    match action {
        ClientInput::SubmitRelicChoice(index) => state
            .relics
            .get(*index)
            .map(|relic| 80 + rule_relic_score(*relic))
            .unwrap_or(-1_000),
        ClientInput::Proceed => -40,
        _ => score_noncombat_fallback(action),
    }
}

fn score_campfire_action(run_state: &RunState, action: &ClientInput) -> i32 {
    match action {
        ClientInput::CampfireOption(CampfireChoice::Rest) => {
            let hp_ratio = run_state.current_hp * 100 / run_state.max_hp.max(1);
            if hp_ratio < 45 {
                160
            } else if hp_ratio < 70 {
                90
            } else {
                10
            }
        }
        ClientInput::CampfireOption(CampfireChoice::Smith(index)) => run_state
            .master_deck
            .get(*index)
            .map(|card| rule_upgrade_score(card.id))
            .unwrap_or(-1_000),
        ClientInput::CampfireOption(CampfireChoice::Toke(index)) => run_state
            .master_deck
            .get(*index)
            .map(|card| 60 + rule_remove_score(card.id, run_state))
            .unwrap_or(-1_000),
        ClientInput::CampfireOption(CampfireChoice::Dig) => 75,
        ClientInput::CampfireOption(CampfireChoice::Lift) => 55,
        ClientInput::CampfireOption(CampfireChoice::Recall) => -20,
        _ => score_noncombat_fallback(action),
    }
}

fn score_shop_action(
    run_state: &RunState,
    shop: &crate::shop::ShopState,
    action: &ClientInput,
) -> i32 {
    match action {
        ClientInput::PurgeCard(index) => run_state
            .master_deck
            .get(*index)
            .map(|card| 100 + rule_remove_score(card.id, run_state))
            .unwrap_or(-1_000),
        ClientInput::BuyCard(index) => shop
            .cards
            .get(*index)
            .map(|card| rule_card_offer_score(card.card_id, run_state) - card.price / 5)
            .unwrap_or(-1_000),
        ClientInput::BuyRelic(index) => shop
            .relics
            .get(*index)
            .map(|relic| 70 + rule_relic_score(relic.relic_id) - relic.price / 8)
            .unwrap_or(-1_000),
        ClientInput::BuyPotion(index) => shop
            .potions
            .get(*index)
            .map(|potion| {
                if run_state
                    .relics
                    .iter()
                    .any(|relic| relic.id == RelicId::Sozu)
                {
                    -80 - potion.price / 4
                } else {
                    35 - potion.price / 8
                }
            })
            .unwrap_or(-1_000),
        ClientInput::Proceed => 0,
        _ => score_noncombat_fallback(action),
    }
}

fn score_run_selection_action(
    run_state: &RunState,
    request: &crate::state::selection::SelectionRequest,
    action: &ClientInput,
) -> i32 {
    match action {
        ClientInput::SubmitSelection(selection) => {
            let mut score = 20 + selection.selected.len() as i32 * 5;
            for selected in &selection.selected {
                let SelectionTargetRef::CardUuid(uuid) = selected;
                if let Some(card) = run_state.master_deck.iter().find(|card| card.uuid == *uuid) {
                    score += rule_remove_score(card.id, run_state).max(0) / 2;
                }
            }
            score
        }
        ClientInput::Cancel if request.can_cancel => 5,
        _ => score_noncombat_fallback(action),
    }
}

fn score_noncombat_fallback(action: &ClientInput) -> i32 {
    match action {
        ClientInput::Proceed => 0,
        ClientInput::Cancel => -5,
        _ => 10,
    }
}

fn rule_card_offer_score(card_id: CardId, run_state: &RunState) -> i32 {
    let def = crate::content::cards::get_card_definition(card_id);
    if matches!(
        def.card_type,
        crate::content::cards::CardType::Curse | crate::content::cards::CardType::Status
    ) {
        return -120;
    }

    let mut score = match def.rarity {
        crate::content::cards::CardRarity::Basic => -60,
        crate::content::cards::CardRarity::Common => 25,
        crate::content::cards::CardRarity::Uncommon => 42,
        crate::content::cards::CardRarity::Rare => 58,
        crate::content::cards::CardRarity::Special => 20,
        crate::content::cards::CardRarity::Curse => -120,
    };
    score += match def.card_type {
        crate::content::cards::CardType::Attack => {
            if run_state.master_deck.len() <= 14 {
                20
            } else {
                5
            }
        }
        crate::content::cards::CardType::Skill => 18,
        crate::content::cards::CardType::Power => 28,
        crate::content::cards::CardType::Status | crate::content::cards::CardType::Curse => -100,
    };
    score += def.base_damage.max(0) + def.base_block.max(0);
    score += match card_id {
        CardId::ShrugItOff | CardId::PommelStrike | CardId::BattleTrance => 45,
        CardId::Disarm | CardId::Shockwave | CardId::Offering | CardId::Adrenaline => 65,
        CardId::Immolate | CardId::Feed | CardId::Reaper | CardId::Bludgeon => 55,
        CardId::Inflame | CardId::FeelNoPain | CardId::DarkEmbrace | CardId::DemonForm => 40,
        CardId::Bash | CardId::Defend | CardId::Strike => -80,
        CardId::PerfectedStrike | CardId::Clash => -45,
        CardId::TwinStrike | CardId::SwordBoomerang => -20,
        _ => 0,
    };
    let copies = run_state
        .master_deck
        .iter()
        .filter(|card| card.id == card_id)
        .count() as i32;
    score -= copies * 12;
    if run_state.master_deck.len() >= 22 && def.card_type == crate::content::cards::CardType::Attack
    {
        score -= 20;
    }
    score
}

fn rule_remove_score(card_id: CardId, run_state: &RunState) -> i32 {
    let def = crate::content::cards::get_card_definition(card_id);
    if def.card_type == crate::content::cards::CardType::Curse {
        return 180;
    }
    match card_id {
        CardId::Strike => 115,
        CardId::Defend => {
            let defend_count = run_state
                .master_deck
                .iter()
                .filter(|card| card.id == CardId::Defend)
                .count();
            if defend_count > 4 {
                75
            } else {
                35
            }
        }
        _ if crate::content::cards::is_starter_basic(card_id) => 70,
        _ if def.card_type == crate::content::cards::CardType::Status => 90,
        _ => -40,
    }
}

fn rule_upgrade_score(card_id: CardId) -> i32 {
    match card_id {
        CardId::Bash => 95,
        CardId::Inflame | CardId::ShrugItOff | CardId::PommelStrike | CardId::BattleTrance => 85,
        CardId::Immolate | CardId::Feed | CardId::Offering | CardId::Adrenaline => 82,
        CardId::Uppercut | CardId::Shockwave | CardId::Disarm => 78,
        CardId::Defend => 50,
        CardId::Strike => 20,
        _ => {
            let def = crate::content::cards::get_card_definition(card_id);
            35 + def.upgrade_damage.max(0) * 3
                + def.upgrade_block.max(0) * 3
                + def.upgrade_magic.max(0) * 4
        }
    }
}

fn rule_relic_score(relic_id: RelicId) -> i32 {
    match relic_id {
        RelicId::BurningBlood => 30,
        RelicId::QuestionCard | RelicId::SingingBowl | RelicId::MoltenEgg | RelicId::ToxicEgg => 45,
        RelicId::BagOfPreparation | RelicId::Anchor | RelicId::Lantern => 55,
        RelicId::CoffeeDripper | RelicId::RunicDome | RelicId::BustedCrown => -25,
        _ => 20,
    }
}

fn episode_reward(
    result: &str,
    floor: i32,
    combat_win_count: usize,
    current_hp: i32,
    max_hp: i32,
) -> f32 {
    let terminal = match result {
        "victory" => 100.0,
        "defeat" => -10.0,
        "crash" => -100.0,
        _ => -2.0,
    };
    let hp_fraction = if max_hp > 0 {
        current_hp.max(0) as f32 / max_hp as f32
    } else {
        0.0
    };
    floor.max(0) as f32 + combat_win_count as f32 * 2.0 + hp_fraction + terminal
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
        if reward_item_claimable(run_state, item) {
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
        deck: build_deck_observation(&ctx.run_state),
        plan_profile: build_deck_plan_profile(&ctx.run_state),
        deck_cards: build_deck_card_observations(&ctx.run_state),
        relics: build_relic_observations(&ctx.run_state),
        potions: build_potion_observations(&ctx.run_state),
        map: build_map_observation_if_relevant(&ctx.engine_state, &ctx.run_state),
        next_nodes: build_next_node_observations(&ctx.run_state),
        act_boss: ctx
            .run_state
            .boss_list
            .first()
            .map(|boss| format!("{boss:?}")),
        reward_source: reward_source_label(&ctx.engine_state, &ctx.run_state),
        combat: combat.map(build_combat_observation),
        screen: build_screen_observation(&ctx.engine_state, &ctx.run_state),
    }
}

fn build_deck_card_observations(run_state: &RunState) -> Vec<RunDeckCardObservationV0> {
    run_state
        .master_deck
        .iter()
        .enumerate()
        .map(|(deck_index, card)| RunDeckCardObservationV0 {
            deck_index,
            uuid: card.uuid,
            card: build_card_feature(card.id, card.upgrades, run_state),
        })
        .collect()
}

fn build_deck_observation(run_state: &RunState) -> RunDeckObservationV0 {
    let mut out = RunDeckObservationV0::default();
    let mut cost_sum = 0i32;
    let mut cost_count = 0i32;
    for card in &run_state.master_deck {
        let def = crate::content::cards::get_card_definition(card.id);
        match def.card_type {
            CardType::Attack => out.attack_count += 1,
            CardType::Skill => out.skill_count += 1,
            CardType::Power => out.power_count += 1,
            CardType::Status => out.status_count += 1,
            CardType::Curse => out.curse_count += 1,
        }
        if crate::content::cards::is_starter_basic(card.id) {
            out.starter_basic_count += 1;
        }
        if def.base_damage > 0 {
            out.damage_card_count += 1;
        }
        if def.base_block > 0 || card_is_block_core(card.id) {
            out.block_card_count += 1;
        }
        if card_draws_cards(card.id) {
            out.draw_card_count += 1;
        }
        if card_is_scaling_piece(card.id) {
            out.scaling_card_count += 1;
        }
        if def.exhaust || card_exhausts_other_cards(card.id) {
            out.exhaust_card_count += 1;
        }
        if def.cost >= 0 {
            cost_sum += def.cost as i32;
            cost_count += 1;
        }
    }
    out.average_cost_milli = if cost_count > 0 {
        cost_sum * 1000 / cost_count
    } else {
        0
    };
    out
}

#[derive(Clone, Copy, Debug, Default)]
struct CardPlanAffordance {
    frontload: i32,
    block: i32,
    draw: i32,
    scaling: i32,
    aoe: i32,
    exhaust: i32,
    kill_window: i32,
    setup_cashout_risk: i32,
}

impl CardPlanAffordance {
    fn subtract(self, other: Self) -> Self {
        Self {
            frontload: self.frontload - other.frontload,
            block: self.block - other.block,
            draw: self.draw - other.draw,
            scaling: self.scaling - other.scaling,
            aoe: self.aoe - other.aoe,
            exhaust: self.exhaust - other.exhaust,
            kill_window: self.kill_window - other.kill_window,
            setup_cashout_risk: self.setup_cashout_risk - other.setup_cashout_risk,
        }
    }
}

fn build_deck_plan_profile(run_state: &RunState) -> DeckPlanProfileV0 {
    let mut profile = DeckPlanProfileV0 {
        score_kind: "heuristic".to_string(),
        ..DeckPlanProfileV0::default()
    };
    for card in &run_state.master_deck {
        let affordance = card_plan_affordance(card.id, card.upgrades);
        profile.frontload_supply += affordance.frontload;
        profile.block_supply += affordance.block;
        profile.draw_supply += affordance.draw;
        profile.scaling_supply += affordance.scaling;
        profile.aoe_supply += affordance.aoe;
        profile.exhaust_supply += affordance.exhaust;
        profile.kill_window_supply += affordance.kill_window;
        if crate::content::cards::is_starter_basic(card.id) {
            profile.starter_basic_burden += 10;
        }
    }
    profile.setup_cashout_risk = setup_cashout_risk_from_supplies(
        profile.frontload_supply,
        profile.block_supply,
        profile.draw_supply,
        profile.scaling_supply,
    );
    profile
}

fn card_plan_affordance(card_id: CardId, upgrades: u8) -> CardPlanAffordance {
    let def = crate::content::cards::get_card_definition(card_id);
    let damage = (def.base_damage + def.upgrade_damage * upgrades as i32).max(0);
    let block = (def.base_block + def.upgrade_block * upgrades as i32).max(0);
    let magic = (def.base_magic + def.upgrade_magic * upgrades as i32).max(0);
    let mut out = CardPlanAffordance::default();
    if damage > 0 {
        out.frontload += damage;
    }
    if block > 0 {
        out.block += block;
    } else if card_is_block_core(card_id) {
        out.block += 8;
    }
    if card_draws_cards(card_id) {
        out.draw += match card_id {
            CardId::Offering | CardId::BattleTrance | CardId::MasterOfStrategy => 18,
            CardId::ShrugItOff | CardId::PommelStrike | CardId::Backflip => 12,
            _ => 10,
        };
    }
    if card_is_scaling_piece(card_id) {
        out.scaling += match card_id {
            CardId::DemonForm | CardId::Corruption => 22,
            CardId::Inflame | CardId::FeelNoPain | CardId::DarkEmbrace => 16,
            _ => 12,
        };
        out.setup_cashout_risk += 4;
    }
    if matches!(def.target, crate::content::cards::CardTarget::AllEnemy) || def.is_multi_damage {
        out.aoe += 12 + damage / 2;
    }
    if card_is_multi_hit(card_id) {
        out.aoe += 4;
    }
    if card_exhausts_other_cards(card_id) {
        out.exhaust += match card_id {
            CardId::TrueGrit if upgrades == 0 => 5,
            CardId::TrueGrit => 14,
            CardId::SecondWind | CardId::FiendFire | CardId::BurningPact => 12,
            _ => 8,
        };
    }
    if matches!(
        card_id,
        CardId::Feed | CardId::HandOfGreed | CardId::RitualDagger
    ) {
        out.kill_window += 18;
    }
    if card_applies_vulnerable(card_id) {
        out.frontload += 8 + magic;
    }
    if card_applies_weak(card_id) {
        out.block += 6 + magic;
    }
    match card_id {
        CardId::Immolate => {
            out.frontload += 20;
            out.aoe += 20;
        }
        CardId::Disarm | CardId::Shockwave => {
            out.block += 18;
            out.scaling += 6;
        }
        CardId::Offering => {
            out.frontload += 8;
            out.draw += 6;
        }
        _ => {}
    }
    out
}

fn setup_cashout_risk_from_supplies(
    frontload_supply: i32,
    block_supply: i32,
    draw_supply: i32,
    scaling_supply: i32,
) -> i32 {
    if scaling_supply <= 0 {
        return 0;
    }
    (scaling_supply * 2 - block_supply - draw_supply - frontload_supply / 3).max(0)
}

fn build_relic_observations(run_state: &RunState) -> Vec<RunRelicObservationV0> {
    run_state
        .relics
        .iter()
        .map(|relic| RunRelicObservationV0 {
            relic_id: format!("{:?}", relic.id),
            counter: relic.counter,
            used_up: relic.used_up,
            amount: relic.amount,
        })
        .collect()
}

fn build_potion_observations(run_state: &RunState) -> Vec<RunPotionSlotObservationV0> {
    run_state
        .potions
        .iter()
        .enumerate()
        .map(|(slot_index, slot)| match slot {
            Some(potion) => RunPotionSlotObservationV0 {
                slot_index,
                potion_id: Some(format!("{:?}", potion.id)),
                uuid: Some(potion.uuid),
                can_use: potion.can_use,
                can_discard: potion.can_discard,
                requires_target: potion.requires_target,
            },
            None => RunPotionSlotObservationV0 {
                slot_index,
                potion_id: None,
                uuid: None,
                can_use: false,
                can_discard: false,
                requires_target: false,
            },
        })
        .collect()
}

fn build_map_observation(run_state: &RunState) -> RunMapObservationV0 {
    let nodes = run_state
        .map
        .graph
        .iter()
        .flat_map(|row| row.iter())
        .filter(|node| {
            node.class.is_some()
                || !node.edges.is_empty()
                || !node.parents.is_empty()
                || node.has_emerald_key
        })
        .map(|node| map_node_observation(run_state, node.x, node.y))
        .collect();
    RunMapObservationV0 {
        current_x: run_state.map.current_x,
        current_y: run_state.map.current_y,
        boss_node_available: run_state.map.boss_node_available,
        has_emerald_key: run_state.map.has_emerald_key,
        nodes,
    }
}

fn build_map_observation_if_relevant(
    engine_state: &EngineState,
    run_state: &RunState,
) -> Option<RunMapObservationV0> {
    match engine_state {
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::EventCombat(_)
        | EngineState::PendingChoice(PendingChoice::GridSelect { .. })
        | EngineState::PendingChoice(PendingChoice::HandSelect { .. })
        | EngineState::PendingChoice(PendingChoice::DiscoverySelect(_))
        | EngineState::PendingChoice(PendingChoice::ScrySelect { .. })
        | EngineState::PendingChoice(PendingChoice::CardRewardSelect { .. })
        | EngineState::PendingChoice(PendingChoice::StanceChoice)
        | EngineState::GameOver(_) => None,
        EngineState::RewardScreen(_)
        | EngineState::Campfire
        | EngineState::Shop(_)
        | EngineState::MapNavigation
        | EngineState::EventRoom
        | EngineState::RunPendingChoice(_)
        | EngineState::BossRelicSelect(_) => Some(build_map_observation(run_state)),
    }
}

fn build_next_node_observations(run_state: &RunState) -> Vec<RunMapNodeObservationV0> {
    legal_map_actions(run_state)
        .into_iter()
        .filter_map(|action| match action {
            ClientInput::SelectMapNode(x) => {
                let y = if run_state.map.current_y == -1 {
                    0
                } else if run_state.map.current_y == 14 {
                    15
                } else {
                    run_state.map.current_y + 1
                };
                Some(map_node_observation(run_state, x as i32, y))
            }
            ClientInput::FlyToNode(x, y) => {
                Some(map_node_observation(run_state, x as i32, y as i32))
            }
            _ => None,
        })
        .collect()
}

fn map_node_observation(run_state: &RunState, x: i32, y: i32) -> RunMapNodeObservationV0 {
    if y == 15 {
        return RunMapNodeObservationV0 {
            x,
            y,
            room_type: Some("MonsterRoomBoss".to_string()),
            has_emerald_key: false,
            reachable_now: run_state.map.can_travel_to(x, y, false),
            edges: Vec::new(),
        };
    }
    let node = run_state
        .map
        .graph
        .get(y.max(0) as usize)
        .and_then(|row| row.get(x.max(0) as usize));
    let edges = node
        .map(|node| {
            node.edges
                .iter()
                .map(|edge| RunMapEdgeObservationV0 {
                    dst_x: edge.dst_x,
                    dst_y: edge.dst_y,
                })
                .collect()
        })
        .unwrap_or_default();
    RunMapNodeObservationV0 {
        x,
        y,
        room_type: node.and_then(|node| node.class).map(room_type_name),
        has_emerald_key: node.is_some_and(|node| node.has_emerald_key),
        reachable_now: run_state.map.can_travel_to(x, y, false),
        edges,
    }
}

fn room_type_name(room_type: RoomType) -> String {
    format!("{room_type:?}")
}

fn reward_source_label(engine_state: &EngineState, run_state: &RunState) -> Option<String> {
    match engine_state {
        EngineState::RewardScreen(reward_state) => {
            if run_state.pending_boss_reward {
                Some("boss_combat_reward".to_string())
            } else {
                Some(format!(
                    "{:?}:{:?}",
                    reward_state.screen_context,
                    run_state.map.get_current_room_type()
                ))
            }
        }
        EngineState::PendingChoice(PendingChoice::CardRewardSelect { .. }) => {
            Some("combat_card_reward_select".to_string())
        }
        _ => None,
    }
}

fn build_combat_observation(combat: &CombatState) -> RunCombatObservationV0 {
    let alive_monsters = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .collect::<Vec<_>>();
    let dying_monster_count = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_dying)
        .count();
    let half_dead_monster_count = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.half_dead)
        .count();
    let zero_hp_monster_count = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.current_hp <= 0)
        .count();
    let pending_rebirth_monster_count = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            crate::content::powers::store::powers_for(combat, monster.id).is_some_and(|powers| {
                powers.iter().any(|power| {
                    matches!(
                        power.power_type,
                        crate::content::powers::PowerId::Regrow
                            | crate::content::powers::PowerId::Unawakened
                    )
                })
            })
        })
        .count();
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
        combat_phase: combat_phase_label(combat).to_string(),
        turn_count: combat.turn.turn_count,
        hand_count: combat.zones.hand.len(),
        hand_cards: build_combat_hand_card_observations(combat),
        draw_count: combat.zones.draw_pile.len(),
        discard_count: combat.zones.discard_pile.len(),
        exhaust_count: combat.zones.exhaust_pile.len(),
        alive_monster_count: alive_monsters.len(),
        dying_monster_count,
        half_dead_monster_count,
        zero_hp_monster_count,
        pending_rebirth_monster_count,
        total_monster_hp: alive_monsters
            .iter()
            .map(|monster| monster.current_hp.max(0))
            .sum(),
        visible_incoming_damage,
        pending_action_count: combat.action_queue_len(),
        queued_card_count: combat.zones.queued_cards.len(),
        limbo_count: combat.zones.limbo.len(),
    }
}

fn build_combat_hand_card_observations(
    combat: &CombatState,
) -> Vec<RunCombatHandCardObservationV0> {
    let context = build_card_role_context(combat);
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .map(|(hand_index, card)| {
            let playable = crate::content::cards::can_play_card(card, combat).is_ok();
            let role = classify_hand_card_with_context(combat, hand_index, &context);
            let mut transient_tags = Vec::new();
            transient_tags.push(if playable { "playable" } else { "unplayable" }.to_string());
            if card.cost_for_turn.is_some() {
                transient_tags.push("cost_for_turn_override".to_string());
            }
            if card.free_to_play_once {
                transient_tags.push("free_to_play_once".to_string());
            }
            transient_tags.push(format!("role:{}", hand_card_role_label(role)));

            RunCombatHandCardObservationV0 {
                hand_index,
                card_instance_id: card.uuid,
                card_id: format!("{:?}", card.id),
                upgraded: card.upgrades > 0,
                upgrades: card.upgrades,
                cost_for_turn: card.get_cost(),
                playable,
                base_semantics: base_semantics_for_card(card.id, card.upgrades),
                transient_tags,
                estimated_role_scores: RunHandCardRoleScoresV0 {
                    score_kind: "heuristic_not_truth".to_string(),
                    role: hand_card_role_label(role).to_string(),
                    keeper: combat_retention_score_for_uuid(combat, card.uuid),
                    fuel: combat_fuel_score_for_uuid(combat, card.uuid),
                    exhaust: combat_exhaust_score_for_uuid(combat, card.uuid),
                    retention: combat_retention_score_for_uuid(combat, card.uuid),
                    copy: combat_copy_score_for_uuid(combat, card.uuid),
                },
            }
        })
        .collect()
}

fn hand_card_role_label(role: HandCardRole) -> &'static str {
    match role {
        HandCardRole::CoreKeeper => "core_keeper",
        HandCardRole::SequencedPiece => "sequenced_piece",
        HandCardRole::SituationalResource => "situational_resource",
        HandCardRole::LowValueFuel => "low_value_fuel",
    }
}

fn base_semantics_for_card(card_id: CardId, upgrades: u8) -> Vec<String> {
    let def = crate::content::cards::get_card_definition(card_id);
    let mut tags = Vec::new();
    match def.card_type {
        CardType::Attack => tags.push("attack".to_string()),
        CardType::Skill => tags.push("skill".to_string()),
        CardType::Power => tags.push("power".to_string()),
        CardType::Status => tags.push("status".to_string()),
        CardType::Curse => tags.push("curse".to_string()),
    }
    if def.base_damage + def.upgrade_damage * upgrades as i32 > 0 {
        tags.push("damage".to_string());
    }
    if def.base_block + def.upgrade_block * upgrades as i32 > 0 || card_is_block_core(card_id) {
        tags.push("block".to_string());
    }
    if def.exhaust {
        tags.push("self_exhaust".to_string());
    }
    if card_draws_cards(card_id) {
        tags.push("draw".to_string());
    }
    if card_gains_energy(card_id) {
        tags.push("energy".to_string());
    }
    if card_applies_weak(card_id) {
        tags.push("apply_weak".to_string());
    }
    if card_applies_vulnerable(card_id) {
        tags.push("apply_vulnerable".to_string());
    }
    if card_is_scaling_piece(card_id) {
        tags.push("setup_or_scaling".to_string());
    }
    if card_exhausts_other_cards(card_id) {
        tags.push("exhaust_outlet".to_string());
    }
    match card_id {
        CardId::TrueGrit if upgrades == 0 => {
            tags.push("random_exhaust".to_string());
            tags.push("risk_overlay_required".to_string());
        }
        CardId::TrueGrit => tags.push("chosen_exhaust".to_string()),
        CardId::SecondWind => {
            tags.push("exhaust_non_attacks".to_string());
            tags.push("block_from_hand_destruction".to_string());
        }
        CardId::FiendFire => {
            tags.push("exhaust_hand_for_damage".to_string());
            tags.push("hand_destruction_risk".to_string());
        }
        _ => {}
    }
    if def.target == crate::content::cards::CardTarget::AllEnemy || def.is_multi_damage {
        tags.push("multi_target_or_multi_damage".to_string());
    }
    tags
}

fn combat_phase_label(combat: &CombatState) -> &'static str {
    match combat.turn.current_phase {
        crate::runtime::combat::CombatPhase::PlayerTurn => "player_turn",
        crate::runtime::combat::CombatPhase::TurnTransition => "turn_transition",
        crate::runtime::combat::CombatPhase::MonsterTurn => "monster_turn",
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
        EngineState::RewardScreen(reward_state) => {
            build_reward_screen_observation(run_state, reward_state)
        }
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
        reward_phase: "none".to_string(),
        reward_items: Vec::new(),
        reward_claimable_item_count: 0,
        reward_unclaimed_card_item_count: 0,
        reward_free_value_score: 0,
        shop_card_count: 0,
        shop_relic_count: 0,
        shop_potion_count: 0,
        boss_relic_choice_count: 0,
        selection_target_count: 0,
    }
}

fn build_reward_screen_observation(
    run_state: &RunState,
    reward_state: &RewardState,
) -> RunScreenObservationV0 {
    let reward_items = reward_state
        .items
        .iter()
        .enumerate()
        .map(|(item_index, item)| reward_item_observation(run_state, item_index, item))
        .collect::<Vec<_>>();
    let reward_claimable_item_count = reward_items.iter().filter(|item| item.claimable).count();
    let reward_unclaimed_card_item_count = reward_items
        .iter()
        .filter(|item| item.opens_card_choice)
        .count();
    let reward_free_value_score = reward_items
        .iter()
        .filter(|item| item.claimable)
        .map(|item| item.free_value_score.max(0))
        .sum::<i32>();
    let reward_phase = if reward_state.pending_card_choice.is_some() {
        "card_choice"
    } else if reward_claimable_item_count > 0 {
        "claim_items"
    } else {
        "cleanup"
    };

    RunScreenObservationV0 {
        reward_item_count: reward_state.items.len(),
        reward_card_choice_count: reward_state
            .pending_card_choice
            .as_ref()
            .map(Vec::len)
            .unwrap_or(0),
        reward_phase: reward_phase.to_string(),
        reward_items,
        reward_claimable_item_count,
        reward_unclaimed_card_item_count,
        reward_free_value_score,
        ..empty_screen_observation()
    }
}

fn reward_item_observation(
    run_state: &RunState,
    item_index: usize,
    item: &RewardItem,
) -> RunRewardItemObservationV0 {
    let claimable = reward_item_claimable(run_state, item);
    let likely_waste = reward_item_likely_waste(run_state, item);
    let capacity_blocked = reward_item_capacity_blocked(run_state, item);
    RunRewardItemObservationV0 {
        item_index,
        item_type: reward_item_type_label(item).to_string(),
        amount: reward_item_amount(item),
        card_choice_count: match item {
            RewardItem::Card { cards } => cards.len(),
            _ => 0,
        },
        relic_id: match item {
            RewardItem::Relic { relic_id } => Some(format!("{relic_id:?}")),
            _ => None,
        },
        potion_id: match item {
            RewardItem::Potion { potion_id } => Some(format!("{potion_id:?}")),
            _ => None,
        },
        claimable,
        opens_card_choice: matches!(item, RewardItem::Card { .. }),
        free_value_score: reward_item_claim_score(run_state, item),
        likely_waste,
        capacity_blocked,
    }
}

fn build_action_candidates(
    legal_actions: &[ClientInput],
    ctx: Option<&EpisodeContext>,
) -> Vec<RunActionCandidate> {
    let combat = ctx.and_then(|ctx| ctx.combat_state.as_ref());
    legal_actions
        .iter()
        .enumerate()
        .map(|(action_index, action)| {
            let action_key = action_key_for_input(action, combat);
            let card = ctx.and_then(|ctx| card_feature_for_action(action, ctx));
            let plan_delta = ctx
                .map(|ctx| candidate_plan_delta_for_action(action, ctx))
                .unwrap_or_else(empty_candidate_plan_delta);
            let reward_structure = ctx
                .map(|ctx| reward_action_structure_for_action(action, ctx))
                .unwrap_or_else(empty_reward_action_structure);
            RunActionCandidate {
                action_index,
                action_id: stable_action_id(&action_key),
                action_key,
                action: trace_input_from_client_input(action),
                card,
                plan_delta,
                reward_structure,
            }
        })
        .collect()
}

fn empty_reward_action_structure() -> RewardActionStructureV0 {
    RewardActionStructureV0 {
        score_kind: "heuristic".to_string(),
        screen_phase: "none".to_string(),
        ..RewardActionStructureV0::default()
    }
}

fn reward_action_structure_for_action(
    action: &ClientInput,
    ctx: &EpisodeContext,
) -> RewardActionStructureV0 {
    let EngineState::RewardScreen(reward_state) = &ctx.engine_state else {
        return empty_reward_action_structure();
    };
    if reward_state.pending_card_choice.is_some() {
        return RewardActionStructureV0 {
            score_kind: "heuristic".to_string(),
            screen_phase: "card_choice".to_string(),
            is_reward_action: matches!(action, ClientInput::SelectCard(_) | ClientInput::Proceed),
            skip_card_choice: matches!(action, ClientInput::Proceed),
            proceed_is_cleanup: false,
            ..RewardActionStructureV0::default()
        };
    }

    let unclaimed_reward_count = reward_state.items.len();
    let unclaimed_card_reward_count = reward_state
        .items
        .iter()
        .filter(|item| matches!(item, RewardItem::Card { .. }))
        .count();
    match action {
        ClientInput::ClaimReward(index) => reward_state
            .items
            .get(*index)
            .map(|item| {
                let item_obs = reward_item_observation(&ctx.run_state, *index, item);
                RewardActionStructureV0 {
                    score_kind: "heuristic".to_string(),
                    screen_phase: "claim_items".to_string(),
                    is_reward_action: true,
                    unclaimed_reward_count,
                    unclaimed_card_reward_count,
                    claim_reward_item_type: Some(item_obs.item_type),
                    claim_opens_card_choice: item_obs.opens_card_choice,
                    claim_free_value_score: item_obs.free_value_score,
                    claim_likely_waste: item_obs.likely_waste,
                    claim_capacity_blocked: item_obs.capacity_blocked,
                    ..RewardActionStructureV0::default()
                }
            })
            .unwrap_or_else(empty_reward_action_structure),
        ClientInput::Proceed => RewardActionStructureV0 {
            score_kind: "heuristic".to_string(),
            screen_phase: if unclaimed_reward_count > 0 {
                "claim_items".to_string()
            } else {
                "cleanup".to_string()
            },
            is_reward_action: true,
            is_proceed_with_unclaimed_rewards: unclaimed_reward_count > 0,
            unclaimed_reward_count,
            unclaimed_card_reward_count,
            proceed_is_cleanup: unclaimed_reward_count == 0,
            ..RewardActionStructureV0::default()
        },
        _ => empty_reward_action_structure(),
    }
}

fn card_feature_for_action(action: &ClientInput, ctx: &EpisodeContext) -> Option<RunCardFeatureV0> {
    match action {
        ClientInput::PlayCard { card_index, .. } => ctx
            .combat_state
            .as_ref()
            .and_then(|combat| combat.zones.hand.get(*card_index))
            .map(|card| build_card_feature(card.id, card.upgrades, &ctx.run_state)),
        ClientInput::SelectCard(index) => match &ctx.engine_state {
            EngineState::RewardScreen(reward_state) => reward_state
                .pending_card_choice
                .as_ref()
                .and_then(|cards| cards.get(*index))
                .map(|card| build_card_feature(card.id, card.upgrades, &ctx.run_state)),
            _ => None,
        },
        ClientInput::BuyCard(index) => match &ctx.engine_state {
            EngineState::Shop(shop) => shop
                .cards
                .get(*index)
                .map(|card| build_card_feature(card.card_id, 0, &ctx.run_state)),
            _ => None,
        },
        ClientInput::CampfireOption(CampfireChoice::Smith(index))
        | ClientInput::CampfireOption(CampfireChoice::Toke(index))
        | ClientInput::PurgeCard(index) => ctx
            .run_state
            .master_deck
            .get(*index)
            .map(|card| build_card_feature(card.id, card.upgrades, &ctx.run_state)),
        _ => None,
    }
}

fn candidate_plan_delta_for_action(
    action: &ClientInput,
    ctx: &EpisodeContext,
) -> CandidatePlanDeltaV0 {
    match action {
        ClientInput::SelectCard(index) => match &ctx.engine_state {
            EngineState::RewardScreen(reward_state) => reward_state
                .pending_card_choice
                .as_ref()
                .and_then(|cards| cards.get(*index))
                .map(|card| add_card_plan_delta(card.id, card.upgrades, &ctx.run_state))
                .unwrap_or_else(empty_candidate_plan_delta),
            _ => empty_candidate_plan_delta(),
        },
        ClientInput::BuyCard(index) => match &ctx.engine_state {
            EngineState::Shop(shop) => shop
                .cards
                .get(*index)
                .map(|card| add_card_plan_delta(card.card_id, 0, &ctx.run_state))
                .unwrap_or_else(empty_candidate_plan_delta),
            _ => empty_candidate_plan_delta(),
        },
        ClientInput::CampfireOption(CampfireChoice::Smith(index)) => ctx
            .run_state
            .master_deck
            .get(*index)
            .map(|card| upgrade_card_plan_delta(card.id, card.upgrades, &ctx.run_state))
            .unwrap_or_else(empty_candidate_plan_delta),
        ClientInput::CampfireOption(CampfireChoice::Toke(index))
        | ClientInput::PurgeCard(index) => ctx
            .run_state
            .master_deck
            .get(*index)
            .map(|card| remove_card_plan_delta(card.id, card.upgrades, &ctx.run_state))
            .unwrap_or_else(empty_candidate_plan_delta),
        _ => empty_candidate_plan_delta(),
    }
}

fn empty_candidate_plan_delta() -> CandidatePlanDeltaV0 {
    CandidatePlanDeltaV0 {
        score_kind: "heuristic".to_string(),
        ..CandidatePlanDeltaV0::default()
    }
}

fn add_card_plan_delta(
    card_id: CardId,
    upgrades: u8,
    run_state: &RunState,
) -> CandidatePlanDeltaV0 {
    let affordance = card_plan_affordance(card_id, upgrades);
    let profile = build_deck_plan_profile(run_state);
    let deck_deficit_bonus = deck_deficit_bonus(&profile, affordance, run_state);
    let bloat_penalty = deck_bloat_penalty(card_id, affordance, run_state);
    let duplicate_penalty = plan_duplicate_penalty(card_id, run_state);
    let rule_score = rule_card_offer_score(card_id, run_state);
    delta_from_affordance(
        affordance,
        0,
        deck_deficit_bonus,
        bloat_penalty,
        duplicate_penalty,
        rule_score + deck_deficit_bonus + bloat_penalty + duplicate_penalty,
    )
}

fn upgrade_card_plan_delta(
    card_id: CardId,
    upgrades: u8,
    run_state: &RunState,
) -> CandidatePlanDeltaV0 {
    let before = card_plan_affordance(card_id, upgrades);
    let after = card_plan_affordance(card_id, upgrades.saturating_add(1));
    let affordance = after.subtract(before);
    let profile = build_deck_plan_profile(run_state);
    let deck_deficit_bonus = deck_deficit_bonus(&profile, affordance, run_state);
    let rule_score = rule_upgrade_score(card_id);
    delta_from_affordance(
        affordance,
        0,
        deck_deficit_bonus,
        0,
        0,
        rule_score + deck_deficit_bonus,
    )
}

fn remove_card_plan_delta(
    card_id: CardId,
    upgrades: u8,
    run_state: &RunState,
) -> CandidatePlanDeltaV0 {
    let affordance = card_plan_affordance(card_id, upgrades);
    let burden_delta = if crate::content::cards::is_starter_basic(card_id) {
        -10
    } else {
        0
    };
    let mut out = delta_from_affordance(
        CardPlanAffordance {
            frontload: -affordance.frontload,
            block: -affordance.block,
            draw: -affordance.draw,
            scaling: -affordance.scaling,
            aoe: -affordance.aoe,
            exhaust: -affordance.exhaust,
            kill_window: -affordance.kill_window,
            setup_cashout_risk: -affordance.setup_cashout_risk,
        },
        burden_delta,
        0,
        0,
        0,
        rule_remove_score(card_id, run_state),
    );
    if burden_delta < 0 {
        out.deck_deficit_bonus += 25;
        out.plan_adjusted_score += 25;
    }
    if run_state.master_deck.len() <= 14 && affordance.frontload > 0 {
        out.deck_deficit_bonus -= 10;
        out.plan_adjusted_score -= 10;
    }
    out
}

fn delta_from_affordance(
    affordance: CardPlanAffordance,
    starter_basic_burden_delta: i32,
    deck_deficit_bonus: i32,
    bloat_penalty: i32,
    duplicate_penalty: i32,
    plan_adjusted_score: i32,
) -> CandidatePlanDeltaV0 {
    CandidatePlanDeltaV0 {
        score_kind: "heuristic".to_string(),
        frontload_delta: affordance.frontload,
        block_delta: affordance.block,
        draw_delta: affordance.draw,
        scaling_delta: affordance.scaling,
        aoe_delta: affordance.aoe,
        exhaust_delta: affordance.exhaust,
        kill_window_delta: affordance.kill_window,
        starter_basic_burden_delta,
        setup_cashout_risk_delta: affordance.setup_cashout_risk,
        deck_deficit_bonus,
        bloat_penalty,
        duplicate_penalty,
        plan_adjusted_score,
    }
}

fn deck_deficit_bonus(
    profile: &DeckPlanProfileV0,
    affordance: CardPlanAffordance,
    run_state: &RunState,
) -> i32 {
    let mut bonus = 0;
    if profile.frontload_supply < 70 {
        bonus += affordance.frontload;
    }
    if profile.block_supply < 50 {
        bonus += affordance.block;
    }
    if profile.draw_supply < 20 {
        bonus += affordance.draw * 2;
    } else if profile.draw_supply < 35 {
        bonus += affordance.draw;
    }
    if profile.scaling_supply < 20 {
        bonus += affordance.scaling * 2;
    } else if profile.scaling_supply < 35 {
        bonus += affordance.scaling;
    }
    if profile.aoe_supply < 18 && (run_state.act_num >= 2 || run_state.floor_num >= 7) {
        bonus += affordance.aoe * 2;
    } else if profile.aoe_supply < 18 {
        bonus += affordance.aoe;
    }
    if profile.exhaust_supply < 12 {
        bonus += affordance.exhaust;
    }
    if profile.kill_window_supply == 0 {
        bonus += affordance.kill_window / 2;
    }
    bonus
}

fn deck_bloat_penalty(
    card_id: CardId,
    affordance: CardPlanAffordance,
    run_state: &RunState,
) -> i32 {
    if run_state.master_deck.len() < 22 {
        return 0;
    }
    let high_value_plan_card = affordance.draw > 0
        || affordance.scaling > 0
        || affordance.aoe > 0
        || affordance.kill_window > 0
        || matches!(
            card_id,
            CardId::Disarm | CardId::Shockwave | CardId::Offering
        );
    if high_value_plan_card {
        -5
    } else {
        -18
    }
}

fn plan_duplicate_penalty(card_id: CardId, run_state: &RunState) -> i32 {
    let copies = run_state
        .master_deck
        .iter()
        .filter(|card| card.id == card_id)
        .count() as i32;
    -(copies * 5)
}

fn build_card_feature(card_id: CardId, upgrades: u8, run_state: &RunState) -> RunCardFeatureV0 {
    let def = crate::content::cards::get_card_definition(card_id);
    let deck_copies = run_state
        .master_deck
        .iter()
        .filter(|card| card.id == card_id)
        .count();
    RunCardFeatureV0 {
        card_id: format!("{card_id:?}"),
        card_id_hash: stable_action_id(&format!("card:{card_id:?}")),
        card_type_id: card_type_id(def.card_type),
        rarity_id: card_rarity_id(def.rarity),
        cost: def.cost,
        upgrades,
        base_damage: def.base_damage,
        base_block: def.base_block,
        base_magic: def.base_magic,
        upgraded_damage: def.base_damage + def.upgrade_damage * upgrades as i32,
        upgraded_block: def.base_block + def.upgrade_block * upgrades as i32,
        upgraded_magic: def.base_magic + def.upgrade_magic * upgrades as i32,
        exhaust: def.exhaust,
        ethereal: def.ethereal,
        innate: def.innate,
        aoe: matches!(def.target, crate::content::cards::CardTarget::AllEnemy),
        multi_damage: def.is_multi_damage || card_is_multi_hit(card_id),
        starter_basic: crate::content::cards::is_starter_basic(card_id),
        draws_cards: card_draws_cards(card_id),
        gains_energy: card_gains_energy(card_id),
        applies_weak: card_applies_weak(card_id),
        applies_vulnerable: card_applies_vulnerable(card_id),
        scaling_piece: card_is_scaling_piece(card_id),
        deck_copies,
        rule_score: rule_card_offer_score(card_id, run_state),
    }
}

fn card_type_id(card_type: CardType) -> u8 {
    match card_type {
        CardType::Attack => 1,
        CardType::Skill => 2,
        CardType::Power => 3,
        CardType::Status => 4,
        CardType::Curse => 5,
    }
}

fn card_rarity_id(rarity: CardRarity) -> u8 {
    match rarity {
        CardRarity::Basic => 1,
        CardRarity::Common => 2,
        CardRarity::Uncommon => 3,
        CardRarity::Rare => 4,
        CardRarity::Special => 5,
        CardRarity::Curse => 6,
    }
}

fn card_draws_cards(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::BattleTrance
            | CardId::BurningPact
            | CardId::DarkEmbrace
            | CardId::DeepBreath
            | CardId::Dropkick
            | CardId::Evolve
            | CardId::Finesse
            | CardId::FlashOfSteel
            | CardId::GoodInstincts
            | CardId::MasterOfStrategy
            | CardId::Offering
            | CardId::PommelStrike
            | CardId::ShrugItOff
            | CardId::Warcry
            | CardId::Acrobatics
            | CardId::Backflip
            | CardId::Prepared
            | CardId::DaggerThrow
            | CardId::Adrenaline
    )
}

fn card_gains_energy(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Bloodletting
            | CardId::Berserk
            | CardId::Offering
            | CardId::SeeingRed
            | CardId::Sentinel
            | CardId::Adrenaline
    )
}

fn card_applies_weak(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Clothesline
            | CardId::Intimidate
            | CardId::Shockwave
            | CardId::Uppercut
            | CardId::Blind
    )
}

fn card_applies_vulnerable(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Bash | CardId::Shockwave | CardId::ThunderClap | CardId::Trip | CardId::Uppercut
    )
}

fn card_is_multi_hit(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Pummel
            | CardId::SwordBoomerang
            | CardId::TwinStrike
            | CardId::Whirlwind
            | CardId::Reaper
    )
}

fn card_exhausts_other_cards(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::BurningPact
            | CardId::FiendFire
            | CardId::SecondWind
            | CardId::SeverSoul
            | CardId::TrueGrit
            | CardId::Purity
    )
}

fn card_is_scaling_piece(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::DemonForm
            | CardId::Inflame
            | CardId::LimitBreak
            | CardId::Rupture
            | CardId::SpotWeakness
            | CardId::Barricade
            | CardId::Entrench
            | CardId::Juggernaut
            | CardId::Metallicize
            | CardId::FeelNoPain
            | CardId::DarkEmbrace
            | CardId::Corruption
            | CardId::Evolve
            | CardId::FireBreathing
            | CardId::Footwork
            | CardId::NoxiousFumes
            | CardId::AfterImage
    )
}

fn card_is_block_core(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Defend
            | CardId::DefendG
            | CardId::Apparition
            | CardId::GhostlyArmor
            | CardId::FlameBarrier
            | CardId::Impervious
            | CardId::PowerThrough
            | CardId::ShrugItOff
            | CardId::Backflip
            | CardId::CloakAndDagger
            | CardId::GoodInstincts
            | CardId::DarkShackles
    )
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

fn action_key_for_input(input: &ClientInput, combat: Option<&CombatState>) -> String {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            let card_label = combat
                .and_then(|combat| combat.zones.hand.get(*card_index))
                .map(|card| format!("{:?}", card.id))
                .unwrap_or_else(|| "unknown".to_string());
            format!(
                "combat/play_card/card:{card_label}/hand:{card_index}/target:{}",
                target_label(*target, combat)
            )
        }
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
        EngineState::RewardScreen(reward_state) if reward_state.pending_card_choice.is_some() => {
            "reward_card_choice"
        }
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
    config: &RunBatchConfig,
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
        config: RunTraceConfigV0 {
            seed: summary.seed,
            ascension: config.ascension,
            final_act: config.final_act,
            player_class: config.player_class.to_string(),
            max_steps: config.max_steps,
            policy: config.policy.as_str().to_string(),
            reward_shaping_profile: config.reward_shaping_profile.as_str().to_string(),
        },
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
        combat.entities.player.current_hp = 35;
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
