use super::PotionCategory;
use crate::content::potions::PotionId;

pub const DONT_PLAY_POTIONS: &[PotionId] = &[PotionId::FairyPotion];

pub fn category_for(potion_id: PotionId) -> PotionCategory {
    match potion_id {
        PotionId::FirePotion
        | PotionId::ExplosivePotion
        | PotionId::PoisonPotion
        | PotionId::FearPotion => PotionCategory::Lethal,
        PotionId::BlockPotion
        | PotionId::BloodPotion
        | PotionId::RegenPotion
        | PotionId::WeakenPotion
        | PotionId::GhostInAJar
        | PotionId::StancePotion => PotionCategory::Survival,
        PotionId::StrengthPotion
        | PotionId::DexterityPotion
        | PotionId::SpeedPotion
        | PotionId::SteroidPotion
        | PotionId::DuplicationPotion
        | PotionId::HeartOfIron
        | PotionId::EssenceOfSteel
        | PotionId::LiquidBronze
        | PotionId::AncientPotion
        | PotionId::CultistPotion
        | PotionId::BlessingOfTheForge
        | PotionId::Ambrosia
        | PotionId::EssenceOfDarkness => PotionCategory::Setup,
        PotionId::SmokeBomb => PotionCategory::Escape,
        PotionId::PowerPotion
        | PotionId::ColorlessPotion
        | PotionId::AttackPotion
        | PotionId::SkillPotion
        | PotionId::SwiftPotion
        | PotionId::DistilledChaosPotion
        | PotionId::EntropicBrew
        | PotionId::CunningPotion
        | PotionId::PotionOfCapacity => PotionCategory::RandomGeneration,
        PotionId::EnergyPotion
        | PotionId::LiquidMemories
        | PotionId::GamblersBrew
        | PotionId::Elixir
        | PotionId::SneckoOil
        | PotionId::FruitJuice
        | PotionId::BottledMiracle => PotionCategory::Recovery,
        PotionId::FairyPotion => PotionCategory::Recovery,
        PotionId::FocusPotion => PotionCategory::Setup,
    }
}

pub fn category_label(category: PotionCategory) -> &'static str {
    match category {
        PotionCategory::Survival => "survival",
        PotionCategory::Lethal => "lethal",
        PotionCategory::Setup => "setup",
        PotionCategory::Recovery => "recovery",
        PotionCategory::Escape => "escape",
        PotionCategory::RandomGeneration => "random_generation",
    }
}
