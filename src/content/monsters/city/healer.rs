use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent, MonsterEntity};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Healer;

impl Healer {
    pub fn roll_move_custom(
        rng: &mut crate::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
        monsters: &[MonsterEntity],
    ) -> (u8, Intent) {
        let magic_dmg = if ascension_level >= 2 { 9 } else { 8 };
        let mut need_to_heal = 0;
        for m in monsters {
            if !m.is_dying && !m.is_escaped {
                need_to_heal += m.max_hp - m.current_hp;
            }
        }

        let num = rng.random_range(0, 99);
        let move_history = &entity.move_history;

        let last_two_moves_2 = move_history.len() >= 2
            && move_history[move_history.len() - 1] == 2
            && move_history[move_history.len() - 2] == 2;
        let last_two_moves_3 = move_history.len() >= 2
            && move_history[move_history.len() - 1] == 3
            && move_history[move_history.len() - 2] == 3;
        let last_two_moves_1 = move_history.len() >= 2
            && move_history[move_history.len() - 1] == 1
            && move_history[move_history.len() - 2] == 1;
        let last_move_1 = move_history.len() >= 1 && move_history[move_history.len() - 1] == 1;

        if ascension_level >= 17 {
            if need_to_heal > 20 && !last_two_moves_2 {
                return (2, Intent::Buff);
            }
            if num >= 40 && !last_move_1 {
                return (
                    1,
                    Intent::AttackDebuff {
                        damage: magic_dmg,
                        hits: 1,
                    },
                );
            }
        } else {
            if need_to_heal > 15 && !last_two_moves_2 {
                return (2, Intent::Buff);
            }
            if num >= 40 && !last_two_moves_1 {
                return (
                    1,
                    Intent::AttackDebuff {
                        damage: magic_dmg,
                        hits: 1,
                    },
                );
            }
        }

        if !last_two_moves_3 {
            return (3, Intent::Buff);
        }

        (
            1,
            Intent::AttackDebuff {
                damage: magic_dmg,
                hits: 1,
            },
        )
    }
}

impl MonsterBehavior for Healer {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        _entity: &MonsterEntity,
        _ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        unreachable!("Healer requires roll_move_custom with monsters slice");
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let magic_dmg = if state.meta.ascension_level >= 2 {
            9
        } else {
            8
        };
        let heal_amt = if state.meta.ascension_level >= 17 {
            20
        } else {
            16
        };
        let str_amt = if state.meta.ascension_level >= 17 {
            4
        } else if state.meta.ascension_level >= 2 {
            3
        } else {
            2
        };

        match entity.next_move_byte {
            1 => {
                // ATTACK
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0, // Player
                    base: magic_dmg,
                    output: magic_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Frail,
                    amount: 2,
                });
            }
            2 => {
                // HEAL
                for m in state.entities.monsters.iter() {
                    if !m.is_dying && !m.is_escaped {
                        actions.push(Action::Heal {
                            target: m.id,
                            amount: heal_amt,
                        });
                    }
                }
            }
            3 => {
                // BUFF
                for m in state.entities.monsters.iter() {
                    if !m.is_dying && !m.is_escaped {
                        actions.push(Action::ApplyPower {
                            source: entity.id,
                            target: m.id,
                            power_id: PowerId::Strength,
                            amount: str_amt,
                        });
                    }
                }
            }
            _ => {}
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
