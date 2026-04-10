use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent, MonsterEntity, PowerId};
use crate::content::monsters::MonsterBehavior;

pub struct Lagavulin;

impl MonsterBehavior for Lagavulin {
    fn use_pre_battle_action(
        entity: &MonsterEntity,
        _hp_rng: &mut crate::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        // Starts asleep with 8 block, 8 Metallicize
        vec![
            Action::GainBlock {
                target: entity.id,
                amount: 8,
            },
            Action::ApplyPower {
                target: entity.id,
                source: entity.id,
                power_id: PowerId::Metallicize,
                amount: 8,
            },
        ]
    }

    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let dmg = if ascension_level >= 3 { 20 } else { 18 };

        if entity.move_history.is_empty() {
            // Initial intent corresponds to SLEEP
            return (5, Intent::Sleep);
        }

        // Check if there are any non-SLEEP moves in the history.
        let mut awake = false;
        for &move_b in &entity.move_history {
            if move_b != 5 {
                awake = true;
                break;
            }
        }
        if !awake {
            return (5, Intent::Sleep);
        }

        // Lagavulin attacks twice, then debuffs.
        // We emulate Java's debuffTurnCount by scanning history backwards for contiguous attacks.
        let mut attack_count = 0;
        for &m in entity.move_history.iter().rev() {
            if m == 1 || m == 4 || m == 5 {
                // 1 (Debuff), 4 (Stun), 5 (Sleep) reset the attack cycle.
                break;
            }
            if m == 3 {
                attack_count += 1;
            }
        }

        if attack_count >= 2 {
            (1, Intent::StrongDebuff)
        } else {
            (
                3,
                Intent::Attack {
                    damage: dmg,
                    hits: 1,
                },
            )
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let dmg = if state.meta.ascension_level >= 3 {
            20
        } else {
            18
        };
        let debuff = if state.meta.ascension_level >= 18 {
            2
        } else {
            1
        }; // Dex/Str down
        let mut actions = Vec::new();

        match entity.next_move_byte {
            1 => {
                // DEBUFF
                actions.push(Action::ApplyPower {
                    target: 0,
                    source: entity.id,
                    power_id: PowerId::Dexterity,
                    amount: -debuff,
                });
                actions.push(Action::ApplyPower {
                    target: 0,
                    source: entity.id,
                    power_id: PowerId::Strength,
                    amount: -debuff,
                });
            }
            3 => {
                // STRONG_ATK
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
                 // Do nothing while stunned.
            }
            5 => {
                // SLEEP
                // Count consecutive sleep turns.
                let mut idle_count = 1; // Includes current turn
                for &b in entity.move_history.iter().rev() {
                    if b == 5 {
                        idle_count += 1;
                    } else {
                        break;
                    }
                }

                // Natural wake up triggers upon reaching 3 sleep cycles.
                if idle_count >= 3 {
                    actions.push(Action::RemovePower {
                        target: entity.id,
                        power_id: PowerId::Metallicize,
                    });

                    // Queue next intent immediately to Attack (3) to skip RollMonsterMove.
                    actions.push(Action::SetMonsterMove {
                        monster_id: entity.id,
                        next_move_byte: 3,
                        intent: Intent::Attack {
                            damage: dmg,
                            hits: 1,
                        },
                    });
                    return actions;
                }
            }
            _ => {}
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }

    fn on_damaged(
        _state: &mut CombatState,
        entity: &MonsterEntity,
        actual_lost: i32,
    ) -> smallvec::SmallVec<[crate::action::ActionInfo; 4]> {
        if actual_lost > 0 && entity.current_intent == Intent::Sleep {
            smallvec::smallvec![
                crate::action::ActionInfo {
                    action: Action::SetMonsterMove {
                        monster_id: entity.id,
                        next_move_byte: 4,
                        intent: crate::combat::Intent::Stun,
                    },
                    insertion_mode: crate::action::AddTo::Top,
                },
                // Java queues ReducePowerAction to BOTTOM via ChangeStateAction("OPEN")
                // Using exactly RemovePower reproduces the correct queue logic without injecting Rust-specific ApplyPower calls.
                crate::action::ActionInfo {
                    action: Action::RemovePower {
                        target: entity.id,
                        power_id: PowerId::Metallicize
                    },
                    insertion_mode: crate::action::AddTo::Bottom,
                }
            ]
        } else {
            smallvec::SmallVec::new()
        }
    }
}
