use sts_simulator::content::cards::CardId;
use sts_simulator::content::monsters::city::gremlin_leader::GremlinLeader;
use sts_simulator::content::monsters::{EnemyId, MonsterBehavior};
use sts_simulator::content::potions::{Potion, PotionId};
use sts_simulator::content::powers::PowerId;
use sts_simulator::engine::action_handlers::execute_action;
use sts_simulator::engine::core::tick_until_stable_turn;
use sts_simulator::runtime::action::Action;
use sts_simulator::runtime::combat::{CombatCard, Power};
use sts_simulator::state::core::{ClientInput, EngineState};
use sts_simulator::test_support::combat_with_monsters;
use sts_simulator::test_support::{planned_monster, test_monster};

fn card(id: CardId, uuid: u32, upgraded: bool) -> CombatCard {
    let mut card = CombatCard::new(id, uuid);
    card.upgrades = u8::from(upgraded);
    card
}

#[test]
fn heal_ignores_dying_monster_targets() {
    let mut monster = test_monster(EnemyId::Cultist);
    monster.id = 7;
    monster.current_hp = 5;
    monster.max_hp = 20;
    monster.is_dying = true;
    let mut state = combat_with_monsters(vec![monster]);

    execute_action(
        Action::Heal {
            target: 7,
            amount: 10,
        },
        &mut state,
    );

    assert_eq!(state.entities.monsters[0].current_hp, 5);
}

#[test]
fn apply_power_ignores_escaped_monster_targets() {
    let mut monster = test_monster(EnemyId::Cultist);
    monster.id = 9;
    monster.is_escaped = true;
    let mut state = combat_with_monsters(vec![monster]);

    execute_action(
        Action::ApplyPower {
            source: 9,
            target: 9,
            power_id: PowerId::Strength,
            amount: 2,
        },
        &mut state,
    );

    let applied = state.entities.power_db.get(&9).and_then(|powers| {
        powers
            .iter()
            .find(|power| power.power_type == PowerId::Strength)
    });
    assert!(applied.is_none());
}

#[test]
fn mayhem_queues_play_top_card_after_turn_draw() {
    let monster = planned_monster(EnemyId::Cultist, 3);
    let mut state = combat_with_monsters(vec![monster]);
    state.entities.power_db.insert(
        0,
        vec![Power {
            power_type: PowerId::MayhemPower,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            just_applied: false,
        }],
    );
    state.zones.draw_pile = vec![
        card(CardId::ThunderClap, 1, true),
        card(CardId::ThunderClap, 2, true),
        card(CardId::Strike, 3, false),
        card(CardId::Strike, 4, false),
        card(CardId::Strike, 5, false),
        card(CardId::Defend, 6, false),
    ];

    let mut engine_state = EngineState::CombatPlayerTurn;
    let alive = tick_until_stable_turn(&mut engine_state, &mut state, ClientInput::EndTurn);

    assert!(alive);
    assert_eq!(engine_state, EngineState::CombatPlayerTurn);
    assert_eq!(state.entities.player.block, 5);
    assert_eq!(state.zones.hand.len(), 5);
    assert!(state.zones.draw_pile.is_empty());
    assert_eq!(
        state
            .entities
            .power_db
            .get(&state.entities.monsters[0].id)
            .and_then(|powers| powers.iter().find(|p| p.power_type == PowerId::Vulnerable)),
        None
    );
}

#[test]
fn distilled_chaos_preserves_java_card_queue_order_with_rage() {
    let mut first = test_monster(EnemyId::Cultist);
    first.id = 1;
    first.current_hp = 20;
    first.max_hp = 20;
    let mut second = test_monster(EnemyId::Cultist);
    second.id = 2;
    second.current_hp = 20;
    second.max_hp = 20;
    second.slot = 1;
    second.logical_position = 1;

    let mut state = combat_with_monsters(vec![first, second]);
    state.entities.player.block = 5;
    state.turn.energy = 1;
    state.entities.potions = vec![
        Some(Potion::new(PotionId::DistilledChaosPotion, 1)),
        None,
        None,
    ];
    state.zones.draw_pile = vec![
        card(CardId::ThunderClap, 10, false),
        card(CardId::Defend, 11, false),
        card(CardId::Rage, 12, false),
    ];

    let mut engine_state = EngineState::CombatPlayerTurn;
    let alive = tick_until_stable_turn(
        &mut engine_state,
        &mut state,
        ClientInput::UsePotion {
            potion_index: 0,
            target: None,
        },
    );

    assert!(alive);
    assert_eq!(engine_state, EngineState::CombatPlayerTurn);
    assert_eq!(state.entities.player.block, 10);
    assert_eq!(
        state
            .entities
            .power_db
            .get(&0)
            .and_then(|powers| powers.iter().find(|p| p.power_type == PowerId::Rage))
            .map(|p| p.amount),
        Some(3)
    );
    assert!(state.zones.draw_pile.is_empty());
    assert_eq!(state.zones.discard_pile.len(), 3);
}

#[test]
fn gremlin_leader_summons_reuse_live_snapshot_slot_draw_xs() {
    let mut first = test_monster(EnemyId::GremlinFat);
    first.id = 1;
    first.current_hp = 0;
    first.is_dying = true;
    first.logical_position = 716;
    let mut second = test_monster(EnemyId::GremlinFat);
    second.id = 2;
    second.current_hp = 0;
    second.is_dying = true;
    second.logical_position = 847;
    second.slot = 1;
    let mut leader = test_monster(EnemyId::GremlinLeader);
    leader.id = 3;
    leader.current_hp = 43;
    leader.slot = 2;
    leader.logical_position = 983;
    leader.set_planned_move_id(2);

    let mut state = combat_with_monsters(vec![first, second, leader]);
    state.monster_protocol_identity_mut(1).draw_x = Some(716);
    state.monster_protocol_identity_mut(2).draw_x = Some(847);
    state.monster_protocol_identity_mut(3).draw_x = Some(983);

    let leader = state.entities.monsters[2].clone();
    let plan = GremlinLeader::turn_plan(&state, &leader);
    let actions = GremlinLeader::take_turn_plan(&mut state, &leader, &plan);
    let spawns = actions
        .iter()
        .filter_map(|action| match action {
            Action::SpawnMonsterSmart {
                monster_id,
                logical_position,
                protocol_draw_x,
                ..
            } => Some((*monster_id, *logical_position, *protocol_draw_x)),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(spawns.len(), 2);
    for (idx, (monster_id, logical_position, protocol_draw_x)) in spawns.iter().enumerate() {
        let slot_draw_x = [716, 847][idx];
        let expected = if *monster_id == EnemyId::GremlinWizard {
            slot_draw_x - 35
        } else {
            slot_draw_x
        };
        assert_eq!(*logical_position, expected);
        assert_eq!(*protocol_draw_x, Some(expected));
    }
}
