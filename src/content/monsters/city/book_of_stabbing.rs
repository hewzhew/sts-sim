use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent, MonsterEntity};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct BookOfStabbing;

impl BookOfStabbing {
    fn calculate_stab_count(
        ascension_level: u8,
        move_history: &std::collections::VecDeque<u8>,
        is_next_move_stab: bool,
    ) -> u8 {
        let stabs_played = move_history.iter().filter(|&&m| m == 1).count() as i32;
        let big_stabs_played = move_history.iter().filter(|&&m| m == 2).count() as i32;

        let mut count = 1 + stabs_played;
        if ascension_level >= 18 {
            count += big_stabs_played;
        }

        if is_next_move_stab {
            count += 1;
        }
        count as u8
    }
}

impl MonsterBehavior for BookOfStabbing {
    fn use_pre_battle_action(
        entity: &MonsterEntity,
        _rng: &mut crate::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::PainfulStabs,
            amount: 1, // Amount doesn't explicitly matter for PainfulStabs in Java, it just checks for presence
        }]
    }

    fn roll_move(
        rng: &mut crate::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let stab_dmg = if ascension_level >= 3 { 7 } else { 6 };
        let big_stab_dmg = if ascension_level >= 3 { 24 } else { 21 };

        let roll = rng.random_range(0, 99);
        let last_move = entity.move_history.back().copied().unwrap_or(0);
        let second_to_last = if entity.move_history.len() >= 2 {
            entity.move_history[entity.move_history.len() - 2]
        } else {
            0
        };

        let last_two_moves = if entity.move_history.len() >= 2 {
            last_move == 1 && second_to_last == 1
        } else {
            false
        };

        let next_move = if roll < 15 {
            if last_move == 2 {
                1 // STAB
            } else {
                2 // BIG_STAB
            }
        } else if last_two_moves {
            2 // BIG_STAB
        } else {
            1 // STAB
        };

        if next_move == 1 {
            let hits = Self::calculate_stab_count(ascension_level, &entity.move_history, true);
            (
                1,
                Intent::Attack {
                    damage: stab_dmg,
                    hits,
                },
            )
        } else {
            (
                2,
                Intent::Attack {
                    damage: big_stab_dmg,
                    hits: 1,
                },
            )
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();

        let stab_dmg = if state.meta.ascension_level >= 3 {
            7
        } else {
            6
        };
        let big_stab_dmg = if state.meta.ascension_level >= 3 {
            24
        } else {
            21
        };

        match entity.next_move_byte {
            1 => {
                // Java uses the already-resolved stabCount for the current turn.
                // Re-projecting from move history here overcounts by one when the
                // current intent is itself a STAB move.
                let actual_hits = match entity.current_intent {
                    Intent::Attack { hits, .. }
                    | Intent::AttackBuff { hits, .. }
                    | Intent::AttackDebuff { hits, .. }
                    | Intent::AttackDefend { hits, .. } => hits,
                    _ => Self::calculate_stab_count(
                        state.meta.ascension_level,
                        &entity.move_history,
                        false,
                    ),
                };

                for _ in 0..actual_hits {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: stab_dmg,
                        output: stab_dmg,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            2 => {
                // BIG_STAB
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: big_stab_dmg,
                    output: big_stab_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            _ => {}
        }
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::BookOfStabbing;
    use crate::action::Action;
    use crate::combat::{Intent, MonsterEntity};
    use crate::content::monsters::MonsterBehavior;
    use crate::content::test_support::basic_combat;
    use std::collections::VecDeque;

    #[test]
    fn stab_turn_uses_current_intent_hits() {
        let combat = basic_combat();
        let mut entity: MonsterEntity = combat.entities.monsters[0].clone();
        entity.next_move_byte = 1;
        entity.current_intent = Intent::Attack { damage: 6, hits: 2 };
        entity.move_history = VecDeque::from(vec![1u8]);

        let actions = BookOfStabbing::take_turn(&mut combat.clone(), &entity);
        let damage_actions = actions
            .iter()
            .filter(|action| matches!(action, Action::Damage(_)))
            .count();

        assert_eq!(damage_actions, 2);
    }
}
