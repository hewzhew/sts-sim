use crate::bot::card_structure::{self, CardStructure};
use crate::content::cards::CardId;

// Compatibility layer: older callers still ask taxonomy-flavored questions, but the answers now
// come straight from structural tags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CardTaxonomy {
    structure: CardStructure,
}

fn taxonomy(card_id: CardId) -> CardTaxonomy {
    CardTaxonomy {
        structure: card_structure::structure(card_id),
    }
}

pub(crate) fn is_strength_enabler(card_id: CardId) -> bool {
    taxonomy(card_id).structure.is_strength_enabler()
}

pub(crate) fn is_strength_payoff(card_id: CardId) -> bool {
    taxonomy(card_id).structure.is_strength_payoff()
}

pub(crate) fn is_multi_attack_payoff(card_id: CardId) -> bool {
    taxonomy(card_id).structure.is_multi_attack_payoff()
}
