use crate::content::cards::CardId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrengthConversionMechanicV1 {
    AmplifyCurrentStrength,
    PreventStrengthDownDebuff,
    ClearStrengthDownDebuff,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CardMechanicsProfileV1 {
    pub persistent_strength_source: bool,
    pub temporary_strength_burst: bool,
    pub applies_strength_down_debuff: bool,
    pub applies_no_draw_debuff: bool,
    pub strength_converter: Option<StrengthConversionMechanicV1>,
    pub strength_payoff: bool,
    pub self_damage_source: bool,
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

pub fn card_mechanics_profile_v1(card: CardId) -> CardMechanicsProfileV1 {
    CardMechanicsProfileV1 {
        persistent_strength_source: matches!(
            card,
            CardId::Inflame | CardId::SpotWeakness | CardId::DemonForm | CardId::JAX
        ),
        temporary_strength_burst: matches!(card, CardId::Flex),
        applies_strength_down_debuff: matches!(card, CardId::Flex),
        applies_no_draw_debuff: matches!(card, CardId::BattleTrance | CardId::BulletTime),
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
