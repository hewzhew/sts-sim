use crate::ai::analysis::card_semantics::{card_definition, CardDefinition, DeckMechanicContext};
use crate::ai::strategy::package_state::{assess_package_state, PackageStateReport};
use crate::ai::strategy::package_transition::{assess_package_transition, PackageTransitionReport};
use crate::content::cards::CardId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardSemanticProbeReport {
    pub deck_package: PackageStateReport,
    pub candidates: Vec<RewardCandidateSemanticProbe>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardCandidateSemanticProbe {
    pub card: CardId,
    pub transition: PackageTransitionReport,
}

pub fn assess_reward_semantics(
    deck: &[CardDefinition],
    candidates: &[CardDefinition],
) -> RewardSemanticProbeReport {
    let deck_context = DeckMechanicContext::from_definitions(deck);
    RewardSemanticProbeReport {
        deck_package: assess_package_state(&deck_context),
        candidates: candidates
            .iter()
            .cloned()
            .map(|candidate| RewardCandidateSemanticProbe {
                card: candidate.card,
                transition: assess_package_transition(deck, candidate),
            })
            .collect(),
    }
}

pub fn assess_reward_semantics_from_cards(
    deck: &[CardId],
    candidates: &[CardId],
) -> RewardSemanticProbeReport {
    let deck_definitions = deck
        .iter()
        .copied()
        .map(card_definition)
        .collect::<Vec<_>>();
    let candidate_definitions = candidates
        .iter()
        .copied()
        .map(card_definition)
        .collect::<Vec<_>>();
    assess_reward_semantics(&deck_definitions, &candidate_definitions)
}
