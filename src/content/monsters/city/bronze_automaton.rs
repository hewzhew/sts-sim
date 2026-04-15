use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent};
use crate::content::monsters::{EnemyId, MonsterBehavior};
use crate::content::powers::PowerId;

pub struct BronzeAutomaton;

impl MonsterBehavior for BronzeAutomaton {
    fn use_pre_battle_action(
        entity: &crate::combat::MonsterEntity,
        _hp_rng: &mut crate::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Artifact,
            amount: 3,
        }]
    }

    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &crate::combat::MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let flail_dmg = if ascension_level >= 4 { 8 } else { 7 };
        let beam_dmg = if ascension_level >= 4 { 50 } else { 45 };

        let turn = entity.move_history.len();
        if turn == 0 {
            return (4, Intent::Unknown);
        }

        match turn % 6 {
            1 | 3 => (
                1,
                Intent::Attack {
                    damage: flail_dmg,
                    hits: 2,
                },
            ), // Flail
            2 | 4 => (5, Intent::DefendBuff), // Boost
            5 => (
                2,
                Intent::Attack {
                    damage: beam_dmg,
                    hits: 1,
                },
            ), // Hyper Beam
            0 => {
                // After Beam
                if ascension_level >= 19 {
                    (5, Intent::DefendBuff) // A19 Boosts instead of Stunned
                } else {
                    (3, Intent::Stun)
                }
            }
            _ => (3, Intent::Stun), // Unreachable
        }
    }

    fn take_turn(state: &mut CombatState, entity: &crate::combat::MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();

        let block_amt = if state.meta.ascension_level >= 9 {
            12
        } else {
            9
        };
        let str_amt = if state.meta.ascension_level >= 4 {
            4
        } else {
            3
        };

        match entity.next_move_byte {
            4 => {
                let left_hp =
                    bronze_orb_spawn_hp(&mut state.rng.monster_hp_rng, state.meta.ascension_level);
                let right_hp =
                    bronze_orb_spawn_hp(&mut state.rng.monster_hp_rng, state.meta.ascension_level);
                let automaton_draw_x = entity.protocol_identity.draw_x;

                // Spawn 2 Orbs
                // Java uses smart positioning based on drawX:
                //   BronzeOrb(-300, 200, 0) -> drawX < Automaton(-50) -> inserts at position 0
                //   BronzeOrb(200, 130, 1)  -> drawX > Automaton(-50) -> inserts at position 2
                // After spawn: [Orb, Automaton, Orb]
                actions.push(Action::SpawnMonster {
                    monster_id: EnemyId::BronzeOrb,
                    slot: 0, // Inserted BEFORE automaton (Java drawX=-300 < automaton drawX=-50)
                    current_hp: left_hp,
                    max_hp: left_hp,
                    logical_position: -1,
                    protocol_draw_x: automaton_draw_x.map(|x| x - 167),
                    is_minion: true,
                });
                actions.push(Action::SpawnMonster {
                    monster_id: EnemyId::BronzeOrb,
                    slot: 2, // Inserted AFTER automaton (Java drawX=200 > automaton drawX=-50)
                    current_hp: right_hp,
                    max_hp: right_hp,
                    logical_position: 1,
                    protocol_draw_x: automaton_draw_x.map(|x| x + 166),
                    is_minion: true,
                });
            }
            1 => {
                let dmg = if state.meta.ascension_level >= 4 {
                    8
                } else {
                    7
                };
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            5 => {
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: block_amt,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Strength,
                    amount: str_amt,
                });
            }
            2 => {
                let dmg = if state.meta.ascension_level >= 4 {
                    50
                } else {
                    45
                };
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            3 => {
                // Stunned
            }
            _ => {}
        }
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}

fn bronze_orb_spawn_hp(hp_rng: &mut crate::rng::StsRng, ascension_level: u8) -> i32 {
    // Java BronzeOrb constructor consumes monsterHpRng twice:
    // once in super(... random(52,58) ...), then again in setHp(...)
    let _unused_ctor_roll = hp_rng.random_range(52, 58);
    if ascension_level >= 9 {
        hp_rng.random_range(54, 60)
    } else {
        hp_rng.random_range(52, 58)
    }
}
