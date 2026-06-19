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
    let startup = crate::ai::deck_startup_profile_v1::deck_startup_profile_v1(run_state);
    let deck_shape = crate::ai::deck_shape_v1::deck_shape_profile_v1(run_state);
    let block_plan = crate::ai::block_plan_profile_v1::block_plan_profile_v1(run_state);
    let run_debt = crate::ai::strategic::run_debt_ledger_v1(run_state);
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
                same_card_count: run_state
                    .master_deck
                    .iter()
                    .filter(|deck_card| deck_card.id == facts.card)
                    .count(),
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
        startup,
        deck_shape,
        block_plan,
        run_debt,
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
