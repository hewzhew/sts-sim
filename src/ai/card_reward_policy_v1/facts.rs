use crate::content::cards::{get_card_definition, CardId, CardTarget, CardType};
use crate::state::run::RunState;

use super::types::{CardRewardPolicyConfigV1, DeckNeedsV1};

pub(crate) fn deck_needs(run_state: &RunState, config: &CardRewardPolicyConfigV1) -> DeckNeedsV1 {
    let mut damage = 0i32;
    let mut block = 0i32;
    let mut draw = 0u8;
    let mut scaling = 0u8;
    let mut starter_cards = 0u8;
    let mut exhaust_payoff = false;

    for card in &run_state.master_deck {
        let def = get_card_definition(card.id);
        let upgrades = i32::from(card.upgrades);
        if def.card_type == CardType::Attack {
            damage += def.base_damage + def.upgrade_damage * upgrades;
        }
        if def.base_block > 0 {
            block += def.base_block + def.upgrade_block * upgrades;
        }
        if is_draw_card(card.id) {
            draw = draw.saturating_add(1);
        }
        if is_scaling_card(card.id) {
            scaling = scaling.saturating_add(1);
        }
        if matches!(card.id, CardId::Strike | CardId::Defend | CardId::Bash) {
            starter_cards = starter_cards.saturating_add(1);
        }
        if matches!(
            card.id,
            CardId::FeelNoPain | CardId::DarkEmbrace | CardId::Corruption
        ) {
            exhaust_payoff = true;
        }
    }

    let deck_size = run_state.master_deck.len();
    DeckNeedsV1 {
        deck_size,
        need_frontload: if damage < 50 || starter_cards >= 8 {
            1.0
        } else if damage < 75 {
            0.65
        } else {
            0.30
        },
        need_block: if block < 28 {
            0.85
        } else if block < 45 {
            0.50
        } else {
            0.25
        },
        need_draw: if draw == 0 {
            0.90
        } else if draw <= 2 {
            0.55
        } else {
            0.25
        },
        need_scaling: if scaling == 0 { 0.80 } else { 0.35 },
        has_exhaust_payoff: exhaust_payoff,
        is_late_deck: deck_size >= config.late_deck_size,
    }
}

pub(crate) fn effective_cost(card_id: CardId) -> f32 {
    let cost = get_card_definition(card_id).cost;
    match cost {
        -2 => 4.0,
        -1 => 2.0,
        0 => 0.75,
        n => f32::from(n.max(1)),
    }
}

pub(crate) fn is_draw_card(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::PommelStrike
            | CardId::ShrugItOff
            | CardId::BattleTrance
            | CardId::Offering
            | CardId::BurningPact
            | CardId::Warcry
            | CardId::MasterOfStrategy
            | CardId::Finesse
            | CardId::FlashOfSteel
    )
}

pub(crate) fn is_scaling_card(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Inflame
            | CardId::SpotWeakness
            | CardId::DemonForm
            | CardId::LimitBreak
            | CardId::Metallicize
            | CardId::Barricade
            | CardId::Corruption
            | CardId::FeelNoPain
            | CardId::DarkEmbrace
            | CardId::Evolve
            | CardId::FireBreathing
            | CardId::Rupture
            | CardId::Juggernaut
            | CardId::Berserk
            | CardId::Shockwave
            | CardId::Disarm
    )
}

pub(crate) fn is_aoe_card(card_id: CardId) -> bool {
    let def = get_card_definition(card_id);
    def.target == CardTarget::AllEnemy && def.is_multi_damage
}
