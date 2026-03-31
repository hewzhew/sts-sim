use crate::combat::{CombatState, MonsterEntity, Intent};
use crate::action::{Action, DamageType, DamageInfo};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct ShelledParasite;

impl ShelledParasite {
    fn get_move_from_num(rng: &mut crate::rng::StsRng, mut num: i32, move_history: &std::collections::VecDeque<u8>, ascension_level: u8, double_strike_dmg: i32, fell_dmg: i32, suck_dmg: i32) -> (u8, Intent) {
        let last_move_1 = move_history.len() >= 1 && move_history[move_history.len() - 1] == 1;
        let last_two_moves_2 = move_history.len() >= 2 && move_history[move_history.len() - 1] == 2 && move_history[move_history.len() - 2] == 2;
        let last_two_moves_3 = move_history.len() >= 2 && move_history[move_history.len() - 1] == 3 && move_history[move_history.len() - 2] == 3;

        if num < 20 {
            if !last_move_1 {
                return (1, Intent::AttackDebuff { damage: fell_dmg, hits: 1 });
            } else {
                num = rng.random_range(20, 99);
                return Self::get_move_from_num(rng, num, move_history, ascension_level, double_strike_dmg, fell_dmg, suck_dmg);
            }
        } else if num < 60 {
            if !last_two_moves_2 {
                return (2, Intent::Attack { damage: double_strike_dmg, hits: 2 });
            } else {
                return (3, Intent::AttackBuff { damage: suck_dmg, hits: 1 });
            }
        } else if !last_two_moves_3 {
            return (3, Intent::AttackBuff { damage: suck_dmg, hits: 1 });
        }
        
        (2, Intent::Attack { damage: double_strike_dmg, hits: 2 })
    }
}

impl MonsterBehavior for ShelledParasite {
    fn use_pre_battle_action(entity: &MonsterEntity, _rng: &mut crate::rng::StsRng, _ascension_level: u8) -> Vec<Action> {
        vec![
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::PlatedArmor,
                amount: 14,
            },
            Action::GainBlock {
                target: entity.id,
                amount: 14,
            }
        ]
    }

    fn roll_move(rng: &mut crate::rng::StsRng, entity: &MonsterEntity, ascension_level: u8, num: i32) -> (u8, Intent) {
        let double_strike_dmg = if ascension_level >= 2 { 7 } else { 6 };
        let fell_dmg = if ascension_level >= 2 { 21 } else { 18 };
        let suck_dmg = if ascension_level >= 2 { 12 } else { 10 };

        if entity.move_history.is_empty() {
            if ascension_level >= 17 {
                return (1, Intent::AttackDebuff { damage: fell_dmg, hits: 1 });
            }
            if rng.random_boolean() {
                return (2, Intent::Attack { damage: double_strike_dmg, hits: 2 });
            } else {
                return (3, Intent::AttackBuff { damage: suck_dmg, hits: 1 });
            }
        }
        Self::get_move_from_num(rng, num, &entity.move_history, ascension_level, double_strike_dmg, fell_dmg, suck_dmg)
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let double_strike_dmg = if state.ascension_level >= 2 { 7 } else { 6 };
        let fell_dmg = if state.ascension_level >= 2 { 21 } else { 18 };
        let suck_dmg = if state.ascension_level >= 2 { 12 } else { 10 };

        match entity.next_move_byte {
            1 => { // FELL
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: fell_dmg,
                    output: fell_dmg,
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
            2 => { // DOUBLE STRIKE
                for _ in 0..2 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: double_strike_dmg,
                        output: double_strike_dmg,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            3 => { // LIFE SUCK
                actions.push(Action::VampireDamage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: suck_dmg,
                    output: suck_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            4 => { // STUNNED
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 1, // Preps FELL next turn
                    intent: Intent::AttackDebuff { damage: fell_dmg, hits: 1 },
                });
            }
            _ => {}
        }

        if entity.next_move_byte != 4 {
            actions.push(Action::RollMonsterMove { monster_id: entity.id });
        }
        actions
    }
}
