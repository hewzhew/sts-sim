use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent};
use crate::content::cards::CardId;
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Nemesis;

impl Nemesis {
    /// Reconstruct scythe cooldown from move history.
    /// Java: scytheCooldown starts at 0, decremented each getMove call (including initial),
    /// resets to 2 when Scythe (byte 3) is chosen.
    /// Since getMove is called (history_len + 1) times total, we need to track:
    ///   - Find last Scythe (byte 3) in history
    ///   - cooldown = 2 - (turns_since_scythe)
    /// Note: The decrement happens BEFORE the decision, so the cooldown after setting
    /// Scythe = 2 is decremented on the next getMove call.
    fn reconstruct_scythe_cooldown(history: &std::collections::VecDeque<u8>) -> i32 {
        // Find position of last Scythe usage (byte 3)
        let mut last_scythe_idx: Option<usize> = None;
        for (i, &m) in history.iter().enumerate() {
            if m == 3 {
                last_scythe_idx = Some(i);
            }
        }

        match last_scythe_idx {
            Some(idx) => {
                // Turns elapsed since scythe was selected (inclusive of the turn after)
                let turns_since = (history.len() - 1 - idx) as i32;
                // Java: cooldown set to 2 when scythe chosen, decremented each subsequent getMove
                // The current getMove call also decrements, so:
                // cooldown_before_this_call = 2 - turns_since
                // cooldown_after_decrement = 2 - turns_since - 1 = 1 - turns_since
                1 - turns_since
            }
            None => {
                // Never used scythe: cooldown started at 0 and has been decremented
                // (history.len() + 1) times (initial call + each roll_move call)
                // But it starts at 0 and only decrements, so it's always <= 0
                -(history.len() as i32) - 1
            }
        }
    }
}

impl MonsterBehavior for Nemesis {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &crate::runtime::combat::MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let fire_dmg = if ascension_level >= 3 { 7 } else { 6 };
        // Java: firstMove flag — true only for the very first getMove call
        if entity.move_history.is_empty() {
            if num < 50 {
                return (
                    2,
                    Intent::Attack {
                        damage: fire_dmg,
                        hits: 3,
                    },
                );
            } else {
                return (4, Intent::Debuff);
            }
        }

        // Reconstruct scythe cooldown (already decremented for this call)
        let scythe_cooldown = Nemesis::reconstruct_scythe_cooldown(&entity.move_history);

        let last_move = entity.move_history.back().copied().unwrap_or(0);
        let last_two_moves = |byte: u8| {
            entity.move_history.len() >= 2
                && entity.move_history[entity.move_history.len() - 1] == byte
                && entity.move_history[entity.move_history.len() - 2] == byte
        };

        // Java: getMove L149-184 — 3 branches based on num
        if num < 30 {
            if last_move != 3 && scythe_cooldown <= 0 {
                // Scythe available and not last move
                return (
                    3,
                    Intent::Attack {
                        damage: 45,
                        hits: 1,
                    },
                );
            } else if _rng.random_range(0, 1) == 0 {
                // Java: aiRng.randomBoolean() — 50/50
                if !last_two_moves(2) {
                    return (
                        2,
                        Intent::Attack {
                            damage: fire_dmg,
                            hits: 3,
                        },
                    );
                } else {
                    return (4, Intent::Debuff);
                }
            } else if last_move != 4 {
                return (4, Intent::Debuff);
            } else {
                return (
                    2,
                    Intent::Attack {
                        damage: fire_dmg,
                        hits: 3,
                    },
                );
            }
        } else if num < 65 {
            if !last_two_moves(2) {
                return (
                    2,
                    Intent::Attack {
                        damage: fire_dmg,
                        hits: 3,
                    },
                );
            } else if _rng.random_range(0, 1) == 0 {
                // Java: aiRng.randomBoolean()
                if scythe_cooldown > 0 {
                    return (4, Intent::Debuff);
                } else {
                    return (
                        3,
                        Intent::Attack {
                            damage: 45,
                            hits: 1,
                        },
                    );
                }
            } else {
                return (4, Intent::Debuff);
            }
        } else {
            if last_move != 4 {
                return (4, Intent::Debuff);
            } else if _rng.random_range(0, 1) == 0 && scythe_cooldown <= 0 {
                return (
                    3,
                    Intent::Attack {
                        damage: 45,
                        hits: 1,
                    },
                );
            } else {
                return (
                    2,
                    Intent::Attack {
                        damage: fire_dmg,
                        hits: 3,
                    },
                );
            }
        }
    }

    fn take_turn(
        state: &mut CombatState,
        entity: &crate::runtime::combat::MonsterEntity,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.meta.ascension_level;

        let fire_dmg = if asc >= 3 { 7 } else { 6 };

        match entity.next_move_byte {
            3 => {
                // SCYTHE
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: 45,
                    output: 45,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            2 => {
                // TRI_ATTACK
                for _ in 0..3 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: fire_dmg,
                        output: fire_dmg,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            4 => {
                // TRI_BURN
                let burn_amt = if asc >= 18 { 5 } else { 3 };
                actions.push(Action::MakeTempCardInDiscard {
                    card_id: CardId::Burn,
                    amount: burn_amt,
                    upgraded: false,
                });
            }
            _ => {}
        }

        // Java: if (!this.hasPower("Intangible")) { apply }
        let has_intangible = state
            .entities
            .power_db
            .get(&entity.id)
            .map(|powers| {
                powers
                    .iter()
                    .any(|p| p.power_type == PowerId::Intangible && p.amount > 0)
            })
            .unwrap_or(false);
        if !has_intangible {
            actions.push(Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Intangible,
                amount: 1,
            });
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
