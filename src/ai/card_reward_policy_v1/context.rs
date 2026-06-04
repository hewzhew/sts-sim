use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

use super::facts::card_facts;
use super::impact::candidate_impact;
use super::profile::{
    deck_profile, route_evidence, run_context, strategy_candidate_facts, strategy_deck_facts,
    strategy_route_future,
};
use super::types::{CardRewardCandidateEvidenceV1, CardRewardDecisionContextV1};

pub fn build_card_reward_decision_context_v1(
    run_state: &RunState,
    cards: Vec<RewardCard>,
    route_trace: Option<&crate::ai::route_planner_v1::RouteDecisionTraceV1>,
) -> CardRewardDecisionContextV1 {
    let deck = deck_profile(run_state);
    let route = route_evidence(route_trace);
    let plans = crate::ai::noncombat_strategy_v1::build_run_strategy_snapshot_v1(
        strategy_deck_facts(&deck),
        strategy_route_future(route.as_ref()),
    );
    let candidates = cards
        .into_iter()
        .enumerate()
        .map(|(index, card)| {
            let facts = card_facts(&card);
            let impact = candidate_impact(&facts, &deck, route.as_ref());
            let plan_delta = crate::ai::noncombat_strategy_v1::candidate_plan_delta_v1(
                strategy_candidate_facts(&facts),
                &plans,
            );
            CardRewardCandidateEvidenceV1 {
                index,
                card: facts.card,
                name: facts.name,
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
        plans,
        candidates,
    }
}
