use crate::combat::{CombatState, MonsterEntity, Intent};
use crate::action::{Action, DamageType, DamageInfo};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct SnakePlant;

impl MonsterBehavior for SnakePlant {
    fn use_pre_battle_action(entity: &MonsterEntity, _rng: &mut crate::rng::StsRng, _ascension_level: u8) -> Vec<Action> {
        vec![
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Malleable,
                amount: 3,
            }
        ]
    }

    fn roll_move(_rng: &mut crate::rng::StsRng, entity: &MonsterEntity, ascension_level: u8, num: i32) -> (u8, Intent) {
        let chomp_dmg = if ascension_level >= 2 { 8 } else { 7 };
        let move_history = &entity.move_history;

        let last_two_moves_1 = move_history.len() >= 2 && move_history[move_history.len() - 1] == 1 && move_history[move_history.len() - 2] == 1;
        let last_move_2 = move_history.len() >= 1 && move_history[move_history.len() - 1] == 2;
        let last_move_before_2 = move_history.len() >= 2 && move_history[move_history.len() - 2] == 2;

        if ascension_level >= 17 {
            if num < 65 {
                if last_two_moves_1 {
                    return (2, Intent::StrongDebuff);
                } else {
                    return (1, Intent::Attack { damage: chomp_dmg, hits: 3 });
                }
            } else if last_move_2 || last_move_before_2 {
                return (1, Intent::Attack { damage: chomp_dmg, hits: 3 });
            } else {
                return (2, Intent::StrongDebuff);
            }
        } else {
            if num < 65 {
                if last_two_moves_1 {
                    return (2, Intent::StrongDebuff);
                } else {
                    return (1, Intent::Attack { damage: chomp_dmg, hits: 3 });
                }
            } else if last_move_2 {
                return (1, Intent::Attack { damage: chomp_dmg, hits: 3 });
            } else {
                return (2, Intent::StrongDebuff);
            }
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let chomp_dmg = if state.ascension_level >= 2 { 8 } else { 7 };

        match entity.next_move_byte {
            1 => { // CHOMP
                for _ in 0..3 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0, // Player
                        base: chomp_dmg,
                        output: chomp_dmg,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            2 => { // SPORES
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Frail,
                    amount: 2,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Weak,
                    amount: 2,
                });
            }
            _ => {}
        }

        actions.push(Action::RollMonsterMove { monster_id: entity.id });
        actions
    }
}
