use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Donu;

impl MonsterBehavior for Donu {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &crate::combat::MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let beam_dmg = if ascension_level >= 4 { 12 } else { 10 };

        if entity.move_history.is_empty() {
            return (2, Intent::Buff); // CIRCLE_OF_PROTECTION
        }

        let last_move = entity.move_history.back().copied().unwrap_or(0);

        if last_move == 0 {
            return (2, Intent::Buff); // Alternate from Attack to Buff
        } else {
            return (
                0,
                Intent::Attack {
                    damage: beam_dmg,
                    hits: 2,
                },
            ); // Alternate from Buff to Attack
        }
    }

    fn use_pre_battle_action(
        entity: &crate::combat::MonsterEntity,
        _hp_rng: &mut crate::rng::StsRng,
        ascension_level: u8,
    ) -> Vec<Action> {
        let artifact_amt = if ascension_level >= 19 { 3 } else { 2 };
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Artifact,
            amount: artifact_amt,
        }]
    }

    fn take_turn(state: &mut CombatState, entity: &crate::combat::MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.meta.ascension_level;

        let beam_dmg = if asc >= 4 { 12 } else { 10 };

        match entity.next_move_byte {
            0 => {
                // BEAM
                for _ in 0..2 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: beam_dmg,
                        output: beam_dmg,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            2 => {
                // CIRCLE_OF_PROTECTION
                let alive_monsters: Vec<crate::core::EntityId> = state
                    .entities
                    .monsters
                    .iter()
                    .filter(|m| m.current_hp > 0 && !m.is_dying)
                    .map(|m| m.id)
                    .collect();

                for target_id in alive_monsters {
                    actions.push(Action::ApplyPower {
                        source: entity.id,
                        target: target_id, // applies to both Donu & Deca realistically
                        power_id: PowerId::Strength,
                        amount: 3,
                    });
                }
            }
            _ => {}
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
