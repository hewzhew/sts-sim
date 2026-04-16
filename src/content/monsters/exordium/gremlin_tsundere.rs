use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity};

pub struct GremlinTsundere;

impl MonsterBehavior for GremlinTsundere {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &MonsterEntity,
        _ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        // First move is always Protect.
        (1, Intent::Defend)
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let asc = state.meta.ascension_level;
        let block_amt = if asc >= 17 {
            11
        } else if asc >= 7 {
            8
        } else {
            7
        };
        let bash_dmg = if asc >= 2 { 8 } else { 6 };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            1 => {
                // PROTECT
                actions.push(Action::GainBlockRandomMonster {
                    source: entity.id,
                    amount: block_amt,
                });

                // Java: checks aliveCount (all non-dying, non-escaping monsters)
                // then decides next move based on whether >1 alive
                let alive_count = state
                    .entities
                    .monsters
                    .iter()
                    .filter(|m| !m.is_dying && !m.is_escaped)
                    .count();

                if alive_count > 1 {
                    actions.push(Action::SetMonsterMove {
                        monster_id: entity.id,
                        next_move_byte: 1,
                        intent: Intent::Defend,
                    });
                } else {
                    actions.push(Action::SetMonsterMove {
                        monster_id: entity.id,
                        next_move_byte: 2,
                        intent: Intent::Attack {
                            damage: bash_dmg,
                            hits: 1,
                        },
                    });
                }
            }
            2 => {
                // BASH
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: bash_dmg,
                    output: bash_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                // BASH repeats forever
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 2,
                    intent: Intent::Attack {
                        damage: bash_dmg,
                        hits: 1,
                    },
                });
            }
            99 => {
                // ESCAPE
                actions.push(Action::Escape { target: entity.id });
            }
            _ => {}
        }

        // NOTE: No RollMonsterMove here due to Java's direct SetMove bypassing RollMove actions.
        // Bypassing RollMonsterMove retains strict RNG alignment syncing.

        actions
    }
}
