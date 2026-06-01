use super::*;
use crate::runtime::combat::CombatCard;

const BASE_TURN_DRAW_COUNT: i32 = 5;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CardPileValueV1 {
    pub(super) damage: i32,
    pub(super) block: i32,
    pub(super) playable_cards: i32,
    pub(super) low_cost: i32,
}

pub(super) fn hand_value(combat: &CombatState) -> CardPileValueV1 {
    card_pile_value(combat.zones.hand.iter(), combat.turn.energy as i32)
}

pub(super) fn next_draw_value(combat: &CombatState) -> CardPileValueV1 {
    let draw_count = (BASE_TURN_DRAW_COUNT + combat.turn.turn_start_draw_modifier)
        .max(0)
        .min(combat.zones.draw_pile.len() as i32) as usize;
    card_pile_value(
        combat.zones.draw_pile.iter().take(draw_count),
        combat.entities.player.energy_master as i32,
    )
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
}
