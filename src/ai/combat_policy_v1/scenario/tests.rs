use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::content::relics::{RelicId, RelicState};
use crate::runtime::combat::{CombatCard, Power, PowerPayload};
use crate::sim::combat::{CombatPosition, CombatStepLimits};
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

#[test]
fn public_discard_contents_form_distinct_information_sets() {
    let mut first = position_with_monster_id(7);
    first.combat.zones.discard_pile = vec![CombatCard::new(CardId::Bash, 31)];
    let mut second = first.clone();
    second.combat.zones.discard_pile = vec![CombatCard::new(CardId::Defend, 32)];

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("public discard contents should remain valid");

    assert_eq!(groups.len(), 2);
}

#[test]
fn public_relic_counters_form_distinct_information_sets() {
    let mut first = position_with_monster_id(7);
    let mut pen_nib = RelicState::new(RelicId::PenNib);
    pen_nib.counter = 8;
    first.combat.entities.player.add_relic(pen_nib);
    let mut second = first.clone();
    second.combat.entities.player.relics[0].counter = 9;

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("public relic counters should remain valid");

    assert_eq!(groups.len(), 2);
}

#[test]
fn public_power_amounts_form_distinct_information_sets() {
    let mut first = position_with_monster_id(7);
    first.combat.entities.power_db.insert(
        first.combat.entities.player.id,
        vec![power(PowerId::Strength, 2)],
    );
    let mut second = first.clone();
    second
        .combat
        .entities
        .power_db
        .get_mut(&second.combat.entities.player.id)
        .expect("player power")[0]
        .amount = 3;

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("public power amounts should remain valid");

    assert_eq!(groups.len(), 2);
}

#[test]
fn monster_power_state_uses_public_slot_not_exact_entity_id() {
    let mut first = position_with_monster_id(700_001);
    first
        .combat
        .entities
        .power_db
        .insert(700_001, vec![power(PowerId::Weak, 2)]);
    let mut second = position_with_monster_id(990_001);
    second
        .combat
        .entities
        .power_db
        .insert(990_001, vec![power(PowerId::Weak, 2)]);

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("exact monster identity must stay private");

    assert_eq!(groups.len(), 1);
    let json = serde_json::to_string(groups[0].view()).expect("public group view serialization");
    assert!(!json.contains("700001"));
    assert!(!json.contains("990001"));
}

#[test]
fn hidden_rng_state_does_not_split_an_information_set() {
    let first = position_with_monster_id(7);
    let mut second = first.clone();
    second.combat.rng.pool.shuffle_rng.counter += 1;
    second.combat.rng.pool.shuffle_rng.seed0 ^= 0x55aa;

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("hidden RNG variants should group");

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].view().scenario_count, 2);
}

#[test]
fn exact_card_uuid_does_not_split_an_information_set() {
    let first = position_with_monster_id(7);
    let mut second = first.clone();
    second.combat.zones.hand[0].uuid = 999_999;

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("exact card identity should stay private");

    assert_eq!(groups.len(), 1);
}

#[test]
fn non_quiescent_player_turn_is_rejected() {
    let mut position = position_with_monster_id(7);
    position.combat.zones.limbo = vec![CombatCard::new(CardId::Bash, 40)];

    let error = match group_combat_scenarios_v1(vec![particle("pending", position)]) {
        Ok(_) => panic!("half-resolved player turn must fail closed"),
        Err(error) => error,
    };

    assert_eq!(
        error,
        CombatScenarioPolicyErrorV1::NonQuiescentBoundary {
            scenario_id: "pending".to_string(),
            pending_work: vec!["limbo".to_string()],
        }
    );
}

#[test]
fn one_public_action_steps_all_worlds_and_regroups_hidden_rng_variants() {
    let first = position_with_monster_id(7);
    let mut second = first.clone();
    second.combat.rng.pool.shuffle_rng.counter += 1;
    second.combat.rng.pool.shuffle_rng.seed0 ^= 0x55aa;
    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("hidden RNG variants should group");
    let strike = groups[0]
        .view()
        .candidates
        .iter()
        .find(|action| matches!(action, CombatPublicActionV1::PlayCard { .. }))
        .expect("Strike action")
        .clone();

    let stepped = step_combat_scenario_group_v1(
        &groups[0],
        &strike,
        CombatStepLimits {
            max_engine_steps: 50,
            deadline: None,
        },
    )
    .expect("one public action should step both exact worlds");

    assert_eq!(stepped.view.scenario_count, 2);
    assert_eq!(stepped.view.continuing_scenario_count, 2);
    assert_eq!(stepped.view.next_information_set_count, 1);
    assert_eq!(stepped.next_groups[0].view().scenario_count, 2);
    assert_ne!(
        stepped.next_groups[0].view().key.public_history_id,
        COMBAT_POLICY_ROOT_HISTORY_ID
    );
}

#[test]
fn newly_observed_draws_split_successor_information_sets() {
    let first = position_with_battle_trance_draw_order([
        CardId::Bash,
        CardId::Defend,
        CardId::Strike,
        CardId::Anger,
    ]);
    let mut second = first.clone();
    second.combat.zones.draw_pile.swap(0, 1);
    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("hidden draw-order variants should group before drawing");
    assert_eq!(groups.len(), 1);
    let battle_trance = groups[0]
        .view()
        .candidates
        .iter()
        .find(|action| {
            matches!(
                action,
                CombatPublicActionV1::PlayCard {
                    card_id,
                    target: None,
                    ..
                } if card_id == "Battle Trance"
            )
        })
        .expect("Battle Trance action")
        .clone();

    let stepped = step_combat_scenario_group_v1(
        &groups[0],
        &battle_trance,
        CombatStepLimits {
            max_engine_steps: 50,
            deadline: None,
        },
    )
    .expect("draw action should reach a new public boundary");

    assert_eq!(stepped.view.scenario_count, 2);
    assert_eq!(stepped.view.next_information_set_count, 2);
    assert_eq!(
        stepped
            .next_groups
            .iter()
            .map(|group| group.view().scenario_count)
            .sum::<usize>(),
        2
    );
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

fn position_with_battle_trance_draw_order(cards: [CardId; 4]) -> CombatPosition {
    let mut position = position_with_monster_id(7);
    position.combat.zones.hand = vec![CombatCard::new(CardId::BattleTrance, 10)];
    position.combat.zones.draw_pile = cards
        .into_iter()
        .enumerate()
        .map(|(index, card_id)| CombatCard::new(card_id, 20 + index as u32))
        .collect();
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

fn power(power_id: PowerId, amount: i32) -> Power {
    Power {
        power_type: power_id,
        instance_id: None,
        amount,
        extra_data: 0,
        payload: PowerPayload::None,
        just_applied: false,
    }
}
