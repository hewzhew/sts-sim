use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity};

pub struct GremlinWarrior;

impl MonsterBehavior for GremlinWarrior {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let dmg = if ascension_level >= 2 { 5 } else { 4 };
        (
            1,
            Intent::Attack {
                damage: dmg,
                hits: 1,
            },
        )
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let dmg = if state.meta.ascension_level >= 2 {
            5
        } else {
            4
        };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            1 => {
                // SCRATCH
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 1,
                    intent: Intent::Attack {
                        damage: dmg,
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

        actions
    }

    fn use_pre_battle_action(
        entity: &MonsterEntity,
        _hp_rng: &mut crate::runtime::rng::StsRng,
        ascension_level: u8,
    ) -> Vec<Action> {
        let amt = if ascension_level >= 17 { 2 } else { 1 };
        vec![Action::ApplyPower {
            target: entity.id,
            source: entity.id,
            power_id: crate::runtime::combat::PowerId::Angry,
            amount: amt,
        }]
    }
}
