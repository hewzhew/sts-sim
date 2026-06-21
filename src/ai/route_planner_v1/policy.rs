use crate::state::core::EngineState;
use crate::state::RunState;

use super::context::build_route_decision_context_v1;
use super::features::{node_features, summarize_route_from};
use super::needs::estimate_needs;
use super::risk::safety_flag;
use super::scorer::{route_reasons, route_value_factors, score_route_candidate};
use super::types::{
    NeedVectorV1, RouteCandidateTraceV1, RouteDecisionContextV1, RouteDecisionTraceV1,
    RouteMoveKindV1, RoutePlannerConfigV1, RouteSafetyFlagV1, ROUTE_DECISION_TRACE_SCHEMA_NAME,
    ROUTE_DECISION_TRACE_SCHEMA_VERSION,
};

pub fn plan_route_decision_v1(
    run_state: &RunState,
    engine_state: &EngineState,
    config: RoutePlannerConfigV1,
) -> RouteDecisionTraceV1 {
    let context = build_route_decision_context_v1(run_state);
    let needs = estimate_needs(&context, &config);
    let mut candidates =
        build_route_candidate_traces_v1(run_state, engine_state, &context, &needs, &config);
    sort_route_candidates_v1(&mut candidates);
    let selected_index = selected_index(&candidates);
    let mut warnings = Vec::new();
    if !engine_state.is_map_surface() {
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
        path_budget: config.path_budget,
        candidates,
        selected_index,
        warnings,
    }
}

fn build_route_candidate_traces_v1(
    run_state: &RunState,
    engine_state: &EngineState,
    context: &RouteDecisionContextV1,
    needs: &NeedVectorV1,
    config: &RoutePlannerConfigV1,
) -> Vec<RouteCandidateTraceV1> {
    context
        .legal_next_nodes
        .iter()
        .map(|target| {
            build_route_candidate_trace_v1(run_state, engine_state, context, needs, config, target)
        })
        .collect()
}

fn build_route_candidate_trace_v1(
    run_state: &RunState,
    engine_state: &EngineState,
    context: &RouteDecisionContextV1,
    needs: &NeedVectorV1,
    config: &RoutePlannerConfigV1,
    target: &super::types::MapRouteTargetV1,
) -> RouteCandidateTraceV1 {
    let path_summary = summarize_route_from(run_state, target.x, target.y, config);
    let features = node_features(
        target,
        &context.counters.unknown_belief,
        context.hp,
        context.max_hp,
        context.potions.filled < context.potions.slots,
        context.relics.has_cursed_key,
        config,
    );
    let value_factors = route_value_factors(
        &features,
        &path_summary,
        target.move_kind,
        context.counters.emerald_key_taken,
        context.relics.has_cursed_key,
        config,
    );
    let score_terms = score_route_candidate(&value_factors, needs, config);
    let safety = safety_flag(&features, &path_summary, needs);
    let (reasons, cautions) = route_reasons(&features, &path_summary, safety);
    RouteCandidateTraceV1 {
        target: target.clone(),
        features,
        path_summary,
        needs: needs.clone(),
        value_factors,
        total_score: score_terms.total(),
        score_terms,
        safety,
        reasons,
        cautions,
        suggested_command: engine_state
            .is_map_surface()
            .then(|| route_command_hint(target.move_kind, target.x, target.y)),
    }
}

fn sort_route_candidates_v1(candidates: &mut [RouteCandidateTraceV1]) {
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

fn route_command_hint(move_kind: RouteMoveKindV1, x: i32, y: i32) -> String {
    match move_kind {
        RouteMoveKindV1::NormalEdge => format!("go {x}"),
        RouteMoveKindV1::WingBootsJump => format!("fly {x} {y}"),
    }
}
