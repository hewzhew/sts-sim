use crate::content::cards::{CardId, CardType};

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardPolicyConfigV1 {
    pub min_auto_pick_score: f32,
    pub min_auto_pick_margin: f32,
    pub late_deck_size: usize,
}

impl Default for CardRewardPolicyConfigV1 {
    fn default() -> Self {
        Self {
            min_auto_pick_score: 7.0,
            min_auto_pick_margin: 2.0,
            late_deck_size: 24,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardDecisionV1 {
    pub action: CardRewardPolicyActionV1,
    pub candidates: Vec<CardRewardCandidateScoreV1>,
    pub label_role: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CardRewardPolicyActionV1 {
    Pick {
        index: usize,
        card: CardId,
        confidence: f32,
        reason: String,
    },
    Stop {
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardCandidateScoreV1 {
    pub index: usize,
    pub card: CardId,
    pub name: &'static str,
    pub card_type: CardType,
    pub score: f32,
    pub terms: CardRewardScoreTermsV1,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CardRewardScoreTermsV1 {
    pub frontload: f32,
    pub block: f32,
    pub draw: f32,
    pub scaling: f32,
    pub aoe: f32,
    pub exhaust_synergy: f32,
    pub rarity: f32,
    pub premium: f32,
    pub risk: f32,
    pub bloat: f32,
}

impl CardRewardScoreTermsV1 {
    pub fn total(&self) -> f32 {
        self.frontload
            + self.block
            + self.draw
            + self.scaling
            + self.aoe
            + self.exhaust_synergy
            + self.rarity
            + self.premium
            + self.risk
            + self.bloat
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DeckNeedsV1 {
    pub deck_size: usize,
    pub need_frontload: f32,
    pub need_block: f32,
    pub need_draw: f32,
    pub need_scaling: f32,
    pub has_exhaust_payoff: bool,
    pub is_late_deck: bool,
}
