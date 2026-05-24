//! Search-facing combat canaries.
//!
//! This is not a card/relic behavior dump. Keep cases here only when a broken
//! engine transition would make Combat Search consume invalid legal actions,
//! miss a stable boundary, or evaluate the wrong public combat state. Put
//! single-card and single-relic semantics in content/engine tests instead.

use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::potions::{Potion, PotionId};
use crate::content::powers::{store, PowerId};
use crate::runtime::combat::{CombatCard, CombatState, MonsterEntity, Power, PowerPayload};
use crate::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepResult, CombatStepper, CombatTerminal,
    EngineCombatStepper,
};
use crate::state::core::{
    ClientInput, EngineState, GridSelectReason, HandSelectReason, PendingChoice, PileType,
};
use crate::test_support::{blank_test_combat, test_monster};

fn step_limits() -> CombatStepLimits {
    CombatStepLimits {
        max_engine_steps: 128,
        deadline: None,
    }
}

fn player_turn_position(combat: CombatState) -> CombatPosition {
    CombatPosition::new(EngineState::CombatPlayerTurn, combat)
}

fn apply_from_player_turn(combat: CombatState, input: ClientInput) -> CombatStepResult {
    apply(&player_turn_position(combat), input)
}

fn apply(position: &CombatPosition, input: ClientInput) -> CombatStepResult {
    let stepper = EngineCombatStepper;
    stepper.apply_to_stable(position, input, step_limits())
}

fn legal_actions(position: &CombatPosition) -> Vec<ClientInput> {
    let stepper = EngineCombatStepper;
    stepper.legal_actions(position)
}

fn assert_stable_player_turn(step: &CombatStepResult) {
    assert!(!step.truncated);
    assert_eq!(step.position.engine, EngineState::CombatPlayerTurn);
}

fn power(power_type: PowerId, amount: i32) -> Power {
    Power {
        power_type,
        instance_id: None,
        amount,
        extra_data: 0,
        payload: PowerPayload::None,
        just_applied: false,
    }
}

fn monster(enemy_id: EnemyId, id: usize, slot: u8, hp: i32) -> MonsterEntity {
    let mut monster = test_monster(enemy_id);
    monster.id = id;
    monster.slot = slot;
    monster.current_hp = hp;
    monster.max_hp = hp.max(monster.max_hp);
    monster
}

fn card_snapshots(cards: &[CombatCard]) -> Vec<(CardId, u32)> {
    cards.iter().map(|card| (card.id, card.uuid)).collect()
}

#[test]
fn stepper_dropkick_against_vulnerable_draws_and_refunds_energy() {
    let mut combat = blank_test_combat();
    combat.turn.energy = 1;
    combat.entities.monsters = vec![monster(EnemyId::JawWorm, 10, 0, 30)];
    store::set_powers_for(&mut combat, 10, vec![power(PowerId::Vulnerable, 3)]);
    combat.zones.hand = vec![CombatCard::new(CardId::Dropkick, 100)];
    combat.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 101)];

    let step = apply_from_player_turn(
        combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(10),
        },
    );

    assert_stable_player_turn(&step);
    assert_eq!(step.terminal, CombatTerminal::Unresolved);
    assert_eq!(
        step.position.combat.turn.energy, 1,
        "Dropkick should spend 1 energy then refund 1 when the target is Vulnerable"
    );
    assert_eq!(
        step.position.combat.entities.monsters[0].current_hp, 23,
        "Dropkick damage should use the target's Vulnerable state at execution"
    );
    assert_eq!(
        card_snapshots(&step.position.combat.zones.hand),
        vec![(CardId::Strike, 101)],
        "Dropkick should draw one card after the damage/effect action resolves"
    );
}

#[test]
fn stepper_headbutt_grid_select_moves_selected_discard_card_to_draw_top() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![monster(EnemyId::JawWorm, 10, 0, 40)];
    combat.zones.hand = vec![CombatCard::new(CardId::Headbutt, 100)];
    combat.zones.discard_pile = vec![
        CombatCard::new(CardId::Strike, 201),
        CombatCard::new(CardId::Defend, 202),
    ];

    let after_headbutt = apply_from_player_turn(
        combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(10),
        },
    );

    assert!(!after_headbutt.truncated);
    match &after_headbutt.position.engine {
        EngineState::PendingChoice(PendingChoice::GridSelect {
            source_pile,
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        }) => {
            assert_eq!(*source_pile, PileType::Discard);
            assert_eq!(candidate_uuids, &vec![201, 202]);
            assert_eq!((*min_cards, *max_cards, *can_cancel), (1, 1, false));
            assert_eq!(*reason, GridSelectReason::MoveToDrawPile);
        }
        other => panic!("Headbutt should suspend on a discard grid select, got {other:?}"),
    }

    let legal = legal_actions(&after_headbutt.position);
    assert!(legal.contains(&ClientInput::SubmitGridSelect(vec![202])));
    assert!(
        !legal.contains(&ClientInput::Proceed),
        "pending grid select must not expose fake Proceed to search"
    );

    let after_select = apply(
        &after_headbutt.position,
        ClientInput::SubmitGridSelect(vec![202]),
    );

    assert_stable_player_turn(&after_select);
    assert_eq!(
        card_snapshots(&after_select.position.combat.zones.draw_pile),
        vec![(CardId::Defend, 202)]
    );
    let discard = card_snapshots(&after_select.position.combat.zones.discard_pile);
    assert!(discard.contains(&(CardId::Headbutt, 100)));
    assert!(discard.contains(&(CardId::Strike, 201)));
    assert!(
        !discard.contains(&(CardId::Defend, 202)),
        "the selected Headbutt card should no longer remain in discard"
    );
}

#[test]
fn stepper_armaments_base_resolves_upgrade_pending_choice() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![monster(EnemyId::JawWorm, 10, 0, 40)];
    combat.zones.hand = vec![
        CombatCard::new(CardId::Armaments, 100),
        CombatCard::new(CardId::Strike, 101),
        CombatCard::new(CardId::Bash, 102),
    ];

    let after_armaments = apply_from_player_turn(
        combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    );

    assert!(!after_armaments.truncated);
    assert_eq!(after_armaments.position.combat.entities.player.block, 5);
    match &after_armaments.position.engine {
        EngineState::PendingChoice(PendingChoice::HandSelect {
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        }) => {
            assert_eq!(candidate_uuids, &vec![101, 102]);
            assert_eq!((*min_cards, *max_cards, *can_cancel), (1, 1, false));
            assert_eq!(*reason, HandSelectReason::Upgrade);
        }
        other => panic!("base Armaments should suspend on an upgrade hand select, got {other:?}"),
    }

    let legal = legal_actions(&after_armaments.position);
    assert!(legal.contains(&ClientInput::SubmitHandSelect(vec![102])));
    assert!(
        !legal.contains(&ClientInput::Proceed),
        "pending hand select must not expose fake Proceed to search"
    );

    let after_select = apply(
        &after_armaments.position,
        ClientInput::SubmitHandSelect(vec![102]),
    );

    assert_stable_player_turn(&after_select);
    let bash = after_select
        .position
        .combat
        .zones
        .hand
        .iter()
        .find(|card| card.uuid == 102)
        .expect("selected Bash should remain in hand after Armaments resolves");
    assert_eq!(bash.upgrades, 1);
}

#[test]
fn stepper_armaments_plus_upgrades_all_eligible_hand_cards_without_pending_choice() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![monster(EnemyId::JawWorm, 10, 0, 40)];
    let mut armaments_plus = CombatCard::new(CardId::Armaments, 100);
    armaments_plus.upgrades = 1;
    let mut already_upgraded_defend = CombatCard::new(CardId::Defend, 102);
    already_upgraded_defend.upgrades = 1;
    combat.zones.hand = vec![
        armaments_plus,
        CombatCard::new(CardId::Strike, 101),
        already_upgraded_defend,
        CombatCard::new(CardId::Wound, 103),
    ];

    let step = apply_from_player_turn(
        combat,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    );

    assert_stable_player_turn(&step);
    assert_eq!(step.position.combat.entities.player.block, 5);
    let hand = &step.position.combat.zones.hand;
    assert_eq!(
        hand.iter()
            .find(|card| card.uuid == 101)
            .map(|card| card.upgrades),
        Some(1),
        "Armaments+ should upgrade eligible cards left in hand"
    );
    assert_eq!(
        hand.iter()
            .find(|card| card.uuid == 102)
            .map(|card| card.upgrades),
        Some(1),
        "already-upgraded non-Searing Blow cards should not gain an extra upgrade"
    );
    assert_eq!(
        hand.iter()
            .find(|card| card.uuid == 103)
            .map(|card| card.upgrades),
        Some(0),
        "statuses are not Armaments upgrade targets"
    );
}

#[test]
fn stepper_fruit_juice_consumes_slot_and_increases_hp_once() {
    let mut combat = blank_test_combat();
    combat.entities.player.current_hp = 13;
    combat.entities.player.max_hp = 87;
    combat.entities.monsters = vec![monster(EnemyId::JawWorm, 10, 0, 30)];
    combat.entities.potions = vec![
        Some(Potion::new(PotionId::FruitJuice, 300)),
        Some(Potion::new(PotionId::FirePotion, 301)),
        None,
    ];

    let step = apply_from_player_turn(
        combat,
        ClientInput::UsePotion {
            potion_index: 0,
            target: None,
        },
    );

    assert_stable_player_turn(&step);
    assert_eq!(step.position.combat.entities.player.current_hp, 18);
    assert_eq!(step.position.combat.entities.player.max_hp, 92);
    assert!(
        step.position.combat.entities.potions[0].is_none(),
        "the used Fruit Juice slot must be empty after the max-hp effect resolves"
    );
    assert_eq!(
        step.position.combat.entities.potions[1]
            .as_ref()
            .map(|potion| potion.id),
        Some(PotionId::FirePotion)
    );
}

#[test]
fn stepper_legal_card_targets_exclude_zero_hp_dying_and_half_dead_monsters() {
    let mut combat = blank_test_combat();
    let mut zero_hp_leftover = monster(EnemyId::AcidSlimeM, 11, 0, 0);
    zero_hp_leftover.is_dying = false;
    zero_hp_leftover.half_dead = false;
    let mut dying_leftover = monster(EnemyId::AcidSlimeL, 12, 1, 0);
    dying_leftover.is_dying = true;
    let mut half_dead_leftover = monster(EnemyId::Darkling, 13, 2, 1);
    half_dead_leftover.half_dead = true;
    let alive = monster(EnemyId::JawWorm, 14, 3, 25);
    combat.entities.monsters = vec![zero_hp_leftover, dying_leftover, half_dead_leftover, alive];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 100)];

    let target_actions = legal_actions(&player_turn_position(combat))
        .into_iter()
        .filter_map(|input| match input {
            ClientInput::PlayCard { card_index, target } => Some((card_index, target)),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        target_actions,
        vec![(0, Some(14))],
        "search-facing legal actions must not target dead/split-leftover monsters"
    );
}
