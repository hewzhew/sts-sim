use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent};
use crate::content::cards::CardId;
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct WrithingMass;

impl MonsterBehavior for WrithingMass {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &crate::combat::MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let (big_dmg, multi_dmg, acc_dmg, debuff_dmg) = if ascension_level >= 2 {
            (38, 9, 16, 12)
        } else {
            (32, 7, 15, 10)
        };

        let last_move = entity.move_history.back().copied().unwrap_or(0);
        let used_mega_debuff = entity.move_history.iter().any(|&m| m == 4);

        if entity.move_history.is_empty() {
            if num < 33 {
                return (
                    1,
                    Intent::Attack {
                        damage: multi_dmg,
                        hits: 3,
                    },
                );
            } else if num < 66 {
                return (
                    2,
                    Intent::AttackDefend {
                        damage: acc_dmg,
                        hits: 1,
                    },
                );
            } else {
                return (
                    3,
                    Intent::AttackDebuff {
                        damage: debuff_dmg,
                        hits: 1,
                    },
                );
            }
        }
        if num < 10 {
            if last_move != 0 {
                return (
                    0,
                    Intent::Attack {
                        damage: big_dmg,
                        hits: 1,
                    },
                );
            } else {
                return (
                    1,
                    Intent::Attack {
                        damage: multi_dmg,
                        hits: 3,
                    },
                ); // Simplified fallback
            }
        } else if num < 20 {
            if !used_mega_debuff && last_move != 4 {
                return (4, Intent::StrongDebuff);
            } else if _rng.random_range(0, 9) == 0 {
                // 10% chance
                return (
                    0,
                    Intent::Attack {
                        damage: big_dmg,
                        hits: 1,
                    },
                );
            } else {
                return (
                    1,
                    Intent::Attack {
                        damage: multi_dmg,
                        hits: 3,
                    },
                ); // Simplified fallback
            }
        } else if num < 40 {
            if last_move != 3 {
                return (
                    3,
                    Intent::AttackDebuff {
                        damage: debuff_dmg,
                        hits: 1,
                    },
                );
            } else if _rng.random_range(0, 9) < 4 {
                // 40% chance
                return (
                    2,
                    Intent::AttackDefend {
                        damage: acc_dmg,
                        hits: 1,
                    },
                ); // fallback
            } else {
                return (
                    1,
                    Intent::Attack {
                        damage: multi_dmg,
                        hits: 3,
                    },
                ); // fallback
            }
        } else if num < 70 {
            if last_move != 1 {
                return (
                    1,
                    Intent::Attack {
                        damage: multi_dmg,
                        hits: 3,
                    },
                );
            } else if _rng.random_range(0, 9) < 3 {
                return (
                    2,
                    Intent::AttackDefend {
                        damage: acc_dmg,
                        hits: 1,
                    },
                );
            } else {
                return (
                    3,
                    Intent::AttackDebuff {
                        damage: debuff_dmg,
                        hits: 1,
                    },
                ); // fallback
            }
        } else if last_move != 2 {
            return (
                2,
                Intent::AttackDefend {
                    damage: acc_dmg,
                    hits: 1,
                },
            );
        } else {
            return (
                1,
                Intent::Attack {
                    damage: multi_dmg,
                    hits: 3,
                },
            ); // fallback
        }
    }

    fn use_pre_battle_action(
        entity: &crate::combat::MonsterEntity,
        _hp_rng: &mut crate::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        vec![
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Reactive,
                amount: 1,
            },
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Malleable,
                amount: 3,
            },
        ]
    }

    fn take_turn(state: &mut CombatState, entity: &crate::combat::MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.meta.ascension_level;

        let (big_dmg, multi_dmg, acc_dmg, debuff_dmg) = if asc >= 2 {
            (38, 9, 16, 12)
        } else {
            (32, 7, 15, 10)
        };

        let normal_debuff_amt = 2; // Vulnerable / Weak

        match entity.next_move_byte {
            0 => {
                // BIG_HIT
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: big_dmg,
                    output: big_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            1 => {
                // MULTI_HIT
                for _ in 0..3 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: multi_dmg,
                        output: multi_dmg,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            2 => {
                // ATTACK_BLOCK
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: acc_dmg,
                    output: acc_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: acc_dmg,
                });
            }
            3 => {
                // ATTACK_DEBUFF
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: debuff_dmg,
                    output: debuff_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Weak,
                    amount: normal_debuff_amt,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Vulnerable,
                    amount: normal_debuff_amt,
                });
            }
            4 => {
                // MEGA_DEBUFF
                actions.push(Action::AddCardToMasterDeck {
                    card_id: CardId::Parasite,
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
