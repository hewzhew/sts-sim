use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity};
use crate::content::monsters::MonsterBehavior;

pub struct Looter;

impl MonsterBehavior for Looter {
    fn use_pre_battle_action(
        entity: &MonsterEntity,
        _hp_rng: &mut crate::runtime::rng::StsRng,
        ascension_level: u8,
    ) -> Vec<Action> {
        vec![Action::ApplyPower {
            target: entity.id,
            source: entity.id,
            power_id: crate::runtime::combat::PowerId::Thievery,
            amount: if ascension_level >= 17 { 20 } else { 15 },
        }]
    }

    fn roll_move(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let slash_count = entity
            .move_history
            .iter()
            .filter(|&&b| b == 1 || b == 4)
            .count();
        let last_move = entity.move_history.back().copied();

        let lunge_dmg = if ascension_level >= 2 { 14 } else { 12 };
        let mug_dmg = if ascension_level >= 2 { 11 } else { 10 };

        match slash_count {
            0 => (
                1,
                Intent::Attack {
                    damage: mug_dmg,
                    hits: 1,
                },
            ),
            1 => (
                1,
                Intent::Attack {
                    damage: mug_dmg,
                    hits: 1,
                },
            ),
            2 => {
                if last_move == Some(2) || last_move == Some(3) {
                    (3, Intent::Escape)
                } else if rng.random_boolean() {
                    (
                        4,
                        Intent::Attack {
                            damage: lunge_dmg,
                            hits: 1,
                        },
                    )
                } else {
                    (2, Intent::Defend)
                }
            }
            _ => {
                if last_move == Some(2) || last_move == Some(3) {
                    (3, Intent::Escape)
                } else {
                    (2, Intent::Defend)
                }
            }
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mug_dmg = if state.meta.ascension_level >= 2 {
            11
        } else {
            10
        };
        let lunge_dmg = if state.meta.ascension_level >= 2 {
            14
        } else {
            12
        };
        let escape_def = 6;
        let mut actions = Vec::new();

        match entity.next_move_byte {
            1 => {
                // MUG
                // Note: Simulator avoids incrementing physical gold variables here since it does not affect combat outcome predictions.
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: mug_dmg,
                    output: mug_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            4 => {
                // LUNGE
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: lunge_dmg,
                    output: lunge_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            2 => {
                // SMOKE BOMB
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: escape_def,
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
