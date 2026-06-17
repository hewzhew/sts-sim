use crate::content::cards::CardId;
use crate::runtime::combat::{CombatCard, CombatState};

pub(crate) fn has_external_payoff_opportunity(combat: &CombatState) -> bool {
    combat
        .meta
        .master_deck_snapshot
        .iter()
        .chain(combat.zones.hand.iter())
        .chain(combat.zones.draw_pile.iter())
        .chain(combat.zones.discard_pile.iter())
        .chain(combat.zones.exhaust_pile.iter())
        .chain(combat.zones.limbo.iter())
        .chain(combat.zones.queued_cards.iter().map(|queued| &queued.card))
        .any(|card| card_has_external_payoff_opportunity(card, combat))
}

fn card_has_external_payoff_opportunity(card: &CombatCard, combat: &CombatState) -> bool {
    if card_has_persistent_or_reward_payoff(card.id) {
        return true;
    }

    combat.entities.player.current_hp < combat.entities.player.max_hp
        && card_has_healing_payoff(card.id)
}

fn card_has_persistent_or_reward_payoff(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Feed
            | CardId::LessonLearned
            | CardId::HandOfGreed
            | CardId::RitualDagger
            | CardId::Alchemize
            | CardId::GeneticAlgorithm
            | CardId::Wish
    )
}

fn card_has_healing_payoff(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::BandageUp | CardId::Bite | CardId::Reaper | CardId::SelfRepair
    )
}
