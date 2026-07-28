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

#[test]
fn semantic_exact_identity_v2_has_a_fixed_cross_process_fixture() {
    let mut combat = blank_test_combat();
    combat
        .entities
        .player
        .add_relic(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::PenNib,
        ));
    combat
        .entities
        .player
        .add_relic(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::Calipers,
        ));

    assert_eq!(
        super::combat_exact_state_hash_v2(&EngineState::CombatPlayerTurn, &combat),
        "5324bee97e289f32e069db1df9e586ce75c2c4f7657861f0089a48eb60ea361a"
    );
}

#[test]
fn exact_identity_ignores_and_rebuilds_the_derived_relic_dispatch_cache() {
    let mut combat = blank_test_combat();
    combat
        .entities
        .player
        .add_relic(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::PenNib,
        ));
    let mut inconsistent_cache = combat.clone();
    std::sync::Arc::make_mut(&mut inconsistent_cache.entities.player.relic_buses)
        .on_use_card
        .push(999);

    let engine = EngineState::CombatPlayerTurn;
    assert_eq!(
        combat_exact_state_key(&engine, &combat),
        combat_exact_state_key(&engine, &inconsistent_cache),
        "a derived dispatch cache is not independent exact combat state"
    );
    assert_eq!(
        super::combat_exact_state_hash_v2(&engine, &combat),
        super::combat_exact_state_hash_v2(&engine, &inconsistent_cache),
        "durable V2 identity must rebuild its compatibility projection from relics"
    );
}
