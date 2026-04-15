use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent};
use crate::content::monsters::MonsterBehavior;

pub struct TorchHead;

impl MonsterBehavior for TorchHead {
    fn use_pre_battle_action(
        _entity: &crate::runtime::combat::MonsterEntity,
        _hp_rng: &mut crate::runtime::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        Vec::new()
    }

    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &crate::runtime::combat::MonsterEntity,
        _ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        (1, Intent::Attack { damage: 7, hits: 1 })
    }

    fn take_turn(
        _state: &mut CombatState,
        entity: &crate::runtime::combat::MonsterEntity,
    ) -> Vec<Action> {
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
