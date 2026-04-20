use sts_simulator::content::monsters::EnemyId;
use sts_simulator::content::powers::PowerId;
use sts_simulator::engine::action_handlers::execute_action;
use sts_simulator::runtime::action::Action;
use sts_simulator::test_support::combat_with_monsters;
use sts_simulator::test_support::test_monster;

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
