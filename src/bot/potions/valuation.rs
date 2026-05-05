#![cfg(test)]

use crate::bot::agent::Agent;
use crate::state::run::RunState;

#[allow(dead_code)]
impl Agent {
    pub(crate) fn shop_potion_score(
        &self,
        rs: &RunState,
        potion_id: crate::content::potions::PotionId,
    ) -> i32 {
        crate::bot::shared::score_shop_potion(rs, potion_id)
    }

    pub(crate) fn reward_potion_score(
        &self,
        rs: &RunState,
        potion_id: crate::content::potions::PotionId,
    ) -> i32 {
        crate::bot::shared::score_reward_potion(rs, potion_id)
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
