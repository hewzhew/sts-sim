use crate::state::core::EngineState;
use crate::state::RunState;

use super::context::build_route_decision_context_v1;
use super::features::{node_features, summarize_route_from};
use super::needs::estimate_needs;
use super::scorer::{route_reasons, safety_flag, score_route_candidate};
use super::types::{
    RouteCandidateTraceV1, RouteDecisionTraceV1, RoutePlannerConfigV1, RouteSafetyFlagV1,
    ROUTE_DECISION_TRACE_SCHEMA_NAME, ROUTE_DECISION_TRACE_SCHEMA_VERSION,
};

pub fn plan_route_decision_v1(
    run_state: &RunState,
    engine_state: &EngineState,
    config: RoutePlannerConfigV1,
) -> RouteDecisionTraceV1 {
    let context = build_route_decision_context_v1(run_state);
    let needs = estimate_needs(&context, &config);
    let has_empty_potion_slot = context.potions.filled < context.potions.slots;
    let mut candidates = context
        .legal_next_nodes
        .iter()
        .map(|target| {
            let path_summary = summarize_route_from(run_state, target.x, target.y, &config);
            let features = node_features(
                target,
                &context.counters.unknown_belief,
                context.hp,
                context.max_hp,
                has_empty_potion_slot,
                &config,
            );
            let score_terms = score_route_candidate(
                &features,
                &path_summary,
                &needs,
                target.move_kind,
                context.counters.emerald_key_taken,
                &config,
            );
            let safety = safety_flag(&features, &path_summary, &needs);
            let (reasons, cautions) = route_reasons(&features, &path_summary, safety);
            RouteCandidateTraceV1 {
                target: target.clone(),
                features,
                path_summary,
                needs: needs.clone(),
                total_score: score_terms.total(),
                score_terms,
                safety,
                reasons,
                cautions,
                suggested_command: matches!(engine_state, EngineState::MapNavigation)
                    .then(|| format!("go {}", target.x)),
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|a, b| {
        safety_sort_key(b.safety)
            .cmp(&safety_sort_key(a.safety))
            .then_with(|| {
                b.total_score
                    .partial_cmp(&a.total_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.target.x.cmp(&b.target.x))
    });
    let selected_index = selected_index(&candidates);
    let mut warnings = Vec::new();
    if !matches!(engine_state, EngineState::MapNavigation) {
        warnings.push(
            "route selection is locked until the current screen returns to map navigation"
                .to_string(),
        );
    }
    if candidates
        .iter()
        .any(|candidate| candidate.safety == RouteSafetyFlagV1::RejectUnlessNoAlternative)
    {
        warnings.push("some route candidates are gated by safety risk".to_string());
    }
    RouteDecisionTraceV1 {
        schema_name: ROUTE_DECISION_TRACE_SCHEMA_NAME.to_string(),
        schema_version: ROUTE_DECISION_TRACE_SCHEMA_VERSION,
        objective: config.objective,
        selection_mode: config.selection_mode,
        label_role: "behavior_policy_not_teacher".to_string(),
        context,
        candidates,
        selected_index,
        warnings,
    }
}

fn selected_index(candidates: &[RouteCandidateTraceV1]) -> Option<usize> {
    candidates
        .iter()
        .position(|candidate| candidate.safety != RouteSafetyFlagV1::RejectUnlessNoAlternative)
        .or_else(|| (!candidates.is_empty()).then_some(0))
}

fn safety_sort_key(flag: RouteSafetyFlagV1) -> u8 {
    match flag {
        RouteSafetyFlagV1::Ok => 2,
        RouteSafetyFlagV1::RiskyButAllowed => 1,
        RouteSafetyFlagV1::RejectUnlessNoAlternative => 0,
    }
}
