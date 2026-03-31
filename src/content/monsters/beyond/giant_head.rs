use crate::action::{Action, DamageType, DamageInfo};
use crate::combat::{CombatState, Intent};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct GiantHead;

impl MonsterBehavior for GiantHead {
    fn roll_move(_rng: &mut crate::rng::StsRng, entity: &crate::combat::MonsterEntity, ascension_level: u8, num: i32) -> (u8, Intent) {
        let count_dmg = 13;
        
        let mut count = 5;
        if ascension_level >= 18 { count -= 1; }
        
        // Accurate simulation of state decrement logic
        // It starts at 5 (or 4). Actions are chosen. Then it drops.
        let elapsed_turns = entity.move_history.len() as i32;
        count -= elapsed_turns;
        let starting_death_dmg = if ascension_level >= 3 { 40 } else { 30 };

        if count <= 1 {
            let mut virtual_count = count;
            // Virtual decrement mapping if we reached the execution threshold
            if virtual_count > -6 {
                virtual_count -= 1;
            }
            // `IT_IS_TIME` attack linearly scales
            let death_dmg = starting_death_dmg - (virtual_count * 5);
            return (2, Intent::Attack { damage: death_dmg, hits: 1 });
        }

        let last_two_moves = |byte| {
            entity.move_history.len() >= 2 && 
            entity.move_history[entity.move_history.len()-1] == byte && 
            entity.move_history[entity.move_history.len()-2] == byte
        };

        if num < 50 {
            if !last_two_moves(1) {
                return (1, Intent::Debuff);
            } else {
                return (3, Intent::Attack { damage: count_dmg, hits: 1 });
            }
        }
        
        if !last_two_moves(3) {
             (3, Intent::Attack { damage: count_dmg, hits: 1 })
        } else {
             (1, Intent::Debuff)
        }
    }

    fn use_pre_battle_action(entity: &crate::combat::MonsterEntity, _hp_rng: &mut crate::rng::StsRng, _ascension_level: u8) -> Vec<Action> {
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Slow,
            amount: 0,
        }]
    }

    fn take_turn(state: &mut CombatState, entity: &crate::combat::MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.ascension_level;
        
        // Recover state calculation manually
        let mut count = 5;
        if asc >= 18 { count -= 1; }
        count -= entity.move_history.len() as i32 - 1; // Since it's already added to history

        match entity.next_move_byte {
            1 => { // GLARE
                 actions.push(Action::ApplyPower {
                     source: entity.id,
                     target: 0,
                     power_id: PowerId::Weak,
                     amount: 1,
                 });
            },
            3 => { // COUNT
                 actions.push(Action::Damage(DamageInfo {
                     source: entity.id,
                     target: 0,
                     base: 13,
                     output: 13,
                     damage_type: DamageType::Normal,
                     is_modified: false,
                 }));
            },
            2 => { // IT_IS_TIME
                 let starting_death_dmg = if asc >= 3 { 40 } else { 30 };
                 let actual_count = if count > -6 { count - 1 } else { -6 }; // Bound logic matching Java mapping
                 let death_dmg = starting_death_dmg - (actual_count * 5);
                 
                 actions.push(Action::Damage(DamageInfo {
                     source: entity.id,
                     target: 0,
                     base: death_dmg,
                     output: death_dmg, // output is handled generically via combat engine modifier stack later
                     damage_type: DamageType::Normal,
                     is_modified: false,
                 }));
            },
            _ => {}
        }

        actions.push(Action::RollMonsterMove { monster_id: entity.id });
        actions
    }
}
