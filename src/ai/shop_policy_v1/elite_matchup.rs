use crate::ai::card_reward_policy_v1::{
    card_facts, card_reward_semantic_profile_v1, CardRewardSemanticRoleV1,
};
use crate::ai::card_semantics_v1::{potion_acquisition_traits_v1, PotionAcquisitionTraitV1};
use crate::content::cards::CardType;
use crate::content::monsters::factory::EncounterId;
use crate::content::relics::RelicId;
use crate::state::rewards::RewardCard;

use super::types::{
    ShopCandidateEvidenceV1, ShopDecisionContextV1, ShopPurchaseTargetV1, ShopThreatWindowV1,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ShopEliteCoverageV1 {
    EliteHpReduction,
    MultiTargetDamage,
    FastAttackDamage,
    SetupOrMitigation,
    TemporaryCombatResource,
}

pub(crate) fn exact_next_elite_coverage_v1(
    context: &ShopDecisionContextV1,
    candidate: &ShopCandidateEvidenceV1,
) -> Option<ShopEliteCoverageV1> {
    if !matches!(context.visit.next_threat, ShopThreatWindowV1::EliteIn(_)) {
        return None;
    }
    let encounter = context.visit.next_elite_encounter?;
    purchase_coverage_v1(candidate.purchase_target?, encounter)
}

fn purchase_coverage_v1(
    target: ShopPurchaseTargetV1,
    encounter: EncounterId,
) -> Option<ShopEliteCoverageV1> {
    match target {
        ShopPurchaseTargetV1::Relic {
            relic: RelicId::PreservedInsect,
            ..
        } => Some(ShopEliteCoverageV1::EliteHpReduction),
        ShopPurchaseTargetV1::Card { card, .. } => card_coverage_v1(card, encounter),
        ShopPurchaseTargetV1::Potion { potion, .. } => {
            potion_coverage_v1(potion_acquisition_traits_v1(potion).as_slice(), encounter)
        }
        _ => None,
    }
}

fn card_coverage_v1(
    card: crate::content::cards::CardId,
    encounter: EncounterId,
) -> Option<ShopEliteCoverageV1> {
    let reward = RewardCard::new(card, 0);
    let facts = card_facts(&reward);
    let profile = card_reward_semantic_profile_v1(&reward);
    match encounter {
        EncounterId::ThreeSentries if facts.is_aoe => Some(ShopEliteCoverageV1::MultiTargetDamage),
        EncounterId::GremlinNob
            if facts.card_type == CardType::Attack && facts.damage.total_damage >= 9 =>
        {
            Some(ShopEliteCoverageV1::FastAttackDamage)
        }
        EncounterId::Lagavulin
            if facts.enemy_strength_down > 0
                || facts.weak > 0
                || facts.damage.total_damage >= 14
                || profile
                    .roles
                    .contains(&CardRewardSemanticRoleV1::ScalingSource) =>
        {
            Some(ShopEliteCoverageV1::SetupOrMitigation)
        }
        _ => None,
    }
}

fn potion_coverage_v1(
    traits: &[PotionAcquisitionTraitV1],
    encounter: EncounterId,
) -> Option<ShopEliteCoverageV1> {
    let has = |trait_| traits.contains(&trait_);
    let covers = match encounter {
        EncounterId::ThreeSentries => has(PotionAcquisitionTraitV1::AoeDamage),
        EncounterId::GremlinNob => {
            has(PotionAcquisitionTraitV1::CombatDamage)
                || has(PotionAcquisitionTraitV1::VulnerableSetup)
                || has(PotionAcquisitionTraitV1::StrengthGain)
        }
        EncounterId::Lagavulin => {
            has(PotionAcquisitionTraitV1::CombatBlock)
                || has(PotionAcquisitionTraitV1::WeakControl)
                || has(PotionAcquisitionTraitV1::DebuffControl)
                || has(PotionAcquisitionTraitV1::StrengthGain)
                || has(PotionAcquisitionTraitV1::ActionAmplifier)
        }
        _ => false,
    };
    covers.then_some(ShopEliteCoverageV1::TemporaryCombatResource)
}
