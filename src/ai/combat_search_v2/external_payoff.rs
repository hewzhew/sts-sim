use crate::ai::card_semantics_v1::{card_mechanics_profile_v1, CombatExternalPayoffV1};
pub use crate::ai::combat_persistent_outcome_v1::{
    external_burden_count, recoverable_gold_delta, recoverable_stolen_gold,
    CombatPersistentOutcomeV1,
};
use crate::runtime::combat::{CombatCard, CombatState};

pub fn has_external_payoff_opportunity(combat: &CombatState) -> bool {
    combat_cards(combat).any(|card| card_has_external_payoff_opportunity(card, combat))
}

pub fn has_persistent_or_reward_payoff_opportunity(combat: &CombatState) -> bool {
    combat_cards(combat).any(|card| {
        matches!(
            card_mechanics_profile_v1(card.id).combat_external_payoff,
            Some(CombatExternalPayoffV1::PersistentOrReward)
        )
    })
}

pub fn has_healing_payoff_opportunity(combat: &CombatState) -> bool {
    combat.entities.player.current_hp < combat.entities.player.max_hp
        && combat_cards(combat).any(|card| {
            matches!(
                card_mechanics_profile_v1(card.id).combat_external_payoff,
                Some(CombatExternalPayoffV1::HealingIfDamaged)
            )
        })
}

fn combat_cards(combat: &CombatState) -> impl Iterator<Item = &CombatCard> {
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
}

fn card_has_external_payoff_opportunity(card: &CombatCard, combat: &CombatState) -> bool {
    match card_mechanics_profile_v1(card.id).combat_external_payoff {
        Some(CombatExternalPayoffV1::PersistentOrReward) => true,
        Some(CombatExternalPayoffV1::HealingIfDamaged) => {
            combat.entities.player.current_hp < combat.entities.player.max_hp
        }
        None => false,
    }
}

/// Returns the exact run value already materialized by this combat state.
///
/// This is shared by the legacy search and the production portfolio so that
/// selecting between verified witnesses cannot silently discard gold, max HP,
/// or persistent card growth while comparing only final combat HP.
pub fn persistent_run_value(combat: &CombatState) -> i32 {
    let outcome = CombatPersistentOutcomeV1::from_combat(combat);
    outcome.max_hp
        + outcome.recoverable_gold_delta.saturating_div(5)
        + outcome.ritual_dagger_value
        + outcome.genetic_algorithm_value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::rewards::RewardItem;

    #[test]
    fn killed_thief_stolen_gold_counts_as_recoverable_run_value() {
        let mut escaped = crate::test_support::blank_test_combat();
        escaped.entities.player.max_hp = 85;
        escaped.entities.player.gold_delta_this_combat = -75;
        let mut killed = escaped.clone();
        killed
            .runtime
            .pending_rewards
            .push(RewardItem::StolenGold { amount: 75 });

        assert_eq!(persistent_run_value(&escaped), 70);
        assert_eq!(persistent_run_value(&killed), 85);
    }
}
