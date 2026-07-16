use serde::{Deserialize, Serialize};

use crate::ai::combat_policy_v1::CombatPolicyCardV1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatPublicPendingChoiceKindV1 {
    HandSelect,
    GridSelect,
    DiscoverySelect,
    ScrySelect,
    CardRewardSelect,
    ForeignInfluenceSelect,
    ChooseOneSelect,
    StanceChoice,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatPublicPileV1 {
    Draw,
    Discard,
    Exhaust,
    Hand,
    Limbo,
    MasterDeck,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatPublicHandSelectionReasonV1 {
    Exhaust,
    Discard,
    Retain,
    PutOnDrawPile,
    PutToBottomOfDraw,
    Setup,
    Copy { amount: u8 },
    Nightmare { amount: u8 },
    Upgrade,
    GamblingChip,
    Recycle,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatPublicGridSelectionReasonV1 {
    MoveToDrawPile,
    Exhume { upgrade: bool },
    DrawPileToHand,
    SkillFromDeckToHand,
    AttackFromDeckToHand,
    DiscardToHand,
    DiscardToHandNoCostChange,
    DiscardToHandRetain,
    Omniscience { play_amount: u8 },
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum CombatPublicCardSelectionContextV1 {
    Hand {
        reason: CombatPublicHandSelectionReasonV1,
    },
    Grid {
        source_pile: CombatPublicPileV1,
        reason: CombatPublicGridSelectionReasonV1,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicCardMultiplicityV1 {
    pub card: CombatPolicyCardV1,
    pub count: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicGeneratedCardOptionV1 {
    pub option_index: usize,
    pub card_id: String,
    pub upgrades: u8,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatPublicGeneratedChoiceKindV1 {
    Discovery,
    CardReward,
    ForeignInfluence,
    ChooseOne,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatPublicCardDestinationV1 {
    Hand,
    DrawPileRandom,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatPublicStanceV1 {
    Wrath,
    Calm,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatPublicPendingChoiceV1 {
    HandSelect {
        context: CombatPublicCardSelectionContextV1,
        min_cards: u8,
        max_cards: u8,
        can_cancel: bool,
        candidates: Vec<CombatPublicCardMultiplicityV1>,
    },
    GridSelect {
        context: CombatPublicCardSelectionContextV1,
        min_cards: u8,
        max_cards: u8,
        can_cancel: bool,
        candidates: Vec<CombatPublicCardMultiplicityV1>,
    },
    DiscoverySelect {
        amount: u8,
        can_skip: bool,
        options: Vec<CombatPublicGeneratedCardOptionV1>,
    },
    ScrySelect {
        revealed_cards: Vec<CombatPolicyCardV1>,
    },
    CardRewardSelect {
        destination: CombatPublicCardDestinationV1,
        can_skip: bool,
        options: Vec<CombatPublicGeneratedCardOptionV1>,
    },
    ForeignInfluenceSelect {
        upgraded: bool,
        options: Vec<CombatPublicGeneratedCardOptionV1>,
    },
    ChooseOneSelect {
        options: Vec<CombatPublicGeneratedCardOptionV1>,
    },
    StanceChoice {
        options: Vec<CombatPublicStanceV1>,
    },
}
