use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity, PowerId};
use crate::content::monsters::MonsterBehavior;

pub struct SlaverRed;

impl MonsterBehavior for SlaverRed {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let stab_dmg = if ascension_level >= 2 { 14 } else { 13 };
        let scrape_dmg = if ascension_level >= 2 { 9 } else { 8 };

        // 1: STAB, 2: ENTANGLE, 3: SCRAPE (Attack + Debuff)
        let last_move = entity.move_history.back().copied();
        let last_move_before = if entity.move_history.len() >= 2 {
            entity
                .move_history
                .get(entity.move_history.len() - 2)
                .copied()
        } else {
            None
        };
        let last_two_moves_were =
            |byte: u8| -> bool { last_move == Some(byte) && last_move_before == Some(byte) };

        // Track states based on history
        let first_turn = entity.move_history.is_empty();
        let used_entangle = entity.move_history.contains(&2);

        if first_turn {
            return (
                1,
                Intent::Attack {
                    damage: stab_dmg,
                    hits: 1,
                },
            );
        }
        if num >= 75 && !used_entangle {
            return (2, Intent::StrongDebuff);
        }
        if num >= 55 && used_entangle && !last_two_moves_were(1) {
            return (
                1,
                Intent::Attack {
                    damage: stab_dmg,
                    hits: 1,
                },
            );
        }
        if !last_two_moves_were(3) {
            return (
                3,
                Intent::AttackDebuff {
                    damage: scrape_dmg,
                    hits: 1,
                },
            );
        }

        (
            1,
            Intent::Attack {
                damage: stab_dmg,
                hits: 1,
            },
        )
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let asc = state.meta.ascension_level;
        let stab_dmg = if asc >= 2 { 14 } else { 13 };
        let scrape_dmg = if asc >= 2 { 9 } else { 8 };
        let vuln_amt = if asc >= 17 { 2 } else { 1 };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            1 => {
                // STAB
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: stab_dmg,
                    output: stab_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            2 => {
                // ENTANGLE
                actions.push(Action::ApplyPower {
                    target: 0, // Player
                    source: entity.id,
                    power_id: PowerId::Entangle,
                    amount: 1, // Doesn't stack usually, but amount is 1
                });
            }
            3 => {
                // SCRAPE
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: scrape_dmg,
                    output: scrape_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::ApplyPower {
                    target: 0,
                    source: entity.id,
                    power_id: PowerId::Vulnerable,
                    amount: vuln_amt,
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
