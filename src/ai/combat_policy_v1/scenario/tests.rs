use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::relics::{RelicId, RelicState};
use crate::runtime::combat::CombatCard;
use crate::sim::combat::CombatPosition;
use crate::state::core::{ClientInput, EngineState};

use super::*;

#[test]
fn hidden_draw_orders_share_one_information_set_without_frozen_eye() {
    let first = position_with_draw_order([CardId::Bash, CardId::Defend]);
    let mut second = first.clone();
    second.combat.zones.draw_pile.swap(0, 1);

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("hidden draw-order variants should group");

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].view().scenario_count, 2);
}

#[test]
fn frozen_eye_draw_orders_form_distinct_information_sets() {
    let mut first = position_with_draw_order([CardId::Bash, CardId::Defend]);
    first
        .combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::FrozenEye));
    let mut second = first.clone();
    second.combat.zones.draw_pile.swap(0, 1);

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("Frozen Eye variants should remain valid");

    assert_eq!(groups.len(), 2);
}

#[test]
fn public_history_separates_otherwise_identical_observations() {
    let position = position_with_draw_order([CardId::Bash, CardId::Defend]);

    let groups = group_combat_scenarios_v1(vec![
        CombatScenarioParticleV1::from_public_history("first", "history-a", position.clone()),
        CombatScenarioParticleV1::from_public_history("second", "history-b", position),
    ])
    .expect("public history should be part of the information set");

    assert_eq!(groups.len(), 2);
}

#[test]
fn one_public_target_action_binds_to_each_worlds_exact_entity_id() {
    let first = position_with_monster_id(700_001);
    let second = position_with_monster_id(990_001);
    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("entity identity must stay behind the public action boundary");

    assert_eq!(groups.len(), 1);
    let action = groups[0]
        .view()
        .candidates
        .iter()
        .find(|action| {
            matches!(
                action,
                CombatPublicActionV1::PlayCard {
                    target: Some(CombatPublicTargetV1 {
                        monster_slot: 0,
                        ..
                    }),
                    ..
                }
            )
        })
        .expect("targeted Strike action")
        .clone();
    let binding = groups[0]
        .bind_action(&action)
        .expect("public action binding");

    assert_eq!(binding.scenario_count(), 2);
    assert_eq!(
        binding
            .exact_inputs()
            .iter()
            .map(|(_, input)| match input {
                ClientInput::PlayCard {
                    target: Some(target),
                    ..
                } => *target,
                other => panic!("unexpected exact input: {other:?}"),
            })
            .collect::<Vec<_>>(),
        vec![700_001, 990_001]
    );

    let public_json =
        serde_json::to_string(groups[0].view()).expect("public group view should serialize");
    assert!(!public_json.contains("first"));
    assert!(!public_json.contains("second"));
    assert!(!public_json.contains("700001"));
    assert!(!public_json.contains("990001"));
}

fn particle(scenario_id: &str, position: CombatPosition) -> CombatScenarioParticleV1 {
    CombatScenarioParticleV1::root(scenario_id, position)
}

fn position_with_draw_order(cards: [CardId; 2]) -> CombatPosition {
    let mut position = position_with_monster_id(7);
    position.combat.zones.draw_pile =
        vec![CombatCard::new(cards[0], 20), CombatCard::new(cards[1], 21)];
    position
}

fn position_with_monster_id(monster_id: usize) -> CombatPosition {
    let mut combat = crate::test_support::blank_test_combat();
    combat.turn.energy = 3;
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
    let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
    monster.id = monster_id;
    monster.slot = 0;
    combat.entities.monsters = vec![monster];
    CombatPosition::new(EngineState::CombatPlayerTurn, combat)
}
