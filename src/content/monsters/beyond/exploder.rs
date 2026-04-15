use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Exploder;

impl MonsterBehavior for Exploder {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &crate::runtime::combat::MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let attack_dmg = if ascension_level >= 2 { 11 } else { 9 };

        let turn_count = entity.move_history.len();
        if turn_count < 2 {
            (
                1,
                Intent::Attack {
                    damage: attack_dmg,
                    hits: 1,
                },
            )
        } else {
            (2, Intent::Unknown)
        }
    }

    fn use_pre_battle_action(
        entity: &crate::runtime::combat::MonsterEntity,
        _hp_rng: &mut crate::runtime::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Explosive,
            amount: 3,
        }]
    }

    fn take_turn(
        state: &mut CombatState,
        entity: &crate::runtime::combat::MonsterEntity,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.meta.ascension_level;

        if entity.next_move_byte == 1 {
            let dmg = if asc >= 2 { 11 } else { 9 };
            actions.push(Action::Damage(DamageInfo {
                source: entity.id,
                target: 0,
                base: dmg,
                output: dmg,
                damage_type: DamageType::Normal,
                is_modified: false,
            }));
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
