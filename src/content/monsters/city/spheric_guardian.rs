use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct SphericGuardian;

impl MonsterBehavior for SphericGuardian {
    fn use_pre_battle_action(
        entity: &MonsterEntity,
        _hp_rng: &mut crate::runtime::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        vec![
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Barricade,
                amount: 1,
            },
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Artifact,
                amount: 3,
            },
            Action::GainBlock {
                target: entity.id,
                amount: 40,
            },
        ]
    }

    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let dmg = if ascension_level >= 2 { 11 } else { 10 };

        if entity.move_history.is_empty() {
            return (2, Intent::Defend);
        }

        if entity.move_history.len() == 1 {
            return (
                4,
                Intent::AttackDebuff {
                    damage: dmg,
                    hits: 1,
                },
            );
        }

        if entity.move_history.back() == Some(&1) {
            (
                3,
                Intent::AttackDefend {
                    damage: dmg,
                    hits: 1,
                },
            )
        } else {
            (
                1,
                Intent::Attack {
                    damage: dmg,
                    hits: 2,
                },
            )
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let target = 0; // Player
        let asc = state.meta.ascension_level;
        let dmg = if asc >= 2 { 11 } else { 10 };

        match entity.next_move_byte {
            1 => {
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            2 => {
                let block_amt = if asc >= 17 { 35 } else { 25 };
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: block_amt,
                });
            }
            3 => {
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: 15,
                });
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            4 => {
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target,
                    power_id: PowerId::Frail,
                    amount: 5,
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
