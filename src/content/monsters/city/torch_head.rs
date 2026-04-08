use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent};
use crate::content::monsters::MonsterBehavior;

pub struct TorchHead;

impl MonsterBehavior for TorchHead {
    fn use_pre_battle_action(
        entity: &crate::combat::MonsterEntity,
        _hp_rng: &mut crate::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: crate::content::powers::PowerId::Minion,
            amount: 1,
        }]
    }

    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        _entity: &crate::combat::MonsterEntity,
        _ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        (1, Intent::Attack { damage: 7, hits: 1 })
    }

    fn take_turn(_state: &mut CombatState, entity: &crate::combat::MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        if entity.next_move_byte == 1 {
            actions.push(Action::Damage(DamageInfo {
                source: entity.id,
                target: 0,
                base: 7,
                output: 7,
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
