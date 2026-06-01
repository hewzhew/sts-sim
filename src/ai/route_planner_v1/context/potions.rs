use crate::content::potions::PotionId;
use crate::state::RunState;

use super::super::types::PotionRouteSummaryV1;

pub(super) fn build_potion_summary(run_state: &RunState) -> PotionRouteSummaryV1 {
    let potions = run_state
        .potions
        .iter()
        .filter_map(|slot| slot.as_ref().map(|potion| potion.id))
        .collect::<Vec<_>>();
    let has_elite_potion_signal = potions.iter().any(|id| {
        matches!(
            id,
            PotionId::FirePotion
                | PotionId::ExplosivePotion
                | PotionId::AttackPotion
                | PotionId::StrengthPotion
                | PotionId::SteroidPotion
                | PotionId::DuplicationPotion
                | PotionId::LiquidMemories
                | PotionId::EntropicBrew
        )
    });
    let has_defensive_potion_signal = potions.iter().any(|id| {
        matches!(
            id,
            PotionId::BlockPotion
                | PotionId::DexterityPotion
                | PotionId::SpeedPotion
                | PotionId::EssenceOfSteel
                | PotionId::LiquidBronze
                | PotionId::RegenPotion
                | PotionId::FairyPotion
                | PotionId::FruitJuice
                | PotionId::BloodPotion
        )
    });
    PotionRouteSummaryV1 {
        slots: run_state.potions.len(),
        filled: potions.len(),
        potions,
        has_elite_potion_signal,
        has_defensive_potion_signal,
    }
}
