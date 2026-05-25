use super::*;
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::runtime::combat::CombatCard;
use crate::test_support::{blank_test_combat, test_monster};

#[test]
fn compresses_duplicate_starter_basic_cards_to_same_target() {
    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::JawWorm);
    monster.id = 1;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::Strike, 11),
    ];
    let choices = vec![
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        ),
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 1,
                target: Some(1),
            },
        ),
    ];

    let result = compress_equivalent_actions(&EngineState::CombatPlayerTurn, &combat, choices);

    assert_eq!(result.choices.len(), 1);
    assert_eq!(result.choices[0].original_action_id, 0);
    assert_eq!(result.summary.actions_removed(), 1);
    assert_eq!(
        result.summary.groups[0].removed_original_action_ids,
        vec![1]
    );
}

#[test]
fn keeps_duplicate_starter_basic_cards_split_by_target() {
    let mut combat = blank_test_combat();
    let mut first = test_monster(EnemyId::LouseNormal);
    first.id = 1;
    let mut second = test_monster(EnemyId::LouseNormal);
    second.id = 2;
    combat.entities.monsters = vec![first, second];
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::Strike, 11),
    ];
    let choices = vec![
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        ),
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 1,
                target: Some(1),
            },
        ),
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(2),
            },
        ),
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 1,
                target: Some(2),
            },
        ),
    ];

    let result = compress_equivalent_actions(&EngineState::CombatPlayerTurn, &combat, choices);

    assert_eq!(result.choices.len(), 2);
    assert_eq!(result.summary.actions_removed(), 2);
    assert_eq!(result.summary.groups.len(), 2);
}

#[test]
fn does_not_compress_non_starter_basic_duplicates() {
    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::JawWorm);
    monster.id = 1;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![
        CombatCard::new(CardId::TwinStrike, 10),
        CombatCard::new(CardId::TwinStrike, 11),
    ];
    let choices = vec![
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        ),
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 1,
                target: Some(1),
            },
        ),
    ];

    let result = compress_equivalent_actions(&EngineState::CombatPlayerTurn, &combat, choices);

    assert_eq!(result.choices.len(), 2);
    assert_eq!(result.summary.actions_removed(), 0);
}

#[test]
fn does_not_compress_cards_with_different_runtime_state() {
    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::JawWorm);
    monster.id = 1;
    combat.entities.monsters = vec![monster];
    let mut free = CombatCard::new(CardId::Strike, 10);
    free.free_to_play_once = true;
    combat.zones.hand = vec![free, CombatCard::new(CardId::Strike, 11)];
    let choices = vec![
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        ),
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 1,
                target: Some(1),
            },
        ),
    ];

    let result = compress_equivalent_actions(&EngineState::CombatPlayerTurn, &combat, choices);

    assert_eq!(result.choices.len(), 2);
    assert_eq!(result.summary.actions_removed(), 0);
}

#[test]
fn compresses_single_card_pending_grid_selection_for_runtime_identical_cards() {
    let mut combat = blank_test_combat();
    combat.zones.discard_pile = vec![
        CombatCard::new(CardId::Slimed, 10),
        CombatCard::new(CardId::Slimed, 11),
        CombatCard::new(CardId::Strike, 12),
    ];
    let engine = EngineState::PendingChoice(crate::state::core::PendingChoice::GridSelect {
        source_pile: crate::state::core::PileType::Discard,
        candidate_uuids: vec![10, 11, 12],
        min_cards: 1,
        max_cards: 1,
        can_cancel: false,
        reason: crate::state::core::GridSelectReason::MoveToDrawPile,
    });
    let choices = vec![
        CombatActionChoice::from_input(&combat, ClientInput::SubmitGridSelect(vec![10])),
        CombatActionChoice::from_input(&combat, ClientInput::SubmitGridSelect(vec![11])),
        CombatActionChoice::from_input(&combat, ClientInput::SubmitGridSelect(vec![12])),
    ];

    let result = compress_equivalent_actions(&engine, &combat, choices);

    assert_eq!(result.choices.len(), 2);
    assert_eq!(result.summary.actions_removed(), 1);
    assert_eq!(
        result.summary.groups[0].key.kind,
        ActionEquivalenceKind::SingleCardPendingChoiceSelection
    );
}

#[test]
fn keeps_pending_grid_multi_select_atomic() {
    let mut combat = blank_test_combat();
    combat.zones.discard_pile = vec![
        CombatCard::new(CardId::Slimed, 10),
        CombatCard::new(CardId::Slimed, 11),
    ];
    let engine = EngineState::PendingChoice(crate::state::core::PendingChoice::GridSelect {
        source_pile: crate::state::core::PileType::Discard,
        candidate_uuids: vec![10, 11],
        min_cards: 2,
        max_cards: 2,
        can_cancel: false,
        reason: crate::state::core::GridSelectReason::DrawPileToHand,
    });
    let choices = vec![CombatActionChoice::from_input(
        &combat,
        ClientInput::SubmitGridSelect(vec![10, 11]),
    )];

    let result = compress_equivalent_actions(&engine, &combat, choices);

    assert_eq!(result.choices.len(), 1);
    assert_eq!(result.summary.actions_removed(), 0);
}

#[test]
fn collector_reports_removed_duplicate_actions() {
    let mut collector = ActionEquivalenceDiagnosticsCollector::default();
    let summary = ActionEquivalenceSummary {
        atomic_actions_in: 3,
        representative_actions_out: 2,
        groups: vec![ActionEquivalenceGroupSummary {
            key: ActionEquivalenceKey {
                kind: ActionEquivalenceKind::StarterBasicPlayCard,
                signature: "play_card/starter_basic/card:Strike_R+0/target:monster_slot:0"
                    .to_string(),
            },
            representative_original_action_id: 0,
            removed_original_action_ids: vec![1],
        }],
    };

    collector.observe(&summary);
    let report = collector.finish();

    assert_eq!(
        report.behavioral_effect,
        "safe_representative_child_generation_for_proven_duplicate_actions_only"
    );
    assert_eq!(report.states_observed, 1);
    assert_eq!(report.states_compressed, 1);
    assert_eq!(report.actions_removed, 1);
    assert_eq!(report.max_group_size, 2);
    assert_eq!(report.largest_groups.len(), 1);
}
