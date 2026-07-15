use crate::content::cards::CardId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct ActionSupplyTraitsV1 {
    pub opening_once_options: u8,
    pub delayed_per_turn: bool,
    pub same_turn_burst_min_follow_ups: u8,
    pub triggered_repeatable: bool,
    pub additional_play: bool,
    pub cost_or_resource_compression: bool,
    pub potentially_recursive: bool,
    pub immediate_hand: bool,
    pub zero_cost_this_turn: bool,
    pub optional_supply: bool,
}

impl ActionSupplyTraitsV1 {
    pub fn is_empty(self) -> bool {
        self == Self::default()
    }

    pub fn same_turn_burst(self) -> bool {
        self.same_turn_burst_min_follow_ups > 0
    }
}

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
    pub action_supply: ActionSupplyTraitsV1,
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
    pub action_supply: ActionSupplyTraitsV1,
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
    AoeDamage,
    CombatBlock,
    VulnerableSetup,
    WeakControl,
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
        action_supply: card_action_supply_traits_v1(card),
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
        action_supply: relic_action_supply_traits_v1(relic),
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

fn card_action_supply_traits_v1(card: CardId) -> ActionSupplyTraitsV1 {
    match card {
        CardId::BladeDance => ActionSupplyTraitsV1 {
            same_turn_burst_min_follow_ups: 3,
            immediate_hand: true,
            ..Default::default()
        },
        CardId::DoubleTap => ActionSupplyTraitsV1 {
            additional_play: true,
            ..Default::default()
        },
        CardId::Corruption | CardId::Offering => ActionSupplyTraitsV1 {
            cost_or_resource_compression: true,
            ..Default::default()
        },
        _ => ActionSupplyTraitsV1::default(),
    }
}

fn relic_action_supply_traits_v1(relic: RelicId) -> ActionSupplyTraitsV1 {
    match relic {
        RelicId::Enchiridion => ActionSupplyTraitsV1 {
            opening_once_options: 1,
            immediate_hand: true,
            zero_cost_this_turn: true,
            ..Default::default()
        },
        RelicId::Toolbox => ActionSupplyTraitsV1 {
            opening_once_options: 1,
            immediate_hand: true,
            ..Default::default()
        },
        RelicId::NilrysCodex => ActionSupplyTraitsV1 {
            delayed_per_turn: true,
            optional_supply: true,
            ..Default::default()
        },
        RelicId::DeadBranch => ActionSupplyTraitsV1 {
            triggered_repeatable: true,
            immediate_hand: true,
            potentially_recursive: true,
            ..Default::default()
        },
        _ => ActionSupplyTraitsV1::default(),
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
        PotionId::FirePotion => {
            push_potion_trait(&mut traits, PotionAcquisitionTraitV1::CombatDamage);
        }
        PotionId::ExplosivePotion => {
            push_potion_trait(&mut traits, PotionAcquisitionTraitV1::CombatDamage);
            push_potion_trait(&mut traits, PotionAcquisitionTraitV1::AoeDamage);
        }
        PotionId::BlockPotion | PotionId::EssenceOfSteel | PotionId::SpeedPotion => {
            push_potion_trait(&mut traits, PotionAcquisitionTraitV1::CombatBlock);
        }
        PotionId::FearPotion => {
            push_potion_trait(&mut traits, PotionAcquisitionTraitV1::VulnerableSetup);
        }
        PotionId::WeakenPotion => {
            push_potion_trait(&mut traits, PotionAcquisitionTraitV1::WeakControl);
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
    fn potion_traits_keep_damage_scope_and_debuff_kind_distinct() {
        let fire = potion_acquisition_traits_v1(PotionId::FirePotion);
        let explosive = potion_acquisition_traits_v1(PotionId::ExplosivePotion);
        let fear = potion_acquisition_traits_v1(PotionId::FearPotion);
        let weaken = potion_acquisition_traits_v1(PotionId::WeakenPotion);

        assert!(fire.contains(&PotionAcquisitionTraitV1::CombatDamage));
        assert!(!fire.contains(&PotionAcquisitionTraitV1::AoeDamage));
        assert!(explosive.contains(&PotionAcquisitionTraitV1::CombatDamage));
        assert!(explosive.contains(&PotionAcquisitionTraitV1::AoeDamage));
        assert!(fear.contains(&PotionAcquisitionTraitV1::VulnerableSetup));
        assert!(!fear.contains(&PotionAcquisitionTraitV1::WeakControl));
        assert!(weaken.contains(&PotionAcquisitionTraitV1::WeakControl));
        assert!(!weaken.contains(&PotionAcquisitionTraitV1::VulnerableSetup));
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

    #[test]
    fn action_supply_traits_distinguish_once_burst_repeatable_and_additional() {
        let enchiridion = relic_mechanics_profile_v1(RelicId::Enchiridion).action_supply;
        assert_eq!(enchiridion.opening_once_options, 1);
        assert!(enchiridion.immediate_hand);
        assert!(enchiridion.zero_cost_this_turn);
        assert!(!enchiridion.triggered_repeatable);
        assert!(!enchiridion.potentially_recursive);

        let toolbox = relic_mechanics_profile_v1(RelicId::Toolbox).action_supply;
        assert_eq!(toolbox.opening_once_options, 1);
        assert!(toolbox.immediate_hand);
        assert!(!toolbox.zero_cost_this_turn);

        let codex = relic_mechanics_profile_v1(RelicId::NilrysCodex).action_supply;
        assert!(codex.delayed_per_turn);
        assert!(codex.optional_supply);
        assert!(!codex.same_turn_burst());

        let branch = relic_mechanics_profile_v1(RelicId::DeadBranch).action_supply;
        assert!(branch.triggered_repeatable);
        assert!(branch.immediate_hand);
        assert!(branch.potentially_recursive);

        let blade_dance = card_mechanics_profile_v1(CardId::BladeDance).action_supply;
        assert_eq!(blade_dance.same_turn_burst_min_follow_ups, 3);
        assert!(blade_dance.same_turn_burst());

        let double_tap = card_mechanics_profile_v1(CardId::DoubleTap).action_supply;
        assert!(double_tap.additional_play);
        assert_eq!(double_tap.same_turn_burst_min_follow_ups, 0);

        assert!(
            card_mechanics_profile_v1(CardId::Corruption)
                .action_supply
                .cost_or_resource_compression
        );
        assert!(
            card_mechanics_profile_v1(CardId::Offering)
                .action_supply
                .cost_or_resource_compression
        );
    }
}
