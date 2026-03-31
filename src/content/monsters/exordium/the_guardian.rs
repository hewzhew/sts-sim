use crate::combat::{CombatState, MonsterEntity, Intent, PowerId};
use crate::action::{Action, DamageInfo, DamageType};
use crate::content::monsters::MonsterBehavior;

pub struct TheGuardian;

impl MonsterBehavior for TheGuardian {
    fn use_pre_battle_action(entity: &MonsterEntity, _hp_rng: &mut crate::rng::StsRng, ascension_level: u8) -> Vec<Action> {
        // Mode Shift amount based on Ascension. We use 30 as base.
        let shift_amt = if ascension_level >= 19 { 40 } else if ascension_level >= 9 { 35 } else { 30 };
        vec![
            Action::ApplyPower {
                target: entity.id,
                source: entity.id,
                power_id: PowerId::ModeShift,
                amount: shift_amt,
            },
            Action::ApplyPower {
                target: entity.id,
                source: entity.id,
                power_id: PowerId::GuardianThreshold,
                amount: shift_amt,
            }
        ]
    }

    fn roll_move(_rng: &mut crate::rng::StsRng, entity: &MonsterEntity, ascension_level: u8, _num: i32) -> (u8, Intent) {
        let fierce_bash_dmg = if ascension_level >= 4 { 36 } else { 32 };
        let roll_dmg = if ascension_level >= 4 { 10 } else { 9 };

        if entity.move_history.is_empty() {
            return (6, Intent::Defend); // CHARGE UP
        }

        // ModeShift power removal triggers Defensive Mode when HP is lost.
        // It injects a NextMove to 1 (CLOSE UP).
        // If we are in defensive mode (meaning MoveHistory's recent items are 1, 3, 4)
        // Defensive mode cycle: 1 -> 3 -> 4 -> (reset to offensive / 5)
        
        // Determine if Guardian is in Offensive (open) or Defensive (close) mode
        let mut is_open = true;
        let last_move = *entity.move_history.back().unwrap_or(&6);
        
        if last_move == 1 || last_move == 3 {
            is_open = false; // We just entered or are within defensive mode
        }
        
        if is_open {
            match last_move {
                6 => (2, Intent::Attack { damage: fierce_bash_dmg, hits: 1 }), // CHARGE -> BASH
                2 => (7, Intent::StrongDebuff), // BASH -> STEAM
                7 => (5, Intent::Attack { damage: 5, hits: 4 }), // STEAM -> WHIRLWIND
                5 => (6, Intent::Defend), // WHIRLWIND -> CHARGE
                4 => (5, Intent::Attack { damage: 5, hits: 4 }), // Just came out of defensive Twin Slam, goes to WHIRLWIND
                _ => (6, Intent::Defend),
            }
        } else {
            match last_move {
                1 => (3, Intent::Attack { damage: roll_dmg, hits: 1 }), // CLOSE UP -> ROLL
                3 => (4, Intent::AttackBuff { damage: 8, hits: 2 }), // ROLL -> TWIN SLAM
                _ => (6, Intent::Defend),
            }
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let asc = state.ascension_level;
        let fierce_bash_dmg = if asc >= 4 { 36 } else { 32 };
        let roll_dmg = if asc >= 4 { 10 } else { 9 };
        let _shift_amt = if asc >= 19 { 40 } else if asc >= 9 { 35 } else { 30 };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            1 => { // CLOSE UP
                // The Guardian talks
                // Gains Sharp Hide
                actions.push(Action::ApplyPower {
                    target: entity.id,
                    source: entity.id,
                    power_id: PowerId::SharpHide,
                    amount: if asc >= 19 { 4 } else { 3 },
                });
            }
            2 => { // FIERCE BASH
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: fierce_bash_dmg,
                    output: fierce_bash_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            3 => { // ROLL ATTACK
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: roll_dmg,
                    output: roll_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            4 => { // TWIN SLAM (exits defensive mode)
                // Remove existing block immediately
                actions.push(Action::LoseBlock {
                    target: entity.id,
                    amount: entity.block,
                });
                
                // Reapply ModeShift with current GuardianThreshold value
                // In actual take_turn, we evaluate at Action time, but we can queue it with a dynamic power fetch.
                // Retrieve the current GuardianThreshold value dynamically from state right now
                let cur_thresh = state.get_power(entity.id, PowerId::GuardianThreshold);
                actions.push(Action::ApplyPower {
                    target: entity.id,
                    source: entity.id,
                    power_id: PowerId::ModeShift,
                    amount: cur_thresh,
                });
                
                // Attack
                for _ in 0..2 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: 8,
                        output: 8,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
                
                // Lose Sharp Hide
                actions.push(Action::RemovePower {
                    target: entity.id,
                    power_id: PowerId::SharpHide,
                });
            }
            5 => { // WHIRLWIND
                for _ in 0..4 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: 5,
                        output: 5,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            6 => { // CHARGE UP
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: 9,
                });
            }
            7 => { // VENT STEAM
                actions.push(Action::ApplyPower {
                    target: 0,
                    source: entity.id,
                    power_id: PowerId::Vulnerable,
                    amount: 2,
                });
                actions.push(Action::ApplyPower {
                    target: 0,
                    source: entity.id,
                    power_id: PowerId::Weak,
                    amount: 2,
                });
            }
            _ => { }
        }

        actions.push(Action::RollMonsterMove { monster_id: entity.id });
        actions
    }
}
