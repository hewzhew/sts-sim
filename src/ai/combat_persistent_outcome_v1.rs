use std::collections::HashMap;

use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::relics::RelicId;
use crate::runtime::combat::{CombatState, MetaChange};
use crate::state::rewards::RewardItem;

/// Exact persistent combat outcomes that must survive tactical witness search.
///
/// This is a fact vector, not a run-level value function. Callers may retain a
/// Pareto frontier with it, then apply their own continuation context when one
/// exact witness must be selected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CombatPersistentOutcomeV1 {
    pub max_hp: i32,
    pub recoverable_gold_delta: i32,
    pub ritual_dagger_value: i32,
    pub genetic_algorithm_value: i32,
    pub external_burden_count: i32,
}

impl CombatPersistentOutcomeV1 {
    pub fn from_combat(combat: &CombatState) -> Self {
        Self {
            max_hp: combat.entities.player.max_hp,
            recoverable_gold_delta: recoverable_gold_delta(combat),
            ritual_dagger_value: persistent_card_value(combat, CardId::RitualDagger),
            genetic_algorithm_value: persistent_card_value(combat, CardId::GeneticAlgorithm),
            external_burden_count: external_burden_count(combat),
        }
    }

    pub fn dominates_or_equals(self, other: Self) -> bool {
        self.max_hp >= other.max_hp
            && self.recoverable_gold_delta >= other.recoverable_gold_delta
            && self.ritual_dagger_value >= other.ritual_dagger_value
            && self.genetic_algorithm_value >= other.genetic_algorithm_value
            && self.external_burden_count <= other.external_burden_count
    }

    pub fn strictly_dominates(self, other: Self) -> bool {
        self.dominates_or_equals(other) && self != other
    }
}

pub fn recoverable_stolen_gold(combat: &CombatState) -> i32 {
    combat
        .runtime
        .pending_rewards
        .iter()
        .filter_map(|reward| match reward {
            RewardItem::StolenGold { amount } => Some(*amount),
            _ => None,
        })
        .fold(0i32, i32::saturating_add)
}

pub fn recoverable_gold_delta(combat: &CombatState) -> i32 {
    combat
        .entities
        .player
        .gold_delta_this_combat
        .saturating_add(recoverable_stolen_gold(combat))
}

pub fn persistent_card_value(combat: &CombatState, card_id: CardId) -> i32 {
    let mut misc_delta_by_uuid = HashMap::<u32, i32>::new();
    for change in &combat.meta.meta_changes {
        if let MetaChange::ModifyCardMisc { card_uuid, amount } = change {
            let delta = misc_delta_by_uuid.entry(*card_uuid).or_default();
            *delta = delta.saturating_add(*amount);
        }
    }
    combat
        .meta
        .master_deck_snapshot
        .iter()
        .filter(|card| card.id == card_id)
        .map(|card| {
            card.misc_value
                .saturating_add(
                    misc_delta_by_uuid
                        .get(&card.uuid)
                        .copied()
                        .unwrap_or_default(),
                )
                .max(0)
        })
        .sum()
}

pub fn external_burden_count(combat: &CombatState) -> i32 {
    let curse_additions = combat
        .meta
        .meta_changes
        .iter()
        .filter(|change| {
            matches!(
                change,
                MetaChange::AddCardToMasterDeck(card_id)
                    if get_card_definition(*card_id).card_type == CardType::Curse
            )
        })
        .count() as i32;
    let omamori_charges = combat
        .entities
        .player
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::Omamori && !relic.used_up)
        .map(|relic| relic.counter.max(0))
        .unwrap_or_default();
    curse_additions.saturating_sub(omamori_charges).max(0)
}
