use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::content::relics::{RelicId, RelicState};
use crate::runtime::action::CardDestination;
use crate::runtime::combat::{CombatCard, Power, PowerPayload};
use crate::sim::combat::{CombatPosition, CombatStepLimits};
use crate::state::core::{
    ChooseOneCardChoice, ClientInput, DiscoveryChoiceState, EngineState, GridSelectReason,
    HandSelectReason, PendingChoice, PileType,
};

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

#[test]
fn pending_hand_choice_groups_different_uuids_and_collapses_identical_cards() {
    let first = pending_hand_select_position(
        vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Strike, 20),
        ],
        1,
        1,
    );
    let second = pending_hand_select_position(
        vec![
            CombatCard::new(CardId::Strike, 70_001),
            CombatCard::new(CardId::Strike, 99_001),
        ],
        1,
        1,
    );

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("publicly identical hand choices should group");

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].view().scenario_count, 2);
    assert_eq!(groups[0].view().candidates.len(), 1);
    let pending = groups[0]
        .view()
        .observation
        .pending_choice
        .as_ref()
        .expect("pending choice observation");
    let CombatPublicPendingChoiceV1::HandSelect { candidates, .. } = pending else {
        panic!("expected hand selection, got {pending:?}");
    };
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].count, 2);

    let binding = groups[0]
        .bind_action(&groups[0].view().candidates[0])
        .expect("public multiset selection should bind in both worlds");
    assert_eq!(binding.scenario_count(), 2);
    let public_json =
        serde_json::to_string(groups[0].view()).expect("public pending choice serialization");
    assert!(!public_json.contains("70001"));
    assert!(!public_json.contains("99001"));
    assert!(!public_json.contains("uuid"));
}

#[test]
fn pending_selection_enumerates_all_combinations_beyond_legacy_sixteen_cap() {
    let cards = [
        CardId::Bash,
        CardId::Defend,
        CardId::Strike,
        CardId::Anger,
        CardId::Cleave,
        CardId::ShrugItOff,
        CardId::PommelStrike,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, card_id)| CombatCard::new(card_id, 100 + index as u32))
    .collect();
    let position = pending_hand_select_position(cards, 2, 2);

    let groups = group_combat_scenarios_v1(vec![particle("all-pairs", position)])
        .expect("seven choose two should be fully enumerated");

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].view().candidates.len(), 21);
}

#[test]
fn hidden_draw_order_does_not_leak_through_grid_selection_candidates() {
    let mut first = position_with_monster_id(7);
    first.combat.zones.draw_pile = vec![
        CombatCard::new(CardId::Bash, 10),
        CombatCard::new(CardId::Defend, 20),
    ];
    first.engine = EngineState::PendingChoice(PendingChoice::GridSelect {
        source_pile: PileType::Draw,
        candidate_uuids: vec![10, 20],
        min_cards: 1,
        max_cards: 1,
        can_cancel: false,
        reason: GridSelectReason::DrawPileToHand,
    });
    let mut second = first.clone();
    second.combat.zones.draw_pile.swap(0, 1);
    if let EngineState::PendingChoice(PendingChoice::GridSelect {
        candidate_uuids, ..
    }) = &mut second.engine
    {
        candidate_uuids.swap(0, 1);
    }

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("grid presentation must not reveal hidden draw order");

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].view().scenario_count, 2);
}

#[test]
fn step_loop_crosses_hand_choice_without_exposing_uuid() {
    let mut position = position_with_monster_id(7);
    position.combat.zones.hand = vec![
        CombatCard::new(CardId::Armaments, 10),
        CombatCard::new(CardId::Strike, 20),
        CombatCard::new(CardId::Defend, 30),
    ];
    let groups = group_combat_scenarios_v1(vec![particle("armaments", position)])
        .expect("Armaments root information set");
    let armaments = groups[0]
        .view()
        .candidates
        .iter()
        .find(|action| {
            matches!(
                action,
                CombatPublicActionV1::PlayCard { card_id, .. } if card_id == "Armaments"
            )
        })
        .expect("Armaments play")
        .clone();

    let pending = step_combat_scenario_group_v1(
        &groups[0],
        &armaments,
        CombatStepLimits {
            max_engine_steps: 100,
            deadline: None,
        },
    )
    .expect("Armaments should step to a public hand choice");

    assert_eq!(pending.next_groups.len(), 1);
    assert_eq!(
        pending.next_groups[0].view().observation.engine_state,
        "combat_pending_choice"
    );
    let choose_strike = pending.next_groups[0]
        .view()
        .candidates
        .iter()
        .find(|action| {
            matches!(
                action,
                CombatPublicActionV1::SelectCards { selected, .. }
                    if selected.len() == 1 && selected[0].card.card_id == "Strike_R"
            )
        })
        .expect("public Strike selection")
        .clone();

    let resumed = step_combat_scenario_group_v1(
        &pending.next_groups[0],
        &choose_strike,
        CombatStepLimits {
            max_engine_steps: 100,
            deadline: None,
        },
    )
    .expect("public selection should resume card resolution");

    assert_eq!(resumed.next_groups.len(), 1);
    assert_eq!(
        resumed.next_groups[0].view().observation.engine_state,
        "combat_player_turn"
    );
    assert!(resumed.next_groups[0]
        .view()
        .observation
        .pending_choice
        .is_none());
}

#[test]
fn oversized_pending_choice_fails_with_typed_gap() {
    let mut position = position_with_monster_id(7);
    position.combat.zones.draw_pile = (0..13)
        .map(|index| CombatCard::new(CardId::Strike, 500 + index))
        .collect();
    position.engine = EngineState::PendingChoice(PendingChoice::ScrySelect {
        cards: vec![CardId::Strike; 13],
        card_uuids: (0..13).map(|index| 500 + index).collect(),
    });

    let error = match group_combat_scenarios_v1(vec![particle("wide-scry", position)]) {
        Ok(_) => panic!("wide Scry should not silently truncate its action set"),
        Err(error) => error,
    };

    assert_eq!(
        error,
        CombatScenarioPolicyErrorV1::CandidateSpaceTooLarge {
            scenario_id: "wide-scry".to_string(),
            choice_kind: CombatPublicPendingChoiceKindV1::ScrySelect,
            candidate_count: 13,
            action_count: 8_192,
            cap: 4_096,
        }
    );
}

#[test]
fn remaining_pending_choice_kinds_expose_typed_public_actions() {
    let cases = vec![
        (
            "discovery",
            PendingChoice::DiscoverySelect(DiscoveryChoiceState {
                cards: vec![CardId::Anger, CardId::Cleave],
                colorless: false,
                card_type: None,
                amount: 1,
                can_skip: true,
            }),
            3,
        ),
        (
            "card-reward",
            PendingChoice::CardRewardSelect {
                cards: vec![CardId::Bash],
                destination: CardDestination::Hand,
                can_skip: true,
            },
            2,
        ),
        (
            "foreign-influence",
            PendingChoice::ForeignInfluenceSelect {
                cards: vec![CardId::Strike],
                upgraded: true,
            },
            1,
        ),
        (
            "choose-one",
            PendingChoice::ChooseOneSelect {
                choices: vec![ChooseOneCardChoice {
                    card_id: CardId::Anger,
                    upgrades: 1,
                }],
            },
            1,
        ),
        ("stance", PendingChoice::StanceChoice, 2),
    ];

    for (scenario_id, choice, expected_actions) in cases {
        let mut position = position_with_monster_id(7);
        position.engine = EngineState::PendingChoice(choice);
        let groups = group_combat_scenarios_v1(vec![particle(scenario_id, position)])
            .expect("typed pending choice should project");
        assert_eq!(
            groups[0].view().candidates.len(),
            expected_actions,
            "{scenario_id}"
        );
        assert!(
            groups[0].view().observation.pending_choice.is_some(),
            "{scenario_id}"
        );
    }

    let mut scry = position_with_monster_id(7);
    scry.combat.zones.draw_pile = vec![
        CombatCard::new(CardId::Strike, 80_001),
        CombatCard::new(CardId::Defend, 80_002),
    ];
    scry.engine = EngineState::PendingChoice(PendingChoice::ScrySelect {
        cards: vec![CardId::Strike, CardId::Defend],
        card_uuids: vec![80_001, 80_002],
    });
    let groups = group_combat_scenarios_v1(vec![particle("scry", scry)])
        .expect("Scry should expose every discard subset");
    assert_eq!(groups[0].view().candidates.len(), 4);
    assert!(groups[0]
        .view()
        .candidates
        .iter()
        .all(|action| matches!(action, CombatPublicActionV1::ScryDiscard { .. })));
    let json = serde_json::to_string(groups[0].view()).expect("public Scry serialization");
    assert!(!json.contains("80001"));
    assert!(!json.contains("80002"));
    assert!(!json.contains("uuid"));
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

fn pending_hand_select_position(
    hand: Vec<CombatCard>,
    min_cards: u8,
    max_cards: u8,
) -> CombatPosition {
    let mut position = position_with_monster_id(7);
    let candidate_uuids = hand.iter().map(|card| card.uuid).collect();
    position.combat.zones.hand = hand;
    position.engine = EngineState::PendingChoice(PendingChoice::HandSelect {
        candidate_uuids,
        min_cards,
        max_cards,
        can_cancel: false,
        reason: HandSelectReason::Discard,
    });
    position
}
