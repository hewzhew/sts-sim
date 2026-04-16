use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent};

pub struct Spiker;

impl MonsterBehavior for Spiker {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &crate::runtime::combat::MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let attack_dmg = if ascension_level >= 2 { 9 } else { 7 };
        let thorns_count = entity.move_history.iter().filter(|&&m| m == 2).count();
        let last_move = entity.move_history.back().copied().unwrap_or(0);

        if thorns_count > 5 {
            return (
                1,
                Intent::Attack {
                    damage: attack_dmg,
                    hits: 1,
                },
            );
        }

        if entity.move_history.len() < 50 && last_move != 1 {
            return (
                1,
                Intent::Attack {
                    damage: attack_dmg,
                    hits: 1,
                },
            );
        }

        (2, Intent::Buff)
    }

    fn use_pre_battle_action(
        entity: &crate::runtime::combat::MonsterEntity,
        _hp_rng: &mut crate::runtime::rng::StsRng,
        ascension_level: u8,
    ) -> Vec<Action> {
        let mut starting_thorns = if ascension_level >= 2 { 4 } else { 3 };
        if ascension_level >= 17 {
            starting_thorns += 3;
        }

        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Thorns,
            amount: starting_thorns,
        }]
    }

    fn take_turn(
        state: &mut CombatState,
        entity: &crate::runtime::combat::MonsterEntity,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.meta.ascension_level;

        match entity.next_move_byte {
            1 => {
                let dmg = if asc >= 2 { 9 } else { 7 };
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
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Thorns,
                    amount: 2,
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
