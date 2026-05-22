use crate::content::cards::{CardId, CardType};
use crate::state::selection::{
    SelectionConstraint, SelectionReason, SelectionRequest, SelectionScope, SelectionTargetRef,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum PendingChoice {
    GridSelect {
        source_pile: PileType,
        candidate_uuids: Vec<u32>,
        min_cards: u8,
        max_cards: u8,
        can_cancel: bool,
        reason: GridSelectReason,
    },
    HandSelect {
        candidate_uuids: Vec<u32>,
        min_cards: u8,
        max_cards: u8,
        can_cancel: bool,
        reason: HandSelectReason,
    },
    DiscoverySelect(DiscoveryChoiceState),
    ScrySelect {
        cards: Vec<CardId>,
        card_uuids: Vec<u32>,
    },
    /// Unified card reward selection (NilrysCodex, Toolbox, etc.)
    /// Player picks 1 card from `cards`. Card goes to `destination`.
    /// If `can_skip`, player can Cancel to skip without picking.
    CardRewardSelect {
        cards: Vec<CardId>,
        destination: crate::runtime::action::CardDestination,
        can_skip: bool,
    },
    /// Java Watcher `ForeignInfluenceAction`: pick one generated any-color
    /// attack. This is not `DiscoveryAction`; it has different RNG and resume
    /// semantics.
    ForeignInfluenceSelect {
        cards: Vec<CardId>,
        upgraded: bool,
    },
    ChooseOneSelect {
        choices: Vec<ChooseOneCardChoice>,
    },
    /// StancePotion: player chooses Wrath or Calm. Index 0 = Wrath, 1 = Calm.
    StanceChoice,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum PileType {
    Draw,
    Discard,
    Exhaust,
    Hand,
    Limbo,
    MasterDeck,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum HandSelectReason {
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DiscoveryChoiceState {
    pub cards: Vec<CardId>,
    pub colorless: bool,
    pub card_type: Option<CardType>,
    pub amount: u8,
    pub can_skip: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ChooseOneCardChoice {
    pub card_id: CardId,
    pub upgrades: u8,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum HandSelectFilter {
    Any,
    Upgradeable,
    AttackOrPower,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum GridSelectReason {
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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum GridSelectFilter {
    Any,
    NonExhume,
    Skill,
    Attack,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum TargetValidation {
    AnyEnemy,
    AnyMonster,
}

impl From<HandSelectReason> for SelectionReason {
    fn from(value: HandSelectReason) -> Self {
        match value {
            HandSelectReason::Exhaust => SelectionReason::Exhaust,
            HandSelectReason::Discard => SelectionReason::Discard,
            HandSelectReason::Retain => SelectionReason::Retain,
            HandSelectReason::PutOnDrawPile => SelectionReason::PutOnDrawPile,
            HandSelectReason::PutToBottomOfDraw => SelectionReason::PutToBottomOfDraw,
            HandSelectReason::Setup => SelectionReason::Setup,
            HandSelectReason::Copy { .. } => SelectionReason::Copy,
            HandSelectReason::Nightmare { .. } => SelectionReason::Nightmare,
            HandSelectReason::Upgrade => SelectionReason::Upgrade,
            HandSelectReason::GamblingChip => SelectionReason::GamblingChip,
            HandSelectReason::Recycle => SelectionReason::Recycle,
        }
    }
}

impl From<GridSelectReason> for SelectionReason {
    fn from(value: GridSelectReason) -> Self {
        match value {
            GridSelectReason::MoveToDrawPile => SelectionReason::MoveToDrawPile,
            GridSelectReason::Exhume { .. } => SelectionReason::Exhume,
            GridSelectReason::DrawPileToHand => SelectionReason::DrawPileToHand,
            GridSelectReason::SkillFromDeckToHand => SelectionReason::SkillFromDeckToHand,
            GridSelectReason::AttackFromDeckToHand => SelectionReason::AttackFromDeckToHand,
            GridSelectReason::DiscardToHand => SelectionReason::DiscardToHand,
            GridSelectReason::DiscardToHandNoCostChange => {
                SelectionReason::DiscardToHandNoCostChange
            }
            GridSelectReason::DiscardToHandRetain => SelectionReason::DiscardToHandRetain,
            GridSelectReason::Omniscience { .. } => SelectionReason::Omniscience,
        }
    }
}

impl PendingChoice {
    pub fn selection_request(&self) -> Option<SelectionRequest> {
        match self {
            PendingChoice::HandSelect {
                candidate_uuids,
                min_cards,
                max_cards,
                can_cancel,
                reason,
            } => Some(SelectionRequest {
                scope: SelectionScope::Hand,
                reason: (*reason).into(),
                constraint: SelectionConstraint::from_bounds(
                    *min_cards as usize,
                    *max_cards as usize,
                    candidate_uuids.len(),
                ),
                can_cancel: *can_cancel,
                targets: candidate_uuids
                    .iter()
                    .copied()
                    .map(SelectionTargetRef::CardUuid)
                    .collect(),
            }),
            PendingChoice::GridSelect {
                candidate_uuids,
                min_cards,
                max_cards,
                can_cancel,
                reason,
                ..
            } => Some(SelectionRequest {
                scope: SelectionScope::Grid,
                reason: (*reason).into(),
                constraint: SelectionConstraint::from_bounds(
                    *min_cards as usize,
                    *max_cards as usize,
                    candidate_uuids.len(),
                ),
                can_cancel: *can_cancel,
                targets: candidate_uuids
                    .iter()
                    .copied()
                    .map(SelectionTargetRef::CardUuid)
                    .collect(),
            }),
            _ => None,
        }
    }
}
