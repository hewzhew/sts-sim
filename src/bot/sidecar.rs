use crate::bot::event_policy::{EventChoiceDecision, EventDecisionContext};
use crate::bot::reward_heuristics::RewardScreenEvaluation;
use crate::bot::search::SearchDiagnostics;
use crate::combat::CombatState;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SidecarMode {
    Disabled,
    Shadow,
}

#[derive(Clone, Debug, Default)]
pub struct SidecarRuntimeConfig {
    pub mode: Option<SidecarMode>,
}

impl SidecarRuntimeConfig {
    pub fn enabled(&self) -> bool {
        matches!(self.mode, Some(SidecarMode::Shadow))
    }
}

pub trait RewardSidecarReranker: Send + Sync {
    fn suggest(
        &self,
        _run: &crate::state::run::RunState,
        _evaluation: &RewardScreenEvaluation,
    ) -> Option<RewardSidecarSuggestion> {
        None
    }
}

pub trait EventSidecarReranker: Send + Sync {
    fn suggest(
        &self,
        _run: &crate::state::run::RunState,
        _context: &EventDecisionContext,
        _decision: &EventChoiceDecision,
    ) -> Option<EventSidecarSuggestion> {
        None
    }
}

pub trait CombatRootReranker: Send + Sync {
    fn suggest(
        &self,
        _combat: &CombatState,
        _search: &SearchDiagnostics,
    ) -> Option<CombatRootSidecarSuggestion> {
        None
    }
}

pub trait CombatPressureSidecar: Send + Sync {
    fn suggest(&self, _combat: &CombatState) -> Option<CombatPressureSidecarSuggestion> {
        None
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RewardSidecarSuggestion {
    pub suggested_index: Option<usize>,
    pub rationale: &'static str,
    pub score_delta: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct EventSidecarSuggestion {
    pub suggested_index: Option<usize>,
    pub rationale: &'static str,
    pub score_delta: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatRootSidecarSuggestion {
    pub suggested_move: String,
    pub rationale: &'static str,
    pub score_delta: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CombatTopCandidateRecord {
    pub move_label: String,
    pub avg_score: f32,
    pub order_score: f32,
    pub leaf_score: f32,
    pub sequence_bonus: f32,
    pub sequence_frontload_bonus: f32,
    pub sequence_defer_bonus: f32,
    pub sequence_branch_bonus: f32,
    pub sequence_downside_penalty: f32,
    pub projected_unblocked: i32,
    pub projected_enemy_total: i32,
    pub survives: bool,
    pub branch_family: Option<String>,
    pub cluster_size: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatPressureSidecarSuggestion {
    pub visible_pressure: Option<i32>,
    pub belief_expected_pressure: f32,
    pub belief_max_pressure: i32,
    pub value_pressure: f32,
    pub survival_guard_pressure: i32,
    pub urgent_probability: f32,
    pub lethal_probability: f32,
    pub rationale: &'static str,
}

pub struct NullRewardSidecarReranker;
impl RewardSidecarReranker for NullRewardSidecarReranker {}

pub struct NullEventSidecarReranker;
impl EventSidecarReranker for NullEventSidecarReranker {}

pub struct NullCombatRootReranker;
impl CombatRootReranker for NullCombatRootReranker {}

pub struct NullCombatPressureSidecar;
impl CombatPressureSidecar for NullCombatPressureSidecar {}

#[derive(Clone, Debug, Serialize)]
pub struct RewardShadowRecord {
    pub kind: &'static str,
    pub frame: u64,
    pub source: &'static str,
    pub recommended_choice: Option<usize>,
    pub chosen_choice: Option<usize>,
    pub skip_chosen: bool,
    pub offered_count: usize,
    pub evaluation: Value,
    pub suggestion: Option<RewardSidecarSuggestion>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EventShadowRecord<'a> {
    pub kind: &'static str,
    pub frame: u64,
    pub source: &'static str,
    pub event_name: &'a str,
    pub family: &'static str,
    pub chosen_choice: usize,
    pub rationale_key: Option<&'static str>,
    pub suggestion: Option<EventSidecarSuggestion>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatShadowRecord {
    pub kind: &'static str,
    pub frame: u64,
    pub source: &'static str,
    pub chosen_move: String,
    pub top_gap: Option<f32>,
    pub legal_moves: usize,
    pub reduced_legal_moves: usize,
    pub top_candidates: Vec<CombatTopCandidateRecord>,
    pub suggestion_move: Option<String>,
    pub suggestion_disagrees: bool,
    pub disagreement_reason: Option<String>,
    pub suggestion_confidence: Option<f32>,
    pub suggestion: Option<CombatRootSidecarSuggestion>,
    pub pressure: Option<CombatPressureSidecarSuggestion>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardTrainingRow {
    pub dataset_kind: String,
    pub state_source: String,
    pub label_source: String,
    pub label_strength: String,
    pub run_id: String,
    pub line_no: usize,
    pub frame: Option<u64>,
    pub response_id: Option<u64>,
    pub state_frame_id: Option<u64>,
    pub floor: Option<i64>,
    pub act: Option<i64>,
    pub class: Option<String>,
    pub current_hp: Option<i64>,
    pub max_hp: Option<i64>,
    pub gold: Option<i64>,
    pub deck_size: Option<i64>,
    pub offered_cards: Vec<Value>,
    pub candidates: Vec<Value>,
    pub recommended_choice: Option<usize>,
    pub label: Value,
    pub skip_chosen: bool,
    pub rule_context_summaries: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventTrainingRow {
    pub dataset_kind: String,
    pub state_source: String,
    pub label_source: String,
    pub label_strength: String,
    pub run_id: String,
    pub line_no: usize,
    pub frame: Option<u64>,
    pub room_phase: Option<String>,
    pub screen: Option<String>,
    pub command: Option<String>,
    pub event_id: Option<String>,
    pub event_name: Option<String>,
    pub family: Option<String>,
    pub rationale_key: Option<String>,
    pub screen_index: Option<usize>,
    pub screen_key: Option<String>,
    pub screen_source: Option<String>,
    pub chosen_option_index: Option<usize>,
    pub chosen_option_label: Option<String>,
    pub chosen_option_text: Option<String>,
    pub command_index: Option<usize>,
    pub score: Option<f32>,
    pub safety_override_applied: Option<bool>,
    pub decision: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CombatTrainingRow {
    pub dataset_kind: String,
    pub state_source: String,
    pub run_id: String,
    pub line_no: usize,
    pub frame_count: Option<u64>,
    pub response_id: Option<u64>,
    pub state_frame_id: Option<u64>,
    pub chosen_move: Option<String>,
    pub heuristic_move: Option<String>,
    pub search_move: Option<String>,
    pub top_candidates: Vec<CombatTopCandidateRecord>,
    pub top_gap: Option<f32>,
    pub sequence_bonus: Option<f32>,
    pub sequence_frontload_bonus: Option<f32>,
    pub sequence_defer_bonus: Option<f32>,
    pub sequence_branch_bonus: Option<f32>,
    pub sequence_downside_penalty: Option<f32>,
    pub branch_family: Option<String>,
    pub sequencing_rationale_key: Option<String>,
    pub branch_rationale_key: Option<String>,
    pub downside_rationale_key: Option<String>,
    pub heuristic_search_gap: bool,
    pub tight_root_gap: bool,
    pub large_sequence_bonus: bool,
    pub reasons: Vec<String>,
    pub sample_weight: f32,
    pub strong_label: bool,
    pub label_source: String,
    pub label_strength: String,
    pub snapshot_id: Option<String>,
    pub snapshot_trigger_kind: Option<String>,
    pub snapshot_reasons: Vec<String>,
    pub snapshot_normalized_state: Option<Value>,
    pub snapshot_decision_context: Option<Value>,
    pub hidden_intent_active: Option<bool>,
    pub visible_incoming: Option<i32>,
    pub visible_unblocked: Option<i32>,
    pub belief_expected_incoming: Option<f32>,
    pub belief_expected_unblocked: Option<i32>,
    pub belief_max_incoming: Option<i32>,
    pub belief_max_unblocked: Option<i32>,
    pub value_incoming: Option<i32>,
    pub value_unblocked: Option<i32>,
    pub survival_guard_incoming: Option<i32>,
    pub survival_guard_unblocked: Option<i32>,
    pub belief_attack_probability: Option<f32>,
    pub belief_lethal_probability: Option<f32>,
    pub belief_urgent_probability: Option<f32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OracleCombatTopCandidate {
    pub move_label: String,
    pub outcome: String,
    pub score: i32,
    pub final_player_hp: i32,
    pub final_player_block: i32,
    pub final_incoming: i32,
    pub final_monster_hp: Vec<i32>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OracleCombatLabelRow {
    pub dataset_kind: String,
    pub state_source: String,
    pub run_id: String,
    pub frame_count: Option<u64>,
    pub response_id: Option<u64>,
    pub state_frame_id: Option<u64>,
    pub baseline_chosen_move: Option<String>,
    pub heuristic_move: Option<String>,
    pub search_move: Option<String>,
    pub oracle_best_move: Option<String>,
    pub oracle_equivalent_best_moves: Vec<String>,
    pub oracle_top_candidates: Vec<OracleCombatTopCandidate>,
    pub oracle_value_estimate: Option<i32>,
    pub oracle_margin: Option<i32>,
    pub oracle_outcome_bucket: Option<String>,
    pub oracle_best_bucket_size: usize,
    pub oracle_report_path: Option<String>,
    pub label_source: String,
    pub label_strength: String,
    pub oracle_disagrees_with_baseline: bool,
    pub oracle_compute_budget: Value,
    pub baseline_row: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OracleRewardLabelRow {
    pub dataset_kind: String,
    pub state_source: String,
    pub run_id: String,
    pub frame: Option<u64>,
    pub label_source: String,
    pub label_strength: String,
    pub baseline_row: Value,
}

pub fn family_str(family: crate::bot::event_policy::EventPolicyFamily) -> &'static str {
    match family {
        crate::bot::event_policy::EventPolicyFamily::DeckSurgery => "deck_surgery",
        crate::bot::event_policy::EventPolicyFamily::PressYourLuck => "press_your_luck",
        crate::bot::event_policy::EventPolicyFamily::CostTradeoff => "cost_tradeoff",
        crate::bot::event_policy::EventPolicyFamily::ResourceShoplike => "resource_shoplike",
        crate::bot::event_policy::EventPolicyFamily::GenericSafe => "generic_safe",
        crate::bot::event_policy::EventPolicyFamily::CompatibilityFallback => {
            "compatibility_fallback"
        }
    }
}

pub fn write_shadow_record<W: Write>(sink: &mut W, record: &impl Serialize) {
    if let Ok(line) = serde_json::to_string(record) {
        let _ = writeln!(sink, "{}", line);
        let _ = sink.flush();
    }
}

pub fn reward_shadow_json(
    frame: u64,
    source: &'static str,
    evaluation: &RewardScreenEvaluation,
    chosen_choice: Option<usize>,
    suggestion: Option<RewardSidecarSuggestion>,
) -> Value {
    json!(RewardShadowRecord {
        kind: "reward_shadow",
        frame,
        source,
        recommended_choice: evaluation.recommended_choice,
        chosen_choice,
        skip_chosen: chosen_choice.is_none(),
        offered_count: evaluation.offered_cards.len(),
        evaluation: reward_screen_evaluation_json(evaluation),
        suggestion,
    })
}

pub fn event_shadow_json(
    frame: u64,
    source: &'static str,
    context: &EventDecisionContext,
    decision: &EventChoiceDecision,
    suggestion: Option<EventSidecarSuggestion>,
) -> Value {
    json!(EventShadowRecord {
        kind: "event_shadow",
        frame,
        source,
        event_name: &context.event_name,
        family: family_str(decision.family),
        chosen_choice: decision.option_index,
        rationale_key: decision.rationale_key,
        suggestion,
    })
}

pub fn combat_shadow_json(
    frame: u64,
    source: &'static str,
    chosen_move: String,
    search: &SearchDiagnostics,
    top_candidates: Vec<CombatTopCandidateRecord>,
    suggestion: Option<CombatRootSidecarSuggestion>,
    pressure: Option<CombatPressureSidecarSuggestion>,
) -> Value {
    let suggestion_move = suggestion
        .as_ref()
        .map(|value| value.suggested_move.clone());
    let suggestion_disagrees = suggestion_move
        .as_ref()
        .is_some_and(|suggested| suggested != &chosen_move);
    let disagreement_reason = if suggestion_disagrees {
        suggestion.as_ref().map(|value| value.rationale.to_string())
    } else {
        None
    };
    let suggestion_confidence = suggestion.as_ref().map(|value| value.score_delta.abs());
    json!(CombatShadowRecord {
        kind: "combat_shadow",
        frame,
        source,
        chosen_move,
        top_gap: search
            .top_moves
            .get(1)
            .map(|second| search.top_moves[0].avg_score - second.avg_score),
        legal_moves: search.legal_moves,
        reduced_legal_moves: search.reduced_legal_moves,
        top_candidates,
        suggestion_move,
        suggestion_disagrees,
        disagreement_reason,
        suggestion_confidence,
        suggestion,
        pressure,
    })
}

fn reward_screen_evaluation_json(evaluation: &RewardScreenEvaluation) -> Value {
    let cards = evaluation
        .offered_cards
        .iter()
        .map(|card| {
            json!({
                "rust_card_id": format!("{:?}", card.card_id),
                "pick_rate": card.pick_rate,
                "local_score": card.local_score,
                "delta_suite": format!("{:?}", card.delta_suite),
                "delta_prior": card.delta_prior,
                "delta_bias": card.delta_bias,
                "delta_rollout": card.delta_rollout,
                "delta_context": card.delta_context,
                "delta_context_rationale_key": card.delta_context_rationale_key,
                "delta_rule_context_summary": card.delta_rule_context_summary,
                "delta_score": card.delta_score,
                "combined_score": card.combined_score
            })
        })
        .collect::<Vec<_>>();
    json!({
        "cards": cards,
        "recommended_choice": evaluation.recommended_choice,
        "best_pick_rate": evaluation.best_pick_rate,
        "best_local_score": evaluation.best_local_score,
        "best_combined_score": evaluation.best_combined_score,
        "skip_probability": evaluation.skip_probability,
        "skip_margin": evaluation.skip_margin,
        "force_pick_in_act1": evaluation.force_pick_in_act1,
        "force_pick_for_shell": evaluation.force_pick_for_shell
    })
}
