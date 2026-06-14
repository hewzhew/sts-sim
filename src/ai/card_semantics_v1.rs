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
    pub strength_converter: Option<StrengthConversionMechanicV1>,
    pub strength_payoff: bool,
    pub self_damage_source: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RelicMechanicsProfileV1 {
    pub persistent_strength_source: bool,
    pub temporary_strength_burst: bool,
    pub strength_converter: Option<StrengthConversionMechanicV1>,
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
