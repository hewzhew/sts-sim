use crate::ai::card_semantics_v1::{card_mechanics_profile_v1, CombatExternalPayoffV1};
use crate::content::cards::CardId;
use crate::state::rewards::RewardCard;

use super::facts::card_facts;
use super::types::{
    CardRewardPickDependencyV1, CardRewardSemanticProfileV1, CardRewardSemanticRoleV1,
};

pub fn card_reward_semantic_profile_v1(card: &RewardCard) -> CardRewardSemanticProfileV1 {
    let facts = card_facts(card);
    let mechanics = card_mechanics_profile_v1(facts.card);
    let mut roles = Vec::new();

    if facts.damage.total_damage > 0 {
        push_role(&mut roles, CardRewardSemanticRoleV1::FrontloadDamage);
    }
    if facts.is_aoe {
        push_role(&mut roles, CardRewardSemanticRoleV1::AoeDamage);
    }
    if facts.block > 0 {
        push_role(&mut roles, CardRewardSemanticRoleV1::Block);
    }
    match facts.card {
        CardId::Barricade => push_role(&mut roles, CardRewardSemanticRoleV1::BlockRetention),
        CardId::Entrench => push_role(&mut roles, CardRewardSemanticRoleV1::BlockMultiplier),
        _ => {}
    }
    if facts.draw_cards > 0 {
        push_role(&mut roles, CardRewardSemanticRoleV1::CardDraw);
    }
    if mechanics.reshuffle_discard_into_draw {
        push_role(&mut roles, CardRewardSemanticRoleV1::CycleAccess);
    }
    if mechanics.discard_pile_topdeck_access {
        push_role(
            &mut roles,
            CardRewardSemanticRoleV1::DiscardPileTopdeckAccess,
        );
    }
    if mechanics.hand_topdeck_selection {
        push_role(&mut roles, CardRewardSemanticRoleV1::HandTopdeckSelection);
    }
    if facts.energy_gain > 0 {
        push_role(&mut roles, CardRewardSemanticRoleV1::EnergySource);
    }
    if facts.vulnerable > 0 {
        push_role(&mut roles, CardRewardSemanticRoleV1::Vulnerable);
    }
    if facts.weak > 0 {
        push_role(&mut roles, CardRewardSemanticRoleV1::Weak);
    }
    if facts.enemy_strength_down > 0 {
        push_role(&mut roles, CardRewardSemanticRoleV1::EnemyStrengthDown);
    }
    if mechanics.temporary_strength_burst && facts.strength_gain > 0 {
        push_role(&mut roles, CardRewardSemanticRoleV1::TemporaryStrengthBurst);
    } else if facts.strength_gain > 0 {
        push_role(&mut roles, CardRewardSemanticRoleV1::ScalingSource);
    }
    if matches!(
        mechanics.combat_external_payoff,
        Some(CombatExternalPayoffV1::PersistentOrReward)
    ) {
        push_role(&mut roles, CardRewardSemanticRoleV1::CombatExternalPayoff);
    }
    if matches!(
        mechanics.combat_external_payoff,
        Some(CombatExternalPayoffV1::HealingIfDamaged)
    ) {
        push_role(&mut roles, CardRewardSemanticRoleV1::CombatSustain);
    }
    if facts.exhausts_other_cards || facts.card == CardId::Corruption {
        push_role(&mut roles, CardRewardSemanticRoleV1::ExhaustGenerator);
    }
    if facts.card == CardId::Exhume {
        push_role(&mut roles, CardRewardSemanticRoleV1::ExhaustReuse);
    }
    if facts.adds_status_cards > 0 {
        push_role(&mut roles, CardRewardSemanticRoleV1::StatusGenerator);
    }
    if facts.is_random_output {
        push_role(&mut roles, CardRewardSemanticRoleV1::RandomOutput);
    }
    if facts.has_conditional_playability {
        push_role(&mut roles, CardRewardSemanticRoleV1::ConditionalPlayability);
    }
    if !facts.unsupported_mechanics.is_empty() {
        push_role(&mut roles, CardRewardSemanticRoleV1::UnsupportedMechanics);
    }

    for dependency in &facts.pick_dependencies {
        match dependency {
            CardRewardPickDependencyV1::StrengthScaling => {
                push_role(&mut roles, CardRewardSemanticRoleV1::StrengthPayoff);
                push_role(&mut roles, CardRewardSemanticRoleV1::PackagePayoff);
            }
            CardRewardPickDependencyV1::BlockDensity => {
                push_role(&mut roles, CardRewardSemanticRoleV1::BlockPayoff);
                push_role(&mut roles, CardRewardSemanticRoleV1::PackagePayoff);
            }
            CardRewardPickDependencyV1::StrikeDensity => {
                push_role(&mut roles, CardRewardSemanticRoleV1::StrikePayoff);
                push_role(&mut roles, CardRewardSemanticRoleV1::PackagePayoff);
            }
            CardRewardPickDependencyV1::RouteUpgradeDensity => {
                push_role(&mut roles, CardRewardSemanticRoleV1::UpgradePayoff);
                push_role(&mut roles, CardRewardSemanticRoleV1::PackagePayoff);
            }
            CardRewardPickDependencyV1::ExhaustPackage => {
                push_role(&mut roles, CardRewardSemanticRoleV1::ExhaustPayoff);
                push_role(&mut roles, CardRewardSemanticRoleV1::PackagePayoff);
            }
            CardRewardPickDependencyV1::StatusPackage => {
                push_role(&mut roles, CardRewardSemanticRoleV1::StatusPayoff);
                push_role(&mut roles, CardRewardSemanticRoleV1::PackagePayoff);
            }
            CardRewardPickDependencyV1::SelfDamagePackage => {
                push_role(&mut roles, CardRewardSemanticRoleV1::SelfDamagePayoff);
                push_role(&mut roles, CardRewardSemanticRoleV1::PackagePayoff);
            }
            CardRewardPickDependencyV1::RandomOutputPolicy
            | CardRewardPickDependencyV1::ConditionalPlayabilityPolicy
            | CardRewardPickDependencyV1::UnsupportedMechanics => {}
        }
    }

    roles.sort();
    roles.dedup();

    CardRewardSemanticProfileV1 {
        card: facts.card,
        name: facts.name,
        roles,
        dependencies: facts.pick_dependencies,
        unsupported_mechanics: facts.unsupported_mechanics,
    }
}

fn push_role(roles: &mut Vec<CardRewardSemanticRoleV1>, role: CardRewardSemanticRoleV1) {
    if !roles.contains(&role) {
        roles.push(role);
    }
}
