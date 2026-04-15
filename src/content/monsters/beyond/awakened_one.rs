use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct AwakenedOne;

impl MonsterBehavior for AwakenedOne {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &crate::runtime::combat::MonsterEntity,
        _ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        // Phase detection: check for Unawakened power
        let is_phase_one = !entity.move_history.contains(&3);

        if entity.current_hp <= 0 && is_phase_one {
            return (3, Intent::Unknown); // REBIRTH TRIGGER
        }

        if is_phase_one {
            // Phase 1 Logic
            let slash_dmg = 20;
            let soul_strike_dmg = 6;

            if entity.move_history.is_empty() {
                return (
                    1,
                    Intent::Attack {
                        damage: slash_dmg,
                        hits: 1,
                    },
                );
            }
            let last_move = entity.move_history.back().copied().unwrap_or(0);
            let last_two_moves = |byte| {
                entity.move_history.len() >= 2
                    && entity.move_history[entity.move_history.len() - 1] == byte
                    && entity.move_history[entity.move_history.len() - 2] == byte
            };

            if num < 25 {
                if last_move != 2 {
                    return (
                        2,
                        Intent::Attack {
                            damage: soul_strike_dmg,
                            hits: 4,
                        },
                    );
                } else {
                    return (
                        1,
                        Intent::Attack {
                            damage: slash_dmg,
                            hits: 1,
                        },
                    );
                }
            } else if !last_two_moves(1) {
                return (
                    1,
                    Intent::Attack {
                        damage: slash_dmg,
                        hits: 1,
                    },
                );
            } else {
                return (
                    2,
                    Intent::Attack {
                        damage: soul_strike_dmg,
                        hits: 4,
                    },
                );
            }
        } else {
            // Phase 2 Logic
            let dark_echo_dmg = 40;
            let sludge_dmg = 18;
            let tackle_dmg = 10;

            // First move after REBIRTH (byte 3) is always Dark Echo
            let last_move = entity.move_history.back().copied().unwrap_or(0);
            if last_move == 3 {
                return (
                    5,
                    Intent::Attack {
                        damage: dark_echo_dmg,
                        hits: 1,
                    },
                );
            }
            let last_two_moves = |byte| {
                entity.move_history.len() >= 2
                    && entity.move_history[entity.move_history.len() - 1] == byte
                    && entity.move_history[entity.move_history.len() - 2] == byte
            };

            if num < 50 {
                if !last_two_moves(6) {
                    return (
                        6,
                        Intent::AttackDebuff {
                            damage: sludge_dmg,
                            hits: 1,
                        },
                    );
                } else {
                    return (
                        8,
                        Intent::Attack {
                            damage: tackle_dmg,
                            hits: 3,
                        },
                    );
                }
            } else if !last_two_moves(8) {
                return (
                    8,
                    Intent::Attack {
                        damage: tackle_dmg,
                        hits: 3,
                    },
                );
            } else {
                return (
                    6,
                    Intent::AttackDebuff {
                        damage: sludge_dmg,
                        hits: 1,
                    },
                );
            }
        }
    }

    fn use_pre_battle_action(
        entity: &crate::runtime::combat::MonsterEntity,
        _hp_rng: &mut crate::runtime::rng::StsRng,
        ascension_level: u8,
    ) -> Vec<Action> {
        let mut actions = Vec::new();

        let regen_amt = if ascension_level >= 19 { 15 } else { 10 };
        let curiosity_amt = if ascension_level >= 19 { 2 } else { 1 };

        actions.push(Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Regen,
            amount: regen_amt,
        });

        actions.push(Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Curiosity, // Requires registration
            amount: curiosity_amt,
        });

        actions.push(Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Unawakened, // Requires registration
            amount: 1,
        });

        if ascension_level >= 4 {
            actions.push(Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Strength,
                amount: 2,
            });
        }

        actions
    }

    fn take_turn(
        state: &mut CombatState,
        entity: &crate::runtime::combat::MonsterEntity,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        let _asc = state.meta.ascension_level;

        match entity.next_move_byte {
            1 => {
                // SLASH
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: 20,
                    output: 20,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            2 => {
                // SOUL STRIKE
                for _ in 0..4 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: 6,
                        output: 6,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            3 => {
                // REBIRTH TRIGGER
                if let Some(monster) = state
                    .entities
                    .monsters
                    .iter_mut()
                    .find(|m| m.id == entity.id)
                {
                    monster.half_dead = false;
                }
                let asc = state.meta.ascension_level;
                let heal_amt = if asc >= 9 { 320 } else { 300 };
                actions.push(Action::Heal {
                    target: entity.id,
                    amount: heal_amt,
                });
            }
            5 => {
                // DARK ECHO
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: 40,
                    output: 40,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            6 => {
                // SLUDGE
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: 18,
                    output: 18,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                // Need MakeTempCardInDrawPileAction equivalent (Void)
                actions.push(Action::MakeTempCardInDrawPile {
                    card_id: crate::content::cards::CardId::Void,
                    amount: 1,
                    random_spot: true,
                    upgraded: false,
                });
            }
            8 => {
                // TACKLE
                for _ in 0..3 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: 10,
                        output: 10,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
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
