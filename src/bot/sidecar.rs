use crate::bot::reward_heuristics::RewardScreenEvaluation;
use crate::bot::search::SearchDiagnostics;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Write;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct RewardSidecarSuggestion {
    pub(crate) suggested_index: Option<usize>,
    pub(crate) rationale: &'static str,
    pub(crate) score_delta: f32,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct CombatRootSidecarSuggestion {
    pub(crate) suggested_move: String,
    pub(crate) rationale: &'static str,
    pub(crate) score_delta: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct CombatTopCandidateRecord {
    pub(crate) move_label: String,
    pub(crate) avg_score: f32,
    pub(crate) order_score: f32,
    pub(crate) leaf_score: f32,
    pub(crate) sequence_bonus: f32,
    pub(crate) sequence_frontload_bonus: f32,
    pub(crate) sequence_defer_bonus: f32,
    pub(crate) sequence_branch_bonus: f32,
    pub(crate) sequence_downside_penalty: f32,
    pub(crate) projected_unblocked: i32,
    pub(crate) projected_enemy_total: i32,
    pub(crate) survives: bool,
    pub(crate) branch_family: Option<String>,
    pub(crate) cluster_size: usize,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct CombatPressureSidecarSuggestion {
    pub(crate) visible_pressure: Option<i32>,
    pub(crate) belief_expected_pressure: f32,
    pub(crate) belief_max_pressure: i32,
    pub(crate) value_pressure: f32,
    pub(crate) survival_guard_pressure: i32,
    pub(crate) urgent_probability: f32,
    pub(crate) lethal_probability: f32,
    pub(crate) rationale: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct RewardShadowRecord {
    pub(crate) kind: &'static str,
    pub(crate) frame: u64,
    pub(crate) source: &'static str,
    pub(crate) recommended_choice: Option<usize>,
    pub(crate) chosen_choice: Option<usize>,
    pub(crate) skip_chosen: bool,
    pub(crate) offered_count: usize,
    pub(crate) evaluation: Value,
    pub(crate) suggestion: Option<RewardSidecarSuggestion>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct CombatShadowRecord {
    pub(crate) kind: &'static str,
    pub(crate) frame: u64,
    pub(crate) source: &'static str,
    pub(crate) chosen_move: String,
    pub(crate) top_gap: Option<f32>,
    pub(crate) legal_moves: usize,
    pub(crate) reduced_legal_moves: usize,
    pub(crate) top_candidates: Vec<CombatTopCandidateRecord>,
    pub(crate) suggestion_move: Option<String>,
    pub(crate) suggestion_disagrees: bool,
    pub(crate) disagreement_reason: Option<String>,
    pub(crate) suggestion_confidence: Option<f32>,
    pub(crate) suggestion: Option<CombatRootSidecarSuggestion>,
    pub(crate) pressure: Option<CombatPressureSidecarSuggestion>,
}

pub(crate) fn write_shadow_record<W: Write>(sink: &mut W, record: &impl Serialize) {
    if let Ok(line) = serde_json::to_string(record) {
        let _ = writeln!(sink, "{}", line);
        let _ = sink.flush();
    }
}

pub(crate) fn reward_shadow_json(
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

pub(crate) fn combat_shadow_json(
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
