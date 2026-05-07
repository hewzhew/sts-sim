use super::*;

pub const FULL_RUN_OBSERVATION_SCHEMA_VERSION: &str = "full_run_observation_v5_reward_structure";
pub const FULL_RUN_ACTION_SCHEMA_VERSION: &str =
    "full_run_action_candidate_set_v3_reward_structure";
pub const COMBAT_CANDIDATE_OUTCOME_PACK_SCHEMA_VERSION: &str = "combat_candidate_outcome_pack_v0";
pub(crate) const NO_PROGRESS_REPEAT_LIMIT: usize = 8;

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
    pub pending_choice_kind: Option<String>,
    pub pending_choice: Option<RunPendingChoiceObservationV0>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunPendingChoiceObservationV0 {
    pub kind: String,
    pub min_select: u8,
    pub max_select: u8,
    pub can_cancel: bool,
    pub reason: Option<String>,
    pub source_pile: Option<String>,
    pub options: Vec<RunPendingChoiceOptionObservationV0>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunPendingChoiceOptionObservationV0 {
    pub option_index: usize,
    pub label: String,
    pub card_id: Option<String>,
    pub card_uuid: Option<u32>,
    pub selection_uuids: Vec<u32>,
    pub source_pile: Option<String>,
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
    pub plan_delta: Option<CandidatePlanDeltaV0>,
    pub reward_structure: Option<RewardActionStructureV0>,
    pub dominated: bool,
    pub dominated_by_index: Option<usize>,
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
pub struct EpisodeRun {
    pub(crate) summary: RunEpisodeSummary,
    pub(crate) trace: Vec<RunStepTrace>,
    pub(crate) actions: Vec<ClientInput>,
}

#[derive(Clone, Debug)]
pub enum EpisodePolicy {
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

#[derive(Clone, Debug, PartialEq)]
pub struct EpisodeContext {
    pub(crate) engine_state: EngineState,
    pub(crate) run_state: RunState,
    pub(crate) combat_state: Option<CombatState>,
    pub(crate) stashed_event_combat: Option<EventCombatState>,
    pub(crate) forced_engine_ticks: usize,
    pub(crate) combat_win_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub floor: i32,
    pub act: u8,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_size: usize,
    pub relic_count: usize,
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

#[derive(Clone, Debug)]
pub struct FullRunTraceDrawMarginalProbeConfig {
    pub trace_file: PathBuf,
    pub step_index: usize,
    pub target_card: crate::content::cards::CardId,
    pub target_hand_index: Option<usize>,
    pub target_action_key: Option<String>,
    pub ascension: Option<u8>,
    pub final_act: Option<bool>,
    pub player_class: Option<String>,
    pub max_steps: Option<usize>,
    pub probe_config: crate::bot::combat::CombatTurnPlanProbeConfig,
}

#[derive(Clone, Debug)]
pub struct FullRunTraceCandidateOutcomePackConfig {
    pub trace_file: PathBuf,
    pub step_index: usize,
    pub ascension: Option<u8>,
    pub final_act: Option<bool>,
    pub player_class: Option<String>,
    pub max_steps: Option<usize>,
    pub max_exact_nodes_per_candidate: usize,
    pub max_engine_steps_per_action: usize,
    pub max_candidates: Option<usize>,
    pub controlled_v0: bool,
    pub min_eligible_candidates: usize,
}

#[derive(Clone, Debug)]
pub struct FullRunTraceCandidateOutcomePackBatchConfig {
    pub trace_inputs: Vec<PathBuf>,
    pub out_dir: PathBuf,
    pub step_start: usize,
    pub step_end: Option<usize>,
    pub step_limit: Option<usize>,
    pub ascension: Option<u8>,
    pub final_act: Option<bool>,
    pub player_class: Option<String>,
    pub max_steps: Option<usize>,
    pub budgets: Vec<usize>,
    pub max_engine_steps_per_action: usize,
    pub min_eligible_candidates: usize,
    pub min_trainable_pairs: usize,
    pub median_runtime_ms_limit: u128,
}

#[derive(Clone, Debug)]
pub struct FullRunTraceRecursiveRolloutValidationConfig {
    pub trace_inputs: Vec<PathBuf>,
    pub out_dir: PathBuf,
    pub step_start: usize,
    pub step_end: Option<usize>,
    pub step_limit: Option<usize>,
    pub ascension: Option<u8>,
    pub final_act: Option<bool>,
    pub player_class: Option<String>,
    pub max_steps: Option<usize>,
    pub horizon_decisions: usize,
    pub continuation_policy: RunPolicyKind,
    pub max_candidates: Option<usize>,
    pub controlled_v0: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatCandidateOutcomePackReport {
    pub schema_version: String,
    pub source_trace: serde_json::Value,
    pub split_group_key: String,
    pub split_group_key_kind: String,
    pub observation_schema_version: String,
    pub action_schema_version: String,
    pub observation: RunObservationV0,
    pub start_outcome: CombatCandidateOutcomeVector,
    pub oracle_config: CombatCandidateOutcomeOracleConfig,
    pub pack_oracle_quality: CombatCandidateOutcomePackOracleQuality,
    pub candidate_count: usize,
    pub candidates: Vec<CombatRootCandidateOutcome>,
    pub pairwise_labels: Vec<CombatCandidatePairwiseLabel>,
    pub training_contract: CombatCandidateOutcomeTrainingContract,
    pub truth_warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatCandidateOutcomeOracleConfig {
    pub oracle_kind: String,
    pub root_action_policy: String,
    pub max_exact_nodes_per_candidate: usize,
    pub max_engine_steps_per_action: usize,
    pub primary_label_policy: String,
    pub controlled_v0: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatCandidateOutcomeTrainingContract {
    pub allowed_primary_targets: Vec<String>,
    pub disallowed_primary_targets: Vec<String>,
    pub required_split_grouping: String,
    pub required_ablations: Vec<String>,
    pub closed_loop_gate: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatRootCandidateOutcome {
    pub candidate_index: usize,
    pub candidate: RunActionCandidate,
    pub exact_turn: CombatExactTurnOutcomeSummary,
    pub oracle_quality: CombatCandidateOracleQuality,
    pub bounded_objectives: CombatCandidateBoundedObjectives,
    pub outcome_aggregate: CombatCandidateOutcomeAggregate,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatCandidateBoundedObjectives {
    pub oracle_kind: String,
    pub root_simulation_status: String,
    pub root_engine_state: String,
    pub root_engine_steps: u32,
    pub root_simulation_truncated: bool,
    pub uncertainty_flags: Vec<String>,
    pub damage_done_immediate: i32,
    pub damage_upper_bound: i32,
    pub hp_loss_lower_bound: i32,
    pub hp_loss_upper_bound: i32,
    pub block_after_root: i32,
    pub block_upper_bound: i32,
    pub lethal_lower_bound: i32,
    pub lethal_upper_bound: i32,
    pub setup_lower_bound: i32,
    pub setup_upper_bound: i32,
    pub objective_bounds: Vec<CombatCandidateObjectiveBound>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatCandidateObjectiveBound {
    pub objective: String,
    pub lower_bound: i32,
    pub upper_bound: i32,
    pub higher_is_better: bool,
    pub confidence: String,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatCandidatePairwiseLabel {
    pub objective: String,
    pub preferred_candidate_index: usize,
    pub rejected_candidate_index: usize,
    pub preferred_action_key: String,
    pub rejected_action_key: String,
    pub confidence: String,
    pub reason: String,
    pub interval_gap: i32,
    pub label_source: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatExactTurnOutcomeSummary {
    pub status: String,
    pub truncated: bool,
    pub explored_nodes: u32,
    pub dominance_prunes: u32,
    pub cycle_cuts: u32,
    pub cache_hits: u32,
    pub cache_misses: u32,
    pub elapsed_ms: u128,
    pub best_line_debug: Vec<String>,
    pub nondominated_end_state_count: usize,
    pub truncation: CombatExactTurnTruncationSummary,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CombatExactTurnTruncationSummary {
    pub max_nodes_hit: bool,
    pub engine_step_limit_hit: bool,
    pub deadline_hit: bool,
    pub cycle_cut: bool,
    pub step_projection_truncated: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatCandidateOracleQuality {
    pub eligible_for_training: bool,
    pub ineligibility_reasons: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatCandidateOutcomePackOracleQuality {
    pub trainable_candidate_count: usize,
    pub ineligible_candidate_count: usize,
    pub trainable_pair_count: usize,
    pub truncated_candidate_count: usize,
    pub truncation_reasons: BTreeMap<String, usize>,
    pub controlled_v0: bool,
    pub trainable_manifest_eligible: bool,
    pub bounded_pairwise_label_count: usize,
    pub bounded_pairwise_manifest_eligible: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatCandidateOutcomeAggregate {
    pub nondominated_count: usize,
    pub unique_outcome_count: usize,
    pub any_combat_cleared: bool,
    pub any_player_dead: bool,
    pub any_no_hp_loss: bool,
    pub min_projected_unblocked_damage: i32,
    pub max_projected_unblocked_damage: i32,
    pub min_total_monster_hp: i32,
    pub max_total_monster_hp: i32,
    pub max_enemy_hp_reduction: i32,
    pub min_hp_lost: i32,
    pub max_hp_lost: i32,
    pub max_final_hp: i32,
    pub min_final_hp: i32,
    pub max_final_block: i32,
    pub min_spent_potions: u8,
    pub min_exhausted_cards: u16,
    pub representative_outcome: Option<CombatCandidateOutcomeVector>,
    pub unique_outcomes: Vec<CombatCandidateOutcomeVector>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CombatCandidateOutcomeVector {
    pub engine_state: String,
    pub terminal_kind: String,
    pub combat_cleared: bool,
    pub player_dead: bool,
    pub player_hp: i32,
    pub player_block: i32,
    pub energy: i32,
    pub visible_incoming_damage: i32,
    pub projected_unblocked_damage: i32,
    pub total_monster_hp: i32,
    pub living_monster_count: usize,
    pub monster_hp_reduction_from_start: i32,
    pub monster_deaths_from_start: usize,
    pub hp_lost_from_start: i32,
    pub hand_count: usize,
    pub draw_count: usize,
    pub discard_count: usize,
    pub exhaust_count: usize,
    pub player_powers: Vec<CombatPowerSnapshot>,
    pub monsters: Vec<CombatMonsterSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CombatPowerSnapshot {
    pub owner: String,
    pub power_id: String,
    pub amount: i32,
    pub extra_data: i32,
    pub just_applied: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CombatMonsterSnapshot {
    pub slot: u8,
    pub entity_id: usize,
    pub monster_id: String,
    pub hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub alive: bool,
    pub dying: bool,
    pub escaped: bool,
    pub half_dead: bool,
    pub planned_move_id: u8,
    pub visible_incoming_damage: i32,
    pub powers: Vec<CombatPowerSnapshot>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatCandidateOutcomePackBatchReport {
    pub schema_version: String,
    pub generated_pack_schema_version: String,
    pub out_dir: String,
    pub budgets: Vec<CombatCandidateOutcomeBudgetSummary>,
    pub selected_budget: Option<usize>,
    pub oracle_ready: bool,
    pub oracle_ready_reason: String,
    pub trainable_manifest: Vec<String>,
    pub diagnostic_manifest: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatCandidateOutcomeBudgetSummary {
    pub budget: usize,
    pub pack_count: usize,
    pub trainable_pack_count: usize,
    pub candidate_count: usize,
    pub eligible_candidate_count: usize,
    pub truncated_candidate_count: usize,
    pub eligible_candidate_ratio: f32,
    pub trainable_pair_count: usize,
    pub median_candidate_elapsed_ms: u128,
    pub truncation_reasons: BTreeMap<String, usize>,
    pub pack_manifest: Vec<String>,
    pub trainable_manifest: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FullRunEnv {
    pub(crate) config: FullRunEnvConfig,
    pub(crate) ctx: EpisodeContext,
    pub(crate) steps: usize,
    pub(crate) done: bool,
    pub(crate) terminal_reason: String,
    pub(crate) crash: Option<String>,
    pub(crate) contract_failure: Option<RunContractFailure>,
    pub(crate) no_progress_tracker: NoProgressTracker,
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
