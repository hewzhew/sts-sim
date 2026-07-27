use super::{
    combat_dominance_key, combat_exact_state_key, combat_exact_state_key_profiled_v1,
    diagnostic_outcome_key, pending_choice::pending_choice_key, stable_dominance_bucket_key,
    stable_outcome_key,
};
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::potions::{Potion, PotionId};
use crate::runtime::combat::{CombatCard, QueuedCardPlay, QueuedCardSource};
use crate::state::core::PendingChoice;
use crate::state::EngineState;
use crate::test_support::{blank_test_combat, planned_monster};

mod cards;
mod dominance;
mod monster;
mod pending_choice;
mod postcombat;
mod stable;

#[test]
fn profiled_exact_key_is_identical_to_the_production_builder() {
    let combat = blank_test_combat();
    let engine = EngineState::CombatPlayerTurn;

    let ordinary = combat_exact_state_key(&engine, &combat);
    let (profiled, _) = combat_exact_state_key_profiled_v1(&engine, &combat);

    assert_eq!(profiled, ordinary);
}
