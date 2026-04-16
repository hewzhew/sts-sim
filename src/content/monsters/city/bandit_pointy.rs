use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent};

pub struct BanditPointy;

impl MonsterBehavior for BanditPointy {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &crate::runtime::combat::MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let atk_dmg = if ascension_level >= 2 { 6 } else { 5 };
        (
            1,
            Intent::Attack {
                damage: atk_dmg,
                hits: 2,
            },
        ) // POINTY_SPECIAL
    }

    fn take_turn(
        state: &mut CombatState,
        entity: &crate::runtime::combat::MonsterEntity,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.meta.ascension_level;
        let atk_dmg = if asc >= 2 { 6 } else { 5 };

        if entity.next_move_byte == 1 {
            actions.push(Action::Damage(DamageInfo {
                source: entity.id,
                target: 0,
                base: atk_dmg,
                output: atk_dmg,
                damage_type: DamageType::Normal,
                is_modified: false,
            }));
            actions.push(Action::Damage(DamageInfo {
                source: entity.id,
                target: 0,
                base: atk_dmg,
                output: atk_dmg,
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
