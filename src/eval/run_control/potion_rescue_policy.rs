use crate::content::potions::PotionId;

use super::RunControlSession;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OraclePotionRescueKindV1 {
    /// Legacy owner-audit refinement. Until that separate control flow can
    /// retain the no-potion incumbent, it admits only combat-local resources.
    ImproveVerifiedWin,
    /// Autonomous run refinement retains the exact no-potion incumbent and
    /// may inspect any active potion, but a spending line may replace that
    /// incumbent only by satisfying the configured strategic quality target.
    ImproveVerifiedWinQualityGated,
    FindAnyWin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OraclePotionRescueTierV1 {
    /// A common, deterministic, combat-only tactical effect. These may compete
    /// with an existing no-potion win, but only when that win misses the
    /// strategic HP-quality target and under the exact one-potion cap.
    BoundedQuality,
    /// Flexible discovery, out-of-combat recovery, and all other uncommon or
    /// rare active resources need continuation-sensitive review. Legacy owner
    /// refinement keeps them reserved while a verified win exists. Autonomous
    /// refinement may inspect them behind the exact no-potion-incumbent
    /// quality gate, or use them when no victory has been found.
    FindAnyWin,
    /// Passive death insurance and explicit escape are not active victory
    /// actions. Their separate run-control contracts remain authoritative.
    Excluded,
}

pub fn oracle_potion_rescue_slot_mask_v1(
    session: &RunControlSession,
    rescue_kind: OraclePotionRescueKindV1,
) -> u64 {
    let Some(active) = session.active_combat.as_ref() else {
        return 0;
    };
    active
        .combat_state
        .entities
        .potions
        .iter()
        .enumerate()
        .filter_map(|(slot, potion)| {
            let potion = potion.as_ref()?;
            if !potion.can_use
                || oracle_potion_rescue_tier_v1(potion.id) == OraclePotionRescueTierV1::Excluded
            {
                return None;
            }
            if rescue_kind == OraclePotionRescueKindV1::ImproveVerifiedWin
                && oracle_potion_rescue_tier_v1(potion.id)
                    != OraclePotionRescueTierV1::BoundedQuality
            {
                return None;
            }
            u32::try_from(slot)
                .ok()
                .and_then(|slot| 1_u64.checked_shl(slot))
        })
        .fold(0, |mask, slot| mask | slot)
}

pub fn oracle_potion_rescue_tier_v1(potion: PotionId) -> OraclePotionRescueTierV1 {
    match potion {
        // Common, deterministic effects whose value is fully realized inside
        // the current combat. This includes direct output and ordinary
        // energy/stat/hand conversion, but not healing or card discovery.
        PotionId::FirePotion
        | PotionId::ExplosivePotion
        | PotionId::PoisonPotion
        | PotionId::WeakenPotion
        | PotionId::FearPotion
        | PotionId::BlockPotion
        | PotionId::EnergyPotion
        | PotionId::StrengthPotion
        | PotionId::DexterityPotion
        | PotionId::SpeedPotion
        | PotionId::SteroidPotion
        | PotionId::SwiftPotion
        | PotionId::FocusPotion
        | PotionId::BottledMiracle
        | PotionId::BlessingOfTheForge => OraclePotionRescueTierV1::BoundedQuality,

        // Blood Potion can be spent outside combat; discovery effects carry
        // broader encounter-specific option value. Uncommon and rare active
        // potions are likewise kept for a genuine no-win emergency rather
        // than exchanged for a marginal improvement to a verified line.
        PotionId::BloodPotion
        | PotionId::AttackPotion
        | PotionId::SkillPotion
        | PotionId::PowerPotion
        | PotionId::ColorlessPotion
        | PotionId::AncientPotion
        | PotionId::RegenPotion
        | PotionId::EssenceOfSteel
        | PotionId::LiquidBronze
        | PotionId::DistilledChaosPotion
        | PotionId::DuplicationPotion
        | PotionId::CunningPotion
        | PotionId::PotionOfCapacity
        | PotionId::LiquidMemories
        | PotionId::GamblersBrew
        | PotionId::Elixir
        | PotionId::StancePotion
        | PotionId::FruitJuice
        | PotionId::EntropicBrew
        | PotionId::SneckoOil
        | PotionId::GhostInAJar
        | PotionId::HeartOfIron
        | PotionId::CultistPotion
        | PotionId::Ambrosia
        | PotionId::EssenceOfDarkness => OraclePotionRescueTierV1::FindAnyWin,

        PotionId::FairyPotion | PotionId::SmokeBomb => OraclePotionRescueTierV1::Excluded,
    }
}
