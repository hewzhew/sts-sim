use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity, PowerId};

// LouseDefensive
pub struct LouseDefensive;

impl MonsterBehavior for LouseDefensive {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let bite_dmg = entity.intent_dmg;

        // 3 = BITE, 4 = WEAKEN
        let last_move = entity.move_history.back().copied();
        let last_move_before = if entity.move_history.len() >= 2 {
            entity
                .move_history
                .get(entity.move_history.len() - 2)
                .copied()
        } else {
            None
        };
        let last_two_moves_were =
            |byte: u8| -> bool { last_move == Some(byte) && last_move_before == Some(byte) };

        // Java: Asc 17+ uses lastMove(4) (single check), below Asc 17 uses lastTwoMoves(4)
        if ascension_level >= 17 {
            if num < 25 {
                if last_move == Some(4) {
                    (
                        3,
                        Intent::Attack {
                            damage: bite_dmg,
                            hits: 1,
                        },
                    )
                } else {
                    (4, Intent::Debuff)
                }
            } else if last_two_moves_were(3) {
                (4, Intent::Debuff)
            } else {
                (
                    3,
                    Intent::Attack {
                        damage: bite_dmg,
                        hits: 1,
                    },
                )
            }
        } else {
            if num < 25 {
                if last_two_moves_were(4) {
                    (
                        3,
                        Intent::Attack {
                            damage: bite_dmg,
                            hits: 1,
                        },
                    )
                } else {
                    (4, Intent::Debuff)
                }
            } else if last_two_moves_were(3) {
                (4, Intent::Debuff)
            } else {
                (
                    3,
                    Intent::Attack {
                        damage: bite_dmg,
                        hits: 1,
                    },
                )
            }
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let _asc = state.meta.ascension_level;
        let bite_dmg = entity.intent_dmg;
        let mut actions = Vec::new();

        match entity.next_move_byte {
            3 => {
                // BITE
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0, // Player
                    base: bite_dmg,
                    output: bite_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            4 => {
                // WEAKEN
                actions.push(Action::ApplyPower {
                    target: 0, // Player
                    source: entity.id,
                    power_id: PowerId::Weak,
                    amount: 2,
                });
            }
            _ => {}
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }

    fn use_pre_battle_action(
        entity: &MonsterEntity,
        hp_rng: &mut crate::runtime::rng::StsRng,
        ascension_level: u8,
    ) -> Vec<Action> {
        let curl_up_amount = if ascension_level >= 17 {
            hp_rng.random_range(9, 12) as i32
        } else if ascension_level >= 7 {
            hp_rng.random_range(4, 8) as i32
        } else {
            hp_rng.random_range(3, 7) as i32
        };
        vec![Action::ApplyPower {
            target: entity.id,
            source: entity.id,
            power_id: PowerId::CurlUp,
            amount: curl_up_amount,
        }]
    }
}
