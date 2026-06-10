use crate::content::cards::{get_card_definition, CardId, CardTag, CardType};
use crate::content::relics::RelicId;
use crate::runtime::combat::CombatCard;
use crate::state::run::RunState;
use crate::state::shop::ShopState;

pub fn shop_conversion_pressure_v1(run_state: &RunState, shop: &ShopState) -> bool {
    let gold = run_state.gold;
    if gold >= 300 {
        return true;
    }
    if shop.purge_available && gold >= shop.purge_cost {
        let removable_curse = run_state
            .master_deck
            .iter()
            .any(|card| purgeable_curse(card, run_state));
        let starter_cards = run_state
            .master_deck
            .iter()
            .filter(|card| starter_card(card))
            .count();
        if removable_curse || (gold >= 250 && starter_cards >= 6) {
            return true;
        }
    }
    run_state.act_num >= 2 && gold >= 250
}

pub fn shop_card_conversion_priority_v1(card: CardId, run_state: &RunState) -> i32 {
    let mut priority = 250;
    if high_impact_shop_card(card) {
        priority += 450;
    }
    if run_state.act_num >= 2 && boss_or_elite_patch_card(card) {
        priority += 200;
    }
    priority
}

pub fn shop_relic_conversion_priority_v1(relic: RelicId) -> i32 {
    if high_impact_shop_relic(relic) {
        950
    } else {
        720
    }
}

pub fn shop_potion_conversion_priority_v1(run_state: &RunState) -> i32 {
    if run_state.act_num >= 2 {
        680
    } else {
        520
    }
}

fn high_impact_shop_card(card: CardId) -> bool {
    matches!(
        card,
        CardId::Shockwave
            | CardId::Disarm
            | CardId::Uppercut
            | CardId::ShrugItOff
            | CardId::FlameBarrier
            | CardId::Impervious
            | CardId::Offering
            | CardId::BattleTrance
            | CardId::PommelStrike
            | CardId::TrueGrit
            | CardId::BurningPact
            | CardId::PowerThrough
            | CardId::FeelNoPain
            | CardId::DarkEmbrace
            | CardId::Corruption
            | CardId::SecondWind
            | CardId::FiendFire
            | CardId::DemonForm
    )
}

fn boss_or_elite_patch_card(card: CardId) -> bool {
    matches!(
        card,
        CardId::Disarm
            | CardId::Shockwave
            | CardId::Uppercut
            | CardId::FlameBarrier
            | CardId::Impervious
            | CardId::DemonForm
            | CardId::FiendFire
            | CardId::PowerThrough
    )
}

fn high_impact_shop_relic(relic: RelicId) -> bool {
    matches!(
        relic,
        RelicId::MembershipCard
            | RelicId::Courier
            | RelicId::ClockworkSouvenir
            | RelicId::MedicalKit
            | RelicId::OrangePellets
            | RelicId::FrozenEye
            | RelicId::ChemicalX
            | RelicId::Waffle
            | RelicId::DollysMirror
            | RelicId::Orrery
    )
}

fn purgeable_curse(card: &CombatCard, run_state: &RunState) -> bool {
    crate::state::core::master_deck_card_is_purgeable(card)
        && !crate::state::core::master_deck_card_is_bottled(card, &run_state.relics)
        && get_card_definition(card.id).card_type == CardType::Curse
}

fn starter_card(card: &CombatCard) -> bool {
    let def = get_card_definition(card.id);
    def.tags.contains(&CardTag::StarterStrike) || def.tags.contains(&CardTag::StarterDefend)
}
