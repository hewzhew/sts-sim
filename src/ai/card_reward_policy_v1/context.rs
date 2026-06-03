use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

use super::facts::card_facts;
use super::impact::candidate_impact;
use super::profile::{deck_profile, route_evidence, run_context};
use super::types::{CardRewardCandidateEvidenceV1, CardRewardDecisionContextV1};

pub fn build_card_reward_decision_context_v1(
    run_state: &RunState,
    cards: Vec<RewardCard>,
    route_trace: Option<&crate::ai::route_planner_v1::RouteDecisionTraceV1>,
) -> CardRewardDecisionContextV1 {
    let deck = deck_profile(run_state);
    let route = route_evidence(route_trace);
    let candidates = cards
        .into_iter()
        .enumerate()
        .map(|(index, card)| {
            let facts = card_facts(&card);
            let impact = candidate_impact(&facts, &deck, route.as_ref());
            CardRewardCandidateEvidenceV1 {
                index,
                card: facts.card,
                name: facts.name,
                card_type: facts.card_type,
                facts,
                impact,
            }
        })
        .collect();

    CardRewardDecisionContextV1 {
        run: run_context(run_state),
        deck,
        route,
        candidates,
    }
}
