use sts_simulator::content::cards::CardId;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::core::ClientInput;
use sts_simulator::state::EngineState;
use sts_simulator::test_support::{blank_test_combat, planned_monster};
use sts_simulator::verification::decision_env::{ActionId, DecisionId};
use sts_simulator::verification::neutral_engine_query::{
    NeutralEngineQueryService, NeutralQueryKind, SearchExecutionContext,
};
use sts_simulator::verification::search_policy::{Exactness, SearchKind};

fn card(id: CardId, uuid: u32) -> CombatCard {
    CombatCard::new(id, uuid)
}

fn cultist_combat_with_hand(hand: &[CardId]) -> sts_simulator::runtime::combat::CombatState {
    let mut combat = blank_test_combat();
    combat.turn.energy = 3;
    combat.turn.turn_count = 2;
    let mut cultist = planned_monster(EnemyId::Cultist, 1);
    cultist.current_hp = 48;
    combat.entities.monsters.push(cultist);
    for (index, id) in hand.iter().enumerate() {
        combat.zones.hand.push(card(*id, index as u32 + 1));
    }
    combat
}

fn decision_id() -> DecisionId {
    DecisionId {
        episode_id: "neutral-query-test".to_string(),
        step_index: 0,
        decision_type: "combat".to_string(),
    }
}

#[test]
fn neutral_engine_query_forces_candidate_without_legacy_or_exact_turn() {
    let combat = cultist_combat_with_hand(&[CardId::Strike, CardId::Defend]);
    let context = SearchExecutionContext::new(
        decision_id(),
        EngineState::CombatPlayerTurn,
        combat,
        vec![
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
            ClientInput::PlayCard {
                card_index: 1,
                target: None,
            },
        ],
    );
    let service = NeutralEngineQueryService::default();
    let result = service.force_to_stable(&context, ActionId(0)).unwrap();

    assert_eq!(result.query_kind, NeutralQueryKind::StableTransition);
    assert!(!result.truncated);
    assert_eq!(result.branch_effect.enemy_hp_removed, 6);
    assert_eq!(result.branch_effect.enemies_killed, 0);
    assert_eq!(result.after.energy, 2);

    let evidence = result.to_search_evidence("strike-stable");
    assert_eq!(evidence.exactness, Exactness::Exact);
    assert!(matches!(
        evidence.search_kind,
        SearchKind::NeutralStableTransition { .. }
    ));
    assert_eq!(
        evidence
            .payload
            .get("schema_version")
            .and_then(|value| value.as_str()),
        Some("neutral_engine_query_v0")
    );
}

#[test]
fn branch_effect_signature_compresses_by_observed_engine_delta() {
    let combat = cultist_combat_with_hand(&[CardId::Strike, CardId::Strike, CardId::Defend]);
    let context = SearchExecutionContext::new(
        decision_id(),
        EngineState::CombatPlayerTurn,
        combat,
        vec![
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
            ClientInput::PlayCard {
                card_index: 1,
                target: Some(1),
            },
            ClientInput::PlayCard {
                card_index: 2,
                target: None,
            },
        ],
    );
    let service = NeutralEngineQueryService::default();
    let results =
        service.branch_effect_evidence(&context, &[ActionId(0), ActionId(1), ActionId(2)]);
    assert_eq!(results.len(), 3);

    let attack = &results[0].branch_effect.signature;
    let block = &results[2].branch_effect.signature;
    assert_ne!(attack, block);
    assert!(attack.enemy_damage_bucket > block.enemy_damage_bucket);

    let groups = service.compress_branch_effects(&results);
    assert_eq!(groups.len(), 2);
    let attack_group = groups
        .iter()
        .find(|group| group.signature.enemy_damage_bucket == attack.enemy_damage_bucket)
        .unwrap();
    assert_eq!(attack_group.count, 2);
}
