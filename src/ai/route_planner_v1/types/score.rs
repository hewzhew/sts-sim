use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct NeedVectorV1 {
    pub need_card_rewards: f32,
    pub need_relics: f32,
    pub need_remove: f32,
    pub need_upgrade: f32,
    pub need_heal: f32,
    pub need_shop: f32,
    pub need_event: f32,
    pub need_potion: f32,
    pub can_take_elite: f32,
    pub avoid_damage: f32,
    pub value_flexibility: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RouteScoreTermsV1 {
    pub card_reward: f32,
    pub relic: f32,
    pub remove: f32,
    pub upgrade: f32,
    pub heal: f32,
    pub shop: f32,
    pub event: f32,
    pub potion: f32,
    #[serde(default)]
    pub curse_debt: f32,
    pub hp_loss: f32,
    pub death_risk: f32,
    pub flexibility: f32,
    pub elite_prep: f32,
    pub wing_boots_cost: f32,
    pub forced_path_penalty: f32,
    pub burning_elite_key_value: f32,
}

impl RouteScoreTermsV1 {
    pub fn total(&self) -> f32 {
        self.card_reward
            + self.relic
            + self.remove
            + self.upgrade
            + self.heal
            + self.shop
            + self.event
            + self.potion
            + self.curse_debt
            + self.hp_loss
            + self.death_risk
            + self.flexibility
            + self.elite_prep
            + self.wing_boots_cost
            + self.forced_path_penalty
            + self.burning_elite_key_value
    }
}
