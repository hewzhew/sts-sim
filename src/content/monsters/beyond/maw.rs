use crate::action::{Action, DamageType, DamageInfo};
use crate::combat::{CombatState, Intent};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Maw;

impl MonsterBehavior for Maw {
    fn roll_move(_rng: &mut crate::rng::StsRng, entity: &crate::combat::MonsterEntity, ascension_level: u8, num: i32) -> (u8, Intent) {
        let slam_dmg = if ascension_level >= 2 { 30 } else { 25 };
        let nom_dmg = 5;
        let turn_count = (entity.move_history.len() as i32) + 2; // +1 for 1-index logic, +1 for impending turn
        
        let roared = entity.move_history.iter().any(|&m| m == 2);
        
        if !roared {
            return (2, Intent::StrongDebuff);
        }
        let last_move = entity.move_history.back().copied().unwrap_or(0);

        if num < 50 && last_move != 5 {
            let hit_counts = if (turn_count / 2) <= 1 { 1 } else { turn_count / 2 };
            return (5, Intent::Attack { damage: nom_dmg, hits: hit_counts as u8 });
        }

        if last_move == 3 || last_move == 5 {
            return (4, Intent::Buff);
        }

        (3, Intent::Attack { damage: slam_dmg, hits: 1 })
    }

    fn take_turn(state: &mut CombatState, entity: &crate::combat::MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.ascension_level;

        let terrify_dur = if asc >= 17 { 5 } else { 3 };
        let str_up = if asc >= 17 { 5 } else { 3 };

        match entity.next_move_byte {
            2 => { // ROAR
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Weak,
                    amount: terrify_dur,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Frail,
                    amount: terrify_dur,
                });
            },
            3 => { // SLAM
                 let dmg = if asc >= 2 { 30 } else { 25 };
                 actions.push(Action::Damage(DamageInfo {
                     source: entity.id,
                     target: 0,
                     base: dmg,
                     output: dmg,
                     damage_type: DamageType::Normal,
                     is_modified: false,
                 }));
            },
            4 => { // DROOL
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Strength,
                    amount: str_up,
                });
            },
            5 => { // NOMNOMNOM
                 let turn_count = (entity.move_history.len() as i32) + 1; // +1 since move already added during sequence
                 let hits = if (turn_count / 2) <= 1 { 1 } else { turn_count / 2 } as u8;
                 for _ in 0..hits {
                     actions.push(Action::Damage(DamageInfo {
                         source: entity.id,
                         target: 0,
                         base: 5,
                         output: 5,
                         damage_type: DamageType::Normal,
                         is_modified: false,
                     }));
                 }
            },
            _ => {}
        }

        actions.push(Action::RollMonsterMove { monster_id: entity.id });
        actions
    }
}
