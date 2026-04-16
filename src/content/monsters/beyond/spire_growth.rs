use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent};

pub struct SpireGrowth;

impl MonsterBehavior for SpireGrowth {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &crate::runtime::combat::MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let tackle_dmg = if ascension_level >= 2 { 18 } else { 16 };
        let smash_dmg = if ascension_level >= 2 { 25 } else { 22 };

        let last_move = entity.move_history.back().copied().unwrap_or(0);
        let constrict_applies = entity.move_history.iter().filter(|&&m| m == 2).count() == 0; // Simplified tracker for Constricted

        let last_two_moves_1 = entity.move_history.len() >= 2
            && entity.move_history[entity.move_history.len() - 1] == 1
            && entity.move_history[entity.move_history.len() - 2] == 1;
        let last_two_moves_3 = entity.move_history.len() >= 2
            && entity.move_history[entity.move_history.len() - 1] == 3
            && entity.move_history[entity.move_history.len() - 2] == 3;

        if ascension_level >= 17 && constrict_applies && last_move != 2 {
            return (2, Intent::StrongDebuff);
        }
        if num < 50 && !last_two_moves_1 {
            return (
                1,
                Intent::Attack {
                    damage: tackle_dmg,
                    hits: 1,
                },
            );
        }

        if constrict_applies && last_move != 2 {
            return (2, Intent::StrongDebuff);
        }

        if !last_two_moves_3 {
            return (
                3,
                Intent::Attack {
                    damage: smash_dmg,
                    hits: 1,
                },
            );
        }

        (
            1,
            Intent::Attack {
                damage: tackle_dmg,
                hits: 1,
            },
        )
    }

    fn take_turn(
        state: &mut CombatState,
        entity: &crate::runtime::combat::MonsterEntity,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.meta.ascension_level;

        match entity.next_move_byte {
            1 => {
                // QUICK_TACKLE
                let dmg = if asc >= 2 { 18 } else { 16 };
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            2 => {
                // CONSTRICT
                let amount = if asc >= 17 { 12 } else { 10 };
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0, // Applies to player
                    power_id: PowerId::Constricted,
                    amount: amount,
                });
            }
            3 => {
                // SMASH
                let dmg = if asc >= 2 { 25 } else { 22 };
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: dmg,
                    output: dmg,
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
