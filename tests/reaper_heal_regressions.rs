use smallvec::smallvec;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::engine::action_handlers::execute_action;
use sts_simulator::runtime::action::{Action, DamageType};
use sts_simulator::test_support::test_monster;

fn simple_monster(
    enemy_id: EnemyId,
    id: usize,
    hp: i32,
) -> sts_simulator::runtime::combat::MonsterEntity {
    let mut monster = test_monster(enemy_id);
    monster.id = id;
    monster.current_hp = hp;
    monster.max_hp = hp;
    monster
}

#[test]
fn reaper_heals_from_actual_hp_lost_not_overkill_damage() {
    let mut combat = sts_simulator::test_support::blank_test_combat();
    combat.entities.player.current_hp = 62;
    combat.entities.monsters = vec![
        simple_monster(EnemyId::Cultist, 1, 4),
        simple_monster(EnemyId::Cultist, 2, 6),
        simple_monster(EnemyId::Cultist, 3, 2),
    ];

    execute_action(
        Action::VampireDamageAllEnemies {
            source: 0,
            damages: smallvec![4, 4, 4],
            damage_type: DamageType::Normal,
        },
        &mut combat,
    );

    assert_eq!(
        combat.entities.player.current_hp, 72,
        "Reaper should heal from actual HP lost (4 + 4 + 2), not overkill damage (4 + 4 + 4)"
    );
    assert_eq!(combat.entities.monsters[0].current_hp, 0);
    assert_eq!(combat.entities.monsters[1].current_hp, 2);
    assert_eq!(combat.entities.monsters[2].current_hp, 0);
}
