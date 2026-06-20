use crate::ai::card_reward_policy_v1::{
    card_facts, card_reward_semantic_profile_v1, CardRewardSemanticProfileV1,
    CardRewardSemanticRoleV1,
};
use crate::ai::card_semantics_v1::{potion_acquisition_traits_v1, relic_acquisition_traits_v1};
use crate::content::cards::{get_card_definition, CardId, CardTag, CardType};
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;
use crate::runtime::combat::CombatCard;
use crate::state::rewards::RewardCard;
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

pub fn legacy_shop_card_purchase_estimate_v1(card: CardId, run_state: &RunState) -> i32 {
    let mut priority = 250;
    if shop_card_has_high_impact_semantics_v1(card) {
        priority += 450;
    }
    if run_state.act_num >= 2 && shop_card_is_combat_patch_v1(card) {
        priority += 200;
    }
    priority
}

pub fn legacy_shop_relic_purchase_estimate_v1(relic: RelicId) -> i32 {
    if high_impact_shop_relic(relic) {
        950
    } else {
        720
    }
}

pub fn legacy_shop_relic_purchase_estimate_for_v1(relic: RelicId, run_state: &RunState) -> i32 {
    if relic == RelicId::ChemicalX && !deck_has_x_cost_payoff_v1(run_state) {
        return 0;
    }
    if relic == RelicId::DollysMirror
        && crate::ai::deck_mutation_compiler_v1::best_duplicate_target_for_shop_v1(run_state)
            .is_none()
    {
        return 0;
    }
    legacy_shop_relic_purchase_estimate_v1(relic)
}

pub fn legacy_shop_potion_purchase_estimate_v1(run_state: &RunState) -> i32 {
    if run_state.act_num >= 2 {
        680
    } else {
        520
    }
}

pub fn legacy_shop_potion_purchase_estimate_for_v1(potion: PotionId, run_state: &RunState) -> i32 {
    let mut priority = legacy_shop_potion_purchase_estimate_v1(run_state);
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
    shop.relics.iter().any(|relic| {
        relic.can_buy
            && relic.price <= gold
            && legacy_shop_relic_purchase_estimate_for_v1(relic.relic_id, run_state) >= 900
    }) || shop.cards.iter().any(|card| {
        card.can_buy
            && card.price <= gold
            && shop_card_has_high_impact_semantics_v1(card.card_id)
            && (run_state.act_num >= 2 || shop_card_is_combat_patch_v1(card.card_id))
    }) || shop.potions.iter().any(|potion| {
        potion.can_buy
            && potion.price <= gold
            && shop_potion_is_combat_patch_v1(potion.potion_id)
            && (run_state.act_num >= 2 || run_state.floor_num >= 6)
    })
}

fn shop_card_has_high_impact_semantics_v1(card: CardId) -> bool {
    let reward_card = RewardCard::new(card, 0);
    let facts = card_facts(&reward_card);
    let profile = card_reward_semantic_profile_v1(&reward_card);

    role(&profile, CardRewardSemanticRoleV1::CardDraw)
        || role(&profile, CardRewardSemanticRoleV1::CycleAccess)
        || role(&profile, CardRewardSemanticRoleV1::EnergySource)
        || role(&profile, CardRewardSemanticRoleV1::EnemyStrengthDown)
        || role(&profile, CardRewardSemanticRoleV1::BlockRetention)
        || role(&profile, CardRewardSemanticRoleV1::BlockMultiplier)
        || role(&profile, CardRewardSemanticRoleV1::ExhaustGenerator)
        || role(&profile, CardRewardSemanticRoleV1::ExhaustReuse)
        || role(&profile, CardRewardSemanticRoleV1::ExhaustPayoff)
        || role(&profile, CardRewardSemanticRoleV1::StatusPayoff)
        || role(&profile, CardRewardSemanticRoleV1::CombatExternalPayoff)
        || role(&profile, CardRewardSemanticRoleV1::CombatSustain)
        || shop_card_has_dual_debuff_semantics_v1(&profile)
        || facts.block >= 12
        || (role(&profile, CardRewardSemanticRoleV1::ScalingSource)
            && facts.card_type == CardType::Power
            && facts.cost >= 2)
}

pub(crate) fn shop_card_is_combat_patch_v1(card: CardId) -> bool {
    let reward_card = RewardCard::new(card, 0);
    let facts = card_facts(&reward_card);
    let profile = card_reward_semantic_profile_v1(&reward_card);

    role(&profile, CardRewardSemanticRoleV1::EnemyStrengthDown)
        || shop_card_has_dual_debuff_semantics_v1(&profile)
        || facts.block >= 12
        || (role(&profile, CardRewardSemanticRoleV1::ScalingSource)
            && facts.card_type == CardType::Power
            && facts.cost >= 2)
        || (role(&profile, CardRewardSemanticRoleV1::ExhaustGenerator)
            && facts.card_type == CardType::Attack)
}

fn shop_card_has_dual_debuff_semantics_v1(profile: &CardRewardSemanticProfileV1) -> bool {
    role(profile, CardRewardSemanticRoleV1::Weak)
        && role(profile, CardRewardSemanticRoleV1::Vulnerable)
}

fn role(profile: &CardRewardSemanticProfileV1, role: CardRewardSemanticRoleV1) -> bool {
    profile.roles.contains(&role)
}

pub(crate) fn shop_potion_is_combat_patch_v1(potion: PotionId) -> bool {
    !potion_acquisition_traits_v1(potion).is_empty()
}

fn high_impact_shop_relic(relic: RelicId) -> bool {
    !relic_acquisition_traits_v1(relic).is_empty()
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
