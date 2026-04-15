use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity};
use crate::content::monsters::MonsterBehavior;

pub struct Centurion;

impl Centurion {
    pub fn roll_move_custom(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
        monsters: &[MonsterEntity],
    ) -> (u8, Intent) {
        let slash_dmg = if ascension_level >= 2 { 14 } else { 12 };
        let fury_dmg = if ascension_level >= 2 { 7 } else { 6 };

        let mut alive_count = 0;
        for m in monsters {
            if !m.is_dying && !m.is_escaped {
                alive_count += 1;
            }
        }

        let num = rng.random_range(0, 99);
        let move_history = &entity.move_history;

        let last_two_moves_2 = move_history.len() >= 2
            && move_history[move_history.len() - 1] == 2
            && move_history[move_history.len() - 2] == 2;
        let last_two_moves_3 = move_history.len() >= 2
            && move_history[move_history.len() - 1] == 3
            && move_history[move_history.len() - 2] == 3;
        let last_two_moves_1 = move_history.len() >= 2
            && move_history[move_history.len() - 1] == 1
            && move_history[move_history.len() - 2] == 1;

        if num >= 65 && !last_two_moves_2 && !last_two_moves_3 {
            if alive_count > 1 {
                return (2, Intent::Defend);
            }
            return (
                3,
                Intent::Attack {
                    damage: fury_dmg,
                    hits: 3,
                },
            );
        }

        if !last_two_moves_1 {
            return (
                1,
                Intent::Attack {
                    damage: slash_dmg,
                    hits: 1,
                },
            );
        }

        (
            3,
            Intent::Attack {
                damage: fury_dmg,
                hits: 3,
            },
        )
    }
}

impl MonsterBehavior for Centurion {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &MonsterEntity,
        _ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        unreachable!("Centurion requires roll_move_custom with monsters slice");
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let slash_dmg = if state.meta.ascension_level >= 2 {
            14
        } else {
            12
        };
        let fury_dmg = if state.meta.ascension_level >= 2 {
            7
        } else {
            6
        };
        let block_amt = if state.meta.ascension_level >= 17 {
            20
        } else {
            15
        };

        match entity.next_move_byte {
            1 => {
                // SLASH
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: slash_dmg,
                    output: slash_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            2 => {
                // PROTECT
                actions.push(Action::GainBlockRandomMonster {
                    source: entity.id,
                    amount: block_amt,
                });
            }
            3 => {
                // FURY
                for _ in 0..3 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: fury_dmg,
                        output: fury_dmg,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            _ => {}
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
