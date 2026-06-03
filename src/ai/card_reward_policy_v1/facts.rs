use crate::content::cards::{get_card_definition, CardId, CardRarity, CardTarget, CardType};
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

pub(crate) fn draw_value(card_id: CardId) -> f32 {
    match card_id {
        CardId::Offering => 3.2,
        CardId::BattleTrance | CardId::MasterOfStrategy => 2.4,
        CardId::PommelStrike | CardId::ShrugItOff | CardId::BurningPact => 1.5,
        CardId::Warcry | CardId::Finesse | CardId::FlashOfSteel => 1.0,
        _ => 0.0,
    }
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

pub(crate) fn scaling_value(card_id: CardId) -> f32 {
    match card_id {
        CardId::Corruption | CardId::DemonForm | CardId::Barricade => 3.0,
        CardId::Shockwave | CardId::Disarm => 2.6,
        CardId::Inflame | CardId::SpotWeakness | CardId::FeelNoPain | CardId::DarkEmbrace => 2.3,
        CardId::Metallicize | CardId::LimitBreak | CardId::Evolve | CardId::FireBreathing => 1.8,
        CardId::Berserk | CardId::Rupture | CardId::Juggernaut => 1.4,
        _ => 0.0,
    }
}

pub(crate) fn rarity_value(card_id: CardId) -> f32 {
    match get_card_definition(card_id).rarity {
        CardRarity::Rare => 1.0,
        CardRarity::Uncommon => 0.55,
        CardRarity::Common => 0.15,
        CardRarity::Basic | CardRarity::Special | CardRarity::Curse => 0.0,
    }
}

pub(crate) fn premium_value(card_id: CardId) -> f32 {
    match card_id {
        CardId::Offering => 5.0,
        CardId::Shockwave => 4.8,
        CardId::Immolate | CardId::Disarm => 4.2,
        CardId::Feed | CardId::Reaper | CardId::Corruption => 3.6,
        CardId::Impervious | CardId::FiendFire | CardId::BattleTrance => 3.1,
        CardId::FlameBarrier | CardId::Uppercut | CardId::Carnage | CardId::PowerThrough => 2.4,
        CardId::ShrugItOff | CardId::PommelStrike | CardId::Armaments => 1.4,
        _ => 0.0,
    }
}

pub(crate) fn risk_penalty(card_id: CardId, needs: &DeckNeedsV1) -> f32 {
    match card_id {
        CardId::Clash => -3.0,
        CardId::PerfectedStrike if needs.deck_size < 13 => -1.0,
        CardId::DemonForm if needs.deck_size < 13 => -0.8,
        CardId::Barricade if needs.need_block > 0.7 => -0.8,
        CardId::BodySlam if needs.need_block > 0.7 => -1.2,
        _ => 0.0,
    }
}

pub(crate) fn is_aoe_card(card_id: CardId) -> bool {
    let def = get_card_definition(card_id);
    def.target == CardTarget::AllEnemy || def.is_multi_damage
}
