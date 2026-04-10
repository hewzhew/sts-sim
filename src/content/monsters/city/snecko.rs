use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent, MonsterEntity};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Snecko;

impl MonsterBehavior for Snecko {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let tail_dmg = if ascension_level >= 2 { 10 } else { 8 };
        let bite_dmg = if ascension_level >= 2 { 18 } else { 15 };

        if entity.move_history.is_empty() {
            return (1, Intent::StrongDebuff);
        }
        if num < 40 {
            return (
                3,
                Intent::AttackDebuff {
                    damage: tail_dmg,
                    hits: 1,
                },
            );
        }

        if entity.move_history.len() >= 2
            && entity.move_history[entity.move_history.len() - 1] == 2
            && entity.move_history[entity.move_history.len() - 2] == 2
        {
            return (
                3,
                Intent::AttackDebuff {
                    damage: tail_dmg,
                    hits: 1,
                },
            );
        } else {
            return (
                2,
                Intent::Attack {
                    damage: bite_dmg,
                    hits: 1,
                },
            );
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let tail_dmg = if state.meta.ascension_level >= 2 {
            10
        } else {
            8
        };
        let bite_dmg = if state.meta.ascension_level >= 2 {
            18
        } else {
            15
        };

        match entity.next_move_byte {
            1 => {
                // GLARE
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Confusion,
                    amount: 1,
                });
            }
            2 => {
                // BITE
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: bite_dmg,
                    output: bite_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            3 => {
                // TAIL
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: tail_dmg,
                    output: tail_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                if state.meta.ascension_level >= 17 {
                    actions.push(Action::ApplyPower {
                        source: entity.id,
                        target: 0,
                        power_id: PowerId::Weak,
                        amount: 2,
                    });
                }
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Vulnerable,
                    amount: 2,
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
