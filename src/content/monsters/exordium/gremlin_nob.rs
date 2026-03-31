use crate::combat::{CombatState, MonsterEntity, Intent, PowerId};
use crate::action::{Action, DamageInfo, DamageType};
use crate::content::monsters::MonsterBehavior;

pub struct GremlinNob;

impl MonsterBehavior for GremlinNob {
    fn roll_move(_rng: &mut crate::rng::StsRng, entity: &MonsterEntity, ascension_level: u8, num: i32) -> (u8, Intent) {
        let bash_dmg = if ascension_level >= 3 { 8 } else { 6 };
        let rush_dmg = if ascension_level >= 3 { 16 } else { 14 };

        if entity.move_history.is_empty() {
            return (3, Intent::Buff);
        }

        let last_move = |byte: u8| entity.move_history.back() == Some(&byte);
        let last_move_before = |byte: u8| {
            entity.move_history.len() >= 2 && entity.move_history[entity.move_history.len() - 2] == byte
        };
        let last_two_moves = |byte: u8| {
            entity.move_history.len() >= 2
                && *entity.move_history.back().unwrap() == byte
                && entity.move_history[entity.move_history.len() - 2] == byte
        };

        // Java Asc 18+: deterministic Skull Bash / Bull Rush rotation
        if ascension_level >= 18 {
            if !last_move(2) && !last_move_before(2) {
                return (2, Intent::AttackDebuff { damage: bash_dmg, hits: 1 });
            }
            if last_two_moves(1) {
                return (2, Intent::AttackDebuff { damage: bash_dmg, hits: 1 });
            }
            return (1, Intent::Attack { damage: rush_dmg, hits: 1 });
        }

        // Below Asc 18: probability-based
        if num < 33 {
            (2, Intent::AttackDebuff { damage: bash_dmg, hits: 1 })
        } else {
            if last_two_moves(1) {
                (2, Intent::AttackDebuff { damage: bash_dmg, hits: 1 })
            } else {
                (1, Intent::Attack { damage: rush_dmg, hits: 1 })
            }
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let bash_dmg = if state.ascension_level >= 3 { 8 } else { 6 };
        let rush_dmg = if state.ascension_level >= 3 { 16 } else { 14 };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            3 => { // BELLOW
                actions.push(Action::ApplyPower {
                    target: entity.id,
                    source: entity.id,
                    power_id: PowerId::Angry, // Enrage in-game
                    amount: if state.ascension_level >= 18 { 3 } else { 2 },
                });
            }
            2 => { // SKULL BASH
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: bash_dmg,
                    output: bash_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                // Applies Vulnerable 2
                actions.push(Action::ApplyPower {
                    target: 0,
                    source: entity.id,
                    power_id: PowerId::Vulnerable,
                    amount: 2,
                });
            }
            1 => { // BULL RUSH
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: rush_dmg,
                    output: rush_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            _ => { }
        }

        actions.push(Action::RollMonsterMove { monster_id: entity.id });
        actions
    }
}
