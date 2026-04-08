use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent};
use crate::content::monsters::EnemyId;
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Reptomancer;

impl MonsterBehavior for Reptomancer {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &crate::combat::MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let scythe_dmg = if ascension_level >= 3 { 16 } else { 13 };
        let big_bite_dmg = if ascension_level >= 3 { 34 } else { 30 };

        if entity.move_history.is_empty() {
            return (2, Intent::Unknown); // SPAWN_DAGGER
        }

        // Mocking `canSpawn` locally. We need to check alive count. Usually handled dynamically during take_turn cleanly,
        // since roll_move needs an answer, we assume intent logic uses state approximation.
        let last_move = entity.move_history.back().copied().unwrap_or(0);

        let last_two_moves = |byte| {
            entity.move_history.len() >= 2
                && entity.move_history[entity.move_history.len() - 1] == byte
                && entity.move_history[entity.move_history.len() - 2] == byte
        };

        if num < 33 {
            if last_move != 1 {
                return (
                    1,
                    Intent::AttackDebuff {
                        damage: scythe_dmg,
                        hits: 2,
                    },
                );
            } else {
                return (
                    3,
                    Intent::Attack {
                        damage: big_bite_dmg,
                        hits: 1,
                    },
                ); // fallback
            }
        } else if num < 66 {
            if !last_two_moves(2) {
                return (2, Intent::Unknown); // SPAWN_DAGGER
            } else {
                return (
                    1,
                    Intent::AttackDebuff {
                        damage: scythe_dmg,
                        hits: 2,
                    },
                );
            }
        } else if last_move != 3 {
            return (
                3,
                Intent::Attack {
                    damage: big_bite_dmg,
                    hits: 1,
                },
            );
        } else {
            return (
                1,
                Intent::AttackDebuff {
                    damage: scythe_dmg,
                    hits: 2,
                },
            ); // fallback
        }
    }

    fn use_pre_battle_action(
        _entity: &crate::combat::MonsterEntity,
        _hp_rng: &mut crate::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        // Technically starts with daggers based on encountering spawn mechanics, commonly handled at Encounter level.
        Vec::new()
    }

    fn take_turn(state: &mut CombatState, entity: &crate::combat::MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.ascension_level;

        let scythe_dmg = if asc >= 3 { 16 } else { 13 };
        let big_bite_dmg = if asc >= 3 { 34 } else { 30 };

        match entity.next_move_byte {
            1 => {
                // SNAKE_STRIKE
                for _ in 0..2 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: scythe_dmg,
                        output: scythe_dmg,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Weak,
                    amount: 1,
                });
            }
            2 => {
                // SPAWN_DAGGER
                let daggers_per_spawn = if asc >= 18 { 2 } else { 1 };
                let alive_count = state.monsters.iter().filter(|m| m.current_hp > 0).count();
                let summon_count = if alive_count <= 3 {
                    std::cmp::min(daggers_per_spawn, 4 - alive_count)
                } else {
                    0
                };

                for _ in 0..summon_count {
                    actions.push(Action::SpawnMonsterSmart {
                        monster_id: EnemyId::SnakeDagger,
                        current_hp: 25,
                        max_hp: 25,
                        logical_position: 0,
                    });
                }
            }
            3 => {
                // BIG_BITE
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: big_bite_dmg,
                    output: big_bite_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            _ => {}
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
