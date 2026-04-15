use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent};
use crate::content::monsters::EnemyId;
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Reptomancer;

impl Reptomancer {
    pub const DAGGER_DRAW_X: [i32; 4] = [210, -220, 180, -250];

    fn occupied_dagger_slots(state: &CombatState, reptomancer_id: usize) -> [bool; 4] {
        let mut occupied = [false; 4];
        for monster in &state.entities.monsters {
            if monster.id == reptomancer_id
                || monster.is_dying
                || EnemyId::from_id(monster.monster_type) != Some(EnemyId::SnakeDagger)
            {
                continue;
            }
            let key = monster
                .protocol_identity
                .draw_x
                .unwrap_or(monster.logical_position);
            for (slot, draw_x) in Self::DAGGER_DRAW_X.iter().enumerate() {
                if key == *draw_x {
                    occupied[slot] = true;
                }
            }
        }
        occupied
    }
}

impl MonsterBehavior for Reptomancer {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &crate::runtime::combat::MonsterEntity,
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
        _entity: &crate::runtime::combat::MonsterEntity,
        _hp_rng: &mut crate::runtime::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        // Technically starts with daggers based on encountering spawn mechanics, commonly handled at Encounter level.
        Vec::new()
    }

    fn take_turn(
        state: &mut CombatState,
        entity: &crate::runtime::combat::MonsterEntity,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.meta.ascension_level;

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
                let other_alive_count = state
                    .entities
                    .monsters
                    .iter()
                    .filter(|m| m.id != entity.id && !m.is_dying)
                    .count();
                if other_alive_count <= 3 {
                    let occupied_slots = Self::occupied_dagger_slots(state, entity.id);
                    let mut daggers_spawned = 0;
                    for (slot, draw_x) in Self::DAGGER_DRAW_X.iter().enumerate() {
                        if daggers_spawned >= daggers_per_spawn || occupied_slots[slot] {
                            continue;
                        }
                        daggers_spawned += 1;
                        actions.push(Action::SpawnMonsterSmart {
                            monster_id: EnemyId::SnakeDagger,
                            current_hp: 0,
                            max_hp: 0,
                            logical_position: *draw_x,
                            protocol_draw_x: Some(*draw_x),
                            is_minion: true,
                        });
                    }
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
