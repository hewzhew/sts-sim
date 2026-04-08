use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Champ;

impl MonsterBehavior for Champ {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &crate::combat::MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let threshold_reached = entity.move_history.contains(&7);
        let hp_below_half = entity.current_hp < entity.max_hp / 2;

        // When crossing 50% HP threshold
        if hp_below_half && !threshold_reached {
            return (7, Intent::Buff); // ANGER
        }

        let last_move = |byte| entity.move_history.back() == Some(&byte);
        let last_move_before = |byte| {
            entity.move_history.len() >= 2
                && entity.move_history[entity.move_history.len() - 2] == byte
        };

        if threshold_reached {
            if !last_move(3) && !last_move_before(3) {
                return (
                    3,
                    Intent::Attack {
                        damage: 10,
                        hits: 2,
                    },
                ); // EXECUTE
            }
        }

        // Taunt cycle — Java: numTurns counter, resets to 0 after Taunt, fires when == 4.
        // We replicate by counting moves since last Taunt (byte 6) or start of history.
        if !threshold_reached {
            let turns_since_taunt = match entity.move_history.iter().rposition(|&b| b == 6) {
                Some(pos) => entity.move_history.len() - pos, // turns after last Taunt (inclusive of current)
                None => entity.move_history.len() + 1, // +1 because Java increments numTurns before check
            };
            if turns_since_taunt >= 4 {
                return (6, Intent::Debuff); // TAUNT
            }
        }
        let forge_times = entity.move_history.iter().filter(|&&m| m == 2).count();
        let forge_threshold = 2;

        if ascension_level >= 19 {
            if !last_move(2) && forge_times < forge_threshold && num <= 30 {
                return (2, Intent::DefendBuff); // DEFENSIVE_STANCE
            }
        } else if !last_move(2) && forge_times < forge_threshold && num <= 15 {
            return (2, Intent::DefendBuff); // DEFENSIVE_STANCE
        }

        if !last_move(5) && !last_move(2) && num <= 30 {
            return (5, Intent::Buff); // GLOAT
        }

        let slap_dmg = if ascension_level >= 4 { 14 } else { 12 };
        if !last_move(4) && num <= 55 {
            return (
                4,
                Intent::AttackDebuff {
                    damage: slap_dmg,
                    hits: 1,
                },
            ); // FACE_SLAP
        }

        let slash_dmg = if ascension_level >= 4 { 18 } else { 16 };
        if !last_move(1) {
            (
                1,
                Intent::Attack {
                    damage: slash_dmg,
                    hits: 1,
                },
            ) // HEAVY_SLASH
        } else {
            (
                4,
                Intent::AttackDebuff {
                    damage: slap_dmg,
                    hits: 1,
                },
            ) // FACE_SLAP
        }
    }

    fn take_turn(state: &mut CombatState, entity: &crate::combat::MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let ascension_level = state.ascension_level;

        let block_amt = if ascension_level >= 19 {
            20
        } else if ascension_level >= 9 {
            18
        } else {
            15
        };
        let forge_amt = if ascension_level >= 19 {
            7
        } else if ascension_level >= 9 {
            6
        } else {
            5
        };
        let str_amt = if ascension_level >= 19 {
            4
        } else if ascension_level >= 4 {
            3
        } else {
            2
        };

        let slash_dmg = if ascension_level >= 4 { 18 } else { 16 };
        let slap_dmg = if ascension_level >= 4 { 14 } else { 12 };

        match entity.next_move_byte {
            1 => {
                // HEAVY_SLASH
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: slash_dmg,
                    output: slash_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            2 => {
                // DEFENSIVE_STANCE
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: block_amt,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Metallicize,
                    amount: forge_amt,
                });
            }
            3 => {
                // EXECUTE
                for _ in 0..2 {
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
            4 => {
                // FACE_SLAP
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: slap_dmg,
                    output: slap_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Frail,
                    amount: 2,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Vulnerable,
                    amount: 2,
                });
            }
            5 => {
                // GLOAT
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Strength,
                    amount: str_amt,
                });
            }
            6 => {
                // TAUNT
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Weak,
                    amount: 2,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Vulnerable,
                    amount: 2,
                });
            }
            7 => {
                // ANGER
                actions.push(Action::RemoveAllDebuffs { target: entity.id });

                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Strength,
                    amount: str_amt * 3,
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
