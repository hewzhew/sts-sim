use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent, MonsterEntity, PowerId};
use crate::content::monsters::MonsterBehavior;

// AcidSlime_S
pub struct AcidSlimeS;

impl MonsterBehavior for AcidSlimeS {
    fn roll_move(
        rng: &mut crate::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let dmg = if ascension_level >= 2 { 4 } else { 3 };
        let last_move = entity.move_history.back().copied();
        let last_move_before = if entity.move_history.len() >= 2 {
            entity
                .move_history
                .get(entity.move_history.len() - 2)
                .copied()
        } else {
            None
        };
        let last_two_moves_were_1 = last_move == Some(1) && last_move_before == Some(1);

        if ascension_level >= 17 {
            if last_two_moves_were_1 {
                (
                    1,
                    Intent::Attack {
                        damage: dmg,
                        hits: 1,
                    },
                )
            } else {
                (2, Intent::Debuff)
            }
        } else if rng.random_boolean() {
            (
                1,
                Intent::Attack {
                    damage: dmg,
                    hits: 1,
                },
            )
        } else {
            (2, Intent::Debuff)
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let dmg = if state.ascension_level >= 2 { 4 } else { 3 };
        match entity.next_move_byte {
            1 => {
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                // Java: this.setMove((byte)2, Intent.DEBUFF) — directly sets next move
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 2,
                    intent: Intent::Debuff,
                });
            }
            2 => {
                actions.push(Action::ApplyPower {
                    target: 0, // Player
                    source: entity.id,
                    power_id: PowerId::Weak,
                    amount: 1,
                });
                // Java: this.setMove((byte)1, Intent.ATTACK, damage) — directly sets next move
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 1,
                    intent: Intent::Attack {
                        damage: dmg,
                        hits: 1,
                    },
                });
            }
            _ => {}
        }
        actions
    }
}

// AcidSlime_M
pub struct AcidSlimeM;

impl MonsterBehavior for AcidSlimeM {
    fn roll_move(
        rng: &mut crate::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let dmg1 = if ascension_level >= 2 { 8 } else { 7 };
        let dmg2 = if ascension_level >= 2 { 12 } else { 10 };

        // 1: Attack + Debuff (Slimed array), 2: Attack, 4: Weak
        let last_move = entity.move_history.back().copied();
        let last_move_before = if entity.move_history.len() >= 2 {
            entity
                .move_history
                .get(entity.move_history.len() - 2)
                .copied()
        } else {
            None
        };
        let last_two_moves_were =
            |byte: u8| -> bool { last_move == Some(byte) && last_move_before == Some(byte) };

        if ascension_level >= 17 {
            if num < 40 {
                if last_two_moves_were(1) {
                    if rng.random_boolean() {
                        (
                            2,
                            Intent::Attack {
                                damage: dmg2,
                                hits: 1,
                            },
                        )
                    } else {
                        (4, Intent::Debuff)
                    }
                } else {
                    (
                        1,
                        Intent::AttackDebuff {
                            damage: dmg1,
                            hits: 1,
                        },
                    )
                }
            } else if num < 80 {
                if last_two_moves_were(2) {
                    if rng.random_boolean_chance(0.5) {
                        (
                            1,
                            Intent::AttackDebuff {
                                damage: dmg1,
                                hits: 1,
                            },
                        )
                    } else {
                        (4, Intent::Debuff)
                    }
                } else {
                    (
                        2,
                        Intent::Attack {
                            damage: dmg2,
                            hits: 1,
                        },
                    )
                }
            } else if last_move == Some(4) {
                if rng.random_boolean_chance(0.4) {
                    (
                        1,
                        Intent::AttackDebuff {
                            damage: dmg1,
                            hits: 1,
                        },
                    )
                } else {
                    (
                        2,
                        Intent::Attack {
                            damage: dmg2,
                            hits: 1,
                        },
                    )
                }
            } else {
                (4, Intent::Debuff)
            }
        } else if num < 30 {
            if last_two_moves_were(1) {
                if rng.random_boolean() {
                    (
                        2,
                        Intent::Attack {
                            damage: dmg2,
                            hits: 1,
                        },
                    )
                } else {
                    (4, Intent::Debuff)
                }
            } else {
                (
                    1,
                    Intent::AttackDebuff {
                        damage: dmg1,
                        hits: 1,
                    },
                )
            }
        } else if num < 70 {
            if last_move == Some(2) {
                if rng.random_boolean_chance(0.4) {
                    (
                        1,
                        Intent::AttackDebuff {
                            damage: dmg1,
                            hits: 1,
                        },
                    )
                } else {
                    (4, Intent::Debuff)
                }
            } else {
                (
                    2,
                    Intent::Attack {
                        damage: dmg2,
                        hits: 1,
                    },
                )
            }
        } else if last_two_moves_were(4) {
            if rng.random_boolean_chance(0.4) {
                (
                    1,
                    Intent::AttackDebuff {
                        damage: dmg1,
                        hits: 1,
                    },
                )
            } else {
                (
                    2,
                    Intent::Attack {
                        damage: dmg2,
                        hits: 1,
                    },
                )
            }
        } else {
            (4, Intent::Debuff)
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        match entity.next_move_byte {
            1 => {
                let dmg = if state.ascension_level >= 2 { 8 } else { 7 };
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                // MakeTempCardInDiscardAction (Slimed)
                actions.push(Action::MakeTempCardInDiscard {
                    card_id: crate::content::cards::CardId::Slimed,
                    amount: 1,
                    upgraded: false,
                });
            }
            2 => {
                let dmg = if state.ascension_level >= 2 { 12 } else { 10 };
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            4 => {
                actions.push(Action::ApplyPower {
                    target: 0, // Player
                    source: entity.id,
                    power_id: PowerId::Weak,
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
