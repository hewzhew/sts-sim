use super::*;

#[test]
fn stable_outcome_key_prefers_visible_card_resolution_over_uuid_fallback() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    baseline
        .zones
        .discard_pile
        .push(CombatCard::new(CardId::Strike, 42));

    let mut variant = baseline.clone();
    variant.zones.discard_pile[0].uuid = 99;

    assert_eq!(
        stable_outcome_key(
            &EngineState::PendingChoice(PendingChoice::GridSelect {
                source_pile: crate::state::core::PileType::Draw,
                candidate_uuids: vec![42],
                min_cards: 0,
                max_cards: 1,
                can_cancel: true,
                reason: crate::state::GridSelectReason::MoveToDrawPile,
            }),
            &baseline,
        ),
        stable_outcome_key(
            &EngineState::PendingChoice(PendingChoice::GridSelect {
                source_pile: crate::state::core::PileType::Draw,
                candidate_uuids: vec![99],
                min_cards: 0,
                max_cards: 1,
                can_cancel: true,
                reason: crate::state::GridSelectReason::MoveToDrawPile,
            }),
            &variant,
        ),
    );
}

#[test]
fn stable_outcome_key_master_deck_grid_select_uses_explicit_master_refs() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));

    let forward = stable_outcome_key(
        &EngineState::PendingChoice(PendingChoice::GridSelect {
            source_pile: crate::state::core::PileType::MasterDeck,
            candidate_uuids: vec![42, 7],
            min_cards: 0,
            max_cards: 1,
            can_cancel: true,
            reason: crate::state::GridSelectReason::MoveToDrawPile,
        }),
        &baseline,
    );
    let reversed = stable_outcome_key(
        &EngineState::PendingChoice(PendingChoice::GridSelect {
            source_pile: crate::state::core::PileType::MasterDeck,
            candidate_uuids: vec![7, 42],
            min_cards: 0,
            max_cards: 1,
            can_cancel: true,
            reason: crate::state::GridSelectReason::MoveToDrawPile,
        }),
        &baseline,
    );

    assert_eq!(forward, reversed);
    let diagnostic = forward.diagnostic_string();
    assert!(diagnostic.contains("master_ref:7") && diagnostic.contains("master_ref:42"));
    assert!(!diagnostic.contains("opaque_uuid"));
}

#[test]
fn stable_outcome_key_master_deck_does_not_resolve_visible_uuid_collisions() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    baseline
        .zones
        .discard_pile
        .push(CombatCard::new(CardId::Strike, 42));

    let mut variant = baseline.clone();
    variant.zones.discard_pile[0].base_damage_mut = 99;

    let choice = PendingChoice::GridSelect {
        source_pile: crate::state::core::PileType::MasterDeck,
        candidate_uuids: vec![42],
        min_cards: 0,
        max_cards: 1,
        can_cancel: true,
        reason: crate::state::GridSelectReason::MoveToDrawPile,
    };

    let baseline_key = pending_choice_key(&choice, &baseline);
    let variant_key = pending_choice_key(&choice, &variant);

    assert_eq!(baseline_key, variant_key);
}

#[test]
fn stable_hand_select_does_not_resolve_cards_outside_hand() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    baseline
        .zones
        .discard_pile
        .push(CombatCard::new(CardId::Strike, 42));

    let mut variant = baseline.clone();
    variant.zones.discard_pile[0].base_damage_mut = 99;

    let choice = PendingChoice::HandSelect {
        candidate_uuids: vec![42],
        min_cards: 0,
        max_cards: 1,
        can_cancel: true,
        reason: crate::state::core::HandSelectReason::Discard,
    };

    assert_eq!(
        pending_choice_key(&choice, &baseline),
        pending_choice_key(&choice, &variant)
    );
}

#[test]
fn stable_scry_select_uses_card_state_and_uuid_fallback() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    baseline
        .zones
        .draw_pile
        .push(CombatCard::new(CardId::Strike, 1));
    baseline
        .zones
        .draw_pile
        .push(CombatCard::new(CardId::Strike, 2));

    let mut variant = baseline.clone();
    variant.zones.draw_pile[1].base_damage_mut = 13;

    assert_ne!(
        stable_outcome_key(
            &EngineState::PendingChoice(PendingChoice::ScrySelect {
                cards: vec![CardId::Strike, CardId::Strike],
                card_uuids: vec![1, 2],
            }),
            &baseline,
        ),
        stable_outcome_key(
            &EngineState::PendingChoice(PendingChoice::ScrySelect {
                cards: vec![CardId::Strike, CardId::Strike],
                card_uuids: vec![1, 2],
            }),
            &variant,
        ),
    );

    let missing = stable_outcome_key(
        &EngineState::PendingChoice(PendingChoice::ScrySelect {
            cards: vec![CardId::Strike],
            card_uuids: vec![99],
        }),
        &baseline,
    );
    assert!(missing.diagnostic_string().contains("scry_ref:99"));
}
