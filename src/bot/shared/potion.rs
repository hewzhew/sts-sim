use crate::content::potions::PotionId;
use crate::state::run::RunState;

pub(crate) fn score_shop_potion(run_state: &RunState, potion_id: PotionId) -> i32 {
    let need = super::need::analyze_run_needs(run_state);
    let mut score = match potion_id {
        PotionId::AncientPotion => 98,
        PotionId::PowerPotion | PotionId::ColorlessPotion => 92,
        PotionId::DuplicationPotion | PotionId::GhostInAJar => 90,
        PotionId::BlessingOfTheForge | PotionId::Elixir => 82,
        PotionId::StrengthPotion
        | PotionId::DexterityPotion
        | PotionId::SpeedPotion
        | PotionId::SteroidPotion
        | PotionId::EssenceOfSteel
        | PotionId::LiquidBronze
        | PotionId::RegenPotion => 84,
        PotionId::EnergyPotion | PotionId::SwiftPotion => 80,
        PotionId::FearPotion | PotionId::ExplosivePotion | PotionId::FirePotion => 78,
        PotionId::BlockPotion | PotionId::WeakenPotion => 76,
        PotionId::FruitJuice | PotionId::BloodPotion | PotionId::FairyPotion => 86,
        _ => 55,
    };

    if need.damage_gap > 0 {
        match potion_id {
            PotionId::FearPotion
            | PotionId::FirePotion
            | PotionId::ExplosivePotion
            | PotionId::AttackPotion
            | PotionId::StrengthPotion
            | PotionId::DuplicationPotion => score += 8 + need.damage_gap / 2,
            _ => {}
        }
    }
    if need.block_gap > 0 {
        match potion_id {
            PotionId::GhostInAJar
            | PotionId::BlockPotion
            | PotionId::WeakenPotion
            | PotionId::DexterityPotion
            | PotionId::EssenceOfSteel
            | PotionId::LiquidBronze => score += 8 + need.block_gap / 3,
            _ => {}
        }
    }
    if need.control_gap > 0 {
        match potion_id {
            PotionId::WeakenPotion | PotionId::FearPotion | PotionId::SwiftPotion => {
                score += 6 + need.control_gap / 3
            }
            _ => {}
        }
    }

    score
}

pub(crate) fn score_reward_potion(run_state: &RunState, potion_id: PotionId) -> i32 {
    score_shop_potion(run_state, potion_id).max(base_reward_potion_score(potion_id))
}

pub(crate) fn best_potion_replacement(
    run_state: &RunState,
    offered_score: i32,
    scorer: impl Fn(PotionId) -> i32,
) -> Option<usize> {
    let (discard_idx, kept_score) = run_state
        .potions
        .iter()
        .enumerate()
        .filter_map(|(idx, potion)| potion.as_ref().map(|potion| (idx, scorer(potion.id))))
        .min_by_key(|(_, score)| *score)?;
    (offered_score > kept_score).then_some(discard_idx)
}

fn base_reward_potion_score(potion_id: PotionId) -> i32 {
    match potion_id {
        PotionId::AncientPotion => 98,
        PotionId::PowerPotion | PotionId::ColorlessPotion => 92,
        PotionId::DuplicationPotion | PotionId::GhostInAJar => 90,
        PotionId::FruitJuice | PotionId::BloodPotion | PotionId::FairyPotion => 88,
        PotionId::BlessingOfTheForge | PotionId::Elixir => 82,
        PotionId::StrengthPotion
        | PotionId::DexterityPotion
        | PotionId::SpeedPotion
        | PotionId::SteroidPotion
        | PotionId::EssenceOfSteel
        | PotionId::LiquidBronze
        | PotionId::RegenPotion => 84,
        PotionId::EnergyPotion | PotionId::SwiftPotion => 80,
        _ => 55,
    }
}
