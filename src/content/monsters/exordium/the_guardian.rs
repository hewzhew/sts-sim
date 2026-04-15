use crate::runtime::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity, PowerId};
use crate::content::monsters::MonsterBehavior;

pub struct TheGuardian;

impl MonsterBehavior for TheGuardian {
    fn use_pre_battle_action(
        entity: &MonsterEntity,
        _hp_rng: &mut crate::runtime::rng::StsRng,
        ascension_level: u8,
    ) -> Vec<Action> {
        // Mode Shift amount based on Ascension. We use 30 as base.
        let shift_amt = if ascension_level >= 19 {
            40
        } else if ascension_level >= 9 {
            35
        } else {
            30
        };
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
            },
        ]
    }

    fn on_damaged(
        state: &mut CombatState,
        entity: &MonsterEntity,
        amount: i32,
    ) -> smallvec::SmallVec<[ActionInfo; 4]> {
        let mut triggered = false;

        if let Some(powers) = state.entities.power_db.get_mut(&entity.id) {
            if let Some(pos) = powers
                .iter()
                .position(|p| p.power_type == PowerId::ModeShift)
            {
                powers[pos].amount -= amount;
                if powers[pos].amount <= 0 {
                    triggered = true;
                    // Java handles Mode Shift inside The Guardian itself. Remove it immediately
                    // so later hits in the same multi-hit sequence cannot re-trigger the transition.
                    powers.remove(pos);
                }
            }
        }

        if triggered {
            // Java queues the defensive-mode transition to the bottom of the action queue from
            // TheGuardian.damage(). That means later hits from the same multi-hit card resolve
            // before the Guardian actually gains 20 block and switches move/intent.
            //
            // We still remove Mode Shift immediately above so subsequent hits cannot retrigger,
            // but the visible state transition itself must wait until the queued follow-up
            // damage actions finish.
            return smallvec::smallvec![
                ActionInfo {
                    action: Action::ApplyPower {
                        target: entity.id,
                        source: entity.id,
                        power_id: PowerId::GuardianThreshold,
                        amount: 10
                    },
                    insertion_mode: AddTo::Bottom
                },
                ActionInfo {
                    action: Action::GainBlock {
                        target: entity.id,
                        amount: 20
                    },
                    insertion_mode: AddTo::Bottom
                },
                ActionInfo {
                    action: Action::SetMonsterMove {
                        monster_id: entity.id,
                        next_move_byte: 1,
                        intent: Intent::Buff
                    },
                    insertion_mode: AddTo::Bottom
                },
            ];
        }

        smallvec::smallvec![]
    }

    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let fierce_bash_dmg = if ascension_level >= 4 { 36 } else { 32 };
        let roll_dmg = if ascension_level >= 4 { 10 } else { 9 };

        if entity.move_history.is_empty() {
            return (6, Intent::Defend); // CHARGE UP
        }

        // Determine if Guardian is in Offensive (open) or Defensive (close) mode
        let mut is_open = true;
        let last_move = *entity.move_history.back().unwrap_or(&6);

        if last_move == 1 || last_move == 3 {
            is_open = false; // We just entered or are within defensive mode
        }

        if is_open {
            match last_move {
                6 => (
                    2,
                    Intent::Attack {
                        damage: fierce_bash_dmg,
                        hits: 1,
                    },
                ), // CHARGE -> BASH
                2 => (7, Intent::StrongDebuff), // BASH -> STEAM
                7 => (5, Intent::Attack { damage: 5, hits: 4 }), // STEAM -> WHIRLWIND
                5 => (6, Intent::Defend),       // WHIRLWIND -> CHARGE
                4 => (5, Intent::Attack { damage: 5, hits: 4 }), // Just came out of defensive Twin Slam, goes to WHIRLWIND
                _ => (6, Intent::Defend),
            }
        } else {
            match last_move {
                1 => (
                    3,
                    Intent::Attack {
                        damage: roll_dmg,
                        hits: 1,
                    },
                ), // CLOSE UP -> ROLL
                3 => (4, Intent::AttackBuff { damage: 8, hits: 2 }), // ROLL -> TWIN SLAM
                _ => (6, Intent::Defend),
            }
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let asc = state.meta.ascension_level;
        let fierce_bash_dmg = if asc >= 4 { 36 } else { 32 };
        let roll_dmg = if asc >= 4 { 10 } else { 9 };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            1 => {
                // CLOSE UP
                // The Guardian talks
                // Gains Sharp Hide
                actions.push(Action::ApplyPower {
                    target: entity.id,
                    source: entity.id,
                    power_id: PowerId::SharpHide,
                    amount: if asc >= 19 { 4 } else { 3 },
                });
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 3,
                    intent: Intent::Attack {
                        damage: roll_dmg,
                        hits: 1,
                    },
                });
            }
            2 => {
                // FIERCE BASH
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: fierce_bash_dmg,
                    output: fierce_bash_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 7,
                    intent: Intent::StrongDebuff,
                });
            }
            3 => {
                // ROLL ATTACK
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: roll_dmg,
                    output: roll_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 4,
                    intent: Intent::AttackBuff { damage: 8, hits: 2 },
                });
            }
            4 => {
                // TWIN SLAM (exits defensive mode)
                // Remove existing block immediately
                actions.push(Action::LoseBlock {
                    target: entity.id,
                    amount: entity.block,
                });

                // Reapply Mode Shift using Guardian's internal threshold tracker.
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
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 5,
                    intent: Intent::Attack { damage: 5, hits: 4 },
                });
            }
            5 => {
                // WHIRLWIND
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
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 6,
                    intent: Intent::Defend,
                });
            }
            6 => {
                // CHARGE UP
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: 9,
                });
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 2,
                    intent: Intent::Attack {
                        damage: fierce_bash_dmg,
                        hits: 1,
                    },
                });
            }
            7 => {
                // VENT STEAM
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
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 5,
                    intent: Intent::Attack { damage: 5, hits: 4 },
                });
            }
            _ => {}
        }
        actions
    }
}
