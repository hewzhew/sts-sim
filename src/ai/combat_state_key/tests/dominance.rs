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
}
