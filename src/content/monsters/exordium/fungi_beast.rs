use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity, PowerId};
use crate::content::monsters::MonsterBehavior;

pub struct FungiBeast;

impl MonsterBehavior for FungiBeast {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let bite_dmg = 6;
        let _str_amt = if ascension_level >= 17 {
            5
        } else if ascension_level >= 2 {
            4
        } else {
            3
        };

        // 1: BITE, 2: GROW
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

        if num < 60 {
            if last_two_moves_were(1) {
                (2, Intent::Buff)
            } else {
                (
                    1,
                    Intent::Attack {
                        damage: bite_dmg,
                        hits: 1,
                    },
                )
            }
        } else if last_move == Some(2) {
            (
                1,
                Intent::Attack {
                    damage: bite_dmg,
                    hits: 1,
                },
            )
        } else {
            (2, Intent::Buff)
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let bite_dmg = 6;
        let str_amt = if state.meta.ascension_level >= 17 {
            5
        } else if state.meta.ascension_level >= 2 {
            4
        } else {
            3
        };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            1 => {
                // BITE
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: bite_dmg,
                    output: bite_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            2 => {
                // GROW
                actions.push(Action::ApplyPower {
                    target: entity.id,
                    source: entity.id,
                    power_id: PowerId::Strength,
                    amount: str_amt,
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
        _hp_rng: &mut crate::runtime::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        // Java: SporeCloudPower(this, 2) — always 2, regardless of ascension
        vec![Action::ApplyPower {
            target: entity.id,
            source: entity.id,
            power_id: PowerId::SporeCloud,
            amount: 2,
        }]
    }
}
