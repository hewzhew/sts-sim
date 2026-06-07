use crate::content::cards::CardId;
use crate::state::events::EventEffect;

#[derive(Clone, Debug, PartialEq)]
pub struct NeowDecisionInputV1 {
    pub player_class: String,
    pub map: NeowMapFeaturesV1,
    pub choices: Vec<NeowChoiceInputV1>,
    pub config: NeowGuidanceConfigV1,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NeowMapFeaturesV1 {
    pub early_shop_available: bool,
    pub shop_before_first_elite: bool,
    pub early_elite_available: bool,
    pub lament_elite_snipe_candidate: bool,
    pub path_flexibility: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NeowGuidanceConfigV1 {
    pub early_shop_gold_bonus: f32,
    pub shop_before_elite_gold_bonus: f32,
    pub lament_elite_snipe_bonus: f32,
    pub early_elite_potion_bonus: f32,
    pub early_elite_immediate_bonus: f32,
    pub ironclad_boss_swap_penalty: f32,
    pub low_flex_boss_swap_penalty: f32,
    pub boss_swap_variance_penalty: f32,
}

impl Default for NeowGuidanceConfigV1 {
    fn default() -> Self {
        Self {
            early_shop_gold_bonus: 2.5,
            shop_before_elite_gold_bonus: 1.0,
            lament_elite_snipe_bonus: 5.0,
            early_elite_potion_bonus: 1.7,
            early_elite_immediate_bonus: 0.7,
            ironclad_boss_swap_penalty: 1.6,
            low_flex_boss_swap_penalty: 1.2,
            boss_swap_variance_penalty: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NeowChoiceInputV1 {
    pub index: usize,
    pub label: String,
    pub effects: Vec<EventEffect>,
    pub class: NeowChoiceClassV1,
}

impl NeowChoiceInputV1 {
    pub fn from_effects(index: usize, label: impl Into<String>, effects: Vec<EventEffect>) -> Self {
        let class = super::choice_guidance::classify_neow_choice(&effects);
        Self {
            index,
            label: label.into(),
            effects,
            class,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NeowChoiceClassV1 {
    Lament,
    MaxHp,
    Gold,
    Potions,
    CardReward,
    RareCardReward,
    ColorlessCardReward,
    CommonRelic,
    RareRelic,
    BossSwap,
    Remove,
    Upgrade,
    Transform,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NeowDecisionTraceV1 {
    pub label_role: &'static str,
    pub map: NeowMapFeaturesV1,
    pub candidates: Vec<NeowCandidateTraceV1>,
    pub selected_index: Option<usize>,
}

impl NeowDecisionTraceV1 {
    pub fn selected(&self) -> Option<&NeowCandidateTraceV1> {
        self.selected_index
            .and_then(|index| self.candidates.get(index))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NeowCandidateTraceV1 {
    pub index: usize,
    pub label: String,
    pub class: NeowChoiceClassV1,
    pub terms: NeowScoreTermsV1,
    pub total: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NeowRunSelectionDecisionV1 {
    pub command: String,
    pub selected_deck_indices: Vec<usize>,
    pub selected_cards: Vec<(CardId, u8)>,
    pub selection_mode: &'static str,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NeowScoreTermsV1 {
    pub immediate_power: f32,
    pub first_elite_security: f32,
    pub boss_matchup_help: f32,
    pub shop_convertibility: f32,
    pub path_flexibility: f32,
    pub character_synergy: f32,
    pub downside_cost: f32,
    pub variance_penalty: f32,
}

impl NeowScoreTermsV1 {
    pub fn total(self) -> f32 {
        self.immediate_power
            + self.first_elite_security
            + self.boss_matchup_help
            + self.shop_convertibility
            + self.path_flexibility
            + self.character_synergy
            + self.downside_cost
            + self.variance_penalty
    }
}
