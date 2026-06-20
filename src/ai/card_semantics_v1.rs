use crate::content::cards::CardId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrengthConversionMechanicV1 {
    AmplifyCurrentStrength,
    PreventStrengthDownDebuff,
    ClearStrengthDownDebuff,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatExternalPayoffV1 {
    PersistentOrReward,
    HealingIfDamaged,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CardMechanicsProfileV1 {
    pub persistent_strength_source: bool,
    pub temporary_strength_burst: bool,
    pub applies_strength_down_debuff: bool,
    pub applies_no_draw_debuff: bool,
    pub reshuffle_discard_into_draw: bool,
    pub discard_pile_topdeck_access: bool,
    pub hand_topdeck_selection: bool,
    pub strength_converter: Option<StrengthConversionMechanicV1>,
    pub strength_payoff: bool,
    pub self_damage_source: bool,
    pub combat_external_payoff: Option<CombatExternalPayoffV1>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RelicMechanicsProfileV1 {
    pub persistent_strength_source: bool,
    pub temporary_strength_burst: bool,
    pub strength_converter: Option<StrengthConversionMechanicV1>,
    pub core_defense_or_survival: bool,
    pub core_card_access: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PotionMechanicsProfileV1 {
    pub temporary_strength_burst: bool,
    pub strength_converter: Option<StrengthConversionMechanicV1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelicAcquisitionTraitV1 {
    CoreDefenseOrSurvival,
    CoreCardAccess,
    ShopEconomyMultiplier,
    StatusDigest,
    DebuffControl,
    XCostPayoff,
    ImmediateRecovery,
    DeckMutation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PotionAcquisitionTraitV1 {
    CombatDamage,
    CombatBlock,
    DebuffSetup,
    EnergyBurst,
    StrengthGain,
    CardAccess,
    ActionAmplifier,
    DeathInsurance,
    DebuffControl,
    EscapeTool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcquisitionRequirementV1 {
    XCostPayoff,
    DuplicateTarget,
    LowHpDeathInsurance,
    RouteEscapeValue,
}

pub fn card_mechanics_profile_v1(card: CardId) -> CardMechanicsProfileV1 {
    CardMechanicsProfileV1 {
        persistent_strength_source: matches!(
            card,
            CardId::Inflame | CardId::SpotWeakness | CardId::DemonForm | CardId::JAX
        ),
        temporary_strength_burst: matches!(card, CardId::Flex),
        applies_strength_down_debuff: matches!(card, CardId::Flex),
        applies_no_draw_debuff: matches!(card, CardId::BattleTrance | CardId::BulletTime),
        reshuffle_discard_into_draw: matches!(card, CardId::DeepBreath),
        discard_pile_topdeck_access: matches!(card, CardId::Headbutt),
        hand_topdeck_selection: matches!(card, CardId::Warcry),
        strength_converter: match card {
            CardId::LimitBreak => Some(StrengthConversionMechanicV1::AmplifyCurrentStrength),
            CardId::Panacea | CardId::CoreSurge => {
                Some(StrengthConversionMechanicV1::PreventStrengthDownDebuff)
            }
            _ => None,
        },
        strength_payoff: matches!(
            card,
            CardId::HeavyBlade
                | CardId::SwordBoomerang
                | CardId::Pummel
                | CardId::LimitBreak
                | CardId::Reaper
                | CardId::Whirlwind
        ),
        self_damage_source: matches!(
            card,
            CardId::Bloodletting
                | CardId::Offering
                | CardId::Hemokinesis
                | CardId::Combust
                | CardId::Brutality
                | CardId::JAX
        ),
        combat_external_payoff: match card {
            CardId::Feed
            | CardId::LessonLearned
            | CardId::HandOfGreed
            | CardId::RitualDagger
            | CardId::Alchemize
            | CardId::GeneticAlgorithm
            | CardId::Wish => Some(CombatExternalPayoffV1::PersistentOrReward),
            CardId::BandageUp | CardId::Bite | CardId::Reaper | CardId::SelfRepair => {
                Some(CombatExternalPayoffV1::HealingIfDamaged)
            }
            _ => None,
        },
    }
}

pub fn relic_mechanics_profile_v1(relic: RelicId) -> RelicMechanicsProfileV1 {
    RelicMechanicsProfileV1 {
        persistent_strength_source: matches!(relic, RelicId::Vajra),
        temporary_strength_burst: matches!(relic, RelicId::MutagenicStrength),
        strength_converter: match relic {
            RelicId::ClockworkSouvenir => {
                Some(StrengthConversionMechanicV1::PreventStrengthDownDebuff)
            }
            RelicId::OrangePellets => Some(StrengthConversionMechanicV1::ClearStrengthDownDebuff),
            _ => None,
        },
        core_defense_or_survival: matches!(
            relic,
            RelicId::BurningBlood
                | RelicId::BlackBlood
                | RelicId::BloodVial
                | RelicId::MagicFlower
                | RelicId::MeatOnTheBone
                | RelicId::Pantograph
                | RelicId::FossilizedHelix
                | RelicId::IncenseBurner
                | RelicId::ThreadAndNeedle
                | RelicId::Torii
                | RelicId::TungstenRod
                | RelicId::Calipers
                | RelicId::Orichalcum
                | RelicId::HornCleat
                | RelicId::CaptainsWheel
                | RelicId::LizardTail
                | RelicId::SacredBark
        ),
        core_card_access: matches!(
            relic,
            RelicId::RunicPyramid
                | RelicId::SneckoEye
                | RelicId::GamblingChip
                | RelicId::FrozenEye
                | RelicId::BagOfPreparation
                | RelicId::Pocketwatch
                | RelicId::QuestionCard
                | RelicId::InkBottle
                | RelicId::RunicCube
                | RelicId::Sundial
                | RelicId::IceCream
        ),
    }
}

pub fn potion_mechanics_profile_v1(potion: PotionId) -> PotionMechanicsProfileV1 {
    PotionMechanicsProfileV1 {
        temporary_strength_burst: matches!(potion, PotionId::SteroidPotion),
        strength_converter: match potion {
            PotionId::AncientPotion => {
                Some(StrengthConversionMechanicV1::PreventStrengthDownDebuff)
            }
            _ => None,
        },
    }
}

pub fn relic_acquisition_traits_v1(relic: RelicId) -> Vec<RelicAcquisitionTraitV1> {
    let mechanics = relic_mechanics_profile_v1(relic);
    let mut traits = Vec::new();
    if mechanics.core_defense_or_survival {
        push_relic_trait(&mut traits, RelicAcquisitionTraitV1::CoreDefenseOrSurvival);
    }
    if mechanics.core_card_access {
        push_relic_trait(&mut traits, RelicAcquisitionTraitV1::CoreCardAccess);
    }
    if mechanics.strength_converter.is_some() {
        push_relic_trait(&mut traits, RelicAcquisitionTraitV1::DebuffControl);
    }
    match relic {
        RelicId::MembershipCard | RelicId::Courier => {
            push_relic_trait(&mut traits, RelicAcquisitionTraitV1::ShopEconomyMultiplier);
        }
        RelicId::MedicalKit => {
            push_relic_trait(&mut traits, RelicAcquisitionTraitV1::StatusDigest);
        }
        RelicId::ChemicalX => {
            push_relic_trait(&mut traits, RelicAcquisitionTraitV1::XCostPayoff);
        }
        RelicId::Waffle => {
            push_relic_trait(&mut traits, RelicAcquisitionTraitV1::ImmediateRecovery);
        }
        RelicId::DollysMirror | RelicId::Orrery => {
            push_relic_trait(&mut traits, RelicAcquisitionTraitV1::DeckMutation);
        }
        _ => {}
    }
    traits
}

pub fn relic_acquisition_requirements_v1(relic: RelicId) -> Vec<AcquisitionRequirementV1> {
    match relic {
        RelicId::ChemicalX => vec![AcquisitionRequirementV1::XCostPayoff],
        RelicId::DollysMirror => vec![AcquisitionRequirementV1::DuplicateTarget],
        _ => Vec::new(),
    }
}

pub fn potion_acquisition_traits_v1(potion: PotionId) -> Vec<PotionAcquisitionTraitV1> {
    let mechanics = potion_mechanics_profile_v1(potion);
    let mut traits = Vec::new();
    if mechanics.temporary_strength_burst || potion == PotionId::StrengthPotion {
        push_potion_trait(&mut traits, PotionAcquisitionTraitV1::StrengthGain);
    }
    if mechanics.strength_converter.is_some() {
        push_potion_trait(&mut traits, PotionAcquisitionTraitV1::DebuffControl);
    }
    match potion {
        PotionId::FirePotion | PotionId::ExplosivePotion => {
            push_potion_trait(&mut traits, PotionAcquisitionTraitV1::CombatDamage);
        }
        PotionId::BlockPotion | PotionId::EssenceOfSteel | PotionId::SpeedPotion => {
            push_potion_trait(&mut traits, PotionAcquisitionTraitV1::CombatBlock);
        }
        PotionId::FearPotion | PotionId::WeakenPotion => {
            push_potion_trait(&mut traits, PotionAcquisitionTraitV1::DebuffSetup);
        }
        PotionId::EnergyPotion => {
            push_potion_trait(&mut traits, PotionAcquisitionTraitV1::EnergyBurst);
        }
        PotionId::GamblersBrew
        | PotionId::LiquidMemories
        | PotionId::PowerPotion
        | PotionId::SkillPotion
        | PotionId::AttackPotion => {
            push_potion_trait(&mut traits, PotionAcquisitionTraitV1::CardAccess);
        }
        PotionId::DuplicationPotion => {
            push_potion_trait(&mut traits, PotionAcquisitionTraitV1::ActionAmplifier);
        }
        PotionId::FairyPotion => {
            push_potion_trait(&mut traits, PotionAcquisitionTraitV1::DeathInsurance);
        }
        PotionId::SmokeBomb => {
            push_potion_trait(&mut traits, PotionAcquisitionTraitV1::EscapeTool);
        }
        _ => {}
    }
    traits
}

pub fn potion_acquisition_requirements_v1(potion: PotionId) -> Vec<AcquisitionRequirementV1> {
    match potion {
        PotionId::FairyPotion => vec![AcquisitionRequirementV1::LowHpDeathInsurance],
        PotionId::SmokeBomb => vec![AcquisitionRequirementV1::RouteEscapeValue],
        _ => Vec::new(),
    }
}

fn push_relic_trait(traits: &mut Vec<RelicAcquisitionTraitV1>, trait_: RelicAcquisitionTraitV1) {
    if !traits.contains(&trait_) {
        traits.push(trait_);
    }
}

fn push_potion_trait(traits: &mut Vec<PotionAcquisitionTraitV1>, trait_: PotionAcquisitionTraitV1) {
    if !traits.contains(&trait_) {
        traits.push(trait_);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_mechanics_exposes_combat_external_payoff_once() {
        assert_eq!(
            card_mechanics_profile_v1(CardId::Feed).combat_external_payoff,
            Some(CombatExternalPayoffV1::PersistentOrReward)
        );
        assert_eq!(
            card_mechanics_profile_v1(CardId::HandOfGreed).combat_external_payoff,
            Some(CombatExternalPayoffV1::PersistentOrReward)
        );
        assert_eq!(
            card_mechanics_profile_v1(CardId::Reaper).combat_external_payoff,
            Some(CombatExternalPayoffV1::HealingIfDamaged)
        );
        assert_eq!(
            card_mechanics_profile_v1(CardId::TwinStrike).combat_external_payoff,
            None
        );
    }

    #[test]
    fn deep_breath_exposes_reshuffle_access() {
        let mechanics = card_mechanics_profile_v1(CardId::DeepBreath);

        assert!(mechanics.reshuffle_discard_into_draw);
    }

    #[test]
    fn strength_potion_is_not_temporary_strength_burst() {
        assert!(!potion_mechanics_profile_v1(PotionId::StrengthPotion).temporary_strength_burst);
        assert!(potion_mechanics_profile_v1(PotionId::SteroidPotion).temporary_strength_burst);
        assert!(potion_acquisition_traits_v1(PotionId::StrengthPotion)
            .contains(&PotionAcquisitionTraitV1::StrengthGain));
    }

    #[test]
    fn topdeck_control_cards_expose_mechanical_access() {
        assert!(card_mechanics_profile_v1(CardId::Headbutt).discard_pile_topdeck_access);
        assert!(card_mechanics_profile_v1(CardId::Warcry).hand_topdeck_selection);
    }

    #[test]
    fn conditional_shop_objects_expose_requirements() {
        assert!(relic_acquisition_requirements_v1(RelicId::ChemicalX)
            .contains(&AcquisitionRequirementV1::XCostPayoff));
        assert!(relic_acquisition_requirements_v1(RelicId::DollysMirror)
            .contains(&AcquisitionRequirementV1::DuplicateTarget));
        assert!(potion_acquisition_traits_v1(PotionId::SmokeBomb)
            .contains(&PotionAcquisitionTraitV1::EscapeTool));
        assert!(potion_acquisition_requirements_v1(PotionId::FairyPotion)
            .contains(&AcquisitionRequirementV1::LowHpDeathInsurance));
        assert!(potion_acquisition_requirements_v1(PotionId::SmokeBomb)
            .contains(&AcquisitionRequirementV1::RouteEscapeValue));
    }
}
