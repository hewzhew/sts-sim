use crate::combat::{CombatState, MonsterEntity, Intent, PowerId};
use crate::action::{Action, DamageInfo, DamageType};
use crate::content::monsters::MonsterBehavior;

pub struct Lagavulin;

impl MonsterBehavior for Lagavulin {
    fn use_pre_battle_action(entity: &MonsterEntity, _hp_rng: &mut crate::rng::StsRng, _ascension_level: u8) -> Vec<Action> {
        // Starts asleep with 8 block, 8 Metallicize, and LagavulinSleep power
        vec![
            Action::GainBlock { target: entity.id, amount: 8 },
            Action::ApplyPower { target: entity.id, source: entity.id, power_id: PowerId::Metallicize, amount: 8 },
            Action::ApplyPower { target: entity.id, source: entity.id, power_id: PowerId::LagavulinSleep, amount: 1 },
        ]
    }

    fn roll_move(_rng: &mut crate::rng::StsRng, entity: &MonsterEntity, ascension_level: u8, _num: i32) -> (u8, Intent) {
        let dmg = if ascension_level >= 3 { 20 } else { 18 };
        
        if entity.move_history.is_empty() {
            // Initial intent corresponds to SLEEP
            return (5, Intent::Sleep);
        }

        // If it was woken up, it sets move to STUN (4).
        // Then it will ALWAYS play ATTACK (3)
        // Check if there are any non-SLEEP moves in the history.
        let mut awake = false;
        for &move_b in &entity.move_history {
            if move_b != 5 {
                awake = true;
                break;
            }
        }
        // Special case: waking up naturally (3 idle turns completed) sets move dynamically in take_turn.
        // But if we are awake, what's next?
        if !awake {
            return (5, Intent::Sleep);
        }

        // If awake, cycle is 3 -> 3 -> 1 -> 3 -> 3 -> 1
        let last_move = *entity.move_history.back().unwrap_or(&5);
        if last_move == 4 { // Just woke up from damage
            return (3, Intent::Attack { damage: dmg, hits: 1 });
        }

        // Attack cycle determination logic:
        // if last move was 1, next is 3.
        // if last two were 3, next is 1.
        // if last was 3 but before that was 1, next is 3.
        if last_move == 1 {
            return (3, Intent::Attack { damage: dmg, hits: 1 });
        }
        
        let mut two_threes = false;
        if entity.move_history.len() >= 2 {
            let mut iter = entity.move_history.iter().rev();
            if *iter.next().unwrap() == 3 && *iter.next().unwrap() == 3 {
                two_threes = true;
            }
        }

        if two_threes {
            (1, Intent::StrongDebuff)
        } else {
            (3, Intent::Attack { damage: dmg, hits: 1 })
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let dmg = if state.ascension_level >= 3 { 20 } else { 18 };
        let debuff = if state.ascension_level >= 18 { 2 } else { 1 }; // Dex/Str down
        let mut actions = Vec::new();

        match entity.next_move_byte {
            1 => { // DEBUFF
                actions.push(Action::ApplyPower { target: 0, source: entity.id, power_id: PowerId::Dexterity, amount: -debuff });
                actions.push(Action::ApplyPower { target: 0, source: entity.id, power_id: PowerId::Strength, amount: -debuff });
            }
            3 => { // STRONG_ATK
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            4 => { // STUNNED FROM WAKING UP
                // Just stunned, no actions.
            }
            5 => { // SLEEP
                // Keep track of sleep turns through move_history
                let mut idle_count = 1; // implicit
                for &b in entity.move_history.iter().rev() {
                    if b == 5 {
                        idle_count += 1;
                    } else {
                        break;
                    }
                }
                if idle_count >= 3 {
                    // Wakes up naturally!
                    // Java uses ReducePowerAction(Metallicize, 8) — reduce, don't remove entirely
                    actions.push(Action::ApplyPower { target: entity.id, source: entity.id, power_id: PowerId::Metallicize, amount: -8 });
                    actions.push(Action::RemovePower { target: entity.id, power_id: PowerId::LagavulinSleep });
                    // Changes intend to 3 (ATTACK) immediately inside setMove, but we just set the intent instead of executing its attack right away!
                    actions.push(Action::SetMonsterMove {
                        monster_id: entity.id,
                        next_move_byte: 3,
                        intent: Intent::Attack { damage: dmg, hits: 1 },
                    });
                    return actions; // Return early, don't execute RollMonsterMove!
                }
            }
            _ => { }
        }

        actions.push(Action::RollMonsterMove { monster_id: entity.id });
        actions
    }
}
