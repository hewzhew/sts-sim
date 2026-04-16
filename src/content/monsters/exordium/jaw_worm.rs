use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity, PowerId};

pub struct JawWorm;

impl MonsterBehavior for JawWorm {
    fn roll_move(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let is_first_move = entity.move_history.is_empty();

        let chomp_dmg = if ascension_level >= 2 { 12 } else { 11 };
        let thrash_dmg = 7;

        if is_first_move {
            return (
                1,
                Intent::Attack {
                    damage: chomp_dmg,
                    hits: 1,
                },
            );
        }
        let last_move = entity.move_history.back().copied();
        let last_move_before = if entity.move_history.len() >= 2 {
            entity
                .move_history
                .get(entity.move_history.len() - 2)
                .copied()
        } else {
            None
        };
        let last_two_moves = last_move.is_some() && last_move == last_move_before;

        if num < 25 {
            if last_move == Some(1) {
                if rng.random_boolean_chance(0.5625) {
                    (2, Intent::DefendBuff)
                } else {
                    (
                        3,
                        Intent::AttackDefend {
                            damage: thrash_dmg,
                            hits: 1,
                        },
                    )
                }
            } else {
                (
                    1,
                    Intent::Attack {
                        damage: chomp_dmg,
                        hits: 1,
                    },
                )
            }
        } else if num < 55 {
            if last_two_moves && last_move == Some(3) {
                if rng.random_boolean_chance(0.357) {
                    (
                        1,
                        Intent::Attack {
                            damage: chomp_dmg,
                            hits: 1,
                        },
                    )
                } else {
                    (2, Intent::DefendBuff)
                }
            } else {
                (
                    3,
                    Intent::AttackDefend {
                        damage: thrash_dmg,
                        hits: 1,
                    },
                )
            }
        } else if last_move == Some(2) {
            if rng.random_boolean_chance(0.416) {
                (
                    1,
                    Intent::Attack {
                        damage: chomp_dmg,
                        hits: 1,
                    },
                )
            } else {
                (
                    3,
                    Intent::AttackDefend {
                        damage: thrash_dmg,
                        hits: 1,
                    },
                )
            }
        } else {
            (2, Intent::DefendBuff)
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let asc = state.meta.ascension_level;
        let (chomp_dmg, _thrash_dmg, thrash_block, bellow_block, bellow_str) = if asc >= 17 {
            (12, 7, 5, 9, 5)
        } else if asc >= 2 {
            (12, 7, 5, 6, 4)
        } else {
            (11, 7, 5, 6, 3)
        };
        // thrash_dmg is utilized actively in the Engine's damage resolution step
        let thrash_dmg = 7; // it stays 7 across all ascensions

        let mut actions = Vec::new();

        // Enqueue actions based on the current locked-in byte from last turn
        match entity.next_move_byte {
            1 => {
                // CHOMP
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0, // Player is always 0
                    base: chomp_dmg,
                    output: chomp_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            2 => {
                // BELLOW
                actions.push(Action::ApplyPower {
                    target: entity.id,
                    source: entity.id,
                    power_id: PowerId::Strength,
                    amount: bellow_str,
                });
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: bellow_block,
                });
            }
            3 => {
                // THRASH
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0, // Player
                    base: thrash_dmg,
                    output: thrash_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: thrash_block,
                });
            }
            _ => {
                // Unknown/Fallback state, just defend
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: 5,
                });
            }
        }

        // Always end a turn by rolling the NEXT move to update intent graphic!
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });

        actions
    }
}
