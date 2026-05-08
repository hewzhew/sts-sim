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

fn cultist_combat_with_zones(
    hand: &[CardId],
    draw: &[CardId],
    discard: &[CardId],
) -> sts_simulator::runtime::combat::CombatState {
    let mut combat = cultist_combat_with_hand(hand);
    let mut uuid = 100;
    for id in draw {
        combat.zones.draw_pile.push(card(*id, uuid));
        uuid += 1;
    }
    for id in discard {
        combat.zones.discard_pile.push(card(*id, uuid));
        uuid += 1;
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

#[test]
fn draw_top_card_branches_compress_without_enumerating_full_future_tree() {
    let combat = cultist_combat_with_zones(
        &[CardId::PommelStrike],
        &[CardId::Strike, CardId::Defend, CardId::Bash],
        &[],
    );
    let context = SearchExecutionContext::new(
        decision_id(),
        EngineState::CombatPlayerTurn,
        combat,
        vec![ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        }],
    );
    let service = NeutralEngineQueryService::default();
    let branches = service.draw_top_card_branch_effects(&context, ActionId(0), 8);
    assert_eq!(branches.len(), 3);
    assert!(branches
        .iter()
        .all(|branch| branch.scenario_debug.is_some()));

    let groups = service.compress_branch_effects(&branches);
    assert!(
        groups.len() < branches.len(),
        "draw samples should be compressed by observed effect signature"
    );
}

#[test]
fn pending_choice_is_intermediate_decision_state_not_flat_root_option() {
    let combat =
        cultist_combat_with_zones(&[CardId::Headbutt], &[], &[CardId::Strike, CardId::Defend]);
    let context = SearchExecutionContext::new(
        decision_id(),
        EngineState::CombatPlayerTurn,
        combat,
        vec![ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        }],
    );
    let service = NeutralEngineQueryService::default();
    let result = service.force_to_stable(&context, ActionId(0)).unwrap();
    assert!(
        result.after.pending_choice,
        "Headbutt should expose a pending choice state instead of being flattened at root"
    );
    assert!(result.branch_effect.pending_choice_created);
}

#[test]
fn commutation_probe_detects_order_equivalence_when_both_orders_are_legal() {
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
    let probe = service
        .commutation_probe(&context, ActionId(0), ActionId(1))
        .expect("probe");

    assert!(probe.left_then_right_legal);
    assert!(probe.right_then_left_legal);
    assert!(probe.both_orders_reached_boundary);
    assert!(probe.summary_equal);
    assert!(probe.order_only_equivalent);
    assert_eq!(probe.enemy_removed_diff, 0);
    assert_eq!(probe.hp_loss_diff, 0);
}

#[test]
fn commutation_probe_marks_mutual_exclusion_when_second_action_becomes_illegal() {
    let mut combat = cultist_combat_with_hand(&[CardId::Strike, CardId::Defend]);
    combat.turn.energy = 1;
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
    let probe = service
        .commutation_probe(&context, ActionId(0), ActionId(1))
        .expect("probe");

    assert!(!probe.left_then_right_legal);
    assert!(!probe.right_then_left_legal);
    assert!(!probe.order_only_equivalent);
}

#[test]
fn enemy_response_public_probe_redacts_future_card_zones() {
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
    let probe = service
        .enemy_response_public_probe(&context, ActionId(0), ActionId(1))
        .expect("enemy response public probe");
    assert!(probe.public_safe);
    let serialized = serde_json::to_string(&probe).expect("serialize probe");
    assert!(serialized.contains("redacted_fields"));
    assert!(!serialized.contains("hand_len"));
    assert!(!serialized.contains("draw_len"));
    assert!(!serialized.contains("discard_len"));
    assert!(!serialized.contains("exhaust_len"));
}
