use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{Intent, MonsterEntity};
use crate::content::monsters::MonsterBehavior;

pub struct CorruptHeart;

impl MonsterBehavior for CorruptHeart {
    fn roll_move(
        rng: &mut crate::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let history = &entity.move_history;

        let echo_dmg = if ascension_level >= 4 { 45 } else { 40 };
        let blood_dmg = 2;
        let blood_hits = if ascension_level >= 4 { 15u8 } else { 12u8 };

        // Java: isFirstMove → byte 3 (DEBILITATE)
        if history.is_empty() {
            return (3, Intent::StrongDebuff);
        }

        // Java: moveCount (starts at 0, incremented AFTER getMove).
        // moveCount excludes the first Debilitate turn.
        // Pattern: moveCount%3 → 0: attack, 1: attack (no same as last), 2: buff (byte 4)
        let move_count = history.len() - 1; // subtract Debilitate first turn

        match move_count % 3 {
            0 => {
                // 50/50 Blood Shots vs Echo
                if rng.random_boolean() {
                    (
                        1,
                        Intent::Attack {
                            damage: blood_dmg,
                            hits: blood_hits,
                        },
                    )
                } else {
                    (
                        2,
                        Intent::Attack {
                            damage: echo_dmg,
                            hits: 1,
                        },
                    )
                }
            }
            1 => {
                // If last was Echo (2), do Blood (1); if last was Blood (1), do Echo (2)
                let last = history.back().copied().unwrap_or(0);
                if last != 2 {
                    (
                        2,
                        Intent::Attack {
                            damage: echo_dmg,
                            hits: 1,
                        },
                    )
                } else {
                    (
                        1,
                        Intent::Attack {
                            damage: blood_dmg,
                            hits: blood_hits,
                        },
                    )
                }
            }
            _ => {
                // Buff turn
                (4, Intent::Buff)
            }
        }
    }

    fn use_pre_battle_action(
        _entity: &MonsterEntity,
        _hp_rng: &mut crate::rng::StsRng,
        ascension_level: u8,
    ) -> Vec<Action> {
        let max_dmg = if ascension_level >= 19 { 200 } else { 300 };
        let beat_amt = if ascension_level >= 19 { 2 } else { 1 };

        vec![
            Action::ApplyPower {
                source: _entity.id,
                target: _entity.id,
                power_id: crate::content::powers::PowerId::Invincible,
                amount: max_dmg,
            },
            Action::ApplyPower {
                source: _entity.id,
                target: _entity.id,
                power_id: crate::content::powers::PowerId::BeatOfDeath,
                amount: beat_amt,
            },
        ]
    }

    fn take_turn(state: &mut crate::combat::CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.ascension_level;
        let move_byte = entity.next_move_byte;

        let echo_dmg = if asc >= 4 { 45 } else { 40 };
        let blood_dmg = 2;
        let blood_hits = if asc >= 4 { 15 } else { 12 };
        let debuff_amt = 2;

        // Buff count: how many times byte 4 has been played
        let buff_count = entity.move_history.iter().filter(|&&m| m == 4).count();

        match move_byte {
            3 => {
                // Debilitate
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: crate::content::powers::PowerId::Vulnerable,
                    amount: debuff_amt,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: crate::content::powers::PowerId::Weak,
                    amount: debuff_amt,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: crate::content::powers::PowerId::Frail,
                    amount: debuff_amt,
                });

                let statuses = vec![
                    crate::content::cards::CardId::Dazed,
                    crate::content::cards::CardId::Slimed,
                    crate::content::cards::CardId::Wound,
                    crate::content::cards::CardId::Burn,
                    crate::content::cards::CardId::Void,
                ];
                for s in statuses {
                    actions.push(Action::MakeTempCardInDrawPile {
                        card_id: s,
                        amount: 1,
                        random_spot: true,
                        upgraded: false,
                    });
                }
            }
            1 => {
                // Blood Shots
                for _ in 0..blood_hits {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: blood_dmg,
                        output: blood_dmg,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            2 => {
                // Echo Strike
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: echo_dmg,
                    output: echo_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            4 => {
                // Buff
                // Clear any negative strength first (Java: additionalAmount)
                let mut additional = 0;
                if let Some(powers) = state.power_db.get(&entity.id) {
                    if let Some(str_pow) = powers
                        .iter()
                        .find(|p| p.power_type == crate::content::powers::PowerId::Strength)
                    {
                        if str_pow.amount < 0 {
                            additional = -str_pow.amount;
                        }
                    }
                }

                // Always +2 Str (plus clearing negative)
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: crate::content::powers::PowerId::Strength,
                    amount: additional + 2,
                });

                // Java buff cycle: 0→Artifact(2), 1→BeatOfDeath(1), 2→PainfulStabs, 3→+10Str, 4+→+50Str
                match buff_count {
                    0 => {
                        actions.push(Action::ApplyPower {
                            source: entity.id,
                            target: entity.id,
                            power_id: crate::content::powers::PowerId::Artifact,
                            amount: 2,
                        });
                    }
                    1 => {
                        actions.push(Action::ApplyPower {
                            source: entity.id,
                            target: entity.id,
                            power_id: crate::content::powers::PowerId::BeatOfDeath,
                            amount: 1,
                        });
                    }
                    2 => {
                        actions.push(Action::ApplyPower {
                            source: entity.id,
                            target: entity.id,
                            power_id: crate::content::powers::PowerId::PainfulStabs,
                            amount: 1,
                        });
                    }
                    3 => {
                        actions.push(Action::ApplyPower {
                            source: entity.id,
                            target: entity.id,
                            power_id: crate::content::powers::PowerId::Strength,
                            amount: 10,
                        });
                    }
                    _ => {
                        actions.push(Action::ApplyPower {
                            source: entity.id,
                            target: entity.id,
                            power_id: crate::content::powers::PowerId::Strength,
                            amount: 50,
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
