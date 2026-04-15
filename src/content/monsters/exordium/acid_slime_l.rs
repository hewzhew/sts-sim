use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent, MonsterEntity, PowerId};
use crate::content::monsters::MonsterBehavior;

pub struct AcidSlimeL;

const ACID_SLIME_M_SPLIT_OFFSET_X: i32 = 134;

impl MonsterBehavior for AcidSlimeL {
    fn use_pre_battle_action(
        entity: &MonsterEntity,
        _hp_rng: &mut crate::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        vec![Action::ApplyPower {
            target: entity.id,
            source: entity.id,
            power_id: PowerId::Split,
            amount: -1,
        }]
    }

    fn roll_move(
        rng: &mut crate::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let w_tackle_dmg = if ascension_level >= 2 { 12 } else { 11 };
        let n_tackle_dmg = if ascension_level >= 2 { 18 } else { 16 };

        // 1: WOUND_TACKLE (Attack + Debuff), 2: NORMAL_TACKLE, 3: SPLIT, 4: WEAK_LICK
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
                    if rng.random_boolean_chance(0.6) {
                        (
                            2,
                            Intent::Attack {
                                damage: n_tackle_dmg,
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
                            damage: w_tackle_dmg,
                            hits: 1,
                        },
                    )
                }
            } else if num < 70 {
                if last_two_moves_were(2) {
                    if rng.random_boolean_chance(0.6) {
                        (
                            1,
                            Intent::AttackDebuff {
                                damage: w_tackle_dmg,
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
                            damage: n_tackle_dmg,
                            hits: 1,
                        },
                    )
                }
            } else if last_move == Some(4) {
                if rng.random_boolean_chance(0.4) {
                    (
                        1,
                        Intent::AttackDebuff {
                            damage: w_tackle_dmg,
                            hits: 1,
                        },
                    )
                } else {
                    (
                        2,
                        Intent::Attack {
                            damage: n_tackle_dmg,
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
                            damage: n_tackle_dmg,
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
                        damage: w_tackle_dmg,
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
                            damage: w_tackle_dmg,
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
                        damage: n_tackle_dmg,
                        hits: 1,
                    },
                )
            }
        } else if last_two_moves_were(4) {
            if rng.random_boolean_chance(0.4) {
                (
                    1,
                    Intent::AttackDebuff {
                        damage: w_tackle_dmg,
                        hits: 1,
                    },
                )
            } else {
                (
                    2,
                    Intent::Attack {
                        damage: n_tackle_dmg,
                        hits: 1,
                    },
                )
            }
        } else {
            (4, Intent::Debuff)
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let asc = state.meta.ascension_level;
        let w_tackle_dmg = if asc >= 2 { 12 } else { 11 };
        let n_tackle_dmg = if asc >= 2 { 18 } else { 16 };
        let _slimed_amt = if asc >= 17 { 2 } else { 2 };
        let weak_amt = 2;
        let mut actions = Vec::new();

        match entity.next_move_byte {
            1 => {
                // WOUND_TACKLE
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: w_tackle_dmg,
                    output: w_tackle_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::MakeTempCardInDiscard {
                    card_id: crate::content::cards::CardId::Slimed,
                    amount: 2,
                    upgraded: false,
                });
            }
            2 => {
                // NORMAL_TACKLE
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: n_tackle_dmg,
                    output: n_tackle_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            3 => {
                // SPLIT
                let base_draw_x = entity
                    .protocol_identity
                    .draw_x
                    .unwrap_or(entity.logical_position);
                actions.push(Action::Suicide { target: entity.id });
                actions.push(Action::SpawnMonsterSmart {
                    monster_id: crate::content::monsters::EnemyId::AcidSlimeM,
                    logical_position: base_draw_x - ACID_SLIME_M_SPLIT_OFFSET_X,
                    current_hp: entity.current_hp,
                    max_hp: entity.current_hp,
                    protocol_draw_x: Some(base_draw_x - ACID_SLIME_M_SPLIT_OFFSET_X),
                    is_minion: false,
                });
                actions.push(Action::SpawnMonsterSmart {
                    monster_id: crate::content::monsters::EnemyId::AcidSlimeM,
                    logical_position: base_draw_x + ACID_SLIME_M_SPLIT_OFFSET_X,
                    current_hp: entity.current_hp,
                    max_hp: entity.current_hp,
                    protocol_draw_x: Some(base_draw_x + ACID_SLIME_M_SPLIT_OFFSET_X),
                    is_minion: false,
                });
            }
            4 => {
                // WEAK_LICK
                actions.push(Action::ApplyPower {
                    target: 0,
                    source: entity.id,
                    power_id: PowerId::Weak,
                    amount: weak_amt,
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

