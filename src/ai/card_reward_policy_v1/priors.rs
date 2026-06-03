use crate::content::cards::{get_card_definition, CardId, CardRarity};

use super::types::DeckNeedsV1;

pub(crate) fn draw_value(card_id: CardId) -> f32 {
    match card_id {
        CardId::Offering => 3.2,
        CardId::BattleTrance | CardId::MasterOfStrategy => 2.4,
        CardId::PommelStrike | CardId::ShrugItOff | CardId::BurningPact => 1.5,
        CardId::Warcry | CardId::Finesse | CardId::FlashOfSteel => 1.0,
        _ => 0.0,
    }
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
