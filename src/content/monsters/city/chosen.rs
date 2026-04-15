use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent, MonsterEntity};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Chosen;

impl MonsterBehavior for Chosen {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let zap_dmg = if ascension_level >= 2 { 21 } else { 18 };
        let debilitate_dmg = if ascension_level >= 2 { 12 } else { 10 };
        let poke_dmg = if ascension_level >= 2 { 6 } else { 5 };
        let (first_turn, used_hex) = if entity.chosen.protocol_seeded {
            (entity.chosen.first_turn, entity.chosen.used_hex)
        } else {
            (
                entity.move_history.is_empty(),
                entity.move_history.contains(&4),
            )
        };

        if ascension_level >= 17 {
            if !used_hex {
                return (4, Intent::StrongDebuff);
            }
        } else {
            if first_turn {
                return (
                    5,
                    Intent::Attack {
                        damage: poke_dmg,
                        hits: 2,
                    },
                );
            }
            if !used_hex {
                return (4, Intent::StrongDebuff);
            }
        }
        let last_move = entity.move_history.back().copied().unwrap_or(0);
        if last_move != 3 && last_move != 2 {
            if num < 50 {
                return (
                    3,
                    Intent::AttackDebuff {
                        damage: debilitate_dmg,
                        hits: 1,
                    },
                );
            }
            return (2, Intent::Debuff);
        }

        if num < 40 {
            return (
                1,
                Intent::Attack {
                    damage: zap_dmg,
                    hits: 1,
                },
            );
        }

        (
            5,
            Intent::Attack {
                damage: poke_dmg,
                hits: 2,
            },
        )
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let target = 0; // Player
        let asc = state.meta.ascension_level;

        let zap_dmg = if asc >= 2 { 21 } else { 18 };
        let debilitate_dmg = if asc >= 2 { 12 } else { 10 };
        let poke_dmg = if asc >= 2 { 6 } else { 5 };

        match entity.next_move_byte {
            1 => {
                // Zap
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target,
                    base: zap_dmg,
                    output: zap_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            2 => {
                // Drain
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target,
                    power_id: PowerId::Weak,
                    amount: 3,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Strength,
                    amount: 3,
                });
            }
            3 => {
                // Debilitate
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target,
                    base: debilitate_dmg,
                    output: debilitate_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target,
                    power_id: PowerId::Vulnerable,
                    amount: 2,
                });
            }
            4 => {
                // Hex
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target,
                    power_id: PowerId::Hex,
                    amount: 1,
                });
            }
            5 => {
                // Poke
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target,
                    base: poke_dmg,
                    output: poke_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target,
                    base: poke_dmg,
                    output: poke_dmg,
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

