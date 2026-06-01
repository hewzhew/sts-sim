use super::*;

#[test]
fn ordering_collector_reports_role_counts_without_action_tree() {
    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::Cultist);
    monster.current_hp = 6;
    monster.max_hp = 6;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
    let ordered = order_action_choices(
        &EngineState::CombatPlayerTurn,
        &combat,
        vec![
            CombatActionChoice::from_input(&combat, ClientInput::EndTurn),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(1),
                },
            ),
        ],
    );
    let mut collector = ActionOrderingDiagnosticsCollector::default();

    collector.observe(&ordered.summary);
    let report = collector.finish();

    assert_eq!(
        report.behavioral_effect,
        "child_generation_order_only_no_prune_no_merge"
    );
    assert_eq!(report.states_reordered, 1);
    assert_eq!(report.max_position_shift, 1);
    assert_eq!(report.largest_reorders.len(), 1);
    assert_eq!(report.largest_reorders[0].first_role, "lethal_card");
}

#[test]
fn ordering_collector_caps_action_effect_samples() {
    let mut combat = blank_test_combat();
    let mut nob = test_monster(EnemyId::GremlinNob);
    nob.id = 1;
    combat.entities.monsters = vec![nob];
    combat.entities.power_db.insert(
        1,
        vec![Power {
            power_type: PowerId::Anger,
            instance_id: None,
            amount: 2,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );
    combat.zones.hand = vec![
        CombatCard::new(CardId::Defend, 10),
        CombatCard::new(CardId::Strike, 11),
    ];
    let ordered = order_action_choices(
        &EngineState::CombatPlayerTurn,
        &combat,
        vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 0,
                    target: None,
                },
            ),
            CombatActionChoice::from_input(
                &combat,
                ClientInput::PlayCard {
                    card_index: 1,
                    target: Some(1),
                },
            ),
        ],
    );
    let mut collector = ActionOrderingDiagnosticsCollector::default();

    for _ in 0..(ACTION_EFFECT_SAMPLE_LIMIT + 3) {
        collector.observe(&ordered.summary);
    }
    let report = collector.finish();

    assert_eq!(
        report.action_effect_actions,
        (ACTION_EFFECT_SAMPLE_LIMIT + 3) as u64
    );
    assert_eq!(
        report.action_effect_samples.len(),
        ACTION_EFFECT_SAMPLE_LIMIT
    );
    assert!(report.action_effect_samples[0].enemy_strength_gain > 0);
    assert!(report.action_effect_samples[0].reactive_risk_score > 0);
}

#[test]
fn ordering_collector_counts_phase_hint_actions() {
    let mut combat = blank_test_combat();
    let mut lagavulin = test_monster(EnemyId::Lagavulin);
    lagavulin.id = 1;
    lagavulin.lagavulin.is_out = false;
    combat.entities.monsters = vec![lagavulin];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
    let ordered = order_action_choices(
        &EngineState::CombatPlayerTurn,
        &combat,
        vec![CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        )],
    );
    let mut collector = ActionOrderingDiagnosticsCollector::default();

    collector.observe(&ordered.summary);
    let report = collector.finish();

    assert_eq!(report.phase_action_hint_actions, 1);
}
