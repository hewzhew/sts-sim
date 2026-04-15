use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct BanditLeader;

impl MonsterBehavior for BanditLeader {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &crate::runtime::combat::MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        if entity.move_history.is_empty() {
            return (2, Intent::Unknown); // MOCK
        }

        let slash_dmg = if ascension_level >= 2 { 17 } else { 15 };
        let agonize_dmg = if ascension_level >= 2 { 12 } else { 10 };

        let last_move = entity.move_history.back().copied().unwrap_or(0);
        let last_two_moves = |byte| {
            entity.move_history.len() >= 2
                && entity.move_history[entity.move_history.len() - 1] == byte
                && entity.move_history[entity.move_history.len() - 2] == byte
        };

        if last_move == 2 {
            (
                3,
                Intent::AttackDebuff {
                    damage: agonize_dmg,
                    hits: 1,
                },
            ) // AGONIZING_SLASH
        } else if last_move == 3 {
            (
                1,
                Intent::Attack {
                    damage: slash_dmg,
                    hits: 1,
                },
            ) // CROSS_SLASH
        } else if last_move == 1 {
            if ascension_level >= 17 && !last_two_moves(1) {
                (
                    1,
                    Intent::Attack {
                        damage: slash_dmg,
                        hits: 1,
                    },
                ) // CROSS_SLASH again
            } else {
                (
                    3,
                    Intent::AttackDebuff {
                        damage: agonize_dmg,
                        hits: 1,
                    },
                ) // AGONIZING_SLASH
            }
        } else {
            (
                1,
                Intent::Attack {
                    damage: slash_dmg,
                    hits: 1,
                },
            ) // default fallback
        }
    }

    fn take_turn(
        state: &mut CombatState,
        entity: &crate::runtime::combat::MonsterEntity,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.meta.ascension_level;
        let weak_amount = if asc >= 17 { 3 } else { 2 };
        let slash_dmg = if asc >= 2 { 17 } else { 15 };
        let agonize_dmg = if asc >= 2 { 12 } else { 10 };

        match entity.next_move_byte {
            2 => { // MOCK
                 // Empty action physically, normally does a TalkAction depending on Bear alive state
            }
            3 => {
                // AGONIZING_SLASH
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: agonize_dmg,
                    output: agonize_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Weak,
                    amount: weak_amount,
                });
            }
            1 => {
                // CROSS_SLASH
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: slash_dmg,
                    output: slash_dmg,
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
