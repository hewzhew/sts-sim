use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Darkling;

impl MonsterBehavior for Darkling {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &crate::combat::MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let chomp_dmg = if ascension_level >= 2 { 9 } else { 8 };
        let nip_dmg = _rng.random_range(
            if ascension_level >= 2 { 9 } else { 7 },
            if ascension_level >= 2 { 13 } else { 11 },
        ) as i32;

        // MVP: Handle halfDead logically externally or simplify to 0-HP checking.
        if entity.current_hp <= 0 {
            return (4, Intent::Unknown); // REINCARNATE
        }

        if entity.move_history.is_empty() {
            if num < 50 {
                return (
                    2,
                    if ascension_level >= 17 {
                        Intent::DefendBuff
                    } else {
                        Intent::Defend
                    },
                );
            } else {
                return (
                    3,
                    Intent::Attack {
                        damage: nip_dmg,
                        hits: 1,
                    },
                );
            }
        }
        let last_move = entity.move_history.back().copied().unwrap_or(0);

        let last_two_moves = |byte| {
            entity.move_history.len() >= 2
                && entity.move_history[entity.move_history.len() - 1] == byte
                && entity.move_history[entity.move_history.len() - 2] == byte
        };

        if num < 40 {
            if last_move != 1 {
                (
                    1,
                    Intent::Attack {
                        damage: chomp_dmg,
                        hits: 2,
                    },
                )
            } else {
                (
                    3,
                    Intent::Attack {
                        damage: nip_dmg,
                        hits: 1,
                    },
                ) // Simplified fallback from deep ai_rng nested
            }
        } else if num < 70 {
            if last_move != 2 {
                (
                    2,
                    if ascension_level >= 17 {
                        Intent::DefendBuff
                    } else {
                        Intent::Defend
                    },
                )
            } else {
                (
                    3,
                    Intent::Attack {
                        damage: nip_dmg,
                        hits: 1,
                    },
                )
            }
        } else if !last_two_moves(3) {
            (
                3,
                Intent::Attack {
                    damage: nip_dmg,
                    hits: 1,
                },
            )
        } else {
            (
                2,
                if ascension_level >= 17 {
                    Intent::DefendBuff
                } else {
                    Intent::Defend
                },
            ) // Simplified fallback
        }
    }

    fn use_pre_battle_action(
        entity: &crate::combat::MonsterEntity,
        _hp_rng: &mut crate::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Regrow,
            amount: 1, // Start logic placeholder
        }]
    }

    fn take_turn(state: &mut CombatState, entity: &crate::combat::MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.ascension_level;

        match entity.next_move_byte {
            1 => {
                // CHOMP
                let dmg = if asc >= 2 { 9 } else { 8 };
                for _ in 0..2 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: dmg,
                        output: dmg,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            2 => {
                // HARDEN
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: 12,
                });
                if asc >= 17 {
                    actions.push(Action::ApplyPower {
                        source: entity.id,
                        target: entity.id,
                        power_id: PowerId::Strength,
                        amount: 2,
                    });
                }
            }
            3 => {
                // NIP
                let dmg = if asc >= 2 { 11 } else { 9 }; // Fallback average
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
                // REINCARNATE
                actions.push(Action::Heal {
                    target: entity.id,
                    amount: entity.max_hp / 2,
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
