mod context;
mod evaluators;
mod rank;
mod types;

#[cfg(test)]
mod tests;

use crate::content::relics::RelicId;
use crate::state::run::RunState;

pub use types::{
    BossRelicCandidate, BossRelicDecisionDiagnostics, RelicCompatibility, RelicJudgement,
};

pub fn decide(run_state: &RunState, relics: &[RelicId]) -> (usize, BossRelicDecisionDiagnostics) {
    let context = context::build_context(run_state);
    let mut candidates = relics
        .iter()
        .copied()
        .enumerate()
        .map(|(index, relic_id)| {
            let judgement = evaluators::evaluate_boss_relic(&context, relic_id);
            rank::candidate_from_judgement(index, relic_id, judgement)
        })
        .collect::<Vec<_>>();
    rank::sort_candidates(&mut candidates);

    let chosen_index = candidates
        .first()
        .map(|candidate| candidate.index)
        .unwrap_or(0);
    (
        chosen_index,
        BossRelicDecisionDiagnostics {
            chosen_index: Some(chosen_index),
            top_candidates: candidates,
        },
    )
}
