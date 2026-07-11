use super::*;
use crate::runtime::combat::CombatCard;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CardPileValueV1 {
    pub(super) damage: i32,
    pub(super) block: i32,
    pub(super) playable_cards: i32,
    pub(super) low_cost: i32,
}

pub(super) fn hand_value(combat: &CombatState) -> CardPileValueV1 {
    let mut value = card_pile_value(combat.zones.hand.iter(), combat.turn.energy as i32);
    if let Some(capacity) = remaining_card_play_capacity(combat) {
        value.playable_cards = value.playable_cards.min(capacity as i32);
    }
    value
}

pub(super) fn next_draw_value(combat: &CombatState) -> CardPileValueV1 {
    let requested =
        crate::engine::core::compute_player_turn_start_draw_count(combat).max(0) as usize;
    let retained = projected_retained_hand_count(combat);
    let hand_capacity = 10usize.saturating_sub(retained);
    let draw_count = requested
        .min(hand_capacity)
        .min(combat.zones.draw_pile.len());
    card_pile_value(
        combat.zones.draw_pile.iter().take(draw_count),
        combat.entities.player.energy_master as i32,
    )
}

fn remaining_card_play_capacity(combat: &CombatState) -> Option<usize> {
    combat
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::VelvetChoker)
        .then(|| 6usize.saturating_sub(combat.turn.counters.cards_played_this_turn as usize))
}

fn projected_retained_hand_count(combat: &CombatState) -> usize {
    let has_pyramid = combat
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::RunicPyramid);
    combat
        .zones
        .hand
        .iter()
        .filter(|card| {
            let explicitly_retained =
                card.retain_override == Some(true) || crate::content::cards::is_self_retain(card);
            explicitly_retained || (has_pyramid && !crate::content::cards::is_ethereal(card))
        })
        .count()
}

pub(super) fn card_pile_value_report(value: CardPileValueV1) -> CombatSearchV2CardPileValueReport {
    CombatSearchV2CardPileValueReport {
        damage: value.damage,
        block: value.block,
        playable_cards: value.playable_cards,
        low_cost: value.low_cost,
    }
}

fn card_pile_value<'a>(
    cards: impl Iterator<Item = &'a CombatCard>,
    playable_energy: i32,
) -> CardPileValueV1 {
    cards.fold(CardPileValueV1::default(), |mut value, card| {
        let def = crate::content::cards::get_card_definition(card.id);
        let cost = card.cost_for_turn_java();
        if cost >= 0 && cost <= playable_energy {
            value.playable_cards += 1;
        }
        value.low_cost -= cost.max(0);
        value.damage += card
            .base_damage_override
            .unwrap_or(def.base_damage + def.upgrade_damage * card.upgrades as i32)
            .max(0);
        value.block += card
            .base_block_override
            .unwrap_or(def.base_block + def.upgrade_block * card.upgrades as i32)
            .max(0);
        value
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::test_support::blank_test_combat;

    #[test]
    fn next_draw_value_uses_turn_start_draw_modifier_and_next_turn_energy() {
        let mut combat = blank_test_combat();
        combat.turn.energy = 0;
        combat.entities.player.energy_master = 3;
        combat.turn.turn_start_draw_modifier = -4;
        combat.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 11),
            CombatCard::new(CardId::Carnage, 12),
        ];

        let value = next_draw_value(&combat);

        assert_eq!(value.damage, 6);
        assert_eq!(value.playable_cards, 1);
    }

    #[test]
    fn hand_value_uses_current_turn_energy_for_playability() {
        let mut combat = blank_test_combat();
        combat.turn.energy = 1;
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 11),
            CombatCard::new(CardId::Carnage, 12),
        ];

        let value = hand_value(&combat);

        assert_eq!(value.damage, 26);
        assert_eq!(value.playable_cards, 1);
    }

    #[test]
    fn hand_playable_count_obeys_remaining_velvet_choker_slots() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .player
            .add_relic(RelicState::new(RelicId::VelvetChoker));
        combat.turn.energy = 3;
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 11),
            CombatCard::new(CardId::Strike, 12),
            CombatCard::new(CardId::Strike, 13),
        ];

        combat.turn.counters.cards_played_this_turn = 5;
        assert_eq!(hand_value(&combat).playable_cards, 1);

        combat.turn.counters.cards_played_this_turn = 6;
        assert_eq!(hand_value(&combat).playable_cards, 0);
    }

    #[test]
    fn pyramid_retained_hand_caps_next_turn_draw() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .player
            .add_relic(RelicState::new(RelicId::RunicPyramid));
        combat.zones.hand = (0..8)
            .map(|index| CombatCard::new(CardId::Defend, 100 + index))
            .collect();
        combat.zones.draw_pile = (0..5)
            .map(|index| CombatCard::new(CardId::Strike, 200 + index))
            .collect();

        let value = next_draw_value(&combat);

        assert_eq!(value.playable_cards, 2);
        assert_eq!(value.damage, 12);
    }

    #[test]
    fn ethereal_apparitions_release_pyramid_draw_capacity_unless_explicitly_retained() {
        let mut combat = blank_test_combat();
        combat
            .entities
            .player
            .add_relic(RelicState::new(RelicId::RunicPyramid));
        combat.zones.hand = (0..5)
            .map(|index| CombatCard::new(CardId::Defend, 100 + index))
            .chain((0..3).map(|index| CombatCard::new(CardId::Apparition, 200 + index)))
            .collect();
        combat.zones.draw_pile = (0..5)
            .map(|index| CombatCard::new(CardId::Strike, 300 + index))
            .collect();

        assert_eq!(next_draw_value(&combat).damage, 30);

        combat.zones.hand[5].retain_override = Some(true);
        assert_eq!(next_draw_value(&combat).damage, 24);
    }
}
