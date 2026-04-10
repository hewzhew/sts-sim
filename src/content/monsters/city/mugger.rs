use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent, MonsterEntity};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Mugger;

impl MonsterBehavior for Mugger {
    fn use_pre_battle_action(
        entity: &MonsterEntity,
        _rng: &mut crate::rng::StsRng,
        ascension_level: u8,
    ) -> Vec<Action> {
        let gold_amt = if ascension_level >= 17 { 20 } else { 15 };
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Thievery,
            amount: gold_amt,
        }]
    }

    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        _entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let swipe_dmg = if ascension_level >= 2 { 11 } else { 10 };
        // Initial move is always MUG (1)
        (
            1,
            Intent::Attack {
                damage: swipe_dmg,
                hits: 1,
            },
        )
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();

        let swipe_dmg = if state.meta.ascension_level >= 2 {
            11
        } else {
            10
        };
        let big_swipe_dmg = if state.meta.ascension_level >= 2 {
            18
        } else {
            16
        };
        let escape_def = if state.meta.ascension_level >= 17 {
            17
        } else {
            11
        };

        let prior_slashes = entity
            .move_history
            .iter()
            .filter(|&&m| m == 1 || m == 4)
            .count();
        let next_slash_count = prior_slashes + 1;

        match entity.next_move_byte {
            1 => {
                // MUG
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: swipe_dmg,
                    output: swipe_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));

                if next_slash_count == 2 {
                    if state.rng.ai_rng.random_boolean() {
                        actions.push(Action::SetMonsterMove {
                            monster_id: entity.id,
                            next_move_byte: 2, // SMOKE BOMB
                            intent: Intent::Defend,
                        });
                    } else {
                        actions.push(Action::SetMonsterMove {
                            monster_id: entity.id,
                            next_move_byte: 4, // BIG SWIPE
                            intent: Intent::Attack {
                                damage: big_swipe_dmg,
                                hits: 1,
                            },
                        });
                    }
                } else {
                    actions.push(Action::SetMonsterMove {
                        monster_id: entity.id,
                        next_move_byte: 1, // MUG
                        intent: Intent::Attack {
                            damage: swipe_dmg,
                            hits: 1,
                        },
                    });
                }
            }
            4 => {
                // BIG SWIPE
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: big_swipe_dmg,
                    output: big_swipe_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));

                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 2, // SMOKE BOMB
                    intent: Intent::Defend,
                });
            }
            2 => {
                // SMOKE BOMB
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: escape_def,
                });

                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 3, // ESCAPE
                    intent: Intent::Escape,
                });
            }
            3 => {
                // ESCAPE
                actions.push(Action::Escape { target: entity.id });
            }
            _ => {}
        }
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });

        actions
    }
}
