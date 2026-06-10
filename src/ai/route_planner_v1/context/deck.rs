use crate::content::cards::{get_card_definition, CardId, CardTarget, CardType};
use crate::state::RunState;

use super::super::types::DeckRouteSummaryV1;

pub(super) fn build_deck_summary(run_state: &RunState) -> DeckRouteSummaryV1 {
    let mut summary = DeckRouteSummaryV1 {
        deck_size: run_state.master_deck.len(),
        starter_strikes: 0,
        starter_defends: 0,
        curses: 0,
        attacks: 0,
        skills: 0,
        powers: 0,
        frontload_damage_score: 0,
        block_score: 0,
        aoe_score: 0,
        scaling_score: 0,
        debuff_score: 0,
        draw_score: 0,
        energy_score: 0,
        key_upgrades_available: 0,
        important_cards_unupgraded: 0,
    };
    for card in &run_state.master_deck {
        let def = get_card_definition(card.id);
        summary.observes_card(card.id, def.card_type, card.upgrades);
        if def.card_type == CardType::Attack {
            summary.frontload_damage_score +=
                def.base_damage + def.upgrade_damage * i32::from(card.upgrades);
        }
        if def.base_block > 0 {
            summary.block_score += def.base_block + def.upgrade_block * i32::from(card.upgrades);
        }
        if def.target == CardTarget::AllEnemy || def.is_multi_damage {
            summary.aoe_score += 1;
        }
        if def.card_type == CardType::Power
            || matches!(
                card.id,
                CardId::DemonForm | CardId::Inflame | CardId::Metallicize | CardId::LimitBreak
            )
        {
            summary.scaling_score += 1;
        }
        if matches!(
            card.id,
            CardId::Bash
                | CardId::ThunderClap
                | CardId::Shockwave
                | CardId::Uppercut
                | CardId::Clothesline
                | CardId::Disarm
                | CardId::Intimidate
                | CardId::Blind
                | CardId::DarkShackles
        ) {
            summary.debuff_score += 1;
        }
        if matches!(
            card.id,
            CardId::PommelStrike | CardId::ShrugItOff | CardId::BattleTrance | CardId::Offering
        ) {
            summary.draw_score += 1;
        }
        if matches!(
            card.id,
            CardId::SeeingRed | CardId::Bloodletting | CardId::Offering | CardId::Berserk
        ) {
            summary.energy_score += 1;
        }
        if card.upgrades == 0
            && def.card_type != CardType::Curse
            && def.card_type != CardType::Status
        {
            summary.key_upgrades_available = summary.key_upgrades_available.saturating_add(1);
        }
    }
    summary
}
