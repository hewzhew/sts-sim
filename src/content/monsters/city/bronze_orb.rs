use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent};
use crate::content::monsters::{EnemyId, MonsterBehavior};

pub struct BronzeOrb;

impl MonsterBehavior for BronzeOrb {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &crate::combat::MonsterEntity,
        _ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let used_stasis = entity.move_history.iter().any(|&m| m == 3);
        let _turn = entity.move_history.len();

        // Emulating Java `getMove(num)` rng mechanics:
        // The `getMove` behavior branches strictly evaluate against the `num` parameter (0..=99 bounded).
        if !used_stasis && num >= 25 {
            return (3, Intent::StrongDebuff); // Apply Stasis effect
        }

        let id_of_2 = 2; // Support Beam (Defend)
        let id_of_1 = 1; // Laser (Attack)

        let last_two_moves = |byte: u8| -> bool {
            entity.move_history.len() >= 2
                && entity.move_history[entity.move_history.len() - 1] == byte
                && entity.move_history[entity.move_history.len() - 2] == byte
        };

        if num >= 70 && !last_two_moves(id_of_2) {
            return (id_of_2, Intent::Defend);
        }

        if !last_two_moves(id_of_1) {
            return (id_of_1, Intent::Attack { damage: 8, hits: 1 });
        }

        (id_of_2, Intent::Defend)
    }

    fn take_turn(state: &mut CombatState, entity: &crate::combat::MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        match entity.next_move_byte {
            1 => {
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: 8,
                    output: 8,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            2 => {
                if let Some(automaton) = state.entities.monsters.iter().find(|m| {
                    crate::content::monsters::EnemyId::from_id(m.monster_type)
                        == Some(EnemyId::BronzeAutomaton)
                        && !m.is_dying
                }) {
                    actions.push(Action::GainBlock {
                        target: automaton.id,
                        amount: 12,
                    });
                }
            }
            3 => {
                actions.push(Action::ApplyStasis {
                    target_id: entity.id,
                });
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
    use super::*;
    use crate::content::monsters::EnemyId;

    #[test]
    fn beam_does_not_grant_block_to_automaton() {
        let mut combat = crate::content::test_support::basic_combat();
        combat.entities.monsters = vec![
            crate::combat::MonsterEntity {
                id: 1,
                monster_type: EnemyId::BronzeOrb as usize,
                current_hp: 55,
                max_hp: 55,
                block: 0,
                slot: 0,
                is_dying: false,
                is_escaped: false,
                half_dead: false,
                next_move_byte: 1,
                current_intent: Intent::Attack { damage: 8, hits: 1 },
                move_history: std::collections::VecDeque::new(),
                intent_dmg: 8,
                logical_position: 0,
                protocol_identity: Default::default(),
                hexaghost: Default::default(),
                chosen: Default::default(),
                darkling: Default::default(),
                lagavulin: Default::default(),
            },
            crate::combat::MonsterEntity {
                id: 2,
                monster_type: EnemyId::BronzeAutomaton as usize,
                current_hp: 190,
                max_hp: 190,
                block: 0,
                slot: 1,
                is_dying: false,
                is_escaped: false,
                half_dead: false,
                next_move_byte: 0,
                current_intent: Intent::Unknown,
                move_history: std::collections::VecDeque::new(),
                intent_dmg: 0,
                logical_position: 1,
                protocol_identity: Default::default(),
                hexaghost: Default::default(),
                chosen: Default::default(),
                darkling: Default::default(),
                lagavulin: Default::default(),
            },
        ];

        let entity = combat.entities.monsters[0].clone();
        let actions = BronzeOrb::take_turn(&mut combat, &entity);

        assert!(actions
            .iter()
            .any(|action| matches!(action, Action::Damage(_))));
        assert!(!actions.iter().any(|action| matches!(
            action,
            Action::GainBlock {
                target: 2,
                amount: 12
            }
        )));
    }
}
