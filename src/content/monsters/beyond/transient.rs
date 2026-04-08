use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Transient;

impl MonsterBehavior for Transient {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &crate::combat::MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let starting_damage = if ascension_level >= 2 { 40 } else { 30 };
        let count = entity.move_history.len() as i32;
        let actual_damage = starting_damage + (count * 10);

        (
            1,
            Intent::Attack {
                damage: actual_damage,
                hits: 1,
            },
        )
    }

    fn use_pre_battle_action(
        entity: &crate::combat::MonsterEntity,
        _hp_rng: &mut crate::rng::StsRng,
        ascension_level: u8,
    ) -> Vec<Action> {
        let fading_turns = if ascension_level >= 17 { 6 } else { 5 };
        vec![
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Fading,
                amount: fading_turns,
            },
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Shifting,
                amount: 1, // Represents presence
            },
        ]
    }

    fn take_turn(state: &mut CombatState, entity: &crate::combat::MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.ascension_level;

        if entity.next_move_byte == 1 {
            let starting_damage = if asc >= 2 { 40 } else { 30 };
            let count = entity.move_history.len() as i32 - 1; // Since it's already in history
            let actual_damage = starting_damage + (count * 10);

            actions.push(Action::Damage(DamageInfo {
                source: entity.id,
                target: 0,
                base: actual_damage,
                output: actual_damage, // Output is managed downstream by engine modifiers typically
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
