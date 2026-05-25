use super::*;
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::runtime::combat::{CombatCard, Power, PowerPayload};
use crate::test_support::blank_test_combat;

#[test]
fn state_value_prefers_survival_before_future_draw_quality() {
    let mut safe = test_node();
    safe.combat.entities.player.current_hp = 20;
    safe.combat.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 11)];

    let mut flashy = test_node();
    flashy.combat.entities.player.current_hp = 10;
    flashy.combat.zones.draw_pile = vec![CombatCard::new(CardId::Carnage, 12)];

    assert!(combat_search_state_value(&safe) > combat_search_state_value(&flashy));
}

#[test]
fn state_value_accounts_for_pending_split_phase_debt() {
    let mut raw_progress = test_node();
    let mut raw_slime = crate::test_support::test_monster(EnemyId::AcidSlimeL);
    raw_slime.id = 12;
    raw_slime.current_hp = 32;
    raw_slime.max_hp = 65;
    raw_slime.set_planned_move_id(1);
    raw_progress.combat.entities.monsters = vec![raw_slime];

    let mut split_pending = test_node();
    let mut split_slime = crate::test_support::test_monster(EnemyId::AcidSlimeL);
    split_slime.id = 13;
    split_slime.current_hp = 31;
    split_slime.max_hp = 65;
    split_slime.set_planned_move_id(3);
    split_pending.combat.entities.monsters = vec![split_slime];
    split_pending.combat.entities.power_db.insert(
        13,
        vec![Power {
            power_type: PowerId::Split,
            instance_id: None,
            amount: -1,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    assert!(combat_search_state_value(&raw_progress) > combat_search_state_value(&split_pending));
}

#[test]
fn state_value_accounts_for_post_phase_enemy_block() {
    let mut open = test_node();
    let mut open_guardian = crate::test_support::test_monster(EnemyId::TheGuardian);
    open_guardian.id = 20;
    open_guardian.current_hp = 180;
    open_guardian.max_hp = 240;
    open_guardian.guardian.is_open = true;
    open.combat.entities.monsters = vec![open_guardian];

    let mut defensive = test_node();
    let mut defensive_guardian = crate::test_support::test_monster(EnemyId::TheGuardian);
    defensive_guardian.id = 20;
    defensive_guardian.current_hp = 180;
    defensive_guardian.max_hp = 240;
    defensive_guardian.block = 20;
    defensive_guardian.guardian.is_open = false;
    defensive.combat.entities.monsters = vec![defensive_guardian];

    assert!(combat_search_state_value(&open) > combat_search_state_value(&defensive));
}

#[test]
fn state_value_accounts_for_gremlin_nob_enrage_pressure() {
    let mut calm = test_node();
    let mut calm_nob = crate::test_support::test_monster(EnemyId::GremlinNob);
    calm_nob.id = 30;
    calm_nob.current_hp = 70;
    calm_nob.max_hp = 85;
    calm.combat.entities.monsters = vec![calm_nob];

    let mut angry = calm.clone();
    angry.combat.entities.power_db.insert(
        30,
        vec![Power {
            power_type: PowerId::Anger,
            instance_id: None,
            amount: 3,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    assert!(combat_search_state_value(&calm) > combat_search_state_value(&angry));
}

#[test]
fn state_value_accounts_for_sentry_dazed_pressure() {
    let mut attacking = test_node();
    let mut sentry_attack = crate::test_support::test_monster(EnemyId::Sentry);
    sentry_attack.id = 40;
    sentry_attack.current_hp = 35;
    sentry_attack.max_hp = 40;
    sentry_attack.set_planned_move_id(1);
    attacking.combat.entities.monsters = vec![sentry_attack];

    let mut dazed = test_node();
    let mut sentry_dazed = crate::test_support::test_monster(EnemyId::Sentry);
    sentry_dazed.id = 40;
    sentry_dazed.current_hp = 35;
    sentry_dazed.max_hp = 40;
    sentry_dazed.set_planned_move_id(3);
    dazed.combat.entities.monsters = vec![sentry_dazed];

    assert!(combat_search_state_value(&attacking) > combat_search_state_value(&dazed));
}

#[test]
fn core_value_facts_feed_state_value_and_report() {
    let mut node = test_node();
    let mut guardian = crate::test_support::test_monster(EnemyId::TheGuardian);
    guardian.id = 20;
    guardian.current_hp = 180;
    guardian.max_hp = 240;
    guardian.block = 20;
    guardian.guardian.is_open = false;
    node.combat.entities.monsters = vec![guardian];

    let facts = combat_search_core_value_facts(&node.engine, &node.combat);
    let state_value = combat_search_state_value(&node);
    let report = combat_search_frontier_value_report(&node);

    assert_eq!(
        state_value.phase_adjusted_enemy_effort_progress,
        -facts
            .phase_profile
            .enemy_phase
            .phase_adjusted_living_enemy_effort
    );
    assert_eq!(
        report.phase_adjusted_enemy_effort,
        facts
            .phase_profile
            .enemy_phase
            .phase_adjusted_living_enemy_effort
    );
    assert_eq!(
        report.guardian_defensive_block,
        facts.phase_profile.enemy_phase.guardian_defensive_block
    );
    assert_eq!(
        report.phase_profile.guardian_defensive_count,
        facts.phase_profile.enemy_mechanics.guardian_defensive_count
    );
    assert_eq!(
        report.guardian_mode_shift_pending_count,
        facts
            .phase_profile
            .enemy_mechanics
            .guardian_mode_shift_pending_count
    );
}

fn test_node() -> SearchNode {
    SearchNode {
        engine: EngineState::CombatPlayerTurn,
        combat: blank_test_combat(),
        actions: Vec::new(),
        turn_prefix: TurnPrefixState::default(),
        initial_hp: 80,
        potions_used: 0,
        potions_discarded: 0,
        cards_played: 0,
        potion_tactical_priority: 0,
        last_turn_branch_priority: 0,
        rollout_estimate: RolloutNodeEstimate::unevaluated(),
    }
}
