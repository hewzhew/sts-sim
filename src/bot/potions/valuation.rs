use crate::bot::agent::Agent;
use crate::state::run::RunState;

impl Agent {
    pub(crate) fn shop_potion_score(
        &self,
        rs: &RunState,
        potion_id: crate::content::potions::PotionId,
    ) -> i32 {
        use crate::content::potions::PotionId;
        let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
        let shop_need = self.build_shop_need_profile(rs);
        let mut score = match potion_id {
            PotionId::AncientPotion => 100,
            PotionId::PowerPotion | PotionId::ColorlessPotion => 94,
            PotionId::DuplicationPotion | PotionId::GhostInAJar => 90,
            PotionId::Elixir => 84,
            PotionId::BlessingOfTheForge => 84,
            PotionId::StrengthPotion
            | PotionId::DexterityPotion
            | PotionId::SpeedPotion
            | PotionId::SteroidPotion
            | PotionId::EssenceOfSteel
            | PotionId::LiquidBronze
            | PotionId::RegenPotion => 85,
            PotionId::EnergyPotion | PotionId::SwiftPotion => 82,
            _ => 55,
        };

        if shop_need.damage_gap > 0 {
            match potion_id {
                PotionId::FearPotion
                | PotionId::FirePotion
                | PotionId::ExplosivePotion
                | PotionId::AttackPotion => score += 10 + shop_need.damage_gap / 2,
                PotionId::StrengthPotion | PotionId::DuplicationPotion => {
                    score += 10 + shop_need.damage_gap / 3
                }
                _ => {}
            }
        }
        if shop_need.block_gap > 0 {
            match potion_id {
                PotionId::GhostInAJar => score += 14 + shop_need.block_gap / 2,
                PotionId::BlockPotion
                | PotionId::WeakenPotion
                | PotionId::DexterityPotion
                | PotionId::EssenceOfSteel
                | PotionId::LiquidBronze => score += 10 + shop_need.block_gap / 3,
                _ => {}
            }
        }
        if shop_need.control_gap > 0 {
            match potion_id {
                PotionId::WeakenPotion | PotionId::FearPotion => {
                    score += 8 + shop_need.control_gap / 3
                }
                _ => {}
            }
        }
        if self.searing_blow_plan_score(rs, &profile) > 0 {
            match potion_id {
                PotionId::DuplicationPotion => score += 20,
                PotionId::StrengthPotion => score += 12,
                PotionId::FearPotion => score += 10,
                PotionId::BlessingOfTheForge => score += 18,
                _ => {}
            }
        }

        score
    }

    pub(crate) fn reward_potion_score(
        &self,
        rs: &RunState,
        potion_id: crate::content::potions::PotionId,
    ) -> i32 {
        self.shop_potion_score(rs, potion_id)
            .max(base_reward_potion_score(potion_id))
    }

    pub(crate) fn shop_potion_purchase_score(
        &self,
        rs: &RunState,
        shop: &crate::shop::ShopState,
        potion_id: crate::content::potions::PotionId,
        price: i32,
    ) -> i32 {
        let base_score = self.shop_potion_score(rs, potion_id);
        self.shop_purchase_score(
            rs,
            shop,
            price,
            base_score,
            crate::bot::noncombat_families::ShopPurchaseKind::Potion,
        )
    }

    pub(crate) fn best_potion_discard_for_score<F>(
        &self,
        rs: &RunState,
        offered_score: i32,
        mut scorer: F,
    ) -> Option<usize>
    where
        F: FnMut(&Self, &RunState, crate::content::potions::PotionId) -> i32,
    {
        let (discard_idx, kept_score) = rs
            .potions
            .iter()
            .enumerate()
            .filter_map(|(idx, potion)| {
                potion
                    .as_ref()
                    .map(|potion| (idx, scorer(self, rs, potion.id)))
            })
            .min_by_key(|(_, score)| *score)?;

        (offered_score > kept_score).then_some(discard_idx)
    }
}

fn base_reward_potion_score(potion_id: crate::content::potions::PotionId) -> i32 {
    use crate::content::potions::PotionId;

    match potion_id {
        PotionId::AncientPotion => 100,
        PotionId::PowerPotion | PotionId::ColorlessPotion => 94,
        PotionId::DuplicationPotion | PotionId::GhostInAJar => 90,
        PotionId::FruitJuice | PotionId::BloodPotion | PotionId::FairyPotion => 88,
        PotionId::Elixir => 84,
        PotionId::BlessingOfTheForge => 84,
        PotionId::StrengthPotion
        | PotionId::DexterityPotion
        | PotionId::SpeedPotion
        | PotionId::SteroidPotion
        | PotionId::EssenceOfSteel
        | PotionId::LiquidBronze
        | PotionId::RegenPotion => 85,
        PotionId::EnergyPotion | PotionId::SwiftPotion => 82,
        _ => 55,
    }
}
