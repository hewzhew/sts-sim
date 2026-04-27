use sts_simulator::content::cards::CardId;
use sts_simulator::content::monsters::EnemyId;
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
