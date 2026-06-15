use crate::content::cards::{get_card_definition, CardId, CardTag, CardType};
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;
use crate::runtime::combat::CombatCard;
use crate::state::run::RunState;
use crate::state::shop::ShopState;

use super::types::ShopNeedProfileV1;

pub fn build_shop_need_profile_v1(run_state: &RunState) -> ShopNeedProfileV1 {
    let floors_to_boss = floors_to_act_boss(run_state.act_num, run_state.floor_num);
    ShopNeedProfileV1 {
        act: run_state.act_num,
        floor: run_state.floor_num,
        boss: run_state.boss_key,
        hp: run_state.current_hp,
        max_hp: run_state.max_hp,
        gold: run_state.gold,
        floors_to_boss,
        near_boss: floors_to_boss <= 4,
        has_curse: run_state.master_deck.iter().any(|card| {
            get_card_definition(card.id).card_type == CardType::Curse
                && crate::state::core::master_deck_card_is_purgeable(card)
                && !crate::state::core::master_deck_card_is_bottled(card, &run_state.relics)
        }),
        starter_count: run_state
            .master_deck
            .iter()
            .filter(|card| starter_card(card))
            .count(),
        strike_count: run_state
            .master_deck
            .iter()
            .filter(|card| {
                get_card_definition(card.id)
                    .tags
                    .contains(&CardTag::StarterStrike)
            })
            .count(),
        defend_count: run_state
            .master_deck
            .iter()
            .filter(|card| {
                get_card_definition(card.id)
                    .tags
                    .contains(&CardTag::StarterDefend)
            })
            .count(),
        empty_potion_slots: run_state
            .potions
            .iter()
            .filter(|potion| potion.is_none())
            .count(),
    }
}

pub fn shop_conversion_pressure_v1(run_state: &RunState, shop: &ShopState) -> bool {
    let gold = run_state.gold;
    if gold >= 300 {
        return true;
    }
    if affordable_high_impact_shop_purchase(run_state, shop) {
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
    if run_state.act_num >= 2 && shop_card_is_combat_patch_v1(card) {
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

pub fn shop_relic_conversion_priority_for_v1(relic: RelicId, run_state: &RunState) -> i32 {
    if relic == RelicId::ChemicalX && !deck_has_x_cost_payoff_v1(run_state) {
        return 720;
    }
    shop_relic_conversion_priority_v1(relic)
}

pub fn shop_potion_conversion_priority_v1(run_state: &RunState) -> i32 {
    if run_state.act_num >= 2 {
        680
    } else {
        520
    }
}

pub fn shop_potion_conversion_priority_for_v1(potion: PotionId, run_state: &RunState) -> i32 {
    let mut priority = shop_potion_conversion_priority_v1(run_state);
    let near_serious_fight = run_state.act_num >= 2 || run_state.floor_num >= 6;
    if near_serious_fight && shop_potion_is_combat_patch_v1(potion) {
        priority += 260;
        if run_state.act_num >= 3 {
            priority += 120;
        }
    }
    if potion == PotionId::FairyPotion && run_state.current_hp * 3 <= run_state.max_hp {
        priority += 220;
    }
    if potion == PotionId::SmokeBomb {
        priority -= 520;
    }
    priority
}

fn affordable_high_impact_shop_purchase(run_state: &RunState, shop: &ShopState) -> bool {
    let gold = run_state.gold;
    shop.relics
        .iter()
        .any(|relic| relic.can_buy && relic.price <= gold && high_impact_shop_relic(relic.relic_id))
        || shop.cards.iter().any(|card| {
            card.can_buy
                && card.price <= gold
                && high_impact_shop_card(card.card_id)
                && (run_state.act_num >= 2 || shop_card_is_combat_patch_v1(card.card_id))
        })
        || shop.potions.iter().any(|potion| {
            potion.can_buy
                && potion.price <= gold
                && shop_potion_is_combat_patch_v1(potion.potion_id)
                && (run_state.act_num >= 2 || run_state.floor_num >= 6)
        })
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

pub(crate) fn shop_card_is_combat_patch_v1(card: CardId) -> bool {
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

pub(crate) fn shop_potion_is_combat_patch_v1(potion: PotionId) -> bool {
    matches!(
        potion,
        PotionId::DuplicationPotion
            | PotionId::FearPotion
            | PotionId::FirePotion
            | PotionId::WeakenPotion
            | PotionId::EssenceOfSteel
            | PotionId::BlockPotion
            | PotionId::EnergyPotion
            | PotionId::StrengthPotion
            | PotionId::SteroidPotion
            | PotionId::SpeedPotion
            | PotionId::AncientPotion
            | PotionId::GamblersBrew
            | PotionId::LiquidMemories
            | PotionId::FairyPotion
            | PotionId::PowerPotion
            | PotionId::SkillPotion
            | PotionId::AttackPotion
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

fn deck_has_x_cost_payoff_v1(run_state: &RunState) -> bool {
    run_state
        .master_deck
        .iter()
        .any(|card| get_card_definition(card.id).cost == -1)
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

fn floors_to_act_boss(act: u8, floor: i32) -> i32 {
    let boss_floor = match act {
        1 => 16,
        2 => 32,
        3 => 48,
        _ => floor,
    };
    boss_floor.saturating_sub(floor)
}
