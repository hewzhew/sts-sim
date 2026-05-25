//! Search-facing combat canaries.
//!
//! This is not a card/relic behavior dump. Keep cases here only when a broken
//! engine transition would make Combat Search consume invalid legal actions,
//! miss a stable boundary, or evaluate the wrong public combat state. Put
//! single-card and single-relic semantics in content/engine tests instead.

use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::potions::{Potion, PotionId};
use crate::content::powers::{store, PowerId};
use crate::runtime::combat::CombatCard;
use crate::sim::combat::CombatTerminal;
use crate::state::core::{
    ClientInput, EngineState, GridSelectReason, HandSelectReason, PendingChoice, PileType,
};

mod cards;
mod potions;
mod support;
mod targets;

use support::*;

fn card_snapshots(cards: &[CombatCard]) -> Vec<(CardId, u32)> {
    cards.iter().map(|card| (card.id, card.uuid)).collect()
}
