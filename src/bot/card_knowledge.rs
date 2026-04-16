use crate::bot::card_facts::facts as card_facts;
use crate::bot::card_structure::structure as card_structure;
use crate::content::cards::CardId;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NoncombatKnowledgeBridge {
    pub draws_cards: bool,
    pub gains_energy: bool,
    pub strength_enabler: bool,
    pub strength_payoff: bool,
    pub exhaust_engine: bool,
    pub exhaust_outlet: bool,
    pub block_core: bool,
    pub block_payoff: bool,
    pub status_engine: bool,
    pub control_tool: bool,
    pub combat_heal: bool,
}

#[allow(dead_code)]
pub(crate) fn noncombat_bridge(card_id: CardId) -> NoncombatKnowledgeBridge {
    let facts = card_facts(card_id);
    let structure = card_structure(card_id);

    NoncombatKnowledgeBridge {
        draws_cards: facts.draws_cards || structure.is_draw_core(),
        gains_energy: facts.gains_energy || structure.is_resource_conversion(),
        strength_enabler: structure.is_strength_enabler(),
        strength_payoff: structure.is_strength_payoff() || structure.is_multi_attack_payoff(),
        exhaust_engine: structure.is_exhaust_engine(),
        exhaust_outlet: structure.is_exhaust_outlet(),
        block_core: structure.is_block_core(),
        block_payoff: structure.is_block_payoff(),
        status_engine: structure.is_status_engine(),
        control_tool: facts.applies_weak || facts.applies_vuln,
        combat_heal: facts.combat_heal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_exposes_descriptive_noncombat_signals() {
        let corruption = noncombat_bridge(CardId::Corruption);
        assert!(corruption.exhaust_engine);
        assert!(!corruption.control_tool);

        let shockwave = noncombat_bridge(CardId::Shockwave);
        assert!(shockwave.control_tool);
        assert!(!shockwave.draws_cards);
    }
}
