use crate::state::core::EngineState;
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

use super::facts::card_facts;
use super::impact::candidate_impact;
use super::profile::{
    deck_profile, route_evidence, run_context, strategy_candidate_facts, strategy_route_future,
};
use super::types::{CardRewardCandidateEvidenceV1, CardRewardDecisionContextV1};

pub fn build_card_reward_decision_context_v1(
    run_state: &RunState,
    cards: Vec<RewardCard>,
    route_trace: Option<&crate::ai::route_planner_v1::RouteDecisionTraceV1>,
) -> CardRewardDecisionContextV1 {
    let deck = deck_profile(run_state);
    let route = route_evidence(route_trace);
    let strategy =
        crate::ai::noncombat_strategy_v1::build_run_strategy_snapshot_from_run_state_with_route_v2(
            run_state,
            strategy_route_future(route.as_ref()),
        );
    let candidates = cards
        .into_iter()
        .enumerate()
        .map(|(index, card)| {
            let facts = card_facts(&card);
            let impact = candidate_impact(&facts, &deck, route.as_ref());
            let plan_delta = crate::ai::noncombat_strategy_v1::candidate_plan_delta_v2(
                strategy_candidate_facts(&facts),
                &strategy,
            );
            let name = facts.name.clone();
            CardRewardCandidateEvidenceV1 {
                index,
                card: facts.card,
                name,
                card_type: facts.card_type,
                facts,
                impact,
                plan_delta,
            }
        })
        .collect();

    CardRewardDecisionContextV1 {
        run: run_context(run_state),
        deck,
        route,
        strategy,
        has_singing_bowl: run_state
            .relics
            .iter()
            .any(|relic| relic.id == crate::content::relics::RelicId::SingingBowl),
        candidates,
    }
}

pub fn build_card_reward_decision_context_with_current_route_v1(
    run_state: &RunState,
    engine_state: &EngineState,
    cards: Vec<RewardCard>,
) -> CardRewardDecisionContextV1 {
    let route_trace = crate::ai::route_planner_v1::plan_route_decision_v1(
        run_state,
        engine_state,
        Default::default(),
    );
    let route_trace = (!route_trace.candidates.is_empty()).then_some(route_trace);
    build_card_reward_decision_context_v1(run_state, cards, route_trace.as_ref())
}
