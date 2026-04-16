use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity};

pub struct GremlinWizard;

impl MonsterBehavior for GremlinWizard {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &MonsterEntity,
        _ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        (2, Intent::Unknown) // First move is always CHARGE
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let magic_dmg = if state.meta.ascension_level >= 2 {
            30
        } else {
            25
        };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            1 => {
                // DOPE_MAGIC
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: magic_dmg,
                    output: magic_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                // After dope magic, optionally reset charge
                if state.meta.ascension_level >= 17 {
                    actions.push(Action::SetMonsterMove {
                        monster_id: entity.id,
                        next_move_byte: 1,
                        intent: Intent::Attack {
                            damage: magic_dmg,
                            hits: 1,
                        },
                    });
                } else {
                    actions.push(Action::SetMonsterMove {
                        monster_id: entity.id,
                        next_move_byte: 2,
                        intent: Intent::Unknown,
                    });
                }
            }
            2 => {
                // CHARGE
                // Count how many times it has consecutively played '2'
                let mut current_charge = 1;
                for byte in entity.move_history.iter().rev() {
                    if *byte == 2 {
                        current_charge += 1;
                    } else {
                        break;
                    }
                }

                if current_charge >= 3 {
                    // Next turn it attacks
                    actions.push(Action::SetMonsterMove {
                        monster_id: entity.id,
                        next_move_byte: 1,
                        intent: Intent::Attack {
                            damage: magic_dmg,
                            hits: 1,
                        },
                    });
                } else {
                    actions.push(Action::SetMonsterMove {
                        monster_id: entity.id,
                        next_move_byte: 2,
                        intent: Intent::Unknown,
                    });
                }
            }
            99 => {
                // ESCAPE
                actions.push(Action::Escape { target: entity.id });
            }
            _ => {}
        }

        actions
    }
}
