use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent, MonsterEntity};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Byrd;

impl MonsterBehavior for Byrd {
    fn use_pre_battle_action(
        entity: &MonsterEntity,
        _hp_rng: &mut crate::rng::StsRng,
        ascension_level: u8,
    ) -> Vec<Action> {
        let flight_amt = if ascension_level >= 17 { 4 } else { 3 };
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Flight,
            amount: flight_amt,
        }]
    }

    fn roll_move(
        rng: &mut crate::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let peck_dmg = 1;
        let peck_count = if ascension_level >= 2 { 6 } else { 5 };
        let swoop_dmg = if ascension_level >= 2 { 14 } else { 12 };
        let headbutt_dmg = 3;

        let is_flying = entity.move_history.back() != Some(&4);

        if entity.move_history.is_empty() {
            if rng.random_boolean_chance(0.375) {
                return (6, Intent::Buff);
            } else {
                return (
                    1,
                    Intent::Attack {
                        damage: peck_dmg,
                        hits: peck_count as u8,
                    },
                );
            }
        }

        if is_flying {
            if num < 50 {
                let mut rev = entity.move_history.iter().rev();
                if rev.next() == Some(&1) && rev.next() == Some(&1) {
                    if rng.random_boolean_chance(0.4) {
                        (
                            3,
                            Intent::Attack {
                                damage: swoop_dmg,
                                hits: 1,
                            },
                        )
                    } else {
                        (6, Intent::Buff)
                    }
                } else {
                    (
                        1,
                        Intent::Attack {
                            damage: peck_dmg,
                            hits: peck_count as u8,
                        },
                    )
                }
            } else if num < 70 {
                if entity.move_history.back() == Some(&3) {
                    if rng.random_boolean_chance(0.375) {
                        (6, Intent::Buff)
                    } else {
                        (
                            1,
                            Intent::Attack {
                                damage: peck_dmg,
                                hits: peck_count as u8,
                            },
                        )
                    }
                } else {
                    (
                        3,
                        Intent::Attack {
                            damage: swoop_dmg,
                            hits: 1,
                        },
                    )
                }
            } else if entity.move_history.back() == Some(&6) {
                if rng.random_boolean_chance(0.2857) {
                    (
                        3,
                        Intent::Attack {
                            damage: swoop_dmg,
                            hits: 1,
                        },
                    )
                } else {
                    (
                        1,
                        Intent::Attack {
                            damage: peck_dmg,
                            hits: peck_count as u8,
                        },
                    )
                }
            } else {
                (6, Intent::Buff)
            }
        } else {
            (
                5,
                Intent::Attack {
                    damage: headbutt_dmg,
                    hits: 1,
                },
            )
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let target = 0; // Player
        let asc = state.ascension_level;
        let peck_dmg = 1;
        let peck_count = if asc >= 2 { 6 } else { 5 };
        let swoop_dmg = if asc >= 2 { 14 } else { 12 };
        let headbutt_dmg = 3;

        match entity.next_move_byte {
            1 => {
                // Peck
                for _ in 0..peck_count {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target,
                        base: peck_dmg,
                        output: peck_dmg,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            2 => {
                // Go Airborne (Not typically used during combat via roll_move, but in case)
                let flight_amt = if asc >= 17 { 4 } else { 3 };
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Flight,
                    amount: flight_amt,
                });
            }
            3 => {
                // Swoop
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target,
                    base: swoop_dmg,
                    output: swoop_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            4 => { // Stunned
                 // No action
            }
            5 => {
                // Headbutt
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target,
                    base: headbutt_dmg,
                    output: headbutt_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                // After Headbutt, queue Go Airborne according to Java (setMove(2))
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 2,
                    intent: Intent::Unknown, // Or Intent::Buff
                });
            }
            6 => {
                // Caw
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Strength,
                    amount: 1,
                });
            }
            _ => {}
        }
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });

        actions
    }
}
