use super::*;

pub const FULL_RUN_OBSERVATION_SCHEMA_VERSION: &str =
    "full_run_observation_v9_event_action_semantics";
pub const FULL_RUN_ACTION_SCHEMA_VERSION: &str =
    "full_run_action_candidate_set_v6_semantic_descriptor";
pub(crate) const NO_PROGRESS_REPEAT_LIMIT: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunActionSelectorKind {
    RandomMasked,
}

impl RunActionSelectorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RandomMasked => "random_masked",
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
    pub action_selector: RunActionSelectorKind,
    pub trace_dir: Option<PathBuf>,
    pub determinism_check: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunBatchSummary {
    pub observation_schema_version: String,
    pub action_schema_version: String,
    pub action_mask_kind: String,
    pub action_selector: String,
    pub episodes_requested: usize,
    pub base_seed: u64,
    pub ascension: u8,
    pub final_act: bool,
    pub player_class: String,
    pub max_steps: usize,
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
    pub action_selector: String,
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
    pub action_selector: String,
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
    pub keys: RunKeyObservationV0,
    pub deck: RunDeckObservationV0,
    pub plan_profile: DeckPlanProfileV0,
    pub deck_cards: Vec<RunDeckCardObservationV0>,
    pub relics: Vec<RunRelicObservationV0>,
    pub potions: Vec<RunPotionSlotObservationV0>,
    pub map: Option<RunMapObservationV0>,
    pub next_nodes: Vec<RunMapNodeObservationV0>,
    pub map_route_context: Option<RunMapRouteContextV1>,
    pub act_boss: Option<String>,
    pub reward_source: Option<String>,
    pub combat: Option<RunCombatObservationV0>,
    pub screen: RunScreenObservationV0,
    pub recording_view: RunRecordingViewV1,
    pub decision_frame: RunDecisionFrameV1,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunRecordingViewV1 {
    pub schema_name: String,
    pub schema_version: u8,
    pub recording_source: String,
    pub state_lines: Vec<String>,
    pub context_lines: Vec<String>,
    pub warning_lines: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RunDecisionFrameV1 {
    pub schema_name: String,
    pub schema_version: u8,
    pub decision_kind: String,
    pub prompt: String,
    pub source: Option<RunDecisionSourceV1>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RunDecisionSourceV1 {
    pub kind: String,
    pub label: String,
    pub action_key: Option<String>,
    pub card_instance_id: Option<u32>,
    pub card_name: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct RunKeyObservationV0 {
    pub ruby: bool,
    pub sapphire: bool,
    pub emerald: bool,
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

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct RunMapRouteContextV1 {
    pub schema_name: String,
    pub schema_version: u8,
    pub decision_authority: String,
    pub not_final_action: bool,
    pub map_scope: String,
    pub context_level: String,
    pub current_x: i32,
    pub current_y: i32,
    pub act_boss: Option<String>,
    pub route_choices: Vec<RunMapRouteChoiceV1>,
    pub truth_warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct RunMapRouteChoiceV1 {
    pub action_key: String,
    pub next_x: i32,
    pub next_y: i32,
    pub room_type: Option<String>,
    pub room_label: String,
    pub burning_elite: bool,
    pub reachable_paths_to_boss: usize,
    pub min_elites: i32,
    pub max_elites: i32,
    pub expected_elites_milli: i32,
    pub min_fires: i32,
    pub max_fires: i32,
    pub expected_fires_milli: i32,
    pub min_shops: i32,
    pub max_shops: i32,
    pub expected_shops_milli: i32,
    pub shops_reachable: i32,
    pub chests_reachable: i32,
    pub events_reachable: i32,
    pub forced_fights_next_3: i32,
    pub earliest_shop_floor: Option<i32>,
    pub earliest_fire_floor: Option<i32>,
    pub rest_before_first_elite: bool,
    pub local_flex: String,
    pub global_path_flex: String,
    pub path_flexibility: String,
    pub branch_count: usize,
    pub burning_elite_reachable: bool,
    pub burning_elite_on_path: bool,
    pub risk_label: String,
    pub risk_vector: RunRouteRiskVectorV1,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RunRouteRiskVectorV1 {
    pub early_pressure: String,
    pub elite_ceiling: String,
    pub shop_access: String,
    pub recovery_access: String,
    pub path_flexibility: String,
    pub boss_prep_support: String,
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
    pub player_powers: Vec<RunPowerObservationV0>,
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
    pub monsters: Vec<RunMonsterObservationV0>,
    pub encounter_hints: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunMonsterObservationV0 {
    pub entity_id: usize,
    pub slot: u8,
    pub monster_id: String,
    pub name: String,
    pub current_hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub alive: bool,
    pub planned_move_id: u8,
    pub visible_intent: Option<String>,
    pub visible_intent_kind: String,
    pub visible_intent_damage_per_hit: Option<i32>,
    pub visible_intent_hits: u8,
    pub visible_intent_total_damage: Option<i32>,
    pub powers: Vec<RunPowerObservationV0>,
    pub mechanic_hints: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunPowerObservationV0 {
    pub power_id: String,
    pub amount: i32,
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
    pub subject_ref: Option<String>,
    pub before_summary: Option<String>,
    pub after_summary: Option<String>,
    pub delta_summary: Option<String>,
    pub preview_status: Option<String>,
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
}

#[derive(Clone, Debug, Serialize)]
pub struct RunScreenObservationV0 {
    pub event_option_count: usize,
    pub event_options: Vec<RunEventOptionObservationV0>,
    pub reward_item_count: usize,
    pub reward_card_choice_count: usize,
    pub reward_phase: String,
    pub reward_items: Vec<RunRewardItemObservationV0>,
    pub reward_card_choices: Vec<RunRewardCardChoiceObservationV0>,
    pub reward_claimable_item_count: usize,
    pub reward_unclaimed_card_item_count: usize,
    pub shop_card_count: usize,
    pub shop_relic_count: usize,
    pub shop_potion_count: usize,
    pub boss_relic_choice_count: usize,
    pub selection_target_count: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RunEventOptionObservationV0 {
    pub option_index: usize,
    pub label: String,
    pub disabled: bool,
    pub disabled_reason: Option<String>,
    pub semantic_descriptor: ActionSemanticDescriptorV1,
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
}

#[derive(Clone, Debug, Serialize)]
pub struct RunRewardCardChoiceObservationV0 {
    pub option_index: usize,
    pub card_id: String,
    pub card_name: String,
    pub upgrades: u8,
    pub card_type: String,
    pub rarity: String,
    pub cost: i8,
    pub base_semantics: Vec<String>,
    pub deck_copies: usize,
    pub card: RunCardFeatureV0,
    pub plan_delta: CandidatePlanDeltaV0,
    pub semantic_descriptor: ActionSemanticDescriptorV1,
}

#[derive(Clone, Debug, Serialize)]
pub struct RunActionCandidate {
    pub action_index: usize,
    pub action_id: u32,
    pub action_key: String,
    pub recording_label: String,
    pub recording_detail: Option<String>,
    pub recording_kind: String,
    pub action: TraceClientInput,
    pub card: Option<RunCardFeatureV0>,
    pub plan_delta: Option<CandidatePlanDeltaV0>,
    pub reward_structure: Option<RewardActionStructureV0>,
    pub semantic_descriptor: Option<ActionSemanticDescriptorV1>,
    pub choice_option: RunChoiceOptionV1,
    pub dominated: bool,
    pub dominated_by_index: Option<usize>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RunChoiceOptionV1 {
    pub schema_name: String,
    pub schema_version: u8,
    pub option_id: usize,
    pub action_id: u32,
    pub action_key: String,
    pub label: String,
    pub subject_ref: Option<String>,
    pub before_summary: Option<String>,
    pub after_summary: Option<String>,
    pub delta_summary: Option<String>,
    pub preview_status: String,
    pub unavailable_reason: Option<String>,
    pub danger_flags: Vec<String>,
    pub requires_confirmation: bool,
    pub confirmation_command: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ActionSemanticDescriptorV1 {
    pub schema_name: String,
    pub schema_version: u8,
    pub action_type: String,
    pub semantic_status: String,
    pub coverage_level: String,
    pub event_id: Option<String>,
    pub event_name: Option<String>,
    pub option_index: Option<usize>,
    pub label: String,
    pub costs: Vec<ActionSemanticEffectV1>,
    pub effects: Vec<ActionSemanticEffectV1>,
    pub constraints: Vec<String>,
    pub transition: String,
    pub unknown_fields: Vec<String>,
    pub source_chain: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ActionSemanticEffectV1 {
    pub effect_type: String,
    pub amount: Option<i32>,
    pub count: Option<usize>,
    pub kind: Option<String>,
    pub target: Option<String>,
    pub details: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct RewardActionStructureV0 {
    pub screen_phase: String,
    pub is_reward_action: bool,
    pub is_proceed_with_unclaimed_rewards: bool,
    pub unclaimed_reward_count: usize,
    pub unclaimed_card_reward_count: usize,
    pub claim_reward_item_type: Option<String>,
    pub claim_opens_card_choice: bool,
    pub proceed_is_cleanup: bool,
    pub skip_card_choice: bool,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct DeckPlanProfileV0 {
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
    pub frontload_delta: i32,
    pub block_delta: i32,
    pub draw_delta: i32,
    pub scaling_delta: i32,
    pub aoe_delta: i32,
    pub exhaust_delta: i32,
    pub kill_window_delta: i32,
    pub starter_basic_burden_delta: i32,
    pub setup_cashout_risk_delta: i32,
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
    OpenChest,
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
pub enum EpisodeActionSelector {
    RandomMasked {
        rng: StsRng,
    },
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
    pub fn batch_config(&self, action_selector: RunActionSelectorKind) -> RunBatchConfig {
        RunBatchConfig {
            episodes: 1,
            base_seed: self.seed,
            ascension: self.ascension,
            final_act: self.final_act,
            player_class: self.player_class,
            max_steps: self.max_steps,
            action_selector,
            trace_dir: None,
            determinism_check: false,
        }
    }
}
