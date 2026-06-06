use super::*;

#[test]
fn combat_dominance_key_separates_state_progress_from_resource_vector() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    baseline.zones.hand.push(CombatCard::new(CardId::Strike, 1));

    let mut resource_variant = baseline.clone();
    resource_variant.entities.player.current_hp -= 7;
    resource_variant.entities.player.block += 12;
    assert_eq!(
        combat_dominance_key(&EngineState::CombatPlayerTurn, &baseline),
        combat_dominance_key(&EngineState::CombatPlayerTurn, &resource_variant),
    );
    assert_ne!(
        combat_exact_state_key(&EngineState::CombatPlayerTurn, &baseline),
        combat_exact_state_key(&EngineState::CombatPlayerTurn, &resource_variant),
    );

    let mut max_hp_variant = baseline.clone();
    max_hp_variant.entities.player.max_hp += 1;
    assert_ne!(
        combat_dominance_key(&EngineState::CombatPlayerTurn, &baseline),
        combat_dominance_key(&EngineState::CombatPlayerTurn, &max_hp_variant),
    );

    let mut enemy_variant = baseline.clone();
    enemy_variant.entities.monsters[0].current_hp -= 1;
    assert_ne!(
        combat_dominance_key(&EngineState::CombatPlayerTurn, &baseline),
        combat_dominance_key(&EngineState::CombatPlayerTurn, &enemy_variant),
    );

    let mut queue_variant = baseline.clone();
    queue_variant.queue_action_back(crate::runtime::action::Action::GainEnergy { amount: 1 });
    assert_ne!(
        combat_dominance_key(&EngineState::CombatPlayerTurn, &baseline),
        combat_dominance_key(&EngineState::CombatPlayerTurn, &queue_variant),
    );

    let mut turn_counter_variant = baseline.clone();
    turn_counter_variant.turn.counters.cards_played_this_turn = 1;
    assert_ne!(
        combat_dominance_key(&EngineState::CombatPlayerTurn, &baseline),
        combat_dominance_key(&EngineState::CombatPlayerTurn, &turn_counter_variant),
    );

    let mut uuid_counter_variant = baseline.clone();
    uuid_counter_variant.zones.card_uuid_counter += 1;
    assert_ne!(
        combat_dominance_key(&EngineState::CombatPlayerTurn, &baseline),
        combat_dominance_key(&EngineState::CombatPlayerTurn, &uuid_counter_variant),
    );
}

#[test]
fn combat_keys_ignore_card_draw_observation_events() {
    let baseline = blank_test_combat();
    let mut observed = baseline.clone();
    observed.emit_event(crate::state::selection::DomainEvent::CardDrawn {
        card: crate::state::selection::DomainCardSnapshot {
            id: CardId::DefendB,
            upgrades: 0,
            uuid: 377,
        },
        source: crate::state::selection::DomainEventSource::CombatDraw,
    });

    assert_eq!(
        combat_exact_state_key(&EngineState::CombatPlayerTurn, &baseline),
        combat_exact_state_key(&EngineState::CombatPlayerTurn, &observed),
    );
    assert_eq!(
        combat_dominance_key(&EngineState::CombatPlayerTurn, &baseline),
        combat_dominance_key(&EngineState::CombatPlayerTurn, &observed),
    );
}

#[test]
fn combat_dominance_key_keeps_special_monster_runtime_state() {
    let mut lagavulin = blank_test_combat();
    lagavulin
        .entities
        .monsters
        .push(planned_monster(EnemyId::Lagavulin, 3));

    let mut lagavulin_variant = lagavulin.clone();
    lagavulin_variant.entities.monsters[0].lagavulin.idle_count += 1;
    assert_ne!(
        combat_dominance_key(&EngineState::CombatPlayerTurn, &lagavulin),
        combat_dominance_key(&EngineState::CombatPlayerTurn, &lagavulin_variant),
    );

    let mut guardian = blank_test_combat();
    guardian
        .entities
        .monsters
        .push(planned_monster(EnemyId::TheGuardian, 3));

    let mut guardian_variant = guardian.clone();
    guardian_variant.entities.monsters[0].guardian.damage_taken += 1;
    assert_ne!(
        combat_dominance_key(&EngineState::CombatPlayerTurn, &guardian),
        combat_dominance_key(&EngineState::CombatPlayerTurn, &guardian_variant),
    );
}

#[test]
fn combat_dominance_key_keeps_monster_protocol_observation_state() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    let monster_id = baseline.entities.monsters[0].id;
    baseline.set_monster_protocol_visible_intent(
        monster_id,
        crate::runtime::combat::Intent::Attack { damage: 6, hits: 1 },
    );
    baseline.set_monster_protocol_preview_damage_per_hit(monster_id, 6);

    let mut intent_variant = baseline.clone();
    intent_variant.set_monster_protocol_visible_intent(
        monster_id,
        crate::runtime::combat::Intent::Attack { damage: 7, hits: 1 },
    );
    assert_ne!(
        combat_dominance_key(&EngineState::CombatPlayerTurn, &baseline),
        combat_dominance_key(&EngineState::CombatPlayerTurn, &intent_variant),
    );

    let mut damage_variant = baseline.clone();
    damage_variant.set_monster_protocol_preview_damage_per_hit(monster_id, 7);
    assert_ne!(
        combat_dominance_key(&EngineState::CombatPlayerTurn, &baseline),
        combat_dominance_key(&EngineState::CombatPlayerTurn, &damage_variant),
    );
}
