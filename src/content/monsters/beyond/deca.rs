use crate::action::{Action, DamageType, DamageInfo};
use crate::combat::{CombatState, Intent};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Deca;

impl MonsterBehavior for Deca {
    fn roll_move(_rng: &mut crate::rng::StsRng, entity: &crate::combat::MonsterEntity, ascension_level: u8, _num: i32) -> (u8, Intent) {
        let beam_dmg = if ascension_level >= 4 { 12 } else { 10 };
        
        if entity.move_history.is_empty() {
             return (0, Intent::AttackDebuff { damage: beam_dmg, hits: 2 }); // BEAM first typically (to alternate cleanly with Donu)
        }

        let last_move = entity.move_history.back().copied().unwrap_or(0);
        
        if last_move == 0 {
             return (2, if ascension_level >= 19 { Intent::DefendBuff } else { Intent::Defend }); // Alternate to Square
        } else {
             return (0, Intent::AttackDebuff { damage: beam_dmg, hits: 2 }); // Alternate to Beam
        }
    }

    fn use_pre_battle_action(entity: &crate::combat::MonsterEntity, _hp_rng: &mut crate::rng::StsRng, ascension_level: u8) -> Vec<Action> {
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
        let asc = state.ascension_level;

        let beam_dmg = if asc >= 4 { 12 } else { 10 };

        match entity.next_move_byte {
            0 => { // BEAM
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
                actions.push(Action::MakeTempCardInDiscard { card_id: crate::content::cards::CardId::Dazed, amount: 2 , upgraded: false });
            },
            2 => { // SQUARE_OF_PROTECTION
                 let alive_monsters: Vec<crate::core::EntityId> = state.monsters.iter()
                     .filter(|m| m.current_hp > 0 && !m.is_dying)
                     .map(|m| m.id)
                     .collect();
                 
                 for target_id in alive_monsters {
                     actions.push(Action::GainBlock {
                         target: target_id,
                         amount: 16,
                     });
                     
                     if asc >= 19 {
                          actions.push(Action::ApplyPower {
                               source: entity.id,
                               target: target_id,
                               power_id: PowerId::PlatedArmor,
                               amount: 3,
                          });
                     }
                 }
            },
            _ => {}
        }

        actions.push(Action::RollMonsterMove { monster_id: entity.id });
        actions
    }
}
