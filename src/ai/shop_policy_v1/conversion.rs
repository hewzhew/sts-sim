use crate::ai::boss_mechanics_v1::{
    boss_mechanic_pressure_profile_v1, BossMechanicMissingAnswerV1, BossMechanicRedFlagV1,
};
use crate::ai::card_admission_policy_v1::{
    evaluate_card_admission_v1, CardAdmissionSourceV1, CardAdmissionVerdictV1,
};
use crate::ai::card_semantics_v1::card_mechanics_profile_v1;
use crate::content::cards::{get_card_definition, CardId, CardTag, CardType};
use crate::content::monsters::factory::EncounterId;
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
    priority += boss_mechanic_shop_card_priority_bonus_v1(card, run_state);
    if run_state.act_num >= 2 && shop_card_is_combat_patch_v1(card) {
        priority += 200;
    }
    let admission = evaluate_card_admission_v1(
        run_state,
        RewardCard::new(card, 0),
        CardAdmissionSourceV1::Shop,
    );
    priority += admission.shop_priority_adjustment;
    if admission.verdict == CardAdmissionVerdictV1::Reject
        && admission
            .reasons
            .iter()
            .any(|reason| reason.starts_with("startup_rejects_"))
    {
        priority -= 350;
    }
    priority
}

fn boss_mechanic_shop_card_priority_bonus_v1(card: CardId, run_state: &RunState) -> i32 {
    match (run_state.act_num, run_state.boss_key) {
        (2, Some(EncounterId::TheChamp)) => champ_shop_card_priority_bonus_v1(card, run_state),
        _ => 0,
    }
}

fn champ_shop_card_priority_bonus_v1(card: CardId, run_state: &RunState) -> i32 {
    let profile = boss_mechanic_pressure_profile_v1(run_state, EncounterId::TheChamp);
    let needs_transition_burst = profile
        .has_missing_answer(BossMechanicMissingAnswerV1::ChampTransitionBurst)
        || profile.has_red_flag(BossMechanicRedFlagV1::PrematureChampTransitionRisk);
    let needs_execute_block = profile
        .has_missing_answer(BossMechanicMissingAnswerV1::ExecuteBlockPlan)
        || profile.has_red_flag(BossMechanicRedFlagV1::NoExecuteBlockPlan);
    let mut bonus = 0;

    if needs_transition_burst && champ_transition_burst_shop_card_v1(card, run_state) {
        bonus += 420;
    }
    if needs_execute_block && champ_execute_block_shop_card_v1(card) {
        bonus += 360;
    }

    bonus
}

fn champ_transition_burst_shop_card_v1(card: CardId, run_state: &RunState) -> bool {
    let strength_profile = crate::ai::strength_profile_v1::strength_profile_v1(run_state);
    let mechanics = card_mechanics_profile_v1(card);
    if mechanics.temporary_strength_burst {
        return strength_profile.payoffs > 0 || strength_profile.converters > 0;
    }

    match card {
        CardId::Carnage
        | CardId::Bludgeon
        | CardId::Immolate
        | CardId::Offering
        | CardId::DemonForm
        | CardId::Whirlwind => true,
        CardId::HeavyBlade => {
            strength_profile.stable_sources > 0
                || strength_profile.temporary_bursts > 0
                || strength_profile.convertible_potential_count > 0
        }
        CardId::LimitBreak => {
            strength_profile.stable_sources > 0 || strength_profile.temporary_bursts > 0
        }
        _ => false,
    }
}

fn champ_execute_block_shop_card_v1(card: CardId) -> bool {
    matches!(
        card,
        CardId::Impervious
            | CardId::PowerThrough
            | CardId::FlameBarrier
            | CardId::SecondWind
            | CardId::TrueGrit
            | CardId::Entrench
            | CardId::Barricade
    )
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
