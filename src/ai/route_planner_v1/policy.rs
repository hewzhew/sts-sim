use crate::ai::route_window_facts::{
    build_route_path_family_from_target, RouteWindowCoverageKind, RouteWindowFactsConfig,
    RouteWindowPath,
};
use crate::state::core::EngineState;
use crate::state::RunState;

use super::context::build_route_decision_context_v1;
use super::features::{
    node_features, project_route_path_viability, summarize_route_path, summarize_route_path_family,
};
use super::needs::estimate_needs;
use super::risk::safety_flag;
use super::scorer::{route_reasons, route_value_factors, score_route_candidate};
use super::types::{
    NeedVectorV1, NodeFeaturesV1, RouteCandidateTraceV1, RouteCandidateViabilityV1,
    RouteDecisionContextV1, RouteDecisionTraceV1, RouteMoveKindV1, RoutePathSummaryV1,
    RoutePathViabilityV1, RoutePlannerConfigV1, RouteSafetyFlagV1, RouteScoreTermsV1,
    RouteValueFactorsV1, ROUTE_DECISION_TRACE_SCHEMA_NAME, ROUTE_DECISION_TRACE_SCHEMA_VERSION,
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
    let horizon_nodes = 15_usize.saturating_sub(target.y.max(0) as usize);
    let family = build_route_path_family_from_target(
        run_state,
        target.x,
        target.y,
        RouteWindowFactsConfig {
            horizon_nodes,
            path_budget: config.path_budget,
        },
    );
    let path_summary = summarize_route_path_family(&family);
    let features = node_features(
        target,
        &context.counters.unknown_belief,
        context.hp,
        context.max_hp,
        context.potions.filled < context.potions.slots,
        context.relics.has_cursed_key,
        config,
    );
    let mut path_evaluations = family
        .paths
        .iter()
        .enumerate()
        .map(|(path_index, path)| {
            evaluate_route_path(
                path_index,
                path,
                family.paths.len(),
                &path_summary,
                &features,
                context,
                needs,
                config,
                target,
            )
        })
        .collect::<Vec<_>>();
    sort_path_evaluations(&mut path_evaluations);
    let coverage_complete = family.coverage.kind == RouteWindowCoverageKind::CompleteWithinHorizon;
    let surviving_path_count = path_evaluations
        .iter()
        .filter(|evaluation| evaluation.viability.survives_projected_segment)
        .count();
    let representative = path_evaluations.first();
    let representative_path_summary = representative
        .map(|evaluation| evaluation.path_summary.clone())
        .unwrap_or_else(|| path_summary.clone());
    let value_factors = representative
        .map(|evaluation| evaluation.value_factors.clone())
        .unwrap_or_else(|| {
            route_value_factors(
                &features,
                &representative_path_summary,
                &path_summary,
                family.paths.len(),
                features.expected_hp_loss_p90,
                target.move_kind,
                context.counters.emerald_key_taken,
                context.relics.has_cursed_key,
                config,
            )
        });
    let score_terms = representative
        .map(|evaluation| evaluation.score_terms.clone())
        .unwrap_or_else(|| score_route_candidate(&value_factors, needs, config));
    let mut safety = representative
        .map(|evaluation| evaluation.safety)
        .unwrap_or(RouteSafetyFlagV1::RejectUnlessNoAlternative);
    if surviving_path_count == 0 && !coverage_complete {
        safety = RouteSafetyFlagV1::RiskyButAllowed;
    }
    let (reasons, mut cautions) = route_reasons(&features, &path_summary, safety);
    if surviving_path_count == 0 {
        cautions.push(if coverage_complete {
            "no visible continuation survives cumulative p90 HP pressure".to_string()
        } else {
            "no observed continuation survives cumulative p90 HP pressure; projection is incomplete"
                .to_string()
        });
    }
    let viability = RouteCandidateViabilityV1 {
        coverage_complete,
        observed_path_count: family.paths.len(),
        surviving_path_count,
        representative_path_index: representative.map(|evaluation| evaluation.path_index),
        representative: representative.map(|evaluation| evaluation.viability.clone()),
        representative_path_summary: representative
            .map(|evaluation| evaluation.path_summary.clone()),
    };
    RouteCandidateTraceV1 {
        target: target.clone(),
        features,
        path_summary,
        viability,
        needs: needs.clone(),
        value_factors,
        total_score: representative
            .map(|evaluation| evaluation.total_score)
            .unwrap_or_else(|| score_terms.total()),
        score_terms,
        safety,
        reasons,
        cautions,
        suggested_command: engine_state
            .is_map_surface()
            .then(|| route_command_hint(target.move_kind, target.x, target.y)),
    }
}

struct RoutePathEvaluation {
    path_index: usize,
    path_summary: RoutePathSummaryV1,
    viability: RoutePathViabilityV1,
    value_factors: RouteValueFactorsV1,
    score_terms: RouteScoreTermsV1,
    total_score: f32,
    safety: RouteSafetyFlagV1,
}

#[allow(clippy::too_many_arguments)]
fn evaluate_route_path(
    path_index: usize,
    path: &RouteWindowPath,
    family_path_count: usize,
    family_summary: &RoutePathSummaryV1,
    features: &NodeFeaturesV1,
    context: &RouteDecisionContextV1,
    needs: &NeedVectorV1,
    config: &RoutePlannerConfigV1,
    target: &super::types::MapRouteTargetV1,
) -> RoutePathEvaluation {
    let path_summary = summarize_route_path(path);
    let viability =
        project_route_path_viability(path, context.hp, &context.counters.unknown_belief, config);
    let value_factors = route_value_factors(
        features,
        &path_summary,
        family_summary,
        family_path_count,
        viability.cumulative_hp_loss_p90,
        target.move_kind,
        context.counters.emerald_key_taken,
        context.relics.has_cursed_key,
        config,
    );
    let score_terms = score_route_candidate(&value_factors, needs, config);
    let total_score = score_terms.total();
    let safety = safety_flag(features, &path_summary, needs, &viability);
    RoutePathEvaluation {
        path_index,
        path_summary,
        viability,
        value_factors,
        score_terms,
        total_score,
        safety,
    }
}

fn sort_path_evaluations(evaluations: &mut [RoutePathEvaluation]) {
    evaluations.sort_by(|a, b| {
        safety_sort_key(b.safety)
            .cmp(&safety_sort_key(a.safety))
            .then_with(|| {
                b.total_score
                    .partial_cmp(&a.total_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.path_index.cmp(&b.path_index))
    });
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
