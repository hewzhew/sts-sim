use crate::content::monsters::{EnemyId, MonsterBehavior};
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent};

pub struct BronzeOrb;

impl MonsterBehavior for BronzeOrb {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &crate::runtime::combat::MonsterEntity,
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

    fn take_turn(
        state: &mut CombatState,
        entity: &crate::runtime::combat::MonsterEntity,
    ) -> Vec<Action> {
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
