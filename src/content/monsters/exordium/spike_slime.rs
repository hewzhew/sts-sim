use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent, MonsterEntity, PowerId};
use crate::content::monsters::MonsterBehavior;

// SpikeSlime_S
pub struct SpikeSlimeS;

impl MonsterBehavior for SpikeSlimeS {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        _entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        // ALWAYS tackles.
        let dmg = if ascension_level >= 2 { 6 } else { 5 };
        (
            1,
            Intent::Attack {
                damage: dmg,
                hits: 1,
            },
        )
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        match entity.next_move_byte {
            1 => {
                let dmg = if state.meta.ascension_level >= 2 {
                    6
                } else {
                    5
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
            _ => {}
        }
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}

// SpikeSlime_M
pub struct SpikeSlimeM;

impl MonsterBehavior for SpikeSlimeM {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let dmg = if ascension_level >= 2 { 10 } else { 8 };

        // 1: Attack + Debuff (Slimed array), 4: Frail
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
            if num < 30 {
                if last_two_moves_were(1) {
                    (4, Intent::Debuff)
                } else {
                    (
                        1,
                        Intent::AttackDebuff {
                            damage: dmg,
                            hits: 1,
                        },
                    )
                }
            } else if last_move == Some(4) {
                (
                    1,
                    Intent::AttackDebuff {
                        damage: dmg,
                        hits: 1,
                    },
                )
            } else {
                (4, Intent::Debuff)
            }
        } else if num < 30 {
            if last_two_moves_were(1) {
                (4, Intent::Debuff)
            } else {
                (
                    1,
                    Intent::AttackDebuff {
                        damage: dmg,
                        hits: 1,
                    },
                )
            }
        } else if last_two_moves_were(4) {
            (
                1,
                Intent::AttackDebuff {
                    damage: dmg,
                    hits: 1,
                },
            )
        } else {
            (4, Intent::Debuff)
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        match entity.next_move_byte {
            1 => {
                let dmg = if state.meta.ascension_level >= 2 {
                    10
                } else {
                    8
                };
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
            4 => {
                actions.push(Action::ApplyPower {
                    target: 0, // Player
                    source: entity.id,
                    power_id: PowerId::Frail,
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
