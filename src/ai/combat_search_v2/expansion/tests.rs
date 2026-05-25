use super::*;
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::potions::{Potion, PotionId};
use crate::runtime::combat::CombatCard;
use crate::test_support::{blank_test_combat, test_monster};

#[test]
fn targeted_card_actions_share_a_diagnostic_fanout_group() {
    let mut combat = blank_test_combat();
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 42)];
    let mut first = test_monster(EnemyId::JawWorm);
    first.id = 11;
    let mut second = test_monster(EnemyId::Cultist);
    second.id = 12;
    combat.entities.monsters = vec![first, second];
    let choices = vec![
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(11),
            },
        ),
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(12),
            },
        ),
        CombatActionChoice::from_input(&combat, ClientInput::EndTurn),
    ];

    let summary = summarize_action_expansion(&EngineState::CombatPlayerTurn, &combat, &choices);

    assert_eq!(summary.action_count, 3);
    assert_eq!(summary.group_count, 2);
    assert!(summary.groups.iter().any(|group| {
        group.key.kind == ActionExpansionKind::PlayCard && group.action_count == 2
    }));
}

#[test]
fn potion_policy_filtered_actions_are_observed_after_filtering() {
    let mut combat = blank_test_combat();
    combat.entities.potions = vec![Some(Potion::new(PotionId::FirePotion, 7))];
    let legal = vec![
        CombatActionChoice::from_input(
            &combat,
            ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            },
        ),
        CombatActionChoice::from_input(&combat, ClientInput::EndTurn),
    ];
    let filtered = filtered_legal_actions(legal, CombatSearchV2PotionPolicy::Never, &combat);

    let summary = summarize_action_expansion(&EngineState::CombatPlayerTurn, &combat, &filtered);

    assert_eq!(summary.action_count, 1);
    assert!(summary.groups.iter().all(|group| {
        group.key.kind != ActionExpansionKind::UsePotion
            && group.key.kind != ActionExpansionKind::DiscardPotion
    }));
}

#[test]
fn expansion_collector_reports_largest_groups_without_changing_behavior() {
    let mut combat = blank_test_combat();
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 42)];
    let mut first = test_monster(EnemyId::JawWorm);
    first.id = 11;
    let mut second = test_monster(EnemyId::Cultist);
    second.id = 12;
    combat.entities.monsters = vec![first, second];
    let choices = vec![
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(11),
            },
        ),
        CombatActionChoice::from_input(
            &combat,
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(12),
            },
        ),
    ];
    let summary = summarize_action_expansion(&EngineState::CombatPlayerTurn, &combat, &choices);
    let mut collector = ActionExpansionDiagnosticsCollector::default();

    collector.observe(&summary);
    let report = collector.finish();

    assert_eq!(
        report.behavioral_effect,
        "diagnostic_only_search_expansion_unchanged"
    );
    assert_eq!(report.total_atomic_actions, 2);
    assert_eq!(report.total_fanout_groups, 1);
    assert_eq!(report.max_group_size, 2);
    assert_eq!(report.largest_groups[0].kind, "play_card");
}
